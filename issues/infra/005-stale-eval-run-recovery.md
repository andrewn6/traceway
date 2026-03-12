# Recover Stale Eval Runs Stuck in "Running" Status

**Labels:** `good first issue`, `backend`, `PRD-09`
**Difficulty:** Easy
**PRD:** [PRD-09: Orphan Cleanup](../../prds/PRD-09-orphan-cleanup.md) — Phase 4

## Summary

If the API server crashes while an eval run is executing, the run stays in `running` status forever. Add a recovery function that detects stale runs and marks them as `failed`.

## What to do

1. Add a `recover_stale_eval_runs()` function (in `crates/api/src/gc.rs` or a new module)
2. Query all eval runs with `status == "running"`
3. If `created_at` is older than a threshold (default: 30 minutes) and `completed_at` is `None`, mark the run as `failed` with `completed_at = now`
4. Wire this into the `POST /admin/gc/stale-runs` endpoint
5. Also call it from the GC background loop (if it exists) or as a standalone background task on a timer

## Files to modify

- `crates/api/src/gc.rs` — add `recover_stale_eval_runs()` function
- `crates/api/src/lib.rs` — add route `POST /admin/gc/stale-runs`

## Acceptance criteria

- [ ] Eval runs stuck in `running` for > 30 minutes are marked `failed`
- [ ] Runs that are legitimately running (recent `created_at`) are left alone
- [ ] Returns count of recovered runs
- [ ] `cargo check -p api` succeeds
