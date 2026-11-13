use super::*;
use crate::onboarding::OnboardingControl;
use prospect_core::{config::AutoUpdatePeriod, messages::Templates};
use std::collections::BTreeMap;

mod scope;
mod templates;
mod tokens;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum HelpTopic {
    Profile,
    Templates,
    Scope,
    Base,
    Tokens,
    Backup,
}

#[derive(Clone, Copy)]
struct HelpContext {
    topic: Signal<Option<HelpTopic>>,
}

#[component]
pub(super) fn HelpButton(topic: HelpTopic, label: &'static str) -> Element {
    let mut help = try_use_context::<HelpContext>();
    rsx! {
        button {
            class: "st-help-btn",
            r#type: "button",
            title: "Ayuda sobre {label}",
            aria_label: "Ayuda sobre {label}",
            onclick: move |_| {
                if let Some(ref mut ctx) = help {
                    ctx.topic.set(Some(topic));
                }
            },
            "?"
        }
    }
}

#[component]
pub(super) fn InlineTag(label: &'static str, own: bool) -> Element {
    rsx! {
        span { class: if own { "st-inline-pill own" } else { "st-inline-pill biz" },
            span { class: "st-chip-dot", "aria-hidden": "true" }
            "{label}"
        }
    }
}

#[derive(Clone, Copy)]
enum HelpSegment {
    Text(&'static str),
    Tag(&'static str, bool),
}

struct HelpCopy {
    title: &'static str,
    lead: &'static str,
    points: &'static [&'static [HelpSegment]],
    note: &'static str,
}

const PROFILE_POINT_1: &[HelpSegment] = &[
    HelpSegment::Text("Tus datos en plantillas: llenan los bloques "),
    HelpSegment::Tag("Nombre completo", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("Primer nombre", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("Número WhatsApp", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("Diferenciador", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("Portafolio", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("LinkedIn", true),
    HelpSegment::Text(", "),
    HelpSegment::Tag("Página web", true),
    HelpSegment::Text(" y cualquier dato extra que crees."),
];
const PROFILE_POINT_2: &[HelpSegment] = &[
    HelpSegment::Text(
        "Diferenciador: es una frase breve que explica qué problema resuelves o qué beneficio concreto ofreces. Lo insertas en cualquier plantilla con ",
    ),
    HelpSegment::Tag("Diferenciador", true),
    HelpSegment::Text("."),
];
const PROFILE_POINT_3: &[HelpSegment] = &[HelpSegment::Text(
    "Metas diarias y apariencia: tus metas ajustan el progreso diario; el modo claro/oscuro se aplica y guarda al instante.",
)];
const PROFILE_POINTS: &[&[HelpSegment]] = &[PROFILE_POINT_1, PROFILE_POINT_2, PROFILE_POINT_3];

const TPL_POINT_1: &[HelpSegment] = &[HelpSegment::Text(
    "Pestañas superiores: cambia entre WhatsApp, Correos (cada correo edita su Asunto y su Mensaje juntos), Seguimientos y Giros.",
)];
const TPL_POINT_2: &[HelpSegment] = &[HelpSegment::Text(
    "Edición a la izquierda y Vista previa a la derecha: mientras editas ves de forma simultánea el mensaje resuelto con cada bloque resaltado.",
)];
const TPL_POINT_3: &[HelpSegment] = &[HelpSegment::Text(
    "Bloques accesibles: haz clic en la paleta para añadir un bloque o arrástralo al lugar exacto del texto. Una pulsación de Retroceso quita un bloque completo.",
)];
const TPL_POINTS: &[&[HelpSegment]] = &[TPL_POINT_1, TPL_POINT_2, TPL_POINT_3];

const SCOPE_POINT_1: &[HelpSegment] = &[HelpSegment::Text(
    "Estados: marca uno o varios estados. Si agregas estados nuevos y ya tienes token DENUE, al guardar puede iniciar la descarga.",
)];
const SCOPE_POINT_2: &[HelpSegment] = &[HelpSegment::Text(
    "Tamaño del negocio: filtra según personal ocupado de acuerdo con la oferta seleccionada (Automatización o Web y marketing).",
)];
const SCOPE_POINT_3: &[HelpSegment] = &[HelpSegment::Text(
    "Actualización automática: elige revisión semanal, quincenal, mensual o sólo manual.",
)];
const SCOPE_POINTS: &[&[HelpSegment]] = &[SCOPE_POINT_1, SCOPE_POINT_2, SCOPE_POINT_3];

const BASE_POINT_1: &[HelpSegment] = &[HelpSegment::Text(
    "Por lotes sin perder avance: cada clic en «Actualizar todo» o «Actualizar» descarga el presupuesto seguro y guarda un punto de reanudación.",
)];
const BASE_POINT_2: &[HelpSegment] = &[HelpSegment::Text(
    "Notas y CRM protegidos: actualizar datos públicos de DENUE nunca borra tus notas, estados ni historial de contactos.",
)];
const BASE_POINT_3: &[HelpSegment] = &[HelpSegment::Text(
    "Historial: consulta cuántas altas, cambios o bajas hubo en cada corrida real.",
)];
const BASE_POINTS: &[&[HelpSegment]] = &[BASE_POINT_1, BASE_POINT_2, BASE_POINT_3];

const TOKENS_POINT_1: &[HelpSegment] = &[HelpSegment::Text(
    "DENUE (INEGI): obligatorio para descargar el directorio oficial de establecimientos por estado.",
)];
const TOKENS_POINT_2: &[HelpSegment] = &[HelpSegment::Text(
    "INEGI Indicadores y Banxico SIE: opcionales; enriquecen Indicadores y convierten cifras en Oportunidades.",
)];
const TOKENS_POINT_3: &[HelpSegment] = &[HelpSegment::Text(
    "Comprobación previa: cada clave se valida con su fuente antes de guardarse localmente en tu carpeta privada.",
)];
const TOKENS_POINTS: &[&[HelpSegment]] = &[TOKENS_POINT_1, TOKENS_POINT_2, TOKENS_POINT_3];

const BACKUP_POINT_1: &[HelpSegment] = &[HelpSegment::Text(
    "Qué incluye: toda tu base de negocios, historial CRM, notas, plantillas personalizadas, ajustes y tokens.",
)];
const BACKUP_POINT_2: &[HelpSegment] = &[HelpSegment::Text(
    "Cuándo hacerlo: antes de experimentar cambios grandes en plantillas o al mover tu trabajo a otra computadora.",
)];
const BACKUP_POINTS: &[&[HelpSegment]] = &[BACKUP_POINT_1, BACKUP_POINT_2];

fn help_copy(topic: HelpTopic) -> HelpCopy {
    match topic {
        HelpTopic::Profile => HelpCopy {
            title: "Perfil y diferenciador",
            lead: "Estos datos personalizan automáticamente cada mensaje y definen el ritmo de trabajo que ves en la barra superior.",
            points: PROFILE_POINTS,
            note: "Si editas texto en Perfil o en Plantillas, confirma al final con «Guardar cambios».",
        },
        HelpTopic::Templates => HelpCopy {
            title: "Plantillas de mensajes",
            lead: "Escribe una base por cada situación; la aplicación rota variantes de forma determinista y reemplaza los bloques de color con los datos de cada negocio.",
            points: TPL_POINTS,
            note: "Usa el botón «Nueva plantilla» para duplicar y adaptar un enfoque; luego podrás medir resultados en Analítica.",
        },
        HelpTopic::Scope => HelpCopy {
            title: "Alcance de prospección",
            lead: "Define en qué estados de México buscar prospectos, qué tamaños de negocio te interesan y cada cuánto revisar novedades.",
            points: SCOPE_POINTS,
            note: "Cada cambio en Alcance queda listo al pulsar «Guardar cambios».",
        },
        HelpTopic::Base => HelpCopy {
            title: "Tu base y actualizaciones",
            lead: "Supervisa cuántos negocios reales ya tienes guardados por estado y cuántos giros o segmentos quedan por explorar.",
            points: BASE_POINTS,
            note: "Si un estado dice «Por explorar», vuelve a actualizar cuando quieras seguir avanzando con la siguiente tanda.",
        },
        HelpTopic::Tokens => HelpCopy {
            title: "Tokens de fuentes públicas",
            lead: "Son claves gratuitas a tu nombre para consultar fuentes oficiales directamente desde tu equipo.",
            points: TOKENS_POINTS,
            note: "La aplicación nunca incluye tokens compartidos ni envía tus claves fuera de las fuentes oficiales.",
        },
        HelpTopic::Backup => HelpCopy {
            title: "Respaldo portátil (.zip)",
            lead: "Genera una copia completa de tu operación en un solo archivo comprimido donde tú decidas guardarlo.",
            points: BACKUP_POINTS,
            note: "Guarda el archivo .zip en un lugar privado porque incluye tus tokens personales.",
        },
    }
}

#[component]
fn HelpModal(mut topic: Signal<Option<HelpTopic>>) -> Element {
    let Some(current) = *topic.read() else {
        return rsx! {};
    };
    let copy = help_copy(current);
    rsx! {
        div {
            class: "modal-backdrop",
            tabindex: "-1",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    topic.set(None);
                }
            },
            onclick: move |_| topic.set(None),
            div {
                class: "modal-card st-help-modal",
                role: "dialog",
                aria_modal: "true",
                aria_labelledby: "st-help-title",
                onclick: move |e: MouseEvent| e.stop_propagation(),
                div { class: "modal-card-head",
                    h2 { id: "st-help-title", "{copy.title}" }
                    button {
                        class: "toast-close",
                        r#type: "button",
                        aria_label: "Cerrar ayuda",
                        onclick: move |_| topic.set(None),
                        "×"
                    }
                }
                p { class: "st-help-lead", "{copy.lead}" }
                ul { class: "st-help-list",
                    for point in copy.points {
                        li {
                            for seg in *point {
                                match *seg {
                                    HelpSegment::Text(text) => rsx! { "{text}" },
                                    HelpSegment::Tag(label, own) => rsx! { InlineTag { label, own } },
                                }
                            }
                        }
                    }
                }
                p { class: "notice small st-help-tip", "{copy.note}" }
                div { class: "modal-card-footer",
                    button {
                        class: "button primary",
                        r#type: "button",
                        onclick: move |_| topic.set(None),
                        "Entendido"
                    }
                }
            }
        }
    }
}

fn svg_check() -> Element {
    rsx! {
        svg {
            view_box: "0 0 16 16",
            width: "13",
            height: "13",
            fill: "none",
            "aria-hidden": "true",
            path {
                d: "M3.2 8.4l3 3 6.6-6.8",
                stroke: "currentColor",
                stroke_width: "2",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

fn svg_backup_shield() -> Element {
    rsx! {
        svg {
            view_box: "0 0 16 16",
            width: "14",
            height: "14",
            fill: "none",
            "aria-hidden": "true",
            path {
                d: "M8 1.5l5 2.1v4c0 3.3-2.1 5.8-5 6.9-2.9-1.1-5-3.6-5-6.9v-4l5-2.1z",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linejoin: "round",
            }
            path {
                d: "M8 5.2v4.3m0 0l-1.8-1.8M8 9.5l1.8-1.8",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
            }
        }
    }
}

/// Desktop: one .zip (base, ajustes, plantillas y tokens) saved where the user chooses.
/// iOS keeps the private SQLite copy shared through Archivos.
fn run_backup(db: Db, mut ui: UiState) {
    if ui.heavy_task.peek().is_some() {
        return;
    }
    #[cfg(not(target_os = "ios"))]
    {
        let settings = ui.settings.peek().clone();
        spawn(async move {
            let name = format!("prospect-respaldo-{}.zip", prospect_core::config::today());
            let Some(file) = rfd::AsyncFileDialog::new()
                .set_title("Guardar respaldo")
                .set_file_name(name)
                .add_filter("Respaldo de Prospect Intelligence", &["zip"])
                .save_file()
                .await
            else {
                return;
            };
            let mut destination = file.path().to_path_buf();
            if destination
                .extension()
                .is_none_or(|e| !e.eq_ignore_ascii_case("zip"))
            {
                destination.set_extension("zip");
            }
            ui.heavy_task.set(Some("Creando respaldo…".into()));
            let message = match db.backup_archive(destination, settings).await {
                Ok(path) => format!("Respaldo guardado: {}", path.display()),
                Err(e) => e.to_string(),
            };
            ui.heavy_task.set(None);
            ui.toast.set(message);
        });
    }
    #[cfg(target_os = "ios")]
    {
        ui.heavy_task
            .set(Some("Creando respaldo privado de SQLite…".into()));
        spawn(async move {
            let message = match db.backup().await {
                Ok(path) => available_file(path).unwrap_or_else(|e| e),
                Err(e) => e.to_string(),
            };
            ui.heavy_task.set(None);
            ui.toast.set(message);
        });
    }
}

/// Two-to-four mutually exclusive choices shown as one compact control.
#[component]
fn Segmented(
    name: &'static str,
    options: Vec<(String, &'static str)>,
    value: String,
    onpick: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "st-toggle", role: "group", aria_label: name,
            for (code, label) in options {
                {
                    let active = code == value;
                    rsx! {
                        button {
                            key: "{code}",
                            class: if active { "active" } else { "" },
                            r#type: "button",
                            aria_pressed: active,
                            onclick: move |_| onpick.call(code.clone()),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

fn normalized(mut settings: Settings, profile: &str) -> Settings {
    settings.profile = profile.to_owned();
    settings.entities.sort();
    settings.entities.dedup();
    settings.estratos.sort();
    settings.estratos.dedup();
    settings
}

#[component]
fn ProfilePanel(draft: Signal<Settings>, tpl: Signal<String>) -> Element {
    let mut ui = use_context::<UiState>();
    let paths = consume_context::<Services>().db.paths;
    let snapshot = draft.read().clone();
    let source = tpl.read().clone();
    let datos = Templates::parse_toml(&source)
        .map(|t| t.datos)
        .unwrap_or_default();
    let used = |key: &str| source.contains(&format!("{{{key}}}"));
    let error = use_signal(String::new);
    let mut value_tip = use_signal(|| false);
    let mut profile_open = use_signal(|| true);

    rsx! {
        section {
            class: if *profile_open.read() { "panel st-profile" } else { "panel st-profile collapsed" },
            div { class: "panel-heading",
                h2 { "Perfil" }
                span { class: "muted small st-head-meta", "Tus datos personales y enlaces se insertan solos al usar sus bloques" }
                button {
                    class: "button secondary compact st-collapse-btn",
                    r#type: "button",
                    aria_expanded: *profile_open.read(),
                    onclick: move |_| {
                        let next = !*profile_open.peek();
                        profile_open.set(next);
                    },
                    if *profile_open.read() { "Minimizar" } else { "Expandir perfil" }
                }
                HelpButton { topic: HelpTopic::Profile, label: "Perfil" }
            }
            div {
                class: "form-body st-profile-body",
                hidden: !*profile_open.read(),
                div { class: "st-profile-grid",
                    label {
                        "Nombre completo"
                        TextEntry {
                            initial: snapshot.owner_name.clone(),
                            placeholder: "Ej. Sofía Méndez",
                            oninput: move |e: FormEvent| draft.write().owner_name = e.value(),
                        }
                    }
                    label {
                        "Número WhatsApp"
                        TextEntry {
                            initial: snapshot.owner_whatsapp.clone(),
                            placeholder: "10 dígitos",
                            inputmode: "tel",
                            oninput: move |e: FormEvent| draft.write().owner_whatsapp = e.value(),
                        }
                    }
                    label {
                        "Portafolio"
                        TextEntry {
                            kind: "url",
                            initial: snapshot.owner_portfolio.clone(),
                            placeholder: "https://github.com/usuario",
                            oninput: move |e: FormEvent| draft.write().owner_portfolio = e.value(),
                        }
                    }
                    label {
                        "LinkedIn"
                        TextEntry {
                            kind: "url",
                            initial: snapshot.owner_linkedin.clone(),
                            placeholder: "https://linkedin.com/in/usuario",
                            oninput: move |e: FormEvent| draft.write().owner_linkedin = e.value(),
                        }
                    }
                    label {
                        "Página web"
                        TextEntry {
                            kind: "url",
                            initial: snapshot.owner_website.clone(),
                            placeholder: "https://tusitio.com",
                            oninput: move |e: FormEvent| draft.write().owner_website = e.value(),
                        }
                    }
                    for (label, value) in datos {
                        {
                            let in_use = used(&prospect_core::messages::data_key(&label));
                            let edit_label = label.clone();
                            let remove_label = label.clone();
                            rsx! {
                                div { key: "dato-{label}", class: "st-datum",
                                    label {
                                        "{label}"
                                        TextEntry {
                                            initial: value,
                                            oninput: move |e: FormEvent| templates::mutate(tpl, error, |t| {
                                                t.datos.insert(edit_label.clone(), e.value());
                                            }),
                                        }
                                    }
                                    button {
                                        class: "st-icon-btn",
                                        r#type: "button",
                                        disabled: in_use,
                                        aria_label: format!("Eliminar {label}"),
                                        title: if in_use { "En uso: quítalo de tus plantillas para eliminarlo" } else { "Eliminar dato" },
                                        onclick: move |_| templates::mutate(tpl, error, |t| {
                                            t.datos.remove(&remove_label);
                                        }),
                                        "×"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "st-profile-bottom",
                    div { class: "st-field-wrap st-profile-diff",
                        div { class: "st-field-head",
                            span { class: "st-field-label", "Diferenciador" }
                            span { class: "st-inline-help",
                                button {
                                    class: "st-help-btn st-help-btn-sm",
                                    r#type: "button",
                                    aria_label: "Ver ejemplos de diferenciador",
                                    aria_expanded: *value_tip.read(),
                                    onclick: move |_| {
                                        let open = *value_tip.peek();
                                        value_tip.set(!open);
                                    },
                                    "?"
                                }
                                div {
                                    class: if *value_tip.read() { "st-hover-tip open" } else { "st-hover-tip" },
                                    role: "tooltip",
                                    strong { "Ejemplos de diferenciador:" }
                                    ul {
                                        li { "«Automatizo reportes semanales y pedidos para talleres y distribuidoras.»" }
                                        li { "«Ayudo a negocios locales a captar más clientes desde Google Maps.»" }
                                        li { "«Conecto inventario y cotizaciones sin cambiar las herramientas que ya usan.»" }
                                    }
                                    span { class: "muted small",
                                        "Se inserta automáticamente donde uses el bloque "
                                        InlineTag { label: "Diferenciador", own: true }
                                        "."
                                    }
                                }
                            }
                        }
                        TextEntry {
                            label: "Diferenciador".to_owned(),
                            initial: snapshot.owner_credential.clone(),
                            placeholder: "Qué problema resuelves o qué resultado aportas en una frase corta",
                            oninput: move |e: FormEvent| draft.write().owner_credential = e.value(),
                        }
                    }
                    div { class: "st-block st-profile-goals",
                        div { class: "st-block-head",
                            strong { "Metas diarias" }
                            span { class: "muted small", "0 desactiva" }
                        }
                        div { class: "st-goals",
                            label {
                                "Revisar"
                                TextEntry {
                                    kind: "number",
                                    inputmode: "numeric",
                                    initial: snapshot.daily_research.to_string(),
                                    oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_research = goal; },
                                }
                            }
                            label {
                                "Contactar"
                                TextEntry {
                                    kind: "number",
                                    inputmode: "numeric",
                                    initial: snapshot.daily_contacts.to_string(),
                                    oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_contacts = goal; },
                                }
                            }
                            label {
                                "Seguimientos"
                                TextEntry {
                                    kind: "number",
                                    inputmode: "numeric",
                                    initial: snapshot.daily_followups.to_string(),
                                    oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_followups = goal; },
                                }
                            }
                        }
                    }
                    div { class: "st-block st-profile-theme",
                        div { class: "st-block-head", strong { "Apariencia" } }
                        Segmented {
                            name: "Apariencia",
                            options: vec![("light".to_owned(), "Claro"), ("dark".to_owned(), "Oscuro")],
                            value: snapshot.theme.clone(),
                            onpick: move |theme: String| {
                                // The theme applies and persists at once; other edits wait for "Guardar cambios".
                                draft.write().theme = theme.clone();
                                let mut applied = ui.settings.peek().clone();
                                if applied.theme != theme {
                                    applied.theme = theme;
                                    if let Err(e) = applied.save(&paths) {
                                        ui.toast.set(e.to_string());
                                    }
                                    ui.settings.set(applied);
                                }
                            },
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

#[component]
pub(super) fn SettingsPanel() -> Element {
    let service = consume_context::<Services>();
    let db = service.db.clone();
    let paths = db.paths.clone();
    let mut ui = use_context::<UiState>();
    let pane = ui.settings_tab;
    let help_topic = use_signal(|| None::<HelpTopic>);
    use_context_provider(|| HelpContext { topic: help_topic });
    let mut draft = use_signal(|| ui.settings.peek().clone());
    let mut seen = use_signal(|| ui.settings.peek().clone());
    let home = paths.home.clone();
    let initial = use_hook(|| {
        Templates::source(&home).unwrap_or_else(|_| Templates::bundled_source().into())
    });
    let tpl = use_signal(|| initial.clone());
    let mut tpl_saved = use_signal(|| initial);

    // Switching the offer profile elsewhere must not discard edits in progress here.
    use_effect(move || {
        let current = ui.settings.read().clone();
        let previous = seen.peek().clone();
        if previous == current {
            return;
        }
        // Offer and theme change outside the draft; keep the rest of the edits in progress.
        let mut aligned = previous;
        aligned.theme = current.theme.clone();
        let only_live =
            normalized(aligned, &current.profile) == normalized(current.clone(), &current.profile);
        if only_live {
            let mut d = draft.write();
            d.profile = current.profile.clone();
            d.theme = current.theme.clone();
        } else {
            draft.set(current.clone());
        }
        seen.set(current);
    });

    if service.layout == Layout::Ios {
        if mobile::destination() == Destination::Templates {
            return rsx! { mobile::TemplateEditor { onclose: move |_| mobile::back() } };
        }
        return rsx! { MobileSettings { draft } };
    }

    let current_tab = *pane.read();
    let saved = ui.settings.read().clone();
    let draft_snap = draft.read().clone();
    let dirty = normalized(draft_snap.clone(), &saved.profile)
        != normalized(saved.clone(), &saved.profile)
        || *tpl.read() != *tpl_saved.read();
    let added_states = draft_snap
        .entities
        .iter()
        .any(|code| !saved.entities.contains(code));

    rsx! {
        div { class: "settings-work",
            div { class: "st-topline",
                PanelTabs {
                    labels: vec!["Perfil y preferencias", "Base de datos y alcance", "Tokens y fuentes"],
                    active: pane,
                    name: "Ajustes",
                }
                if current_tab < 2 {
                    div { class: "st-savebar",
                        if dirty {
                            span { class: "st-dirty", role: "status",
                                if added_states {
                                    "Nuevo estado seleccionado · guarda cambios"
                                } else {
                                    "Cambios sin guardar"
                                }
                            }
                        }
                        button {
                            class: if dirty && added_states { "button primary st-save-attention" } else { "button primary" },
                            disabled: !dirty,
                            onclick: move |_| {
                                let before = ui.settings.peek().clone();
                                let next = normalized(draft.peek().clone(), &before.profile);
                                let source = tpl.peek().clone();
                                if source != *tpl_saved.peek() {
                                    if let Err(e) = Templates::save(&paths.home, &source) {
                                        ui.toast.set(format!("Plantillas sin guardar: {e}"));
                                        return;
                                    }
                                    tpl_saved.set(source);
                                }
                                if let Err(e) = next.save(&paths) {
                                    ui.toast.set(e.to_string());
                                    return;
                                }
                                let added: Vec<String> = next
                                    .entities
                                    .iter()
                                    .filter(|code| !before.entities.contains(code))
                                    .cloned()
                                    .collect();
                                draft.set(next.clone());
                                seen.set(next.clone());
                                ui.settings.set(next);
                                ui.revision += 1;
                                ui.toast.set("Cambios guardados.".into());
                                if added.is_empty() {
                                    return;
                                }
                                let names = added
                                    .iter()
                                    .map(|code| prospect_core::config::entity_name(code))
                                    .collect::<Vec<_>>()
                                    .join(", ");
                                let has_token = paths
                                    .secrets()
                                    .map(|s| !s.denue.trim().is_empty())
                                    .unwrap_or(false);
                                if has_token {
                                    job_start_sync(
                                        ui,
                                        db.clone(),
                                        Some(added),
                                        false,
                                        true,
                                        Some(format!("Actualizando · {names}")),
                                    );
                                } else {
                                    ui.toast.set(format!(
                                        "{names} agregado. Configura tu token DENUE en Tokens y fuentes para descargarlo."
                                    ));
                                }
                            },
                            "Guardar cambios"
                        }
                    }
                }
            }
            if current_tab == 0 {
                div { class: "st-grid st-grid-profile",
                    ProfilePanel { draft, tpl }
                    templates::TemplateCanvas { tpl, settings: draft.read().clone() }
                }
            } else if current_tab == 1 {
                scope::ScopeTab { draft, pane }
            } else {
                tokens::TokensTab { pane }
            }
            HelpModal { topic: help_topic }
        }
    }
}

#[component]
fn MobileSettings(mut draft: Signal<Settings>) -> Element {
    let service = consume_context::<Services>();
    let paths = service.db.paths.clone();
    let mut ui = use_context::<UiState>();
    let onboarding = try_use_context::<OnboardingControl>();
    if let Some(o) = onboarding {
        let _ = *o.revision.read();
    }
    let configured = paths.secrets().unwrap_or_default();
    let mut entity_to_add = use_signal(|| "09".to_owned());
    let mut denue = use_signal(String::new);
    let mut inegi = use_signal(String::new);
    let mut banxico = use_signal(String::new);
    let mut token_epoch = use_signal(|| 0u64);
    let backup_db = service.db.clone();
    let save_paths = paths.clone();
    let secret_paths = paths.clone();
    let page = mobile::destination();

    rsx! {
        div { class: "settings-work",
            if page == Destination::Root {
                mobile::PageMenu {
                    pages: vec![
                        Destination::Identity,
                        Destination::Appearance,
                        Destination::Goals,
                        Destination::DataPreferences,
                        Destination::Credentials,
                        Destination::Backup,
                        Destination::Templates,
                    ],
                }
            }
            div { class: "settings-grid",
                section {
                    class: "panel preferences-panel",
                    hidden: !matches!(page, Destination::Identity | Destination::Appearance | Destination::Goals | Destination::DataPreferences),
                    div { class: "form-body",
                        div { hidden: !matches!(page, Destination::Identity | Destination::Appearance | Destination::Goals),
                            div { hidden: page != Destination::Identity,
                                div { class: "form-row",
                                    label { "Nombre completo" TextEntry { initial: draft.read().owner_name.clone(), oninput: move |e: FormEvent| draft.write().owner_name = e.value() } }
                                    label { "Número WhatsApp" TextEntry { initial: draft.read().owner_whatsapp.clone(), oninput: move |e: FormEvent| draft.write().owner_whatsapp = e.value() } }
                                }
                                label { "Diferenciador" TextEntry { multiline: true, initial: draft.read().owner_credential.clone(), oninput: move |e: FormEvent| draft.write().owner_credential = e.value() } }
                                label { "Portafolio" TextEntry { kind: "url", initial: draft.read().owner_portfolio.clone(), placeholder: "https://github.com/usuario", oninput: move |e: FormEvent| draft.write().owner_portfolio = e.value() } }
                                label { "LinkedIn" TextEntry { kind: "url", initial: draft.read().owner_linkedin.clone(), placeholder: "https://linkedin.com/in/usuario", oninput: move |e: FormEvent| draft.write().owner_linkedin = e.value() } }
                                label { "Página web" TextEntry { kind: "url", initial: draft.read().owner_website.clone(), placeholder: "https://tusitio.com", oninput: move |e: FormEvent| draft.write().owner_website = e.value() } }
                            }
                            label {
                                hidden: page != Destination::Appearance,
                                "Apariencia"
                                select {
                                    value: draft.read().theme.clone(),
                                    onchange: move |e| draft.write().theme = e.value(),
                                    option { value: "light", "Claro" }
                                    option { value: "dark", "Oscuro" }
                                }
                            }
                            div { hidden: page != Destination::Goals,
                                h3 { "Metas diarias" }
                                div { class: "goals-settings",
                                    label { "Revisar" TextEntry { kind: "number", initial: draft.read().daily_research.to_string(), oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_research = goal; } } }
                                    label { "Contactar" TextEntry { kind: "number", initial: draft.read().daily_contacts.to_string(), oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_contacts = goal; } } }
                                    label { "Seguimientos" TextEntry { kind: "number", initial: draft.read().daily_followups.to_string(), oninput: move |e: FormEvent| if let Ok(goal) = e.value().trim().parse() { draft.write().daily_followups = goal; } } }
                                }
                            }
                        }
                        div { hidden: page != Destination::DataPreferences,
                            h3 { "Estados de interés" }
                            div { class: "selected-entities",
                                for code in draft.read().entities.clone() {
                                    {
                                        let remove = code.clone();
                                        let name = prospect_core::config::entity_name(&code).to_owned();
                                        rsx! {
                                            button {
                                                class: "pill-button",
                                                aria_label: format!("Quitar estado {name}"),
                                                disabled: draft.read().entities.len() <= 1,
                                                onclick: move |_| draft.write().entities.retain(|c| c != &remove),
                                                "{name} ×"
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "entity-add",
                                select {
                                    aria_label: "Estado para añadir",
                                    value: entity_to_add.read().clone(),
                                    onchange: move |e| entity_to_add.set(e.value()),
                                    for (code, name) in prospect_core::config::ENTITIES {
                                        option { value: *code, "{name}" }
                                    }
                                }
                                button {
                                    class: "button secondary",
                                    disabled: draft.read().entities.contains(&entity_to_add.read()),
                                    onclick: move |_| {
                                        let code = entity_to_add.read().clone();
                                        let mut d = draft.write();
                                        if !d.entities.contains(&code) {
                                            d.entities.push(code);
                                            d.entities.sort();
                                        }
                                    },
                                    "Añadir estado"
                                }
                            }
                        }
                        div { class: "settings-save",
                            button {
                                class: "button primary",
                                onclick: move |_| {
                                    let mut s = draft.read().clone();
                                    s.profile = ui.settings.peek().profile.clone();
                                    s.entities.sort();
                                    s.entities.dedup();
                                    s.estratos.sort();
                                    s.estratos.dedup();
                                    match s.save(&save_paths) {
                                        Ok(()) => {
                                            ui.settings.set(s);
                                            ui.revision += 1;
                                            ui.toast.set("Preferencias guardadas.".into());
                                        }
                                        Err(e) => ui.toast.set(e.to_string()),
                                    }
                                },
                                "Guardar"
                            }
                        }
                    }
                }
                section {
                    class: "panel",
                    hidden: !matches!(page, Destination::Credentials | Destination::Backup),
                    div { class: "form-body",
                        div { hidden: page != Destination::Credentials,
                            label { "DENUE (INEGI)" TextEntry { key: "m-denue:{token_epoch}", kind: "password", placeholder: if configured.denue.is_empty() { "Sin configurar" } else { "Configurado" }, initial: String::new(), oninput: move |e: FormEvent| denue.set(e.value()) } }
                            crate::onboarding::TokenSourceHint { guide: crate::onboarding::DENUE_GUIDE, hint_class: "token-hint", link_class: "link-button small", url_class: "token-url", on_error: move |msg| ui.toast.set(msg) }
                            label { "INEGI Indicadores (opcional)" TextEntry { key: "m-inegi:{token_epoch}", kind: "password", placeholder: if configured.inegi.is_empty() { "Usar token DENUE" } else { "Configurado" }, initial: String::new(), oninput: move |e: FormEvent| inegi.set(e.value()) } }
                            crate::onboarding::TokenSourceHint { guide: crate::onboarding::INEGI_INDICATORS_GUIDE, hint_class: "token-hint", link_class: "link-button small", url_class: "token-url", on_error: move |msg| ui.toast.set(msg) }
                            label { "Banxico SIE (opcional)" TextEntry { key: "m-banxico:{token_epoch}", kind: "password", placeholder: if configured.banxico.is_empty() { "Sin configurar" } else { "Configurado" }, initial: String::new(), oninput: move |e: FormEvent| banxico.set(e.value()) } }
                            crate::onboarding::TokenSourceHint { guide: crate::onboarding::BANXICO_GUIDE, hint_class: "token-hint", link_class: "link-button small", url_class: "token-url", on_error: move |msg| ui.toast.set(msg) }
                            button {
                                class: "button secondary",
                                disabled: denue.read().trim().is_empty() && inegi.read().trim().is_empty() && banxico.read().trim().is_empty(),
                                onclick: move |_| {
                                    let result = crate::onboarding::save_tokens(&secret_paths, &denue.read(), &inegi.read(), &banxico.read(), false);
                                    match result {
                                        Ok(()) => {
                                            denue.set(String::new());
                                            inegi.set(String::new());
                                            banxico.set(String::new());
                                            token_epoch += 1;
                                            if let Some(mut o) = onboarding {
                                                o.revision += 1;
                                            }
                                            ui.toast.set("Tokens guardados en tu directorio personal".into());
                                            ui.revision += 1;
                                        }
                                        Err(e) => ui.toast.set(e),
                                    }
                                },
                                "Guardar tokens"
                            }
                        }
                        div { hidden: page != Destination::Backup,
                            h3 { "Base personal" }
                            p { class: "muted small path", "{paths.db.display()}" }
                            button {
                                class: "button backup",
                                disabled: ui.heavy_task.read().is_some(),
                                onclick: move |_| run_backup(backup_db.clone(), ui),
                                {svg_backup_shield()}
                                span { "Crear respaldo" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn HelpModalProbe(initial: HelpTopic) -> Element {
        let topic = use_signal(|| Some(initial));
        rsx! { HelpModal { topic } }
    }

    #[test]
    fn help_modal_renders_dialog_for_each_section() {
        let cases = [
            (HelpTopic::Profile, "Perfil y diferenciador"),
            (HelpTopic::Templates, "Plantillas de mensajes"),
            (HelpTopic::Scope, "Alcance de prospección"),
            (HelpTopic::Base, "Tu base y actualizaciones"),
            (HelpTopic::Tokens, "Tokens de fuentes públicas"),
            (HelpTopic::Backup, "Respaldo portátil (.zip)"),
        ];
        for (initial, title) in cases {
            let mut dom =
                VirtualDom::new_with_props(HelpModalProbe, HelpModalProbeProps { initial });
            dom.rebuild_in_place();
            let html = dioxus_ssr::render(&dom);
            assert!(html.contains("role=\"dialog\""), "{title}");
            assert!(html.contains("aria-modal=\"true\""), "{title}");
            assert!(html.contains(title), "{title}");
            assert!(!html.contains("[Tu nombre]"), "{title}");
            assert!(!html.contains("[Valor agregado]"), "{title}");
            assert!(!html.contains("Valor agregado"), "{title}");
            if initial == HelpTopic::Profile {
                assert!(html.contains("Diferenciador"), "{title}");
                assert!(html.contains("Portafolio"), "{title}");
            }
            assert!(html.contains("Entendido"), "{title}");
        }
    }
}
