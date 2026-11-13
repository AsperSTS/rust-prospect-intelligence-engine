//! iPhone navigation over the same panels, commands and queries as Desktop.
use super::*;
use crate::view_cache::CachedRead;
use prospect_core::Summary;

#[derive(Clone, Copy)]
pub(super) struct MobileData {
    pub ui: UiState,
    pub page: CachedRead<Page>,
    pub summary: CachedRead<Summary>,
    pub dimensions: CachedRead<(Vec<String>, Vec<String>)>,
    pub job_state: Signal<JobState>,
}

fn screen_title(screen: Screen) -> &'static str {
    match screen {
        Screen::Dashboard => "Resumen",
        Screen::Updates => "Fuentes",
        Screen::Conversations => "Chats",
        _ => screen.label(),
    }
}

/// One navigation stack inside the selected tab. Panel state stays mounted on pushes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub(super) enum Destination {
    #[default]
    Root,
    Filters,
    BaseTools,
    TodayActions,
    Evidence,
    History,
    Message,
    Progress,
    Notes,
    TerritoryMap,
    TerritoryProspect,
    TerritoryComparison,
    Opportunity,
    Funnel,
    Revenue,
    Segments,
    Day,
    Results,
    DataStatus,
    IndicatorQuery,
    Denue,
    Coverage,
    OtherSources,
    Identity,
    Appearance,
    Goals,
    DataPreferences,
    Credentials,
    Backup,
    Templates,
}
impl Destination {
    pub fn title(self) -> &'static str {
        match self {
            Self::Root => "",
            Self::Filters => "Filtros",
            Self::BaseTools => "Gestionar base",
            Self::TodayActions => "Jornada",
            Self::Evidence => "Evidencia",
            Self::History => "Historial",
            Self::Message => "Mensaje",
            Self::Progress => "Avance",
            Self::Notes => "Notas",
            Self::TerritoryMap => "Mapa",
            Self::TerritoryProspect => "Ficha",
            Self::TerritoryComparison => "Municipios",
            Self::Opportunity => "Oportunidad",
            Self::Funnel => "Embudo",
            Self::Revenue => "Ingresos",
            Self::Segments => "Segmentos",
            Self::Day => "Jornada",
            Self::Results => "Resultados",
            Self::DataStatus => "Estado de datos",
            Self::IndicatorQuery => "Consulta INEGI",
            Self::Denue => "DENUE",
            Self::Coverage => "Cobertura",
            Self::OtherSources => "Otras fuentes",
            Self::Identity => "Perfil y firma",
            Self::Appearance => "Apariencia",
            Self::Goals => "Metas diarias",
            Self::DataPreferences => "Datos y límites",
            Self::Credentials => "Credenciales",
            Self::Backup => "Respaldos",
            Self::Templates => "Plantillas",
        }
    }
    fn icon(self) -> &'static str {
        match self {
            Self::TerritoryMap | Self::TerritoryComparison => "map",
            Self::Filters => "filter",
            Self::Evidence | Self::History => "base",
            Self::Message | Self::Identity | Self::Templates => "chat",
            Self::Progress => "followups",
            Self::Notes => "notes",
            Self::Funnel | Self::Revenue | Self::Segments | Self::Results => "metrics",
            Self::TodayActions | Self::Day | Self::Goals => "today",
            Self::IndicatorQuery => "indicators",
            Self::Denue | Self::Coverage | Self::OtherSources | Self::DataStatus => "refresh",
            _ => "settings",
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Navigation {
    stack: Signal<Vec<Destination>>,
    revision: Signal<u64>,
    editing: Signal<bool>,
    main: Signal<Option<std::rc::Rc<MountedData>>>,
    offsets: Signal<std::collections::HashMap<Destination, f64>>,
}
impl Navigation {
    pub fn new() -> Self {
        Self {
            stack: Signal::new(Vec::new()),
            revision: Signal::new(0),
            editing: Signal::new(false),
            main: Signal::new(None),
            offsets: Signal::new(std::collections::HashMap::new()),
        }
    }
    pub fn page(self) -> Destination {
        self.stack.read().last().copied().unwrap_or_default()
    }
    fn push_now(mut self, page: Destination) {
        self.editing.set(false);
        self.offsets.write().remove(&page);
        self.stack.write().push(page);
        self.revision += 1;
    }
    pub fn push(mut self, page: Destination) {
        let origin = self.page();
        if origin == page {
            return;
        }
        let element = self.main.peek().clone();
        let revision = *self.revision.peek();
        if let Some(element) = element {
            spawn(async move {
                let offset = element.get_scroll_offset().await.ok();

                // A tab change or a second tap invalidates an outstanding read.
                if *self.revision.peek() != revision {
                    return;
                }
                if let Some(offset) = offset {
                    self.offsets.write().insert(origin, offset.y);
                }
                self.push_now(page);
            });
        } else {
            self.push_now(page);
        }
    }
    pub fn pop(mut self) {
        self.editing.set(false);
        if !self.stack.peek().is_empty() {
            self.stack.write().pop();
            self.revision += 1;
        }
    }
    pub fn reset(mut self) {
        self.editing.set(false);
        if !self.stack.peek().is_empty() {
            self.stack.set(Vec::new());
        }
        self.offsets.write().clear();
        self.revision += 1;
    }
    async fn restore(self) {
        let element = self.main.peek().clone();
        let y = self
            .offsets
            .peek()
            .get(&self.page())
            .copied()
            .unwrap_or(0.0);
        if let Some(element) = element {
            let _ = element.set_focus(true).await;
            let _ = element
                .scroll(
                    dioxus::html::geometry::PixelsVector2D::new(0.0, y),
                    ScrollBehavior::Instant,
                )
                .await;
        }
    }
    fn back_title(self, root: &'static str) -> &'static str {
        self.stack
            .read()
            .iter()
            .rev()
            .nth(1)
            .map(|p| p.title())
            .unwrap_or(root)
    }
}
pub(super) fn navigation() -> Option<Navigation> {
    let runtime = dioxus::core::Runtime::try_current()?;
    let scope = runtime.try_current_scope_id().unwrap_or(ScopeId::APP);
    let services = dioxus::core::consume_context_from_scope::<Services>(scope)?;
    if services.layout != Layout::Ios {
        return None;
    }
    dioxus::core::consume_context_from_scope::<Navigation>(scope)
}

pub(super) fn set_editing(editing: bool) {
    if let Some(mut nav) = navigation() {
        nav.editing.set(editing);
    }
}
pub(super) fn destination() -> Destination {
    navigation().map(|n| n.page()).unwrap_or_default()
}
pub(super) fn push(page: Destination) {
    if let Some(nav) = navigation() {
        nav.push(page);
    }
}
pub(super) fn reset() {
    if let Some(nav) = navigation() {
        nav.reset();
    }
}
pub(super) fn back() {
    if let Some(nav) = navigation() {
        nav.pop();
    }
}

#[component]
pub(super) fn PageMenu(pages: Vec<Destination>) -> Element {
    rsx! { div { class:"ios-group ios-menu ios-section-menu", for page in pages {
        button { class:"ios-menu-row", onclick:move |_|push(page), Icon { name:page.icon() } span { strong { "{page.title()}" } } Icon { name:"chevron" } }
    } } }
}

#[component]
pub(super) fn QuickActions(pages: Vec<Destination>) -> Element {
    rsx! { div { class:"ios-quick-actions", for page in pages { button { onclick:move |_|push(page), Icon { name:page.icon() } span { "{page.title()}" } } } } }
}

#[component]
pub(super) fn MobileShell() -> Element {
    let mut navigation = use_context::<Navigation>();
    #[cfg(target_os = "ios")]
    let mut viewport = crate::ios::use_viewport_style(navigation.editing);
    #[cfg(not(target_os = "ios"))]
    let viewport = String::new();
    let MobileData {
        mut ui,
        page,
        summary,
        dimensions,
        mut job_state,
    } = use_context::<MobileData>();
    #[cfg(target_os = "ios")]
    use_effect(move || crate::ios::set_appearance(&ui.settings.read().theme));
    let mut more = use_signal(|| false);
    let destination = navigation.page();
    let mut toast_epoch = use_signal(|| 0u64);
    use_effect(move || {
        let message = ui.toast.read().clone();
        toast_epoch += 1;
        let epoch = *toast_epoch.peek();
        if !message.is_empty() {
            spawn(async move {
                tokio::time::sleep(Duration::from_secs(4)).await;
                if *toast_epoch.peek() == epoch {
                    ui.toast.set(String::new());
                }
            });
        }
    });
    let current = *ui.screen.read();
    let q = ui.query.read().clone();
    let settings = ui.settings.read().clone();
    let profile = q.profile;
    let totals = summary.value().unwrap_or_default();
    let list_screen = matches!(
        current,
        Screen::Today | Screen::Followups | Screen::Conversations | Screen::Base
    );
    let detail = list_screen && !ui.selected.read().is_empty() && !*more.read();
    let state = job_state.read().clone();
    let running = state.status == "running";
    let title = if detail {
        "Ficha"
    } else if *more.read() {
        "Más"
    } else {
        screen_title(current)
    };
    let root_title = title;
    let child_page = destination != Destination::Root;
    let title = if child_page {
        destination.title()
    } else {
        title
    };
    let back_title = navigation.back_title(if current == Screen::Opportunities {
        "Cartera"
    } else {
        root_title
    });
    let transition = *navigation.revision.read() % 2;
    let tabs = [
        (Screen::Today, "Hoy", "today", totals.today),
        (Screen::Followups, "Seguimientos", "followups", totals.due),
        (Screen::Conversations, "Chats", "chat", totals.conversations),
        (Screen::Base, "Base", "base", 0),
        (Screen::Dashboard, "Más", "more", 0),
    ];
    rsx! {
        style { {include_str!("../style.css")} }
        style { {include_str!("mobile.css")} }
        div { class:"app ios-app", "data-theme":settings.theme, style:"{viewport}",
            onmounted:move |_| {
                #[cfg(target_os="ios")]
                { crate::ios::configure_viewport(); viewport.set(crate::ios::viewport_style(*navigation.editing.peek())); }
            },
            header { class:"ios-navbar",
                if child_page { button { class:"ios-back", onclick:move |_|navigation.pop(), Icon { name:"back" } span {class:"ios-back-label", "{back_title}"} } }
                else if detail { button { class:"ios-back", onclick:move |_|ui.selected.set(String::new()), Icon { name:"back" } span {class:"ios-back-label", "{screen_title(current)}"} } }
                else if !list_screen && !*more.read() { button { class:"ios-back", onclick:move |_|{navigation.reset();more.set(true);}, Icon { name:"back" } span {class:"ios-back-label", "Más"} } }
                else { div { class:"ios-navbar-profile", ProfilePicker {} } }
                if child_page || detail || (!list_screen && !*more.read()) { h1 { class:"ios-nav-title", "{title}" } }
                if child_page { if destination==Destination::Filters { button { class:"ios-icon-button", aria_label:"Aplicar filtros", onclick:move |_|navigation.pop(), "Listo" } } else { div { class:"ios-navbar-trailing" } } }
                else if !detail && !list_screen && !*more.read() { div { class:"ios-navbar-trailing" } }
                else { button { class:"ios-icon-button", aria_label:"Ajustes", onclick:move |_|{navigate(Screen::Settings,ui);more.set(false);}, Icon { name:"settings" } } }
            }
            main { class:"ios-content", id:"main-content", tabindex:"-1", aria_label:title, onmounted:move |event|navigation.main.set(Some(event.data())),
                div { class:"ios-page-anchor", key:"{current.label()}:{detail}:{more}:{ui.selected}:{profile.id()}:{destination:?}", onmounted:move |_|async move { navigation.restore().await; } }
                div { class:"ios-page", key:"page:{current.label()}:{detail}:{more}:{ui.selected}:{profile.id()}", "data-navigation":if detail || child_page {"push"}else{"tab"}, "data-transition":"{transition}",
                if !child_page && !detail && (list_screen || *more.read()) { div { class:"ios-heading", h1 { "{title}" } } }
                if !ui.toast.read().is_empty() { div { class:"toast", role:"status", span { "{ui.toast}" } button { class:"ios-icon-button", aria_label:"Cerrar aviso", onclick:move |_|ui.toast.set(String::new()), "×" } } }
                if !state.id.is_empty() { div { class:"job-strip", role:"status", strong { "{state.kind}" } p { "{state.message}" }
                    if running { button { class:"button secondary", onclick:move |_|{if let Some(j)=ui.job.peek().as_ref(){j.token.cancel();}}, "Cancelar" } }
                    else { button { class:"link-button", onclick:move |_|{ui.job.set(None);job_state.set(JobState::default());}, "Cerrar" } }
                } }
                if *more.read() { MoreMenu { more } }
                else if detail { ProspectDetail { key:"{ui.selected}:{profile.id()}", id:ui.selected.read().clone(), profile } }
                else if list_screen {
                    if current==Screen::Today && matches!(destination,Destination::Root|Destination::TodayActions) {
                        div { class:"ios-goals", DailyGoal { label:"Revisados", done:totals.researched_today, goal:settings.daily_research } DailyGoal { label:"Contactos", done:totals.contacted_today, goal:settings.daily_contacts } DailyGoal { label:"Seguimientos", done:totals.followed_today, goal:settings.daily_followups } }
                    }
                    if destination==Destination::Root {
                    div { class:"ios-search", Icon { name:"search" } SearchBox { query:ui.query, compact:true } }
                    div { class:"ios-list-tools", button { class:"ios-back", onclick:move |_|push(Destination::Filters), Icon { name:"filter" } "Filtros" }
                        if matches!(current,Screen::Today|Screen::Base) { button { class:"ios-icon-button", aria_label:"Acciones", onclick:move |_|push(if current==Screen::Base {Destination::BaseTools}else{Destination::TodayActions}), Icon { name:"more" } } }
                    }
                    }
                    section { class:"panel", hidden:destination!=Destination::Filters,
                        div { class:"form-body",
                            label { "Giro" select { value:q.giro.clone(), onchange:move |e|{ui.query.write().giro=e.value();ui.query.write().page=0;}, option { value:"", "Todos los giros" } for giro in dimensions.value().unwrap_or_default().0 { option { value:giro.clone(), "{giro}" } } } }
                            label { "Municipio" select { value:q.municipio.clone(), onchange:move |e|{ui.query.write().municipio=e.value();ui.query.write().page=0;}, option { value:"", "Todos los municipios" } for city in dimensions.value().unwrap_or_default().1 { option { value:city.clone(), "{city}" } } } }
                            label { "Entidad" select { value:q.entidad.clone(), onchange:move |e|{let mut query=ui.query.write();query.entidad=e.value();query.municipio.clear();query.giro.clear();query.page=0;}, option { value:"", "Todas las entidades" } for (_,name) in prospect_core::config::ENTITIES { option { value:*name, "{name}" } } } }
                            label { "Estado comercial" select { value:q.state.clone(), onchange:move |e|{ui.query.write().state=e.value();ui.query.write().page=0;}, option { value:"", "Todos" } for state in prospect_core::config::STATES { option { value:*state, "{state}" } } } }
                            label { "Fit mínimo · {q.fit_min}" input { r#type:"range", min:"0", max:"100", value:"{q.fit_min}", oninput:move |e|{ui.query.write().fit_min=e.value().parse().unwrap_or(0);ui.query.write().page=0;} } }
                            label { class:"check-label", input { r#type:"checkbox", checked:q.contactable, onchange:move |e|{ui.query.write().contactable=e.checked();ui.query.write().page=0;} } "Sólo con contacto" }
                            button { class:"button secondary", onclick:move |_|ui.query.set(Query{profile,view:current.view(),size:40,contactable:current==Screen::Today,..Default::default()}), "Restablecer filtros" }
                            SavedFilters { query:ui.query }
                        }
                    }
                    if destination==Destination::Root { if let Some(status)=page.status("los establecimientos") { {status} }
                    else {
                        ProspectList { data:page.value().unwrap_or_default() }
                        if page.value().is_some_and(|p|p.total>p.size) { Pagination { data:page.value().unwrap_or_default(), query:ui.query } }
                    }
                    }
                    if current==Screen::Base && destination==Destination::BaseTools {
                        section { class:"panel", div { class:"form-body",
                            h2 { "Tu base de trabajo" } p { class:"muted", "{number(totals.total)} establecimientos guardados en este dispositivo." }
                            button { class:"button primary full", onclick:move |_|navigate(Screen::Updates,ui), "Gestionar datos" }
                            button { class:"button secondary full", disabled:running, onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Analyze,ui,s.db,String::new());}, "Recalcular prioridades" }
                            button { class:"button secondary full", disabled:running, onclick:move |_|{let s=consume_context::<Services>();job_start(JobKind::Enrich,ui,s.db,String::new());}, "Revisar sitios · hasta 100" }
                            div { class:"action-row", ExportButtons { query:q.clone() } }
                        } }
                    }
                    if current==Screen::Today && destination==Destination::TodayActions { button { class:"button secondary full", disabled:*ui.busy.read(), onclick:move |_|{let s=consume_context::<Services>();mutation(Command::Publication{},String::new(),None,profile,ui,s.db);}, "Registrar publicación · {totals.publications_today}" } }
                } else { match current {
                    Screen::Dashboard=>rsx! { DashboardPanel { key:"{profile.id()}", profile } },
                    Screen::Metrics=>rsx! { MetricsPanel { key:"{profile.id()}", profile } },
                    Screen::Territories=>rsx! { TerritoriesPanel { key:"{profile.id()}", profile, screen:ui.screen, query:ui.query, selected:ui.selected } },
                    Screen::Opportunities=>rsx! { OpportunitiesPanel { key:"{profile.id()}", profile } },
                    Screen::Indicators=>rsx! { IndicatorsPanel {} },
                    Screen::Updates=>rsx! { UpdatesPanel { key:"{profile.id()}", profile } },
                    Screen::Settings=>rsx! { SettingsPanel {} },
                    _=>rsx! {},
                } }
                }
            }
            nav { class:"ios-tabbar", aria_label:"Navegación principal",
                for (target,label,icon,count) in tabs {
                    { let active=if target==Screen::Dashboard { !list_screen || *more.read() } else { current==target && !*more.read() };
                        rsx! { button { class:if active { "ios-tab active" } else { "ios-tab" }, aria_current:if active { "page" } else { "false" },
                            onclick:move |_|{navigate(target,ui);more.set(target==Screen::Dashboard);},
                            span { class:"ios-tab-icon", Icon { name:icon } if count>0 { span { class:"ios-tab-badge", if count>99 { "99+" } else { "{count}" } } } }
                            span { "{label}" }
                        } }
                    }
                }
            }
        }
    }
}

#[component]
fn ProfilePicker() -> Element {
    let mut ui = use_context::<UiState>();
    let profile = ui.query.read().profile;
    rsx! { label { class:"ios-profile",
        select { aria_label:format!("Oferta: {}",profile.label()), value:profile.id(),
            onchange:move |event| {
                if let Ok(profile)=Profile::parse(&event.value()) {
                    ui.query.write().profile=profile;ui.query.write().page=0;ui.selected.set(String::new());
                    let mut settings=ui.settings.peek().clone();settings.profile=profile.id().into();
                    let paths=consume_context::<Services>().db.paths;
                    match settings.save(&paths) { Ok(())=>ui.settings.set(settings), Err(e)=>ui.toast.set(e.to_string()) }
                }
            },
            option { value:"automatizacion", "Sistemas" }
            option { value:"web_marketing", "Web y marketing" }
        }
    } }
}

#[component]
fn ProspectList(data: Page) -> Element {
    let mut ui = use_context::<UiState>();
    if data.items.is_empty() {
        return rsx! { section { class:"ios-empty panel", Icon { name:"base" } h2 { "Sin pendientes" } p { class:"muted", "Prueba otros filtros o añade negocios a tu base." } button { class:"button secondary", onclick:move |_|navigate(Screen::Updates,ui), "Gestionar datos" } } };
    }
    rsx! { section { class:"ios-prospect-list", aria_label:"Establecimientos",
        div { class:"ios-section-heading", h2 { "{number(data.total)} negocios" } span { "Prioridad" } }
        div { class:"ios-group", for p in data.items { { let id=p.id.clone();rsx! {
            button { class:"ios-prospect-row", onclick:move |_|ui.selected.set(id.clone()),
                span { class:"ios-prospect-copy", strong { "{p.nombre}" } span { class:"muted", "{p.municipio} · {p.giro}" } span { class:"ios-row-meta", span { class:"badge", "{p.estado}" } span { "Fit {p.fit_score}" } } }
                span { class:"ios-priority", strong { "{p.priority_score}" } }
                Icon { name:"chevron" }
            }
        } } } }
    } }
}

#[component]
fn MoreMenu(mut more: Signal<bool>) -> Element {
    let ui = use_context::<UiState>();
    let menu = [
        (
            Screen::Dashboard,
            "Resumen",
            "Embudo y próximas acciones",
            "dashboard",
        ),
        (
            Screen::Opportunities,
            "Oportunidades",
            "Propuestas, acuerdos y cobros",
            "briefcase",
        ),
        (
            Screen::Metrics,
            "Métricas",
            "Resultados por perfil de oferta",
            "metrics",
        ),
        (
            Screen::Territories,
            "Territorios",
            "Mapa y comparación de municipios",
            "map",
        ),
        (
            Screen::Indicators,
            "Indicadores",
            "Contexto INEGI y Banxico",
            "indicators",
        ),
        (
            Screen::Updates,
            "Fuentes",
            "Fuentes públicas y lotes controlados",
            "refresh",
        ),
        (
            Screen::Settings,
            "Ajustes",
            "Apariencia, metas y credenciales",
            "settings",
        ),
    ];
    rsx! { div { class:"ios-group ios-menu", for (target,title,subtitle,icon) in menu {
        button { class:"ios-menu-row", title:subtitle, onclick:move |_|{navigate(target,ui);more.set(false);}, Icon { name:icon } span { strong { "{title}" } } Icon { name:"chevron" } }
    } } p { class:"ios-footnote", "Personal · datos locales" } }
}

#[component]
pub(super) fn Icon(name: &'static str) -> Element {
    let path = match name {
        "today" => "M8 2v4m8-4v4M3 10h18M5 4h14a2 2 0 0 1 2 2v14H3V6a2 2 0 0 1 2-2m3 10h3v3H8z",
        "followups" => "M12 8v5l3 2M3 12a9 9 0 1 0 3-6M3 3v5h5",
        "chat" => "M21 11a8 8 0 0 1-8 8H8l-5 3V11a8 8 0 0 1 8-8h2a8 8 0 0 1 8 8M7 10h10M7 14h6",
        "base" => "M3 7h18v14H3zM3 7l3-4h12l3 4M9 11h6",
        "notes" => "M5 3h14v18H5zM8 8h8M8 12h8M8 16h5",
        "more" => "M5 11v2m7-2v2m7-2v2",
        "back" => "m15 5-7 7 7 7",
        "chevron" => "m9 5 7 7-7 7",
        "search" => "M21 21l-5-5m2-6a8 8 0 1 1-16 0 8 8 0 0 1 16 0",
        "filter" => "M4 6h16M7 12h10M10 18h4",
        "settings" => {
            "m10 3-1 3-3 1-3 3v4l3 3 3 1 1 3h4l1-3 3-1 3-3v-4l-3-3-3-1-1-3zM15 12a3 3 0 1 1-6 0 3 3 0 0 1 6 0"
        }
        "dashboard" => "M3 3h7v7H3zM14 3h7v7h-7zM3 14h7v7H3zM14 14h7v7h-7z",
        "briefcase" => "M8 6V3h8v3M3 6h18v15H3zM3 11l9 4 9-4M10 12h4",
        "metrics" => "M3 3v18h18M7 16v-5m5 5V7m5 9V4",
        "map" => "m3 5 6-2 6 2 6-2v16l-6 2-6-2-6 2zM9 3v16m6-14v16",
        "indicators" => "m3 17 6-6 4 3 8-10m-6 0h6v6",
        "refresh" => "M3 11a9 9 0 0 1 16-6l2 3M21 3v5h-5M21 13a9 9 0 0 1-16 6l-2-3M3 21v-5h5",
        _ => "M12 3v18M3 12h18",
    };
    rsx! { svg { class:"ios-icon", view_box:"0 0 24 24", fill:"none", stroke:"currentColor", stroke_width:"1.8", stroke_linecap:"round", stroke_linejoin:"round", "aria-hidden":"true", path { d:path } } }
}

#[component]
pub(super) fn TemplateEditor(onclose: EventHandler) -> Element {
    use prospect_core::messages::Templates;
    let paths = use_context::<Services>().db.paths;
    let mut ui = use_context::<UiState>();
    let initial = use_hook(|| Templates::source(&paths.home).map_err(|e| e.to_string()));
    let mut text = use_signal(|| initial.as_ref().cloned().unwrap_or_default());
    let mut error = use_signal(|| initial.as_ref().err().cloned().unwrap_or_default());
    rsx! { section { class:"panel",
        div { class:"panel-heading", hidden: navigation().is_some(), h2 { "Plantillas de mensajes" } button { class:"link-button", onclick:move |_|onclose.call(()), "Volver a Ajustes" } }
        div { class:"form-body",
            p { class:"muted", "Edita tus plantillas TOML. Se valida el formato y las claves antes de guardar." }
            label { "Contenido de las plantillas" TextEntry { multiline:true, rows:20, initial:initial.as_ref().cloned().unwrap_or_default(), oninput:move |e:FormEvent|text.set(e.value()) } }
            if !error.read().is_empty() { p { class:"error", role:"alert", "{error}" } }
            button { class:"button primary full", onclick:move |_|match Templates::save(&paths.home,&text.read()) {
                Ok(())=>{error.set(String::new());ui.revision+=1;ui.toast.set("Plantillas guardadas".into());onclose.call(());},
                Err(e)=>error.set(e.to_string()),
            }, "Guardar plantillas" }
        }
    } }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn settle(dom: &mut VirtualDom, expected: &str) -> String {
        // Drive pending effects as the native event loop does, even for cached screens.
        for _ in 0..4 {
            let _ = tokio::time::timeout(Duration::from_millis(2), dom.wait_for_work()).await;
            dom.render_immediate_to_vec();
        }
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                dom.render_immediate_to_vec();
                let html = dioxus_ssr::render(dom);
                if html.contains(expected) {
                    return html;
                }
                dom.wait_for_work().await;
            }
        })
        .await
        .unwrap_or_else(|_| {
            let html = dioxus_ssr::render(dom);
            let main = html.split("<main").nth(1).unwrap_or(&html);
            panic!(
                "Expected {expected}; got {}",
                main.chars().take(2400).collect::<String>()
            )
        })
    }

    #[tokio::test]
    async fn mobile_navigation_reuses_desktop_queries_without_starting_source_jobs() {
        let temp = tempfile::tempdir().unwrap();
        let db = Db::open(prospect_core::AppPaths::at(temp.path()).unwrap()).unwrap();
        let settings = Settings::default();
        db.import_records(vec![Prospect {
            id: "ios-test".into(),
            nombre: "Establecimiento de prueba aislada".into(),
            clase_id: "336310".into(),
            clase_actividad: "Autopartes".into(),
            municipio: "Puebla".into(),
            entidad: "Puebla".into(),
            telefono: "2221234567".into(),
            estrato: "31 a 50 personas".into(),
            tipo_establecimiento: "fijo".into(),
            ..Default::default()
        }])
        .await
        .unwrap();
        db.analyze(settings.clone(), JobControl::new("test"))
            .await
            .unwrap();
        let mut dom = VirtualDom::new(App).with_root_context(Services {
            db: db.clone(),
            settings,
            layout: Layout::Ios,
        });
        dom.rebuild_in_place();
        let ui = dom.in_runtime(|| {
            dioxus::core::consume_context_from_scope::<UiState>(ScopeId::APP).unwrap()
        });
        let html = settle(&mut dom, "Establecimiento de prueba aislada").await;
        assert!(html.contains("ios-tabbar"));
        assert!(!html.contains("class=\"sidebar\""));
        assert!(
            dom.in_runtime(|| ui.selected.peek().is_empty()),
            "mobile must not auto-open the first record"
        );
        for (screen, expected) in [
            (Screen::Base, "Establecimiento de prueba aislada"),
            (Screen::Dashboard, "Una jornada pequeña, concreta"),
            (Screen::Territories, "Dónde concentrar tu jornada"),
            (Screen::Followups, "Sin pendientes"),
            (Screen::Conversations, "Sin pendientes"),
            (Screen::Opportunities, "Nueva oportunidad"),
            (Screen::Metrics, "Embudo comercial"),
            (Screen::Indicators, "Sin indicadores guardados"),
            (Screen::Settings, "Perfil y firma"),
            (Screen::Updates, "DENUE"),
        ] {
            dom.in_runtime(|| navigate(screen, ui));
            let html = settle(&mut dom, expected).await;
            assert!(html.contains("ios-tabbar"));
            assert!(
                dom.in_runtime(|| ui.job.peek().is_none()),
                "navigation cannot fetch public sources"
            );
        }
        for profile in [Profile::WebMarketing, Profile::Automatizacion] {
            dom.in_runtime(|| {
                let mut ui = ui;
                ui.query.write().profile = profile;
                navigate(Screen::Base, ui);
            });
            settle(&mut dom, "Establecimiento de prueba aislada").await;
            assert_eq!(dom.in_runtime(|| ui.query.peek().profile), profile);
        }
        let nav = dom.in_runtime(|| {
            dioxus::core::consume_context_from_scope::<Navigation>(ScopeId::APP).unwrap()
        });
        dom.in_runtime(|| navigate(Screen::Base, ui));
        settle(&mut dom, "Establecimiento de prueba aislada").await;
        dom.in_runtime(|| nav.push(Destination::Filters));
        settle(&mut dom, "Aplicar filtros").await;
        dom.in_runtime(|| {
            let mut ui = ui;
            ui.query.write().fit_min = 20;
        });
        for _ in 0..4 {
            tokio::task::yield_now().await;
            dom.render_immediate_to_vec();
        }
        assert_eq!(
            dom.in_runtime(|| nav.page()),
            Destination::Filters,
            "editing filters must preserve the filter page"
        );
        dom.in_runtime(|| {
            set_editing(true);
            nav.pop();
        });
        assert!(
            !dom.in_runtime(|| *nav.editing.peek()),
            "back must release the keyboard reservation"
        );
        assert_eq!(
            dom.in_runtime(|| ui.query.peek().fit_min),
            20,
            "back preserves filters"
        );
        for (screen, pages) in [
            (Screen::Territories, vec![Destination::TerritoryComparison]),
            (Screen::Opportunities, vec![Destination::Opportunity]),
            (
                Screen::Metrics,
                vec![
                    Destination::Funnel,
                    Destination::Revenue,
                    Destination::Segments,
                ],
            ),
            (
                Screen::Dashboard,
                vec![
                    Destination::Day,
                    Destination::Results,
                    Destination::DataStatus,
                ],
            ),
            (
                Screen::Updates,
                vec![
                    Destination::Denue,
                    Destination::Coverage,
                    Destination::OtherSources,
                ],
            ),
            (Screen::Indicators, vec![Destination::IndicatorQuery]),
            (
                Screen::Settings,
                vec![
                    Destination::Identity,
                    Destination::Appearance,
                    Destination::Goals,
                    Destination::DataPreferences,
                    Destination::Credentials,
                    Destination::Backup,
                    Destination::Templates,
                ],
            ),
        ] {
            dom.in_runtime(|| navigate(screen, ui));
            settle(&mut dom, screen_title(screen)).await;
            for destination in pages {
                dom.in_runtime(|| nav.push(destination));
                let html = settle(&mut dom, destination.title()).await;
                assert!(html.contains("data-navigation=\"push\""));
                assert_eq!(dom.in_runtime(|| nav.page()), destination);
                assert!(dom.in_runtime(|| ui.job.peek().is_none()));
                dom.in_runtime(|| nav.pop());
                assert_eq!(dom.in_runtime(|| nav.page()), Destination::Root);
            }
        }
        dom.in_runtime(|| navigate(Screen::Territories, ui));
        let html = settle(&mut dom, "Elegir negocio").await;
        assert!(
            !html.contains("class=\"street-map\""),
            "the map must not mount or fetch tiles below the territory list"
        );
        dom.in_runtime(|| {
            navigate(Screen::Base, ui);
        });
        settle(&mut dom, "Establecimiento de prueba aislada").await;
        dom.in_runtime(|| {
            let mut ui = ui;
            ui.selected.set("ios-test".into());
        });
        settle(&mut dom, "Ficha").await;
        for destination in [
            Destination::Message,
            Destination::Evidence,
            Destination::History,
            Destination::Progress,
            Destination::Notes,
        ] {
            dom.in_runtime(|| nav.push(destination));
            settle(&mut dom, destination.title()).await;
            assert_eq!(dom.in_runtime(|| ui.selected.peek().clone()), "ios-test");
            dom.in_runtime(|| nav.pop());
        }
        assert!(db.history("ios-test".into()).await.unwrap().is_empty());
    }
}
