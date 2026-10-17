use sqlx::PgPool;
use topcoat::router::request::Request;
use topcoat::router::{Body, Method, Router, StatusCode};

#[expect(dead_code, reason = "each test binary uses a subset of the shared helpers")]
mod common;

use common::{TestResult, app, send};

async fn get(router: &Router, path: &str) -> TestResult<(StatusCode, String)> {
    let request =
        Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
    let reply = send(router, request).await?;
    Ok((reply.status, reply.body))
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
