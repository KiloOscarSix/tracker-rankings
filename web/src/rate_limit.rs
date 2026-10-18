use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;

use governor::clock::{Clock, DefaultClock};
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use topcoat::context::Cx;
use topcoat::router::error::too_many_requests;
use topcoat::router::request::client_ip;
use topcoat::router::{Body, LayerFn, LayerFuture, Next};

use crate::context::rate_limiters;

type KeyedLimiter =
    RateLimiter<IpAddr, DefaultKeyedStateStore<IpAddr>, DefaultClock>;

const AUTH_BURST: NonZeroU32 = NonZeroU32::MIN.saturating_add(9);
/// Sign-in and sign-out attempts per client: 10 per minute.
pub const AUTH_QUOTA: Quota = Quota::per_minute(AUTH_BURST);
const VOTE_BURST: NonZeroU32 = NonZeroU32::MIN.saturating_add(29);
/// Vote and data-deletion posts per client: 30 per minute.
pub const VOTE_QUOTA: Quota = Quota::per_minute(VOTE_BURST);
const CLEANUP_INTERVAL: Duration = Duration::from_mins(10);

pub struct RateLimiters {
    auth: KeyedLimiter,
    votes: KeyedLimiter,
}

impl RateLimiters {
    #[must_use]
    pub fn new(auth: Quota, votes: Quota) -> Self {
        Self { auth: RateLimiter::keyed(auth), votes: RateLimiter::keyed(votes) }
    }

    /// `Err` carries the Retry-After seconds (at least 1).
    pub fn check_auth(&self, ip: Option<IpAddr>) -> Result<(), u64> {
        Self::check(&self.auth, ip)
    }

    /// `Err` carries the Retry-After seconds (at least 1).
    pub fn check_votes(&self, ip: Option<IpAddr>) -> Result<(), u64> {
        Self::check(&self.votes, ip)
    }

    pub fn retain_recent(&self) {
        self.auth.retain_recent();
        self.votes.retain_recent();
    }

    fn check(limiter: &KeyedLimiter, ip: Option<IpAddr>) -> Result<(), u64> {
        limiter.check_key(&bucket(ip)).map_err(|not_until| {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            wait.as_secs().saturating_add(u64::from(wait.subsec_nanos() > 0)).max(1)
        })
    }
}

pub fn spawn_rate_limit_cleanup(limiters: Arc<RateLimiters>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(CLEANUP_INTERVAL);
        loop {
            ticker.tick().await;
            limiters.retain_recent();
        }
    });
}

#[must_use]
pub fn auth_rate_limit() -> LayerFn {
    LayerFn::new(Some("/auth"), limit_auth)
}

#[must_use]
pub fn logout_rate_limit() -> LayerFn {
    LayerFn::new(Some("/logout"), limit_auth)
}

#[must_use]
pub fn vote_rate_limit() -> LayerFn {
    LayerFn::new(Some("/vote"), limit_votes)
}

#[must_use]
pub fn delete_rate_limit() -> LayerFn {
    LayerFn::new(Some("/me/delete"), limit_votes)
}

fn limit_auth<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    limited(cx, body, next, RateLimiters::check_auth)
}

fn limit_votes<'a>(cx: &'a Cx, body: Body, next: Next<'a>) -> LayerFuture<'a> {
    limited(cx, body, next, RateLimiters::check_votes)
}

fn limited<'a>(
    cx: &'a Cx,
    body: Body,
    next: Next<'a>,
    check: fn(&RateLimiters, Option<IpAddr>) -> Result<(), u64>,
) -> LayerFuture<'a> {
    Box::pin(async move {
        if let Err(secs) = check(rate_limiters(cx)?, client_ip(cx)) {
            return Err(too_many_requests(secs).into());
        }
        next.run(cx, body).await
    })
}

fn bucket(ip: Option<IpAddr>) -> IpAddr {
    match ip {
        None => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        Some(IpAddr::V4(addr)) => IpAddr::V4(addr),
        Some(IpAddr::V6(addr)) => addr.to_ipv4_mapped().map_or_else(
            || {
                let [s0, s1, s2, s3, ..] = addr.segments();
                IpAddr::V6(Ipv6Addr::new(s0, s1, s2, s3, 0, 0, 0, 0))
            },
            IpAddr::V4,
        ),
    }
}
