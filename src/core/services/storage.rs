use leptos::task::spawn_local;
use crate::core::models::{Task, Project};
use super::tauri_commands::*;

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
    match load_projects_data().await {
        Ok(js_result) => {
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
        Err(e) => Err(format!("Failed to load projects from storage: {}", e))
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

    match save_projects_data(projects_json).await {
        Ok(_) => {
            web_sys::console::log_1(&"Projects saved successfully".into());
            Ok(())
        }
        Err(e) => Err(format!("Failed to save projects to storage: {}", e))
    }
}

// Load tasks for a specific project
pub async fn load_tasks(project_id: &str) -> Result<Vec<Task>, String> {
    match load_tasks_data(project_id).await {
        Ok(js_result) => {
            if let Ok(tasks_json) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                let tasks: Vec<Task> = tasks_json.into_iter()
                    .filter_map(|v| serde_json::from_value(v).ok())
                    .collect();
                Ok(tasks)
            } else {
                Ok(Vec::new()) // No tasks exist yet
            }
        }
        Err(_) => Ok(Vec::new()) // No tasks exist yet
    }
}

// Save tasks for a specific project
pub async fn save_tasks(project_id: &str, tasks: &[Task]) -> Result<(), String> {
    let json_tasks = serialize_tasks_safely(tasks);

    // Only save if we didn't lose any tasks during serialization
    if json_tasks.len() != tasks.len() {
        return Err(format!("DATA LOSS WARNING: Lost {} tasks during serialization! Aborting save to prevent corruption.", tasks.len() - json_tasks.len()));
    }

    match save_tasks_data(project_id, json_tasks).await {
        Ok(_) => {
            web_sys::console::log_1(&format!("Tasks for project {} saved successfully", project_id).into());
            Ok(())
        }
        Err(e) => Err(format!("Failed to save tasks to storage: {}", e))
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