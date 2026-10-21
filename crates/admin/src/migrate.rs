use sqlx::PgPool;
use sqlx::migrate::{MigrateError, Migrator};

static MIGRATOR: Migrator = sqlx::migrate!("../../migrations");

pub async fn migrate(pool: &PgPool) -> Result<(), MigrateError> {
    MIGRATOR.run(pool).await
}
