//! Session-only view snapshots. A key includes the local data revision and offer.
use dioxus::prelude::*;
use std::{any::Any, cell::RefCell, collections::VecDeque, future::Future, rc::Rc};

#[derive(Default)]
pub(crate) struct ViewCache(RefCell<VecDeque<(String, Rc<dyn Any>)>>);
impl ViewCache {
    fn get<T: Clone + 'static>(&self, key: &str) -> Option<T> {
        let mut entries = self.0.borrow_mut();
        let i = entries.iter().position(|(k, _)| k == key)?;
        let entry = entries.remove(i)?;
        let value = entry.1.downcast_ref::<T>().cloned();
        entries.push_back(entry);
        value
    }
    fn insert<T: 'static>(&self, key: String, value: T) {
        let mut entries = self.0.borrow_mut();
        entries.retain(|(k, _)| *k != key);
        if entries.len() >= 24 {
            entries.pop_front();
        }
        entries.push_back((key, Rc::new(value)));
    }
}
pub(crate) struct CachedRead<T: 'static> {
    resource: Resource<(String, Result<T, String>)>,
    key: Memo<String>,
    cache: Signal<ViewCache>,
}
impl<T> Copy for CachedRead<T> {}
impl<T> Clone for CachedRead<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T: Clone + 'static> CachedRead<T> {
    pub(crate) fn value(self) -> Option<T> {
        // Subscribe to completion, and only accept results for the current key.
        let result = self.resource.read();
        let key = self.key.read();
        self.cache.peek().get(&key).or_else(|| {
            result
                .as_ref()
                .filter(|(k, _)| k == &*key)
                .and_then(|(_, value)| value.as_ref().ok())
                .cloned()
        })
    }
    pub(crate) fn status(mut self, label: &'static str) -> Option<Element> {
        if self.value().is_some() {
            return None;
        }
        if *self.resource.state().read() != UseResourceState::Pending {
            let error = self
                .resource
                .read()
                .as_ref()
                .filter(|(key, _)| key == &*self.key.read())
                .and_then(|(_, value)| value.as_ref().err())
                .cloned();
            if let Some(error) = error {
                return Some(
                    rsx! {section{class:"panel load-status",role:"alert",h2{"Error al cargar {label}"}p{class:"error","{error}"}button{class:"button secondary",onclick:move |_|self.resource.restart(),"Volver a intentar"}}},
                );
            }
        }
        Some(
            rsx! {section{class:"panel load-status",role:"status",aria_busy:"true",h2{"Cargando {label}…"}p{class:"muted","Espera un momento."}}},
        )
    }
}

pub(crate) fn use_cached_read<
    T: Clone + 'static,
    F: Future<Output = Result<T, String>> + 'static,
