use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rankings_admin::error::ImportError;
use rankings_admin::{import, seed};
use sqlx::PgPool;
use tracing::{error, info};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry, fmt};

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[arg(long, env = "DATABASE_URL", hide_env_values = true)]
    database_url: String,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Upsert the vendored trackerpathways seed. Never touches votes.
    Import {
        #[arg(long, default_value = "data/trackerpathways")]
        data_dir: PathBuf,
    },
}

#[tokio::main]
async fn main() -> ExitCode {
    let _ = dotenvy::dotenv();
    logging();

    match run(Cli::parse()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            error!("{e}");
            ExitCode::FAILURE
        },
    }
}

async fn run(cli: Cli) -> Result<(), ImportError> {
    let pool = PgPool::connect(&cli.database_url).await?;

    match cli.command {
        Command::Import { data_dir } => {
            let (seed, source) = seed::load(&data_dir)?;
            let report = import::import(&pool, &seed, &source).await?;
            info!(
                commit = %source.commit,
                listed = report.listed,
                stubs = report.stubs,
                routes = report.routes,
                stale_routes_removed = report.stale_routes_removed,
                "import complete"
            );
        },
    }
    Ok(())
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}
