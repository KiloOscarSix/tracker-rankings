use topcoat::context::Cx;
use topcoat::cookie::time::Duration;
use topcoat::cookie::{Cookie, Cookies, SameSite, cookies};
use topcoat::router::header::USER_AGENT;
use topcoat::router::request::{client_ip, headers};

use crate::auth::fingerprint::{fingerprint, network_prefix};
use crate::auth::state::random_id;
use crate::context::{cookie_key, db};

pub const ANON_COOKIE: &str = "anon_id";
const ANON_MAX_AGE_SECS: i64 = 400 * 24 * 60 * 60;

fn cookie_id(cx: &Cx) -> Option<String> {
    cookies(cx)
        .get(ANON_COOKIE)
        .map(|found| found.value().to_owned())
        .filter(|value| !value.is_empty())
}

fn request_fingerprint(cx: &Cx, cookie_id: &str) -> topcoat::Result<[u8; 32]> {
    let prefix = network_prefix(client_ip(cx));
    let user_agent = headers(cx)
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    Ok(fingerprint(cookie_key(cx)?, cookie_id, &prefix, user_agent)?)
}

fn anon_cookie(value: String, max_age_secs: i64) -> Cookie<'static> {
    Cookie::build((ANON_COOKIE, value))
        .path("/")
        .http_only(true)
        .secure(!cfg!(debug_assertions))
        .same_site(SameSite::Lax)
        .max_age(Duration::seconds(max_age_secs))
        .build()
}

/// Voter id for an anonymous voter, minting the long-lived cookie when absent.
/// Only vote handlers call this, so browsing never sets the cookie.
pub async fn anon_voter(cx: &Cx) -> topcoat::Result<i64> {
    let id = cookie_id(cx).unwrap_or_else(|| {
        let id = random_id();
        cookies(cx).add(anon_cookie(id.clone(), ANON_MAX_AGE_SECS));
        id
    });
    let fp = request_fingerprint(cx, &id)?;

    let voter_id = sqlx::query_scalar!(
        r#"WITH anon AS (
               INSERT INTO anon_voters (fingerprint_hmac) VALUES ($1)
               ON CONFLICT (fingerprint_hmac)
                   DO UPDATE SET fingerprint_hmac = EXCLUDED.fingerprint_hmac
               RETURNING id
           ), voter AS (
               INSERT INTO voters (kind, anon_id)
               SELECT 'anon'::voter_kind, id FROM anon
               ON CONFLICT (anon_id) DO UPDATE SET anon_id = EXCLUDED.anon_id
               RETURNING id
           )
           SELECT id AS "id!" FROM voter"#,
        fp.as_slice(),
    )
    .fetch_one(db(cx)?)
    .await?;
    Ok(voter_id)
}

/// The anonymous voter this request already has, if any. Never inserts and
/// never sets a cookie, so read-only pages can call it.
pub async fn existing_anon_voter(cx: &Cx) -> topcoat::Result<Option<i64>> {
    let Some(id) = cookie_id(cx) else {
        return Ok(None);
    };
    let fp = request_fingerprint(cx, &id)?;

    let voter_id = sqlx::query_scalar!(
        "SELECT v.id
         FROM voters v
         JOIN anon_voters a ON a.id = v.anon_id
         WHERE a.fingerprint_hmac = $1",
        fp.as_slice(),
    )
    .fetch_optional(db(cx)?)
    .await?;
    Ok(voter_id)
}

pub fn clear_anon_cookie(cx: &Cx) {
    cookies(cx).add(anon_cookie(String::new(), 0));
}
