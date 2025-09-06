use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Project {
    pub id: String,
    pub name: String,
    pub git_path: Option<String>,
    pub setup_script: Option<String>,
    pub cleanup_script: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl Project {
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new(name: String, git_path: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            git_path,
            setup_script: None,
            cleanup_script: None,
            created_at: Utc::now(),
        }
    }
    
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new_git_project(name: String) -> Self {
        Self::new(name, None)
    }
    
    #[allow(dead_code)] // Will be used when project creation is implemented
    pub fn new_existing_project(name: String, git_path: String) -> Self {
        Self::new(name, Some(git_path))
    }
}