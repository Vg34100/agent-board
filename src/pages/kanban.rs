use leptos::prelude::*;
use leptos::html::Dialog;
use crate::app::AppView;
use crate::models::{Task, TaskStatus};
use crate::components::{TaskModal, TaskSidebar, EditTaskModal};

#[component]
pub fn Kanban(project_id: String) -> impl IntoView {
    // Get the navigation signal from context - this allows us to change the current view
    // The expect() will panic if the context wasn't provided, which helps catch setup errors
    let navigate = use_context::<WriteSignal<AppView>>().expect("navigate context");
    
    // Sample tasks for demonstration purposes
    // In a real app, these would be loaded from localStorage or a database
    let sample_tasks = vec![
        Task {
            id: "1".to_string(),
            project_id: project_id.clone(),
            title: "Setup project structure".to_string(),
            description: "Create basic folder structure and files".to_string(),
            status: TaskStatus::Done,
            created_at: chrono::Utc::now(),
        },
        Task {
            id: "2".to_string(),
            project_id: project_id.clone(),
            title: "Implement user authentication".to_string(),
            description: "Add login and registration functionality".to_string(),
            status: TaskStatus::InProgress,
            created_at: chrono::Utc::now(),
        },
        Task {
            id: "3".to_string(),
            project_id: project_id.clone(),
            title: "Design database schema".to_string(),
            description: "Plan the database structure for the application".to_string(),
            status: TaskStatus::ToDo,
            created_at: chrono::Utc::now(),
        },
        Task {
            id: "4".to_string(),
            project_id: project_id.clone(),
            title: "Write API documentation".to_string(),
            description: "Document all API endpoints and usage".to_string(),
            status: TaskStatus::InReview,
            created_at: chrono::Utc::now(),
        },
    ];
    
    // Create a reactive signal to hold the tasks list
    // Signal automatically triggers re-renders when the data changes
    let (tasks, set_tasks) = signal(sample_tasks);
    
    // Create a signal to track which task is currently selected for the sidebar
    // None means no sidebar is open, Some(task) means sidebar is showing that task
    let (selected_task, set_selected_task) = signal::<Option<Task>>(None);
    
    // Track which dropdown is currently open (task ID)
    let (open_dropdown, set_open_dropdown) = signal::<Option<String>>(None);
    
    // Create references to the HTML dialog elements that we can use to control them
    // from Rust code (open/close modals programmatically)
    let dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let edit_dialog_ref: NodeRef<Dialog> = NodeRef::new();
    
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
    
    // Callback function that gets called when TaskModal creates a new task
    // This function takes ownership of the Task and adds it to the kanban board
    let create_task = Box::new(move |task: Task| {
        // Update the tasks signal by pushing the new task to the vector
        // This will automatically trigger a re-render of the kanban board
        set_tasks.update(|tasks| {
            tasks.push(task);
        });
    }) as Box<dyn Fn(Task) + 'static>;

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
                <h1>"Project: " {project_id.clone()}</h1>
                <div class="kanban-actions">
                    <button class="btn-secondary" on:click=back_to_projects>"←"</button>
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
                                                                                move |e| {
                                                                                    e.stop_propagation();
                                                                                    set_tasks_mobile_cancel.update(|tasks| {
                                                                                        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id_mobile_cancel) {
                                                                                            task.update_status(TaskStatus::Cancelled);
                                                                                        }
                                                                                    });
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
        </div>
    }
}