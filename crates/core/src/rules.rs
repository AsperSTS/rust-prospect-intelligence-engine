use crate::{Action, Analysis, Group, Profile, Prospect, ScoreItem, Signal};
use chrono::NaiveDate;
use regex::Regex;
use serde::Deserialize;
use std::{
    collections::{BTreeMap, HashSet},
    sync::LazyLock,
};
use unicode_categories::UnicodeCategories;
use unicode_normalization::UnicodeNormalization;

#[derive(Deserialize)]
struct Taxonomy {
    #[serde(rename = "GIRO_INFO")]
    info: BTreeMap<String, (i64, i64, String)>,
    #[serde(rename = "PREFIJOS")]
    prefixes: BTreeMap<String, (String, Option<String>)>,
    #[serde(rename = "BONO_COMPLEJIDAD")]
    bonus: BTreeMap<String, i64>,
}
#[derive(Deserialize)]
struct SignalsData {
    #[serde(rename = "SENALES")]
    types: BTreeMap<String, (String, String, bool)>,
    #[serde(rename = "ORDEN_OBS")]
    order: Vec<String>,
    #[serde(rename = "SIGNAL_ORDER")]
    all_order: Vec<String>,
}
static TAXONOMY: LazyLock<Taxonomy> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../config/giros.json")).expect("Embedded taxonomy")
});
static SIGNALS: LazyLock<SignalsData> = LazyLock::new(|| {
    serde_json::from_str(include_str!("../../../config/senales.json")).expect("Embedded signals")
});
static SIGNAL_REGEX: LazyLock<BTreeMap<String, Regex>> = LazyLock::new(|| {
    SIGNALS
        .types
        .iter()
        .map(|(k, (rx, _, _))| {
            (
                k.clone(),
                Regex::new(&format!("(?i){rx}")).expect("Embedded signal regex"),
            )
        })
        .collect()
});
#[derive(Clone, Debug)]
pub struct Giro {
    pub giro: String,
    pub display: String,
    pub fit: i64,
    pub complexity: i64,
    pub hypothesis: String,
}
pub fn classify(code: &str, name: &str) -> Giro {
    let entry = TAXONOMY
        .prefixes
        .iter()
        .filter(|(p, _)| code.starts_with(p.as_str()))
        .max_by_key(|(p, _)| p.len())
        .map(|(_, e)| e);
    let giro = entry.map(|e| e.0.as_str()).unwrap_or("Otro");
    let (fit, base, hypothesis) = &TAXONOMY.info[giro];
    let bonus = TAXONOMY
        .bonus
        .iter()
        .filter(|(p, _)| code.starts_with(p.as_str()))
        .max_by_key(|(p, _)| p.len())
        .map(|(_, v)| *v)
        .unwrap_or(0);
    Giro {
        giro: giro.into(),
        display: entry
            .and_then(|e| e.1.clone())
            .unwrap_or_else(|| display_from_class(name, giro)),
        fit: *fit,
        complexity: (base + bonus).min(20),
        hypothesis: hypothesis.clone(),
    }
}
pub fn display_from_class(name: &str, giro: &str) -> String {
    let s = name
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    static WHOLESALE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^comercio al por mayor (de|a)\s+").unwrap());
    let s = WHOLESALE.replace(&s, "distribución de ");
    static PREFIX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(fabricación|elaboración|servicios?) de\s+").unwrap());
    let s = PREFIX.replace(&s, "");
    let s = s
        .split(' ')
        .take(6)
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches([' ', ',', '.', ';'])
        .to_owned();
    if s.is_empty() { giro.to_lowercase() } else { s }
}
pub fn title(s: &str) -> String {
    let mut word = false;
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphabetic() {
            if word {
                out.extend(c.to_lowercase());
            } else {
                out.extend(c.to_uppercase());
            }
            word = true;
        } else {
            out.push(c);
            word = false;
        }
    }
    out
}
pub fn clean_text(s: &str) -> String {
    static ALLOWED: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"[^\w\s.,;:¿?¡!()/%&+\-–—\"'’“”…@#°ºª·$|*=<>\[\]{}~^_\\]"#).unwrap()
    });
    let decoded = html_escape::decode_html_entities(s);
    let markless: String = decoded
        .chars()
        .filter(|c| !matches!(c, '®' | '©' | '™' | '℠'))
        .collect();
    let normalized: String = markless
        .nfkc()
        .filter(|c| {
            (!c.is_other() || matches!(c, '\t' | '\n' | '\r'))
                && !matches!(c, '®' | '©' | '™' | '℠')
        })
        .map(|c| if c.is_whitespace() { ' ' } else { c })
        .collect();
    ALLOWED
        .replace_all(&normalized, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn without_menu(s: &str) -> String {
    static MENU: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"(?i)\b(skip to (main )?content|saltar al contenido|ir al contenido( principal)?|toggle (navigation|menu)|open menu|close menu|menu toggle|loading\.\.\.|cargando\.\.\.|aviso de privacidad|todos los derechos reservados|all rights reserved)\b").unwrap()
    });
    MENU.replace_all(s, " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn related(types: &[String]) -> Vec<String> {
    types
        .iter()
        .filter(|t| SIGNALS.types.get(*t).is_some_and(|v| v.2))
        .cloned()
        .collect()
}
pub fn signal_label(t: &str) -> String {
    SIGNALS
        .types
        .get(t)
        .map(|v| v.1.clone())
        .unwrap_or_else(|| t.to_owned())
}
pub fn observation(rows: &[Signal]) -> Option<(String, String)> {
    let valid: HashSet<&str> = rows
        .iter()
        .filter(|s| !s.signal_source_url.is_empty() && !s.signal_text.is_empty())
        .map(|s| s.signal_type.as_str())
        .collect();
    SIGNALS
        .order
        .iter()
        .find(|t| valid.contains(t.as_str()))
        .map(|t| {
            (
                t.clone(),
                match t.as_str() {
                    "sobre_pedido" => "Vi en su sitio que fabrican sobre pedido.".into(),
                    "cotizacion" => "Vi en su sitio que atienden por cotización.".into(),
                    "servicio_tecnico" => "Vi en su sitio que ofrecen servicio técnico.".into(),
                    _ => format!("Vi en su sitio que mencionan {}.", signal_label(t)),
                },
            )
        })
}
fn fragment_quality(s: &str) -> f64 {
    static WORDS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[^\W\d_]{2,}").unwrap());
    static CAPITALS: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:\b[A-ZÁÉÍÓÚÑ][\wáéíóúñ]+\b\s*){3,}").unwrap());
    let words = WORDS.find_iter(s).collect::<Vec<_>>();
    if words.len() < 5 {
        return 0.0;
    }
    let lower = words
        .iter()
        .filter(|m| m.as_str().chars().next().is_some_and(char::is_lowercase))
        .count();
    let caps = CAPITALS
        .find_iter(s)
        .map(|m| m.as_str().split_whitespace().count())
        .max()
        .unwrap_or(0);
    lower as f64 / words.len() as f64 + words.len().min(14) as f64 / 28.0 - 0.15 * caps as f64
}
pub fn detect(text: &str, url: &str, seen: &mut HashSet<String>) -> Vec<Signal> {
    let text = clean_text(text);
    let chars = text.chars().collect::<Vec<_>>();
    let mut out = Vec::new();
    for kind in &SIGNALS.all_order {
        let rx = &SIGNAL_REGEX[kind];
        if seen.contains(kind) {
            continue;
        }
        let mut best: Option<(f64, String)> = None;
        for hit in rx.find_iter(&text).take(8) {
            let start = text[..hit.start()].chars().count();
            let end = start + hit.as_str().chars().count();
            let mut a = start.saturating_sub(70);
            let mut b = (end + 70).min(chars.len());
            while a > 0 && a < start && chars[a - 1] != ' ' {
                a += 1;
            }
            while b > end && b < chars.len() && chars[b] != ' ' {
                b -= 1;
            }
            let frag = without_menu(&chars[a..b].iter().collect::<String>());
            let q = fragment_quality(&frag);
            if best.as_ref().is_none_or(|(old, _)| q > *old) {
                best = Some((
                    q,
                    format!(
                        "{}{}{}",
                        if a > 0 { "…" } else { "" },
                        frag,
                        if b < chars.len() { "…" } else { "" }
                    ),
                ));
            }
        }
        if let Some((_, evidence)) = best {
            out.push(Signal {
                signal_type: kind.clone(),
                signal_text: evidence,
                signal_source_url: url.into(),
            });
            seen.insert(kind.clone());
        }
    }
    out
}
fn item(k: &str, label: &str, points: i64, max: i64, note: impl Into<String>) -> ScoreItem {
    ScoreItem {
        clave: k.into(),
        etiqueta: label.into(),
        pts: points,
        maximum: max,
        nota: note.into(),
    }
}
fn corridor(name: &str, kind: &str) -> (i64, String) {
    let n = name.trim().to_lowercase();
    let t = kind.trim().to_lowercase();
    let industrial = [
        "parque industrial",
        "zona industrial",
        "ciudad industrial",
        "corredor industrial",
    ]
    .contains(&t.as_str());
    let named =
        n.contains("industrial") || n.starts_with("parque ") || n.contains("ciudad industrial");
    let ok = !["sin nombre", "sin nombre.", ""].contains(&n.as_str());
    if industrial && ok {
        (15, format!("{}: {}", title(kind), title(name)))
    } else if industrial || named {
        (
            10,
            if !ok {
                format!(
                    "{} (sin nombre específico)",
                    if t.is_empty() {
                        "Zona industrial".into()
                    } else {
                        title(kind)
                    }
                )
            } else {
                format!(
                    "{} ({})",
                    title(name),
                    if t.is_empty() {
                        "nombre indica zona industrial".into()
                    } else {
                        title(kind)
                    }
                )
            },
        )
    } else if !t.is_empty() || !n.is_empty() {
        (
            0,
            format!(
                "{}: no industrial",
                title(if t.is_empty() { name } else { kind })
            ),
        )
    } else {
        (0, "sin dato".into())
    }
}
pub fn fit(p: &Prospect, g: &Giro, types: &[String]) -> (i64, Vec<ScoreItem>) {
    let size = match p.estrato.as_str() {
        "11 a 30 personas" => 14,
        "31 a 50 personas" | "51 a 100 personas" => 20,
        _ => 4,
    };
    let (kind, note) = match p.tipo_establecimiento.trim().to_lowercase().as_str() {
        "fijo" => (10, "fijo".into()),
        "" => (4, "sin dato".into()),
        s => (0, format!("{s}: poco compatible")),
    };
    let (corr, note_corr) = corridor(&p.corredor_nombre, &p.corredor_tipo);
    let rel = related(types)
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let rel_note = if rel.is_empty() {
        "ninguna".into()
    } else {
        rel.iter()
            .map(|t| signal_label(t))
            .collect::<Vec<_>>()
            .join(", ")
    };
    let items = vec![
        item("giro", "Giro", g.fit, 25, &g.giro),
        item("estrato", "Tamaño (estrato)", size, 20, &p.estrato),
        item("tipo", "Tipo de establecimiento", kind, 10, note),
        item("corredor", "Corredor industrial", corr, 15, note_corr),
        item(
            "complejidad",
            "Complejidad operativa probable",
            g.complexity,
            20,
            "regla por giro/SCIAN, no un hecho",
        ),
        item(
            "senales",
            "Señales web relacionadas",
            (3 * rel.len() as i64).min(10),
            10,
            rel_note,
        ),
    ];
    (items.iter().map(|i| i.pts).sum(), items)
}
fn days_since(s: &str, today: NaiveDate) -> Option<i64> {
    s.get(..10)
        .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok())
        .map(|d| (today - d).num_days())
}
pub fn priority(
    p: &Prospect,
    fit: i64,
    types: &[String],
    group_contacted: bool,
    today: NaiveDate,
) -> (i64, Vec<ScoreItem>, bool) {
    let state = if p.estado.is_empty() {
        "nuevo"
    } else {
        &p.estado
    };
    if crate::config::CLOSED.contains(&state) {
        return (
            0,
            vec![item(
                "estado",
                "Estado",
                0,
                0,
                format!("{state}: fuera de la cola"),
            )],
            true,
        );
    }
    let email =
        crate::contacts::email_valid(&p.correo) || crate::contacts::email_valid(&p.correo_web);
    let tel = !p.phone().is_empty();
    let web = !p.sitio.trim().is_empty();
    let n = i64::from(email) + i64::from(tel) + i64::from(web);
    let contact = if email { 10 } else { 0 }
        + if tel { 10 } else { 0 }
        + if web { 5 } else { 0 }
        + if n >= 2 { 5 } else { 0 };
    let distinct = types.iter().collect::<HashSet<_>>();
    let context =
        if distinct.is_empty() { 0 } else { 10 } + if related(types).is_empty() { 0 } else { 10 };
    let mut op = 0;
    let mut reasons = Vec::new();
    let due = !p.proximo_seguimiento.is_empty() && p.proximo_seguimiento <= today.to_string();
    match state {
        "nuevo" => {
            op += 5;
            reasons.push("nuevo, sin revisar".to_owned());
        }
        "contactado" => {
            if due {
                op += 15;
                reasons.push("seguimiento vencido".into());
            } else {
                op -= 8;
                reasons.push("contactado, en secuencia".into());
            }
        }
        "respondio" | "respuesta_positiva" => {
            op += 15;
            reasons.push("respondió: requiere acción".into());
        }
        "reunion" | "propuesta" => {
            op += 10;
            reasons.push(format!("{state}: requiere acción"));
        }
        _ => {}
    }
    if matches!(state, "nuevo" | "revisado") && p.possible_chain != 0 && group_contacted {
        op -= 15;
        reasons.push("posible grupo ya contactado".into());
    }
    if n == 0 {
        op -= 10;
        reasons.push("sin canal de contacto".into());
    }
    op = op.clamp(-20, 15);
    let fresh = if days_since(&p.website_checked_at, today).is_some_and(|d| d <= 30) {
        3
    } else {
        0
    } + if days_since(&p.creado_en, today).is_some_and(|d| d <= 14) {
        2
    } else {
        0
    };
    let weighted = (fit as f64 * 0.35).round_ties_even() as i64;
    let mut items = vec![
        item(
            "fit",
            "Fit ponderado",
            weighted,
            35,
            format!("fit {fit} × 0.35"),
        ),
        item(
            "contacto",
            "Contacto",
            contact,
            30,
            "email · teléfono · web · multicanal",
        ),
        item(
            "contexto",
            "Contexto web",
            context,
            20,
            "señal concreta + señal relacionada",
        ),
        item(
            "estado",
            "Estado operativo",
            op,
            15,
            if reasons.is_empty() {
                "sin ajuste".into()
            } else {
                reasons.join("; ")
            },
        ),
        item(
            "frescura",
            "Frescura",
            fresh,
            5,
            "enriquecimiento ≤30 días / registro ≤14 días",
        ),
    ];
    let raw: i64 = items.iter().map(|i| i.pts).sum();
    let total = raw.clamp(0, 100);
    if raw != total {
        items.push(item(
            "limite",
            "Límite de la escala",
            total - raw,
            (total - raw).abs(),
            format!("Ajuste del subtotal {raw} al rango 0–100"),
        ));
    }
    (total, items, false)
}
pub fn next_action(p: &Prospect, fit: i64, evidence: bool, today: NaiveDate) -> Action {
    let email = !p.email().is_empty();
    let tel = !p.phone().is_empty();
    let alternatives = if tel {
        vec!["whatsapp_or_copy".into()]
    } else {
        vec![]
    };
    let make = |action: &str, reason: String, confidence: &str, alt: Option<Vec<String>>| Action {
        action: action.into(),
        reason,
        confidence: confidence.into(),
        alternativas: alt.unwrap_or_else(|| alternatives.clone()),
    };
    let state = if p.estado.is_empty() {
        "nuevo"
    } else {
        &p.estado
    };
    if p.closed() {
        return make(
            "skip",
            format!("Prospecto cerrado ({state})."),
            "alta",
            Some(vec![]),
        );
    }
    if state == "contactado" {
        return if !p.proximo_seguimiento.is_empty() && p.proximo_seguimiento <= today.to_string() {
            make(
                "follow_up",
                format!(
                    "Seguimiento vencido (D+{}).",
                    match p.toques {
                        1 => "3",
                        2 => "7",
                        3 => "14",
                        _ => "?",
                    }
                ),
                "alta",
                None,
            )
        } else {
            make(
                "skip",
                format!(
                    "En secuencia; próximo seguimiento {}.",
                    if p.proximo_seguimiento.is_empty() {
                        "None"
                    } else {
                        &p.proximo_seguimiento
                    }
                ),
                "alta",
                Some(vec![]),
            )
        };
    }
    if ["respondio", "respuesta_positiva", "reunion", "propuesta"].contains(&state) {
        return make(
            "manual_review",
            "Hay conversación abierta: contesta manualmente, sin mensajes automáticos.".into(),
            "alta",
            Some(vec![]),
        );
    }
    if p.possible_chain != 0 {
        return make(
            "manual_review",
            format!(
                "Posible grupo/cadena ({}): valida si decide localmente.",
                if p.group_reason.is_empty() {
                    "sin motivo"
                } else {
                    &p.group_reason
                }
            ),
            "media",
            None,
        );
    }
    if p.giro == "Otro" {
        return make(
            "manual_review",
            "SCIAN poco claro: confirma que el giro encaja.".into(),
            "media",
            None,
        );
    }
    let name = p.nombre.to_lowercase();
    let tokens = name
        .split_whitespace()
        .filter(|t| !["de", "del", "la", "el", "los", "las", "y"].contains(t))
        .collect::<Vec<_>>();
    if tokens.len() <= 1
        && (tokens.is_empty()
            || [
                "bodega",
                "taller",
                "ferreteria",
                "abarrotes",
                "comercializadora",
                "distribuidora",
                "transportes",
                "construcciones",
                "servicios",
            ]
            .contains(&tokens[0]))
    {
        return make(
            "manual_review",
            "Nombre genérico: confirma que es la empresa correcta.".into(),
            "baja",
            None,
        );
    }
    if !email && !tel {
        return if fit >= 50 {
            make(
                "manual_review",
                "Fit alto pero sin canal de contacto claro: búscalo en su sitio o redes.".into(),
                "media",
                Some(vec![]),
            )
        } else {
            make(
                "skip",
                "Sin canal de contacto y fit bajo: datos insuficientes.".into(),
                "media",
                Some(vec![]),
            )
        };
    }
    if fit < 50 {
        return make(
            "skip",
            format!("Fit {fit} por debajo de 50: no conviene gastar el contacto de hoy."),
            "media",
            None,
        );
    }
    if email {
        return make(
            "email",
            format!(
                "Email válido + fit alto{}",
                if evidence {
                    " + contexto web concreto"
                } else {
                    ""
                }
            ),
            if evidence { "alta" } else { "media" },
            None,
        );
    }
    make(
        "telefono",
        "Teléfono disponible, fit alto y sin email. La herramienta no llama por ti.".into(),
        "media",
        None,
    )
}
pub fn analyze(
    p: &Prospect,
    signals: &[Signal],
    group: Group,
    contacted: bool,
    today: NaiveDate,
    profile: Profile,
) -> Analysis {
    let mut row = p.clone();
    row.group_key = group.group_key.clone();
    row.group_reason = group.group_reason.clone();
    row.possible_chain = group.possible_chain;
    let mut g = classify(&p.clase_id, &p.clase_actividad);
    row.giro = g.giro.clone();
    row.giro_display = g.display.clone();
    let types = signals
        .iter()
        .map(|s| s.signal_type.clone())
        .collect::<Vec<_>>();
    let (fit, fit_items) = if profile == Profile::Automatizacion {
        fit(&row, &g, &types)
    } else {
        let sector = if ["46", "72", "54", "81", "61", "62", "71"]
            .iter()
            .any(|x| p.clase_id.starts_with(x))
        {
            30
        } else {
            18
        };
        let size = if p.estrato.starts_with("0 a 5") || p.estrato.starts_with("6 a 10") {
            20
        } else {
            15
        };
        let has_web = !p.sitio.is_empty();
        let items = vec![
            item(
                "giro",
                "Compatibilidad comercial",
                sector,
                30,
                &p.clase_actividad,
            ),
            item("estrato", "Tamaño", size, 20, &p.estrato),
            item(
                "contacto",
                "Canal disponible",
                if p.contactable() { 25 } else { 0 },
                25,
                "formato; requiere verificar",
            ),
            item(
                "web",
                "Presencia digital por verificar",
                if has_web { 15 } else { 5 },
                15,
                if has_web {
                    "sitio declarado; revisar necesidad"
                } else {
                    "sitio desconocido, no ausencia comprobada"
                },
            ),
            item(
                "estado",
                "Registro revisado",
                if p.estado != "nuevo" { 10 } else { 0 },
                10,
                "regla inicial de trabajo",
            ),
        ];
        g.hypothesis =
            "podría ser útil revisar cómo reciben consultas y presentan sus servicios en línea."
                .into();
        g.giro = if sector == 30 {
            "Comercio y servicios".into()
        } else {
            g.giro
        };
        (items.iter().map(|i| i.pts).sum(), items)
    };
    row.giro = g.giro.clone();
    row.giro_display = g.display.clone();
    let (priority, priority_items, _) = priority(&row, fit, &types, contacted, today);
    let rel = related(&types)
        .iter()
        .take(3)
        .map(|s| signal_label(s))
        .collect::<Vec<_>>();
    let (hyp, basis, confidence) = if profile == Profile::WebMarketing {
        (
            format!("Por el giro ({}), {}", g.display, g.hypothesis),
            "Regla de oferta web/marketing; requiere diagnóstico".into(),
            "baja".into(),
        )
    } else if rel.is_empty() {
        (
            format!("Por el giro ({}), {}", g.display, g.hypothesis),
            "Solo giro SCIAN; sin evidencia web".into(),
            "baja".into(),
        )
    } else {
        let words = rel.join(", ");
        (
            format!(
                "Por el giro ({}) y porque el sitio menciona {words}, {}",
                g.display, g.hypothesis
            ),
            format!("Giro SCIAN + evidencia web: {words}"),
            if rel.len() >= 3 {
                "alta".into()
            } else {
                "media".into()
            },
        )
    };
    let action = next_action(&row, fit, !types.is_empty(), today);
    Analysis {
        giro: g.giro,
        giro_display: g.display,
        fit_score: fit,
        fit_breakdown: fit_items,
        priority_score: priority,
        priority_breakdown: priority_items,
        hypothesis_text: hyp,
        hypothesis_basis: basis,
        hypothesis_confidence: confidence,
        action,
        group,
    }
}
