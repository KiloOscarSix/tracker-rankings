use topcoat::cookie::RouterBuilderCookieExt;
use topcoat::router::RouterBuilder;
use topcoat::runtime::{PrefetchMode, RouterBuilderRuntimeExt};
use topcoat::session::RouterBuilderSessionExt;

use crate::app;
use crate::auth::sessions::session_config;
use crate::rate_limit::{
    auth_rate_limit,
    delete_rate_limit,
    logout_rate_limit,
    vote_rate_limit,
};

#[must_use]
pub fn router() -> RouterBuilder {
    app::routes()
        .layer(auth_rate_limit())
        .layer(logout_rate_limit())
        .layer(vote_rate_limit())
        .layer(delete_rate_limit())
        .sessions(session_config())
        .runtime()
        .prefetch(PrefetchMode::Never)
        .cookies()
}
