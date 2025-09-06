use leptos::prelude::*;
use crate::pages::{Projects, Kanban};

#[derive(Clone, Debug)]
pub enum AppView {
    Projects,
    Kanban(String),
}

#[component]
pub fn App() -> impl IntoView {
    let (current_view, set_current_view) = signal(AppView::Projects);
    
    provide_context(set_current_view);

    view! {
        <main class="app">
            {move || match current_view.get() {
                AppView::Projects => view! { <Projects /> }.into_any(),
                AppView::Kanban(project_id) => view! { <Kanban project_id=project_id /> }.into_any(),
            }}
        </main>
    }
}
