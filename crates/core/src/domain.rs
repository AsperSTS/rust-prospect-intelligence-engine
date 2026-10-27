use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
fn null_string<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}
fn null_int<'de, D: serde::Deserializer<'de>>(d: D) -> std::result::Result<i64, D::Error> {
    Ok(Option::<i64>::deserialize(d)?.unwrap_or_default())
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Prospect {
    #[serde(deserialize_with = "null_string")]
    pub id: String,
    #[serde(deserialize_with = "null_string")]
    pub nombre: String,
    #[serde(deserialize_with = "null_string")]
    pub razon_social: String,
    #[serde(deserialize_with = "null_string")]
    pub giro: String,
    #[serde(deserialize_with = "null_string")]
    pub clase_actividad: String,
    #[serde(deserialize_with = "null_string")]
    pub clase_id: String,
    #[serde(deserialize_with = "null_string")]
    pub rama_id: String,
    #[serde(deserialize_with = "null_string")]
    pub subrama_id: String,
    #[serde(deserialize_with = "null_string")]
    pub estrato: String,
    #[serde(deserialize_with = "null_string")]
    pub tipo_establecimiento: String,
    #[serde(deserialize_with = "null_string")]
    pub corredor_nombre: String,
    #[serde(deserialize_with = "null_string")]
    pub corredor_tipo: String,
    #[serde(deserialize_with = "null_string")]
    pub entidad: String,
    #[serde(deserialize_with = "null_string")]
    pub municipio: String,
    #[serde(deserialize_with = "null_string")]
    pub localidad: String,
    #[serde(deserialize_with = "null_string")]
    pub direccion: String,
    #[serde(deserialize_with = "null_string")]
    pub calle: String,
    #[serde(deserialize_with = "null_string")]
    pub num_exterior: String,
    #[serde(deserialize_with = "null_string")]
    pub num_interior: String,
    #[serde(deserialize_with = "null_string")]
    pub edificio: String,
    #[serde(deserialize_with = "null_string")]
    pub numero_local: String,
    #[serde(deserialize_with = "null_string")]
    pub cp: String,
    #[serde(deserialize_with = "null_string")]
    pub telefono: String,
    #[serde(deserialize_with = "null_string")]
    pub correo: String,
    #[serde(deserialize_with = "null_string")]
    pub sitio: String,
    #[serde(deserialize_with = "null_string")]
    pub dominio: String,
    #[serde(deserialize_with = "null_string")]
    pub fecha_alta: String,
    #[serde(deserialize_with = "null_string")]
    pub whatsapp: String,
    #[serde(deserialize_with = "null_string")]
    pub correo_web: String,
    #[serde(deserialize_with = "null_string")]
    pub senales: String,
    #[serde(deserialize_with = "null_string")]
    pub enriquecido_en: String,
    #[serde(deserialize_with = "null_string")]
    pub problema: String,
    #[serde(deserialize_with = "null_string")]
    pub canal: String,
    #[serde(deserialize_with = "null_string")]
    pub mensaje: String,
    #[serde(deserialize_with = "null_string")]
    pub estado: String,
    #[serde(deserialize_with = "null_string")]
    pub notas: String,
    #[serde(deserialize_with = "null_string")]
    pub contactado_en: String,
    #[serde(deserialize_with = "null_string")]
    pub proximo_seguimiento: String,
    #[serde(deserialize_with = "null_string")]
    pub variante: String,
    #[serde(deserialize_with = "null_string")]
    pub creado_en: String,
    #[serde(deserialize_with = "null_string")]
    pub website_checked_at: String,
    #[serde(deserialize_with = "null_string")]
    pub group_key: String,
    #[serde(deserialize_with = "null_string")]
    pub group_reason: String,
    #[serde(deserialize_with = "null_string")]
    pub giro_display: String,
    #[serde(deserialize_with = "null_string")]
    pub hypothesis_text: String,
    #[serde(deserialize_with = "null_string")]
    pub hypothesis_basis: String,
    #[serde(deserialize_with = "null_string")]
    pub hypothesis_confidence: String,
    #[serde(deserialize_with = "null_string")]
    pub snooze_until: String,
    #[serde(deserialize_with = "null_string")]
    pub analizado_en: String,
    #[serde(deserialize_with = "null_string")]
    pub fit_breakdown: String,
    #[serde(deserialize_with = "null_string")]
    pub priority_breakdown: String,
    #[serde(deserialize_with = "null_int")]
    pub score: i64,
    #[serde(deserialize_with = "null_int")]
    pub toques: i64,
    #[serde(deserialize_with = "null_int")]
    pub source_website: i64,
    #[serde(deserialize_with = "null_int")]
    pub possible_chain: i64,
    #[serde(deserialize_with = "null_int")]
    pub possible_chain_count: i64,
    #[serde(deserialize_with = "null_int")]
    pub fit_score: i64,
    #[serde(deserialize_with = "null_int")]
    pub priority_score: i64,
    #[serde(deserialize_with = "null_int")]
    pub etapa_max: i64,
    pub latitud: Option<f64>,
    pub longitud: Option<f64>,
}

impl Prospect {
    pub fn closed(&self) -> bool {
        crate::config::CLOSED.contains(&self.estado.as_str())
    }
    pub fn contactable(&self) -> bool {
        crate::contacts::email_valid(&self.correo)
            || crate::contacts::email_valid(&self.correo_web)
            || !crate::contacts::phone(&self.telefono).is_empty()
            || !crate::contacts::phone(&self.whatsapp).is_empty()
    }
    pub fn email(&self) -> &str {
        if crate::contacts::email_valid(&self.correo_web) {
            &self.correo_web
        } else if crate::contacts::email_valid(&self.correo) {
            &self.correo
        } else {
            ""
        }
    }
    pub fn phone(&self) -> String {
        let t = crate::contacts::phone(&self.telefono);
        if t.is_empty() {
            crate::contacts::phone(&self.whatsapp)
        } else {
            t
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Signal {
    pub signal_type: String,
    pub signal_text: String,
    pub signal_source_url: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ScoreItem {
    pub clave: String,
    pub etiqueta: String,
    pub pts: i64,
    #[serde(rename = "max")]
    pub maximum: i64,
    pub nota: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Action {
    pub action: String,
    pub reason: String,
    pub confidence: String,
    pub alternativas: Vec<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub group_key: String,
    pub possible_chain: i64,
    pub possible_chain_count: i64,
    pub group_reason: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Analysis {
    pub giro: String,
    pub giro_display: String,
    pub fit_score: i64,
    pub fit_breakdown: Vec<ScoreItem>,
    pub priority_score: i64,
    pub priority_breakdown: Vec<ScoreItem>,
    pub hypothesis_text: String,
    pub hypothesis_basis: String,
    pub hypothesis_confidence: String,
    pub action: Action,
    pub group: Group,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub canal: String,
    pub variante: String,
    pub cuerpo: String,
    pub asunto: String,
    pub problemas: Vec<String>,
}
impl Message {
    pub fn valid(&self) -> bool {
        self.problemas.iter().all(|p| p.starts_with("Aviso:"))
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Profile {
    #[default]
    Automatizacion,
    WebMarketing,
}
impl Profile {
    pub fn id(self) -> &'static str {
        match self {
            Self::Automatizacion => "automatizacion",
            Self::WebMarketing => "web_marketing",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatizacion => "Automatización y sistemas",
            Self::WebMarketing => "Web y marketing",
        }
    }
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "automatizacion" => Ok(Self::Automatizacion),
            "web_marketing" => Ok(Self::WebMarketing),
            _ => Err(invalid("Perfil inválido")),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum View {
    #[default]
    Today,
    Due,
    Conversations,
    All,
    Clients,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryOrder {
    #[default]
    Automatic,
    Priority,
    Name,
    Municipality,
    Fit,
    State,
    Followup,
}
impl QueryOrder {
    pub const ALL: [Self; 7] = [
        Self::Automatic,
        Self::Priority,
        Self::Name,
        Self::Municipality,
        Self::Fit,
        Self::State,
        Self::Followup,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Automatic => "Automático",
            Self::Priority => "Prioridad · mayor primero",
            Self::Name => "Nombre · A–Z",
            Self::Municipality => "Municipio · A–Z",
            Self::Fit => "Fit · mayor primero",
            Self::State => "Estado comercial · A–Z",
            Self::Followup => "Seguimiento · próximo primero",
        }
    }
    pub(crate) fn sql(self, view: View) -> &'static str {
        match self {
            Self::Automatic if view == View::Due => "p.proximo_seguimiento,p.id",
            Self::Automatic | Self::Priority => {
                "COALESCE(s.priority_score,p.priority_score) DESC,p.id"
            }
            Self::Name => "lower_unicode(p.nombre),p.id",
            Self::Municipality => "lower_unicode(p.municipio),p.entidad,p.id",
            Self::Fit => "COALESCE(s.fit_score,p.fit_score) DESC,p.id",
            Self::State => "p.estado,p.id",
            Self::Followup => {
                "CASE WHEN COALESCE(p.proximo_seguimiento,'')='' THEN 1 ELSE 0 END,p.proximo_seguimiento,p.id"
            }
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryGroup {
    #[default]
    None,
    Entity,
    Municipality,
    Sector,
}
impl QueryGroup {
    pub const ALL: [Self; 4] = [Self::None, Self::Entity, Self::Municipality, Self::Sector];
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "Sin agrupar",
            Self::Entity => "Entidad",
            Self::Municipality => "Municipio",
            Self::Sector => "Giro",
        }
    }
    pub(crate) fn sql(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Entity => "p.entidad,",
            Self::Municipality => "p.entidad,p.municipio,",
            Self::Sector => "p.giro,",
        }
    }
    pub fn key(self, p: &Prospect) -> String {
        match self {
            Self::None => String::new(),
            Self::Entity => {
                if p.entidad.is_empty() {
                    "Sin entidad".into()
                } else {
                    p.entidad.clone()
                }
            }
            Self::Municipality => format!(
                "{} · {}",
                if p.municipio.is_empty() {
                    "Sin municipio"
                } else {
                    &p.municipio
                },
                p.entidad
            ),
            Self::Sector => {
                if p.giro.is_empty() {
                    "Sin giro".into()
                } else {
                    p.giro.clone()
                }
            }
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Query {
    pub entidad: String,
    pub search: String,
    pub view: View,
    pub profile: Profile,
    pub state: String,
    pub giro: String,
    pub municipio: String,
    pub fit_min: i64,
    pub contactable: bool,
    pub page: usize,
    pub size: usize,
    pub order: QueryOrder,
    pub group: QueryGroup,
}
impl Query {
    pub fn validate(&self) -> Result<()> {
        if self.search.chars().count() > 300
            || self.giro.chars().count() > 300
            || self.entidad.chars().count() > 100
            || self.municipio.chars().count() > 300
            || !(0..=100).contains(&self.fit_min)
            || self.page > 100_000
            || self.size > 500
        {
            return Err(invalid("Filtros fuera de rango"));
        }
        if !self.state.is_empty() && !crate::config::STATES.contains(&self.state.as_str()) {
            return Err(invalid("Estado inválido"));
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Prospect>,
    pub total: usize,
    pub page: usize,
    pub size: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Summary {
    pub total: usize,
    pub today: usize,
    pub due: usize,
    pub conversations: usize,
    pub clients: usize,
    pub with_contact: usize,
    pub researched_today: usize,
    pub contacted_today: usize,
    pub followed_today: usize,
    pub publications_today: usize,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Interaction {
    pub id: i64,
    pub prospect_id: String,
    pub created_at: String,
    pub channel: Option<String>,
    pub interaction_type: String,
    pub sequence_step: Option<i64>,
    pub template_variant: Option<String>,
    pub message_rendered: Option<String>,
    pub response_type: Option<String>,
    pub notes: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Command {
    Contact {
        channel: String,
        variant: String,
        message: String,
        subject: String,
    },
    Followup {
        channel: String,
        variant: String,
        message: String,
    },
    Response {
        response: String,
    },
    State {
        state: String,
    },
    Note {},
    Snooze {
        days: i64,
    },
    Research {},
    Publication {},
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mutation {
    pub operation_id: String,
    pub prospect_id: String,
    pub profile: Profile,
    pub command: Command,
    pub notes: Option<String>,
}
impl Mutation {
    pub fn validate(&self) -> Result<()> {
        if self.operation_id.is_empty()
            || self.operation_id.len() > 200
            || (self.prospect_id.is_empty() && !matches!(self.command, Command::Publication { .. }))
            || self.prospect_id.len() > 200
            || self
                .notes
                .as_ref()
                .is_some_and(|v| v.chars().count() > 20_000)
        {
            return Err(invalid("Acción inválida"));
        }
        Ok(())
    }
}
pub fn operation_id() -> String {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    format!(
        "{}-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    )
}
