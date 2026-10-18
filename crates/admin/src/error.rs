use std::io;
use std::num::{ParseIntError, TryFromIntError};
use std::path::PathBuf;

use rankings_core::error::{InvalidScore, UnknownAxisKey, UnknownCategoryToken};

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("could not read {}: {source}", path.display())]
    Read {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("trackers.json does not match the expected shape: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("SOURCE has no `{0}:` line")]
    SourceKey(&'static str),
    #[error("tracker {tracker:?} has a non-numeric birthdate {value:?}")]
    InvalidYear {
        tracker: String,
        value: String,
        #[source]
        source: ParseIntError,
    },
    #[error("tracker {tracker:?} has an unmapped type token: {source}")]
    Category {
        tracker: String,
        #[source]
        source: UnknownCategoryToken,
    },
    #[error("unlockInviteClass names {0:?}, which trackerInfo does not describe")]
    UnlockWithoutTracker(String),
    #[error("tracker name {0:?} has no ASCII letters or digits to slug")]
    EmptySlug(String),
    #[error("trackers {first:?} and {second:?} share the slug {slug:?}")]
    SlugCollision { slug: String, first: String, second: String },
    #[error("upserted {upserted} of {expected} routes; some endpoints are missing")]
    UnresolvedRoutes { expected: u64, upserted: u64 },
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum RecomputeError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
    #[error(transparent)]
    Axis(#[from] UnknownAxisKey),
    #[error(transparent)]
    Score(#[from] InvalidScore),
    #[error("count out of range: {0}")]
    Count(#[from] TryFromIntError),
    #[error("voter of kind {kind:?} has an inconsistent account age")]
    InvalidVoter { kind: String },
}

#[derive(Debug, thiserror::Error)]
pub enum VoidError {
    #[error("no voter has id {0}")]
    UnknownVoter(i64),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum AdminError {
    #[error(transparent)]
    Import(#[from] ImportError),
    #[error(transparent)]
    Recompute(#[from] RecomputeError),
    #[error(transparent)]
    Void(#[from] VoidError),
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),
}
