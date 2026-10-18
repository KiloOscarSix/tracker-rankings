use rankings_core::axis::Axis;
use rankings_core::membership::MembershipStatus;
use rankings_core::score::Score;
use sqlx::PgPool;

use crate::error::PageError;
use crate::rankings::axis_index;

#[derive(Debug, Clone)]
pub struct MyTracker {
    pub status: MembershipStatus,
    /// Indexed by [`axis_index`].
    pub ratings: [Option<Score>; 5],
}

#[derive(Debug, Clone)]
pub struct MyMembership {
    pub slug: String,
    pub name: String,
    pub status: MembershipStatus,
    /// Indexed by [`axis_index`].
    pub ratings: [Option<Score>; 5],
}

pub async fn listed_tracker_id(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<i32>, PageError> {
    let id = sqlx::query_scalar!(
        "SELECT id FROM trackers WHERE slug = $1 AND listed",
        slug
    )
    .fetch_optional(pool)
    .await?;
    Ok(id)
}

pub async fn set_membership(
    pool: &PgPool,
    voter_id: i64,
    tracker_id: i32,
    status: MembershipStatus,
) -> Result<(), PageError> {
    sqlx::query!(
        "INSERT INTO memberships (voter_id, tracker_id, status)
         VALUES ($1, $2, $3::text::membership_status)
         ON CONFLICT (voter_id, tracker_id)
             DO UPDATE SET status = EXCLUDED.status, updated_at = now()",
        voter_id,
        tracker_id,
        status.key(),
    )
    .execute(pool)
    .await?;
    Ok(())
}

/// Ratings for the tracker go with it through the foreign key cascade.
pub async fn remove_membership(
    pool: &PgPool,
    voter_id: i64,
    tracker_id: i32,
) -> Result<(), PageError> {
    sqlx::query!(
        "DELETE FROM memberships WHERE voter_id = $1 AND tracker_id = $2",
        voter_id,
        tracker_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn is_member(
    pool: &PgPool,
    voter_id: i64,
    tracker_id: i32,
) -> Result<bool, PageError> {
    let member = sqlx::query_scalar!(
        r#"SELECT EXISTS(
               SELECT 1 FROM memberships WHERE voter_id = $1 AND tracker_id = $2
           ) AS "member!""#,
        voter_id,
        tracker_id,
    )
    .fetch_one(pool)
    .await?;
    Ok(member)
}

pub async fn upsert_ratings(
    pool: &PgPool,
    voter_id: i64,
    tracker_id: i32,
    ratings: &[(Axis, Score)],
) -> Result<(), PageError> {
    let axes: Vec<String> =
        ratings.iter().map(|(axis, _)| axis.key().to_owned()).collect();
    let scores: Vec<i16> =
        ratings.iter().map(|(_, score)| i16::from(score.get())).collect();

    sqlx::query!(
        "INSERT INTO ratings (voter_id, tracker_id, axis, score)
         SELECT $1::bigint, $2::integer, u.axis::rating_axis, u.score
         FROM UNNEST($3::text[], $4::smallint[]) AS u(axis, score)
         ON CONFLICT (voter_id, tracker_id, axis)
             DO UPDATE SET score = EXCLUDED.score, updated_at = now()",
        voter_id,
        tracker_id,
        &axes,
        &scores,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn voter_tracker(
    pool: &PgPool,
    voter_id: i64,
    tracker_id: i32,
) -> Result<Option<MyTracker>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT m.status::text AS "status!", r.axis::text AS "axis?",
                  r.score AS "score?"
           FROM memberships m
           LEFT JOIN ratings r
               ON r.voter_id = m.voter_id AND r.tracker_id = m.tracker_id
           WHERE m.voter_id = $1 AND m.tracker_id = $2"#,
        voter_id,
        tracker_id,
    )
    .fetch_all(pool)
    .await?;

    let Some(first) = rows.first() else {
        return Ok(None);
    };
    let status = first.status.parse::<MembershipStatus>()?;
    let mut ratings = [None; 5];
    for row in &rows {
        place_rating(&mut ratings, row.axis.as_deref(), row.score)?;
    }
    Ok(Some(MyTracker { status, ratings }))
}

pub async fn voter_memberships(
    pool: &PgPool,
    voter_id: i64,
) -> Result<Vec<MyMembership>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT t.slug, t.name, m.status::text AS "status!",
                  r.axis::text AS "axis?", r.score AS "score?"
           FROM memberships m
           JOIN trackers t ON t.id = m.tracker_id
           LEFT JOIN ratings r
               ON r.voter_id = m.voter_id AND r.tracker_id = m.tracker_id
           WHERE m.voter_id = $1
           ORDER BY t.name, t.slug"#,
        voter_id,
    )
    .fetch_all(pool)
    .await?;

    let mut memberships: Vec<MyMembership> = Vec::new();
    for row in rows {
        if memberships.last().is_none_or(|last| last.slug != row.slug) {
            memberships.push(MyMembership {
                status: row.status.parse::<MembershipStatus>()?,
                slug: row.slug,
                name: row.name,
                ratings: [None; 5],
            });
        }
        if let Some(last) = memberships.last_mut() {
            place_rating(&mut last.ratings, row.axis.as_deref(), row.score)?;
        }
    }
    Ok(memberships)
}

/// Cascades to the account's sessions, voter, memberships, ratings and
/// pairwise votes.
pub async fn delete_account(
    pool: &PgPool,
    account_id: i64,
) -> Result<(), PageError> {
    sqlx::query!("DELETE FROM accounts WHERE id = $1", account_id)
        .execute(pool)
        .await?;
    Ok(())
}

/// Cascades to the anonymous voter and everything it voted.
pub async fn delete_anon_identity(
    pool: &PgPool,
    voter_id: i64,
) -> Result<(), PageError> {
    sqlx::query!(
        "DELETE FROM anon_voters
         WHERE id = (SELECT anon_id FROM voters WHERE id = $1 AND kind = 'anon')",
        voter_id,
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn place_rating(
    ratings: &mut [Option<Score>; 5],
    axis: Option<&str>,
    score: Option<i16>,
) -> Result<(), PageError> {
    if let (Some(axis), Some(score)) = (axis, score) {
        let axis = axis.parse::<Axis>()?;
        if let Some(slot) = ratings.get_mut(axis_index(axis)) {
            *slot = Some(Score::try_from(score)?);
        }
    }
    Ok(())
}
