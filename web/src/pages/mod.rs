use topcoat::router::RouterBuilder;

pub mod about;
pub mod login;
pub mod not_found;
pub mod rankings;
pub mod tracker;

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(rankings::rankings)
        .page(tracker::tracker)
        .page(about::about)
        .page(login::login)
        .page(not_found::not_found)
}
