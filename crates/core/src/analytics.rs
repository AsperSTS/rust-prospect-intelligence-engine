use crate::{Db, Interaction, Profile, Prospect, Result, invalid};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Funnel {
    pub revisados: usize,
    pub contactados: usize,
    pub respondieron: usize,
    pub respuesta_positiva: usize,
    pub reuniones: usize,
    pub propuestas: usize,
    pub clientes: usize,
    pub response_rate: Option<f64>,
    pub positive_response_rate: Option<f64>,
    pub meeting_rate: Option<f64>,
    pub proposal_rate: Option<f64>,
    pub win_rate: Option<f64>,
}
pub fn funnel(rows: &[Prospect]) -> Funnel {
    let count = |stage: i64| rows.iter().filter(|p| p.etapa_max >= stage).count();
    let n = count(1);
    let ratio = |x: usize| {
        if n == 0 {
            None
        } else {
            Some((1000.0 * x as f64 / n as f64).round_ties_even() / 10.0)
        }
    };
    Funnel {
        revisados: rows.iter().filter(|p| p.estado != "nuevo").count(),
        contactados: n,
        respondieron: count(2),
        respuesta_positiva: count(3),
        reuniones: count(4),
        propuestas: count(5),
        clientes: count(6),
        response_rate: ratio(count(2)),
        positive_response_rate: ratio(count(3)),
        meeting_rate: ratio(count(4)),
        proposal_rate: ratio(count(5)),
        win_rate: ratio(count(6)),
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Cut {
    pub label: String,
    pub counts: Funnel,
    pub lectura: String,
}
pub fn reading(n: usize) -> String {
    if n < 20 {
        "Muestra pequeña — todavía no concluir"
    } else if n < 30 {
        "Señal débil: ideal 30+ antes de decisiones fuertes"
    } else {
        "Muestra suficiente para una primera lectura"
    }
    .into()
}
pub fn cuts(rows: &[Prospect], interactions: &[Interaction], dimension: &str) -> Result<Vec<Cut>> {
    let mut first = BTreeMap::new();
    for i in interactions
        .iter()
        .filter(|i| i.interaction_type == "contacto")
    {
        first.entry(&i.prospect_id).or_insert(i);
    }
    let mut groups: BTreeMap<String, Vec<Prospect>> = BTreeMap::new();
    for p in rows.iter().filter(|p| p.etapa_max >= 1) {
        let value = match dimension {
            "giro" => p.giro.clone(),
            "clase_id" => p.clase_id.clone(),
            "estrato" => p.estrato.clone(),
            "municipio" => p.municipio.clone(),
            "canal_contacto" => first
                .get(&p.id)
                .and_then(|i| i.channel.clone())
                .unwrap_or_default(),
            "variante_usada" => first
                .get(&p.id)
                .and_then(|i| i.template_variant.clone())
                .unwrap_or_default(),
            "fit_bucket" => match p.fit_score {
                0..=49 => "<50",
                50..=59 => "50-59",
                60..=69 => "60-69",
                _ => "70+",
            }
            .into(),
            "priority_bucket" => match p.priority_score {
                0..=29 => "<30",
                30..=49 => "30-49",
                50..=69 => "50-69",
                _ => "70+",
            }
            .into(),
            _ => return Err(invalid("Dimensión de métricas inválida")),
        };
        if !value.is_empty() {
            groups.entry(value).or_default().push(p.clone());
        }
    }
    let mut out = groups
        .into_iter()
        .map(|(label, rows)| {
            let counts = funnel(&rows);
            Cut {
                label,
                lectura: reading(counts.contactados),
                counts,
            }
        })
        .collect::<Vec<_>>();
    out.sort_by(|a, b| {
        b.counts
            .contactados
            .cmp(&a.counts.contactados)
            .then(a.label.cmp(&b.label))
    });
    Ok(out)
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Territory {
    pub entidad: String,
    pub municipio: String,
    pub establishments: usize,
    pub contactable: usize,
    pub high_fit: usize,
    pub clients: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct MapPoint {
    pub entidad: String,
    pub id: String,
    pub name: String,
    pub municipio: String,
    pub lat: f64,
    pub lon: f64,
    pub fit: i64,
    pub priority: i64,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Metrics {
    pub funnel: Funnel,
    pub cuts: Vec<Cut>,
    pub territories: Vec<Territory>,
    pub points: Vec<MapPoint>,
    pub opportunity_count: usize,
    pub won_mxn_cents: i64,
    pub won_usd_cents: i64,
    pub received_mxn_cents: i64,
    pub received_usd_cents: i64,
}
impl Db {
    pub async fn metrics(&self, profile: Profile, dimension: String) -> Result<Metrics> {
        self.memoized(format!("metrics:{}:{dimension}", profile.id()), move|c|{
        // One projection for analytics; no 4,709 per-record profile queries or full analysis JSON decoding.
        let mut statement=c.prepare("SELECT p.id,p.nombre,p.entidad,p.municipio,p.giro,p.clase_id,p.estrato,p.estado,p.etapa_max,p.correo,p.correo_web,p.telefono,p.whatsapp,p.latitud,p.longitud,COALESCE(s.fit_score,p.fit_score) AS fit_score,COALESCE(s.priority_score,p.priority_score) AS priority_score FROM prospectos p LEFT JOIN profile_scores s ON s.prospect_id=p.id AND s.profile_id=? ORDER BY p.rowid")?;
        let rows=statement.query_map([profile.id()],crate::db::decode)?.collect::<std::result::Result<Vec<Prospect>,_>>()?;
        let mut interacted=c.prepare("SELECT i.* FROM interactions i LEFT JOIN interaction_offers o ON o.interaction_id=i.id WHERE COALESCE(o.profile_id,'automatizacion')=? ORDER BY i.id")?;let interactions=interacted.query_map([profile.id()],|r|serde_json::from_value(crate::db::row_json(r)?).map_err(|e|rusqlite::Error::FromSqlConversionFailure(0,rusqlite::types::Type::Text,Box::new(e))))?.collect::<std::result::Result<Vec<Interaction>,_>>()?;
        let cohort=interactions.iter().filter(|i|i.interaction_type=="contacto").map(|i|i.prospect_id.as_str()).collect::<std::collections::HashSet<_>>();let counted=rows.iter().filter(|p|cohort.contains(p.id.as_str())||p.etapa_max==0).cloned().collect::<Vec<_>>();
        let mut areas:BTreeMap<String,Territory>=BTreeMap::new();let mut points=Vec::new();
        for p in &rows{let t=areas.entry(format!("{} · {}",p.municipio,p.entidad)).or_insert_with(||Territory{entidad:p.entidad.clone(),municipio:p.municipio.clone(),..Default::default()});t.establishments+=1;t.contactable+=usize::from(p.contactable());t.high_fit+=usize::from(p.fit_score>=50);t.clients+=usize::from(p.estado=="cliente");if let (Some(lat),Some(lon))=(p.latitud,p.longitud){points.push(MapPoint{entidad:p.entidad.clone(),id:p.id.clone(),name:p.nombre.clone(),municipio:p.municipio.clone(),lat,lon,fit:p.fit_score,priority:p.priority_score});}}
        points.sort_by_key(|b|std::cmp::Reverse(b.priority));let mut territories=areas.into_values().collect::<Vec<_>>();territories.sort_by_key(|t|std::cmp::Reverse(t.establishments));
        let aggregate:(i64,i64,i64,i64,i64)=c.query_row("SELECT COUNT(*),COALESCE(SUM(CASE WHEN stage='ganado' AND currency='MXN' THEN amount_cents ELSE 0 END),0),COALESCE(SUM(CASE WHEN stage='ganado' AND currency='USD' THEN amount_cents ELSE 0 END),0),COALESCE(SUM(CASE WHEN currency='MXN' THEN received_cents ELSE 0 END),0),COALESCE(SUM(CASE WHEN currency='USD' THEN received_cents ELSE 0 END),0) FROM opportunities WHERE profile_id=?",[profile.id()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?;
        Ok(Metrics{funnel:funnel(&counted),cuts:cuts(&counted,&interactions,&dimension)?,territories,points,opportunity_count:aggregate.0 as usize,won_mxn_cents:aggregate.1,won_usd_cents:aggregate.2,received_mxn_cents:aggregate.3,received_usd_cents:aggregate.4})
    }).await
    }
    pub async fn save_filter(&self, name: String, q: crate::Query) -> Result<()> {
        q.validate()?;
        if name.trim().is_empty() || name.chars().count() > 100 {
            return Err(invalid("Nombre de filtro inválido"));
        }
        self.call(move|c|{c.execute("INSERT INTO saved_filters VALUES (?,?) ON CONFLICT(name) DO UPDATE SET query_json=excluded.query_json",rusqlite::params![name,serde_json::to_string(&q)?])?;Ok(())}).await
    }
    pub async fn saved_filters(&self) -> Result<Vec<(String, crate::Query)>> {
        self.call(|c| {
            let mut s = c.prepare("SELECT name,query_json FROM saved_filters ORDER BY name")?;
            let raw = s
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<std::result::Result<Vec<_>, _>>()?;
            raw.into_iter()
                .map(|(n, q)| Ok((n, serde_json::from_str(&q)?)))
                .collect()
        })
        .await
    }
    pub async fn save_opportunity(&self, draft: OpportunityDraft) -> Result<i64> {
        draft.validate()?;
        self.call(move|c|{
        let tx=c.transaction()?;let name:String=tx.query_row("SELECT nombre FROM prospectos WHERE id=?",[&draft.prospect_id],|r|r.get(0)).optional()?.ok_or_else(||invalid("Establecimiento inexistente"))?;let account=format!("denue:{}",draft.prospect_id);let now=crate::config::now();
        tx.execute("INSERT INTO accounts(id,denue_id,name,created_at) VALUES (?,?,?,?) ON CONFLICT(denue_id) DO UPDATE SET name=excluded.name",rusqlite::params![account,draft.prospect_id,name,now])?;
        if !draft.contact_name.trim().is_empty(){let old:Option<i64>=tx.query_row("SELECT id FROM contacts WHERE account_id=? AND name=? AND role=?",rusqlite::params![account,draft.contact_name,draft.contact_role],|r|r.get(0)).optional()?;if let Some(id)=old {tx.execute("UPDATE contacts SET email=?,phone=?,notes=? WHERE id=?",rusqlite::params![draft.contact_email,draft.contact_phone,draft.notes,id])?;}else{tx.execute("INSERT INTO contacts(account_id,name,role,email,phone,notes) VALUES (?,?,?,?,?,?)",rusqlite::params![account,draft.contact_name,draft.contact_role,draft.contact_email,draft.contact_phone,draft.notes])?;}}
        let id=if let Some(id)=draft.id{let rows=tx.execute("UPDATE opportunities SET title=?,stage=?,amount_cents=?,received_cents=?,currency=?,next_action=?,due_date=?,notes=?,updated_at=? WHERE id=? AND account_id=? AND profile_id=?",rusqlite::params![draft.title,draft.stage,draft.amount_cents,draft.received_cents,draft.currency,draft.next_action,draft.due_date,draft.notes,now,id,account,draft.profile.id()])?;if rows!=1{return Err(invalid("Oportunidad inexistente o pertenece a otra oferta"));}id}else{tx.execute("INSERT INTO opportunities(account_id,profile_id,title,stage,amount_cents,received_cents,currency,next_action,due_date,notes,created_at,updated_at) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)",rusqlite::params![account,draft.profile.id(),draft.title,draft.stage,draft.amount_cents,draft.received_cents,draft.currency,draft.next_action,draft.due_date,draft.notes,now,now])?;tx.last_insert_rowid()};tx.commit()?;Ok(id)
    }).await
    }
    pub async fn opportunities(&self, profile: Profile) -> Result<Vec<Opportunity>> {
        self.call(move|c|{let mut s=c.prepare("SELECT o.id,a.denue_id,a.name,o.profile_id,o.title,o.stage,o.amount_cents,o.received_cents,o.currency,COALESCE(o.next_action,''),COALESCE(o.due_date,''),COALESCE(o.notes,'') FROM opportunities o JOIN accounts a ON a.id=o.account_id WHERE o.profile_id=? ORDER BY o.updated_at DESC,o.id DESC")?;Ok(s.query_map([profile.id()],|r|Ok(Opportunity{id:r.get(0)?,prospect_id:r.get(1)?,account_name:r.get(2)?,profile:Profile::parse(&r.get::<_,String>(3)?).unwrap_or_default(),title:r.get(4)?,stage:r.get(5)?,amount_cents:r.get(6)?,received_cents:r.get(7)?,currency:r.get(8)?,next_action:r.get(9)?,due_date:r.get(10)?,notes:r.get(11)?}))?.collect::<std::result::Result<Vec<_>,_>>()?)}).await
    }
}
use rusqlite::OptionalExtension;
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Opportunity {
    pub id: i64,
    pub prospect_id: String,
    pub account_name: String,
    pub profile: Profile,
    pub title: String,
    pub stage: String,
    pub amount_cents: i64,
    pub received_cents: i64,
    pub currency: String,
    pub next_action: String,
    pub due_date: String,
    pub notes: String,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct OpportunityDraft {
    pub id: Option<i64>,
    pub prospect_id: String,
    pub profile: Profile,
    pub title: String,
    pub stage: String,
    pub amount_cents: i64,
    pub received_cents: i64,
    pub currency: String,
    pub next_action: String,
    pub due_date: String,
    pub notes: String,
    pub contact_name: String,
    pub contact_role: String,
    pub contact_email: String,
    pub contact_phone: String,
}
impl OpportunityDraft {
    pub fn validate(&self) -> Result<()> {
        if self.prospect_id.is_empty()
            || self.title.trim().is_empty()
            || self.title.chars().count() > 200
            || self.notes.chars().count() > 20_000
            || ![
                "diagnostico",
                "propuesta",
                "negociacion",
                "ganado",
                "perdido",
            ]
            .contains(&self.stage.as_str())
            || !["MXN", "USD"].contains(&self.currency.as_str())
            || self.amount_cents < 0
            || self.amount_cents > 10_000_000_000_000
            || self.received_cents < 0
            || self.received_cents > self.amount_cents
        {
            return Err(invalid(
                "Oportunidad inválida; revisa título, etapa e importes",
            ));
        }
        if !self.due_date.is_empty()
            && chrono::NaiveDate::parse_from_str(&self.due_date, "%Y-%m-%d").is_err()
        {
            return Err(invalid("Fecha inválida"));
        }
        if !self.contact_email.is_empty() && !crate::contacts::email_valid(&self.contact_email) {
            return Err(invalid("Email del contacto inválido"));
        }
        Ok(())
    }
}
pub fn money_cents(text: &str) -> Result<i64> {
    let cleaned = text.trim().replace(',', "");
    let parts = cleaned.split('.').collect::<Vec<_>>();
    if parts.len() > 2
        || parts[0].is_empty()
        || parts.iter().any(|s| !s.chars().all(|c| c.is_ascii_digit()))
        || parts.get(1).is_some_and(|s| s.len() > 2)
    {
        return Err(invalid("Importe inválido; usa hasta dos decimales"));
    }
    let units: i64 = parts[0]
        .parse()
        .map_err(|_| invalid("Importe fuera de rango"))?;
    let cents = parts
        .get(1)
        .map(|s| format!("{s:0<2}").parse::<i64>().unwrap_or_default())
        .unwrap_or(0);
    units
        .checked_mul(100)
        .and_then(|n| n.checked_add(cents))
        .ok_or_else(|| invalid("Importe fuera de rango"))
}
