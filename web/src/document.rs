use topcoat::Result;
use topcoat::asset::{Asset, asset};
use topcoat::context::Cx;
use topcoat::router::error::NotFoundError;
use topcoat::router::{HeaderValue, Slot, StatusCode, header, layout};
use topcoat::view::{Child, View, component, error_boundary, view};

use crate::auth::sessions::current_account;
use crate::pages::not_found::not_found_page;

pub const STYLESHEET: Asset = asset!(concat!(env!("OUT_DIR"), "/tailwind.css"));

pub const RANKINGS_SCRIPT: Asset = asset!("assets/rankings.js");

pub const SITE_NAME: &str = "Tracker Rankings";

const PAGE_CACHE_CONTROL: HeaderValue =
    HeaderValue::from_static("private, no-cache");

#[layout("/")]
pub(crate) async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        error_boundary(
            fallback: |error| {
                if error.downcast_ref::<NotFoundError>().is_none() {
                    return Err(error);
                }
                Ok(
                    view! {
                        (StatusCode::NOT_FOUND)
                        document(not_found_page())
                    },
                )
            },
            document((slot))
        )
    })
}

#[component]
async fn document(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        ((header::CACHE_CONTROL, PAGE_CACHE_CONTROL))
        <!DOCTYPE html>
        <html lang="en" class="dark">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(SITE_NAME)</title>
                <link rel="stylesheet" href=(STYLESHEET)>
                topcoat::runtime::script()
            </head>
            <body class="min-h-screen bg-background text-foreground antialiased">
                <header class="border-b border-border">
                    <div
                        class="mx-auto flex max-w-7xl flex-wrap items-baseline justify-between gap-x-6 gap-y-2 px-4 py-3"
                    >
                        <a
                            href="/"
                            class="font-semibold tracking-tight whitespace-nowrap"
                        >
                            (SITE_NAME)
                        </a>
                        <nav
                            class="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-sm text-muted-foreground"
                        >
                            <a
                                href="/"
                                class="whitespace-nowrap hover:text-foreground"
                            >
                                "Rankings"
                            </a>
                            <a
                                href="/compare"
                                class="whitespace-nowrap hover:text-foreground"
                            >
                                "Compare"
                            </a>
                            <a
                                href="/about"
                                class="whitespace-nowrap hover:text-foreground"
                            >
                                "About"
                            </a>
                            <a
                                href="/me"
                                class="whitespace-nowrap hover:text-foreground"
                            >
                                "My votes"
                            </a>
                            account_nav()
                        </nav>
                    </div>
                </header>
                (child)
                <footer class="mt-16 border-t border-border">
                    <p
                        class="mx-auto max-w-7xl px-4 py-6 text-xs text-muted-foreground"
                    >
                        "Tracker and route data from "
                        <a
                            href="https://github.com/handokota/trackerpathways"
                            class="underline hover:text-foreground"
                        >
                            "trackerpathways"
                        </a>
                        " (MIT)."
                    </p>
                </footer>
            </body>
        </html>
    })
}

#[component]
async fn account_nav(cx: &Cx) -> Result<impl View> {
    let account = current_account(cx).await?;

    Ok(view! {
        if let Some(account) = account {
            <span class="text-foreground">(account.display_name)</span>
            <form method="post" action="/logout">
                <button type="submit" class="hover:text-foreground">"Sign out"</button>
            </form>
        } else {
            <a href="/login" class="hover:text-foreground">"Sign in"</a>
        }
    })
}
