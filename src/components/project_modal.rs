use leptos::prelude::*;
use leptos::{ev, html::Dialog};
use leptos::task::spawn_local;
use crate::models::Project;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DirectoryItem {
    pub name: String,
    pub is_directory: bool,
    pub path: String,
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Debug, Clone, PartialEq)]
enum ModalStep {
    SourceSelection,
    CreateBlank,
    FromGitRepo,
    DirectoryBrowser,
}

#[component]
pub fn ProjectModal(
    #[prop(into)] on_create: Callback<Project>,
    dialog_ref: NodeRef<Dialog>,
) -> impl IntoView {
    let (current_step, set_current_step) = signal(ModalStep::SourceSelection);
    let (project_name, set_project_name) = signal(String::new());
    let (parent_directory, set_parent_directory) = signal(String::new());
    let (selected_git_repo, set_selected_git_repo) = signal(String::new());
    
    // Directory browser state
    let (current_directory, set_current_directory) = signal("C:\\Users".to_string());
    let (manual_path, set_manual_path) = signal(String::new());
    let (directory_search, set_directory_search) = signal(String::new());
    let (directory_items, set_directory_items) = signal(Vec::<DirectoryItem>::new());

    // Directory loading function
    let load_directory = {
        let set_directory_items = set_directory_items.clone();
        move |path: String| {
            let set_directory_items = set_directory_items.clone();
            spawn_local(async move {
                let args = serde_json::json!({ "path": path });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("list_directory", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(items) = serde_wasm_bindgen::from_value::<Vec<DirectoryItem>>(js_result) {
                                set_directory_items.set(items);
                            }
                        }
                        _ => {
                            set_directory_items.set(Vec::new());
                        }
                    }
                }
            });
        }
    };

    let close_modal = move |_| {
        // Reset all state
        set_current_step.set(ModalStep::SourceSelection);
        set_project_name.set(String::new());
        set_parent_directory.set(String::new());
        set_selected_git_repo.set(String::new());
        set_current_directory.set("C:\\Users".to_string());
        set_manual_path.set(String::new());
        set_directory_search.set(String::new());
        set_directory_items.set(Vec::new());
        
        if let Some(dialog) = dialog_ref.get() {
            dialog.close();
        }
    };

    // Load directory when entering DirectoryBrowser step
    Effect::new(move |_| {
        if current_step.get() == ModalStep::DirectoryBrowser {
            let current_dir = current_directory.get();
            load_directory(current_dir);
        }
    });

    let go_back = move |_| {
        match current_step.get() {
            ModalStep::CreateBlank | ModalStep::FromGitRepo => {
                set_current_step.set(ModalStep::SourceSelection);
            }
            ModalStep::DirectoryBrowser => {
                set_current_step.set(ModalStep::CreateBlank);
            }
            _ => {}
        }
    };

    view! {
        <dialog node_ref=dialog_ref class="modal project-modal">
            <div class="modal-content">
                {move || match current_step.get() {
                    ModalStep::SourceSelection => view! {
                        <div class="modal-header">
                            <h2>"Create Project"</h2>
                            <button class="modal-close" on:click=close_modal>"×"</button>
                        </div>
                        
                        <div class="modal-subheader">
                            <h3>"Choose your repository source"</h3>
                        </div>
                        
                        <div class="source-selection">
                            <button 
                                class="source-option"
                                on:click=move |_| set_current_step.set(ModalStep::FromGitRepo)
                            >
                                <div class="source-icon">"🗁"</div>
                                <div class="source-content">
                                    <h3>"From Git Repository"</h3>
                                    <p>"Use an existing repo as your project base"</p>
                                </div>
                            </button>
                            
                            <button 
                                class="source-option"
                                on:click=move |_| set_current_step.set(ModalStep::CreateBlank)
                            >
                                <div class="source-icon">"✚"</div>
                                <div class="source-content">
                                    <h3>"Create Blank Project"</h3>
                                    <p>"Start a new project from scratch"</p>
                                </div>
                            </button>
                        </div>
                    }.into_any(),
                    
                    ModalStep::CreateBlank => view! {
                        <div class="modal-header">
                            <h2>"Create Blank Project"</h2>
                            <button class="modal-close" on:click=close_modal>"×"</button>
                        </div>
                        
                        <div class="modal-nav">
                            <button class="back-button" on:click=go_back>"◀ Back to options"</button>
                        </div>
                        
                        <form on:submit=move |ev: ev::SubmitEvent| {
                            ev.prevent_default();
                            
                            if project_name.get().trim().is_empty() {
                                return;
                            }
                            
                            // Sanitize project name
                            let sanitized_name = project_name.get()
                                .trim()
                                .to_lowercase()
                                .replace(" ", "_")
                                .chars()
                                .filter(|c| c.is_alphanumeric() || *c == '_' || *c == '-')
                                .collect::<String>();
                            
                            let base_dir = if parent_directory.get().trim().is_empty() {
                                "C:\\Users".to_string() // Default home directory for Windows
                            } else {
                                parent_directory.get().trim().to_string()
                            };
                            
                            let project_path = format!("{}\\{}", base_dir, sanitized_name);
                            
                            // Create the project directory and initialize git repo
                            let project_path_clone = project_path.clone();
                            let project_name_clone = project_name.get().trim().to_string();
                            let on_create_clone = on_create.clone();
                            
                            spawn_local(async move {
                                // Create directory - Tauri converts snake_case to camelCase
                                let create_args = serde_json::json!({ "projectPath": project_path_clone });
                                if let Ok(js_value) = to_value(&create_args) {
                                    match invoke("create_project_directory", js_value).await {
                                        js_result if !js_result.is_undefined() => {
                                            // Directory created, now initialize git repo
                                            let git_args = serde_json::json!({ "projectPath": project_path_clone });
                                            if let Ok(git_js_value) = to_value(&git_args) {
                                                let _ = invoke("initialize_git_repo", git_js_value).await;
                                            }
                                            
                                            // Create project with the actual path
                                            let project = Project::new_git_project(
                                                project_name_clone,
                                                project_path_clone
                                            );
                                            
                                            on_create_clone.run(project);
                                        }
                                        _ => {
                                            // Handle error - could show toast/notification
                                            // For now, just don't create the project
                                        }
                                    }
                                }
                            });
                            
                            // Close modal after successful creation
                            if let Some(dialog) = dialog_ref.get() {
                                dialog.close();
                            }
                            
                            // Reset state
                            set_current_step.set(ModalStep::SourceSelection);
                            set_project_name.set(String::new());
                            set_parent_directory.set(String::new());
                        }>
                            <div class="form-group">
                                <label for="project-name">"Project Name"</label>
                                <input 
                                    id="project-name"
                                    type="text" 
                                    prop:value=project_name
                                    on:input=move |ev| set_project_name.set(event_target_value(&ev))
                                    placeholder="my awesome project"
                                    required
                                />
                                <small class="form-help">"The folder name will be auto-generated from the project name"</small>
                            </div>

                            <div class="form-group">
                                <label for="parent-directory">"Parent Directory"</label>
                                <div class="directory-input">
                                    <input 
                                        id="parent-directory"
                                        type="text" 
                                        prop:value=parent_directory
                                        on:input=move |ev| set_parent_directory.set(event_target_value(&ev))
                                        placeholder="C:\\Users"
                                    />
                                    <button 
                                        type="button" 
                                        class="browse-button"
                                        on:click=move |_| set_current_step.set(ModalStep::DirectoryBrowser)
                                    >"🗁"</button>
                                </div>
                                <small class="form-help">"Leave empty to use your home directory, or specify a custom path"</small>
                            </div>

                            <div class="modal-actions">
                                <button type="button" class="btn-secondary" on:click=close_modal>"Cancel"</button>
                                <button type="submit" class="btn-primary">"Create Project"</button>
                            </div>
                        </form>
                    }.into_any(),
                    
                    ModalStep::FromGitRepo => view! {
                        <div class="modal-header">
                            <h2>"From Git Repository"</h2>
                            <button class="modal-close" on:click=close_modal>"×"</button>
                        </div>
                        
                        <div class="modal-nav">
                            <button class="back-button" on:click=go_back>"◀ Back to options"</button>
                        </div>
                        
                        <div class="git-repo-selection">
                            <p>"Select an existing Git repository directory:"</p>
                            
                            <div class="form-group">
                                <label for="git-repo-path">"Repository Path"</label>
                                <div class="directory-input">
                                    <input 
                                        id="git-repo-path"
                                        type="text" 
                                        prop:value=selected_git_repo
                                        on:input=move |ev| set_selected_git_repo.set(event_target_value(&ev))
                                        placeholder="C:\\path\\to\\repository"
                                    />
                                    <button 
                                        type="button" 
                                        class="browse-button"
                                        on:click=move |_| set_current_step.set(ModalStep::DirectoryBrowser)
                                    >"🗁"</button>
                                </div>
                                <small class="form-help">"Must be a directory containing a .git folder"</small>
                            </div>

                            <div class="modal-actions">
                                <button type="button" class="btn-secondary" on:click=close_modal>"Cancel"</button>
                                <button 
                                    type="button" 
                                    class="btn-primary"
                                    disabled=move || selected_git_repo.get().trim().is_empty()
                                    on:click=move |_| {
                                        // TODO: Validate .git folder exists
                                        let repo_path = selected_git_repo.get().trim().to_string();
                                        let project_name = repo_path.split("\\").last().unwrap_or("Unknown").to_string();
                                        
                                        // Validate that it's actually a git repository
                                        let validation_args = serde_json::json!({ "path": repo_path });
                                        if let Ok(js_value) = to_value(&validation_args) {
                                            spawn_local(async move {
                                                match invoke("validate_git_repository", js_value).await {
                                                    js_result if !js_result.is_undefined() => {
                                                        if let Ok(is_valid) = serde_wasm_bindgen::from_value::<bool>(js_result) {
                                                            if is_valid {
                                                                let project = Project::new_existing_project(
                                                                    project_name,
                                                                    repo_path
                                                                );
                                                                
                                                                on_create.run(project);
                                                            } else {
                                                                // TODO: Show error that directory is not a git repository
                                                            }
                                                        }
                                                    }
                                                    _ => {
                                                        // TODO: Handle validation error
                                                    }
                                                }
                                            });
                                        }
                                        
                                        // Close modal after successful creation
                                        if let Some(dialog) = dialog_ref.get() {
                                            dialog.close();
                                        }
                                        
                                        // Reset state
                                        set_current_step.set(ModalStep::SourceSelection);
                                        set_selected_git_repo.set(String::new());
                                    }
                                >"Use Repository"</button>
                            </div>
                        </div>
                    }.into_any(),
                    
                    ModalStep::DirectoryBrowser => view! {
                        <div class="modal-header">
                            <h2>"Select Parent Directory"</h2>
                            <button class="modal-close" on:click=close_modal>"×"</button>
                        </div>
                        
                        <div class="modal-subheader">
                            <h3>"Choose where to create the new repository"</h3>
                        </div>
                        
                        <div class="directory-browser">
                            <div class="manual-path">
                                <label>"Enter path manually:"</label>
                                <div class="path-input">
                                    <input 
                                        type="text" 
                                        prop:value=manual_path
                                        on:input=move |ev| set_manual_path.set(event_target_value(&ev))
                                        placeholder=current_directory
                                    />
                                    <button 
                                        type="button" 
                                        class="go-button"
                                        on:click=move |_| {
                                            if !manual_path.get().trim().is_empty() {
                                                let new_path = manual_path.get().trim().to_string();
                                                set_current_directory.set(new_path.clone());
                                                load_directory(new_path);
                                            }
                                        }
                                    >"Go"</button>
                                </div>
                            </div>

                            <div class="search-directory">
                                <label>"Search current directory:"</label>
                                <input 
                                    type="text" 
                                    prop:value=directory_search
                                    on:input=move |ev| set_directory_search.set(event_target_value(&ev))
                                    placeholder="Filter folders..."
                                />
                            </div>

                            <div class="directory-navigation">
                                <button 
                                    class="nav-button" 
                                    title="Home"
                                    on:click=move |_| {
                                        spawn_local({
                                            let set_current_directory = set_current_directory.clone();
                                            let load_directory = load_directory.clone();
                                            async move {
                                                let args = serde_json::json!({});
                                                if let Ok(js_value) = to_value(&args) {
                                                    if let Ok(home_result) = serde_wasm_bindgen::from_value::<String>(invoke("get_home_directory", js_value).await) {
                                                        set_current_directory.set(home_result.clone());
                                                        load_directory(home_result);
                                                    }
                                                }
                                            }
                                        });
                                    }
                                >"~"</button>
                                <button 
                                    class="nav-button" 
                                    title="Up"
                                    on:click=move |_| {
                                        let current = current_directory.get();
                                        spawn_local({
                                            let set_current_directory = set_current_directory.clone();
                                            let load_directory = load_directory.clone();
                                            async move {
                                                let args = serde_json::json!({ "path": current });
                                                if let Ok(js_value) = to_value(&args) {
                                                    if let Ok(parent_result) = serde_wasm_bindgen::from_value::<String>(invoke("get_parent_directory", js_value).await) {
                                                        set_current_directory.set(parent_result.clone());
                                                        load_directory(parent_result);
                                                    }
                                                }
                                            }
                                        });
                                    }
                                >"↑"</button>
                                <span class="current-path">{current_directory}</span>
                                <button 
                                    class="select-current-button"
                                    on:click=move |_| {
                                        set_parent_directory.set(current_directory.get());
                                        set_current_step.set(ModalStep::CreateBlank);
                                    }
                                >"Select Current"</button>
                            </div>

                            <div class="folder-list">
                                {move || {
                                    let items = directory_items.get();
                                    let search_term = directory_search.get();
                                    
                                    let filtered_items: Vec<DirectoryItem> = if search_term.is_empty() {
                                        items
                                    } else {
                                        items.into_iter()
                                            .filter(|item| item.name.to_lowercase().contains(&search_term.to_lowercase()))
                                            .collect()
                                    };
                                    
                                    if filtered_items.is_empty() {
                                        view! {
                                            <div class="folder-item empty">"No folders found"</div>
                                        }.into_any()
                                    } else {
                                        filtered_items.into_iter().map(|item| {
                                            let item_clone = item.clone();
                                            let icon = if item.is_directory { "📁" } else { "📄" };
                                            
                                            view! {
                                                <div 
                                                    class="folder-item"
                                                    class:is-directory=item.is_directory
                                                    on:click=move |_| {
                                                        if item_clone.is_directory {
                                                            set_current_directory.set(item_clone.path.clone());
                                                            load_directory(item_clone.path.clone());
                                                        }
                                                    }
                                                >{icon} " " {item.name}</div>
                                            }
                                        }).collect::<Vec<_>>().into_any()
                                    }
                                }}
                            </div>

                            <div class="directory-actions">
                                <button type="button" class="btn-secondary" on:click=go_back>"Cancel"</button>
                                <button 
                                    type="button" 
                                    class="btn-primary"
                                    on:click=move |_| {
                                        let path = if manual_path.get().trim().is_empty() {
                                            current_directory.get()
                                        } else {
                                            manual_path.get().trim().to_string()
                                        };
                                        set_parent_directory.set(path);
                                        set_current_step.set(ModalStep::CreateBlank);
                                    }
                                >"Select Path"</button>
                            </div>
                        </div>
                    }.into_any(),
                }}
            </div>
        </dialog>
    }
}