use leptos::prelude::*;
use crate::features::kanban::KanbanPage;

#[component]
pub fn Kanban(project_id: String) -> impl IntoView {
    view! {
        <KanbanPage project_id />
    }
}
