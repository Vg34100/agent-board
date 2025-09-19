use leptos::task::spawn_local;
use serde_wasm_bindgen::from_value;
use crate::core::models::{Task, AgentProfile};
use super::tauri_commands::*;
use super::storage::{load_projects, save_tasks_async};

// Create a worktree for a task
pub async fn create_worktree_for_task(project_id: &str, task_id: &str) -> Result<String, String> {
    // First, get the project path from storage
    let projects = load_projects().await?;
    let project = projects.iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("Project {} not found", project_id))?;

    web_sys::console::log_1(&format!("Creating worktree for task {}", task_id).into());

    match create_task_worktree(&project.project_path, task_id).await {
        Ok(js_result) => {
            match from_value::<String>(js_result) {
                Ok(worktree_path) => {
                    web_sys::console::log_1(&format!("Worktree created successfully at: {}", worktree_path).into());
                    Ok(worktree_path)
                }
                Err(e) => Err(format!("Failed to parse worktree creation result: {:?}", e))
            }
        }
        Err(e) => Err(format!("Failed to create worktree: {}", e))
    }
}

// Remove a worktree for a task
pub async fn remove_worktree_for_task(project_id: &str, worktree_path: &str) -> Result<(), String> {
    let projects = load_projects().await?;
    let project = projects.iter()
        .find(|p| p.id == project_id)
        .ok_or_else(|| format!("Project {} not found", project_id))?;

    match remove_task_worktree(worktree_path, &project.project_path).await {
        Ok(js_result) => {
            match from_value::<Result<String, String>>(js_result) {
                Ok(Ok(_)) => {
                    web_sys::console::log_1(&"Worktree removed successfully".into());
                    Ok(())
                }
                Ok(Err(error_msg)) => Err(format!("Worktree removal failed: {}", error_msg)),
                Err(e) => Err(format!("Failed to parse worktree removal result: {:?}", e))
            }
        }
        Err(e) => Err(format!("Failed to remove worktree: {}", e))
    }
}

// Start agent process for a task
pub async fn start_agent_for_task(task: &Task, worktree_path: &str) -> Result<String, String> {
    let profile_str = match task.profile {
        AgentProfile::Codex => "codex",
        AgentProfile::ClaudeCode => "claude",
    };

    web_sys::console::log_1(&format!("Starting agent process for task: {} with profile {}", task.id, profile_str).into());

    match start_agent_process(&task.id, &task.title, &task.description, worktree_path, profile_str).await {
        Ok(js_result) => {
            match from_value::<String>(js_result) {
                Ok(process_id) => {
                    web_sys::console::log_1(&format!("Agent process started successfully with ID: {}", process_id).into());
                    Ok(process_id)
                }
                Err(e) => Err(format!("Failed to parse agent process ID: {:?}", e))
            }
        }
        Err(e) => Err(format!("Failed to start agent process: {}", e))
    }
}

// Open worktree location in file manager
pub fn open_worktree_location_async(worktree_path: String) {
    spawn_local(async move {
        match open_worktree_location(&worktree_path).await {
            Ok(js_result) => {
                match from_value::<Result<String, String>>(js_result) {
                    Ok(Ok(_)) => {
                        web_sys::console::log_1(&"File manager opened successfully".into());
                    }
                    Ok(Err(error_msg)) => {
                        web_sys::console::error_1(&format!("Failed to open file manager: {}", error_msg).into());
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse file manager result: {:?}", e).into());
                    }
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to open file manager: {}", e).into());
            }
        }
    });
}

// Open worktree in IDE
pub fn open_worktree_in_ide_async(worktree_path: String) {
    spawn_local(async move {
        match open_worktree_in_ide(&worktree_path).await {
            Ok(js_result) => {
                match from_value::<Result<String, String>>(js_result) {
                    Ok(Ok(_)) => {
                        web_sys::console::log_1(&"IDE opened successfully".into());
                    }
                    Ok(Err(error_msg)) => {
                        web_sys::console::error_1(&format!("Failed to open IDE: {}", error_msg).into());
                    }
                    Err(e) => {
                        web_sys::console::error_1(&format!("Failed to parse IDE result: {:?}", e).into());
                    }
                }
            }
            Err(e) => {
                web_sys::console::error_1(&format!("Failed to open IDE: {}", e).into());
            }
        }
    });
}