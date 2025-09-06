use leptos::prelude::*;
use leptos::{ev, html::Dialog};
use crate::models::Task;

#[component]
pub fn EditTaskModal(
    #[prop(into)] task: Task,
    #[prop(into)] on_edit: Box<dyn Fn(String, String, String) + 'static>, // task_id, title, description
    dialog_ref: NodeRef<Dialog>,
) -> impl IntoView {
    let (title, set_title) = signal(task.title.clone());
    let (description, set_description) = signal(task.description.clone());

    let task_id = task.id.clone();
    
    let handle_submit = move |ev: ev::SubmitEvent| {
        ev.prevent_default();
        
        // Call the parent's callback function with the updated task data
        on_edit(task_id.clone(), title.get_untracked(), description.get_untracked());
        
        // Close the HTML dialog element by calling its close() method
        if let Some(dialog) = dialog_ref.get() {
            dialog.close();
        }
    };

    // Handler for closing the modal without submitting (cancel button or close X)
    let close_modal_x = {
        let dialog_ref_clone = dialog_ref.clone();
        let task_title = task.title.clone();
        let task_description = task.description.clone();
        let set_title_clone = set_title.clone();
        let set_description_clone = set_description.clone();
        move |_| {
            if let Some(dialog) = dialog_ref_clone.get() {
                dialog.close();
            }
            // Reset form fields to original values when canceling
            set_title_clone.set(task_title.clone());
            set_description_clone.set(task_description.clone());
        }
    };

    let close_modal_cancel = {
        let dialog_ref_clone = dialog_ref.clone();
        let task_title = task.title.clone();
        let task_description = task.description.clone();
        let set_title_clone = set_title.clone();
        let set_description_clone = set_description.clone();
        move |_| {
            if let Some(dialog) = dialog_ref_clone.get() {
                dialog.close();
            }
            // Reset form fields to original values when canceling
            set_title_clone.set(task_title.clone());
            set_description_clone.set(task_description.clone());
        }
    };

    view! {
        <dialog node_ref=dialog_ref class="task-modal">
            <div class="modal-content">
                <div class="modal-header">
                    <h3>"EDIT TASK"</h3>
                    <button type="button" class="modal-close" on:click=close_modal_x>"×"</button>
                </div>
                <form on:submit=handle_submit>
                    <div class="form-group">
                        <label>"TITLE"</label>
                        <input 
                            type="text" 
                            placeholder="Task title..."
                            on:input=move |ev| set_title.set(event_target_value(&ev))
                            prop:value=move || title.get()
                            required
                        />
                    </div>
                    <div class="form-group">
                        <label>"DESCRIPTION"</label>
                        <textarea 
                            placeholder="Task description..."
                            rows="4"
                            on:input=move |ev| set_description.set(event_target_value(&ev))
                            prop:value=move || description.get()
                        ></textarea>
                    </div>
                    <div class="modal-actions">
                        <button type="button" class="btn-secondary" on:click=close_modal_cancel>"CANCEL"</button>
                        <button type="submit" class="btn-primary">"SAVE CHANGES"</button>
                    </div>
                </form>
            </div>
        </dialog>
    }
}