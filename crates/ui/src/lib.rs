#![forbid(unsafe_code)]
#[cfg(target_os = "ios")]
mod ios;
mod ui;
mod view_cache;

use prospect_core::{Db, Settings};
pub use ui::{App, StartupFailure, StartupProblem};

#[derive(Clone, Copy, Default, PartialEq)]
pub enum Layout {
    #[default]
    Desktop,
    Ios,
}

#[derive(Clone)]
pub struct Services {
    pub db: Db,
    pub settings: Settings,
    pub layout: Layout,
}
