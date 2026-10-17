use oauth2::{CsrfToken, Scope};
use serde::Deserialize;

use crate::auth::oauth::{
    ConfiguredClient,
    ExchangeFuture,
    OAuthClient,
    ProviderIdentity,
    configured_client,
    exchange_token,
    get_json,
    http_client,
};
use crate::auth::provider::Provider;
use crate::config::OAuthCredentials;
use crate::error::{AuthError, StartupError};

const AUTHORIZE_URL: &str = "https://discord.com/oauth2/authorize";
const TOKEN_URL: &str = "https://discord.com/api/oauth2/token";
const USER_URL: &str = "https://discord.com/api/v10/users/@me";
const USER_AGENT: &str = concat!("trackerrankings/", env!("CARGO_PKG_VERSION"));
const DISCORD_EPOCH_MS: u64 = 1_420_070_400_000;

pub struct DiscordOAuth {
    client: ConfiguredClient,
    http: oauth2::reqwest::Client,
}

impl DiscordOAuth {
    pub fn new(
        creds: &OAuthCredentials,
        public_base_url: &str,
    ) -> Result<Self, StartupError> {
        let redirect_url = format!("{public_base_url}/auth/discord/callback");
        Ok(Self {
            client: configured_client(
                creds,
                AUTHORIZE_URL,
                TOKEN_URL,
                redirect_url,
            )?,
            http: http_client(USER_AGENT)?,
        })
    }
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
}

impl OAuthClient for DiscordOAuth {
    fn provider(&self) -> Provider {
        Provider::Discord
    }

    fn authorize_url(&self, state: &str) -> String {
        self.client
            .authorize_url(|| CsrfToken::new(state.to_owned()))
            .add_scope(Scope::new("identify".to_owned()))
            .url()
            .0
            .to_string()
    }

    fn exchange<'a>(&'a self, code: &'a str) -> ExchangeFuture<'a> {
        Box::pin(async move {
            let token = exchange_token(&self.client, &self.http, code).await?;
            let user: DiscordUser = get_json(&self.http, USER_URL, &token).await?;
            let created_at_unix = user
                .id
                .parse::<u64>()
                .ok()
                .and_then(snowflake_created_at)
                .ok_or(AuthError::InvalidAccount("discord id"))?;
            let display_name = user
                .global_name
                .filter(|name| !name.is_empty())
                .unwrap_or(user.username);
            Ok(ProviderIdentity { subject: user.id, display_name, created_at_unix })
        })
    }
}

/// Creation time (Unix seconds) encoded in a Discord snowflake
#[must_use]
pub fn snowflake_created_at(id: u64) -> Option<f64> {
    u32::try_from(((id >> 22) + DISCORD_EPOCH_MS) / 1000).ok().map(f64::from)
}
