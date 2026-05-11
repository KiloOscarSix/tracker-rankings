use std::net::SocketAddr;

use crate::error::StartupError;

pub const BIND_ADDR_ENV: &str = "BIND_ADDR";
const DEFAULT_BIND_ADDR: &str = "127.0.0.1:3000";

pub fn bind_addr() -> Result<SocketAddr, StartupError> {
    bind_addr_from(|key| std::env::var(key).ok())
}

pub fn bind_addr_from(
    lookup: impl Fn(&str) -> Option<String>,
) -> Result<SocketAddr, StartupError> {
    let raw = lookup(BIND_ADDR_ENV).unwrap_or_else(|| DEFAULT_BIND_ADDR.to_owned());
    Ok(raw.parse()?)
}
