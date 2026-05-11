use topcoat::Result;
use topcoat::asset::{Asset, asset};
use topcoat::router::error::NotFoundError;
use topcoat::router::{Slot, StatusCode, layout};
use topcoat::view::{Child, View, component, error_boundary, view};

use crate::pages::not_found::not_found_page;

pub const STYLESHEET: Asset = asset!(concat!(env!("OUT_DIR"), "/tailwind.css"));

pub const SITE_NAME: &str = "Tracker Ladder";

#[layout("/")]
pub(crate) async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
    Ok(view! {
        error_boundary(
            fallback: |error| {
                if error.downcast_ref::<NotFoundError>().is_none() {
                    return Err(error);
                }
                Ok(view! {
                    (StatusCode::NOT_FOUND)
                    document(not_found_page())
                })
            },
            document((slot))
        )
    })
}

#[component]
async fn document(#[default] child: Child<'_>) -> Result<impl View> {
    Ok(view! {
        <!DOCTYPE html>
        <html lang="en">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <title>(SITE_NAME)</title>
                <link rel="stylesheet" href=(STYLESHEET)>
                topcoat::runtime::script()
            </head>
            <body class="min-h-screen bg-neutral-950 text-neutral-100 antialiased">
                (child)
            </body>
        </html>
    })
}
