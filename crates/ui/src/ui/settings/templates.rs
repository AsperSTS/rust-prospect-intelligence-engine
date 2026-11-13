//! Visual message-template editor: visible category tabs, tokenized variant previews
//! (no bracket clutter), single-surface rich block canvas, and live preview underneath.
use super::*;
use prospect_core::messages::{RESERVED_KEYS, data_key, email_subject_key_for_body};
use std::collections::BTreeMap;
use std::rc::Rc;

struct TagMeta {
    key: &'static str,
    label: &'static str,
    own: bool,
    description: &'static str,
    fallback: &'static str,
}

/// Application data available to every template.
const BUILTIN_TAGS: [TagMeta; 13] = [
    TagMeta {
        key: "nombre",
        label: "Nombre de la empresa",
        own: false,
        description: "El nombre comercial del negocio que vas a contactar (sin «S.A. de C.V.» ni sucursal).",
        fallback: "Taller Industrial del Valle",
    },
    TagMeta {
        key: "giro_corto",
        label: "Giro",
        own: false,
        description: "A qué se dedica el negocio en lenguaje natural para que encaje en la frase.",
        fallback: "manufactura metálica",
    },
    TagMeta {
        key: "cosa",
        label: "Proceso clave",
        own: false,
        description: "Actividad diaria típica de ese negocio donde suele haber trabajo manual (se edita en Giros).",
        fallback: "órdenes y refacciones",
    },
    TagMeta {
        key: "pregunta",
        label: "Pregunta",
        own: false,
        description: "Pregunta concreta para iniciar conversación según el tipo de negocio (se edita en Giros).",
        fallback: "¿Cómo llevan el seguimiento de las órdenes desde que entran hasta que se entregan?",
    },
    TagMeta {
        key: "ejemplo",
        label: "Ejemplo de mejora",
        own: false,
        description: "Idea práctica de cómo simplificar esa operación, muy útil en mensajes de seguimiento.",
        fallback: "un control de órdenes, estatus y refacciones asociadas",
    },
    TagMeta {
        key: "obs",
        label: "Hallazgo web",
        own: false,
        description: "Nota breve tomada automáticamente si se encontró algún detalle útil en su página web.",
        fallback: "Vi en su sitio web que cotizan por correo.",
    },
    TagMeta {
        key: "owner",
        label: "Nombre completo",
        own: true,
        description: "Tu nombre completo guardado en Perfil; sirve para presentarte y firmar tus correos.",
        fallback: "Sofía Méndez",
    },
    TagMeta {
        key: "owner_corto",
        label: "Primer nombre",
        own: true,
        description: "Sólo tu primer nombre (tomado automático de Perfil) para saludos más cercanos.",
        fallback: "Sofía",
    },
    TagMeta {
        key: "credencial",
        label: "Diferenciador",
        own: true,
        description: "Tu frase corta de Perfil que explica qué problema resuelves o qué te hace diferente.",
        fallback: "Automatizo reportes y órdenes para pymes industriales.",
    },
    TagMeta {
        key: "whatsapp",
        label: "Número WhatsApp",
        own: true,
        description: "Tu número de WhatsApp configurado en Perfil para el cierre del mensaje o correo.",
        fallback: "55 0123 4567",
    },
    TagMeta {
        key: "portafolio",
        label: "Portafolio",
        own: true,
        description: "Enlace a tu GitHub o portafolio (configurado en Perfil) para mostrar trabajos reales.",
        fallback: "https://github.com/sofiamendez",
    },
    TagMeta {
        key: "linkedin",
        label: "LinkedIn",
        own: true,
        description: "Enlace a tu perfil de LinkedIn (configurado en Perfil) para dar confianza profesional.",
        fallback: "https://linkedin.com/in/sofiamendez",
    },
    TagMeta {
        key: "web",
        label: "Página web",
        own: true,
        description: "Enlace a tu sitio web (configurado en Perfil) por si quieren ver más de lo que haces.",
        fallback: "https://sofiamendez.dev",
    },
];

fn tag_meta(key: &str) -> &'static TagMeta {
    BUILTIN_TAGS
        .iter()
        .find(|m| m.key == key)
        .expect("builtin tag key must exist")
}

const CATEGORIES: [&str; 4] = ["WhatsApp", "Correos", "Seguimientos", "Giros"];

const REQUIRED: [&str; 9] = [
    "A",
    "B",
    "C",
    "D",
    "cuerpo_con_credencial",
    "cuerpo_sin_credencial",
    "d3",
    "d7",
    "d14",
];

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum EditorField {
    Subject,
    Body,
}

