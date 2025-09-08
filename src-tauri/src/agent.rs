use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    pub id: String,
    pub sender: String, // "user", "agent", "system"
    pub content: String,
    pub timestamp: String,
    pub message_type: String, // "text", "file_read", "file_edit", "tool_call"
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentProcess {
    pub id: String,
    pub task_id: String,
    pub status: String, // "running", "completed", "failed", "killed"
    pub start_time: String,
    pub end_time: Option<String>,
    pub messages: Vec<AgentMessage>,
    pub raw_output: Vec<String>,
}

// Global process storage
type ProcessMap = Arc<Mutex<HashMap<String, AgentProcess>>>;
static PROCESSES: std::sync::OnceLock<ProcessMap> = std::sync::OnceLock::new();

fn get_processes() -> &'static ProcessMap {
    PROCESSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// Generates a unique process ID
fn generate_process_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("proc_{}", timestamp)
}

/// Generates a unique message ID
fn generate_message_id() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("msg_{}", timestamp)
}

/// Gets current timestamp as string
fn get_timestamp() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    // Simple timestamp format - can be improved with chrono if needed
    format!("{}", now)
}

/// Parses Claude Code JSON output into structured messages
fn parse_claude_output(line: &str) -> Option<AgentMessage> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        // Basic parsing - this will need refinement based on actual Claude Code JSON format
        if let Some(content) = json.get("content").and_then(|v| v.as_str()) {
            let message_type = json.get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("text");
            
            let sender = match message_type {
                "tool_call" => "system",
                "file_read" => "agent", 
                "file_edit" => "agent",
                _ => "agent"
            };

            return Some(AgentMessage {
                id: generate_message_id(),
                sender: sender.to_string(),
                content: content.to_string(),
                timestamp: get_timestamp(),
                message_type: message_type.to_string(),
                metadata: Some(json.clone()),
            });
        }
    }
    None
}

/// Spawns a new Claude Code process
pub fn spawn_claude_process(
    task_id: String, 
    initial_message: String, 
    worktree_path: String,
    context: Option<String>
) -> Result<String, String> {
    let process_id = generate_process_id();
    println!("Spawning Claude Code process {} for task {}", process_id, task_id);

    // Construct the full message with context if provided
    let full_message = if let Some(ctx) = context {
        format!("Previous conversation:\n{}\n\nNew message: {}", ctx, initial_message)
    } else {
        initial_message
    };

    // Build Claude Code command
    let mut cmd = Command::new("claude");
    cmd.arg("-p")
       .arg(&full_message)
       .arg("--output-format")
       .arg("stream-json")
       .arg("--add-dir")
       .arg(&worktree_path)
       .stdout(Stdio::piped())
       .stderr(Stdio::piped());

    println!("Claude Code command: {:?}", cmd);

    // Create initial process entry
    let mut process = AgentProcess {
        id: process_id.clone(),
        task_id: task_id.clone(),
        status: "running".to_string(),
        start_time: get_timestamp(),
        end_time: None,
        messages: vec![
            AgentMessage {
                id: generate_message_id(),
                sender: "user".to_string(),
                content: initial_message,
                timestamp: get_timestamp(),
                message_type: "text".to_string(),
                metadata: None,
            }
        ],
        raw_output: Vec::new(),
    };

    // Store process before spawning
    {
        let processes = get_processes();
        let mut map = processes.lock().unwrap();
        map.insert(process_id.clone(), process.clone());
    }

    // Spawn the process (for now, we'll simulate it since Claude Code might not be installed)
    // In production, uncomment the actual spawn:
    /*
    match cmd.spawn() {
        Ok(child) => {
            // TODO: Handle child process output in a separate thread
            // Read stdout/stderr and parse JSON messages
            // Update process status and messages
            println!("Claude Code process spawned successfully");
        }
        Err(e) => {
            // Mark process as failed
            let processes = get_processes();
            let mut map = processes.lock().unwrap();
            if let Some(proc) = map.get_mut(&process_id) {
                proc.status = "failed".to_string();
                proc.end_time = Some(get_timestamp());
            }
            return Err(format!("Failed to spawn Claude Code process: {}", e));
        }
    }
    */

    // For now, simulate a successful process with some dummy messages
    {
        let processes = get_processes();
        let mut map = processes.lock().unwrap();
        if let Some(proc) = map.get_mut(&process_id) {
            proc.messages.push(AgentMessage {
                id: generate_message_id(),
                sender: "system".to_string(),
                content: "Claude Code process initialized".to_string(),
                timestamp: get_timestamp(),
                message_type: "system".to_string(),
                metadata: None,
            });
            proc.messages.push(AgentMessage {
                id: generate_message_id(),
                sender: "agent".to_string(),
                content: format!("I'll help you work on this task. I'm now working in the worktree at: {}", worktree_path),
                timestamp: get_timestamp(),
                message_type: "text".to_string(),
                metadata: None,
            });
        }
    }

    println!("Process {} created successfully", process_id);
    Ok(process_id)
}

