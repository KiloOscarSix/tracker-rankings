use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use rankings_admin::error::RecomputeError;
use rankings_admin::import::import;
use rankings_admin::seed;
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use tokio::sync::watch;
use tokio::time::sleep;
use web::scores::{
    PgScoreStore,
    REFRESH_POLICY,
    RefreshPolicy,
    ScoreSignal,
    ScoreStore,
    spawn_score_refresh,
};

const POLICY: RefreshPolicy = RefreshPolicy {
    quiet_period: Duration::from_secs(5),
    max_delay: Duration::from_mins(1),
};

/// Counts refreshes; the first one reports `first_wake`, later ones none.
struct FakeStore {
    calls: Arc<AtomicU32>,
    first_wake: Option<Duration>,
}

impl ScoreStore for FakeStore {
    fn refresh(
        &self,
    ) -> impl Future<Output = Result<Option<Duration>, RecomputeError>> + Send {
        let before = self.calls.fetch_add(1, Ordering::SeqCst);
        std::future::ready(Ok(if before == 0 { self.first_wake } else { None }))
    }
}

fn start(first_wake: Option<Duration>) -> (ScoreSignal, Arc<AtomicU32>) {
    let (signal, changes) = ScoreSignal::channel();
    let counter = Arc::new(AtomicU32::new(0));
    let store = FakeStore { calls: Arc::clone(&counter), first_wake };
    spawn_score_refresh(store, POLICY, changes);
    (signal, counter)
}

fn calls(counter: &AtomicU32) -> u32 {
    counter.load(Ordering::SeqCst)
}

#[tokio::test(start_paused = true)]
async fn idle_refresher_only_recomputes_at_startup() {
    let (_signal, counter) = start(None);
    sleep(Duration::from_millis(1)).await;
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_hours(24)).await;
    assert_eq!(calls(&counter), 1);
}

#[tokio::test(start_paused = true)]
async fn burst_recomputes_once_after_rest() {
    let (signal, counter) = start(None);
    sleep(Duration::from_millis(1)).await;
    assert_eq!(calls(&counter), 1);

    for _ in 0..20 {
        signal.notify();
        sleep(Duration::from_secs(1)).await;
    }
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_secs(3)).await;
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_secs(2)).await;
    assert_eq!(calls(&counter), 2);

    sleep(Duration::from_hours(24)).await;
    assert_eq!(calls(&counter), 2);
}

#[tokio::test(start_paused = true)]
async fn steady_stream_is_capped_by_max_delay() {
    let (signal, counter) = start(None);
    sleep(Duration::from_millis(1)).await;
    assert_eq!(calls(&counter), 1);

    signal.notify();
    for _ in 0..29 {
        sleep(Duration::from_secs(2)).await;
        assert_eq!(calls(&counter), 1);
        signal.notify();
    }

    sleep(Duration::from_millis(1_500)).await;
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_secs(1)).await;
    assert_eq!(calls(&counter), 2);
}

#[tokio::test(start_paused = true)]
async fn age_crossing_wakes_without_a_signal() {
    let (_signal, counter) = start(Some(Duration::from_secs(10)));
    sleep(Duration::from_millis(1)).await;
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_secs(9)).await;
    assert_eq!(calls(&counter), 1);

    sleep(Duration::from_secs(2)).await;
    assert_eq!(calls(&counter), 2);

    sleep(Duration::from_hours(24)).await;
    assert_eq!(calls(&counter), 2);
}

#[sqlx::test(migrations = "../migrations")]
async fn young_vote_reaches_full_weight_without_a_write(pool: PgPool) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../data/trackerpathways");
    let (data, source) = seed::load(&dir).unwrap();
    import(&pool, &data, &source).await.unwrap();

    let account = sqlx::query_scalar!(
        "INSERT INTO accounts (
             provider, provider_subject, display_name, provider_created_at
         )
         VALUES (
             'discord', 'young', 'young',
             now() - interval '30 days' + interval '2 seconds'
         )
         RETURNING id"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let voter = sqlx::query_scalar!(
        "INSERT INTO voters (kind, account_id) VALUES ('account', $1) RETURNING id",
        account
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO memberships (voter_id, tracker_id, status)
         SELECT $1, id, 'current' FROM trackers WHERE slug = 'aither'",
        voter
    )
    .execute(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "INSERT INTO ratings (voter_id, tracker_id, axis, score)
         SELECT $1, id, 'overall', 4 FROM trackers WHERE slug = 'aither'",
        voter
    )
    .execute(&pool)
    .await
    .unwrap();

    let (_signal, changes) = ScoreSignal::channel();
    let (done, mut refreshes) = watch::channel(Vec::new());
    let inner = PgScoreStore::new(pool.clone(), ScoringConfig::default());
    spawn_score_refresh(ReportingStore { inner, done }, REFRESH_POLICY, changes);

    let wake = refreshes.wait_for(|wakes| wakes.len() == 1).await.unwrap()[0];
    assert!(wake.is_some_and(|wait| wait <= Duration::from_secs(3)), "{wake:?}");
    let weight = overall_weight(&pool).await.unwrap();
    assert!((weight - 0.8).abs() < 1e-9, "{weight}");

    let wake = refreshes.wait_for(|wakes| wakes.len() == 2).await.unwrap()[1];
    assert_eq!(wake, None);
    let weight = overall_weight(&pool).await.unwrap();
    assert!((weight - 1.0).abs() < 1e-9, "{weight}");
}

/// Records each finished refresh's wake, so the test waits on refreshes.
struct ReportingStore {
    inner: PgScoreStore,
    done: watch::Sender<Vec<Option<Duration>>>,
}

impl ScoreStore for ReportingStore {
    async fn refresh(&self) -> Result<Option<Duration>, RecomputeError> {
        let wake = self.inner.refresh().await?;
        self.done.send_modify(|wakes| wakes.push(wake));
        Ok(wake)
    }
}

async fn overall_weight(pool: &PgPool) -> Result<f64, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT s.effective_weight
         FROM tracker_scores s
         JOIN trackers t ON t.id = s.tracker_id
         WHERE t.slug = 'aither' AND s.axis = 'overall'"
    )
    .fetch_one(pool)
    .await
}
