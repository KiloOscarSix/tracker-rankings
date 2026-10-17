use std::pin::Pin;

use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl,
    AuthorizationCode,
    ClientId,
    ClientSecret,
    EndpointNotSet,
    EndpointSet,
    RedirectUrl,
    TokenResponse,
    TokenUrl,
};
use serde::de::DeserializeOwned;

use crate::auth::provider::Provider;
use crate::config::OAuthCredentials;
use crate::error::{AuthError, StartupError};

pub struct ProviderIdentity {
    /// The provider's stable user id.
    pub subject: String,
    pub display_name: String,
    /// Seconds since the Unix epoch when the provider account was created.
    pub created_at_unix: f64,
}

pub type ExchangeFuture<'a> =
    Pin<Box<dyn Future<Output = Result<ProviderIdentity, AuthError>> + Send + 'a>>;

pub trait OAuthClient: Send + Sync + 'static {
    fn provider(&self) -> Provider;

    /// Absolute URL of the provider's consent page carrying `state`.
    fn authorize_url(&self, state: &str) -> String;

    fn exchange<'a>(&'a self, code: &'a str) -> ExchangeFuture<'a>;
}

pub struct OAuthClients(Vec<Box<dyn OAuthClient>>);

impl OAuthClients {
    #[must_use]
    pub const fn new(clients: Vec<Box<dyn OAuthClient>>) -> Self {
        Self(clients)
    }

    #[must_use]
    pub fn get(&self, provider: Provider) -> Option<&dyn OAuthClient> {
        self.0.iter().map(Box::as_ref).find(|client| client.provider() == provider)
    }

    pub fn enabled(&self) -> impl Iterator<Item = Provider> + '_ {
        Provider::ALL.into_iter().filter(|provider| self.get(*provider).is_some())
    }
}

pub(crate) type ConfiguredClient = BasicClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

pub(crate) fn http_client(
    user_agent: &str,
) -> Result<oauth2::reqwest::Client, StartupError> {
    oauth2::reqwest::ClientBuilder::new()
        .redirect(oauth2::reqwest::redirect::Policy::none())
        .user_agent(user_agent)
        .build()
        .map_err(StartupError::HttpClient)
}

pub(crate) fn configured_client(
    creds: &OAuthCredentials,
    auth_url: &str,
    token_url: &str,
    redirect_url: String,
) -> Result<ConfiguredClient, StartupError> {
    Ok(BasicClient::new(ClientId::new(creds.client_id.clone()))
        .set_client_secret(ClientSecret::new(creds.client_secret.clone()))
        .set_auth_uri(AuthUrl::new(auth_url.to_owned())?)
        .set_token_uri(TokenUrl::new(token_url.to_owned())?)
        .set_redirect_uri(RedirectUrl::new(redirect_url)?))
}

pub(crate) async fn exchange_token(
    client: &ConfiguredClient,
    http: &oauth2::reqwest::Client,
    code: &str,
) -> Result<String, AuthError> {
    let token = client
        .exchange_code(AuthorizationCode::new(code.to_owned()))
        .request_async(http)
        .await
        .map_err(|e| AuthError::TokenExchange(e.to_string()))?;
    Ok(token.access_token().secret().clone())
}

pub(crate) async fn get_json<T: DeserializeOwned>(
    http: &oauth2::reqwest::Client,
    url: &str,
    access_token: &str,
) -> Result<T, AuthError> {
    let bytes = http
        .get(url)
        .bearer_auth(access_token)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    Ok(serde_json::from_slice(&bytes)?)
}
