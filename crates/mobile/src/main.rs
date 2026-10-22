#![forbid(unsafe_code)]

mod ui;

use dioxus::prelude::*;
use prospect_core::{AppPaths, Db, Settings};
use ui::{App, MobileServices};

fn main() {
    if let Err(error) = start() {
        eprintln!("Error iniciando Prospect Intelligence Mobile: {error}");
    }
}

fn start() -> prospect_core::Result<()> {
    let paths = AppPaths::discover()?;
    let settings = Settings::load(&paths)?;
    let db = Db::open(paths.clone())?;

    LaunchBuilder::mobile()
        .with_context(MobileServices {
            db,
            settings,
            paths,
        })
        .launch(App);

    Ok(())
}
