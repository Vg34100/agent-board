use git2::{Repository, WorktreeAddOptions, Worktree, Reference};
use std::path::{Path, PathBuf};
use std::fs;
use tauri::AppHandle;

#[derive(Debug)]
pub struct GitWorktree {
    pub path: PathBuf,
    pub branch_name: String,
}

/// Gets the app data directory for storing worktrees
/// Uses Tauri's app data directory instead of system temp folder
fn get_worktrees_base_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app.path().app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;
    
    let worktrees_dir = app_data_dir.join("worktrees");
    
    // Create the worktrees directory if it doesn't exist
    if !worktrees_dir.exists() {
        fs::create_dir_all(&worktrees_dir)
            .map_err(|e| format!("Failed to create worktrees directory: {}", e))?;
        println!("Created worktrees directory: {:?}", worktrees_dir);
    }
    
    Ok(worktrees_dir)
}

/// Creates a new git worktree for a task using proper git2-rs API
/// 
/// # Arguments
/// * `app` - Tauri app handle for getting app data directory
/// * `project_path` - Path to the main project repository
/// * `task_id` - Unique identifier for the task
/// 
/// # Returns
/// * `Ok(GitWorktree)` - Contains the path to the created worktree and branch name
/// * `Err(String)` - Error message if worktree creation fails
pub fn create_worktree(app: &AppHandle, project_path: &str, task_id: &str) -> Result<GitWorktree, String> {
    println!("Creating worktree for task {} in project {}", task_id, project_path);
    
    // Open the repository
    let repo = Repository::open(project_path)
        .map_err(|e| format!("Failed to open repository at {}: {}", project_path, e))?;
    
    println!("Successfully opened repository: {}", project_path);

    // Generate branch name following vibe-kanban pattern
    let branch_name = format!("task/{}", task_id);
    
    // Get worktrees base directory (in app data, not temp)
    let worktrees_base = get_worktrees_base_dir(app)?;
    let worktree_dir = worktrees_base.join(&task_id);
    
    println!("Worktree will be created at: {:?}", worktree_dir);
    
    // Remove existing worktree directory if it exists
    if worktree_dir.exists() {
        println!("Removing existing worktree directory: {:?}", worktree_dir);
        fs::remove_dir_all(&worktree_dir)
            .map_err(|e| format!("Failed to remove existing worktree directory: {}", e))?;
    }

    // Get the current HEAD commit to create branch from
    let head = repo.head()
        .map_err(|e| format!("Failed to get HEAD: {}", e))?;
    let head_commit = head.peel_to_commit()
        .map_err(|e| format!("Failed to get HEAD commit: {}", e))?;

    println!("Creating branch '{}' from commit {}", branch_name, head_commit.id());

    // Create a new branch from HEAD
    let branch = repo.branch(&branch_name, &head_commit, false)
        .map_err(|e| format!("Failed to create branch '{}': {}", branch_name, e))?;

    println!("Successfully created branch: {}", branch_name);

    // Get the reference for the new branch
    let branch_ref = branch.get();
    
    // Setup worktree options
    let mut opts = WorktreeAddOptions::new();
    opts.reference(Some(branch_ref));
    
    println!("Creating worktree with git2-rs API...");

    // Create the worktree using git2-rs proper API
    let _worktree = repo.worktree(&worktree_dir, Some(&opts))
        .map_err(|e| format!("Failed to create worktree: {}", e))?;

    println!("Successfully created worktree at: {:?}", worktree_dir);

    Ok(GitWorktree {
        path: worktree_dir,
        branch_name,
    })
}

/// Removes a git worktree and cleans up the branch
/// 
/// # Arguments
/// * `app` - Tauri app handle for directory management
/// * `worktree_path` - Path to the worktree to remove
/// * `project_path` - Path to the main project repository (for branch cleanup)
/// 
/// # Returns
/// * `Ok(())` - If worktree was successfully removed
/// * `Err(String)` - Error message if removal fails
pub fn remove_worktree(app: &AppHandle, worktree_path: &str, project_path: &str) -> Result<(), String> {
    println!("Removing worktree at: {}", worktree_path);
    
    let worktree_path = Path::new(worktree_path);
    
    if !worktree_path.exists() {
        println!("Worktree path does not exist, considering it already removed: {}", worktree_path.display());
        return Ok(()); // Already removed
    }

    // Open main repository to clean up branch
    let repo = Repository::open(project_path)
        .map_err(|e| format!("Failed to open repository for cleanup: {}", e))?;

    // Extract task ID from worktree path to determine branch name
    if let Some(task_id) = worktree_path.file_name().and_then(|name| name.to_str()) {
        let branch_name = format!("task/{}", task_id);
        
        println!("Attempting to remove branch: {}", branch_name);
        
        // Try to remove the branch (non-fatal if it fails)
        if let Ok(mut branch) = repo.find_branch(&branch_name, git2::BranchType::Local) {
            match branch.delete() {
                Ok(_) => println!("Successfully removed branch: {}", branch_name),
                Err(e) => println!("Warning: Failed to remove branch '{}': {}", branch_name, e),
            }
        } else {
            println!("Branch '{}' not found or already removed", branch_name);
        }
    }

    // Remove the worktree directory
    fs::remove_dir_all(worktree_path)
        .map_err(|e| format!("Failed to remove worktree directory: {}", e))?;

    println!("Successfully removed worktree directory: {}", worktree_path.display());

    Ok(())
}

