use sqlx::{PgConnection, PgPool};

use crate::error::ImportError;
use crate::rows::{ListedRows, RouteRows, SeedRows, StubRows};
use crate::seed::{Seed, Source};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImportReport {
    pub listed: u64,
    pub stubs: u64,
    pub routes: u64,
    pub stale_routes_removed: u64,
}

/// Upserts trackers and routes in one transaction. Trackers are never deleted
/// and vote tables are never touched; routes no longer upstream are removed.
pub async fn import(
    pool: &PgPool,
    seed: &Seed,
    source: &Source,
) -> Result<ImportReport, ImportError> {
    let rows = SeedRows::from_seed(seed)?;
    let mut tx = pool.begin().await?;

    let listed = upsert_listed(&mut tx, &rows.listed, &source.commit).await?;
    let stubs = upsert_stubs(&mut tx, &rows.stubs, &source.commit).await?;
    let routes = upsert_routes(&mut tx, &rows.routes).await?;
    let expected = rows.routes.sources.len() as u64;
    if routes != expected {
        return Err(ImportError::UnresolvedRoutes { expected, upserted: routes });
    }
    let stale_routes_removed = delete_stale_routes(&mut tx, &rows.routes).await?;

    tx.commit().await?;
    Ok(ImportReport { listed, stubs, routes, stale_routes_removed })
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param override, required because `query!` infers non-null array elements"
)]
async fn upsert_listed(
    conn: &mut PgConnection,
    rows: &ListedRows,
    commit: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO trackers (
             slug, name, listed, abbr, raw_type, description, country,
             founded_year, unlock_days, unlock_reqs, source_sha, categories
         )
         SELECT u.slug, u.name, TRUE, u.abbr, u.raw_type, u.description,
                u.country, u.founded_year, u.unlock_days, u.unlock_reqs, $10,
                string_to_array(u.categories, ',')
         FROM UNNEST(
             $1::text[], $2::text[], $3::text[], $4::text[], $5::text[],
             $6::text[], $7::smallint[], $8::integer[], $9::text[], $11::text[]
         ) AS u(
             slug, name, abbr, raw_type, description, country,
             founded_year, unlock_days, unlock_reqs, categories
         )
         ON CONFLICT (name) DO UPDATE SET
             slug = EXCLUDED.slug,
             listed = TRUE,
             abbr = EXCLUDED.abbr,
             raw_type = EXCLUDED.raw_type,
             description = EXCLUDED.description,
             country = EXCLUDED.country,
             founded_year = EXCLUDED.founded_year,
             unlock_days = EXCLUDED.unlock_days,
             unlock_reqs = EXCLUDED.unlock_reqs,
             categories = EXCLUDED.categories,
             source_sha = EXCLUDED.source_sha,
             updated_at = now()",
        &rows.slugs,
        &rows.names,
        &rows.abbrs,
        &rows.raw_types,
        &rows.descriptions,
        &rows.countries as &[Option<String>],
        &rows.founded_years as &[Option<i16>],
        &rows.unlock_days as &[Option<i32>],
        &rows.unlock_reqs as &[Option<String>],
        commit,
        &rows.categories,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

async fn upsert_stubs(
    conn: &mut PgConnection,
    rows: &StubRows,
    commit: &str,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO trackers (slug, name, listed, source_sha)
         SELECT u.slug, u.name, FALSE, $3
         FROM UNNEST($1::text[], $2::text[]) AS u(slug, name)
         ON CONFLICT (name) DO UPDATE SET
             source_sha = EXCLUDED.source_sha,
             updated_at = now()",
        &rows.slugs,
        &rows.names,
        commit,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

#[expect(
    trivial_casts,
    reason = "not a cast: `as T` is sqlx's bind-param override, required because `query!` infers non-null array elements"
)]
async fn upsert_routes(
    conn: &mut PgConnection,
    rows: &RouteRows,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "INSERT INTO routes (
             source_id, target_id, days, reqs, active, updated_label
         )
         SELECT s.id, t.id, u.days, u.reqs, u.active, u.updated_label
         FROM UNNEST(
             $1::text[], $2::text[], $3::integer[], $4::text[], $5::text[],
             $6::text[]
         ) AS u(source, target, days, reqs, active, updated_label)
         JOIN trackers s ON s.name = u.source
         JOIN trackers t ON t.name = u.target
         ON CONFLICT (source_id, target_id) DO UPDATE SET
             days = EXCLUDED.days,
             reqs = EXCLUDED.reqs,
             active = EXCLUDED.active,
             updated_label = EXCLUDED.updated_label",
        &rows.sources,
        &rows.targets,
        &rows.days as &[Option<i32>],
        &rows.reqs,
        &rows.active,
        &rows.updated,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

async fn delete_stale_routes(
    conn: &mut PgConnection,
    rows: &RouteRows,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM routes r
         USING trackers s, trackers t
         WHERE r.source_id = s.id
           AND r.target_id = t.id
           AND (s.name, t.name) NOT IN (
               SELECT * FROM UNNEST($1::text[], $2::text[])
           )",
        &rows.sources,
        &rows.targets,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}
