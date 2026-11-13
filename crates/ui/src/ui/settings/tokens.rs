//! Access tokens for the official sources and the portable backup.
use super::*;
use crate::onboarding::{
    BANXICO_GUIDE, DENUE_GUIDE, INEGI_INDICATORS_GUIDE, TokenInput, TokenSourceGuide,
    use_token_form,
};

#[component]
fn TokenRow(
    id: &'static str,
    title: &'static str,
    purpose: &'static str,
    required: bool,
    ok: bool,
    ok_note: &'static str,
    guide: TokenSourceGuide,
    value: Signal<String>,
    error: Signal<String>,
) -> Element {
    let mut ui = use_context::<UiState>();
    rsx! {
        div { class: "st-token",
            div { class: "st-token-head",
                label { r#for: id, "{title}" }
                span { class: "muted small", if required { "Obligatorio" } else { "Opcional" } }
                if ok {
                    span { class: "st-status ok", {svg_check()} "Ok" }
                } else {
                    span { class: if required { "st-status missing" } else { "st-status" },
                        if required { "Falta" } else { "Sin configurar" }
                    }
                }
            }
            p { class: "muted small",
                "{purpose}"
                if ok && !ok_note.is_empty() { " {ok_note}" }
            }
            TokenInput {
                id: id.to_owned(),
                value,
                error,
                required,
                placeholder: if ok { "Pega otro token sólo si quieres reemplazarlo" } else { "Pega aquí tu token" },
            }
            button {
                class: "link-button small",
                r#type: "button",
                onclick: move |_| {
                    if let Err(e) = open_link(guide.url) {
                        ui.toast.set(e);
                    }
                },
                "{guide.link_label} ↗"
            }
        }
    }
}

#[component]
pub(super) fn TokensTab(mut pane: Signal<usize>) -> Element {
    let service = consume_context::<Services>();
    let paths = service.db.paths.clone();
    let mut ui = use_context::<UiState>();
    let onboarding = try_use_context::<OnboardingControl>();
    if let Some(o) = onboarding {
        let _ = *o.revision.read();
    }
    let _ = *ui.revision.read();
    let configured = paths.secrets().unwrap_or_default();
    let form = use_token_form();
    let on_saved = use_callback(move |()| {
        if let Some(mut o) = onboarding {
            o.revision += 1;
        }
        ui.revision += 1;
        ui.toast.set("Tokens guardados.".into());
    });
    let checking = *form.checking.read();
    let save_paths = paths.clone();
    let skip_paths = paths.clone();
    let backup_db = service.db.clone();
    let shares_denue = !configured.inegi.is_empty() && configured.inegi == configured.denue;
    let inegi_note: &'static str = if shares_denue {
        "Hoy usa tu token DENUE."
    } else {
        ""
    };

    rsx! {
        div { class: "st-grid st-grid-tokens",
            section { class: "panel st-tokens",
                div { class: "panel-heading",
                    h2 { "Tokens" }
                    span { class: "muted small", "Gratuitos y a tu nombre" }
                }
                div { class: "form-body",
                    TokenRow {
                        id: "token-denue",
                        title: "DENUE (INEGI)",
                        purpose: "Descarga los negocios de cada estado. Sin él no puedes crear ni actualizar tu base.",
                        required: true,
                        ok: !configured.denue.is_empty(),
                        ok_note: "",
                        guide: DENUE_GUIDE,
                        value: form.denue,
                        error: form.denue_error,
                    }
                    TokenRow {
                        id: "token-inegi",
                        title: "INEGI Indicadores",
                        purpose: "Trae indicadores económicos para la pantalla Indicadores.",
                        required: false,
                        ok: !configured.inegi.is_empty(),
                        ok_note: inegi_note,
                        guide: INEGI_INDICATORS_GUIDE,
                        value: form.inegi,
                        error: form.inegi_error,
                    }
                    TokenRow {
                        id: "token-banxico",
                        title: "Banxico SIE",
                        purpose: "Trae el tipo de cambio oficial dólar–peso para tus oportunidades.",
                        required: false,
                        ok: !configured.banxico.is_empty(),
                        ok_note: "",
                        guide: BANXICO_GUIDE,
                        value: form.banxico,
                        error: form.banxico_error,
                    }
                    div { class: "st-footer",
                        span { class: "muted small", "Cada token se comprueba con su fuente antes de guardarse." }
                        if *form.unverified.read() {
                            button {
                                class: "button secondary",
                                onclick: move |_| form.submit(skip_paths.clone(), false, true, on_saved),
                                "Guardar sin verificar"
                            }
                        }
                        button {
                            class: "button primary",
                            disabled: checking || form.is_empty(),
                            onclick: move |_| form.submit(save_paths.clone(), false, false, on_saved),
                            if checking { "Verificando…" } else { "Verificar y guardar" }
                        }
                    }
                }
            }
            section { class: "panel st-backup",
                div { class: "panel-heading", h2 { "Respaldo" } }
                div { class: "form-body",
                    div { class: "st-block",
                        strong { "Llévate todo en un archivo" }
                        p { class: "muted small",
                            "Un .zip con tus empresas, contactos, notas, plantillas, ajustes y tokens. Tú eliges dónde guardarlo."
                        }
                        div { class: "st-actions",
                            button {
                                class: "button backup",
                                disabled: ui.heavy_task.read().is_some(),
                                onclick: move |_| run_backup(backup_db.clone(), ui),
                                {svg_backup_shield()}
                                span { "Crear respaldo" }
                            }
                        }
                        p { class: "st-warning small",
                            "Incluye tus tokens: guárdalo en un lugar privado y no lo compartas."
                        }
                    }
                    div { class: "st-block",
                        strong { "Cómo recuperarlo" }
                        p { class: "muted small",
                            "En una instalación nueva, la primera pantalla ofrece «Cargar respaldo». Elige tu .zip y continúas donde te quedaste, también al cambiar de equipo o de versión."
                        }
                    }
                    div { class: "st-block",
                        strong { "Tu información es tuya" }
                        p { class: "muted small",
                            "La base, tus notas y tus tokens se guardan sólo en este equipo. La app únicamente consulta las fuentes oficiales cuando tú lo pides."
                        }
                        button {
                            class: "link-button small",
                            onclick: move |_| pane.set(1),
                            "Ver el estado de tu base →"
                        }
                    }
                }
            }
        }
    }
}
