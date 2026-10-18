use rankings_core::aggregate::{AxisRating, aggregate};
use rankings_core::axis::Axis;
use rankings_core::config::ScoringConfig;
use rankings_core::score::{Rating, Score};
use rankings_core::weight::Voter;
use sqlx::PgPool;

use crate::error::RecomputeError;

/// Serialises refreshes from the web task and the CLI.
const REFRESH_LOCK_KEY: i64 = 0x7472_6163_6b72_616e;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RefreshReport {
    pub ratings: u64,
    pub scores: u64,
}

/// Rewrites `tracker_scores` from every rating on a listed tracker, weighting each
/// vote by its voter's class as of now
pub async fn refresh_scores(
    pool: &PgPool,
    cfg: &ScoringConfig,
) -> Result<RefreshReport, RecomputeError> {
    let rows = sqlx::query!(
        r#"SELECT r.tracker_id, r.axis::text AS "axis!", r.score,
                  v.kind::text AS "kind!",
                  floor(
                      extract(epoch FROM now() - a.provider_created_at) / 86400
                  )::integer AS "age_days?"
           FROM ratings r
           JOIN voters v ON v.id = r.voter_id
           JOIN trackers t ON t.id = r.tracker_id AND t.listed
           LEFT JOIN accounts a ON a.id = v.account_id"#
    )
    .fetch_all(pool)
    .await?;

    let mut ratings = Vec::with_capacity(rows.len());
    for row in rows {
        let voter = match (row.kind.as_str(), row.age_days) {
            ("anon", None) => Voter::Anonymous,
            ("account", Some(days)) => {
                Voter::Account { provider_age_days: u32::try_from(days.max(0))? }
            },
            _ => return Err(RecomputeError::InvalidVoter { kind: row.kind }),
        };
        ratings.push(AxisRating {
            tracker_id: row.tracker_id,
            axis: row.axis.parse::<Axis>()?,
            rating: Rating { score: Score::try_from(row.score)?, voter },
        });
    }

    let results = aggregate(&ratings, cfg);
    let ids: Vec<i32> = results.iter().map(|s| s.tracker_id).collect();
    let axes: Vec<String> =
        results.iter().map(|s| s.axis.key().to_owned()).collect();
    let scores: Vec<f64> = results.iter().map(|s| s.score.score).collect();
    let weights: Vec<f64> =
        results.iter().map(|s| s.score.effective_weight).collect();
    let counts = results
        .iter()
        .map(|s| i32::try_from(s.score.vote_count))
        .collect::<Result<Vec<i32>, _>>()?;

    let mut tx = pool.begin().await?;

    sqlx::query!("SELECT pg_advisory_xact_lock($1)", REFRESH_LOCK_KEY)
        .execute(&mut *tx)
        .await?;
    sqlx::query!("DELETE FROM tracker_scores").execute(&mut *tx).await?;
    let written = sqlx::query!(
        "INSERT INTO tracker_scores (
             tracker_id, axis, score, effective_weight, vote_count
         )
         SELECT u.tracker_id, u.axis::rating_axis, u.score, u.effective_weight,
                u.vote_count
         FROM UNNEST(
             $1::integer[], $2::text[], $3::float8[], $4::float8[], $5::integer[]
         ) AS u(tracker_id, axis, score, effective_weight, vote_count)",
        &ids,
        &axes,
        &scores,
        &weights,
        &counts,
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();

    tx.commit().await?;
    Ok(RefreshReport { ratings: u64::try_from(ratings.len())?, scores: written })
}
