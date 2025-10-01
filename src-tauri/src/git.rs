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

/// Merges a task branch into the base branch
///
/// # Arguments
/// * `worktree_path` - Path to the worktree
/// * `base_branch` - Name of the base branch to merge into
/// * `project_path` - Path to the main project repository
///
/// # Returns
/// * `Ok(String)` - Success message with merge details
/// * `Err(String)` - Error message if merge fails (including conflict details)
pub fn merge_to_base_branch(worktree_path: &str, base_branch: &str, project_path: &str) -> Result<String, String> {
    println!("Merging worktree at {} to base branch {}", worktree_path, base_branch);

    // Extract task_id from worktree path to get branch name
    let worktree_path_obj = Path::new(worktree_path);
    let task_id = worktree_path_obj
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Failed to extract task ID from worktree path".to_string())?;

    let task_branch = format!("task/{}", task_id);
    println!("Task branch: {}", task_branch);

    // Open the main repository
    let repo = Repository::open(project_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    // Get current branch before switching
    let current_head = repo.head()
        .map_err(|e| format!("Failed to get current HEAD: {}", e))?;
    let current_branch_name = current_head.shorthand().unwrap_or("unknown");
    println!("Current branch: {}", current_branch_name);

    // Checkout the base branch
    println!("Checking out base branch: {}", base_branch);
    let base_branch_ref = repo.find_branch(base_branch, git2::BranchType::Local)
        .map_err(|e| format!("Failed to find base branch '{}': {}", base_branch, e))?;

    let base_commit = base_branch_ref.get().peel_to_commit()
        .map_err(|e| format!("Failed to get base branch commit: {}", e))?;

    repo.checkout_tree(base_commit.as_object(), None)
        .map_err(|e| format!("Failed to checkout base branch: {}", e))?;

    repo.set_head(&format!("refs/heads/{}", base_branch))
        .map_err(|e| format!("Failed to set HEAD to base branch: {}", e))?;

    println!("Successfully checked out {}", base_branch);

    // Find the task branch
    let task_branch_ref = repo.find_branch(&task_branch, git2::BranchType::Local)
        .map_err(|e| format!("Failed to find task branch '{}': {}", task_branch, e))?;

    let task_commit = task_branch_ref.get().peel_to_commit()
        .map_err(|e| format!("Failed to get task branch commit: {}", e))?;

    // Perform the merge
    println!("Merging {} into {}", task_branch, base_branch);
    let mut merge_options = git2::MergeOptions::new();
    let annotated_commit = repo.find_annotated_commit(task_commit.id())
        .map_err(|e| format!("Failed to create annotated commit: {}", e))?;

    let (merge_analysis, _merge_pref) = repo.merge_analysis(&[&annotated_commit])
        .map_err(|e| format!("Failed to analyze merge: {}", e))?;

    if merge_analysis.is_up_to_date() {
        return Ok("Already up to date, no merge needed".to_string());
    }

    if merge_analysis.is_fast_forward() {
        println!("Fast-forward merge possible");
        // Fast-forward merge
        let target_oid = annotated_commit.id();
        let mut reference = repo.find_reference(&format!("refs/heads/{}", base_branch))
            .map_err(|e| format!("Failed to find base branch reference: {}", e))?;

        reference.set_target(target_oid, &format!("Fast-forward merge {} into {}", task_branch, base_branch))
            .map_err(|e| format!("Failed to fast-forward: {}", e))?;

        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| format!("Failed to checkout after fast-forward: {}", e))?;

        return Ok(format!("Successfully fast-forward merged {} into {}", task_branch, base_branch));
    }

    // Normal merge
    println!("Performing normal merge");
    repo.merge(&[&annotated_commit], Some(&mut merge_options), None)
        .map_err(|e| format!("Merge failed: {}", e))?;

    // Check for conflicts
    let mut index = repo.index()
        .map_err(|e| format!("Failed to get repository index: {}", e))?;

    if index.has_conflicts() {
        println!("Merge has conflicts");

        // Collect conflicted files
        let mut conflicted_files = Vec::new();
        for entry in index.conflicts()
            .map_err(|e| format!("Failed to get conflicts: {}", e))?
            .flatten() {
            if let Some(our) = entry.our {
                if let Ok(path) = std::str::from_utf8(&our.path) {
                    conflicted_files.push(path.to_string());
                }
            }
        }

        // Abort the merge
        repo.cleanup_state()
            .map_err(|e| format!("Failed to cleanup merge state: {}", e))?;

        return Err(format!("Merge conflict in files: {}", conflicted_files.join(", ")));
    }

    // Commit the merge
    println!("Committing merge");
    let signature = repo.signature()
        .map_err(|e| format!("Failed to get signature: {}", e))?;

    let tree_id = index.write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_id)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    let base_commit_obj = repo.find_commit(base_commit.id())
        .map_err(|e| format!("Failed to find base commit: {}", e))?;
    let task_commit_obj = repo.find_commit(task_commit.id())
        .map_err(|e| format!("Failed to find task commit: {}", e))?;

    let merge_commit_oid = repo.commit(
        Some(&format!("refs/heads/{}", base_branch)),
        &signature,
        &signature,
        &format!("Merge branch '{}' into {}", task_branch, base_branch),
        &tree,
        &[&base_commit_obj, &task_commit_obj],
    ).map_err(|e| format!("Failed to create merge commit: {}", e))?;

    // Cleanup merge state
    repo.cleanup_state()
        .map_err(|e| format!("Failed to cleanup merge state: {}", e))?;

    println!("Merge successful, commit: {}", merge_commit_oid);
    Ok(format!("Successfully merged {} into {} (commit: {})", task_branch, base_branch, merge_commit_oid))
}

