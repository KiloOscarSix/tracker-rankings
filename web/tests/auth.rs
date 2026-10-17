use std::net::SocketAddr;
use std::num::NonZeroU32;

use governor::Quota;
use sqlx::PgPool;
use topcoat::context::Cx;
use topcoat::router::request::Request;
use topcoat::router::{Body, Method, RemoteAddr, Router, StatusCode, header, route};
use web::auth::anon::anon_voter;
use web::auth::sessions::prune_expired_sessions;
use web::rate_limit::AUTH_QUOTA;

mod common;

use common::{
    CREATED_AT,
    GOOD_CODE,
    TestResult,
    app,
    app_with,
    location,
    send,
    set_cookie,
};

const CLIENT: &str = "203.0.113.5:4000";
const FAILED: &str = "/login?error=auth_failed";

#[route(POST "/test/anon")]
async fn anon_probe(cx: &Cx) -> topcoat::Result<String> {
    Ok(anon_voter(cx).await?.to_string())
}

fn request(
    method: Method,
    uri: &str,
    headers: &[(&str, &str)],
    peer: &str,
) -> TestResult<Request> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .extension(RemoteAddr(peer.parse::<SocketAddr>()?));
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    Ok(builder.body(Body::empty())?)
}

async fn begin(router: &Router, provider: &str) -> TestResult<String> {
    let uri = format!("/auth/{provider}");
    let reply = send(router, request(Method::POST, &uri, &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    Ok(set_cookie(&reply, "oauth_state").ok_or("no oauth_state cookie")?)
}

async fn sign_in(router: &Router, provider: &str) -> TestResult<String> {
    let state = begin(router, provider).await?;
    let uri = format!("/auth/{provider}/callback?code={GOOD_CODE}&state={state}");
    let cookie = format!("oauth_state={state}");
    let reply =
        send(router, request(Method::GET, &uri, &[("cookie", &cookie)], CLIENT)?)
            .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/"));
    Ok(set_cookie(&reply, "__Host-session").ok_or("no session cookie")?)
}

async fn get_with_session(
    router: &Router,
    path: &str,
    session: &str,
) -> TestResult<String> {
    let cookie = format!("__Host-session={session}");
    let reply =
        send(router, request(Method::GET, path, &[("cookie", &cookie)], CLIENT)?)
            .await?;
    assert_eq!(reply.status, StatusCode::OK);
    Ok(reply.body)
}

async fn count_accounts(pool: &PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM accounts"#)
        .fetch_one(pool)
        .await?)
}

async fn count_sessions(pool: &PgPool) -> TestResult<i64> {
    Ok(sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM sessions"#)
        .fetch_one(pool)
        .await?)
}

#[sqlx::test(migrations = "../migrations")]
async fn browsing_sets_no_cookies(pool: PgPool) -> TestResult<()> {
    let router = app(pool, false).await?;

    for path in ["/", "/about", "/login"] {
        let reply = send(&router, request(Method::GET, path, &[], CLIENT)?).await?;
        assert_eq!(reply.status, StatusCode::OK, "{path}");
        assert!(reply.headers.get(header::SET_COOKIE).is_none(), "{path}");
    }
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn login_redirects_to_the_provider_with_a_state_cookie(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool, false).await?;

    let reply =
        send(&router, request(Method::POST, "/auth/discord", &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let state = set_cookie(&reply, "oauth_state").ok_or("no oauth_state cookie")?;
    let target = location(&reply).ok_or("no location")?;
    assert_eq!(
        target.strip_prefix("https://fake.test/discord/authorize?state="),
        Some(state.as_str())
    );

    let reply =
        send(&router, request(Method::POST, "/auth/nope", &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn callback_signs_in_and_stores_provider_age(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), false).await?;
    let session = sign_in(&router, "discord").await?;

    let epoch = sqlx::query_scalar!(
        r#"SELECT extract(epoch FROM provider_created_at)::float8 AS "e!" FROM accounts"#
    )
    .fetch_one(&pool)
    .await?;
    assert!((epoch - CREATED_AT).abs() < 0.001);

    let account_voters = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM voters WHERE kind = 'account'"#
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(account_voters, 1);
    assert_eq!(count_sessions(&pool).await?, 1);

    let html = get_with_session(&router, "/", &session).await?;
    assert!(html.contains("Discord Tester"));
    assert!(html.contains("Sign out"));

    sign_in(&router, "discord").await?;
    assert_eq!(count_accounts(&pool).await?, 1);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn callback_rejects_a_wrong_state_or_failed_exchange(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool.clone(), false).await?;

    let mismatched = format!("/auth/discord/callback?code={GOOD_CODE}&state=b");
    let reply = send(
        &router,
        request(Method::GET, &mismatched, &[("cookie", "oauth_state=a")], CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some(FAILED));

    let reply =
        send(&router, request(Method::GET, &mismatched, &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some(FAILED));

    let bad_code = "/auth/discord/callback?code=bad&state=a";
    let reply = send(
        &router,
        request(Method::GET, bad_code, &[("cookie", "oauth_state=a")], CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some(FAILED));

    assert_eq!(count_accounts(&pool).await?, 0);

    let reply = send(&router, request(Method::GET, FAILED, &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.body.contains("Sign-in failed"));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn logout_ends_the_session(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), false).await?;
    let session = sign_in(&router, "discord").await?;
    assert_eq!(count_sessions(&pool).await?, 1);

    let cookie = format!("__Host-session={session}");
    let reply = send(
        &router,
        request(Method::POST, "/logout", &[("cookie", &cookie)], CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/"));
    assert_eq!(count_sessions(&pool).await?, 0);

    let html = get_with_session(&router, "/", &session).await?;
    assert!(html.contains("Sign in"));
    assert!(!html.contains("Sign out"));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn cross_site_posts_are_forbidden(pool: PgPool) -> TestResult<()> {
    let router = app(pool, false).await?;
    let cross_site = [("sec-fetch-site", "cross-site")];

    for path in ["/logout", "/auth/discord"] {
        let reply =
            send(&router, request(Method::POST, path, &cross_site, CLIENT)?).await?;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{path}");
    }
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn auth_routes_are_rate_limited_per_ip(pool: PgPool) -> TestResult<()> {
    let burst_of_two = Quota::per_hour(NonZeroU32::MIN.saturating_add(1));
    let router = app_with(pool, false, burst_of_two, |builder| builder).await?;

    for expected in [StatusCode::SEE_OTHER, StatusCode::SEE_OTHER] {
        let reply =
            send(&router, request(Method::POST, "/auth/discord", &[], CLIENT)?)
                .await?;
        assert_eq!(reply.status, expected);
    }
    let limited =
        send(&router, request(Method::POST, "/auth/discord", &[], CLIENT)?).await?;
    assert_eq!(limited.status, StatusCode::TOO_MANY_REQUESTS);
    assert!(limited.headers.get(header::RETRY_AFTER).is_some());

    let other = send(
        &router,
        request(Method::POST, "/auth/discord", &[], "198.51.100.7:4000")?,
    )
    .await?;
    assert_eq!(other.status, StatusCode::SEE_OTHER);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn anon_voter_is_stable_per_cookie_and_network(
    pool: PgPool,
) -> TestResult<()> {
    let router = app_with(pool.clone(), false, AUTH_QUOTA, |builder| {
        builder.route(anon_probe)
    })
    .await?;
    let agent = ("user-agent", "test-agent");

    let first = send(
        &router,
        request(Method::POST, "/test/anon", &[agent], "203.0.113.5:4000")?,
    )
    .await?;
    assert_eq!(first.status, StatusCode::OK);
    let cookie_id = set_cookie(&first, "anon_id").ok_or("no anon_id cookie")?;
    let cookie = format!("anon_id={cookie_id}");

    let same_network = send(
        &router,
        request(
            Method::POST,
            "/test/anon",
            &[agent, ("cookie", &cookie)],
            "203.0.113.99:4000",
        )?,
    )
    .await?;
    assert_eq!(same_network.body, first.body);
    assert!(set_cookie(&same_network, "anon_id").is_none());

    let other_network = send(
        &router,
        request(
            Method::POST,
            "/test/anon",
            &[agent, ("cookie", &cookie)],
            "198.51.100.7:4000",
        )?,
    )
    .await?;
    assert_ne!(other_network.body, first.body);

    let anon_voters =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM anon_voters"#)
            .fetch_one(&pool)
            .await?;
    assert_eq!(anon_voters, 2);
    let all_32_bytes = sqlx::query_scalar!(
        r#"SELECT bool_and(octet_length(fingerprint_hmac) = 32) AS "ok!" FROM anon_voters"#
    )
    .fetch_one(&pool)
    .await?;
    assert!(all_32_bytes);
    let voters = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM voters WHERE kind = 'anon'"#
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(voters, 2);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn expired_sessions_are_pruned(pool: PgPool) -> TestResult<()> {
    let account_id = sqlx::query_scalar!(
        r#"INSERT INTO accounts (provider, provider_subject, display_name, provider_created_at)
           VALUES ('discord'::oauth_provider, 'prune', 'Pruner', now())
           RETURNING id AS "id!""#
    )
    .fetch_one(&pool)
    .await?;
    sqlx::query!(
        "INSERT INTO sessions (token_hash, account_id, expires_at) \
         VALUES (decode(repeat('00', 32), 'hex'), $1, now() - interval '1 day')",
        account_id,
    )
    .execute(&pool)
    .await?;
    sqlx::query!(
        "INSERT INTO sessions (token_hash, account_id, expires_at) \
         VALUES (decode(repeat('11', 32), 'hex'), $1, now() + interval '1 day')",
        account_id,
    )
    .execute(&pool)
    .await?;

    assert_eq!(prune_expired_sessions(&pool).await?, 1);
    assert_eq!(count_sessions(&pool).await?, 1);
    Ok(())
}
