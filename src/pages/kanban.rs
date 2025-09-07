use leptos::prelude::*;
use leptos::html::Dialog;
use leptos::task::spawn_local;
use crate::app::AppView;
use crate::models::{Task, TaskStatus, Project};
use crate::components::{TaskModal, TaskSidebar, EditTaskModal, EditProjectModal};
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn Kanban(project_id: String) -> impl IntoView {
    // Get the navigation signal from context - this allows us to change the current view
    // The expect() will panic if the context wasn't provided, which helps catch setup errors
    let navigate = use_context::<WriteSignal<AppView>>().expect("navigate context");
    
    // Project name signal
    let (project_name, set_project_name) = signal(String::from("Loading..."));
    
    // Load project name from store using proper Tauri commands
    {
        let project_id_clone = project_id.clone();
        let set_project_name = set_project_name.clone();
        spawn_local(async move {
            let empty_args = serde_json::json!({});
            if let Ok(js_value) = to_value(&empty_args) {
                match invoke("load_projects_data", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(projects_wrapper) = serde_wasm_bindgen::from_value::<Vec<Vec<Project>>>(js_result) {
                            if let Some(stored_projects) = projects_wrapper.first() {
                                if let Some(project) = stored_projects.iter().find(|p| p.id == project_id_clone) {
                                    set_project_name.set(project.name.clone());
                                } else {
                                    set_project_name.set("Unknown Project".to_string());
                                }
                            } else {
                                set_project_name.set("Unknown Project".to_string());
                            }
                        }
                    }
                    _ => {
                        set_project_name.set("Unknown Project".to_string());
                    }
                }
            }
        });
    }
    
    // Create a reactive signal to hold the tasks list
    // Signal automatically triggers re-renders when the data changes
    let (tasks, set_tasks) = signal(Vec::<Task>::new());
    
    // Load project-specific tasks from storage
    {
        let project_id_clone = project_id.clone();
        let set_tasks = set_tasks.clone();
        spawn_local(async move {
            let load_args = serde_json::json!({ "projectId": project_id_clone });
            if let Ok(js_value) = to_value(&load_args) {
                match invoke("load_tasks_data", js_value).await {
                    js_result if !js_result.is_undefined() => {
                        if let Ok(tasks_json) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(js_result) {
                            let tasks: Vec<Task> = tasks_json.into_iter()
                                .filter_map(|v| serde_json::from_value(v).ok())
                                .collect();
                            set_tasks.set(tasks);
                        }
                    }
                    _ => {
                        // No tasks exist yet, start with empty list
                        set_tasks.set(Vec::new());
                    }
                }
            }
        });
    }
    
    // Simple save function for tasks (we'll implement auto-save later)
    // For now, tasks will be saved when creating new tasks via TaskModal
    
    // Create a signal to track which task is currently selected for the sidebar
    // None means no sidebar is open, Some(task) means sidebar is showing that task
    let (selected_task, set_selected_task) = signal::<Option<Task>>(None);
    
    // Track which dropdown is currently open (task ID)
    let (open_dropdown, set_open_dropdown) = signal::<Option<String>>(None);
    
    // Create references to the HTML dialog elements that we can use to control them
    // from Rust code (open/close modals programmatically)
    let dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let edit_dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let edit_project_dialog_ref: NodeRef<Dialog> = NodeRef::new();
    
    // Track which task is being edited
    let (editing_task, set_editing_task) = signal::<Option<Task>>(None);

    // Navigation handlers
    let back_to_projects = move |_| {
        // Change the app view back to the Projects page
        navigate.set(AppView::Projects);
    };
    
    // Handler for the "+" button to open the task creation modal
    let open_modal = move |_| {
        // Get the dialog DOM element and call show_modal() to display it as a modal
        // show_modal() makes it block interaction with the rest of the page
        // We ignore the Result using let _ = since we don't need to handle the error case here
        if let Some(dialog) = dialog_ref.get() {
            let _ = dialog.show_modal();
        }
    };
    
    // Handler to open the edit project modal
    let open_edit_project_modal = move |_| {
        if let Some(dialog) = edit_project_dialog_ref.get() {
            let _ = dialog.show_modal();
        }
    };
    
    // Handler for when project is updated
    let update_project = {
        let set_project_name = set_project_name.clone();
        Callback::new(move |updated_project: Project| {
            set_project_name.set(updated_project.name);
        })
    };
    
    // Callback function that gets called when TaskModal creates a new task
    // This function takes ownership of the Task and adds it to the kanban board
    let create_task = {
        let project_id = project_id.clone();
        let tasks = tasks.clone();
        Box::new(move |task: Task| {
            // Update the tasks signal by pushing the new task to the vector
            // This will automatically trigger a re-render of the kanban board
            set_tasks.update(|tasks| {
                tasks.push(task);
            });
            
            // Save tasks to storage using proper Tauri commands
            let project_id = project_id.clone();
            let tasks = tasks.clone();
            spawn_local(async move {
                let current_tasks = tasks.get_untracked();
                // Convert tasks to JSON values for the backend
                let json_tasks: Vec<serde_json::Value> = current_tasks.iter()
                    .filter_map(|t| serde_json::to_value(t).ok())
                    .collect();
                
                let save_args = serde_json::json!({ 
                    "projectId": project_id,
                    "tasks": json_tasks 
                });
                if let Ok(js_value) = to_value(&save_args) {
                    let _ = invoke("save_tasks_data", js_value).await;
                }
            });
        }) as Box<dyn Fn(Task) + 'static>
    };

    // Task management functions are now inlined where they're used
    
    // Handler for when a task is clicked - opens the sidebar with task details
    let select_task = move |task: Task| {
        set_selected_task.set(Some(task));
    };
    
    // No need for separate close handler - TaskSidebar will use the signal directly

    view! {
        <div 
            class="kanban-page" 
            class:with-sidebar=move || selected_task.with(|task| task.is_some())
            on:click=move |_| {
                // Close any open dropdown when clicking outside
                set_open_dropdown.set(None);
            }
        >
            <div class="main-content">
                <header class="kanban-header">
                <h1>"Project: " {project_name}</h1>
                <div class="kanban-actions">
                    <button class="btn-secondary" on:click=back_to_projects>"←"</button>
                    <button class="btn-secondary" on:click=open_edit_project_modal>"✎"</button>
                    <button class="btn-primary" on:click=open_modal>"+"</button>
                </div>
            </header>
            
            <div class="kanban-board">
                {TaskStatus::all().into_iter().map(|status| {
                    // Clone status for each closure to avoid move errors
                    // Each reactive closure needs its own copy to filter by status
                    let status_for_count = status.clone();
                    let status_for_tasks = status.clone();
                    
                    view! {
                        <div class="kanban-column">
                            <div class="column-header">
                                <h3>{status.as_str()}</h3>
                                // Reactive task count - updates automatically when tasks change
                                <span class="task-count">
                                    {move || {
                                        tasks.with(|tasks| {
                                            tasks.iter()
                                                .filter(|task| task.status == status_for_count)
                                                .count()
                                        })
                                    }}
                                </span>
                            </div>
                            <div class="column-content">
                                // Reactive task list - re-renders when tasks signal changes
                                {move || {
                                    tasks.with(|tasks| {
                                        tasks.iter()
                                            .filter(|task| task.status == status_for_tasks)
                                            .cloned()
                                            .map(|task| {
                                                // Clone task for the click handler closure
                                                let task_for_click = task.clone();
                                                let task_id_for_dropdown_open = task.id.clone();
                                                let task_id_for_dropdown_show = task.id.clone();
                                                let select_task_handler = select_task.clone();
                                                
                                                view! {
                                                    <div 
                                                        class="task-card clickable"
                                                        class:dropdown-open=move || open_dropdown.get() == Some(task_id_for_dropdown_open.clone())
                                                        on:click=move |_| {
                                                            select_task_handler(task_for_click.clone());
                                                        }
                                                    >
                                                        <div class="task-content">
                                                            <h4>{task.title.clone()}</h4>
                                                            <p>{task.description.clone()}</p>
                                                        </div>
                                                        <div class="task-menu">
                                                            {
                                                                let task_id = task.id.clone();
                                                                let open_dropdown_clone = open_dropdown.clone();
                                                                let set_open_dropdown_clone = set_open_dropdown.clone();
                                                                
                                                                view! {
                                                                    // Desktop dropdown button
                                                                    <button 
                                                                        class="task-menu-btn"
                                                                        on:click=move |e| {
                                                                            e.stop_propagation();
                                                                            // Toggle dropdown for this task
                                                                            if open_dropdown_clone.get() == Some(task_id.clone()) {
                                                                                set_open_dropdown_clone.set(None);
                                                                            } else {
                                                                                set_open_dropdown_clone.set(Some(task_id.clone()));
                                                                            }
                                                                        }
                                                                    >"⋯"</button>
                                                                    
                                                                    // Mobile action buttons (hidden by default, shown on mobile via CSS)
                                                                    <div class="task-actions-mobile" style="display: none;">
                                                                        <button 
                                                                            class="task-action-btn edit-btn"
                                                                            on:click={
                                                                                let task_for_mobile_edit = task.clone();
                                                                                let set_editing_task_mobile = set_editing_task.clone();
                                                                                let edit_dialog_ref_mobile = edit_dialog_ref.clone();
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_editing_task_mobile.set(Some(task_for_mobile_edit.clone()));
                                                                                    if let Some(dialog) = edit_dialog_ref_mobile.get() {
                                                                                        let _ = dialog.show_modal();
                                                                                    }
                                                                                }
                                                                            }
                                                                        >"✎"</button>
                                                                        <button 
                                                                            class="task-action-btn cancel-btn"
                                                                            on:click={
                                                                                let task_id_mobile_cancel = task.id.clone();
                                                                                let set_tasks_mobile_cancel = set_tasks.clone();
                                                                                // Note: Auto-save for task actions will be implemented later
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_tasks_mobile_cancel.update(|tasks| {
                                                                                        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id_mobile_cancel) {
                                                                                            task.update_status(TaskStatus::Cancelled);
                                                                                        }
                                                                                    });
                                                                                    // TODO: Add auto-save here
                                                                                }
                                                                            }
                                                                        >"⚠"</button>
                                                                        <button 
                                                                            class="task-action-btn delete-btn"
                                                                            on:click={
                                                                                let task_id_mobile_delete = task.id.clone();
                                                                                let set_tasks_mobile_delete = set_tasks.clone();
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_tasks_mobile_delete.update(|tasks| {
                                                                                        tasks.retain(|t| t.id != task_id_mobile_delete);
                                                                                    });
                                                                                }
                                                                            }
                                                                        >"🗑"</button>
                                                                    </div>
                                                                }
                                                            }
                                                            
                                                            // Desktop dropdown menu
                                                            <div 
                                                                class="task-dropdown"
                                                                class:show=move || open_dropdown.get() == Some(task_id_for_dropdown_show.clone())
                                                            >
                                                                {
                                                                    view! {
                                                                        <button 
                                                                            class="dropdown-item edit-item"
                                                                            on:click={
                                                                                let task_for_dropdown_edit = task.clone();
                                                                                let set_editing_task_dropdown = set_editing_task.clone();
                                                                                let edit_dialog_ref_dropdown = edit_dialog_ref.clone();
                                                                                let set_open_dropdown_edit = set_open_dropdown.clone();
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_open_dropdown_edit.set(None); // Close dropdown
                                                                                    set_editing_task_dropdown.set(Some(task_for_dropdown_edit.clone()));
                                                                                    if let Some(dialog) = edit_dialog_ref_dropdown.get() {
                                                                                        let _ = dialog.show_modal();
                                                                                    }
                                                                                }
                                                                            }
                                                                        >"Edit"</button>
                                                                        <button 
                                                                            class="dropdown-item cancel-item"
                                                                            on:click={
                                                                                let task_id_dropdown_cancel = task.id.clone();
                                                                                let set_tasks_dropdown_cancel = set_tasks.clone();
                                                                                let set_open_dropdown_cancel = set_open_dropdown.clone();
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_open_dropdown_cancel.set(None); // Close dropdown
                                                                                    set_tasks_dropdown_cancel.update(|tasks| {
                                                                                        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id_dropdown_cancel) {
                                                                                            task.update_status(TaskStatus::Cancelled);
                                                                                        }
                                                                                    });
                                                                                }
                                                                            }
                                                                        >"Cancel"</button>
                                                                        <button 
                                                                            class="dropdown-item delete-item"
                                                                            on:click={
                                                                                let task_id_dropdown_delete = task.id.clone();
                                                                                let set_tasks_dropdown_delete = set_tasks.clone();
                                                                                let set_open_dropdown_delete = set_open_dropdown.clone();
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_open_dropdown_delete.set(None); // Close dropdown
                                                                                    set_tasks_dropdown_delete.update(|tasks| {
                                                                                        tasks.retain(|t| t.id != task_id_dropdown_delete);
                                                                                    });
                                                                                }
                                                                            }
                                                                        >"Delete"</button>
                                                                    }
                                                                }
                                                            </div>
                                                        </div>
                                                    </div>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                }}
                            </div>
                        </div>
                    }
                }).collect::<Vec<_>>()}
            </div>
            </div> {/* Close main-content */}
            
            {/* Conditional Sidebar - only shows when a task is selected */}
            {move || {
                selected_task.with(|task_opt| {
                    if let Some(task) = task_opt {
                        let sidebar_edit_callback = {
                            let set_editing_task_clone = set_editing_task.clone();
                            let edit_dialog_ref_clone = edit_dialog_ref.clone();
                            Box::new(move |task: Task| {
                                set_editing_task_clone.set(Some(task));
                                if let Some(dialog) = edit_dialog_ref_clone.get() {
                                    let _ = dialog.show_modal();
                                }
                            }) as Box<dyn Fn(Task) + 'static>
                        };

                        let sidebar_status_callback = {
                            let set_tasks_for_status = set_tasks.clone();
                            Box::new(move |task_id: String, status: TaskStatus| {
                                set_tasks_for_status.update(|tasks| {
                                    if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                                        task.update_status(status);
                                    }
                                });
                            }) as Box<dyn Fn(String, TaskStatus) + 'static>
                        };

                        let sidebar_delete_callback = {
                            let set_tasks_for_delete = set_tasks.clone();
                            Box::new(move |task_id: String| {
                                set_tasks_for_delete.update(|tasks| {
                                    tasks.retain(|t| t.id != task_id);
                                });
                            }) as Box<dyn Fn(String) + 'static>
                        };

                        view! {
                            <TaskSidebar 
                                task=task.clone()
                                selected_task=set_selected_task
                                on_edit=sidebar_edit_callback
                                on_update_status=sidebar_status_callback
                                on_delete=sidebar_delete_callback
                            />
                        }.into_any()
                    } else {
                        view! {}.into_any()
                    }
                })
            }}
            
            <TaskModal 
                project_id=project_id.clone()
                on_create=create_task
                dialog_ref=dialog_ref
            />
            
            {/* Edit Task Modal - always rendered but only shown when editing_task is Some */}
            {move || {
                if let Some(task) = editing_task.get() {
                    let edit_callback = {
                        let set_tasks_for_edit = set_tasks.clone();
                        Box::new(move |task_id: String, new_title: String, new_description: String| {
                            set_tasks_for_edit.update(|tasks| {
                                if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
                                    task.update_title(new_title);
                                    task.update_description(new_description);
                                }
                            });
                        }) as Box<dyn Fn(String, String, String) + 'static>
                    };
                    
                    view! {
                        <EditTaskModal 
                            task=task
                            on_edit=edit_callback
                            dialog_ref=edit_dialog_ref
                        />
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
            
            <EditProjectModal
                project_id=project_id
                on_update=update_project
                dialog_ref=edit_project_dialog_ref
            />
        </div>
    }
}