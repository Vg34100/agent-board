use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    pub async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Generic Tauri command executor with error handling
pub async fn execute_tauri_command(command: &str, args: serde_json::Value) -> Result<JsValue, String> {
    match to_value(&args) {
        Ok(js_value) => {
            let result = invoke(command, js_value).await;
            if result.is_undefined() {
                Err(format!("No response from {} command", command))
            } else {
                Ok(result)
            }
        }
        Err(_) => Err(format!("Failed to serialize arguments for {} command", command))
    }
}

// Specific command helpers
pub async fn load_projects_data() -> Result<JsValue, String> {
    execute_tauri_command("load_projects_data", serde_json::json!({})).await
}

#[allow(dead_code)]
pub async fn save_projects_data(projects: Vec<serde_json::Value>) -> Result<JsValue, String> {
    execute_tauri_command("save_projects_data", serde_json::json!({ "projects": projects })).await
}

pub async fn load_tasks_data(project_id: &str) -> Result<JsValue, String> {
    execute_tauri_command("load_tasks_data", serde_json::json!({ "projectId": project_id })).await
}

pub async fn save_tasks_data(project_id: &str, tasks: Vec<serde_json::Value>) -> Result<JsValue, String> {
    execute_tauri_command("save_tasks_data", serde_json::json!({
        "projectId": project_id,
        "tasks": tasks
    })).await
}

pub async fn create_task_worktree(project_path: &str, task_id: &str) -> Result<JsValue, String> {
    execute_tauri_command("create_task_worktree", serde_json::json!({
        "projectPath": project_path,
        "taskId": task_id
    })).await
}

pub async fn remove_task_worktree(worktree_path: &str, project_path: &str) -> Result<JsValue, String> {
    execute_tauri_command("remove_task_worktree", serde_json::json!({
        "worktreePath": worktree_path,
        "projectPath": project_path
    })).await
}

pub async fn start_agent_process(task_id: &str, task_title: &str, task_description: &str, worktree_path: &str, profile: &str) -> Result<JsValue, String> {
    execute_tauri_command("start_agent_process", serde_json::json!({
        "taskId": task_id,
        "taskTitle": task_title,
        "taskDescription": task_description,
        "worktreePath": worktree_path,
        "profile": profile
    })).await
}

pub async fn open_worktree_location(worktree_path: &str) -> Result<JsValue, String> {
    execute_tauri_command("open_worktree_location", serde_json::json!({
        "worktreePath": worktree_path
    })).await
}

pub async fn open_worktree_in_ide(worktree_path: &str) -> Result<JsValue, String> {
    execute_tauri_command("open_worktree_in_ide", serde_json::json!({
        "worktreePath": worktree_path
    })).await
}