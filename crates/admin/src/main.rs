use std::io;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use rankings_admin::error::AdminError;
use rankings_admin::recompute::{refresh_pairwise, refresh_scores};
use rankings_admin::void::void_voter;
use rankings_admin::{import, seed};
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use tracing::{error, info, warn};
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
    /// Recompute `tracker_scores` and `pairwise_scores` from the current votes.
    Recompute,
    /// Delete every rating, membership and pairwise vote of one voter, then
    /// recompute.
    VoidVoter { voter_id: i64 },
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

async fn run(cli: Cli) -> Result<(), AdminError> {
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
        Command::Recompute => recompute(&pool).await?,
        Command::VoidVoter { voter_id } => {
            let voided = void_voter(&pool, voter_id).await?;
            info!(
                voter_id,
                ratings = voided.ratings,
                memberships = voided.memberships,
                pairwise_votes = voided.pairwise_votes,
                "voter voided"
            );
            recompute(&pool).await?;
        },
    }
    Ok(())
}

async fn recompute(pool: &PgPool) -> Result<(), AdminError> {
    let cfg = ScoringConfig::default();

    let report = refresh_scores(pool, &cfg).await?;
    info!(ratings = report.ratings, scores = report.scores, "recompute complete");

    let report = refresh_pairwise(pool, &cfg).await?;
    info!(
        votes = report.votes,
        scores = report.scores,
        "pairwise recompute complete"
    );
    if !report.converged {
        warn!("Bradley–Terry fit did not converge");
    }
    Ok(())
}

fn logging() {
    let stdout_log =
        fmt::layer().with_writer(io::stdout).with_filter(LevelFilter::INFO);

    Registry::default().with(stdout_log).init();
}
