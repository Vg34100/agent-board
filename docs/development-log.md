# Agent Board Development Log

## Overview
This document tracks the development progress, mistakes made, and lessons learned while building Agent Board - an AI Kanban management tool using Tauri + Leptos.

## Architecture Decisions

### ✅ What's Working
- **Tauri v2 + Leptos v0.7**: Good combo for desktop applications
- **Simple Navigation**: Context-based navigation without complex routing (initially tried leptos_router but simplified)
- **CSS-first Design**: Custom CSS over component libraries for better control
- **Monospace Fonts**: Cascadia Code, Fira Code, JetBrains Mono work well for programmer aesthetic
- **Full-width Layout**: Using `100vw` and `flex: 1` for proper screen utilization
- **Signal-based State**: Leptos signals work well for reactive state management

### ❌ Common Mistakes & Lessons Learned

#### 1. **Leptos Router Complexity** (MISTAKE)
**Problem**: Tried to use `leptos_router` with complex route definitions and params
**Error**: Complex type errors with `PossibleRouteMatch` and route parameter extraction
**Solution**: Simplified to context-based navigation using enums and signals
**Lesson**: Start simple, add complexity later. Leptos router needs more setup than expected.

#### 2. **Component Callback Types** (ONGOING ISSUE)
**Problem**: Struggled with passing callbacks to components, especially with `Callback<T>` vs `Box<dyn Fn>`
**Errors**: 
- `From<closure>` not implemented for `Callback<T>`
- Optional vs required callback parameters
- Tuple vs individual parameters for closures
**Attempted Solutions**: 
- `Option<Box<dyn Fn(String, String) + 'static>>`
- `Option<Callback<(String, String)>>`
- `Option<Callback<String, String>>`
**Status**: Still working on this - callbacks are tricky in Leptos components
**Lesson**: Component callbacks need more research. Simple event handlers work, complex callbacks are hard.

#### 3. **Cargo Commands** (MISTAKE)
**Problem**: Initially tried `npm run tauri dev` instead of `cargo tauri dev`
**Solution**: Use `cargo tauri dev` for Rust-based Tauri projects
**Lesson**: Always check the project type - this is a Rust project, not Node.js

#### 4. **CSS Specificity Issues** (SOLVED)
**Problem**: Wanted full-width, square design but initial CSS was rounded and padded
**Solution**: 
- Set `height: 100vh; width: 100vw` on root elements
- Use `flex: 1` for equal column distribution  
- Remove all `border-radius` and minimize padding
- Use monospace fonts for programmer aesthetic
**Lesson**: Start with layout structure, then add styling details

#### 5. **Import/Export Cleanup** (ONGOING)
**Problem**: Unused imports causing warnings
**Status**: Need to clean up unused imports in models and components
**Lesson**: Clean up imports regularly during development

#### 6. **Build Failures** (SOLVED)
**Problem**: Hot reload was failing due to compilation errors
**Solution**: Fix all compilation errors before expecting hot reload to work
**Lesson**: Rust compilation is strict - fix errors immediately

#### 7. **File Structure** (WORKING WELL)
**Structure Used**:
```
src/
├── models/         # Data structures (Project, Task, TaskStatus)
├── pages/          # Main views (Projects, Kanban)
├── components/     # Reusable UI components (TaskModal)
└── app.rs          # Navigation and routing logic
```
**Lesson**: This structure scales well, keeps concerns separated

## Current Status (as of implementation)

### ✅ Completed Features
1. **Project Structure**: Proper Tauri + Leptos setup with dependencies
2. **Data Models**: Task, Project, TaskStatus enums and structs  
3. **Navigation**: Context-based navigation between Projects and Kanban views
4. **Kanban Board**: 5-column layout (ToDo → InProgress → InReview → Done → Cancelled)
5. **UI Design**: Square, minimal, programmer-focused dark theme design
6. **Sample Data**: Working kanban board with sample tasks displayed in columns

