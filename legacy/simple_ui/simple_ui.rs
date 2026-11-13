use crate::{Layout, Services, ui};
use dioxus::prelude::*;
use prospect_core::{
    Command, Db, Interaction, Message, Mutation, Page, Profile, Prospect, Query, QueryOrder,
    Settings, View,
};

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum WorkScreen {
    #[default]
    Today,
    Followups,
    Conversations,
}

impl WorkScreen {
    fn label(self) -> &'static str {
        match self {
            Self::Today => "Hoy",
            Self::Followups => "En seguimiento",
            Self::Conversations => "Conversaciones",
        }
    }
}

#[derive(Clone, PartialEq)]
struct DetailData {
    prospect: Prospect,
    history: Vec<Interaction>,
    message: Message,
}

fn status_label(status: &str) -> &'static str {
    match status {
        "nuevo" => "Sin revisar",
        "revisado" => "Listo para contactar",
        "contactado" => "Esperando respuesta",
        "respondio" => "Respondió",
        "respuesta_positiva" => "Interesado",
        "reunion" => "Reunión",
        "propuesta" => "Propuesta enviada",
        "cliente" => "Cliente",
        "no_interesado" => "No interesado",
        "sin_respuesta" => "Sin respuesta",
        "descartado" => "Descartado",
        _ => "En proceso",
    }
}

fn event_label(kind: &str) -> &'static str {
    match kind {
        "contacto" => "Contacto inicial enviado",
        "seguimiento" => "Seguimiento enviado",
        "respuesta" => "Respuesta registrada",
        "nota" => "Nota añadida",
        _ => "Actividad registrada",
    }
}

fn pretty_date(raw: &str) -> String {
    let mut parts = raw.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(year), Some(month), Some(day)) if year.len() == 4 => {
            let month = match month {
                "01" => "ene",
                "02" => "feb",
                "03" => "mar",
                "04" => "abr",
                "05" => "may",
                "06" => "jun",
                "07" => "jul",
                "08" => "ago",
                "09" => "sep",
                "10" => "oct",
                "11" => "nov",
                "12" => "dic",
                _ => month,
            };
            format!("{} {} {}", day.trim_start_matches('0'), month, year)
        }
        _ => raw.to_owned(),
    }
}

fn copy_text(text: &str) -> Result<(), String> {
    #[cfg(not(target_os = "ios"))]
    {
        arboard::Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text.to_owned()))
            .map_err(|e| e.to_string())
    }
    #[cfg(target_os = "ios")]
    {
        let _ = text;
        Err("Usa la vista móvil para copiar el mensaje".into())
    }
}

fn open_link(link: &str) -> Result<(), String> {
    #[cfg(not(target_os = "ios"))]
    {
        opener::open(link).map_err(|e| e.to_string())
    }
    #[cfg(target_os = "ios")]
    {
        let _ = link;
        Err("Usa la vista móvil para abrir el enlace".into())
    }
}

fn run_mutation(
    db: Db,
    settings: Settings,
    mutation: Mutation,
    mut revision: Signal<u64>,
    mut toast: Signal<String>,
    mut busy: Signal<bool>,
) {
    if *busy.peek() {
        return;
    }
    let success = match &mutation.command {
        Command::Contact { .. } => {
            "Contacto registrado. Ya está en En seguimiento y el próximo seguimiento quedó programado."
        }
        Command::Followup { .. } => {
            "Seguimiento registrado. La siguiente fecha se actualizó automáticamente."
        }
        Command::Response { response } if response == "interesado" => {
            "Respuesta registrada como interesado."
        }
        Command::Response { .. } => "Respuesta registrada.",
        Command::State { state } if state == "reunion" => "Reunión registrada.",
        Command::State { state } if state == "propuesta" => "Propuesta registrada.",
        Command::State { state } if state == "cliente" => "Cliente registrado.",
        Command::State { .. } => "Estado actualizado.",
        Command::Snooze { .. } => "Acción pospuesta.",
        Command::Research { .. } => "Negocio marcado como revisado.",
        Command::Note { .. } => "Nota guardada.",
        Command::Publication { .. } => "Publicación registrada.",
    }
    .to_owned();
    busy.set(true);
    spawn(async move {
        match db.mutate(mutation, settings).await {
            Ok(_) => {
                revision += 1;
                toast.set(success);
            }
            Err(error) => toast.set(error.to_string()),
        }
        busy.set(false);
    });
}

