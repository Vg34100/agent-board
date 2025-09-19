use leptos::prelude::*;
use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::models::{Task, TaskStatus, AgentProfile};
use super::storage_manager::{save_tasks_async, load_projects};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Create a new task and save it
pub fn create_task_handler(
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) -> Box<dyn Fn(Task) + 'static> {
    Box::new(move |task: Task| {
        // Update the tasks signal by pushing the new task to the vector
        tasks_signal.update(|tasks| {
            tasks.push(task);
        });

        // Save tasks to storage
        let project_id = project_id.clone();
        let current_tasks = tasks_signal.get_untracked();
        save_tasks_async(project_id, current_tasks);
    })
}

// Update task status and handle worktree operations
pub fn update_task_status(
    task_id: String,
    new_status: TaskStatus,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            let old_status = task.status.clone();
            task.update_status(new_status.clone());

            // If task is moving to InProgress, create a worktree only if missing
            if new_status == TaskStatus::InProgress && old_status != TaskStatus::InProgress {
                if task.worktree_path.is_some() {
                    web_sys::console::log_1(&format!("Reusing existing worktree for task {}", task.id).into());
                } else {
                    let task_id_clone = task_id.clone();
                    let project_id_clone = project_id.clone();
                    let tasks_signal_clone = tasks_signal.clone();

                    spawn_local(async move {
                        if let Ok(projects) = load_projects().await {
                            if let Some(project) = projects.iter().find(|p| p.id == project_id_clone) {
                                let create_args = serde_json::json!({
                                    "projectPath": project.project_path,
                                    "taskId": task_id_clone
                                });

                                web_sys::console::log_1(&format!("Creating worktree for task {}", task_id_clone).into());

                                if let Ok(create_js_value) = to_value(&create_args) {
                                    match invoke("create_task_worktree", create_js_value).await {
                                        js_result if !js_result.is_undefined() => {
                                            if let Ok(worktree_path) = serde_wasm_bindgen::from_value::<String>(js_result) {
                                                web_sys::console::log_1(&format!("Worktree created successfully at: {}", worktree_path).into());

                                                // Start agent process
                                                start_agent_process(&task_id_clone, &worktree_path, &tasks_signal_clone).await;

                                                // Update task with worktree path and save
                                                update_task_worktree_path(task_id_clone.clone(), Some(worktree_path), project_id_clone, tasks_signal_clone);
                                            }
                                        }
                                        _ => {
                                            web_sys::console::error_1(&"No response from create_task_worktree command".into());
                                        }
                                    }
                                }
                            }
                        }
                    });
                }
            }

            // If task is moving away from InProgress (to Done/Cancelled), remove worktree
            if (new_status == TaskStatus::Done || new_status == TaskStatus::Cancelled) && old_status == TaskStatus::InProgress {
                if let Some(worktree_path) = &task.worktree_path {
                    let worktree_path_clone = worktree_path.clone();
                    let task_id_clone = task_id.clone();
                    let project_id_clone = project_id.clone();
                    let tasks_signal_clone = tasks_signal.clone();

                    spawn_local(async move {
                        remove_task_worktree(&worktree_path_clone, &project_id_clone, &task_id_clone, tasks_signal_clone).await;
                    });
                }
            }
        }
    });

    // Save status change immediately to storage
    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task worktree path
fn update_task_worktree_path(
    task_id: String,
    worktree_path: Option<String>,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.set_worktree_path(worktree_path);
        }
    });

    // Save updated tasks to storage
    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Start agent process for a task
async fn start_agent_process(task_id: &str, worktree_path: &str, tasks_signal: &RwSignal<Vec<Task>>) {
    let task_for_agent = {
        let tasks = tasks_signal.get_untracked();
        tasks.iter().find(|t| t.id == task_id).cloned()
    };

    if let Some(task) = task_for_agent {
        let agent_args = serde_json::json!({
            "taskId": task_id,
            "taskTitle": task.title,
            "taskDescription": task.description,
            "worktreePath": worktree_path,
            "profile": match task.profile {
                AgentProfile::Codex => "codex",
                AgentProfile::ClaudeCode => "claude",
            }
        });

        web_sys::console::log_1(&format!("Starting agent process for task: {}", task_id).into());

        if let Ok(agent_js_value) = to_value(&agent_args) {
            match invoke("start_agent_process", agent_js_value).await {
                js_result if !js_result.is_undefined() => {
                    if let Ok(process_id) = serde_wasm_bindgen::from_value::<String>(js_result) {
                        web_sys::console::log_1(&format!("Agent process started successfully with ID: {}", process_id).into());
                    }
                }
                _ => {
                    web_sys::console::error_1(&"Failed to start agent process".into());
                }
            }
        }
    }
}

// Remove task worktree
async fn remove_task_worktree(
    worktree_path: &str,
    project_id: &str,
    task_id: &str,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    if let Ok(projects) = load_projects().await {
        if let Some(project) = projects.iter().find(|p| p.id == project_id) {
            let remove_args = serde_json::json!({
                "worktreePath": worktree_path,
                "projectPath": project.project_path
            });

            if let Ok(remove_js_value) = to_value(&remove_args) {
                match invoke("remove_task_worktree", remove_js_value).await {
                    js_result if !js_result.is_undefined() => {
                        match serde_wasm_bindgen::from_value::<Result<String, String>>(js_result) {
                            Ok(Ok(_)) => {
                                web_sys::console::log_1(&format!("Worktree removed successfully for task: {}", task_id).into());
                                update_task_worktree_path(task_id.to_string(), None, project_id.to_string(), tasks_signal);
                            }
                            Ok(Err(error_msg)) => {
                                web_sys::console::error_1(&format!("Worktree removal failed: {}", error_msg).into());
                            }
                            Err(parse_error) => {
                                web_sys::console::error_1(&format!("Failed to parse worktree removal result: {:?}", parse_error).into());
                            }
                        }
                    }
                    _ => {
                        web_sys::console::error_1(&"No response from remove_task_worktree command".into());
                    }
                }
            }
        }
    }
}

// Delete a task
pub fn delete_task(
    task_id: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        tasks.retain(|t| t.id != task_id);
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task details (title and description)
pub fn update_task_details(
    task_id: String,
    new_title: String,
    new_description: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.update_title(new_title);
            task.update_description(new_description);
        }
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task agent profile
pub fn update_task_profile(
    task_id: String,
    profile: AgentProfile,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.profile = profile;
        }
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Cancel a task (set status to Cancelled)
pub fn cancel_task(
    task_id: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    update_task_status(task_id, TaskStatus::Cancelled, project_id, tasks_signal);
}