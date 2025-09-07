pub mod project;
pub mod task;

// Export the Task and TaskStatus types for use throughout the app
// Project is now being used for the ProjectModal
pub use project::Project;
pub use task::{Task, TaskStatus};