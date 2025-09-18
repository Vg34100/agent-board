// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use std::fs;
use std::path::Path;
use serde::{Deserialize, Serialize};

mod git;
mod agent;
mod web;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct CodexSettings {
    command: Option<String>,
    args: Option<Vec<String>>, // extra args before prompt
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct AgentSettings {
    codex: Option<CodexSettings>,
}

use tauri_plugin_store::StoreExt;
use tauri::Manager;
use tauri::Emitter;

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
    let init_output = std::process::Command::new("git")
        .args(&["init"])
        .current_dir(&path)
        .output();
    
    match init_output {
        Ok(output) => {
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Git init failed: {}", error));
            }
        }
        Err(e) => return Err(format!("Failed to run git init: {}", e)),
    }
    
    // Create initial README.md file
    let project_name = path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("New Project");
    
    let readme_content = format!(
        "# {}\n\nThis project was created using Agent Board.\n\n## Getting Started\n\nThis is a new project workspace created automatically by Agent Board for managing development tasks and AI coding agents.\n",
        project_name
    );
    
    let readme_path = path.join("README.md");
    match fs::write(&readme_path, readme_content) {
        Ok(_) => {},
        Err(e) => return Err(format!("Failed to create README.md: {}", e)),
    }
    
    // Add README.md to git
    let add_output = std::process::Command::new("git")
        .args(&["add", "README.md"])
        .current_dir(&path)
        .output();
        
    match add_output {
        Ok(output) => {
            if !output.status.success() {
                let error = String::from_utf8_lossy(&output.stderr);
                return Err(format!("Git add failed: {}", error));
            }
        }
        Err(e) => return Err(format!("Failed to run git add: {}", e)),
    }
    
    // Create initial commit
    let commit_output = std::process::Command::new("git")
        .args(&["commit", "-m", "Initial commit: Add README.md"])
        .current_dir(&path)
        .output();
        
    match commit_output {
        Ok(output) => {
            if output.status.success() {
                Ok("Git repository initialized successfully with initial commit".to_string())
            } else {
                let error = String::from_utf8_lossy(&output.stderr);
                Err(format!("Git commit failed: {}", error))
            }
        }
        Err(e) => Err(format!("Failed to run git commit: {}", e)),
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

#[tauri::command]
async fn create_task_worktree(app: tauri::AppHandle, project_path: String, task_id: String) -> Result<String, String> {
    println!("Tauri command: create_task_worktree called with project_path='{}', task_id='{}'", project_path, task_id);
    match git::create_worktree(&app, &project_path, &task_id) {
        Ok(worktree) => {
            let path_str = worktree.path.to_string_lossy().to_string();
            println!("Tauri command: create_task_worktree succeeded, returning path: {}", path_str);
            Ok(path_str)
        },
        Err(e) => {
            println!("Tauri command: create_task_worktree failed with error: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn remove_task_worktree(app: tauri::AppHandle, worktree_path: String, project_path: String) -> Result<String, String> {
    println!("Tauri command: remove_task_worktree called with worktree_path='{}', project_path='{}'", worktree_path, project_path);
    match git::remove_worktree(&app, &worktree_path, &project_path) {
        Ok(_) => {
            println!("Tauri command: remove_task_worktree succeeded");
            Ok("Worktree removed successfully".to_string())
        },
        Err(e) => {
            println!("Tauri command: remove_task_worktree failed with error: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn open_worktree_location(worktree_path: String) -> Result<String, String> {
    println!("Tauri command: open_worktree_location called with worktree_path='{}'", worktree_path);
    match git::open_worktree_location(&worktree_path) {
        Ok(_) => {
            println!("Tauri command: open_worktree_location succeeded");
            Ok("File manager opened successfully".to_string())
        },
        Err(e) => {
            println!("Tauri command: open_worktree_location failed with error: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn open_worktree_in_ide(worktree_path: String) -> Result<String, String> {
    println!("Tauri command: open_worktree_in_ide called with worktree_path='{}'", worktree_path);
    match git::open_worktree_in_ide(&worktree_path) {
        Ok(_) => {
            println!("Tauri command: open_worktree_in_ide succeeded");
            Ok("IDE opened successfully".to_string())
        },
        Err(e) => {
            println!("Tauri command: open_worktree_in_ide failed with error: {}", e);
            Err(e)
        }
    }
}

#[tauri::command]
async fn list_app_worktrees(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    println!("Tauri command: list_app_worktrees called");
    match git::list_app_worktrees(&app) {
        Ok(worktrees) => {
            println!("Tauri command: list_app_worktrees succeeded, found {} worktrees", worktrees.len());
            Ok(worktrees)
        },
        Err(e) => {
            println!("Tauri command: list_app_worktrees failed with error: {}", e);
            Err(e)
        }
    }
}

// Agent Commands

#[tauri::command]
async fn start_agent_process(
    app: tauri::AppHandle,
    task_id: String,
    task_title: String,
    task_description: String,
    worktree_path: String,
    #[allow(non_snake_case)] profile: Option<String>,
) -> Result<String, String> {
    println!("Tauri command: start_agent_process called for task '{}' in worktree '{}'", task_id, worktree_path);
    let initial_message = format!("{}: {}", task_title, task_description);
    println!("start_agent_process: received profile = {:?}", profile);
    let which = profile
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "claude".to_string())
        .to_lowercase();
    println!("start_agent_process: launching agent kind = {}", which);
    match which.as_str() {
        "codex" | "chat-codex" | "chatgpt-codex" => {
            agent::spawn_codex_process(app, task_id, initial_message, worktree_path, None)
        }
        _ => agent::spawn_claude_process(app, task_id, initial_message, worktree_path, None)
    }
}

// Global agent settings: load and save
#[tauri::command]
async fn load_agent_settings(app: tauri::AppHandle) -> Result<AgentSettings, String> {
    let store = app.store("agent_settings.json").map_err(|e| e.to_string())?;
    if let Some(val) = store.get("settings") {
        serde_json::from_value::<AgentSettings>(val.clone()).map_err(|e| e.to_string())
    } else {
        Ok(AgentSettings::default())
    }
}

#[tauri::command]
async fn save_agent_settings(app: tauri::AppHandle, settings: AgentSettings) -> Result<String, String> {
    let store = app.store("agent_settings.json").map_err(|e| e.to_string())?;
    let val = serde_json::to_value(&settings).map_err(|e| e.to_string())?;
    store.set("settings", val);
    store.save().map_err(|e| e.to_string())?;
    Ok("Agent settings saved".to_string())
}

// Persisted agent messages per task
#[tauri::command]
async fn load_task_agent_messages(app: tauri::AppHandle, task_id: String) -> Result<Vec<agent::AgentMessage>, String> {
    let file = format!("agent_messages_{}.json", task_id);
    let store = app.store(&file).map_err(|e| e.to_string())?;
    if let Some(val) = store.get("messages") {
        serde_json::from_value::<Vec<agent::AgentMessage>>(val.clone()).map_err(|e| e.to_string())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn save_task_agent_messages(app: tauri::AppHandle, task_id: String, messages: Vec<agent::AgentMessage>) -> Result<String, String> {
    let file = format!("agent_messages_{}.json", task_id);
    let store = app.store(&file).map_err(|e| e.to_string())?;
    let val = serde_json::to_value(&messages).map_err(|e| e.to_string())?;
    store.set("messages", val);
    store.save().map_err(|e| e.to_string())?;
    Ok("Agent messages saved".to_string())
}

// Persisted agent processes list
#[tauri::command]
async fn load_agent_processes(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let store = app.store("agent_processes.json").map_err(|e| e.to_string())?;
    if let Some(val) = store.get("processes") {
        if let serde_json::Value::Array(processes_array) = val {
            Ok(processes_array)
        } else {
            Ok(vec![])
        }
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn save_agent_processes(app: tauri::AppHandle, processes: Vec<serde_json::Value>) -> Result<String, String> {
    let store = app.store("agent_processes.json").map_err(|e| e.to_string())?;
    let val = serde_json::Value::Array(processes);
    store.set("processes", val);
    store.save().map_err(|e| e.to_string())?;
    Ok("Agent processes saved".to_string())
}

#[tauri::command]
async fn send_agent_message(
    app: tauri::AppHandle,
    process_id: String,
    message: String,
    worktree_path: String,
) -> Result<String, String> {
    println!("Tauri command: send_agent_message called for process '{}' with message: {}", process_id, message);
    agent::send_message_to_process(app, &process_id, message, worktree_path)
}

// Per-process agent messages persistence
#[tauri::command]
async fn load_process_agent_messages(app: tauri::AppHandle, task_id: String, process_id: String) -> Result<Vec<agent::AgentMessage>, String> {
    let file = format!("agent_messages_{}_{}.json", task_id, process_id);
    let store = app.store(&file).map_err(|e| e.to_string())?;
    if let Some(val) = store.get("messages") {
        serde_json::from_value::<Vec<agent::AgentMessage>>(val.clone()).map_err(|e| e.to_string())
    } else {
        Ok(vec![])
    }
}

#[tauri::command]
async fn save_process_agent_messages(app: tauri::AppHandle, task_id: String, process_id: String, messages: Vec<agent::AgentMessage>) -> Result<String, String> {
    let file = format!("agent_messages_{}_{}.json", task_id, process_id);
    let store = app.store(&file).map_err(|e| e.to_string())?;
    let val = serde_json::to_value(&messages).map_err(|e| e.to_string())?;
    store.set("messages", val);
    store.save().map_err(|e| e.to_string())?;
    Ok("Process agent messages saved".to_string())
}

#[tauri::command]
async fn send_agent_message_with_profile(
    app: tauri::AppHandle,
    process_id: String,
    message: String,
    worktree_path: String,
    profile: String,
) -> Result<String, String> {
    println!("Tauri command: send_agent_message_with_profile called for process '{}' with profile '{}'", process_id, profile);
    agent::send_message_with_profile(app, &process_id, message, worktree_path, &profile)
}

#[tauri::command]
async fn get_process_list() -> Result<Vec<serde_json::Value>, String> {
    println!("Tauri command: get_process_list called");
    Ok(agent::get_process_list())
}

#[tauri::command]
async fn get_process_details(process_id: String) -> Result<Option<agent::AgentProcess>, String> {
    println!("Tauri command: get_process_details called for process '{}'", process_id);
    Ok(agent::get_process_by_id(&process_id))
}

#[tauri::command]
async fn get_agent_messages(process_id: String) -> Result<Vec<agent::AgentMessage>, String> {
    println!("Tauri command: get_agent_messages called for process '{}'", process_id);
    Ok(agent::get_process_messages(&process_id))
}

#[tauri::command]
async fn kill_agent_process(process_id: String) -> Result<String, String> {
    println!("Tauri command: kill_agent_process called for process '{}'", process_id);
    agent::kill_process(&process_id)?;
    Ok("Process killed successfully".to_string())
}

#[tauri::command]
fn is_dev_mode() -> bool {
    cfg!(debug_assertions)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![greet, list_directory, get_parent_directory, get_home_directory, create_project_directory, initialize_git_repo, validate_git_repository, load_projects_data, save_projects_data, load_tasks_data, save_tasks_data, create_task_worktree, remove_task_worktree, open_worktree_location, open_worktree_in_ide, list_app_worktrees, start_agent_process, send_agent_message, send_agent_message_with_profile, get_process_list, get_process_details, get_agent_messages, kill_agent_process, load_agent_settings, save_agent_settings, load_task_agent_messages, save_task_agent_messages, load_process_agent_messages, save_process_agent_messages, load_agent_processes, save_agent_processes, is_dev_mode])
        .setup(|app| {
            // Bind to preferred fixed port, with fallback to a random high port if occupied
            let listener = match std::net::TcpListener::bind(("0.0.0.0", 17872)) {
                Ok(l) => l,
                Err(_) => std::net::TcpListener::bind(("0.0.0.0", 0))?,
            };
            let port = listener.local_addr()?.port();

            // Spawn the web server (serves the embedded dist and HTTP API)
            web::spawn(listener, app.handle().clone());
            println!("Embedded web server listening on 0.0.0.0:{}", port);

            // Compute a LAN URL if available and expose it to the user
            let lan_url = local_ip_address::local_ip()
                .ok()
                .map(|ip| format!("http://{}:{}", ip, port));

            // Point the main window to the local server so desktop and web share the same UI
            if let Some(win) = app.get_webview_window("main") {
                if let Ok(url) = tauri::Url::parse(&format!("http://127.0.0.1:{port}")) {
                    // Ignore navigate errors; window might not be ready yet in some environments
                    let _ = win.navigate(url);
                }
                if let Some(ref u) = lan_url {
                    let _ = win.set_title(&format!("agent-board — {}", u));
                }
            }

            // Emit an event the frontend can listen to, if desired
            let _ = app.emit("server_info", serde_json::json!({
                "port": port,
                "lan_url": lan_url,
            }));

            // Self-test: ping /health a few times and log the result to help diagnose connectivity
            std::thread::spawn(move || {
                use std::io::{Read, Write};
                use std::net::TcpStream;
                for i in 0..5 {
                    match TcpStream::connect(("127.0.0.1", port)) {
                        Ok(mut stream) => {
                            let _ = stream.write_all(b"GET /health HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
                            let mut buf = [0u8; 128];
                            if let Ok(n) = stream.read(&mut buf) {
                                let head = String::from_utf8_lossy(&buf[..n]).to_string();
                                println!("Self-test attempt {}: received {} bytes: {}", i+1, n, head.lines().next().unwrap_or(""));
                                break;
                            } else {
                                println!("Self-test attempt {}: connected but no data", i+1);
                            }
                        }
                        Err(e) => {
                            println!("Self-test attempt {}: connect failed: {}", i+1, e);
                            std::thread::sleep(std::time::Duration::from_millis(200));
                        }
                    }
                }
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
