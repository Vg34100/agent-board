use git2::{Repository, Branch, BranchType};
use std::path::{Path, PathBuf};
use std::fs;

pub struct GitWorktree {
    pub path: PathBuf,
    pub branch_name: String,
}

/// Creates a new git worktree for a task
/// 
/// # Arguments
/// * `project_path` - Path to the main project repository
/// * `task_id` - Unique identifier for the task
/// 
/// # Returns
/// * `Ok(GitWorktree)` - Contains the path to the created worktree and branch name
/// * `Err(String)` - Error message if worktree creation fails
pub fn create_worktree(project_path: &str, task_id: &str) -> Result<GitWorktree, String> {
    // Open the repository
    let repo = Repository::open(project_path)
        .map_err(|e| format!("Failed to open repository at {}: {}", project_path, e))?;

    // Generate branch name
    let branch_name = format!("task/{}", task_id);
    
    // Create temporary directory for worktree
    let temp_dir = std::env::temp_dir();
    let worktree_dir = temp_dir.join(format!("agent-board-worktree-{}", task_id));
    
    // Remove existing worktree directory if it exists
    if worktree_dir.exists() {
        fs::remove_dir_all(&worktree_dir)
            .map_err(|e| format!("Failed to remove existing worktree directory: {}", e))?;
    }

    // Get the current HEAD commit
    let head = repo.head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_commit = head.peel_to_commit()
        .map_err(|e| format!("Failed to get HEAD commit: {}", e))?;

    // Create the worktree with git2
    // Note: git2 doesn't have direct worktree support, so we'll use git command for now
    let output = std::process::Command::new("git")
        .args(&["worktree", "add", "-b", &branch_name, worktree_dir.to_str().unwrap(), "HEAD"])
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git worktree creation failed: {}", error));
    }

    Ok(GitWorktree {
        path: worktree_dir,
        branch_name,
    })
}

/// Removes a git worktree
/// 
/// # Arguments
/// * `worktree_path` - Path to the worktree to remove
/// 
/// # Returns
/// * `Ok(())` - If worktree was successfully removed
/// * `Err(String)` - Error message if removal fails
pub fn remove_worktree(worktree_path: &str) -> Result<(), String> {
    let worktree_path = Path::new(worktree_path);
    
    if !worktree_path.exists() {
        return Ok(()); // Already removed
    }

    // Use git command to remove worktree
    let output = std::process::Command::new("git")
        .args(&["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git worktree removal failed: {}", error));
    }

    Ok(())
}

/// Lists all worktrees for a repository
/// 
/// # Arguments
/// * `project_path` - Path to the main project repository
/// 
/// # Returns
/// * `Ok(Vec<String>)` - List of worktree paths
/// * `Err(String)` - Error message if listing fails
pub fn list_worktrees(project_path: &str) -> Result<Vec<String>, String> {
    let output = std::process::Command::new("git")
        .args(&["worktree", "list", "--porcelain"])
        .current_dir(project_path)
        .output()
        .map_err(|e| format!("Failed to execute git command: {}", e))?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Git worktree list failed: {}", error));
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut worktrees = Vec::new();
    
    for line in output_str.lines() {
        if line.starts_with("worktree ") {
            let path = line.strip_prefix("worktree ").unwrap_or("");
            worktrees.push(path.to_string());
        }
    }

    Ok(worktrees)
}

/// Opens the file manager to the specified worktree path
/// 
/// # Arguments
/// * `worktree_path` - Path to the worktree directory
/// 
/// # Returns
/// * `Ok(())` - If file manager was opened successfully
/// * `Err(String)` - Error message if opening fails
pub fn open_worktree_location(worktree_path: &str) -> Result<(), String> {
    let path = Path::new(worktree_path);
    
    if !path.exists() {
        return Err(format!("Worktree path does not exist: {}", worktree_path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
    }

    Ok(())
}