### 🟡 Partially Working
1. **TaskModal Component**: Created but callback integration blocked by type issues

### ❌ TODO
1. **Fix TaskModal Callbacks**: Need to resolve Leptos component callback patterns
2. **Task Creation**: Implement working "+" button to add tasks
3. **Project Creation**: Implement working "CREATE PROJECT" functionality  
4. **Data Persistence**: LocalStorage integration for saving projects/tasks
5. **Tauri Commands**: Backend integration for file operations and Git
6. **Drag & Drop**: Move tasks between columns
7. **Task Editing**: Inline editing of task titles/descriptions

## Technical Debts
1. **Unused Imports**: Clean up warnings in models and components
2. **Error Handling**: Add proper error handling throughout
3. **Type Safety**: Resolve callback type issues in components
4. **Performance**: Optimize re-renders and state updates

## Recent Major Fixes & Implementations (January 2025)

### ✅ 7. **Task Creation Reactivity Bug** (CRITICAL BUG - FIXED)
**Problem**: New tasks weren't appearing in kanban columns after creation
**Root Cause**: Kanban column rendering was using `.with()` which captured tasks at render time but wasn't reactive
**Error Pattern**: 
```rust
// NON-REACTIVE (BROKEN)
let status_tasks = tasks.with(|tasks| {
    tasks.iter().filter(|task| task.status == status).collect()
});
```
**Solution**: Made column rendering reactive using `move ||` closures
```rust
// REACTIVE (FIXED)
{move || {
    tasks.with(|tasks| {
        tasks.iter().filter(|task| task.status == status_for_tasks)
            .cloned().map(|task| view! { /* task view */ })
            .collect::<Vec<_>>()
    })
}}
```
**Lesson**: In Leptos, `.with()` is NOT reactive by itself. Must wrap in reactive closure `move ||` for UI updates

### ✅ 8. **TaskSidebar Implementation** (NEW FEATURE - COMPLETE)
**Challenge**: Create a comprehensive sidebar with status-dependent sections and agent chat interface
**Implementation Approach**:
- **Task Details**: Dynamic description with "show more" for long content
- **Status Sections**: Different UI for ToDo (create attempt) vs In Progress+ (attempt status)
- **Agent Window**: Mock chat interface with expandable sessions
- **Layout**: 50% width sidebar that squishes main content (not overlay)

**Callback Type Issues Encountered**:
```rust
// FAILED ATTEMPT 1: impl Fn + Clone
#[prop(into)] on_close: impl Fn() + Clone + 'static,
// Error: Type inference issues in Leptos 0.7

// FAILED ATTEMPT 2: Box<dyn Fn()> with closure moves
// Error: FnOnce vs FnMut, Send trait issues

// WORKING SOLUTION: Direct signal passing
#[prop(into)] selected_task: WriteSignal<Option<Task>>,
```
**Lesson**: Avoid complex callback types in Leptos. Pass signals directly for cleaner code.

### ✅ 9. **TaskModal Callback Types** (FINALLY RESOLVED)
**Previous Issue**: Complex callback types causing compilation errors
**Final Solution**: Used `Box<dyn Fn(Task) + 'static>` approach consistently
**Key Learning**: Leptos 0.7 prefers concrete types over `impl Fn` for component props

### ✅ 10. **Responsive Sidebar Layout** (UI/UX COMPLETE)
**Requirements**: Sidebar takes 50% width, kanban board squishes left (not hidden)
**Implementation**:
```css
.kanban-page {
  display: flex;
  height: 100vh;
  width: 100vw;
}

.kanban-page.with-sidebar .main-content {
  width: 50%;
  flex: none;
}

.task-sidebar {
  position: fixed;
  right: 0;
  width: 50%;
  height: 100vh;
}
```
**Result**: Smooth transition, content squishes rather than overlays

