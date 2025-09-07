use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub project_path: String, // The actual directory path where the project is located
    pub git_path: Option<String>, // For existing git repos, this is the same as project_path
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Project {
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new(name: String, project_path: String, git_path: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            project_path,
            git_path,
            setup_script: None,
            cleanup_script: None,
            created_at: Utc::now(),
        }
    }
    
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new_git_project(name: String, project_path: String) -> Self {
        Self::new(name, project_path, None)
    }
    
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new_existing_project(name: String, git_path: String) -> Self {
        Self::new(name, git_path.clone(), Some(git_path))
    }
}