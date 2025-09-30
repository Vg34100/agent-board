use leptos::prelude::*;
use crate::core::models::{Task, TaskStatus, AgentProfile};

#[component]
pub fn TaskDetailsSection(
    #[prop(into)] task: Task,
    #[prop(into)] selected_profile: ReadSignal<AgentProfile>,
    #[prop(into)] set_selected_profile: WriteSignal<AgentProfile>,
    #[prop(into)] on_update_status: Box<dyn Fn(String, TaskStatus) + 'static>,
    #[prop(into)] on_update_profile: Box<dyn Fn(String, AgentProfile) + 'static>,
) -> impl IntoView {
    let task_description = task.description.clone();
    let task_status = task.status.clone();
    let task_id = task.id.clone();

    // State for showing/hiding full description
    let (show_full_description, set_show_full_description) = signal(false);

    // Determine if description is long (more than 5 lines approximately)
    let description_is_long = task_description.len() > 200;

    // Get display description based on show_full state
    let get_display_description = move || {
        if description_is_long && !show_full_description.get() {
            let truncated = task_description.chars().take(200).collect::<String>();
            format!("{}...", truncated)
        } else {
            task_description.clone()
        }
    };

    view! {
        <div class="task-details">
            <div class="detail-section">
                <h3>"Description"</h3>
                <div class="task-description">
                    <p>{get_display_description}</p>
                    {description_is_long.then(|| view! {
                        <button
                            class="show-more-btn"
                            on:click=move |_| set_show_full_description.update(|show| *show = !*show)
                        >
                            {move || if show_full_description.get() { "Show Less" } else { "Show More" }}
                        </button>
                    })}
                </div>
            </div>

            {/* Status-Dependent Section */}
            <div class="status-section">
                {match task_status {
                    TaskStatus::ToDo => view! {
                        <div class="create-attempt">
                            <h3>"Create Attempt"</h3>
                            <div class="attempt-config">
                                <div class="config-row">
                                    <label>"Base Branch:"</label>
                                    <select>
                                        <option value="main">"main"</option>
                                        <option value="develop">"develop"</option>
                                    </select>
                                </div>
                                <div class="config-row">
                                    <label>"Profile:"</label>
                                    <select on:change=move |ev| {
                                        let value = event_target_value(&ev);
                                        let profile = match value.as_str() {
                                            "codex" => AgentProfile::Codex,
                                            _ => AgentProfile::ClaudeCode,
                                        };
                                        set_selected_profile.set(profile);
                                    }>
                                        <option value="claude" selected=matches!(selected_profile.get(), AgentProfile::ClaudeCode)>"Claude Code"</option>
                                        <option value="codex" selected=matches!(selected_profile.get(), AgentProfile::Codex)>"Codex"</option>
                                    </select>
                                </div>
                                <div class="config-row">
                                    <label>"Variant:"</label>
                                    <select>
                                        <option value="standard">"Standard"</option>
                                        <option value="focused">"Focused"</option>
                                    </select>
                                </div>
                                <button
                                    class="start-btn"
                                    on:click={
                                        let task_id_for_start = task_id.clone();
                                        let profile = selected_profile.clone();
                                        move |_| {
                                            on_update_profile(task_id_for_start.clone(), profile.get());
                                            on_update_status(task_id_for_start.clone(), TaskStatus::InProgress);
                                        }
                                    }
                                >"Start"</button>
                            </div>
                        </div>
                    }.into_any(),
                    _ => view! {
                        <div class="attempt-status">
                            <h3>"Attempt Status"</h3>
                            <div class="status-display">
                                <span class=format!("status-badge status-{}", task_status.as_str().to_lowercase().replace(" ", "-"))>
                                    {task_status.as_str()}
                                </span>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}