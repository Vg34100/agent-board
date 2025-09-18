use leptos::prelude::*;
use std::collections::HashMap;
use std::rc::Rc;
use crate::components::task_sidebar::AgentMessage;

#[component]
pub fn AgentsPanel(
    #[prop(into)] task_id: String,
    processes: Vec<serde_json::Value>,
    messages_by_process: ReadSignal<HashMap<String, Vec<AgentMessage>>>,
    on_load_messages: Rc<dyn Fn(String) + 'static>,
    active_process_id: ReadSignal<Option<String>>,
) -> impl IntoView {
    // Filter and sort processes for this task
    let mut groups: Vec<serde_json::Value> = processes
        .into_iter()
        .filter(|p| p.get("task_id").and_then(|v| v.as_str()) == Some(task_id.as_str()))
        .collect();
    groups.sort_by(|a, b| a
        .get("start_time")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .cmp(b.get("start_time").and_then(|v| v.as_str()).unwrap_or("")));
    let total = groups.len();

    view! {
        <div class="agents-tab">
            <div class="agent-sessions" id={format!("agent-sessions-{}", task_id)}>
                { if groups.is_empty() {
                    view! { <div class="no-agents"><p>"No agent processes yet"</p><p class="hint">"Agent will be spawned automatically when you start the task"</p></div> }.into_any()
                } else {
                    view! { <div class="process-groups">{groups.into_iter().enumerate().map(|(idx, proc)| {
                        let pid = proc.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let status = proc.get("status").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                        let kind = proc.get("kind").and_then(|v| v.as_str()).unwrap_or("").to_string();
                        let default_open = idx + 1 == total;
                        let short_id = pid.chars().take(8).collect::<String>();
                        let pid_for_msgs = pid.clone();
                        let pid_for_click = pid.clone();
                        let pid_for_list = pid.clone();
                        let loader = on_load_messages.clone();
                        let pid_for_open = pid.clone();
                        view! { <details open={move || active_process_id.get().as_deref() == Some(pid_for_open.as_str()) || (active_process_id.get().is_none() && default_open)}>
                            <summary class="process-summary" on:click=move |_| { (loader)(pid_for_click.clone()); }>
                                <span class="proc-kind">{kind.clone()}</span>
                                <span class="proc-id">{short_id}</span>
                                <span class=move || format!("proc-status {}", status)> {status.clone()} </span>
                            </summary>
                            <div class="message-list" id={format!("agent-messages-{}", pid_for_list)}>
                                { move || {
                                    if let Some(msgs) = messages_by_process.get().get(&pid_for_msgs).cloned() {
                                        msgs.into_iter().map(|msg| {
                                            let icon = match msg.sender.as_str() { "user"=>"🞓","agent"=>"🟆","system"=>"🞧",_=>"" };
                                            let message_class = format!("message {}", msg.sender);
                                            let short_code = pid_for_msgs.chars().take(6).collect::<String>();
                                            view! { <div class=message_class>
                                                <div class="message-header">
                                                    <span class="message-icon">{icon}</span>
                                                    <span class="sender">{msg.sender.clone()}</span>
                                                    <span class="proc-code">{short_code}</span>
                                                    <span class="time">{msg.timestamp.clone()}</span>
                                                </div>
                                                <div class="message-content">{ view! { <div>{msg.content.clone()}</div> } }</div>
                                            </div> }.into_any()
                                        }).collect::<Vec<_>>()
                                    } else {
                                        vec![view!{ <div class={"no-agents".to_string()}><p class="hint">"Expand to load messages"</p></div> }.into_any()]
                                    }
                                } }
                            </div>
                        </details> }.into_any()}).collect::<Vec<_>>()}</div> }.into_any()
                }}
            </div>
        </div>
    }
}
