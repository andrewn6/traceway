# Implement Job Queue Worker (Process Enqueued Jobs)

**Labels:** `enhancement`, `backend`
**Difficulty:** Medium
**Priority:** P2

## Summary

The job system in `crates/api/src/jobs.rs` is write-only: jobs can be enqueued but no worker process ever picks them up and executes them. The `MemoryJobQueue` stores jobs in a HashMap but has no `poll` / `claim` / `process` loop.

## Context

This is the runtime counterpart to #008 (SQLite-backed job queue storage). Even if #008 adds durable storage, without a worker loop, jobs sit in the queue forever. This issue adds the processing side.

## What to do

1. Add a background `tokio::spawn` task that polls the job queue on an interval (e.g., every 1-5 seconds)

2. The worker should:
   - Claim the next pending job (atomically mark it as `processing`)
   - Execute the job based on its type (eval run, backfill, etc.)
   - Mark as `completed` or `failed` with error details
   - Respect a configurable concurrency limit (e.g., max 4 concurrent jobs)

3. Integrate with the eval runner: when an eval run is submitted, it should enqueue a job rather than spawning a raw `tokio::spawn`

4. Add a `GET /api/jobs` endpoint to list recent jobs with status (for the settings/admin page)

## Files to modify

- `crates/api/src/jobs.rs` — Add worker loop, claim/complete/fail methods
- `crates/api/src/lib.rs` — Spawn worker on startup, add jobs list endpoint

## Acceptance criteria

- [ ] Worker processes enqueued jobs
- [ ] Jobs transition through pending → processing → completed/failed
- [ ] Failed jobs include error details
- [ ] Concurrency limit is respected
- [ ] `GET /api/jobs` returns recent job history
- [ ] `cargo check -p api` passes
