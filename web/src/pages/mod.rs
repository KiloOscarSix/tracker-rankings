use topcoat::router::RouterBuilder;

pub mod about;
pub mod compare;
pub mod login;
pub mod me;
pub mod not_found;
pub mod rankings;
pub mod tracker;

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(rankings::rankings)
        .page(tracker::tracker)
        .page(about::about)
        .page(compare::compare)
        .page(login::login)
        .page(me::me)
        .page(not_found::not_found)
}
