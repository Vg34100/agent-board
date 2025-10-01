use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{BufRead, BufReader};
use std::thread;
use tauri::Emitter;

// Set to true to see all verbose debug messages, false for production filtering
const AGENT_DEBUG: bool = false;

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
    #[serde(default)]
    pub kind: AgentKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AgentKind {
    Claude,
    Codex,
}

impl Default for AgentKind {
    fn default() -> Self { AgentKind::Claude }
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

/// Checks if agent debug mode is enabled
fn is_debug_mode() -> bool {
    AGENT_DEBUG
}

/// Splits a line that may contain multiple JSON objects
fn split_json_objects(line: &str) -> Vec<String> {
    let mut objects = Vec::new();
    let mut current = String::new();
    let mut brace_count = 0;
    let mut in_string = false;
    let mut escape_next = false;
    
    for ch in line.chars() {
        if escape_next {
            current.push(ch);
            escape_next = false;
            continue;
        }
        
        match ch {
            '\\' if in_string => {
                escape_next = true;
                current.push(ch);
            }
            '"' => {
                in_string = !in_string;
                current.push(ch);
            }
            '{' if !in_string => {
                brace_count += 1;
                current.push(ch);
            }
            '}' if !in_string => {
                current.push(ch);
                brace_count -= 1;
                if brace_count == 0 && !current.trim().is_empty() {
                    objects.push(current.trim().to_string());
                    current.clear();
                }
            }
            _ => {
                current.push(ch);
            }
        }
    }
    
    // If there's remaining content that doesn't form a complete JSON object, discard it
    // This handles malformed or incomplete JSON
    objects
}

#[cfg(test)]
mod tests {
    use super::split_json_objects;

