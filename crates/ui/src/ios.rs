//! Safe UIKit APIs only. No evaluated programs or interpolated scripts.
use objc2::MainThreadMarker;
use objc2_foundation::{NSData, NSString, NSURL};
use objc2_ui_kit::{UIApplication, UIPasteboard};

fn active_window() -> Option<objc2::rc::Retained<objc2_ui_kit::UIWindow>> {
    let main = MainThreadMarker::new()?;
    let app = UIApplication::sharedApplication(main);
    let mut fallback = None;
    for scene in app.connectedScenes() {
        if let Some(scene) = scene.downcast_ref::<objc2_ui_kit::UIWindowScene>() {
            for window in scene.windows() {
                if window.isKeyWindow() {
                    return Some(window);
                }
                if fallback.is_none() {
                    fallback = Some(window);
                }
            }
        }
    }
    fallback
}

/// WKWebView may report a viewport with the safe areas subtracted twice.
/// UIKit is the source of truth for the full frame and each system inset.
pub(crate) fn viewport_style(editing: bool) -> String {
    let Some(window) = active_window() else {
        return String::new();
    };
    let size = window.bounds().size;
    let safe = window.safeAreaInsets();
    // UIKit may retain the accessory inset after dismissal; focus owns the reservation.
    let keyboard = editing
        .then(|| {
            window
                .rootViewController()
                .and_then(|controller| controller.view())
                .map(|view| view.keyboardLayoutGuide().layoutFrame())
        })
        .flatten();
    let height = keyboard
        .filter(|frame| {
            frame.size.height > safe.bottom + 1.0
                && frame.origin.y.is_finite()
                && frame.origin.y > safe.top
                && frame.origin.y < size.height
        })
        .map(|frame| frame.origin.y)
        .unwrap_or(size.height);
    let bottom = if height < size.height {
        0.0
    } else {
        safe.bottom
    };
    format!(
        "--ios-height:{}px;--ios-top:{}px;--ios-right:{}px;--ios-bottom:{}px;--ios-left:{}px",
        height, safe.top, safe.right, bottom, safe.left
    )
}

pub(crate) fn use_viewport_style(
    editing: dioxus::prelude::Signal<bool>,
) -> dioxus::prelude::Signal<String> {
    use dioxus::mobile::tao::event::{Event, WindowEvent};
    use dioxus::prelude::*;
    let mut style = use_signal(move || viewport_style(*editing.peek()));
    use_effect(move || {
        let next = viewport_style(*editing.read());
        if *style.peek() != next {
            style.set(next);
        }
    });
    dioxus::mobile::use_wry_event_handler(move |event, _| {
        if matches!(
            event,
            Event::WindowEvent {
                event: WindowEvent::Resized(_) | WindowEvent::ScaleFactorChanged { .. },
                ..
            } | Event::Resumed
                | Event::MainEventsCleared
        ) {
            let next = viewport_style(*editing.peek());
            if *style.peek() != next {
                style.set(next);
            }
        }
    });
    style
}

pub(crate) fn configure_viewport() {
    use objc2_ui_kit::{UIScrollView, UIScrollViewContentInsetAdjustmentBehavior, UIView};
    fn configure(view: &UIView, depth: usize) {
        if let Some(scroll) = view.downcast_ref::<UIScrollView>() {
            scroll.setContentInsetAdjustmentBehavior(
                UIScrollViewContentInsetAdjustmentBehavior::Never,
            );
            scroll.setAutomaticallyAdjustsScrollIndicatorInsets(false);
            return;
        }
        if depth < 6 {
            for child in view.subviews() {
                configure(&child, depth + 1);
            }
        }
    }
    if let Some(window) = active_window() {
        configure(&window, 0);
    }
}

/// Match native controls, keyboard and system bar contrast to the saved appearance.
pub(crate) fn set_appearance(theme: &str) {
    use objc2_ui_kit::UIUserInterfaceStyle;
    if let Some(window) = active_window() {
        let style = if theme == "dark" {
            UIUserInterfaceStyle::Dark
        } else {
            UIUserInterfaceStyle::Light
        };
        if window.overrideUserInterfaceStyle() != style {
            window.setOverrideUserInterfaceStyle(style);
        }
    }
}

pub(crate) fn copy(text: &str) -> Result<(), String> {
    MainThreadMarker::new().ok_or("El portapapeles requiere el hilo principal")?;
    UIPasteboard::generalPasteboard().setData_forPasteboardType(
        &NSData::with_bytes(text.as_bytes()),
        &NSString::from_str("public.utf8-plain-text"),
    );
    Ok(())
}

pub(crate) fn open(link: &str) -> Result<(), String> {
    let main = MainThreadMarker::new().ok_or("Los enlaces requieren el hilo principal")?;
    let url = NSURL::URLWithString(&NSString::from_str(link)).ok_or("Enlace inválido")?;
    let app = UIApplication::sharedApplication(main);
    if !app.canOpenURL(&url) {
        return Err(if link.starts_with("mailto:") {
            "Configura tu app de correo predeterminada en iOS o copia el borrador para continuar."
        } else {
            "No hay una app disponible para este canal. Puedes copiar los datos desde la ficha."
        }
        .into());
    }
    if let Some(scene) = active_window().and_then(|window| window.windowScene()) {
        scene.openURL_options_completionHandler(&url, None, None);
        return Ok(());
    }
    if link.starts_with("http://") || link.starts_with("https://") {
        return webbrowser::open(link).map_err(|e| e.to_string());
    }
    Err("No se encontró una ventana activa de iOS. Vuelve a intentarlo.".into())
}
