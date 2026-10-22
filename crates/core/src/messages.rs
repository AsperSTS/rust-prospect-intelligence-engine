use crate::{Message, Profile, Prospect, Result, Settings, Signal, invalid};
use regex::{Captures, Regex};
use sha1::{Digest, Sha1};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::LazyLock,
};

#[derive(Clone, serde::Deserialize)]
pub struct Templates {
    pub whatsapp: BTreeMap<String, String>,
    pub email: BTreeMap<String, String>,
    pub seguimiento: BTreeMap<String, String>,
    pub giros: BTreeMap<String, BTreeMap<String, String>>,
}
impl Templates {
    pub fn load(home: &Path) -> Result<Self> {
        let path = home.join("templates.toml");
        let data = if path.exists() {
            std::fs::read_to_string(path)?
        } else {
            include_str!("../../../config/templates.toml").into()
        };
        let t: Self = toml::from_str(&data)?;
        t.validate()?;
        Ok(t)
    }
    pub fn validate(&self) -> Result<()> {
        for (map, keys) in [
            (&self.whatsapp, vec!["A", "B", "C", "D"]),
            (
                &self.email,
                vec![
                    "asunto_A",
                    "asunto_B",
                    "cuerpo_con_credencial",
                    "cuerpo_sin_credencial",
                ],
            ),
            (&self.seguimiento, vec!["d3", "d7", "d14"]),
        ] {
            if keys.iter().any(|k| !map.contains_key(*k)) {
                return Err(invalid("Faltan claves en las plantillas"));
            }
        }
        if !self.giros.contains_key("Otro")
            || self.giros.values().any(|g| {
                ["cosa", "pregunta", "ejemplo"]
                    .iter()
                    .any(|k| !g.contains_key(*k))
            })
        {
            return Err(invalid("Plantillas por giro incompletas"));
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
    let s = BRANCH.find(raw).map(|m| &raw[..m.start()]).unwrap_or(raw);
    let mut s = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', ',', '.', '-'])
        .to_owned();
    for _ in 0..3 {
        s = LEGAL
            .replace(&s, "")
            .trim_matches([' ', ',', '.', '-'])
            .into();
    }
    let mut words = Vec::new();
    for (i, w) in s.split(' ').enumerate() {
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
    let s = words.join(" ");
    if s.is_empty() {
        crate::rules::title(raw)
    } else {
        s
    }
}
fn hash_mod(id: &str, modulus: usize) -> usize {
    Sha1::digest(id.as_bytes())
        .iter()
        .fold(0usize, |r, b| (r * 256 + usize::from(*b)) % modulus)
}
pub fn variants(id: &str, evidence: bool) -> (String, bool, String) {
    let options = if evidence {
        vec!["A", "B", "D", "C"]
    } else {
        vec!["A", "B", "D"]
    };
    (
        options[hash_mod(id, options.len())].into(),
        hash_mod(id, 14) < 7,
        if hash_mod(id, 26) < 13 {
            "A".into()
        } else {
            "B".into()
        },
    )
}
fn vars(p: &Prospect, settings: &Settings, t: &Templates, obs: &str) -> BTreeMap<String, String> {
    let g = t.giros.get(&p.giro).unwrap_or(&t.giros["Otro"]);
    let mut v = g.clone();
    v.insert("nombre".into(), clean_name(&p.nombre));
    v.insert(
        "giro_corto".into(),
        if !p.giro_display.is_empty() {
            p.giro_display.clone()
        } else if !p.giro.is_empty() {
            p.giro.to_lowercase()
        } else {
            "su sector".into()
        },
    );
    v.insert("obs".into(), obs.into());
    v.insert("credencial".into(), settings.owner_credential.clone());
    v.insert("owner".into(), settings.owner_name.clone());
    v.insert(
        "owner_corto".into(),
        settings
            .owner_name
            .split_whitespace()
            .next()
            .unwrap_or("")
            .into(),
    );
    v.insert(
        "whatsapp".into(),
        crate::contacts::pretty_phone(&settings.owner_whatsapp),
    );
    v
}
fn render(s: &str, v: &BTreeMap<String, String>) -> String {
    static PLACEHOLDER: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{(\w+)\}").unwrap());
    PLACEHOLDER
        .replace_all(s, |c: &Captures| {
            v.get(&c[1]).cloned().unwrap_or_else(|| c[0].into())
        })
        .trim()
        .into()
}
pub fn first(
    p: &Prospect,
    channel: &str,
    signals: &[Signal],
    settings: &Settings,
    t: &Templates,
    profile: Profile,
) -> Result<Message> {
    if !["whatsapp", "email"].contains(&channel) {
        return Err(invalid("Canal de plantilla inválido"));
    }
    let obs = crate::rules::observation(signals);
    let (letter, credential, subject_letter) = variants(&p.id, obs.is_some());
    let v = vars(
        p,
        settings,
        t,
        obs.as_ref().map(|o| o.1.as_str()).unwrap_or(""),
    );
    let mut m = if profile == Profile::WebMarketing {
        let question =
            "¿Con quién podría revisar cómo reciben hoy consultas de nuevos clientes por internet?";
        if channel == "whatsapp" {
            Message {
                canal: channel.into(),
                variante: "WEB-WA-A".into(),
                cuerpo: format!(
                    "Hola, buen día. Soy {}. Vi que {} trabaja en {}. Desarrollo sitios web y herramientas para captar clientes. {question}",
                    v["owner"], v["nombre"], v["giro_corto"]
                ),
                asunto: String::new(),
                problemas: vec![],
            }
        } else {
            Message {
                canal: channel.into(),
                variante: "WEB-EM-A".into(),
                cuerpo: format!(
                    "Buen día,\n\nSoy {}. Desarrollo sitios web y herramientas para captar clientes. Vi a {} y quería hacerles una pregunta:\n\n{question}\n\nSi tiene sentido revisarlo, puedo mostrarles un ejemplo sencillo por este medio.\n\nSaludos,\n{}\nWhatsApp: {}",
                    v["owner"], v["nombre"], v["owner"], v["whatsapp"]
                ),
                asunto: if format!("Pregunta sobre el sitio de {}", v["nombre"])
                    .chars()
                    .count()
                    <= 60
                {
                    format!("Pregunta sobre el sitio de {}", v["nombre"])
                } else {
                    "Una pregunta sobre su sitio web".into()
                },
                problemas: vec![],
            }
        }
    } else if channel == "whatsapp" {
        Message {
            canal: channel.into(),
            variante: format!("WA-{letter}"),
            cuerpo: render(&t.whatsapp[&letter], &v),
            asunto: String::new(),
            problemas: vec![],
        }
    } else {
        let v = vars(p, settings, t, "");
        let other = if subject_letter == "A" { "B" } else { "A" };
        let candidates = [
            (
                subject_letter.clone(),
                render(&t.email[&format!("asunto_{subject_letter}")], &v),
            ),
            (
                other.into(),
                render(&t.email[&format!("asunto_{other}")], &v),
            ),
            ("S".into(), format!("Pregunta sobre {}", v["nombre"])),
            ("S".into(), "Una pregunta sobre sus procesos".into()),
        ];
        let (letra, subject) = candidates
            .iter()
            .find(|(_, s)| s.chars().count() <= 60)
            .unwrap_or(candidates.last().unwrap())
            .clone();
        Message {
            canal: channel.into(),
            variante: format!("EM-{}-{letra}", if credential { "con" } else { "sin" }),
            cuerpo: render(
                &t.email[if credential {
                    "cuerpo_con_credencial"
                } else {
                    "cuerpo_sin_credencial"
                }],
                &v,
            ),
            asunto: subject,
            problemas: vec![],
        }
    };
    m.problemas = validate(&m.cuerpo, &m.canal, &m.asunto, obs.is_some(), settings);
    Ok(m)
}
pub fn followup(
    p: &Prospect,
    step: i64,
    settings: &Settings,
    t: &Templates,
    profile: Profile,
) -> Message {
    let key = match step {
        1 => "d3",
        2 => "d7",
        _ => "d14",
    };
    let v = vars(p, settings, t, "");
    let text = if profile == Profile::Automatizacion {
        render(&t.seguimiento[key], &v)
    } else {
        match step{1=>"Hola nuevamente. Para aterrizar la idea, podría mostrarle un ejemplo de cómo presentar sus servicios y recibir consultas en una página sencilla. Si es útil, con gusto lo revisamos por este medio.",2=>"Buen día. Una forma simple de comenzar sería revisar cómo llegan hoy sus consultas y qué información necesitan sus clientes antes de escribirles. Si le sirve, puedo explicarle la idea por aquí.",_=>"Hola. Cierro el tema por aquí para no llenar su bandeja. Si más adelante quieren revisar su sitio web o la captación de consultas, con gusto puedo ayudarles."}.into()
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
            .map(|r| Regex::new(r).unwrap())
            .collect()
    });
    let mut errors = Vec::new();
    let low = text.to_lowercase();
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
    let n = text.split_whitespace().count();
    let max = match channel {
        "whatsapp" => 65,
        "email" => 120,
        _ => 80,
    };
    if n > max {
        errors.push(format!("Demasiado largo: {n} palabras (máx. {max})"));
    }
    for f in &CHECKS.prohibited {
        if low.contains(f) {
            errors.push(format!("Frase prohibida: «{f}»"));
        }
    }
    if CLAIMS.iter().any(|r| r.is_match(&low)) {
        errors.push("Afirmación no sustentada sobre la empresa".into());
    }
    if low.contains("vi en su sitio") && !evidence {
        errors.push("Cita el sitio web sin evidencia registrada".into());
    }
    if channel == "whatsapp" {
        let questions = text.matches('?').count();
        if questions != 1 {
            errors.push(format!(
                "Debe tener exactamente 1 pregunta (tiene {questions})"
            ));
        }
        if low.contains("http") || low.contains("www.") {
            errors.push("No incluyas enlaces en el primer mensaje".into());
        }
    }
    if channel == "email" {
        let n = subject.chars().count();
        if subject.is_empty() {
            errors.push("Falta el asunto".into());
        } else if n > 70 {
            errors.push(format!(
                "Asunto largo: {n} caracteres (máx. 70, ideal ≤ 60)"
            ));
        } else if n > 60 {
            errors.push(format!("Aviso: asunto de {n} caracteres (ideal ≤ 60)"));
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
