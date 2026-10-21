use sqlx::PgPool;
use topcoat::router::request::Request;
use topcoat::router::{Body, Method, Router, StatusCode, header};

#[expect(dead_code, reason = "each test binary uses a subset of the shared helpers")]
mod common;

use common::{TestResult, app, send};

async fn get(router: &Router, path: &str) -> TestResult<(StatusCode, String)> {
    let request =
        Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
    let reply = send(router, request).await?;
    Ok((reply.status, reply.body))
}

fn tag_with<'a>(body: &'a str, marker: &str) -> TestResult<&'a str> {
    let at = body.find(marker).ok_or("marker not found")?;
    let start = body
        .get(..at)
        .and_then(|head| head.rfind("<tr"))
        .ok_or("no <tr before the marker")?;
    let end = body
        .get(at..)
        .and_then(|tail| tail.find('>'))
        .map(|offset| at + offset)
        .ok_or("unterminated tag")?;
    Ok(body.get(start..end).ok_or("bad tag slice")?)
}

fn is_hidden(tag: &str) -> bool {
    tag.split_whitespace()
        .any(|token| token == "hidden" || token.starts_with("hidden="))
}

#[sqlx::test(migrations = "../migrations")]
async fn aither_lists_anthelion_as_a_next_step_with_prometheus_plus(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool, true).await?;
    let (status, html) = get(&router, "/t/aither").await?;
    assert_eq!(status, StatusCode::OK);

    let start = html.find("id=\"next-steps\"").ok_or("no next-steps section")?;
    let end = html.find("id=\"incoming\"").ok_or("no incoming section")?;
    let next = html.get(start..end).ok_or("sections out of order")?;
    let row = next
        .split("<tr")
        .find(|row| row.contains(">Anthelion<"))
        .ok_or("no Anthelion row in next steps")?;
    assert!(row.contains("Prometheus+"));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn an_unknown_slug_is_a_404(pool: PgPool) -> TestResult<()> {
    let router = app(pool, true).await?;
    let (status, html) = get(&router, "/t/no-such-tracker").await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.contains("We couldn't find that page."));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn rankings_filter_by_category_and_show_every_tracker_unranked(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool, true).await?;

    let (status, html) = get(&router, "/?category=music").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(">Orpheus<"));
    assert!(!html.contains(">Aither<"));
    assert!(html.contains("Unranked"));

    let (status, _) = get(&router, "/?sort=bogus").await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn rankings_search_hides_rows_that_do_not_match(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool, true).await?;

    let (status, html) = get(&router, "/?q=ANIME").await?;
    assert_eq!(status, StatusCode::OK);
    let matching = tag_with(&html, "id=\"tracker-animebytes\"")?;
    assert!(!is_hidden(matching), "{matching}");
    let other = tag_with(&html, "id=\"tracker-orpheus\"")?;
    assert!(is_hidden(other), "{other}");
    assert!(html.contains("href=\"/?category=music&amp;sort=overall&amp;q=ANIME\""));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn vote_page_renders_the_panel_and_404s_for_stubs(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool.clone(), true).await?;

    let (status, html) = get(&router, "/t/aither/vote?sort=content&q=ait").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains("data-vote-panel"));
    assert!(html.contains("action=\"/vote/aither\""));
    assert!(html.contains("name=\"sort\" value=\"content\""));
    assert!(html.contains("value=\"ait\""));

    let (status, html) = get(&router, "/?q=").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("data-vote-panel"));

    let stub = sqlx::query_scalar!(
        r#"SELECT slug AS "slug!" FROM trackers WHERE NOT listed ORDER BY slug LIMIT 1"#
    )
    .fetch_one(&pool)
    .await?;
    let (status, _) = get(&router, &format!("/t/{stub}/vote")).await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn pages_revalidate_while_hashed_assets_stay_immutable(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool, true).await?;
    for path in ["/", "/t/aither", "/t/no-such-tracker"] {
        let request =
            Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
        let reply = send(&router, request).await?;
        assert_eq!(reply.headers.get_all(header::CACHE_CONTROL).iter().count(), 1);
        assert_eq!(
            reply.headers[header::CACHE_CONTROL],
            "private, no-cache",
            "{path}"
        );
    }

    let request =
        Request::builder().method(Method::GET).uri("/").body(Body::empty())?;
    let html = send(&router, request).await?.body;
    let stylesheet = html
        .split('"')
        .find(|part| part.contains("tailwind-"))
        .ok_or("no stylesheet link")?;
    let request =
        Request::builder().method(Method::GET).uri(stylesheet).body(Body::empty())?;
    let reply = send(&router, request).await?;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(
        reply.headers[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    Ok(())
}
