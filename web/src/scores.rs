use std::time::Duration;

use rankings_admin::error::RecomputeError;
use rankings_admin::recompute::{
    next_weight_change,
    refresh_pairwise,
    refresh_scores,
};
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use tokio::sync::watch;
use tokio::time::{Instant, sleep_until};
use tracing::warn;

const AGE_CROSSING_SLACK: Duration = Duration::from_secs(1);

pub type UntilWeightChange = Option<Duration>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshPolicy {
    pub quiet_period: Duration,
    pub max_delay: Duration,
}

pub const REFRESH_POLICY: RefreshPolicy = RefreshPolicy {
    quiet_period: Duration::from_secs(5),
    max_delay: Duration::from_mins(1),
};

#[derive(Debug)]
pub struct ScoreSignal(watch::Sender<u64>);

impl ScoreSignal {
    #[must_use]
    pub fn channel() -> (Self, watch::Receiver<u64>) {
        let (sender, receiver) = watch::channel(0);
        (Self(sender), receiver)
    }

    pub fn notify(&self) {
        self.0.send_modify(|count| *count = count.wrapping_add(1));
    }
}

pub trait ScoreStore: Send + Sync + 'static {
    fn refresh(
        &self,
    ) -> impl Future<Output = Result<UntilWeightChange, RecomputeError>> + Send;
}

pub struct PgScoreStore {
    pool: PgPool,
    cfg: ScoringConfig,
}

impl PgScoreStore {
    #[must_use]
    pub const fn new(pool: PgPool, cfg: ScoringConfig) -> Self {
        Self { pool, cfg }
    }
}

impl ScoreStore for PgScoreStore {
    async fn refresh(&self) -> Result<UntilWeightChange, RecomputeError> {
        refresh_scores(&self.pool, &self.cfg).await?;
        let pairwise = refresh_pairwise(&self.pool, &self.cfg).await?;
        if !pairwise.converged {
            warn!("Bradley–Terry fit did not converge");
        }
        let until = next_weight_change(&self.pool, &self.cfg).await?;
        Ok(until.map(|until| until.saturating_add(AGE_CROSSING_SLACK)))
    }
}

pub fn spawn_score_refresh(
    store: impl ScoreStore,
    policy: RefreshPolicy,
    changes: watch::Receiver<u64>,
) {
    tokio::spawn(run(store, policy, changes));
}

async fn run(
    store: impl ScoreStore,
    policy: RefreshPolicy,
    mut changes: watch::Receiver<u64>,
) {
    let mut until_weight_change = refresh(&store, policy).await;
    loop {
        let weight_change_at =
            until_weight_change.and_then(|until| Instant::now().checked_add(until));
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() {
                    return;
                }
                wait_for_quiet(&mut changes, policy).await;
            },
            () = sleep_until_or_forever(weight_change_at) => {},
        }
        until_weight_change = refresh(&store, policy).await;
    }
}

async fn wait_for_quiet(changes: &mut watch::Receiver<u64>, policy: RefreshPolicy) {
    let latest = Instant::now() + policy.max_delay;
    loop {
        let deadline = (Instant::now() + policy.quiet_period).min(latest);
        tokio::select! {
            changed = changes.changed() => {
                if changed.is_err() {
                    return;
                }
            },
            () = sleep_until(deadline) => return,
        }
    }
}

async fn refresh(
    store: &impl ScoreStore,
    policy: RefreshPolicy,
) -> UntilWeightChange {
    match store.refresh().await {
        Ok(until) => until,
        Err(e) => {
            warn!(error = %e, "could not refresh tracker scores");
            Some(policy.max_delay)
        },
    }
}

async fn sleep_until_or_forever(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}
