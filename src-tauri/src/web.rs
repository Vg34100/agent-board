use std::net::TcpListener as StdTcpListener;
use std::thread;

use axum::{
    body::Body,
    extract::State,
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response, Sse},
    response::sse::{Event, KeepAlive},
    routing::{get, post},
    Json, Router,
};
use tower_http::trace::TraceLayer;
use mime_guess::from_path;
use rust_embed::RustEmbed;
use serde::Deserialize;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;
use std::convert::Infallible;
use once_cell;
use std::env;

#[derive(Clone)]
pub struct WebState {
    pub app: tauri::AppHandle,
    pub event_sender: broadcast::Sender<String>,
}

// Global event sender for the HTTP interface
pub static EVENT_SENDER: once_cell::sync::Lazy<Arc<Mutex<Option<broadcast::Sender<String>>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(RustEmbed)]
#[folder = "../dist"]
struct Frontend;

// Debug helper to reduce logging noise unless AGENT_BOARD_DEBUG=1
fn debug_log(message: &str) {
    if env::var("AGENT_BOARD_DEBUG").unwrap_or_default() == "1" {
        println!("{}", message);
    }
}

// Check if we should filter out noisy requests
fn should_suppress_request_log(path: &str) -> bool {
    // Suppress Chrome DevTools requests and other development noise
    path.contains("/.well-known/") ||
    path.contains("/devtools/") || 
    path.contains("appspecific") ||
    path.contains("favicon.ico")
}

pub fn spawn(listener: StdTcpListener, app: tauri::AppHandle) {
    // Run the web server on its own dedicated Tokio runtime to avoid coupling
    // with Tauri's async runtime and to ensure it always progresses.
    thread::spawn(move || {
        let (event_sender, _) = broadcast::channel(100);
        
        // Store the event sender globally so agent.rs can access it
        {
            let mut global_sender = EVENT_SENDER.lock().unwrap();
            *global_sender = Some(event_sender.clone());
        }
        
        let state = WebState { 
            app, 
            event_sender,
        };
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("build tokio runtime");
        rt.block_on(async move {
            let router = build_router(state);
            // Ensure nonblocking before handing to Tokio
            let _ = listener.set_nonblocking(true);
            let tokio_listener = tokio::net::TcpListener::from_std(listener)
                .expect("to create tokio listener");
            println!("Axum server: starting accept loop");
            if let Err(e) = axum::serve(tokio_listener, router).await {
                eprintln!("Axum server error: {e}");
            }
        });
    });
}