    #[test]
    fn splits_multiple_json_objects_on_one_line() {
        let input = r#"{"a":1}{"b":2}{"c":3}"#;
        let objs = split_json_objects(input);
        assert_eq!(objs.len(), 3);
        assert_eq!(objs[0], "{\"a\":1}");
        assert_eq!(objs[1], "{\"b\":2}");
        assert_eq!(objs[2], "{\"c\":3}");
    }
}

/// Parses Codex CLI JSONL events into AgentMessage based on actual Codex output format
fn parse_codex_output(line: &str) -> Option<AgentMessage> {
    let trimmed = line.trim();
    if trimmed.is_empty() { return None; }
    
    // First try to parse as JSON
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(trimmed) {
        // Handle session configuration JSON (first line output)
        if json.get("workdir").is_some() || json.get("sandbox").is_some() || json.get("approval").is_some() {
            let workdir = json.get("workdir").and_then(|v| v.as_str()).unwrap_or("unknown");
            let sandbox = json.get("sandbox").and_then(|v| v.as_str()).unwrap_or("unknown");
            let approval = json.get("approval").and_then(|v| v.as_str()).unwrap_or("unknown");
            
            return Some(AgentMessage {
                id: generate_message_id(),
                sender: "system".to_string(),
                content: format!("Codex session initialized (workdir: {}, sandbox: {}, approval: {})", workdir, sandbox, approval),
                timestamp: get_timestamp(),
                message_type: "config".to_string(),
                metadata: Some(json),
            });
        }
        
        // Handle prompt input JSON (user message sent to Codex)
        if let Some(prompt) = json.get("prompt").and_then(|v| v.as_str()) {
            return Some(AgentMessage {
                id: generate_message_id(),
                sender: "user".to_string(),
                content: prompt.to_string(),
                timestamp: get_timestamp(),
                message_type: "text".to_string(),
                metadata: Some(json),
            });
        }
        
        // Handle Codex event messages with id/msg structure
        if let (Some(id), Some(msg)) = (json.get("id"), json.get("msg")) {
            let event_id = id.as_str().unwrap_or("unknown");
            let msg_type = msg.get("type").and_then(|v| v.as_str()).unwrap_or("");
            
            match msg_type {
                "task_started" => {
                    let model_context = msg.get("model_context_window").and_then(|v| v.as_u64()).unwrap_or(0);
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "system".to_string(),
                        content: format!("Task started (ID: {}, context window: {})", event_id, model_context),
                        timestamp: get_timestamp(),
                        message_type: "task_started".to_string(),
                        metadata: Some(json),
                    })
                }
                "agent_reasoning_section_break" => {
                    // Skip these as they're just formatting breaks - don't display in UI
                    None
                }
                "agent_reasoning" => {
                    let text = msg.get("text").and_then(|v| v.as_str()).unwrap_or("");
                    if !text.is_empty() {
                        Some(AgentMessage {
                            id: generate_message_id(),
                            sender: "agent".to_string(),
                            content: text.to_string(),
                            timestamp: get_timestamp(),
                            message_type: "agent_reasoning".to_string(),
                            metadata: Some(json),
                        })
                    } else {
                        None
                    }
                }
                "agent_message" => {
                    let text = msg.get("message").and_then(|v| v.as_str()).unwrap_or("");
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: text.to_string(),
                        timestamp: get_timestamp(),
                        message_type: "agent_message".to_string(),
                        metadata: Some(json),
                    })
                }
                "token_count" => {
                    let input_tokens = msg.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let output_tokens = msg.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    let total_tokens = msg.get("total_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
                    
                    // Filter out zero-value token counts (just noise)
                    if input_tokens == 0 && output_tokens == 0 && total_tokens == 0 {
                        if is_debug_mode() {
                            Some(AgentMessage {
                                id: generate_message_id(),
                                sender: "system".to_string(),
                                content: "[DEBUG] Token usage: 0 input, 0 output, 0 total".to_string(),
                                timestamp: get_timestamp(),
                                message_type: "debug_tokens".to_string(),
                                metadata: Some(json),
                            })
                        } else {
                            None // Filter out zero token counts in production
                        }
                    } else {
                        // Show meaningful token counts
                        Some(AgentMessage {
                            id: generate_message_id(),
                            sender: "system".to_string(),
                            content: format!("💰 Token usage: {} input, {} output, {} total", input_tokens, output_tokens, total_tokens),
                            timestamp: get_timestamp(),
                            message_type: "token_count".to_string(),
                            metadata: Some(json),
                        })
                    }
                }
                "tool_use" => {
                    let tool_name = msg.get("tool").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let tool_input = msg.get("input").unwrap_or(&serde_json::Value::Null);
                    
                    let message_type = match tool_name {
                        "read_file" | "read" => "file_read",
                        "edit_file" | "write_file" | "edit" | "write" => "file_edit",
                        "bash" | "shell" => "tool_call",
                        _ => "tool_call"
                    };
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: format!("Using tool: {} - {}", tool_name, tool_input),
                        timestamp: get_timestamp(),
                        message_type: message_type.to_string(),
                        metadata: Some(json),
                    })
                }
                "tool_result" => {
                    let content = msg.get("content").and_then(|v| v.as_str()).unwrap_or("Tool executed");
                    let is_error = msg.get("is_error").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: if is_error { "system".to_string() } else { "user".to_string() },
                        content: content.to_string(),
                        timestamp: get_timestamp(),
                        message_type: if is_error { "error".to_string() } else { "tool_result".to_string() },
                        metadata: Some(json),
                    })
                }
                "patch_apply_begin" => {
                    let _call_id = msg.get("call_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let auto_approved = msg.get("auto_approved").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    // Extract file changes information
                    let changes = msg.get("changes").and_then(|v| v.as_object());
                    let files_count = changes.map(|c| c.len()).unwrap_or(0);
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: format!("📝 Starting file operation ({} files) - {}", files_count, 
                            if auto_approved { "auto-approved" } else { "pending approval" }),
                        timestamp: get_timestamp(),
                        message_type: "file_edit_start".to_string(),
                        metadata: Some(json),
                    })
                }
                "patch_apply_end" => {
                    let _call_id = msg.get("call_id").and_then(|v| v.as_str()).unwrap_or("unknown");
                    let success = msg.get("success").and_then(|v| v.as_bool()).unwrap_or(false);
                    let stdout = msg.get("stdout").and_then(|v| v.as_str()).unwrap_or("");
                    let stderr = msg.get("stderr").and_then(|v| v.as_str()).unwrap_or("");
                    
                    let content = if success {
                        format!("✅ File operation completed: {}", stdout.trim())
                    } else {
                        format!("❌ File operation failed: {}", stderr.trim())
                    };
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content,
                        timestamp: get_timestamp(),
                        message_type: "file_edit_end".to_string(),
                        metadata: Some(json),
                    })
                }
                "turn_diff" => {
                    let unified_diff = msg.get("unified_diff").and_then(|v| v.as_str()).unwrap_or("");
                    
                    // Parse the diff to extract file information and changes
                    let mut file_name = "unknown file";
                    let mut additions = 0;
                    let mut deletions = 0;
                    let mut diff_lines = Vec::new();
                    
                    for line in unified_diff.lines() {
                        if line.starts_with("+++") {
                            // Extract filename from +++ b/filename
                            if let Some(name) = line.strip_prefix("+++ b/") {
                                file_name = name;
                            } else if let Some(name) = line.strip_prefix("+++ ") {
                                // Handle absolute paths
                                file_name = name.split('/').last().unwrap_or(name);
                            }
                        } else if line.starts_with('+') && !line.starts_with("+++") {
                            additions += 1;
                            diff_lines.push(format!("+ {}", &line[1..])); // Remove the + and add it back with formatting
                        } else if line.starts_with('-') && !line.starts_with("---") {
                            deletions += 1;
                            diff_lines.push(format!("- {}", &line[1..])); // Remove the - and add it back with formatting
                        } else if line.starts_with(' ') {
                            // Context lines
                            diff_lines.push(format!("  {}", &line[1..]));
                        } else if line.starts_with("@@") {
                            // Line number info
                            diff_lines.push(line.to_string());
                        }
                    }
                    
                    // Create the diff display content with actual diff lines
                    let diff_content = if !diff_lines.is_empty() {
                        let header = format!("📄 Modified {} (+{} -{} lines)", file_name, additions, deletions);
                        let diff_body = diff_lines.join("\n");
                        format!("{}\n\n{}", header, diff_body)
                    } else {
                        format!("📄 Modified {} (+{} -{} lines)", file_name, additions, deletions)
                    };
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: diff_content,
                        timestamp: get_timestamp(),
                        message_type: "file_diff".to_string(),
                        metadata: Some(json.clone()),
                    })
                }
                // Filter out noisy exec_command events
                "exec_command_output_delta" => {
                    // These are just base64 chunks - completely useless for UI
                    // Only show in debug mode for troubleshooting
                    if is_debug_mode() {
                        Some(AgentMessage {
                            id: generate_message_id(),
                            sender: "system".to_string(),
                            content: format!("[DEBUG] Command output chunk (ID: {})", event_id),
                            timestamp: get_timestamp(),
                            message_type: "debug_chunk".to_string(),
                            metadata: Some(json),
                        })
                    } else {
                        None // Filter out completely in production
                    }
                }
                "exec_command_begin" => {
                    // Show command start with actual command info
                    let command = json.get("msg")
                        .and_then(|m| m.get("command"))
                        .and_then(|c| c.as_array())
                        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_else(|| "unknown command".to_string());
                    
                    let call_id = json.get("msg")
                        .and_then(|m| m.get("call_id"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("unknown");
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: format!("🔧 Executing: {}", command),
                        timestamp: get_timestamp(),
                        message_type: "command_start".to_string(),
                        metadata: Some(serde_json::json!({
                            "call_id": call_id,
                            "command": command,
                            "original": json
                        })),
                    })
                }
                "exec_command_end" => {
                    // Show command completion with status
                    let call_id = json.get("msg")
                        .and_then(|m| m.get("call_id"))
                        .and_then(|c| c.as_str())
                        .unwrap_or("unknown");
                    
                    let exit_code = json.get("msg")
                        .and_then(|m| m.get("exit_code"))
                        .and_then(|c| c.as_i64())
                        .unwrap_or(-1);
                    
                    let status_icon = if exit_code == 0 { "✅" } else { "❌" };
                    let status_text = if exit_code == 0 { "completed" } else { "failed" };
                    
                    Some(AgentMessage {
                        id: generate_message_id(),
                        sender: "agent".to_string(),
                        content: format!("{} Command {} (exit code: {})", status_icon, status_text, exit_code),
                        timestamp: get_timestamp(),
                        message_type: "command_end".to_string(),
                        metadata: Some(serde_json::json!({
                            "call_id": call_id,
                            "exit_code": exit_code,
                            "original": json
                        })),
                    })
                }
                // Add other commonly noisy events to blacklist
                "exec_command_output" | "exec_command_stderr" => {
                    // These tend to be verbose - only show in debug mode
                    if is_debug_mode() {
                        Some(AgentMessage {
                            id: generate_message_id(),
                            sender: "system".to_string(),
                            content: format!("[DEBUG] Command {}: {}", msg_type, event_id),
                            timestamp: get_timestamp(),
                            message_type: "debug_output".to_string(),
                            metadata: Some(json),
                        })
                    } else {
                        None
                    }
                }
                _ => {
                    // Handle truly unknown event types - only show in debug mode or if important looking
                    if is_debug_mode() || msg_type.contains("error") || msg_type.contains("fail") {
                        Some(AgentMessage {
                            id: generate_message_id(),
                            sender: "system".to_string(),
                            content: format!("Codex event: {} (ID: {})", msg_type, event_id),
                            timestamp: get_timestamp(),
                            message_type: msg_type.to_string(),
                            metadata: Some(json),
                        })
                    } else {
                        None // Filter out unknown events in production
                    }
                }
            }
        } else { 
            // Handle other JSON structures as raw data
            Some(AgentMessage {
                id: generate_message_id(),
                sender: "system".to_string(),
                content: format!("Raw Codex data: {}", trimmed),
                timestamp: get_timestamp(),
                message_type: "json_data".to_string(),
                metadata: Some(json),
            })
        }
    } else {
        // Handle non-JSON output
        println!("Codex non-JSON output: {}", trimmed);
        
        // Skip log lines from Codex CLI
        if trimmed.contains("INFO") || trimmed.contains("DEBUG") || trimmed.contains("WARN") {
            if trimmed.contains("codex_core") || trimmed.contains("codex_exec") {
                // These are Codex internal logs, skip for cleaner output
                return None;
            }
        }
        
        // Handle special status messages
        if trimmed.contains("Shutting down") || trimmed.contains("interrupt received") {
            return Some(AgentMessage {
                id: generate_message_id(),
                sender: "system".to_string(),
                content: trimmed.to_string(),
                timestamp: get_timestamp(),
                message_type: "system_status".to_string(),
                metadata: None,
            });
        }
        
        // Handle any other non-empty text as agent output
        if !trimmed.is_empty() {
            Some(AgentMessage {
                id: generate_message_id(),
                sender: "agent".to_string(),
                content: trimmed.to_string(),
                timestamp: get_timestamp(),
                message_type: "text".to_string(),
                metadata: None,
            })
        } else {
            None
        }
    }
}
/// Parses Claude Code JSON output into structured messages based on real format
fn parse_claude_output(line: &str) -> Option<AgentMessage> {
    // First try to parse as JSON
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
                        // Return None if no content found
                        return None;
                    } else {
                        return None;
                    }
                } else {
                    return None;
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
                                    sender: if is_error { "system".to_string() } else { "user".to_string() },
                                    content: content.to_string(),
                                    timestamp: get_timestamp(),
                                    message_type: if is_error { "error".to_string() } else { "tool_result".to_string() },
                                    metadata: Some(json.clone()),
                                });
                            }
                        }
                        // Return None if no tool_result found
                        return None;
                    } else {
                        return None;
                    }
                } else {
                    return None;
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
        // Handle non-JSON responses
        if !line.trim().is_empty() {
            // Skip empty lines and common non-JSON output
            let trimmed = line.trim();
            if trimmed.starts_with("Error:") || trimmed.starts_with("Warning:") {
                // System error/warning message
                Some(AgentMessage {
                    id: generate_message_id(),
                    sender: "system".to_string(),
                    content: trimmed.to_string(),
                    timestamp: get_timestamp(),
                    message_type: "error".to_string(),
                    metadata: None,
                })
            } else {
                // Plain text response from Claude
                println!("Received non-JSON response, treating as plain text: {}", line);
                Some(AgentMessage {
                    id: generate_message_id(),
                    sender: "agent".to_string(),
                    content: trimmed.to_string(),
                    timestamp: get_timestamp(),
                    message_type: "text".to_string(),
                    metadata: None,
                })
            }
        } else {
            None
        }
    }
}

