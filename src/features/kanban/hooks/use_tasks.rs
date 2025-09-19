use leptos::prelude::*;
use leptos::task::spawn_local;
use crate::core::models::{Task, TaskStatus, AgentProfile};
use crate::core::services::load_tasks;
use crate::features::kanban::services::{
    create_task_handler, update_task_status, delete_task,
    update_task_details, update_task_profile, cancel_task
};

pub struct TasksHook {
    pub tasks: ReadSignal<Vec<Task>>,
    pub create_task: Box<dyn Fn(Task) + 'static>,
    pub update_status: Box<dyn Fn(String, TaskStatus) + 'static>,
    pub update_details: Box<dyn Fn(String, String, String) + 'static>,
    pub update_profile: Box<dyn Fn(String, AgentProfile) + 'static>,
    pub delete_task: Box<dyn Fn(String) + 'static>,
    pub cancel_task: Box<dyn Fn(String) + 'static>,
}

pub fn use_tasks(project_id: String) -> TasksHook {
    let tasks = RwSignal::new(Vec::<Task>::new());

    // Load tasks on mount
    {
        let project_id_clone = project_id.clone();
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

    // Create handlers
    let create_task = create_task_handler(project_id.clone(), tasks);

    let update_status = {
        let project_id = project_id.clone();
        Box::new(move |task_id: String, status: TaskStatus| {
            update_task_status(task_id, status, project_id.clone(), tasks);
        }) as Box<dyn Fn(String, TaskStatus) + 'static>
    };

    let update_details = {
        let project_id = project_id.clone();
        Box::new(move |task_id: String, title: String, description: String| {
            update_task_details(task_id, title, description, project_id.clone(), tasks);
        }) as Box<dyn Fn(String, String, String) + 'static>
    };

    let update_profile = {
        let project_id = project_id.clone();
        Box::new(move |task_id: String, profile: AgentProfile| {
            update_task_profile(task_id, profile, project_id.clone(), tasks);
        }) as Box<dyn Fn(String, AgentProfile) + 'static>
    };

    let delete_task_fn = {
        let project_id = project_id.clone();
        Box::new(move |task_id: String| {
            delete_task(task_id, project_id.clone(), tasks);
        }) as Box<dyn Fn(String) + 'static>
    };

    let cancel_task_fn = {
        let project_id = project_id.clone();
        Box::new(move |task_id: String| {
            cancel_task(task_id, project_id.clone(), tasks);
        }) as Box<dyn Fn(String) + 'static>
    };

    TasksHook {
        tasks: tasks.read_only(),
        create_task,
        update_status,
        update_details,
        update_profile,
        delete_task: delete_task_fn,
        cancel_task: cancel_task_fn,
    }
}