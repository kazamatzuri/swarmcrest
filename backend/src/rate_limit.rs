// In-memory rate limiter for challenge endpoints.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, Instant};

/// Different rate limit types with their constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RateLimitType {
    /// Max concurrent live games per user.
    LiveGames,
    /// Max live challenges per hour.
    LiveChallenges,
    /// Max headless challenges per hour.
    HeadlessChallenges,
}

impl RateLimitType {
    /// Maximum number of events allowed in the window.
    pub fn max_count(&self) -> usize {
        match self {
            RateLimitType::LiveGames => 3,
            RateLimitType::LiveChallenges => 10,
            RateLimitType::HeadlessChallenges => 100,
        }
    }

    /// Time window for the rate limit.
    pub fn window(&self) -> Duration {
        match self {
            // LiveGames is a concurrency limit, but we model it as a short window
            // that gets cleaned up when games end. Use a large window.
            RateLimitType::LiveGames => Duration::from_secs(3600),
            RateLimitType::LiveChallenges => Duration::from_secs(3600),
            RateLimitType::HeadlessChallenges => Duration::from_secs(3600),
        }
    }
}

impl std::fmt::Display for RateLimitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RateLimitType::LiveGames => write!(f, "live games"),
            RateLimitType::LiveChallenges => write!(f, "live challenges per hour"),
            RateLimitType::HeadlessChallenges => write!(f, "headless challenges per hour"),
        }
    }
}

/// Error returned when a rate limit is exceeded.
#[derive(Debug, Clone)]
pub struct RateLimitError {
    pub limit_type: RateLimitType,
    pub max: usize,
}

impl std::fmt::Display for RateLimitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Rate limit exceeded: max {} {}",
            self.max, self.limit_type
        )
    }
}

/// Key for the rate limit map: (user_id, limit_type).
type LimitKey = (i64, RateLimitType);