/// Spawns a new Claude Code process
pub fn spawn_claude_process(
    app: tauri::AppHandle,
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
        initial_message.clone()
    };

    // Try multiple Claude command variations (similar to VS Code code.cmd issue)
    let claude_commands = ["claude", "claude.exe", "claude.cmd"];
    let mut cmd = None;

    for command in &claude_commands {
        let mut test_cmd = Command::new(command);
        test_cmd.arg("--version")
               .stdout(Stdio::null())
               .stderr(Stdio::null())
               .current_dir(&worktree_path); // Set working directory

        // Inherit environment variables to ensure PATH is available
        for (key, value) in std::env::vars() {
            test_cmd.env(key, value);
        }

        println!("Testing Claude command: {}", command);
        match test_cmd.status() {
            Ok(status) if status.success() => {
                println!("Found working Claude command: {}", command);
                // let mut working_cmd = Command::new(command);
                // working_cmd.arg("-p")
                //           .arg(&full_message)
                //           .arg("--output-format")
                //           .arg("stream-json")
                //           .arg("--verbose")
                //           .arg("--dangerously-skip-permissions")
                //           .arg("--add-dir")
                //           .arg(&worktree_path)
                //           .stdout(Stdio::piped())
                //           .stderr(Stdio::piped())
                //           .current_dir(&worktree_path); // Set working directory
                let mut working_cmd = if command.ends_with(".cmd") {
                    // Important: call via cmd.exe to avoid Rust's .cmd escaping guard
                    let mut c = Command::new("cmd");
                    // Sanitize prompt for cmd.exe: avoid literal newlines which can break argument parsing
                    let prompt_arg = full_message.replace("\r\n", " ").replace('\n', " ");
                    c.arg("/C")
                    .arg(command) // "claude.cmd"
                    .arg("-p").arg(prompt_arg)
                    .arg("--output-format").arg("stream-json")
                    .arg("--verbose")
                    .arg("--permission-mode").arg("acceptEdits")
                    .arg("--dangerously-skip-permissions")
                    .arg("--allowedTools").arg("Read,Write,Edit,MultiEdit,Bash")
                    .arg("--add-dir").arg(&worktree_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .current_dir(&worktree_path);
                    c
                } else {
                    let mut c = Command::new(command);
                    c.arg("-p").arg(&full_message)
                    .arg("--output-format").arg("stream-json")
                    .arg("--verbose")
                    .arg("--permission-mode").arg("acceptEdits")
                    .arg("--dangerously-skip-permissions")
                    .arg("--allowedTools").arg("Read,Write,Edit,MultiEdit,Bash")
                    .arg("--add-dir").arg(&worktree_path)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .current_dir(&worktree_path);
                    c
                };

                // Inherit environment variables
                for (key, value) in std::env::vars() {
                    working_cmd.env(key, value);
                }

                cmd = Some(working_cmd);
                break;
            }
            Ok(status) => {
                println!("Command {} exists but failed with status: {}", command, status);
            }
            Err(e) => {
                println!("Command {} not found or failed: {:?}", command, e);
            }
        }
    }

    let mut cmd = cmd.ok_or_else(|| {
        let attempted = claude_commands.join(", ");
        format!("Claude Code CLI not found. Tried commands: {}. Please ensure Claude Code is installed and in PATH.", attempted)
    })?;

    println!("Claude Code command: {:?}", cmd);

    // Create initial process entry with user message
    let process = AgentProcess {
        id: process_id.clone(),
        task_id: task_id.clone(),
        status: "starting".to_string(),
        start_time: get_timestamp(),
        end_time: None,
        messages: vec![
            AgentMessage {
                id: generate_message_id(),
                sender: "user".to_string(),
                content: format!("Task: {}", initial_message),
                timestamp: get_timestamp(),
                message_type: "text".to_string(),
                metadata: Some(serde_json::json!({
                    "task_id": task_id,
                    "worktree_path": worktree_path
                })),
            }
        ],
        raw_output: Vec::new(),
        session_id: None,
        total_cost_usd: None,
        num_turns: None,
        worktree_path: worktree_path.clone(),
        kind: AgentKind::Claude,
    };

    // Store process before spawning
    {
        let processes = get_processes();
        let mut map = processes.lock().unwrap();
        map.insert(process_id.clone(), process.clone());
        
        // Emit process creation event
        let status_payload = serde_json::json!({
            "process_id": process_id,
            "task_id": task_id,
            "status": "starting"
        });
        
        match app.emit("agent_process_status", status_payload.clone()) {
            Ok(_) => println!("✅ Emitted agent_process_status event: {} starting", process_id),
            Err(e) => println!("❌ Failed to emit process status event: {:?}", e)
        };
        
        // Also broadcast to HTTP clients
        crate::web::broadcast_to_http("agent_process_status", status_payload);
    }

    // Create a temporary config file to set permissions
    // COMMENTED OUT: Temporarily disabled
    // let config_dir = format!("{}/.claude", worktree_path);
    // std::fs::create_dir_all(&config_dir).ok();
    // let config_path = format!("{}/settings.local.json", config_dir);
    // let config_content = serde_json::json!({
    //     "permissionMode": "acceptEdits",
    //     "allowedTools": ["Read", "Write", "Edit", "MultiEdit", "Bash"]
    // });
    // if let Ok(config_str) = serde_json::to_string_pretty(&config_content) {
    //     std::fs::write(&config_path, config_str).ok();
    //     println!("Created Claude config at: {}", config_path);
    // }

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
                    
                    // Emit process status update event
                    let status_payload = serde_json::json!({
                        "process_id": process_id,
                        "task_id": proc.task_id,
                        "status": "running"
                    });
                    
                    match app.emit("agent_process_status", status_payload.clone()) {
                        Ok(_) => println!("✅ Emitted agent_process_status event: {} running", process_id),
                        Err(e) => println!("❌ Failed to emit process status event: {:?}", e)
                    };
                    
                    // Also broadcast to HTTP clients
                    crate::web::broadcast_to_http("agent_process_status", status_payload);
                }
            }

            // Spawn thread to read stdout (JSON messages)
            let process_id_stdout = process_id.clone();
            let processes_stdout = get_processes().clone();
            let app_handle_stdout = app.clone();
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
                            match parse_claude_output(&line_content) {
                                Some(message) => {
                                    println!("Parsed message: {:?}", message);
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

                                        proc.messages.push(message.clone());
                                        println!("Message stored. Total messages: {}", proc.messages.len());
                                        
                                        // Emit Tauri event for real-time updates
                                        let message_payload = serde_json::json!({
                                            "process_id": process_id_stdout,
                                            "task_id": proc.task_id,
                                            "message": message
                                        });
                                        
                                        match app_handle_stdout.emit("agent_message_update", message_payload.clone()) {
                                            Ok(_) => println!("✅ Emitted agent_message_update event for process {}", process_id_stdout),
                                            Err(e) => println!("❌ Failed to emit event: {:?}", e)
                                        };
                                        
                                        // Also broadcast to HTTP clients
                                        crate::web::broadcast_to_http("agent_message_update", message_payload);
                                    }
                                },
                                None => {
                                    println!("Failed to parse message: {}", line_content);
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
            let app_handle_monitor = app.clone();
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
                                            proc.status = if status.success() { "completed".to_string() } else { "failed".to_string() };
                                            proc.end_time = Some(get_timestamp());
                                        }
                                    }

                                    // Emit status update event with task_id
                                    let final_status = if status.success() { "completed" } else { "failed" };
                                    let task_id = proc_map.get(&process_id_monitor)
                                        .map(|p| p.task_id.clone())
                                        .unwrap_or_else(|| "unknown".to_string());

                                    let status_payload = serde_json::json!({
                                        "process_id": process_id_monitor,
                                        "task_id": task_id,
                                        "status": final_status
                                    });

                                    match app_handle_monitor.emit("agent_process_status", status_payload.clone()) {
                                        Ok(_) => println!("✅ Emitted agent_process_status event: {} {} for task {}", process_id_monitor, final_status, task_id),
                                        Err(e) => println!("❌ Failed to emit completion status event: {:?}", e)
                                    };

                                    // Also broadcast to HTTP clients
                                    crate::web::broadcast_to_http("agent_process_status", status_payload);

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
            // Log detailed error information
            println!("Failed to spawn Claude Code process: {:?}", e);
            println!("Error kind: {:?}", e.kind());
            println!("Current working directory: {:?}", std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("unknown")));
            println!("Environment PATH: {:?}", std::env::var("PATH").unwrap_or_else(|_| "not found".to_string()));

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

