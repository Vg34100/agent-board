use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AgentProfile {
    ClaudeCode,
    Codex,
}

fn default_agent_profile() -> AgentProfile {
    AgentProfile::ClaudeCode
}

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
    pub worktree_path: Option<String>,
    #[serde(default = "default_agent_profile")]
    pub profile: AgentProfile,
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
            worktree_path: None,
            profile: default_agent_profile(),
        }
    }

    pub fn update_title(&mut self, new_title: String) {
        self.title = new_title;
    }

    pub fn update_description(&mut self, new_description: String) {
        self.description = new_description;
    }

    pub fn update_status(&mut self, new_status: TaskStatus) {
        self.status = new_status;
    }

    pub fn set_worktree_path(&mut self, path: Option<String>) {
        self.worktree_path = path;
    }
}
