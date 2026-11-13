mod mobile;
mod settings;
mod workspace;
use crate::{
    Layout, Services,
    view_cache::{ViewCache, use_cached_read},
};
use dioxus::prelude::*;
use mobile::Destination;
use prospect_core::{
    Command, Db, Interaction, Message, Mutation, Page, Profile, Prospect, Query, Settings,
    Signal as WebSignal, View,
    analytics::OpportunityDraft,
    jobs::{JobControl, JobState},
};
use std::time::Duration;

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Screen {
    #[default]
    Today,
    Dashboard,
    Updates,
    Followups,
    Conversations,
    Base,
    Metrics,
    Territories,
    Opportunities,
    Indicators,
    Settings,
}
impl Screen {
    fn label(self) -> &'static str {
        match self {
            Self::Today => "Hoy",
            Self::Dashboard => "Dashboard",
            Self::Updates => "Actualizaciones",
            Self::Followups => "Seguimientos",
            Self::Conversations => "Conversaciones",
            Self::Base => "Base",
            Self::Metrics => "Métricas",
            Self::Territories => "Territorios",
            Self::Opportunities => "Oportunidades",
            Self::Indicators => "Indicadores",
            Self::Settings => "Ajustes",
        }
    }
    fn view(self) -> View {
        match self {
            Self::Today => View::Today,
            Self::Followups => View::Due,
            Self::Conversations => View::Conversations,
            _ => View::All,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
struct FloatingNotification {
    id: u64,
    message: String,
    is_error: bool,
}

#[derive(Clone, Copy)]
struct UiState {
    screen: Signal<Screen>,
    settings_tab: Signal<usize>,
    query: Signal<Query>,
    selected: Signal<String>,
    revision: Signal<u64>,
    settings: Signal<Settings>,
    job: Signal<Option<JobControl>>,
    job_minimized: Signal<bool>,
    dismissed_job_id: Signal<String>,
    heavy_task: Signal<Option<String>>,
    toast: Signal<String>,
    notifications: Signal<Vec<FloatingNotification>>,
    notif_seq: Signal<u64>,
    busy: Signal<bool>,
    pending: Signal<Option<Mutation>>,
    mobile_layout: bool,
}
#[derive(Clone, Copy)]
enum JobKind {
    Import,
    Resume,
    Recheck,
    Analyze,
    Enrich,
    Banxico,
    Inegi,
}
fn job_start(kind: JobKind, ui: UiState, db: Db, indicator: String) {
    job_start_scoped(kind, ui, db, indicator, None);
}
fn job_start_sync(
    mut ui: UiState,
    db: Db,
    entities_override: Option<Vec<String>>,
    analyze_all: bool,
    minimized: bool,
    custom_label: Option<String>,
) {
    if ui
        .job
        .peek()
        .as_ref()
        .is_some_and(|j| j.snapshot().status == "running")
    {
        ui.toast
            .set("Ya hay una actualización en curso; espera a que termine o cancélala.".into());
        return;
    }
    let mut settings = ui.settings.peek().clone();
    if let Some(entities) = entities_override {
        if entities.is_empty() {
            return;
        }
        settings.entities = entities;
    }
    let profile = Profile::parse(&settings.profile).unwrap_or_default();
    let label = custom_label.unwrap_or_else(|| {
        if analyze_all {
            "Actualizar todo y recalcular".into()
        } else {
            "Actualizar nuevos registros".into()
        }
    });
    let control = JobControl::new(&label);
    ui.dismissed_job_id.set(String::new());
    ui.job_minimized.set(minimized);
    ui.job.set(Some(control.clone()));
    tokio::spawn(async move {
        let denue_ok = db
            .paths
            .secrets()
            .map(|s| !s.denue.trim().is_empty())
            .unwrap_or(false);
        if denue_ok {
            let progress = match db.denue_progress(settings.clone(), profile).await {
                Ok(p) => p,
                Err(e) => {
                    control.finish(&Err(e));
                    let _ = db.log_job(control.snapshot()).await;
                    return;
                }
            };
            let has_pending = progress.iter().any(|p| p.started && !p.completed);
            let mut pull_result: prospect_core::Result<usize> = Ok(0);
            if has_pending {
                pull_result = prospect_core::updates::pull_batch(
                    &db,
                    &settings,
                    profile,
                    prospect_core::updates::ImportMode::Resume,
                    &control,
                )
                .await;
            }
            if pull_result.is_ok()
                && !control.token.is_cancelled()
                && control.snapshot().pages < settings.denue_pages
            {
                let refreshed = db.denue_progress(settings.clone(), profile).await;
                if let Ok(list) = refreshed {
                    let still_pending = list.iter().any(|p| p.started && !p.completed);
                    let has_ready = list.iter().any(|p| !p.started);
                    if !still_pending && has_ready {
                        pull_result = prospect_core::updates::pull_batch(
                            &db,
                            &settings,
                            profile,
                            prospect_core::updates::ImportMode::New,
                            &control,
                        )
                        .await;
                    }
                }
            }
            if let Err(e) = pull_result {
                let changed = control.snapshot().inserted + control.snapshot().updated;
                if !control.token.is_cancelled() && changed > 0 {
                    let _ = db.analyze_pending(settings, control.clone()).await;
                }
                control.finish(&Err(e));
                let _ = db.log_job(control.snapshot()).await;
                return;
            }
        }
        let analysis = if analyze_all {
            db.analyze(settings, control.clone()).await
        } else {
            db.analyze_pending(settings, control.clone()).await
        };
        control.finish(&analysis);
        let _ = db.log_job(control.snapshot()).await;
    });
}
fn job_start_scoped(kind: JobKind, ui: UiState, db: Db, indicator: String, entity: Option<String>) {
    if ui
        .job
        .peek()
        .as_ref()
        .is_some_and(|j| j.snapshot().status == "running")
    {
        return;
    }
    let mut ui = ui;
    let mut settings = ui.settings.peek().clone();
    if let Some(ref entity) = entity {
        if !matches!(kind, JobKind::Import | JobKind::Resume) || !settings.entities.contains(entity)
        {
            ui.toast
                .set("Selecciona primero este estado en Ajustes → Base de datos y alcance".into());
            return;
        }
        settings.entities = vec![entity.clone()];
    }
    let profile = Profile::parse(&settings.profile).unwrap_or_default();
    let paths = db.paths.clone();
    let label = match kind {
        JobKind::Import => "Actualizar registros",
        JobKind::Resume => "Completar lote",
        JobKind::Recheck => "Revisar datos consultados",
        JobKind::Analyze => "Calcular prioridades",
        JobKind::Enrich => "Verificar sitios web",
        JobKind::Banxico => "Banxico",
        JobKind::Inegi => "INEGI",
    };
    let label = entity
        .as_ref()
        .map(|code| format!("{label} · {}", prospect_core::config::entity_name(code)))
        .unwrap_or_else(|| label.to_owned());
    let control = JobControl::new(&label);
    ui.dismissed_job_id.set(String::new());
    ui.job_minimized.set(false);
    ui.job.set(Some(control.clone()));
    tokio::spawn(async move {
        let result = match kind {
            JobKind::Import | JobKind::Resume | JobKind::Recheck => {
                let mode = match kind {
                    JobKind::Resume => prospect_core::updates::ImportMode::Resume,
                    JobKind::Recheck => prospect_core::updates::ImportMode::Recheck,
                    _ => prospect_core::updates::ImportMode::New,
                };
                let result =
                    prospect_core::updates::pull_batch(&db, &settings, profile, mode, &control)
                        .await;
                let changed = control.snapshot().inserted + control.snapshot().updated;
                if (result.is_ok() || (!control.token.is_cancelled() && changed > 0))
                    && let Err(e) = db.analyze_pending(settings, control.clone()).await
                {
                    control.finish(&Err(e));
                    let _ = db.log_job(control.snapshot()).await;
                    return;
                }
                result
            }
            JobKind::Analyze => db.analyze(settings, control.clone()).await,
            JobKind::Enrich => {
                match prospect_core::enrich::pending(&db, 100, false, &control).await {
                    Ok(n) => db.analyze(settings, control.clone()).await.map(|_| n),
                    Err(e) => Err(e),
                }
            }
            JobKind::Banxico => {
                prospect_core::sources::sync_economics(&db, &paths, "banxico", "", &control).await
            }
            JobKind::Inegi => {
                prospect_core::sources::sync_economics(&db, &paths, "inegi", &indicator, &control)
                    .await
            }
        };
        control.finish(&result);
        let _ = db.log_job(control.snapshot()).await;
    });
}
fn mutation(
    command: Command,
    pid: String,
    notes: Option<String>,
    profile: Profile,
    mut ui: UiState,
    _db: Db,
) {
    if *ui.busy.peek() {
        return;
    }
    ui.busy.set(true);
    ui.pending.set(Some(Mutation {
        operation_id: prospect_core::operation_id(),
        prospect_id: pid,
        profile,
        command,
        notes,
    }));
}
pub(crate) fn open_link(link: &str) -> std::result::Result<(), String> {
    let u = prospect_core::contacts::normalize_url(link);
    if link.starts_with("mailto:") || link.starts_with("tel:") || u.is_some() {
        #[cfg(not(target_os = "ios"))]
        {
            opener::open(link).map_err(|e| format!("No se pudo abrir el enlace: {e}"))
        }
        #[cfg(target_os = "ios")]
        {
            crate::ios::open(link)
        }
    } else {
        Err("Enlace no permitido".into())
    }
}
#[cfg(not(target_os = "ios"))]
fn copied(text: &str, mut ui: UiState) {
    let result = arboard::Clipboard::new().and_then(|mut c| c.set_text(text.to_owned()));
    ui.toast.set(match result {
        Ok(()) => "Copiado al portapapeles".into(),
        Err(_) => "No se pudo acceder al portapapeles".into(),
    });
}
#[cfg(target_os = "ios")]
fn copied(text: &str, mut ui: UiState) {
    ui.toast.set(match crate::ios::copy(text) {
        Ok(()) => "Copiado al portapapeles".into(),
        Err(error) => error,
    });
}
fn available_file(path: std::path::PathBuf) -> std::result::Result<String, String> {
    #[cfg(not(target_os = "ios"))]
    {
        Ok(format!("Guardado: {}", path.display()))
    }
    #[cfg(target_os = "ios")]
    {
        let home = std::env::var_os("HOME").ok_or("No se encontró la carpeta de Archivos")?;
        let folder = std::path::PathBuf::from(home).join("Documents/Prospect");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let filename = path
            .file_name()
            .ok_or("Archivo inválido")?
            .to_string_lossy();
        let destination = folder.join(format!("{}-{filename}", prospect_core::operation_id()));
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        prospect_core::config::write_private(&destination, &bytes).map_err(|e| e.to_string())?;
        let metadata = path.with_extension(format!(
            "{}.metadata.json",
            path.extension().unwrap_or_default().to_string_lossy()
        ));
        if metadata.is_file() {
            let bytes = std::fs::read(metadata).map_err(|e| e.to_string())?;
            prospect_core::config::write_private(
                &destination.with_extension(format!(
                    "{}.metadata.json",
                    destination
                        .extension()
                        .unwrap_or_default()
                        .to_string_lossy()
                )),
                &bytes,
            )
            .map_err(|e| e.to_string())?;
        }
        Ok("Guardado en Archivos → En mi iPhone → Prospect → Prospect".into())
    }
}
fn close_window() {
    #[cfg(feature = "desktop")]
    consume_context::<dioxus::desktop::DesktopContext>().close();
    #[cfg(all(feature = "mobile", not(feature = "desktop")))]
    consume_context::<dioxus::mobile::DesktopContext>().close();
}
fn open_file(path: &std::path::Path) -> std::result::Result<(), String> {
    #[cfg(not(target_os = "ios"))]
    {
        opener::open(path).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "ios")]
    {
        let _ = path;
        Err("Abre Archivos → En mi iPhone → Prospect para acceder a tus archivos".into())
    }
}
fn number(n: usize) -> String {
    let chars = n.to_string().chars().rev().collect::<Vec<_>>();
    chars
        .chunks(3)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect()
}
fn amount(cents: i64) -> String {
    format!("{:.2}", cents as f64 / 100.0)
}

pub(crate) fn brand_icon_url() -> String {
    use std::fmt::Write;
    let mut result = String::from("data:image/svg+xml,");
    for byte in include_str!("../../../assets/app-icon.svg").bytes() {
        write!(&mut result, "%{byte:02X}").expect("writing to String");
    }
    result
}

pub(crate) fn loading_spinner_svg() -> Element {
    rsx! {
        svg {
            class: "loading-spinner",
            view_box: "0 0 28 28",
            width: "28",
            height: "28",
            fill: "none",
            "aria-hidden": "true",
            circle {
                cx: "14",
                cy: "14",
                r: "11",
                stroke: "currentColor",
                stroke_opacity: "0.18",
                stroke_width: "3",
            }
            g {
                path {
                    d: "M14 3a11 11 0 0 1 11 11",
                    stroke: "currentColor",
                    stroke_width: "3",
                    stroke_linecap: "round",
                }
                animateTransform {
                    attribute_name: "transform",
                    r#type: "rotate",
                    "from": "0 14 14",
                    "to": "360 14 14",
                    "dur": "0.8s",
                    repeat_count: "indefinite",
                }
            }
        }
    }
}

pub(crate) fn job_progress_percent(state: &JobState) -> Option<usize> {
    if state.total == 0 {
        return None;
    }
    let done = if state.analyzing {
        state.processed
    } else {
        state.pages
    };
    Some(
        done.saturating_mul(100)
            .checked_div(state.total)
            .unwrap_or(0)
            .min(100),
    )
}

pub(crate) fn loading_progress_bar(pct: Option<usize>, status: &str) -> Element {
    let tone = match status {
        "completed" => "progress-fill complete",
        "failed" | "cancelled" => "progress-fill error",
        _ => "progress-fill",
    };
    match pct {
        Some(value) => rsx! {
            div { class: "progress-track",
                div { class: "{tone}", style: "width: {value}%" }
            }
        },
        None => rsx! {
            div { class: "progress-track",
                div { class: "progress-indeterminate" }
            }
        },
    }
}

pub(crate) fn loading_screen_card(label: &'static str) -> Element {
    rsx! {
        section {
            class: "panel load-status loading-screen-card",
            role: "status",
            aria_busy: "true",
            {loading_spinner_svg()}
            div { class: "loading-screen-copy",
                h2 { "Cargando {label}…" }
                p { class: "muted", "Consultando la base local. Espera un momento." }
                {loading_progress_bar(None, "running")}
            }
        }
    }
}

#[component]
fn JobLoadingOverlay(state: JobState, running: bool) -> Element {
    let mut ui = use_context::<UiState>();
    let heavy_label = ui.heavy_task.read().clone();
    let show_job = running && !*ui.job_minimized.read();
    if !show_job && heavy_label.is_none() {
        return rsx! {};
    }
    let title = if show_job {
        state.kind.clone()
    } else {
        heavy_label.clone().unwrap_or_else(|| "Procesando…".into())
    };
    let detail = if show_job {
        if state.message.is_empty() {
            "Conectando y procesando registros…".to_owned()
        } else {
            state.message.clone()
        }
    } else {
        "Operación pesada en curso. No cierres la aplicación.".to_owned()
    };
    let pct = if show_job {
        job_progress_percent(&state)
    } else {
        None
    };
    rsx! {
        div { class: "loading-backdrop", role: "status", aria_live: "polite", aria_busy: "true",
            div { class: "loading-card",
                div { class: "loading-card-head",
                    {loading_spinner_svg()}
                    div {
                        p { class: "eyebrow", "OPERACIÓN EN CURSO" }
                        h2 { "{title}" }
                    }
                    if let Some(p) = pct {
                        span { class: "badge loading-pct-badge", "{p}%" }
                    }
                }
                p { class: "loading-card-copy", "{detail}" }
                {loading_progress_bar(pct, &state.status)}
                if show_job && state.requests > 0 {
                    p { class: "muted small", "Consultas enviadas: {state.requests}" }
                }
                if show_job {
                    div { class: "loading-card-actions",
                        button {
                            class: "button secondary compact",
                            onclick: move |_| ui.job_minimized.set(true),
                            "Seguir en segundo plano"
                        }
                        button {
                            class: "button secondary compact",
                            onclick: move |_| {
                                if let Some(j) = ui.job.peek().as_ref() {
                                    j.token.cancel();
                                }
                            },
                            "Cancelar"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NotificationCenter(state: JobState, running: bool, mut job_state: Signal<JobState>) -> Element {
    let mut ui = use_context::<UiState>();
    let notifications = ui.notifications.read().clone();
    let show_job = !state.id.is_empty()
        && *ui.dismissed_job_id.read() != state.id
        && (!running || *ui.job_minimized.read());
    if !show_job && notifications.is_empty() {
        return rsx! {};
    }
    let pct = if running {
        job_progress_percent(&state)
    } else if state.status == "completed" {
        Some(100)
    } else {
        None
    };
    let is_failed = matches!(state.status.as_str(), "failed" | "cancelled");
    rsx! {
        aside { class: "toast-stack", aria_label: "Notificaciones",
            if show_job {
                section {
                    class: if is_failed { "toast-card toast-job toast-error" } else { "toast-card toast-job" },
                    role: "status",
                    div { class: "toast-card-head",
                        if running {
                            {loading_spinner_svg()}
                        } else {
                            span { class: if is_failed { "toast-dot error" } else { "toast-dot ok" } }
                        }
                        div { class: "toast-card-title",
                            strong { "{state.kind}" }
                            if let Some(p) = pct {
                                span { class: "muted small", "{p}%" }
                            }
                        }
                        if !running {
                            button {
                                class: "toast-close",
                                aria_label: "Cerrar progreso",
                                onclick: move |_| {
                                    let id = state.id.clone();
                                    ui.dismissed_job_id.set(id);
                                    ui.job.set(None);
                                    job_state.set(JobState::default());
                                },
                                "×"
                            }
                        }
                    }
                    p { class: "toast-card-msg", "{state.message}" }
                    {loading_progress_bar(pct, &state.status)}
                    if running {
                        div { class: "toast-card-actions",
                            if state.requests > 0 {
                                span { class: "muted small", "Consultas: {state.requests}" }
                            }
                            button {
                                class: "link-button small",
                                onclick: move |_| ui.job_minimized.set(false),
                                "Ampliar"
                            }
                            button {
                                class: "button secondary compact",
                                onclick: move |_| {
                                    if let Some(j) = ui.job.peek().as_ref() {
                                        j.token.cancel();
                                    }
                                },
                                "Cancelar"
                            }
                        }
                    }
                }
            }
            for item in notifications {
                div {
                    key: "toast-{item.id}",
                    class: if item.is_error { "toast-card toast-error" } else { "toast-card" },
                    role: "status",
                    span { class: if item.is_error { "toast-dot error" } else { "toast-dot ok" } }
                    p { class: "toast-card-msg", "{item.message}" }
                    button {
                        class: "toast-close",
                        aria_label: "Cerrar aviso",
                        onclick: move |_| {
                            ui.notifications.write().retain(|n| n.id != item.id);
                        },
                        "×"
                    }
                }
            }
        }
    }
}

fn resource_status<T: 'static>(
    mut resource: Resource<Result<T, String>>,
    label: &'static str,
) -> Option<Element> {
    if *resource.state().read() == UseResourceState::Pending {
        return Some(loading_screen_card(label));
    }
    let value = resource.read();
    match value.as_ref() {
        None => Some(loading_screen_card(label)),
        Some(Err(error)) => {
            let error = error.clone();
            Some(
                rsx! {section{class:"panel load-status",role:"alert",h2{"Error al cargar {label}"}p{class:"error","{error}"}button{class:"button secondary",onclick:move |_|resource.restart(),"Volver a intentar"}}},
            )
        }
        Some(Ok(_)) => None,
    }
}

fn indicator_value(raw: &str) -> String {
    let raw = raw.trim();
    let (sign, number) = if let Some(number) = raw.strip_prefix('-') {
        ("-", number)
    } else {
        ("", raw.strip_prefix('+').unwrap_or(raw))
    };
    let (whole, decimal) = number.split_once('.').unwrap_or((number, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !decimal.bytes().all(|b| b.is_ascii_digit())
    {
        return raw.to_owned();
    }
    let whole = whole.trim_start_matches('0');
    let whole = if whole.is_empty() { "0" } else { whole };
    let reversed = whole.chars().rev().collect::<Vec<_>>();
    let whole = reversed
        .chunks(3)
        .map(|c| c.iter().collect::<String>())
        .collect::<Vec<_>>()
        .join(",")
        .chars()
        .rev()
        .collect::<String>();
    let decimal = decimal.trim_end_matches('0');
    if decimal.is_empty() {
        format!("{sign}{whole}")
    } else {
        format!("{sign}{whole}.{decimal}")
    }
}

#[derive(Clone)]
pub struct StartupFailure {
    pub message: String,
    pub directory: String,
}
#[component]
pub fn StartupProblem() -> Element {
    let failure = consume_context::<StartupFailure>();
    let icon = use_hook(brand_icon_url);
    let mut notice = use_signal(String::new);
    let directory = failure.directory.clone();
    rsx! {
        style{{include_str!("style.css")}}
        div{class:"app startup-surface","data-theme":"light",
            main{class:"startup-error",img{src:icon,width:"72",height:"72",alt:"",aria_hidden:"true"},
                p{class:"eyebrow","PROSPECT INTELLIGENCE"}h1{"No se pudo iniciar la app"}
                p{"Revisa la configuración y los permisos de tu carpeta de datos. Puedes corregir el problema sin borrar tus registros."}
                div{class:"notice",role:"alert",p{class:"error","{failure.message}"}}
                p{class:"muted small path","Carpeta de datos: {failure.directory}"}
                div{class:"action-row",button{class:"button secondary",disabled:directory.is_empty(),onclick:move |_|{if let Err(e)=open_file(std::path::Path::new(&directory)){notice.set(format!("No se pudo abrir la carpeta: {e}"));}},"Abrir carpeta de datos"}
                    button{class:"button primary",onclick:move |_|{close_window();},"Cerrar"}}
                if !notice.read().is_empty(){p{role:"status",class:"error","{notice}"}}
                p{class:"muted small","Después de corregirlo, vuelve a abrir Prospect Intelligence."}
            }
        }
    }
}

#[component]
pub fn App() -> Element {
    let service = consume_context::<Services>();
    let mobile_layout = service.layout == Layout::Ios;
    let brand_icon = use_hook(brand_icon_url);
    let db = service.db.clone();
    let onboarding = try_use_context::<crate::onboarding::OnboardingControl>();
    let configured_denue_missing = service
        .db
        .paths
        .secrets()
        .map(|s| s.denue.trim().is_empty())
        .unwrap_or(true);
    let mut screen = use_signal(Screen::default);
    let mut query = use_signal(|| Query {
        contactable: true,
        size: if service.layout == Layout::Ios {
            40
        } else {
            60
        },
        profile: Profile::parse(&service.settings.profile).unwrap_or_default(),
        ..Default::default()
    });
    let mut selected = use_signal(String::new);
    let table_layout = use_signal(prospect_core::workspace::TableLayout::default);
    let ui = UiState {
        screen,
        settings_tab: use_signal(move || if configured_denue_missing { 2 } else { 0 }),
        query,
        selected,
        revision: use_signal(|| 0u64),
        settings: use_signal(|| service.settings.clone()),
        job: use_signal(|| None),
        job_minimized: use_signal(|| false),
        dismissed_job_id: use_signal(String::new),
        heavy_task: use_signal(|| None),
        toast: use_signal(String::new),
        notifications: use_signal(Vec::new),
        notif_seq: use_signal(|| 0u64),
        busy: use_signal(|| false),
        pending: use_signal(|| None),
        mobile_layout,
    };
    use_context_provider(|| ui);
    let mut toast_ui = ui;
    use_effect(move || {
        let message = toast_ui.toast.read().clone();
        if message.trim().is_empty() {
            return;
        }
        toast_ui.toast.set(String::new());
        let next_id = *toast_ui.notif_seq.peek() + 1;
        toast_ui.notif_seq.set(next_id);
        let lower = message.to_lowercase();
        let is_error = lower.starts_with("error")
            || lower.contains("no se ")
            || lower.contains("cancelado")
            || lower.contains("falta")
            || lower.contains("inválid");
        toast_ui.notifications.with_mut(|list| {
            list.insert(
                0,
                FloatingNotification {
                    id: next_id,
                    message,
                    is_error,
                },
            );
            if list.len() > 4 {
                list.truncate(4);
            }
        });
        let mut auto_clear_ui = toast_ui;
        spawn(async move {
            tokio::time::sleep(Duration::from_secs(5)).await;
            auto_clear_ui
                .notifications
                .write()
                .retain(|n| n.id != next_id);
        });
    });
    let first_run_db = db.clone();
    let mut restored_ui = ui;
    use_effect(move || {
        let Some(mut ctrl) = onboarding else {
            return;
        };
        let Some(restored) = ctrl.restored.read().clone() else {
            return;
        };
        ctrl.restored.set(None);
        restored_ui.settings.set(restored.settings);
        restored_ui.revision += 1;
        restored_ui.toast.set(format!(
            "Respaldo cargado: {} empresas{}.",
            number(restored.records),
            if restored.tokens { " y tus tokens" } else { "" }
        ));
    });
    let mut first_run_ui = ui;
    use_effect(move || {
        if mobile_layout {
            return;
        }
        let Some(mut ctrl) = onboarding else {
            return;
        };
        if !*ctrl.first_run_pending_sync.read() {
            return;
        }
        ctrl.first_run_pending_sync.set(false);
        let mut updated = first_run_ui.settings.peek().clone();
        if !updated.entities.iter().any(|e| e == "09") {
            updated.entities.push("09".into());
            updated.entities.sort();
            updated.entities.dedup();
            if let Err(e) = updated.save(&first_run_db.paths) {
                first_run_ui
                    .toast
                    .set(format!("No se guardó CDMX por defecto: {e}"));
            } else {
                first_run_ui.settings.set(updated);
            }
        }
        first_run_ui.settings_tab.set(1);
        first_run_ui.screen.set(Screen::Settings);
        job_start_sync(
            first_run_ui,
            first_run_db.clone(),
            Some(vec!["09".into()]),
            false,
            false,
            Some("Descarga inicial · Ciudad de México".into()),
        );
    });
    let auto_db = db.clone();
    let mut auto_ui = ui;
    let mut auto_checked = use_signal(|| false);
    use_effect(move || {
        if mobile_layout || *auto_checked.peek() {
            return;
        }
        auto_checked.set(true);
        if onboarding.is_some_and(|o| *o.first_run_pending_sync.peek()) {
            return;
        }
        let running_now = auto_ui
            .job
            .peek()
            .as_ref()
            .is_some_and(|j| j.snapshot().status == "running");
        if running_now {
            return;
        }
        let denue_ok = auto_db
            .paths
            .secrets()
            .map(|s| !s.denue.trim().is_empty())
            .unwrap_or(false);
        let today = prospect_core::config::today();
        let mut s = auto_ui.settings.peek().clone();
        if denue_ok && s.auto_update_due(today) {
            s.last_auto_update_date = today.to_string();
            if s.save(&auto_db.paths).is_ok() {
                auto_ui.settings.set(s);
                job_start_sync(
                    auto_ui,
                    auto_db.clone(),
                    None,
                    false,
                    true,
                    Some("Actualización automática".into()),
                );
            }
        }
    });
    let mobile_navigation = use_context_provider(mobile::Navigation::new);
    let navigation_identity = use_memo(move || {
        (
            *screen.read(),
            selected.read().clone(),
            query.read().profile,
        )
    });
    use_effect(move || {
        let _identity = navigation_identity.read();
        mobile_navigation.reset();
    });
    use_context_provider(|| Signal::new(ViewCache::default()));
    let mutation_db = db.clone();
    let mut mutation_ui = ui;
    use_effect(move || {
        let request = mutation_ui.pending.read().clone();
        if let Some(request) = request {
            let db = mutation_db.clone();
            let settings = mutation_ui.settings.peek().clone();
            spawn(async move {
                match db.mutate(request, settings).await {
                    Ok(outcome) => {
                        mutation_ui.toast.set(format!("Guardado · {outcome}"));
                        mutation_ui.revision += 1;
                    }
                    Err(e) => mutation_ui.toast.set(e.to_string()),
                }
                mutation_ui.busy.set(false);
                mutation_ui.pending.set(None);
            });
        }
    });
    let mut job_state = use_signal(JobState::default);
    let mut job_ui = ui;
    use_future(move || async move {
        let mut last_id = String::new();
        let mut last_status = String::new();
        loop {
            if let Some(control) = job_ui.job.peek().as_ref() {
                let state = control.snapshot();
                if state.id != last_id || state.status != last_status {
                    if state.status != "running" {
                        job_ui.revision += 1;
                    }
                    last_id = state.id.clone();
                    last_status = state.status.clone();
                }
                if *job_state.peek() != state {
                    job_state.set(state);
                }
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    });
    let refresh_db = db.clone();
    let mut refresh_ui = ui;
    use_future(move || {
        let refresh_db = refresh_db.clone();
        async move {
            let mut date = None;
            let mut external_version = None;
            loop {
                // Only local SQLite metadata; never a source download or priority scan.
                match refresh_db.external_data_version().await {
                    Ok(version) => {
                        if external_version.is_some_and(|previous| previous != version) {
                            refresh_ui.revision += 1;
                        }
                        external_version = Some(version);
                    }
                    Err(e) => refresh_ui.toast.set(e.to_string()),
                }
                let current = prospect_core::config::today();
                if date != Some(current) {
                    match refresh_db.refresh_priorities().await {
                        Ok(changed) => {
                            if changed > 0 || date != Some(current) {
                                refresh_ui.revision += 1;
                            }
                        }
                        Err(e) => refresh_ui.toast.set(e.to_string()),
                    }
                }
                date = Some(current);
                tokio::time::sleep(Duration::from_secs(60)).await;
            }
        }
    });
    let page_db = db.clone();
    let page = use_cached_read(
        move || {
            if !matches!(
                *screen.read(),
                Screen::Today | Screen::Followups | Screen::Conversations | Screen::Base
            ) {
                return format!("{}|page:inactive", ui.revision.read());
            }
            format!(
                "{}|page:{}",
                ui.revision.read(),
                serde_json::to_string(&*query.read()).expect("query key")
            )
        },
        move || {
            let active = matches!(
                *screen.read(),
                Screen::Today | Screen::Followups | Screen::Conversations | Screen::Base
            );
            let q = query.read().clone();
            let _ = *ui.revision.read();
            let db = page_db.clone();
            async move {
                if active {
                    db.page(q).await.map_err(|e| e.to_string())
                } else {
                    Ok(Page::default())
                }
            }
        },
    );
    let summary_db = db.clone();
    let summary = use_cached_read(
        move || {
            format!(
                "{}|summary:{}",
                ui.revision.read(),
                query.read().profile.id()
            )
        },
        move || {
            let _ = *ui.revision.read();
            let profile = query.read().profile;
            let db = summary_db.clone();
            async move { db.summary(profile).await.map_err(|e| e.to_string()) }
        },
    );
    let dim_db = db.clone();
    let dimensions = use_cached_read(
        move || format!("{}|dimensions:{}", ui.revision.read(), query.read().entidad),
        move || {
            let _ = *ui.revision.read();
            let db = dim_db.clone();
            let entity = query.read().entidad.clone();
            async move { db.dimensions_for(entity).await.map_err(|e| e.to_string()) }
        },
    );
    use_effect(move || {
        if let Some(p) = page.value()
            && !mobile_layout
            && !p.items.iter().any(|p| p.id == *selected.peek())
        {
            selected.set(p.items.first().map(|p| p.id.clone()).unwrap_or_default());
        }
    });
    let page_data = page.value().unwrap_or_default();
    let totals = summary.value().unwrap_or_default();
    let dims = dimensions.value().unwrap_or_default();
    let q = query.read().clone();
    let settings = ui.settings.read().clone();
    let profile = q.profile;
    let running = job_state.read().status == "running";
    let state = job_state.read().clone();
    let current = *screen.read();
    if mobile_layout {
        use_context_provider(|| mobile::MobileData {
            ui,
            page,
            summary,
            dimensions,
            job_state,
        });
        return rsx! { mobile::MobileShell {} };
    }
    let menu = [
        (Screen::Dashboard, 0),
        (Screen::Today, totals.today),
        (Screen::Followups, totals.due),
        (Screen::Conversations, totals.conversations),
        (Screen::Base, totals.total),
        (Screen::Metrics, 0),
        (Screen::Territories, 0),
        (Screen::Opportunities, 0),
        (Screen::Indicators, 0),
    ];
    let paths_settings_btn = db.paths.clone();
    let paths_profile = db.paths.clone();
    rsx! {
        style { {include_str!("style.css")} }
        div { class:"app", "data-theme":settings.theme,
            aside {class:"sidebar", aria_label:"Navegación principal",
                div {class:"brand",img{class:"brand-icon",src:brand_icon.clone(),alt:"",aria_hidden:"true"}div{strong{"Prospect"}span{"Intelligence"}}}
                p {class:"eyebrow", "TU MESA DE TRABAJO"}
                nav {for (target,count)in menu{button{class:if current==target{"nav active"}else{"nav"},aria_current:if current==target{"page"}else{"false"},onclick:move |_|{screen.set(target);query.write().view=target.view();query.write().page=0;query.write().contactable=target==Screen::Today;selected.set(String::new());},span{"{target.label()}"}if count>0{span{class:"nav-count","{number(count)}"}}}}}
                div {class:"sidebar-bottom",div{class:"local-badge",span{class:"dot"} "Personal · datos locales"}button{class:if current==Screen::Settings && *ui.settings_tab.read()==1{"nav active"}else{"nav"},onclick:move |_|navigate(Screen::Updates,ui),"Actualizaciones"}button{class:if current==Screen::Settings && *ui.settings_tab.read()!=1{"nav active"}else{"nav"},onclick:move |_|{let mut u=ui;let missing=paths_settings_btn.secrets().map(|s|s.denue.trim().is_empty()).unwrap_or(true);u.settings_tab.set(if missing{2}else{0});u.screen.set(Screen::Settings);},"Ajustes"}p{class:"muted small","Fuente principal: INEGI / DENUE"}}
            }
            main {class:"main",
                header {class:"topbar",div{p{class:"eyebrow","PROSPECCIÓN Y NEGOCIO"}h1{"{current.label()}"}p{class:"muted small","{prospect_core::config::today()} · Una próxima acción clara."}}
                    div{class:"header-controls",label{class:"sr-only",r#for:"offer-profile","Perfil comercial"}select{id:"offer-profile",value:profile.id(),onchange:move|e|{if let Ok(p)=Profile::parse(&e.value()){query.write().profile=p;query.write().page=0;let mut u=ui;u.settings.write().profile=p.id().into();if let Err(e)=u.settings.peek().save(&paths_profile){u.toast.set(format!("No se guardaron las preferencias: {e}"));}}},option{value:"automatizacion","Automatización y sistemas"}option{value:"web_marketing","Web y marketing"}}

                    }
                }
                if summary.value().is_some_and(|s|s.total==0) && current!=Screen::Settings && current!=Screen::Updates {
                    div{class:"first-run notice",strong{"Empieza con tu propia base"}p{class:"small","En Ajustes → Tokens y fuentes, añade tu token DENUE. Elige estados y ejecuta lotes en Base de datos y alcance."}if let Some(mut o) = onboarding { { let _ = *o.revision.read(); let missing = db.paths.secrets().map(|s|s.denue.trim().is_empty()).unwrap_or(true); rsx! { if missing { button{class:"button primary compact",onclick:move |_|o.open.set(true),"Configurar tokens"} } } } }button{class:"button secondary compact",onclick:move |_|{let mut u=ui;u.settings_tab.set(0);u.screen.set(Screen::Settings);},"Abrir Ajustes"}button{class:"button secondary compact",onclick:move |_|navigate(Screen::Updates,ui),"Gestionar datos"}}
                }
                NotificationCenter{state:state.clone(),running,job_state}
                JobLoadingOverlay{state:state.clone(),running}
                match current {
                    Screen::Today|Screen::Followups|Screen::Conversations|Screen::Base=>rsx!{
                        div{class:"daily-strip",DailyGoal{label:"Revisados",done:totals.researched_today,goal:settings.daily_research}DailyGoal{label:"Contactos",done:totals.contacted_today,goal:settings.daily_contacts}DailyGoal{label:"Seguimientos",done:totals.followed_today,goal:settings.daily_followups}button{class:"link-button",disabled:*ui.busy.read(),onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Publication{},String::new(),None,profile,ui,s.db);},"Registrar publicación · {totals.publications_today}"}}
                        div{class:"toolbar",SearchBox{query}
                            select{aria_label:"Filtrar por giro",value:q.giro,onchange:move|e|{query.write().giro=e.value();query.write().page=0;},option{value:"","Todos los giros"}for giro in dims.0{option{value:giro.clone(),"{giro}"}}}
                            select{aria_label:"Filtrar por municipio",value:q.municipio,onchange:move|e|{query.write().municipio=e.value();query.write().page=0;},option{value:"","Todos los municipios"}for municipality in dims.1{option{value:municipality.clone(),"{municipality}"}}}
                            details{class:"filter-popover",summary{"Más filtros"}div{class:"filter-body",label{"Fit mínimo · {q.fit_min}" input{r#type:"range",min:"0",max:"100",value:"{q.fit_min}",oninput:move|e|{query.write().fit_min=e.value().parse().unwrap_or(0);query.write().page=0;}}}label{class:"check-label",input{r#type:"checkbox",checked:q.contactable,onchange:move|e|{query.write().contactable=e.checked();query.write().page=0;}}"Solo con contacto"}label{"Entidad" select{value:q.entidad.clone(),onchange:move|e|{query.write().entidad=e.value();query.write().municipio.clear();query.write().giro.clear();query.write().page=0;},option{value:"","Todas las entidades"}for (_,name)in prospect_core::config::ENTITIES{option{value:*name,"{name}"}}}}label{"Estado comercial" select{value:q.state,onchange:move|e|{query.write().state=e.value();query.write().page=0;},option{value:"","Todos"}for status in prospect_core::config::STATES{option{value:*status,"{status}"}}}}}}
                            if current==Screen::Base{ExportButtons{query:query.read().clone()} }
                        }
                        div{class:"saved-tools",SavedFilters{query}if current==Screen::Base{workspace::SavedWorkViews{key:"{profile.id()}",query,layout:table_layout}}}
                        if current==Screen::Base{workspace::TableControls{query,layout:table_layout}if let Some(status)=page.status("la base"){ {status} }else{BaseTable{data:page_data.clone(),selected,query,layout:table_layout.read().clone()}}}
                        else{div{class:"work-grid",section{class:"panel queue",aria_label:"Lista priorizada",div{class:"panel-heading",h2{if current==Screen::Followups{"Pendientes de seguimiento"}else if current==Screen::Conversations{"Conversaciones abiertas"}else{"Tu lista priorizada"}}span{class:"badge","{number(page_data.total)}"}}
                            div{class:"queue-list",if let Some(status)=page.status("los establecimientos"){ {status} }else if page_data.items.is_empty(){div{class:"empty",h3{"Sin pendientes con estos filtros"}p{"Ajusta filtros o incorpora más establecimientos desde Base."}}}for p in page_data.items.iter(){{let id=p.id.clone();let active=*selected.read()==p.id;rsx!{button{class:if active{"queue-item selected"}else{"queue-item"},onclick:move |_|selected.set(id.clone()),div{class:"queue-title",strong{"{p.nombre}"}span{class:"score-chip","{p.priority_score}"}}p{"{p.municipio} · {p.giro}"}div{class:"queue-footer",span{"Fit {p.fit_score}"}span{"{p.estado}"}}}}}}
                            }Pagination{data:page_data.clone(),query}
                        }if !selected.read().is_empty(){ProspectDetail{key:"{selected}:{profile.id()}",id:selected.read().clone(),profile}}else{section{class:"panel detail-empty",h2{"Elige un establecimiento"}p{class:"muted","Verás contexto, evidencia y la siguiente acción."}}}}}
                    },
                    Screen::Dashboard=>rsx!{DashboardPanel{key:"{profile.id()}",profile}},
                    Screen::Updates|Screen::Settings=>rsx!{settings::SettingsPanel{}},
                    Screen::Metrics=>rsx!{MetricsPanel{key:"{profile.id()}",profile}},
                    Screen::Territories=>rsx!{TerritoriesPanel{key:"{profile.id()}",profile,screen,query,selected}},
                    Screen::Opportunities=>rsx!{OpportunitiesPanel{key:"{profile.id()}",profile}},
                    Screen::Indicators=>rsx!{IndicatorsPanel{}},
                }
                if current==Screen::Base{div{class:"base-actions",button{class:"button primary",onclick:move |_|navigate(Screen::Updates,ui),"Actualizar datos y prioridades"}span{class:"muted small","Los datos públicos se actualizan sin borrar tu seguimiento."}}}
            }
        }
    }
}
#[component]
fn DailyGoal(label: &'static str, done: usize, goal: usize) -> Element {
    let pct = done
        .saturating_mul(100)
        .checked_div(goal)
        .unwrap_or(0)
        .min(100);
    rsx! {div{class:"daily-goal",span{class:"muted small","{label}"}strong{"{done}" span{class:"muted",if goal==0{" · sin meta"}else{" / {goal}"}}}div{class:"goal-track",div{style:"width:{pct}%"}}}}
}
#[component]
fn SearchBox(mut query: Signal<Query>, #[props(default)] compact: bool) -> Element {
    let mut draft = use_signal(|| query.peek().search.clone());
    let mut external = use_signal(|| query.peek().search.clone());
    use_effect(move || {
        let search = query.read().search.clone();
        if *external.peek() != search {
            external.set(search.clone());
            if *draft.peek() != search {
                draft.set(search);
            }
        }
    });
    let debounced = use_resource(move || {
        let text = draft.read().clone();
        async move {
            tokio::time::sleep(Duration::from_millis(250)).await;
            text
        }
    });
    use_effect(move || {
        if let Some(text) = debounced.read().as_ref()
            && query.peek().search != *text
        {
            query.with_mut(|q| {
                q.search = text.clone();
                q.page = 0;
            });
        }
    });
    rsx! {label{class:"search",span{aria_hidden:"true","⌕"}input{r#type:"search",placeholder:if compact{"Buscar negocios"}else{"Nombre, razón social o municipio…"},aria_label:"Buscar establecimientos",onfocus:move |_|mobile::set_editing(true),onblur:move |_|mobile::set_editing(false),value:"{external}",oninput:move|event|draft.set(event.value())}}}
}
// Desktop events cross an IPC boundary. Keep the native editable value stable
// while typing; remount with a key only when loading another draft.
#[derive(Props, Clone)]
struct TextEntryProps {
    initial: String,
    oninput: EventHandler<FormEvent>,
    #[props(default)]
    multiline: bool,
    #[props(default = "text")]
    kind: &'static str,
    #[props(default)]
    placeholder: String,
    #[props(default)]
    label: String,
    #[props(default = "")]
    class: &'static str,
    #[props(default = "text")]
    inputmode: &'static str,
    #[props(default = 3)]
    rows: usize,
}
impl PartialEq for TextEntryProps {
    fn eq(&self, other: &Self) -> bool {
        // Handlers capture the same live signals for this keyed draft. Ignore
        // callback allocation changes so IPC events do not reapply input.value.
        self.initial == other.initial
            && self.multiline == other.multiline
            && self.kind == other.kind
            && self.placeholder == other.placeholder
            && self.label == other.label
            && self.class == other.class
            && self.inputmode == other.inputmode
            && self.rows == other.rows
    }
}
#[allow(non_snake_case)]
fn TextEntry(props: TextEntryProps) -> Element {
    let TextEntryProps {
        initial,
        oninput,
        multiline,
        kind,
        placeholder,
        label,
        class,
        inputmode,
        rows,
    } = props;
    if multiline {
        rsx! {textarea{class, rows, aria_label:(!label.is_empty()).then_some(label), value:initial, onfocus:move |_|mobile::set_editing(true),onblur:move |_|mobile::set_editing(false), oninput:move |e|oninput.call(e)}}
    } else {
        rsx! {input{class, r#type:kind, placeholder, aria_label:(!label.is_empty()).then_some(label), inputmode, autocomplete:if kind=="password"{"off"}else{"on"}, value:initial, onfocus:move |_|mobile::set_editing(true),onblur:move |_|mobile::set_editing(false), oninput:move |e|oninput.call(e)}}
    }
}

#[component]
fn Pagination(data: Page, mut query: Signal<Query>) -> Element {
    let max = if data.total == 0 {
        1
    } else {
        data.total.div_ceil(data.size.max(1))
    };
    rsx! {div{class:"pagination",button{class:"button secondary compact",disabled:data.page==0,onclick:move |_|query.write().page-=1,"Anterior"}span{"{data.page+1} / {max}"}button{class:"button secondary compact",disabled:data.page+1>=max,onclick:move |_|query.write().page+=1,"Siguiente"}}}
}
#[component]
fn ExportButtons(query: Query) -> Element {
    let ui = use_context::<UiState>();
    rsx! {for format in ["csv","xlsx"]{{let q=query.clone();rsx!{button{class:"button secondary compact",disabled:ui.heavy_task.read().is_some(),onclick:move |_|{let db=consume_context::<Services>().db;let q=q.clone();let mut u=ui;u.heavy_task.set(Some(format!("Exportando registros ({})…",format.to_uppercase())));spawn(async move{let res=db.export(q,format).await;u.heavy_task.set(None);match res{Ok(p)=>u.toast.set(available_file(p).unwrap_or_else(|e|e)),Err(e)=>u.toast.set(e.to_string())}});},"{format.to_uppercase()}"}}}}}
}
/// Opens a saved search on the screen that shows its view and keeps the chosen offer.
fn open_saved_filter(
    q: Query,
    mut query: Signal<Query>,
    mut ui: UiState,
    paths: &prospect_core::AppPaths,
) {
    let screen = match q.view {
        View::Today => Screen::Today,
        View::Due => Screen::Followups,
        View::Conversations => Screen::Conversations,
        View::Clients | View::All => Screen::Base,
    };
    if ui.settings.peek().profile != q.profile.id() {
        let mut settings = ui.settings.peek().clone();
        settings.profile = q.profile.id().into();
        if let Err(e) = settings.save(paths) {
            ui.toast
                .set(format!("No se guardaron las preferencias: {e}"));
        }
        ui.settings.set(settings);
    }
    if *ui.screen.peek() != screen {
        ui.selected.set(String::new());
        ui.screen.set(screen);
    }
    query.set(q);
}
#[component]
fn SavedFilters(mut query: Signal<Query>) -> Element {
    let ui = use_context::<UiState>();
    let db = consume_context::<Services>().db;
    let mut name = use_signal(String::new);
    let mut name_epoch = use_signal(|| 0u64);
    let paths = db.paths.clone();
    let list_db = db.clone();
    let filters = use_resource(move || {
        let _ = *ui.revision.read();
        let db = list_db.clone();
        async move { db.saved_filters().await.unwrap_or_default() }
    });
    let items = filters.read().as_ref().cloned().unwrap_or_default();
    rsx! {details{class:"saved-filters",summary{"Búsquedas guardadas"}div{class:"saved-inner",TextEntry{key:"saved-name:{name_epoch}",label:"Nombre de la búsqueda",initial:String::new(),placeholder:"Ej. Industria Puebla",oninput:move|e:FormEvent|name.set(e.value())}button{class:"button secondary compact",onclick:move |_|{let db=db.clone();let n=name.read().clone();let q=query.read().clone();let mut ui=ui;spawn(async move{match db.save_filter(n,q).await{Ok(())=>{name.set(String::new());name_epoch+=1;ui.revision+=1;ui.toast.set("Búsqueda guardada".into());},Err(e)=>ui.toast.set(e.to_string())}});},"Guardar"}for (label,q)in items{{let paths=paths.clone();rsx!{button{class:"pill-button",onclick:move |_|open_saved_filter(q.clone(),query,ui,&paths),"{label}"}}}}}}}
}
#[component]
fn PanelTabs(labels: Vec<&'static str>, mut active: Signal<usize>, name: &'static str) -> Element {
    rsx! {div{class:"panel-tabs",role:"group",aria_label:name,
        for (index,label) in labels.iter().enumerate(){button{
            class:if *active.read()==index{"panel-tab active"}else{"panel-tab"},
            aria_pressed:*active.read()==index,
            onclick:move |_|active.set(index),"{label}"
        }}
    }}
}

#[component]
fn BaseTable(
    data: Page,
    mut selected: Signal<String>,
    query: Signal<Query>,
    layout: prospect_core::workspace::TableLayout,
) -> Element {
    use prospect_core::workspace::{Density, TableColumn};
    let q = query.read().clone();
    let profile = q.profile;
    let mut pane = use_signal(|| usize::from(!query.peek().search.is_empty()));
    rsx! {div{class:"base-work",
        PanelTabs{labels:vec!["Listado","Ficha seleccionada"],active:pane,name:"Vista de la base"}
        div{class:"base-list-pane",hidden:*pane.read()!=0,
            section{class:if layout.density==Density::Compact{"panel base-table configurable-table compact-table"}else{"panel base-table configurable-table"},
                div{class:"panel-heading",h2{"{number(data.total)} establecimientos"}span{class:"muted small","No necesariamente empresas distintas"}}
                div{class:"table-wrap",table{
                    thead{tr{for column in &layout.columns{th{scope:"col","{column.label()}"}}}}
                    tbody{for (index,p) in data.items.iter().enumerate(){
                        if q.group!=prospect_core::QueryGroup::None && (index==0 || q.group.key(p)!=q.group.key(&data.items[index-1])) {
                            tr{class:"table-group",th{scope:"rowgroup",colspan:layout.columns.len().to_string(),"{q.group.label()} · {q.group.key(p)}" span{class:"muted small"," · en esta página"}}}
                        }
                        tr{class:if *selected.read()==p.id{"selected-row"}else{""},
                            for column in &layout.columns {{let id=p.id.clone();rsx!{td{
                                match column {
                                    TableColumn::Name=>rsx!{button{class:"table-name",onclick:move |_|{selected.set(id.clone());pane.set(1);},"{p.nombre}"}},
                                    TableColumn::Entity=>rsx!{"{p.entidad}"},
                                    TableColumn::Municipality=>rsx!{"{p.municipio}"},
                                    TableColumn::Sector=>rsx!{"{p.giro}"},
                                    TableColumn::Fit=>rsx!{"{p.fit_score}"},
                                    TableColumn::Priority=>rsx!{strong{"{p.priority_score}"}},
                                    TableColumn::State=>rsx!{span{class:"badge","{p.estado}"}},
                                    TableColumn::Contact=>rsx!{if p.contactable(){"Disponible"}else{"Por verificar"}},
                                }
                            }}}}
                        }
                    }}
                }}
                if data.items.is_empty(){p{class:"empty","Sin resultados. Cambia filtros o actualiza DENUE."}}
            }
            Pagination{data,query}
        }
        if !selected.read().is_empty(){div{class:"base-detail",hidden:*pane.read()!=1,ProspectDetail{key:"{selected}:{profile.id()}",id:selected.read().clone(),profile}}}
    }}
}

#[derive(Clone, PartialEq)]
struct DetailData {
    prospect: Prospect,
    signals: Vec<WebSignal>,
    history: Vec<Interaction>,
    message: Message,
}
fn channel_available(p: &Prospect, channel: &str) -> bool {
    if channel == "email" {
        !p.email().is_empty()
    } else {
        !p.phone().is_empty()
    }
}
/// The recommended channel when the establishment has it; otherwise whichever it has.
fn start_channel(p: &Prospect) -> &'static str {
    let recommended = ["email", "telefono", "whatsapp"]
        .into_iter()
        .find(|channel| p.canal == *channel);
    match recommended {
        Some(channel) if channel_available(p, channel) => channel,
        _ if !p.email().is_empty() || p.phone().is_empty() => "email",
        _ => "telefono",
    }
}
#[component]
fn ProspectDetail(id: String, profile: Profile) -> Element {
    let service = consume_context::<Services>();
    let ui = use_context::<UiState>();
    let mut channel = use_signal(|| "email".to_owned());
    let mut channel_for = use_signal(String::new);
    let mut body = use_signal(String::new);
    let mut subject = use_signal(String::new);
    let mut notes = use_signal(String::new);
    let mut signature = use_signal(String::new);
    let context_pane = use_signal(|| 0usize);
    let mut action_pane = use_signal(|| 0usize);
    let mut response = use_signal(|| "interesado".to_owned());
    let mut state = use_signal(|| "revisado".to_owned());
    let id_for_resource = id.clone();
    let db = service.db.clone();
    let detail = use_resource(move || {
        let id = id_for_resource.clone();
        let _ = *ui.revision.read();
        let settings = ui.settings.read().clone();
        let ch = channel.read().clone();
        let db = db.clone();
        async move {
            let p = db
                .get(id.clone(), profile)
                .await
                .map_err(|e| e.to_string())?;
            let signals = db.signals(id.clone()).await.map_err(|e| e.to_string())?;
            let history = db.history(id).await.map_err(|e| e.to_string())?;
            let t = prospect_core::messages::Templates::load(&db.paths.home)
                .map_err(|e| e.to_string())?;
            let message = if p.estado == "contactado" {
                prospect_core::messages::followup(&p, p.toques.max(1), &settings, &t, profile)
            } else if ["nuevo", "revisado"].contains(&p.estado.as_str()) {
                prospect_core::messages::first(
                    &p,
                    if ch == "email" { "email" } else { "whatsapp" },
                    &signals,
                    &settings,
                    &t,
                    profile,
                )
                .map_err(|e| e.to_string())?
            } else {
                Message {
                    canal: "manual".into(),
                    variante: "MANUAL".into(),
                    cuerpo: String::new(),
                    asunto: String::new(),
                    problemas: vec![],
                }
            };
            Ok::<_, String>(DetailData {
                prospect: p,
                signals,
                history,
                message,
            })
        }
    });
    // Start each establishment on a channel it actually has.
    use_effect(move || {
        if let Some(Ok(d)) = detail.read().as_ref()
            && *channel_for.peek() != d.prospect.id
        {
            channel_for.set(d.prospect.id.clone());
            let start = start_channel(&d.prospect);
            if *channel.peek() != start {
                channel.set(start.to_owned());
            }
        }
    });
    use_effect(move || {
        if let Some(Ok(d)) = detail.read().as_ref() {
            let key = format!(
                "{}:{}:{}:{}:{}",
                d.prospect.id,
                profile.id(),
                channel.read(),
                d.prospect.estado,
                d.prospect.toques
            );
            if *signature.peek() != key {
                body.set(d.message.cuerpo.clone());
                subject.set(d.message.asunto.clone());
                notes.set(d.prospect.notas.clone());
                state.set(d.prospect.estado.clone());
                action_pane.set(usize::from(
                    !["nuevo", "revisado", "contactado"].contains(&d.prospect.estado.as_str()),
                ));
                signature.set(key);
            }
        }
    });
    let data = detail
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned();
    let Some(data) = data else {
        return rsx! {section{class:"panel",p{class:"empty",if let Some(Err(e))=detail.read().as_ref(){"{e}"}else{"Cargando contexto…"}}}};
    };
    let p = data.prospect;
    let settings = ui.settings.read().clone();
    let actual_channel = channel.read().clone();
    let problems = prospect_core::messages::validate(
        &body.read(),
        if p.estado == "contactado" {
            "seguimiento"
        } else if actual_channel == "email" {
            "email"
        } else {
            "whatsapp"
        },
        &subject.read(),
        prospect_core::rules::observation(&data.signals).is_some(),
        &settings,
    );
    let valid = problems.iter().all(|e| e.starts_with("Aviso:"));
    let channel_ok = channel_available(&p, &actual_channel);
    let active = !*ui.busy.read();
    let fresh = ["nuevo", "revisado"].contains(&p.estado.as_str());
    let due = p.estado == "contactado"
        && !p.proximo_seguimiento.is_empty()
        && p.proximo_seguimiento <= prospect_core::config::today().to_string();
    let email_link = prospect_core::contacts::mailto(p.email(), &subject.read(), &body.read());
    let wa_link = prospect_core::contacts::whatsapp_url(&p.phone(), &body.read());
    let phone_link = if p.phone().is_empty() {
        String::new()
    } else {
        format!("tel:+52{}", p.phone())
    };
    let website = prospect_core::contacts::normalize_url(&p.sitio)
        .map(|u| u.to_string())
        .unwrap_or_default();
    let breakdown: Vec<prospect_core::ScoreItem> =
        serde_json::from_str(&p.fit_breakdown).unwrap_or_default();
    let priority_breakdown: Vec<prospect_core::ScoreItem> =
        serde_json::from_str(&p.priority_breakdown).unwrap_or_default();
    let score_class = if p.fit_score >= 70 {
        "score-large good"
    } else {
        "score-large"
    };
    let mobile_layout = service.layout == Layout::Ios;
    let page = mobile::destination();
    let action_page = matches!(
        page,
        Destination::Message | Destination::Progress | Destination::Notes
    );
    let allow_template = fresh || p.estado == "contactado";
    let primary_text = if actual_channel == "telefono" {
        "Registrar llamada realizada"
    } else {
        "Registrar contacto enviado"
    };
    rsx! {div{class:"detail-columns",
            section{class:"panel context",hidden:mobile_layout && action_page,aria_label:"Contexto del establecimiento",div{class:"panel-heading",h2{"Contexto"}span{class:"badge","{p.estado}"}}if !mobile_layout {PanelTabs{labels:vec!["Resumen","Evidencia","Historial"],active:context_pane,name:"Contenido del establecimiento"}}div{class:"context-body",div{hidden:if mobile_layout {!matches!(page,Destination::Root|Destination::TerritoryProspect)}else{*context_pane.read()!=0},h2{class:"company-title","{p.nombre}"}if mobile_layout{span{class:"badge ios-crm-state","{p.estado}"}}p{class:"muted","{p.clase_actividad}"}p{class:"muted small","{p.municipio}, {p.entidad}"}if mobile_layout {mobile::QuickActions {pages:vec![Destination::Message,Destination::Progress,Destination::Notes]}}div{class:"score-row",div{class:score_class,strong{"{p.fit_score}"}span{"Fit"}}div{class:"score-large",strong{"{p.priority_score}"}span{"Prioridad"}}div{class:"size-label",span{"Personal"}strong{"{p.estrato}"}}}
                div{class:"contact-lines",if !p.email().is_empty(){p{strong{"Correo"}span{"{p.email()}"}}}if !p.phone().is_empty(){p{strong{"Teléfono"}span{"{prospect_core::contacts::pretty_phone(&p.phone())}"}}}if !website.is_empty(){{let link=website.clone();rsx!{button{class:"link-button",onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}},"Abrir sitio declarado ↗"}}}}}
                if mobile_layout {div{class:"action-row",if !p.email().is_empty(){{let value=p.email().to_owned();rsx!{button{class:"button secondary",onclick:move |_|copied(&value,ui),"Copiar correo"}}}}if !p.phone().is_empty(){{let value=p.phone();rsx!{button{class:"button secondary",onclick:move |_|copied(&value,ui),"Copiar teléfono"}}}}}}
                if mobile_layout { mobile::PageMenu {pages:vec![Destination::Evidence,Destination::History]} }
                if p.possible_chain!=0{div{class:"notice",strong{"Posible grupo · revisar"}p{"{p.group_reason}"}span{class:"small","{p.possible_chain_count} establecimientos asociados por reglas; no acredita empresa única."}}}
                div{class:"hypothesis",p{class:"eyebrow","HIPÓTESIS PARA CONVERSAR"}p{"{p.hypothesis_text}"}p{class:"muted small","{p.hypothesis_basis} · Confianza de la regla: {p.hypothesis_confidence}"}}
                }div{hidden:if mobile_layout {page!=Destination::Evidence}else{*context_pane.read()!=1},h3{class:"section-title","Dirección DENUE"}p{class:"small","{p.direccion} · {p.municipio}, {p.entidad}"}details{summary{"Por qué tiene este Fit"}for item in breakdown{div{class:"breakdown",span{"{item.etiqueta}"}strong{"{item.pts}"}p{class:"muted small","{item.nota}"}}}}
                details{summary{"Cómo se calcula la prioridad"}for item in priority_breakdown{div{class:"breakdown",span{"{item.etiqueta}"}strong{"{item.pts}"}p{class:"muted small","{item.nota}"}}}}
                h3{class:"section-title","Evidencia pública"}if data.signals.is_empty(){p{class:"muted small","Sin señales web verificadas. Puedes revisar su sitio o enriquecer desde Base."}}for signal in data.signals{{let link=signal.signal_source_url.clone();rsx!{div{class:"evidence",strong{"{prospect_core::rules::signal_label(&signal.signal_type)}"}p{"{signal.signal_text}"}button{class:"link-button small",onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}},"Ver fuente ↗"}}}}}
                }div{hidden:if mobile_layout {page!=Destination::History}else{*context_pane.read()!=2},h3{class:"section-title","Historial"}if data.history.is_empty(){p{class:"muted small","Aún no hay contactos registrados."}}for event in data.history{div{class:"history-entry",strong{"{event.interaction_type}"}span{class:"muted small","{event.created_at} · {event.channel.clone().unwrap_or_default()}"}if let Some(text)=event.message_rendered{details{summary{"Mensaje utilizado"}p{class:"message-preview","{text}"}}}if let Some(note)=event.notes{p{class:"small","{note}"}}}}
            }}}
            section{class:"panel actions",hidden:mobile_layout && !action_page,aria_label:"Acciones comerciales",div{class:"panel-heading",h2{"Siguiente acción"}}if !mobile_layout {PanelTabs{labels:vec!["Mensaje","Avance","Notas"],active:action_pane,name:"Acción comercial"}}div{class:"action-body",div{class:"message-pane",hidden:if mobile_layout {page!=Destination::Message}else{*action_pane.read()!=0},if p.estado=="contactado"{div{class:"notice",strong{if due{"Seguimiento pendiente"}else{"En secuencia"}}p{"Próximo: {p.proximo_seguimiento}"}}}else if ["respondio","respuesta_positiva","reunion","propuesta"].contains(&p.estado.as_str()){div{class:"notice",strong{"Conversación abierta"}p{"Continúa a partir de su respuesta; registra el siguiente avance."}}}
                if allow_template { div{class:"message-meta",label{"Canal" select{value:"{channel}",onchange:move|e|channel.set(e.value()),option{value:"email","Correo"}option{value:"telefono","Llamada"}option{value:"whatsapp","WhatsApp"}}}
                if actual_channel=="email"{label{"Asunto" TextEntry{key:"subject:{signature}",initial:data.message.asunto.clone(),oninput:move|e:FormEvent|subject.set(e.value())}}}
                }label{"Borrador editable" TextEntry{key:"body:{signature}",multiline:true,class:"message-editor",rows:9,initial:data.message.cuerpo.clone(),oninput:move|e:FormEvent|body.set(e.value())}}
                for problem in &problems{p{class:if problem.starts_with("Aviso:"){"muted small"}else{"error small"},"{problem}"}}
                div{class:"action-row",button{class:"button secondary",onclick:move |_|copied(&body.read(),ui),"Copiar mensaje"}if actual_channel=="email"&&!email_link.is_empty(){{let link=email_link.clone();rsx!{button{class:"button secondary",disabled:!valid,onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}},if mobile_layout{"Preparar correo"}else{"Abrir correo ↗"}}}}}if actual_channel=="whatsapp"&&!wa_link.is_empty(){{let link=wa_link.clone();rsx!{button{class:"button secondary",disabled:!valid,onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}},"Abrir WhatsApp ↗"}}}}if actual_channel=="telefono"&&!phone_link.is_empty(){{let link=phone_link.clone();rsx!{button{class:"button secondary",onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}},"Abrir llamada ↗"}}}}}
                p{class:"muted small",if mobile_layout&&actual_channel=="email"{"Revisa y envía en tu app de correo; después registra el contacto aquí."}else{"Revisa y envía tú. Abrir o copiar no registra el envío."}}
                if allow_template && !channel_ok {p{class:"error small",role:"status",if actual_channel=="email"{"Este negocio no tiene correo válido. Elige Llamada o WhatsApp para registrar el contacto."}else{"Este negocio no tiene teléfono válido. Elige Correo para registrar el contacto."}}}
                if fresh{{let pid=p.id.clone();let variant=data.message.variante.clone();rsx!{button{class:"button primary full",disabled:!active||!valid||!channel_ok,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Contact{channel:channel.read().clone(),variant:variant.clone(),message:body.read().clone(),subject:subject.read().clone()},pid.clone(),Some(notes.read().clone()),profile,ui,s.db);},"{primary_text}"}}}}
                if due{{let pid=p.id.clone();let variant=data.message.variante.clone();rsx!{button{class:"button primary full",disabled:!active||!valid||!channel_ok,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Followup{channel:channel.read().clone(),variant:variant.clone(),message:body.read().clone()},pid.clone(),Some(notes.read().clone()),profile,ui,s.db);},"Registrar seguimiento enviado"}}}}
                }
                if !allow_template{p{class:"muted small","Esta conversación ya está abierta. Registra el resultado en Avance o añade tus notas."}}
                }div{hidden:if mobile_layout {page!=Destination::Progress}else{*action_pane.read()!=1},details{open:true,summary{"Registrar respuesta"}label{"Resultado" select{value:"{response}",onchange:move|e|response.set(e.value()),for (value,label)in [("interesado","Interesado"),("pregunta","Hizo una pregunta"),("redirigido","Me redirigió"),("no_interesado","No interesado"),("ya_tienen_solucion","Ya tienen solución"),("no_es_prioridad","No es prioridad"),("numero_incorrecto","Número incorrecto"),("correo_rebotado","Correo rebotado"),("otro","Otra respuesta")]{option{value,"{label}"}}}}{let pid=p.id.clone();rsx!{button{class:"button secondary full",disabled:!active,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Response{response:response.read().clone()},pid.clone(),Some(notes.read().clone()),profile,ui,s.db);},"Guardar respuesta"}}}}
                details{summary{"Estado y revisión"}label{"Estado" select{value:"{state}",onchange:move|e|state.set(e.value()),for status in prospect_core::config::STATES{option{value:*status,"{status}"}}}}{let pid=p.id.clone();rsx!{button{class:"button secondary full",disabled:!active,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::State{state:state.read().clone()},pid.clone(),Some(notes.read().clone()),profile,ui,s.db);},"Guardar estado"}}}}
                div{class:"action-row",{let pid=p.id.clone();rsx!{button{class:"button secondary compact",disabled:!active,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Research{},pid.clone(),None,profile,ui,s.db);},"Revisado"}}}{let pid=p.id.clone();rsx!{button{class:"button secondary compact",disabled:!active,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Snooze{days:3},pid.clone(),None,profile,ui,s.db);},"Posponer 3 días"}}}}
                p{class:"muted small","Los puntajes son reglas de prioridad, no probabilidades de compra."}
                }div{hidden:if mobile_layout {page!=Destination::Notes}else{*action_pane.read()!=2},            label{"Notas" TextEntry{key:"notes:{signature}:{p.notas}",multiline:true,initial:p.notas.clone(),oninput:move|e:FormEvent|notes.set(e.value())}}{let pid=p.id.clone();rsx!{button{class:"button secondary full",disabled:!active,onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Note{},pid.clone(),Some(notes.read().clone()),profile,ui,s.db);},"Guardar notas"}}}
    }
            }}
        }}
}

