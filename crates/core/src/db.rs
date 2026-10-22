use crate::{
    Analysis, AppPaths, Command, Error, Interaction, Mutation, Page, Profile, Prospect, Query,
    Result, Settings, Signal, Summary, View, config, groups, invalid, messages, rules,
};
use chrono::{Duration, NaiveDate};
use rusqlite::{
    Connection, OpenFlags, OptionalExtension, params,
    types::{Value, ValueRef},
};
use serde::{Deserialize, Serialize};
use std::{
    any::Any,
    collections::{BTreeMap, HashSet, VecDeque},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration as Wait,
};
use tokio::sync::{mpsc, oneshot};

pub type SqliteConnection = Connection;
pub fn all_mut(c: &mut Connection) -> Result<Vec<Prospect>> {
    all(c)
}

// Owned by each database actor; no disk cache, credentials or network work.
#[derive(Default)]
struct ReadCache {
    entries: VecDeque<(String, Arc<dyn Any + Send + Sync>)>,
    data_version: i64,
    date: String,
}
impl ReadCache {
    fn observe(&mut self, c: &Connection) -> Result<()> {
        let version = c.query_row("PRAGMA data_version", [], |r| r.get(0))?;
        let date = config::today().to_string();
        if self.data_version != version || self.date != date {
            self.entries.clear();
            self.data_version = version;
            self.date = date;
        }
        Ok(())
    }
    fn get<T: Clone + 'static>(&mut self, key: &str) -> Option<T> {
        let i = self.entries.iter().position(|(k, _)| k == key)?;
        let entry = self.entries.remove(i)?;
        let value = entry.1.downcast_ref::<T>().cloned();
        self.entries.push_back(entry);
        value
    }
    fn insert<T: Send + Sync + 'static>(&mut self, key: String, value: T) {
        self.entries.retain(|(k, _)| *k != key);
        if self.entries.len() >= 32 {
            self.entries.pop_front();
        }
        self.entries.push_back((key, Arc::new(value)));
    }
}
type Task = Box<dyn FnOnce(&mut Connection) + Send>;
#[derive(Clone)]
pub struct Db {
    sender: mpsc::Sender<Task>,
    pub paths: AppPaths,
    cache: Arc<Mutex<ReadCache>>,
}
impl Db {
    pub fn open(paths: AppPaths) -> Result<Self> {
        let (sender, mut receiver) = mpsc::channel::<Task>(32);
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);
        let worker_paths = paths.clone();
        std::thread::Builder::new()
            .name("prospect-sqlite".into())
            .spawn(move || {
                let result = Connection::open(&worker_paths.db)
                    .map_err(Error::from)
                    .and_then(|mut c| {
                        initialize(&mut c, &worker_paths)?;
                        Ok(c)
                    });
                match result {
                    Ok(mut connection) => {
                        let _ = ready_tx.send(Ok(()));
                        while let Some(task) = receiver.blocking_recv() {
                            task(&mut connection);
                        }
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e));
                    }
                }
            })?;
        ready_rx.recv().map_err(|_| Error::WorkerClosed)??;
        Ok(Self {
            sender,
            paths,
            cache: Arc::default(),
        })
    }
    pub async fn call<T: Send + 'static>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let (tx, rx) = oneshot::channel();
        let cache = self.cache.clone();
        self.sender
            .send(Box::new(move |con| {
                let before = con.total_changes();
                let result = f(con);
                // Also invalidate after partial writes followed by an error/rollback.
                if con.total_changes() != before {
                    cache
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .entries
                        .clear();
                }
                let _ = tx.send(result);
            }))
            .await
            .map_err(|_| Error::WorkerClosed)?;
        rx.await.map_err(|_| Error::WorkerClosed)?
    }
    pub(crate) async fn memoized<T: Clone + Send + Sync + 'static>(
        &self,
        key: String,
        f: impl FnOnce(&mut Connection) -> Result<T> + Send + 'static,
    ) -> Result<T> {
        let cache = self.cache.clone();
        self.call(move |c| {
            {
                let mut cache = cache.lock().unwrap_or_else(|e| e.into_inner());
                cache.observe(c)?;
                if let Some(value) = cache.get(&key) {
                    return Ok(value);
                }
            }
            let value = f(c)?;
            cache
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(key, value.clone());
            Ok(value)
        })
        .await
    }
    pub async fn page(&self, q: Query) -> Result<Page> {
        q.validate()?;
        self.memoized(format!("page:{}", serde_json::to_string(&q)?), move |c| {
            list(c, &q)
        })
        .await
    }
    pub async fn get(&self, id: String, profile: Profile) -> Result<Prospect> {
        self.call(move|c|{

        let sql="SELECT p.*, COALESCE(s.fit_score,p.fit_score) AS fit_score, COALESCE(s.priority_score,p.priority_score) AS priority_score, s.analysis_json AS profile_analysis FROM prospectos p LEFT JOIN profile_scores s ON s.prospect_id=p.id AND s.profile_id=? WHERE p.id=?";
        c.query_row(sql,params![profile.id(),id],decode_profile).optional()?.ok_or_else(||invalid("Establecimiento inexistente"))
    }).await
    }
    pub async fn signals(&self, id: String) -> Result<Vec<Signal>> {
        self.call(move |c| signals(c, &id)).await
    }
    pub async fn history(&self, id: String) -> Result<Vec<Interaction>> {
        self.call(move |c| {
            let mut st = c.prepare("SELECT * FROM interactions WHERE prospect_id=? ORDER BY id")?;
            let items = st
                .query_map([id], |r| {
                    let json = row_json(r)?;
                    serde_json::from_value(json).map_err(|e| {
                        rusqlite::Error::FromSqlConversionFailure(
                            0,
                            rusqlite::types::Type::Text,
                            Box::new(e),
                        )
                    })
                })?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            Ok(items)
        })
        .await
    }
    /// SQLite metadata only. Own actor writes do not change this counter.
    pub async fn external_data_version(&self) -> Result<i64> {
        self.call(|c| Ok(c.query_row("PRAGMA data_version", [], |r| r.get(0))?))
            .await
    }
    pub async fn summary(&self) -> Result<Summary> {
        self.memoized("summary".into(), summary).await
    }
    pub async fn refresh_priorities(&self) -> Result<usize> {
        self.call(|c| {
            let tx = c.transaction()?;
            let count = refresh_scores(&tx, None, config::today())?;
            tx.commit()?;
            Ok(count)
        })
        .await
    }
    pub async fn dimensions(&self) -> Result<(Vec<String>, Vec<String>)> {
        self.dimensions_for(String::new()).await
    }
    pub async fn dimensions_for(&self, entity: String) -> Result<(Vec<String>, Vec<String>)> {
        self.memoized(format!("dimensions:{entity}"), move|c|{let get=|column:&str|->Result<Vec<String>>{let mut st=c.prepare(&format!("SELECT DISTINCT {column} FROM prospectos WHERE (?='' OR entidad=?) AND COALESCE({column},'')<>'' ORDER BY {column}"))?;Ok(st.query_map(params![entity,entity],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?)};Ok((get("giro")?,get("municipio")?))}).await
    }
    pub async fn analyze(
        &self,
        settings: Settings,
        control: crate::jobs::JobControl,
    ) -> Result<usize> {
        let paths = self.paths.clone();
        self.call(move |c| analyze_all(c, &paths, &settings, &control, config::today()))
            .await
    }
    pub async fn mutate(&self, m: Mutation, settings: Settings) -> Result<String> {
        self.mutate_at(m, settings, config::today()).await
    }
    pub async fn mutate_at(
        &self,
        m: Mutation,
        settings: Settings,
        today: NaiveDate,
    ) -> Result<String> {
        m.validate()?;
        self.call(move |c| mutate(c, &m, &settings, today)).await
    }
    pub async fn backup(&self) -> Result<PathBuf> {
        let p = self.paths.clone();
        self.call(move |c| backup(c, &p.backups, "manual")).await
    }
    pub async fn restore_to(&self, destination: PathBuf) -> Result<PathBuf> {
        self.call(move |c| {
            if destination.exists() {
                return Err(invalid(
                    "El destino de recuperación ya existe; usa una ruta nueva",
                ));
            }
            if let Some(parent) = destination.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut copy = Connection::open(&destination)?;
            rusqlite::backup::Backup::new(c, &mut copy)?.run_to_completion(
                100,
                Wait::from_millis(5),
                None,
            )?;
            verify(&copy)?;
            Ok(destination)
        })
        .await
    }
    pub async fn export(&self, q: Query, format: &str) -> Result<PathBuf> {
        q.validate()?;
        let query_metadata = q.clone();
        let data = self.call(move |c| export_rows(c, &q)).await?;
        let metadata = serde_json::json!({
            "source": "INEGI — Directorio Estadístico Nacional de Unidades Económicas (DENUE)",
            "source_url": "https://www.inegi.org.mx/app/mapa/denue/",
            "exported_at": crate::config::now(), "records": data.len(),
            "filters": query_metadata, "application": format!("Prospect Intelligence {}", env!("CARGO_PKG_VERSION")),
            "transformations": ["Normalización de contactos", "Reglas comerciales por oferta", "Señales públicas cuando existe evidencia", "Notas y estados capturados por el usuario"],
            "scope": "Establecimientos de la base local; no representa el universo completo del mercado.",
            "csv_formula_protection": true
        });
        let path = self.paths.exports.join(format!(
            "prospectos_{}_{}.{}",
            chrono::Local::now().format("%Y%m%d_%H%M%S"),
            crate::operation_id(),
            if format == "xlsx" { "xlsx" } else { "csv" }
        ));
        let fmt = format.to_owned();
        let out = path.clone();
        tokio::task::spawn_blocking(move || write_export(&out, &data, &fmt))
            .await
            .map_err(|_| Error::WorkerClosed)??;
        crate::config::write_private(
            &path.with_extension(format!("{format}.metadata.json")),
            &serde_json::to_vec_pretty(&metadata)?,
        )?;
        Ok(path)
    }
    pub async fn checkpoint(&self, key: String) -> Result<usize> {
        self.call(move |c| {
            Ok(c.query_row(
                "SELECT next_index FROM ingestion_cursors WHERE key=? AND completed=0",
                [key],
                |r| r.get::<_, i64>(0).map(|n| n as usize),
            )
            .optional()?
            .unwrap_or(1))
        })
        .await
    }
    pub async fn import_page(
        &self,
        records: Vec<Prospect>,
        key: String,
        next: usize,
        done: bool,
    ) -> Result<(usize, usize)> {
        self.call(move|c|{
        let tx=c.transaction()?;let stats=upsert(&tx,&records)?;tx.execute("INSERT INTO ingestion_cursors(key,next_index,completed,updated_at) VALUES (?,?,?,?) ON CONFLICT(key) DO UPDATE SET next_index=excluded.next_index,completed=excluded.completed,updated_at=excluded.updated_at",params![key,next as i64,done,config::now()])?;tx.commit()?;Ok(stats)
    }).await
    }
    pub async fn import_records(&self, records: Vec<Prospect>) -> Result<(usize, usize)> {
        self.call(move |c| {
            let tx = c.transaction()?;
            let stats = upsert(&tx, &records)?;
            tx.commit()?;
            Ok(stats)
        })
        .await
    }
    pub async fn pending_domains(&self, limit: usize, force: bool) -> Result<Vec<String>> {
        self.call(move|c|{
        let cutoff=(chrono::Local::now()-Duration::days(30)).format("%Y-%m-%dT%H:%M:%S").to_string();
        let mut st=c.prepare("SELECT p.dominio FROM prospectos p LEFT JOIN web_cache w ON w.domain=p.dominio WHERE p.dominio IS NOT NULL AND p.dominio<>'' AND p.estado NOT IN ('descartado','no_interesado','sin_respuesta','cliente') AND (? OR w.checked_at IS NULL OR w.status='error' OR w.checked_at<?) GROUP BY p.dominio ORDER BY MAX(p.fit_score) DESC,p.dominio LIMIT ?")?;Ok(st.query_map(params![force,cutoff,limit as i64],|r|r.get(0))?.collect::<std::result::Result<Vec<_>,_>>()?)
    }).await
    }
    pub async fn apply_enrichment(
        &self,
        domain: String,
        res: crate::enrich::Enrichment,
    ) -> Result<usize> {
        self.call(move|c|{
        let tx=c.transaction()?;let now=config::now();let mut count=0;
        if res.status=="ok"{
            let ids={let mut st=tx.prepare("SELECT id FROM prospectos WHERE dominio=?")?;st.query_map([&domain],|r|r.get::<_,String>(0))?.collect::<std::result::Result<Vec<_>,_>>()?};
            for id in &ids{tx.execute("DELETE FROM web_signals WHERE prospect_id=?",[id])?;for signal in &res.signals{tx.execute("INSERT INTO web_signals(prospect_id,signal_type,signal_text,signal_source_url) VALUES (?,?,?,?)",params![id,signal.signal_type,signal.signal_text,signal.signal_source_url])?;}tx.execute("UPDATE prospectos SET source_website=1,website_checked_at=?,enriquecido_en=?,correo_web=CASE WHEN ?<>'' THEN ? ELSE correo_web END,whatsapp=CASE WHEN ?<>'' THEN ? ELSE whatsapp END WHERE id=?",params![now,config::today().to_string(),res.email,res.email,res.whatsapp,res.whatsapp,id])?;}count=ids.len();
        }
        tx.execute("INSERT INTO web_cache(domain,checked_at,status,pages_fetched,email,whatsapp) VALUES (?,?,?,?,?,?) ON CONFLICT(domain) DO UPDATE SET checked_at=excluded.checked_at,status=excluded.status,pages_fetched=excluded.pages_fetched,email=CASE WHEN excluded.status='ok' THEN excluded.email ELSE web_cache.email END,whatsapp=CASE WHEN excluded.status='ok' THEN excluded.whatsapp ELSE web_cache.whatsapp END",params![domain,now,res.status,res.pages as i64,res.email,res.whatsapp])?;tx.commit()?;Ok(count)
    }).await
    }
    pub async fn log_job(&self, state: crate::jobs::JobState) -> Result<()> {
        self.call(move|c|{c.execute("INSERT INTO jobs(id,kind,status,state_json,updated_at) VALUES (?,?,?,?,?) ON CONFLICT(id) DO UPDATE SET status=excluded.status,state_json=excluded.state_json,updated_at=excluded.updated_at",params![state.id,state.kind,state.status,serde_json::to_string(&state)?,config::now()])?;Ok(())}).await
    }
}

