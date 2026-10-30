use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::{SeeOther, see_other};
use topcoat::router::route;

use crate::auth::sessions::sign_out;
use crate::context::db;

#[route(POST)]
pub(crate) async fn logout(cx: &Cx) -> Result<SeeOther> {
    sign_out(cx, db(cx)?).await?;
    Ok(see_other("/"))
}