#[derive(Clone, PartialEq)]
struct ResolvedTag {
    key: String,
    label: String,
    own: bool,
    description: &'static str,
    sample: String,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum RichPart {
    Text(String),
    Tag { label: String, own: bool },
}

#[derive(Clone, PartialEq)]
struct Item {
    key: String,
    title_parts: Vec<RichPart>,
    plain_title: String,
    hint: String,
    excerpt_parts: Vec<RichPart>,
}

/// (`{key}`, `[Etiqueta]`) for built-in data plus the user's reusable data.
pub(super) fn tag_pairs(datos: &BTreeMap<String, String>) -> Vec<(String, String)> {
    BUILTIN_TAGS
        .iter()
        .map(|meta| (format!("{{{}}}", meta.key), format!("[{}]", meta.label)))
        .chain(
            datos
                .keys()
                .map(|label| (format!("{{{}}}", data_key(label)), format!("[{label}]"))),
        )
        .collect()
}

pub(super) fn to_friendly_text(raw: &str, pairs: &[(String, String)]) -> String {
    pairs.iter().fold(raw.to_owned(), |text, (key, label)| {
        text.replace(key, label)
    })
}

pub(super) fn from_friendly_text(friendly: &str, pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .fold(friendly.to_owned(), |text, (key, label)| {
            text.replace(label, key)
        })
}

/// Applies one change to the template draft; the draft stays untouched when it would be invalid.
pub(super) fn mutate(
    mut tpl: Signal<String>,
    mut error: Signal<String>,
    change: impl FnOnce(&mut Templates),
) {
    let Ok(mut parsed) = Templates::parse_toml(&tpl.peek()) else {
        error.set("No se pudo actualizar la plantilla actual.".into());
        return;
    };
    change(&mut parsed);
    match parsed.validate().and_then(|()| parsed.to_toml()) {
        Ok(serialized) => {
            tpl.set(serialized);
            error.set(String::new());
        }
        Err(e) => error.set(e.to_string()),
    }
}

fn effective_owner(owner_name: &str) -> String {
    let trimmed = owner_name.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("Tu nombre") {
        tag_meta("owner").fallback.to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn resolved_tag(key: &str, t: &Templates, settings: &Settings) -> Option<ResolvedTag> {
    let pick = |raw: &str, fallback: &str| {
        if raw.trim().is_empty() {
            fallback.to_owned()
        } else {
            raw.trim().to_owned()
        }
    };
    if let Some(meta) = BUILTIN_TAGS.iter().find(|m| m.key == key) {
        let sample = match meta.key {
            "owner" => effective_owner(&settings.owner_name),
            "owner_corto" => {
                let full = effective_owner(&settings.owner_name);
                full.split_whitespace().next().unwrap_or(&full).to_owned()
            }
            "credencial" => pick(&settings.owner_credential, meta.fallback),
            "whatsapp" => pick(&settings.owner_whatsapp, meta.fallback),
            "portafolio" => pick(&settings.owner_portfolio, meta.fallback),
            "linkedin" => pick(&settings.owner_linkedin, meta.fallback),
            "web" => pick(&settings.owner_website, meta.fallback),
            _ => meta.fallback.to_owned(),
        };
        return Some(ResolvedTag {
            key: meta.key.to_owned(),
            label: meta.label.to_owned(),
            own: meta.own,
            description: meta.description,
            sample,
        });
    }
    t.datos
        .iter()
        .find(|(label, _)| data_key(label) == key)
        .map(|(label, val)| ResolvedTag {
            key: key.to_owned(),
            label: label.clone(),
            own: true,
            description: "Dato propio guardado en Perfil para reutilizar en cualquier plantilla.",
            sample: pick(val, label),
        })
}

fn palette_tags(t: &Templates, settings: &Settings) -> Vec<ResolvedTag> {
    BUILTIN_TAGS
        .iter()
        .filter_map(|m| resolved_tag(m.key, t, settings))
        .chain(
            t.datos
                .keys()
                .filter_map(|label| resolved_tag(&data_key(label), t, settings)),
        )
        .collect()
}

fn labels_ownership_map(tags: &[ResolvedTag]) -> BTreeMap<String, bool> {
    tags.iter()
        .map(|tag| (tag.label.clone(), tag.own))
        .collect()
}

fn keys_label_map(tags: &[ResolvedTag]) -> BTreeMap<String, (String, bool)> {
    tags.iter()
        .map(|tag| (tag.key.clone(), (tag.label.clone(), tag.own)))
        .collect()
}

type FriendlyTag = (String, bool);
type FriendlySpan = (usize, usize, Option<FriendlyTag>);

/// Splits a friendly string into alternating byte spans `[start..end]`, tagging recognized
/// `[Label]` tokens while keeping exact UTF-8 byte offsets aligned 1:1 with the `<textarea>`.
fn parse_friendly_spans(friendly: &str, labels: &BTreeMap<String, bool>) -> Vec<FriendlySpan> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut scan = 0usize;
    while let Some(rel_open) = friendly[scan..].find('[') {
        let open = scan + rel_open;
        let after = open + 1;
        let Some(rel_close) = friendly[after..].find(']') else {
            break;
        };
        let close = after + rel_close;
        let candidate = &friendly[after..close];
        if !candidate.contains(['[', '\n', '\r'])
            && let Some(&own) = labels.get(candidate)
        {
            if open > cursor {
                out.push((cursor, open, None));
            }
            out.push((open, close + 1, Some((candidate.to_owned(), own))));
            cursor = close + 1;
            scan = close + 1;
        } else {
            scan = after;
        }
    }
    if cursor < friendly.len() {
        out.push((cursor, friendly.len(), None));
    }
    out
}

fn common_prefix_bytes(a: &str, b: &str) -> usize {
    let mut idx = 0;
    for (ca, cb) in a.chars().zip(b.chars()) {
        if ca != cb {
            break;
        }
        idx += ca.len_utf8();
    }
    idx
}

fn common_suffix_bytes(a: &str, b: &str) -> usize {
    let mut idx = 0;
    for (ca, cb) in a.chars().rev().zip(b.chars().rev()) {
        if ca != cb {
            break;
        }
        idx += ca.len_utf8();
    }
    idx
}

fn collapse_join_spaces(left: &str, right: &str) -> String {
    if left.ends_with(' ') && right.starts_with(' ') {
        format!("{left}{}", &right[1..])
    } else if left.is_empty() && right.starts_with(' ') {
        right[1..].to_owned()
    } else {
        format!("{left}{right}")
    }
}

/// Keeps word-boundary spacing clean around newly inserted/dropped tokens without altering
/// existing prose or tight punctuation (`,`, `.`, `:`, `?`, `!`).
fn ensure_new_token_spacing(
    friendly: &str,
    edit_start: usize,
    edit_end: usize,
    labels: &BTreeMap<String, bool>,
) -> String {
    let spans = parse_friendly_spans(friendly, labels);
    let mut out = String::with_capacity(friendly.len() + 4);
    for (start, end, tag) in spans {
        let chunk = &friendly[start..end];
        if tag.is_some() && end > edit_start && start < edit_end {
            if let Some(prev_ch) = out.chars().next_back()
                && (prev_ch.is_alphanumeric() || prev_ch == ']')
            {
                out.push(' ');
            }
            out.push_str(chunk);
            if let Some(next_ch) = friendly[end..].chars().next()
                && (next_ch.is_alphanumeric() || next_ch == '[')
            {
                out.push(' ');
            }
        } else {
            out.push_str(chunk);
        }
    }
    out
}

/// Enforces atomic block semantics on a single continuous `<textarea>`:
/// - Deleting any character inside/touching an intact `[Label]` removes the entire block once.
/// - Typing inside an intact `[Label]` pushes the new text just outside the block cleanly.
/// - Dropping/pasting a new `[Label]` between words keeps single-space separation intact.
fn normalize_block_edit(
    prev_friendly: &str,
    typed_raw: &str,
    pairs: &[(String, String)],
    labels: &BTreeMap<String, bool>,
) -> String {
    let typed = to_friendly_text(typed_raw, pairs);
    if typed == prev_friendly {
        return typed;
    }

    let prefix = common_prefix_bytes(prev_friendly, &typed);
    let suffix = common_suffix_bytes(&prev_friendly[prefix..], &typed[prefix..]);
    let prev_end = prev_friendly.len() - suffix;
    let typed_end = typed.len() - suffix;
    let typed_mid = &typed[prefix..typed_end];
    let prev_spans = parse_friendly_spans(prev_friendly, labels);

    if typed_mid.is_empty() && prefix < prev_end {
        for (ts, te, tag) in &prev_spans {
            if tag.is_some() && prefix >= *ts && prev_end <= *te {
                return collapse_join_spaces(&prev_friendly[..*ts], &prev_friendly[*te..]);
            }
        }
    }

    if prefix == prev_end && !typed_mid.is_empty() && !typed_mid.contains('\n') {
        for (ts, te, tag) in &prev_spans {
            if tag.is_some() && prefix > *ts && prefix < *te {
                let left_half = prefix - *ts <= (*te - *ts) / 2;
                let cut = if left_half { *ts } else { *te };
                return format!(
                    "{}{typed_mid}{}",
                    &prev_friendly[..cut],
                    &prev_friendly[cut..]
                );
            }
        }
    }

    let contains_new_tag = parse_friendly_spans(typed_mid, labels)
        .iter()
        .any(|(_, _, tag)| tag.is_some())
        || parse_friendly_spans(&typed, labels)
            .iter()
            .any(|(s, e, tag)| tag.is_some() && *e > prefix && *s < typed_end);
    if contains_new_tag {
        return ensure_new_token_spacing(&typed, prefix, typed_end, labels);
    }
    typed
}

fn append_token_spacing(existing: &str, label: &str) -> String {
    let token = format!("[{label}]");
    if existing.is_empty() || existing.ends_with([' ', '\n', '\r', '(', '¿', '¡']) {
        format!("{existing}{token}")
    } else {
        format!("{existing} {token}")
    }
}

fn raw_to_rich_parts(raw: &str, tags_by_key: &BTreeMap<String, (String, bool)>) -> Vec<RichPart> {
    let mut parts = Vec::new();
    let mut rest = raw;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let key = &after[..close];
        if !key.contains(['{', '\n', '\r'])
            && let Some((label, own)) = tags_by_key.get(key)
        {
            if open > 0 {
                parts.push(RichPart::Text(rest[..open].to_owned()));
            }
            parts.push(RichPart::Tag {
                label: label.clone(),
                own: *own,
            });
            rest = &after[close + 1..];
        } else {
            push_rich_text(&mut parts, &rest[..open + 1]);
            rest = after;
        }
    }
    if !rest.is_empty() {
        push_rich_text(&mut parts, rest);
    }
    parts
}

fn push_rich_text(parts: &mut Vec<RichPart>, text: &str) {
    if let Some(RichPart::Text(prev)) = parts.last_mut() {
        prev.push_str(text);
    } else {
        parts.push(RichPart::Text(text.to_owned()));
    }
}

fn compact_whitespace_run(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(ch);
            prev_space = false;
        }
    }
    out
}

