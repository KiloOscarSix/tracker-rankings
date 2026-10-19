use rankings_admin::recompute::refresh_scores;
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::router::{Method, StatusCode};

mod common;

use common::{
    CLIENT,
    TestResult,
    app,
    app_observed,
    form_request,
    location,
    request,
    send,
    set_cookie,
    sign_in,
};

const AGENT: (&str, &str) = ("user-agent", "voting-test");
const ALL_FOURS: &str = "overall=4&content=4&community=4&retention=4&longevity=4";

#[sqlx::test(migrations = "../migrations")]
async fn anonymous_vote_carries_0_7_weight(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), true).await?;

    let reply = send(
        &router,
        form_request("/vote/aither/membership", "status=current", &[AGENT], CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/t/aither#vote"));
    let anon_id = set_cookie(&reply, "anon_id").ok_or("no anon_id cookie")?;
    let cookie = format!("anon_id={anon_id}");

    let reply = send(
        &router,
        form_request(
            "/vote/aither/ratings",
            ALL_FOURS,
            &[AGENT, ("cookie", &cookie)],
            CLIENT,
        )?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);

    refresh_scores(&pool, &ScoringConfig::default()).await?;

    let rows = sqlx::query!(
        "SELECT s.effective_weight, s.vote_count
         FROM tracker_scores s
         JOIN trackers t ON t.id = s.tracker_id
         WHERE t.slug = 'aither'"
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(rows.len(), 5);
    for row in rows {
        assert!((row.effective_weight - 0.7).abs() < 1e-9);
        assert_eq!(row.vote_count, 1);
    }
    let total =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM tracker_scores"#)
            .fetch_one(&pool)
            .await?;
    assert_eq!(total, 5);

    let page = send(
        &router,
        request(Method::GET, "/t/aither", &[AGENT, ("cookie", &cookie)], CLIENT)?,
    )
    .await?;
    assert_eq!(page.status, StatusCode::OK);
    let marker =
        page.body.find("id=\"rate-overall-4\"").ok_or("no overall-4 radio")?;
    let start = page
        .body
        .get(..marker)
        .and_then(|head| head.rfind("<input"))
        .ok_or("no input tag before the radio id")?;
    let end = page
        .body
        .get(marker..)
        .and_then(|tail| tail.find('>'))
        .map(|offset| marker + offset)
        .ok_or("unterminated input tag")?;
    let tag = page.body.get(start..end).ok_or("bad input slice")?;
    assert!(
        tag.split_whitespace()
            .any(|token| token == "checked" || token.starts_with("checked=")),
        "{tag}"
    );
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn re_voting_overwrites(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), true).await?;
    let session = sign_in(&router, "discord").await?;
    let cookie = format!("__Host-session={session}");
    let headers = [AGENT, ("cookie", cookie.as_str())];

    let reply = send(
        &router,
        form_request("/vote/aither/ratings", "overall=3", &headers, CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);

    for (path, form) in [
        ("/vote/aither/membership", "status=current"),
        ("/vote/aither/ratings", "overall=2"),
        ("/vote/aither/ratings", "overall=5&content=3"),
    ] {
        let reply =
            send(&router, form_request(path, form, &headers, CLIENT)?).await?;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path} {form}");
    }

    let rows = sqlx::query!(
        r#"SELECT axis::text AS "axis!", score FROM ratings ORDER BY axis"#
    )
    .fetch_all(&pool)
    .await?;
    let stored: Vec<(String, i16)> =
        rows.into_iter().map(|row| (row.axis, row.score)).collect();
    assert_eq!(stored, [("overall".to_owned(), 5), ("content".to_owned(), 3)]);

    refresh_scores(&pool, &ScoringConfig::default()).await?;
    let count = sqlx::query_scalar!(
        "SELECT s.vote_count
         FROM tracker_scores s
         JOIN trackers t ON t.id = s.tracker_id
         WHERE t.slug = 'aither' AND s.axis = 'overall'"
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 1);

    let stub = sqlx::query_scalar!(
        r#"SELECT slug AS "slug!" FROM trackers WHERE NOT listed ORDER BY slug LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;
    let uri = format!("/vote/{stub}/membership");
    let reply =
        send(&router, form_request(&uri, "status=current", &headers, CLIENT)?)
            .await?;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn deleting_my_data_cascades(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), true).await?;
    let session = sign_in(&router, "discord").await?;
    let cookie = format!("__Host-session={session}");
    let headers = [AGENT, ("cookie", cookie.as_str())];

    for (path, form) in [
        ("/vote/aither/membership", "status=current"),
        ("/vote/aither/ratings", "overall=3"),
    ] {
        let reply =
            send(&router, form_request(path, form, &headers, CLIENT)?).await?;
        assert_eq!(reply.status, StatusCode::SEE_OTHER, "{path} {form}");
    }
    sqlx::query!(
        "INSERT INTO pairwise_votes (voter_id, winner_id, loser_id)
         SELECT v.id, w.id, l.id
         FROM voters v, trackers w, trackers l
         WHERE v.kind = 'account' AND w.slug = 'aither' AND l.slug = 'anthelion'"
    )
    .execute(&pool)
    .await?;

    let reply =
        send(&router, form_request("/me/delete", "", &headers, CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/me?deleted=1"));
    assert_eq!(set_cookie(&reply, "__Host-session"), Some(String::new()));

    let left = sqlx::query!(
        r#"SELECT (SELECT count(*) FROM accounts) AS "accounts!",
                  (SELECT count(*) FROM sessions) AS "sessions!",
                  (SELECT count(*) FROM voters) AS "voters!",
                  (SELECT count(*) FROM memberships) AS "memberships!",
                  (SELECT count(*) FROM ratings) AS "ratings!",
                  (SELECT count(*) FROM pairwise_votes) AS "pairwise!""#
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(
        (
            left.accounts,
            left.sessions,
            left.voters,
            left.memberships,
            left.ratings,
            left.pairwise
        ),
        (0, 0, 0, 0, 0, 0)
    );
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn score_writes_signal_the_refresher(pool: PgPool) -> TestResult<()> {
    let (router, changes) = app_observed(pool, true).await?;
    let session = sign_in(&router, "discord").await?;
    let cookie = format!("__Host-session={session}");
    let headers = [AGENT, ("cookie", cookie.as_str())];

    // (path, form, expected status, expected signal count after the request)
    for (path, form, status, signals) in [
        ("/vote/aither/ratings", "overall=4", StatusCode::FORBIDDEN, 0),
        ("/vote/aither/membership", "status=current", StatusCode::SEE_OTHER, 0),
        ("/vote/aither/ratings", "overall=4", StatusCode::SEE_OTHER, 1),
        ("/vote/aither/membership", "status=former", StatusCode::SEE_OTHER, 1),
        ("/vote/aither/membership", "status=none", StatusCode::SEE_OTHER, 2),
        ("/me/delete", "", StatusCode::SEE_OTHER, 3),
    ] {
        let reply =
            send(&router, form_request(path, form, &headers, CLIENT)?).await?;
        assert_eq!(reply.status, status, "{path} {form}");
        assert_eq!(*changes.borrow(), signals, "{path} {form}");
    }
    Ok(())
}