#[component]
fn MetricsPanel(profile: Profile) -> Element {
    let db = consume_context::<Services>().db;
    let ui = use_context::<UiState>();
    let mut dimension = use_signal(|| "giro".to_owned());
    let metrics = use_cached_read(
        move || {
            format!(
                "{}|metrics:{}:{}",
                ui.revision.read(),
                profile.id(),
                dimension.read()
            )
        },
        move || {
            let _ = *ui.revision.read();
            let d = dimension.read().clone();
            let db = db.clone();
            async move { db.metrics(profile, d).await.map_err(|e| e.to_string()) }
        },
    );
    if let Some(status) = metrics.status("los datos de análisis") {
        return status;
    }
    let data = metrics.value().unwrap_or_default();
    let f = data.funnel.clone();
    let mobile = mobile::navigation().is_some();
    let page = mobile::destination();
    let stages = [
        ("Contactados", f.contactados),
        ("Respondieron", f.respondieron),
        ("Respuesta positiva", f.respuesta_positiva),
        ("Reuniones", f.reuniones),
        ("Propuestas", f.propuestas),
        ("Clientes", f.clientes),
    ];
    rsx! {div{class:"metrics-work",
        if mobile && page==Destination::Root {mobile::PageMenu {pages:vec![Destination::Funnel,Destination::Revenue,Destination::Segments]}}
        div{class:"stats-grid",hidden:mobile && page!=Destination::Root,StatCard{label:"Contactados",value:number(f.contactados),note:"Cohorte de la oferta"}StatCard{label:"Respuesta",value:f.response_rate.map(|x|format!("{x}%")).unwrap_or_else(||"—".into()),note:"Sobre contactados"}StatCard{label:"Reuniones",value:number(f.reuniones),note:"Etapa alcanzada"}StatCard{label:"Clientes",value:number(f.clientes),note:"Etapa alcanzada"}}
        div{class:"stats-grid revenue compact-revenue",hidden:mobile && page!=Destination::Revenue,StatCard{label:"Oportunidades",value:number(data.opportunity_count),note:"Captura manual"}StatCard{label:"Importe ganado MXN",value:format!("${}",amount(data.won_mxn_cents)),note:"No equivale a cobrado"}StatCard{label:"Cobrado MXN",value:format!("${}",amount(data.received_mxn_cents)),note:"Acumulado registrado"}StatCard{label:"Cobrado USD",value:format!("${}",amount(data.received_usd_cents)),note:"Moneda separada"}}
        div { class:"analytics-grid",
            section { class:"panel", hidden:mobile && page!=Destination::Funnel,
                div { class:"panel-heading", h2 { "Embudo comercial" } }
                div { class:"funnel",
                    if f.contactados>0 {for (label,count) in stages {
                        div { class:"funnel-row", span { "{label}" }
                            div { class:"funnel-track", div { style:format!("width:{}%",count.saturating_mul(100).checked_div(f.contactados).unwrap_or(0)) } }
                            strong { "{number(count)}" }
                        }
                    }
                    }if f.contactados==0 { p { class:"empty", "Registra tus primeros contactos para evaluar qué funciona." } }
                }
                p { class:"muted small panel-note", "Las tasas describen lo registrado. Una muestra pequeña no permite declarar ganadores." }
            }
        section{class:"panel",hidden:mobile && page!=Destination::Segments,div{class:"panel-heading",h2{"Qué segmentos avanzan"}select{aria_label:"Dimensión de métricas",value:"{dimension}",onchange:move|e|dimension.set(e.value()),for (value,label)in [("giro","Giro"),("clase_id","SCIAN"),("estrato","Personal"),("municipio","Municipio"),("canal_contacto","Canal"),("variante_usada","Variante"),("fit_bucket","Rango de fit"),("priority_bucket","Rango de prioridad")]{option{value,"{label}"}}}}if mobile {div{class:"ios-data-list",for c in &data.cuts {article{class:"ios-data-row",strong{"{c.label}"}span{class:"small","{number(c.counts.contactados)} contactos · {number(c.counts.respondieron)} respuestas · {number(c.counts.clientes)} clientes"}p{class:"muted small","{c.lectura}"}}}if data.cuts.is_empty(){p{class:"empty","Todavía no hay resultados comerciales para comparar."}}}}
        else {div{class:"table-wrap",table{thead{tr{for header in ["Segmento","Contactados","Respuestas","Clientes","Lectura"]{th{scope:"col","{header}"}}}}tbody{for c in data.cuts{tr{td{"{c.label}"}td{"{c.counts.contactados}"}td{"{c.counts.respondieron}"}td{"{c.counts.clientes}"}td{class:"muted small","{c.lectura}"}}}}}}if f.contactados==0{p{class:"empty","Todavía no hay resultados comerciales para comparar."}}}}}

    }}
}
#[component]
#[allow(unused_braces)]
fn StatCard(label: &'static str, value: String, note: &'static str) -> Element {
    rsx! {div{class:"stat-card",span{class:"muted small","{label}"}strong{"{value}"}span{class:"muted small","{note}"}}}
}
// Public DENUE coordinates only; map requests receive no CRM fields.
fn map_link(lat: f64, lon: f64) -> Option<String> {
    if !lat.is_finite()
        || !lon.is_finite()
        || !(-85.05112878..=85.05112878).contains(&lat)
        || !(-180.0..=180.0).contains(&lon)
    {
        return None;
    }
    Some(format!(
        "https://www.openstreetmap.org/?mlat={lat:.6}&mlon={lon:.6}#map=16/{lat:.6}/{lon:.6}"
    ))
}
#[component]
fn StreetMap(lat: f64, lon: f64) -> Element {
    let services = consume_context::<Services>();
    let aspect = if services.layout == Layout::Ios {
        "xMidYMid slice"
    } else {
        "xMidYMid meet"
    };
    let home = services.db.paths.home;
    let mut zoom = use_signal(|| 15u8);
    let tiles = use_resource(move || {
        let home = home.clone();
        let z = *zoom.read();
        async move {
            prospect_core::maps::load_map(home, lat, lon, z)
                .await
                .map_err(|e| e.to_string())
        }
    });
    rsx! {div{class:"street-map",
        if let Some(status)=resource_status(tiles,"el mapa"){div{class:"map-status",{status}}}else if let Some(Ok(images))=tiles.read().as_ref(){
            svg{class:"map-canvas",view_box:"0 0 896 640",preserve_aspect_ratio:aspect,role:"img","aria-label":"Calles y marcador del establecimiento seleccionado",
                for (tile,url)in images{image{x:"{tile.left}",y:"{tile.top}",width:"256",height:"256",href:url.clone()}}
                g{transform:"translate(448,320)",circle{cx:"0",cy:"0",r:"16",fill:"#ffffff",opacity:"0.65"}path{d:"M0 0C-5-7-12-14-12-22A12 12 0 0 1 12-22C12-14 5-7 0 0Z",fill:"#226e68",stroke:"#fff",stroke_width:"2"}circle{cx:"0",cy:"-22",r:"4",fill:"#fff"}}
            }
        }
        div{class:"map-zoom",aria_label:"Escala del mapa",button{class:"icon-button",aria_label:"Acercar mapa",disabled:*zoom.read()>=18,onclick:move |_|zoom+=1,"+"}button{class:"icon-button",aria_label:"Alejar mapa",disabled:*zoom.read()<=12,onclick:move |_|zoom-=1,"−"}}
        button{class:"map-attribution link-button small",onclick:move |_|{let mut ui=consume_context::<UiState>();if let Err(e)=open_link("https://www.openstreetmap.org/copyright"){ui.toast.set(e);}},"© OpenStreetMap"}
    }}
}
#[component]
fn TerritoriesPanel(
    profile: Profile,
    mut screen: Signal<Screen>,
    mut query: Signal<Query>,
    mut selected: Signal<String>,
) -> Element {
    let db = consume_context::<Services>().db;
    let ui = use_context::<UiState>();
    let mut municipality = use_signal(String::new);
    let mut location = use_signal(String::new);
    let pane = use_signal(|| 0usize);
    let metrics = use_cached_read(
        move || format!("{}|metrics:{}:municipio", ui.revision.read(), profile.id()),
        move || {
            let _ = *ui.revision.read();
            let db = db.clone();
            async move {
                db.metrics(profile, "municipio".into())
                    .await
                    .map_err(|e| e.to_string())
            }
        },
    );
    if let Some(status) = metrics.status("los territorios") {
        return status;
    }
    let data = metrics.value().unwrap_or_default();
    let area = municipality.read().clone();
    let points = data
        .points
        .iter()
        .filter(|p| area.is_empty() || format!("{} · {}", p.municipio, p.entidad) == area)
        .filter(|p| map_link(p.lat, p.lon).is_some())
        .collect::<Vec<_>>();
    let current = points
        .iter()
        .find(|p| p.id == *location.read())
        .copied()
        .or_else(|| points.first().copied());
    let geocoded = points.len();
    let mobile = mobile::navigation().is_some();
    let page = mobile::destination();
    if mobile
        && matches!(
            page,
            Destination::TerritoryProspect
                | Destination::Message
                | Destination::Evidence
                | Destination::History
                | Destination::Progress
                | Destination::Notes
        )
        && let Some(p) = current
    {
        return rsx! {ProspectDetail{key:"{p.id}:{profile.id()}",id:p.id.clone(),profile}};
    }
    rsx! { div { class:if mobile && page==Destination::TerritoryMap {"territory-work ios-map-page"} else {"territory-work"},
        div { class:"territory-toolbar", hidden:mobile && page!=Destination::Root,
            label { class:"sr-only", r#for:"territory-city", "Municipio" }
            select { id:"territory-city", value:area.clone(), onchange:move |e|{municipality.set(e.value());location.set(String::new());},
                option { value:"", "Todos los municipios" }
                for t in &data.territories { option { value:format!("{} · {}",t.municipio,t.entidad), "{t.municipio} · {t.entidad}" } }
            }
            span { class:"muted small", "{number(geocoded)} ubicaciones" }
            if !mobile { PanelTabs { labels:vec!["Mapa y visitas","Comparar municipios"], active:pane, name:"Vista territorial" } }
        }
        if mobile && page==Destination::Root { mobile::PageMenu { pages:vec![Destination::TerritoryComparison] } }
        if *pane.read()==0 || mobile { div { class:"territory-visit",
            section { class:"panel territory-list", hidden:mobile && page!=Destination::Root,
                div { class:"panel-heading", h2 { if mobile {"Elegir negocio"} else {"Selecciona un negocio"} } span { class:"badge", "Top 30" } }
                div { class:"territory-businesses", for p in points.iter().take(30) { { let id=p.id.clone(); rsx! {
                    button { class:if current.is_some_and(|c|c.id==p.id){"queue-item selected"}else{"queue-item"},
                        onclick:move |_|{location.set(id.clone());if mobile {mobile::push(Destination::TerritoryMap);}},
                        div { class:"queue-title", strong { "{p.name}" } span { class:"score-chip", "{p.priority}" } }
                        p { "{p.municipio} · Fit {p.fit}" }
                        if mobile { span { class:"ios-row-disclosure", mobile::Icon { name:"map" } "Ver mapa" mobile::Icon { name:"chevron" } } }
                    }
                } } } }
                if points.is_empty() { p { class:"empty", "Sin coordenadas válidas. Prueba otro municipio." } }
            }
            section { class:"panel map-panel", hidden:mobile && page!=Destination::TerritoryMap,
                if let Some(p)=current { { let link=if mobile {format!("https://maps.apple.com/?ll={:.6},{:.6}&q=Ubicaci%C3%B3n",p.lat,p.lon)}else{map_link(p.lat,p.lon).expect("validated public coordinates")};let id=p.id.clone();let name=p.name.clone();let city=p.municipio.clone();let entity=p.entidad.clone(); rsx! {
                    div { class:"panel-heading", h2 { "{p.name}" }
                        button { class:"button secondary compact", onclick:move |_|{if mobile {mobile::push(Destination::TerritoryProspect);}else{query.set(Query{profile,view:View::All,search:name.clone(),municipio:city.clone(),entidad:entity.clone(),size:60,..Default::default()});selected.set(id.clone());screen.set(Screen::Base);}}, "Ver ficha" }
                    }
                    if !mobile || page==Destination::TerritoryMap { StreetMap { key:"{p.id}", lat:p.lat, lon:p.lon } }
                    div { class:"map-footer", button { class:"link-button small", onclick:move |_|{let mut u=ui;if let Err(e)=open_link(&link){u.toast.set(e);}}, "Abrir en Mapas ↗" } }
                } } } else { p { class:"empty", "Selecciona un negocio con coordenadas." } }
            }
        } }
        section { class:"panel territory-comparison", hidden:if mobile {page!=Destination::TerritoryComparison} else {*pane.read()!=1},
            div { class:"panel-heading", h2 { "Dónde concentrar tu jornada" } }
            if mobile { div { class:"ios-data-list", for t in &data.territories { { let city=format!("{} · {}",t.municipio,t.entidad); rsx! {
                button { class:"ios-data-row", onclick:move |_|{municipality.set(city.clone());location.set(String::new());mobile::back();},
                    strong { "{t.municipio} · {t.entidad}" }
                    span { class:"muted small", "{number(t.establishments)} negocios · {number(t.contactable)} con contacto" }
                    span { class:"small", "Fit ≥50: {number(t.high_fit)} · Clientes: {number(t.clients)}" }
                    mobile::Icon { name:"chevron" }
                }
            } } } if data.territories.is_empty() {p{class:"empty","Sin municipios guardados."}} } }
            else { div { class:"table-wrap", table { thead { tr { for h in ["Municipio","Establecimientos","Con contacto","Fit ≥50","Clientes"] { th { scope:"col", "{h}" } } } }
                tbody { for t in data.territories { { let city=format!("{} · {}",t.municipio,t.entidad); rsx! { tr { td { button { class:"table-name", onclick:move |_|{municipality.set(city.clone());location.set(String::new());let mut pane=pane;pane.set(0);}, "{t.municipio} · {t.entidad}" } } td { "{number(t.establishments)}" } td { "{number(t.contactable)}" } td { "{number(t.high_fit)}" } td { "{number(t.clients)}" } } } } } }
            } } }
        }
        p { class:"muted small territory-note", hidden:mobile && page==Destination::TerritoryMap, "Conteos de tu base. Mapa © OpenStreetMap; requiere internet." }
    } }
}

#[component]
fn OpportunitiesPanel(profile: Profile) -> Element {
    let db = consume_context::<Services>().db;
    let ui = use_context::<UiState>();
    let navigation = mobile::navigation();
    let mobile = navigation.is_some();
    let page = mobile::destination();
    let mut account_name = use_signal(String::new);
    let list_db = db.clone();
    let list = use_resource(move || {
        let _ = *ui.revision.read();
        let db = list_db.clone();
        async move { db.opportunities(profile).await.map_err(|e| e.to_string()) }
    });
    let mut draft = use_signal(|| OpportunityDraft {
        profile,
        stage: "diagnostico".into(),
        currency: "MXN".into(),
        ..Default::default()
    });
    let mut value = use_signal(|| "0.00".to_owned());
    let mut loaded_draft = use_signal(|| draft.peek().clone());
    let mut form_epoch = use_signal(|| 0u64);
    let mut received = use_signal(|| "0.00".to_owned());
    let mut search = use_signal(String::new);
    let mut saving = use_signal(|| false);
    let pane = use_signal(|| 0usize);
    let picker_db = db.clone();
    let picker = use_resource(move || {
        let search = search.read().clone();
        let db = picker_db.clone();
        async move {
            if search.trim().len() < 2 {
                return Ok(Page::default());
            }
            db.page(Query {
                search,
                profile,
                view: View::All,
                size: 12,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())
        }
    });
    let items = list
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let picks = picker
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    use_effect(move || {
        draft.write().profile = profile;
    });
    rsx! {div{class:"opportunity-grid",section{class:"panel",hidden:mobile && page!=Destination::Root,div{class:"panel-heading",h2{if mobile {"Cartera"}else{"Cartera de oportunidades"}}span{class:"badge","{items.len()}"}
    if mobile {button{class:"button primary",onclick:move |_|{draft.set(OpportunityDraft{profile,stage:"diagnostico".into(),currency:"MXN".into(),..Default::default()});value.set("0.00".into());received.set("0.00".into());account_name.set(String::new());loaded_draft.set(draft.peek().clone());form_epoch+=1;mobile::push(Destination::Opportunity);},"Nueva"}}}div{class:"opportunity-list",if let Some(status)=resource_status(list,"las oportunidades"){ {status} }else if items.is_empty(){p{class:"empty","Registra una oportunidad después de hablar con un negocio. Separa propuestas, acuerdos y dinero cobrado."}}for o in items{{let row=o.clone();rsx!{button{class:"opportunity-card",onclick:move |_|{draft.set(OpportunityDraft{id:Some(row.id),prospect_id:row.prospect_id.clone(),profile:row.profile,title:row.title.clone(),stage:row.stage.clone(),amount_cents:row.amount_cents,received_cents:row.received_cents,currency:row.currency.clone(),next_action:row.next_action.clone(),due_date:row.due_date.clone(),notes:row.notes.clone(),..Default::default()});value.set(amount(row.amount_cents));received.set(amount(row.received_cents));account_name.set(row.account_name.clone());loaded_draft.set(draft.peek().clone());form_epoch+=1;if mobile {mobile::push(Destination::Opportunity);}},div{strong{"{o.title}"}span{class:"badge","{o.stage}"}}p{class:"muted","{o.account_name}"}div{span{"{o.currency} ${amount(o.amount_cents)}"}span{class:"muted small","Cobrado ${amount(o.received_cents)}"}}if !o.next_action.is_empty(){p{class:"small","{o.next_action} · {o.due_date}"}}}}}}}}
        section{class:"panel ios-opportunity-editor",hidden:mobile && page!=Destination::Opportunity,div{class:"panel-heading",hidden:mobile,h2{if draft.read().id.is_some(){"Editar oportunidad"}else{"Nueva oportunidad"}}button{class:"link-button",onclick:move |_|{draft.set(OpportunityDraft{profile,stage:"diagnostico".into(),currency:"MXN".into(),..Default::default()});value.set("0.00".into());received.set("0.00".into());loaded_draft.set(draft.peek().clone());form_epoch+=1;},"Nueva"}}PanelTabs{labels:vec!["Acuerdo","Seguimiento","Contacto"],active:pane,name:"Datos de la oportunidad"}div{class:"form-body",div{hidden:*pane.read()!=0,label{if mobile {"Negocio"}else{"Buscar establecimiento"} TextEntry{key:"picker:{draft.read().prospect_id}",initial:String::new(),placeholder:"Nombre o municipio",oninput:move|e:FormEvent|search.set(e.value())}}if !picks.items.is_empty(){div{class:"picker-results",for p in picks.items{{let pid=p.id.clone();let name=p.nombre.clone();rsx!{button{class:"picker-row",onclick:move |_|{draft.write().prospect_id=pid.clone();account_name.set(name.clone());search.set(String::new());},"{p.nombre} · {p.municipio}"}}}}}}if let Some(Err(e))=picker.read().as_ref(){p{class:"error",role:"alert","No se pudo buscar: {e}"}}if mobile {p{class:"muted small",if account_name.read().is_empty(){"Elige un negocio para esta oportunidad"}else{"{account_name}"}}}else{p{class:"muted small","ID DENUE seleccionado: {draft.read().prospect_id}"}}
            label{"Título" TextEntry{key:"title:{form_epoch}",initial:loaded_draft.read().title.clone(),placeholder:"Ej. Sitio web y formulario de consultas",oninput:move|e:FormEvent|draft.write().title=e.value()}}
            div{class:"form-row",label{"Etapa" select{value:draft.read().stage.clone(),onchange:move|e|draft.write().stage=e.value(),for stage in ["diagnostico","propuesta","negociacion","ganado","perdido"]{option{value:stage,"{stage}"}}}}label{"Moneda" select{value:draft.read().currency.clone(),onchange:move|e|draft.write().currency=e.value(),option{value:"MXN","MXN"}option{value:"USD","USD"}}}}
            div{class:"form-row",label{if mobile {"Importe"}else{"Importe de la oportunidad"} TextEntry{key:"amount:{form_epoch}",inputmode:"decimal",initial:amount(loaded_draft.read().amount_cents),oninput:move|e:FormEvent|value.set(e.value())}}label{if mobile {"Cobrado"}else{"Cobrado acumulado"} TextEntry{key:"received:{form_epoch}",inputmode:"decimal",initial:amount(loaded_draft.read().received_cents),oninput:move|e:FormEvent|received.set(e.value())}}}
            }div{hidden:*pane.read()!=1,label{"Próxima acción" TextEntry{key:"next:{form_epoch}",initial:loaded_draft.read().next_action.clone(),placeholder:"Enviar propuesta, confirmar reunión…",oninput:move|e:FormEvent|draft.write().next_action=e.value()}}label{"Fecha" TextEntry{key:"date:{form_epoch}",kind:"date",initial:loaded_draft.read().due_date.clone(),oninput:move|e:FormEvent|draft.write().due_date=e.value()}}
            label{"Notas" TextEntry{key:"notes:{form_epoch}",multiline:true,initial:loaded_draft.read().notes.clone(),oninput:move|e:FormEvent|draft.write().notes=e.value()}}
            }            div{hidden:*pane.read()!=2,h3{"Persona de contacto"}label{"Nombre" TextEntry{key:"contact_name:{form_epoch}",initial:loaded_draft.read().contact_name.clone(),oninput:move|e:FormEvent|draft.write().contact_name=e.value()}}label{"Cargo o función" TextEntry{key:"contact_role:{form_epoch}",initial:loaded_draft.read().contact_role.clone(),oninput:move|e:FormEvent|draft.write().contact_role=e.value()}}label{"Correo confirmado" TextEntry{key:"contact_email:{form_epoch}",kind:"email",initial:loaded_draft.read().contact_email.clone(),oninput:move|e:FormEvent|draft.write().contact_email=e.value()}}}

            button{class:"button primary full",disabled:*saving.read(),onclick:move |_|{let db=db.clone();let mut d=draft.read().clone();let mut u=ui;match (prospect_core::analytics::money_cents(&value.read()),prospect_core::analytics::money_cents(&received.read())){(Ok(v),Ok(r))=>{d.amount_cents=v;d.received_cents=r;saving.set(true);spawn(async move{match db.save_opportunity(d).await{Ok(id)=>{draft.write().id=Some(id);u.revision+=1;u.toast.set("Oportunidad guardada".into());if let Some(nav)=navigation {nav.pop();}},Err(e)=>u.toast.set(e.to_string())}saving.set(false);});},_=>u.toast.set("Revisa los importes; usa hasta dos decimales".into())}},"Guardar oportunidad"}
            p{class:"muted small","Marca ganado sólo cuando exista acuerdo. El importe de una propuesta no es ingreso cobrado."}
        }}}
        if let Some(Err(e))=list.read().as_ref(){p{class:"error","{e}"}}
    }
}
#[component]
fn IndicatorsPanel() -> Element {
    let db = consume_context::<Services>().db;
    let ui = use_context::<UiState>();
    let mobile = mobile::navigation().is_some();
    let page = mobile::destination();
    let read_db = db.clone();
    let indicators = use_resource(move || {
        let _ = *ui.revision.read();
        let db = read_db.clone();
        async move { db.economics().await.map_err(|e| e.to_string()) }
    });
    let mut indicator = use_signal(|| "1002000001".to_owned());
    if let Some(status) = resource_status(indicators, "los indicadores") {
        return status;
    }
    let points = indicators
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let running = ui
        .job
        .read()
        .as_ref()
        .is_some_and(|j| j.snapshot().status == "running");
    rsx! {section{class:"panel",div{class:"panel-heading",hidden:mobile && page!=Destination::Root,h2{"Contexto económico"}button{class:"button primary",disabled:running,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Banxico,ui,s.db,String::new());},"Actualizar Banxico"}}if mobile && page==Destination::Root {mobile::PageMenu{pages:vec![Destination::IndicatorQuery]}}
        p{class:"muted panel-note",hidden:mobile && page!=Destination::Root,"Tipo de cambio y tasa objetivo para contexto y cotizaciones. No identifican intención de compra de un negocio."}
        div{class:"indicator-grid",hidden:mobile && page!=Destination::Root,if points.is_empty(){div{class:"empty",h3{"Sin indicadores guardados"}p{"Actualiza una fuente para conservar su última observación y consultarla sin conexión."}}}for p in points{div{class:"indicator-card",p{class:"eyebrow","{p.source} · {p.series_id}"}strong{class:"indicator-value","{indicator_value(&p.value)}"}p{if p.unit.chars().all(|c|c.is_ascii_digit()){ "Unidad descriptiva pendiente" }else{"{p.unit}"}}p{class:"small","{p.description}"}p{class:"muted small","Periodo: {p.period} · Consulta: {p.updated_at}"}}}}
        div{class:"form-body narrow",hidden:mobile && page!=Destination::IndicatorQuery,h3{"Consultar indicador INEGI"}label{"ID del indicador · BISE o BIE:ID" TextEntry{initial:"1002000001",oninput:move|e:FormEvent|indicator.set(e.value())}}button{class:"button secondary",disabled:running,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Inegi,ui,s.db,indicator.read().clone());},"Consultar y guardar"}p{class:"muted small","Se usa el token compartido configurado para INEGI y DENUE. Cada indicador conserva el periodo y la unidad de la fuente."}}
        if let Some(Err(e))=indicators.read().as_ref(){p{class:"error panel-note","{e}"}}
    }}
}
fn navigate(target: Screen, mut ui: UiState) {
    mobile::reset();
    let effective = if !ui.mobile_layout && target == Screen::Updates {
        ui.settings_tab.set(1);
        Screen::Settings
    } else {
        target
    };
    ui.screen.set(effective);
    ui.query.write().view = effective.view();
    ui.query.write().page = 0;
    ui.query.write().contactable = effective == Screen::Today;
    ui.selected.set(String::new());
}
#[derive(Clone, PartialEq)]
struct DashboardData {
    summary: prospect_core::Summary,
    metrics: prospect_core::analytics::Metrics,
    next: Page,
    progress: Vec<prospect_core::updates::ScopeProgress>,
}
#[component]
fn DashboardPanel(profile: Profile) -> Element {
    let ui = use_context::<UiState>();
    let db = consume_context::<Services>().db;
    let data = use_cached_read(
        move || {
            format!(
                "{}|dashboard:{}:{}",
                ui.revision.read(),
                profile.id(),
                serde_json::to_string(&*ui.settings.read()).expect("settings key")
            )
        },
        move || {
            let _ = *ui.revision.read();
            let settings = ui.settings.read().clone();
            let db = db.clone();
            async move {
                let summary = db.summary(profile).await.map_err(|e| e.to_string())?;
                let metrics = db
                    .metrics(profile, "giro".into())
                    .await
                    .map_err(|e| e.to_string())?;
                let next = db
                    .page(Query {
                        profile,
                        view: View::Today,
                        contactable: true,
                        size: 3,
                        ..Default::default()
                    })
                    .await
                    .map_err(|e| e.to_string())?;
                let progress = db
                    .denue_progress(settings, profile)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok::<_, String>(DashboardData {
                    summary,
                    metrics,
                    next,
                    progress,
                })
            }
        },
    );
    if let Some(status) = data.status("tu dashboard") {
        return status;
    }
    let d = data.value().expect("resolved dashboard");
    let f = d.metrics.funnel;
    let mobile = mobile::navigation().is_some();
    let page = mobile::destination();
    let settings = ui.settings.read().clone();
    let complete = d.progress.iter().filter(|p| p.completed).count();
    let partial = d
        .progress
        .iter()
        .filter(|p| p.started && !p.completed)
        .count();
    rsx! {div{class:"dashboard-work",
        if mobile && page==Destination::Root {mobile::PageMenu {pages:vec![Destination::Day,Destination::Results,Destination::DataStatus]}}
        div{class:"stats-grid",hidden:mobile && page!=Destination::Root,StatCard{label:"Establecimientos",value:number(d.summary.total),note:"Guardados en tu base"}StatCard{label:"Seguimientos",value:number(d.summary.due),note:"Pendientes hoy · CRM común"}StatCard{label:"Conversaciones",value:number(d.summary.conversations),note:"Abiertas · CRM común"}StatCard{label:"Oportunidades",value:number(d.metrics.opportunity_count),note:"De la oferta seleccionada"}}
        div{class:"dashboard-grid",
            section{class:"panel dashboard-day",hidden:mobile && page!=Destination::Day,div{class:"panel-heading",h2{"Una jornada pequeña, concreta"}button{class:"button primary compact",onclick:move |_|{let mut ui=ui;ui.query.set(Query{profile,view:View::Today,contactable:true,size:60,..Default::default()});ui.selected.set(String::new());ui.screen.set(Screen::Today);},"Empezar en Hoy"}}
                div{class:"dashboard-goals",DailyGoal{label:"Revisados",done:d.summary.researched_today,goal:settings.daily_research}DailyGoal{label:"Contactos",done:d.summary.contacted_today,goal:settings.daily_contacts}DailyGoal{label:"Seguimientos",done:d.summary.followed_today,goal:settings.daily_followups}}
                p{class:"muted small dashboard-note","Empieza con un negocio. Revisa su contexto, adapta el mensaje y decide el siguiente paso. La app registra sólo lo que realmente haces."}
                div{class:"dashboard-candidates",for p in d.next.items{{let row=p.clone();rsx!{button{class:"queue-item",onclick:move |_|{let mut ui=ui;ui.query.set(Query{profile,view:View::Today,contactable:true,size:60,search:row.nombre.clone(),entidad:row.entidad.clone(),municipio:row.municipio.clone(),..Default::default()});ui.selected.set(row.id.clone());ui.screen.set(Screen::Today);},div{class:"queue-title",strong{"{p.nombre}"}span{class:"score-chip","{p.priority_score}"}}p{"{p.municipio}, {p.entidad} · {p.giro}"}}}}}if d.next.total==0{p{class:"empty",if d.summary.total==0{"Tu base está vacía. Configura tus fuentes y elige estados antes de importar."}else{"No hay prospectos pendientes con contacto. Puedes revisar tu base y tus conversaciones."}}}}
                div{class:"dashboard-next-actions",button{class:"button secondary",onclick:move |_|navigate(Screen::Followups,ui),"Seguimientos · {d.summary.due}"}button{class:"button secondary",onclick:move |_|navigate(Screen::Conversations,ui),"Conversaciones · {d.summary.conversations}"}}
            }
            div{class:"dashboard-right",
                section{class:"panel",hidden:mobile && page!=Destination::Results,div{class:"panel-heading",h2{"Resultados de esta oferta"}}div{class:"dashboard-results",div{span{class:"muted small","Contactados"}strong{"{number(f.contactados)}"}}div{span{class:"muted small","Respondieron"}strong{"{number(f.respondieron)}"}}div{span{class:"muted small","Clientes"}strong{"{number(f.clientes)}"}}}div{class:"dashboard-money",p{span{class:"muted small","Cobrado MXN"}strong{"${amount(d.metrics.received_mxn_cents)}"}}p{span{class:"muted small","Cobrado USD"}strong{"${amount(d.metrics.received_usd_cents)}"}}}div{class:"dashboard-card-footer",span{class:"muted small",if f.contactados<20{"Muestra pequeña; acumula conversaciones antes de concluir."}else{"Resultados registrados; las propuestas no son dinero cobrado."}}button{class:"link-button small",onclick:move |_|navigate(Screen::Metrics,ui),"Ver métricas →"}}}
                section{class:"panel",hidden:mobile && page!=Destination::DataStatus,div{class:"panel-heading",h2{"Datos bajo tu control"}}div{class:"form-body",p{strong{"Actualización manual"}span{class:"muted small"," · No se consultan fuentes al abrir el dashboard."}}p{class:"small","DENUE: {complete}/{d.progress.len()} consultados · {partial} pendientes."}p{class:"muted small","Consultar no acredita una publicación nueva de la fuente."}button{class:"button secondary",onclick:move |_|navigate(Screen::Updates,ui),"Gestionar actualizaciones"}}}
            }
        }
    }}
}
#[derive(Clone, PartialEq)]
struct UpdateData {
    progress: Vec<prospect_core::updates::ScopeProgress>,
    status: Vec<prospect_core::updates::SourceStatus>,
    denue_configured: bool,
    counts: std::collections::BTreeMap<String, usize>,
}
#[component]
fn UpdatesPanel(profile: Profile) -> Element {
    let ui = use_context::<UiState>();
    let onboarding = try_use_context::<crate::onboarding::OnboardingControl>();
    let mobile = mobile::navigation().is_some();
    let page = mobile::destination();
    let db = consume_context::<Services>().db;
    let data = use_resource(move || {
        let _ = *ui.revision.read();
        if let Some(o) = onboarding {
            let _ = *o.revision.read();
        }
        let settings = ui.settings.read().clone();
        let db = db.clone();
        async move {
            let progress = db
                .denue_progress(settings, profile)
                .await
                .map_err(|e| e.to_string())?;
            let status = db.source_statuses().await.map_err(|e| e.to_string())?;
            let counts = db.entity_counts().await.map_err(|e| e.to_string())?;
            let denue_configured = !db
                .paths
                .secrets()
                .map_err(|e| e.to_string())?
                .denue
                .is_empty();
            Ok::<_, String>(UpdateData {
                progress,
                status,
                denue_configured,
                counts,
            })
        }
    });
    if let Some(status) = resource_status(data, "el estado de tus fuentes") {
        return status;
    }
    let d = data
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .expect("resolved sources");
    let settings = ui.settings.read().clone();
    let ready = d.progress.iter().filter(|p| !p.started).count();
    let pending = d
        .progress
        .iter()
        .filter(|p| p.started && !p.completed)
        .count();
    let complete = d.progress.iter().filter(|p| p.completed).count();
    let running = ui
        .job
        .read()
        .as_ref()
        .is_some_and(|j| j.snapshot().status == "running");
    let mut by_entity =
        std::collections::BTreeMap::<String, (usize, usize, usize, String, String)>::new();
    for p in d.progress {
        let entry = by_entity.entry(p.entity).or_default();
        entry.0 += usize::from(!p.started);
        entry.1 += usize::from(p.started && !p.completed);
        entry.2 += usize::from(p.completed);
        if !p.last_success.is_empty() && (entry.3.is_empty() || p.last_success < entry.3) {
            entry.3 = p.last_success;
        }
        if !p.last_error.is_empty() {
            entry.4 = p.last_error;
        }
    }
    rsx! {div{class:"updates-work",
        if mobile && page==Destination::Root {mobile::PageMenu{pages:vec![Destination::Denue,Destination::Coverage,Destination::OtherSources]}}
        div{class:"stats-grid",hidden:mobile && page!=Destination::Root,StatCard{label:"Sin consultar",value:number(ready),note:"Segmentos de tu selección"}StatCard{label:"Por reanudar",value:number(pending),note:"Lotes incompletos"}StatCard{label:"Completos",value:number(complete),note:"Consultas registradas"}StatCard{label:"Límite por clic",value:format!("{} {}",settings.denue_pages,if settings.denue_pages==1{"página"}else{"páginas"}),note:"Hasta 200 registros por página"}}
        section{class:"panel",hidden:mobile && page!=Destination::Denue,div{class:"panel-heading",h2{if profile==Profile::Automatizacion{"DENUE · Automatización y sistemas"}else{"DENUE · Web y marketing"}}button{class:"link-button small",onclick:move |_|navigate(Screen::Settings,ui),"Cambiar estados y límites →"}}div{class:"form-body updates-controls",p{class:"muted small","Cada botón ejecuta un lote y se detiene. Sin programación automática; hasta tres intentos por página si falla la conexión. Los segmentos completos se omiten al importar nuevos."}
            if !d.denue_configured{div{class:"notice",strong{"Configura tu token DENUE"}p{"Ajustes → Fuentes. Usa tus propias credenciales; no se incluyen en la aplicación."}button{class:"link-button",onclick:move |_|{let mut ui=ui;if let Err(e)=open_link(crate::onboarding::DENUE_GUIDE.url){ui.toast.set(e);}},"Cómo obtener tu token DENUE en INEGI ↗"}button{class:"link-button",onclick:move |_|navigate(Screen::Settings,ui),"Abrir Ajustes"}}}
            div{class:"action-row",button{class:"button primary",disabled:running||!d.denue_configured||ready==0||pending>0,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Import,ui,s.db,String::new());},"Importar nuevos"}button{class:"button secondary",disabled:running||!d.denue_configured||pending==0,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Resume,ui,s.db,String::new());},"Reanudar pendientes"}button{class:"button secondary",disabled:running,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Analyze,ui,s.db,String::new());},"Recalcular reglas · local"}}
            details{summary{"Revisar datos ya consultados"}p{class:"muted small","Inicia una revisión de los estados, sectores y tamaños seleccionados desde la primera página. Se detendrá al alcanzar el límite; después usa Reanudar. Conserva notas, contactos e historial."}button{class:"button secondary",disabled:running||!d.denue_configured||pending>0||complete==0,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Recheck,ui,s.db,String::new());},"Iniciar revisión"}}
        }}
        div{class:"updates-grid",section{class:"panel updates-table",hidden:mobile && page!=Destination::Coverage,div{class:"panel-heading",h2{"Cobertura y consultas por estado"}}p{class:"coverage-help muted small","Los establecimientos de la base son compartidos; los segmentos y consultas corresponden a esta oferta. Importa o reanuda un estado por clic, con el mismo límite de páginas."}if mobile {div{class:"ios-data-list",for (entity,(new,pending,done,date,error)) in &by_entity {article{class:"ios-data-row",strong{"{prospect_core::config::entity_name(entity)}"}span{class:"small","{d.counts.get(prospect_core::config::entity_name(entity)).copied().unwrap_or(0)} establecimientos en tu base"}workspace::StateImportAction{entity:entity.clone(),pending:*pending,new:*new,configured:d.denue_configured,running}span{class:"small","{new} nuevos · {pending} pendientes · {done} completos"}span{class:"muted small",{prospect_core::updates::age_label(date,settings.denue_max_age_days)}}if !error.is_empty(){p{class:"error small","{error}"}}}}}}
            else {div{class:"table-wrap",table{thead{tr{for h in ["Estado","En tu base","Sin consultar","Incompletos","Completos","Consulta","Acción"]{th{scope:"col","{h}"}}}}tbody{for (entity,(new,pending,done,date,error))in by_entity{tr{td{strong{"{prospect_core::config::entity_name(&entity)}"}}td{"{number(d.counts.get(prospect_core::config::entity_name(&entity)).copied().unwrap_or(0))}"}td{"{new}"}td{"{pending}"}td{"{done}"}td{class:"small",span{"{prospect_core::updates::age_label(&date,settings.denue_max_age_days)}"}if !date.is_empty(){p{class:"muted small","Más antigua registrada: {date}"}}if !error.is_empty(){p{class:"error small","{error}"}}}td{workspace::StateImportAction{entity:entity.clone(),pending,new,configured:d.denue_configured,running}}}}}}}}}
            if !mobile {workspace::BatchHistory{key:"{profile.id()}",profile}}
            section{class:"panel",hidden:mobile && page!=Destination::OtherSources,div{class:"panel-heading",h2{"Otras fuentes"}}div{class:"form-body",p{class:"small","Sitios: sólo dominios sin revisar o con más de {settings.web_max_age_days} días. Máximo 100 por clic."}button{class:"button secondary",disabled:running,onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Enrich,ui,s.db,String::new());},"Revisar sitios"}h3{"Banxico e INEGI"}p{class:"muted small","Se consultan desde Indicadores mediante botones. Se conserva el periodo publicado y la última consulta exitosa."}for status in d.status.into_iter().filter(|s|s.source!="denue"){div{class:"source-status",strong{class:"small","{status.scope}"}p{class:"muted small",{prospect_core::updates::age_label(&status.last_success,if status.source=="banxico"{settings.banxico_max_age_days}else{settings.inegi_max_age_days})}}if !status.last_attempt.is_empty(){p{class:"muted small","Intento: {status.last_attempt}"}}if !status.last_error.is_empty(){p{class:"error small","{status.last_error}"}}}}button{class:"link-button",onclick:move |_|navigate(Screen::Indicators,ui),"Abrir Indicadores →"}}
            }
        }
        p{class:"muted small","Antigüedad = tiempo desde una consulta, no garantía de vigencia. Cambiar estados o días no inicia ninguna descarga."}
    }}
}