/// Builds inline visual parts for sidebar previews and email titles with zero `[...]` brackets,
/// truncating cleanly on word or tag boundaries.
fn clip_rich_parts(
    raw: &str,
    tags_by_key: &BTreeMap<String, (String, bool)>,
    max_chars: usize,
) -> Vec<RichPart> {
    let raw_parts = raw_to_rich_parts(raw, tags_by_key);
    let mut out: Vec<RichPart> = Vec::new();
    let mut used = 0usize;
    let total = raw_parts.len();
    for (idx, part) in raw_parts.into_iter().enumerate() {
        match part {
            RichPart::Text(txt) => {
                let mut clean = compact_whitespace_run(&txt);
                if idx == 0 {
                    clean = clean.trim_start().to_owned();
                }
                if idx + 1 == total {
                    clean = clean.trim_end().to_owned();
                }
                if clean.is_empty() {
                    continue;
                }
                let chars: Vec<char> = clean.chars().collect();
                if used + chars.len() <= max_chars {
                    used += chars.len();
                    push_rich_text(&mut out, &clean);
                } else {
                    let room = max_chars.saturating_sub(used);
                    if room > 0 {
                        let snippet: String = chars.into_iter().take(room).collect();
                        let trimmed = snippet.trim_end();
                        if !trimmed.is_empty() {
                            push_rich_text(&mut out, trimmed);
                        }
                    }
                    push_rich_text(&mut out, "…");
                    return out;
                }
            }
            RichPart::Tag { label, own } => {
                let len = label.chars().count();
                if !out.is_empty() && used + len > max_chars {
                    if let Some(RichPart::Text(last)) = out.last_mut() {
                        *last = format!("{}…", last.trim_end());
                    } else {
                        out.push(RichPart::Text("…".into()));
                    }
                    return out;
                }
                used += len;
                out.push(RichPart::Tag { label, own });
            }
        }
    }
    out
}

fn plain_title_from_parts(parts: &[RichPart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            RichPart::Text(t) => t.as_str(),
            RichPart::Tag { label, .. } => label.as_str(),
        })
        .collect::<String>()
        .trim()
        .to_owned()
}

fn email_subject_of(t: &Templates, body_key: &str) -> String {
    let paired = email_subject_key_for_body(body_key);
    if let Some(val) = t.email.get(&paired) {
        return val.clone();
    }
    if let Some(num) = body_key.rsplit('_').next()
        && let Some(val) = t.email.get(&format!("asunto_{num}"))
    {
        return val.clone();
    }
    let fallback_key = if body_key.starts_with("cuerpo_sin_") {
        "asunto_B"
    } else {
        "asunto_A"
    };
    t.email
        .get(fallback_key)
        .cloned()
        .unwrap_or_else(|| "Consulta sobre {cosa}".into())
}

fn whatsapp_title(key: &str, idx: usize) -> &'static str {
    match key {
        "A" => "Quién lleva el área",
        "B" => "Duda directa de proceso",
        "C" => "Con hallazgo del sitio web",
        "D" => "Con valor agregado",
        "E" => "Consulta por giro",
        "F" => "Consulta breve y directa",
        "G" => "Software sin suposiciones",
        "H" => "Proceso concreto del giro",
        "I" => "Confirmar responsable",
        "J" => "Quitar trabajo repetitivo",
        _ => {
            let _ = idx;
            ""
        }
    }
}

fn clip_chars(text: &str, max_chars: usize) -> String {
    let mut chars = text.chars();
    let head: String = chars.by_ref().take(max_chars).collect();
    if chars.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn stage_info(key: &str) -> (usize, &'static str) {
    if key == "d3" || key.starts_with("d3_") {
        (0, "Día 3")
    } else if key == "d7" || key.starts_with("d7_") {
        (1, "Día 7")
    } else if key == "d14" || key.starts_with("d14_") {
        (2, "Día 14")
    } else {
        (3, "Seguimiento")
    }
}

fn items(t: &Templates, category: usize, settings: &Settings) -> Vec<Item> {
    let tags = palette_tags(t, settings);
    let tags_by_key = keys_label_map(&tags);
    match category {
        0 => t
            .whatsapp
            .iter()
            .enumerate()
            .map(|(i, (k, body))| {
                let title = whatsapp_title(k, i);
                let plain_title = if title.is_empty() {
                    format!("Variante personal {k}")
                } else {
                    title.to_owned()
                };
                let hint = if k == "C" {
                    "Hallazgo web".to_owned()
                } else {
                    String::new()
                };
                Item {
                    key: k.clone(),
                    title_parts: vec![RichPart::Text(plain_title.clone())],
                    plain_title,
                    hint,
                    excerpt_parts: clip_rich_parts(body, &tags_by_key, 86),
                }
            })
            .collect(),
        1 => {
            let mut keys: Vec<&String> = t
                .email
                .keys()
                .filter(|k| !k.starts_with("asunto_"))
                .collect();
            keys.sort_by_key(|k| (!k.starts_with("cuerpo_con"), (*k).clone()));
            keys.into_iter()
                .enumerate()
                .map(|(i, k)| {
                    let subj = email_subject_of(t, k);
                    let mut title_parts = clip_rich_parts(&subj, &tags_by_key, 48);
                    let mut plain_title = plain_title_from_parts(&title_parts);
                    if plain_title.is_empty() {
                        plain_title = format!("Correo sin asunto {}", i + 1);
                        title_parts = vec![RichPart::Text(plain_title.clone())];
                    }
                    let hint = if k.starts_with("cuerpo_con") {
                        "Con diferenciador".to_owned()
                    } else {
                        "Directo".to_owned()
                    };
                    Item {
                        key: k.clone(),
                        title_parts,
                        plain_title,
                        hint,
                        excerpt_parts: clip_rich_parts(&t.email[k], &tags_by_key, 86),
                    }
                })
                .collect()
        }
        2 => {
            let mut keys: Vec<&String> = t.seguimiento.keys().collect();
            keys.sort_by_key(|k| (stage_info(k).0, (*k).clone()));
            let mut seen = BTreeMap::<&'static str, usize>::new();
            keys.into_iter()
                .map(|k| {
                    let (_, day) = stage_info(k);
                    let count = seen.entry(day).or_default();
                    *count += 1;
                    let plain_title = if !k.contains('_') {
                        format!("{day} · Base")
                    } else {
                        format!("{day} · Variante {}", *count - 1)
                    };
                    Item {
                        key: k.clone(),
                        title_parts: vec![RichPart::Text(plain_title.clone())],
                        plain_title,
                        hint: String::new(),
                        excerpt_parts: clip_rich_parts(&t.seguimiento[k], &tags_by_key, 86),
                    }
                })
                .collect()
        }
        _ => t
            .giros
            .iter()
            .map(|(k, fields)| {
                let cosa = fields.get("cosa").map(String::as_str).unwrap_or("");
                let excerpt_parts = if cosa.trim().is_empty() {
                    Vec::new()
                } else {
                    vec![RichPart::Text(clip_chars(
                        &format!("Proceso clave: {cosa}"),
                        84,
                    ))]
                };
                Item {
                    key: k.clone(),
                    title_parts: vec![RichPart::Text(k.clone())],
                    plain_title: k.clone(),
                    hint: String::new(),
                    excerpt_parts,
                }
            })
            .collect(),
    }
}

fn section(t: &mut Templates, category: usize) -> &mut BTreeMap<String, String> {
    match category {
        0 => &mut t.whatsapp,
        2 => &mut t.seguimiento,
        _ => &mut t.email,
    }
}

fn text_of(t: &Templates, category: usize, key: &str) -> String {
    match category {
        0 => t.whatsapp.get(key),
        2 => t.seguimiento.get(key),
        _ => t.email.get(key),
    }
    .cloned()
    .unwrap_or_default()
}

/// Key for a new variant that the message picker will rotate in next to `like`.
fn new_key(t: &Templates, category: usize, like: &str) -> Option<String> {
    let taken = |key: &str| match category {
        0 => t.whatsapp.contains_key(key),
        2 => t.seguimiento.contains_key(key),
        _ => t.email.contains_key(key),
    };
    let numbered = |prefix: &str| {
        (1..=99)
            .map(|n| format!("{prefix}{n:02}"))
            .find(|key| !taken(key))
    };
    match category {
        0 => ('A'..='Z')
            .map(String::from)
            .find(|key| !taken(key))
            .or_else(|| numbered("V")),
        1 => numbered(if like.starts_with("cuerpo_con") {
            "cuerpo_con_"
        } else {
            "cuerpo_sin_"
        }),
        2 => numbered(&format!("{}_", like.split('_').next().unwrap_or("d3"))),
        _ => None,
    }
}

#[derive(Clone, PartialEq, Eq)]
enum PreviewPart {
    Text(String),
    Block {
        label: String,
        value: String,
        own: bool,
    },
}

/// Resolves every template placeholder into highlighted preview chunks alongside prose.
fn resolve_preview_parts(
    raw: &str,
    settings: &Settings,
    t: &Templates,
    giro: Option<&BTreeMap<String, String>>,
) -> Vec<PreviewPart> {
    let pick = |value: &str, fallback: &str| {
        if value.trim().is_empty() {
            fallback.to_owned()
        } else {
            value.trim().to_owned()
        }
    };
    let owner = effective_owner(&settings.owner_name);
    let first_name = owner.split_whitespace().next().unwrap_or(&owner).to_owned();
    let from_giro = |field: &str, fallback: &str| {
        giro.and_then(|g| g.get(field))
            .cloned()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| fallback.to_owned())
    };
    let mut resolved = BTreeMap::<String, (String, String, bool)>::new();
    for meta in BUILTIN_TAGS {
        let value = match meta.key {
            "nombre" | "giro_corto" | "obs" => meta.fallback.to_owned(),
            "cosa" => from_giro("cosa", meta.fallback),
            "pregunta" => from_giro("pregunta", meta.fallback),
            "ejemplo" => from_giro("ejemplo", meta.fallback),
            "owner" => owner.clone(),
            "owner_corto" => first_name.clone(),
            "credencial" => pick(&settings.owner_credential, meta.fallback),
            "whatsapp" => pick(&settings.owner_whatsapp, meta.fallback),
            "portafolio" => pick(&settings.owner_portfolio, meta.fallback),
            "linkedin" => pick(&settings.owner_linkedin, meta.fallback),
            "web" => pick(&settings.owner_website, meta.fallback),
            _ => meta.fallback.to_owned(),
        };
        resolved.insert(
            meta.key.to_owned(),
            (meta.label.to_owned(), value, meta.own),
        );
    }
    for (label, value) in &t.datos {
        resolved.insert(data_key(label), (label.clone(), pick(value, label), true));
    }

    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut scan = 0usize;
    while let Some(rel_open) = raw[scan..].find('{') {
        let open = scan + rel_open;
        let after = open + 1;
        let Some(rel_close) = raw[after..].find('}') else {
            break;
        };
        let close = after + rel_close;
        let key = &raw[after..close];
        if !key.contains(['{', '\n', '\r'])
            && let Some((label, value, own)) = resolved.get(key)
        {
            if open > cursor {
                out.push(PreviewPart::Text(raw[cursor..open].to_owned()));
            }
            out.push(PreviewPart::Block {
                label: label.clone(),
                value: value.clone(),
                own: *own,
            });
            cursor = close + 1;
            scan = close + 1;
        } else {
            scan = after;
        }
    }
    if cursor < raw.len() {
        out.push(PreviewPart::Text(raw[cursor..].to_owned()));
    }
    out
}

