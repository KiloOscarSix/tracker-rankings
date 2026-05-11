use topcoat::Result;
use topcoat::router::not_found;
use topcoat::view::{View, component, view};

not_found!("/");

#[component]
pub(crate) async fn not_found_page() -> Result<impl View> {
    Ok(view! {
        <main class="mx-auto max-w-3xl px-4 py-16">
            <h1 class="text-3xl font-semibold">"404"</h1>
            <p class="mt-4 text-neutral-400">"We couldn't find that page."</p>
            <a href="/" class="mt-6 inline-block underline">"Back to the rankings"</a>
        </main>
    })
}
