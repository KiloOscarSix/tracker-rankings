use ladder_core::config::ScoringConfig;
use sqlx::PgPool;
use topcoat::context::{Cx, try_app_context};

use crate::error::PageError;

pub fn db(cx: &Cx) -> Result<&PgPool, PageError> {
    try_app_context::<PgPool>(cx).ok_or(PageError::MissingContext("PgPool"))
}

pub fn scoring(cx: &Cx) -> Result<&ScoringConfig, PageError> {
    try_app_context::<ScoringConfig>(cx)
        .ok_or(PageError::MissingContext("ScoringConfig"))
}
