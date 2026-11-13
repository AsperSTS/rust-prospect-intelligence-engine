use crate::{Message, Profile, Prospect, Result, Settings, Signal, invalid};
use regex::{Captures, Regex};
use sha1::{Digest, Sha1};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::LazyLock,
};

pub const BUNDLED_TEMPLATES: &str = include_str!("../../../config/templates.toml");

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct Templates {
    pub whatsapp: BTreeMap<String, String>,
    pub email: BTreeMap<String, String>,
    pub seguimiento: BTreeMap<String, String>,
    pub giros: BTreeMap<String, BTreeMap<String, String>>,
    /// Reusable user data: visible label → value. Templates reference it as `{data_key(label)}`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub datos: BTreeMap<String, String>,
}

/// Placeholder names owned by the application; user data cannot shadow them.
pub const RESERVED_KEYS: [&str; 13] = [
    "nombre",
    "owner",
    "owner_corto",
    "credencial",
    "whatsapp",
    "portafolio",
    "linkedin",
    "web",
    "giro_corto",
    "cosa",
    "pregunta",
    "ejemplo",
    "obs",
];

/// Placeholder key for a reusable datum: lowercase words joined by `_`.
pub fn data_key(label: &str) -> String {
    label
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

impl Templates {
    pub fn bundled_source() -> &'static str {
        BUNDLED_TEMPLATES
    }

    pub fn ensure_file(home: &Path) -> Result<std::path::PathBuf> {
        let path = home.join("templates.toml");
        if !path.exists() {
            crate::config::write_private(&path, BUNDLED_TEMPLATES.as_bytes())?;
        }
        Ok(path)
    }

    pub fn source(home: &Path) -> Result<String> {
        let path = home.join("templates.toml");
        Ok(if path.exists() {
            std::fs::read_to_string(path)?
        } else {
            BUNDLED_TEMPLATES.into()
        })
    }

    pub fn parse_toml(source: &str) -> Result<Self> {
        Ok(toml::from_str(source)?)
    }

    pub fn to_toml(&self) -> Result<String> {
        toml::to_string_pretty(self).map_err(|e| invalid(e.to_string()))
    }

    pub fn load(home: &Path) -> Result<Self> {
        let t: Self = toml::from_str(&Self::source(home)?)?;
        t.validate()?;
        Ok(t)
    }

    pub fn save(home: &Path, source: &str) -> Result<()> {
        if source.len() > 256_000 {
            return Err(invalid("Las plantillas exceden el tamaño permitido"));
        }
        let templates: Self = toml::from_str(source)?;
        templates.validate()?;
        crate::config::write_private(&home.join("templates.toml"), source.as_bytes())
    }

    pub fn validate(&self) -> Result<()> {
        // Keep the original required keys so existing local template files remain valid.
        for (map, keys) in [
            (&self.whatsapp, vec!["A", "B", "C", "D"]),
            (
                &self.email,
                vec!["cuerpo_con_credencial", "cuerpo_sin_credencial"],
            ),
            (&self.seguimiento, vec!["d3", "d7", "d14"]),
        ] {
            if keys.iter().any(|key| !map.contains_key(*key)) {
                return Err(invalid("Faltan claves en las plantillas"));
            }
        }
        let has_unified_subjects = self.email.contains_key("asunto_con_credencial")
            && self.email.contains_key("asunto_sin_credencial");
        let has_legacy_subjects =
            self.email.contains_key("asunto_A") && self.email.contains_key("asunto_B");
        if !has_unified_subjects && !has_legacy_subjects {
            return Err(invalid("Faltan claves en las plantillas"));
        }
        if !self.giros.contains_key("Otro")
            || self.giros.values().any(|g| {
                ["cosa", "pregunta", "ejemplo"]
                    .iter()
                    .any(|key| !g.contains_key(*key))
            })
        {
            return Err(invalid("Plantillas por giro incompletas"));
        }
        let mut seen = BTreeSet::new();
        for (label, value) in &self.datos {
            let key = data_key(label);
            if key.is_empty()
                || label.chars().count() > 40
                || label.contains(['[', ']', '{', '}'])
                || value.chars().count() > 500
                || RESERVED_KEYS.contains(&key.as_str())
                || !seen.insert(key)
            {
                return Err(invalid(format!("Dato reutilizable inválido: {label}")));
            }
        }
        if self.datos.len() > 30 {
            return Err(invalid("Demasiados datos reutilizables (máximo 30)"));
        }
        Ok(())
    }
}

