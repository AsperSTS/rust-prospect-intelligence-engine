#![forbid(unsafe_code)]
use dioxus::prelude::*;
use prospect_core::{AppPaths, Db, Settings};
use prospect_ui::{App, Layout, Services, StartupFailure, StartupProblem};

fn main() {
    if let Err(error) = start() {
        eprintln!("No se pudo iniciar Prospect: {error}");
        LaunchBuilder::mobile()
            .with_cfg(
                dioxus::mobile::Config::new().with_custom_index(include_str!("index.html").into()),
            )
            .with_context(StartupFailure {
                message: error.to_string(),
                directory: AppPaths::location()
                    .map(|p| p.display().to_string())
                    .unwrap_or_default(),
            })
            .launch(StartupProblem);
    }
}

fn start() -> prospect_core::Result<()> {
    let paths = AppPaths::discover()?;
    let settings = Settings::load(&paths)?;
    let db = Db::open(paths.clone())?;
    let config = dioxus::mobile::Config::new()
        .with_custom_index(include_str!("index.html").into())
        .with_data_directory(paths.home.join("webview"));
    LaunchBuilder::mobile()
        .with_cfg(config)
        .with_context(Services {
            db,
            settings,
            layout: Layout::Ios,
        })
        .launch(App);
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn ios_document_has_one_full_screen_viewport() {
        let index = include_str!("index.html");
        assert_eq!(index.matches("name=\"viewport\"").count(), 1);
        assert!(index.contains("viewport-fit=cover"));
        assert!(!index.contains("user-scalable=no"));
        assert!(index.contains("id=\"main\""));
    }
}
