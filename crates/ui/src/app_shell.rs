use crate::{Layout, Services, simple_ui, ui};
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    let services = consume_context::<Services>();
    if services.layout == Layout::Ios {
        return rsx! { ui::App {} };
    }
    rsx! {
        style { ".simple-link,.simple-advanced-state{display:none!important;}" }
        simple_ui::App {}
    }
}
