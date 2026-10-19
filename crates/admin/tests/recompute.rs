use std::path::Path;
use std::time::Duration;

use rankings_admin::error::{ImportError, VoidError};
use rankings_admin::import::import;
use rankings_admin::recompute::{RefreshReport, next_weight_change, refresh_scores};
use rankings_admin::seed::{self, Seed, Source};
use rankings_admin::void::{VoidReport, void_voter};
use rankings_core::config::ScoringConfig;
use sqlx::PgPool;

fn vendored() -> Result<(Seed, Source), ImportError> {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trackerpathways");
    seed::load(&dir)
}

async fn anon_voter(pool: &PgPool, byte: i32) -> Result<i64, sqlx::Error> {
    let anon_id = sqlx::query_scalar!(
        "INSERT INTO anon_voters (fingerprint_hmac)
         VALUES (decode(repeat(lpad(to_hex($1::integer), 2, '0'), 32), 'hex'))
         RETURNING id",
        byte
    )
    .fetch_one(pool)
    .await?;
    sqlx::query_scalar!(
        "INSERT INTO voters (kind, anon_id) VALUES ('anon', $1) RETURNING id",
        anon_id
    )
    .fetch_one(pool)
    .await
}

async fn account_voter(
    pool: &PgPool,
    subject: &str,
    age_days: i32,
) -> Result<i64, sqlx::Error> {
    let account_id = sqlx::query_scalar!(
        "INSERT INTO accounts (
             provider, provider_subject, display_name, provider_created_at
         )
         VALUES ('discord', $1, $1, now() - make_interval(days => $2))
         RETURNING id",
        subject,
        age_days
    )
    .fetch_one(pool)
    .await?;
    sqlx::query_scalar!(
        "INSERT INTO voters (kind, account_id) VALUES ('account', $1) RETURNING id",
        account_id
    )
    .fetch_one(pool)
    .await
}

async fn rate(
    pool: &PgPool,
    voter: i64,
    slug: &str,
    axis: &str,
    score: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO memberships (voter_id, tracker_id, status)
         SELECT $1, id, 'current' FROM trackers WHERE slug = $2
         ON CONFLICT DO NOTHING",
        voter,
        slug
    )
    .execute(pool)
    .await?;
    sqlx::query!(
        "INSERT INTO ratings (voter_id, tracker_id, axis, score)
         SELECT $1, id, $3::text::rating_axis, $4 FROM trackers WHERE slug = $2",
        voter,
        slug,
        axis,
        score
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn score_rows(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT count(*) AS "n!" FROM tracker_scores"#)
        .fetch_one(pool)
        .await
}

#[sqlx::test(migrations = "../../migrations")]
async fn refresh_weights_each_voter_class(pool: PgPool) {
    let (seed, source) = vendored().unwrap();
    import(&pool, &seed, &source).await.unwrap();

    let anon = anon_voter(&pool, 1).await.unwrap();
    let young = account_voter(&pool, "young", 10).await.unwrap();
    let old = account_voter(&pool, "old", 400).await.unwrap();
    rate(&pool, anon, "aither", "overall", 5).await.unwrap();
    rate(&pool, young, "aither", "overall", 4).await.unwrap();
    rate(&pool, old, "aither", "overall", 3).await.unwrap();

    let report = refresh_scores(&pool, &ScoringConfig::default()).await.unwrap();
    assert_eq!(report, RefreshReport { ratings: 3, scores: 1 });

    let row = sqlx::query!(
        "SELECT s.effective_weight, s.vote_count FROM tracker_scores s
         JOIN trackers t ON t.id = s.tracker_id
         WHERE t.slug = 'aither' AND s.axis = 'overall'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!((row.effective_weight - 2.5).abs() < 1e-9, "{}", row.effective_weight);
    assert_eq!(row.vote_count, 3);
    assert_eq!(score_rows(&pool).await.unwrap(), 1);
}

#[sqlx::test(migrations = "../../migrations")]
async fn void_voter_removes_votes_and_refresh_drops_them(pool: PgPool) {
    let (seed, source) = vendored().unwrap();
    import(&pool, &seed, &source).await.unwrap();

    let voter = account_voter(&pool, "old", 400).await.unwrap();
    rate(&pool, voter, "aither", "overall", 4).await.unwrap();
    rate(&pool, voter, "aither", "content", 3).await.unwrap();
    sqlx::query!(
        "INSERT INTO pairwise_votes (voter_id, winner_id, loser_id)
         SELECT $1, w.id, l.id FROM trackers w, trackers l
         WHERE w.slug = 'aither' AND l.slug = 'anthelion'",
        voter
    )
    .execute(&pool)
    .await
    .unwrap();

    let cfg = ScoringConfig::default();
    assert_eq!(refresh_scores(&pool, &cfg).await.unwrap().scores, 2);

    let report = void_voter(&pool, voter).await.unwrap();
    assert_eq!(report, VoidReport { ratings: 2, memberships: 1, pairwise_votes: 1 });

    assert_eq!(refresh_scores(&pool, &cfg).await.unwrap().scores, 0);
    assert_eq!(score_rows(&pool).await.unwrap(), 0);

    assert!(matches!(
        void_voter(&pool, i64::MAX).await,
        Err(VoidError::UnknownVoter(id)) if id == i64::MAX
    ));
}

#[sqlx::test(migrations = "../../migrations")]
async fn next_weight_change_finds_the_oldest_young_rater(pool: PgPool) {
    let (seed, source) = vendored().unwrap();
    import(&pool, &seed, &source).await.unwrap();
    let cfg = ScoringConfig::default();

    assert_eq!(next_weight_change(&pool, &cfg).await.unwrap(), None);

    let young10 = account_voter(&pool, "young10", 10).await.unwrap();
    rate(&pool, young10, "aither", "overall", 4).await.unwrap();
    let young20 = account_voter(&pool, "young20", 20).await.unwrap();
    rate(&pool, young20, "aither", "content", 4).await.unwrap();
    account_voter(&pool, "unrated", 25).await.unwrap();
    let old = account_voter(&pool, "old", 400).await.unwrap();
    rate(&pool, old, "aither", "overall", 3).await.unwrap();
    let anon = anon_voter(&pool, 1).await.unwrap();
    rate(&pool, anon, "aither", "overall", 5).await.unwrap();

    let wait = next_weight_change(&pool, &cfg).await.unwrap().unwrap();
    let expected = Duration::from_hours(10 * 24);
    assert!(wait.abs_diff(expected) < Duration::from_secs(60), "{wait:?}");
}
