//! One canvas for every message template: pick it on the left, write or preview on the right.
use super::*;
use prospect_core::messages::{RESERVED_KEYS, data_key};

/// Application data available to every template: (placeholder key, visible label, is yours).
const BUILTIN_TAGS: [(&str, &str, bool); 10] = [
    ("nombre", "Nombre del negocio", false),
    ("giro_corto", "Giro del negocio", false),
    ("cosa", "Proceso del giro", false),
    ("pregunta", "Pregunta sugerida", false),
    ("ejemplo", "Ejemplo práctico", false),
    ("obs", "Hallazgo web", false),
    ("owner", "Tu nombre", true),
    ("owner_corto", "Tu primer nombre", true),
    ("credencial", "Tu credencial", true),
    ("whatsapp", "Tu WhatsApp", true),
];

const CATEGORIES: [&str; 5] = [
    "WhatsApp",
    "Asuntos de correo",
    "Correos",
    "Seguimientos",
    "Giros",
];
const REQUIRED: [&str; 11] = [
    "A",
    "B",
    "C",
    "D",
    "asunto_A",
    "asunto_B",
    "cuerpo_con_credencial",
    "cuerpo_sin_credencial",
    "d3",
    "d7",
    "d14",
];

/// (`{key}`, `[Etiqueta]`) for built-in data plus the user's reusable data.
pub(super) fn tag_pairs(datos: &BTreeMap<String, String>) -> Vec<(String, String)> {
    BUILTIN_TAGS
        .iter()
        .map(|(key, label, _)| (format!("{{{key}}}"), format!("[{label}]")))
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

#[derive(Clone, PartialEq)]
struct Item {
    key: String,
    label: String,
    hint: &'static str,
}

fn stage(key: &str) -> (usize, &'static str) {
    if key == "d3" || key.starts_with("d3_") {
        (0, "Día 3 ·")
    } else if key == "d7" || key.starts_with("d7_") {
        (1, "Día 7 ·")
    } else if key == "d14" || key.starts_with("d14_") {
        (2, "Día 14 ·")
    } else {
        (3, "Seguimiento ·")
    }
}

fn numbered(keys: Vec<(String, &'static str, &'static str)>) -> Vec<Item> {
    let mut seen = BTreeMap::<&'static str, usize>::new();
    keys.into_iter()
        .map(|(key, group, hint)| {
            let n = seen.entry(group).or_default();
            *n += 1;
            Item {
                key,
                label: format!("{group} {n}"),
                hint,
            }
        })
        .collect()
}

fn items(t: &Templates, category: usize) -> Vec<Item> {
    match category {
        0 => numbered(
            t.whatsapp
                .keys()
                .map(|k| {
                    let hint = if k == "C" {
                        "Sólo con hallazgo web"
                    } else {
                        ""
                    };
                    (k.clone(), "Mensaje", hint)
                })
                .collect(),
        ),
        1 => numbered(
            t.email
                .keys()
                .filter(|k| k.starts_with("asunto_"))
                .map(|k| (k.clone(), "Asunto", ""))
                .collect(),
        ),
        2 => {
            let mut keys: Vec<&String> = t
                .email
                .keys()
                .filter(|k| !k.starts_with("asunto_"))
                .collect();
            keys.sort_by_key(|k| (!k.starts_with("cuerpo_con"), (*k).clone()));
            numbered(
                keys.into_iter()
                    .map(|k| {
                        if k.starts_with("cuerpo_con") {
                            (k.clone(), "Con credencial", "")
                        } else {
                            (k.clone(), "Directo", "")
                        }
                    })
                    .collect(),
            )
        }
        3 => {
            let mut keys: Vec<&String> = t.seguimiento.keys().collect();
            keys.sort_by_key(|k| (stage(k).0, (*k).clone()));
            numbered(
                keys.into_iter()
                    .map(|k| (k.clone(), stage(k).1, ""))
                    .collect(),
            )
        }
        _ => t
            .giros
            .keys()
            .map(|k| Item {
                key: k.clone(),
                label: k.clone(),
                hint: "",
            })
            .collect(),
    }
}

fn section(t: &mut Templates, category: usize) -> &mut BTreeMap<String, String> {
    match category {
        0 => &mut t.whatsapp,
        3 => &mut t.seguimiento,
        _ => &mut t.email,
    }
}

fn text_of(t: &Templates, category: usize, key: &str) -> String {
    match category {
        0 => t.whatsapp.get(key),
        3 => t.seguimiento.get(key),
        _ => t.email.get(key),
    }
    .cloned()
    .unwrap_or_default()
}

/// Key for a new variant that the message picker will rotate in next to `like`.
fn new_key(t: &Templates, category: usize, like: &str) -> Option<String> {
    let taken = |key: &str| match category {
        0 => t.whatsapp.contains_key(key),
        3 => t.seguimiento.contains_key(key),
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
        1 => numbered("asunto_"),
        2 => numbered(if like.starts_with("cuerpo_con") {
            "cuerpo_con_"
        } else {
            "cuerpo_sin_"
        }),
        3 => numbered(&format!("{}_", like.split('_').next().unwrap_or("d3"))),
        _ => None,
    }
}

/// Renders a template with sample business data and the user's own profile.
fn sample(
    raw: &str,
    settings: &Settings,
    t: &Templates,
    giro: Option<&BTreeMap<String, String>>,
) -> String {
    let or = |value: &str, fallback: &str| {
        if value.trim().is_empty() {
            fallback.to_owned()
        } else {
            value.trim().to_owned()
        }
    };
    let owner = or(&settings.owner_name, "Sofía Méndez");
    let from_giro = |field: &str, fallback: &str| {
        giro.and_then(|g| g.get(field))
            .cloned()
            .unwrap_or_else(|| fallback.to_owned())
    };
    let mut out = raw
        .replace("{nombre}", "Taller Industrial del Valle")
        .replace(
            "{owner_corto}",
            owner.split_whitespace().next().unwrap_or(&owner),
        )
        .replace("{owner}", &owner)
        .replace(
            "{credencial}",
            &or(
                &settings.owner_credential,
                "Especialista en automatización y sistemas internos.",
            ),
        )
        .replace("{whatsapp}", &or(&settings.owner_whatsapp, "55 0123 4567"))
        .replace("{giro_corto}", "taller industrial")
        .replace("{cosa}", &from_giro("cosa", "órdenes y refacciones"))
        .replace(
            "{pregunta}",
            &from_giro(
                "pregunta",
                "¿Cómo llevan el seguimiento de las órdenes desde que entran hasta que se entregan?",
            ),
        )
        .replace(
            "{ejemplo}",
            &from_giro(
                "ejemplo",
                "un control de órdenes, estatus y refacciones asociadas",
            ),
        )
        .replace("{obs}", "Vi su sitio web.");
    for (label, value) in &t.datos {
        out = out.replace(&format!("{{{}}}", data_key(label)), &or(value, label));
    }
    out
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
            .any(|(_, builtin, _)| builtin.eq_ignore_ascii_case(label))
        || t.datos.keys().any(|existing| data_key(existing) == key)
    {
        Some("Ya existe un dato con ese nombre.")
    } else {
        None
    }
}

#[component]
pub(super) fn TemplateCanvas(tpl: Signal<String>, settings: Settings) -> Element {
    let home = consume_context::<Services>().db.paths.home;
    let mut ui = use_context::<UiState>();
    let mut preview = use_signal(|| false);
    let mut category = use_signal(|| 0usize);
    let mut picks = use_signal(|| [0usize; 5]);
    let mut epoch = use_signal(|| 0u64);
    let error = use_signal(String::new);
    let mut adding = use_signal(|| false);
    let mut new_label = use_signal(String::new);
    let mut new_value = use_signal(String::new);

    let Ok(t) = Templates::parse_toml(&tpl.read()) else {
        return rsx! {
            section { class: "panel st-tpl",
                div { class: "panel-heading", h2 { "Plantillas de mensajes" } }
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
    let list = items(&t, cat);
    let index = picks.read()[cat].min(list.len().saturating_sub(1));
    let current = list.get(index).cloned().unwrap_or(Item {
        key: String::new(),
        label: "Sin plantillas".into(),
        hint: "",
    });
    let pairs = tag_pairs(&t.datos);
    let showing_preview = *preview.read();
    let is_giro = cat == 4;
    let raw = if is_giro {
        String::new()
    } else {
        text_of(&t, cat, &current.key)
    };
    let removable = !is_giro && !REQUIRED.contains(&current.key.as_str());
    let cur_epoch = *epoch.read();
    let giro = t.giros.get(&current.key).cloned().unwrap_or_default();

    let (subject, body) = match cat {
        1 => (
            Some(sample(&raw, &settings, &t, None)),
            sample(
                &text_of(&t, 2, "cuerpo_sin_credencial"),
                &settings,
                &t,
                None,
            ),
        ),
        2 => (
            Some(sample(&text_of(&t, 1, "asunto_A"), &settings, &t, None)),
            sample(&raw, &settings, &t, None),
        ),
        4 => (
            None,
            sample(&text_of(&t, 0, "A"), &settings, &t, Some(&giro)),
        ),
        _ => (None, sample(&raw, &settings, &t, None)),
    };

    let chips: Vec<(String, bool)> = BUILTIN_TAGS
        .iter()
        .map(|(_, label, own)| ((*label).to_owned(), *own))
        .chain(t.datos.keys().map(|label| (label.clone(), true)))
        .collect();
    let duplicate_key = new_key(&t, cat, &current.key);
    let label_draft = new_label.read().clone();
    let datum_error = if label_draft.trim().is_empty() {
        None
    } else {
        new_datum_error(&t, label_draft.trim())
    };

    rsx! {
        section { class: "panel st-tpl",
            div { class: "st-tpl-bar",
                Segmented {
                    name: "Modo del lienzo",
                    options: vec![("edit".to_owned(), "Editar"), ("view".to_owned(), "Vista previa")],
                    value: if showing_preview { "view" } else { "edit" },
                    onpick: move |mode: String| preview.set(mode == "view"),
                }
                div { class: "st-tpl-title",
                    strong { "{current.label}" }
                    span { class: "muted small",
                        if current.hint.is_empty() { "{CATEGORIES[cat]}" } else { "{CATEGORIES[cat]} · {current.hint}" }
                    }
                }
                if !is_giro {
                    div { class: "st-tpl-actions",
                        {
                            let source_key = current.key.clone();
                            let copy = raw.clone();
                            let total = list.len();
                            rsx! {
                                button {
                                    class: "link-button small",
                                    r#type: "button",
                                    disabled: duplicate_key.is_none(),
                                    title: "Crea otra variante a partir de esta",
                                    onclick: move |_| {
                                        let Some(key) = Templates::parse_toml(&tpl.peek())
                                            .ok()
                                            .and_then(|fresh| new_key(&fresh, cat, &source_key))
                                        else {
                                            return;
                                        };
                                        let text = copy.clone();
                                        let created = key.clone();
                                        mutate(tpl, error, move |t| {
                                            section(t, cat).insert(key, text);
                                        });
                                        if let Ok(fresh) = Templates::parse_toml(&tpl.peek()) {
                                            let position = items(&fresh, cat)
                                                .iter()
                                                .position(|item| item.key == created)
                                                .unwrap_or(total);
                                            picks.write()[cat] = position;
                                        }
                                        preview.set(false);
                                        epoch += 1;
                                    },
                                    "+ Nueva variante"
                                }
                            }
                        }
                        {
                            let remove_key = current.key.clone();
                            rsx! {
                                button {
                                    class: "link-button small st-danger",
                                    r#type: "button",
                                    disabled: !removable,
                                    title: if removable { "Eliminar esta variante" } else { "Las plantillas base no se eliminan; puedes reescribirlas" },
                                    onclick: move |_| {
                                        mutate(tpl, error, |t| {
                                            section(t, cat).remove(&remove_key);
                                        });
                                        picks.write()[cat] = index.saturating_sub(1);
                                        epoch += 1;
                                    },
                                    "Eliminar"
                                }
                            }
                        }
                    }
                }
            }
            div { class: "st-tpl-main",
                nav { class: "st-tpl-list", aria_label: "Plantillas",
                    select {
                        aria_label: "Tipo de plantilla",
                        value: cat.to_string(),
                        onchange: move |e| {
                            if let Ok(next) = e.value().parse::<usize>() {
                                category.set(next.min(CATEGORIES.len() - 1));
                                adding.set(false);
                                epoch += 1;
                            }
                        },
                        for (i, name) in CATEGORIES.iter().enumerate() {
                            option { value: i.to_string(), selected: i == cat, "{name}" }
                        }
                    }
                    div { class: "st-tpl-items",
                        for (i, item) in list.iter().enumerate() {
                            button {
                                key: "{cat}-{item.key}",
                                class: if i == index { "st-tpl-item active" } else { "st-tpl-item" },
                                r#type: "button",
                                aria_current: i == index,
                                onclick: move |_| {
                                    picks.write()[cat] = i;
                                    epoch += 1;
                                },
                                span { "{item.label}" }
                                if !item.hint.is_empty() {
                                    span { class: "muted", "{item.hint}" }
                                }
                            }
                        }
                    }
                }
                div { class: "st-canvas",
                    if showing_preview {
                        div { class: "st-preview",
                            if let Some(subject) = subject {
                                div { class: "st-mail",
                                    p { class: "st-mail-subject", span { class: "muted", "Asunto  " } "{subject}" }
                                    p { class: "message-preview", "{body}" }
                                }
                            } else {
                                div { class: "st-bubble", p { class: "message-preview", "{body}" } }
                            }
                            p { class: "muted small",
                                if is_giro {
                                    "Así se lee el primer mensaje de WhatsApp para un negocio de este giro. Datos de muestra."
                                } else if cat == 1 {
                                    "El asunto se muestra con un correo de ejemplo. Datos de muestra."
                                } else {
                                    "Ejemplo con un negocio de muestra y tus datos de Perfil."
                                }
                            }
                        }
                    } else if is_giro {
                        div { key: "giro-{current.key}-{cur_epoch}", class: "st-giro",
                            p { class: "muted small",
                                "Estos tres datos se insertan en tus mensajes cuando el negocio es de este giro."
                            }
                            for (field, title, rows) in [("cosa", "Proceso del giro", 1usize), ("pregunta", "Pregunta sugerida", 2), ("ejemplo", "Ejemplo práctico", 2)] {
                                {
                                    let giro_key = current.key.clone();
                                    rsx! {
                                        label {
                                            "{title}"
                                            TextEntry {
                                                multiline: rows > 1,
                                                rows,
                                                initial: giro.get(field).cloned().unwrap_or_default(),
                                                oninput: move |e: FormEvent| mutate(tpl, error, |t| {
                                                    t.giros.entry(giro_key.clone()).or_default().insert(field.into(), e.value());
                                                }),
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "st-chips",
                            span { class: "st-chips-label", "Insertar dato" }
                            for (label, own) in chips {
                                {
                                    let key = current.key.clone();
                                    let base = raw.clone();
                                    let insert_pairs = pairs.clone();
                                    let tag = format!("[{label}]");
                                    rsx! {
                                        button {
                                            key: "chip-{label}",
                                            class: if own { "st-chip own" } else { "st-chip" },
                                            r#type: "button",
                                            title: "Insertar al final del texto",
                                            onclick: move |_| {
                                                let mut text = to_friendly_text(&base, &insert_pairs);
                                                if !text.is_empty() && !text.ends_with([' ', '\n']) {
                                                    text.push(' ');
                                                }
                                                text.push_str(&tag);
                                                let value = from_friendly_text(&text, &insert_pairs);
                                                let key = key.clone();
                                                mutate(tpl, error, move |t| {
                                                    section(t, cat).insert(key, value);
                                                });
                                                epoch += 1;
                                            },
                                            "{label}"
                                        }
                                    }
                                }
                            }
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
                                        mutate(tpl, error, |t| {
                                            t.datos.insert(label, value);
                                        });
                                        new_label.set(String::new());
                                        new_value.set(String::new());
                                        adding.set(false);
                                        epoch += 1;
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
                        {
                            let key = current.key.clone();
                            let edit_pairs = pairs.clone();
                            rsx! {
                                TextEntry {
                                    key: "tpl-{cat}-{current.key}-{cur_epoch}",
                                    multiline: true,
                                    class: "st-canvas-text",
                                    label: format!("Texto de {}", current.label),
                                    rows: if cat == 1 { 2 } else { 8 },
                                    initial: to_friendly_text(&raw, &pairs),
                                    oninput: move |e: FormEvent| {
                                        let value = from_friendly_text(&e.value(), &edit_pairs);
                                        let key = key.clone();
                                        mutate(tpl, error, move |t| {
                                            section(t, cat).insert(key, value);
                                        });
                                    },
                                }
                            }
                        }
                        p { class: "muted small st-note",
                            "Lo que va entre [corchetes] se reemplaza solo en cada mensaje. Deja una línea en blanco para separar párrafos."
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendly_tags_roundtrip_bundled_templates_and_reusable_data() {
        let bundled = Templates::bundled_source();
        let mut datos = BTreeMap::new();
        datos.insert("Mi empresa".to_owned(), "Soluciones del Valle".to_owned());
        let pairs = tag_pairs(&datos);
        let friendly = to_friendly_text(bundled, &pairs);
        assert!(!friendly.contains("{nombre}"));
        assert!(!friendly.contains("{owner_corto}"));
        assert_eq!(from_friendly_text(&friendly, &pairs), bundled);
        assert_eq!(
            from_friendly_text("Soy [Tu nombre] de [Mi empresa].", &pairs),
            "Soy {owner} de {mi_empresa}."
        );
    }

    #[test]
    fn new_variants_join_the_rotation_and_reusable_data_fills_previews() {
        let mut t = Templates::parse_toml(Templates::bundled_source()).unwrap();
        assert_eq!(new_key(&t, 0, "A").as_deref(), Some("K"));
        assert_eq!(new_key(&t, 1, "asunto_A").as_deref(), Some("asunto_11"));
        assert_eq!(
            new_key(&t, 2, "cuerpo_sin_credencial").as_deref(),
            Some("cuerpo_sin_11")
        );
        assert_eq!(new_key(&t, 3, "d7_02").as_deref(), Some("d7_11"));
        assert_eq!(new_key(&t, 4, "Otro"), None);
        assert_eq!(items(&t, 3)[0].label, "Día 3 · 1");
        assert!(new_datum_error(&t, "Tu nombre").is_some());
        assert!(new_datum_error(&t, "owner").is_some());
        assert!(new_datum_error(&t, "Mi empresa").is_none());
        t.datos.insert("Mi empresa".into(), "Soluciones".into());
        assert!(new_datum_error(&t, "mi  empresa").is_some());
        t.validate().unwrap();
        let text = sample(
            "Soy {owner} de {mi_empresa}.",
            &Settings::default(),
            &t,
            None,
        );
        assert_eq!(text, "Soy Tu nombre de Soluciones.");
    }
}
