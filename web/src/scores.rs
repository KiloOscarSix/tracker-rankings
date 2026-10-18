use std::time::Duration;

use rankings_admin::recompute::refresh_scores;
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use tracing::warn;

const REFRESH_INTERVAL: Duration = Duration::from_mins(1);

pub fn spawn_score_refresh(pool: PgPool, cfg: ScoringConfig) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(REFRESH_INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(e) = refresh_scores(&pool, &cfg).await {
                warn!(error = %e, "could not refresh tracker scores");
            }
        }
    });
}
