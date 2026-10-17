use sqlx::PgPool;

use crate::auth::oauth::ProviderIdentity;
use crate::auth::provider::Provider;

pub async fn upsert_account(
    pool: &PgPool,
    provider: Provider,
    identity: &ProviderIdentity,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"WITH account AS (
               INSERT INTO accounts (provider, provider_subject, display_name, provider_created_at)
               VALUES ($1::text::oauth_provider, $2, $3, to_timestamp($4))
               ON CONFLICT (provider, provider_subject)
                   DO UPDATE SET display_name = EXCLUDED.display_name
               RETURNING id
           ), voter AS (
               INSERT INTO voters (kind, account_id)
               SELECT 'account'::voter_kind, id FROM account
               ON CONFLICT (account_id) DO NOTHING
           )
           SELECT id AS "id!" FROM account"#,
        provider.key(),
        identity.subject,
        identity.display_name,
        identity.created_at_unix,
    )
    .fetch_one(pool)
    .await
}
