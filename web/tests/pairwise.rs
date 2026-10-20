use rankings_admin::recompute::refresh_pairwise;
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::router::{Method, StatusCode};

#[expect(dead_code, reason = "each test binary uses a subset of the shared helpers")]
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
};

const AGENT: (&str, &str) = ("user-agent", "pairwise-test");

#[sqlx::test(migrations = "../migrations")]
async fn vote_is_recorded_and_ranked(pool: PgPool) -> TestResult<()> {
    let router = app(pool.clone(), true).await?;

    let page = send(
        &router,
        request(Method::GET, "/compare?category=movies-tv", &[AGENT], CLIENT)?,
    )
    .await?;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("action=\"/vote/compare\""));
    assert!(page.body.contains("name=\"winner\""));

    let reply = send(
        &router,
        form_request(
            "/vote/compare",
            "a=aither&b=anthelion&winner=aither&category=movies-tv",
            &[AGENT],
            CLIENT,
        )?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/compare?category=movies-tv#matchup"));
    let anon_id = set_cookie(&reply, "anon_id").ok_or("no anon_id cookie")?;
    let cookie = format!("anon_id={anon_id}");

    let reply = send(
        &router,
        form_request(
            "/vote/compare",
            "a=aither&b=anthelion&winner=anthelion",
            &[AGENT, ("cookie", &cookie)],
            CLIENT,
        )?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply), Some("/compare#matchup"));

    let votes = sqlx::query!(
        r#"SELECT w.slug AS "winner!", l.slug AS "loser!"
           FROM pairwise_votes p
           JOIN trackers w ON w.id = p.winner_id
           JOIN trackers l ON l.id = p.loser_id"#
    )
    .fetch_all(&pool)
    .await?;
    assert_eq!(votes.len(), 1);
    let vote = votes.first().ok_or("no pairwise vote")?;
    assert_eq!((vote.winner.as_str(), vote.loser.as_str()), ("anthelion", "aither"));

    let report = refresh_pairwise(&pool, &ScoringConfig::default()).await?;
    assert_eq!(report.votes, 1);
    assert_eq!(report.scores, 2);
    assert!(report.converged);

    let strengths = sqlx::query!(
        "SELECT t.slug, s.strength
         FROM pairwise_scores s
         JOIN trackers t ON t.id = s.tracker_id
         ORDER BY s.strength DESC"
    )
    .fetch_all(&pool)
    .await?;
    let [first, second] = strengths.as_slice() else {
        return Err("expected exactly two strengths".into());
    };
    assert_eq!((first.slug.as_str(), second.slug.as_str()), ("anthelion", "aither"));

    let page =
        send(&router, request(Method::GET, "/compare", &[AGENT], CLIENT)?).await?;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains(&format!("{:.2}", first.strength)));
    assert!(page.body.contains(&format!("{:.2}", second.strength)));
    let ranking = page
        .body
        .find("id=\"ranking\"")
        .and_then(|start| page.body.get(start..))
        .ok_or("no ranking section")?;
    let anthelion = ranking.find(">Anthelion<").ok_or("Anthelion not ranked")?;
    let aither = ranking.find(">Aither<").ok_or("Aither not ranked")?;
    assert!(anthelion < aither);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn matchups_come_from_memberships(pool: PgPool) -> TestResult<()> {
    let router = app(pool, true).await?;

    let reply = send(
        &router,
        form_request("/vote/aither/membership", "status=current", &[AGENT], CLIENT)?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    let anon_id = set_cookie(&reply, "anon_id").ok_or("no anon_id cookie")?;
    let cookie = format!("anon_id={anon_id}");
    let headers = [AGENT, ("cookie", cookie.as_str())];
    let reply = send(
        &router,
        form_request(
            "/vote/redacted/membership",
            "status=current",
            &headers,
            CLIENT,
        )?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);

    let page =
        send(&router, request(Method::GET, "/compare", &headers, CLIENT)?).await?;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.body.contains("action=\"/vote/compare\""));
    assert!(page.body.contains("value=\"aither\""));
    assert!(page.body.contains("value=\"redacted\""));

    let page =
        send(&router, request(Method::GET, "/compare", &[AGENT], CLIENT)?).await?;
    assert_eq!(page.status, StatusCode::OK);
    assert!(!page.body.contains("action=\"/vote/compare\""));
    assert!(page.body.contains("Declare membership"));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn ineligible_pairs_are_refused(pool: PgPool) -> TestResult<()> {
    let (router, changes) = app_observed(pool.clone(), true).await?;
    let stub = sqlx::query_scalar!(
        r#"SELECT slug AS "slug!" FROM trackers WHERE NOT listed ORDER BY slug LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;

    let refused = [
        ("a=aither&b=redacted&winner=aither".to_owned(), StatusCode::FORBIDDEN),
        ("a=aither&b=aither&winner=aither".to_owned(), StatusCode::BAD_REQUEST),
        ("a=aither&b=anthelion&winner=redacted".to_owned(), StatusCode::BAD_REQUEST),
        (
            "a=aither&b=anthelion&winner=aither&category=nope".to_owned(),
            StatusCode::BAD_REQUEST,
        ),
        (format!("a=aither&b={stub}&winner=aither"), StatusCode::NOT_FOUND),
    ];
    for (form, status) in refused {
        let reply =
            send(&router, form_request("/vote/compare", &form, &[AGENT], CLIENT)?)
                .await?;
        assert_eq!(reply.status, status, "{form}");
        assert_eq!(*changes.borrow(), 0, "{form}");
    }
    let stored =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM pairwise_votes"#)
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored, 0);

    let reply = send(
        &router,
        form_request("/vote/aither/membership", "status=current", &[AGENT], CLIENT)?,
    )
    .await?;
    let anon_id = set_cookie(&reply, "anon_id").ok_or("no anon_id cookie")?;
    let cookie = format!("anon_id={anon_id}");
    let headers = [AGENT, ("cookie", cookie.as_str())];
    send(
        &router,
        form_request(
            "/vote/redacted/membership",
            "status=current",
            &headers,
            CLIENT,
        )?,
    )
    .await?;

    let reply = send(
        &router,
        form_request(
            "/vote/compare",
            "a=aither&b=redacted&winner=redacted",
            &headers,
            CLIENT,
        )?,
    )
    .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(*changes.borrow(), 1);
    let stored =
        sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM pairwise_votes"#)
            .fetch_one(&pool)
            .await?;
    assert_eq!(stored, 1);
    Ok(())
}
