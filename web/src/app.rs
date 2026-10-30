pub mod about;
pub mod auth;
pub mod compare;
pub mod login;
pub mod logout;
pub mod me;
pub mod rankings;
pub mod t;
pub mod vote;

use topcoat::Result;
use topcoat::router::error::NotFoundError;
use topcoat::router::{
    RouterBuilder,
    Slot,
    StatusCode,
    layout,
    module_router,
    not_found,
};
use topcoat::view::{View, error_boundary, view};

use crate::components::not_found::not_found_page;
use crate::document::document;

not_found!();

#[must_use]
pub(crate) fn routes() -> RouterBuilder {
    module_router!()
}

#[layout]
async fn root_layout(slot: Slot<'_>) -> Result<impl View> {
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
