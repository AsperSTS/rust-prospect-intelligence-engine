use crate::{
    Services,
    onboarding::{FirstRunCredentialsModal, OnboardingControl},
    ui,
};
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    let services = consume_context::<Services>();
    let missing_denue = services
        .db
        .paths
        .secrets()
        .map(|s| s.denue.trim().is_empty())
        .unwrap_or(true);
    let onboarding = use_context_provider(|| OnboardingControl {
        open: Signal::new(missing_denue),
        revision: Signal::new(0),
        first_run_pending_sync: Signal::new(false),
        restored: Signal::new(None),
    });

    rsx! {
        ui::App {}
        if *onboarding.open.read() {
            FirstRunCredentialsModal {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Layout, onboarding::save_tokens};
    use prospect_core::{AppPaths, Db, Settings};

    #[test]
    fn first_run_without_credentials_shows_onboarding_modal_over_original_ui() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        let db = Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(App).with_root_context(Services {
            db,
            settings: Settings::default(),
            layout: Layout::Desktop,
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("TU MESA DE TRABAJO"));
        assert!(html.contains("Dashboard"));
        assert!(html.contains("Territorios"));
        assert!(html.contains("Oportunidades"));
        assert!(html.contains("Configura tus credenciales"));
        assert!(html.contains("Obligatorio"));
        assert!(html.contains("Opcional"));
        assert!(html.contains("<svg"));
        assert!(html.contains("https://www.inegi.org.mx/servicios/api_denue.html"));
        assert!(html.contains("https://www.inegi.org.mx/servicios/api_indicadores.html"));
        assert!(html.contains("https://www.banxico.org.mx/SieAPIRest/service/v1/token"));
        assert!(html.contains("Guardar tokens"));
        assert!(html.contains("Configurar después"));
    }

    #[test]
    fn launch_with_saved_credentials_skips_onboarding_modal() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        save_tokens(&paths, "tok-denue-test", "", "tok-banxico-test", true).unwrap();
        let db = Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(App).with_root_context(Services {
            db,
            settings: Settings::default(),
            layout: Layout::Desktop,
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("TU MESA DE TRABAJO"));
        assert!(!html.contains("Configura tus credenciales"));
    }

    #[test]
    fn ios_first_run_without_credentials_shows_onboarding_modal() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        let db = Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(App).with_root_context(Services {
            db,
            settings: Settings::default(),
            layout: Layout::Ios,
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("onboarding-dialog ios"));
        assert!(html.contains("Configura tus credenciales"));
    }

    #[tokio::test]
    async fn empty_desktop_shows_original_ui_and_import_guidance() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        save_tokens(&paths, "tok-denue-test", "", "", true).unwrap();
        let db = Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(App).with_root_context(Services {
            db,
            settings: Settings::default(),
            layout: Layout::Desktop,
        });
        dom.rebuild_in_place();
        let html = tokio::time::timeout(std::time::Duration::from_secs(3), async {
            loop {
                dom.render_immediate_to_vec();
                let html = dioxus_ssr::render(&dom);
                if html.contains("Empieza con tu propia base") {
                    return html;
                }
                dom.wait_for_work().await;
            }
        })
        .await
        .expect("desktop should settle empty-base guidance");
        assert!(html.contains("Abrir Ajustes"));
        assert!(html.contains("Gestionar datos"));
        assert!(html.contains("Actualizaciones"));
    }

    #[test]
    fn settings_screen_renders_full_width_renamed_tabs() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        save_tokens(&paths, "tok-denue-test", "", "", true).unwrap();
        let db = Db::open(paths).unwrap();
        let mut dom = VirtualDom::new(crate::ui::App).with_root_context(Services {
            db,
            settings: Settings::default(),
            layout: Layout::Desktop,
        });
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("Actualizaciones"));
        assert!(html.contains("Ajustes"));
    }

    #[test]
    fn onboarding_save_tokens_requires_denue_and_persists_privately() {
        let temp = tempfile::tempdir().unwrap();
        let paths = AppPaths::at(temp.path()).unwrap();
        assert!(save_tokens(&paths, "   ", "", "", true).is_err());
        save_tokens(&paths, "  denue-123  ", "", "  banxico-456 ", true).unwrap();
        let secrets = paths.secrets().unwrap();
        assert_eq!(secrets.denue, "denue-123");
        assert_eq!(secrets.inegi, "denue-123");
        assert_eq!(secrets.banxico, "banxico-456");
    }
}
