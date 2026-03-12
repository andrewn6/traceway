# Normalize Email Addresses in Auth

**Labels:** `bug`, `backend`
**Difficulty:** Easy
**Priority:** P2

## Summary

Email addresses are stored as-is in the auth database. `User@Example.com` and `user@example.com` are treated as different accounts. This causes login failures, duplicate accounts, and invite confusion.

## What to do

1. Normalize emails on input: lowercase the entire email, trim whitespace

2. Apply normalization in all auth endpoints:
   - `POST /api/auth/signup`
   - `POST /api/auth/login`
   - `POST /api/auth/invite`
   - `POST /api/auth/password-reset`

3. Add a helper function:

```rust
fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
```

4. Add a one-time migration to normalize existing emails in the `users` table (handle potential duplicates gracefully)

## Files to modify

- `crates/api/src/auth_routes.rs` — Apply `normalize_email` to all email inputs
- `crates/storage-postgres/src/lib.rs` — Optional: add migration to normalize existing data

## Acceptance criteria

- [ ] `User@Example.com` and `user@example.com` resolve to the same account
- [ ] Login works regardless of email casing
- [ ] Invites work regardless of email casing
- [ ] `cargo check -p api` passes
