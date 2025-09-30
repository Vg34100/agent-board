use leptos::prelude::*;
use leptos::task::spawn_local;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::features::agent_chat::models::{AgentMessage, AgentProcess};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Helper: sticky scroll to bottom for a container id
pub fn scroll_to_bottom(id: &str) {
    if let Some(window) = web_sys::window() {
        if let Some(doc) = window.document() {
            if let Some(el) = doc.get_element_by_id(id) {
                use wasm_bindgen::JsCast;
                if let Ok(div) = el.dyn_into::<web_sys::HtmlElement>() {
                    let sh = div.scroll_height() as i32;
                    div.set_scroll_top(sh);
                }
            }
        }
    }
}

// Load agent messages for current process with force refresh
pub fn create_load_agent_messages(
    task_id: String,
    set_agent_messages: WriteSignal<Vec<AgentMessage>>,
    set_messages_by_process: WriteSignal<HashMap<String, Vec<AgentMessage>>>,
) -> impl Fn(String) + Clone {
    move |process_id: String| {
        let task_id = task_id.clone();
        let set_agent_messages = set_agent_messages.clone();
        let set_messages_by_process = set_messages_by_process.clone();

        spawn_local(async move {
            let args = serde_json::json!({
                "processId": process_id,
                "taskId": task_id
            });

            if let Ok(js_value) = to_value(&args) {
                match invoke("get_agent_messages", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(messages) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(js_result) {
                            if messages.is_empty() {
                                // Fallback: hydrate from persisted per-process store instead of overwriting with empty
                                let args_load = serde_json::json!({
                                    "processId": process_id
                                });
                                if let Ok(js_value_load) = to_value(&args_load) {
                                    if let Some(stored_result) = invoke("load_agent_messages", js_value_load).await.as_string() {
                                        if let Ok(stored_messages) = serde_json::from_str::<Vec<AgentMessage>>(&stored_result) {
                                            if !stored_messages.is_empty() {
                                                // Update state only if we actually have stored content
                                                set_agent_messages.set(stored_messages.clone());
                                                set_messages_by_process.update(|map| {
                                                    map.insert(process_id.clone(), stored_messages);
                                                });
                                                return;
                                            }
                                        }
                                    }
                                }
                            } else {
                                // Non-empty runtime messages: accept and persist
                                set_agent_messages.set(messages.clone());
                                set_messages_by_process.update(|map| {
                                    map.insert(process_id.clone(), messages.clone());
                                });

                                // Persist the fresh messages
                                let persist_args = serde_json::json!({
                                    "processId": process_id,
                                    "messages": messages
                                });
                                if let Ok(persist_js) = to_value(&persist_args) {
                                    let _ = invoke("save_agent_messages", persist_js).await;
                                }

                                // Auto-scroll to bottom after loading messages
                                let outer_id = format!("agent-sessions-{}", task_id);
                                let inner_id = format!("agent-messages-{}", process_id);
                                let outer_id2 = outer_id.clone();
                                let inner_id2 = inner_id.clone();
                                spawn_local(async move {
                                    gloo_timers::future::TimeoutFuture::new(32).await;
                                    scroll_to_bottom(&outer_id2);
                                    scroll_to_bottom(&inner_id2);
                                    gloo_timers::future::TimeoutFuture::new(160).await;
                                    scroll_to_bottom(&outer_id2);
                                    scroll_to_bottom(&inner_id2);
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }
}

// Load all processes (from memory and persisted store)
pub fn create_load_all_processes(
    set_all_processes: WriteSignal<Vec<serde_json::Value>>,
) -> impl Fn() + Clone {
    move || {
        let set_all_processes = set_all_processes.clone();

        spawn_local(async move {
            // Load current processes from memory
            let mut current_processes = Vec::new();
            match invoke("get_process_list", to_value(&serde_json::json!({})).unwrap()).await {
                js_result if !js_result.is_undefined() => {
                    if let Ok(processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                        current_processes = processes;
                    }
                }
                _ => {}
            }

            // Also load persisted processes
            match invoke("load_agent_processes", to_value(&serde_json::json!({})).unwrap()).await {
                js_result if !js_result.is_undefined() => {
                    if let Ok(persisted_processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                        // Merge persisted processes with current ones (current processes take priority)
                        let mut merged_processes = current_processes.clone();

                        for persisted in persisted_processes {
                            let persisted_id = persisted.get("id").and_then(|v| v.as_str()).unwrap_or("");
                            // Only add persisted process if it's not already in current processes
                            if !current_processes.iter().any(|current| {
                                current.get("id").and_then(|v| v.as_str()).unwrap_or("") == persisted_id
                            }) {
                                merged_processes.push(persisted);
                            }
                        }

                        set_all_processes.set(merged_processes);
                    }
                }
                _ => {
                    // Just set current processes if persisted loading fails
                    set_all_processes.set(current_processes);
                }
            }
        });
    }
}