fn preview_char_len(parts: &[PreviewPart]) -> usize {
    parts
        .iter()
        .map(|part| match part {
            PreviewPart::Text(s) | PreviewPart::Block { value: s, .. } => s.chars().count(),
        })
        .sum()
}

fn new_datum_error(t: &Templates, label: &str) -> Option<&'static str> {
    let key = data_key(label);
    if key.is_empty() {
        Some("Ponle un nombre al dato.")
    } else if label.contains(['[', ']', '{', '}']) || label.chars().count() > 40 {
        Some("Usa un nombre corto, sin corchetes ni llaves.")
    } else if RESERVED_KEYS.contains(&key.as_str())
        || BUILTIN_TAGS
            .iter()
            .any(|meta| meta.label.eq_ignore_ascii_case(label))
        || t.datos.keys().any(|existing| data_key(existing) == key)
    {
        Some("Ya existe un dato con ese nombre.")
    } else {
        None
    }
}

#[derive(Clone, Copy, PartialEq)]
struct EditorSignals {
    tpl: Signal<String>,
    error: Signal<String>,
    active_field: Signal<EditorField>,
    hovered_tag: Signal<Option<String>>,
    mounted_inputs: Signal<BTreeMap<EditorField, Rc<MountedData>>>,
    pending_focus: Signal<Option<EditorField>>,
}

#[derive(Clone, PartialEq)]
struct EditorDocContext {
    category: usize,
    template_key: String,
    subject_raw: String,
    body_raw: String,
    pairs: Vec<(String, String)>,
    labels: BTreeMap<String, bool>,
    tags: BTreeMap<String, ResolvedTag>,
}

fn write_template_field(
    ed: EditorSignals,
    category: usize,
    template_key: String,
    field: EditorField,
    next_raw: String,
) {
    mutate(ed.tpl, ed.error, move |t| {
        let store_key = if category == 1 && field == EditorField::Subject {
            email_subject_key_for_body(&template_key)
        } else {
            template_key
        };
        section(t, category).insert(store_key, next_raw);
    });
}

#[component]
fn RichPartsView(parts: Vec<RichPart>) -> Element {
    rsx! {
        for (idx, part) in parts.into_iter().enumerate() {
            match part {
                RichPart::Text(txt) => rsx! { "{txt}" },
                RichPart::Tag { label, own } => rsx! {
                    span {
                        key: "mp-{idx}-{label}",
                        class: if own { "st-mini-pill own" } else { "st-mini-pill biz" },
                        "{label}"
                    }
                },
            }
        }
    }
}

#[component]
fn BlockCanvasField(
    field: EditorField,
    title: &'static str,
    meta_note: String,
    raw_value: String,
    single_line: bool,
    cur_epoch: u64,
    ctx: EditorDocContext,
    mut ed: EditorSignals,
) -> Element {
    let friendly = to_friendly_text(&raw_value, &ctx.pairs);
    let spans = parse_friendly_spans(&friendly, &ctx.labels);
    let friendly_for_input = friendly.clone();
    let ctx_input = ctx.clone();

    rsx! {
        div { class: "st-ed-group",
            div { class: "st-ed-head",
                span { class: "st-ed-label", "{title}" }
                if !meta_note.is_empty() {
                    span {
                        class: if field == EditorField::Subject && raw_value.chars().count() > 55 {
                            "muted small st-warn-count"
                        } else {
                            "muted small"
                        },
                        "{meta_note}"
                    }
                }
            }
            div {
                class: if single_line {
                    "st-rich-box single-line st-canvas-text"
                } else {
                    "st-rich-box st-canvas-text"
                },
                div { class: "st-rich-mirror", "aria-hidden": "true",
                    for (si, (start, end, tag)) in spans.into_iter().enumerate() {
                        match tag {
                            None => rsx! {
                                span { key: "rt-{cur_epoch}-{si}", class: "st-rich-text", "{&friendly[start..end]}" }
                            },
                            Some((label, own)) => rsx! {
                                span {
                                    key: "rk-{cur_epoch}-{si}",
                                    class: if own { "st-rich-token own" } else { "st-rich-token biz" },
                                    span { class: "st-rich-pad", "[" }
                                    span { class: "st-rich-lbl", "{label}" }
                                    span { class: "st-rich-pad", "]" }
                                }
                            },
                        }
                    }
                    span { class: "st-rich-pad", "\u{200b}" }
                }
                textarea {
                    key: "ed-ta-{cur_epoch}-{single_line}",
                    class: if single_line { "st-rich-input single-line" } else { "st-rich-input" },
                    rows: if single_line { "1" } else { "3" },
                    aria_label: "{title}",
                    value: "{friendly}",
                    onmounted: move |e: MountedEvent| {
                        let data = e.data();
                        ed.mounted_inputs.write().insert(field, data.clone());
                        if *ed.pending_focus.peek() == Some(field) {
                            ed.pending_focus.set(None);
                            spawn(async move {
                                let _ = data.set_focus(true).await;
                            });
                        }
                    },
                    onfocus: move |_| {
                        mobile::set_editing(true);
                        ed.active_field.set(field);
                    },
                    onblur: move |_| mobile::set_editing(false),
                    onkeydown: move |e: KeyboardEvent| {
                        if single_line && e.key() == Key::Enter {
                            e.prevent_default();
                        }
                    },
                    oninput: move |e: FormEvent| {
                        let mut typed = e.value();
                        if single_line {
                            typed = typed.replace(['\r', '\n'], " ");
                        }
                        let normalized = normalize_block_edit(
                            &friendly_for_input,
                            &typed,
                            &ctx_input.pairs,
                            &ctx_input.labels,
                        );
                        let next_raw = from_friendly_text(&normalized, &ctx_input.pairs);
                        write_template_field(
                            ed,
                            ctx_input.category,
                            ctx_input.template_key.clone(),
                            field,
                            next_raw,
                        );
                    },
                }
            }
        }
    }
}

