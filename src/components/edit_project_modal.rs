use leptos::prelude::*;
use leptos::{ev, html::Dialog};
use leptos::task::spawn_local;
use crate::models::Project;
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn EditProjectModal(
    #[prop(into)] project_id: String,
    #[prop(into)] on_update: Callback<Project>,
    dialog_ref: NodeRef<Dialog>,
) -> impl IntoView {
    let (project_name, set_project_name) = signal(String::new());
    let (project_path, set_project_path) = signal(String::new());
    let (loading, set_loading) = signal(true);
    
    // Load project data whenever the modal opens
    // We'll create a reactive load function that can be called
    let load_project_data = {
        let project_id = project_id.clone();
        let set_project_name = set_project_name.clone();
        let set_project_path = set_project_path.clone();
        let set_loading = set_loading.clone();
        
        move || {
            let project_id = project_id.clone();
            let set_project_name = set_project_name.clone();
            let set_project_path = set_project_path.clone();
            let set_loading = set_loading.clone();
            
            set_loading.set(true);
            
            spawn_local(async move {
                let empty_args = serde_json::json!({});
                if let Ok(js_value) = to_value(&empty_args) {
                    match invoke("load_projects_data", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                                if let Some(stored_projects) = projects_wrapper.first() {
                                    if let Some(project) = stored_projects.iter().find(|p| p.id == project_id) {
                                        set_project_name.set(project.name.clone());
                                        set_project_path.set(project.project_path.clone());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                set_loading.set(false);
            });
        }
    };
    
    // Load data initially
    load_project_data();
    
    let close_modal = move |_| {
        set_project_name.set(String::new());
        set_project_path.set(String::new());
        set_loading.set(true);
        
        if let Some(dialog) = dialog_ref.get() {
            dialog.close();
        }
    };
    
    let save_project = {
        let project_id = project_id.clone();
        let project_name = project_name.clone();
        let project_path = project_path.clone();
        let on_update = on_update.clone();
        let dialog_ref = dialog_ref.clone();
        let set_project_name = set_project_name.clone();
        let set_project_path = set_project_path.clone();
        let set_loading = set_loading.clone();
        
        move |ev: ev::SubmitEvent| {
            ev.prevent_default();
            
            if project_name.get().trim().is_empty() || project_path.get().trim().is_empty() {
                return;
            }
            
            let project_id = project_id.clone();
            let name = project_name.get().trim().to_string();
            let path = project_path.get().trim().to_string();
            let on_update = on_update.clone();
            let dialog_ref = dialog_ref.clone();
            let set_project_name = set_project_name.clone();
            let set_project_path = set_project_path.clone();
            let set_loading = set_loading.clone();
            
            spawn_local(async move {
                let empty_args = serde_json::json!({});
                if let Ok(js_value) = to_value(&empty_args) {
                    match invoke("load_projects_data", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                                if let Some(mut stored_projects) = projects_wrapper.first().cloned() {
                                    if let Some(project) = stored_projects.iter_mut().find(|p| p.id == project_id) {
                                        project.name = name;
                                        project.project_path = path;
                                        let updated_project = project.clone();
                                        
                                        // Save updated projects using proper command
                                        let json_projects: Vec<serde_json::Value> = stored_projects.iter()
                                            .filter_map(|p| serde_json::to_value(p).ok())
                                            .collect();
                                        
                                        let save_args = serde_json::json!({ "projects": json_projects });
                                        if let Ok(save_js_value) = to_value(&save_args) {
                                            let _ = invoke("save_projects_data", save_js_value).await;
                                            on_update.run(updated_project);
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                
                // Close the modal
                if let Some(dialog) = dialog_ref.get() {
                    dialog.close();
                }
                
                // Reset form state
                set_project_name.set(String::new());
                set_project_path.set(String::new());
                set_loading.set(true);
            });
        }
    };
    
    view! {
        <dialog node_ref=dialog_ref class="modal">
            <div class="modal-content">
                <div class="modal-header">
                    <h2>"Edit Project"</h2>
                    <button class="modal-close" on:click=close_modal>"×"</button>
                </div>
                
{
                    let save_project = save_project.clone();
                    let close_modal = close_modal.clone();
                    let set_project_name = set_project_name.clone();
                    let set_project_path = set_project_path.clone();
                    
                    move || {
                        if loading.get() {
                            view! {
                                <div class="loading">
                                    <p>"Loading project..."</p>
                                </div>
                            }.into_any()
                        } else {
                            let save_project = save_project.clone();
                            let close_modal = close_modal.clone();
                            let set_project_name = set_project_name.clone();
                            let set_project_path = set_project_path.clone();
                            
                            view! {
                                <form on:submit=save_project>
                                    <div class="form-group">
                                        <label for="edit-project-name">"Project Name"</label>
                                        <input 
                                            id="edit-project-name"
                                            type="text" 
                                            prop:value=project_name
                                            on:input=move |ev| set_project_name.set(event_target_value(&ev))
                                            placeholder="Project name..."
                                            required
                                        />
                                    </div>

                                    <div class="form-group">
                                        <label for="edit-project-path">"Project Path"</label>
                                        <input 
                                            id="edit-project-path"
                                            type="text" 
                                            prop:value=project_path
                                            on:input=move |ev| set_project_path.set(event_target_value(&ev))
                                            placeholder="C:\\path\\to\\project"
                                            required
                                        />
                                        <small class="form-help">"The directory where your project is located"</small>
                                    </div>

                                    <div class="modal-actions">
                                        <button type="button" class="btn-secondary" on:click=close_modal>"Cancel"</button>
                                        <button type="submit" class="btn-primary">"Save Changes"</button>
                                    </div>
                                </form>
                            }.into_any()
                        }
                    }
                }
            </div>
        </dialog>
    }
}