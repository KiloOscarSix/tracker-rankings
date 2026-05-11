use std::io;
use std::process::ExitCode;

use tokio::net::TcpListener;
use topcoat::asset::{AssetBundle, RouterBuilderAssetExt};
use topcoat::router::Router;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};
use web::error::StartupError;
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

    let addr = config::bind_addr()?;
    let assets = AssetBundle::load().map_err(StartupError::AssetBundle)?;
    let listener = TcpListener::bind(addr).await.map_err(StartupError::Listen)?;

    let base = Router::builder().assets(assets);

    info!("Listening on {addr}");
    topcoat::serve(listener, router(base)).await.map_err(StartupError::Serve)
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}
