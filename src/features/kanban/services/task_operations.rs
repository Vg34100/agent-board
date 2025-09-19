use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::core::models::{Task, TaskStatus, AgentProfile};
use crate::core::services::{save_tasks_async, create_worktree_for_task, remove_worktree_for_task, start_agent_for_task};

// Create a new task and save it
pub fn create_task_handler(
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) -> Box<dyn Fn(Task) + 'static> {
    Box::new(move |task: Task| {
        // Update the tasks signal by pushing the new task to the vector
        tasks_signal.update(|tasks| {
            tasks.push(task);
        });

        // Save tasks to storage
        let project_id = project_id.clone();
        let current_tasks = tasks_signal.get_untracked();
        save_tasks_async(project_id, current_tasks);
    })
}

// Update task status and handle worktree operations
pub fn update_task_status(
    task_id: String,
    new_status: TaskStatus,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            let old_status = task.status.clone();
            task.update_status(new_status.clone());

            // If task is moving to InProgress, create a worktree only if missing
            if new_status == TaskStatus::InProgress && old_status != TaskStatus::InProgress {
                if task.worktree_path.is_some() {
                    web_sys::console::log_1(&format!("Reusing existing worktree for task {}", task.id).into());
                } else {
                    let task_id_clone = task_id.clone();
                    let project_id_clone = project_id.clone();
                    let tasks_signal_clone = tasks_signal.clone();

                    spawn_local(async move {
                        // Create worktree
                        match create_worktree_for_task(&project_id_clone, &task_id_clone).await {
                            Ok(worktree_path) => {
                                // Start agent process
                                let task_for_agent = {
                                    let tasks = tasks_signal_clone.get_untracked();
                                    tasks.iter().find(|t| t.id == task_id_clone).cloned()
                                };

                                if let Some(task) = task_for_agent {
                                    if let Err(e) = start_agent_for_task(&task, &worktree_path).await {
                                        web_sys::console::error_1(&format!("Failed to start agent: {}", e).into());
                                    }
                                }

                                // Update task with worktree path and save
                                update_task_worktree_path(task_id_clone.clone(), Some(worktree_path), project_id_clone, tasks_signal_clone);
                            }
                            Err(e) => {
                                web_sys::console::error_1(&format!("Failed to create worktree: {}", e).into());
                            }
                        }
                    });
                }
            }

            // If task is moving away from InProgress (to Done/Cancelled), remove worktree
            if (new_status == TaskStatus::Done || new_status == TaskStatus::Cancelled) && old_status == TaskStatus::InProgress {
                if let Some(worktree_path) = &task.worktree_path {
                    let worktree_path_clone = worktree_path.clone();
                    let task_id_clone = task_id.clone();
                    let project_id_clone = project_id.clone();
                    let tasks_signal_clone = tasks_signal.clone();

                    spawn_local(async move {
                        match remove_worktree_for_task(&project_id_clone, &worktree_path_clone).await {
                            Ok(_) => {
                                update_task_worktree_path(task_id_clone, None, project_id_clone, tasks_signal_clone);
                            }
                            Err(e) => {
                                web_sys::console::error_1(&format!("Failed to remove worktree: {}", e).into());
                            }
                        }
                    });
                }
            }
        }
    });

    // Save status change immediately to storage
    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task worktree path
fn update_task_worktree_path(
    task_id: String,
    worktree_path: Option<String>,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.set_worktree_path(worktree_path);
        }
    });

    // Save updated tasks to storage
    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Delete a task
pub fn delete_task(
    task_id: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        tasks.retain(|t| t.id != task_id);
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task details (title and description)
pub fn update_task_details(
    task_id: String,
    new_title: String,
    new_description: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.update_title(new_title);
            task.update_description(new_description);
        }
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task agent profile
pub fn update_task_profile(
    task_id: String,
    profile: AgentProfile,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    tasks_signal.update(|tasks| {
        if let Some(task) = tasks.iter_mut().find(|t| t.id == task_id) {
            task.profile = profile;
        }
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Cancel a task (set status to Cancelled)
pub fn cancel_task(
    task_id: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    update_task_status(task_id, TaskStatus::Cancelled, project_id, tasks_signal);
}