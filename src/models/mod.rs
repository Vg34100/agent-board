pub mod project;
pub mod task;

// Export the Task and TaskStatus types for use throughout the app
// Project is commented out since it's not currently used but will be needed later for Phase 2
// pub use project::Project;
pub use task::{Task, TaskStatus};