/// Spawns a new Codex (ChatGPT Code) process
pub fn spawn_codex_process(
    app: tauri::AppHandle,
    task_id: String,
    initial_message: String,
    worktree_path: String,
    context: Option<String>,
) -> Result<String, String> {
    let process_id = generate_process_id();
    println!("Spawning Codex process {} for task {}", process_id, task_id);

    // Try codex.cmd directly first, then fallback to npx if needed
    let mut cmd = None;
    
    // First try direct codex.cmd command
    println!("Testing codex.cmd availability...");
    let mut codex_test = Command::new("cmd");
    codex_test.arg("/C")
        .arg("codex.cmd")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .current_dir(&worktree_path);
    
    for (key, value) in std::env::vars() {
        codex_test.env(key, value);
    }
    
    match codex_test.status() {
        Ok(status) if status.success() => {
            println!("Found codex.cmd, using direct command: codex.cmd exec");
            let mut working_cmd = Command::new("cmd");
            working_cmd
                .arg("/C")
                .arg("codex.cmd")
                .arg("exec")
                .arg("--json")
                .arg("--skip-git-repo-check")
                .arg("--dangerously-bypass-approvals-and-sandbox")
                .arg("--sandbox").arg("danger-full-access")
                .stdin(Stdio::piped())  // We'll pass prompt via stdin
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .current_dir(&worktree_path);

            for (key, value) in std::env::vars() {
                working_cmd.env(key, value);
            }

            cmd = Some(working_cmd);
        }
        Ok(status) => {
            println!("codex.cmd exists but failed with status: {}", status);
        }
        Err(e) => {
            println!("codex.cmd not found: {:?}, trying npx fallback...", e);
        }
    }
    
    // Fallback to npx approach if codex.cmd is not available
    if cmd.is_none() {
        println!("Testing npx availability for Codex fallback...");
        let mut npx_test = Command::new("npx");
        npx_test.arg("--version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir(&worktree_path);
        
        for (key, value) in std::env::vars() {
            npx_test.env(key, value);
        }
        
        match npx_test.status() {
            Ok(status) if status.success() => {
                println!("Found npx, using fallback Codex command: npx -y @openai/codex exec");
                let mut working_cmd = Command::new("npx");
                working_cmd
                    .arg("-y")
                    .arg("@openai/codex")
                    .arg("exec")
                    .arg("--json")
                    .arg("--skip-git-repo-check")
                    .arg("--dangerously-bypass-approvals-and-sandbox")
                    .arg("--sandbox").arg("danger-full-access")
                    .stdin(Stdio::piped())  // We'll pass prompt via stdin
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .current_dir(&worktree_path);

                for (key, value) in std::env::vars() {
                    working_cmd.env(key, value);
                }

                cmd = Some(working_cmd);
            }
            Ok(status) => {
                println!("npx exists but failed with status: {}", status);
            }
            Err(e) => {
                println!("npx not found: {:?}, trying other direct codex commands...", e);
            }
        }
    }
    
    // Final fallback to other direct codex commands if both codex.cmd and npx fail
    if cmd.is_none() {
        let codex_commands = ["codex", "codex.exe"];
        
        for command in &codex_commands {
            let mut test_cmd = Command::new(command);
            test_cmd.arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .current_dir(&worktree_path);

            for (key, value) in std::env::vars() {
                test_cmd.env(key, value);
            }

            println!("Testing fallback Codex command: {}", command);
            match test_cmd.status() {
                Ok(status) if status.success() => {
                    println!("Found working fallback Codex command: {}", command);
                    let mut working_cmd = Command::new(command);
                    working_cmd
                        .arg("exec")
                        .arg("--json")
                        .arg("--skip-git-repo-check")
                        .arg("--dangerously-bypass-approvals-and-sandbox")
                        .arg("--sandbox").arg("danger-full-access")
                        .stdin(Stdio::piped())  // Pass prompt via stdin
                        .stdout(Stdio::piped())
                        .stderr(Stdio::piped())
                        .current_dir(&worktree_path);

                    for (key, value) in std::env::vars() {
                        working_cmd.env(key, value);
                    }

                    cmd = Some(working_cmd);
                    break;
                }
                Ok(status) => {
                    println!("Command {} exists but failed with status: {}", command, status);
                }
                Err(e) => {
                    println!("Error testing command {}: {:?}", command, e);
                }
            }
        }
    }

    let Some(mut cmd) = cmd else {
        return Err(format!(
            "Codex CLI not found. Tried codex.cmd exec, npx -y @openai/codex exec, and direct codex commands. Ensure Codex is installed (npm install -g @openai/codex) or available in PATH."
        ));
    };

    // Create process record (reuse structure; messages will still parse as text if not JSON)
    let process = AgentProcess {
        id: process_id.clone(),
        task_id: task_id.clone(),
        status: "starting".to_string(),
        start_time: get_timestamp(),
        end_time: None,
        messages: vec![AgentMessage {
            id: generate_message_id(),
            sender: "system".to_string(),
            content: format!("Starting Codex agent for task: {}", task_id),
            timestamp: get_timestamp(),
            message_type: "text".to_string(),
            metadata: Some(serde_json::json!({
                "task_id": task_id,
                "worktree_path": worktree_path
            })),
        }],
        raw_output: Vec::new(),
        session_id: None,
        total_cost_usd: None,
        num_turns: None,
        worktree_path: worktree_path.clone(),
        kind: AgentKind::Codex,
    };

    {
        let processes = get_processes();
        let mut map = processes.lock().unwrap();
        map.insert(process_id.clone(), process.clone());

        let status_payload = serde_json::json!({
            "process_id": process_id,
            "task_id": task_id,
            "status": "starting"
        });
        
        match app.emit("agent_process_status", status_payload.clone()) {
            Ok(_) => println!("✅ Emitted agent_process_status event: {} starting", process_id),
            Err(e) => println!("⚠️ Failed to emit process status event: {:?}", e)
        };
        
        // Also broadcast to HTTP clients
        crate::web::broadcast_to_http("agent_process_status", status_payload);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            println!("Codex process spawned successfully with PID: {:?}", child.id());

            // Write the prompt to stdin and close it
            if let Some(stdin) = child.stdin.take() {
                use std::io::Write;
                let mut stdin_writer = stdin;
                let full_message = if let Some(ctx) = context {
                    format!("Previous conversation:\n{}\n\nNew message: {}", ctx, initial_message)
                } else {
                    initial_message.clone()
                };
                if let Err(e) = stdin_writer.write_all(full_message.as_bytes()) {
                    println!("Failed to write prompt to Codex stdin: {}", e);
                }
                if let Err(e) = stdin_writer.write_all(b"\n") {
                    println!("Failed to write newline to Codex stdin: {}", e);
                }
                // stdin_writer is dropped here, closing the pipe
            }

            let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
            let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;

            {
                let child_processes = get_child_processes();
                let mut map = child_processes.lock().unwrap();
                map.insert(process_id.clone(), child);
            }

            {
                let processes = get_processes();
                let mut map = processes.lock().unwrap();
                if let Some(proc) = map.get_mut(&process_id) {
                    proc.status = "running".to_string();
                    let status_payload = serde_json::json!({
                        "process_id": process_id,
                        "task_id": proc.task_id,
                        "status": "running"
                    });
                    
                    match app.emit("agent_process_status", status_payload.clone()) {
                        Ok(_) => println!("✅ Emitted agent_process_status event: running"),
                        Err(e) => println!("⚠️ Failed to emit process status event: {:?}", e)
                    };
                    
                    // Also broadcast to HTTP clients
                    crate::web::broadcast_to_http("agent_process_status", status_payload);
                }
            }

            // stdout reader (handle possible multi-line JSON)
            let process_id_stdout = process_id.clone();
            let processes_stdout = get_processes().clone();
            let app_handle_stdout = app.clone();
            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                let mut buf = String::new();
                for line in reader.lines() {
                    if let Ok(line_content) = line {
                        println!("Codex stdout: {}", line_content);
                        let trimmed = line_content.trim();
                        // append to buffer to allow multi-line JSON
                        if !trimmed.is_empty() {
                            if !buf.is_empty() { buf.push('\n'); }
                            buf.push_str(trimmed);
                        }

                        // Store raw output
                        {
                            let mut map = processes_stdout.lock().unwrap();
                            if let Some(proc) = map.get_mut(&process_id_stdout) {
                                proc.raw_output.push(line_content.clone());
                            }
                        }

                        // Handle multiple JSON objects on the same line
                        let json_objects = split_json_objects(trimmed);
                        let objects_found = !json_objects.is_empty();
                        
                        for json_str in json_objects {
                            if let Some(message) = parse_codex_output(&json_str) {
                                {
                                    let mut map = processes_stdout.lock().unwrap();
                                    if let Some(proc) = map.get_mut(&process_id_stdout) {
                                        proc.messages.push(message.clone());
                                        println!("Codex message stored. Total messages: {}", proc.messages.len());
                                        
                                        // Emit Tauri event for real-time updates (fixed to match Claude pattern)
                                        let message_payload = serde_json::json!({
                                            "process_id": process_id_stdout,
                                            "task_id": proc.task_id,
                                            "message": message
                                        });
                                        
                                        match app_handle_stdout.emit("agent_message_update", message_payload.clone()) {
                                            Ok(_) => println!("✅ Emitted Codex agent_message_update event for process {}", process_id_stdout),
                                            Err(e) => println!("❌ Failed to emit Codex event: {:?}", e)
                                        };
                                        
                                        // Also broadcast to HTTP clients
                                        crate::web::broadcast_to_http("agent_message_update", message_payload);
                                    }
                                }
                            }
                        }
                        
                        // If no JSON objects found, try accumulated buffer
                        if !objects_found && !buf.is_empty() {
                            if let Some(message) = parse_codex_output(&buf) {
                                buf.clear();
                                {
                                    let mut map = processes_stdout.lock().unwrap();
                                    if let Some(proc) = map.get_mut(&process_id_stdout) {
                                        proc.messages.push(message.clone());
                                        println!("Codex message stored from buffer. Total messages: {}", proc.messages.len());
                                        
                                        // Emit Tauri event for real-time updates (fixed to match Claude pattern)
                                        let message_payload = serde_json::json!({
                                            "process_id": process_id_stdout,
                                            "task_id": proc.task_id,
                                            "message": message
                                        });
                                        
                                        match app_handle_stdout.emit("agent_message_update", message_payload.clone()) {
                                            Ok(_) => println!("✅ Emitted Codex agent_message_update event from buffer for process {}", process_id_stdout),
                                            Err(e) => println!("❌ Failed to emit Codex buffer event: {:?}", e)
                                        };
                                        
                                        // Also broadcast to HTTP clients
                                        crate::web::broadcast_to_http("agent_message_update", message_payload);
                                    }
                                }
                            }
                        }
                    }
                }
            });

            // stderr reader
            let process_id_stderr = process_id.clone();
            let processes_stderr = get_processes().clone();
            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line_content) = line {
                        println!("Codex stderr: {}", line_content);
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
                }
            });

            // monitor completion
            let process_id_monitor = process_id.clone();
            let processes_monitor = get_processes().clone();
            let child_processes_monitor = get_child_processes().clone();
            let app_handle_monitor = app.clone();
            thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(1));
                let mut should_wait = true;
                while should_wait {
                    {
                        let mut child_map = child_processes_monitor.lock().unwrap();
                        if let Some(child) = child_map.get_mut(&process_id_monitor) {
                            match child.try_wait() {
                                Ok(Some(status)) => {
                                    println!("Codex process {} finished with status: {}", process_id_monitor, status);
                                    should_wait = false;
                                    let mut proc_map = processes_monitor.lock().unwrap();
                                    if let Some(proc) = proc_map.get_mut(&process_id_monitor) {
                                        if proc.status == "running" {
                                            proc.status = if status.success() { "completed".to_string() } else { "failed".to_string() };
                                            proc.end_time = Some(get_timestamp());
                                        }
                                    }
                                    // Emit status update event with task_id
                                    let final_status = if status.success() { "completed" } else { "failed" };
                                    let task_id = proc_map.get(&process_id_monitor)
                                        .map(|p| p.task_id.clone())
                                        .unwrap_or_else(|| "unknown".to_string());

                                    let status_payload = serde_json::json!({
                                        "process_id": process_id_monitor,
                                        "task_id": task_id,
                                        "status": final_status
                                    });

                                    match app_handle_monitor.emit("agent_process_status", status_payload.clone()) {
                                        Ok(_) => println!("✅ Emitted Codex agent_process_status event: {} {} for task {}", process_id_monitor, final_status, task_id),
                                        Err(e) => println!("❌ Failed to emit Codex completion status event: {:?}", e)
                                    };

                                    // Also broadcast to HTTP clients
                                    crate::web::broadcast_to_http("agent_process_status", status_payload);
                                    child_map.remove(&process_id_monitor);
                                }
                                Ok(None) => {
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
            });

            Ok(process_id)
        }
        Err(e) => {
            println!("Failed to spawn Codex process: {:?}", e);
            let processes = get_processes();
            let mut map = processes.lock().unwrap();
            if let Some(proc) = map.get_mut(&process_id) {
                proc.status = "failed".to_string();
                proc.end_time = Some(get_timestamp());
            }
            Err(format!("Failed to spawn Codex process: {}", e))
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
    app: tauri::AppHandle,
    process_id: &str,
    message: String,
    worktree_path: String,
) -> Result<String, String> {
    let processes = get_processes();

    // Get existing process and its context
    let (context, agent_kind, task_id) = {
        let map = processes.lock().unwrap();
        if let Some(proc) = map.get(process_id) {
            // Build context from existing messages
            // Limit context to last 20 messages to avoid huge prompts
            let take_last = 20usize;
            let start = proc.messages.len().saturating_sub(take_last);
            let context_messages: Vec<String> = proc.messages.iter()
                .skip(start)
                .map(|msg| format!("{}: {}", msg.sender, msg.content))
                .collect();
            let ctx = Some(context_messages.join("\n"));
            (ctx, proc.kind.clone(), proc.task_id.clone())
        } else {
            return Err("Process not found".to_string());
        }
    };

    // Spawn new process with context, matching the agent kind used previously
    let new_process_id = match agent_kind {
        AgentKind::Claude => spawn_claude_process(
            app,
            task_id,
            message,
            worktree_path,
            context,
        )?,
        AgentKind::Codex => spawn_codex_process(
            app,
            task_id,
            message,
            worktree_path,
            context,
        )?,
    };

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

/// Sends a new message continuing from an existing process, but forcing a specific agent kind
pub fn send_message_with_profile(
    app: tauri::AppHandle,
    base_process_id: &str,
    message: String,
    worktree_path: String,
    profile: &str,
) -> Result<String, String> {
    let processes = get_processes();

    // Build context from base process
    let (context, task_id) = {
        let map = processes.lock().unwrap();
        if let Some(proc) = map.get(base_process_id) {
            let take_last = 20usize;
            let start = proc.messages.len().saturating_sub(take_last);
            let context_messages: Vec<String> = proc.messages.iter()
                .skip(start)
                .map(|msg| format!("{}: {}", msg.sender, msg.content))
                .collect();
            (Some(context_messages.join("\n")), proc.task_id.clone())
        } else {
            return Err("Process not found".to_string());
        }
    };

    // Map profile to AgentKind
    let which = profile.trim().to_lowercase();
    let new_process_id = match which.as_str() {
        "codex" | "chat-codex" | "chatgpt-codex" => spawn_codex_process(
            app,
            task_id,
            message,
            worktree_path,
            context,
        )?,
        _ => spawn_claude_process(
            app,
            task_id,
            message,
            worktree_path,
            context,
        )?,
    };

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
            "message_count": proc.messages.len(),
            "kind": match proc.kind { AgentKind::Claude => "claude", AgentKind::Codex => "codex" }
        }))
        .collect()
}