/// Lists all worktrees in the app's worktrees directory
/// 
/// # Arguments
/// * `app` - Tauri app handle for getting app data directory
/// 
/// # Returns
/// * `Ok(Vec<String>)` - List of worktree directory names (task IDs)
/// * `Err(String)` - Error message if listing fails
pub fn list_app_worktrees(app: &AppHandle) -> Result<Vec<String>, String> {
    let worktrees_base = get_worktrees_base_dir(app)?;
    
    println!("Listing worktrees in: {:?}", worktrees_base);
    
    let mut worktrees = Vec::new();
    
    if worktrees_base.exists() {
        let entries = fs::read_dir(&worktrees_base)
            .map_err(|e| format!("Failed to read worktrees directory: {}", e))?;
        
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        worktrees.push(name.to_string());
                        println!("Found worktree: {}", name);
                    }
                }
            }
        }
    }
    
    println!("Total worktrees found: {}", worktrees.len());
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
    
    println!("Opening file manager for: {}", worktree_path);
    
    if !path.exists() {
        return Err(format!("Worktree path does not exist: {}", worktree_path));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
        println!("Opened Windows Explorer for: {}", worktree_path);
    }

    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
        println!("Opened Finder for: {}", worktree_path);
    }

    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(worktree_path)
            .spawn()
            .map_err(|e| format!("Failed to open file manager: {}", e))?;
        println!("Opened file manager for: {}", worktree_path);
    }

    Ok(())
}

/// Opens the worktree in VS Code IDE
/// 
/// # Arguments
/// * `worktree_path` - Path to the worktree directory
/// 
/// # Returns
/// * `Ok(())` - If IDE was opened successfully
/// * `Err(String)` - Error message if opening fails
pub fn open_worktree_in_ide(worktree_path: &str) -> Result<(), String> {
    let path = Path::new(worktree_path);
    
    println!("Opening IDE for: {}", worktree_path);
    
    if !path.exists() {
        return Err(format!("Worktree path does not exist: {}", worktree_path));
    }

    // Try to open with VS Code (code command)
    let result = std::process::Command::new("code")
        .arg(worktree_path)
        .spawn();

    match result {
        Ok(_) => {
            println!("Opened VS Code for: {}", worktree_path);
            Ok(())
        }
        Err(e) => {
            println!("Failed to open VS Code ({}), trying alternative methods...", e);
            
            // Fallback: try with full path to VS Code
            #[cfg(target_os = "windows")]
            {
                let vscode_paths = [
                    "C:\\Users\\{}\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
                    "C:\\Program Files\\Microsoft VS Code\\Code.exe",
                    "C:\\Program Files (x86)\\Microsoft VS Code\\Code.exe",
                ];
                
                if let Ok(username) = std::env::var("USERNAME") {
                    let user_path = vscode_paths[0].replace("{}", &username);
                    if Path::new(&user_path).exists() {
                        return std::process::Command::new(&user_path)
                            .arg(worktree_path)
                            .spawn()
                            .map(|_| {
                                println!("Opened VS Code with full path: {}", user_path);
                            })
                            .map_err(|e| format!("Failed to open VS Code with full path: {}", e));
                    }
                }
                
                for path in &vscode_paths[1..] {
                    if Path::new(path).exists() {
                        return std::process::Command::new(path)
                            .arg(worktree_path)
                            .spawn()
                            .map(|_| {
                                println!("Opened VS Code with path: {}", path);
                            })
                            .map_err(|e| format!("Failed to open VS Code: {}", e));
                    }
                }
            }
            
            Err(format!("VS Code not found. Please ensure VS Code is installed and 'code' command is in PATH"))
        }
    }
}