/// Gets all processes
pub fn get_all_processes() -> HashMap<String, AgentProcess> {
    let processes = get_processes();
    let map = processes.lock().unwrap();
    map.clone()
}

/// Gets a specific process by ID
pub fn get_process_by_id(process_id: &str) -> Option<AgentProcess> {
    let processes = get_processes();
    let map = processes.lock().unwrap();
    map.get(process_id).cloned()
}

/// Gets messages for a specific process
pub fn get_process_messages(process_id: &str) -> Vec<AgentMessage> {
    let processes = get_processes();
    let map = processes.lock().unwrap();
    map.get(process_id)
        .map(|proc| proc.messages.clone())
        .unwrap_or_default()
}

/// Sends a new message to an existing process (spawns new process with context)
pub fn send_message_to_process(
    process_id: &str,
    message: String,
    worktree_path: String,
) -> Result<String, String> {
    let processes = get_processes();
    
    // Get existing process and its context
    let context = {
        let map = processes.lock().unwrap();
        if let Some(proc) = map.get(process_id) {
            // Build context from existing messages
            let context_messages: Vec<String> = proc.messages.iter()
                .map(|msg| format!("{}: {}", msg.sender, msg.content))
                .collect();
            Some(context_messages.join("\n"))
        } else {
            return Err("Process not found".to_string());
        }
    };

    // Get task_id from existing process
    let task_id = {
        let map = processes.lock().unwrap();
        map.get(process_id)
            .map(|proc| proc.task_id.clone())
            .ok_or("Process not found")?
    };

    // Spawn new process with context
    let new_process_id = spawn_claude_process(
        task_id,
        message,
        worktree_path,
        context,
    )?;

    // Mark old process as completed
    {
        let mut map = processes.lock().unwrap();
        if let Some(proc) = map.get_mut(process_id) {
            proc.status = "completed".to_string();
            proc.end_time = Some(get_timestamp());
        }
    }

    Ok(new_process_id)
}

/// Kills a running process
pub fn kill_process(process_id: &str) -> Result<(), String> {
    let processes = get_processes();
    let mut map = processes.lock().unwrap();
    
    if let Some(proc) = map.get_mut(process_id) {
        proc.status = "killed".to_string();
        proc.end_time = Some(get_timestamp());
        println!("Process {} marked as killed", process_id);
        Ok(())
    } else {
        Err("Process not found".to_string())
    }
}

/// Gets process list summary for UI
pub fn get_process_list() -> Vec<serde_json::Value> {
    let processes = get_processes();
    let map = processes.lock().unwrap();
    
    map.values()
        .map(|proc| serde_json::json!({
            "id": proc.id,
            "task_id": proc.task_id,
            "status": proc.status,
            "start_time": proc.start_time,
            "message_count": proc.messages.len()
        }))
        .collect()
}