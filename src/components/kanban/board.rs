use leptos::prelude::*;

#[component]
pub fn KanbanBoard(children: Children) -> impl IntoView {
    view! { <div class="kanban-board">{children()}</div> }
}
