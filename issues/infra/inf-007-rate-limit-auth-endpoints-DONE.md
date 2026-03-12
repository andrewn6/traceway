# Add Rate Limiting to Login and Password Reset Endpoints

**Labels:** `enhancement`, `backend`
**Difficulty:** Easy
**Priority:** P1

## Summary

The `POST /api/auth/login` and `POST /api/auth/password-reset` endpoints have no rate limiting. An attacker can brute-force passwords or flood the password reset flow with unlimited requests.

## What to do

1. Add an in-memory rate limiter (per IP address) to the login and password-reset endpoints. Use a token bucket or sliding window approach.

2. Recommended limits:
   - Login: 10 attempts per IP per minute, then 429 Too Many Requests
   - Password reset: 3 requests per email per hour, 10 per IP per hour

3. Implementation options:
   - Use `tower::limit::RateLimitLayer` if it fits Axum's middleware model
   - Or a simple `DashMap<IpAddr, (count, window_start)>` with periodic cleanup
   - For cloud mode, consider Redis-backed rate limiting (the `RedisEventBus` already connects to Redis)

4. Return `429 Too Many Requests` with a `Retry-After` header.

5. Log rate-limited attempts at `warn` level for monitoring.

## Files to modify

- `crates/api/src/auth_routes.rs` — Add rate limit middleware or inline checks to login/password-reset handlers

## Acceptance criteria

- [ ] Login endpoint returns 429 after 10 failed attempts from same IP in 1 minute
- [ ] Password reset returns 429 after 3 requests for same email in 1 hour
- [ ] `Retry-After` header is set on 429 responses
- [ ] Legitimate users are not affected by normal usage
- [ ] `cargo check -p api` passes
