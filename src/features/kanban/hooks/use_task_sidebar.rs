use leptos::prelude::*;
use leptos::html::Dialog;
use std::sync::Arc;
use crate::core::models::{Task, TaskStatus, AgentProfile};
use crate::core::services::{open_worktree_location_async, open_worktree_in_ide_async};
use crate::components::TaskSidebar;
use crate::features::kanban::services::{delete_task, update_task_details, update_task_profile, update_task_status};

// Hook for managing task sidebar state and callbacks
pub fn use_task_sidebar(
    project_id: String,
    tasks: RwSignal<Vec<Task>>,
    selected_task: ReadSignal<Option<String>>,
    set_selected_task: WriteSignal<Option<String>>,
    edit_dialog_ref: NodeRef<Dialog>,
    set_editing_task: WriteSignal<Option<Task>>,
) -> impl IntoView {
    move || {
        if let Some(task_id) = selected_task.get() {
            let maybe_task = tasks.with(|ts| ts.iter().find(|t| t.id == task_id).cloned());
            if let Some(task) = maybe_task {
                create_task_sidebar(
                    task,
                    project_id.clone(),
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

// Create sidebar component with all necessary callbacks
fn create_task_sidebar(
    task: Task,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
    selected_task_signal: WriteSignal<Option<String>>,
    edit_dialog_ref: NodeRef<Dialog>,
    set_editing_task: WriteSignal<Option<Task>>,
) -> impl IntoView {
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

    let sidebar_status_callback: Arc<dyn Fn(String, TaskStatus) + Send + Sync> = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Arc::new(move |task_id: String, status: TaskStatus| {
            update_task_status(task_id, status, project_id_clone.clone(), tasks_signal_clone);
        })
    };

    let sidebar_delete_callback = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Box::new(move |task_id: String| {
            delete_task(task_id, project_id_clone.clone(), tasks_signal_clone);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_worktree_callback = {
        Box::new(move |worktree_path: String| {
            open_worktree_location_async(worktree_path);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_ide_callback = {
        Box::new(move |worktree_path: String| {
            open_worktree_in_ide_async(worktree_path);
        }) as Box<dyn Fn(String) + 'static>
    };

    let sidebar_profile_callback = {
        let project_id_clone = project_id.clone();
        let tasks_signal_clone = tasks_signal.clone();
        Box::new(move |task_id: String, profile: AgentProfile| {
            update_task_profile(task_id, profile, project_id_clone.clone(), tasks_signal_clone);
        }) as Box<dyn Fn(String, AgentProfile) + 'static>
    };

    view! {
        <TaskSidebar
            task=task.clone()
            selected_task=selected_task_signal
            on_edit=sidebar_edit_callback
            on_update_status=sidebar_status_callback
            on_delete=sidebar_delete_callback
            on_open_worktree=Some(sidebar_worktree_callback)
            on_open_ide=Some(sidebar_ide_callback)
            on_update_profile=sidebar_profile_callback
        />
    }
}

// Create edit task callback for the sidebar
pub fn create_edit_task_callback(
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) -> Box<dyn Fn(String, String, String) + 'static> {
    Box::new(move |task_id: String, new_title: String, new_description: String| {
        update_task_details(task_id, new_title, new_description, project_id.clone(), tasks_signal);
    })
}