pub fn clean_name(raw: &str) -> String {
    static LEGAL: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)[\s,.]+(S\.?\s?A\.?\s?P\.?\s?I\.?(\s?DE\s?C\.?\s?V\.?)?|S\.?\s?A\.?(\s?DE\s?C\.?\s?V\.?)?|S\.?\s?DE\s?R\.?\s?L\.?(\s?DE\s?C\.?\s?V\.?)?|S\.?\s?C\.?|A\.?\s?C\.?)\s*$").unwrap()
    });
    static BRANCH: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?i)\b(?:SUCURSAL|SUC\.?)(?:\s|$)").unwrap());
    // Keep `raw` intact: a name that starts with "SUCURSAL" must fall back to itself.
    let head = BRANCH.find(raw).map(|m| &raw[..m.start()]).unwrap_or(raw);
    let mut value = head
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', ',', '.', '-'])
        .to_owned();
    for _ in 0..3 {
        value = LEGAL
            .replace(&value, "")
            .trim_matches([' ', ',', '.', '-'])
            .into();
    }
    let mut words = Vec::new();
    for (i, w) in value.split(' ').enumerate() {
        let base = w.trim_matches(['.', ',']).to_lowercase();
        let chars = w.chars().collect::<Vec<_>>();
        let acronym = !chars.is_empty()
            && chars.iter().all(|c| c.is_alphabetic() && c.is_uppercase())
            && ![
                "san", "sur", "luz", "oro", "rio", "gas", "sol", "mar", "pan", "uno", "dos", "sin",
                "mas", "una", "ver", "mex", "don", "doña",
            ]
            .contains(&base.as_str())
            && (chars.len() <= 3
                || (chars.len() == 4
                    && !"AEIOU".contains(chars[0])
                    && !"AEIOU".contains(chars[1])
                    && ![
                        "pl", "pr", "bl", "br", "cl", "cr", "dr", "fl", "fr", "gl", "gr", "tr",
                        "ch", "ll", "rr",
                    ]
                    .contains(&base.chars().take(2).collect::<String>().as_str())));
        let allcase = w == w.to_uppercase() || w == w.to_lowercase();
        let shown = if i > 0
            && [
                "de", "del", "la", "las", "los", "el", "y", "e", "en", "para", "por", "con", "al",
            ]
            .contains(&base.as_str())
        {
            base
        } else if acronym {
            w.to_owned()
        } else if allcase {
            let mut cs = w.chars();
            cs.next()
                .map(|c| c.to_uppercase().collect::<String>() + &cs.as_str().to_lowercase())
                .unwrap_or_default()
        } else {
            w.to_owned()
        };
        words.push(shown);
    }
    let formatted = words.join(" ");
    if formatted.is_empty() {
        crate::rules::title(raw)
    } else {
        formatted
    }
}

fn hash_mod(id: &str, modulus: usize) -> usize {
    if modulus == 0 {
        return 0;
    }
    Sha1::digest(id.as_bytes())
        .iter()
        .fold(0usize, |result, byte| {
            (result * 256 + usize::from(*byte)) % modulus
        })
}

fn choose_key(keys: Vec<String>, id: &str, salt: &str) -> Option<String> {
    if keys.is_empty() {
        return None;
    }
    let seed = format!("{id}:{salt}");
    keys.get(hash_mod(&seed, keys.len())).cloned()
}

fn keys_with_prefix(map: &BTreeMap<String, String>, prefix: &str) -> Vec<String> {
    map.keys()
        .filter(|key| key.starts_with(prefix))
        .cloned()
        .collect()
}

fn whatsapp_key(t: &Templates, id: &str, evidence: bool) -> String {
    let keys = t
        .whatsapp
        .keys()
        .filter(|key| !key.contains('_'))
        .filter(|key| evidence || key.as_str() != "C")
        .cloned()
        .collect::<Vec<_>>();
    choose_key(keys, id, "whatsapp").unwrap_or_else(|| "A".into())
}

pub fn email_subject_key_for_body(body_key: &str) -> String {
    if let Some(rest) = body_key.strip_prefix("cuerpo_") {
        format!("asunto_{rest}")
    } else {
        format!("asunto_{body_key}")
    }
}

