use leptos::prelude::*;
use leptos::html::Dialog;
use wasm_bindgen::prelude::*;
use serde::{Deserialize, Serialize};
use serde_wasm_bindgen::to_value;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct CodexSettings { command: Option<String>, args: Option<Vec<String>> }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AgentSettings { codex: Option<CodexSettings> }

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn SettingsModal(dialog_ref: NodeRef<Dialog>) -> impl IntoView {
    let (codex_command, set_codex_command) = signal(String::new());
    let (codex_args, set_codex_args) = signal(String::new()); // space-separated

    // Load settings on open invocation
    let load_settings = {
        let set_codex_command = set_codex_command.clone();
        let set_codex_args = set_codex_args.clone();
        move || {
            leptos::task::spawn_local(async move {
                let args = serde_json::json!({});
                if let Ok(js) = to_value(&args) {
                    let resp = invoke("load_agent_settings", js).await;
                    if !resp.is_undefined() {
                        if let Ok(settings) = serde_wasm_bindgen::from_value::<AgentSettings>(resp) {
                            if let Some(codex) = settings.codex {
                                if let Some(cmd) = codex.command { set_codex_command.set(cmd); }
                                if let Some(a) = codex.args { set_codex_args.set(a.join(" ")); }
                            }
                        }
                    }
                }
            });
        }
    };

    let close_modal = move |_| { if let Some(d) = dialog_ref.get() { d.close(); } };

    let save_settings = move |_| {
        let command = codex_command.get();
        let args_line = codex_args.get();
        let args_vec: Vec<String> = if args_line.trim().is_empty() { vec![] } else { args_line.split_whitespace().map(|s| s.to_string()).collect() };
        leptos::task::spawn_local(async move {
            let payload = AgentSettings { codex: Some(CodexSettings { command: if command.is_empty() { None } else { Some(command) }, args: Some(args_vec) }) };
            if let Ok(js) = to_value(&payload) {
                let _ = invoke("save_agent_settings", js).await;
            }
        });
        if let Some(d) = dialog_ref.get() { d.close(); }
    };

    view! {
        <dialog node_ref=dialog_ref class="modal settings-modal">
            <div class="modal-content">
                <div class="modal-header">
                    <h2>"Settings"</h2>
                    <button class="modal-close" on:click=close_modal>"x"</button>
                </div>
                <div class="modal-section">
                    <h3>"Codex Agent"</h3>
                    <div class="form-group">
                        <label>"Command"</label>
                        <input type="text" placeholder="codex or codex.cmd" prop:value=move || codex_command.get() on:input=move |ev| set_codex_command.set(event_target_value(&ev)) />
                    </div>
                    <div class="form-group">
                        <label>"Extra Args (space separated, before prompt)"</label>
                        <input type="text" placeholder="-- --output-format json" prop:value=move || codex_args.get() on:input=move |ev| set_codex_args.set(event_target_value(&ev)) />
                    </div>
                </div>
                <div class="modal-actions">
                    <button class="btn-secondary" on:click=close_modal>"Cancel"</button>
                    <button class="btn-primary" on:click=save_settings>"Save"</button>
                </div>
            </div>
            {move || { load_settings(); view! { <span></span> } }}
        </dialog>
    }
}

