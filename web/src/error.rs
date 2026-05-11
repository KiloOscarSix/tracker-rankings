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
}
