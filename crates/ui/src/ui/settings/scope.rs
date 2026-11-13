//! Territory, business size and the state of the local base.
use super::*;

const SIZES: [(&str, &str, &str); 7] = [
    ("1", "0–5", "Micro"),
    ("2", "6–10", "Micro"),
    ("3", "11–30", "Pequeña"),
    ("4", "31–50", "Pequeña"),
    ("5", "51–100", "Mediana"),
    ("6", "101–250", "Mediana"),
    ("7", "251+", "Grande"),
];

fn simple(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' => 'a',
            'é' => 'e',
            'í' => 'i',
            'ó' => 'o',
            'ú' => 'u',
            other => other,
        })
        .collect()
}

#[component]
pub(super) fn ScopeTab(mut draft: Signal<Settings>, mut pane: Signal<usize>) -> Element {
    let db = consume_context::<Services>().db;
    let ui = use_context::<UiState>();
    let onboarding = try_use_context::<OnboardingControl>();
    let profile = Profile::parse(&ui.settings.read().profile).unwrap_or_default();
    let mut update_modal = use_signal(|| false);
    let mut history = use_signal(|| false);
    let mut filter = use_signal(String::new);

    let data_db = db.clone();
    let data = use_resource(move || {
        let _ = *ui.revision.read();
        if let Some(o) = onboarding {
            let _ = *o.revision.read();
        }
        let settings = ui.settings.read().clone();
        let profile = Profile::parse(&settings.profile).unwrap_or_default();
        let db = data_db.clone();
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

    let snapshot = draft.read().clone();
    let saved = ui.settings.read().clone();
    let resolved = data.read().as_ref().and_then(|r| r.as_ref().ok()).cloned();
    let counts = resolved
        .as_ref()
        .map(|d| d.counts.clone())
        .unwrap_or_default();
    let count_of = |code: &str| {
        counts
            .get(prospect_core::config::entity_name(code))
            .copied()
            .unwrap_or(0)
    };
    let running = ui
        .job
        .read()
        .as_ref()
        .is_some_and(|j| j.snapshot().status == "running");
    let total_records: usize = saved.entities.iter().map(|code| count_of(code)).sum();
    let loaded = saved
        .entities
        .iter()
        .filter(|code| count_of(code) > 0)
        .count();
    let needle = simple(filter.read().trim());
    let suggested_missing = prospect_core::config::DESKTOP_EXPANSION
        .iter()
        .any(|(code, _)| !snapshot.entities.iter().any(|e| e == code));
    let showing_history = *history.read();
    let backup_db = db.clone();
    let modal_db = db.clone();

    rsx! {
        div { class: "st-grid st-grid-scope",
            section { class: "panel st-scope",
                div { class: "panel-heading",
                    h2 { "Alcance" }
                    span { class: "muted small st-head-meta",
                        if snapshot.entities.len() == 1 { "1 estado elegido" } else { "{snapshot.entities.len()} estados elegidos" }
                    }
                    super::HelpButton { topic: super::HelpTopic::Scope, label: "Alcance" }
                }
                div { class: "form-body",
                    div { class: "st-block st-block-grow",
                        div { class: "st-block-head",
                            strong { "Estados" }
                            TextEntry {
                                kind: "search",
                                class: "st-filter",
                                label: "Buscar estado",
                                initial: String::new(),
                                placeholder: "Buscar…",
                                oninput: move |e: FormEvent| filter.set(e.value()),
                            }
                            button {
                                class: "link-button small",
                                r#type: "button",
                                disabled: !suggested_missing,
                                title: "Marca Estado de México, Querétaro, Guanajuato, Jalisco y Nuevo León",
                                onclick: move |_| {
                                    draft.write().add_desktop_expansion();
                                },
                                "+ Sugeridos"
                            }
                        }
                        div { class: "st-states", role: "group", aria_label: "Estados de interés",
                            for (code, name) in prospect_core::config::ENTITIES {
                                if needle.is_empty() || simple(name).contains(&needle) {
                                    {
                                        let selected = snapshot.entities.iter().any(|c| c == code);
                                        let records = count_of(code);
                                        let only_one = selected && snapshot.entities.len() == 1;
                                        rsx! {
                                            button {
                                                key: "{code}",
                                                class: if selected { "st-state active" } else { "st-state" },
                                                r#type: "button",
                                                aria_pressed: selected,
                                                disabled: only_one,
                                                title: if only_one { "Debe quedar al menos un estado" } else if records > 0 { "{name} · ya tiene empresas en tu base" } else { "{name}" },
                                                onclick: move |_| {
                                                    let mut d = draft.write();
                                                    if selected {
                                                        d.entities.retain(|c| c != code);
                                                    } else {
                                                        d.entities.push((*code).to_owned());
                                                        d.entities.sort();
                                                    }
                                                },
                                                span { class: "st-state-mark", if selected { {svg_check()} } }
                                                span { class: "st-state-name", "{name}" }
                                                if records > 0 {
                                                    span { class: "st-state-count", "{number(records)}" }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        p { class: "muted small st-note",
                            "Al guardar se descargan los estados nuevos. Desmarcar uno no borra sus empresas."
                        }
                    }
                    div { class: "st-block",
                        div { class: "st-block-head",
                            strong { "Tamaño del negocio" }
                            span { class: "muted small", "Personas que trabajan en el negocio" }
                        }
                        div { class: "st-sizes", role: "group", aria_label: "Tamaño del negocio",
                            for (code, range, kind) in SIZES {
                                {
                                    let selected = snapshot.estratos.iter().any(|c| c == code);
                                    let only_one = selected && snapshot.estratos.len() == 1;
                                    rsx! {
                                        button {
                                            key: "{code}",
                                            class: if selected { "st-size active" } else { "st-size" },
                                            r#type: "button",
                                            aria_pressed: selected,
                                            disabled: only_one,
                                            title: if only_one { "Debe quedar al menos un tamaño" } else { "" },
                                            onclick: move |_| {
                                                let mut d = draft.write();
                                                if selected {
                                                    d.estratos.retain(|c| c != code);
                                                } else {
                                                    d.estratos.push(code.into());
                                                    d.estratos.sort();
                                                }
                                            },
                                            strong { "{range}" }
                                            span { "{kind}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "st-block",
                        div { class: "st-block-head",
                            strong { "Actualización automática" }
                            span { class: "muted small", "Al abrir la app descarga el siguiente lote" }
                        }
                        Segmented {
                            name: "Actualización automática",
                            options: vec![
                                (AutoUpdatePeriod::Disabled.code().to_owned(), "Sólo manual"),
                                (AutoUpdatePeriod::Weekly.code().to_owned(), "Cada semana"),
                                (AutoUpdatePeriod::Monthly.code().to_owned(), "Cada mes"),
                            ],
                            value: snapshot.auto_update_period.code(),
                            onpick: move |code: String| {
                                if let Some(period) = AutoUpdatePeriod::parse(&code) {
                                    draft.write().auto_update_period = period;
                                }
                            },
                        }
                    }
                }
            }
            section { class: "panel st-base",
                div { class: "panel-heading",
                    h2 {
                        if profile == Profile::Automatizacion {
                            "Tu base · Automatización"
                        } else {
                            "Tu base · Web y marketing"
                        }
                    }
                    div { class: "st-head-actions",
                        Segmented {
                            name: "Vista de la base",
                            options: vec![("states".to_owned(), "Estados"), ("history".to_owned(), "Historial")],
                            value: if showing_history { "history" } else { "states" },
                            onpick: move |view: String| history.set(view == "history"),
                        }
                        super::HelpButton { topic: super::HelpTopic::Base, label: "Tu base" }
                    }
                }
                if let Some(status) = resource_status(data, "el estado de tu base") {
                    div { class: "form-body", {status} }
                } else {
                    {
                        let d = resolved.expect("resolved updates");
                        let waiting = d.progress.iter().filter(|p| !p.completed).count();
                        let complete = d.progress.iter().filter(|p| p.completed).count();
                        let mut by_entity = BTreeMap::<String, (usize, usize, usize, String, String)>::new();
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
                        rsx! {
                            div { class: "st-base-body",
                                div { class: "stats-grid st-stats",
                                    StatCard { label: "Empresas", value: number(total_records), note: "En tu base" }
                                    StatCard { label: "Estados con datos", value: format!("{loaded} / {}", saved.entities.len()), note: "De tu selección" }
                                    StatCard { label: "Grupos listos", value: number(complete), note: "Ya consultados" }
                                    StatCard { label: "Por explorar", value: number(waiting), note: "Grupos pendientes" }
                                }
                                div { class: "st-actions",
                                    button {
                                        class: "button primary",
                                        disabled: running || (!d.denue_configured && total_records == 0),
                                        onclick: move |_| update_modal.set(true),
                                        "Actualizar todo"
                                    }
                                    button {
                                        class: "button backup",
                                        disabled: ui.heavy_task.read().is_some(),
                                        onclick: move |_| run_backup(backup_db.clone(), ui),
                                        {svg_backup_shield()}
                                        span { "Crear respaldo" }
                                    }
                                    if !d.denue_configured {
                                        span { class: "st-inline-notice",
                                            "Falta tu token DENUE para descargar."
                                            button {
                                                class: "link-button small",
                                                onclick: move |_| pane.set(2),
                                                "Configurarlo"
                                            }
                                        }
                                    }
                                }
                                if showing_history {
                                    div { class: "st-scroll st-history",
                                        workspace::BatchHistory { key: "bh-{profile.id()}", profile, embedded: true }
                                    }
                                } else {
                                    div { class: "st-scroll st-table",
                                        table {
                                            thead {
                                                tr {
                                                    for h in ["Estado", "En base", "Avance", ""] {
                                                        th { scope: "col", "{h}" }
                                                    }
                                                }
                                            }
                                            tbody {
                                                for (entity, (new, pending, done, date, error)) in by_entity {
                                                    {
                                                        let name = prospect_core::config::entity_name(&entity);
                                                        let count = d.counts.get(name).copied().unwrap_or(0);
                                                        let groups = new + pending + done;
                                                        rsx! {
                                                            tr { key: "{entity}",
                                                                td {
                                                                    div { class: "state-name-cell",
                                                                        if count > 0 {
                                                                            span { class: "state-loaded-dot", {svg_check()} }
                                                                        }
                                                                        strong { "{name}" }
                                                                    }
                                                                    p { class: "muted st-age", "{prospect_core::updates::age_label(&date, saved.denue_max_age_days)}" }
                                                                    if !error.is_empty() {
                                                                        p { class: "error st-age", "{error}" }
                                                                    }
                                                                }
                                                                td { class: "st-num", "{number(count)}" }
                                                                td { class: "st-num", "{done} / {groups}" }
                                                                td { class: "st-action",
                                                                    workspace::StateImportAction {
                                                                        entity: entity.clone(),
                                                                        pending,
                                                                        new,
                                                                        configured: d.denue_configured,
                                                                        running,
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                if *update_modal.read() {
                                    {
                                        let up_to_date = waiting == 0 && complete > 0;
                                        let db_new = modal_db.clone();
                                        let db_all = modal_db.clone();
                                        let db_again = modal_db.clone();
                                        rsx! {
                                            div { class: "modal-backdrop", role: "dialog", aria_modal: "true", aria_labelledby: "update-choice-title",
                                                div { class: "modal-card",
                                                    div { class: "modal-card-head",
                                                        h2 { id: "update-choice-title",
                                                            if up_to_date { "Todo parece actualizado" } else { "¿Qué quieres actualizar?" }
                                                        }
                                                        button {
                                                            class: "toast-close",
                                                            aria_label: "Cerrar",
                                                            onclick: move |_| update_modal.set(false),
                                                            "×"
                                                        }
                                                    }
                                                    if up_to_date {
                                                        p { class: "muted",
                                                            "Ya consultaste todos los grupos de tus estados. ¿Quieres volver a consultarlos desde el inicio y actualizar? Tus contactos, notas e historial se conservan."
                                                        }
                                                        div { class: "modal-card-footer",
                                                            button {
                                                                class: "button secondary",
                                                                onclick: move |_| update_modal.set(false),
                                                                "Cancelar"
                                                            }
                                                            button {
                                                                class: "button primary",
                                                                disabled: !d.denue_configured,
                                                                onclick: move |_| {
                                                                    update_modal.set(false);
                                                                    job_start(JobKind::Recheck, ui, db_again.clone(), String::new());
                                                                },
                                                                "Volver a consultar"
                                                            }
                                                        }
                                                    } else {
                                                        p { class: "muted", "Se descarga el siguiente lote de tus estados y se calculan prioridades." }
                                                        div { class: "modal-choice-grid",
                                                            button {
                                                                class: "modal-choice-card",
                                                                r#type: "button",
                                                                onclick: move |_| {
                                                                    update_modal.set(false);
                                                                    job_start_sync(ui, db_new.clone(), None, false, false, Some("Actualizar solo nuevos".into()));
                                                                },
                                                                strong { "Sólo lo nuevo" }
                                                                span { class: "muted small", "Más rápido: descarga el siguiente lote y calcula sólo empresas nuevas o sin puntaje." }
                                                            }
                                                            button {
                                                                class: "modal-choice-card",
                                                                r#type: "button",
                                                                onclick: move |_| {
                                                                    update_modal.set(false);
                                                                    job_start_sync(ui, db_all.clone(), None, true, false, Some("Actualizar y recalcular todo".into()));
                                                                },
                                                                strong { "Recalcular toda la base" }
                                                                span { class: "muted small", "Descarga el siguiente lote y recalcula prioridades y mensajes de todas tus empresas." }
                                                            }
                                                        }
                                                        div { class: "modal-card-footer",
                                                            button {
                                                                class: "button secondary",
                                                                onclick: move |_| update_modal.set(false),
                                                                "Cancelar"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
