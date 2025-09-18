use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::models::{Task, TaskStatus, AgentProfile};
use std::rc::Rc;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use js_sys;
use web_sys;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub sender: String,
    pub content: String,
    pub timestamp: String,
    pub message_type: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcess {
    pub id: String,
    pub task_id: String,
    pub status: String,
    pub start_time: String,
    pub end_time: Option<String>,
    pub messages: Vec<AgentMessage>,
    pub raw_output: Vec<String>,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn TaskSidebar(
    #[prop(into)] task: Task,
    #[prop(into)] selected_task: WriteSignal<Option<Task>>,
    #[prop(into)] on_edit: Box<dyn Fn(Task) + 'static>, // Callback to trigger edit modal
    #[prop(into)] on_update_status: Rc<dyn Fn(String, TaskStatus) + 'static>,
    #[prop(into)] on_delete: Box<dyn Fn(String) + 'static>,
    #[prop(into)] on_open_worktree: Option<Box<dyn Fn(String) + 'static>>,
    #[prop(into)] on_open_ide: Option<Box<dyn Fn(String) + 'static>>,
    on_update_profile: Box<dyn Fn(String, AgentProfile) + 'static>,
    #[prop(into, optional)] _active_process_id: Option<RwSignal<Option<String>>>,
) -> impl IntoView {
    // State for showing/hiding full description
    let (show_full_description, set_show_full_description) = signal(false);

    // Agent process state
    let (agent_messages, set_agent_messages) = signal(Vec::<AgentMessage>::new());
    let (all_processes, set_all_processes) = signal(Vec::<serde_json::Value>::new());
    let (current_process_id, set_current_process_id) = signal(Option::<String>::None);
    // Messages per process id so multiple groups can render simultaneously
    let (messages_by_process, set_messages_by_process) = signal(HashMap::<String, Vec<AgentMessage>>::new());
    let (message_input, set_message_input) = signal(String::new());
    let (is_sending_message, set_is_sending_message) = signal(false);

    // Clone task data for use in closures
    let task_title = task.title.clone();
    let task_description = task.description.clone();
    let task_status = task.status.clone();
    let task_id = task.id.clone();
    let _task_worktree_path = task.worktree_path.clone();
    let worktree_available = _task_worktree_path.is_some();

    // Local selected profile (default from task)
    let (selected_profile, set_selected_profile) = signal(task.profile.clone());

    // On mount: scroll agent sessions to bottom once, to show latest messages
    {
        let tid = task.id.clone();
        // One-shot mount scroll using a local closure that captures tid by clone
        let scroll_on_mount = move || {
            let tid2 = tid.clone();
            spawn_local(async move {
                gloo_timers::future::TimeoutFuture::new(50).await;
                if let Some(window) = web_sys::window() {
                    if let Some(doc) = window.document() {
                        let container_id = format!("agent-sessions-{}", tid2);
                        if let Some(el) = doc.get_element_by_id(&container_id) {
                            use wasm_bindgen::JsCast;
                            if let Ok(div) = el.dyn_into::<web_sys::HtmlElement>() {
                                let sh = div.scroll_height() as i32;
                                div.set_scroll_top(sh);
                            }
                        }
                    }
                }
            });
        };
        scroll_on_mount();
    }

    // Determine if description is long (more than 5 lines approximately)
    let description_is_long = task_description.len() > 200; // Rough estimate

    // Get display description based on show_full state
    let get_display_description = move || {
        if description_is_long && !show_full_description.get() {
            format!("{}...", &task_description.chars().take(200).collect::<String>())
        } else {
            task_description.clone()
        }
    };

    let close_sidebar = move |_| {
        selected_task.set(None);
    };

    // Helper: sticky scroll to bottom for a container id
    fn scroll_to_bottom(id: &str) {
        if let Some(window) = web_sys::window() {
            if let Some(doc) = window.document() {
                if let Some(el) = doc.get_element_by_id(id) {
                    use wasm_bindgen::JsCast;
                    if let Ok(div) = el.dyn_into::<web_sys::HtmlElement>() {
                        let scroll_top = div.scroll_top() as f64;
                        let client_height = div.client_height() as f64;
                        let scroll_height = div.scroll_height() as f64;
                        let gap = scroll_height - (scroll_top + client_height);
                        if gap < 180.0 {
                            div.set_scroll_top(scroll_height as i32);
                        }
                    }
                }
            }
        }
    }

    // Load agent messages for current process with force refresh
    let load_agent_messages = {
        let set_agent_messages = set_agent_messages.clone();
        let task_id_for_save = task.id.clone();
        move |process_id: String| {
            let set_agent_messages = set_agent_messages.clone();
            let task_id_for_save = task_id_for_save.clone();
            let set_messages_by_process = set_messages_by_process.clone();
            let pid_for_map = process_id.clone();
            spawn_local(async move {
                let args = serde_json::json!({ "processId": process_id });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("get_agent_messages", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(messages) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(js_result) {
                                if messages.is_empty() {
                                    // Fallback: hydrate from persisted per-process store instead of overwriting with empty
                                    let args2 = serde_json::json!({ "taskId": task_id_for_save, "processId": pid_for_map });
                                    if let Ok(js2) = serde_wasm_bindgen::to_value(&args2) {
                                        let resp2 = invoke("load_process_agent_messages", js2).await;
                                        if !resp2.is_undefined() {
                                            if let Ok(stored) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(resp2) {
                                                // Update state only if we actually have stored content
                                                if !stored.is_empty() {
                                                    set_agent_messages.set(stored.clone());
                                                    set_messages_by_process.update(|map| {
                                                        map.insert(pid_for_map.clone(), stored.clone());
                                                    });
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    // Non-empty runtime messages: accept and persist
                                    set_agent_messages.update(|current| {
                                        current.clear();
                                        current.extend(messages.clone());
                                    });
                                    set_messages_by_process.update(|map| {
                                        map.insert(pid_for_map.clone(), messages.clone());
                                    });
                                    // Persist per-process and task-level snapshots
                                    let save_proc_args = serde_json::json!({
                                        "taskId": task_id_for_save,
                                        "processId": pid_for_map,
                                        "messages": messages,
                                    });
                                    if let Ok(save_proc_js) = serde_wasm_bindgen::to_value(&save_proc_args) {
                                        let _ = invoke("save_process_agent_messages", save_proc_js).await;
                                    }
                                    let save_task_args = serde_json::json!({
                                        "taskId": task_id_for_save,
                                        "messages": messages,
                                    });
                                    if let Ok(save_task_js) = serde_wasm_bindgen::to_value(&save_task_args) {
                                        let _ = invoke("save_task_agent_messages", save_task_js).await;
                                    }
                                }

                                // Sticky-scroll attempts: outer sessions and inner message list
                                let outer_id = format!("agent-sessions-{}", task_id_for_save);
                                let inner_id = format!("agent-messages-{}", pid_for_map);
                                scroll_to_bottom(&outer_id);
                                scroll_to_bottom(&inner_id);
                                // Schedule additional scrolls to handle late reflow (e.g., long diffs)
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
                        _ => {}
                    }
                }
            });
        }
    };

    // Load all processes (from memory and persisted store)
    let load_all_processes = {
        let set_all_processes = set_all_processes.clone();
        move || {
            spawn_local(async move {
                // Load current processes from memory
                let mut current_processes = Vec::new();
                match invoke("get_process_list", serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap()).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                            current_processes = processes;
                        }
                    }
                    _ => {}
                }

                // Also load persisted processes
                match invoke("load_agent_processes", serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap()).await {
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

                            set_all_processes.set(merged_processes.clone());

                            // Save the merged list back to store for persistence
                            let save_args = serde_json::json!({ "processes": merged_processes });
                            if let Ok(js_value) = serde_wasm_bindgen::to_value(&save_args) {
                                let _ = invoke("save_agent_processes", js_value).await;
                            }
                        } else {
                            set_all_processes.set(current_processes);
                        }
                    }
                    _ => {
                        set_all_processes.set(current_processes);
                    }
                }
            });
        }
    };


    // Load processes on component mount
    {
        let load_all_processes = load_all_processes.clone();
        spawn_local(async move {
            // Initial load
            load_all_processes();
        });
    }

    // Ensure messages for this task's processes are loaded from persisted store on startup
    {
        let task_id_for_effect = task.id.clone();
        let set_messages_by_process_effect = set_messages_by_process.clone();
        let set_agent_messages_effect = set_agent_messages.clone();
        Effect::new(move |_| {
            let procs = all_processes.get();
            let map_now = messages_by_process.get();
            for p in procs.iter().filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(&task_id_for_effect)) {
                if let Some(pid) = p.get("id").and_then(|v| v.as_str()) {
                    if !map_now.contains_key(pid) {
                        // Load from persisted per-process store
                        let pid_s = pid.to_string();
                        let task_id_arg = task_id_for_effect.clone();
                        let set_map = set_messages_by_process_effect.clone();
                        let set_current = set_agent_messages_effect.clone();
                        spawn_local(async move {
                            let args = serde_json::json!({ "taskId": task_id_arg, "processId": pid_s });
                            if let Ok(jsv) = serde_wasm_bindgen::to_value(&args) {
                                let resp = invoke("load_process_agent_messages", jsv).await;
                                if !resp.is_undefined() {
                                    if let Ok(stored) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(resp) {
                                        if !stored.is_empty() {
                                            let pid_for_map = pid_s.clone();
                                            set_map.update(|m| { m.insert(pid_for_map.clone(), stored.clone()); });
                                            // If this process is active, also reflect into set_agent_messages
                                            set_current.set(stored);
                                            // Sticky scroll after hydration
                                            let outer_id = format!("agent-sessions-{}", task_id_arg);
                                            let inner_id = format!("agent-messages-{}", pid_for_map);
                                            scroll_to_bottom(&outer_id);
                                            scroll_to_bottom(&inner_id);
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }
        });
    }

    // Load persisted agent messages for this task once and seed the latest process group
    {
        let task_id_for_persist = task.id.clone();
        let set_agent_messages_for_persist = set_agent_messages.clone();
        let all_processes_sig = all_processes.clone();
        let set_messages_by_process_sig = set_messages_by_process.clone();
        spawn_local(async move {
            let args = serde_json::json!({ "taskId": task_id_for_persist });
            if let Ok(js_value) = serde_wasm_bindgen::to_value(&args) {
                let resp = invoke("load_task_agent_messages", js_value).await;
                if !resp.is_undefined() {
                    if let Ok(persisted) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(resp) {
                        if !persisted.is_empty() {
                            set_agent_messages_for_persist.set(persisted.clone());
                            // Seed per-process from persisted per-process store if available
                            let procs = all_processes_sig.get_untracked();
                            for p in procs.iter().filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(&task_id_for_persist)) {
                                if let Some(pid) = p.get("id").and_then(|v| v.as_str()) {
                                    let args2 = serde_json::json!({ "taskId": task_id_for_persist, "processId": pid });
                                    if let Ok(js2) = serde_wasm_bindgen::to_value(&args2) {
                                        let resp2 = invoke("load_process_agent_messages", js2).await;
                                        if !resp2.is_undefined() {
                                            if let Ok(per_proc) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(resp2) {
                                                if !per_proc.is_empty() {
                                                    set_messages_by_process_sig.update(|m| { m.insert(pid.to_string(), per_proc); });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    // Auto-detect active process on initial load
    {
        let task_id = task.id.clone();
        let set_current_process_id = set_current_process_id.clone();
        let load_agent_messages = load_agent_messages.clone();

        // Initial process detection
        spawn_local(async move {
            // Check for active processes for this task and select the newest by start_time
            let args = serde_json::json!({});
            if let Ok(js_value) = to_value(&args) {
                match invoke("get_process_list", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                            // Find the latest process for this task by start_time (RFC3339 string; lexicographically sortable)
                            let mut latest: Option<serde_json::Value> = None;
                            for p in processes.iter().filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(&task_id)) {
                                let ts = p.get("start_time").and_then(|v| v.as_str()).unwrap_or("");
                                let lt = latest.as_ref().and_then(|lp| lp.get("start_time").and_then(|v| v.as_str())).unwrap_or("");
                                if ts >= lt { latest = Some(p.clone()); }
                            }
                            if let Some(proc) = latest {
                                if let Some(process_id) = proc.get("id").and_then(|v| v.as_str()) {
                                    set_current_process_id.set(Some(process_id.to_string()));
                                    load_agent_messages(process_id.to_string());
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        // Timer-based polling removed - now using event-driven updates only
    }

    // Set up Tauri event listener for real-time updates
    {
        let task_id_for_events = task.id.clone();
        let load_agent_messages_for_events = load_agent_messages.clone();
        let set_current_process_id_for_events = set_current_process_id.clone();

        spawn_local(async move {
            // Listen for agent message update events AND process status updates
            // Prefer our HTTP-safe SSE bridge to avoid native Tauri permission errors on http:// origins
            let listen_js = js_sys::Function::new_with_args(
                "eventName,handler",
                "if (window.AGENT_EVENT_LISTEN) { return window.AGENT_EVENT_LISTEN(eventName, handler); } else { return window.__TAURI__.event.listen(eventName, handler); }"
            );

            // Handler for message updates
            let task_id_msg = task_id_for_events.clone();
            let load_msg = load_agent_messages_for_events.clone();
            let set_id_msg = set_current_process_id_for_events.clone();
            let message_handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
                web_sys::console::log_1(&"📥 Received agent_message_update event".into());
                if let Ok(event_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(event) {
                    web_sys::console::log_1(&format!("📥 Event data: {:?}", event_data).into());
                    if let Some(payload) = event_data.get("payload") {
                        if let Some(event_task_id) = payload.get("task_id").and_then(|v| v.as_str()) {
                            web_sys::console::log_1(&format!("📥 Task ID: {} (looking for: {})", event_task_id, task_id_msg).into());
                            // Only handle events for this task
                            if event_task_id == task_id_msg {
                                if let Some(process_id) = payload.get("process_id").and_then(|v| v.as_str()) {
                                    web_sys::console::log_1(&format!("✅ Processing message event for process {}", process_id).into());
                                    // Update current process ID if needed
                                    set_id_msg.set(Some(process_id.to_string()));
                                    // Refresh messages for this process
                                    load_msg(process_id.to_string());
                                    // Message persistence now handled inside load_agent_messages after refresh
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(JsValue)>);

            // Handler for process status updates
            let task_id_status = task_id_for_events.clone();
            let load_status = load_agent_messages_for_events.clone();
            let set_id_status = set_current_process_id_for_events.clone();
            let load_all_processes_for_status = load_all_processes.clone();
            let status_handler = wasm_bindgen::closure::Closure::wrap(Box::new(move |event: JsValue| {
                web_sys::console::log_1(&"📊 Received agent_process_status event".into());
                if let Ok(event_data) = serde_wasm_bindgen::from_value::<serde_json::Value>(event) {
                    web_sys::console::log_1(&format!("📊 Status event data: {:?}", event_data).into());
                    if let Some(payload) = event_data.get("payload") {
                        if let Some(event_task_id) = payload.get("task_id").and_then(|v| v.as_str()) {
                            web_sys::console::log_1(&format!("📊 Task ID: {} (looking for: {})", event_task_id, task_id_status).into());
                            // Only handle events for this task
                            if event_task_id == task_id_status {
                                if let Some(process_id) = payload.get("process_id").and_then(|v| v.as_str()) {
                                    let status = payload.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                                    web_sys::console::log_1(&format!("✅ Processing status event: {} -> {}", process_id, status).into());
                                    // Update current process ID when process starts
                                    set_id_status.set(Some(process_id.to_string()));
                                    // Load initial messages for new process
                                    load_status(process_id.to_string());
                                    // CRITICAL: Also refresh process list to trigger UI re-render
                                    load_all_processes_for_status();

                                    // Save processes to store for persistence
                                    let processes_for_save = all_processes.get_untracked();
                                    let save_args = serde_json::json!({ "processes": processes_for_save });
                                    let _ = wasm_bindgen_futures::spawn_local(async move {
                                        if let Ok(jsv) = serde_wasm_bindgen::to_value(&save_args) {
                                            let _ = invoke("save_agent_processes", jsv).await;
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }) as Box<dyn FnMut(JsValue)>);

            // Set up both listeners
            let _ = listen_js.call2(
                &JsValue::NULL,
                &JsValue::from_str("agent_message_update"),
                message_handler.as_ref().unchecked_ref()
            );

            let _ = listen_js.call2(
                &JsValue::NULL,
                &JsValue::from_str("agent_process_status"),
                status_handler.as_ref().unchecked_ref()
            );

            // Keep the closures alive
            message_handler.forget();
            status_handler.forget();
        });
    }

    view! {
        <div class="task-sidebar">
            {/* Sidebar Header */}
            <div class="sidebar-header">
                <h2>{task_title.clone()}</h2>
                <div class="header-actions">
                    <button
                        class="action-btn edit-btn"
                        title="Edit Task"
                        on:click={
                            let task_for_edit = task.clone();
                            move |_| {
                                on_edit(task_for_edit.clone());
                            }
                        }
                    >"✎"</button>
                    <button
                        class="action-btn cancel-btn"
                        title="Move to Cancelled"
                        on:click={
                            let task_id_for_cancel = task_id.clone();
                            let on_update_status = on_update_status.clone();
                            move |_| {
                                on_update_status(task_id_for_cancel.clone(), TaskStatus::Cancelled);
                            }
                        }
                    >"⚠"</button>
                    <button
                        class="action-btn delete-btn"
                        title="Delete Task"
                        on:click={
                            let task_id_for_delete = task_id.clone();
                            move |_| {
                                on_delete(task_id_for_delete.clone());
                                // Close sidebar when task is deleted
                                selected_task.set(None);
                            }
                        }
                    >"🗑"</button>
                    <button class="sidebar-close" on:click=close_sidebar>"×"</button>
                </div>
            </div>

            {/* Task Details Section */}
            <div class="sidebar-content">
                <div class="task-details">
                    <div class="detail-section">
                        <h3>"Description"</h3>
                        <div class="task-description">
                            <p>{get_display_description}</p>
                            {description_is_long.then(|| view! {
                                <button
                                    class="show-more-btn"
                                    on:click=move |_| set_show_full_description.update(|show| *show = !*show)
                                >
                                    {move || if show_full_description.get() { "Show Less" } else { "Show More" }}
                                </button>
                            })}
                        </div>
                    </div>

                    {/* Status-Dependent Section */}
                    <div class="status-section">
                        {match task_status {
                            TaskStatus::ToDo => view! {
                                <div class="create-attempt">
                                    <h3>"Create Attempt"</h3>
                                    <div class="attempt-config">
                                        <div class="config-row">
                                            <label>"Base Branch:"</label>
                                            <select>
                                                <option value="main">"main"</option>
                                                <option value="develop">"develop"</option>
                                            </select>
                                        </div>
                                        <div class="config-row">
                                            <label>"Profile:"</label>
                                            <select on:change=move |ev| {
                                                let value = event_target_value(&ev);
                                                let profile = match value.as_str() {
                                                    "codex" => AgentProfile::Codex,
                                                    _ => AgentProfile::ClaudeCode,
                                                };
                                                set_selected_profile.set(profile);
                                            }>
                                                <option value="claude" selected=matches!(selected_profile.get(), AgentProfile::ClaudeCode)>"Claude Code"</option>
                                                <option value="codex" selected=matches!(selected_profile.get(), AgentProfile::Codex)>"Codex"</option>
                                            </select>
                                        </div>
                                        <div class="config-row">
                                            <label>"Variant:"</label>
                                            <select>
                                                <option value="standard">"Standard"</option>
                                                <option value="focused">"Focused"</option>
                                            </select>
                                        </div>
                                        <button
                                            class="start-btn"
                                            on:click={
                                                let task_id_for_start = task_id.clone();
                                                let on_update_status = on_update_status.clone();
                                                move |_| {
                                                    // Persist selected profile first if callback is provided
                                                    on_update_profile(task_id_for_start.clone(), selected_profile.get_untracked());
                                                    on_update_status(task_id_for_start.clone(), TaskStatus::InProgress);
                                                }
                                            }
                                        >"Start"</button>
                                    </div>
                                </div>
                            }.into_any(),
                            _ => view! {
                                <div class="attempt-status">
                                    <h3>"Attempt 1/1"</h3>
                                    <div class="status-info">
                                        <span class="profile-info">"Profile: default"</span>
                                        <span class="branch-info">{format!("Branch: task/{}", task.id)}</span>
                                        <span class="diff-info">"Diffs: " <span class="diff-added">"+0"</span> " " <span class="diff-removed">"-0"</span></span>
                                    </div>
                                </div>

                                {/* Worktree Actions - Only show for tasks with worktree, outside the attempt block */}
                                {task.worktree_path.as_ref().map(|worktree_path| {
                                    let worktree_path_for_files = worktree_path.clone();
                                    let worktree_path_for_ide = worktree_path.clone();
                                    view! {
                                        <div class="worktree-actions">
                                            <button
                                                class="action-btn files-btn"
                                                title="Open Files in Explorer"
                                                on:click={
                                                    let path = worktree_path_for_files.clone();
                                                    move |_| {
                                                        if let Some(ref callback) = on_open_worktree {
                                                            callback(path.clone());
                                                        }
                                                    }
                                                }
                                            >
                                                "🖿"
                                            </button>
                                            <button
                                                class="action-btn ide-btn"
                                                title="Open in VS Code"
                                                on:click={
                                                    let path = worktree_path_for_ide.clone();
                                                    move |_| {
                                                        if let Some(ref callback) = on_open_ide {
                                                            callback(path.clone());
                                                        }
                                                    }
                                                }
                                            >
                                                "🟐"
                                            </button>

                                            {/* Git Actions - TODO: Implement functionality */}
                                            <button
                                                class="action-btn pr-btn"
                                                title="Create Pull Request (disables worktree)"
                                                disabled=true
                                            >
                                                "🡽" {/* Alternative: 🞑 */}
                                            </button>
                                            <button
                                                class="action-btn merge-btn"
                                                title="Merge to Main"
                                                disabled=true
                                            >
                                                "🡺" {/* Alternative: 🞈 */}
                                            </button>
                                            <button
                                                class="action-btn rebase-btn"
                                                title="Rebase (Interactive)"
                                                disabled=true
                                            >
                                                "🡿" {/* Alternative: 🞴 */}
                                            </button>
                                        </div>
                                    }
                                })}
                            }.into_any()
                        }}
                    </div>
                </div>

                {/* Tabbed Interface - Only show for non-TODO statuses */}
                {match task_status {
                    TaskStatus::ToDo => view! {}.into_any(),
                    _ => {
                        // Tab state management
                        let (active_tab, set_active_tab) = signal("agents".to_string());

                        view! {
                            <div class="tabbed-interface">
                                {/* Tab Headers */}
                                <div class="tab-headers">
                                    <button
                                        class=move || format!("tab-header {}", if active_tab.get() == "agents" { "active" } else { "" })
                                        on:click={
                                            let set_active_tab = set_active_tab.clone();
                                            let load_agent_messages = load_agent_messages.clone();
                                            let current_process_id = current_process_id.clone();
                                            let task_id_for_scroll = task_id.clone();
                                            move |_| {
                                                set_active_tab.set("agents".to_string());
                                                // Refresh messages when agents tab is clicked
                                                if let Some(process_id) = current_process_id.get_untracked() {
                                                    load_agent_messages(process_id);
                                                }
                                                // Force scroll to bottom when switching to Agents tab
                                                let tid = task_id_for_scroll.clone();
                                                spawn_local(async move {
                                                    if let Some(window) = web_sys::window() {
                                                        if let Some(doc) = window.document() {
                                                            let container_id = format!("agent-sessions-{}", tid);
                                                            if let Some(el) = doc.get_element_by_id(&container_id) {
                                                                use wasm_bindgen::JsCast;
                                                                if let Ok(div) = el.dyn_into::<web_sys::HtmlElement>() {
                                                                    let scroll_height = div.scroll_height() as i32;
                                                                    div.set_scroll_top(scroll_height);
                                                                }
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    >"Agents"</button>
                                    <button
                                        class=move || format!("tab-header {}", if active_tab.get() == "diff" { "active" } else { "" })
                                        on:click=move |_| set_active_tab.set("diff".to_string())
                                    >"Diff"</button>
                                    <button
                                        class=move || format!("tab-header {}", if active_tab.get() == "processes" { "active" } else { "" })
                                        on:click={
                                            let set_active_tab = set_active_tab.clone();
                                            let load_all_processes = load_all_processes.clone();
                                            move |_| {
                                                set_active_tab.set("processes".to_string());
                                                // Refresh process list when processes tab is clicked
                                                load_all_processes();
                                            }
                                        }
                                    >"Processes"</button>
                                </div>

                                {/* Tab Content */}
                                <div class="tab-content">
                                    { move || { let task_id_for_closure = task_id.clone(); match active_tab.get().as_str() {
                                        "agents" => {
                                            let messages = agent_messages.get();
                                            let processes = all_processes.get();
                                            let has_messages = !messages.is_empty();

                                            // Check if there are any processes for this task
                                            let current_task_processes: Vec<_> = processes.iter()
                                                .filter(|proc| proc.get("task_id").and_then(|v| v.as_str()) == Some(&task.id))
                                                .collect();

                                            let has_processes = !current_task_processes.is_empty();
                                            let latest_process_status = current_task_processes.last()
                                                .and_then(|proc| proc.get("status").and_then(|v| v.as_str()));

                                            // Compact process chips for quick switching
                                            let chips = current_task_processes.iter().map(|proc| {
                                                let pid = proc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let status = proc.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                                                let kind = proc.get("kind").and_then(|v| v.as_str()).unwrap_or("");
                                                let is_active = current_process_id.get().as_deref() == Some(pid.as_str());
                                                view! {
                                                    <button class=move || format!("process-chip {}", if is_active {"active"} else {""}) on:click={
                                                        let pidc = pid.clone(); let set_current = set_current_process_id.clone(); let load = load_agent_messages.clone();
                                                        move |_| { set_current.set(Some(pidc.clone())); load(pidc.clone()); }
                                                    }>
                                                        {format!("{} · {}", &pid[..pid.len().min(6)], status)}
                                                        <span class="kind">{format!(" {}", kind)}</span>
                                                    </button>
                                                }
                                            }).collect::<Vec<_>>();

                                            view! {
                                                <div class="agents-tab">
                                                    <crate::components::agents::AgentsPanel
                                                        task_id=task_id_for_closure.clone()
                                                        processes=all_processes.get()
                                                        messages_by_process=messages_by_process
                                                        on_load_messages={
                                                            let loader = load_agent_messages.clone();
                                                            std::rc::Rc::new(move |pid: String| { loader(pid); })
                                                        }
                                                        active_process_id=current_process_id
                                                     />

                                                    {/* Chat Input */}
                                                    <div class="chat-input-section">
                                                        <div class="input-container">
                                                            <button class="profile-btn" title="Agent profile from previous run" disabled=true>
                                                                {match selected_profile.get() {
                                                                    AgentProfile::ClaudeCode => "Claude",
                                                                    AgentProfile::Codex => "Codex",
                                                                }}
                                                            </button>
                                                            <input
                                                                type="text"
                                                                placeholder={
                                                                    if current_process_id.get().is_some() && worktree_available {
                                                                        "Type a reply and press Send"
                                                                    } else {
                                                                        "Start an agent first to enable replies"
                                                                    }
                                                                }
                                                                class="message-input"
                                                                on:input=move |ev| set_message_input.set(event_target_value(&ev))
                                                                on:keydown={
                                                                    let current_process_id = current_process_id.clone();
                                                                    let message_input = message_input.clone();
                                                                    let set_message_input = set_message_input.clone();
                                                                    let set_is_sending_message = set_is_sending_message.clone();
                                                                    let set_current_process_id = set_current_process_id.clone();
                                                                    let load_agent_messages = load_agent_messages.clone();
                                                                    let load_all2 = load_all_processes.clone();
                                                                    let worktree_opt = _task_worktree_path.clone();
                                                                    let task_id_for_keydown = task_id_for_closure.clone();
                                                                    move |ev| {
                                                                        let ke: web_sys::KeyboardEvent = ev.clone().unchecked_into();
                                                                        if ke.key() == "Enter" && !ke.shift_key() {
                                                                            ev.prevent_default();
                                                                            if current_process_id.get().is_none() { return; }
                                                                            if worktree_opt.is_none() { return; }
                                                                            let pid = current_process_id.get().unwrap();
                                                                            let worktree_path = worktree_opt.clone().unwrap();
                                                                            let msg = message_input.get();
                                                                            if msg.trim().is_empty() { return; }
                                                                            set_is_sending_message.set(true);
                                                                            let lam = load_agent_messages.clone();
                                                                            let tid_for_proc = task_id_for_keydown.clone();
                                                                            let now = chrono::Utc::now().to_rfc3339();
                                                                            let kind_str = match selected_profile.get_untracked() {
                                                                                AgentProfile::ClaudeCode => "claude".to_string(),
                                                                                AgentProfile::Codex => "codex".to_string(),
                                                                            };
                                                                    let set_all = set_all_processes.clone();
                                                                            let tid_for_proc_value = tid_for_proc.clone();
                                                                            let now_value = now.clone();
                                                                            let kind_value = kind_str.clone();
                                                                            spawn_local(async move {
                                                                                let args = serde_json::json!({
                                                                                    "processId": pid,
                                                                                    "message": msg,
                                                                                    "worktreePath": worktree_path,
                                                                                });
                                                                                if let Ok(js_value) = to_value(&args) {
                                                                                    let resp = invoke("send_agent_message", js_value).await;
                                                                                    if !resp.is_undefined() {
                                                                                    if let Ok(new_pid) = serde_wasm_bindgen::from_value::<String>(resp) {
                                                                                        set_current_process_id.set(Some(new_pid.clone()));
                                                                                        set_message_input.set(String::new());
                                                                                        // Optimistically add new process to the list so its group appears immediately
                                                                                        set_all.update(|procs| {
                                                                                            procs.push(serde_json::json!({
                                                                                                "id": new_pid.clone(),
                                                                                                "task_id": tid_for_proc_value,
                                                                                                "status": "starting",
                                                                                                "start_time": now_value,
                                                                                                "message_count": 0,
                                                                                                "kind": kind_value
                                                                                            }));
                                                                                        });
                                                                                        lam(new_pid);
                                                                                        let _ = gloo_timers::future::TimeoutFuture::new(300).await;
                                                                                        load_all2();
                                                                                    }
                                                                                }
                                                                            }
                                                                            set_is_sending_message.set(false);
                                                                        });
                                                                        }
                                                                    }
                                                                }
                                                                prop:value=move || message_input.get()
                                                                disabled={move || current_process_id.get().is_none() || !worktree_available || is_sending_message.get()}
                                                            />
                                                            <button
                                                                class="send-btn"
                                                                disabled={move || current_process_id.get().is_none() || !worktree_available || message_input.get().trim().is_empty() || is_sending_message.get()}
                                                                on:click={
                                                                    let current_process_id = current_process_id.clone();
                                                                    let message_input = message_input.clone();
                                                                    let set_message_input = set_message_input.clone();
                                                                    let set_is_sending_message = set_is_sending_message.clone();
                                                                    let set_current_process_id = set_current_process_id.clone();
                                                                    let load_agent_messages = load_agent_messages.clone();
                                                                    let worktree_opt = _task_worktree_path.clone();
                                                                    let set_all = set_all_processes.clone();
                                                                    let load_all3 = load_all_processes.clone();
                                                                    let task_id_for_click = task_id_for_closure.clone();
                                                                    let now_click = chrono::Utc::now().to_rfc3339();
                                                                    let kind_click = match selected_profile.get_untracked() {
                                                                        AgentProfile::ClaudeCode => "claude".to_string(),
                                                                        AgentProfile::Codex => "codex".to_string(),
                                                                    };
                                                                    move |_| {
                                                                        if current_process_id.get().is_none() { return; }
                                                                        if worktree_opt.is_none() { return; }
                                                                        let pid = current_process_id.get().unwrap();
                                                                        let worktree_path = worktree_opt.clone().unwrap();
                                                                        let msg = message_input.get();
                                                                        if msg.trim().is_empty() { return; }
                                                                        set_is_sending_message.set(true);
                                                                        let lam = load_agent_messages.clone();
                                                                        let tid_for_proc_value = task_id_for_click.clone();
                                                                        let now_click_value = now_click.clone();
                                                                        let kind_click_value = kind_click.clone();
                                                                        spawn_local(async move {
                                                                            let args = serde_json::json!({
                                                                                "processId": pid,
                                                                                "message": msg,
                                                                                "worktreePath": worktree_path,
                                                                            });
                                                                            if let Ok(js_value) = to_value(&args) {
                                                                                let resp = invoke("send_agent_message", js_value).await;
                                                                                // Expect a new process_id string
                                                                                if !resp.is_undefined() {
                                                                                    if let Ok(new_pid) = serde_wasm_bindgen::from_value::<String>(resp) {
                                                                                        set_current_process_id.set(Some(new_pid.clone()));
                                                                                        // Clear input and load messages for the new process immediately
                                                                                        set_message_input.set(String::new());
                                                                                        // Optimistically add new process so its group appears instantly
                                                                                        set_all.update(|procs| {
                                                                                            procs.push(serde_json::json!({
                                                                                                "id": new_pid.clone(),
                                                                                                "task_id": tid_for_proc_value,
                                                                                                "status": "starting",
                                                                                                "start_time": now_click_value,
                                                                                                "message_count": 0,
                                                                                                "kind": kind_click_value
                                                                                            }));
                                                                                        });
                                                                                        lam(new_pid);
                                                                                        let _ = gloo_timers::future::TimeoutFuture::new(300).await;
                                                                                        load_all3();
                                                                                    }
                                                                                }
                                                                            }
                                                                            set_is_sending_message.set(false);
                                                                        });
                                                                    }
                                                                }
                                                            >
                                                                "Send"
                                                            </button>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }.into_any(),
                                        "diff" => view! { <crate::components::agents::DiffTab /> }.into_any(),
                                        "processes" => view! { <crate::components::agents::ProcessesTab processes=all_processes.get() task_id=task_id_for_closure.clone() /> }.into_any(),
                                        _ => view! {}.into_any()
                                    } }
                                }
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}
