use leptos::prelude::*;
use crate::core::models::{Task, TaskStatus};

#[component]
pub fn SidebarHeader(
    #[prop(into)] task: Task,
    #[prop(into)] on_edit: Box<dyn Fn(Task) + 'static>,
    #[prop(into)] on_update_status: Box<dyn Fn(String, TaskStatus) + 'static>,
    #[prop(into)] on_delete: Box<dyn Fn(String) + 'static>,
    #[prop(into)] on_close: Box<dyn Fn() + 'static>,
) -> impl IntoView {
    let task_title = task.title.clone();
    let task_id = task.id.clone();

    view! {
        <div class="sidebar-header">
            <h2>{task_title}</h2>
            <div class="header-actions">
                <button
                    class="action-btn edit-btn"
                    title="Edit Task"
                    on:click={
                        let task_for_edit = task.clone();
                        move |_| {
                            on_edit(task_for_edit.clone());
                        }
                    }
                >"✎"</button>
                <button
                    class="action-btn cancel-btn"
                    title="Move to Cancelled"
                    on:click={
                        let task_id_for_cancel = task_id.clone();
                        move |_| {
                            on_update_status(task_id_for_cancel.clone(), TaskStatus::Cancelled);
                        }
                    }
                >"⚠"</button>
                <button
                    class="action-btn delete-btn"
                    title="Delete Task"
                    on:click={
                        let task_id_for_delete = task_id.clone();
                        move |_| {
                            on_delete(task_id_for_delete.clone());
                        }
                    }
                >"🗑"</button>
                <button class="sidebar-close" on:click=move |_| on_close()>"×"</button>
            </div>
        </div>
    }
}