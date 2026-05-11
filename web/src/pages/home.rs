use topcoat::Result;
use topcoat::router::page;
use topcoat::view::{View, view};

use crate::document::SITE_NAME;

#[page("/")]
pub(crate) async fn home() -> Result<impl View> {
    Ok(view! {
        <main class="mx-auto max-w-3xl px-4 py-16">
            <h1 class="text-3xl font-semibold">(SITE_NAME)</h1>
            <p class="mt-4 text-neutral-400">
                "Community rankings of private trackers, and whether the next rung is worth the climb."
            </p>
        </main>
    })
}
