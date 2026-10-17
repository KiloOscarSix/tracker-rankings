use std::time::Duration;

use sqlx::PgPool;
use topcoat::context::Cx;
use topcoat::session::SessionConfig;
use tracing::warn;

use crate::auth::provider::Provider;
use crate::context::db;

pub const SESSION_DAYS: u16 = 30;
const PRUNE_INTERVAL: Duration = Duration::from_hours(1);

#[derive(Debug, Clone)]
pub struct CurrentAccount {
    pub id: i64,
    pub display_name: String,
    pub provider: Provider,
}

#[must_use]
pub fn session_config() -> SessionConfig {
    SessionConfig::builder()
        .lifetime(Duration::from_hours(u64::from(SESSION_DAYS) * 24))
        .build()
}

pub async fn sign_in(
    cx: &Cx,
    pool: &PgPool,
    account_id: i64,
) -> topcoat::Result<()> {
    if let Some(old) = topcoat::session::token_hash(cx).await? {
        sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", old.as_slice())
            .execute(pool)
            .await?;
    }

    let started = topcoat::session::start(cx).await?;
    sqlx::query!(
        "INSERT INTO sessions (token_hash, account_id, expires_at) \
         VALUES ($1, $2, now() + make_interval(days => $3))",
        started.token_hash.as_slice(),
        account_id,
        i32::from(SESSION_DAYS),
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn sign_out(cx: &Cx, pool: &PgPool) -> topcoat::Result<()> {
    if let Some(hash) = topcoat::session::stop(cx).await? {
        sqlx::query!("DELETE FROM sessions WHERE token_hash = $1", hash.as_slice())
            .execute(pool)
            .await?;
    }
    Ok(())
}

pub async fn current_account(cx: &Cx) -> topcoat::Result<Option<CurrentAccount>> {
    let Some(hash) = topcoat::session::token_hash(cx).await? else {
        return Ok(None);
    };

    let row = sqlx::query!(
        r#"SELECT a.id, a.display_name, a.provider::text AS "provider!"
           FROM sessions s
           JOIN accounts a ON a.id = s.account_id
           WHERE s.token_hash = $1 AND s.expires_at > now()"#,
        hash.as_slice(),
    )
    .fetch_optional(db(cx)?)
    .await?;

    let Some(row) = row else {
        return Ok(None);
    };
    Ok(Some(CurrentAccount {
        id: row.id,
        display_name: row.display_name,
        provider: row.provider.parse::<Provider>()?,
    }))
}

pub async fn prune_expired_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub fn spawn_session_pruning(pool: PgPool) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(PRUNE_INTERVAL);
        loop {
            ticker.tick().await;
            if let Err(e) = prune_expired_sessions(&pool).await {
                warn!(error = %e, "could not prune expired sessions");
            }
        }
    });
}
