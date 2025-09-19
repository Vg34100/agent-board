use leptos::prelude::*;
use crate::models::{Task};
use std::rc::Rc;

#[component]
pub fn TaskMenu(
    #[prop(into)] task: Task,
    #[prop(into)] open_dropdown: ReadSignal<Option<String>>,
    #[prop(into)] set_open_dropdown: WriteSignal<Option<String>>,
    on_edit: Rc<dyn Fn(Task) + 'static>,
    on_cancel: Rc<dyn Fn(String) + 'static>,
    on_delete: Rc<dyn Fn(String) + 'static>,
) -> impl IntoView {
    let toggle_dropdown = {
        let task_id = task.id.clone();
        let open_dropdown = open_dropdown.clone();
        let set_open_dropdown = set_open_dropdown.clone();
        move |e: leptos::ev::MouseEvent| {
            e.stop_propagation();
            if open_dropdown.get() == Some(task_id.clone()) {
                set_open_dropdown.set(None);
            } else {
                set_open_dropdown.set(Some(task_id.clone()));
            }
        }
    };

    view! {
        <>
            <button class="task-menu-btn" on:click=toggle_dropdown>"⋯"</button>

            <div class="task-actions-mobile" style="display: none;">
                <button class="task-action-btn edit-btn" on:click={
                    let on_edit = on_edit.clone();
                    let task = task.clone();
                    move |e| { e.stop_propagation(); on_edit(task.clone()); }
                }>"✎"</button>
                <button class="task-action-btn cancel-btn" on:click={
                    let on_cancel = on_cancel.clone();
                    let id = task.id.clone();
                    move |e| { e.stop_propagation(); on_cancel(id.clone()); }
                }>"⚠"</button>
                <button class="task-action-btn delete-btn" on:click={
                    let on_delete = on_delete.clone();
                    let id = task.id.clone();
                    let set_open_dropdown = set_open_dropdown.clone();
                    move |e| { e.stop_propagation(); set_open_dropdown.set(None); on_delete(id.clone()); }
                }>"🞮"</button>
            </div>

            <div class="task-dropdown" class:show=move || open_dropdown.get() == Some(task.id.clone())>
                <button class="dropdown-item edit-item" on:click={
                    let on_edit = on_edit.clone();
                    let set_open_dropdown = set_open_dropdown.clone();
                    let task = task.clone();
                    move |e| { e.stop_propagation(); set_open_dropdown.set(None); on_edit(task.clone()); }
                }>"Edit"</button>
                <button class="dropdown-item cancel-item" on:click={
                    let on_cancel = on_cancel.clone();
                    let set_open_dropdown = set_open_dropdown.clone();
                    let id = task.id.clone();
                    move |e| { e.stop_propagation(); set_open_dropdown.set(None); on_cancel(id.clone()); }
                }>"Cancel"</button>
                <button class="dropdown-item delete-item" on:click={
                    let on_delete = on_delete.clone();
                    let set_open_dropdown = set_open_dropdown.clone();
                    let id = task.id.clone();
                    move |e| { e.stop_propagation(); set_open_dropdown.set(None); on_delete(id.clone()); }
                }>"Delete"</button>
            </div>
        </>
    }
}
