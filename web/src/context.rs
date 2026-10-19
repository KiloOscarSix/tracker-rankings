use std::sync::Arc;

use rankings_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::context::{Cx, try_app_context};

use crate::auth::oauth::OAuthClients;
use crate::config::CookieKey;
use crate::error::PageError;
use crate::rate_limit::RateLimiters;
use crate::scores::ScoreSignal;

pub fn db(cx: &Cx) -> Result<&PgPool, PageError> {
    try_app_context::<PgPool>(cx).ok_or(PageError::MissingContext("PgPool"))
}

pub fn scoring(cx: &Cx) -> Result<&ScoringConfig, PageError> {
    try_app_context::<ScoringConfig>(cx)
        .ok_or(PageError::MissingContext("ScoringConfig"))
}

pub fn oauth_clients(cx: &Cx) -> Result<&OAuthClients, PageError> {
    try_app_context::<OAuthClients>(cx)
        .ok_or(PageError::MissingContext("OAuthClients"))
}

pub fn cookie_key(cx: &Cx) -> Result<&CookieKey, PageError> {
    try_app_context::<CookieKey>(cx).ok_or(PageError::MissingContext("CookieKey"))
}

pub fn rate_limiters(cx: &Cx) -> Result<&Arc<RateLimiters>, PageError> {
    try_app_context::<Arc<RateLimiters>>(cx)
        .ok_or(PageError::MissingContext("RateLimiters"))
}

pub fn score_signal(cx: &Cx) -> Result<&ScoreSignal, PageError> {
    try_app_context::<ScoreSignal>(cx)
        .ok_or(PageError::MissingContext("ScoreSignal"))
}
