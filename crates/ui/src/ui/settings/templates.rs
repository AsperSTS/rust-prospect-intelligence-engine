//! Visual unified message-template composer: visible category tabs, tokenized variant previews,
//! single-surface interactive block canvas (drag, add, hover-inspect, delete), and live sample resolution.
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

#[derive(Clone, Copy, PartialEq)]
pub(super) struct DraftState {
    pub source: Signal<String>,
    pub parsed: Signal<Option<Templates>>,
}

pub(super) fn templates_use_key(t: &Templates, key: &str) -> bool {
    let needle = format!("{{{key}}}");
    t.whatsapp.values().any(|v| v.contains(&needle))
        || t.email.values().any(|v| v.contains(&needle))
        || t.seguimiento.values().any(|v| v.contains(&needle))
        || t.giros
            .values()
            .any(|g| g.values().any(|v| v.contains(&needle)))
}

#[derive(Clone, PartialEq, Eq)]
struct ProfileVars {
    owner_name: String,
    owner_credential: String,
    owner_whatsapp: String,
    owner_portfolio: String,
    owner_linkedin: String,
    owner_website: String,
}

impl ProfileVars {
    fn from_settings(s: &Settings) -> Self {
        Self {
            owner_name: s.owner_name.clone(),
            owner_credential: s.owner_credential.clone(),
            owner_whatsapp: s.owner_whatsapp.clone(),
            owner_portfolio: s.owner_portfolio.clone(),
            owner_linkedin: s.owner_linkedin.clone(),
            owner_website: s.owner_website.clone(),
        }
    }
}

