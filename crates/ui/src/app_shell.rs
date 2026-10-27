use crate::{Layout, Services, simple_ui, ui};
use dioxus::prelude::*;

#[component]
pub fn App() -> Element {
    let services = consume_context::<Services>();
    if services.layout == Layout::Ios {
        return rsx! { ui::App {} };
    }

    let mut advanced = use_signal(|| false);
    if *advanced.read() {
        rsx! {
            style { {include_str!("shell.css")} }
            button {
                class:"simple-shell-return",
                onclick:move |_|advanced.set(false),
                "← Volver al flujo simple"
            }
            ui::App {}
        }
    } else {
        rsx! {
            style { {include_str!("shell.css")} }
            simple_ui::App {}
            button {
                class:"simple-shell-tools",
                onclick:move |_|advanced.set(true),
                "Herramientas avanzadas"
            }
        }
    }
}