/// Resolves the email subject key paired with `body_key`, falling back to
/// detached legacy `asunto_*` keys when reading older local `templates.toml` files.
pub fn resolve_email_subject_key(t: &Templates, body_key: &str, id: &str) -> String {
    let paired = email_subject_key_for_body(body_key);
    if t.email.contains_key(&paired) {
        return paired;
    }
    if let Some(num) = body_key.rsplit('_').next() {
        let numbered = format!("asunto_{num}");
        if t.email.contains_key(&numbered) {
            return numbered;
        }
    }
    let legacy_default = if body_key.starts_with("cuerpo_sin_") {
        "asunto_B"
    } else {
        "asunto_A"
    };
    if t.email.contains_key(legacy_default) {
        return legacy_default.into();
    }
    choose_key(keys_with_prefix(&t.email, "asunto_"), id, "email-subject").unwrap_or(paired)
}

fn email_body_key(t: &Templates, id: &str, credential: bool) -> String {
    let prefix = if credential {
        "cuerpo_con_"
    } else {
        "cuerpo_sin_"
    };
    choose_key(keys_with_prefix(&t.email, prefix), id, "email-body").unwrap_or_else(|| {
        if credential {
            "cuerpo_con_credencial".into()
        } else {
            "cuerpo_sin_credencial".into()
        }
    })
}

fn followup_key(t: &Templates, id: &str, base: &str) -> String {
    let mut keys = t
        .seguimiento
        .keys()
        .filter(|key| key.as_str() == base || key.starts_with(&format!("{base}_")))
        .cloned()
        .collect::<Vec<_>>();
    keys.sort();
    choose_key(keys, id, base).unwrap_or_else(|| base.into())
}

// Kept for callers/tests that use the original public helper.
pub fn variants(id: &str, evidence: bool) -> (String, bool, String) {
    let options = if evidence {
        vec!["A", "B", "C", "D"]
    } else {
        vec!["A", "B", "D"]
    };
    (
        options[hash_mod(id, options.len())].into(),
        hash_mod(&format!("{id}:credential"), 2) == 0,
        if hash_mod(&format!("{id}:subject"), 2) == 0 {
            "A".into()
        } else {
            "B".into()
        },
    )
}

fn vars(
    p: &Prospect,
    settings: &Settings,
    templates: &Templates,
    observation: &str,
) -> BTreeMap<String, String> {
    let giro = templates
        .giros
        .get(&p.giro)
        .unwrap_or(&templates.giros["Otro"]);
    let mut values = giro.clone();
    for (label, value) in &templates.datos {
        values.insert(data_key(label), value.clone());
    }
    values.insert("nombre".into(), clean_name(&p.nombre));
    values.insert(
        "giro_corto".into(),
        if !p.giro_display.is_empty() {
            p.giro_display.clone()
        } else if !p.giro.is_empty() {
            p.giro.to_lowercase()
        } else {
            "su sector".into()
        },
    );
    values.insert("obs".into(), observation.into());
    values.insert("credencial".into(), settings.owner_credential.clone());
    values.insert("owner".into(), settings.owner_name.clone());
    values.insert(
        "owner_corto".into(),
        settings
            .owner_name
            .split_whitespace()
            .next()
            .unwrap_or("")
            .into(),
    );
    values.insert(
        "whatsapp".into(),
        crate::contacts::pretty_phone(&settings.owner_whatsapp),
    );
    values.insert("portafolio".into(), settings.owner_portfolio.trim().into());
    values.insert("linkedin".into(), settings.owner_linkedin.trim().into());
    values.insert("web".into(), settings.owner_website.trim().into());
    values
}

fn render(source: &str, values: &BTreeMap<String, String>) -> String {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());
    let mut rendered = PLACEHOLDER
        .replace_all(source, |captures: &Captures| {
            values
                .get(&captures[1])
                .cloned()
                .unwrap_or_else(|| captures[0].into())
        })
        .to_string();
    let empty_labels: Vec<&str> = [
        ("whatsapp", &["WhatsApp:"][..]),
        ("portafolio", &["Portafolio:"][..]),
        ("linkedin", &["LinkedIn:", "Linkedin:"][..]),
        ("web", &["Página web:", "Web:"][..]),
    ]
    .into_iter()
    .filter(|(k, _)| values.get(*k).is_some_and(|v| v.trim().is_empty()))
    .flat_map(|(_, labels)| labels.iter().copied())
    .collect();
    if !empty_labels.is_empty() {
        rendered = rendered
            .lines()
            .filter(|line| !empty_labels.contains(&line.trim()))
            .collect::<Vec<_>>()
            .join("\n");
    }
    rendered.trim().into()
}