/// File status information
#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct FileStatus {
    pub path: String,
    pub status: String, // "modified", "untracked", "deleted", etc.
}

/// Gets the status of files in a worktree
///
/// # Arguments
/// * `worktree_path` - Path to the worktree
///
/// # Returns
/// * `Ok(Vec<FileStatus>)` - List of files with their status
/// * `Err(String)` - Error message if status check fails
pub fn get_worktree_status(worktree_path: &str) -> Result<Vec<FileStatus>, String> {
    println!("Getting worktree status for: {}", worktree_path);

    let repo = Repository::open(worktree_path)
        .map_err(|e| format!("Failed to open worktree repository: {}", e))?;

    let mut files = Vec::new();
    let statuses = repo.statuses(None)
        .map_err(|e| format!("Failed to get repository status: {}", e))?;

    for entry in statuses.iter() {
        let status_flags = entry.status();
        let path = entry.path().unwrap_or("unknown").to_string();

        let status_str = if status_flags.contains(git2::Status::WT_NEW) {
            "untracked"
        } else if status_flags.contains(git2::Status::WT_MODIFIED) {
            "modified"
        } else if status_flags.contains(git2::Status::WT_DELETED) {
            "deleted"
        } else if status_flags.contains(git2::Status::WT_RENAMED) {
            "renamed"
        } else if status_flags.contains(git2::Status::INDEX_NEW) {
            "staged-new"
        } else if status_flags.contains(git2::Status::INDEX_MODIFIED) {
            "staged-modified"
        } else if status_flags.contains(git2::Status::INDEX_DELETED) {
            "staged-deleted"
        } else {
            "unknown"
        };

        files.push(FileStatus {
            path,
            status: status_str.to_string(),
        });
    }

    println!("Found {} changed files", files.len());
    Ok(files)
}