#[component]
pub(super) fn TemplateCanvas(tpl: Signal<String>, settings: Settings) -> Element {
    let home = consume_context::<Services>().db.paths.home;
    let mut ui = use_context::<UiState>();
    let mut category = use_signal(|| 0usize);
    let picks = use_signal(|| [0usize; 4]);
    let mut epoch = use_signal(|| 0u64);
    let error = use_signal(String::new);
    let mut adding = use_signal(|| false);
    let new_label = use_signal(String::new);
    let new_value = use_signal(String::new);

    let active_field = use_signal(|| EditorField::Body);
    let hovered_tag = use_signal(|| None::<String>);
    let mut mounted_inputs = use_signal(BTreeMap::<EditorField, Rc<MountedData>>::new);
    let mut pending_focus = use_signal(|| None::<EditorField>);

    use_effect(move || {
        let Some(target) = *pending_focus.read() else {
            return;
        };
        if let Some(el) = mounted_inputs.peek().get(&target).cloned() {
            pending_focus.set(None);
            spawn(async move {
                let _ = el.set_focus(true).await;
            });
        }
    });

    let Ok(t) = Templates::parse_toml(&tpl.read()) else {
        return rsx! {
            section { class: "panel st-tpl",
                div { class: "panel-heading",
                    h2 { "Plantillas de mensajes" }
                    super::HelpButton { topic: super::HelpTopic::Templates, label: "Plantillas de mensajes" }
                }
                div { class: "form-body",
                    p { class: "error small", role: "alert",
                        "El archivo de plantillas tiene un error y no se puede editar aquí."
                    }
                    button {
                        class: "button secondary",
                        onclick: move |_| {
                            if let Err(e) = Templates::ensure_file(&home)
                                .map_err(|err| err.to_string())
                                .and_then(|p| open_file(&p))
                            {
                                ui.toast.set(format!("No se pudo abrir el archivo: {e}"));
                            }
                        },
                        "Abrir archivo"
                    }
                }
            }
        };
    };

    let cat = (*category.read()).min(CATEGORIES.len() - 1);
    let list = items(&t, cat, &settings);
    let index = picks.read()[cat].min(list.len().saturating_sub(1));
    let current = list.get(index).cloned().unwrap_or(Item {
        key: String::new(),
        title_parts: vec![RichPart::Text("Sin plantillas".into())],
        plain_title: "Sin plantillas".into(),
        hint: String::new(),
        excerpt_parts: Vec::new(),
    });
    let is_email = cat == 1;
    let is_giro = cat == 3;
    let body_raw = if is_giro {
        String::new()
    } else {
        text_of(&t, cat, &current.key)
    };
    let subject_raw = if is_email {
        email_subject_of(&t, &current.key)
    } else {
        String::new()
    };
    let removable = !is_giro && !REQUIRED.contains(&current.key.as_str());
    let cur_epoch = *epoch.read();
    let giro = t.giros.get(&current.key).cloned().unwrap_or_default();

    let (preview_subject, preview_body) = match cat {
        1 => (
            Some(resolve_preview_parts(&subject_raw, &settings, &t, None)),
            resolve_preview_parts(&body_raw, &settings, &t, None),
        ),
        3 => (
            None,
            resolve_preview_parts(&text_of(&t, 0, "A"), &settings, &t, Some(&giro)),
        ),
        _ => (None, resolve_preview_parts(&body_raw, &settings, &t, None)),
    };
    let preview_subject_len = preview_subject
        .as_deref()
        .map(preview_char_len)
        .unwrap_or(0);

    let all_tags = palette_tags(&t, &settings);
    let biz_tags: Vec<ResolvedTag> = all_tags.iter().filter(|tag| !tag.own).cloned().collect();
    let own_tags: Vec<ResolvedTag> = all_tags.iter().filter(|tag| tag.own).cloned().collect();
    let tag_map: BTreeMap<String, ResolvedTag> = all_tags
        .iter()
        .map(|tag| (tag.key.clone(), tag.clone()))
        .collect();
    let pairs = tag_pairs(&t.datos);
    let labels = labels_ownership_map(&all_tags);

    let duplicate_key = new_key(&t, cat, &current.key);
    let total = list.len();

    let ed = EditorSignals {
        tpl,
        error,
        active_field,
        hovered_tag,
        mounted_inputs,
        pending_focus,
    };
    let doc_ctx = EditorDocContext {
        category: cat,
        template_key: current.key.clone(),
        subject_raw: subject_raw.clone(),
        body_raw: body_raw.clone(),
        pairs,
        labels,
        tags: tag_map,
    };

    rsx! {
        section { class: "panel st-tpl",
            div { class: "panel-heading st-tpl-head",
                h2 { "Plantillas de mensajes" }
                div {
                    class: "st-cat-tabs",
                    role: "tablist",
                    aria_label: "Categorías de plantillas",
                    for (i, name) in CATEGORIES.iter().enumerate() {
                        button {
                            key: "cat-tab-{i}",
                            class: if i == cat { "st-cat-tab active" } else { "st-cat-tab" },
                            r#type: "button",
                            role: "tab",
                            aria_selected: i == cat,
                            onclick: move |_| {
                                category.set(i);
                                adding.set(false);
                                pending_focus.set(None);
                                mounted_inputs.write().clear();
                                epoch += 1;
                            },
                            "{name}"
                        }
                    }
                }
                super::HelpButton { topic: super::HelpTopic::Templates, label: "Plantillas de mensajes" }
            }
            div { class: "st-tpl-main",
                TemplateVariantList {
                    cat,
                    index,
                    list,
                    picks,
                    pending_focus,
                    mounted_inputs,
                    epoch,
                }
                div { class: "st-canvas",
                    div { class: "st-tpl-bar",
                        div { class: "st-tpl-title",
                            strong { "{current.plain_title}" }
                            span { class: "muted small",
                                if current.hint.is_empty() {
                                    "{CATEGORIES[cat]}"
                                } else {
                                    "{CATEGORIES[cat]} · {current.hint}"
                                }
                            }
                        }
                        if !is_giro {
                            TemplateVariantActions {
                                cat,
                                is_email,
                                current_key: current.key.clone(),
                                body_raw: body_raw.clone(),
                                subject_raw: subject_raw.clone(),
                                total,
                                index,
                                duplicate_available: duplicate_key.is_some(),
                                removable,
                                tpl,
                                error,
                                picks,
                                mounted_inputs,
                                epoch,
                            }
                        }
                    }

                    if !is_giro {
                        PalettePanel {
                            is_email,
                            cur_epoch,
                            biz_tags,
                            own_tags,
                            doc_ctx: doc_ctx.clone(),
                            ed,
                            adding,
                            new_label,
                            new_value,
                            templates_ref: t,
                        }
                    }

                    div { class: "st-split-work",
                        div { class: "st-split-ed",
                            TemplateEditorPane {
                                cat,
                                is_giro,
                                is_email,
                                cur_epoch,
                                current_key: current.key.clone(),
                                giro,
                                doc_ctx,
                                ed,
                                subject_raw,
                                body_raw,
                                preview_subject_len,
                            }
                        }
                        div { class: "st-split-prev",
                            PreviewCard {
                                is_giro,
                                preview_subject,
                                preview_body,
                            }
                        }
                    }
                    if !error.read().is_empty() {
                        p { class: "error small", role: "alert", "{error}" }
                    }
                }
            }
        }
    }
}

