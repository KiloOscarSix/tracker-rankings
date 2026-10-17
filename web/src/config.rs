use std::net::{IpAddr, SocketAddr};

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use topcoat::router::IpNet;

use crate::error::StartupError;

pub const BIND_ADDR_ENV: &str = "BIND_ADDR";
pub const DATABASE_URL_ENV: &str = "DATABASE_URL";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";
pub const COOKIE_KEY_ENV: &str = "COOKIE_KEY";
pub const PUBLIC_BASE_URL_ENV: &str = "PUBLIC_BASE_URL";
pub const TRUSTED_PROXIES_ENV: &str = "TRUSTED_PROXIES";
pub const DISCORD_CLIENT_ID_ENV: &str = "DISCORD_CLIENT_ID";
pub const DISCORD_CLIENT_SECRET_ENV: &str = "DISCORD_CLIENT_SECRET";
pub const REDDIT_CLIENT_ID_ENV: &str = "REDDIT_CLIENT_ID";
pub const REDDIT_CLIENT_SECRET_ENV: &str = "REDDIT_CLIENT_SECRET";
pub const REDDIT_USER_AGENT_ENV: &str = "REDDIT_USER_AGENT";
const DEFAULT_PUBLIC_BASE_URL: &str = "http://localhost:3000";
pub const COOKIE_KEY_MIN_BYTES: usize = 64;

/// Server secret keying the anonymous-voter fingerprint HMAC.
pub struct CookieKey(Box<[u8]>);

impl CookieKey {
    pub fn new(bytes: Vec<u8>) -> Result<Self, StartupError> {
        if bytes.len() < COOKIE_KEY_MIN_BYTES {
            return Err(StartupError::CookieKey);
        }
        Ok(Self(bytes.into_boxed_slice()))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub struct RedditConfig {
    pub credentials: OAuthCredentials,
    pub user_agent: String,
}

pub struct AuthConfig {
    pub public_base_url: String,
    pub cookie_key: CookieKey,
    pub discord: Option<OAuthCredentials>,
    pub reddit: Option<RedditConfig>,
    pub trusted_proxies: Vec<IpNet>,
}

pub fn bind_addr() -> Result<SocketAddr, StartupError> {
    bind_addr_from(|key| std::env::var(key).ok())
}

pub fn bind_addr_from(
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<SocketAddr, StartupError> {
    let raw = lookup(BIND_ADDR_ENV).unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned());
    Ok(raw.parse()?)
}

pub fn database_url() -> Result<String, StartupError> {
    database_url_from(|key| std::env::var(key).ok())
}

pub fn database_url_from(
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<String, StartupError> {
    lookup(DATABASE_URL_ENV)
        .filter(|url| !url.is_empty())
        .ok_or(StartupError::DatabaseUrl)
}

pub fn auth_config() -> Result<AuthConfig, StartupError> {
    auth_config_from(|key| std::env::var(key).ok())
}

pub fn auth_config_from(
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<AuthConfig, StartupError> {
    let get = |key: &str| {
        lookup(key)
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
    };
    let credentials = |provider: &'static str, id_env: &str, secret_env: &str| match (
        get(id_env),
        get(secret_env),
    ) {
        (Some(client_id), Some(client_secret)) => {
            Ok(Some(OAuthCredentials { client_id, client_secret }))
        },
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            Err(StartupError::PartialCredentials(provider))
        },
    };

    let key_bytes = get(COOKIE_KEY_ENV)
        .and_then(|raw| STANDARD.decode(raw).ok())
        .ok_or(StartupError::CookieKey)?;
    let cookie_key = CookieKey::new(key_bytes)?;

    let public_base_url = get(PUBLIC_BASE_URL_ENV)
        .unwrap_or_else(|| DEFAULT_PUBLIC_BASE_URL.to_owned())
        .trim_end_matches('/')
        .to_owned();
    let has_host = ["http://", "https://"].into_iter().any(|scheme| {
        public_base_url.strip_prefix(scheme).is_some_and(|rest| !rest.is_empty())
    });
    if !has_host {
        return Err(StartupError::PublicBaseUrl);
    }

    let discord =
        credentials("Discord", DISCORD_CLIENT_ID_ENV, DISCORD_CLIENT_SECRET_ENV)?;
    let reddit =
        credentials("Reddit", REDDIT_CLIENT_ID_ENV, REDDIT_CLIENT_SECRET_ENV)?
            .map(|credentials| {
                get(REDDIT_USER_AGENT_ENV)
                    .ok_or(StartupError::RedditUserAgent)
                    .map(|user_agent| RedditConfig { credentials, user_agent })
            })
            .transpose()?;

    let proxies = get(TRUSTED_PROXIES_ENV).unwrap_or_default();
    let trusted_proxies = proxies
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .map(|entry| {
            entry
                .parse::<IpNet>()
                .ok()
                .or_else(|| entry.parse::<IpAddr>().ok().map(IpNet::from))
                .ok_or_else(|| StartupError::TrustedProxy(entry.to_owned()))
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(AuthConfig { public_base_url, cookie_key, discord, reddit, trusted_proxies })
}
