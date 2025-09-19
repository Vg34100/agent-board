use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::core::models::Project;
use crate::core::services::{load_projects, save_projects};
use crate::app::AppView;

// Load project data (name and path) by project ID
pub fn load_project_data(
    project_id: String,
    set_project_name: WriteSignal<String>,
    set_project_path: WriteSignal<Option<String>>,
) {
    spawn_local(async move {
        match load_projects().await {
            Ok(projects) => {
                if let Some(project) = projects.iter().find(|p| p.id == project_id) {
                    set_project_name.set(project.name.clone());
                    set_project_path.set(Some(project.project_path.clone()));
                } else {
                    set_project_name.set("Unknown Project".to_string());
                }
            }
            Err(_) => {
                set_project_name.set("Unknown Project".to_string());
            }
        }
    });
}

// Delete project and navigate back to projects view
pub fn delete_project(project_id: String, navigate: WriteSignal<AppView>) {
    if web_sys::window()
        .map(|w| w.confirm_with_message("Are you sure you want to delete this project? This action cannot be undone.").unwrap_or(false))
        .unwrap_or(false)
    {
        spawn_local(async move {
            match load_projects().await {
                Ok(mut projects) => {
                    let original_count = projects.len();
                    projects.retain(|p| p.id != project_id);

                    if projects.len() < original_count {
                        if let Ok(()) = save_projects(&projects).await {
                            web_sys::console::log_1(&format!("Project {} deleted successfully", project_id).into());
                            navigate.set(AppView::Projects);
                        } else {
                            web_sys::console::error_1(&format!("Failed to save projects after deleting {}", project_id).into());
                        }
                    } else {
                        web_sys::console::error_1(&format!("Project {} not found for deletion", project_id).into());
                    }
                }
                Err(_) => {
                    web_sys::console::error_1(&"Failed to load projects for deletion".into());
                }
            }
        });
    }
}

// Create project update callback
pub fn create_project_update_callback(
    set_project_name: WriteSignal<String>,
) -> impl Fn(Project) + 'static {
    move |updated_project: Project| {
        set_project_name.set(updated_project.name);
    }
}