#[component]
fn TemplateEditorPane(
    cat: usize,
    is_giro: bool,
    is_email: bool,
    cur_epoch: u64,
    current_key: String,
    giro: BTreeMap<String, String>,
    doc_ctx: EditorDocContext,
    ed: EditorSignals,
    subject_raw: String,
    body_raw: String,
    preview_subject_len: usize,
) -> Element {
    if is_giro {
        return rsx! {
            GiroFields {
                cur_epoch,
                current_key,
                giro,
                tpl: ed.tpl,
                error: ed.error,
            }
        };
    }

    let subject_note = format!("{preview_subject_len}/60 caracteres aprox.");
    let body_title: &'static str = if is_email {
        "Mensaje del correo"
    } else {
        "Mensaje"
    };
    let body_note: String = if is_email {
        "Máx. 120 palabras · incluye firma · Enter separa párrafos.".to_owned()
    } else if cat == 0 {
        "Máx. 65 palabras y 1 pregunta · sin enlaces.".to_owned()
    } else {
        "Máx. 80 palabras · deja una línea en blanco entre párrafos.".to_owned()
    };

    rsx! {
        div { class: "st-editor-stack",
            if is_email {
                BlockCanvasField {
                    field: EditorField::Subject,
                    title: "Asunto del correo",
                    meta_note: subject_note,
                    raw_value: subject_raw,
                    single_line: true,
                    cur_epoch,
                    ctx: doc_ctx.clone(),
                    ed,
                }
            }
            BlockCanvasField {
                field: EditorField::Body,
                title: body_title,
                meta_note: body_note,
                raw_value: body_raw,
                single_line: false,
                cur_epoch,
                ctx: doc_ctx,
                ed,
            }
        }
    }
}