pub fn first(
    p: &Prospect,
    channel: &str,
    signals: &[Signal],
    settings: &Settings,
    templates: &Templates,
    profile: Profile,
) -> Result<Message> {
    if !["whatsapp", "email"].contains(&channel) {
        return Err(invalid("Canal de plantilla inválido"));
    }
    let observation = crate::rules::observation(signals);
    let values = vars(
        p,
        settings,
        templates,
        observation
            .as_ref()
            .map(|item| item.1.as_str())
            .unwrap_or(""),
    );

    let mut message = if profile == Profile::WebMarketing {
        let question =
            "¿Con quién podría revisar cómo reciben hoy consultas de nuevos clientes por internet?";
        if channel == "whatsapp" {
            Message {
                canal: channel.into(),
                variante: "WEB-WA-A".into(),
                cuerpo: format!(
                    "Hola, buen día. Soy {}. Vi que {} trabaja en {}. Desarrollo sitios web y herramientas para captar clientes. {question}",
                    values["owner"], values["nombre"], values["giro_corto"]
                ),
                asunto: String::new(),
                problemas: vec![],
            }
        } else {
            let subject = format!("Pregunta sobre el sitio de {}", values["nombre"]);
            let whatsapp_line = if values["whatsapp"].trim().is_empty() {
                String::new()
            } else {
                format!("\nWhatsApp: {}", values["whatsapp"])
            };
            Message {
                canal: channel.into(),
                variante: "WEB-EM-A".into(),
                cuerpo: format!(
                    "Buen día,\n\nSoy {}. Desarrollo sitios web y herramientas para captar clientes. Estuve revisando a {} y quería hacerles una pregunta:\n\n{question}\n\nSi tiene sentido revisarlo, puedo mostrarles un ejemplo sencillo por este medio.\n\nSaludos,\n{}{}",
                    values["owner"], values["nombre"], values["owner"], whatsapp_line
                ),
                asunto: if subject.chars().count() <= 60 {
                    subject
                } else {
                    "Una pregunta sobre su sitio web".into()
                },
                problemas: vec![],
            }
        }
    } else if channel == "whatsapp" {
        let key = whatsapp_key(templates, &p.id, observation.is_some());
        Message {
            canal: channel.into(),
            variante: format!("WA-{key}"),
            cuerpo: render(&templates.whatsapp[&key], &values),
            asunto: String::new(),
            problemas: vec![],
        }
    } else {
        let credential = !settings.owner_credential.trim().is_empty()
            && hash_mod(&format!("{}:credential", p.id), 2) == 0;
        let body_key = email_body_key(templates, &p.id, credential);
        let subject_key = resolve_email_subject_key(templates, &body_key, &p.id);
        let mut subject = templates
            .email
            .get(&subject_key)
            .map(|tpl| render(tpl, &values))
            .unwrap_or_default();
        if subject.chars().count() > 60 {
            subject = format!("Pregunta sobre {}", values["cosa"]);
        }
        if subject.chars().count() > 60 {
            subject = "Una pregunta sobre sus procesos".into();
        }
        let variante = if subject_key == email_subject_key_for_body(&body_key) {
            format!("EM-{body_key}")
        } else {
            format!("EM-{body_key}-{subject_key}")
        };
        Message {
            canal: channel.into(),
            variante,
            cuerpo: render(&templates.email[&body_key], &values),
            asunto: subject,
            problemas: vec![],
        }
    };

    message.problemas = validate(
        &message.cuerpo,
        &message.canal,
        &message.asunto,
        observation.is_some(),
        settings,
    );
    Ok(message)
}

pub fn followup(
    p: &Prospect,
    step: i64,
    settings: &Settings,
    templates: &Templates,
    profile: Profile,
) -> Message {
    let base = match step {
        1 => "d3",
        2 => "d7",
        _ => "d14",
    };
    let values = vars(p, settings, templates, "");
    let (text, key) = if profile == Profile::Automatizacion {
        let key = followup_key(templates, &p.id, base);
        (render(&templates.seguimiento[&key], &values), key)
    } else {
        let text = match step {
            1 => "Hola nuevamente. Para aterrizar la idea, podría mostrarle un ejemplo de cómo presentar sus servicios y recibir consultas en una página sencilla. Si es útil, con gusto lo revisamos por este medio.",
            2 => "Buen día. Una forma simple de comenzar sería revisar cómo llegan hoy sus consultas y qué información necesitan sus clientes antes de escribirles. Si le sirve, puedo explicarle la idea por aquí.",
            _ => "Hola. Cierro el tema por aquí para no llenar su bandeja. Si más adelante quieren revisar su sitio web o la captación de consultas, con gusto puedo ayudarles.",
        }
        .to_owned();
        (text, base.into())
    };
    Message {
        canal: "seguimiento".into(),
        variante: format!(
            "{}FU-{key}",
            if profile == Profile::WebMarketing {
                "WEB-"
            } else {
                ""
            }
        ),
        problemas: validate(&text, "seguimiento", "", false, settings),
        cuerpo: text,
        asunto: String::new(),
    }
}

