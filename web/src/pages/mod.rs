use topcoat::router::RouterBuilder;

pub mod home;
pub mod not_found;

#[must_use]
pub fn routes(base: RouterBuilder) -> RouterBuilder {
    base.page(home::home).page(not_found::not_found)
}
