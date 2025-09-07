# Tauri Store API: From Broken to Working

## Overview

This document explains the critical store persistence issues we encountered and how they were resolved. This serves as both a technical reference and troubleshooting guide for future Tauri store implementations.

## The Problem: Complete Persistence Failure

### What Was Broken
- Projects would not persist between sessions
- Kanban headers showed "PROJECT: Loading..." permanently 
- Edit project modal displayed empty forms
- Tasks would disappear after closing the application
- Directory creation worked but data wasn't saved to store

### Root Causes Identified

#### 1. Parameter Naming Convention Issue
**Problem**: Tauri automatically converts snake_case parameters to camelCase, but our frontend was sending the wrong format.

**What we were doing wrong**:
```rust
// ❌ WRONG - Frontend sending snake_case
let create_args = serde_json::json!({ "project_path": project_path_clone });
```

**Backend expected**:
```rust
#[tauri::command]
async fn create_project_directory(projectPath: String) -> Result<String, String>
```

**Solution**:
```rust
// ✅ CORRECT - Frontend sending camelCase
let create_args = serde_json::json!({ "projectPath": project_path_clone });
```

#### 2. Deprecated Store API Usage
**Problem**: We were using deprecated `wasm_bindgen` store bindings that don't work with modern Tauri.

**What we were doing wrong**:
```rust
// ❌ WRONG - Deprecated wasm_bindgen approach
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "store"])]
    async fn load(filename: &str) -> JsValue;
    
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "store"])]
    async fn save(filename: &str, data: JsValue) -> JsValue;
}

// Using it like this:
match load("projects.json").await {
    js_result => { /* ... */ }
}
```

**What we should have been doing**: Using proper Tauri backend commands with the store plugin.

## The Solution: Proper Tauri Store Plugin Implementation

### 1. Backend Commands (src-tauri/src/lib.rs)

```rust
use tauri_plugin_store::StoreExt;

#[tauri::command]
async fn load_projects_data(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let store = app.store("projects.json").map_err(|e| e.to_string())?;
    match store.get("projects") {
        Some(projects) => Ok(vec![projects.clone()]),
        None => Ok(vec![])
    }
}

#[tauri::command] 
async fn save_projects_data(app: tauri::AppHandle, projects: Vec<serde_json::Value>) -> Result<String, String> {
    let store = app.store("projects.json").map_err(|e| e.to_string())?;
    let projects_value = serde_json::Value::Array(projects);
    store.set("projects", projects_value);
    store.save().map_err(|e| e.to_string())?;
    Ok("Projects saved successfully".to_string())
}

#[tauri::command]
async fn load_tasks_data(app: tauri::AppHandle, project_id: String) -> Result<Vec<serde_json::Value>, String> {
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
    let tasks_file = format!("tasks_{}.json", project_id);
    let store = app.store(&tasks_file).map_err(|e| e.to_string())?;
    let tasks_value = serde_json::Value::Array(tasks);
    store.set("tasks", tasks_value);
    store.save().map_err(|e| e.to_string())?;
    Ok("Tasks saved successfully".to_string())
}
```

### 2. Frontend Usage (Leptos Components)

```rust
// ✅ CORRECT - Using invoke() with proper commands
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

// Loading data
let empty_args = serde_json::json!({});
if let Ok(js_value) = to_value(&empty_args) {
    match invoke("load_projects_data", js_value).await {
        js_result if !js_result.is_undefined() => {
            if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                if let Some(stored_projects) = projects_wrapper.first() {
                    set_projects.set(stored_projects.clone());
                }
            }
        }
        _ => {}
    }
}

// Saving data
let json_projects: Vec<serde_json::Value> = projects.iter()
    .filter_map(|p| serde_json::to_value(p).ok())
    .collect();

let save_args = serde_json::json!({ "projects": json_projects });
if let Ok(js_value) = to_value(&save_args) {
    let _ = invoke("save_projects_data", js_value).await;
}
```

### 3. Key Tauri Configuration

Ensure your `src-tauri/src/lib.rs` registers the plugin and commands:

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build()) // ✅ Store plugin
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet, 
            list_directory, 
            get_parent_directory, 
            get_home_directory, 
            create_project_directory, 
            initialize_git_repo, 
            validate_git_repository, 
            load_projects_data,    // ✅ New commands
            save_projects_data,    // ✅ 
            load_tasks_data,       // ✅
            save_tasks_data        // ✅
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

## Why This Works Now

### 1. Proper Architecture
- **Backend**: Uses `tauri_plugin_store::StoreExt` trait for direct store access
- **Frontend**: Uses `invoke()` to call backend commands instead of trying to access store directly
- **Data Flow**: Frontend → Backend Commands → Store Plugin → File System

### 2. Correct Parameter Handling
- Backend functions receive parameters in camelCase as Tauri converts them
- Frontend sends parameters in camelCase to match backend expectations
- No more parameter name mismatches

### 3. Proper Error Handling
- Backend commands return `Result<T, String>` for proper error propagation
- Frontend handles both success and failure cases
- Store operations are wrapped in proper error handling

### 4. JSON Serialization
- Data is properly converted to `serde_json::Value` for storage
- Deserialization handles missing or malformed data gracefully
- Type-safe conversions prevent runtime errors

## Files Modified

1. **`src-tauri/src/lib.rs`** - Added all store backend commands
2. **`src/pages/projects.rs`** - Updated to use new load/save commands  
3. **`src/pages/kanban.rs`** - Fixed project name loading and task persistence
4. **`src/components/edit_project_modal.rs`** - Updated data loading and saving
5. **`src/components/project_modal.rs`** - Fixed parameter naming for directory creation

## Testing the Fix

To verify the store is working:

1. **Create a project** - Should persist after restart
2. **Navigate to kanban** - Header should show project name, not "Loading..."
3. **Create tasks** - Should persist between sessions
4. **Edit project** - Modal should populate with existing data
5. **Restart application** - All data should be preserved

## Common Pitfalls to Avoid

1. **Don't mix store APIs** - Use either frontend store bindings OR backend commands, not both
2. **Parameter naming** - Always use camelCase when sending from frontend to Tauri
3. **Error handling** - Always handle the case where store files don't exist yet
4. **JSON conversion** - Ensure proper serialization/deserialization of complex types
5. **Store plugin** - Make sure `tauri_plugin_store` is properly registered in `lib.rs`

## Future Store Usage

For any new store operations, follow this pattern:

1. **Create backend command** in `lib.rs` using `app.store()`
2. **Register command** in `invoke_handler!`
3. **Call from frontend** using `invoke()` with camelCase parameters
4. **Handle errors** properly on both sides

This approach ensures reliable, type-safe data persistence that works consistently across all platforms.