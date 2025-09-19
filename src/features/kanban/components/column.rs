use leptos::prelude::*;
use crate::core::models::TaskStatus;

#[component]
pub fn KanbanColumn(
    #[prop(into)] status: TaskStatus,
    #[prop(into)] tasks: ReadSignal<Vec<crate::core::models::Task>>,
    children: Children,
) -> impl IntoView {
    let status_for_count = status.clone();
    view! {
        <div class="kanban-column">
            <div class="column-header">
                <h3>{status.as_str()}</h3>
                <span class="task-count">
                    {move || {
                        tasks.with(|tasks| {
                            tasks.iter().filter(|t| t.status == status_for_count).count()
                        })
                    }}
                </span>
            </div>
            <div class="column-content">{children()}</div>
        </div>
    }
}