### ✅ 11. **Project Persistence System** (MAJOR FIX - COMPLETE)
**Problem**: ProjectModal was not actually creating/saving projects, kanban showed "LOADING..." for project names, EditProjectModal had styling issues
**Root Causes**: 
- Parameter name mismatch: frontend sent `projectPath` but backend expected `project_path`
- Sample tasks were hardcoded instead of loading project-specific tasks
- Missing CSS specificity for EditProjectModal styling
**Solution**:
- Fixed parameter naming in ProjectModal → ProjectCreate Tauri commands
- Implemented project-specific task storage using `tasks_{project_id}.json`
- Added proper CSS for modal headers and borders
- Tasks now load per-project and save automatically when created
**Lesson**: Always verify frontend-backend parameter naming matches exactly

### ✅ 12. **Task Storage Architecture** (MAJOR IMPROVEMENT - COMPLETE)  
**Challenge**: Replace hardcoded sample tasks with proper project-specific persistence
**Implementation**: 
- Each project stores tasks in separate files: `tasks_{project_id}.json`
- Kanban page loads tasks on mount using project ID
- TaskModal creation callback now saves tasks after adding to signal
- Empty projects show clean empty boards instead of sample data
**Result**: Proper data isolation between projects, no more fake data pollution

### ✅ 13. **Store API Modernization** (CRITICAL SYSTEM FIX - COMPLETE)
**Problem**: Complete persistence system failure - projects wouldn't save, tasks disappeared, "Loading..." headers permanent
**Root Causes Found**: 
1. **Parameter Naming Convention**: Tauri auto-converts snake_case → camelCase, but frontend was sending wrong format
2. **Deprecated API Usage**: Using old `wasm_bindgen` store bindings instead of proper Tauri plugin commands

**Incorrect Implementation**:
```rust
// ❌ WRONG - Deprecated wasm_bindgen store API
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "store"])]
    async fn load(filename: &str) -> JsValue;
    async fn save(filename: &str, data: JsValue) -> JsValue;
}

// ❌ WRONG - Parameter naming
let args = serde_json::json!({ "project_path": path }); // Backend expects "projectPath"
```

**Correct Implementation**:
```rust
// ✅ CORRECT - Proper Tauri backend commands
#[tauri::command]
async fn load_projects_data(app: tauri::AppHandle) -> Result<Vec<serde_json::Value>, String> {
    let store = app.store("projects.json").map_err(|e| e.to_string())?;
    match store.get("projects") {
        Some(projects) => Ok(vec![projects.clone()]),
        None => Ok(vec![])
    }
}

// ✅ CORRECT - Frontend using invoke() with camelCase parameters
let args = serde_json::json!({ "projectPath": path });
match invoke("create_project_directory", js_value).await {
    // Proper handling
}
```

**Files Completely Rewritten**:
- `src-tauri/src/lib.rs` - Added 4 new store commands
- `src/pages/projects.rs` - Updated to use modern API
- `src/pages/kanban.rs` - Fixed project name loading and task persistence  
- `src/components/edit_project_modal.rs` - Updated data loading/saving
- `src/components/project_modal.rs` - Fixed parameter naming

**Result**: Complete persistence system now works perfectly - projects save, tasks persist, editing works
**Critical Learning**: Always use proper Tauri plugin APIs, never try to access store directly from frontend

## Updated Next Steps (Phase 3)
1. **Priority 1**: ✅ COMPLETED - Store API modernization and complete persistence fix
2. **Priority 2**: Add drag & drop for task status changes  
3. **Priority 3**: ✅ COMPLETED - Project creation modal functionality  
4. **Priority 4**: Add auto-save for task editing/deletion actions
5. **Priority 5**: Implement git worktree integration

## Resources & References
- [Leptos Book](https://leptos-rs.github.io/leptos/) - Component patterns and examples
- [Tauri Docs](https://tauri.app/docs/) - Desktop app integration
- [Leptos Examples](https://github.com/leptos-rs/leptos/tree/main/examples) - Real-world patterns
- [Tauri Store Plugin](https://github.com/tauri-apps/plugins-workspace/tree/v2/plugins/store) - Official store plugin documentation