use topcoat::router::RouterBuilder;

pub mod account;
pub mod anon;
pub mod discord;
pub mod fingerprint;
pub mod oauth;
pub mod provider;
pub mod reddit;
pub mod routes;
pub mod sessions;
mod state;
pub mod voter;

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.route(routes::start_sign_in)
        .route(routes::oauth_callback)
        .route(routes::logout)
}
