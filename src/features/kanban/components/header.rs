use leptos::prelude::*;
use leptos::task::spawn_local;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;
use crate::app::AppView;
use crate::core::models::Project;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn KanbanHeader(
    #[prop(into)] project_id: String,
    #[prop(into)] project_name: ReadSignal<String>,
    #[prop(into)] project_path: ReadSignal<Option<String>>,
    on_back: Rc<dyn Fn() + 'static>,
    on_open_modal: Rc<dyn Fn() + 'static>,
    on_open_settings: Rc<dyn Fn() + 'static>,
    on_open_edit: Rc<dyn Fn() + 'static>,
) -> impl IntoView {
    let open_files = {
        let project_path = project_path.clone();
        move |_| {
            if let Some(path) = project_path.get_untracked() {
                let args = serde_json::json!({ "worktreePath": path });
                if let Ok(js) = to_value(&args) {
                    spawn_local(async move { let _ = invoke("open_worktree_location", js).await; });
                }
            } else {
                web_sys::console::error_1(&"No project path available".into());
            }
        }
    };

    let open_ide = {
        let project_path = project_path.clone();
        move |_| {
            if let Some(path) = project_path.get_untracked() {
                let args = serde_json::json!({ "worktreePath": path });
                if let Ok(js) = to_value(&args) {
                    spawn_local(async move { let _ = invoke("open_worktree_in_ide", js).await; });
                }
            } else {
                web_sys::console::error_1(&"No project path available".into());
            }
        }
    };

    view! {
        <header class="kanban-header">
            <div class="kanban-header-left">
                <h1>{move || format!("Project: {}", project_name.get())}</h1>
                <div class="project-subactions">
                    <button class="action-btn files-btn" title="Open repository in File Explorer" on:click=open_files>"🖿"</button>
                    <button class="action-btn ide-btn" title="Open repository in IDE" on:click=open_ide>"🟐"</button>
                    <button class="action-btn edit-btn" title="Edit Project" on:click={
                        let cb = on_open_edit.clone();
                        move |_| (cb.as_ref())()
                    }>"✎"</button>
                    <button class="action-btn delete-btn" title="Delete Project" on:click={
                        let pid = project_id.clone();
                        let navigate = use_context::<WriteSignal<AppView>>().expect("AppView context");
                        move |_| {
                            if web_sys::window()
                                .map(|w| w.confirm_with_message(&"Are you sure you want to delete this project? This action cannot be undone.").unwrap_or(false))
                                .unwrap_or(false)
                            {
                                let project_id_to_delete = pid.clone();
                                let navigate = navigate.clone();
                                spawn_local(async move {
                                    let empty_args = serde_json::json!({});
                                    if let Ok(js_value) = to_value(&empty_args) {
                                        match invoke("load_projects_data", js_value).await {
                                            js_result if !js_result.is_undefined() => {
                                                if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                                                    if let Some(mut projects) = projects_wrapper.into_iter().next() {
                                                        projects.retain(|p| p.id != project_id_to_delete);
                                                        let projects_json: Vec<serde_json::Value> = projects.into_iter()
                                                            .filter_map(|project| serde_json::to_value(&project).ok())
                                                            .collect();
                                                        let save_args = serde_json::json!({ "projects": projects_json });
                                                        if let Ok(save_js) = to_value(&save_args) {
                                                            let _ = invoke("save_projects_data", save_js).await;
                                                            navigate.set(AppView::Projects);
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {
                                                web_sys::console::error_1(&"Failed to load projects for deletion".into());
                                            }
                                        }
                                    }
                                });
                            }
                        }
                    }>"🞮"</button>
                </div>
            </div>
            <div class="kanban-actions">
                <button class="btn-secondary kanban-header-btn" on:click={
                    let cb = on_back.clone();
                    move |_| (cb.as_ref())()
                }>"🡄"</button>
                <button class="btn-secondary kanban-header-btn" on:click={
                    let cb = on_open_edit.clone();
                    move |_| (cb.as_ref())()
                }>"✎"</button>
                <button class="btn-primary kanban-header-btn" on:click={
                    let cb = on_open_modal.clone();
                    move |_| (cb.as_ref())()
                }>"🞦"</button>
                <button class="btn-secondary kanban-header-btn" title="Settings" on:click={
                    let cb = on_open_settings.clone();
                    move |_| (cb.as_ref())()
                }>"⚙"</button>
            </div>
        </header>
    }
}
