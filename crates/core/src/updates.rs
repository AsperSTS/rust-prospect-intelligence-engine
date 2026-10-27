//! Manual, bounded source checks. A completed check is not a publication date.
use crate::{Db, Profile, Result, Settings, config, invalid, jobs::JobControl};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS source_checks(scope TEXT PRIMARY KEY,source TEXT NOT NULL,last_attempt TEXT NOT NULL DEFAULT '',last_success TEXT NOT NULL DEFAULT '',last_error TEXT NOT NULL DEFAULT '');";
pub(crate) fn initialize(c: &Connection) -> Result<()> {
    use sha1::{Digest, Sha1};
    let checksum = format!("{:x}", Sha1::digest(SCHEMA.as_bytes()));
    if let Some(old) = c
        .query_row(
            "SELECT checksum FROM app_migrations WHERE version=2",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        if old != checksum {
            return Err(invalid("La migración Rust 2 cambió después de aplicarse"));
        }
    } else {
        c.execute_batch(SCHEMA)?;
        c.execute(
            "INSERT INTO app_migrations VALUES (2,?,?)",
            params![checksum, config::now()],
        )?;
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ImportMode {
    New,
    Resume,
    Recheck,
}
#[derive(Clone, Debug)]
pub struct DenueScope {
    pub key: String,
    pub entity: String,
    pub sector: String,
    pub sub: String,
    pub stratum: String,
}
pub fn scopes(settings: &Settings, profile: Profile) -> Result<Vec<DenueScope>> {
    settings.validate()?;
    let sectors = match profile {
        Profile::Automatizacion => vec![
            ("31", "0"),
            ("32", "0"),
            ("33", "0"),
            ("43", "0"),
            ("48", "484"),
            ("48", "488"),
            ("49", "492"),
            ("49", "493"),
            ("23", "0"),
            ("81", "811"),
        ],
        Profile::WebMarketing => vec![
            ("46", "0"),
            ("72", "0"),
            ("54", "0"),
            ("81", "0"),
            ("61", "0"),
            ("62", "0"),
            ("71", "0"),
        ],
    };
    let mut entities = settings
        .entities
        .iter()
        .map(|e| format!("{:02}", e.parse::<u8>().expect("validated entity")))
        .collect::<Vec<_>>();
    entities.sort();
    entities.dedup();
    let mut sizes = settings.estratos.clone();
    sizes.sort();
    sizes.dedup();
    Ok(entities
        .into_iter()
        .flat_map(|entity| {
            sectors
                .iter()
                .flat_map(|&(sector, sub)| {
                    sizes
                        .iter()
                        .map(|stratum| DenueScope {
                            key: format!(
                                "denue:{}:{entity}:{sector}:{sub}:{stratum}",
                                profile.id()
                            ),
                            entity: entity.clone(),
                            sector: sector.into(),
                            sub: sub.into(),
                            stratum: stratum.clone(),
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .collect())
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ScopeProgress {
    pub scope: String,
    pub entity: String,
    pub sector: String,
    pub stratum: String,
    pub next: usize,
    pub started: bool,
    pub completed: bool,
    pub last_success: String,
    pub last_attempt: String,
    pub last_error: String,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SourceStatus {
    pub source: String,
    pub scope: String,
    pub last_attempt: String,
    pub last_success: String,
    pub last_error: String,
}
impl Db {
    pub async fn entity_counts(&self) -> Result<std::collections::BTreeMap<String, usize>> {
        self.memoized("entity-counts".into(), |c| {
            let mut statement =
                c.prepare("SELECT COALESCE(entidad,''),COUNT(*) FROM prospectos GROUP BY entidad")?;
            Ok(statement
                .query_map([], |row| {
                    Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
                })?
                .collect::<std::result::Result<std::collections::BTreeMap<_, _>, _>>()?)
        })
        .await
    }
    pub async fn denue_progress(
        &self,
        settings: Settings,
        profile: Profile,
    ) -> Result<Vec<ScopeProgress>> {
        let scopes = scopes(&settings, profile)?;
        self.call(move|c|scopes.into_iter().map(|scope|{
            let cursor=c.query_row("SELECT next_index,completed,updated_at FROM ingestion_cursors WHERE key=?",[&scope.key],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,bool>(1)?,r.get::<_,String>(2)?))).optional()?;
            let check=c.query_row("SELECT last_attempt,last_success,last_error FROM source_checks WHERE scope=?",[&scope.key],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?))).optional()?;
            let (attempt,success,error)=check.unwrap_or_default();
            let legacy_success=cursor.as_ref().filter(|(_,complete,_)|*complete).map(|(_,_,date)|date.clone()).unwrap_or_default();
            Ok(ScopeProgress{scope:scope.key,entity:scope.entity,sector:scope.sector,stratum:scope.stratum,next:cursor.as_ref().map(|(n,_,_)|*n as usize).unwrap_or(1),started:cursor.is_some(),completed:cursor.as_ref().is_some_and(|(_,done,_)|*done),last_success:if success.is_empty(){legacy_success}else{success},last_attempt:attempt,last_error:error})
        }).collect()).await
    }
    pub async fn prepare_recheck(&self, settings: Settings, profile: Profile) -> Result<()> {
        let scopes = scopes(&settings, profile)?;
        self.call(move|c|{
            let tx=c.transaction()?;
            for scope in &scopes{let pending=tx.query_row("SELECT completed=0 FROM ingestion_cursors WHERE key=?",[&scope.key],|r|r.get::<_,bool>(0)).optional()?.unwrap_or(false);if pending{return Err(invalid("Hay una importación incompleta. Reanúdala antes de iniciar otra revisión."));}}
            for scope in scopes{
                tx.execute("INSERT INTO source_checks(scope,source,last_success) SELECT key,'denue',updated_at FROM ingestion_cursors WHERE key=? AND completed=1 ON CONFLICT(scope) DO NOTHING",[&scope.key])?;
                tx.execute("INSERT INTO ingestion_cursors(key,next_index,completed,updated_at) VALUES (?,1,0,?) ON CONFLICT(key) DO UPDATE SET next_index=1,completed=0,updated_at=excluded.updated_at",params![scope.key,config::now()])?;
            }
            tx.commit()?;Ok(())
        }).await
    }
    pub async fn source_attempt(&self, scope: String, source: String) -> Result<()> {
        self.call(move|c|{c.execute("INSERT INTO source_checks(scope,source,last_attempt) VALUES (?,?,?) ON CONFLICT(scope) DO UPDATE SET last_attempt=excluded.last_attempt,last_error=''",params![scope,source,config::now()])?;Ok(())}).await
    }
    pub async fn source_result(&self, scope: String, success: bool, error: String) -> Result<()> {
        self.call(move|c|{c.execute("UPDATE source_checks SET last_success=CASE WHEN ? THEN ? ELSE last_success END,last_error=? WHERE scope=?",params![success,config::now(),error,scope])?;Ok(())}).await
    }
    pub async fn source_statuses(&self) -> Result<Vec<SourceStatus>> {
        self.call(|c|{let mut st=c.prepare("SELECT source,scope,last_attempt,last_success,last_error FROM source_checks ORDER BY source,scope")?;Ok(st.query_map([],|r|Ok(SourceStatus{source:r.get(0)?,scope:r.get(1)?,last_attempt:r.get(2)?,last_success:r.get(3)?,last_error:r.get(4)?}))?.collect::<std::result::Result<Vec<_>,_>>()?)}).await
    }
    pub async fn pending_domains_with_age(
        &self,
        limit: usize,
        force: bool,
        days: u32,
    ) -> Result<Vec<String>> {
        if !(1..=3650).contains(&days) {
            return Err(invalid("Vigencia web fuera de rango"));
        }
        self.call(move|c|{let cutoff=(chrono::Local::now()-chrono::Duration::days(i64::from(days))).format("%Y-%m-%dT%H:%M:%S").to_string();let mut st=c.prepare("SELECT p.dominio FROM prospectos p LEFT JOIN web_cache w ON w.domain=p.dominio WHERE p.dominio IS NOT NULL AND p.dominio<>'' AND p.estado NOT IN ('descartado','no_interesado','sin_respuesta','cliente') AND (? OR w.checked_at IS NULL OR w.status='error' OR w.checked_at<?) GROUP BY p.dominio ORDER BY MAX(p.fit_score) DESC,p.dominio LIMIT ?")?;Ok(st.query_map(params![force,cutoff,limit as i64],|r|r.get(0))?.collect::<std::result::Result<Vec<_>,_>>()?)}).await
    }
}

/// Injected fetch permits deterministic tests without a key or external requests.
pub async fn run_batch<F, Fut>(
    db: &Db,
    settings: &Settings,
    profile: Profile,
    mode: ImportMode,
    control: &JobControl,
    mut fetch: F,
) -> Result<usize>
where
    F: FnMut(DenueScope, usize) -> Fut,
    Fut: std::future::Future<Output = Result<Vec<Value>>>,
{
    control.check()?;
    let all = scopes(settings, profile)?;
    if mode == ImportMode::Recheck {
        db.prepare_recheck(settings.clone(), profile).await?;
    }
    let progress = db.denue_progress(settings.clone(), profile).await?;
    if mode == ImportMode::New && progress.iter().any(|p| p.started && !p.completed) {
        return Err(invalid(
            "Hay lotes incompletos. Usa Reanudar antes de importar otros.",
        ));
    }
    let selected = all
        .into_iter()
        .zip(progress)
        .filter(|(_, p)| match mode {
            ImportMode::New => !p.started,
            ImportMode::Resume | ImportMode::Recheck => p.started && !p.completed,
        })
        .collect::<Vec<_>>();
    let mut read = 0;
    for (scope, p) in selected {
        let mut start = p.next;
        loop {
            control.check()?;
            if control.snapshot().pages >= settings.denue_pages {
                return Ok(read);
            }
            control.update(|s| {
                s.message = format!(
                    "DENUE · {} · sector {} · página desde {start}",
                    config::entity_name(&scope.entity),
                    scope.sector
                )
            });
            db.source_attempt(scope.key.clone(), "denue".into()).await?;
            let raw = match fetch(scope.clone(), start).await {
                Ok(raw) => raw,
                Err(error) => {
                    db.source_result(scope.key.clone(), false, error.to_string())
                        .await?;
                    return Err(error);
                }
            };
            control.check()?;
            if raw.len() > 200 {
                let message = "DENUE devolvió más registros que los solicitados";
                db.source_result(scope.key.clone(), false, message.into())
                    .await?;
                return Err(invalid(message));
            }
            let returned = raw.len();
            let complete = raw.len() < 200;
            let mut records = Vec::new();
            let mut invalid_rows = 0;
            for raw in raw {
                match crate::sources::denue_record(raw, &scope.entity) {
                    Ok(p) => records.push(p),
                    Err(_) => {
                        invalid_rows += 1;
                        control.update(|s| s.invalid += 1);
                    }
                }
            }
            if returned > 0 && records.is_empty() {
                let message =
                    "La página recibida no contiene registros válidos. Se conserva el cursor.";
                db.source_result(scope.key.clone(), false, message.into())
                    .await?;
                return Err(invalid(message));
            }
            let count = records.len();
            let summary = db
                .import_audited_page(
                    records,
                    crate::workspace::DenuePageAudit {
                        job_id: control.snapshot().id,
                        profile,
                        scope: scope.key.clone(),
                        start,
                        invalid: invalid_rows,
                    },
                    complete,
                )
                .await?;
            let (new, changed) = (summary.inserted, summary.updated);
            if complete {
                db.source_result(scope.key.clone(), true, String::new())
                    .await?;
            }
            read += count;
            control.update(|s| {
                s.pages += 1;
                s.processed = read;
                s.inserted += new;
                s.updated += changed;
                s.unchanged += count - new - changed;
            });
            db.log_job(control.snapshot()).await?;
            if complete {
                break;
            }
            start += 200;
            if control.snapshot().pages < settings.denue_pages {
                control.wait(std::time::Duration::from_millis(400)).await?;
            }
        }
    }
    Ok(read)
}
pub fn record_limit_pages(limit: Option<usize>, default: usize) -> Result<usize> {
    match limit {
        None => Ok(default),
        Some(n) if (200..=4000).contains(&n) && n.is_multiple_of(200) => Ok(n / 200),
        _ => Err(invalid(
            "El límite debe ser un múltiplo de 200, entre 200 y 4000 registros por lote",
        )),
    }
}
pub async fn pull_batch(
    db: &Db,
    settings: &Settings,
    profile: Profile,
    mode: ImportMode,
    control: &JobControl,
) -> Result<usize> {
    let token = db.paths.secrets()?.denue;
    if token.is_empty() {
        return Err(invalid("Configura tu token DENUE en Ajustes → Fuentes."));
    }
    let client = crate::sources::GovernmentClient::new()?;
    run_batch(db, settings, profile, mode, control, |scope, start| {
        let token = token.clone();
        let client = client.clone();
        let control = control.clone();
        async move {
            client
                .denue_page(
                    &token,
                    &scope.entity,
                    &scope.sector,
                    &scope.sub,
                    &scope.stratum,
                    start,
                    &control,
                )
                .await
        }
    })
    .await
}
pub fn age_label(timestamp: &str, days: u32) -> String {
    if timestamp.is_empty() {
        return "Sin consulta completa registrada".into();
    }
    match chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%dT%H:%M:%S") {
        Ok(date) => {
            let age = (config::today() - date.date()).num_days().max(0);
            if age >= i64::from(days) {
                format!("Revisión sugerida · hace {age} días")
            } else {
                format!("Consulta reciente · hace {age} días")
            }
        }
        Err(_) => "Fecha de consulta por verificar".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AppPaths, Prospect, Query};
    #[tokio::test]
    async fn bounded_import_resumes_and_completed_scopes_are_not_repeated() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        let settings = Settings {
            entities: vec!["21".into()],
            estratos: vec!["3".into()],
            denue_pages: 1,
            ..Default::default()
        };
        let first = JobControl::new("test");
        run_batch(
            &db,
            &settings,
            Profile::Automatizacion,
            ImportMode::New,
            &first,
            |_, start| async move {
                assert_eq!(start, 1);
                Ok((0..200)
                    .map(
                        |i| serde_json::json!({"Id":format!("{i}"),"Nombre":format!("Prueba {i}")}),
                    )
                    .collect())
            },
        )
        .await
        .unwrap();
        assert_eq!(first.snapshot().pages, 1);
        let summaries = db
            .denue_batch_summaries(Profile::Automatizacion)
            .await
            .unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, first.snapshot().id);
        assert_eq!((summaries[0].pages, summaries[0].inserted), (1, 200));
        assert_eq!(
            db.page(Query {
                view: crate::View::All,
                size: 1,
                ..Default::default()
            })
            .await
            .unwrap()
            .total,
            200
        );
        assert!(
            run_batch(
                &db,
                &settings,
                Profile::Automatizacion,
                ImportMode::New,
                &JobControl::new("test"),
                |_, _| async { panic!("must resume before a new import") }
            )
            .await
            .is_err()
        );
        let resumed = JobControl::new("test");
        run_batch(
            &db,
            &settings,
            Profile::Automatizacion,
            ImportMode::Resume,
            &resumed,
            |_, start| async move {
                assert_eq!(start, 201);
                Ok(vec![])
            },
        )
        .await
        .unwrap();
        let repeat = JobControl::new("test");
        run_batch(
            &db,
            &settings,
            Profile::Automatizacion,
            ImportMode::Resume,
            &repeat,
            |_, _| async { panic!("completed scope must not fetch") },
        )
        .await
        .unwrap();
        assert_eq!(repeat.snapshot().pages, 0);
        let old = db
            .denue_progress(settings.clone(), Profile::Automatizacion)
            .await
            .unwrap()[0]
            .last_success
            .clone();
        assert!(!old.is_empty());
        db.prepare_recheck(settings.clone(), Profile::Automatizacion)
            .await
            .unwrap();
        assert_eq!(
            db.denue_progress(settings.clone(), Profile::Automatizacion)
                .await
                .unwrap()[0]
                .last_success,
            old
        );
        assert!(
            db.prepare_recheck(settings, Profile::Automatizacion)
                .await
                .is_err()
        );
    }
    #[tokio::test]
    async fn unchanged_public_rows_are_not_written_and_crm_is_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        let mut p = Prospect {
            id: "1".into(),
            nombre: "Prueba".into(),
            ..Default::default()
        };
        assert_eq!(db.import_records(vec![p.clone()]).await.unwrap(), (1, 0));
        assert_eq!(db.import_records(vec![p.clone()]).await.unwrap(), (0, 0));
        db.call(|c| {
            c.execute(
                "UPDATE prospectos SET notas='conservar',estado='revisado' WHERE id='1'",
                [],
            )?;
            Ok(())
        })
        .await
        .unwrap();
        p.nombre = "Prueba actualizada".into();
        assert_eq!(db.import_records(vec![p]).await.unwrap(), (0, 1));
        let p = db.get("1".into(), Profile::Automatizacion).await.unwrap();
        assert_eq!(p.notas, "conservar");
        assert_eq!(p.estado, "revisado");
    }
    #[tokio::test]
    async fn equal_municipality_names_are_kept_separate_by_entity() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        let a = Prospect {
            id: "a".into(),
            nombre: "Uno".into(),
            entidad: "Puebla".into(),
            municipio: "Municipio compartido".into(),
            latitud: Some(19.0),
            longitud: Some(-98.0),
            ..Default::default()
        };
        let b = Prospect {
            id: "b".into(),
            nombre: "Dos".into(),
            entidad: "Tlaxcala".into(),
            municipio: a.municipio.clone(),
            latitud: a.latitud,
            longitud: a.longitud,
            ..Default::default()
        };
        db.import_records(vec![a, b]).await.unwrap();
        let page = db
            .page(Query {
                view: crate::View::All,
                entidad: "Puebla".into(),
                size: 10,
                ..Default::default()
            })
            .await
            .unwrap();
        assert_eq!(page.total, 1);
        assert_eq!(page.items[0].nombre, "Uno");
        let metrics = db
            .metrics(Profile::Automatizacion, "municipio".into())
            .await
            .unwrap();
        assert_eq!(metrics.territories.len(), 2);
        assert_eq!(metrics.points.len(), 2);
        assert_ne!(metrics.points[0].entidad, metrics.points[1].entidad);
    }
    #[tokio::test]
    async fn failed_website_checks_can_be_retried_manually_before_the_success_cache_expires() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        db.import_records(vec![Prospect {
            id: "site".into(),
            nombre: "Prueba".into(),
            dominio: "example.org".into(),
            ..Default::default()
        }])
        .await
        .unwrap();
        db.apply_enrichment(
            "example.org".into(),
            crate::enrich::Enrichment {
                status: "ok".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert!(
            db.pending_domains_with_age(100, false, 30)
                .await
                .unwrap()
                .is_empty()
        );
        db.apply_enrichment(
            "example.org".into(),
            crate::enrich::Enrichment {
                status: "error".into(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        assert_eq!(
            db.pending_domains_with_age(100, false, 30).await.unwrap(),
            vec!["example.org"]
        );
    }
    #[tokio::test]
    async fn failed_and_cancelled_checks_keep_prior_success() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        db.source_attempt("banxico".into(), "banxico".into())
            .await
            .unwrap();
        db.source_result("banxico".into(), true, String::new())
            .await
            .unwrap();
        let old = db.source_statuses().await.unwrap()[0].last_success.clone();
        db.source_attempt("banxico".into(), "banxico".into())
            .await
            .unwrap();
        db.source_result("banxico".into(), false, "sin conexión".into())
            .await
            .unwrap();
        assert_eq!(db.source_statuses().await.unwrap()[0].last_success, old);
        let control = JobControl::new("test");
        control.token.cancel();
        assert!(
            run_batch(
                &db,
                &Settings::default(),
                Profile::Automatizacion,
                ImportMode::New,
                &control,
                |_, _| async { panic!("cancelled must not fetch") }
            )
            .await
            .is_err()
        );
    }
}
