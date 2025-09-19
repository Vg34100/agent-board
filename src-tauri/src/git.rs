use git2::{Repository, WorktreeAddOptions};
use std::path::{Path, PathBuf};
use std::fs;
use tauri::{AppHandle, Manager};
use serde::{Serialize, Deserialize};
use std::process::Command;

#[derive(Debug)]
pub struct GitWorktree {
    pub path: PathBuf,
    pub _branch_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffFile {
    pub path: String,
    pub added: u32,
    pub removed: u32,
    pub patch: String,
}

fn count_added_removed_from_patch(patch_text: &str) -> (u32, u32) {
    let mut added = 0u32;
    let mut removed = 0u32;
    for line in patch_text.lines() {
        if line.starts_with("+++") || line.starts_with("---") || line.starts_with("@@") || line.starts_with("diff --git") || line.starts_with("index ") {
            continue;
        }
        if line.starts_with('+') { added += 1; }
        else if line.starts_with('-') { removed += 1; }
    }
    (added, removed)
}

fn list_untracked_files(worktree_path: &str) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["-C", worktree_path, "ls-files", "--others", "--exclude-standard"])
        .output()
        .map_err(|e| format!("Failed to run git ls-files: {}", e))?;
    if !out.status.success() {
        return Err(format!("git ls-files failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let mut files = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if !line.trim().is_empty() {
            files.push(line.trim().to_string());
        }
    }
    Ok(files)
}

pub fn get_worktree_diffs(worktree_path: &str) -> Result<Vec<DiffFile>, String> {
    println!("[diffs] worktree_path={} ", worktree_path);
    // Get added/removed counts per file
    let numstat = Command::new("git")
        .args(["-C", worktree_path, "diff", "--numstat"])
        .output()
        .map_err(|e| format!("Failed to run git diff --numstat: {}", e))?;
    if !numstat.status.success() {
        println!("[diffs] git diff --numstat failed: {}", String::from_utf8_lossy(&numstat.stderr));
        // Don't early-return; continue to try untracked path
    }
    let mut counts: std::collections::HashMap<String, (u32, u32)> = std::collections::HashMap::new();
    for line in String::from_utf8_lossy(&numstat.stdout).lines() {
        // format: added<TAB>removed<TAB>path
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let added = parts[0].parse::<u32>().unwrap_or(0);
            let removed = parts[1].parse::<u32>().unwrap_or(0);
            let path = parts[2].to_string();
            counts.insert(path, (added, removed));
        }
    }

    // Get unified diff
    let patch_out = Command::new("git")
        .args(["-C", worktree_path, "diff", "--unified=3", "--no-color"])
        .output()
        .map_err(|e| format!("Failed to run git diff: {}", e))?;
    if !patch_out.status.success() {
        println!("[diffs] git diff failed: {}", String::from_utf8_lossy(&patch_out.stderr));
    }
    let patch_text = String::from_utf8_lossy(&patch_out.stdout);
    println!("[diffs] tracked patch bytes: {}", patch_out.stdout.len());

    // Split by files
    let mut files = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in patch_text.lines() {
        if line.starts_with("diff --git ") {
            if !current.is_empty() {
                if let Some(df) = parse_diff_file(&current.join("\n"), &counts) {
                    files.push(df);
                }
                current.clear();
            }
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        if let Some(df) = parse_diff_file(&current.join("\n"), &counts) {
            files.push(df);
        }
    }

    // Include untracked files as new-file patches via --no-index
    if let Ok(untracked) = list_untracked_files(worktree_path) {
        println!("[diffs] untracked count: {}", untracked.len());
        for rel in untracked {
            println!("[diffs] untracked: {}", rel);
            // Build patch for new file by comparing /dev/null to the file
            // Use OS-specific null path
            #[cfg(target_os = "windows")]
            let null_path = "NUL"; // git accepts NUL as null device
            #[cfg(not(target_os = "windows"))]
            let null_path = "/dev/null";

            let out = Command::new("git")
                .args(["-C", worktree_path, "diff", "--no-index", "--unified=3", "--no-color", null_path, &rel])
                .output()
                .map_err(|e| format!("Failed to run git diff --no-index: {}", e))?;
            println!("[diffs] no-index status: {:?} stdout_len={} stderr_len={}", out.status.code(), out.stdout.len(), out.stderr.len());
            let patch = if out.stdout.is_empty() {
                // Fallback: synthesize patch by reading file content
                let full_path = std::path::Path::new(worktree_path).join(&rel);
                match std::fs::read_to_string(&full_path) {
                    Ok(content) => {
                        let mut p = String::new();
                        p.push_str(&format!("diff --git a/{0} b/{0}\n", rel));
                        p.push_str("new file mode 100644\n");
                        p.push_str("index 0000000..0000000\n");
                        p.push_str("--- /dev/null\n");
                        p.push_str(&format!("+++ b/{}\n", rel));
                        p.push_str("@@ -0,0 +1,? @@\n");
                        for line in content.lines() {
                            p.push('+');
                            p.push_str(line);
                            p.push('\n');
                        }
                        p
                    }
                    Err(e) => {
                        println!("[diffs] failed to read untracked file {}: {}", full_path.display(), e);
                        String::new()
                    }
                }
            } else {
                String::from_utf8_lossy(&out.stdout).to_string()
            };
            if !patch.is_empty() {
                let (added, removed) = count_added_removed_from_patch(&patch);
                println!("[diffs] synthesized patch for {} (+{} -{})", rel, added, removed);
                let df = DiffFile { path: rel.clone(), added, removed, patch };
                files.push(df);
            }
        }
    }

    println!("[diffs] total files collected: {}", files.len());
    Ok(files)
}

