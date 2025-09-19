use leptos::prelude::*;
use leptos::html::Dialog;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use std::sync::Arc;
use crate::models::{Task, TaskStatus, AgentProfile};
use crate::components::TaskSidebar;
use super::task_operations::{delete_task, update_task_details, update_task_profile, update_task_status};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Create sidebar component with all necessary callbacks
pub fn create_task_sidebar(
    task: Task,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
    selected_task_signal: WriteSignal<Option<String>>,
    edit_dialog_ref: NodeRef<Dialog>,
    set_editing_task: WriteSignal<Option<Task>>,
) -> impl IntoView {
    let sidebar_edit_callback = {
        let set_editing_task_clone = set_editing_task.clone();
        let edit_dialog_ref_clone = edit_dialog_ref.clone();
        Box::new(move |task: Task| {
            set_editing_task_clone.set(Some(task));
            if let Some(dialog) = edit_dialog_ref_clone.get() {
                let _ = dialog.show_modal();
            }
        }) as Box<dyn Fn(Task) + 'static>
    };

    let sidebar_status_callback: Arc<dyn Fn(String, TaskStatus) + Send + Sync> = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Arc::new(move |task_id: String, status: TaskStatus| {
            update_task_status(task_id, status, project_id_clone.clone(), tasks_signal_clone);
        })
    };

    let sidebar_delete_callback = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Box::new(move |task_id: String| {
            delete_task(task_id, project_id_clone.clone(), tasks_signal_clone);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_worktree_callback = {
        Box::new(move |worktree_path: String| {
            open_worktree_location(worktree_path);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_ide_callback = {
        Box::new(move |worktree_path: String| {
            open_worktree_in_ide(worktree_path);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_profile_callback = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Box::new(move |task_id: String, profile: AgentProfile| {
            update_task_profile(task_id, profile, project_id_clone.clone(), tasks_signal_clone);
        }) as Box<dyn Fn(String, AgentProfile) + 'static>
    };

    view! {
        <TaskSidebar
            task=task.clone()
            selected_task=selected_task_signal
            on_edit=sidebar_edit_callback
            on_update_status=sidebar_status_callback
            on_delete=sidebar_delete_callback
            on_open_worktree=Some(sidebar_worktree_callback)
            on_open_ide=Some(sidebar_ide_callback)
            on_update_profile=sidebar_profile_callback
        />
    }
}

// Create edit task callback for the sidebar
pub fn create_edit_task_callback(
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) -> Box<dyn Fn(String, String, String) + 'static> {
    Box::new(move |task_id: String, new_title: String, new_description: String| {
        update_task_details(task_id, new_title, new_description, project_id.clone(), tasks_signal);
    })
}

// Open worktree location in file manager
fn open_worktree_location(worktree_path: String) {
    web_sys::console::log_1(&format!("Opening file manager for: {}", worktree_path).into());
    spawn_local(async move {
        let open_args = serde_json::json!({
            "worktreePath": worktree_path.clone()
        });

        if let Ok(open_js_value) = to_value(&open_args) {
            match invoke("open_worktree_location", open_js_value).await {
                js_result if !js_result.is_undefined() => {
                    match serde_wasm_bindgen::from_value::<Result<String, String>>(js_result) {
                        Ok(Ok(_)) => {
                            web_sys::console::log_1(&"File manager opened successfully".into());
                        }
                        Ok(Err(error_msg)) => {
                            web_sys::console::error_1(&format!("Failed to open file manager: {}", error_msg).into());
                        }
                        Err(parse_error) => {
                            web_sys::console::error_1(&format!("Failed to parse file manager result: {:?}", parse_error).into());
                        }
                    }
                }
                _ => {
                    web_sys::console::error_1(&"No response from open_worktree_location command".into());
                }
            }
        }
    });
}

// Open worktree in IDE
fn open_worktree_in_ide(worktree_path: String) {
    web_sys::console::log_1(&format!("Opening IDE for: {}", worktree_path).into());
    spawn_local(async move {
        let open_args = serde_json::json!({
            "worktreePath": worktree_path.clone()
        });

        if let Ok(open_js_value) = to_value(&open_args) {
            match invoke("open_worktree_in_ide", open_js_value).await {
                js_result if !js_result.is_undefined() => {
                    match serde_wasm_bindgen::from_value::<Result<String, String>>(js_result) {
                        Ok(Ok(_)) => {
                            web_sys::console::log_1(&"IDE opened successfully".into());
                        }
                        Ok(Err(error_msg)) => {
                            web_sys::console::error_1(&format!("Failed to open IDE: {}", error_msg).into());
                        }
                        Err(parse_error) => {
                            web_sys::console::error_1(&format!("Failed to parse IDE result: {:?}", parse_error).into());
                        }
                    }
                }
                _ => {
                    web_sys::console::error_1(&"No response from open_worktree_in_ide command".into());
                }
            }
        }
    });
}