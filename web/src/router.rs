use topcoat::router::{Router, RouterBuilder};
use topcoat::runtime::RouterBuilderRuntimeExt;

use crate::{document, pages};

const ROUTE_GROUPS: [fn(RouterBuilder) -> RouterBuilder; 1] = [pages::routes];

#[must_use]
pub fn router(base: RouterBuilder) -> Router {
    let base = base.layout(document::root_layout);

    ROUTE_GROUPS
        .into_iter()
        .fold(base, |builder, group| group(builder))
        .runtime()
        .build()
}
