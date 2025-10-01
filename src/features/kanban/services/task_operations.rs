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
    web_sys::console::log_1(&format!("update_task_status called: task_id={}, new_status={:?}", task_id, new_status).into());

    // Capture old status and worktree path before mutation
    let (old_status, worktree_path_opt) = tasks_signal.with_untracked(|tasks| {
        tasks.iter()
            .find(|t| t.id == task_id)
            .map(|t| (t.status.clone(), t.worktree_path.clone()))
            .unwrap_or((TaskStatus::ToDo, None))
    });

    web_sys::console::log_1(&format!("old_status={:?}, new_status={:?}", old_status, new_status).into());

    // Update the task status by replacing the task in the vector
    // This ensures Leptos detects the change and triggers reactivity
    tasks_signal.update(|tasks| {
        if let Some(index) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks[index].clone();
            task.update_status(new_status.clone());
            tasks[index] = task;
            web_sys::console::log_1(&format!("Task status updated in vector at index {}", index).into());
        } else {
            web_sys::console::error_1(&format!("Task {} not found in tasks vector!", task_id).into());
        }
    });

    // Handle worktree operations based on status transition
    // If task is moving to InProgress, create a worktree only if missing
    if new_status == TaskStatus::InProgress && old_status != TaskStatus::InProgress {
        let has_worktree = tasks_signal.with_untracked(|tasks| {
            tasks.iter()
                .find(|t| t.id == task_id)
                .and_then(|t| t.worktree_path.as_ref())
                .is_some()
        });

        if has_worktree {
            web_sys::console::log_1(&format!("Reusing existing worktree for task {}", task_id).into());
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

    // If task is moving away from InProgress/InReview (to Done/Cancelled), remove worktree
    if (new_status == TaskStatus::Done || new_status == TaskStatus::Cancelled) && (old_status == TaskStatus::InProgress || old_status == TaskStatus::InReview) {
        if let Some(worktree_path) = worktree_path_opt {
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
    // Replace the task to ensure reactivity
    tasks_signal.update(|tasks| {
        if let Some(index) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks[index].clone();
            task.set_worktree_path(worktree_path);
            tasks[index] = task;
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
    // Replace the task to ensure reactivity
    tasks_signal.update(|tasks| {
        if let Some(index) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks[index].clone();
            task.update_title(new_title);
            task.update_description(new_description);
            tasks[index] = task;
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
    // Replace the task to ensure reactivity
    tasks_signal.update(|tasks| {
        if let Some(index) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks[index].clone();
            task.profile = profile;
            tasks[index] = task;
        }
    });

    let current_tasks = tasks_signal.get_untracked();
    save_tasks_async(project_id, current_tasks);
}

// Update task base branch
pub fn update_task_base_branch(
    task_id: String,
    base_branch: String,
    project_id: String,
    tasks_signal: RwSignal<Vec<Task>>,
) {
    // Replace the task to ensure reactivity
    tasks_signal.update(|tasks| {
        if let Some(index) = tasks.iter().position(|t| t.id == task_id) {
            let mut task = tasks[index].clone();
            task.set_base_branch(base_branch);
            tasks[index] = task;
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