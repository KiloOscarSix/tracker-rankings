use topcoat::context::Cx;
use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookie, Cookies, SameSite, cookies};
use topcoat::session::Token;

pub(crate) const OAUTH_STATE_COOKIE: &str = "oauth_state";
const STATE_MAX_AGE_SECS: i64 = 600;
const STATE_PATH: &str = "/auth";

/// 32 random bytes as unpadded URL-safe base64, so the value needs no escaping
/// in a cookie or a query string.
pub(crate) fn random_id() -> String {
    Token::random().encode().trim_end_matches('=').to_owned()
}

pub(crate) fn remember(cx: &Cx) -> String {
    let value = random_id();
    cookies(cx).add(cookie(value.clone(), STATE_MAX_AGE_SECS));
    value
}

pub(crate) fn take(cx: &Cx) -> Option<String> {
    let jar = cookies(cx);
    let value = jar
        .get(OAUTH_STATE_COOKIE)
        .map(|found| found.value().to_owned())
        .filter(|value| !value.is_empty());
    jar.add(cookie(String::new(), 0));
    value
}

fn cookie(value: String, max_age_secs: i64) -> Cookie<'static> {
    Cookie::build((OAUTH_STATE_COOKIE, value))
        .path(STATE_PATH)
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(Duration::seconds(max_age_secs))
        .build()
}
