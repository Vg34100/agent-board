// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct DirectoryItem {
    pub name: String,
    pub is_directory: bool,
    pub path: String,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn list_directory(path: String) -> Result<Vec<DirectoryItem>, String> {
    let dir_path = Path::new(&path);
    
    if !dir_path.exists() {
        return Err("Directory does not exist".to_string());
    }
    
    if !dir_path.is_dir() {
        return Err("Path is not a directory".to_string());
    }
    
    let mut items = Vec::new();
    
    match fs::read_dir(dir_path) {
        Ok(entries) => {
            for entry in entries {
                if let Ok(entry) = entry {
                    let file_name = entry.file_name().to_string_lossy().to_string();
                    let file_path = entry.path().to_string_lossy().to_string();
                    let is_directory = entry.path().is_dir();
                    
                    // Skip hidden files/directories (starting with .)
                    if !file_name.starts_with('.') {
                        items.push(DirectoryItem {
                            name: file_name,
                            is_directory,
                            path: file_path,
                        });
                    }
                }
            }
        }
        Err(e) => return Err(format!("Failed to read directory: {}", e)),
    }
    
    // Sort directories first, then files
    items.sort_by(|a, b| {
        match (a.is_directory, b.is_directory) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });
    
    Ok(items)
}

#[tauri::command]
async fn get_parent_directory(path: String) -> Result<String, String> {
    let dir_path = Path::new(&path);
    
    match dir_path.parent() {
        Some(parent) => Ok(parent.to_string_lossy().to_string()),
        None => Err("No parent directory".to_string()),
    }
}

#[tauri::command]
async fn get_home_directory() -> Result<String, String> {
    match dirs::home_dir() {
        Some(home) => Ok(home.to_string_lossy().to_string()),
        None => Err("Could not determine home directory".to_string()),
    }
}

#[tauri::command]
async fn create_project_directory(project_path: String) -> Result<String, String> {
    let path = Path::new(&project_path);
    
    if path.exists() {
        return Err(format!("Directory already exists: {}", project_path));
    }
    
    match fs::create_dir_all(&path) {
        Ok(_) => Ok(format!("Created directory: {}", project_path)),
        Err(e) => Err(format!("Failed to create directory: {}", e)),
    }
}

#[tauri::command]
async fn initialize_git_repo(project_path: String) -> Result<String, String> {
    let path = Path::new(&project_path);
    
    if !path.exists() {
        return Err("Project directory does not exist".to_string());
    }
    
    // Check if .git already exists
    let git_dir = path.join(".git");
    if git_dir.exists() {
        return Err("Git repository already initialized".to_string());
    }
    
    // Initialize git repository using git command
    let output = std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&path)
        .output();
    
    match output {
        Ok(output) => {
            if output.status.success() {
                Ok("Git repository initialized successfully".to_string())
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                Err(format!("Git init failed: {}", error))
            }
        }
        Err(e) => Err(format!("Failed to run git init: {}", e)),
    }
}

#[tauri::command]
async fn validate_git_repository(path: String) -> Result<bool, String> {
    let git_path = Path::new(&path).join(".git");
    Ok(git_path.exists() && git_path.is_dir())
}

#[tauri::command]
async fn load_projects_data(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    use tauri_plugin_store::StoreExt;
    
    let store = app.store("projects.json").map_err(|e| e.to_string())?;
    match store.get("projects") {
        Some(projects) => Ok(vec![projects.clone()]),
        None => Ok(vec![])
    }
}

#[tauri::command] 
async fn save_projects_data(app: tauri::AppHandle, projects: Vec<serde_json::Value>) -> Result<String, String> {
    use tauri_plugin_store::StoreExt;
    
    let store = app.store("projects.json").map_err(|e| e.to_string())?;
    let projects_value = serde_json::Value::Array(projects);
    store.set("projects", projects_value);
    store.save().map_err(|e| e.to_string())?;
    Ok("Projects saved successfully".to_string())
}

#[tauri::command]
async fn load_tasks_data(app: tauri::AppHandle, project_id: String) -> Result<Vec<serde_json::Value>, String> {
    use tauri_plugin_store::StoreExt;
    
    let tasks_file = format!("tasks_{}.json", project_id);
    let store = app.store(&tasks_file).map_err(|e| e.to_string())?;
    match store.get("tasks") {
        Some(tasks) => {
            if let serde_json::Value::Array(tasks_array) = tasks {
                Ok(tasks_array)
            } else {
                Ok(vec![])
            }
        }
        None => Ok(vec![])
    }
}

#[tauri::command]
async fn save_tasks_data(app: tauri::AppHandle, project_id: String, tasks: Vec<serde_json::Value>) -> Result<String, String> {
    use tauri_plugin_store::StoreExt;
    
    let tasks_file = format!("tasks_{}.json", project_id);
    let store = app.store(&tasks_file).map_err(|e| e.to_string())?;
    let tasks_value = serde_json::Value::Array(tasks);
    store.set("tasks", tasks_value);
    store.save().map_err(|e| e.to_string())?;
    Ok("Tasks saved successfully".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, list_directory, get_parent_directory, get_home_directory, create_project_directory, initialize_git_repo, validate_git_repository, load_projects_data, save_projects_data, load_tasks_data, save_tasks_data])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
