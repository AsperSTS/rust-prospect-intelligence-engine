//! Local work views and an audit of committed DENUE pages. No source requests.
use crate::{Db, Profile, Query, Result, config, invalid};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

const SCHEMA: &str = r#"
CREATE TABLE saved_views(profile_id TEXT NOT NULL,name TEXT NOT NULL,query_json TEXT NOT NULL,layout_json TEXT NOT NULL,updated_at TEXT NOT NULL,PRIMARY KEY(profile_id,name));
CREATE TABLE denue_page_audits(id INTEGER PRIMARY KEY,job_id TEXT NOT NULL,profile_id TEXT NOT NULL,scope TEXT NOT NULL,start_index INTEGER NOT NULL,queried_at TEXT NOT NULL,inserted INTEGER NOT NULL,updated INTEGER NOT NULL,unchanged INTEGER NOT NULL,invalid INTEGER NOT NULL,changes_json TEXT NOT NULL,UNIQUE(job_id,scope,start_index));
CREATE INDEX ix_denue_audits_profile ON denue_page_audits(profile_id,id DESC);
"#;

pub(crate) fn initialize(c: &Connection) -> Result<()> {
    use sha1::{Digest, Sha1};
    let checksum = format!("{:x}", Sha1::digest(SCHEMA.as_bytes()));
    match c
        .query_row(
            "SELECT checksum FROM app_migrations WHERE version=3",
            [],
            |r| r.get::<_, String>(0),
        )
        .optional()?
    {
        Some(old) if old != checksum => {
            return Err(invalid("La migración Rust 3 cambió después de aplicarse"));
        }
        Some(_) => (),
        None => {
            c.execute_batch(SCHEMA)?;
            c.execute(
                "INSERT INTO app_migrations VALUES (3,?,?)",
                params![checksum, config::now()],
            )?;
        }
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TableColumn {
    #[default]
    Name,
    Entity,
    Municipality,
    Sector,
    Fit,
    Priority,
    State,
    Contact,
}
impl TableColumn {
    pub const ALL: [Self; 8] = [
        Self::Name,
        Self::Entity,
        Self::Municipality,
        Self::Sector,
        Self::Fit,
        Self::Priority,
        Self::State,
        Self::Contact,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Name => "Establecimiento",
            Self::Entity => "Entidad",
            Self::Municipality => "Municipio",
            Self::Sector => "Giro",
            Self::Fit => "Fit",
            Self::Priority => "Prioridad",
            Self::State => "Estado",
            Self::Contact => "Contacto",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    Compact,
    #[default]
    Comfortable,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TableLayout {
    pub columns: Vec<TableColumn>,
    pub density: Density,
}
impl Default for TableLayout {
    fn default() -> Self {
        Self {
            columns: TableColumn::ALL
                .into_iter()
                .filter(|c| *c != TableColumn::Entity)
                .collect(),
            density: Density::Comfortable,
        }
    }
}
impl TableLayout {
    pub fn validate(&self) -> Result<()> {
        if self.columns.first() != Some(&TableColumn::Name)
            || self.columns.len() > TableColumn::ALL.len()
            || self.columns.iter().collect::<HashSet<_>>().len() != self.columns.len()
        {
            return Err(invalid(
                "La vista necesita Establecimiento al inicio y columnas sin duplicar",
            ));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SavedView {
    pub name: String,
    pub query: Query,
    pub layout: TableLayout,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct FieldChanges {
    pub modified: BTreeMap<String, usize>,
    pub emptied: BTreeMap<String, usize>,
    pub records_with_losses: usize,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportSummary {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub changes: FieldChanges,
}
pub struct DenuePageAudit {
    pub job_id: String,
    pub profile: Profile,
    pub scope: String,
    pub start: usize,
    pub invalid: usize,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BatchSummary {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub queried_at: String,
    pub pages: usize,
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
    pub invalid: usize,
    pub changes: FieldChanges,
}
impl Db {
    pub async fn save_view(
        &self,
        name: String,
        mut query: Query,
        layout: TableLayout,
    ) -> Result<()> {
        let name = name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 100 || name.chars().any(char::is_control) {
            return Err(invalid("Nombre de vista inválido"));
        }
        query.validate()?;
        layout.validate()?;
        query.page = 0;
        self.call(move |c| {
            c.execute("INSERT INTO saved_views VALUES (?,?,?,?,?) ON CONFLICT(profile_id,name) DO UPDATE SET query_json=excluded.query_json,layout_json=excluded.layout_json,updated_at=excluded.updated_at",params![query.profile.id(),name,serde_json::to_string(&query)?,serde_json::to_string(&layout)?,config::now()])?;
            Ok(())
        }).await
    }
    pub async fn saved_views(&self, profile: Profile) -> Result<Vec<SavedView>> {
        self.call(move |c| {
            let mut st=c.prepare("SELECT name,query_json,layout_json FROM saved_views WHERE profile_id=? ORDER BY name")?;
            let raw=st.query_map([profile.id()],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?.collect::<std::result::Result<Vec<_>,_>>()?;
            raw.into_iter().map(|(name,q,l)| {
                let query: Query=serde_json::from_str(&q)?; let layout: TableLayout=serde_json::from_str(&l)?;
                query.validate()?; layout.validate()?;
                if query.profile!=profile { return Err(invalid("La vista pertenece a otra oferta")); }
                Ok(SavedView{name,query,layout})
            }).collect()
        }).await
    }
    pub async fn delete_view(&self, profile: Profile, name: String) -> Result<()> {
        self.call(move |c| {
            c.execute(
                "DELETE FROM saved_views WHERE profile_id=? AND name=?",
                params![profile.id(), name],
            )?;
            Ok(())
        })
        .await
    }
    /// Page changes, audit and cursor commit together; failed pages leave none of them.
    pub async fn import_audited_page(
        &self,
        records: Vec<crate::Prospect>,
        audit: DenuePageAudit,
        done: bool,
    ) -> Result<ImportSummary> {
        if audit.job_id.is_empty()
            || audit.job_id.len() > 100
            || audit.scope.is_empty()
            || audit.scope.len() > 200
            || audit.start == 0
            || audit.start > 100_000_000
            || records.len() + audit.invalid > 200
        {
            return Err(invalid("Página DENUE fuera de rango"));
        }
        self.call(move |c| {
            let tx=c.transaction()?;
            let mut changes=FieldChanges::default();
            let (inserted,updated)=crate::db::upsert_with_changes(&tx,&records,Some(&mut changes))?;
            let unchanged=records.len()-inserted-updated;
            let now=config::now();
            tx.execute("INSERT INTO denue_page_audits(job_id,profile_id,scope,start_index,queried_at,inserted,updated,unchanged,invalid,changes_json) VALUES (?,?,?,?,?,?,?,?,?,?)",params![audit.job_id,audit.profile.id(),audit.scope,audit.start as i64,now,inserted as i64,updated as i64,unchanged as i64,audit.invalid as i64,serde_json::to_string(&changes)?])?;
            tx.execute("INSERT INTO ingestion_cursors(key,next_index,completed,updated_at) VALUES (?,?,?,?) ON CONFLICT(key) DO UPDATE SET next_index=excluded.next_index,completed=excluded.completed,updated_at=excluded.updated_at",params![audit.scope,(audit.start+200) as i64,done,now])?;
            tx.commit()?;
            Ok(ImportSummary{inserted,updated,unchanged,changes})
        }).await
    }
    /// Most recent twenty batches for this offer; retained across app restarts.
    pub async fn denue_batch_summaries(&self, profile: Profile) -> Result<Vec<BatchSummary>> {
        self.call(move |c| {
            let mut st=c.prepare("SELECT a.job_id,COALESCE(j.kind,'DENUE'),COALESCE(j.status,'interrupted'),MAX(a.queried_at),COUNT(*),SUM(a.inserted),SUM(a.updated),SUM(a.unchanged),SUM(a.invalid) FROM denue_page_audits a LEFT JOIN jobs j ON j.id=a.job_id WHERE a.profile_id=? GROUP BY a.job_id ORDER BY MAX(a.id) DESC LIMIT 20")?;
            let mut batches=st.query_map([profile.id()],|r|Ok(BatchSummary{id:r.get(0)?,kind:r.get(1)?,status:r.get(2)?,queried_at:r.get(3)?,pages:r.get::<_,i64>(4)? as usize,inserted:r.get::<_,i64>(5)? as usize,updated:r.get::<_,i64>(6)? as usize,unchanged:r.get::<_,i64>(7)? as usize,invalid:r.get::<_,i64>(8)? as usize,..Default::default()}))?.collect::<std::result::Result<Vec<_>,_>>()?;
            for batch in &mut batches {
                let mut changes=c.prepare("SELECT changes_json FROM denue_page_audits WHERE job_id=? AND profile_id=? ORDER BY id")?;
                for raw in changes.query_map(params![batch.id,profile.id()],|r|r.get::<_,String>(0))? {
                    let data: FieldChanges=serde_json::from_str(&raw?)?;
                    batch.changes.records_with_losses+=data.records_with_losses;
                    for (key,n) in data.modified { *batch.changes.modified.entry(key).or_default()+=n; }
                    for (key,n) in data.emptied { *batch.changes.emptied.entry(key).or_default()+=n; }
                }
            }
            Ok(batches)
        }).await
    }
}

pub fn field_label(field: &str) -> &str {
    match field {
        "nombre" => "Nombre",
        "razon_social" => "Razón social",
        "correo" => "Correo DENUE",
        "telefono" => "Teléfono DENUE",
        "sitio" => "Sitio declarado",
        "dominio" => "Dominio",
        "direccion" => "Dirección",
        "clase_actividad" => "Actividad",
        "clase_id" => "SCIAN",
        "estrato" => "Tamaño",
        "fecha_alta" => "Alta en DENUE",
        "latitud" => "Latitud",
        "longitud" => "Longitud",
        _ => field,
    }
}
