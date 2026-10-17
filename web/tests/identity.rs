use std::net::IpAddr;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use web::auth::discord::snowflake_created_at;
use web::auth::fingerprint::{fingerprint, network_prefix};
use web::config::{
    AuthConfig,
    COOKIE_KEY_ENV,
    CookieKey,
    DISCORD_CLIENT_ID_ENV,
    PUBLIC_BASE_URL_ENV,
    REDDIT_CLIENT_ID_ENV,
    REDDIT_CLIENT_SECRET_ENV,
    TRUSTED_PROXIES_ENV,
    auth_config_from,
};
use web::error::StartupError;

fn config_with(vars: &[(&str, &str)]) -> Result<AuthConfig, StartupError> {
    auth_config_from(|key| {
        vars.iter()
            .find(|(name, _)| *name == key)
            .map(|(_, value)| (*value).to_owned())
    })
}

#[test]
fn discord_snowflake_decodes_its_creation_time() {
    assert_eq!(snowflake_created_at(175_928_847_299_117_063), Some(1_462_015_105.0));
}

#[test]
fn network_prefix_keeps_a_24_or_48() {
    let v4 = "203.0.113.77".parse::<IpAddr>().unwrap();
    let v6 = "2001:db8:abcd:12::1".parse::<IpAddr>().unwrap();
    let mapped = "::ffff:203.0.113.77".parse::<IpAddr>().unwrap();

    assert_eq!(network_prefix(Some(v4)), "203.0.113.0/24");
    assert_eq!(network_prefix(Some(v6)), "2001:db8:abcd::/48");
    assert_eq!(network_prefix(Some(mapped)), "203.0.113.0/24");
    assert_eq!(network_prefix(None), "unknown");
}

#[test]
fn fingerprint_is_stable_and_depends_on_every_input() {
    let key = CookieKey::new(vec![7; 64]).unwrap();
    let other_key = CookieKey::new(vec![8; 64]).unwrap();
    let base = fingerprint(&key, "cookie", "203.0.113.0/24", "agent").unwrap();

    assert_eq!(
        base,
        fingerprint(&key, "cookie", "203.0.113.0/24", "agent").unwrap()
    );
    assert_ne!(
        base,
        fingerprint(&other_key, "cookie", "203.0.113.0/24", "agent").unwrap()
    );
    assert_ne!(base, fingerprint(&key, "other", "203.0.113.0/24", "agent").unwrap());
    assert_ne!(
        base,
        fingerprint(&key, "cookie", "198.51.100.0/24", "agent").unwrap()
    );
    assert_ne!(
        base,
        fingerprint(&key, "cookie", "203.0.113.0/24", "other").unwrap()
    );
    assert_ne!(
        fingerprint(&key, "ab", "c", "agent").unwrap(),
        fingerprint(&key, "a", "bc", "agent").unwrap()
    );
}

#[test]
fn auth_config_requires_a_long_cookie_key_and_whole_credentials() {
    let key = STANDARD.encode([7_u8; 64]);
    let key = key.as_str();
    let short_key = STANDARD.encode([7_u8; 32]);
    let short_key = short_key.as_str();

    let config = config_with(&[(COOKIE_KEY_ENV, key)]).unwrap();
    assert_eq!(config.public_base_url, "http://localhost:3000");
    assert!(config.discord.is_none());
    assert!(config.reddit.is_none());
    assert_eq!(config.trusted_proxies, []);

    assert!(matches!(
        config_with(&[(COOKIE_KEY_ENV, short_key)]).err(),
        Some(StartupError::CookieKey)
    ));
    assert!(matches!(config_with(&[]).err(), Some(StartupError::CookieKey)));

    assert!(matches!(
        config_with(&[(COOKIE_KEY_ENV, key), (DISCORD_CLIENT_ID_ENV, "id")]).err(),
        Some(StartupError::PartialCredentials("Discord"))
    ));
    assert!(matches!(
        config_with(&[
            (COOKIE_KEY_ENV, key),
            (REDDIT_CLIENT_ID_ENV, "id"),
            (REDDIT_CLIENT_SECRET_ENV, "secret"),
        ])
        .err(),
        Some(StartupError::RedditUserAgent)
    ));

    let proxied = config_with(&[
        (COOKIE_KEY_ENV, key),
        (TRUSTED_PROXIES_ENV, "10.0.0.0/8, 127.0.0.1"),
    ])
    .unwrap();
    assert_eq!(proxied.trusted_proxies.len(), 2);
    assert!(matches!(
        config_with(&[(COOKIE_KEY_ENV, key), (TRUSTED_PROXIES_ENV, "nope")]).err(),
        Some(StartupError::TrustedProxy(_))
    ));

    let slashed = config_with(&[
        (COOKIE_KEY_ENV, key),
        (PUBLIC_BASE_URL_ENV, "https://ladder.example/"),
    ])
    .unwrap();
    assert_eq!(slashed.public_base_url, "https://ladder.example");
    assert!(matches!(
        config_with(&[(COOKIE_KEY_ENV, key), (PUBLIC_BASE_URL_ENV, "ftp://x")])
            .err(),
        Some(StartupError::PublicBaseUrl)
    ));
}
