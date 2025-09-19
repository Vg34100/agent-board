use leptos::prelude::*;
// Generic visual wrapper for a task card

#[component]
pub fn TaskCard(
    #[prop(into)] dropdown_open: MaybeSignal<bool>,
    on_click: Box<dyn Fn() + 'static>,
    children: Children,
) -> impl IntoView {
    view! {
        <div
            class="task-card clickable"
            class:dropdown-open=move || dropdown_open.get()
            on:click=move |_| { on_click(); }
        >
            {children()}
        </div>
    }
}
