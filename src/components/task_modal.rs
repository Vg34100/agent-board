use leptos::prelude::*;
use leptos::{ev, html::Dialog};
use crate::models::{Task, TaskStatus};
use uuid::Uuid;
use chrono::Utc;

#[component]
pub fn TaskModal(
    #[prop(into)] project_id: String,
    #[prop(into)] on_create: Box<dyn Fn(Task) + 'static>,
    dialog_ref: NodeRef<Dialog>,
) -> impl IntoView {
    let (title, set_title) = signal(String::new());
    let (description, set_description) = signal(String::new());

    // Clone the callback and project_id so they can be moved into the closure
    // The on_create callback is Box<dyn Fn(Task)> which doesn't implement Clone,
    // so we need to work around this by capturing it in the closure scope
    let project_id_clone = project_id.clone();
    
    let handle_submit = move |ev: ev::SubmitEvent| {
        // Prevent the default form submission behavior (page reload)
        ev.prevent_default();
        
        // Create a new Task struct with the form data and default values
        let task = Task {
            id: Uuid::new_v4().to_string(),           // Generate unique ID
            project_id: project_id_clone.clone(),     // Use the project this task belongs to
            title: title.get_untracked(),             // Get title without creating reactive dependency
            description: description.get_untracked(), // Get description without reactive dependency
            status: TaskStatus::ToDo,                 // New tasks always start in ToDo column
            created_at: Utc::now(),                   // Timestamp for when task was created
        };
        
        // Call the parent's callback function to add the task to the kanban board
        on_create(task);
        
        // Reset form fields to empty state after successful submission
        set_title.set(String::new());
        set_description.set(String::new());
        
        // Close the HTML dialog element by calling its close() method
        if let Some(dialog) = dialog_ref.get() {
            dialog.close();
        }
    };

    // Handler for closing the modal without submitting (cancel button or close X)
    let close_modal = move |_| {
        // Access the dialog DOM element and close it
        if let Some(dialog) = dialog_ref.get() {
            dialog.close();
        }
    };

    view! {
        <dialog node_ref=dialog_ref class="task-modal">
            <div class="modal-content">
                <div class="modal-header">
                    <h3>"CREATE TASK"</h3>
                    <button type="button" class="modal-close" on:click=close_modal>"×"</button>
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
                        <button type="button" class="btn-secondary" on:click=close_modal>"CANCEL"</button>
                        <button type="submit" class="btn-primary">"CREATE"</button>
                    </div>
                </form>
            </div>
        </dialog>
    }
}