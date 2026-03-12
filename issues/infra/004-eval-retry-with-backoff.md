# Add Retry with Exponential Backoff to Eval LLM Calls

**Labels:** `good first issue`, `backend`, `PRD-07`
**Difficulty:** Easy
**PRD:** [PRD-07: Eval Job Queue](../../prds/PRD-07-eval-job-queue.md) — Phase 5

## Summary

The eval runner currently makes a single HTTP call per datapoint. If the LLM provider returns a transient error (429 rate limit, 500, 503), the result fails permanently. Add retry logic with exponential backoff to the LLM call in `execute_eval_run`.

## Context

The eval runner lives in `crates/api/src/lib.rs` inside `execute_eval_run()`. It uses `reqwest` to POST to `/v1/chat/completions`. This is a self-contained change that doesn't require the full job queue system.

## What to do

1. Add a `RetryConfig` struct:
   ```rust
   pub struct RetryConfig {
       pub max_retries: u32,        // default: 3
       pub initial_backoff_ms: u64,  // default: 1000
       pub max_backoff_ms: u64,      // default: 60000
       pub multiplier: f64,          // default: 2.0
   }
   ```

2. Create a `call_llm_with_retry()` wrapper around the existing HTTP call logic
3. Retry on HTTP status codes: 429, 500, 502, 503, 504, and on `reqwest::Error` where `is_connect()` or `is_timeout()` is true
4. Log each retry with `tracing::warn!`
5. Replace the direct HTTP call in `execute_eval_run` with the retry wrapper

## Files to modify

- `crates/api/src/lib.rs` — add retry wrapper, use it in `execute_eval_run`

## Acceptance criteria

- [ ] Transient HTTP errors are retried up to `max_retries` times
- [ ] Backoff doubles each attempt, capped at `max_backoff_ms`
- [ ] Non-retryable errors (400, 401, 403, 404) fail immediately
- [ ] Each retry is logged with attempt number and backoff duration
- [ ] `cargo check -p api` succeeds