fn parse_diff_file(block: &str, counts: &std::collections::HashMap<String, (u32, u32)>) -> Option<DiffFile> {
    // Try to find +++ b/<path> as canonical new path
    let mut path: Option<String> = None;
    for line in block.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            path = Some(rest.to_string());
            break;
        }
    }
    if path.is_none() {
        // Fallback to --- a/<path>
        for line in block.lines() {
            if let Some(rest) = line.strip_prefix("--- a/") {
                path = Some(rest.to_string());
                break;
            }
        }
    }
    let path = path?;
    let (added, removed) = counts.get(&path).cloned().unwrap_or((0, 0));
    Some(DiffFile { path, added, removed, patch: block.to_string() })
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
    let _worktree = repo.worktree(
        &task_id,  // worktree name
        worktree_dir.as_path(),  // worktree path
        Some(&opts)  // options
    ).map_err(|e| format!("Failed to create worktree: {}", e))?;

    println!("Successfully created worktree at: {:?}", worktree_dir);

    Ok(GitWorktree {
        path: worktree_dir,
        _branch_name: branch_name,
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
pub fn remove_worktree(_app: &AppHandle, worktree_path: &str, project_path: &str) -> Result<(), String> {
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

    // Try common VS Code paths first since PATH might not be available
    #[cfg(target_os = "windows")]
    {
        let vscode_paths = [
            // Most common locations
            "C:\\Users\\{}\\AppData\\Local\\Programs\\Microsoft VS Code\\Code.exe",
            "C:\\Program Files\\Microsoft VS Code\\Code.exe", 
            "C:\\Program Files (x86)\\Microsoft VS Code\\Code.exe",
            // Alternative locations
            "C:\\Users\\{}\\AppData\\Local\\Programs\\Microsoft VS Code\\bin\\code.cmd",
            "C:\\Program Files\\Microsoft VS Code\\bin\\code.cmd",
            "C:\\Program Files (x86)\\Microsoft VS Code\\bin\\code.cmd",
        ];
        
        // Try user-specific paths first
        if let Ok(username) = std::env::var("USERNAME") {
            for path_template in &vscode_paths {
                if path_template.contains("{}") {
                    let user_path = path_template.replace("{}", &username);
                    if Path::new(&user_path).exists() {
                        match std::process::Command::new(&user_path)
                            .arg(worktree_path)
                            .spawn() 
                        {
                            Ok(_) => {
                                println!("Opened VS Code with path: {}", user_path);
                                return Ok(());
                            }
                            Err(e) => {
                                println!("Failed to open VS Code at {}: {}", user_path, e);
                                continue;
                            }
                        }
                    }
                }
            }
        }
        
        // Try system-wide paths
        for path in &vscode_paths {
            if !path.contains("{}") && Path::new(path).exists() {
                match std::process::Command::new(path)
                    .arg(worktree_path)
                    .spawn()
                {
                    Ok(_) => {
                        println!("Opened VS Code with path: {}", path);
                        return Ok(());
                    }
                    Err(e) => {
                        println!("Failed to open VS Code at {}: {}", path, e);
                        continue;
                    }
                }
            }
        }
    }
    
    // Try code.cmd first on Windows (what's actually in PATH)
    #[cfg(target_os = "windows")]
    {
        match std::process::Command::new("code.cmd")
            .arg(worktree_path)
            .spawn()
        {
            Ok(_) => {
                println!("Opened VS Code with 'code.cmd' command");
                return Ok(());
            }
            Err(e) => {
                println!("Failed to open VS Code with 'code.cmd' command: {}", e);
            }
        }
    }
    
    // Finally try the code command (fallback)
    match std::process::Command::new("code")
        .arg(worktree_path)
        .spawn()
    {
        Ok(_) => {
            println!("Opened VS Code with 'code' command");
            Ok(())
        }
        Err(e) => {
            println!("Failed to open VS Code with 'code' command: {}", e);
            Err(format!("VS Code not found. Tried installation paths, 'code.cmd', and 'code' command. Please ensure VS Code is installed and in PATH."))
        }
    }
}
