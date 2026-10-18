use topcoat::context::Cx;

use crate::auth::anon::{anon_voter, existing_anon_voter};
use crate::auth::sessions::current_account;
use crate::context::db;

/// The voter behind this request, if one exists
pub async fn current_voter(cx: &Cx) -> topcoat::Result<Option<i64>> {
    if let Some(account) = current_account(cx).await? {
        let voter = sqlx::query_scalar!(
            "SELECT id FROM voters WHERE account_id = $1",
            account.id,
        )
        .fetch_optional(db(cx)?)
        .await?;
        return Ok(voter);
    }
    existing_anon_voter(cx).await
}

/// The voter behind this request, created on first use. Anonymous voters get
/// their long-lived cookie here.
pub async fn voter_for_write(cx: &Cx) -> topcoat::Result<i64> {
    if let Some(account) = current_account(cx).await? {
        let voter = sqlx::query_scalar!(
            r#"INSERT INTO voters (kind, account_id) VALUES ('account', $1)
               ON CONFLICT (account_id)
                   DO UPDATE SET account_id = EXCLUDED.account_id
               RETURNING id AS "id!""#,
            account.id,
        )
        .fetch_one(db(cx)?)
        .await?;
        return Ok(voter);
    }
    anon_voter(cx).await
}