use settings::SettingsPanel;

#[cfg(test)]
mod review_tests {
    use super::{StartupFailure, StartupProblem, indicator_value, map_link};
    use dioxus::prelude::*;
    #[component]
    fn ReadingFailureProbe() -> Element {
        let reading = use_resource(|| async {
            tokio::task::yield_now().await;
            Err::<(), String>("Lectura local no disponible".into())
        });
        super::resource_status(reading, "la base").unwrap_or_else(|| rsx! {p{"Lectura correcta"}})
    }
    #[tokio::test]
    async fn failed_reading_has_loading_feedback_then_error_and_retry() {
        let mut dom = VirtualDom::new(ReadingFailureProbe);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Cargando la base"));
        assert!(html.contains("loading-spinner"));
        let html = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                dom.wait_for_work().await;
                dom.render_immediate_to_vec();
                let html = dioxus_ssr::render(&dom);
                if html.contains("Error al cargar la base") {
                    break html;
                }
            }
        })
        .await
        .expect("reading should resolve");
        assert!(html.contains("Lectura local no disponible"));
        assert!(html.contains("Volver a intentar"));
        assert!(!html.contains("Sin resultados"));
    }
    #[test]
    fn startup_failure_renders_without_database_context() {
        let mut dom = VirtualDom::new(StartupProblem).with_root_context(StartupFailure {
            message: "Configuración local inválida".into(),
            directory: "/tmp/prospect-review-data".into(),
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("No se pudo iniciar la app"));
        assert!(html.contains("Configuración local inválida"));
        assert!(html.contains("Abrir carpeta de datos"));
        assert!(html.contains("/tmp/prospect-review-data"));
    }
    #[test]
    fn rejects_invalid_coordinates_before_loading_external_map() {
        for (lat, lon) in [
            (f64::NAN, -98.2),
            (19.0, f64::INFINITY),
            (91.0, -98.2),
            (19.0, -181.0),
        ] {
            assert!(map_link(lat, lon).is_none());
        }
        let external = map_link(19.0433, -98.2019).unwrap();
        assert!(external.starts_with("https://www.openstreetmap.org/?"));
    }
    #[test]
    fn formats_official_numeric_strings_without_float_rounding() {
        for (input, expected) in [
            ("130911314.00000000000000000000", "130,911,314"),
            ("18.4163000", "18.4163"),
            ("6.5000", "6.5"),
            ("-12345.0100", "-12,345.01"),
            ("0.000001", "0.000001"),
            ("N/E", "N/E"),
            ("1e9", "1e9"),
        ] {
            assert_eq!(indicator_value(input), expected);
        }
    }

    fn make_test_ui_state(services: &crate::Services, settings_tab: usize) -> super::UiState {
        super::UiState {
            screen: Signal::new(super::Screen::Settings),
            settings_tab: Signal::new(settings_tab),
            query: Signal::new(super::Query::default()),
            selected: Signal::new(String::new()),
            revision: Signal::new(0),
            settings: Signal::new(services.settings.clone()),
            job: Signal::new(None),
            job_minimized: Signal::new(false),
            dismissed_job_id: Signal::new(String::new()),
            heavy_task: Signal::new(None),
            toast: Signal::new(String::new()),
            notifications: Signal::new(Vec::new()),
            notif_seq: Signal::new(0),
            busy: Signal::new(false),
            pending: Signal::new(None),
            mobile_layout: services.layout == crate::Layout::Ios,
        }
    }

    #[component]
    fn OverlayProbe() -> Element {
        let services = consume_context::<crate::Services>();
        use_context_provider(|| make_test_ui_state(&services, 0));
        let state = prospect_core::jobs::JobState {
            id: "job-1".into(),
            kind: "DENUE · actualizar todo".into(),
            status: "running".into(),
            message: "Descargando Ciudad de México…".into(),
            pages: 2,
            total: 4,
            requests: 3,
            ..Default::default()
        };
        rsx! { super::JobLoadingOverlay { state, running: true } }
    }

    #[component]
    fn NotificationProbe() -> Element {
        let services = consume_context::<crate::Services>();
        let mut ui = make_test_ui_state(&services, 0);
        ui.job_minimized.set(true);
        ui.notifications.set(vec![super::FloatingNotification {
            id: 1,
            message: "Preferencias guardadas.".into(),
            is_error: false,
        }]);
        use_context_provider(|| ui);
        let state = prospect_core::jobs::JobState {
            id: "job-2".into(),
            kind: "Actualizar todo".into(),
            status: "running".into(),
            message: "Analizando 1 de 2…".into(),
            processed: 1,
            total: 2,
            analyzing: true,
            requests: 4,
            ..Default::default()
        };
        let job_state = Signal::new(state.clone());
        rsx! { super::NotificationCenter { state, running: true, job_state } }
    }

    #[test]
    fn job_loading_overlay_and_floating_notifications_render_animated_progress() {
        let temp = tempfile::tempdir().unwrap();
        let paths = prospect_core::AppPaths::at(temp.path()).unwrap();
        let settings = prospect_core::Settings::load_desktop(&paths).unwrap_or_default();
        let db = prospect_core::Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(OverlayProbe).with_root_context(crate::Services {
            db: db.clone(),
            settings: settings.clone(),
            layout: crate::Layout::Desktop,
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("loading-backdrop"));
        assert!(html.contains("loading-spinner"));
        assert!(html.contains("animateTransform"));
        assert!(html.contains("progress-track"));
        assert!(html.contains("50%"));
        assert!(html.contains("Descargando Ciudad de México…"));
        assert!(html.contains("Consultas enviadas: 3"));
        assert!(html.contains("Seguir en segundo plano"));
        assert!(html.contains("Cancelar"));

        let mut notif_dom = VirtualDom::new(NotificationProbe).with_root_context(crate::Services {
            db,
            settings,
            layout: crate::Layout::Desktop,
        });
        notif_dom.rebuild_in_place();
        let notif_html = dioxus_ssr::render(&notif_dom);
        assert!(notif_html.contains("toast-stack"));
        assert!(notif_html.contains("toast-job"));
        assert!(notif_html.contains("progress-track"));
        assert!(notif_html.contains("50%"));
        assert!(notif_html.contains("Preferencias guardadas."));
        assert!(notif_html.contains("Ampliar"));
    }

    #[derive(Clone, Copy, PartialEq, Props)]
    struct SettingsProbeProps {
        tab: usize,
    }

    #[component]
    fn SettingsProbe(props: SettingsProbeProps) -> Element {
        let services = consume_context::<crate::Services>();
        use_context_provider(|| Signal::new(super::ViewCache::default()));
        use_context_provider(|| make_test_ui_state(&services, props.tab));
        rsx! { super::SettingsPanel {} }
    }

    #[tokio::test]
    async fn settings_panel_renders_renamed_tabs_template_editor_and_auto_update() {
        let temp = tempfile::tempdir().unwrap();
        let paths = prospect_core::AppPaths::at(temp.path()).unwrap();
        crate::onboarding::save_tokens(&paths, "tok-denue-test", "", "", true).unwrap();
        let settings = prospect_core::Settings::load_desktop(&paths).unwrap();
        assert!(settings.entities.iter().any(|e| e == "09"));
        let db = prospect_core::Db::open(paths).unwrap();
        assert!(temp.path().join("templates.toml").is_file());

        // Tab 0: Perfil + un solo lienzo de plantillas con modo edición / vista previa
        let mut dom0 = VirtualDom::new_with_props(SettingsProbe, SettingsProbeProps { tab: 0 })
            .with_root_context(crate::Services {
                db: db.clone(),
                settings: settings.clone(),
                layout: crate::Layout::Desktop,
            });
        dom0.rebuild_in_place();
        let html0 = dioxus_ssr::render(&dom0);
        assert!(html0.contains("Perfil y preferencias"));
        assert!(html0.contains("Base de datos y alcance"));
        assert!(html0.contains("Tokens y fuentes"));
        assert!(html0.contains(">Perfil<"));
        assert!(!html0.contains("Tu firma y metas"));
        assert!(html0.contains("Tus datos en plantillas"));
        assert!(html0.contains("Tu credencial"));
        assert!(html0.contains("Metas diarias"));
        assert!(html0.contains(">Editar<"));
        assert!(html0.contains(">Vista previa<"));
        for category in [
            "WhatsApp",
            "Asuntos de correo",
            "Correos",
            "Seguimientos",
            "Giros",
        ] {
            assert!(html0.contains(category), "{category}");
        }
        assert!(html0.contains("Mensaje 10"));
        assert!(html0.contains("st-canvas-text"));
        assert_eq!(
            html0.matches("<textarea").count(),
            2,
            "one canvas plus the credential"
        );
        assert!(html0.contains("Nuevo dato"));
        assert!(html0.contains("[Tu nombre]"));
        assert!(html0.contains("Guardar cambios"));
        assert!(!html0.contains("Cambios sin guardar"));
        assert!(!html0.contains("Bloque 1"));
        assert!(!html0.contains("Restaurar predeterminadas"));
        assert!(!html0.contains(">TOML<"));
        assert!(!html0.contains("Archivo local:"));
        assert!(!html0.contains("{nombre}"));
        assert!(!html0.contains("Audi México"));

        // Tab 1: Base de datos y alcance + Auto-update + Unified updates
        let mut dom1 = VirtualDom::new_with_props(SettingsProbe, SettingsProbeProps { tab: 1 })
            .with_root_context(crate::Services {
                db: db.clone(),
                settings: settings.clone(),
                layout: crate::Layout::Desktop,
            });
        dom1.rebuild_in_place();
        let initial_html1 = dioxus_ssr::render(&dom1);
        assert!(initial_html1.contains(">Alcance<"));
        assert!(initial_html1.contains("1 estado elegido"));
        assert_eq!(initial_html1.matches("st-state active").count(), 1);
        assert_eq!(initial_html1.matches("st-state-name").count(), 32);
        assert!(initial_html1.contains("Ciudad de México"));
        assert!(initial_html1.contains("Sugeridos"));
        assert!(initial_html1.contains("Tamaño del negocio"));
        assert!(initial_html1.contains("Pequeña"));
        assert!(initial_html1.contains("Actualización automática"));
        assert!(initial_html1.contains("Sólo manual"));
        assert!(!initial_html1.contains("Días para sugerir revisión"));
        assert!(initial_html1.contains("loading-spinner"));
        assert!(!initial_html1.contains("Páginas DENUE por clic"));
        let html1 = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                dom1.render_immediate_to_vec();
                let html = dioxus_ssr::render(&dom1);
                if html.contains("Por explorar") {
                    return html;
                }
                dom1.wait_for_work().await;
            }
        })
        .await
        .expect("tab 1 updates should resolve");
        assert!(html1.contains("Actualizar todo"));
        assert!(!html1.contains("Registros por actualización"));
        assert!(!html1.contains("Volver a consultar estados ya revisados"));
        assert!(html1.contains(">Historial<"));
        assert!(html1.contains("Crear respaldo"));
        assert!(html1.contains("Por explorar"));
        assert!(!html1.contains("Importar nuevos"));
        assert!(!html1.contains("Reanudar pendientes"));
        assert!(!html1.contains("Revisar sitios"));

        // Tab 2: Tokens y fuentes
        let mut dom2 = VirtualDom::new_with_props(SettingsProbe, SettingsProbeProps { tab: 2 })
            .with_root_context(crate::Services {
                db,
                settings,
                layout: crate::Layout::Desktop,
            });
        dom2.rebuild_in_place();
        let html2 = dioxus_ssr::render(&dom2);
        assert!(html2.contains(">Tokens<"));
        assert!(!html2.contains("Tokens locales"));
        assert!(!html2.contains("0600"));
        assert!(html2.contains("st-status ok"));
        assert!(html2.contains("Mostrar token"));
        assert!(html2.contains("Verificar y guardar"));
        assert!(html2.contains(">Respaldo<"));
        assert!(html2.contains("Crear respaldo"));
        assert!(!html2.contains(&temp.path().display().to_string()));
    }
}
