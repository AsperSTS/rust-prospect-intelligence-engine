#![forbid(unsafe_code)]
use dioxus::prelude::*;
use prospect_core::{AppPaths, Db, Settings};
use prospect_ui::{self as ui, Layout, Services};
fn main() {
    if let Err(error) = start() {
        eprintln!("{error}");
        let directory = AppPaths::location()
            .map(|p| p.display().to_string())
            .unwrap_or_default();
        LaunchBuilder::desktop()
            .with_cfg(
                dioxus::desktop::Config::new().with_window(
                    dioxus::desktop::WindowBuilder::new()
                        .with_title("Prospect Intelligence · Inicio")
                        .with_inner_size(dioxus::desktop::LogicalSize::new(780.0, 580.0)),
                ),
            )
            .with_context(ui::StartupFailure {
                message: error.to_string(),
                directory,
            })
            .launch(ui::StartupProblem);
    }
}
fn start() -> prospect_core::Result<()> {
    let paths = AppPaths::discover()?;
    let settings = Settings::load_desktop(&paths)?;
    let db = Db::open(paths.clone())?;
    let config = dioxus::desktop::Config::new()
        .with_window(
            dioxus::desktop::WindowBuilder::new()
                .with_title("Prospect Intelligence")
                .with_inner_size(dioxus::desktop::LogicalSize::new(1440.0, 900.0))
                .with_min_inner_size(dioxus::desktop::LogicalSize::new(1000.0, 650.0)),
        )
        .with_data_directory(paths.home.join("webview"));
    LaunchBuilder::desktop()
        .with_cfg(config)
        .with_context(Services {
            db,
            settings,
            layout: Layout::Desktop,
        })
        .launch(ui::App);
    Ok(())
}