/// Thread-safe in-memory rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimiter {
    inner: Arc<Mutex<HashMap<LimitKey, Vec<Instant>>>>,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the user is within the rate limit for the given type.
    /// If within limits, records the event and returns Ok(()).
    /// If exceeded, returns Err(RateLimitError).
    /// In local mode, rate limiting is always bypassed.
    pub fn check_limit(
        &self,
        user_id: i64,
        limit_type: RateLimitType,
    ) -> Result<(), RateLimitError> {
        if crate::config::is_local_mode() {
            return Ok(());
        }
        let mut map = self.inner.lock().unwrap();
        let key = (user_id, limit_type);
        let window = limit_type.window();
        let max = limit_type.max_count();
        let now = Instant::now();

        let entries = map.entry(key).or_insert_with(Vec::new);

        // Remove expired entries
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max {
            return Err(RateLimitError { limit_type, max });
        }

        entries.push(now);
        Ok(())
    }

    /// Remove one event from the rate limiter (e.g., when a live game ends).
    /// This is useful for the LiveGames concurrency limit.
    pub fn release(&self, user_id: i64, limit_type: RateLimitType) {
        let mut map = self.inner.lock().unwrap();
        let key = (user_id, limit_type);
        if let Some(entries) = map.get_mut(&key) {
            // Remove the oldest entry
            if !entries.is_empty() {
                entries.remove(0);
            }
        }
    }

    /// Get the current count for a user and limit type (for testing/diagnostics).
    pub fn current_count(&self, user_id: i64, limit_type: RateLimitType) -> usize {
        let mut map = self.inner.lock().unwrap();
        let key = (user_id, limit_type);
        let window = limit_type.window();
        let now = Instant::now();

        if let Some(entries) = map.get_mut(&key) {
            entries.retain(|t| now.duration_since(*t) < window);
            entries.len()
        } else {
            0
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

// ── Per-IP rate limiter for auth endpoints ────────────────────────────

/// Simple per-IP rate limiter for auth endpoints.
#[derive(Debug, Clone)]
pub struct IpRateLimiter {
    inner: Arc<Mutex<HashMap<IpAddr, Vec<Instant>>>>,
}

impl IpRateLimiter {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Check if the IP is within the rate limit.
    /// Returns Ok(()) if allowed, Err with message if rate limited.
    pub fn check(&self, ip: IpAddr, max_requests: usize, window: Duration) -> Result<(), String> {
        if crate::config::is_local_mode() {
            return Ok(());
        }
        let mut map = self.inner.lock().unwrap();
        let now = Instant::now();
        let entries = map.entry(ip).or_insert_with(Vec::new);
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= max_requests {
            return Err(format!(
                "Rate limit exceeded: max {} requests per {} seconds",
                max_requests,
                window.as_secs()
            ));
        }

        entries.push(now);
        Ok(())
    }
}

impl Default for IpRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

static AUTH_IP_LIMITER: LazyLock<IpRateLimiter> = LazyLock::new(IpRateLimiter::new);

/// Check auth endpoint rate limit for an IP.
/// Default: 10 requests per 60 seconds. Skipped in local mode.
/// Override with `AUTH_RATE_LIMIT` env var (e.g. `AUTH_RATE_LIMIT=1000`).
pub fn check_auth_rate_limit(ip: IpAddr) -> Result<(), String> {
    if crate::config::is_local_mode() {
        return Ok(());
    }
    let max: usize = std::env::var("AUTH_RATE_LIMIT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(10);
    AUTH_IP_LIMITER.check(ip, max, Duration::from_secs(60))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new();

        // LiveChallenges allows 10 per hour
        for _ in 0..10 {
            assert!(limiter
                .check_limit(1, RateLimitType::LiveChallenges)
                .is_ok());
        }
    }

    #[test]
    fn test_rate_limiter_denies_over_limit() {
        let limiter = RateLimiter::new();

        // LiveGames allows 3 concurrent
        for _ in 0..3 {
            assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_ok());
        }
        // 4th should fail
        let result = limiter.check_limit(1, RateLimitType::LiveGames);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.max, 3);
        assert_eq!(err.limit_type, RateLimitType::LiveGames);
    }

    #[test]
    fn test_rate_limiter_separate_users() {
        let limiter = RateLimiter::new();

        // Fill up user 1's live games
        for _ in 0..3 {
            assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_ok());
        }
        assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_err());

        // User 2 should still be fine
        assert!(limiter.check_limit(2, RateLimitType::LiveGames).is_ok());
    }

    #[test]
    fn test_rate_limiter_separate_types() {
        let limiter = RateLimiter::new();

        // Fill up live games for user 1
        for _ in 0..3 {
            assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_ok());
        }
        assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_err());

        // Live challenges should still work for user 1
        assert!(limiter
            .check_limit(1, RateLimitType::LiveChallenges)
            .is_ok());
    }

    #[test]
    fn test_rate_limiter_release() {
        let limiter = RateLimiter::new();

        // Fill up live games
        for _ in 0..3 {
            assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_ok());
        }
        assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_err());

        // Release one
        limiter.release(1, RateLimitType::LiveGames);

        // Now should allow one more
        assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_ok());
        assert!(limiter.check_limit(1, RateLimitType::LiveGames).is_err());
    }

    #[test]
    fn test_rate_limiter_current_count() {
        let limiter = RateLimiter::new();

        assert_eq!(limiter.current_count(1, RateLimitType::LiveGames), 0);

        limiter.check_limit(1, RateLimitType::LiveGames).unwrap();
        assert_eq!(limiter.current_count(1, RateLimitType::LiveGames), 1);

        limiter.check_limit(1, RateLimitType::LiveGames).unwrap();
        assert_eq!(limiter.current_count(1, RateLimitType::LiveGames), 2);
    }

    #[test]
    fn test_headless_challenges_limit() {
        let limiter = RateLimiter::new();

        // HeadlessChallenges allows 100 per hour
        for _ in 0..100 {
            assert!(limiter
                .check_limit(1, RateLimitType::HeadlessChallenges)
                .is_ok());
        }
        // 101st should be rejected
        assert!(limiter
            .check_limit(1, RateLimitType::HeadlessChallenges)
            .is_err());
    }

    #[test]
    fn test_rate_limit_error_display() {
        let err = RateLimitError {
            limit_type: RateLimitType::LiveChallenges,
            max: 10,
        };
        assert_eq!(
            err.to_string(),
            "Rate limit exceeded: max 10 live challenges per hour"
        );
    }
}
