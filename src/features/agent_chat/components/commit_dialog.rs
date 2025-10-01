use leptos::prelude::*;
use leptos::html::Dialog;
use leptos::task::spawn_local;
use serde_wasm_bindgen::to_value;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct FileStatus {
    path: String,
    status: String,
}

#[derive(Debug, Clone, serde::Deserialize)]
struct DiffFile {
    path: String,
    added: u32,
    removed: u32,
    patch: String,
}

fn render_patch(patch: &str) -> Vec<leptos::prelude::AnyView> {
    let mut views: Vec<leptos::prelude::AnyView> = Vec::new();
    for line in patch.lines() {
        let class = if line.starts_with("+++") || line.starts_with("---") || line.starts_with("diff --git") || line.starts_with("index ") {
            "meta"
        } else if line.starts_with("@@") {
            "hunk"
        } else if line.starts_with('+') {
            "add"
        } else if line.starts_with('-') {
            "del"
        } else {
            "ctx"
        };
        views.push(view! {
            <div class=move || format!("diff-line {}", class)>
                {line.to_string()}
            </div>
        }.into_any());
    }
    views
}

#[component]
pub fn CommitDialog(
    dialog_ref: NodeRef<Dialog>,
    #[prop(into)] worktree_path: String,
    #[prop(into)] on_commit_success: Callback<()>,
) -> impl IntoView {
    let (files, set_files) = signal(Vec::<FileStatus>::new());
    let (diffs, set_diffs) = signal(Vec::<DiffFile>::new());
    let (selected_files, set_selected_files) = signal(Vec::<String>::new());
    let (selected_file, set_selected_file) = signal::<Option<String>>(None);
    let (select_all, set_select_all) = signal(false);
    let (dialog_open_trigger, set_dialog_open_trigger) = signal(0u32);

    // Load files and diffs when dialog opens
    let load_files = {
        let worktree = worktree_path.clone();
        let set_files = set_files.clone();
        let set_diffs = set_diffs.clone();
        move || {
            let worktree = worktree.clone();
            let worktree2 = worktree.clone();
            let set_files = set_files.clone();
            let set_diffs = set_diffs.clone();

            // Load file status
            spawn_local(async move {
                let args = serde_json::json!({ "worktreePath": worktree });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("get_worktree_status", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(file_list) = serde_wasm_bindgen::from_value::<Vec<FileStatus>>(js_result) {
                                set_files.set(file_list);
                            }
                        }
                        _ => {
                            web_sys::console::error_1(&"Failed to fetch worktree status".into());
                        }
                    }
                }
            });

            // Load diffs
            spawn_local(async move {
                let args = serde_json::json!({ "worktreePath": worktree2 });
                if let Ok(js_value) = to_value(&args) {
                    match invoke("get_worktree_diffs", js_value).await {
                        js_result if !js_result.is_undefined() => {
                            if let Ok(diff_list) = serde_wasm_bindgen::from_value::<Vec<DiffFile>>(js_result) {
                                web_sys::console::log_1(&format!("Loaded {} diffs", diff_list.len()).into());
                                set_diffs.set(diff_list);
                            }
                        }
                        _ => {
                            web_sys::console::error_1(&"Failed to fetch worktree diffs".into());
                        }
                    }
                }
            });
        }
    };

    // Select file for diff viewing
    let select_file_for_diff = {
        move |file_path: String| {
            web_sys::console::log_1(&format!("Selected file for diff: {}", file_path).into());
            set_selected_file.set(Some(file_path));
        }
    };

    // Toggle file selection
    let toggle_file = {
        let set_selected = set_selected_files.clone();
        move |file_path: String| {
            set_selected.update(|selected| {
                if let Some(pos) = selected.iter().position(|f| f == &file_path) {
                    selected.remove(pos);
                } else {
                    selected.push(file_path);
                }
            });
        }
    };

    // Toggle select all
    let toggle_select_all = {
        let set_selected = set_selected_files.clone();
        move |checked: bool| {
            set_select_all.set(checked);
            if checked {
                set_selected.set(files.get().iter().map(|f| f.path.clone()).collect());
            } else {
                set_selected.set(Vec::new());
            }
        }
    };

    // Commit changes
    let commit_changes = {
        let worktree = worktree_path.clone();
        let dialog = dialog_ref.clone();
        move || {
            let selected = selected_files.get();
            if selected.is_empty() {
                web_sys::window()
                    .and_then(|w| w.alert_with_message("Please select at least one file to commit").ok());
                return;
            }

            // Prompt for commit message
            let commit_msg = web_sys::window()
                .and_then(|w| w.prompt_with_message("Enter commit message:").ok())
                .flatten();

            if let Some(message) = commit_msg {
                if message.trim().is_empty() {
                    web_sys::window()
                        .and_then(|w| w.alert_with_message("Commit message cannot be empty").ok());
                    return;
                }

                let worktree = worktree.clone();
                let dialog = dialog.clone();

                spawn_local(async move {
                    let args = serde_json::json!({
                        "worktreePath": worktree,
                        "files": selected,
                        "message": message
                    });

                    if let Ok(js_value) = to_value(&args) {
                        match invoke("commit_worktree_changes", js_value).await {
                            js_result if !js_result.is_undefined() => {
                                if let Ok(commit_hash) = serde_wasm_bindgen::from_value::<String>(js_result) {
                                    web_sys::window()
                                        .and_then(|w| w.alert_with_message(&format!("✓ Committed successfully!\n\nCommit: {}", commit_hash)).ok());

                                    // Close dialog and trigger success callback
                                    if let Some(dialog) = dialog.get() {
                                        dialog.close();
                                    }
                                    on_commit_success.run(());
                                } else {
                                    web_sys::window()
                                        .and_then(|w| w.alert_with_message("Failed to commit changes").ok());
                                }
                            }
                            _ => {
                                web_sys::window()
                                    .and_then(|w| w.alert_with_message("Failed to invoke commit command").ok());
                            }
                        }
                    }
                });
            }
        }
    };

    // Close dialog
    let close_dialog = {
        let dialog = dialog_ref.clone();
        move || {
            if let Some(dialog) = dialog.get() {
                dialog.close();
            }
        }
    };

    // Load files when dialog is shown (triggered by incrementing the signal)
    Effect::new(move || {
        let _ = dialog_open_trigger.get(); // Track this signal
        load_files();
    });

    // Auto-trigger load when dialog element is mounted
    Effect::new(move || {
        if dialog_ref.get().is_some() {
            set_dialog_open_trigger.update(|n| *n += 1);
        }
    });

    view! {
        <dialog node_ref=dialog_ref class="commit-dialog">
            <div class="commit-dialog-content">
                <div class="commit-dialog-header">
                    <h3>"COMMIT CHANGES"</h3>
                    <button type="button" class="modal-close" on:click=move |_| close_dialog()>"×"</button>
                </div>
                <div class="commit-dialog-body">
                    {/* Left Panel - File List */}
                    <div class="commit-file-list">
                        <div class="select-all-row">
                            <button
                                class="select-all-btn"
                                type="button"
                                on:click=move |_| {
                                    let new_state = !select_all.get();
                                    toggle_select_all(new_state);
                                }
                            >
                                <input
                                    type="checkbox"
                                    checked=move || select_all.get()
                                    on:click=|ev| ev.stop_propagation()
                                />
                                "Select All"
                            </button>
                        </div>
                        <div class="file-list-items">
                            {
                                let select_diff_clone = select_file_for_diff.clone();
                                let toggle_file_clone = toggle_file.clone();
                                move || files.get().iter().map(|file| {
                                    let file_path = file.path.clone();
                                    let file_path_for_toggle = file_path.clone();
                                    let file_path_for_diff = file_path.clone();
                                    let file_path_for_is_selected = file_path.clone();
                                    let file_path_for_display = file_path.clone();
                                    let status = file.status.clone();
                                    let is_selected = move || selected_files.get().contains(&file_path_for_is_selected);
                                    let is_selected_for_input = move || selected_files.get().contains(&file_path);
                                    let select_diff_local = select_diff_clone.clone();
                                    let toggle_file_local = toggle_file_clone.clone();

                                    // Determine status badge letter and class
                                    let (badge_letter, badge_class) = match status.to_lowercase().as_str() {
                                        "modified" => ("M", "modified"),
                                        "untracked" => ("U", "untracked"),
                                        "deleted" => ("D", "deleted"),
                                        "added" => ("A", "added"),
                                        _ => ("M", "modified"),
                                    };
                                    let badge_title = match status.to_lowercase().as_str() {
                                        "modified" => "Modified",
                                        "untracked" => "Untracked",
                                        "deleted" => "Deleted",
                                        "added" => "Added",
                                        _ => "Modified",
                                    };

                                    view! {
                                        <div class="file-item" class:selected=is_selected>
                                            <div
                                                class="file-item-checkbox"
                                                on:click=move |_| {
                                                    toggle_file_local(file_path_for_toggle.clone());
                                                }
                                            >
                                                <input
                                                    type="checkbox"
                                                    checked=is_selected_for_input
                                                    on:click=|ev| ev.stop_propagation()
                                                />
                                            </div>
                                            <div class="file-info" on:click=move |_| {
                                                select_diff_local(file_path_for_diff.clone());
                                            }>
                                                <span class="file-name">{file_path_for_display}</span>
                                                <span class=format!("file-status-badge {}", badge_class) title=badge_title>
                                                    {badge_letter}
                                                </span>
                                            </div>
                                        </div>
                                    }
                                }).collect_view()
                            }
                        </div>
                    </div>

                    {/* Right Panel - Diff Viewer */}
                    <div class="commit-diff-panel">
                        {
                            let commit_changes_clone = commit_changes.clone();
                            move || {
                                let selected = selected_file.get();
                                let all_diffs = diffs.get();
                                let commit_changes_local = commit_changes_clone.clone();
                                let commit_changes_local2 = commit_changes_clone.clone();

                                if let Some(selected_path) = selected {
                                    // Find the diff for this file
                                    let patch = all_diffs.iter()
                                        .find(|d| d.path == selected_path)
                                        .map(|d| d.patch.clone())
                                        .unwrap_or_else(|| "No diff available".to_string());

                                    view! {
                                        <div class="diff-content">
                                            <div class="diff-patch">
                                                {render_patch(&patch)}
                                            </div>
                                        </div>
                                        <button
                                            class="commit-dialog-btn"
                                            on:click=move |_| commit_changes_local()
                                            disabled=move || selected_files.get().is_empty()
                                        >
                                            "COMMIT CHANGES"
                                        </button>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div class="diff-placeholder">
                                            <p>"Select a file to view its diff"</p>
                                        </div>
                                        <button
                                            class="commit-dialog-btn"
                                            on:click=move |_| commit_changes_local2()
                                            disabled=move || selected_files.get().is_empty()
                                        >
                                            "COMMIT CHANGES"
                                        </button>
                                    }.into_any()
                                }
                            }
                        }
                    </div>
                </div>
            </div>
        </dialog>
    }
}
