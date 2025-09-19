use leptos::prelude::*;
use leptos::html::Dialog;
use leptos::task::spawn_local;
use crate::app::AppView;
use crate::models::{Task, TaskStatus};
use crate::components::{TaskModal, EditTaskModal, EditProjectModal, SettingsModal, TaskCard, TaskMenu};
use crate::components::kanban::{
    KanbanHeader, KanbanBoard, KanbanColumn,
    storage_manager::load_tasks,
    task_operations::{create_task_handler, delete_task, cancel_task},
    sidebar_manager::{create_task_sidebar, create_edit_task_callback},
    project_actions::{load_project_data, create_project_update_callback}
};
use std::rc::Rc;
use std::sync::Arc;

#[component]
pub fn Kanban(project_id: String) -> impl IntoView {
    // Clone project_id for different uses
    let project_id_for_tasks = project_id.clone();
    let project_id_for_sidebar = project_id.clone();
    let project_id_for_mobile = project_id.clone();
    let project_id_for_dropdown = project_id.clone();

    // Get the navigation signal from context
    let navigate = use_context::<WriteSignal<AppView>>().expect("navigate context");

    // Project signals
    let (project_name, set_project_name) = signal(String::from("Loading..."));
    let (project_path, set_project_path) = signal::<Option<String>>(None);

    // Load project data
    load_project_data(project_id.clone(), set_project_name, set_project_path);

    // Task signals
    let tasks = RwSignal::new(Vec::<Task>::new());

    // Load project-specific tasks from storage
    {
        let project_id_clone = project_id_for_tasks.clone();
        spawn_local(async move {
            match load_tasks(&project_id_clone).await {
                Ok(loaded_tasks) => {
                    tasks.set(loaded_tasks);
                }
                Err(e) => {
                    web_sys::console::error_1(&format!("Failed to load tasks: {}", e).into());
                    tasks.set(Vec::new());
                }
            }
        });
    }

    // UI state signals
    let (selected_task, set_selected_task) = signal::<Option<String>>(None);
    let (open_dropdown, set_open_dropdown) = signal::<Option<String>>(None);
    let (editing_task, set_editing_task) = signal::<Option<Task>>(None);

    // Dialog references
    let dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let edit_dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let edit_project_dialog_ref: NodeRef<Dialog> = NodeRef::new();
    let settings_dialog_ref: NodeRef<Dialog> = NodeRef::new();

    // Navigation and modal handlers
    let back_to_projects = move || {
        navigate.set(AppView::Projects);
    };

    let open_modal = move || {
        if let Some(dialog) = dialog_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    let open_settings_modal = move || {
        if let Some(dialog) = settings_dialog_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    let open_edit_project_modal = move || {
        if let Some(dialog) = edit_project_dialog_ref.get() {
            let _ = dialog.show_modal();
        }
    };

    // Create task handler
    let create_task = create_task_handler(project_id.clone(), tasks);

    // Project update handler
    let update_project = Callback::new(create_project_update_callback(set_project_name));

    // Task selection handler
    let select_task = move |task: Task| {
        set_selected_task.set(Some(task.id.clone()));
    };

    view! {
        <div
            class="kanban-page"
            class:with-sidebar=move || selected_task.with(|task| task.is_some())
            on:click=move |_| {
                set_open_dropdown.set(None);
            }
        >
            <div class="main-content">
                <KanbanHeader
                    project_id=project_id.clone()
                    project_name=project_name
                    project_path=project_path
                    on_back=Rc::new(back_to_projects)
                    on_open_modal=Rc::new(open_modal)
                    on_open_settings=Rc::new(open_settings_modal)
                    on_open_edit=Rc::new(open_edit_project_modal)
                />

                <KanbanBoard>
                    {TaskStatus::all().into_iter().map(|status| {
                        let status_for_tasks = status.clone();
                        let project_id_mobile = project_id_for_mobile.clone();
                        let project_id_dropdown = project_id_for_dropdown.clone();

                        view! {
                            <KanbanColumn
                                status=status.clone()
                                tasks=tasks.read_only()
                            >
                                {move || {
                                    let _project_id_mobile_rc = Arc::new(project_id_mobile.clone());
                                    let project_id_dropdown_rc = Arc::new(project_id_dropdown.clone());

                                    tasks.with(|task_list| {
                                        task_list.iter()
                                            .filter(|task| task.status == status_for_tasks)
                                            .cloned()
                                            .map(|task| {
                                                let task_for_click = task.clone();
                                                let task_id_for_dropdown_open = task.id.clone();
                                                let select_task_handler = select_task.clone();
                                                let project_id_dropdown_rc = project_id_dropdown_rc.clone();

                                                view! {
                                                    <TaskCard
                                                        dropdown_open=Signal::derive({
                                                            let id = task_id_for_dropdown_open.clone();
                                                            let open_dropdown = open_dropdown.clone();
                                                            move || open_dropdown.get() == Some(id.clone())
                                                        })
                                                        on_click=Box::new({
                                                            let select_task_handler = select_task_handler.clone();
                                                            let t = task_for_click.clone();
                                                            move || select_task_handler(t.clone())
                                                        })
                                                    >
                                                        <div class="task-content">
                                                            <h4>{task.title.clone()}</h4>
                                                            <p class="task-description">{task.description.clone()}</p>
                                                        </div>
                                                        <div class="task-menu">
                                                            <TaskMenu
                                                                task=task.clone()
                                                                open_dropdown=open_dropdown
                                                                set_open_dropdown=set_open_dropdown
                                                                on_edit={
                                                                    let set_editing_task = set_editing_task.clone();
                                                                    let edit_dialog_ref = edit_dialog_ref.clone();
                                                                    Rc::new(move |task: Task| {
                                                                        set_editing_task.set(Some(task));
                                                                        if let Some(dialog) = edit_dialog_ref.get() {
                                                                            let _ = dialog.show_modal();
                                                                        }
                                                                    })
                                                                }
                                                                on_cancel={
                                                                    let project_id = project_id_dropdown_rc.as_ref().clone();
                                                                    Rc::new(move |task_id: String| {
                                                                        cancel_task(task_id, project_id.clone(), tasks);
                                                                    })
                                                                }
                                                                on_delete={
                                                                    let project_id = project_id_dropdown_rc.as_ref().clone();
                                                                    Rc::new(move |task_id: String| {
                                                                        delete_task(task_id, project_id.clone(), tasks);
                                                                    })
                                                                }
                                                            />
                                                        </div>
                                                    </TaskCard>
                                                }
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                }}
                            </KanbanColumn>
                        }
                    }).collect::<Vec<_>>()}
                </KanbanBoard>
            </div>

            // Conditional Sidebar
            {
                let project_id_for_sidebar_use = project_id_for_sidebar.clone();
                move || {
                    if let Some(task_id) = selected_task.get() {
                        let maybe_task = tasks.with(|ts| ts.iter().find(|t| t.id == task_id).cloned());
                        if let Some(task) = maybe_task {
                            create_task_sidebar(
                                task,
                                project_id_for_sidebar_use.clone(),
                                tasks,
                                set_selected_task,
                                edit_dialog_ref,
                                set_editing_task,
                            ).into_any()
                        } else {
                            view! {}.into_any()
                        }
                    } else {
                        view! {}.into_any()
                    }
                }
            }

            <TaskModal
                project_id=project_id.clone()
                on_create=create_task
                dialog_ref=dialog_ref
            />

            {move || {
                if let Some(task) = editing_task.get() {
                    let edit_callback = create_edit_task_callback(project_id.clone(), tasks);
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
                project_id=project_id_for_sidebar
                on_update=update_project
                dialog_ref=edit_project_dialog_ref
            />

            <SettingsModal dialog_ref=settings_dialog_ref />
        </div>
    }
}
