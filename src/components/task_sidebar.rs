use leptos::prelude::*;
use crate::models::{Task, TaskStatus};

#[component]
pub fn TaskSidebar(
    #[prop(into)] task: Task,
    #[prop(into)] selected_task: WriteSignal<Option<Task>>,
    #[prop(into)] on_edit: Box<dyn Fn(Task) + 'static>, // Callback to trigger edit modal
    #[prop(into)] on_update_status: Box<dyn Fn(String, TaskStatus) + 'static>,
    #[prop(into)] on_delete: Box<dyn Fn(String) + 'static>,
) -> impl IntoView {
    // State for showing/hiding full description
    let (show_full_description, set_show_full_description) = signal(false);
    
    // Clone task data for use in closures
    let task_title = task.title.clone();
    let task_description = task.description.clone();
    let task_status = task.status.clone();
    let task_id = task.id.clone();
    
    // Determine if description is long (more than 5 lines approximately)
    let description_is_long = task_description.len() > 200; // Rough estimate
    
    // Get display description based on show_full state
    let get_display_description = move || {
        if description_is_long && !show_full_description.get() {
            format!("{}...", &task_description.chars().take(200).collect::<String>())
        } else {
            task_description.clone()
        }
    };

    let close_sidebar = move |_| {
        selected_task.set(None);
    };

    view! {
        <div class="task-sidebar">
            {/* Sidebar Header */}
            <div class="sidebar-header">
                <h2>{task_title.clone()}</h2>
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
                                // Close sidebar when task is deleted
                                selected_task.set(None);
                            }
                        }
                    >"🗑"</button>
                    <button class="sidebar-close" on:click=close_sidebar>"×"</button>
                </div>
            </div>
            
            {/* Task Details Section */}
            <div class="sidebar-content">
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
                                            <select>
                                                <option value="default">"Default"</option>
                                                <option value="expert">"Expert"</option>
                                            </select>
                                        </div>
                                        <div class="config-row">
                                            <label>"Variant:"</label>
                                            <select>
                                                <option value="standard">"Standard"</option>
                                                <option value="focused">"Focused"</option>
                                            </select>
                                        </div>
                                        <button class="start-btn">"Start"</button>
                                    </div>
                                </div>
                            }.into_any(),
                            _ => view! {
                                <div class="attempt-status">
                                    <h3>"Attempt 1/1"</h3>
                                    <div class="status-info">
                                        <span class="profile-info">"Profile: default"</span>
                                        <span class="branch-info">"Branch: feature/task-123"</span>
                                        <span class="diff-info">"Diffs: +0 -0"</span>
                                        <div class="action-menu">
                                            <button class="menu-dots">"⋯"</button>
                                            <div class="dropdown-menu">
                                                <button>"Open in IDE"</button>
                                                <button>"Start Dev Server"</button>
                                                <button>"Rebase"</button>
                                                <button>"Create PR"</button>
                                                <button>"Merge"</button>
                                            </div>
                                        </div>
                                    </div>
                                </div>
                            }.into_any()
                        }}
                    </div>
                </div>
                
                {/* Agent Chat Window - Only show for non-TODO statuses */}
                {match task_status {
                    TaskStatus::ToDo => view! {}.into_any(),
                    _ => view! {
                        <div class="agent-window">
                            <h3>"Coding Agents"</h3>
                    <div class="agent-sessions">
                        {/* Sample agent sessions - these would be dynamic in real implementation */}
                        <div class="agent-session">
                            <div class="session-header">
                                <span class="session-title">"<> Coding Agent"</span>
                                <span class="session-time">"10:45:29 PM"</span>
                                <button class="restore-btn">"Restore"</button>
                                <span class="session-status completed">"Completed"</span>
                                <button class="expand-btn">"▼"</button>
                            </div>
                            <div class="session-content hidden">
                                <div class="message">
                                    <div class="message-header">
                                        <span class="sender">"Agent"</span>
                                        <span class="time">"10:45:30 PM"</span>
                                    </div>
                                    <div class="message-content">
                                        "I'll help you implement this feature. Let me start by examining the current code structure..."
                                    </div>
                                </div>
                                <div class="message">
                                    <div class="message-header">
                                        <span class="sender">"User"</span>
                                        <span class="time">"10:46:15 PM"</span>
                                    </div>
                                    <div class="message-content">
                                        "Please make sure to follow the existing patterns in the codebase."
                                    </div>
                                </div>
                            </div>
                        </div>
                        
                        <div class="agent-session">
                            <div class="session-header">
                                <span class="session-title">"<> Coding Agent"</span>
                                <span class="session-time">"11:22:15 PM"</span>
                                <button class="restore-btn">"Restore"</button>
                                <span class="session-status completed">"Completed"</span>
                                <button class="expand-btn">"▼"</button>
                            </div>
                            <div class="session-content hidden">
                                <div class="message">
                                    <div class="message-header">
                                        <span class="sender">"Agent"</span>
                                        <span class="time">"11:22:16 PM"</span>
                                    </div>
                                    <div class="message-content">
                                        "Task completed successfully. All tests are passing."
                                    </div>
                                </div>
                            </div>
                        </div>
                        
                        {/* Most recent session - automatically expanded */}
                        <div class="agent-session active">
                            <div class="session-header">
                                <span class="session-title">"<> Coding Agent"</span>
                                <span class="session-time">"12:05:42 PM"</span>
                                <button class="restore-btn">"Restore"</button>
                                <span class="session-status in-progress">"In Progress"</span>
                                <button class="expand-btn">"▲"</button>
                            </div>
                            <div class="session-content">
                                <div class="message">
                                    <div class="message-header">
                                        <span class="sender">"Agent"</span>
                                        <span class="time">"12:05:43 PM"</span>
                                    </div>
                                    <div class="message-content">
                                        "Starting work on the sidebar component implementation..."
                                    </div>
                                </div>
                                <div class="message">
                                    <div class="message-header">
                                        <span class="sender">"Agent"</span>
                                        <span class="time">"12:06:12 PM"</span>
                                    </div>
                                    <div class="message-content">
                                        "I've created the basic component structure. Now implementing the status-dependent sections."
                                    </div>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
                
                {/* Chat Input */}
                <div class="chat-input-section">
                    <div class="input-container">
                        <button class="profile-btn">"Profile"</button>
                        <input 
                            type="text" 
                            placeholder="Send a message..."
                            class="message-input"
                        />
                        <button class="send-btn">"Send"</button>
                    </div>
                </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}