fn build_router(state: WebState) -> Router {
    Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/", get(index))
        .route("/index.html", get(index))
        .route("/api/invoke", post(invoke))
        .route("/api/events", get(sse_handler))
        .route("/*path", get(static_asset))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

async fn index() -> impl IntoResponse {
    debug_log("GET / -> index.html");
    asset_to_response("index.html")
}

async fn static_asset(uri: Uri) -> impl IntoResponse {
    // Strip leading '/'
    let p = uri.path().trim_start_matches('/');
    
    // Only log requests if debug is enabled and not filtered
    if !should_suppress_request_log(p) {
        debug_log(&format!("GET /{}", p));
    }
    
    // If empty, serve index
    if p.is_empty() {
        return asset_to_response("index.html");
    }
    // Try exact match first
    if Frontend::get(p).is_some() {
        return asset_to_response(p);
    }
    // SPA fallback to index.html
    asset_to_response("index.html")
}

fn asset_to_response(path: &str) -> Response {
    // Try embedded first (works in release/bundled exe)
    if let Some(content) = Frontend::get(path) {
        let body = Body::from(content.data.into_owned());
        let mime = from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(body)
            .unwrap();
    }

    // In dev, fall back to reading from the on-disk dist folder so changes appear without recompiling
    #[cfg(debug_assertions)]
    if let Some(bytes) = read_from_dist(path) {
        let mime = from_path(path).first_or_octet_stream();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime.as_ref())
            .body(Body::from(bytes))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not Found"))
        .unwrap()
}

#[cfg(debug_assertions)]
fn read_from_dist(path: &str) -> Option<Vec<u8>> {
    // Candidate base directories where dist may reside in dev
    let bases = [
        PathBuf::from("dist"),
        PathBuf::from("../dist"),
        PathBuf::from("../../dist"),
    ];
    for base in bases {
        // Direct path
        let p = base.join(path);
        if p.exists() && p.is_file() {
            if let Ok(bytes) = fs::read(&p) {
                return Some(bytes);
            }
        }
        // Fallback to index.html for SPA
        let idx = base.join("index.html");
        if idx.exists() && idx.is_file() {
            if let Ok(bytes) = fs::read(&idx) {
                return Some(bytes);
            }
        }
    }
    None
}

#[derive(Deserialize)]
struct InvokeReq {
    cmd: String,
    #[serde(default)]
    args: Value,
    #[serde(default)]
    args_string: Option<String>,
}

fn parse_request_args(req: &InvokeReq) -> Value {
    // First, try args_string if it's provided and not empty
    if let Some(s) = &req.args_string {
        if !s.trim().is_empty() {
            println!("Trying to parse args_string: {}", s);
            match serde_json::from_str::<Value>(s) {
                Ok(v) => {
                    println!("Successfully parsed args_string");
                    return v;
                }
                Err(e) => {
                    println!("Failed to parse args_string: {}", e);
                }
            }
        }
    }
    
    // Fall back to the args field
    println!("Falling back to args field");
    let mut args = req.args.clone();
    
    // Handle case where args itself might be a JSON string
    if let Some(s) = args.as_str() {
        println!("args is a string, trying to parse: {}", s);
        if let Ok(v) = serde_json::from_str::<Value>(s) {
            println!("Successfully parsed args as JSON string");
            args = v;
        }
    }
    
    // Ensure we have at least an empty object if args is null
    if args.is_null() {
        println!("Args is null, defaulting to empty object");
        args = serde_json::json!({});
    }
    
    args
}

async fn invoke(State(state): State<WebState>, Json(req): Json<InvokeReq>) -> impl IntoResponse {
    println!("POST /api/invoke cmd={}", req.cmd);
    
    // Debug: Log the raw request structure
    println!("Raw args field: {:?}", req.args);
    if let Some(ref s) = req.args_string {
        println!("Raw args_string field: {}", s);
    } else {
        println!("No args_string field provided");
    }
    
    // Map the command string to the existing Tauri command functions defined in the parent module.
    // Always return a JSON value (no HTTP error) so the browser shim can convert it to a JS value.
    let app = state.app.clone();
    
    // Improved argument parsing with better error handling
    let args = parse_request_args(&req);
    
    if let Ok(s) = serde_json::to_string(&args) {
        let preview: String = s.chars().take(300).collect();
        println!("Parsed args: {}", preview);
    } else {
        println!("Failed to serialize parsed args back to string");
    }

    use super::*;

    fn str_arg_from(args: &Value, keys: &[&str]) -> Option<String> {
        println!("str_arg_from: searching for keys {:?} in args: {}", keys, args);
        
        // Direct key lookup first (most common case)
        if let Some(obj) = args.as_object() {
            for k in keys {
                if let Some(v) = obj.get(*k) {
                    println!("Found key '{}' with value: {:?}", k, v);
                    if let Some(s) = v.as_str() { 
                        println!("Extracted string value: '{}'", s);
                        return Some(s.to_string()); 
                    }
                    // Handle case where value is not a string but can be converted
                    if !v.is_null() {
                        let s = v.to_string();
                        // Remove quotes if it's a JSON string representation
                        let cleaned = if s.starts_with('"') && s.ends_with('"') {
                            s.trim_matches('"').to_string()
                        } else {
                            s
                        };
                        println!("Converted value to string: '{}'", cleaned);
                        return Some(cleaned);
                    }
                }
            }
        }
        
        // Fallback: breadth-first search through objects/arrays
        println!("Direct lookup failed, falling back to breadth-first search");
        use std::collections::VecDeque;
        let mut q = VecDeque::new();
        q.push_back(args);
        while let Some(cur) = q.pop_front() {
            if let Some(obj) = cur.as_object() {
                for k in keys {
                    if let Some(v) = obj.get(*k) {
                        if let Some(s) = v.as_str() { 
                            println!("Found in nested search - key '{}': '{}'", k, s);
                            return Some(s.to_string()); 
                        }
                    }
                }
                for v in obj.values() { q.push_back(v); }
            } else if let Some(arr) = cur.as_array() {
                for v in arr { q.push_back(v); }
            }
        }
        
        println!("str_arg_from: No value found for keys {:?}", keys);
        None
    }
    fn array_arg_from(args: &Value, keys: &[&str]) -> Option<Vec<Value>> {
        println!("array_arg_from: searching for keys {:?} in args", keys);
        
        // Direct key lookup first
        if let Some(obj) = args.as_object() {
            for k in keys {
                if let Some(v) = obj.get(*k) {
                    println!("Found key '{}' with value: {:?}", k, v);
                    if let Some(arr) = v.as_array() { 
                        println!("Extracted array with {} items", arr.len());
                        return Some(arr.clone()); 
                    }
                }
            }
        }
        
        // Check if the whole value is an array
        if let Some(arr) = args.as_array() { 
            println!("Args itself is an array with {} items", arr.len());
            return Some(arr.clone()); 
        }
        
        // Nested search
        use std::collections::VecDeque;
        let mut q = VecDeque::new();
        q.push_back(args);
        while let Some(cur) = q.pop_front() {
            if let Some(obj) = cur.as_object() {
                for k in keys {
                    if let Some(v) = obj.get(*k) {
                        if let Some(arr) = v.as_array() { 
                            println!("Found in nested search - key '{}': array with {} items", k, arr.len());
                            return Some(arr.clone()); 
                        }
                    }
                }
                for v in obj.values() { q.push_back(v); }
            } else if let Some(arr) = cur.as_array() {
                for v in arr { q.push_back(v); }
            }
        }
        
        println!("array_arg_from: No array found for keys {:?}", keys);
        None
    }

    let out: Value = match req.cmd.as_str() {
        // Basic helpers
        "is_dev_mode" => json!(is_dev_mode()),

        "list_directory" => {
            if let Some(path) = str_arg_from(&args, &["path"]) {
                match list_directory(path).await {
                    Ok(v) => json!(v),
                    Err(_) => json!([]),
                }
            } else { json!([]) }
        }
        "get_parent_directory" => {
            if let Some(path) = str_arg_from(&args, &["path"]) {
                match get_parent_directory(path).await {
                    Ok(v) => json!(v),
                    Err(_) => json!(null),
                }
            } else { json!(null) }
        }
        "get_home_directory" => {
            match get_home_directory().await {
                Ok(v) => json!(v),
                Err(_) => json!(null),
            }
        }
        "create_project_directory" => {
            if let Some(project_path) = str_arg_from(&args, &["projectPath", "project_path"]) {
                match create_project_directory(project_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing projectPath") }
        }
        "initialize_git_repo" => {
            if let Some(project_path) = str_arg_from(&args, &["projectPath", "project_path"]) {
                match initialize_git_repo(project_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing projectPath") }
        }
        "validate_git_repository" => {
            if let Some(path) = str_arg_from(&args, &["path"]) {
                match validate_git_repository(path).await {
                    Ok(v) => json!(v),
                    Err(_) => json!(false),
                }
            } else { json!(false) }
        }

        // Store-backed project/tasks
        "load_projects_data" => match load_projects_data(app.clone()).await {
            Ok(v) => json!(v),
            Err(_) => json!([]),
        },
        "save_projects_data" => {
            if let Some(arr) = array_arg_from(&args, &["projects"]) {
                match save_projects_data(app.clone(), arr).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing projects") }
        }
        "load_tasks_data" => {
            if let Some(project_id) = str_arg_from(&args, &["projectId", "project_id"]) {
                match load_tasks_data(app.clone(), project_id).await {
                    Ok(v) => json!(v),
                    Err(_) => json!([]),
                }
            } else { json!([]) }
        }
        "save_tasks_data" => {
            if let (Some(project_id), Some(tasks)) = (
                str_arg_from(&args, &["projectId", "project_id"]),
                array_arg_from(&args, &["tasks"]),
            ) {
                match save_tasks_data(app.clone(), project_id, tasks).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing projectId/tasks") }
        }

        // Worktree and OS actions
        "create_task_worktree" => {
            if let (Some(project_path), Some(task_id)) = (
                str_arg_from(&args, &["projectPath", "project_path"]),
                str_arg_from(&args, &["taskId", "task_id"]),
            ) {
                match create_task_worktree(app.clone(), project_path, task_id).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing projectPath/taskId") }
        }
        "remove_task_worktree" => {
            if let (Some(worktree_path), Some(project_path)) = (
                str_arg_from(&args, &["worktreePath", "worktree_path"]),
                str_arg_from(&args, &["projectPath", "project_path"]),
            ) {
                match remove_task_worktree(app.clone(), worktree_path, project_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing worktreePath/projectPath") }
        }
        "open_worktree_location" => {
            if let Some(worktree_path) = str_arg_from(&args, &["worktreePath", "worktree_path"]) {
                match open_worktree_location(worktree_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing worktreePath") }
        }
        "open_worktree_in_ide" => {
            if let Some(worktree_path) = str_arg_from(&args, &["worktreePath", "worktree_path"]) {
                match open_worktree_in_ide(worktree_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing worktreePath") }
        }

        // Agent operations
        "start_agent_process" => {
            let task_id = str_arg_from(&args, &["taskId", "task_id"]);
            let task_title = str_arg_from(&args, &["taskTitle", "task_title"]);
            let task_description = str_arg_from(&args, &["taskDescription", "task_description"]);
            let worktree_path = str_arg_from(&args, &["worktreePath", "worktree_path"]);
            let profile = args
                .get("profile")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            if let (Some(task_id), Some(task_title), Some(task_description), Some(worktree_path)) = (task_id, task_title, task_description, worktree_path) {
                match start_agent_process(app.clone(), task_id, task_title, task_description, worktree_path, profile).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing taskId/taskTitle/taskDescription/worktreePath") }
        }
        "send_agent_message" => {
            if let (Some(process_id), Some(message), Some(worktree_path)) = (
                str_arg_from(&args, &["processId", "process_id"]),
                str_arg_from(&args, &["message"]),
                str_arg_from(&args, &["worktreePath", "worktree_path"]),
            ) {
                match send_agent_message(app.clone(), process_id, message, worktree_path).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing processId/message/worktreePath") }
        }
        "send_agent_message_with_profile" => {
            if let (Some(process_id), Some(message), Some(worktree_path), Some(profile)) = (
                str_arg_from(&args, &["processId", "process_id"]),
                str_arg_from(&args, &["message"]),
                str_arg_from(&args, &["worktreePath", "worktree_path"]),
                str_arg_from(&args, &["profile"]),
            ) {
                match send_agent_message_with_profile(app.clone(), process_id, message, worktree_path, profile).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing processId/message/worktreePath/profile") }
        }
        "get_process_list" => match get_process_list().await {
            Ok(v) => json!(v),
            Err(_) => json!([]),
        },
        "get_process_details" => {
            if let Some(process_id) = str_arg_from(&args, &["processId", "process_id"]) {
                match get_process_details(process_id).await {
                    Ok(v) => json!(v),
                    Err(_) => json!(null),
                }
            } else { json!(null) }
        }
        "get_agent_messages" => {
            if let Some(process_id) = str_arg_from(&args, &["processId", "process_id"]) {
                match get_agent_messages(process_id).await {
                    Ok(v) => json!(v),
                    Err(_) => json!([]),
                }
            } else { json!([]) }
        }
        "kill_agent_process" => {
            if let Some(process_id) = str_arg_from(&args, &["processId", "process_id"]) {
                match kill_agent_process(process_id).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing processId") }
        }

        // Settings and persistence
        "load_agent_settings" => match load_agent_settings(app.clone()).await {
            Ok(v) => json!(v),
            Err(_) => json!({}),
        },
        "save_agent_settings" => {
            match serde_json::from_value(args.clone()) {
                Ok(settings) => match save_agent_settings(app.clone(), settings).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                },
                Err(_) => json!("Invalid settings"),
            }
        }
        "load_task_agent_messages" => {
            if let Some(task_id) = str_arg_from(&args, &["taskId", "task_id"]) {
                match load_task_agent_messages(app.clone(), task_id).await {
                    Ok(v) => json!(v),
                    Err(_) => json!([]),
                }
            } else { json!([]) }
        }
        "save_task_agent_messages" => {
            if let (Some(task_id), Some(messages_val)) = (
                str_arg_from(&args, &["taskId", "task_id"]),
                args.get("messages"),
            ) {
                match serde_json::from_value(messages_val.clone()) {
                    Ok(messages) => match save_task_agent_messages(app.clone(), task_id, messages).await {
                        Ok(v) => json!(v),
                        Err(e) => json!(e),
                    },
                    Err(_) => json!("Invalid messages"),
                }
            } else { json!("Missing taskId/messages") }
        }
        "load_process_agent_messages" => {
            if let (Some(task_id), Some(process_id)) = (
                str_arg_from(&args, &["taskId", "task_id"]),
                str_arg_from(&args, &["processId", "process_id"]) ,
            ) {
                match load_process_agent_messages(app.clone(), task_id, process_id).await {
                    Ok(v) => json!(v),
                    Err(_) => json!([]),
                }
            } else { json!([]) }
        }
        "save_process_agent_messages" => {
            if let (Some(task_id), Some(process_id), Some(messages_val)) = (
                str_arg_from(&args, &["taskId", "task_id"]),
                str_arg_from(&args, &["processId", "process_id"]) ,
                args.get("messages"),
            ) {
                match serde_json::from_value(messages_val.clone()) {
                    Ok(messages) => match save_process_agent_messages(app.clone(), task_id, process_id, messages).await {
                        Ok(v) => json!(v),
                        Err(e) => json!(e),
                    },
                    Err(_) => json!("Invalid messages"),
                }
            } else { json!("Missing taskId/processId/messages") }
        }
        "load_agent_processes" => match load_agent_processes(app.clone()).await {
            Ok(v) => json!(v),
            Err(_) => json!([]),
        },
        "save_agent_processes" => {
            if let Some(processes) = array_arg_from(&args, &["processes"]) {
                match save_agent_processes(app.clone(), processes).await {
                    Ok(v) => json!(v),
                    Err(e) => json!(e),
                }
            } else { json!("Missing processes") }
        }

        _ => json!({ "error": format!("Unknown command: {}", req.cmd) }),
    };

    (StatusCode::OK, Json(out))
}

// SSE handler for real-time events
async fn sse_handler(State(state): State<WebState>) -> impl IntoResponse {
    debug_log("SSE connection established");
    
    let mut receiver = state.event_sender.subscribe();
    
    let stream = async_stream::stream! {
        // Send a heartbeat event to confirm connection
        yield Ok::<Event, std::convert::Infallible>(Event::default().event("heartbeat").data("connected"));
        
        loop {
            match receiver.recv().await {
                Ok(event_data) => {
                    debug_log(&format!("Broadcasting SSE event: {}", event_data));
                    
                    // Parse the event to determine type
                    if let Ok(event_json) = serde_json::from_str::<Value>(&event_data) {
                        if let Some(event_type) = event_json.get("event").and_then(|v| v.as_str()) {
                            yield Ok::<Event, std::convert::Infallible>(Event::default().event(event_type).data(event_data));
                        } else {
                            yield Ok::<Event, std::convert::Infallible>(Event::default().event("unknown").data(event_data));
                        }
                    }
                },
                Err(broadcast::error::RecvError::Closed) => {
                    debug_log("SSE event channel closed");
                    break;
                },
                Err(broadcast::error::RecvError::Lagged(_)) => {
                    // Client is lagging, continue
                    continue;
                }
            }
        }
    };

    Sse::new(stream).keep_alive(KeepAlive::default())
}

// Function to broadcast events to HTTP clients
pub fn broadcast_to_http(event_name: &str, payload: Value) {
    if let Some(sender) = EVENT_SENDER.lock().unwrap().as_ref() {
        let event_data = json!({
            "event": event_name,
            "payload": payload
        });
        
        if let Err(e) = sender.send(event_data.to_string()) {
            debug_log(&format!("Failed to broadcast HTTP event: {}", e));
        } else {
            debug_log(&format!("✅ Broadcasted HTTP event: {} with payload: {}", event_name, payload));
        }
    }
}
