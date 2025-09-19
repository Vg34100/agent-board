use leptos::prelude::*;
use leptos::html::Dialog;
use leptos::task::spawn_local;
use crate::app::AppView;
use crate::components::ProjectModal;
use crate::core::models::{Project, Task, TaskStatus};
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;
use std::collections::HashMap;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn Projects() -> impl IntoView {
    let navigate = use_context::<WriteSignal<AppView>>().expect("navigate context");
    let (projects, set_projects) = signal(Vec::<Project>::new());
    let (project_task_counts, set_project_task_counts) = signal(HashMap::<String, (usize, usize)>::new());
    let project_modal_ref = NodeRef::<Dialog>::new();
    
    // Load projects from Tauri store on component mount using proper commands
    {
        let set_projects = set_projects.clone();
        let set_project_task_counts = set_project_task_counts.clone();
        spawn_local(async move {
            let empty_args = serde_json::json!({});
            if let Ok(js_value) = to_value(&empty_args) {
                match invoke("load_projects_data", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                            if let Some(stored_projects) = projects_wrapper.first() {
                                set_projects.set(stored_projects.clone());
                                
                                // Load task counts for each project
                                let mut task_counts = HashMap::new();
                                for project in stored_projects {
                                    let project_id = project.id.clone();
                                    let load_args = serde_json::json!({ "projectId": project_id });
                                    if let Ok(js_value) = to_value(&load_args) {
                                        match invoke("load_tasks_data", js_value).await {
                                            js_result if !js_result.is_undefined() => {
                                                if let Ok(tasks) = serde_wasm_bindgen::from_value::<Vec<Task>>(js_result) {
                                                    let total_tasks = tasks.len();
                                                    let in_progress_count = tasks.iter()
                                                        .filter(|task| matches!(task.status, TaskStatus::InProgress))
                                                        .count();
                                                    task_counts.insert(project_id, (total_tasks, in_progress_count));
                                                } else {
                                                    task_counts.insert(project_id, (0, 0));
                                                }
                                            }
                                            _ => {
                                                task_counts.insert(project_id, (0, 0));
                                            }
                                        }
                                    }
                                }
                                set_project_task_counts.set(task_counts);
                            }
                        }
                    }
                    _ => {
                        // No projects stored yet, start with empty list
                    }
                }
            }
        });
    }
    
    let open_project = move |project_id: &str| {
        let id = project_id.to_string();
        navigate.set(AppView::Kanban(id));
    };

    let open_project_modal = move |_| {
        if let Some(dialog) = project_modal_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    let create_project = Callback::new(move |project: Project| {
        set_projects.update(|projects| {
            projects.push(project.clone());
        });
        
        // Save updated projects using proper Tauri command
        let projects_to_save = projects.get_untracked();
        spawn_local(async move {
            // Convert to JSON values for the backend
            let json_projects: Vec<serde_json::Value> = projects_to_save.iter()
                .filter_map(|p| serde_json::to_value(p).ok())
                .collect();
            
            let save_args = serde_json::json!({ "projects": json_projects });
            if let Ok(js_value) = to_value(&save_args) {
                let _ = invoke("save_projects_data", js_value).await;
            }
        });
    });

    view! {
        <div class="projects-page">
            <header class="projects-header">
                <h1>"Agent Board"</h1>
                <button class="btn-primary" on:click=open_project_modal>"+ CREATE PROJECT"</button>
            </header>
            
            <div class="projects-grid">
                {move || {
                    let project_list = projects.get();
                    if project_list.is_empty() {
                        view! {
                            <div class="empty-state">
                                <p>"No projects yet. Create your first project to get started!"</p>
                            </div>
                        }.into_any()
                    } else {
                        project_list.into_iter().map(|project| {
                            let project_id = project.id.clone();
                            let project_name = project.name.clone();
                            let git_status = if project.git_path.is_some() { 
                                "Existing Repository" 
                            } else { 
                                "New Repository" 
                            };
                            
                            // Get task counts from the signal
                            let task_counts = project_task_counts.get();
                            let (total_tasks, in_progress_count) = task_counts.get(&project.id)
                                .copied()
                                .unwrap_or((0, 0));
                            
                            view! {
                                <div class="project-card" on:click=move |_| open_project(&project_id)>
                                    <h3>{project_name}</h3>
                                    <p>{git_status}</p>
                                    <div class="project-stats">
                                        <span>{format!("{} tasks", total_tasks)}</span>
                                        <span>{format!("{} in progress", in_progress_count)}</span>
                                    </div>
                                </div>
                            }
                        }).collect::<Vec<_>>().into_any()
                    }
                }}
            </div>
            
            <ProjectModal 
                on_create=create_project
                dialog_ref=project_modal_ref
            />
        </div>
    }
}