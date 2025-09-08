use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::models::{Task, TaskStatus};
use std::rc::Rc;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
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
    #[prop(into, optional)] active_process_id: Option<RwSignal<Option<String>>>,
) -> impl IntoView {
    // State for showing/hiding full description
    let (show_full_description, set_show_full_description) = signal(false);
    
    // Agent process state
    let (agent_messages, set_agent_messages) = signal(Vec::<AgentMessage>::new());
    let (all_processes, set_all_processes) = signal(Vec::<serde_json::Value>::new());
    let (current_process_id, set_current_process_id) = signal(Option::<String>::None);
    let (message_input, set_message_input) = signal(String::new());
    let (is_sending_message, set_is_sending_message) = signal(false);
    
    // Clone task data for use in closures
    let task_title = task.title.clone();
    let task_description = task.description.clone();
    let task_status = task.status.clone();
    let task_id = task.id.clone();
    let task_worktree_path = task.worktree_path.clone();
    
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

    // Load agent messages for current process
    let load_agent_messages = {
        let set_agent_messages = set_agent_messages.clone();
        move |process_id: String| {
            spawn_local(async move {
                let args = serde_json::json!({ "process_id": process_id });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("get_agent_messages", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(messages) = serde_wasm_bindgen::from_value::<Vec<AgentMessage>>(js_result) {
                                set_agent_messages.set(messages);
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

    // Send message callback
    let handle_send_message = {
        let set_is_sending_message = set_is_sending_message.clone();
        let set_message_input = set_message_input.clone();
        let set_current_process_id = set_current_process_id.clone();
        let load_agent_messages = load_agent_messages.clone();
        let task_worktree_path = task_worktree_path.clone();
        let current_process_id = current_process_id.clone();
        let message_input = message_input.clone();
        
        move |_: web_sys::MouseEvent| {
            let message = message_input.get_untracked();
            if let Some(process_id) = current_process_id.get_untracked() {
                if message.trim().is_empty() || task_worktree_path.is_none() {
                    return;
                }
                
                let worktree_path = task_worktree_path.clone().unwrap();
                set_is_sending_message.set(true);
                set_message_input.set(String::new());
                
                let set_current_process_id = set_current_process_id.clone();
                let load_agent_messages = load_agent_messages.clone();
                let set_is_sending_message = set_is_sending_message.clone();
                
                spawn_local(async move {
                    let args = serde_json::json!({
                        "process_id": process_id,
                        "message": message,
                        "worktree_path": worktree_path
                    });
                    
                    if let Ok(js_value) = to_value(&args) {
                        match invoke("send_agent_message", js_value).await {
                            js_result if !js_result.is_undefined() => {
                                if let Ok(new_process_id) = serde_wasm_bindgen::from_value::<String>(js_result) {
                                    set_current_process_id.set(Some(new_process_id.clone()));
                                    load_agent_messages(new_process_id);
                                }
                            }
                            _ => {}
                        }
                    }
                    set_is_sending_message.set(false);
                });
            }
        }
    };

    // Load processes on component mount
    {
        let load_all_processes = load_all_processes.clone();
        spawn_local(async move {
            load_all_processes();
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
                                            <select>
                                                <option value="default">"Default"</option>
                                                <option value="expert">"Expert"</option>
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
                                        on:click=move |_| set_active_tab.set("agents".to_string())
                                    >"Agents"</button>
                                    <button 
                                        class=move || format!("tab-header {}", if active_tab.get() == "diff" { "active" } else { "" })
                                        on:click=move |_| set_active_tab.set("diff".to_string())
                                    >"Diff"</button>
                                    <button 
                                        class=move || format!("tab-header {}", if active_tab.get() == "processes" { "active" } else { "" })
                                        on:click=move |_| set_active_tab.set("processes".to_string())
                                    >"Processes"</button>
                                </div>
                                
                                {/* Tab Content */}
                                <div class="tab-content">
                                    {move || match active_tab.get().as_str() {
                                        "agents" => view! {
                                            <div class="agents-tab">
                                                <div class="agent-sessions">
                                                    <div class="no-agents">
                                                        <p>"No agent messages yet"</p>
                                                        <p class="hint">"Agent will be spawned automatically when you start the task"</p>
                                                    </div>
                                                </div>
                                                
                                                {/* Chat Input */}
                                                <div class="chat-input-section">
                                                    <div class="input-container">
                                                        <button class="profile-btn" disabled=true>"Profile"</button>
                                                        <input 
                                                            type="text" 
                                                            placeholder="No active agent session..."
                                                            class="message-input"
                                                            disabled=true
                                                        />
                                                        <button class="send-btn" disabled=true>"Send"</button>
                                                    </div>
                                                </div>
                                            </div>
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
                                        "processes" => view! {
                                            <div class="processes-tab">
                                                <div class="process-list">
                                                    <h4>"Claude Code Processes"</h4>
                                                    <div class="no-processes">
                                                        <p>"No processes spawned yet"</p>
                                                        <p class="hint">"Process details with expandable JSON will appear here"</p>
                                                    </div>
                                                </div>
                                            </div>
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