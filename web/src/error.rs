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
}

#[derive(Debug, thiserror::Error)]
pub enum PageError {
    #[error("{0} is not registered as app context")]
    MissingContext(&'static str),
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Category(#[from] ladder_core::error::UnknownCategoryKey),
    #[error(transparent)]
    Axis(#[from] ladder_core::error::UnknownAxisKey),
    #[error("negative vote count: {0}")]
    NegativeCount(#[from] std::num::TryFromIntError),
}
