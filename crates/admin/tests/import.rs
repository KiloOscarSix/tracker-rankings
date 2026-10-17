use std::path::Path;

use rankings_admin::error::ImportError;
use rankings_admin::import::{ImportReport, import};
use rankings_admin::seed::{self, Seed, Source};
use sqlx::PgPool;

const EXPECTED: ImportReport =
    ImportReport { listed: 182, stubs: 19, routes: 598, stale_routes_removed: 0 };

fn vendored() -> Result<(Seed, Source), ImportError> {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/trackerpathways");
    seed::load(&dir)
}

#[sqlx::test(migrations = "../../migrations")]
async fn importing_twice_gives_182_trackers_and_598_routes(pool: PgPool) {
    let (seed, source) = vendored().unwrap();

    assert_eq!(import(&pool, &seed, &source).await.unwrap(), EXPECTED);
    assert_eq!(import(&pool, &seed, &source).await.unwrap(), EXPECTED);

    let counts = sqlx::query!(
        r#"SELECT
             (SELECT count(*) FROM trackers WHERE listed) AS "listed!",
             (SELECT count(*) FROM trackers WHERE NOT listed) AS "stubs!",
             (SELECT count(*) FROM routes) AS "routes!""#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((counts.listed, counts.stubs, counts.routes), (182, 19, 598));

    let route = sqlx::query!(
        "SELECT r.days, r.reqs FROM routes r
         JOIN trackers s ON s.id = r.source_id
         JOIN trackers t ON t.id = r.target_id
         WHERE s.slug = 'aither' AND t.slug = 'anthelion'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!((route.days, route.reqs.as_str()), (Some(0), "Prometheus+"));

    let unlock_days = sqlx::query_scalar!(
        "SELECT unlock_days FROM trackers WHERE slug = 'aither'"
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unlock_days, Some(240));

    let uncategorised = sqlx::query_scalar!(
        r#"SELECT count(*) AS "n!" FROM trackers WHERE listed AND categories = '{}'"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(uncategorised, 0);

    let aither =
        sqlx::query_scalar!("SELECT categories FROM trackers WHERE slug = 'aither'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(aither, ["movies-tv"]);
}

#[sqlx::test(migrations = "../../migrations")]
async fn reimport_leaves_votes_untouched(pool: PgPool) {
    let (seed, source) = vendored().unwrap();
    import(&pool, &seed, &source).await.unwrap();

    let anon_id = sqlx::query_scalar!(
        "INSERT INTO anon_voters (fingerprint_hmac) VALUES ($1) RETURNING id",
        &[7_u8; 32][..]
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let voter_id = sqlx::query_scalar!(
        "INSERT INTO voters (kind, anon_id) VALUES ('anon', $1) RETURNING id",
        anon_id
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query!(
        "WITH m AS (
             INSERT INTO memberships (voter_id, tracker_id, status)
             SELECT $1, id, 'current' FROM trackers WHERE slug = 'aither'
             RETURNING voter_id, tracker_id
         )
         INSERT INTO ratings (voter_id, tracker_id, axis, score)
         SELECT voter_id, tracker_id, 'overall', 4 FROM m",
        voter_id
    )
    .execute(&pool)
    .await
    .unwrap();

    import(&pool, &seed, &source).await.unwrap();

    let scores = sqlx::query_scalar!(
        "SELECT score FROM ratings WHERE voter_id = $1",
        voter_id
    )
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(scores, [4]);
}