>(
    key: impl FnMut() -> String + 'static,
    mut fetch: impl FnMut() -> F + 'static,
) -> CachedRead<T> {
    let cache = use_context::<Signal<ViewCache>>();
    let key = use_memo(key);
    let resource = use_resource(move || {
        let current = key.read().clone();
        let future = fetch();
        async move {
            if let Some(value) = cache.peek().get(&current) {
                return (current, Ok(value));
            }
            let value = future.await;
            if let Ok(value) = &value {
                cache.peek().insert(current.clone(), value.clone());
            }
            (current, value)
        }
    });
    CachedRead {
        resource,
        key,
        cache,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    #[derive(Clone, Copy)]
    struct Controls {
        offer: Signal<u8>,
        revision: Signal<u64>,
        visible: Signal<bool>,
    }
    #[derive(Clone, Default)]
    struct Harness {
        controls: Rc<RefCell<Option<Controls>>>,
        calls: Rc<Cell<usize>>,
    }
    #[component]
    fn ProbeRoot() -> Element {
        let harness = use_context::<Harness>();
        let controls = Controls {
            offer: use_signal(|| 0),
            revision: use_signal(|| 0),
            visible: use_signal(|| true),
        };
        *harness.controls.borrow_mut() = Some(controls);
        use_context_provider(|| controls);
        use_context_provider(|| Signal::new(ViewCache::default()));
        rsx! {if *controls.visible.read(){Probe{key:"{controls.offer}"}}}
    }
    #[component]
    fn Probe() -> Element {
        let controls = use_context::<Controls>();
        let calls = use_context::<Harness>().calls;
        let read = use_cached_read(
            move || {
                format!(
                    "{}|offer:{}",
                    controls.revision.read(),
                    controls.offer.read()
                )
            },
            move || {
                let offer = *controls.offer.read();
                let revision = *controls.revision.read();
                let calls = calls.clone();
                async move {
                    calls.set(calls.get() + 1);
                    tokio::task::yield_now().await;
                    Ok(format!("offer={offer},revision={revision}"))
                }
            },
        );
        read.status("la vista")
            .unwrap_or_else(|| rsx! {p{"{read.value().unwrap()}"}})
    }
    async fn settle(dom: &mut VirtualDom, expected: &str) -> String {
        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                dom.render_immediate_to_vec();
                let html = dioxus_ssr::render(dom);
                if html.contains(expected) {
                    break html;
                }
                dom.wait_for_work().await;
            }
        })
        .await
        .expect("resource should resolve")
    }
    #[tokio::test]
    async fn revisits_and_offer_toggles_render_cached_content_on_the_first_frame() {
        let harness = Harness::default();
        let mut dom = VirtualDom::new(ProbeRoot).with_root_context(harness.clone());
        dom.rebuild_in_place();
        assert!(dioxus_ssr::render(&dom).contains("Cargando"));
        settle(&mut dom, "offer=0,revision=0").await;
        let mut controls = harness.controls.borrow().unwrap();
        dom.in_runtime(|| controls.visible.set(false));
        dom.render_immediate_to_vec();
        dom.in_runtime(|| controls.visible.set(true));
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("offer=0,revision=0"));
        assert!(!html.contains("Cargando"));
        assert_eq!(harness.calls.get(), 1);
        dom.in_runtime(|| controls.offer.set(1));
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("offer=0,revision=0"));
        settle(&mut dom, "offer=1,revision=0").await;
        dom.in_runtime(|| controls.offer.set(0));
        dom.render_immediate_to_vec();
        let html = dioxus_ssr::render(&dom);
        assert!(html.contains("offer=0,revision=0"));
        assert!(!html.contains("Cargando"));
        assert_eq!(harness.calls.get(), 2);
        dom.in_runtime(|| controls.revision.set(1));
        dom.render_immediate_to_vec();
        assert!(!dioxus_ssr::render(&dom).contains("offer=0,revision=0"));
        settle(&mut dom, "offer=0,revision=1").await;
        assert_eq!(harness.calls.get(), 3);
    }
    #[component]
    fn FailureProbe() -> Element {
        use_context_provider(|| Signal::new(ViewCache::default()));
        let read = use_cached_read(
            || "0|failed".into(),
            || async {
                tokio::task::yield_now().await;
                Err::<String, _>("Base temporalmente no disponible".into())
            },
        );
        read.status("la vista")
            .unwrap_or_else(|| rsx! {p{"unexpected success"}})
    }
    #[tokio::test]
    async fn cached_read_failures_still_show_error_and_retry() {
        let mut dom = VirtualDom::new(FailureProbe);
        dom.rebuild_in_place();
        let html = settle(&mut dom, "Error al cargar la vista").await;
        assert!(html.contains("Base temporalmente no disponible"));
        assert!(html.contains("Volver a intentar"));
    }
    #[test]
    fn view_cache_has_a_bounded_lru() {
        let cache = ViewCache::default();
        for i in 0..30 {
            cache.insert(format!("{i}"), i);
        }
        assert_eq!(cache.0.borrow().len(), 24);
        assert_eq!(cache.get::<i32>("0"), None);
        assert_eq!(cache.get::<i32>("29"), Some(29));
    }
}
