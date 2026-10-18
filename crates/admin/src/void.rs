use sqlx::PgPool;

use crate::error::VoidError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoidReport {
    pub ratings: u64,
    pub memberships: u64,
    pub pairwise_votes: u64,
}

/// Deletes every rating, membership and pairwise vote a voter has cast in one
/// transaction
pub async fn void_voter(
    pool: &PgPool,
    voter_id: i64,
) -> Result<VoidReport, VoidError> {
    let mut tx = pool.begin().await?;

    sqlx::query_scalar!("SELECT id FROM voters WHERE id = $1 FOR UPDATE", voter_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(VoidError::UnknownVoter(voter_id))?;

    let ratings = sqlx::query!("DELETE FROM ratings WHERE voter_id = $1", voter_id)
        .execute(&mut *tx)
        .await?
        .rows_affected();
    let memberships =
        sqlx::query!("DELETE FROM memberships WHERE voter_id = $1", voter_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();
    let pairwise_votes =
        sqlx::query!("DELETE FROM pairwise_votes WHERE voter_id = $1", voter_id)
            .execute(&mut *tx)
            .await?
            .rows_affected();

    tx.commit().await?;
    Ok(VoidReport { ratings, memberships, pairwise_votes })
}