#[derive(Deserialize)]
struct LegacySchema {
    #[serde(rename = "SCHEMA")]
    schema: String,
    #[serde(rename = "V2_COLUMNS")]
    columns: Vec<(String, String)>,
    #[serde(rename = "DENUE_COLS")]
    denue: Vec<String>,
}
static LEGACY: std::sync::LazyLock<LegacySchema> = std::sync::LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../config/db.json")).expect("Embedded schema")
});
const RUST_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS app_migrations(version INTEGER PRIMARY KEY,checksum TEXT NOT NULL,applied_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS profile_scores(prospect_id TEXT NOT NULL,profile_id TEXT NOT NULL,rule_version INTEGER NOT NULL DEFAULT 1,fit_score INTEGER NOT NULL,priority_score INTEGER NOT NULL,analysis_json TEXT NOT NULL,updated_at TEXT NOT NULL,PRIMARY KEY(prospect_id,profile_id));
CREATE TABLE IF NOT EXISTS command_receipts(operation_id TEXT PRIMARY KEY,outcome TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS interaction_offers(interaction_id INTEGER PRIMARY KEY,profile_id TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS ingestion_cursors(key TEXT PRIMARY KEY,next_index INTEGER NOT NULL,completed INTEGER NOT NULL,updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY,kind TEXT NOT NULL,status TEXT NOT NULL,state_json TEXT NOT NULL,updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS economic_series(source TEXT NOT NULL,series_id TEXT NOT NULL,period TEXT NOT NULL,value TEXT,unit TEXT,description TEXT,updated_at TEXT NOT NULL,PRIMARY KEY(source,series_id,period));
CREATE TABLE IF NOT EXISTS accounts(id TEXT PRIMARY KEY,denue_id TEXT UNIQUE NOT NULL,name TEXT NOT NULL,created_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS contacts(id INTEGER PRIMARY KEY,account_id TEXT NOT NULL REFERENCES accounts(id),name TEXT NOT NULL,role TEXT NOT NULL,email TEXT,phone TEXT,verified_at TEXT,notes TEXT);
CREATE TABLE IF NOT EXISTS opportunities(id INTEGER PRIMARY KEY,account_id TEXT NOT NULL REFERENCES accounts(id),profile_id TEXT NOT NULL,title TEXT NOT NULL,stage TEXT NOT NULL CHECK(stage IN ('diagnostico','propuesta','negociacion','ganado','perdido')),amount_cents INTEGER NOT NULL CHECK(amount_cents>=0),received_cents INTEGER NOT NULL DEFAULT 0 CHECK(received_cents>=0),currency TEXT NOT NULL CHECK(currency IN ('MXN','USD')),next_action TEXT,due_date TEXT,notes TEXT,created_at TEXT NOT NULL,updated_at TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS saved_filters(name TEXT PRIMARY KEY,query_json TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS ix_profile_priority ON profile_scores(profile_id,priority_score DESC);
CREATE INDEX IF NOT EXISTS ix_prio ON prospectos(estado,priority_score DESC);
CREATE INDEX IF NOT EXISTS ix_group ON prospectos(group_key);
CREATE INDEX IF NOT EXISTS ix_inter_prospect ON interactions(prospect_id,created_at);
CREATE INDEX IF NOT EXISTS ix_signals_prospect ON web_signals(prospect_id);
CREATE INDEX IF NOT EXISTS ix_dominio ON prospectos(dominio);
"#;
pub fn initialize(c: &mut Connection, paths: &AppPaths) -> Result<()> {
    c.busy_timeout(Wait::from_secs(5))?;
    c.execute_batch("PRAGMA foreign_keys=ON;")?;
    c.create_scalar_function(
        "lower_unicode",
        1,
        rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| Ok(ctx.get::<String>(0)?.to_lowercase()),
    )?;
    let version: i64 = c.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > 2 {
        return Err(invalid(
            "Esquema legacy más reciente que v2; migración requerida",
        ));
    }
    let exists: i64 = c.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE name='prospectos'",
        [],
        |r| r.get(0),
    )?;
    if exists != 0 && version < 2 {
        backup(c, &paths.backups, "pre_legacy_migration")?;
    }
    let tx = c.transaction()?;
    tx.execute_batch(&LEGACY.schema)?;
    let columns = {
        let mut st = tx.prepare("PRAGMA table_info(prospectos)")?;
        st.query_map([], |r| r.get::<_, String>(1))?
            .collect::<std::result::Result<HashSet<_>, _>>()?
    };
    for (name, ddl) in [
        vec![
            ("toques".into(), "INTEGER DEFAULT 0".into()),
            ("variante".into(), "TEXT".into()),
            ("contactado_en".into(), "TEXT".into()),
            ("proximo_seguimiento".into(), "TEXT".into()),
            ("score".into(), "INTEGER DEFAULT 0".into()),
        ],
        LEGACY.columns.clone(),
    ]
    .concat()
    {
        if !columns.contains(&name) {
            tx.execute_batch(&format!("ALTER TABLE prospectos ADD COLUMN {name} {ddl};"))?;
        }
    }
    if version < 2 {
        tx.execute_batch("UPDATE prospectos SET estado='revisado' WHERE estado='contactar'; UPDATE prospectos SET estado='cliente' WHERE estado='ganado'; INSERT INTO interactions(prospect_id,created_at,interaction_type,sequence_step,channel,template_variant,notes) SELECT id,COALESCE(contactado_en,date('now')),'contacto',0,'manual',variante,'migrado' FROM prospectos WHERE toques>=1 AND id NOT IN (SELECT prospect_id FROM interactions); UPDATE prospectos SET etapa_max=MAX(COALESCE(etapa_max,0),1) WHERE toques>=1; PRAGMA user_version=2;")?;
    }
    tx.execute_batch(RUST_SCHEMA)?;
    let checksum = {
        use sha1::{Digest, Sha1};
        format!("{:x}", Sha1::digest(RUST_SCHEMA.as_bytes()))
    };
    if let Some(old) = tx
        .query_row(
            "SELECT checksum FROM app_migrations WHERE version=1",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        if old != checksum {
            return Err(invalid("La migración Rust 1 cambió después de aplicarse"));
        }
    } else {
        tx.execute(
            "INSERT INTO app_migrations VALUES (1,?,?)",
            params![checksum, config::now()],
        )?;
    }
    crate::updates::initialize(&tx)?;
    tx.execute(
        "UPDATE jobs SET status='interrupted' WHERE status='running'",
        [],
    )?;
    tx.commit()?;
    Ok(())
}
pub fn verify(c: &Connection) -> Result<()> {
    let integrity: String = c.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(invalid(
            "La copia SQLite falló la verificación de integridad",
        ));
    }
    let count: i64 = c.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |r| {
        r.get(0)
    })?;
    if count > 0 {
        return Err(invalid(format!(
            "Se encontraron {count} relaciones inválidas"
        )));
    }
    Ok(())
}
fn backup(c: &Connection, folder: &Path, reason: &str) -> Result<PathBuf> {
    std::fs::create_dir_all(folder)?;
    let path = folder.join(format!("prospectos_{reason}_{}.db", crate::operation_id()));
    let mut dest = Connection::open(&path)?;
    rusqlite::backup::Backup::new(c, &mut dest)?.run_to_completion(
        100,
        Wait::from_millis(5),
        None,
    )?;
    verify(&dest)?;
    Ok(path)
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MigrationReport {
    pub legacy_version: i64,
    pub counts: BTreeMap<String, usize>,
    pub destination: PathBuf,
    pub integrity: String,
}
pub fn migrate_snapshot(source: &Path, paths: &AppPaths) -> Result<MigrationReport> {
    if paths.db.exists() {
        return Err(invalid(
            "La base Rust ya existe; no se sobrescribirá. Usa otro PROSPECTOS_HOME para ensayar la migración.",
        ));
    }
    let src = Connection::open_with_flags(source, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    verify(&src)?;
    let version: i64 = src.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version > 2 {
        return Err(invalid("Versión legacy no compatible"));
    }
    let counts = table_counts(&src)?;
    let temp = paths
        .home
        .join(format!("migration-{}.db", crate::operation_id()));
    let result = (|| -> Result<()> {
        let mut dest = Connection::open(&temp)?;
        rusqlite::backup::Backup::new(&src, &mut dest)?.run_to_completion(
            100,
            Wait::from_millis(5),
            None,
        )?;
        verify(&dest)?;
        if counts != table_counts(&dest)? {
            return Err(invalid("La copia no conserva los conteos"));
        }
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&temp);
        return Err(e);
    }
    std::fs::rename(&temp, &paths.db)?;
    let report = MigrationReport {
        legacy_version: version,
        counts,
        destination: paths.db.clone(),
        integrity: "ok".into(),
    };
    config::write_private(
        &paths.home.join("migration-report.json"),
        &serde_json::to_vec_pretty(&report)?,
    )?;
    Ok(report)
}
pub fn table_counts(c: &Connection) -> Result<BTreeMap<String, usize>> {
    let mut out = BTreeMap::new();
    for name in [
        "prospectos",
        "actividad",
        "interactions",
        "web_signals",
        "web_cache",
    ] {
        let exists: i64 = c.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?",
            [name],
            |r| r.get(0),
        )?;
        if exists != 0 {
            out.insert(
                name.into(),
                c.query_row(&format!("SELECT COUNT(*) FROM {name}"), [], |r| {
                    r.get::<_, i64>(0).map(|n| n as usize)
                })?,
            );
        }
    }
    Ok(out)
}
pub fn row_json(row: &rusqlite::Row<'_>) -> rusqlite::Result<serde_json::Value> {
    let mut values = serde_json::Map::new();
    for (i, key) in row.as_ref().column_names().iter().enumerate() {
        let v = match row.get_ref(i)? {
            ValueRef::Null => serde_json::Value::Null,
            ValueRef::Integer(n) => n.into(),
            ValueRef::Real(n) => serde_json::json!(n),
            ValueRef::Text(s) => String::from_utf8_lossy(s).into_owned().into(),
            ValueRef::Blob(_) => {
                return Err(rusqlite::Error::InvalidColumnType(
                    i,
                    (*key).into(),
                    rusqlite::types::Type::Blob,
                ));
            }
        };
        values.insert((*key).into(), v);
    }
    Ok(values.into())
}
pub fn decode(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prospect> {
    serde_json::from_value(row_json(row)?).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}
fn decode_profile(row: &rusqlite::Row<'_>) -> rusqlite::Result<Prospect> {
    let mut p = decode(row)?;
    if let Some(raw) = row.get::<_, Option<String>>("profile_analysis")? {
        let a: Analysis = serde_json::from_str(&raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        p.fit_score = a.fit_score;
        p.priority_score = a.priority_score;
        p.fit_breakdown = serde_json::to_string(&a.fit_breakdown).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        p.priority_breakdown = serde_json::to_string(&a.priority_breakdown).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })?;
        p.hypothesis_text = a.hypothesis_text;
        p.hypothesis_basis = a.hypothesis_basis;
        p.hypothesis_confidence = a.hypothesis_confidence;
        p.canal = a.action.action;
        p.giro_display = a.giro_display;
    }
    Ok(p)
}
fn predicates(q: &Query) -> (String, Vec<Value>) {
    let mut clauses = vec!["1=1".to_owned()];
    let mut args = Vec::new();
    let today = config::today().to_string();
    match q.view {
        View::Today => {
            clauses.push("p.estado IN ('nuevo','revisado') AND COALESCE(s.priority_score,p.priority_score)>0 AND COALESCE(p.snooze_until,'')<=?".into());
            args.push(today.into());
        }
        View::Due => {
            clauses.push("p.estado='contactado' AND p.proximo_seguimiento IS NOT NULL AND p.proximo_seguimiento<=?".into());
            args.push(today.into());
        }
        View::Conversations => clauses
            .push("p.estado IN ('respondio','respuesta_positiva','reunion','propuesta')".into()),
        View::Clients => clauses.push("p.estado='cliente'".into()),
        View::All => {}
    }
    if !q.search.is_empty() {
        clauses.push("(instr(lower_unicode(COALESCE(p.nombre,'')),lower_unicode(?))>0 OR instr(lower_unicode(COALESCE(p.razon_social,'')),lower_unicode(?))>0 OR instr(lower_unicode(COALESCE(p.municipio,'')),lower_unicode(?))>0)".into());
        for _ in 0..3 {
            args.push(q.search.trim().to_owned().into());
        }
    }
    for (col, value) in [
        ("p.estado", &q.state),
        ("p.giro", &q.giro),
        ("p.municipio", &q.municipio),
        ("p.entidad", &q.entidad),
    ] {
        if !value.is_empty() {
            clauses.push(format!("{col}=?"));
            args.push(value.clone().into());
        }
    }
    clauses.push("COALESCE(s.fit_score,p.fit_score)>=?".into());
    args.push(q.fit_min.into());
    if q.contactable {
        clauses.push("(email_valid(COALESCE(p.correo,'')) OR email_valid(COALESCE(p.correo_web,'')) OR phone_valid(COALESCE(p.telefono,'')) OR phone_valid(COALESCE(p.whatsapp,''))) AND COALESCE(json_extract(s.analysis_json,'$.action.action'),p.canal,'')<>'skip'".into());
    }
    (clauses.join(" AND "), args)
}
fn contact_functions(c: &Connection) -> Result<()> {
    let flags = rusqlite::functions::FunctionFlags::SQLITE_DETERMINISTIC;
    c.create_scalar_function("email_valid", 1, flags, |ctx| {
        Ok(crate::contacts::email_valid(&ctx.get::<String>(0)?))
    })?;
    c.create_scalar_function("phone_valid", 1, flags, |ctx| {
        Ok(!crate::contacts::phone(&ctx.get::<String>(0)?).is_empty())
    })?;
    Ok(())
}
pub fn list(c: &Connection, q: &Query) -> Result<Page> {
    contact_functions(c)?;
    let (where_sql, args) = predicates(q);
    let join =
        "FROM prospectos p LEFT JOIN profile_scores s ON s.prospect_id=p.id AND s.profile_id=?";
    let mut params = vec![Value::Text(q.profile.id().into())];
    params.extend(args);
    let total: usize = c.query_row(
        &format!("SELECT COUNT(*) {join} WHERE {where_sql}"),
        rusqlite::params_from_iter(&params),
        |r| r.get::<_, i64>(0).map(|n| n as usize),
    )?;
    let size = if q.size == 0 { 60 } else { q.size };
    let order = if q.view == View::Due {
        "p.proximo_seguimiento,p.id"
    } else {
        "COALESCE(s.priority_score,p.priority_score) DESC,p.id"
    };
    let sql = format!(
        "SELECT p.*,COALESCE(s.fit_score,p.fit_score) AS fit_score,COALESCE(s.priority_score,p.priority_score) AS priority_score,s.analysis_json AS profile_analysis {join} WHERE {where_sql} ORDER BY {order} LIMIT ? OFFSET ?"
    );
    params.push((size as i64).into());
    params.push(((q.page * size) as i64).into());
    let mut statement = c.prepare(&sql)?;
    let items = statement
        .query_map(rusqlite::params_from_iter(params), decode_profile)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    Ok(Page {
        items,
        total,
        page: q.page,
        size,
    })
}
pub fn signals(c: &Connection, id: &str) -> Result<Vec<Signal>> {
    let mut st=c.prepare("SELECT signal_type,signal_text,signal_source_url FROM web_signals WHERE prospect_id=? ORDER BY id")?;
    Ok(st
        .query_map([id], |r| {
            Ok(Signal {
                signal_type: r.get(0)?,
                signal_text: r.get(1)?,
                signal_source_url: r.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
pub fn all(c: &Connection) -> Result<Vec<Prospect>> {
    let mut st = c.prepare("SELECT * FROM prospectos ORDER BY rowid")?;
    Ok(st
        .query_map([], decode)?
        .collect::<std::result::Result<Vec<_>, _>>()?)
}
fn summary(c: &mut Connection) -> Result<Summary> {
    let count = |predicate: &str| -> Result<usize> {
        Ok(c.query_row(
            &format!("SELECT COUNT(*) FROM prospectos WHERE {predicate}"),
            [],
            |r| r.get::<_, i64>(0).map(|n| n as usize),
        )?)
    };
    contact_functions(c)?;
    let today = config::today().to_string();
    let mut out = Summary {
        total: count("1=1")?,
        today: count(&format!(
            "estado IN ('nuevo','revisado') AND priority_score>0 AND COALESCE(snooze_until,'')<='{today}'"
        ))?,
        due: count(&format!(
            "estado='contactado' AND proximo_seguimiento<='{today}'"
        ))?,
        conversations: count("estado IN ('respondio','respuesta_positiva','reunion','propuesta')")?,
        clients: count("estado='cliente'")?,
        with_contact: count(
            "email_valid(COALESCE(correo,'')) OR email_valid(COALESCE(correo_web,'')) OR phone_valid(COALESCE(telefono,'')) OR phone_valid(COALESCE(whatsapp,''))",
        )?,
        ..Default::default()
    };
    let mut st = c.prepare("SELECT tipo,COUNT(*) FROM actividad WHERE fecha=? GROUP BY tipo")?;
    for r in st.query_map([today], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? as usize))
    })? {
        let (t, n) = r?;
        match t.as_str() {
            "investigado" => out.researched_today = n,
            "contacto" => out.contacted_today = n,
            "seguimiento" => out.followed_today = n,
            "publicacion" => out.publications_today = n,
            _ => {}
        }
    }
    Ok(out)
}
pub fn upsert(c: &Connection, records: &[Prospect]) -> Result<(usize, usize)> {
    let columns = &LEGACY.denue;
    let updates = columns
        .iter()
        .map(|s| format!("{s}=excluded.{s}"))
        .collect::<Vec<_>>()
        .join(",");
    let changed = columns
        .iter()
        .map(|name| format!("prospectos.{name} IS NOT excluded.{name}"))
        .collect::<Vec<_>>()
        .join(" OR ");
    let insert_cols = [vec!["id".to_owned()], columns.clone()].concat();
    let sql = format!(
        "INSERT INTO prospectos({}) VALUES ({}) ON CONFLICT(id) DO UPDATE SET {updates} WHERE {changed}",
        insert_cols.join(","),
        vec!["?"; insert_cols.len()].join(",")
    );
    let mut st = c.prepare(&sql)?;
    let mut new = 0;
    let mut updated = 0;
    for p in records {
        if p.id.trim().is_empty() || p.nombre.trim().is_empty() {
            return Err(invalid("Registro sin ID o nombre"));
        }
        let exists: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM prospectos WHERE id=?)",
            [&p.id],
            |r| r.get(0),
        )?;
        let raw = serde_json::to_value(p)?;
        let values = insert_cols
            .iter()
            .map(|k| match &raw[k] {
                serde_json::Value::Null => Value::Null,
                serde_json::Value::String(s) => {
                    if s.is_empty() && k == "dominio" {
                        Value::Null
                    } else {
                        Value::Text(s.clone())
                    }
                }
                serde_json::Value::Number(n) => {
                    if let Some(n) = n.as_i64() {
                        Value::Integer(n)
                    } else {
                        Value::Real(n.as_f64().unwrap_or_default())
                    }
                }
                _ => Value::Null,
            })
            .collect::<Vec<_>>();
        let wrote = st.execute(rusqlite::params_from_iter(values))?;
        new += usize::from(!exists);
        updated += if exists { wrote } else { 0 };
    }
    Ok((new, updated))
}
fn save_analysis(
    c: &Connection,
    p: &Prospect,
    a: &Analysis,
    profile: Profile,
    message: &crate::Message,
) -> Result<()> {
    let now = config::now();
    let json = serde_json::to_string(a)?;
    c.execute("INSERT INTO profile_scores(prospect_id,profile_id,fit_score,priority_score,analysis_json,updated_at) VALUES (?,?,?,?,?,?) ON CONFLICT(prospect_id,profile_id) DO UPDATE SET fit_score=excluded.fit_score,priority_score=excluded.priority_score,analysis_json=excluded.analysis_json,updated_at=excluded.updated_at",params![p.id,profile.id(),a.fit_score,a.priority_score,json,now])?;
    if profile == Profile::Automatizacion {
        c.execute("UPDATE prospectos SET giro=?,giro_display=?,fit_score=?,fit_breakdown=?,priority_score=?,priority_breakdown=?,hypothesis_text=?,hypothesis_basis=?,hypothesis_confidence=?,score=?,canal=?,mensaje=?,variante=?,group_key=?,possible_chain=?,possible_chain_count=?,group_reason=?,analizado_en=? WHERE id=?",params![a.giro,a.giro_display,a.fit_score,serde_json::to_string(&a.fit_breakdown)?,a.priority_score,serde_json::to_string(&a.priority_breakdown)?,a.hypothesis_text,a.hypothesis_basis,a.hypothesis_confidence,a.priority_score,a.action.action,message.cuerpo,message.variante,a.group.group_key,a.group.possible_chain,a.group.possible_chain_count,a.group.group_reason,now,p.id])?;
    }
    Ok(())
}
fn analyze_all(
    c: &mut Connection,
    paths: &AppPaths,
    settings: &Settings,
    control: &crate::jobs::JobControl,
    today: NaiveDate,
) -> Result<usize> {
    let rows = all(c)?;
    let groups = groups::group(&rows);
    let contacted = rows
        .iter()
        .filter(|p| p.etapa_max >= 1 && groups[&p.id].possible_chain != 0)
        .map(|p| groups[&p.id].group_key.clone())
        .collect::<HashSet<_>>();
    let templates = messages::Templates::load(&paths.home)?;
    let tx = c.transaction()?;
    for (i, p) in rows.iter().enumerate() {
        control.check()?;
        let signals = signals(&tx, &p.id)?;
        for profile in [Profile::Automatizacion, Profile::WebMarketing] {
            let analysis = rules::analyze(
                p,
                &signals,
                groups[&p.id].clone(),
                contacted.contains(&groups[&p.id].group_key),
                today,
                profile,
            );
            let mut rendered = p.clone();
            rendered.giro = analysis.giro.clone();
            rendered.giro_display = analysis.giro_display.clone();
            let message = messages::first(
                &rendered, "whatsapp", &signals, settings, &templates, profile,
            )?;
            save_analysis(&tx, p, &analysis, profile, &message)?;
        }
        if i % 100 == 0 {
            control.update(|s| {
                s.processed = i + 1;
                s.message = format!("Analizando {} de {}", i + 1, rows.len());
            });
        }
    }
    tx.commit()?;
    control.update(|s| s.processed = rows.len());
    Ok(rows.len())
}
fn mutate(
    c: &mut Connection,
    m: &Mutation,
    settings: &Settings,
    today: NaiveDate,
) -> Result<String> {
    let tx = c.transaction()?;
    if let Some(outcome) = tx
        .query_row(
            "SELECT outcome FROM command_receipts WHERE operation_id=?",
            [&m.operation_id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        return Ok(outcome);
    }
    let date = today.to_string();
    let mut outcome = "guardado".to_owned();
    let now = config::now();
    let pid = &m.prospect_id;
    if matches!(m.command, Command::Publication { .. }) {
        tx.execute(
            "INSERT INTO actividad(fecha,tipo) VALUES (?,'publicacion')",
            [&date],
        )?;
    } else {
        let p = tx
            .query_row("SELECT * FROM prospectos WHERE id=?", [pid], decode)
            .optional()?
            .ok_or_else(|| invalid("Establecimiento inexistente"))?;
        let researched = |tx: &Connection| -> Result<()> {
            if p.estado == "nuevo" {
                tx.execute(
                    "INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,'investigado',?)",
                    params![date, pid],
                )?;
            }
            Ok(())
        };
        let step = |tx: &Connection, n: i64| -> Result<()> {
            tx.execute(
                "UPDATE prospectos SET etapa_max=MAX(COALESCE(etapa_max,0),?) WHERE id=?",
                params![n, pid],
            )?;
            Ok(())
        };
        let interaction = |tx: &Connection,
                           kind: &str,
                           channel: Option<&str>,
                           sequence: Option<i64>,
                           variant: Option<&str>,
                           text: Option<&str>,
                           response: Option<&str>|
         -> Result<()> {
            tx.execute("INSERT INTO interactions(prospect_id,created_at,channel,interaction_type,sequence_step,template_variant,message_rendered,response_type,notes) VALUES (?,?,?,?,?,?,?,?,?)",params![pid,now,channel,kind,sequence,variant,text,response,m.notes])?;
            let id = tx.last_insert_rowid();
            tx.execute(
                "INSERT INTO interaction_offers VALUES (?,?)",
                params![id, m.profile.id()],
            )?;
            Ok(())
        };
        let validate_contact =
            |channel: &str, text: &str, subject: &str, is_followup: bool| -> Result<()> {
                if !["email", "whatsapp", "telefono", "manual"].contains(&channel) {
                    return Err(invalid("Canal inválido"));
                }
                if channel == "email" && p.email().is_empty() {
                    return Err(invalid("No hay correo válido"));
                }
                if ["whatsapp", "telefono"].contains(&channel) && p.phone().is_empty() {
                    return Err(invalid("No hay teléfono válido"));
                }
                if text.chars().count() > 20_000 {
                    return Err(invalid("Mensaje demasiado largo"));
                }
                if !text.is_empty() && ["email", "whatsapp"].contains(&channel) {
                    let evidence = rules::observation(&signals(&tx, pid)?).is_some();
                    let problems = messages::validate(
                        text,
                        if is_followup { "seguimiento" } else { channel },
                        subject,
                        evidence,
                        settings,
                    );
                    if let Some(p) = problems.iter().find(|p| !p.starts_with("Aviso:")) {
                        return Err(invalid(p.clone()));
                    }
                }
                Ok(())
            };
        match &m.command {
            Command::Contact {
                channel,
                variant,
                message,
                subject,
            } => {
                if !["nuevo", "revisado"].contains(&p.estado.as_str()) {
                    return Err(invalid(
                        "Este prospecto ya tiene contacto o está cerrado; usa seguimiento o revisión manual",
                    ));
                }
                validate_contact(channel, message, subject, false)?;
                researched(&tx)?;
                tx.execute(
                    "INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,'contacto',?)",
                    params![date, pid],
                )?;
                interaction(
                    &tx,
                    "contacto",
                    Some(channel),
                    Some(0),
                    Some(variant),
                    Some(message),
                    None,
                )?;
                tx.execute("UPDATE prospectos SET estado='contactado',toques=1,contactado_en=?,proximo_seguimiento=?,snooze_until=NULL,notas=COALESCE(?,notas) WHERE id=?",params![date,(today+Duration::days(3)).to_string(),m.notes,pid])?;
                step(&tx, 1)?;
                outcome = "contactado".into();
            }
            Command::Followup {
                channel,
                variant,
                message,
            } => {
                if p.estado != "contactado" {
                    return Err(invalid("No hay una secuencia activa"));
                }
                validate_contact(channel, message, "", true)?;
                let touches = if p.toques == 0 { 2 } else { p.toques + 1 };
                let sequence = touches - 1;
                tx.execute(
                    "INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,'seguimiento',?)",
                    params![date, pid],
                )?;
                interaction(
                    &tx,
                    "seguimiento",
                    Some(channel),
                    Some(sequence),
                    Some(variant),
                    Some(message),
                    None,
                )?;
                if sequence >= 3 {
                    tx.execute("UPDATE prospectos SET toques=?,estado='sin_respuesta',proximo_seguimiento=NULL WHERE id=?",params![touches,pid])?;
                    outcome = "sin_respuesta".into();
                } else {
                    tx.execute(
                        "UPDATE prospectos SET toques=?,proximo_seguimiento=? WHERE id=?",
                        params![
                            touches,
                            (today + Duration::days(if sequence == 1 { 4 } else { 7 })).to_string(),
                            pid
                        ],
                    )?;
                    outcome = "contactado".into();
                }
            }
            Command::Response { response } => {
                let (state, n) = match response.as_str() {
                    "interesado" => ("respuesta_positiva", 3),
                    "redirigido" | "pregunta" | "otro" => ("respondio", 2),
                    "no_interesado" | "ya_tienen_solucion" | "no_es_prioridad" => {
                        ("no_interesado", 2)
                    }
                    "numero_incorrecto" | "correo_rebotado" => ("revisado", 0),
                    _ => return Err(invalid("Tipo de respuesta inválido")),
                };
                interaction(&tx, "respuesta", None, None, None, None, Some(response))?;
                tx.execute("UPDATE prospectos SET estado=?,proximo_seguimiento=NULL,notas=COALESCE(?,notas) WHERE id=?",params![state,m.notes,pid])?;
                if n > 0 {
                    step(&tx, n)?;
                }
                outcome = state.into();
            }
            Command::State { state } => {
                if !config::STATES.contains(&state.as_str()) {
                    return Err(invalid("Estado inválido"));
                }
                if state == "contactado" && p.estado != "contactado" {
                    researched(&tx)?;
                    tx.execute(
                        "INSERT INTO actividad(fecha,tipo,prospecto_id) VALUES (?,'contacto',?)",
                        params![date, pid],
                    )?;
                    interaction(&tx, "contacto", Some("manual"), Some(0), None, None, None)?;
                    tx.execute("UPDATE prospectos SET estado='contactado',toques=1,contactado_en=?,proximo_seguimiento=?,snooze_until=NULL,notas=COALESCE(?,notas) WHERE id=?",params![date,(today+Duration::days(3)).to_string(),m.notes,pid])?;
                    step(&tx, 1)?;
                } else {
                    if state != "nuevo" {
                        researched(&tx)?;
                    }
                    tx.execute("UPDATE prospectos SET estado=?,proximo_seguimiento=NULL,snooze_until=NULL,notas=COALESCE(?,notas) WHERE id=?",params![state,m.notes,pid])?;
                    let n = match state.as_str() {
                        "respondio" => 2,
                        "respuesta_positiva" => 3,
                        "reunion" => 4,
                        "propuesta" => 5,
                        "cliente" => 6,
                        _ => 0,
                    };
                    if n > 0 {
                        step(&tx, n)?;
                    }
                }
                outcome = state.clone();
            }
            Command::Note { .. } => {
                tx.execute(
                    "UPDATE prospectos SET notas=? WHERE id=?",
                    params![m.notes.as_deref().unwrap_or(""), pid],
                )?;
                interaction(&tx, "nota", None, None, None, None, None)?;
            }
            Command::Snooze { days } => {
                if !(1..=365).contains(days) {
                    return Err(invalid("Posponer requiere entre 1 y 365 días"));
                }
                tx.execute(
                    "UPDATE prospectos SET snooze_until=? WHERE id=?",
                    params![(today + Duration::days(*days)).to_string(), pid],
                )?;
                outcome = "pospuesto".into();
            }
            Command::Research { .. } => {
                researched(&tx)?;
                tx.execute("UPDATE prospectos SET estado=CASE WHEN estado='nuevo' THEN 'revisado' ELSE estado END WHERE id=?",[pid])?;
                outcome = "revisado".into();
            }
            Command::Publication { .. } => {}
        }
    }
    if !matches!(m.command, Command::Publication { .. }) {
        refresh_scores(&tx, Some(pid), today)?;
    }
    tx.execute(
        "INSERT INTO command_receipts VALUES (?,?,?)",
        params![m.operation_id, outcome, now],
    )?;
    tx.commit()?;
    Ok(outcome)
}
fn refresh_scores(c: &Connection, target: Option<&str>, today: NaiveDate) -> Result<usize> {
    let rows = if let Some(id) = target {
        let mut s=c.prepare("SELECT * FROM prospectos WHERE id=? OR (group_key=(SELECT group_key FROM prospectos WHERE id=?) AND group_key IS NOT NULL AND group_key<>'')")?;
        s.query_map(params![id, id], decode)?
            .collect::<std::result::Result<Vec<_>, _>>()?
    } else {
        all(c)?
    };
    let contacted = {
        let mut s = c.prepare(
            "SELECT DISTINCT group_key FROM prospectos WHERE etapa_max>=1 AND possible_chain=1",
        )?;
        s.query_map([], |r| r.get::<_, Option<String>>(0))?
            .collect::<std::result::Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<HashSet<_>>()
    };
    let mut updated = 0;
    for p in rows {
        let types = signals(c, &p.id)?
            .into_iter()
            .map(|s| s.signal_type)
            .collect::<Vec<_>>();
        for profile in [Profile::Automatizacion, Profile::WebMarketing] {
            let cached = c
                .query_row(
                    "SELECT analysis_json FROM profile_scores WHERE prospect_id=? AND profile_id=?",
                    params![p.id, profile.id()],
                    |r| r.get::<_, String>(0),
                )
                .optional()?;
            if let Some(raw) = cached {
                let mut a: Analysis = serde_json::from_str(&raw)?;
                let mut row = p.clone();
                row.giro = a.giro.clone();
                row.giro_display = a.giro_display.clone();
                let (score, items, _) = rules::priority(
                    &row,
                    a.fit_score,
                    &types,
                    contacted.contains(&p.group_key),
                    today,
                );
                let action = rules::next_action(&row, a.fit_score, !types.is_empty(), today);
                if score != a.priority_score || items != a.priority_breakdown || action != a.action
                {
                    a.priority_score = score;
                    a.priority_breakdown = items;
                    a.action = action;
                    c.execute("UPDATE profile_scores SET priority_score=?,analysis_json=?,updated_at=? WHERE prospect_id=? AND profile_id=?",params![score,serde_json::to_string(&a)?,config::now(),p.id,profile.id()])?;
                    if profile == Profile::Automatizacion {
                        c.execute("UPDATE prospectos SET priority_score=?,priority_breakdown=?,score=?,canal=? WHERE id=?",params![score,serde_json::to_string(&a.priority_breakdown)?,score,a.action.action,p.id])?;
                    }
                    updated += 1;
                }
            }
        }
    }
    Ok(updated)
}
fn export_rows(c: &Connection, q: &Query) -> Result<Vec<BTreeMap<String, serde_json::Value>>> {
    let mut out = Vec::new();
    let mut q = q.clone();
    q.page = 0;
    q.size = 500;
    loop {
        let page = list(c, &q)?;
        for p in page.items {
            let obj = serde_json::to_value(p)?;
            out.push(
                obj.as_object()
                    .unwrap()
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
            );
        }
        if out.len() >= page.total {
            break;
        }
        q.page += 1;
    }
    Ok(out)
}
pub fn safe_cell(value: &str) -> String {
    if value
        .trim_start_matches([' ', '\t', '\r', '\n'])
        .starts_with(['=', '+', '-', '@'])
    {
        format!("'{value}")
    } else {
        value.into()
    }
}
fn write_export(
    path: &Path,
    rows: &[BTreeMap<String, serde_json::Value>],
    format: &str,
) -> Result<()> {
    let keys = if let Some(row) = rows.first() {
        row.keys().cloned().collect::<Vec<_>>()
    } else {
        vec!["id".into(), "nombre".into()]
    };
    if format == "xlsx" {
        let mut book = rust_xlsxwriter::Workbook::new();
        let sheet = book.add_worksheet();
        sheet.set_name("Prospectos")?;
        for (i, k) in keys.iter().enumerate() {
            sheet.write_string(0, i as u16, k)?;
        }
        for (i, row) in rows.iter().enumerate() {
            for (j, k) in keys.iter().enumerate() {
                let value = &row[k];
                match value {
                    serde_json::Value::Number(n) => {
                        sheet.write_number(
                            (i + 1) as u32,
                            j as u16,
                            n.as_f64().unwrap_or_default(),
                        )?;
                    }
                    serde_json::Value::Null => {}
                    _ => {
                        let s = value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string());
                        sheet.write_string((i + 1) as u32, j as u16, s)?;
                    }
                }
            }
        }
        sheet.set_freeze_panes(1, 0)?;
        sheet.autofilter(0, 0, rows.len() as u32, (keys.len() - 1) as u16)?;
        book.save(path)?;
    } else if format == "csv" {
        let mut file = std::fs::File::create(path)?;
        use std::io::Write;
        file.write_all(&[0xef, 0xbb, 0xbf])?;
        let mut writer = csv::Writer::from_writer(file);
        writer.write_record(&keys)?;
        for row in rows {
            writer.write_record(keys.iter().map(|k| {
                let v = &row[k];
                match v {
                    serde_json::Value::Null => String::new(),
                    serde_json::Value::String(s) => safe_cell(s),
                    v => v.to_string(),
                }
            }))?;
        }
        writer.flush()?;
    } else {
        return Err(invalid("Formato de exportación inválido"));
    }
    Ok(())
}

#[cfg(test)]
mod read_cache_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[tokio::test]
    async fn reads_are_coalesced_and_invalidated_even_after_failed_writes() {
        let temp = tempfile::TempDir::new().unwrap();
        let db = Db::open(AppPaths::at(temp.path()).unwrap()).unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut tasks = Vec::new();
        for _ in 0..8 {
            let db = db.clone();
            let calls = calls.clone();
            tasks.push(tokio::spawn(async move {
                db.memoized("probe".into(), move |_| {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Ok(7u64)
                })
                .await
                .unwrap()
            }));
        }
        for task in tasks {
            assert_eq!(task.await.unwrap(), 7);
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        db.call(|c| {
            c.execute("CREATE TABLE cache_probe(value INTEGER)", [])?;
            c.execute("INSERT INTO cache_probe VALUES (1)", [])?;
            Ok(())
        })
        .await
        .unwrap();
        assert_eq!(db.memoized("probe".into(), |_| Ok(8u64)).await.unwrap(), 8);
        let failed: Result<()> = db
            .call(|c| {
                c.execute("INSERT INTO cache_probe VALUES (2)", [])?;
                Err(invalid("Failure after a committed write"))
            })
            .await;
        assert!(failed.is_err());
        assert_eq!(db.memoized("probe".into(), |_| Ok(9u64)).await.unwrap(), 9);
        let external = Connection::open(&db.paths.db).unwrap();
        external
            .execute("INSERT INTO cache_probe VALUES (3)", [])
            .unwrap();
        assert_eq!(
            db.memoized("probe".into(), |_| Ok(10u64)).await.unwrap(),
            10
        );
    }
    #[test]
    fn read_cache_is_bounded_and_expires_at_the_day_boundary() {
        let mut cache = ReadCache::default();
        let c = Connection::open_in_memory().unwrap();
        cache.observe(&c).unwrap();
        for i in 0..40 {
            cache.insert(format!("{i}"), i);
        }
        assert_eq!(cache.entries.len(), 32);
        assert_eq!(cache.get::<i32>("0"), None);
        assert_eq!(cache.get::<i32>("39"), Some(39));
        cache.date = "yesterday".into();
        cache.observe(&c).unwrap();
        assert!(cache.entries.is_empty());
    }
}
