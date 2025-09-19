use leptos::prelude::*;
use wasm_bindgen::prelude::*;
use leptos::task::spawn_local;
use serde::Deserialize;
use serde_wasm_bindgen::to_value;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[derive(Debug, Clone, Deserialize)]
struct DiffFile {
    path: String,
    added: u32,
    removed: u32,
    patch: String,
}

fn render_patch(patch: &str) -> Vec<leptos::prelude::AnyView> {
    let mut views: Vec<leptos::prelude::AnyView> = Vec::new();
    for line in patch.lines() {
        // Skip global headers except hunk/file lines
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
pub fn DiffTab(
    #[prop(into)] task_id: String,
    #[prop(optional)] worktree_path: Option<String>,
) -> impl IntoView {
    let (diffs, set_diffs) = signal(Vec::<DiffFile>::new());
    let (expanded, set_expanded) = signal(std::collections::HashSet::<String>::new());
    let (error, set_error) = signal::<Option<String>>(None);
    // Normalize provided path: treat empty string as None
    let initial_path_opt = match worktree_path.clone() {
        Some(p) if !p.trim().is_empty() => Some(p),
        _ => None,
    };
    let (path_sig, set_path_sig) = signal::<Option<String>>(initial_path_opt);

    // Resolve worktree path if not provided via latest process details for this task
    let should_resolve = match worktree_path.as_ref() {
        Some(p) => p.trim().is_empty(),
        None => true,
    };
    if should_resolve {
        let task_id_clone = task_id.clone();
        let set_path_sig = set_path_sig.clone();
        spawn_local(async move {
            web_sys::console::log_1(&format!("[DiffTab] resolving path for task {}", task_id_clone).into());
            // get_process_list -> pick latest for task -> get_process_details -> worktree_path
            let empty = serde_json::json!({});
            if let Ok(js) = to_value(&empty) {
                let resp = invoke("get_process_list", js).await;
                if !resp.is_undefined() {
                    if let Ok(list) = serde_wasm_bindgen::from_value::<Vec<serde_json::Value>>(resp) {
                        web_sys::console::log_1(&format!("[DiffTab] processes fetched: {}", list.len()).into());
                        // filter by task_id and pick latest by start_time string desc
                        let mut candidates: Vec<_> = list.into_iter()
                            .filter(|v| v.get("task_id").and_then(|x| x.as_str()) == Some(task_id_clone.as_str()))
                            .collect();
                        web_sys::console::log_1(&format!("[DiffTab] candidates for task: {}", candidates.len()).into());
                        candidates.sort_by(|a, b| {
                            let sa = a.get("start_time").and_then(|x| x.as_str()).unwrap_or("");
                            let sb = b.get("start_time").and_then(|x| x.as_str()).unwrap_or("");
                            sb.cmp(sa)
                        });
                        if let Some(first) = candidates.first() {
                            if let Some(pid) = first.get("id").and_then(|x| x.as_str()) {
                                web_sys::console::log_1(&format!("[DiffTab] using latest process {}", pid).into());
                                let args = serde_json::json!({ "processId": pid });
                                if let Ok(js2) = to_value(&args) {
                                    let resp2 = invoke("get_process_details", js2).await;
                                    if !resp2.is_undefined() {
                                        if let Ok(proc) = serde_wasm_bindgen::from_value::<serde_json::Value>(resp2) {
                                            if let Some(path) = proc.get("worktree_path").and_then(|x| x.as_str()) {
                                                web_sys::console::log_1(&format!("[DiffTab] resolved path {}", path).into());
                                                set_path_sig.set(Some(path.to_string()));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    // When we have a path, load diffs
    {
        let set_diffs = set_diffs.clone();
        let set_error = set_error.clone();
        Effect::new(move |_| {
            if let Some(path) = path_sig.get() {
                if path.trim().is_empty() { return; }
                let set_diffs = set_diffs.clone();
                let set_error = set_error.clone();
                spawn_local(async move {
                    web_sys::console::log_1(&format!("[DiffTab] loading diffs for {}", path).into());
                    let args = serde_json::json!({ "worktreePath": path });
                    if let Ok(js) = to_value(&args) {
                        let resp = invoke("get_worktree_diffs", js).await;
                        if !resp.is_undefined() {
                            match serde_wasm_bindgen::from_value::<Vec<DiffFile>>(resp) {
                                Ok(files) => { web_sys::console::log_1(&format!("[DiffTab] got {} files", files.len()).into()); set_diffs.set(files) },
                                Err(e) => set_error.set(Some(format!("Failed to parse diffs: {}", e))),
                            }
                        }
                    }
                });
            }
        });
    }

    view! {
        <div class="diff-tab">
            <div class="diff-content">
                {move || if let Some(err) = error.get() {
                    view! { <div class="diff-error">{err}</div> }.into_any()
                } else if path_sig.get().is_none() {
                    view! { <div class="placeholder-content"><p>"No worktree for this task yet."</p></div> }.into_any()
                } else if diffs.get().is_empty() {
                    view! { <div class="placeholder-content"><p>"No changes detected."</p></div> }.into_any()
                } else {
                    let files = diffs.get();
                    view! {
                        <div class="diff-file-list">
                            {files.into_iter().map(|file| {
                                let key = file.path.clone();
                                let path_display = file.path.clone();
                                let added = file.added;
                                let removed = file.removed;
                                let patch = file.patch.clone();
                                let is_open = expanded.with(|s| s.contains(&key));
                                view! {
                                    <div class="diff-file-item">
                                        <button class="diff-file-header" on:click={
                                            let key2 = key.clone();
                                            let set_expanded = set_expanded.clone();
                                            move |_| {
                                                set_expanded.update(|s| {
                                                    if s.contains(&key2) { s.remove(&key2); } else { s.insert(key2.clone()); }
                                                });
                                            }
                                        }>
                                            <span class="file-name">{path_display}</span>
                                            <span class="chip add">{format!("+{}", added)}</span>
                                            <span class="chip del">{format!("-{}", removed)}</span>
                                        </button>
                                        {move || if expanded.with(|s| s.contains(&key)) {
                                            view! { <div class="diff-patch">{ render_patch(&patch) }</div> }.into_any()
                                        } else { view! {}.into_any() }}
                                    </div>
                                }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