#[component]
fn TemplateVariantList(
    cat: usize,
    index: usize,
    list: Vec<Item>,
    mut picks: Signal<[usize; 4]>,
    mut pending_focus: Signal<Option<EditorField>>,
    mut mounted_inputs: Signal<BTreeMap<EditorField, Rc<MountedData>>>,
    mut epoch: Signal<u64>,
) -> Element {
    rsx! {
        nav { class: "st-tpl-list", aria_label: format!("Variantes de {}", CATEGORIES[cat]),
            div { class: "st-tpl-items",
                for (i, item) in list.into_iter().enumerate() {
                    button {
                        key: "{cat}-{item.key}",
                        class: if i == index { "st-tpl-item active" } else { "st-tpl-item" },
                        r#type: "button",
                        aria_current: i == index,
                        onclick: move |_| {
                            picks.write()[cat] = i;
                            pending_focus.set(None);
                            mounted_inputs.write().clear();
                            epoch += 1;
                        },
                        div { class: "st-tpl-item-top",
                            span { class: "st-tpl-name", RichPartsView { parts: item.title_parts } }
                            if !item.hint.is_empty() {
                                span { class: "st-tpl-badge", "{item.hint}" }
                            }
                        }
                        if !item.excerpt_parts.is_empty() {
                            span { class: "st-tpl-excerpt", RichPartsView { parts: item.excerpt_parts } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn TemplateVariantActions(
    cat: usize,
    is_email: bool,
    current_key: String,
    body_raw: String,
    subject_raw: String,
    total: usize,
    index: usize,
    duplicate_available: bool,
    removable: bool,
    tpl: Signal<String>,
    error: Signal<String>,
    mut picks: Signal<[usize; 4]>,
    mut mounted_inputs: Signal<BTreeMap<EditorField, Rc<MountedData>>>,
    mut epoch: Signal<u64>,
) -> Element {
    let dup_key = current_key.clone();
    rsx! {
        div { class: "st-tpl-actions",
            button {
                class: "button primary compact st-new-btn",
                r#type: "button",
                disabled: !duplicate_available,
                title: "Crea una plantilla nueva a partir de esta",
                onclick: move |_| {
                    let Some(key) = Templates::parse_toml(&tpl.peek())
                        .ok()
                        .and_then(|fresh| new_key(&fresh, cat, &dup_key))
                    else {
                        return;
                    };
                    let text = body_raw.clone();
                    let subj = subject_raw.clone();
                    let created = key.clone();
                    mutate(tpl, error, move |t| {
                        if is_email {
                            t.email.insert(email_subject_key_for_body(&key), subj);
                        }
                        section(t, cat).insert(key, text);
                    });
                    if let Ok(fresh) = Templates::parse_toml(&tpl.peek()) {
                        let position = items(&fresh, cat, &Settings::default())
                            .iter()
                            .position(|item| item.key == created)
                            .unwrap_or(total);
                        picks.write()[cat] = position;
                    }
                    mounted_inputs.write().clear();
                    epoch += 1;
                },
                svg {
                    width: "13",
                    height: "13",
                    view_box: "0 0 16 16",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "2.2",
                    stroke_linecap: "round",
                    "aria-hidden": "true",
                    path { d: "M8 3v10M3 8h10" }
                }
                span { "Nueva plantilla" }
            }
            button {
                class: "button secondary compact st-danger",
                r#type: "button",
                disabled: !removable,
                title: if removable {
                    "Eliminar esta plantilla"
                } else {
                    "Las plantillas base no se eliminan; puedes reescribirlas"
                },
                onclick: move |_| {
                    let rm_key = current_key.clone();
                    mutate(tpl, error, |t| {
                        if is_email {
                            t.email.remove(&email_subject_key_for_body(&rm_key));
                        }
                        section(t, cat).remove(&rm_key);
                    });
                    picks.write()[cat] = index.saturating_sub(1);
                    mounted_inputs.write().clear();
                    epoch += 1;
                },
                "Eliminar"
            }
        }
    }
}

#[component]
fn GiroFields(
    cur_epoch: u64,
    current_key: String,
    giro: BTreeMap<String, String>,
    tpl: Signal<String>,
    error: Signal<String>,
) -> Element {
    rsx! {
        div { key: "giro-{current_key}-{cur_epoch}", class: "st-giro",
            p { class: "muted small",
                "Estos tres campos llenan automáticamente los bloques "
                super::InlineTag { label: "Proceso clave", own: false }
                ", "
                super::InlineTag { label: "Pregunta", own: false }
                " y "
                super::InlineTag { label: "Ejemplo de mejora", own: false }
                " cuando la empresa pertenece a este giro."
            }
            for (field, title, rows) in [
                ("cosa", "Proceso clave", 1usize),
                ("pregunta", "Pregunta", 2),
                ("ejemplo", "Ejemplo de mejora", 2),
            ] {
                label { key: "{field}",
                    "{title}"
                    TextEntry {
                        multiline: rows > 1,
                        rows,
                        initial: giro.get(field).cloned().unwrap_or_default(),
                        oninput: {
                            let key = current_key.clone();
                            move |e: FormEvent| {
                                let key = key.clone();
                                mutate(tpl, error, |t| {
                                    t.giros
                                        .entry(key)
                                        .or_default()
                                        .insert(field.into(), e.value());
                                });
                            }
                        },
                    }
                }
            }
        }
    }
}

#[component]
fn PalettePanel(
    is_email: bool,
    cur_epoch: u64,
    biz_tags: Vec<ResolvedTag>,
    own_tags: Vec<ResolvedTag>,
    doc_ctx: EditorDocContext,
    mut ed: EditorSignals,
    mut adding: Signal<bool>,
    mut new_label: Signal<String>,
    mut new_value: Signal<String>,
    templates_ref: Templates,
) -> Element {
    let inspected_tag = ed
        .hovered_tag
        .read()
        .as_deref()
        .and_then(|key| doc_ctx.tags.get(key))
        .cloned();
    let label_draft = new_label.read().clone();
    let datum_error = if label_draft.trim().is_empty() {
        None
    } else {
        new_datum_error(&templates_ref, label_draft.trim())
    };

    rsx! {
        div { class: "st-palette",
            div { class: "st-palette-groups",
                for (group_title, is_own_group, group_tags) in [
                    ("Sobre la empresa", false, biz_tags),
                    ("Tus datos (Perfil)", true, own_tags),
                ] {
                    div { class: "st-chip-group", key: "grp-{group_title}",
                        span { class: "st-chips-label", "{group_title}" }
                        div { class: "st-chips",
                            for tag in group_tags {
                                {
                                    let tag_key_hover = tag.key.clone();
                                    let tag_key_focus = tag.key.clone();
                                    let drag_friendly = format!("[{}]", tag.label);
                                    let click_label = tag.label.clone();
                                    let ctx_click = doc_ctx.clone();
                                    rsx! {
                                        button {
                                            key: "chip-{tag.key}",
                                            class: if tag.own { "st-chip own" } else { "st-chip biz" },
                                            r#type: "button",
                                            draggable: "true",
                                            title: "{tag.description} Ejemplo: «{tag.sample}»",
                                            onmouseenter: move |_| ed.hovered_tag.set(Some(tag_key_hover.clone())),
                                            onfocus: move |_| ed.hovered_tag.set(Some(tag_key_focus.clone())),
                                            ondragstart: move |e: DragEvent| {
                                                e.data_transfer().set_effect_allowed("copy");
                                                let _ = e.data_transfer().set_data("text/plain", &drag_friendly);
                                            },
                                            onclick: move |_| {
                                                let mut target_field = *ed.active_field.peek();
                                                if !is_email {
                                                    target_field = EditorField::Body;
                                                }
                                                let src = if target_field == EditorField::Subject {
                                                    &ctx_click.subject_raw
                                                } else {
                                                    &ctx_click.body_raw
                                                };
                                                let cur_friendly = to_friendly_text(src, &ctx_click.pairs);
                                                let next_friendly = append_token_spacing(&cur_friendly, &click_label);
                                                let next_raw = from_friendly_text(&next_friendly, &ctx_click.pairs);
                                                write_template_field(
                                                    ed,
                                                    ctx_click.category,
                                                    ctx_click.template_key.clone(),
                                                    target_field,
                                                    next_raw,
                                                );
                                                ed.pending_focus.set(Some(target_field));
                                            },
                                            span { class: "st-chip-dot", "aria-hidden": "true" }
                                            "{tag.label}"
                                        }
                                    }
                                }
                            }
                            if is_own_group {
                                button {
                                    class: "st-chip add",
                                    r#type: "button",
                                    aria_expanded: *adding.read(),
                                    onclick: move |_| {
                                        let open = *adding.peek();
                                        adding.set(!open);
                                    },
                                    "+ Nuevo dato"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "st-chip-info",
                if let Some(info) = inspected_tag {
                    strong { "{info.label}" }
                    span { class: "muted", " — {info.description} " }
                    span { class: "st-chip-sample", "Ejemplo: «{info.sample}»" }
                } else {
                    span { class: "muted",
                        strong { "Empresa de ejemplo: " }
                        "«Taller Industrial del Valle» (manufactura metálica). Haz clic en cualquier bloque para añadirlo o arrástralo dentro del texto; pasa el cursor sobre uno para ver qué texto inserta."
                    }
                }
            }
        }
        if *adding.read() {
            div { class: "st-newdata",
                TextEntry {
                    key: "nd-label-{cur_epoch}",
                    label: "Nombre del dato",
                    initial: String::new(),
                    placeholder: "Nombre · ej. Mi empresa",
                    oninput: move |e: FormEvent| new_label.set(e.value()),
                }
                TextEntry {
                    key: "nd-value-{cur_epoch}",
                    label: "Valor del dato",
                    initial: String::new(),
                    placeholder: "Valor · ej. Soluciones del Valle",
                    oninput: move |e: FormEvent| new_value.set(e.value()),
                }
                button {
                    class: "button secondary compact",
                    r#type: "button",
                    disabled: label_draft.trim().is_empty() || datum_error.is_some(),
                    onclick: move |_| {
                        let label = new_label.peek().trim().to_owned();
                        let value = new_value.peek().trim().to_owned();
                        mutate(ed.tpl, ed.error, |t| {
                            t.datos.insert(label, value);
                        });
                        new_label.set(String::new());
                        new_value.set(String::new());
                        adding.set(false);
                    },
                    "Agregar"
                }
                if let Some(message) = datum_error {
                    span { class: "error small", role: "alert", "{message}" }
                } else {
                    span { class: "muted small", "Queda guardado en Perfil para usarlo en cualquier plantilla." }
                }
            }
        }
    }
}

#[component]
fn PreviewPartsView(parts: Vec<PreviewPart>) -> Element {
    rsx! {
        for (idx, part) in parts.into_iter().enumerate() {
            match part {
                PreviewPart::Text(text) => rsx! { span { key: "pt-{idx}", "{text}" } },
                PreviewPart::Block { label, value, own } => rsx! {
                    span {
                        key: "pb-{idx}",
                        class: if own { "st-prev-token own" } else { "st-prev-token biz" },
                        title: "Bloque: {label}",
                        "{value}"
                    }
                },
            }
        }
    }
}

#[component]
fn PreviewCard(
    is_giro: bool,
    preview_subject: Option<Vec<PreviewPart>>,
    preview_body: Vec<PreviewPart>,
) -> Element {
    rsx! {
        div { class: "st-preview",
            div { class: "st-preview-head",
                strong { class: "st-preview-title", "Vista previa en vivo" }
                span { class: "st-preview-badge",
                    if is_giro {
                        "Muestra: WhatsApp base del giro"
                    } else {
                        "Muestra: Taller Industrial del Valle"
                    }
                }
            }
            if let Some(subject) = preview_subject {
                div { class: "st-mail",
                    p { class: "st-mail-subject",
                        span { class: "muted", "Asunto  " }
                        PreviewPartsView { parts: subject }
                    }
                    p { class: "message-preview", PreviewPartsView { parts: preview_body } }
                }
            } else {
                div { class: "st-bubble",
                    p { class: "message-preview", PreviewPartsView { parts: preview_body } }
                }
            }
            div { class: "st-prev-legend",
                span { class: "st-prev-token biz", "Dato de la empresa" }
                span { class: "st-prev-token own", "Dato de tu Perfil" }
                span { class: "muted", "Pasa el cursor sobre un recuadro para ver su bloque." }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_tags_roundtrip_and_atomic_block_editing() {
        let bundled = Templates::bundled_source();
        let mut datos = BTreeMap::new();
        datos.insert("Mi empresa".to_owned(), "Soluciones del Valle".to_owned());
        let t = Templates::parse_toml(bundled).unwrap();
        let settings = Settings {
            owner_name: "Tu nombre".into(),
            ..Settings::default()
        };
        let tags = palette_tags(&t, &settings);
        let owner_tag = tags.iter().find(|tag| tag.key == "owner").unwrap();
        assert_eq!(owner_tag.sample, "Sofía Méndez");

        let pairs = tag_pairs(&datos);
        let friendly = to_friendly_text(bundled, &pairs);
        assert!(!friendly.contains("{nombre}"));
        assert!(!friendly.contains("Tu credencial"));
        assert!(!friendly.contains("Valor agregado"));
        assert!(friendly.contains("[Diferenciador]"));
        assert_eq!(from_friendly_text(&friendly, &pairs), bundled);

        let mut labels = labels_ownership_map(&tags);
        labels.insert("Mi empresa".into(), true);

        // One Backspace touching a pill removes the entire block atomically without exposing '['.
        let before = "Hola, soy [Nombre completo] y visito a [Nombre de la empresa].";
        let backspaced_mid = "Hola, soy [Nombre complet] y visito a [Nombre de la empresa].";
        assert_eq!(
            normalize_block_edit(before, backspaced_mid, &pairs, &labels),
            "Hola, soy y visito a [Nombre de la empresa]."
        );
        let backspaced_end = "Hola, soy [Nombre completo y visito a [Nombre de la empresa].";
        assert_eq!(
            normalize_block_edit(before, backspaced_end, &pairs, &labels),
            "Hola, soy y visito a [Nombre de la empresa]."
        );

        // Typing accidentally inside a pill keeps the pill whole and pushes text cleanly outside.
        let typed_inside = "Hola, soy [Nombre comxpleto] y visito a [Nombre de la empresa].";
        let outside = normalize_block_edit(before, typed_inside, &pairs, &labels);
        assert!(outside.contains("[Nombre completo]"));
        assert!(!outside.contains("[Nombre comxpleto]"));

        // Dropping a chip between two words or adjacent blocks preserves spacing while keeping punctuation flush.
        let pre_drop = "Estuve revisando ay quería.";
        let dropped_mid = "Estuve revisando a[Nombre de la empresa]y quería.";
        assert_eq!(
            normalize_block_edit(pre_drop, dropped_mid, &pairs, &labels),
            "Estuve revisando a [Nombre de la empresa] y quería."
        );
        let pre_dot = "Soy .";
        let dropped_dot = "Soy [Nombre completo].";
        assert_eq!(
            normalize_block_edit(pre_dot, dropped_dot, &pairs, &labels),
            "Soy [Nombre completo]."
        );
        let pre_pair = "Soy .";
        let dropped_pair = "Soy [Nombre completo][Nombre de la empresa].";
        assert_eq!(
            normalize_block_edit(pre_pair, dropped_pair, &pairs, &labels),
            "Soy [Nombre completo] [Nombre de la empresa]."
        );
    }

    #[test]
    fn sidebar_items_and_new_variants_render_cleanly_without_brackets() {
        let mut t = Templates::parse_toml(Templates::bundled_source()).unwrap();
        let settings = Settings::default();
        let wa_items = items(&t, 0, &settings);
        assert!(wa_items[0].excerpt_parts.iter().any(
            |p| matches!(p, RichPart::Tag { label, own: true } if label == "Nombre completo")
        ));
        assert!(!wa_items[0].plain_title.contains('['));

        assert_eq!(new_key(&t, 0, "A").as_deref(), Some("K"));
        assert_eq!(
            new_key(&t, 1, "cuerpo_sin_credencial").as_deref(),
            Some("cuerpo_sin_11")
        );
        assert_eq!(
            new_key(&t, 1, "cuerpo_con_credencial").as_deref(),
            Some("cuerpo_con_11")
        );
        assert_eq!(new_key(&t, 2, "d7_02").as_deref(), Some("d7_11"));
        assert_eq!(new_key(&t, 3, "Otro"), None);
        assert_eq!(items(&t, 2, &settings)[0].plain_title, "Día 3 · Base");
        assert!(
            !items(&t, 1, &settings)
                .iter()
                .any(|item| item.plain_title.contains("Asunto 1"))
        );
        assert!(
            items(&t, 1, &settings)
                .iter()
                .any(|item| item.hint == "Con diferenciador")
        );
        assert!(new_datum_error(&t, "Nombre completo").is_some());
        assert!(new_datum_error(&t, "portafolio").is_some());
        assert!(new_datum_error(&t, "owner").is_some());
        assert!(new_datum_error(&t, "Mi empresa").is_none());
        t.datos.insert("Mi empresa".into(), "Soluciones".into());
        assert!(new_datum_error(&t, "mi  empresa").is_some());
        t.validate().unwrap();
        let text: String =
            resolve_preview_parts("Soy {owner} de {mi_empresa}.", &settings, &t, None)
                .into_iter()
                .map(|part| match part {
                    PreviewPart::Text(s) | PreviewPart::Block { value: s, .. } => s,
                })
                .collect();
        assert_eq!(text, "Soy Sofía Méndez de Soluciones.");

        let parts = resolve_preview_parts(
            "Soy {owner} y visito a {nombre} ({portafolio}).",
            &settings,
            &t,
            None,
        );
        assert!(parts.iter().any(|p| matches!(
            p,
            PreviewPart::Block { label, value, own: true }
                if label == "Nombre completo" && value == "Sofía Méndez"
        )));
        assert!(parts.iter().any(|p| matches!(
            p,
            PreviewPart::Block { label, value, own: false }
                if label == "Nombre de la empresa" && value == "Taller Industrial del Valle"
        )));
        assert!(parts.iter().any(|p| matches!(
            p,
            PreviewPart::Block { label, own: true, .. } if label == "Portafolio"
        )));

        // Paired email subjects stay attached to their body keys and fall back cleanly on legacy TOML.
        assert_eq!(
            email_subject_of(&t, "cuerpo_con_credencial"),
            "Consulta sobre {cosa}"
        );
        t.email.remove("asunto_con_credencial");
        t.email
            .insert("asunto_A".into(), "Asunto legado {cosa}".into());
        assert_eq!(
            email_subject_of(&t, "cuerpo_con_credencial"),
            "Asunto legado {cosa}"
        );
    }

    #[component]
    fn EmailPaneProbe() -> Element {
        let bundled = Templates::bundled_source();
        let t = Templates::parse_toml(bundled).unwrap();
        let settings = Settings::default();
        let tpl = use_signal(|| bundled.to_owned());
        let error = use_signal(String::new);
        let active_field = use_signal(|| EditorField::Body);
        let hovered_tag = use_signal(|| None::<String>);
        let mounted_inputs = use_signal(BTreeMap::<EditorField, Rc<MountedData>>::new);
        let pending_focus = use_signal(|| None::<EditorField>);
        let adding = use_signal(|| false);
        let new_label = use_signal(String::new);
        let new_value = use_signal(String::new);
        let all_tags = palette_tags(&t, &settings);
        let biz_tags: Vec<ResolvedTag> = all_tags.iter().filter(|x| !x.own).cloned().collect();
        let own_tags: Vec<ResolvedTag> = all_tags.iter().filter(|x| x.own).cloned().collect();
        let tag_map: BTreeMap<String, ResolvedTag> = all_tags
            .iter()
            .map(|x| (x.key.clone(), x.clone()))
            .collect();
        let subject_raw = email_subject_of(&t, "cuerpo_con_credencial");
        let body_raw = text_of(&t, 1, "cuerpo_con_credencial");
        let preview_subject = resolve_preview_parts(&subject_raw, &settings, &t, None);
        let preview_body = resolve_preview_parts(&body_raw, &settings, &t, None);
        let preview_subject_len = preview_char_len(&preview_subject);
        let ed = EditorSignals {
            tpl,
            error,
            active_field,
            hovered_tag,
            mounted_inputs,
            pending_focus,
        };
        let doc_ctx = EditorDocContext {
            category: 1,
            template_key: "cuerpo_con_credencial".into(),
            subject_raw: subject_raw.clone(),
            body_raw: body_raw.clone(),
            pairs: tag_pairs(&t.datos),
            labels: labels_ownership_map(&all_tags),
            tags: tag_map,
        };
        rsx! {
            PalettePanel {
                is_email: true,
                cur_epoch: 0,
                biz_tags,
                own_tags,
                doc_ctx: doc_ctx.clone(),
                ed,
                adding,
                new_label,
                new_value,
                templates_ref: t,
            }
            div { class: "st-split-work",
                div { class: "st-split-ed",
                    TemplateEditorPane {
                        cat: 1,
                        is_giro: false,
                        is_email: true,
                        cur_epoch: 0,
                        current_key: "cuerpo_con_credencial".to_owned(),
                        giro: BTreeMap::new(),
                        doc_ctx,
                        ed,
                        subject_raw,
                        body_raw,
                        preview_subject_len,
                    }
                }
                div { class: "st-split-prev",
                    PreviewCard {
                        is_giro: false,
                        preview_subject: Some(preview_subject),
                        preview_body,
                    }
                }
            }
        }
    }

    #[test]
    fn unified_email_editor_renders_paired_subject_body_blocks_and_preview() {
        let mut dom = VirtualDom::new(EmailPaneProbe);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Asunto del correo"));
        assert!(html.contains("Mensaje del correo"));
        assert!(html.contains("st-split-work"));
        assert!(html.contains("st-mail-subject"));
        assert!(html.contains("st-prev-token biz"));
        assert!(html.contains("st-prev-token own"));
        assert!(html.contains("órdenes y refacciones"));
        assert!(html.contains("class=\"st-rich-lbl\">Diferenciador<"));
        assert!(html.contains("Portafolio"));
        assert!(html.contains("LinkedIn"));
        assert!(html.contains("Página web"));
        assert!(!html.contains("{cosa}"));
        assert!(!html.contains("{credencial}"));
    }
}
