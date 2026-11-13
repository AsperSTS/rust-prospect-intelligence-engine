use crate::{Layout, Services, ui};
use dioxus::prelude::*;
use prospect_core::{
    AppPaths,
    archive::Restored,
    config::token_shape_ok,
    sources::{GovernmentClient, TokenCheck},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct TokenSourceGuide {
    pub url: &'static str,
    pub link_label: &'static str,
    pub steps: &'static str,
}

pub(crate) const DENUE_GUIDE: TokenSourceGuide = TokenSourceGuide {
    url: "https://www.inegi.org.mx/servicios/api_denue.html",
    link_label: "Obtener en INEGI",
    steps: "Descarga el padrón de negocios de INEGI para prospectar. Registra tu correo en su portal para recibir tu token gratis.",
};

pub(crate) const INEGI_INDICATORS_GUIDE: TokenSourceGuide = TokenSourceGuide {
    url: "https://www.inegi.org.mx/servicios/api_indicadores.html",
    link_label: "Portal INEGI Indicadores",
    steps: "Consulta indicadores económicos (BIE/BISE). Registra tu correo en INEGI; si lo omites, reutiliza tu token DENUE.",
};

pub(crate) const BANXICO_GUIDE: TokenSourceGuide = TokenSourceGuide {
    url: "https://www.banxico.org.mx/SieAPIRest/service/v1/token",
    link_label: "Generar en Banxico SIE",
    steps: "Consulta el tipo de cambio oficial USD/MXN en oportunidades. Resuelve el captcha en Banxico y genera tu clave.",
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum TokenFieldKind {
    Directory,
    Indicators,
    Currency,
}

#[component]
fn TokenFieldIcon(kind: TokenFieldKind) -> Element {
    match kind {
        TokenFieldKind::Directory => rsx! {
            svg {
                class: "onboarding-field-svg",
                view_box: "0 0 16 16",
                width: "16",
                height: "16",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                "aria-hidden": "true",
                rect { x: "2.5", y: "2", width: "11", height: "12", rx: "1.5" }
                path { d: "M5.5 5.5h5M5.5 8.5h5M5.5 11.5h3" }
            }
        },
        TokenFieldKind::Indicators => rsx! {
            svg {
                class: "onboarding-field-svg",
                view_box: "0 0 16 16",
                width: "16",
                height: "16",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                "aria-hidden": "true",
                path { d: "M2.5 13.5h11M4.5 11V8M8 11V5M11.5 11V3.5" }
            }
        },
        TokenFieldKind::Currency => rsx! {
            svg {
                class: "onboarding-field-svg",
                view_box: "0 0 16 16",
                width: "16",
                height: "16",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.5",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                "aria-hidden": "true",
                circle { cx: "8", cy: "8", r: "5.5" }
                path { d: "M8 4.5v7M9.8 6.1a1.7 1.7 0 0 0-1.8-.9c-1 0-1.6.5-1.6 1.2 0 .8.6 1.1 1.6 1.4 1.1.3 1.8.6 1.8 1.5 0 .8-.7 1.4-1.8 1.4a2 2 0 0 1-1.9-.9" }
            }
        },
    }
}

#[component]
fn RequirementBadgeIcon(required: bool) -> Element {
    if required {
        rsx! {
            svg {
                view_box: "0 0 16 16",
                width: "12",
                height: "12",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.8",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                "aria-hidden": "true",
                circle { cx: "8", cy: "8", r: "6" }
                path { d: "M5.6 8.2l1.6 1.6 3.2-3.4" }
            }
        }
    } else {
        rsx! {
            svg {
                view_box: "0 0 16 16",
                width: "12",
                height: "12",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.6",
                stroke_linecap: "round",
                stroke_linejoin: "round",
                "aria-hidden": "true",
                circle { cx: "8", cy: "8", r: "5.5" }
                path { d: "M5.5 8h5" }
            }
        }
    }
}

#[component]
fn ExternalLinkIcon() -> Element {
    rsx! {
        svg {
            class: "token-link-svg",
            view_box: "0 0 16 16",
            width: "12",
            height: "12",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.7",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M6.5 3.5H4a1.5 1.5 0 0 0-1.5 1.5v7A1.5 1.5 0 0 0 4 13.5h7a1.5 1.5 0 0 0 1.5-1.5V9.5M9.5 2.5h4v4M13.5 2.5L7.5 8.5" }
        }
    }
}

/// Shows where and how to obtain a token, with a direct link button and selectable URL.
#[component]
pub(crate) fn TokenSourceHint(
    guide: TokenSourceGuide,
    hint_class: &'static str,
    link_class: &'static str,
    url_class: &'static str,
    on_error: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: hint_class,
            p { class: "token-hint-steps", "{guide.steps}" }
            div { class: "token-source-row",
                button {
                    class: link_class,
                    r#type:"button",
                    onclick:move |_| {
                        if let Err(e) = ui::open_link(guide.url) {
                            on_error.call(e);
                        }
                    },
                    ExternalLinkIcon {}
                    span { "{guide.link_label}" }
                }
                code { class: url_class, "{guide.url}" }
            }
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct OnboardingControl {
    pub open: Signal<bool>,
    pub revision: Signal<u64>,
    pub first_run_pending_sync: Signal<bool>,
    /// Set once after a backup is loaded so the workspace reloads its settings.
    pub restored: Signal<Option<Restored>>,
}

pub(crate) fn save_tokens(
    paths: &AppPaths,
    denue_raw: &str,
    inegi_raw: &str,
    banxico_raw: &str,
    require_denue: bool,
) -> std::result::Result<(), String> {
    let denue_val = denue_raw.trim().to_owned();
    let inegi_val = inegi_raw.trim().to_owned();
    let banxico_val = banxico_raw.trim().to_owned();

    let mut current = paths.secrets().map_err(|e| e.to_string())?;
    if !denue_val.is_empty() {
        current.denue = denue_val;
        if inegi_val.is_empty() {
            current.inegi = current.denue.clone();
        }
    }
    if !inegi_val.is_empty() {
        current.inegi = inegi_val;
    } else if !current.denue.is_empty() && current.inegi.is_empty() {
        current.inegi = current.denue.clone();
    }
    if !banxico_val.is_empty() {
        current.banxico = banxico_val;
    }
    if require_denue && current.denue.is_empty() {
        return Err("Ingresa tu token DENUE o elige \"Configurar después\".".into());
    }
    paths.write_secrets(&current).map_err(|e| e.to_string())
}

const SHAPE_ERROR: &str =
    "Este token está incompleto o tiene caracteres de más. Cópialo de nuevo, sin espacios.";

fn check_message(source: &str, check: TokenCheck) -> String {
    match check {
        TokenCheck::Valid => String::new(),
        TokenCheck::Rejected => {
            format!(
                "{source} no reconoce este token. Revisa que sea el más reciente que recibiste."
            )
        }
        TokenCheck::Unreachable => {
            format!(
                "{source} no respondió, así que no se pudo comprobar. Revisa tu conexión y reintenta."
            )
        }
    }
}

/// Token drafts with their inline errors; shared by first run and Ajustes.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct TokenForm {
    pub denue: Signal<String>,
    pub inegi: Signal<String>,
    pub banxico: Signal<String>,
    pub denue_error: Signal<String>,
    pub inegi_error: Signal<String>,
    pub banxico_error: Signal<String>,
    pub checking: Signal<bool>,
    /// A source did not answer: the user may save explicitly without confirmation.
    pub unverified: Signal<bool>,
}

pub(crate) fn use_token_form() -> TokenForm {
    TokenForm {
        denue: use_signal(String::new),
        inegi: use_signal(String::new),
        banxico: use_signal(String::new),
        denue_error: use_signal(String::new),
        inegi_error: use_signal(String::new),
        banxico_error: use_signal(String::new),
        checking: use_signal(|| false),
        unverified: use_signal(|| false),
    }
}

impl TokenForm {
    pub fn is_empty(&self) -> bool {
        [self.denue, self.inegi, self.banxico]
            .iter()
            .all(|field| field.read().trim().is_empty())
    }

    fn store(mut self, paths: &AppPaths, require_denue: bool, on_saved: Callback<()>) {
        let saved = save_tokens(
            paths,
            &self.denue.peek(),
            &self.inegi.peek(),
            &self.banxico.peek(),
            require_denue,
        );
        match saved {
            Ok(()) => {
                self.denue.set(String::new());
                self.inegi.set(String::new());
                self.banxico.set(String::new());
                self.unverified.set(false);
                on_saved.call(());
            }
            Err(e) => self.denue_error.set(e),
        }
    }

    /// Checks the typed tokens with their sources and saves only what was confirmed.
    /// `skip_check` is the explicit "save without verifying" path after an outage.
    pub fn submit(
        mut self,
        paths: AppPaths,
        require_denue: bool,
        skip_check: bool,
        on_saved: Callback<()>,
    ) {
        if *self.checking.peek() {
            return;
        }
        let denue = self.denue.peek().trim().to_owned();
        let inegi = self.inegi.peek().trim().to_owned();
        let banxico = self.banxico.peek().trim().to_owned();
        self.unverified.set(false);
        let shape = |token: &str| {
            if token.is_empty() || token_shape_ok(token) {
                String::new()
            } else {
                SHAPE_ERROR.to_owned()
            }
        };
        self.denue_error.set(shape(&denue));
        self.inegi_error.set(shape(&inegi));
        self.banxico_error.set(shape(&banxico));
        if require_denue
            && denue.is_empty()
            && paths.secrets().map(|s| s.denue.is_empty()).unwrap_or(true)
        {
            self.denue_error
                .set("Pega tu token DENUE para continuar, o elige «Configurar después».".into());
        }
        if [self.denue_error, self.inegi_error, self.banxico_error]
            .iter()
            .any(|e| !e.peek().is_empty())
        {
            return;
        }
        if skip_check || (denue.is_empty() && banxico.is_empty()) {
            self.store(&paths, require_denue, on_saved);
            return;
        }
        self.checking.set(true);
        spawn(async move {
            // Network work stays off the interface thread.
            let checks = tokio::spawn(async move {
                let client = GovernmentClient::new().ok()?;
                let denue_check = if denue.is_empty() {
                    TokenCheck::Valid
                } else {
                    client.check_denue(&denue).await
                };
                let banxico_check = if banxico.is_empty() {
                    TokenCheck::Valid
                } else {
                    client.check_banxico(&banxico).await
                };
                Some((denue_check, banxico_check))
            })
            .await
            .ok()
            .flatten()
            .unwrap_or((TokenCheck::Unreachable, TokenCheck::Unreachable));
            self.checking.set(false);
            self.denue_error.set(check_message("INEGI", checks.0));
            self.banxico_error.set(check_message("Banxico", checks.1));
            if checks.0 == TokenCheck::Valid && checks.1 == TokenCheck::Valid {
                self.store(&paths, require_denue, on_saved);
            } else if checks.0 != TokenCheck::Rejected && checks.1 != TokenCheck::Rejected {
                self.unverified.set(true);
            }
        });
    }
}

#[component]
fn EyeIcon(open: bool) -> Element {
    rsx! {
        svg {
            view_box: "0 0 16 16",
            width: "16",
            height: "16",
            fill: "none",
            stroke: "currentColor",
            stroke_width: "1.4",
            stroke_linecap: "round",
            stroke_linejoin: "round",
            "aria-hidden": "true",
            path { d: "M1.5 8S4 3.5 8 3.5 14.5 8 14.5 8 12 12.5 8 12.5 1.5 8 1.5 8z" }
            circle { cx: "8", cy: "8", r: "2" }
            if !open {
                path { d: "M2.5 13.5l11-11" }
            }
        }
    }
}

/// Token field with a show/hide toggle and its error right below.
#[component]
pub(crate) fn TokenInput(
    id: String,
    value: Signal<String>,
    error: Signal<String>,
    placeholder: String,
    #[props(default)] required: bool,
) -> Element {
    let mut visible = use_signal(|| false);
    let shown = *visible.read();
    let message = error.read().clone();
    rsx! {
        div { class: if message.is_empty() { "token-input" } else { "token-input invalid" },
            input {
                id: "{id}",
                r#type: if shown { "text" } else { "password" },
                autocomplete: "off",
                spellcheck: "false",
                aria_required: required,
                aria_invalid: !message.is_empty(),
                placeholder,
                value: "{value}",
                oninput: move |e| {
                    value.set(e.value());
                    error.set(String::new());
                },
            }
            button {
                class: "token-eye",
                r#type: "button",
                aria_label: if shown { "Ocultar token" } else { "Mostrar token" },
                aria_pressed: shown,
                title: if shown { "Ocultar token" } else { "Mostrar token" },
                onclick: move |_| visible.set(!shown),
                EyeIcon { open: shown }
            }
        }
        if !message.is_empty() {
            p { class: "token-error", role: "alert", "{message}" }
        }
    }
}

/// Native file picker for a backup archive. Not available on iOS.
pub(crate) async fn pick_backup_file() -> Option<std::path::PathBuf> {
    #[cfg(not(target_os = "ios"))]
    {
        rfd::AsyncFileDialog::new()
            .set_title("Elige tu respaldo")
            .add_filter("Respaldo de Prospect Intelligence", &["zip"])
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf())
    }
    #[cfg(target_os = "ios")]
    {
        None
    }
}