#[derive(Clone, PartialEq)]
struct TagsBundle {
    biz_tags: Vec<ResolvedTag>,
    own_tags: Vec<ResolvedTag>,
    tags: BTreeMap<String, ResolvedTag>,
    pairs: Vec<(String, String)>,
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

pub(super) fn from_friendly_text(friendly: &str, pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .fold(friendly.to_owned(), |text, (key, label)| {
            text.replace(label, key)
        })
}

/// Applies one change to the template draft without re-parsing TOML on every edit.
pub(super) fn mutate(
    mut tpl: DraftState,
    mut error: Signal<String>,
    change: impl FnOnce(&mut Templates),
) {
    let cached = tpl.parsed.peek().clone();
    let Some(mut parsed) = cached.or_else(|| Templates::parse_toml(&tpl.source.peek()).ok()) else {
        error.set("No se pudo actualizar la plantilla actual.".into());
        return;
    };
    change(&mut parsed);
    match parsed.validate().and_then(|()| parsed.to_toml()) {
        Ok(serialized) => {
            tpl.parsed.set(Some(parsed));
            tpl.source.set(serialized);
            if !error.peek().is_empty() {
                error.set(String::new());
            }
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

fn resolve_builtin_tag(meta: &TagMeta, vars: &ProfileVars) -> ResolvedTag {
    let pick = |raw: &str, fallback: &str| {
        if raw.trim().is_empty() {
            fallback.to_owned()
        } else {
            raw.trim().to_owned()
        }
    };
    let sample = match meta.key {
        "owner" => effective_owner(&vars.owner_name),
        "owner_corto" => {
            let full = effective_owner(&vars.owner_name);
            full.split_whitespace().next().unwrap_or(&full).to_owned()
        }
        "credencial" => pick(&vars.owner_credential, meta.fallback),
        "whatsapp" => pick(&vars.owner_whatsapp, meta.fallback),
        "portafolio" => pick(&vars.owner_portfolio, meta.fallback),
        "linkedin" => pick(&vars.owner_linkedin, meta.fallback),
        "web" => pick(&vars.owner_website, meta.fallback),
        _ => meta.fallback.to_owned(),
    };
    ResolvedTag {
        key: meta.key.to_owned(),
        label: meta.label.to_owned(),
        own: meta.own,
        description: meta.description,
        sample,
    }
}

fn build_tags_bundle(vars: &ProfileVars, datos: &BTreeMap<String, String>) -> TagsBundle {
    let pick = |raw: &str, fallback: &str| {
        if raw.trim().is_empty() {
            fallback.to_owned()
        } else {
            raw.trim().to_owned()
        }
    };
    let mut biz_tags = Vec::with_capacity(6);
    let mut own_tags = Vec::with_capacity(7 + datos.len());
    let mut tags = BTreeMap::new();

    for meta in BUILTIN_TAGS {
        let tag = resolve_builtin_tag(&meta, vars);
        if tag.own {
            own_tags.push(tag.clone());
        } else {
            biz_tags.push(tag.clone());
        }
        tags.insert(tag.key.clone(), tag);
    }
    for (label, val) in datos {
        let key = data_key(label);
        let tag = ResolvedTag {
            key: key.clone(),
            label: label.clone(),
            own: true,
            description: "Dato propio guardado en Perfil para reutilizar en cualquier plantilla.",
            sample: pick(val, label),
        };
        own_tags.push(tag.clone());
        tags.insert(key, tag);
    }
    TagsBundle {
        biz_tags,
        own_tags,
        tags,
        pairs: tag_pairs(datos),
    }
}

#[cfg(test)]
fn palette_tags(t: &Templates, settings: &Settings) -> Vec<ResolvedTag> {
    let vars = ProfileVars::from_settings(settings);
    let bundle = build_tags_bundle(&vars, &t.datos);
    bundle.biz_tags.into_iter().chain(bundle.own_tags).collect()
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct TemplateDoc {
    /// Length is always `blocks.len() + 1`; `slots[i]` precedes `blocks[i]`.
    slots: Vec<String>,
    blocks: Vec<String>,
}

fn clamp_char_boundary(s: &str, idx: usize) -> usize {
    let mut pos = idx.min(s.len());
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

fn collapse_join_spaces(left: &str, right: &str) -> String {
    if left.ends_with(' ') && right.starts_with(['.', ',', ';', ':', '?', '!', ')']) {
        format!("{}{right}", &left[..left.len() - 1])
    } else if left.ends_with(' ') && right.starts_with(' ') {
        format!("{left}{}", &right[1..])
    } else if left.is_empty() && right.starts_with(' ') {
        right[1..].to_owned()
    } else {
        format!("{left}{right}")
    }
}

impl TemplateDoc {
    fn parse(raw: &str, tags: &BTreeMap<String, ResolvedTag>) -> Self {
        let mut slots = Vec::new();
        let mut blocks = Vec::new();
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
            if !key.contains(['{', '\n', '\r']) && tags.contains_key(key) {
                slots.push(raw[cursor..open].to_owned());
                blocks.push(key.to_owned());
                cursor = close + 1;
                scan = close + 1;
            } else {
                scan = after;
            }
        }
        slots.push(raw[cursor..].to_owned());
        Self { slots, blocks }
    }

    fn to_raw(&self) -> String {
        let mut out = String::new();
        for (i, slot) in self.slots.iter().enumerate() {
            out.push_str(slot);
            if let Some(key) = self.blocks.get(i) {
                out.push('{');
                out.push_str(key);
                out.push('}');
            }
        }
        out
    }

    fn insert_block_at(&mut self, slot_idx: usize, byte_offset: usize, key: String) -> usize {
        let idx = slot_idx.min(self.slots.len().saturating_sub(1));
        let cut = clamp_char_boundary(&self.slots[idx], byte_offset);
        let mut left = self.slots[idx][..cut].to_owned();
        let mut right = self.slots[idx][cut..].to_owned();

        let needs_space_after_prev_block = idx > 0 && cut == 0;
        if needs_space_after_prev_block
            || left.chars().next_back().is_some_and(|c| {
                c.is_alphanumeric() || matches!(c, '.' | ',' | ';' | ':' | ')' | '?' | '!')
            })
        {
            left.push(' ');
        }

        if right
            .chars()
            .next()
            .is_some_and(|c| c.is_alphanumeric() || matches!(c, '(' | '¿' | '¡'))
        {
            right.insert(0, ' ');
        }

        self.slots[idx] = left;
        self.slots.insert(idx + 1, right);
        self.blocks.insert(idx, key);
        idx + 1
    }

    fn remove_block(&mut self, block_idx: usize) -> usize {
        if block_idx >= self.blocks.len() {
            return self.slots.len().saturating_sub(1);
        }
        self.blocks.remove(block_idx);
        let right = self.slots.remove(block_idx + 1);
        let merged = collapse_join_spaces(&self.slots[block_idx], &right);
        self.slots[block_idx] = merged;
        block_idx
    }

    fn move_block(&mut self, from_block: usize, to_slot: usize, byte_offset: usize) -> usize {
        if from_block >= self.blocks.len() {
            return from_block;
        }
        let target_slot = to_slot.min(self.slots.len().saturating_sub(1));
        if (target_slot == from_block && byte_offset >= self.slots[from_block].len())
            || (target_slot == from_block + 1 && byte_offset == 0)
        {
            return from_block;
        }
        let key = self.blocks[from_block].clone();
        // Insert first so the original byte offsets remain exact, then remove the old block.
        self.insert_block_at(target_slot, byte_offset, key);
        if target_slot <= from_block {
            self.remove_block(from_block + 1);
            target_slot
        } else {
            self.remove_block(from_block);
            target_slot.saturating_sub(1)
        }
    }

    fn nudge_block(&mut self, block_idx: usize, dir_right: bool) -> usize {
        if block_idx >= self.blocks.len() {
            return block_idx;
        }
        if dir_right {
            let right_slot = &self.slots[block_idx + 1];
            if right_slot.trim().is_empty() {
                if block_idx + 1 < self.blocks.len() {
                    let next_len = self.slots[block_idx + 2].len();
                    return self.move_block(block_idx, block_idx + 2, next_len);
                }
                return block_idx;
            }
            let next_cut = next_word_end(right_slot);
            self.move_block(block_idx, block_idx + 1, next_cut)
        } else {
            let left_slot = &self.slots[block_idx];
            if left_slot.trim().is_empty() {
                if block_idx > 0 {
                    return self.move_block(block_idx, block_idx - 1, 0);
                }
                return block_idx;
            }
            let prev_cut = prev_word_start(left_slot);
            self.move_block(block_idx, block_idx, prev_cut)
        }
    }

    fn expand_inline_tokens(
        &mut self,
        slot_idx: usize,
        pairs: &[(String, String)],
        tags: &BTreeMap<String, ResolvedTag>,
    ) -> Option<usize> {
        let slot = self.slots.get(slot_idx)?;
        if !((slot.contains('{') && slot.contains('}'))
            || (slot.contains('[') && slot.contains(']')))
        {
            return None;
        }
        let normalized = from_friendly_text(slot, pairs);
        let parsed = Self::parse(&normalized, tags);
        if parsed.blocks.is_empty() {
            return None;
        }
        let inserted = parsed.blocks.len();
        let mut parsed_slots = parsed.slots.into_iter();
        if let Some(first) = parsed_slots.next() {
            self.slots[slot_idx] = first;
        }
        for (offset, (key, tail)) in parsed.blocks.into_iter().zip(parsed_slots).enumerate() {
            self.blocks.insert(slot_idx + offset, key);
            self.slots.insert(slot_idx + offset + 1, tail);
        }
        Some(slot_idx + inserted)
    }
}

fn next_word_end(s: &str) -> usize {
    let mut in_word = false;
    for (idx, ch) in s.char_indices() {
        if !ch.is_whitespace() {
            in_word = true;
        } else if in_word {
            return idx;
        }
    }
    s.len()
}

fn prev_word_start(s: &str) -> usize {
    let trimmed = s.trim_end();
    for (idx, ch) in trimmed.char_indices().rev() {
        if ch.is_whitespace() {
            return idx + ch.len_utf8();
        }
    }
    0
}

fn split_lines_with_offsets(slot: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (idx, ch) in slot.char_indices() {
        if ch == '\n' {
            out.push((start, &slot[start..idx]));
            start = idx + 1;
        }
    }
    out.push((start, &slot[start..]));
    out
}

fn split_word_pieces(line: &str, line_start: usize) -> Vec<(usize, usize, &str)> {
    let mut out = Vec::new();
    let mut start = 0usize;
    let mut chars = line.char_indices().peekable();
    while let Some(&(idx, ch)) = chars.peek() {
        let is_ws = ch.is_whitespace();
        while let Some(&(_, next_ch)) = chars.peek() {
            if next_ch.is_whitespace() != is_ws {
                break;
            }
            chars.next();
        }
        let end = chars.peek().map_or(line.len(), |&(i, _)| i);
        out.push((line_start + start, line_start + end, &line[start..end]));
        start = end;
        let _ = idx;
    }
    out
}

fn raw_to_rich_parts(raw: &str, tags_by_key: &BTreeMap<String, ResolvedTag>) -> Vec<RichPart> {
    let mut parts = Vec::new();
    let mut rest = raw;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        let key = &after[..close];
        if !key.contains(['{', '\n', '\r'])
            && let Some(tag) = tags_by_key.get(key)
        {
            if open > 0 {
                parts.push(RichPart::Text(rest[..open].to_owned()));
            }
            parts.push(RichPart::Tag {
                label: tag.label.clone(),
                own: tag.own,
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
/// truncating cleanly on word or tag boundaries without allocating a char buffer.
fn clip_rich_parts(
    raw: &str,
    tags_by_key: &BTreeMap<String, ResolvedTag>,
    max_chars: usize,
) -> Vec<RichPart> {
    let raw_parts = raw_to_rich_parts(raw, tags_by_key);
    let mut out: Vec<RichPart> = Vec::new();
    let mut used = 0usize;
    let total = raw_parts.len();
    for (idx, part) in raw_parts.into_iter().enumerate() {
        match part {
            RichPart::Text(txt) => {
                let compact = compact_whitespace_run(&txt);
                let mut slice = compact.as_str();
                if idx == 0 {
                    slice = slice.trim_start();
                }
                if idx + 1 == total {
                    slice = slice.trim_end();
                }
                if slice.is_empty() {
                    continue;
                }
                let char_count = slice.chars().count();
                if used + char_count <= max_chars {
                    used += char_count;
                    push_rich_text(&mut out, slice);
                } else {
                    let room = max_chars.saturating_sub(used);
                    if room > 0 {
                        let cut = slice
                            .char_indices()
                            .nth(room)
                            .map_or(slice.len(), |(pos, _)| pos);
                        let trimmed = slice[..cut].trim_end();
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

#[cfg(test)]
fn items(t: &Templates, category: usize, settings: &Settings) -> Vec<Item> {
    let vars = ProfileVars::from_settings(settings);
    let bundle = build_tags_bundle(&vars, &t.datos);
    items_with_tags(t, category, &bundle.tags)
}

fn items_with_tags(
    t: &Templates,
    category: usize,
    tags_by_key: &BTreeMap<String, ResolvedTag>,
) -> Vec<Item> {
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
                    excerpt_parts: clip_rich_parts(body, tags_by_key, 86),
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
                    let mut title_parts = clip_rich_parts(&subj, tags_by_key, 48);
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
                        excerpt_parts: clip_rich_parts(&t.email[k], tags_by_key, 86),
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
                        excerpt_parts: clip_rich_parts(&t.seguimiento[k], tags_by_key, 86),
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

fn plain_sample_text(parts: &[PreviewPart]) -> String {
    parts
        .iter()
        .map(|part| match part {
            PreviewPart::Text(s) | PreviewPart::Block { value: s, .. } => s.as_str(),
        })
        .collect()
}

fn count_doc_words(doc: &TemplateDoc, tags: &BTreeMap<String, ResolvedTag>) -> usize {
    let mut words = 0usize;
    let mut in_word = false;
    let mut scan = |s: &str| {
        for ch in s.chars() {
            if ch.is_whitespace() {
                in_word = false;
            } else if !in_word {
                in_word = true;
                words += 1;
            }
        }
    };
    for (idx, slot) in doc.slots.iter().enumerate() {
        scan(slot);
        if let Some(key) = doc.blocks.get(idx) {
            if let Some(tag) = tags.get(key) {
                scan(&tag.sample);
            } else {
                scan("{");
                scan(key);
                scan("}");
            }
        }
    }
    words
}

fn count_doc_chars(doc: &TemplateDoc, tags: &BTreeMap<String, ResolvedTag>) -> usize {
    doc.slots.iter().map(|s| s.chars().count()).sum::<usize>()
        + doc
            .blocks
            .iter()
            .map(|key| {
                tags.get(key)
                    .map_or(key.chars().count() + 2, |tag| tag.sample.chars().count())
            })
            .sum::<usize>()
}

fn doc_to_sample_string(doc: &TemplateDoc, tags: &BTreeMap<String, ResolvedTag>) -> String {
    let mut out = String::new();
    for (idx, slot) in doc.slots.iter().enumerate() {
        out.push_str(slot);
        if let Some(key) = doc.blocks.get(idx) {
            if let Some(tag) = tags.get(key) {
                out.push_str(&tag.sample);
            } else {
                out.push('{');
                out.push_str(key);
                out.push('}');
            }
        }
    }
    out
}

fn new_datum_error_in_keys<'a>(
    existing_keys: impl IntoIterator<Item = &'a String>,
    label: &str,
) -> Option<&'static str> {
    let key = data_key(label);
    if key.is_empty() {
        Some("Ponle un nombre al dato.")
    } else if label.contains(['[', ']', '{', '}']) || label.chars().count() > 40 {
        Some("Usa un nombre corto, sin corchetes ni llaves.")
    } else if RESERVED_KEYS.contains(&key.as_str())
        || BUILTIN_TAGS
            .iter()
            .any(|meta| meta.label.eq_ignore_ascii_case(label))
        || existing_keys
            .into_iter()
            .any(|existing| data_key(existing) == key)
    {
        Some("Ya existe un dato con ese nombre.")
    } else {
        None
    }
}

#[cfg(test)]
fn new_datum_error(t: &Templates, label: &str) -> Option<&'static str> {
    new_datum_error_in_keys(t.datos.keys(), label)
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum DragPayload {
    AddKey(String),
    MoveBlock {
        field: EditorField,
        block_idx: usize,
        key: String,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct DropPos {
    field: EditorField,
    slot_idx: usize,
    byte_offset: usize,
}

type SlotTarget = (EditorField, usize);
type MountedSlot = (SlotTarget, Rc<MountedData>);

#[derive(Clone, Copy, PartialEq)]
struct EditorSignals {
    tpl: DraftState,
    error: Signal<String>,
    cursor_slot: Signal<SlotTarget>,
    editing_slot: Signal<Option<SlotTarget>>,
    pending_focus: Signal<Option<SlotTarget>>,
    mounted_slot: Signal<Option<MountedSlot>>,
    selected_block: Signal<Option<SlotTarget>>,
    hovered_tag: Signal<Option<String>>,
    drag_payload: Signal<Option<DragPayload>>,
    drop_target: Signal<Option<DropPos>>,
}

fn set_drop_target(mut sig: Signal<Option<DropPos>>, pos: DropPos) {
    if *sig.peek() != Some(pos) {
        sig.set(Some(pos));
    }
}

fn set_hovered_tag(mut sig: Signal<Option<String>>, next: &str) {
    if sig.peek().as_deref() != Some(next) {
        sig.set(Some(next.to_owned()));
    }
}

fn clear_hovered_tag(mut sig: Signal<Option<String>>, expected: &str) {
    if sig.peek().as_deref() == Some(expected) {
        sig.set(None);
    }
}

fn current_field_raw(
    tpl: &DraftState,
    category: usize,
    template_key: &str,
    field: EditorField,
) -> String {
    let cached = tpl.parsed.peek().clone();
    let Some(t) = cached.or_else(|| Templates::parse_toml(&tpl.source.peek()).ok()) else {
        return String::new();
    };
    if category == 1 && field == EditorField::Subject {
        email_subject_of(&t, template_key)
    } else {
        text_of(&t, category, template_key)
    }
}

fn current_field_doc(
    tpl: &DraftState,
    category: usize,
    template_key: &str,
    field: EditorField,
    tags: &BTreeMap<String, ResolvedTag>,
) -> TemplateDoc {
    let raw = current_field_raw(tpl, category, template_key, field);
    TemplateDoc::parse(&raw, tags)
}

fn store_key_for(category: usize, template_key: &str, field: EditorField) -> String {
    if category == 1 && field == EditorField::Subject {
        email_subject_key_for_body(template_key)
    } else {
        template_key.to_owned()
    }
}

fn write_template_field(
    ed: EditorSignals,
    category: usize,
    template_key: &str,
    field: EditorField,
    next_raw: String,
) {
    let store_key = store_key_for(category, template_key, field);
    mutate(ed.tpl, ed.error, move |t| {
        section(t, category).insert(store_key, next_raw);
    });
}

fn begin_slot_edit(mut ed: EditorSignals, field: EditorField, slot_idx: usize) {
    if ed.selected_block.peek().is_some() {
        ed.selected_block.set(None);
    }
    ed.cursor_slot.set((field, slot_idx));
    ed.editing_slot.set(Some((field, slot_idx)));
    ed.pending_focus.set(Some((field, slot_idx)));
}

fn clear_editor_interaction(mut ed: EditorSignals, body_slots_len: usize) {
    ed.editing_slot.set(None);
    ed.pending_focus.set(None);
    ed.mounted_slot.set(None);
    ed.selected_block.set(None);
    ed.hovered_tag.set(None);
    ed.drag_payload.set(None);
    ed.drop_target.set(None);
    ed.cursor_slot
        .set((EditorField::Body, body_slots_len.saturating_sub(1)));
}

fn remove_block(
    mut ed: EditorSignals,
    category: usize,
    template_key: &str,
    tags: &BTreeMap<String, ResolvedTag>,
    field: EditorField,
    block_idx: usize,
    refocus: bool,
) {
    let mut doc = current_field_doc(&ed.tpl, category, template_key, field, tags);
    let target_slot = doc.remove_block(block_idx);
    ed.selected_block.set(None);
    ed.hovered_tag.set(None);
    ed.cursor_slot.set((field, target_slot));
    if refocus {
        ed.editing_slot.set(Some((field, target_slot)));
        ed.pending_focus.set(Some((field, target_slot)));
    }
    write_template_field(ed, category, template_key, field, doc.to_raw());
}

fn apply_drop(
    mut ed: EditorSignals,
    category: usize,
    template_key: &str,
    tags: &BTreeMap<String, ResolvedTag>,
    target_field: EditorField,
    fallback_slot: usize,
    fallback_byte: usize,
) {
    let Some(payload) = ed.drag_payload.peek().clone() else {
        ed.drop_target.set(None);
        return;
    };
    let pos = (*ed.drop_target.peek())
        .filter(|p| p.field == target_field)
        .unwrap_or(DropPos {
            field: target_field,
            slot_idx: fallback_slot,
            byte_offset: fallback_byte,
        });

    ed.drag_payload.set(None);
    ed.drop_target.set(None);
    ed.selected_block.set(None);

    match payload {
        DragPayload::AddKey(key) => {
            let mut doc = current_field_doc(&ed.tpl, category, template_key, target_field, tags);
            let next_slot = doc.insert_block_at(pos.slot_idx, pos.byte_offset, key);
            ed.cursor_slot.set((target_field, next_slot));
            write_template_field(ed, category, template_key, target_field, doc.to_raw());
        }
        DragPayload::MoveBlock {
            field: from_field,
            block_idx,
            key,
        } => {
            if from_field == target_field {
                let mut doc =
                    current_field_doc(&ed.tpl, category, template_key, target_field, tags);
                let landed = doc.move_block(block_idx, pos.slot_idx, pos.byte_offset);
                ed.cursor_slot.set((target_field, landed + 1));
                write_template_field(ed, category, template_key, target_field, doc.to_raw());
            } else {
                let mut from_doc =
                    current_field_doc(&ed.tpl, category, template_key, from_field, tags);
                let mut to_doc =
                    current_field_doc(&ed.tpl, category, template_key, target_field, tags);
                from_doc.remove_block(block_idx);
                let next_slot = to_doc.insert_block_at(pos.slot_idx, pos.byte_offset, key);
                ed.cursor_slot.set((target_field, next_slot));
                let from_store = store_key_for(category, template_key, from_field);
                let to_store = store_key_for(category, template_key, target_field);
                let from_raw = from_doc.to_raw();
                let to_raw = to_doc.to_raw();
                mutate(ed.tpl, ed.error, move |t| {
                    section(t, category).insert(from_store, from_raw);
                    section(t, category).insert(to_store, to_raw);
                });
            }
        }
    }
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
fn UnifiedFieldSurface(
    field: EditorField,
    single_line: bool,
    placeholder: &'static str,
    cur_epoch: u64,
    category: usize,
    template_key: Rc<str>,
    doc: Rc<TemplateDoc>,
    tags_bundle: Memo<Rc<TagsBundle>>,
    mut ed: EditorSignals,
) -> Element {
    let slot_count = doc.slots.len();
    let last_slot = slot_count.saturating_sub(1);
    let last_len = doc.slots.last().map_or(0, String::len);
    let bundle = tags_bundle.read();

    let active_edit_sig = use_memo(move || {
        ed.editing_slot
            .read()
            .filter(|(f, _)| *f == field)
            .map(|(_, s)| s)
    });
    let active_drop_sig = use_memo(move || ed.drop_target.read().filter(|p| p.field == field));
    let active_sel_sig = use_memo(move || {
        ed.selected_block
            .read()
            .filter(|(f, _)| *f == field)
            .map(|(_, s)| s)
    });

    let is_dragging = ed.drag_payload.read().is_some();
    let active_edit = *active_edit_sig.read();
    let active_drop = *active_drop_sig.read();
    let active_sel = *active_sel_sig.read();
    let total_empty = doc.blocks.is_empty() && doc.slots.iter().all(|s| s.is_empty());

    let tkey_surface_drop = template_key.clone();

    rsx! {
        div {
            class: if single_line {
                if is_dragging { "st-prose-surface single-line dragging" } else { "st-prose-surface single-line" }
            } else if is_dragging {
                "st-prose-surface dragging"
            } else {
                "st-prose-surface"
            },
            ondragover: move |e: DragEvent| {
                e.prevent_default();
                let current_in_field = ed.drop_target.peek().is_some_and(|p| p.field == field);
                if !current_in_field {
                    set_drop_target(
                        ed.drop_target,
                        DropPos {
                            field,
                            slot_idx: last_slot,
                            byte_offset: last_len,
                        },
                    );
                }
            },
            ondrop: move |e: DragEvent| {
                e.prevent_default();
                e.stop_propagation();
                apply_drop(
                    ed,
                    category,
                    &tkey_surface_drop,
                    &tags_bundle.peek().tags,
                    field,
                    last_slot,
                    last_len,
                );
            },
            onclick: move |_| {
                if ed.drag_payload.peek().is_none() {
                    begin_slot_edit(ed, field, last_slot);
                }
            },
            for i in 0..slot_count {
                {
                    let slot_text = doc.slots[i].as_str();
                    let slot_len = slot_text.len();
                    let is_editing = active_edit == Some(i);
                    let slot_block = doc.blocks.get(i).map(String::as_str);

                    rsx! {
                        if is_editing {
                            if single_line {
                                {
                                    let ch_w = (slot_text.chars().count() + 3).max(8);
                                    let text_empty = slot_text.is_empty();
                                    let tkey_del = template_key.clone();
                                    let tkey_in = template_key.clone();
                                    rsx! {
                                        input {
                                            key: "slot-in-{cur_epoch}-{category}-{template_key}-subj-{i}",
                                            class: "st-slot-input",
                                            r#type: "text",
                                            style: "width: min(100%, {ch_w}ch);",
                                            aria_label: "Asunto del correo",
                                            value: "{slot_text}",
                                            onmounted: move |e: MountedEvent| {
                                                ed.mounted_slot.set(Some(((field, i), e.data())));
                                            },
                                            onfocus: move |_| {
                                                mobile::set_editing(true);
                                                ed.cursor_slot.set((field, i));
                                                if ed.selected_block.peek().is_some() {
                                                    ed.selected_block.set(None);
                                                }
                                            },
                                            onblur: move |_| {
                                                mobile::set_editing(false);
                                                if *ed.editing_slot.peek() == Some((field, i)) {
                                                    ed.editing_slot.set(None);
                                                }
                                            },
                                            onclick: move |e: MouseEvent| e.stop_propagation(),
                                            onkeydown: move |e: KeyboardEvent| {
                                                match e.key() {
                                                    Key::Enter | Key::Escape => {
                                                        e.prevent_default();
                                                        if *ed.editing_slot.peek() == Some((field, i)) {
                                                            ed.editing_slot.set(None);
                                                        }
                                                    }
                                                    Key::Backspace if text_empty && i > 0 => {
                                                        e.prevent_default();
                                                        remove_block(
                                                            ed,
                                                            category,
                                                            &tkey_del,
                                                            &tags_bundle.peek().tags,
                                                            field,
                                                            i - 1,
                                                            true,
                                                        );
                                                    }
                                                    _ => {}
                                                }
                                            },
                                            oninput: move |e: FormEvent| {
                                                let typed = e.value().replace(['\r', '\n'], " ");
                                                let tb = tags_bundle.peek();
                                                let mut next_doc = current_field_doc(&ed.tpl, category, &tkey_in, field, &tb.tags);
                                                if i < next_doc.slots.len() {
                                                    next_doc.slots[i] = typed;
                                                    if let Some(landed) = next_doc.expand_inline_tokens(i, &tb.pairs, &tb.tags) {
                                                        ed.cursor_slot.set((field, landed));
                                                        ed.editing_slot.set(Some((field, landed)));
                                                        ed.pending_focus.set(Some((field, landed)));
                                                    }
                                                    write_template_field(ed, category, &tkey_in, field, next_doc.to_raw());
                                                }
                                            },
                                        }
                                    }
                                }
                            } else {
                                {
                                    let chars = slot_text.chars().count();
                                    let newlines = slot_text.matches('\n').count();
                                    let wide = newlines > 0 || chars > 30;
                                    let rows = if !wide { 1 } else { ((chars / 56) + newlines + 1).clamp(2, 9) };
                                    let area_style = if wide {
                                        "display:block;width:100%;margin:4px 0;".to_owned()
                                    } else {
                                        format!("display:inline-block;width:min(100%, {}ch);vertical-align:middle;", (chars + 3).max(8))
                                    };
                                    let text_empty = slot_text.is_empty();
                                    let tkey_del = template_key.clone();
                                    let tkey_in = template_key.clone();
                                    rsx! {
                                        textarea {
                                            key: "slot-ta-{cur_epoch}-{category}-{template_key}-body-{i}",
                                            class: "st-slot-area",
                                            rows: "{rows}",
                                            style: "{area_style}",
                                            aria_label: "Texto del mensaje",
                                            value: "{slot_text}",
                                            onmounted: move |e: MountedEvent| {
                                                ed.mounted_slot.set(Some(((field, i), e.data())));
                                            },
                                            onfocus: move |_| {
                                                mobile::set_editing(true);
                                                ed.cursor_slot.set((field, i));
                                                if ed.selected_block.peek().is_some() {
                                                    ed.selected_block.set(None);
                                                }
                                            },
                                            onblur: move |_| {
                                                mobile::set_editing(false);
                                                if *ed.editing_slot.peek() == Some((field, i)) {
                                                    ed.editing_slot.set(None);
                                                }
                                            },
                                            onclick: move |e: MouseEvent| e.stop_propagation(),
                                            onkeydown: move |e: KeyboardEvent| {
                                                match e.key() {
                                                    Key::Escape => {
                                                        e.prevent_default();
                                                        if *ed.editing_slot.peek() == Some((field, i)) {
                                                            ed.editing_slot.set(None);
                                                        }
                                                    }
                                                    Key::Backspace if text_empty && i > 0 => {
                                                        e.prevent_default();
                                                        remove_block(
                                                            ed,
                                                            category,
                                                            &tkey_del,
                                                            &tags_bundle.peek().tags,
                                                            field,
                                                            i - 1,
                                                            true,
                                                        );
                                                    }
                                                    _ => {}
                                                }
                                            },
                                            oninput: move |e: FormEvent| {
                                                let typed = e.value();
                                                let tb = tags_bundle.peek();
                                                let mut next_doc = current_field_doc(&ed.tpl, category, &tkey_in, field, &tb.tags);
                                                if i < next_doc.slots.len() {
                                                    next_doc.slots[i] = typed;
                                                    if let Some(landed) = next_doc.expand_inline_tokens(i, &tb.pairs, &tb.tags) {
                                                        ed.cursor_slot.set((field, landed));
                                                        ed.editing_slot.set(Some((field, landed)));
                                                        ed.pending_focus.set(Some((field, landed)));
                                                    }
                                                    write_template_field(ed, category, &tkey_in, field, next_doc.to_raw());
                                                }
                                            },
                                        }
                                    }
                                }
                            }
                        } else if slot_text.is_empty() {
                            if is_dragging {
                                span {
                                    key: "dz-empty-{i}",
                                    class: "st-drop-gap",
                                    ondragover: move |e: DragEvent| {
                                        e.prevent_default();
                                        e.stop_propagation();
                                        set_drop_target(ed.drop_target, DropPos { field, slot_idx: i, byte_offset: 0 });
                                    },
                                    if active_drop == Some(DropPos { field, slot_idx: i, byte_offset: 0 }) {
                                        span { class: "st-drop-caret" }
                                    }
                                }
                            } else if i == 0 && total_empty {
                                span {
                                    key: "empty-hint-{i}",
                                    class: "st-empty-hint",
                                    onclick: move |e: MouseEvent| {
                                        e.stop_propagation();
                                        begin_slot_edit(ed, field, 0);
                                    },
                                    "{placeholder}"
                                }
                            } else if i > 0 && slot_block.is_some() {
                                span {
                                    key: "gap-click-{i}",
                                    class: "st-slot-gap",
                                    title: "Haz clic para escribir aquí",
                                    onclick: move |e: MouseEvent| {
                                        e.stop_propagation();
                                        begin_slot_edit(ed, field, i);
                                    },
                                }
                            }
                        } else {
                            for (line_idx, (line_start, line_str)) in split_lines_with_offsets(slot_text).into_iter().enumerate() {
                                if line_idx > 0 {
                                    br { key: "br-{i}-{line_idx}" }
                                }
                                if line_str.is_empty() {
                                    if is_dragging {
                                        span {
                                            key: "dz-line-{i}-{line_idx}",
                                            class: "st-drop-gap",
                                            ondragover: move |e: DragEvent| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                set_drop_target(ed.drop_target, DropPos { field, slot_idx: i, byte_offset: line_start });
                                            },
                                            if active_drop == Some(DropPos { field, slot_idx: i, byte_offset: line_start }) {
                                                span { class: "st-drop-caret" }
                                            }
                                        }
                                    }
                                } else if !is_dragging {
                                    span {
                                        key: "pr-{i}-{line_idx}",
                                        class: "st-prose-run",
                                        onclick: move |e: MouseEvent| {
                                            e.stop_propagation();
                                            begin_slot_edit(ed, field, i);
                                        },
                                        "{line_str}"
                                    }
                                } else {
                                    span {
                                        key: "dz-pre-{i}-{line_idx}",
                                        class: "st-drop-pre",
                                        ondragover: move |e: DragEvent| {
                                            e.prevent_default();
                                            e.stop_propagation();
                                            set_drop_target(ed.drop_target, DropPos { field, slot_idx: i, byte_offset: line_start });
                                        },
                                        if active_drop == Some(DropPos { field, slot_idx: i, byte_offset: line_start }) {
                                            span { class: "st-drop-caret" }
                                        }
                                    }
                                    for (pi, (_w_start, w_end, piece)) in split_word_pieces(line_str, line_start).into_iter().enumerate() {
                                        span {
                                            key: "wp-{i}-{line_idx}-{pi}",
                                            class: "st-drop-word",
                                            ondragover: move |e: DragEvent| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                set_drop_target(ed.drop_target, DropPos { field, slot_idx: i, byte_offset: w_end });
                                            },
                                            "{piece}"
                                        }
                                        if active_drop == Some(DropPos { field, slot_idx: i, byte_offset: w_end }) {
                                            span { key: "dc-{i}-{line_idx}-{pi}", class: "st-drop-caret" }
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(key) = slot_block {
                            if let Some(tag) = bundle.tags.get(key) {
                                {
                                    let is_selected = active_sel == Some(i);
                                    let tkey_drop = template_key.clone();
                                    let tkey_key = template_key.clone();
                                    let tkey_del = template_key.clone();
                                    let hover_key = tag.key.clone();
                                    let leave_key = tag.key.clone();
                                    let drag_key = tag.key.clone();
                                    let click_key = tag.key.clone();
                                    let drag_label = tag.label.clone();
                                    rsx! {
                                        span {
                                            key: "pill-{cur_epoch}-{field:?}-{i}-{tag.key}",
                                            class: if tag.own {
                                                if is_selected { "st-token-pill own selected" } else { "st-token-pill own" }
                                            } else if is_selected {
                                                "st-token-pill biz selected"
                                            } else {
                                                "st-token-pill biz"
                                            },
                                            draggable: "true",
                                            tabindex: "0",
                                            title: "Bloque: {tag.label} — {tag.description}",
                                            onmouseenter: move |_| set_hovered_tag(ed.hovered_tag, &hover_key),
                                            onmouseleave: move |_| clear_hovered_tag(ed.hovered_tag, &leave_key),
                                            ondragstart: move |e: DragEvent| {
                                                e.data_transfer().set_effect_allowed("move");
                                                let _ = e.data_transfer().set_data("text/plain", &drag_label);
                                                ed.editing_slot.set(None);
                                                ed.selected_block.set(None);
                                                ed.drag_payload.set(Some(DragPayload::MoveBlock {
                                                    field,
                                                    block_idx: i,
                                                    key: drag_key.clone(),
                                                }));
                                            },
                                            ondragend: move |_| {
                                                ed.drag_payload.set(None);
                                                ed.drop_target.set(None);
                                            },
                                            ondragover: move |e: DragEvent| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                set_drop_target(ed.drop_target, DropPos {
                                                    field,
                                                    slot_idx: i,
                                                    byte_offset: slot_len,
                                                });
                                            },
                                            ondrop: move |e: DragEvent| {
                                                e.prevent_default();
                                                e.stop_propagation();
                                                apply_drop(
                                                    ed,
                                                    category,
                                                    &tkey_drop,
                                                    &tags_bundle.peek().tags,
                                                    field,
                                                    i,
                                                    slot_len,
                                                );
                                            },
                                            onclick: move |e: MouseEvent| {
                                                e.stop_propagation();
                                                ed.editing_slot.set(None);
                                                ed.selected_block.set(Some((field, i)));
                                                set_hovered_tag(ed.hovered_tag, &click_key);
                                            },
                                            onkeydown: move |e: KeyboardEvent| {
                                                match e.key() {
                                                    Key::Backspace | Key::Delete => {
                                                        e.prevent_default();
                                                        remove_block(
                                                            ed,
                                                            category,
                                                            &tkey_key,
                                                            &tags_bundle.peek().tags,
                                                            field,
                                                            i,
                                                            false,
                                                        );
                                                    }
                                                    Key::ArrowLeft | Key::ArrowRight => {
                                                        e.prevent_default();
                                                        let tb = tags_bundle.peek();
                                                        let mut next_doc = current_field_doc(&ed.tpl, category, &tkey_key, field, &tb.tags);
                                                        let moved_to = next_doc.nudge_block(i, e.key() == Key::ArrowRight);
                                                        ed.selected_block.set(Some((field, moved_to)));
                                                        write_template_field(ed, category, &tkey_key, field, next_doc.to_raw());
                                                    }
                                                    Key::Escape => {
                                                        ed.selected_block.set(None);
                                                    }
                                                    _ => {}
                                                }
                                            },
                                            span { class: "st-pill-text", "{tag.sample}" }
                                            button {
                                                class: "st-pill-del",
                                                r#type: "button",
                                                draggable: "false",
                                                aria_label: "Quitar bloque {tag.label}",
                                                title: "Quitar {tag.label}",
                                                onclick: move |e: MouseEvent| {
                                                    e.stop_propagation();
                                                    remove_block(
                                                        ed,
                                                        category,
                                                        &tkey_del,
                                                        &tags_bundle.peek().tags,
                                                        field,
                                                        i,
                                                        false,
                                                    );
                                                },
                                                "×"
                                            }
                                        }
                                    }
                                }
                            } else {
                                span { key: "raw-blk-{i}", class: "st-prose-run", "{{{key}}}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn TemplateCanvas(tpl: DraftState, settings: Signal<Settings>) -> Element {
    let home = consume_context::<Services>().db.paths.home;
    let mut ui = use_context::<UiState>();
    let mut category = use_signal(|| 0usize);
    let picks = use_signal(|| [0usize; 4]);
    let mut epoch = use_signal(|| 0u64);
    let error = use_signal(String::new);
    let mut adding = use_signal(|| false);
    let new_label = use_signal(String::new);
    let new_value = use_signal(String::new);

    let cursor_slot = use_signal(|| (EditorField::Body, 0usize));
    let editing_slot = use_signal(|| None::<(EditorField, usize)>);
    let mut pending_focus = use_signal(|| None::<(EditorField, usize)>);
    let mounted_slot = use_signal(|| None::<((EditorField, usize), Rc<MountedData>)>);
    let selected_block = use_signal(|| None::<(EditorField, usize)>);
    let hovered_tag = use_signal(|| None::<String>);
    let drag_payload = use_signal(|| None::<DragPayload>);
    let drop_target = use_signal(|| None::<DropPos>);

    let profile_vars = use_memo(move || ProfileVars::from_settings(&settings.read()));
    let datos_memo = use_memo(move || {
        tpl.parsed
            .read()
            .as_ref()
            .map(|t| t.datos.clone())
            .unwrap_or_default()
    });
    let tags_bundle =
        use_memo(move || Rc::new(build_tags_bundle(&profile_vars.read(), &datos_memo.read())));
    let datos_keys = use_memo(move || datos_memo.read().keys().cloned().collect::<Vec<String>>());

    use_effect(move || {
        let Some(target) = *pending_focus.read() else {
            return;
        };
        if let Some((slot_key, data)) = mounted_slot.read().as_ref().cloned()
            && slot_key == target
        {
            pending_focus.set(None);
            spawn(async move {
                let _ = data.set_focus(true).await;
            });
        }
    });

    let parsed_guard = tpl.parsed.read();
    let Some(t) = parsed_guard.as_ref() else {
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

    let bundle = tags_bundle.read();
    let cat = (*category.read()).min(CATEGORIES.len() - 1);
    let list = items_with_tags(t, cat, &bundle.tags);
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
    let cur_epoch = *epoch.read();
    let template_key: Rc<str> = Rc::from(current.key.as_str());
    let giro = if is_giro {
        t.giros.get(&current.key).cloned().unwrap_or_default()
    } else {
        BTreeMap::new()
    };

    let (body_doc, subj_doc, words, subj_len, giro_sample) = if is_giro {
        let sample = plain_sample_text(&resolve_preview_parts(
            &text_of(t, 0, "A"),
            &settings.peek(),
            t,
            Some(&giro),
        ));
        (
            Rc::new(TemplateDoc {
                slots: vec![String::new()],
                blocks: Vec::new(),
            }),
            None,
            0usize,
            0usize,
            sample,
        )
    } else {
        let body_raw = text_of(t, cat, &current.key);
        let b_doc = Rc::new(TemplateDoc::parse(&body_raw, &bundle.tags));
        let w = count_doc_words(&b_doc, &bundle.tags);
        if is_email {
            let subject_raw = email_subject_of(t, &current.key);
            let s_doc = Rc::new(TemplateDoc::parse(&subject_raw, &bundle.tags));
            let s_len = count_doc_chars(&s_doc, &bundle.tags);
            (b_doc, Some(s_doc), w, s_len, String::new())
        } else {
            (b_doc, None, w, 0, String::new())
        }
    };

    let body_raw_copy = if is_giro {
        String::new()
    } else {
        text_of(t, cat, &current.key)
    };
    let subject_raw_copy = if is_email {
        email_subject_of(t, &current.key)
    } else {
        String::new()
    };
    let body_slots_len = body_doc.slots.len();
    let removable = !is_giro && !REQUIRED.contains(&current.key.as_str());
    let duplicate_key = new_key(t, cat, &current.key);
    let total = list.len();

    let ed = EditorSignals {
        tpl,
        error,
        cursor_slot,
        editing_slot,
        pending_focus,
        mounted_slot,
        selected_block,
        hovered_tag,
        drag_payload,
        drop_target,
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
                                clear_editor_interaction(ed, body_slots_len);
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
                    ed,
                    body_slots_len,
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
                                body_raw: body_raw_copy,
                                subject_raw: subject_raw_copy,
                                total,
                                index,
                                duplicate_available: duplicate_key.is_some(),
                                removable,
                                tpl,
                                error,
                                picks,
                                ed,
                                body_slots_len,
                                epoch,
                            }
                        }
                    }

                    if !is_giro {
                        PalettePanel {
                            is_email,
                            cur_epoch,
                            category: cat,
                            template_key: template_key.clone(),
                            tags_bundle,
                            datos_keys,
                            ed,
                            adding,
                            new_label,
                            new_value,
                        }
                    }

                    UnifiedComposer {
                        cat,
                        is_giro,
                        is_email,
                        cur_epoch,
                        template_key,
                        giro,
                        body_doc,
                        subj_doc,
                        words,
                        subj_len,
                        giro_sample,
                        tags_bundle,
                        ed,
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
fn UnifiedComposer(
    cat: usize,
    is_giro: bool,
    is_email: bool,
    cur_epoch: u64,
    template_key: Rc<str>,
    giro: BTreeMap<String, String>,
    body_doc: Rc<TemplateDoc>,
    subj_doc: Option<Rc<TemplateDoc>>,
    words: usize,
    subj_len: usize,
    giro_sample: String,
    tags_bundle: Memo<Rc<TagsBundle>>,
    ed: EditorSignals,
) -> Element {
    let maybe_ui = try_use_context::<UiState>();

    if is_giro {
        return rsx! {
            GiroFields {
                cur_epoch,
                current_key: template_key.to_string(),
                giro,
                sample_body: giro_sample,
                tpl: ed.tpl,
                error: ed.error,
            }
        };
    }

    let max_words = if is_email {
        120
    } else if cat == 0 {
        65
    } else {
        80
    };
    let copy_key = template_key.clone();

    if is_email {
        let s_doc = subj_doc.unwrap_or_else(|| {
            Rc::new(TemplateDoc {
                slots: vec![String::new()],
                blocks: Vec::new(),
            })
        });
        return rsx! {
            div { class: "st-unified-card st-mail",
                div { class: "st-mail-subject-row",
                    span { class: "st-mail-subj-label muted", "Asunto" }
                    UnifiedFieldSurface {
                        key: "uf-subj-{cat}-{cur_epoch}",
                        field: EditorField::Subject,
                        single_line: true,
                        placeholder: "Escribe el asunto o arrastra un bloque…",
                        cur_epoch,
                        category: cat,
                        template_key: template_key.clone(),
                        doc: s_doc,
                        tags_bundle,
                        ed,
                    }
                    span {
                        class: if subj_len > 55 { "muted small st-warn-count" } else { "muted small st-count" },
                        "{subj_len}/60"
                    }
                }
                div { class: "st-mail-body",
                    UnifiedFieldSurface {
                        key: "uf-body-{cat}-{cur_epoch}",
                        field: EditorField::Body,
                        single_line: false,
                        placeholder: "Haz clic para escribir o arrastra un bloque…",
                        cur_epoch,
                        category: cat,
                        template_key,
                        doc: body_doc,
                        tags_bundle,
                        ed,
                    }
                }
                div { class: "st-card-foot",
                    span {
                        class: if words > max_words { "muted small st-warn-count" } else { "muted small" },
                        "{words} / {max_words} palabras"
                    }
                    if let Some(ui) = maybe_ui {
                        button {
                            class: "link-button small",
                            r#type: "button",
                            onclick: move |_| {
                                let tb = tags_bundle.peek();
                                let s = current_field_doc(&ed.tpl, cat, &copy_key, EditorField::Subject, &tb.tags);
                                let b = current_field_doc(&ed.tpl, cat, &copy_key, EditorField::Body, &tb.tags);
                                let payload = format!(
                                    "Asunto: {}\n\n{}",
                                    doc_to_sample_string(&s, &tb.tags),
                                    doc_to_sample_string(&b, &tb.tags),
                                );
                                super::super::copied(&payload, ui);
                            },
                            "Copiar ejemplo"
                        }
                    }
                }
            }
        };
    }

    rsx! {
        div { class: "st-unified-card st-bubble",
            UnifiedFieldSurface {
                key: "uf-body-{cat}-{cur_epoch}",
                field: EditorField::Body,
                single_line: false,
                placeholder: "Haz clic para escribir o arrastra un bloque…",
                cur_epoch,
                category: cat,
                template_key,
                doc: body_doc,
                tags_bundle,
                ed,
            }
            div { class: "st-card-foot",
                span {
                    class: if words > max_words { "muted small st-warn-count" } else { "muted small" },
                    "{words} / {max_words} palabras"
                }
                if let Some(ui) = maybe_ui {
                    button {
                        class: "link-button small",
                        r#type: "button",
                        onclick: move |_| {
                            let tb = tags_bundle.peek();
                            let b = current_field_doc(&ed.tpl, cat, &copy_key, EditorField::Body, &tb.tags);
                            let payload = doc_to_sample_string(&b, &tb.tags);
                            super::super::copied(&payload, ui);
                        },
                        "Copiar ejemplo"
                    }
                }
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
    ed: EditorSignals,
    body_slots_len: usize,
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
                            clear_editor_interaction(ed, body_slots_len);
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
    tpl: DraftState,
    error: Signal<String>,
    mut picks: Signal<[usize; 4]>,
    ed: EditorSignals,
    body_slots_len: usize,
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
                    let cached = tpl.parsed.peek().clone();
                    let Some(key) = cached
                        .or_else(|| Templates::parse_toml(&tpl.source.peek()).ok())
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
                    if let Some(fresh) = tpl.parsed.peek().as_ref() {
                        let empty_tags = BTreeMap::new();
                        let position = items_with_tags(fresh, cat, &empty_tags)
                            .iter()
                            .position(|item| item.key == created)
                            .unwrap_or(total);
                        picks.write()[cat] = position;
                    }
                    clear_editor_interaction(ed, body_slots_len);
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
                    clear_editor_interaction(ed, body_slots_len);
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
    sample_body: String,
    tpl: DraftState,
    error: Signal<String>,
) -> Element {
    rsx! {
        div { key: "giro-{current_key}-{cur_epoch}", class: "st-giro",
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
            div { class: "st-giro-sample",
                span { class: "muted small", "Muestra en WhatsApp:" }
                p { class: "small", "{sample_body}" }
            }
        }
    }
}

#[component]
fn PaletteInfoBar(
    hovered_tag: Signal<Option<String>>,
    tags_bundle: Memo<Rc<TagsBundle>>,
) -> Element {
    let key_guard = hovered_tag.read();
    let bundle = tags_bundle.read();
    let inspected_tag = key_guard.as_deref().and_then(|key| bundle.tags.get(key));

    rsx! {
        div { class: "st-chip-info",
            if let Some(info) = inspected_tag {
                strong { "{info.label}" }
                span { class: "muted", " — {info.description} " }
                span { class: "st-chip-sample", "Ej: «{info.sample}»" }
            } else {
                span { class: "muted",
                    "Clic para añadir · arrastra en el mensaje para mover · pasa el cursor sobre un bloque para ver cuál es."
                }
            }
        }
    }
}

#[component]
fn PalettePanel(
    is_email: bool,
    cur_epoch: u64,
    category: usize,
    template_key: Rc<str>,
    tags_bundle: Memo<Rc<TagsBundle>>,
    datos_keys: Memo<Vec<String>>,
    mut ed: EditorSignals,
    mut adding: Signal<bool>,
    mut new_label: Signal<String>,
    mut new_value: Signal<String>,
) -> Element {
    let bundle = tags_bundle.read();
    let existing_keys = datos_keys.read();
    let is_adding = *adding.read();
    let (label_draft, datum_error) = if is_adding {
        let draft = new_label.read().clone();
        let err = if draft.trim().is_empty() {
            None
        } else {
            new_datum_error_in_keys(existing_keys.iter(), draft.trim())
        };
        (draft, err)
    } else {
        (String::new(), None)
    };

    rsx! {
        div { class: "st-palette",
            div { class: "st-palette-groups",
                for (group_title, is_own_group, group_tags) in [
                    ("Empresa", false, bundle.biz_tags.as_slice()),
                    ("Tu perfil", true, bundle.own_tags.as_slice()),
                ] {
                    div { class: "st-chip-group", key: "grp-{group_title}",
                        span { class: "st-chips-label", "{group_title}" }
                        div { class: "st-chips",
                            for tag in group_tags {
                                {
                                    let tag_key_hover = tag.key.clone();
                                    let tag_key_leave = tag.key.clone();
                                    let tag_key_focus = tag.key.clone();
                                    let tag_key_drag = tag.key.clone();
                                    let tag_key_click = tag.key.clone();
                                    let drag_label = tag.label.clone();
                                    let tkey_click = template_key.clone();
                                    rsx! {
                                        button {
                                            key: "chip-{tag.key}",
                                            class: if tag.own { "st-chip own" } else { "st-chip biz" },
                                            r#type: "button",
                                            draggable: "true",
                                            title: "{tag.description} Ej: «{tag.sample}»",
                                            onmouseenter: move |_| set_hovered_tag(ed.hovered_tag, &tag_key_hover),
                                            onmouseleave: move |_| clear_hovered_tag(ed.hovered_tag, &tag_key_leave),
                                            onfocus: move |_| set_hovered_tag(ed.hovered_tag, &tag_key_focus),
                                            ondragstart: move |e: DragEvent| {
                                                e.data_transfer().set_effect_allowed("copy");
                                                let _ = e.data_transfer().set_data("text/plain", &drag_label);
                                                ed.drag_payload.set(Some(DragPayload::AddKey(tag_key_drag.clone())));
                                            },
                                            ondragend: move |_| {
                                                ed.drag_payload.set(None);
                                                ed.drop_target.set(None);
                                            },
                                            onclick: move |_| {
                                                let (last_field, last_slot) = *ed.cursor_slot.peek();
                                                let target_field = if is_email {
                                                    last_field
                                                } else {
                                                    EditorField::Body
                                                };
                                                let tb = tags_bundle.peek();
                                                let mut doc = current_field_doc(&ed.tpl, category, &tkey_click, target_field, &tb.tags);
                                                let slot_idx = if target_field == last_field {
                                                    last_slot.min(doc.slots.len().saturating_sub(1))
                                                } else {
                                                    doc.slots.len().saturating_sub(1)
                                                };
                                                let byte_end = doc.slots[slot_idx].len();
                                                let next_slot = doc.insert_block_at(
                                                    slot_idx,
                                                    byte_end,
                                                    tag_key_click.clone(),
                                                );
                                                ed.selected_block.set(None);
                                                ed.cursor_slot.set((target_field, next_slot));
                                                ed.editing_slot.set(Some((target_field, next_slot)));
                                                ed.pending_focus.set(Some((target_field, next_slot)));
                                                write_template_field(
                                                    ed,
                                                    category,
                                                    &tkey_click,
                                                    target_field,
                                                    doc.to_raw(),
                                                );
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
                                    aria_expanded: is_adding,
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
            PaletteInfoBar {
                hovered_tag: ed.hovered_tag,
                tags_bundle,
            }
        }
        if is_adding {
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
                    span { class: "muted small", "Queda disponible para todas tus plantillas." }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_tags() -> BTreeMap<String, ResolvedTag> {
        let bundled = Templates::bundled_source();
        let mut t = Templates::parse_toml(bundled).unwrap();
        t.datos
            .insert("Mi empresa".to_owned(), "Soluciones del Valle".to_owned());
        let settings = Settings {
            owner_name: "Tu nombre".into(),
            ..Settings::default()
        };
        palette_tags(&t, &settings)
            .into_iter()
            .map(|tag| (tag.key.clone(), tag))
            .collect()
    }

    #[test]
    fn unified_slots_roundtrip_move_insert_delete_and_expand() {
        let tags = test_tags();
        assert_eq!(tags["owner"].sample, "Sofía Méndez");

        let raw = "Hola, soy {owner}. Visito a {nombre} por {cosa}.";
        let doc = TemplateDoc::parse(raw, &tags);
        assert_eq!(doc.blocks, vec!["owner", "nombre", "cosa"]);
        assert_eq!(doc.slots, vec!["Hola, soy ", ". Visito a ", " por ", "."]);
        assert_eq!(doc.to_raw(), raw);

        // Removing a block before punctuation collapses spaces cleanly.
        let mut trimmed = doc.clone();
        trimmed.remove_block(0);
        assert_eq!(trimmed.to_raw(), "Hola, soy. Visito a {nombre} por {cosa}.");

        // Moving a block earlier inside an existing slot keeps clean spacing.
        let mut moved_left = doc.clone();
        let visito_offset = " ".len();
        moved_left.move_block(1, 1, visito_offset);
        assert_eq!(
            moved_left.to_raw(),
            "Hola, soy {owner}. {nombre} Visito a por {cosa}."
        );

        // Moving a block later into the final slot moves cleanly without offset drift.
        let mut moved_right = doc.clone();
        moved_right.move_block(0, 3, 0);
        assert_eq!(
            moved_right.to_raw(),
            "Hola, soy. Visito a {nombre} por {cosa} {owner}."
        );

        // Keyboard nudge swaps adjacent word boundaries cleanly.
        let mut nudged = TemplateDoc::parse("Hola {owner} de México", &tags);
        nudged.nudge_block(0, true);
        assert_eq!(nudged.to_raw(), "Hola de {owner} México");
        nudged.nudge_block(0, false);
        assert_eq!(nudged.to_raw(), "Hola {owner} de México");

        // Typing or pasting raw braces or friendly labels turns straight into blocks.
        let mut datos = BTreeMap::new();
        datos.insert("Mi empresa".to_owned(), "Soluciones del Valle".to_owned());
        let pairs = tag_pairs(&datos);
        let mut pasted = TemplateDoc::parse("Soy [Nombre completo] en {mi_empresa}.", &tags);
        // Raw braces parse immediately on load; brackets expand inline inside slots.
        assert_eq!(pasted.blocks, vec!["mi_empresa"]);
        let landed = pasted.expand_inline_tokens(0, &pairs, &tags);
        assert_eq!(landed, Some(1));
        assert_eq!(pasted.to_raw(), "Soy {owner} en {mi_empresa}.");
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
        let text = plain_sample_text(&resolve_preview_parts(
            "Soy {owner} de {mi_empresa}.",
            &settings,
            &t,
            None,
        ));
        assert_eq!(text, "Soy Sofía Méndez de Soluciones.");

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
        let tpl_source = use_signal(|| bundled.to_owned());
        let tpl_parsed = use_signal(|| Some(t.clone()));
        let tpl = DraftState {
            source: tpl_source,
            parsed: tpl_parsed,
        };
        let error = use_signal(String::new);
        let cursor_slot = use_signal(|| (EditorField::Body, 0usize));
        let editing_slot = use_signal(|| None::<(EditorField, usize)>);
        let pending_focus = use_signal(|| None::<(EditorField, usize)>);
        let mounted_slot = use_signal(|| None::<((EditorField, usize), Rc<MountedData>)>);
        let selected_block = use_signal(|| None::<(EditorField, usize)>);
        let hovered_tag = use_signal(|| None::<String>);
        let drag_payload = use_signal(|| None::<DragPayload>);
        let drop_target = use_signal(|| None::<DropPos>);
        let adding = use_signal(|| false);
        let new_label = use_signal(String::new);
        let new_value = use_signal(String::new);

        let settings_for_memo = settings.clone();
        let tags_bundle = use_memo(move || {
            let vars = ProfileVars::from_settings(&settings_for_memo);
            let datos = tpl
                .parsed
                .read()
                .as_ref()
                .map(|tr| tr.datos.clone())
                .unwrap_or_default();
            Rc::new(build_tags_bundle(&vars, &datos))
        });
        let datos_keys = use_memo(move || {
            tpl.parsed
                .read()
                .as_ref()
                .map(|tr| tr.datos.keys().cloned().collect())
                .unwrap_or_default()
        });
        let bundle = tags_bundle.read();
        let subject_raw = email_subject_of(&t, "cuerpo_con_credencial");
        let body_raw = text_of(&t, 1, "cuerpo_con_credencial");
        let subj_doc = Rc::new(TemplateDoc::parse(&subject_raw, &bundle.tags));
        let body_doc = Rc::new(TemplateDoc::parse(&body_raw, &bundle.tags));
        let words = count_doc_words(&body_doc, &bundle.tags);
        let subj_len = count_doc_chars(&subj_doc, &bundle.tags);
        assert_eq!(
            words,
            plain_sample_text(&resolve_preview_parts(&body_raw, &settings, &t, None))
                .split_whitespace()
                .count()
        );
        assert_eq!(
            subj_len,
            plain_sample_text(&resolve_preview_parts(&subject_raw, &settings, &t, None))
                .chars()
                .count()
        );
        assert_eq!(
            doc_to_sample_string(&body_doc, &bundle.tags),
            plain_sample_text(&resolve_preview_parts(&body_raw, &settings, &t, None))
        );

        let ed = EditorSignals {
            tpl,
            error,
            cursor_slot,
            editing_slot,
            pending_focus,
            mounted_slot,
            selected_block,
            hovered_tag,
            drag_payload,
            drop_target,
        };
        let tkey: Rc<str> = Rc::from("cuerpo_con_credencial");
        rsx! {
            PalettePanel {
                is_email: true,
                cur_epoch: 0,
                category: 1,
                template_key: tkey.clone(),
                tags_bundle,
                datos_keys,
                ed,
                adding,
                new_label,
                new_value,
            }
            UnifiedComposer {
                cat: 1,
                is_giro: false,
                is_email: true,
                cur_epoch: 0,
                template_key: tkey,
                giro: BTreeMap::new(),
                body_doc,
                subj_doc: Some(subj_doc),
                words,
                subj_len,
                giro_sample: String::new(),
                tags_bundle,
                ed,
            }
        }
    }

    #[test]
    fn unified_email_composer_renders_subject_and_body_with_interactive_sample_pills() {
        let mut dom = VirtualDom::new(EmailPaneProbe);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("st-unified-card st-mail"));
        assert!(html.contains("st-mail-subject-row"));
        assert!(html.contains("Asunto"));
        assert!(html.contains("st-token-pill biz"));
        assert!(html.contains("st-token-pill own"));
        assert!(html.contains("st-pill-del"));
        assert!(html.contains("órdenes y refacciones"));
        assert!(html.contains("Sofía Méndez"));
        assert!(html.contains("Bloque: Diferenciador"));
        assert!(html.contains("Quitar bloque Nombre completo"));
        assert!(html.contains("Portafolio"));
        assert!(html.contains("LinkedIn"));
        assert!(html.contains("Página web"));
        assert!(!html.contains("st-split-work"));
        assert!(!html.contains("Vista previa en vivo"));
        assert!(!html.contains("{cosa}"));
        assert!(!html.contains("{credencial}"));
    }
}
