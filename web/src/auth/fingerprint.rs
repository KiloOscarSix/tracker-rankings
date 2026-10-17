use std::net::{IpAddr, Ipv4Addr};

use hmac::digest::InvalidLength;
use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;

use crate::config::CookieKey;

const FINGERPRINT_DOMAIN: &[u8] = b"trackerrankings-anon-v1";

/// Network an address belongs to for anonymous-voter fingerprints
#[must_use]
pub fn network_prefix(ip: Option<IpAddr>) -> String {
    let v4_prefix = |addr: Ipv4Addr| {
        let [a, b, c, _] = addr.octets();
        format!("{a}.{b}.{c}.0/24")
    };

    match ip {
        None => "unknown".to_owned(),
        Some(IpAddr::V4(addr)) => v4_prefix(addr),
        Some(IpAddr::V6(addr)) => addr.to_ipv4_mapped().map_or_else(
            || {
                let [s0, s1, s2, ..] = addr.segments();
                format!("{s0:x}:{s1:x}:{s2:x}::/48")
            },
            v4_prefix,
        ),
    }
}

pub fn fingerprint(
    key: &CookieKey,
    cookie_id: &str,
    prefix: &str,
    user_agent: &str,
) -> Result<[u8; 32], InvalidLength> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())?;
    for part in [
        FINGERPRINT_DOMAIN,
        cookie_id.as_bytes(),
        prefix.as_bytes(),
        user_agent.as_bytes(),
    ] {
        mac.update(&part.len().to_le_bytes());
        mac.update(part);
    }
    Ok(mac.finalize().into_bytes().into())
}