#[component]
pub(crate) fn FirstRunCredentialsModal() -> Element {
    let services = consume_context::<Services>();
    let mut onboarding = consume_context::<OnboardingControl>();
    let paths = services.db.paths.clone();
    let configured = paths.secrets().unwrap_or_default();
    let desktop = services.layout != Layout::Ios;

    let form = use_token_form();
    let mut from_backup = use_signal(|| false);
    let mut restoring = use_signal(|| false);
    let mut error = use_signal(String::new);
    let restore_db = services.db.clone();
    let was_missing_denue = configured.denue.trim().is_empty();
    let on_saved = use_callback(move |()| {
        onboarding.revision += 1;
        if was_missing_denue {
            onboarding.first_run_pending_sync.set(true);
        }
        onboarding.open.set(false);
    });
    let save_paths = paths.clone();
    let skip_paths = paths.clone();
    let checking = *form.checking.read();
    let restore_mode = *from_backup.read();

    rsx! {
        style { {include_str!("onboarding.css")} }
        div { class:"onboarding-backdrop", "data-theme":services.settings.theme,
            div {
                class:if services.layout == Layout::Ios { "onboarding-dialog ios" } else { "onboarding-dialog" },
                role:"dialog",
                aria_modal:"true",
                aria_labelledby:"onboarding-title",
                div { class:"onboarding-head",
                    img {
                        class:"onboarding-icon",
                        src:ui::brand_icon_url(),
                        width:"40",
                        height:"40",
                        alt:"",
                        aria_hidden:"true"
                    }
                    div {
                        p { class:"onboarding-eyebrow", "PRIMER USO" }
                        h2 { id:"onboarding-title", "Configura tus credenciales" }
                    }
                }
                if desktop {
                    div { class:"onboarding-modes", role:"group", aria_label:"Cómo quieres empezar",
                        button {
                            class:if restore_mode { "onboarding-mode" } else { "onboarding-mode active" },
                            r#type:"button",
                            aria_pressed:!restore_mode,
                            onclick:move |_| { from_backup.set(false); error.set(String::new()); },
                            strong { "Empezar de cero" }
                            span { "Pega tus tokens y descarga tu base." }
                        }
                        button {
                            class:if restore_mode { "onboarding-mode active" } else { "onboarding-mode" },
                            r#type:"button",
                            aria_pressed:restore_mode,
                            onclick:move |_| { from_backup.set(true); error.set(String::new()); },
                            strong { "Cargar respaldo" }
                            span { "Recupera base, plantillas y tokens desde un .zip." }
                        }
                    }
                }
                if restore_mode {
                    div { class:"onboarding-restore",
                        p { class:"onboarding-copy",
                            "Elige el archivo .zip que creaste con «Crear respaldo». Se cargan tus empresas, notas, plantillas, ajustes y tokens."
                        }
                        button {
                            class:"onboarding-primary",
                            r#type:"button",
                            disabled:*restoring.read(),
                            onclick:move |_| {
                                let db = restore_db.clone();
                                restoring.set(true);
                                error.set(String::new());
                                spawn(async move {
                                    if let Some(file) = pick_backup_file().await {
                                        match db.restore_archive(file).await {
                                            Ok(restored) => {
                                                onboarding.restored.set(Some(restored));
                                                onboarding.revision += 1;
                                                onboarding.open.set(false);
                                            }
                                            Err(e) => error.set(e.to_string()),
                                        }
                                    }
                                    restoring.set(false);
                                });
                            },
                            if *restoring.read() { "Cargando respaldo…" } else { "Elegir archivo de respaldo" }
                        }
                    }
                } else {
                    p { class:"onboarding-copy",
                        "La app no trae datos ni claves. Sólo el token DENUE es necesario para empezar; se comprueba al guardar y se queda en tu equipo."
                    }
                    div { class:"onboarding-form",
                        div { class:"onboarding-field required",
                            div { class:"onboarding-field-head",
                                div { class:"onboarding-field-title",
                                    TokenFieldIcon { kind: TokenFieldKind::Directory }
                                    label { r#for:"onboarding-denue", "Token DENUE (INEGI)" }
                                }
                                span { class:"onboarding-badge required",
                                    RequirementBadgeIcon { required: true }
                                    "Obligatorio"
                                }
                            }
                            TokenSourceHint {
                                guide: DENUE_GUIDE,
                                hint_class: "onboarding-hint",
                                link_class: "onboarding-link",
                                url_class: "onboarding-url",
                                on_error: move |msg| error.set(msg),
                            }
                            TokenInput {
                                id: "onboarding-denue",
                                value: form.denue,
                                error: form.denue_error,
                                required: true,
                                placeholder: if configured.denue.is_empty() { "Pega tu token DENUE" } else { "Ya configurado · pega otro para reemplazarlo" },
                            }
                        }
                        div { class:"onboarding-field optional",
                            div { class:"onboarding-field-head",
                                div { class:"onboarding-field-title",
                                    TokenFieldIcon { kind: TokenFieldKind::Indicators }
                                    label { r#for:"onboarding-inegi", "INEGI Indicadores" }
                                }
                                span { class:"onboarding-badge optional",
                                    RequirementBadgeIcon { required: false }
                                    "Opcional"
                                }
                            }
                            TokenSourceHint {
                                guide: INEGI_INDICATORS_GUIDE,
                                hint_class: "onboarding-hint",
                                link_class: "onboarding-link",
                                url_class: "onboarding-url",
                                on_error: move |msg| error.set(msg),
                            }
                            TokenInput {
                                id: "onboarding-inegi",
                                value: form.inegi,
                                error: form.inegi_error,
                                placeholder: if configured.inegi.is_empty() { "Opcional · si lo omites se usa tu token DENUE" } else { "Ya configurado · pega otro para reemplazarlo" },
                            }
                        }
                        div { class:"onboarding-field optional",
                            div { class:"onboarding-field-head",
                                div { class:"onboarding-field-title",
                                    TokenFieldIcon { kind: TokenFieldKind::Currency }
                                    label { r#for:"onboarding-banxico", "Banxico SIE" }
                                }
                                span { class:"onboarding-badge optional",
                                    RequirementBadgeIcon { required: false }
                                    "Opcional"
                                }
                            }
                            TokenSourceHint {
                                guide: BANXICO_GUIDE,
                                hint_class: "onboarding-hint",
                                link_class: "onboarding-link",
                                url_class: "onboarding-url",
                                on_error: move |msg| error.set(msg),
                            }
                            TokenInput {
                                id: "onboarding-banxico",
                                value: form.banxico,
                                error: form.banxico_error,
                                placeholder: if configured.banxico.is_empty() { "Opcional · clave de 64 caracteres" } else { "Ya configurado · pega otro para reemplazarlo" },
                            }
                        }
                    }
                }
                if !error.read().is_empty() {
                    p { class:"onboarding-error", role:"alert", "{error}" }
                }
                div { class:"onboarding-actions",
                    button {
                        class:"onboarding-secondary",
                        r#type:"button",
                        onclick:move |_| onboarding.open.set(false),
                        "Configurar después"
                    }
                    if !restore_mode && *form.unverified.read() {
                        button {
                            class:"onboarding-secondary",
                            r#type:"button",
                            onclick:move |_| form.submit(skip_paths.clone(), true, true, on_saved),
                            "Guardar sin verificar"
                        }
                    }
                    if !restore_mode {
                        button {
                            class:"onboarding-primary",
                            r#type:"button",
                            disabled:checking,
                            onclick:move |_| form.submit(save_paths.clone(), true, false, on_saved),
                            if checking { "Verificando…" } else { "Guardar tokens" }
                        }
                    }
                }
                p { class:"onboarding-note", "Puedes cambiarlos después en Ajustes → Tokens y fuentes." }
            }
        }
    }
}
