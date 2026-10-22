use dioxus::prelude::*;
use prospect_core::{AppPaths, Command, Db, Mutation, Page, Profile, Query, Settings, View};

#[derive(Clone)]
pub struct MobileServices {
    pub db: Db,
    pub settings: Settings,
    #[allow(dead_code)]
    pub paths: AppPaths,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub enum MobileTab {
    #[default]
    Today,
    Followups,
    Conversations,
    Metrics,
    Settings,
}

impl MobileTab {
    fn label(self) -> &'static str {
        match self {
            Self::Today => "Hoy",
            Self::Followups => "Seguimientos",
            Self::Conversations => "Conversaciones",
            Self::Metrics => "Métricas",
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

pub fn copy_text(text: &str) {
    let escaped = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    let script =
        format!("if (navigator.clipboard) {{ navigator.clipboard.writeText({escaped}); }}");
    document::eval(&script);
}

pub fn open_external(url: &str) {
    let escaped = serde_json::to_string(url).unwrap_or_else(|_| "\"\"".into());
    let script = format!("window.open({escaped}, '_blank');");
    document::eval(&script);
}

#[component]
pub fn App() -> Element {
    let services = consume_context::<MobileServices>();
    let mut tab = use_signal(|| MobileTab::Today);
    let mut profile = use_signal(|| {
        Profile::parse(&services.settings.profile).unwrap_or(Profile::Automatizacion)
    });
    let mut selected_id = use_signal(|| None::<String>);
    let mut toast = use_signal(String::new);
    let revision = use_signal(|| 0u64);

    let active_tab = *tab.read();
    let current_profile = *profile.read();
    let db = services.db.clone();

    // Summary data for badges
    let summary = use_resource(move || {
        let _ = *revision.read();
        let db = db.clone();
        async move { db.summary().await.map_err(|e| e.to_string()) }
    });

    // Query for active tab
    let list_db = services.db.clone();
    let prospect_list = use_resource(move || {
        let _ = *revision.read();
        let db = list_db.clone();
        let q = Query {
            view: active_tab.view(),
            profile: current_profile,
            size: 40,
            ..Default::default()
        };
        async move {
            match active_tab {
                MobileTab::Today | MobileTab::Followups | MobileTab::Conversations => {
                    db.page(q).await.map_err(|e| e.to_string())
                }
                _ => Ok(Page::default()),
            }
        }
    });

    let totals = summary
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let page_data = prospect_list
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    rsx! {
        style { {include_str!("style.css")} }
        div { class: "mobile-app",
            if !toast.read().is_empty() {
                div {
                    class: "toast",
                    onclick: move |_| toast.set(String::new()),
                    "{toast}"
                }
            }

            header { class: "topbar",
                span { class: "topbar-title", "{active_tab.label()}" }
                select {
                    class: "topbar-profile",
                    value: current_profile.id(),
                    onchange: move |e| {
                        if let Ok(p) = Profile::parse(&e.value()) {
                            profile.set(p);
                            selected_id.set(None);
                        }
                    },
                    option { value: "automatizacion", "Automatización" }
                    option { value: "web_marketing", "Web y Marketing" }
                }
            }

            main { class: "content-pane",
                if let Some(id) = selected_id.read().clone() {
                    ProspectDetailView {
                        id,
                        profile: current_profile,
                        selected_id,
                        toast,
                        revision,
                    }
                } else {
                    match active_tab {
                        MobileTab::Today | MobileTab::Followups | MobileTab::Conversations => rsx! {
                            if page_data.items.is_empty() {
                                div { class: "empty-state",
                                    p { "No hay registros pendientes en esta vista." }
                                }
                            } else {
                                for item in page_data.items.iter() {
                                    {
                                        let pid = item.id.clone();
                                        rsx! {
                                            div {
                                                key: "{pid}",
                                                class: "card",
                                                onclick: move |_| selected_id.set(Some(pid.clone())),
                                                div { class: "card-header",
                                                    span { class: "card-title", "{item.nombre}" }
                                                    span { class: "badge", "Score {item.priority_score}" }
                                                }
                                                div { class: "card-meta",
                                                    "{item.municipio} · {item.giro}"
                                                }
                                                div { class: "card-preview",
                                                    "Fit: {item.fit_score} | Estado: {item.estado}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        MobileTab::Metrics => rsx! {
                            div { class: "card",
                                div { class: "card-header", strong { "Resumen de Embudo" } }
                                div { class: "card-meta", "Estadísticas acumuladas en base local" }
                                p { "Establecimientos registrados: {totals.total}" }
                                p { "Pendientes para hoy: {totals.today}" }
                                p { "Seguimientos vencidos: {totals.due}" }
                                p { "Conversaciones abiertas: {totals.conversations}" }
                                p { "Contactados hoy: {totals.contacted_today}" }
                            }
                        },
                        MobileTab::Settings => rsx! {
                            div { class: "card",
                                div { class: "card-header", strong { "Configuración del Sistema" } }
                                div { class: "card-meta", "Versión 0.2.1 · Soporte Móvil" }
                                p { "Base local: prospectos.db" }
                                p { "Perfil activo: {current_profile.label()}" }
                                p { "Persistencia: Directorio de aplicación seguro de iOS" }
                            }
                        },
                    }
                }
            }

            nav { class: "bottom-bar",
                button {
                    class: if active_tab == MobileTab::Today && selected_id.read().is_none() { "nav-item active" } else { "nav-item" },
                    onclick: move |_| { tab.set(MobileTab::Today); selected_id.set(None); },
                    span { "Hoy" }
                    if totals.today > 0 { span { class: "nav-count", "{totals.today}" } }
                }
                button {
                    class: if active_tab == MobileTab::Followups && selected_id.read().is_none() { "nav-item active" } else { "nav-item" },
                    onclick: move |_| { tab.set(MobileTab::Followups); selected_id.set(None); },
                    span { "Seguimiento" }
                    if totals.due > 0 { span { class: "nav-count", "{totals.due}" } }
                }
                button {
                    class: if active_tab == MobileTab::Conversations && selected_id.read().is_none() { "nav-item active" } else { "nav-item" },
                    onclick: move |_| { tab.set(MobileTab::Conversations); selected_id.set(None); },
                    span { "Chats" }
                    if totals.conversations > 0 { span { class: "nav-count", "{totals.conversations}" } }
                }
                button {
                    class: if active_tab == MobileTab::Metrics && selected_id.read().is_none() { "nav-item active" } else { "nav-item" },
                    onclick: move |_| { tab.set(MobileTab::Metrics); selected_id.set(None); },
                    span { "Métricas" }
                }
                button {
                    class: if active_tab == MobileTab::Settings && selected_id.read().is_none() { "nav-item active" } else { "nav-item" },
                    onclick: move |_| { tab.set(MobileTab::Settings); selected_id.set(None); },
                    span { "Ajustes" }
                }
            }
        }
    }
}

#[component]
fn ProspectDetailView(
    id: String,
    profile: Profile,
    mut selected_id: Signal<Option<String>>,
    mut toast: Signal<String>,
    mut revision: Signal<u64>,
) -> Element {
    let services = consume_context::<MobileServices>();
    let id_for_fetch = id.clone();
    let db = services.db.clone();

    let prospect_detail = use_resource(move || {
        let id = id_for_fetch.clone();
        let db = db.clone();
        async move {
            let p = db
                .get(id.clone(), profile)
                .await
                .map_err(|e| e.to_string())?;
            let signals = db.signals(id.clone()).await.map_err(|e| e.to_string())?;
            Ok::<_, String>((p, signals))
        }
    });

    let (prospect, signals) = match prospect_detail.read().as_ref() {
        Some(Ok(data)) => (Some(data.0.clone()), data.1.clone()),
        _ => (None, Vec::new()),
    };

    let p = match prospect {
        Some(p) => p,
        None => return rsx! { div { class: "empty-state", "Cargando ficha..." } },
    };

    let mut message_body = use_signal(|| p.mensaje.clone());
    let prospect_phone = p.telefono.clone();
    let prospect_email = p.correo.clone();

    rsx! {
        div { class: "detail-view",
            button {
                class: "btn btn-secondary",
                style: "margin-bottom: 12px;",
                onclick: move |_| selected_id.set(None),
                "← Volver a la lista"
            }

            div { class: "detail-title", "{p.nombre}" }
            div { class: "card-meta", "{p.municipio}, {p.entidad} · {p.giro}" }

            div { class: "btn-row",
                span { class: "badge", "Fit: {p.fit_score}" }
                span { class: "badge", "Prioridad: {p.priority_score}" }
                span { class: "badge", "Estado: {p.estado}" }
            }

            if !p.problema.is_empty() {
                div { class: "detail-section",
                    div { class: "detail-section-title", "Hipótesis comercial" }
                    div { class: "card-preview", "{p.problema}" }
                }
            }

            if !signals.is_empty() {
                div { class: "detail-section",
                    div { class: "detail-section-title", "Señales web detectadas" }
                    for sig in signals.iter() {
                        div {
                            key: "{sig.signal_source_url}:{sig.signal_type}",
                            class: "card-meta",
                            "• {sig.signal_type}: {sig.signal_text}"
                        }
                    }
                }
            }

            div { class: "detail-section",
                div { class: "detail-section-title", "Borrador de mensaje" }
                textarea {
                    class: "textarea",
                    value: "{message_body}",
                    oninput: move |e| message_body.set(e.value()),
                }

                div { class: "btn-row",
                    button {
                        class: "btn btn-primary",
                        onclick: move |_| {
                            copy_text(&message_body.read());
                            toast.set("Mensaje copiado al portapapeles".into());
                        },
                        "Copiar mensaje"
                    }
                    if !prospect_phone.is_empty() {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                let link = format!("https://wa.me/52{}", prospect_phone.replace([' ', '-'], ""));
                                open_external(&link);
                            },
                            "WhatsApp"
                        }
                    }
                    if !prospect_email.is_empty() {
                        button {
                            class: "btn btn-secondary",
                            onclick: move |_| {
                                let link = format!("mailto:{}?body={}", prospect_email, message_body.read());
                                open_external(&link);
                            },
                            "Email"
                        }
                    }
                }
            }

            div { class: "detail-section",
                div { class: "detail-section-title", "Acciones de contacto" }
                div { class: "btn-row",
                    button {
                        class: "btn btn-secondary",
                        onclick: {
                            let db = services.db.clone();
                            let pid = p.id.clone();
                            let settings = services.settings.clone();
                            let msg = message_body.read().clone();
                            move |_| {
                                let db = db.clone();
                                let pid = pid.clone();
                                let settings = settings.clone();
                                let msg = msg.clone();
                                spawn(async move {
                                    let req = Mutation {
                                        operation_id: prospect_core::operation_id(),
                                        prospect_id: pid,
                                        profile,
                                        command: Command::Contact {
                                            channel: "whatsapp".into(),
                                            variant: "default".into(),
                                            message: msg,
                                            subject: String::new(),
                                        },
                                        notes: None,
                                    };
                                    if let Ok(outcome) = db.mutate(req, settings).await {
                                        toast.set(format!("Registrado: {outcome}"));
                                        revision += 1;
                                        selected_id.set(None);
                                    }
                                });
                            }
                        },
                        "Marcar Contactado"
                    }
                    button {
                        class: "btn btn-secondary",
                        onclick: {
                            let db = services.db.clone();
                            let pid = p.id.clone();
                            let settings = services.settings.clone();
                            move |_| {
                                let db = db.clone();
                                let pid = pid.clone();
                                let settings = settings.clone();
                                spawn(async move {
                                    let req = Mutation {
                                        operation_id: prospect_core::operation_id(),
                                        prospect_id: pid,
                                        profile,
                                        command: Command::Response {
                                            response: "positivo".into(),
                                        },
                                        notes: None,
                                    };
                                    if let Ok(outcome) = db.mutate(req, settings).await {
                                        toast.set(format!("Registrado: {outcome}"));
                                        revision += 1;
                                        selected_id.set(None);
                                    }
                                });
                            }
                        },
                        "Respuesta Positiva"
                    }
                }
            }
        }
    }
}
