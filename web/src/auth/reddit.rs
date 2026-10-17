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
use crate::config::RedditConfig;
use crate::error::{AuthError, StartupError};

// Endpoints from the Reddit OAuth2 docs
const AUTHORIZE_URL: &str = "https://www.reddit.com/api/v1/authorize";
const TOKEN_URL: &str = "https://www.reddit.com/api/v1/access_token";
const USER_URL: &str = "https://oauth.reddit.com/api/v1/me";

pub struct RedditOAuth {
    client: ConfiguredClient,
    http: oauth2::reqwest::Client,
}

impl RedditOAuth {
    pub fn new(
        config: &RedditConfig,
        public_base_url: &str,
    ) -> Result<Self, StartupError> {
        let redirect_url = format!("{public_base_url}/auth/reddit/callback");
        Ok(Self {
            client: configured_client(
                &config.credentials,
                AUTHORIZE_URL,
                TOKEN_URL,
                redirect_url,
            )?,
            http: http_client(&config.user_agent)?,
        })
    }
}

#[derive(Deserialize)]
struct RedditUser {
    id: String,
    name: String,
    created_utc: f64,
}

impl OAuthClient for RedditOAuth {
    fn provider(&self) -> Provider {
        Provider::Reddit
    }

    fn authorize_url(&self, state: &str) -> String {
        self.client
            .authorize_url(|| CsrfToken::new(state.to_owned()))
            .add_scope(Scope::new("identity".to_owned()))
            .add_extra_param("duration", "temporary")
            .url()
            .0
            .to_string()
    }

    fn exchange<'a>(&'a self, code: &'a str) -> ExchangeFuture<'a> {
        Box::pin(async move {
            let token = exchange_token(&self.client, &self.http, code).await?;
            let user: RedditUser = get_json(&self.http, USER_URL, &token).await?;
            if !user.created_utc.is_finite() {
                return Err(AuthError::InvalidAccount("reddit created_utc"));
            }
            Ok(ProviderIdentity {
                subject: user.id,
                display_name: user.name,
                created_at_unix: user.created_utc,
            })
        })
    }
}
