use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{BufRead, BufReader};
use std::thread;
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
    pub session_id: Option<String>,
    pub total_cost_usd: Option<f64>,
    pub num_turns: Option<i32>,
    pub worktree_path: String,
}

// Store for active child processes
type ChildProcessMap = Arc<Mutex<HashMap<String, Child>>>;
static CHILD_PROCESSES: std::sync::OnceLock<ChildProcessMap> = std::sync::OnceLock::new();

fn get_child_processes() -> &'static ChildProcessMap {
    CHILD_PROCESSES.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
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

/// Parses Claude Code JSON output into structured messages based on real format
fn parse_claude_output(line: &str) -> Option<AgentMessage> {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(line) {
        let event_type = json.get("type").and_then(|v| v.as_str())?;
        
        match event_type {
            "system" => {
                // System initialization event
                let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("init");
                let session_id = json.get("session_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                
                Some(AgentMessage {
                    id: generate_message_id(),
                    sender: "system".to_string(),
                    content: format!("Claude Code session initialized ({})", session_id),
                    timestamp: get_timestamp(),
                    message_type: subtype.to_string(),
                    metadata: Some(json.clone()),
                })
            },
            "assistant" => {
                // Assistant message with content array
                if let Some(message) = json.get("message") {
                    if let Some(content_array) = message.get("content").and_then(|v| v.as_array()) {
                        // Process content array - can have text and tool_use
                        for content_item in content_array {
                            if let Some(content_type) = content_item.get("type").and_then(|v| v.as_str()) {
                                match content_type {
                                    "text" => {
                                        if let Some(text) = content_item.get("text").and_then(|v| v.as_str()) {
                                            return Some(AgentMessage {
                                                id: generate_message_id(),
                                                sender: "agent".to_string(),
                                                content: text.to_string(),
                                                timestamp: get_timestamp(),
                                                message_type: "text".to_string(),
                                                metadata: Some(json.clone()),
                                            });
                                        }
                                    },
                                    "tool_use" => {
                                        let tool_name = content_item.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                                        let tool_input = content_item.get("input").unwrap_or(&serde_json::Value::Null);
                                        
                                        let message_type = match tool_name {
                                            "Read" => "file_read",
                                            "Edit" | "Write" | "MultiEdit" => "file_edit",
                                            "Bash" => "tool_call",
                                            _ => "tool_call"
                                        };
                                        
                                        return Some(AgentMessage {
                                            id: generate_message_id(),
                                            sender: "agent".to_string(),
                                            content: format!("Using tool: {} - {}", tool_name, tool_input),
                                            timestamp: get_timestamp(),
                                            message_type: message_type.to_string(),
                                            metadata: Some(json.clone()),
                                        });
                                    },
                                    _ => continue
                                }
                            }
                        }
                    }
                }
            },
            "user" => {
                // User message with tool results
                if let Some(message) = json.get("message") {
                    if let Some(content_array) = message.get("content").and_then(|v| v.as_array()) {
                        for content_item in content_array {
                            if content_item.get("type").and_then(|v| v.as_str()) == Some("tool_result") {
                                let content = content_item.get("content").and_then(|v| v.as_str()).unwrap_or("Tool executed");
                                let is_error = content_item.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                                
                                return Some(AgentMessage {
                                    id: generate_message_id(),
                                    sender: if is_error { "system" } else { "user" },
                                    content: content.to_string(),
                                    timestamp: get_timestamp(),
                                    message_type: if is_error { "error" } else { "tool_result" },
                                    metadata: Some(json.clone()),
                                });
                            }
                        }
                    }
                }
            },
            "result" => {
                // Final result with cost and session info
                let subtype = json.get("subtype").and_then(|v| v.as_str()).unwrap_or("success");
                let cost = json.get("total_cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let turns = json.get("num_turns").and_then(|v| v.as_i64()).unwrap_or(0);
                
                return Some(AgentMessage {
                    id: generate_message_id(),
                    sender: "system".to_string(),
                    content: format!("Session completed: {} (${:.4}, {} turns)", subtype, cost, turns),
                    timestamp: get_timestamp(),
                    message_type: "result".to_string(),
                    metadata: Some(json.clone()),
                });
            },
            _ => None
        }
    } else {
        None
    }
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
        status: "starting".to_string(),
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
        session_id: None,
        total_cost_usd: None,
        num_turns: None,
        worktree_path: worktree_path.clone(),
    };

    // Store process before spawning
    {
        let processes = get_processes();
        let mut map = processes.lock().unwrap();
        map.insert(process_id.clone(), process.clone());
    }

    // Spawn the actual Claude Code process
    match cmd.spawn() {
        Ok(mut child) => {
            println!("Claude Code process spawned successfully with PID: {:?}", child.id());
            
            // Take ownership of stdout and stderr
            let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
            let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
            
            // Store the child process
            {
                let child_processes = get_child_processes();
                let mut map = child_processes.lock().unwrap();
                map.insert(process_id.clone(), child);
            }
            
            // Update process status to running
            {
                let processes = get_processes();
                let mut map = processes.lock().unwrap();
                if let Some(proc) = map.get_mut(&process_id) {
                    proc.status = "running".to_string();
                }
            }
            
            // Spawn thread to read stdout (JSON messages)
            let process_id_stdout = process_id.clone();
            let processes_stdout = get_processes().clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines() {
                    match line {
                        Ok(line_content) => {
                            println!("Claude Code stdout: {}", line_content);
                            
                            // Store raw output
                            {
                                let mut map = processes_stdout.lock().unwrap();
                                if let Some(proc) = map.get_mut(&process_id_stdout) {
                                    proc.raw_output.push(line_content.clone());
                                }
                            }
                            
                            // Parse and store structured message
                            if let Some(message) = parse_claude_output(&line_content) {
                                let mut map = processes_stdout.lock().unwrap();
                                if let Some(proc) = map.get_mut(&process_id_stdout) {
                                    // Update session info from system events
                                    if message.message_type == "init" || message.message_type == "system" {
                                        if let Some(metadata) = &message.metadata {
                                            if let Some(session_id) = metadata.get("session_id").and_then(|v| v.as_str()) {
                                                proc.session_id = Some(session_id.to_string());
                                            }
                                        }
                                    }
                                    
                                    // Update cost and turns from result events
                                    if message.message_type == "result" {
                                        if let Some(metadata) = &message.metadata {
                                            if let Some(cost) = metadata.get("total_cost_usd").and_then(|v| v.as_f64()) {
                                                proc.total_cost_usd = Some(cost);
                                            }
                                            if let Some(turns) = metadata.get("num_turns").and_then(|v| v.as_i64()) {
                                                proc.num_turns = Some(turns as i32);
                                            }
                                        }
                                        proc.status = "completed".to_string();
                                        proc.end_time = Some(get_timestamp());
                                    }
                                    
                                    proc.messages.push(message);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Error reading Claude Code stdout: {}", e);
                            break;
                        }
                    }
                }
                
                println!("Claude Code stdout reader thread finished for process {}", process_id_stdout);
            });
            
            // Spawn thread to read stderr (error messages)
            let process_id_stderr = process_id.clone();
            let processes_stderr = get_processes().clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    match line {
                        Ok(line_content) => {
                            println!("Claude Code stderr: {}", line_content);
                            
                            // Store error as a message
                            let error_message = AgentMessage {
                                id: generate_message_id(),
                                sender: "system".to_string(),
                                content: line_content,
                                timestamp: get_timestamp(),
                                message_type: "error".to_string(),
                                metadata: None,
                            };
                            
                            let mut map = processes_stderr.lock().unwrap();
                            if let Some(proc) = map.get_mut(&process_id_stderr) {
                                proc.messages.push(error_message);
                            }
                        }
                        Err(e) => {
                            println!("Error reading Claude Code stderr: {}", e);
                            break;
                        }
                    }
                }
                
                println!("Claude Code stderr reader thread finished for process {}", process_id_stderr);
            });
            
            // Spawn thread to monitor process completion
            let process_id_monitor = process_id.clone();
            let processes_monitor = get_processes().clone();
            let child_processes_monitor = get_child_processes().clone();
            thread::spawn(move || {
                // Wait a bit for the process to potentially finish
                std::thread::sleep(std::time::Duration::from_secs(1));
                
                // Check if child process is still alive
                let mut should_wait = true;
                while should_wait {
                    {
                        let mut child_map = child_processes_monitor.lock().unwrap();
                        if let Some(child) = child_map.get_mut(&process_id_monitor) {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    println!("Claude Code process {} finished with status: {}", process_id_monitor, status);
                                    should_wait = false;
                                    
                                    // Update process status
                                    let mut proc_map = processes_monitor.lock().unwrap();
                                    if let Some(proc) = proc_map.get_mut(&process_id_monitor) {
                                        if proc.status == "running" {
                                            proc.status = if status.success() { "completed" } else { "failed" };
                                            proc.end_time = Some(get_timestamp());
                                        }
                                    }
                                    
                                    // Remove from child processes
                                    child_map.remove(&process_id_monitor);
                                }
                                Ok(None) => {
                                    // Process still running, wait a bit more
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                }
                                Err(e) => {
                                    println!("Error checking process status: {}", e);
                                    should_wait = false;
                                }
                            }
                        } else {
                            should_wait = false;
                        }
                    }
                }
                
                println!("Process monitor thread finished for process {}", process_id_monitor);
            });
            
            println!("Process {} started successfully with monitoring threads", process_id);
            Ok(process_id)
        }
        Err(e) => {
            // Mark process as failed
            let processes = get_processes();
            let mut map = processes.lock().unwrap();
            if let Some(proc) = map.get_mut(&process_id) {
                proc.status = "failed".to_string();
                proc.end_time = Some(get_timestamp());
            }
            Err(format!("Failed to spawn Claude Code process: {}", e))
        }
    }
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
    // First, try to kill the actual child process
    {
        let child_processes = get_child_processes();
        let mut child_map = child_processes.lock().unwrap();
        if let Some(mut child) = child_map.remove(process_id) {
            match child.kill() {
                Ok(_) => println!("Successfully killed child process {}", process_id),
                Err(e) => println!("Failed to kill child process {}: {}", process_id, e),
            }
            // Wait for the process to actually terminate
            let _ = child.wait();
        }
    }
    
    // Update process status
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