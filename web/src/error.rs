use std::io;
use std::net::AddrParseError;

#[derive(Debug, thiserror::Error)]
pub enum StartupError {
    #[error("invalid BIND_ADDR: {0}")]
    BindAddr(#[from] AddrParseError),
    #[error(
        "could not load the asset bundle (run `topcoat asset bundle -p web`): {0}"
    )]
    AssetBundle(#[source] io::Error),
    #[error("could not listen on the bind address: {0}")]
    Listen(#[source] io::Error),
    #[error("server error: {0}")]
    Serve(#[source] io::Error),
    #[error("DATABASE_URL is not set")]
    DatabaseUrl,
    #[error("could not connect to Postgres: {0}")]
    Database(#[from] sqlx::Error),
    #[error("COOKIE_KEY must be set to standard base64 of at least 64 random bytes")]
    CookieKey,
    #[error("PUBLIC_BASE_URL must be an http:// or https:// origin")]
    PublicBaseUrl,
    #[error("{0} sign-in needs both its client id and client secret")]
    PartialCredentials(&'static str),
    #[error("REDDIT_USER_AGENT is required when Reddit sign-in is configured")]
    RedditUserAgent,
    #[error("invalid TRUSTED_PROXIES entry `{0}`")]
    TrustedProxy(String),
    #[error("invalid OAuth endpoint URL: {0}")]
    OAuthUrl(#[from] oauth2::url::ParseError),
    #[error("could not build the HTTP client: {0}")]
    HttpClient(#[source] oauth2::reqwest::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error("{0} is not registered as app context")]
    MissingContext(&'static str),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Category(#[from] rankings_core::error::UnknownCategoryKey),
    #[error(transparent)]
    Axis(#[from] rankings_core::error::UnknownAxisKey),
    #[error(transparent)]
    Score(#[from] rankings_core::error::InvalidScore),
    #[error(transparent)]
    Membership(#[from] rankings_core::error::UnknownMembershipStatus),
    #[error("negative vote count: {0}")]
    NegativeCount(#[from] std::num::TryFromIntError),
}

#[derive(Debug, thiserror::Error)]
#[error("unknown sign-in provider `{0}`")]
pub struct UnknownProvider(pub String);

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("provider request failed: {0}")]
    Http(#[from] oauth2::reqwest::Error),
    #[error("provider response was not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("token exchange failed: {0}")]
    TokenExchange(String),
    #[error("provider returned an invalid account: {0}")]
    InvalidAccount(&'static str),
}
