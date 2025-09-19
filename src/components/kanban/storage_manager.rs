use leptos::task::spawn_local;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::models::{Task, Project};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Safe task serialization function with error handling
pub fn serialize_tasks_safely(tasks: &[Task]) -> Vec<serde_json::Value> {
    let json_tasks: Vec<serde_json::Value> = tasks.iter()
        .filter_map(|t| {
            match serde_json::to_value(t) {
                Ok(value) => {
                    // Check if the serialization actually produced a valid object
                    if value.is_object() && !value.as_object().unwrap().is_empty() {
                        Some(value)
                    } else {
                        web_sys::console::error_1(&format!("Task serialization produced empty object for task ID: {}", t.id).into());
                        None
                    }
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to serialize task ID {}: {}", t.id, e).into());
                    None
                }
            }
        })
        .collect();

    // Verify no data loss
    if json_tasks.len() != tasks.len() {
        web_sys::console::error_1(&format!("DATA LOSS WARNING: Lost {} tasks during serialization!", tasks.len() - json_tasks.len()).into());
    } else {
        web_sys::console::log_1(&format!("Successfully serialized {} tasks", json_tasks.len()).into());
    }

    json_tasks
}

// Load projects from storage
pub async fn load_projects() -> Result<Vec<Project>, String> {
    let empty_args = serde_json::json!({});
    if let Ok(js_value) = to_value(&empty_args) {
        match invoke("load_projects_data", js_value).await {
            js_result if !js_result.is_undefined() => {
                if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                    if let Some(projects) = projects_wrapper.first() {
                        Ok(projects.clone())
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Err("Failed to parse projects data".to_string())
                }
            }
            _ => Err("Failed to load projects from storage".to_string())
        }
    } else {
        Err("Failed to serialize load arguments".to_string())
    }
}

// Save projects to storage
pub async fn save_projects(projects: &[Project]) -> Result<(), String> {
    let projects_json: Vec<serde_json::Value> = projects.iter()
        .filter_map(|project| {
            match serde_json::to_value(project) {
                Ok(value) => Some(value),
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to serialize project ID {}: {}", project.id, e).into());
                    None
                }
            }
        })
        .collect();

    let save_args = serde_json::json!({
        "projects": projects_json
    });

    if let Ok(save_js_value) = to_value(&save_args) {
        match invoke("save_projects_data", save_js_value).await {
            js_result if !js_result.is_undefined() => {
                web_sys::console::log_1(&"Projects saved successfully".into());
                Ok(())
            }
            _ => {
                Err("Failed to save projects to storage".to_string())
            }
        }
    } else {
        Err("Failed to serialize save arguments".to_string())
    }
}

// Load tasks for a specific project
pub async fn load_tasks(project_id: &str) -> Result<Vec<Task>, String> {
    let load_args = serde_json::json!({ "projectId": project_id });
    if let Ok(js_value) = to_value(&load_args) {
        match invoke("load_tasks_data", js_value).await {
            js_result if !js_result.is_undefined() => {
                if let Ok(tasks_json) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                    let tasks: Vec<Task> = tasks_json.into_iter()
                        .filter_map(|v| serde_json::from_value(v).ok())
                        .collect();
                    Ok(tasks)
                } else {
                    Ok(Vec::new()) // No tasks exist yet
                }
            }
            _ => {
                Ok(Vec::new()) // No tasks exist yet
            }
        }
    } else {
        Err("Failed to serialize load arguments".to_string())
    }
}

// Save tasks for a specific project
pub async fn save_tasks(project_id: &str, tasks: &[Task]) -> Result<(), String> {
    let json_tasks = serialize_tasks_safely(tasks);

    // Only save if we didn't lose any tasks during serialization
    if json_tasks.len() != tasks.len() {
        return Err(format!("DATA LOSS WARNING: Lost {} tasks during serialization! Aborting save to prevent corruption.", tasks.len() - json_tasks.len()));
    }

    let save_args = serde_json::json!({
        "projectId": project_id,
        "tasks": json_tasks
    });

    if let Ok(js_value) = to_value(&save_args) {
        match invoke("save_tasks_data", js_value).await {
            js_result if !js_result.is_undefined() => {
                web_sys::console::log_1(&format!("Tasks for project {} saved successfully", project_id).into());
                Ok(())
            }
            _ => {
                Err("Failed to save tasks to storage".to_string())
            }
        }
    } else {
        Err("Failed to serialize save arguments".to_string())
    }
}

// Save tasks with spawn_local for use in reactive contexts
pub fn save_tasks_async(project_id: String, tasks: Vec<Task>) {
    spawn_local(async move {
        if let Err(e) = save_tasks(&project_id, &tasks).await {
            web_sys::console::error_1(&format!("Failed to save tasks: {}", e).into());
        }
    });
}