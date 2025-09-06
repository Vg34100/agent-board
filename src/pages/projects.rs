use leptos::prelude::*;
use crate::app::AppView;

#[component]
pub fn Projects() -> impl IntoView {
    let navigate = use_context::<WriteSignal<AppView>>().expect("navigate context");
    
    let open_project = move |project_id: &str| {
        let id = project_id.to_string();
        navigate.set(AppView::Kanban(id));
    };

    view! {
        <div class="projects-page">
            <header class="projects-header">
                <h1>"Agent Board"</h1>
                <button class="btn-primary">"+ CREATE PROJECT"</button>
            </header>
            
            <div class="projects-grid">
                <div class="project-card" on:click=move |_| open_project("sample-1")>
                    <h3>"Sample Project"</h3>
                    <p>"Click to open kanban board"</p>
                    <div class="project-stats">
                        <span>"3 tasks"</span>
                        <span>"1 in progress"</span>
                    </div>
                </div>
                <div class="project-card" on:click=move |_| open_project("demo-project")>
                    <h3>"Demo Project"</h3>
                    <p>"Another sample project"</p>
                    <div class="project-stats">
                        <span>"5 tasks"</span>
                        <span>"2 in progress"</span>
                    </div>
                </div>
            </div>
        </div>
    }
}