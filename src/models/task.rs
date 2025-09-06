use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    ToDo,
    InProgress,
    InReview,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::ToDo => "To Do",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::InReview => "In Review", 
            TaskStatus::Done => "Done",
            TaskStatus::Cancelled => "Cancelled",
        }
    }
    
    pub fn all() -> Vec<TaskStatus> {
        vec![
            TaskStatus::ToDo,
            TaskStatus::InProgress,
            TaskStatus::InReview,
            TaskStatus::Done,
            TaskStatus::Cancelled,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Utc>,
}

impl Task {
    #[allow(dead_code)] // Will be used when task editing is implemented
    pub fn new(project_id: String, title: String, description: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            project_id,
            title,
            description,
            status: TaskStatus::ToDo,
            created_at: Utc::now(),
        }
    }
}