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

/// Voter id for an anonymous voter, minting the long-lived cookie when absent.
/// Only vote handlers call this, so browsing never sets the cookie.
pub async fn anon_voter(cx: &Cx) -> topcoat::Result<i64> {
    let jar = cookies(cx);
    let id = jar
        .get(ANON_COOKIE)
        .map(|found| found.value().to_owned())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| {
            let id = random_id();
            jar.add(
                Cookie::build((ANON_COOKIE, id.clone()))
                    .path("/")
                    .http_only(true)
                    .secure(!cfg!(debug_assertions))
                    .same_site(SameSite::Lax)
                    .max_age(Duration::seconds(ANON_MAX_AGE_SECS))
                    .build(),
            );
            id
        });

    let prefix = network_prefix(client_ip(cx));
    let user_agent = headers(cx)
        .get(USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    let fp = fingerprint(cookie_key(cx)?, &id, &prefix, user_agent)?;

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
