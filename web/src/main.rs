use std::io;
use std::process::ExitCode;
use std::sync::Arc;

use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use tokio::net::TcpListener;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::{OriginPolicy, Router, TrustedProxies};
use tracing::{error, info, warn};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};
use web::auth::discord::DiscordOAuth;
use web::auth::oauth::{OAuthClient, OAuthClients};
use web::auth::reddit::RedditOAuth;
use web::auth::sessions::spawn_session_pruning;
use web::error::StartupError;
use web::rate_limit::{
    AUTH_QUOTA,
    RateLimiters,
    VOTE_QUOTA,
    spawn_rate_limit_cleanup,
};
use web::scores::spawn_score_refresh;
use web::{config, router};

#[tokio::main]
async fn main() -> ExitCode {
    logging();

    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        },
    }
}

async fn run() -> Result<(), StartupError> {
    let _ = dotenvy::dotenv();

    if rustls::crypto::aws_lc_rs::default_provider().install_default().is_err() {
        warn!("a rustls crypto provider was already installed");
    }
    let auth = config::auth_config()?;

    let addr = config::bind_addr()?;
    let pool = PgPool::connect(&config::database_url()?).await?;
    let assets = AssetBundle::load().map_err(StartupError::AssetBundle)?;
    let listener = TcpListener::bind(addr).await.map_err(StartupError::Listen)?;

    let mut clients: Vec<Box<dyn OAuthClient>> = Vec::new();
    if let Some(credentials) = &auth.discord {
        clients
            .push(Box::new(DiscordOAuth::new(credentials, &auth.public_base_url)?));
    }
    if let Some(reddit) = &auth.reddit {
        clients.push(Box::new(RedditOAuth::new(reddit, &auth.public_base_url)?));
    }
    if clients.is_empty() {
        warn!("no OAuth provider is configured, so sign-in is disabled");
    }

    let scoring = ScoringConfig::default();
    let limiters = Arc::new(RateLimiters::new(AUTH_QUOTA, VOTE_QUOTA));
    spawn_session_pruning(pool.clone());
    spawn_score_refresh(pool.clone(), scoring.clone());
    spawn_rate_limit_cleanup(Arc::clone(&limiters));

    let mut base = Router::builder()
        .app_context(pool)
        .app_context(scoring)
        .app_context(OAuthClients::new(clients))
        .app_context(auth.cookie_key)
        .app_context(limiters)
        .origin_policy(
            OriginPolicy::new().trust_origins([auth.public_base_url.clone()]),
        )
        .assets(assets);
    if !auth.trusted_proxies.is_empty() {
        base = base
            .trusted_proxies(TrustedProxies::new().networks(auth.trusted_proxies));
    }

    info!("Listening on {addr}");
    topcoat::serve(listener, router(base)).await.map_err(StartupError::Serve)
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}