/// Commits selected files in a worktree
///
/// # Arguments
/// * `worktree_path` - Path to the worktree
/// * `files` - List of file paths to commit
/// * `message` - Commit message
///
/// # Returns
/// * `Ok(String)` - Commit hash
/// * `Err(String)` - Error message if commit fails
pub fn commit_worktree_changes(worktree_path: &str, files: Vec<String>, message: &str) -> Result<String, String> {
    println!("Committing {} files in worktree: {}", files.len(), worktree_path);

    let repo = Repository::open(worktree_path)
        .map_err(|e| format!("Failed to open worktree repository: {}", e))?;

    let mut index = repo.index()
        .map_err(|e| format!("Failed to get repository index: {}", e))?;

    // Add each file to the index
    for file_path in &files {
        println!("Adding file to index: {}", file_path);
        index.add_path(Path::new(file_path))
            .map_err(|e| format!("Failed to add file '{}': {}", file_path, e))?;
    }

    // Write the index
    index.write()
        .map_err(|e| format!("Failed to write index: {}", e))?;

    // Create commit
    let tree_id = index.write_tree()
        .map_err(|e| format!("Failed to write tree: {}", e))?;
    let tree = repo.find_tree(tree_id)
        .map_err(|e| format!("Failed to find tree: {}", e))?;

    let signature = repo.signature()
        .map_err(|e| format!("Failed to get signature: {}", e))?;

    let parent_commit = repo.head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());

    let parents: Vec<&git2::Commit> = if let Some(ref parent) = parent_commit {
        vec![parent]
    } else {
        vec![]
    };

    let commit_id = repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    ).map_err(|e| format!("Failed to create commit: {}", e))?;

    println!("Created commit: {}", commit_id);
    Ok(commit_id.to_string())
}

/// Gets the diff for a specific file in a worktree
///
/// # Arguments
/// * `worktree_path` - Path to the worktree
/// * `file_path` - Path to the file (relative to worktree)
///
/// # Returns
/// * `Ok(String)` - Diff content
/// * `Err(String)` - Error message if diff fails
pub fn get_file_diff(worktree_path: &str, file_path: &str) -> Result<String, String> {
    println!("Getting diff for file: {} in worktree: {}", file_path, worktree_path);

    let repo = Repository::open(worktree_path)
        .map_err(|e| format!("Failed to open worktree repository: {}", e))?;

    // Get the HEAD tree
    let head = repo.head()
        .ok()
        .and_then(|head| head.peel_to_tree().ok());

    let mut diff_options = git2::DiffOptions::new();
    diff_options.pathspec(file_path);

    let diff = repo.diff_tree_to_workdir_with_index(
        head.as_ref(),
        Some(&mut diff_options),
    ).map_err(|e| format!("Failed to create diff: {}", e))?;

    let mut diff_text = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let content = std::str::from_utf8(line.content()).unwrap_or("");
        match line.origin() {
            '+' => diff_text.push_str(&format!("+{}", content)),
            '-' => diff_text.push_str(&format!("-{}", content)),
            ' ' => diff_text.push_str(&format!(" {}", content)),
            _ => diff_text.push_str(content),
        }
        true
    }).map_err(|e| format!("Failed to print diff: {}", e))?;

    Ok(diff_text)
}

/// Lists all git branches in a repository
///
/// # Arguments
/// * `repo_path` - Path to the git repository
///
/// # Returns
/// * `Ok(Vec<String>)` - List of branch names
/// * `Err(String)` - Error message if listing fails
pub fn list_git_branches(repo_path: &str) -> Result<Vec<String>, String> {
    println!("Listing git branches for repository: {}", repo_path);

    let repo = Repository::open(repo_path)
        .map_err(|e| format!("Failed to open repository: {}", e))?;

    let mut branches = Vec::new();

    // List local branches
    let branch_iter = repo.branches(Some(git2::BranchType::Local))
        .map_err(|e| format!("Failed to list branches: {}", e))?;

    for branch_result in branch_iter {
        if let Ok((branch, _branch_type)) = branch_result {
            if let Ok(Some(branch_name)) = branch.name() {
                branches.push(branch_name.to_string());
                println!("Found branch: {}", branch_name);
            }
        }
    }

    println!("Total branches found: {}", branches.len());
    Ok(branches)
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
