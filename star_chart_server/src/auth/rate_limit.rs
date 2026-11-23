use std::{
    collections::HashMap,
    hash::Hash,
    net::{IpAddr, Ipv6Addr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::AppState;

const MAX_TRACKED: usize = 10_000;

/// The limits on `/login` and `/create_login`.
///
/// A school usually reaches the internet through one shared address, so the
/// per-address limit is loose and only stops floods. The strict limits count
/// wrong passwords per username, and wrong admin codes for the whole server.
pub struct LoginLimits {
    /// Every request, per client address
    pub per_ip: RateLimiter<IpAddr>,
    /// Wrong passwords, per username
    pub password_failures: RateLimiter<String>,
    /// Wrong admin codes, for the whole server
    pub admin_code_failures: RateLimiter<()>,
}

pub struct RateLimiter<K> {
    capacity: f64,
    per_second: f64,
    buckets: Mutex<HashMap<K, Bucket>>,
}

struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl<K: Eq + Hash> RateLimiter<K> {
    pub fn new(capacity: u32, refill_time: Duration) -> Self {
        Self {
            capacity: capacity as f64,
            per_second: capacity as f64 / refill_time.as_secs_f64(),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Uses up one attempt for `key`. If none are left, returns how long until the next one.
    pub fn check(&self, key: K) -> Result<(), Duration> {
        self.check_at(key, Instant::now())
    }

    /// Gives back an attempt taken by `check`, e.g. once a password turns out to be right
    pub fn refund(&self, key: &K) {
        let mut buckets = self.buckets.lock().unwrap();
        if let Some(bucket) = buckets.get_mut(key) {
            bucket.tokens = (bucket.tokens + 1.0).min(self.capacity);
        }
    }

    fn check_at(&self, key: K, now: Instant) -> Result<(), Duration> {
        let mut buckets = self.buckets.lock().unwrap();

        if buckets.len() >= MAX_TRACKED && !buckets.contains_key(&key) {
            // A full bucket is the same as no bucket, so those can go
            buckets.retain(|_, bucket| self.refill(bucket, now) < self.capacity);
            if buckets.len() >= MAX_TRACKED {
                return Err(Duration::from_secs_f64(1.0 / self.per_second));
            }
        }

        let bucket = buckets.entry(key).or_insert(Bucket {
            tokens: self.capacity,
            updated: now,
        });

        let tokens = self.refill(bucket, now);
        if tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Ok(())
        } else {
            Err(Duration::from_secs_f64((1.0 - tokens) / self.per_second))
        }
    }

    /// Adds the attempts earned since the bucket was last used, and returns the new count
    fn refill(&self, bucket: &mut Bucket, now: Instant) -> f64 {
        let elapsed = now.saturating_duration_since(bucket.updated).as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.per_second).min(self.capacity);
        bucket.updated = now;
        bucket.tokens
    }
}

/// The client address to rate limit by.
///
/// Behind a reverse proxy, every connection comes from the proxy on this machine,
/// and the proxy puts the real client address last in `X-Forwarded-For`. That
/// header is only trusted on connections from loopback: anyone else could put any
/// address in it to dodge the limit.
pub fn client_ip(peer: SocketAddr, headers: &HeaderMap) -> IpAddr {
    let ip = if peer.ip().is_loopback() {
        headers
            .get_all("x-forwarded-for")
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .last()
            .and_then(|last| last.trim().parse().ok())
            .unwrap_or(peer.ip())
    } else {
        peer.ip()
    };

    // An IPv6 user usually has a whole /64 block of addresses, so count the block as one
    match ip {
        IpAddr::V6(v6) => IpAddr::V6(Ipv6Addr::from(u128::from(v6) & (u128::MAX << 64))),
        v4 => v4,
    }
}

/// A `429 Too Many Requests` reply, with `Retry-After` in whole seconds
pub fn too_many_requests(wait: Duration) -> Response {
    let retry_after = (wait.as_secs_f64().ceil() as u64).to_string();
    (
        StatusCode::TOO_MANY_REQUESTS,
        [(header::RETRY_AFTER, retry_after)],
    )
        .into_response()
}

/// Refuses a request once its address has no attempts left.
/// It runs before the body is read or a password is hashed, so a refused request is cheap.
pub async fn login_rate_limit(
    State(state): State<Arc<AppState>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    let ip = client_ip(peer, request.headers());
    match state.login_limits.per_ip.check(ip) {
        Ok(()) => next.run(request).await,
        Err(wait) => {
            tracing::warn!(%ip, "login rate limit reached");
            too_many_requests(wait)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

    use axum::http::HeaderValue;

    use super::*;

    const IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    const OTHER_IP: IpAddr = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2));

    #[test]
    fn allows_a_burst_then_refuses() {
        // 3 attempts, one back every 20 seconds
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let now = Instant::now();

        for _ in 0..3 {
            assert!(limiter.check_at(IP, now).is_ok());
        }

        let wait = limiter.check_at(IP, now).unwrap_err();
        assert!((wait.as_secs_f64() - 20.0).abs() < 0.01, "waited {wait:?}");
    }

    #[test]
    fn attempts_come_back_over_time() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let now = Instant::now();
        for _ in 0..3 {
            limiter.check_at(IP, now).unwrap();
        }

        let later = now + Duration::from_secs(21);
        assert!(limiter.check_at(IP, later).is_ok());
        assert!(limiter.check_at(IP, later).is_err());
    }

    #[test]
    fn never_saves_up_more_than_the_burst() {
        let limiter = RateLimiter::new(3, Duration::from_secs(60));
        let now = Instant::now();
        limiter.check_at(IP, now).unwrap();

        let much_later = now + Duration::from_secs(60 * 60);
        for _ in 0..3 {
            assert!(limiter.check_at(IP, much_later).is_ok());
        }
        assert!(limiter.check_at(IP, much_later).is_err());
    }

    #[test]
    fn keys_are_limited_separately() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();

        assert!(limiter.check_at(IP, now).is_ok());
        assert!(limiter.check_at(IP, now).is_err());
        assert!(limiter.check_at(OTHER_IP, now).is_ok());
    }

    #[test]
    fn refund_gives_an_attempt_back() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();

        limiter.check_at("teacher1".to_string(), now).unwrap();
        limiter.refund(&"teacher1".to_string());
        assert!(limiter.check_at("teacher1".to_string(), now).is_ok());
    }

    fn forwarded_for(value: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_str(value).unwrap());
        headers
    }

    #[test]
    fn uses_forwarded_for_from_a_local_proxy() {
        let proxy: SocketAddr = "127.0.0.1:50000".parse().unwrap();

        // The proxy adds the real address last, anything before it came from the client
        let headers = forwarded_for("1.2.3.4, 81.2.69.160");
        assert_eq!(client_ip(proxy, &headers), "81.2.69.160".parse::<IpAddr>().unwrap());

        // No header, e.g. in development without a proxy
        assert_eq!(client_ip(proxy, &HeaderMap::new()), proxy.ip());
    }

    #[test]
    fn ignores_forwarded_for_from_anyone_else() {
        let client: SocketAddr = "81.2.69.160:50000".parse().unwrap();
        let headers = forwarded_for("10.9.9.9");

        assert_eq!(client_ip(client, &headers), client.ip());
    }

    #[test]
    fn counts_an_ipv6_block_as_one_address() {
        let a: SocketAddr = "[2001:db8:1:2::1]:50000".parse().unwrap();
        let b: SocketAddr = "[2001:db8:1:2:ffff::9]:50000".parse().unwrap();

        assert_eq!(client_ip(a, &HeaderMap::new()), client_ip(b, &HeaderMap::new()));
    }
}
