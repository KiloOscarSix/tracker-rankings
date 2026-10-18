use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;
use topcoat::session::RouterBuilderSessionExt;

use crate::auth::sessions::session_config;
use crate::rate_limit::{
    auth_rate_limit,
    delete_rate_limit,
    logout_rate_limit,
    vote_rate_limit,
};
use crate::{auth, document, pages, votes};

const ROUTE_GROUPS: [fn(RouterBuilder) -> RouterBuilder; 3] =
    [pages::routes, auth::routes, votes::routes];

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    let base = base.layout(document::root_layout);

    ROUTE_GROUPS
        .into_iter()
        .fold(base, |builder, group| group(builder))
        .layer(auth_rate_limit())
        .layer(logout_rate_limit())
        .layer(vote_rate_limit())
        .layer(delete_rate_limit())
        .sessions(session_config())
        .runtime()
        .cookies()
        .build()
}
