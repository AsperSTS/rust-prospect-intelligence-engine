#![forbid(unsafe_code)]
mod app_shell;
#[cfg(target_os = "ios")]
mod ios;
mod onboarding;
mod ui;
mod view_cache;

pub use app_shell::App;
use prospect_core::{Db, Settings};
pub use ui::{StartupFailure, StartupProblem};

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
