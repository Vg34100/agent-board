use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::models::{Task, TaskStatus, AgentProfile};
use std::rc::Rc;
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
    let (_message_input, _set_message_input) = signal(String::new());
    let (_is_sending_message, _set_is_sending_message) = signal(false);
    
    // Clone task data for use in closures
    let task_title = task.title.clone();
    let task_description = task.description.clone();
    let task_status = task.status.clone();
    let task_id = task.id.clone();
    let _task_worktree_path = task.worktree_path.clone();
    
    // Local selected profile (default from task)
    let (selected_profile, set_selected_profile) = signal(task.profile.clone());

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

    // Load agent messages for current process with force refresh
    let load_agent_messages = {
        let set_agent_messages = set_agent_messages.clone();
        move |process_id: String| {
            let set_agent_messages = set_agent_messages.clone();
            spawn_local(async move {
                let args = serde_json::json!({ "processId": process_id });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("get_agent_messages", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(messages) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(js_result) {
                                // Use update to ensure reactivity even if content is similar
                                set_agent_messages.update(|current| {
                                    current.clear();
                                    current.extend(messages);
                                });
                            }
                        }
                        _ => {}
                    }
                }
            });
        }
    };

    // Load all processes
    let load_all_processes = {
        let set_all_processes = set_all_processes.clone();
        move || {
            spawn_local(async move {
                match invoke("get_process_list", serde_wasm_bindgen::to_value(&serde_json::json!({})).unwrap()).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                            set_all_processes.set(processes);
                        }
                    }
                    _ => {}
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

    // Load persisted agent messages for this task once
    {
        let task_id_for_persist = task.id.clone();
        let set_agent_messages_for_persist = set_agent_messages.clone();
        spawn_local(async move {
            let args = serde_json::json!({ "taskId": task_id_for_persist });
            if let Ok(js_value) = serde_wasm_bindgen::to_value(&args) {
                let resp = invoke("load_task_agent_messages", js_value).await;
                if !resp.is_undefined() {
                    if let Ok(persisted) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(resp) {
                        if !persisted.is_empty() {
                            set_agent_messages_for_persist.set(persisted);
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
            // Check for active processes for this task
            let args = serde_json::json!({});
            if let Ok(js_value) = to_value(&args) {
                match invoke("get_process_list", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(processes) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                            // Find the latest process for this task
                            if let Some(proc) = processes.iter()
                                .filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(&task_id))
                                .last() {
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
            let listen_js = js_sys::Function::new_with_args(
                "eventName,handler",
                "return window.__TAURI__.event.listen(eventName, handler)"
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
                                    // Also persist current messages snapshot for this task
                                    let task_id_copy = task_id_msg.to_string();
                                    let messages_snapshot = agent_messages.get_untracked();
                                    let save_args = serde_json::json!({
                                        "taskId": task_id_copy,
                                        "messages": messages_snapshot
                                    });
                                    let _ = wasm_bindgen_futures::spawn_local(async move {
                                        if let Ok(jsv) = serde_wasm_bindgen::to_value(&save_args) {
                                            let _ = invoke("save_task_agent_messages", jsv).await;
                                        }
                                    });
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
                                            move |_| {
                                                set_active_tab.set("agents".to_string());
                                                // Refresh messages when agents tab is clicked
                                                if let Some(process_id) = current_process_id.get_untracked() {
                                                    load_agent_messages(process_id);
                                                }
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
                                    {move || match active_tab.get().as_str() {
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
                                            
                                            view! {
                                                <div class="agents-tab">
                                                    <div class="agent-sessions">
                                                        {if !has_processes {
                                                            // No processes started yet
                                                            view! {
                                                                <div class="no-agents">
                                                                    <p>"No agent processes yet"</p>
                                                                    <p class="hint">"Agent will be spawned automatically when you start the task"</p>
                                                                </div>
                                                            }.into_any()
                                                        } else if let Some(status) = latest_process_status {
                                                            match status {
                                                                "starting" => {
                                                                    view! {
                                                                        <div class="agent-loading">
                                                                            <div class="loading-indicator">
                                                                                <span class="spinner">"⏳"</span>
                                                                                <p>"Starting agent..."</p>
                                                                            </div>
                                                                            <p class="hint">"Setting up process and connecting to worktree"</p>
                                                                        </div>
                                                                    }.into_any()
                                                                },
                                                                "running" => {
                                                                    if has_messages {
                                                                        view! {
                                                                            <div class="message-list">
                                                                                {
                                                                                    // Group agent_reasoning messages together
                                                                                    let mut processed_messages = Vec::new();
                                                                                    let mut reasoning_messages = Vec::new();
                                                                                    
                                                                                    for msg in messages.into_iter().filter(|msg| !(msg.sender == "system" && msg.content.trim().is_empty())) {
                                                                                        if msg.message_type == "agent_reasoning" {
                                                                                            reasoning_messages.push(msg);
                                                                                        } else {
                                                                                            // If we have accumulated reasoning messages, add them as a group first
                                                                                            if !reasoning_messages.is_empty() {
                                                                                                let reasoning_count = reasoning_messages.len();
                                                                                                let reasoning_preview = reasoning_messages.first().map(|m| {
                                                                                                    if m.content.len() > 100 {
                                                                                                        format!("{}...", &m.content[..100])
                                                                                                    } else {
                                                                                                        m.content.clone()
                                                                                                    }
                                                                                                }).unwrap_or_default();
                                                                                                
                                                                                                let reasoning_details = reasoning_messages.clone();
                                                                                                processed_messages.push(view! {
                                                                                                    <div class="message agent reasoning-group">
                                                                                                        <div class="message-header">
                                                                                                            <span class="message-icon">"🧠"</span>
                                                                                                            <span class="sender">"agent"</span>
                                                                                                            <span class="reasoning-count">"{reasoning_count} thoughts"</span>
                                                                                                        </div>
                                                                                                        <div class="message-content">
                                                                                                            <details>
                                                                                                                <summary>{reasoning_preview}</summary>
                                                                                                                <div class="reasoning-details">
                                                                                                                    {reasoning_details.into_iter().map(|reasoning_msg| {
                                                                                                                        view! {
                                                                                                                            <div class="reasoning-item">
                                                                                                                                <div class="reasoning-content">{reasoning_msg.content}</div>
                                                                                                                            </div>
                                                                                                                        }
                                                                                                                    }).collect::<Vec<_>>()}
                                                                                                                </div>
                                                                                                            </details>
                                                                                                        </div>
                                                                                                    </div>
                                                                                                }.into_any());
                                                                                                reasoning_messages.clear();
                                                                                            }
                                                                                            
                                                                                            // Add the current non-reasoning message
                                                                                            let icon = match msg.sender.as_str() {
                                                                                                "user" => "👤",
                                                                                                "agent" => match msg.message_type.as_str() {
                                                                                                    "file_read" => "👁",
                                                                                                    "file_edit" => "✏️",
                                                                                                    "file_edit_start" => "📝",
                                                                                                    "file_edit_end" => "✅",
                                                                                                    "file_diff" => "📄",
                                                                                                    "tool_call" => "🔧",
                                                                                                    _ => "🤖"
                                                                                                },
                                                                                                "system" => "🔧",
                                                                                                _ => "💬"
                                                                                            };
                                                                                            
                                                                                            let message_class = format!("message {}", msg.sender);
                                                                                            
                                                                                            processed_messages.push(view! {
                                                                                                <div class=message_class>
                                                                                                    <div class="message-header">
                                                                                                        <span class="message-icon">{icon}</span>
                                                                                                        <span class="sender">{msg.sender.clone()}</span>
                                                                                                        <span class="time">{msg.timestamp.clone()}</span>
                                                                                                    </div>
                                                                                                    <div class="message-content">
                                                                                                        {msg.content}
                                                                                                    </div>
                                                                                                </div>
                                                                                            }.into_any());
                                                                                        }
                                                                                    }
                                                                                    
                                                                                    // Don't forget any remaining reasoning messages at the end
                                                                                    if !reasoning_messages.is_empty() {
                                                                                        let reasoning_count = reasoning_messages.len();
                                                                                        let reasoning_preview = reasoning_messages.first().map(|m| {
                                                                                            if m.content.len() > 100 {
                                                                                                format!("{}...", &m.content[..100])
                                                                                            } else {
                                                                                                m.content.clone()
                                                                                            }
                                                                                        }).unwrap_or_default();
                                                                                        
                                                                                        let reasoning_details = reasoning_messages.clone();
                                                                                        processed_messages.push(view! {
                                                                                            <div class="message agent reasoning-group">
                                                                                                <div class="message-header">
                                                                                                    <span class="message-icon">"🧠"</span>
                                                                                                    <span class="sender">"agent"</span>
                                                                                                    <span class="reasoning-count">"{reasoning_count} thoughts"</span>
                                                                                                </div>
                                                                                                <div class="message-content">
                                                                                                    <details>
                                                                                                        <summary>{reasoning_preview}</summary>
                                                                                                        <div class="reasoning-details">
                                                                                                            {reasoning_details.into_iter().map(|reasoning_msg| {
                                                                                                                view! {
                                                                                                                    <div class="reasoning-item">
                                                                                                                        <div class="reasoning-content">{reasoning_msg.content}</div>
                                                                                                                    </div>
                                                                                                                }
                                                                                                            }).collect::<Vec<_>>()}
                                                                                                        </div>
                                                                                                    </details>
                                                                                                </div>
                                                                                            </div>
                                                                                        }.into_any());
                                                                                    }
                                                                                    
                                                                                    processed_messages
                                                                                }
                                                                            </div>
                                                                        }.into_any()
                                                                    } else {
                                                                        view! {
                                                                            <div class="agent-running">
                                                                                <div class="status-indicator">
                                                                                    <span class="spinner">"🤖"</span>
                                                                                    <p>"Agent is running..."</p>
                                                                                </div>
                                                                                <p class="hint">"Waiting for agent to process your request"</p>
                                                                            </div>
                                                                        }.into_any()
                                                                    }
                                                                },
                                                                "failed" => {
                                                                    view! {
                                                                        <div class="agent-error">
                                                                            <div class="error-indicator">
                                                                                <span class="error-icon">"❌"</span>
                                                                                <p>"Agent process failed to start"</p>
                                                                            </div>
                                                                            <p class="error-details">"Check that Claude Code CLI is installed and accessible"</p>
                                                                            <p class="hint">"Try starting the task again or check the Processes tab for details"</p>
                                                                        </div>
                                                                    }.into_any()
                                                                },
                                                                "completed" => {
                                                                    view! {
                                                                        <div class="agent-completed">
                                                                            <div class="completion-indicator">
                                                                                <span class="complete-icon">"✅"</span>
                                                                                <p>"Agent process completed successfully"</p>
                                                                            </div>
                                                                            {if has_messages {
                                                                                view! {
                                                                                    <div class="message-list">
                                                                                        {
                                                                                            // Group agent_reasoning messages together for completed status too
                                                                                            let mut processed_messages = Vec::new();
                                                                                            let mut reasoning_messages = Vec::new();
                                                                                            
                                                                                            for msg in messages.into_iter().filter(|msg| !(msg.sender == "system" && msg.content.trim().is_empty())) {
                                                                                                if msg.message_type == "agent_reasoning" {
                                                                                                    reasoning_messages.push(msg);
                                                                                                } else {
                                                                                                    // If we have accumulated reasoning messages, add them as a group first
                                                                                                    if !reasoning_messages.is_empty() {
                                                                                                        let reasoning_count = reasoning_messages.len();
                                                                                                        let reasoning_preview = reasoning_messages.first().map(|m| {
                                                                                                            if m.content.len() > 100 {
                                                                                                                format!("{}...", &m.content[..100])
                                                                                                            } else {
                                                                                                                m.content.clone()
                                                                                                            }
                                                                                                        }).unwrap_or_default();
                                                                                                        
                                                                                                        let reasoning_details = reasoning_messages.clone();
                                                                                                        processed_messages.push(view! {
                                                                                                            <div class="message agent reasoning-group">
                                                                                                                <div class="message-header">
                                                                                                                    <span class="message-icon">"🧠"</span>
                                                                                                                    <span class="sender">"agent"</span>
                                                                                                                    <span class="reasoning-count">"{reasoning_count} thoughts"</span>
                                                                                                                </div>
                                                                                                                <div class="message-content">
                                                                                                                    <details>
                                                                                                                        <summary>{reasoning_preview}</summary>
                                                                                                                        <div class="reasoning-details">
                                                                                                                            {reasoning_details.into_iter().map(|reasoning_msg| {
                                                                                                                                view! {
                                                                                                                                    <div class="reasoning-item">
                                                                                                                                        <div class="reasoning-content">{reasoning_msg.content}</div>
                                                                                                                                    </div>
                                                                                                                                }
                                                                                                                            }).collect::<Vec<_>>()}
                                                                                                                        </div>
                                                                                                                    </details>
                                                                                                                </div>
                                                                                                            </div>
                                                                                                        }.into_any());
                                                                                                        reasoning_messages.clear();
                                                                                                    }
                                                                                                    
                                                                                                    // Add the current non-reasoning message
                                                                                                    let icon = match msg.sender.as_str() {
                                                                                                        "user" => "👤",
                                                                                                        "agent" => match msg.message_type.as_str() {
                                                                                                            "file_read" => "👁",
                                                                                                            "file_edit" => "✏️",
                                                                                                            "file_edit_start" => "📝",
                                                                                                            "file_edit_end" => "✅",
                                                                                                            "file_diff" => "📄",
                                                                                                            "tool_call" => "🔧",
                                                                                                            _ => "🤖"
                                                                                                        },
                                                                                                        "system" => "🔧",
                                                                                                        _ => "💬"
                                                                                                    };
                                                                                                    
                                                                                                    let message_class = format!("message {}", msg.sender);
                                                                                                    
                                                                                                    processed_messages.push(view! {
                                                                                                        <div class=message_class>
                                                                                                            <div class="message-header">
                                                                                                                <span class="message-icon">{icon}</span>
                                                                                                                <span class="sender">{msg.sender.clone()}</span>
                                                                                                                <span class="time">{msg.timestamp.clone()}</span>
                                                                                                            </div>
                                                                                                            <div class="message-content">
                                                                                                                {msg.content}
                                                                                                            </div>
                                                                                                        </div>
                                                                                                    }.into_any());
                                                                                                }
                                                                                            }
                                                                                            
                                                                                            // Don't forget any remaining reasoning messages at the end
                                                                                            if !reasoning_messages.is_empty() {
                                                                                                let reasoning_count = reasoning_messages.len();
                                                                                                let reasoning_preview = reasoning_messages.first().map(|m| {
                                                                                                    if m.content.len() > 100 {
                                                                                                        format!("{}...", &m.content[..100])
                                                                                                    } else {
                                                                                                        m.content.clone()
                                                                                                    }
                                                                                                }).unwrap_or_default();
                                                                                                
                                                                                                let reasoning_details = reasoning_messages.clone();
                                                                                                processed_messages.push(view! {
                                                                                                    <div class="message agent reasoning-group">
                                                                                                        <div class="message-header">
                                                                                                            <span class="message-icon">"🧠"</span>
                                                                                                            <span class="sender">"agent"</span>
                                                                                                            <span class="reasoning-count">"{reasoning_count} thoughts"</span>
                                                                                                        </div>
                                                                                                        <div class="message-content">
                                                                                                            <details>
                                                                                                                <summary>{reasoning_preview}</summary>
                                                                                                                <div class="reasoning-details">
                                                                                                                    {reasoning_details.into_iter().map(|reasoning_msg| {
                                                                                                                        view! {
                                                                                                                            <div class="reasoning-item">
                                                                                                                                <div class="reasoning-content">{reasoning_msg.content}</div>
                                                                                                                            </div>
                                                                                                                        }
                                                                                                                    }).collect::<Vec<_>>()}
                                                                                                                </div>
                                                                                                            </details>
                                                                                                        </div>
                                                                                                    </div>
                                                                                                }.into_any());
                                                                                            }
                                                                                            
                                                                                            processed_messages
                                                                                        }
                                                                                    </div>
                                                                                }.into_any()
                                                                            } else {
                                                                                view! {
                                                                                    <p class="hint">"No messages captured during execution"</p>
                                                                                }.into_any()
                                                                            }}
                                                                        </div>
                                                                    }.into_any()
                                                                },
                                                                _ => {
                                                                    view! {
                                                                        <div class="agent-unknown">
                                                                            <p>{format!("Agent status: {}", status)}</p>
                                                                        </div>
                                                                    }.into_any()
                                                                }
                                                            }
                                                        } else {
                                                            view! {
                                                                <div class="no-agents">
                                                                    <p>"Agent process status unknown"</p>
                                                                </div>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                    
                                                    {/* Chat Input - TODO: Enable when interactive chat is implemented */}
                                                    <div class="chat-input-section">
                                                        <div class="input-container">
                                                            <button class="profile-btn" disabled=true>"Profile"</button>
                                                            <input 
                                                                type="text" 
                                                                placeholder="Interactive chat coming soon..."
                                                                class="message-input"
                                                                disabled=true
                                                            />
                                                            <button class="send-btn" disabled=true>"Send"</button>
                                                        </div>
                                                    </div>
                                                </div>
                                            }
                                        }.into_any(),
                                        "diff" => view! {
                                            <div class="diff-tab">
                                                <div class="diff-content">
                                                    <div class="placeholder-content">
                                                        <h4>"File Diffs"</h4>
                                                        <p>"TODO: Show file changes from worktree"</p>
                                                        <p class="hint">"This will display modified files with +/- line changes"</p>
                                                    </div>
                                                </div>
                                            </div>
                                        }.into_any(),
                                        "processes" => {
                                            let processes = all_processes.get();
                                            let has_processes = !processes.is_empty();
                                            
                                            view! {
                                                <div class="processes-tab">
                                                    <div class="process-list">
                                                        <h4>"Agent Processes"</h4>
                                                        {if !has_processes {
                                                            view! {
                                                                <div class="no-processes">
                                                                    <p>"No processes spawned yet"</p>
                                                                    <p class="hint">"Process details with expandable JSON will appear here"</p>
                                                                </div>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <div class="process-items">
                                                                    {processes.into_iter().map(|proc| {
                                                                        let proc_id = proc.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                                                        let status = proc.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                                                        let msg_count = proc.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                                                        let task_id = proc.get("task_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                                                        let json_content = serde_json::to_string_pretty(&proc).unwrap_or_else(|_| "Invalid JSON".to_string());
                                                                        
                                                                        let status_class = format!("process-status {}", status);
                                                                        let proc_id_short = format!("{}...", &proc_id[..8.min(proc_id.len())]);
                                                                        let task_id_display = format!("Task: {}", task_id);
                                                                        let status_display = status.clone();
                                                                        let msg_count_display = format!("{} msgs", msg_count);
                                                                        
                                                                        view! {
                                                                            <div class="process-item">
                                                                                <div class="process-header">
                                                                                    <span class="process-id">{proc_id_short}</span>
                                                                                    <span class="task-id">{task_id_display}</span>
                                                                                    <span class=status_class>{status_display}</span>
                                                                                    <span class="message-count">{msg_count_display}</span>
                                                                                </div>
                                                                                <details>
                                                                                    <summary>"Show JSON Details"</summary>
                                                                                    <pre class="json-content">{json_content}</pre>
                                                                                </details>
                                                                            </div>
                                                                        }
                                                                    }).collect::<Vec<_>>()}
                                                                </div>
                                                            }.into_any()
                                                        }}
                                                    </div>
                                                </div>
                                            }
                                        }.into_any(),
                                        _ => view! {}.into_any()
                                    }}
                                </div>
                            </div>
                        }.into_any()
                    }
                }}
            </div>
        </div>
    }
}

