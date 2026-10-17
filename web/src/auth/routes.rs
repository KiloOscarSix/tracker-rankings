use topcoat::Result;
use topcoat::context::Cx;
use topcoat::router::error::{RouterErrorExt, SeeOther, see_other};
use topcoat::router::{path_param, query_params, route};
use tracing::warn;

use crate::auth::account::upsert_account;
use crate::auth::provider::Provider;
use crate::auth::sessions::{sign_in, sign_out};
use crate::auth::state;
use crate::context::{db, oauth_clients};

path_param!(pub provider_name: Provider, error = not_found);

const AUTH_FAILED_PATH: &str = "/login?error=auth_failed";

#[query_params(error = bad_request)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
}

#[route(POST "/auth/{provider_name}")]
pub async fn start_sign_in(cx: &Cx) -> Result<SeeOther> {
    let provider = *path_param::<ProviderName>(cx)?;
    let client = oauth_clients(cx)?.get(provider).ok_or_not_found()?;
    let state = state::remember(cx);
    Ok(see_other(client.authorize_url(&state)))
}

#[route(GET "/auth/{provider_name}/callback")]
pub async fn oauth_callback(cx: &Cx) -> Result<SeeOther> {
    let expected = state::take(cx);
    let provider = *path_param::<ProviderName>(cx)?;
    let client = oauth_clients(cx)?.get(provider).ok_or_not_found()?;
    let query = query_params::<CallbackQuery>(cx)?;

    let (Some(code), Some(returned), Some(expected)) =
        (query.code.as_deref(), query.state.as_deref(), expected.as_deref())
    else {
        return Ok(see_other(AUTH_FAILED_PATH));
    };
    if returned != expected {
        return Ok(see_other(AUTH_FAILED_PATH));
    }

    let identity = match client.exchange(code).await {
        Ok(identity) => identity,
        Err(e) => {
            warn!(error = %e, provider = provider.key(), "OAuth sign-in failed");
            return Ok(see_other(AUTH_FAILED_PATH));
        },
    };

    let pool = db(cx)?;
    let id = upsert_account(pool, provider, &identity).await?;
    sign_in(cx, pool, id).await?;
    Ok(see_other("/"))
}

#[route(POST "/logout")]
pub async fn logout(cx: &Cx) -> Result<SeeOther> {
    sign_out(cx, db(cx)?).await?;
    Ok(see_other("/"))
}
