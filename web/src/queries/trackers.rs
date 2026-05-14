use ladder_core::axis::Axis;
use ladder_core::category::Category;
use ladder_core::score::AxisScore;
use sqlx::PgPool;

use crate::error::PageError;

#[derive(Debug, Clone)]
pub struct ListedTracker {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub abbr: String,
    pub categories: Vec<Category>,
}

#[derive(Debug, Clone)]
pub struct ScoreRow {
    pub tracker_id: i32,
    pub axis: Axis,
    pub score: AxisScore,
}

#[derive(Debug, Clone)]
pub struct TrackerDetail {
    pub id: i32,
    pub slug: String,
    pub name: String,
    pub listed: bool,
    pub abbr: Option<String>,
    pub raw_type: Option<String>,
    pub description: Option<String>,
    pub categories: Vec<Category>,
    pub country: Option<String>,
    pub founded_year: Option<i16>,
    pub unlock_days: Option<i32>,
    pub unlock_reqs: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RouteRow {
    pub slug: String,
    pub name: String,
    pub listed: bool,
    pub days: Option<i32>,
    pub reqs: String,
    pub active: String,
    pub updated_label: String,
    pub overall: Option<AxisScore>,
}

pub async fn listed_trackers(
    pool: &PgPool,
) -> Result<Vec<ListedTracker>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT id, slug, name, abbr AS "abbr!", categories
           FROM trackers
           WHERE listed
           ORDER BY name"#
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ListedTracker {
                id: row.id,
                slug: row.slug,
                name: row.name,
                abbr: row.abbr,
                categories: categories(&row.categories)?,
            })
        })
        .collect()
}

pub async fn all_scores(pool: &PgPool) -> Result<Vec<ScoreRow>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT tracker_id, axis::text AS "axis!", score, effective_weight,
                  vote_count
           FROM tracker_scores"#
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ScoreRow {
                tracker_id: row.tracker_id,
                axis: row.axis.parse::<Axis>()?,
                score: axis_score(row.score, row.effective_weight, row.vote_count)?,
            })
        })
        .collect()
}

pub async fn tracker_scores(
    pool: &PgPool,
    tracker_id: i32,
) -> Result<Vec<ScoreRow>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT tracker_id, axis::text AS "axis!", score, effective_weight,
                  vote_count
           FROM tracker_scores
           WHERE tracker_id = $1"#,
        tracker_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(ScoreRow {
                tracker_id: row.tracker_id,
                axis: row.axis.parse::<Axis>()?,
                score: axis_score(row.score, row.effective_weight, row.vote_count)?,
            })
        })
        .collect()
}

pub async fn tracker_by_slug(
    pool: &PgPool,
    slug: &str,
) -> Result<Option<TrackerDetail>, PageError> {
    let row = sqlx::query!(
        "SELECT id, slug, name, listed, abbr, raw_type, description, categories,
                country, founded_year, unlock_days, unlock_reqs
         FROM trackers
         WHERE slug = $1",
        slug
    )
    .fetch_optional(pool)
    .await?;

    row.map(|row| {
        Ok(TrackerDetail {
            id: row.id,
            slug: row.slug,
            name: row.name,
            listed: row.listed,
            abbr: row.abbr,
            raw_type: row.raw_type,
            description: row.description,
            categories: categories(&row.categories)?,
            country: row.country,
            founded_year: row.founded_year,
            unlock_days: row.unlock_days,
            unlock_reqs: row.unlock_reqs,
        })
    })
    .transpose()
}

pub async fn outgoing_routes(
    pool: &PgPool,
    tracker_id: i32,
) -> Result<Vec<RouteRow>, PageError> {
    let rows = sqlx::query!(
        r#"SELECT t.slug, t.name, t.listed, r.days, r.reqs, r.active,
                  r.updated_label,
                  s.score AS "score?",
                  s.effective_weight AS "effective_weight?",
                  s.vote_count AS "vote_count?"
           FROM routes r
           JOIN trackers t ON t.id = r.target_id
           LEFT JOIN tracker_scores s ON s.tracker_id = t.id AND s.axis = 'overall'
           WHERE r.source_id = $1
           ORDER BY s.score DESC NULLS LAST, r.days ASC NULLS LAST, t.name"#,
        tracker_id
    )
    .fetch_all(pool)
    .await?;

    rows.into_iter()
        .map(|row| {
            let overall = match (row.score, row.effective_weight, row.vote_count) {
                (Some(score), Some(weight), Some(count)) => {
                    Some(axis_score(score, weight, count)?)
                },
                _ => None,
            };
            Ok(RouteRow {
                slug: row.slug,
                name: row.name,
                listed: row.listed,
                days: row.days,
                reqs: row.reqs,
                active: row.active,
                updated_label: row.updated_label,
                overall,
            })
        })
        .collect()
}

pub async fn incoming_routes(
    pool: &PgPool,
    tracker_id: i32,
) -> Result<Vec<RouteRow>, PageError> {
    let rows = sqlx::query!(
        "SELECT t.slug, t.name, t.listed, r.days, r.reqs, r.active, r.updated_label
         FROM routes r
         JOIN trackers t ON t.id = r.source_id
         WHERE r.target_id = $1
         ORDER BY t.name",
        tracker_id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| RouteRow {
            slug: row.slug,
            name: row.name,
            listed: row.listed,
            days: row.days,
            reqs: row.reqs,
            active: row.active,
            updated_label: row.updated_label,
            overall: None,
        })
        .collect())
}

fn categories(keys: &[String]) -> Result<Vec<Category>, PageError> {
    keys.iter().map(|key| Ok(key.parse::<Category>()?)).collect()
}

fn axis_score(
    score: f64,
    effective_weight: f64,
    vote_count: i32,
) -> Result<AxisScore, PageError> {
    Ok(AxisScore { score, effective_weight, vote_count: u32::try_from(vote_count)? })
}