pub fn validate(
    text: &str,
    channel: &str,
    subject: &str,
    evidence: bool,
    settings: &Settings,
) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Checks {
        #[serde(rename = "PROHIBIDAS")]
        prohibited: Vec<String>,
        #[serde(rename = "CLAIMS_NO_SUSTENTADOS")]
        claims: Vec<String>,
    }
    static CHECKS: LazyLock<Checks> = LazyLock::new(|| {
        serde_json::from_str(include_str!("../../../config/mensajes.json")).unwrap()
    });
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{\w+\}").unwrap());
    static CLAIMS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
        CHECKS
            .claims
            .iter()
            .map(|rule| Regex::new(rule).unwrap())
            .collect()
    });

    let mut errors = Vec::new();
    let lower = text.to_lowercase();
    let joined = format!("{text}{subject}");
    let variables = PLACEHOLDER
        .find_iter(&joined)
        .map(|m| m.as_str().to_owned())
        .collect::<BTreeSet<_>>();
    if !variables.is_empty() {
        errors.push(format!(
            "Quedan variables sin renderizar: {}",
            variables.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    let words = text.split_whitespace().count();
    let max = match channel {
        "whatsapp" => 65,
        "email" => 120,
        _ => 80,
    };
    if words > max {
        errors.push(format!("Demasiado largo: {words} palabras (máx. {max})"));
    }
    for phrase in &CHECKS.prohibited {
        if lower.contains(phrase) {
            errors.push(format!("Frase prohibida: «{phrase}»"));
        }
    }
    if CLAIMS.iter().any(|rule| rule.is_match(&lower)) {
        errors.push("Afirmación no sustentada sobre la empresa".into());
    }
    if lower.contains("vi en su sitio") && !evidence {
        errors.push("Cita el sitio web sin evidencia registrada".into());
    }
    if channel == "whatsapp" {
        let questions = text.matches('?').count();
        if questions != 1 {
            errors.push(format!(
                "Debe tener exactamente 1 pregunta (tiene {questions})"
            ));
        }
        if lower.contains("http") || lower.contains("www.") {
            errors.push("No incluyas enlaces en el primer mensaje".into());
        }
    }
    if channel == "email" {
        let length = subject.chars().count();
        if subject.is_empty() {
            errors.push("Falta el asunto".into());
        } else if length > 70 {
            errors.push(format!(
                "Asunto largo: {length} caracteres (máx. 70, ideal ≤ 60)"
            ));
        } else if length > 60 {
            errors.push(format!("Aviso: asunto de {length} caracteres (ideal ≤ 60)"));
        }
        if text.contains('<') && text.contains('>') {
            errors.push("Contiene HTML".into());
        }
        if !text.contains(&settings.owner_name) {
            errors.push("Falta la firma".into());
        }
    }
    errors
}

#[cfg(test)]
mod editor_tests {
    use super::*;

    #[test]
    fn editing_templates_validates_before_replacing_the_saved_file() {
        let home = tempfile::tempdir().unwrap();
        let original = Templates::source(home.path()).unwrap();
        Templates::save(home.path(), &original).unwrap();
        assert!(Templates::save(home.path(), "invalid = [").is_err());
        assert!(Templates::save(home.path(), "[whatsapp]\nA = 'incompleta'").is_err());
        assert_eq!(Templates::source(home.path()).unwrap(), original);
        let edited = original.replace("Hola, buen día", "Buenos días");
        Templates::save(home.path(), &edited).unwrap();
        assert_eq!(Templates::source(home.path()).unwrap(), edited);
        Templates::load(home.path()).unwrap();
    }

    #[test]
    fn bundled_templates_offer_real_message_variety() {
        let raw = Templates::bundled_source();
        assert!(!raw.contains("Audi México"));
        assert!(!raw.contains("Yael"));
        assert!(!raw.contains("954"));
        let templates: Templates = toml::from_str(raw).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let path = Templates::ensure_file(temp.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), raw);
        std::fs::write(&path, "personal").unwrap();
        Templates::ensure_file(temp.path()).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "personal");
        assert!(templates.whatsapp.len() >= 10);
        let bodies: Vec<String> = keys_with_prefix(&templates.email, "cuerpo_");
        assert!(keys_with_prefix(&templates.email, "cuerpo_con_").len() >= 10);
        assert!(keys_with_prefix(&templates.email, "cuerpo_sin_").len() >= 10);
        for body in &bodies {
            let subject_key = email_subject_key_for_body(body);
            assert!(
                templates.email.contains_key(&subject_key),
                "Falta asunto pareado {subject_key} para {body}"
            );
        }
        for base in ["d3", "d7", "d14"] {
            assert!(
                templates
                    .seguimiento
                    .keys()
                    .filter(|key| key.as_str() == base || key.starts_with(&format!("{base}_")))
                    .count()
                    >= 10
            );
        }
    }

    #[test]
    fn names_that_start_with_a_branch_word_keep_their_text_and_data_fills_messages() {
        assert_eq!(clean_name("SUCURSAL CENTRO"), "Sucursal Centro");
        assert_eq!(clean_name("SUCURSAL PUEBLA NORTE"), "Sucursal Puebla Norte");
        assert_eq!(clean_name("TALLER ALFA SUCURSAL SUR"), "Taller Alfa");
        assert_eq!(data_key("Mi empresa"), "mi_empresa");
        let mut templates = Templates::parse_toml(Templates::bundled_source()).unwrap();
        templates
            .datos
            .insert("Mi empresa".into(), "Soluciones del Valle".into());
        templates.validate().unwrap();
        let values = vars(&Prospect::default(), &Settings::default(), &templates, "");
        assert_eq!(
            render("Soy de {mi_empresa}.", &values),
            "Soy de Soluciones del Valle."
        );
        let with_links = Settings {
            owner_portfolio: "https://github.com/ejemplo".into(),
            owner_linkedin: "https://linkedin.com/in/ejemplo".into(),
            owner_website: "https://ejemplo.dev".into(),
            ..Settings::default()
        };
        let link_values = vars(&Prospect::default(), &with_links, &templates, "");
        assert_eq!(
            render(
                "Portafolio: {portafolio}\nLinkedIn: {linkedin}\nPágina web: {web}",
                &link_values
            ),
            "Portafolio: https://github.com/ejemplo\nLinkedIn: https://linkedin.com/in/ejemplo\nPágina web: https://ejemplo.dev"
        );
        assert_eq!(
            render(
                "Hola\nPortafolio: {portafolio}\nLinkedIn: {linkedin}\nPágina web: {web}",
                &values
            ),
            "Hola"
        );
        templates.datos.insert("owner".into(), "x".into());
        assert!(templates.validate().is_err());
    }

    #[test]
    fn email_uses_paired_subject_and_preserves_legacy_subject_fallback() {
        let mut templates = Templates::parse_toml(Templates::bundled_source()).unwrap();
        let prospect = Prospect {
            id: "em-test".into(),
            nombre: "Taller Norte".into(),
            giro: "Manufactura".into(),
            ..Prospect::default()
        };
        let settings = Settings::default();
        let msg = first(
            &prospect,
            "email",
            &[],
            &settings,
            &templates,
            Profile::Automatizacion,
        )
        .unwrap();
        assert_eq!(msg.canal, "email");
        let body_key = msg.variante.strip_prefix("EM-").unwrap();
        let subject_key = email_subject_key_for_body(body_key);
        templates
            .email
            .insert(subject_key, "Asunto único {nombre}".into());
        let updated = first(
            &prospect,
            "email",
            &[],
            &settings,
            &templates,
            Profile::Automatizacion,
        )
        .unwrap();
        assert_eq!(updated.asunto, "Asunto único Taller Norte");

        // Legacy file without paired asuntos falls back cleanly to asunto_A / asunto_B.
        templates.email.retain(|k, _| !k.starts_with("asunto_"));
        templates.email.insert("asunto_A".into(), "Legado A".into());
        templates.email.insert("asunto_B".into(), "Legado B".into());
        templates.validate().unwrap();
        let legacy = first(
            &prospect,
            "email",
            &[],
            &settings,
            &templates,
            Profile::Automatizacion,
        )
        .unwrap();
        assert!(!legacy.asunto.is_empty());
    }
}