#[component]
pub fn App() -> Element {
    let services = consume_context::<Services>();
    if services.layout == Layout::Ios {
        return rsx! { ui::App {} };
    }
    rsx! { SimpleDesktopApp {} }
}

#[component]
fn SimpleDesktopApp() -> Element {
    let services = consume_context::<Services>();
    let db = services.db.clone();
    let settings = services.settings.clone();
    let onboarding = try_use_context::<crate::onboarding::OnboardingControl>();
    let profile = Profile::parse(&settings.profile).unwrap_or_default();
    let mut screen = use_signal(WorkScreen::default);
    let mut selected = use_signal(String::new);
    let mut revision = use_signal(|| 0u64);
    let mut toast = use_signal(String::new);
    let busy = use_signal(|| false);
    let mut advanced = use_signal(|| false);
    let mut notification_hidden = use_signal(|| false);

    if *advanced.read() {
        return rsx! {
            style { {include_str!("simple_ui.css")} }
            div { class:"simple-advanced-return",
                button {
                    onclick:move |_| {
                        if let Some(mut o) = onboarding {
                            o.initial_screen.set(None);
                        }
                        revision += 1;
                        advanced.set(false);
                    },
                    "← Volver al flujo simple"
                }
            }
            ui::App {}
        };
    }

    let summary_db = db.clone();
    let summary = use_resource(move || {
        let _ = *revision.read();
        let db = summary_db.clone();
        async move { db.summary().await.map_err(|e| e.to_string()) }
    });

    let due_db = db.clone();
    let due = use_resource(move || {
        let _ = *revision.read();
        let db = due_db.clone();
        async move {
            db.page(Query {
                profile,
                view: View::Due,
                size: 20,
                order: QueryOrder::Followup,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())
        }
    });

    let today_db = db.clone();
    let today = use_resource(move || {
        let _ = *revision.read();
        let db = today_db.clone();
        async move {
            db.page(Query {
                profile,
                view: View::Today,
                contactable: true,
                size: 30,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())
        }
    });

    let following_db = db.clone();
    let following = use_resource(move || {
        let _ = *revision.read();
        let db = following_db.clone();
        async move {
            db.page(Query {
                profile,
                view: View::All,
                state: "contactado".into(),
                size: 100,
                order: QueryOrder::Followup,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())
        }
    });

    let conversations_db = db.clone();
    let conversations = use_resource(move || {
        let _ = *revision.read();
        let db = conversations_db.clone();
        async move {
            db.page(Query {
                profile,
                view: View::Conversations,
                size: 100,
                ..Default::default()
            })
            .await
            .map_err(|e| e.to_string())
        }
    });

    let summary_loaded = summary
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned();
    let totals = summary_loaded.clone().unwrap_or_default();
    let _ = onboarding.map(|o| *o.revision.read());
    let denue_configured = !db
        .paths
        .secrets()
        .map(|s| s.denue.trim().is_empty())
        .unwrap_or(true);
    let due_page = due
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let today_page = today
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let following_page = following
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();
    let conversations_page = conversations
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned()
        .unwrap_or_default();

    rsx! {
        style { {include_str!("simple_ui.css")} }
        div { class:"simple-app",
            aside { class:"simple-sidebar",
                div { class:"simple-brand", strong { "Prospect" } span { "Intelligence" } }
                p { class:"simple-kicker", "TRABAJO COMERCIAL" }
                nav {
                    SimpleNavButton { label:"Hoy", count:totals.due + totals.today + totals.conversations, active:*screen.read()==WorkScreen::Today, onclick:move |_|{screen.set(WorkScreen::Today);selected.set(String::new());} }
                    SimpleNavButton { label:"En seguimiento", count:following_page.total, active:*screen.read()==WorkScreen::Followups, onclick:move |_|{screen.set(WorkScreen::Followups);selected.set(String::new());} }
                    SimpleNavButton { label:"Conversaciones", count:conversations_page.total, active:*screen.read()==WorkScreen::Conversations, onclick:move |_|{screen.set(WorkScreen::Conversations);selected.set(String::new());} }
                }
                div { class:"simple-sidebar-bottom",
                    button { class:"simple-advanced", onclick:move |_|advanced.set(true), "Herramientas avanzadas" }
                    span { "Base, métricas, oportunidades, mapas y ajustes" }
                }
            }
            main { class:"simple-main",
                header { class:"simple-header",
                    div { p { class:"simple-kicker", "QUÉ SIGUE" } h1 { "{screen.read().label()}" } }
                    div { class:"simple-day-summary",
                        span { strong { "{totals.contacted_today}" } " contactos hoy" }
                        span { strong { "{totals.followed_today}" } " seguimientos hoy" }
                    }
                }
                if summary_loaded.as_ref().is_some_and(|s| s.total == 0) {
                    div { class:"simple-notification", role:"status",
                        div {
                            if !denue_configured {
                                strong { "Configura tu token DENUE para comenzar" }
                                p { "La aplicación no incluye credenciales ni bases precargadas. Configura tus tokens gratuitos de INEGI (DENUE / Indicadores) y Banxico SIE con enlaces directos a los portales oficiales." }
                            } else {
                                strong { "Tu base local está vacía" }
                                p { "Tus credenciales están listas. Abre Actualizaciones para importar tu primer lote de establecimientos." }
                            }
                            div { class:"simple-action-row",
                                if !denue_configured {
                                    button {
                                        class:"primary",
                                        onclick:move |_| {
                                            if let Some(mut o) = onboarding {
                                                o.open.set(true);
                                            }
                                        },
                                        "Configurar tokens"
                                    }
                                    button {
                                        class:"secondary",
                                        onclick:move |_| {
                                            if let Some(mut o) = onboarding {
                                                o.initial_screen.set(Some(ui::Screen::Settings));
                                            }
                                            advanced.set(true);
                                        },
                                        "Abrir Ajustes"
                                    }
                                } else {
                                    button {
                                        class:"primary",
                                        onclick:move |_| {
                                            if let Some(mut o) = onboarding {
                                                o.initial_screen.set(Some(ui::Screen::Updates));
                                            }
                                            advanced.set(true);
                                        },
                                        "Ir a descargar datos"
                                    }
                                }
                            }
                        }
                    }
                }
                if totals.due > 0 && !*notification_hidden.read() {
                    div { class:"simple-notification", role:"status",
                        div { strong { "Tienes {totals.due} seguimiento(s) para hoy" } p { "Están arriba de tu lista para que no tengas que recordarlos manualmente." } }
                        button { onclick:move |_|notification_hidden.set(true), aria_label:"Cerrar aviso", "×" }
                    }
                }
                if !toast.read().is_empty() {
                    div { class:"simple-toast", role:"status", span { "{toast}" } button { onclick:move |_|toast.set(String::new()), "×" } }
                }
                div { class:if selected.read().is_empty(){"simple-workspace"}else{"simple-workspace with-detail"},
                    section { class:"simple-list-area",
                        match *screen.read() {
                            WorkScreen::Today => rsx! {
                                ActionSection { title:"Requieren acción hoy", subtitle:"Seguimientos que ya llegaron a su fecha", empty:"No tienes seguimientos vencidos.", data:due_page.clone(), selected, kind:"due" }
                                ActionSection { title:"Conversaciones abiertas", subtitle:"Ya respondieron; tú decides el siguiente paso", empty:"No hay conversaciones abiertas.", data:conversations_page.clone(), selected, kind:"conversation" }
                                ActionSection { title:"Para contactar", subtitle:"Negocios listos para un primer mensaje", empty:"No hay nuevos contactos con los filtros actuales.", data:today_page.clone(), selected, kind:"new" }
                            },
                            WorkScreen::Followups => rsx! {
                                ActionSection { title:"Todos tus seguimientos", subtitle:"Aquí permanecen desde que registras el primer contacto", empty:"Aún no tienes negocios en seguimiento.", data:following_page.clone(), selected, kind:"followup" }
                            },
                            WorkScreen::Conversations => rsx! {
                                ActionSection { title:"Conversaciones", subtitle:"Negocios que ya respondieron o avanzaron", empty:"Todavía no hay conversaciones abiertas.", data:conversations_page.clone(), selected, kind:"conversation" }
                            },
                        }
                    }
                    if !selected.read().is_empty() {
                        SimpleProspectDetail { key:"{selected}:{revision}", id:selected.read().clone(), profile, revision, toast, busy }
                    }
                }
            }
        }
    }
}

#[component]
fn SimpleNavButton(
    label: &'static str,
    count: usize,
    active: bool,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        button { class:if active{"simple-nav active"}else{"simple-nav"}, onclick,
            span { "{label}" }
            if count > 0 { strong { "{count}" } }
        }
    }
}

#[component]
fn ActionSection(
    title: &'static str,
    subtitle: &'static str,
    empty: &'static str,
    data: Page,
    mut selected: Signal<String>,
    kind: &'static str,
) -> Element {
    rsx! {
        section { class:"simple-section",
            div { class:"simple-section-heading",
                div { h2 { "{title}" } p { "{subtitle}" } }
                span { "{data.total}" }
            }
            if data.items.is_empty() {
                p { class:"simple-empty", "{empty}" }
            } else {
                div { class:"simple-cards",
                    for p in data.items.iter() {
                        { let id=p.id.clone(); let active=*selected.read()==p.id; rsx! {
                            button { class:if active{"simple-card selected"}else{"simple-card"}, onclick:move |_|selected.set(id.clone()),
                                div { class:"simple-card-main",
                                    strong { "{p.nombre}" }
                                    span { "{p.municipio} · {p.giro}" }
                                }
                                div { class:"simple-card-status",
                                    if p.estado=="contactado" {
                                        if !p.proximo_seguimiento.is_empty() && p.proximo_seguimiento <= prospect_core::config::today().to_string() {
                                            strong { "Seguimiento pendiente" }
                                        } else {
                                            strong { "Esperando respuesta" }
                                        }
                                        if !p.proximo_seguimiento.is_empty() { span { "Próximo: {pretty_date(&p.proximo_seguimiento)}" } }
                                    } else {
                                        strong { "{status_label(&p.estado)}" }
                                        if kind=="new" { span { "Prioridad {p.priority_score}" } }
                                    }
                                }
                            }
                        } }
                    }
                }
            }
        }
    }
}

#[component]
fn SimpleProspectDetail(
    id: String,
    profile: Profile,
    mut revision: Signal<u64>,
    mut toast: Signal<String>,
    busy: Signal<bool>,
) -> Element {
    let services = consume_context::<Services>();
    let db = services.db.clone();
    let settings = services.settings.clone();
    let mut channel = use_signal(|| "email".to_owned());
    let mut body = use_signal(String::new);
    let mut subject = use_signal(String::new);
    let mut draft_key = use_signal(String::new);
    let mut advanced_state = use_signal(|| "revisado".to_owned());
    let id_for_resource = id.clone();
    let detail_db = db.clone();
    let detail_settings = settings.clone();

    let detail = use_resource(move || {
        let id = id_for_resource.clone();
        let _ = *revision.read();
        let ch = channel.read().clone();
        let db = detail_db.clone();
        let settings = detail_settings.clone();
        async move {
            let p = db
                .get(id.clone(), profile)
                .await
                .map_err(|e| e.to_string())?;
            let history = db.history(id.clone()).await.map_err(|e| e.to_string())?;
            let signals = db.signals(id).await.map_err(|e| e.to_string())?;
            let templates = prospect_core::messages::Templates::load(&db.paths.home)
                .map_err(|e| e.to_string())?;
            let message = if p.estado == "contactado" {
                prospect_core::messages::followup(
                    &p,
                    p.toques.max(1),
                    &settings,
                    &templates,
                    profile,
                )
            } else if ["nuevo", "revisado"].contains(&p.estado.as_str()) {
                prospect_core::messages::first(
                    &p,
                    if ch == "email" { "email" } else { "whatsapp" },
                    &signals,
                    &settings,
                    &templates,
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
                history,
                message,
            })
        }
    });

    use_effect(move || {
        if let Some(Ok(data)) = detail.read().as_ref() {
            let key = format!(
                "{}:{}:{}:{}",
                data.prospect.id,
                data.prospect.estado,
                data.prospect.toques,
                channel.read()
            );
            if *draft_key.peek() != key {
                body.set(data.message.cuerpo.clone());
                subject.set(data.message.asunto.clone());
                advanced_state.set(data.prospect.estado.clone());
                draft_key.set(key);
            }
        }
    });

    let data = detail
        .read()
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .cloned();
    let Some(data) = data else {
        return rsx! { aside { class:"simple-detail", p { class:"simple-empty", "Cargando…" } } };
    };
    let p = data.prospect;

    let fresh = ["nuevo", "revisado"].contains(&p.estado.as_str());
    let contacted = p.estado == "contactado";
    let due = contacted
        && !p.proximo_seguimiento.is_empty()
        && p.proximo_seguimiento <= prospect_core::config::today().to_string();
    let can_send = fresh || due;
    let email_link = prospect_core::contacts::mailto(p.email(), &subject.read(), &body.read());
    let wa_link = prospect_core::contacts::whatsapp_url(&p.phone(), &body.read());

    let action = |command: Command| Mutation {
        operation_id: prospect_core::operation_id(),
        prospect_id: p.id.clone(),
        profile,
        command,
        notes: None,
    };

    rsx! {
        aside { class:"simple-detail",
            div { class:"simple-detail-head",
                div { p { class:"simple-kicker", "NEGOCIO" } h2 { "{p.nombre}" } span { "{p.municipio} · {p.entidad}" } }
                span { class:"simple-status", "{status_label(&p.estado)}" }
            }

            if contacted {
                div { class:if due{"simple-next-action due"}else{"simple-next-action"},
                    strong { if due { "Seguimiento pendiente" } else { "Esperando respuesta" } }
                    if !p.contactado_en.is_empty() { p { "Contacto inicial: {pretty_date(&p.contactado_en)}" } }
                    if !p.proximo_seguimiento.is_empty() { p { "Próximo seguimiento: {pretty_date(&p.proximo_seguimiento)}" } }
                    span { {if p.toques<=1 { "Contacto inicial enviado".to_owned() } else { format!("{} seguimiento(s) enviados", p.toques-1) }} }
                }
            } else if ["respondio", "respuesta_positiva", "reunion", "propuesta"].contains(&p.estado.as_str()) {
                div { class:"simple-next-action",
                    strong { "Conversación abierta" }
                    p { "Registra lo que ocurrió; el sistema moverá el negocio a la siguiente etapa." }
                }
            }

            if can_send {
                section { class:"simple-detail-section",
                    h3 { if fresh { "Mensaje inicial" } else { "Seguimiento" } }
                    div { class:"simple-channel-row",
                        button { class:if *channel.read()=="email"{"active"}else{""}, disabled:p.email().is_empty(), onclick:move |_|channel.set("email".into()), "Correo" }
                        button { class:if *channel.read()=="whatsapp"{"active"}else{""}, disabled:p.phone().is_empty(), onclick:move |_|channel.set("whatsapp".into()), "WhatsApp" }
                    }
                    if *channel.read()=="email" {
                        label { "Asunto" input { value:"{subject}", oninput:move |e|subject.set(e.value()) } }
                    }
                    label { "Mensaje" textarea { rows:"9", value:"{body}", oninput:move |e|body.set(e.value()) } }
                    div { class:"simple-action-row",
                        button { class:"secondary", onclick:move |_|match copy_text(&body.read()){Ok(())=>toast.set("Mensaje copiado".into()),Err(e)=>toast.set(e)}, "Copiar" }
                        if *channel.read()=="email" && !email_link.is_empty() { {let link=email_link.clone();rsx!{button{class:"secondary",onclick:move |_|if let Err(e)=open_link(&link){toast.set(e)},"Abrir correo"}}} }
                        if *channel.read()=="whatsapp" && !wa_link.is_empty() { {let link=wa_link.clone();rsx!{button{class:"secondary",onclick:move |_|if let Err(e)=open_link(&link){toast.set(e)},"Abrir WhatsApp"}}} }
                    }
                    p { class:"simple-help", "Tú envías el mensaje. Después registra el envío para que el seguimiento quede programado." }
                    if fresh {
                        {let mutation=action(Command::Contact{channel:channel.read().clone(),variant:data.message.variante.clone(),message:body.read().clone(),subject:subject.read().clone()});let db=db.clone();let settings=settings.clone();rsx!{
                            button { class:"primary full", disabled:*busy.read(), onclick:move |_|run_mutation(db.clone(),settings.clone(),mutation.clone(),revision,toast,busy), "Registrar contacto enviado" }
                        }}
                    } else if due {
                        {let mutation=action(Command::Followup{channel:channel.read().clone(),variant:data.message.variante.clone(),message:body.read().clone()});let db=db.clone();let settings=settings.clone();rsx!{
                            button { class:"primary full", disabled:*busy.read(), onclick:move |_|run_mutation(db.clone(),settings.clone(),mutation.clone(),revision,toast,busy), "Registrar seguimiento enviado" }
                        }}
                    }
                }
            }

            if contacted || ["respondio", "respuesta_positiva", "reunion", "propuesta"].contains(&p.estado.as_str()) {
                section { class:"simple-detail-section",
                    h3 { "¿Qué pasó?" }
                    p { class:"simple-help", "Elige el hecho real. No necesitas cambiar estados manualmente." }
                    div { class:"simple-outcome-grid",
                        if contacted {
                            {let m=action(Command::Response{response:"pregunta".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Respondió"}}}
                            {let m=action(Command::Response{response:"interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Está interesado"}}}
                            {let m=action(Command::Response{response:"redirigido".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Me redirigió"}}}
                            {let m=action(Command::Response{response:"no_interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"No interesado"}}}
                        }
                        if ["respondio", "respuesta_positiva"].contains(&p.estado.as_str()) {
                            {let m=action(Command::Response{response:"interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Está interesado"}}}
                            {let m=action(Command::State{state:"reunion".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Agendamos reunión"}}}
                            {let m=action(Command::State{state:"propuesta".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Envié propuesta"}}}
                            {let m=action(Command::State{state:"no_interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"No interesado"}}}
                        }
                        if p.estado=="reunion" {
                            {let m=action(Command::State{state:"propuesta".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Envié propuesta"}}}
                            {let m=action(Command::State{state:"no_interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"No avanzó"}}}
                        }
                        if p.estado=="propuesta" {
                            {let m=action(Command::State{state:"cliente".into()});let db=db.clone();let settings=settings.clone();rsx!{button{class:"primary",disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Se convirtió en cliente"}}}
                            {let m=action(Command::State{state:"no_interesado".into()});let db=db.clone();let settings=settings.clone();rsx!{button{disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"No avanzó"}}}
                        }
                    }
                    if contacted {
                        {let m=action(Command::Snooze{days:3});let db=db.clone();let settings=settings.clone();rsx!{button{class:"simple-link",disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Posponer 3 días"}}}
                    }
                }
            }

            section { class:"simple-detail-section",
                h3 { "Actividad" }
                if data.history.is_empty() {
                    p { class:"simple-help", "Aún no hay actividad registrada." }
                } else {
                    div { class:"simple-timeline",
                        for event in data.history.iter().rev().take(6) {
                            div { class:"simple-timeline-item",
                                span {}
                                div { strong { "{event_label(&event.interaction_type)}" } p { "{event.created_at}" } }
                            }
                        }
                        if contacted && !p.proximo_seguimiento.is_empty() {
                            div { class:"simple-timeline-item upcoming",
                                span {}
                                div { strong { "Próximo seguimiento" } p { "{pretty_date(&p.proximo_seguimiento)}" } }
                            }
                        }
                    }
                }
            }

            details { class:"simple-advanced-state",
                summary { "Corrección avanzada" }
                p { "Úsalo solo si necesitas corregir un registro manualmente. El flujo normal actualiza el estado por ti." }
                select { value:"{advanced_state}", onchange:move |e|advanced_state.set(e.value()),
                    for (value,label) in [
                        ("nuevo","Sin revisar"),("revisado","Listo para contactar"),("contactado","Esperando respuesta"),("respondio","Respondió"),("respuesta_positiva","Interesado"),("reunion","Reunión"),("propuesta","Propuesta enviada"),("cliente","Cliente"),("no_interesado","No interesado"),("sin_respuesta","Sin respuesta"),("descartado","Descartado")
                    ] { option { value, "{label}" } }
                }
                {let m=action(Command::State{state:advanced_state.read().clone()});let db=db.clone();let settings=settings.clone();rsx!{button{class:"secondary",disabled:*busy.read(),onclick:move |_|run_mutation(db.clone(),settings.clone(),m.clone(),revision,toast,busy),"Aplicar corrección"}}}
            }
        }
    }
}
