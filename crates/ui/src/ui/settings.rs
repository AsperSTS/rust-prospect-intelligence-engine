use super::*;
use crate::onboarding::OnboardingControl;
use prospect_core::{config::AutoUpdatePeriod, messages::Templates};
use std::collections::BTreeMap;

mod scope;
mod templates;
mod tokens;

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
    let snapshot = draft.read().clone();
    let source = tpl.read().clone();
    let datos = Templates::parse_toml(&source)
        .map(|t| t.datos)
        .unwrap_or_default();
    let used = |key: &str| source.contains(&format!("{{{key}}}"));
    let uses_whatsapp = used("whatsapp");
    let uses_credential = used("credencial");
    let error = use_signal(String::new);
    let mut hidden = Vec::new();
    if !uses_whatsapp {
        hidden.push("Tu WhatsApp");
    }
    if !uses_credential {
        hidden.push("Tu credencial");
    }

    rsx! {
        section { class: "panel st-profile",
            div { class: "panel-heading", h2 { "Perfil" } }
            div { class: "form-body",
                div { class: "st-block",
                    div { class: "st-block-head",
                        strong { "Tus datos en plantillas" }
                        span { class: "muted small", "Se insertan solos en cada mensaje" }
                    }
                    label {
                        "Tu nombre"
                        TextEntry {
                            initial: snapshot.owner_name.clone(),
                            oninput: move |e: FormEvent| draft.write().owner_name = e.value(),
                        }
                    }
                    if uses_whatsapp {
                        label {
                            "Tu WhatsApp"
                            TextEntry {
                                initial: snapshot.owner_whatsapp.clone(),
                                placeholder: "10 dígitos",
                                inputmode: "tel",
                                oninput: move |e: FormEvent| draft.write().owner_whatsapp = e.value(),
                            }
                        }
                    }
                    if uses_credential {
                        label {
                            "Tu credencial"
                            TextEntry {
                                multiline: true,
                                rows: 2,
                                initial: snapshot.owner_credential.clone(),
                                placeholder: "Una frase comprobable sobre tu experiencia",
                                oninput: move |e: FormEvent| draft.write().owner_credential = e.value(),
                            }
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
                    if !hidden.is_empty() {
                        p { class: "muted small st-note",
                            "{hidden.join(\" y \")} aparecerán aquí cuando los insertes en una plantilla."
                        }
                    }
                    if !error.read().is_empty() {
                        p { class: "error small", role: "alert", "{error}" }
                    }
                }
                div { class: "st-block",
                    div { class: "st-block-head",
                        strong { "Metas diarias" }
                        span { class: "muted small", "0 desactiva una meta" }
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
                div { class: "st-block",
                    div { class: "st-block-head", strong { "Apariencia" } }
                    Segmented {
                        name: "Apariencia",
                        options: vec![("light".to_owned(), "Claro"), ("dark".to_owned(), "Oscuro")],
                        value: snapshot.theme.clone(),
                        onpick: move |theme: String| draft.write().theme = theme,
                    }
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
        let only_profile =
            normalized(previous, &current.profile) == normalized(current.clone(), &current.profile);
        if only_profile {
            draft.write().profile = current.profile.clone();
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
    let dirty = normalized(draft.read().clone(), &saved.profile)
        != normalized(saved.clone(), &saved.profile)
        || *tpl.read() != *tpl_saved.read();

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
                            span { class: "st-dirty", role: "status", "Cambios sin guardar" }
                        }
                        button {
                            class: "button primary",
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
                                    label { "Nombre del remitente" TextEntry { initial: draft.read().owner_name.clone(), oninput: move |e: FormEvent| draft.write().owner_name = e.value() } }
                                    label { "WhatsApp de tu firma" TextEntry { initial: draft.read().owner_whatsapp.clone(), oninput: move |e: FormEvent| draft.write().owner_whatsapp = e.value() } }
                                }
                                label { "Credencial comprobable" TextEntry { multiline: true, initial: draft.read().owner_credential.clone(), oninput: move |e: FormEvent| draft.write().owner_credential = e.value() } }
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
