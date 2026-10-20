use rankings_core::category::Category;
use sqlx::PgPool;

use crate::error::PageError;

#[derive(Debug, Clone)]
pub struct Contender {
    pub slug: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy)]
pub struct PairIds {
    pub winner_id: i32,
    pub loser_id: i32,
    pub share_category: bool,
}

#[derive(Debug, Clone)]
pub struct StrengthRow {
    pub slug: String,
    pub name: String,
    pub strength: f64,
    pub vote_count: u32,
}

/// Up to two of the voter's listed memberships, in random order
pub async fn member_matchup(
    pool: &PgPool,
    voter_id: i64,
) -> Result<Vec<Contender>, PageError> {
    let rows = sqlx::query_as!(
        Contender,
        "SELECT t.slug, t.name
         FROM memberships m
         JOIN trackers t ON t.id = m.tracker_id AND t.listed
         WHERE m.voter_id = $1
         ORDER BY random()
         LIMIT 2",
        voter_id,
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Up to two listed trackers in the category, in random order
pub async fn category_matchup(
    pool: &PgPool,
    category: Category,
) -> Result<Vec<Contender>, PageError> {
    let rows = sqlx::query_as!(
        Contender,
        "SELECT slug, name
         FROM trackers
         WHERE listed AND $1 = ANY(categories)
         ORDER BY random()
         LIMIT 2",
        category.key(),
    )
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

/// Ids of two listed trackers, and whether they share a category
pub async fn pair_ids(
    pool: &PgPool,
    winner_slug: &str,
    loser_slug: &str,
) -> Result<Option<PairIds>, PageError> {
    let ids = sqlx::query_as!(
        PairIds,
        r#"SELECT w.id AS winner_id, l.id AS loser_id,
                  (w.categories && l.categories) AS "share_category!"
           FROM trackers w, trackers l
           WHERE w.slug = $1 AND w.listed AND l.slug = $2 AND l.listed"#,
        winner_slug,
        loser_slug,
    )
    .fetch_optional(pool)
    .await?;
    Ok(ids)
}

/// Re-voting on the same pair overwrites the earlier vote.
pub async fn upsert_pairwise_vote(
    pool: &PgPool,
    voter_id: i64,
    winner_id: i32,
    loser_id: i32,
) -> Result<(), PageError> {
    sqlx::query!(
        "INSERT INTO pairwise_votes (voter_id, winner_id, loser_id)
         VALUES ($1, $2, $3)
         ON CONFLICT (voter_id, LEAST(winner_id, loser_id), GREATEST(winner_id, loser_id))
             DO UPDATE SET winner_id = EXCLUDED.winner_id,
                           loser_id = EXCLUDED.loser_id,
                           created_at = now()",
        voter_id,
        winner_id,
        loser_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn pairwise_ranking(pool: &PgPool) -> Result<Vec<StrengthRow>, PageError> {
    let rows = sqlx::query!(
        "SELECT t.slug, t.name, s.strength, s.vote_count
         FROM pairwise_scores s
         JOIN trackers t ON t.id = s.tracker_id AND t.listed
         ORDER BY s.strength DESC, t.name"
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(StrengthRow {
                slug: row.slug,
                name: row.name,
                strength: row.strength,
                vote_count: u32::try_from(row.vote_count)?,
            })
        })
        .collect()
}
