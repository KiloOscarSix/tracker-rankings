use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use http_body_util::BodyExt;
use ladder_admin::import::import;
use ladder_admin::seed;
use ladder_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::{Body, Method, Router, StatusCode};
use web::document::STYLESHEET;
use web::router;

const STYLESHEET_FILE: &str = "tailwind-0123456789abcdef.css";
const STYLESHEET_BODY: &str = ".page{display:block}";
const RUNTIME_FILE: &str = "topcoat-0123456789abcdef.js";
const RUNTIME_BODY: &str = "export {};";

type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

/// A bundle directory laid out like `topcoat asset bundle` output, holding
/// stand-ins for the stylesheet and the browser runtime. Written once per
/// test process, since tests run concurrently.
fn bundle_dir() -> TestResult<PathBuf> {
    static DIR: OnceLock<Result<PathBuf, String>> = OnceLock::new();
    DIR.get_or_init(|| write_bundle().map_err(|e| e.to_string()))
        .clone()
        .map_err(Into::into)
}

fn write_bundle() -> TestResult<PathBuf> {
    let dir = std::env::temp_dir()
        .join(format!("web-pages-assets-{}", std::process::id()));
    std::fs::create_dir_all(&dir)?;
    std::fs::write(dir.join(STYLESHEET_FILE), STYLESHEET_BODY)?;
    std::fs::write(dir.join(RUNTIME_FILE), RUNTIME_BODY)?;
    std::fs::write(
        dir.join("manifest.toml"),
        format!(
            "version = 1\n\n\
             [[assets]]\nid = {}\nfile = \"{STYLESHEET_FILE}\"\nhash = \"0\"\ncontent_type = \"text/css\"\n\n\
             [[assets]]\nid = {}\nfile = \"{RUNTIME_FILE}\"\nhash = \"0\"\ncontent_type = \"text/javascript\"\n",
            STYLESHEET.id().as_u64(),
            topcoat::runtime::SCRIPT.id().as_u64(),
        ),
    )?;
    Ok(dir)
}

async fn app(pool: PgPool) -> TestResult<Router> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/trackerpathways");
    let (seed, source) = seed::load(&dir)?;
    import(&pool, &seed, &source).await?;

    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    Ok(router(
        Router::builder()
            .assets(bundle)
            .app_context(pool)
            .app_context(ScoringConfig::default()),
    ))
}

async fn get(router: &Router, path: &str) -> TestResult<(StatusCode, String)> {
    let request =
        Request::builder().method(Method::GET).uri(path).body(Body::empty())?;
    let response = router.handle(request).await;
    let status = response.status();
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok((status, String::from_utf8(bytes.to_vec())?))
}

#[sqlx::test(migrations = "../migrations")]
async fn aither_lists_anthelion_as_a_next_step_with_prometheus_plus(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool).await?;
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
    let router = app(pool).await?;
    let (status, html) = get(&router, "/t/no-such-tracker").await?;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert!(html.contains("We couldn't find that page."));
    Ok(())
}

#[sqlx::test(migrations = "../migrations")]
async fn rankings_filter_by_category_and_show_every_tracker_unranked(
    pool: PgPool,
) -> TestResult<()> {
    let router = app(pool).await?;

    let (status, html) = get(&router, "/?category=music").await?;
    assert_eq!(status, StatusCode::OK);
    assert!(html.contains(">Orpheus<"));
    assert!(!html.contains(">Aither<"));
    assert!(html.contains("Unranked"));

    let (status, _) = get(&router, "/?sort=bogus").await?;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    Ok(())
}
