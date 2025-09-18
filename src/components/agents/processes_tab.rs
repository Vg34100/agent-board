use leptos::prelude::*;

#[component]
pub fn ProcessesTab(
    processes: Vec<serde_json::Value>,
    #[prop(into)] task_id: String,
) -> impl IntoView {
    let current_task_processes: Vec<_> = processes
        .iter()
        .filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(task_id.as_str()))
        .cloned()
        .collect();
    let has_processes = !current_task_processes.is_empty();

    view! {
        <div class="processes-tab">
            <div class="process-list">
                <h4>"Agent Processes"</h4>
                {if !has_processes {
                    view! {
                        <div class="no-processes">
                            <p>"No processes spawned yet"</p>
                            <p class="hint">"Process details with expandable JSON will appear here"</p>
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="process-items">
                            {current_task_processes.into_iter().map(|proc| {
                                let proc_id = proc.get("id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let status = proc.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let msg_count = proc.get("message_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                let task_id = proc.get("task_id").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                                let json_content = serde_json::to_string_pretty(&proc).unwrap_or_else(|_| "Invalid JSON".to_string());

                                let status_class = format!("process-status {}", status);
                                let proc_id_short = format!("{}...", &proc_id[..8.min(proc_id.len())]);
                                let task_id_display = format!("Task: {}", task_id);
                                let status_display = status.clone();
                                let msg_count_display = format!("{} msgs", msg_count);

                                view! { <div class="process-item">
                                    <div class="process-header">
                                        <span class="process-id">{proc_id_short}</span>
                                        <span class="task-id">{task_id_display}</span>
                                        <span class=status_class>{status_display}</span>
                                        <span class="message-count">{msg_count_display}</span>
                                    </div>
                                    <details>
                                        <summary>"Show JSON Details"</summary>
                                        <pre class="json-content">{json_content}</pre>
                                    </details>
                                </div> }
                            }).collect::<Vec<_>>()}
                        </div>
                    }.into_any()
                }}
            </div>
        </div>
    }
}

