use std::error::Error;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use governor::Quota;
use http_body_util::BodyExt;
use rankings_admin::import::import;
use rankings_admin::seed;
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::request::Request;
use topcoat::router::{
    Body,
    HeaderMap,
    Method,
    RemoteAddr,
    Router,
    RouterBuilder,
    StatusCode,
    header,
};
use web::auth::oauth::{
    ExchangeFuture,
    OAuthClient,
    OAuthClients,
    ProviderIdentity,
};
use web::auth::provider::Provider;
use web::config::CookieKey;
use web::document::STYLESHEET;
use web::error::AuthError;
use web::rate_limit::{AUTH_QUOTA, RateLimiters, VOTE_QUOTA};
use web::router;

const STYLESHEET_FILE: &str = "tailwind-0123456789abcdef.css";
const STYLESHEET_BODY: &str = ".page{display:block}";
const RUNTIME_FILE: &str = "topcoat-0123456789abcdef.js";
const RUNTIME_BODY: &str = "export {};";

pub(crate) const GOOD_CODE: &str = "good-code";
pub(crate) const CREATED_AT: f64 = 1_600_000_000.0;
pub(crate) const CLIENT: &str = "203.0.113.5:4000";

pub(crate) type TestResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

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

pub(crate) struct FakeOAuth {
    pub(crate) provider: Provider,
    pub(crate) subject: &'static str,
    pub(crate) display_name: &'static str,
}

impl OAuthClient for FakeOAuth {
    fn provider(&self) -> Provider {
        self.provider
    }

    fn authorize_url(&self, state: &str) -> String {
        format!("https://fake.test/{}/authorize?state={state}", self.provider.key())
    }

    fn exchange<'a>(&'a self, code: &'a str) -> ExchangeFuture<'a> {
        Box::pin(async move {
            if code == GOOD_CODE {
                Ok(ProviderIdentity {
                    subject: self.subject.to_owned(),
                    display_name: self.display_name.to_owned(),
                    created_at_unix: CREATED_AT,
                })
            } else {
                Err(AuthError::TokenExchange("bad code".to_owned()))
            }
        })
    }
}

pub(crate) fn cookie_key() -> TestResult<CookieKey> {
    Ok(CookieKey::new(vec![7; 64])?)
}

pub(crate) async fn app(pool: PgPool, seed: bool) -> TestResult<Router> {
    app_with(pool, seed, AUTH_QUOTA, |base| base).await
}

pub(crate) async fn app_with(
    pool: PgPool,
    seed: bool,
    auth_quota: Quota,
    extra: impl FnOnce(RouterBuilder) -> RouterBuilder,
) -> TestResult<Router> {
    if seed {
        let dir =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/trackerpathways");
        let (data, source) = seed::load(&dir)?;
        import(&pool, &data, &source).await?;
    }

    let clients: Vec<Box<dyn OAuthClient>> = vec![
        Box::new(FakeOAuth {
            provider: Provider::Discord,
            subject: "1001",
            display_name: "Discord Tester",
        }),
        Box::new(FakeOAuth {
            provider: Provider::Reddit,
            subject: "t2_abc",
            display_name: "reddit_tester",
        }),
    ];

    let bundle = AssetBundle::load_dir(bundle_dir()?)?;
    let base = Router::builder()
        .assets(bundle)
        .app_context(pool)
        .app_context(ScoringConfig::default())
        .app_context(OAuthClients::new(clients))
        .app_context(cookie_key()?)
        .app_context(Arc::new(RateLimiters::new(auth_quota, VOTE_QUOTA)));
    Ok(router(extra(base)))
}

pub(crate) struct Reply {
    pub(crate) status: StatusCode,
    pub(crate) headers: HeaderMap,
    pub(crate) body: String,
}

pub(crate) async fn send(router: &Router, request: Request) -> TestResult<Reply> {
    let response = router.handle(request).await;
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = response.into_body().collect().await?.to_bytes();
    Ok(Reply { status, headers, body: String::from_utf8(bytes.to_vec())? })
}

/// Value of the named cookie from the response's `Set-Cookie` headers.
pub(crate) fn set_cookie(reply: &Reply, name: &str) -> Option<String> {
    reply
        .headers
        .get_all(header::SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(|line| {
            let pair = line.split(';').next()?;
            let (cookie_name, value) = pair.split_once('=')?;
            (cookie_name.trim() == name).then(|| value.to_owned())
        })
}

pub(crate) fn location(reply: &Reply) -> Option<&str> {
    reply.headers.get(header::LOCATION).and_then(|value| value.to_str().ok())
}

pub(crate) fn request(
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

pub(crate) fn form_request(
    uri: &str,
    form: &str,
    headers: &[(&str, &str)],
    peer: &str,
) -> TestResult<Request> {
    let mut builder = Request::builder()
        .method(Method::POST)
        .uri(uri)
        .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
        .extension(RemoteAddr(peer.parse::<SocketAddr>()?));
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    Ok(builder.body(Body::from(form.to_owned()))?)
}

pub(crate) async fn begin(router: &Router, provider: &str) -> TestResult<String> {
    let uri = format!("/auth/{provider}");
    let reply = send(router, request(Method::POST, &uri, &[], CLIENT)?).await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    Ok(set_cookie(&reply, "oauth_state").ok_or("no oauth_state cookie")?)
}

pub(crate) async fn sign_in(router: &Router, provider: &str) -> TestResult<String> {
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
