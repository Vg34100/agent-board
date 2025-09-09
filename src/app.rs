use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::pages::{Projects, Kanban};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Clone, Debug)]
pub enum AppView {
    Projects,
    Kanban(String),
}

#[component]
pub fn App() -> impl IntoView {
    let (current_view, set_current_view) = signal(AppView::Projects);
    let (is_dev, set_is_dev) = signal(false);
    
    provide_context(set_current_view);

    // Check if we're in dev mode on component mount
    Effect::new(move |_| {
        spawn_local(async move {
            let result = invoke("is_dev_mode", to_value(&()).unwrap()).await;
            match result.as_bool() {
                Some(dev_mode) => {
                    set_is_dev.set(dev_mode);
                    if dev_mode {
                        web_sys::console::log_1(&"🚩 Running in development mode".into());
                    }
                }
                None => {
                    web_sys::console::log_1(&"Failed to determine dev mode status".into());
                }
            }
        });
    });

    view! {
        // Dev mode banner (only shown in development)
        {move || {
            if is_dev.get() {
                view! {
                    <div class="dev-banner">
                        "🚩 DEV MODE"
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }
        }}
        
        <main class="app">
            {move || match current_view.get() {
                AppView::Projects => view! { <Projects /> }.into_any(),
                AppView::Kanban(project_id) => view! { <Kanban project_id=project_id /> }.into_any(),
            }}
        </main>
    }
}
