//! In-memory rate limiter using a sliding window approach.
//!
//! Each key (e.g. IP address or email) tracks a list of request timestamps.
//! On each check, expired entries are pruned and the count is compared against
//! the configured limit.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::http::HeaderMap;

/// A sliding-window rate limiter.
pub struct RateLimiter {
    /// Map of key → list of request timestamps within the window.
    entries: Mutex<HashMap<String, Vec<Instant>>>,
    /// Maximum number of requests allowed within the window.
    max_requests: u32,
    /// Duration of the sliding window.
    window: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter.
    ///
    /// - `max_requests`: number of requests allowed per key within the window
    /// - `window`: duration of the sliding window
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
            max_requests,
            window,
        }
    }

    /// Check if the key is allowed to make a request.
    ///
    /// Returns `Ok(())` if allowed, or `Err(retry_after_secs)` if rate limited.
    pub fn check(&self, key: &str) -> Result<(), u64> {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());

        let timestamps = entries.entry(key.to_string()).or_default();

        // Prune expired entries
        timestamps.retain(|t| now.duration_since(*t) < self.window);

        if timestamps.len() >= self.max_requests as usize {
            // Calculate retry-after from the oldest entry in the window
            let oldest = timestamps.first().copied().unwrap_or(now);
            let retry_after = self
                .window
                .checked_sub(now.duration_since(oldest))
                .unwrap_or(Duration::from_secs(1));
            return Err(retry_after.as_secs().max(1));
        }

        timestamps.push(now);
        Ok(())
    }

    /// Periodically clean up entries that have fully expired.
    /// Call this from a background task every few minutes.
    pub fn cleanup(&self) {
        let now = Instant::now();
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        entries.retain(|_, timestamps| {
            timestamps.retain(|t| now.duration_since(*t) < self.window);
            !timestamps.is_empty()
        });
    }
}

/// Extract the client IP address from request headers.
///
/// Checks `X-Forwarded-For` first (first IP in the chain), then `X-Real-IP`,
/// then falls back to `"unknown"`.
pub fn client_ip(headers: &HeaderMap) -> String {
    // X-Forwarded-For: client, proxy1, proxy2
    if let Some(xff) = headers.get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(first) = xff.split(',').next() {
            let ip = first.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }
    // X-Real-IP: client
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|v| v.to_str().ok()) {
        let ip = real_ip.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }
    "unknown".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_under_limit() {
        let rl = RateLimiter::new(3, Duration::from_secs(60));
        assert!(rl.check("ip1").is_ok());
        assert!(rl.check("ip1").is_ok());
        assert!(rl.check("ip1").is_ok());
    }

    #[test]
    fn blocks_over_limit() {
        let rl = RateLimiter::new(2, Duration::from_secs(60));
        assert!(rl.check("ip1").is_ok());
        assert!(rl.check("ip1").is_ok());
        assert!(rl.check("ip1").is_err());
    }

    #[test]
    fn separate_keys_independent() {
        let rl = RateLimiter::new(1, Duration::from_secs(60));
        assert!(rl.check("ip1").is_ok());
        assert!(rl.check("ip2").is_ok());
        assert!(rl.check("ip1").is_err());
        assert!(rl.check("ip2").is_err());
    }

    #[test]
    fn cleanup_removes_expired() {
        let rl = RateLimiter::new(100, Duration::from_nanos(1));
        rl.check("ip1").ok();
        // Entries should already be expired (1ns window)
        std::thread::sleep(Duration::from_millis(1));
        rl.cleanup();
        let entries = rl.entries.lock().unwrap();
        assert!(entries.is_empty());
    }
}
