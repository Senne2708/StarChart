use std::{
    collections::HashMap,
    net::{IpAddr, SocketAddr},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::AppState;

const MAX_TRACKED: usize = 10_000;

pub struct RateLimiter {
    capacity: f64,
    per_second: f64,
    buckets: Mutex<HashMap<IpAddr, Bucket>>,
}

struct Bucket {
    tokens: f64,
    updated: Instant,
}

impl RateLimiter {
    pub fn new(capacity: u32, refill_time: Duration) -> Self {
        Self {
            capacity: capacity as f64,
            per_second: capacity as f64 / refill_time.as_secs_f64(),
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Uses up one attempt for `ip`. If none are left, returns how long until the next one.
    pub fn check(&self, ip: IpAddr) -> Result<(), Duration> {
        self.check_at(ip, Instant::now())
    }

    fn check_at(&self, ip: IpAddr, now: Instant) -> Result<(), Duration> {
        let mut buckets = self.buckets.lock().unwrap();

        if buckets.len() >= MAX_TRACKED && !buckets.contains_key(&ip) {
            // A full bucket is the same as no bucket, so those can go
            buckets.retain(|_, bucket| self.refill(bucket, now) < self.capacity);
            if buckets.len() >= MAX_TRACKED {
                return Err(Duration::from_secs_f64(1.0 / self.per_second));
            }
        }

        let bucket = buckets.entry(ip).or_insert(Bucket {
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

/// Refuses a request with `429 Too Many Requests` once its address has no attempts left.
/// It runs before the body is read or a password is hashed, so a refused request is cheap.
pub async fn login_rate_limit(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    request: Request,
    next: Next,
) -> Response {
    match state.login_limit.check(addr.ip()) {
        Ok(()) => next.run(request).await,
        Err(wait) => {
            tracing::warn!(ip = %addr.ip(), "login rate limit reached");
            let retry_after = (wait.as_secs_f64().ceil() as u64).to_string();
            (
                StatusCode::TOO_MANY_REQUESTS,
                [(header::RETRY_AFTER, retry_after)],
            )
                .into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::Ipv4Addr;

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
    fn addresses_are_limited_separately() {
        let limiter = RateLimiter::new(1, Duration::from_secs(60));
        let now = Instant::now();

        assert!(limiter.check_at(IP, now).is_ok());
        assert!(limiter.check_at(IP, now).is_err());
        assert!(limiter.check_at(OTHER_IP, now).is_ok());
    }
}
