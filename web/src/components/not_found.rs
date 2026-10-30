use topcoat::Result;
use topcoat::context::Cx;
use topcoat::runtime::{link_attrs, prefetch_mode};
use topcoat::view::{View, component, view};

#[component]
pub async fn not_found_page(cx: &Cx) -> Result<impl View> {
    Ok(view! {
        <main class="mx-auto max-w-3xl px-4 py-16">
            <h1 class="text-3xl font-semibold">"404"</h1>
            <p class="mt-4 text-muted-foreground">"We couldn't find that page."</p>
            <a
                class="mt-6 inline-block underline"
                (link_attrs(cx, "/", prefetch_mode(cx)))
            >
                "Back to the rankings"
            </a>
        </main>
    })
}
