# Implement SQLite-backed Job Queue

**Labels:** `enhancement`, `backend`, `PRD-07`
**Difficulty:** Medium
**PRD:** [PRD-07: Eval Job Queue](../../prds/PRD-07-eval-job-queue.md) — Phases 1-2

## Summary

Implement a durable job queue backed by a SQLite table. This replaces the unused `MemoryJobQueue` scaffolding in `crates/api/src/jobs.rs` with a real implementation that survives server restarts.

## Context

The eval runner currently executes in-process via `tokio::spawn`. If the server restarts, running eval jobs are lost. A SQLite job queue adds durability for local mode — jobs persist across restarts and can be claimed/completed atomically.

## What to do

1. Define the `JobQueue` trait in `crates/api/src/jobs.rs`:
   ```rust
   #[async_trait]
   pub trait JobQueue: Send + Sync {
       async fn enqueue(&self, job: Job) -> Result<JobId, JobError>;
       async fn claim(&self, worker_id: &str, timeout: Duration) -> Result<Option<Job>, JobError>;
       async fn complete(&self, job_id: JobId) -> Result<(), JobError>;
       async fn fail(&self, job_id: JobId, error: &str) -> Result<(), JobError>;
       async fn heartbeat(&self, job_id: JobId) -> Result<(), JobError>;
       async fn reap_stale(&self, stale_threshold: Duration) -> Result<usize, JobError>;
   }
   ```

2. Add a SQLite migration for the `jobs` table:
   ```sql
   CREATE TABLE IF NOT EXISTS jobs (
       id TEXT PRIMARY KEY,
       job_type TEXT NOT NULL,
       payload TEXT NOT NULL,
       status TEXT NOT NULL DEFAULT 'pending',
       attempts INTEGER NOT NULL DEFAULT 0,
       max_retries INTEGER NOT NULL DEFAULT 3,
       claimed_by TEXT,
       claimed_at TEXT,
       completed_at TEXT,
       created_at TEXT NOT NULL,
       scheduled_for TEXT NOT NULL,
       last_error TEXT
   );
   CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status, scheduled_for);
   ```

3. Implement `SqliteJobQueue` with atomic claim:
   ```sql
   UPDATE jobs SET status = 'claimed', claimed_by = ?, claimed_at = ?
   WHERE id = (SELECT id FROM jobs WHERE status = 'pending' AND scheduled_for <= ? ORDER BY scheduled_for LIMIT 1)
   RETURNING *;
   ```

4. Implement `reap_stale` for jobs claimed > threshold ago with no heartbeat.

## Files to modify

- `crates/api/src/jobs.rs` — replace scaffolding with real implementation
- `crates/storage-sqlite/src/lib.rs` — add migration (next version after v4)

## Acceptance criteria

- [ ] Jobs survive server restart (persisted in SQLite)
- [ ] `claim()` is atomic — two concurrent callers never claim the same job
- [ ] `fail()` increments attempt count; jobs with `attempts >= max_retries` become `dead`
- [ ] `reap_stale()` reclaims jobs stuck in `claimed` status
- [ ] `cargo check -p api` succeeds
- [ ] Unit test: enqueue 10 jobs, claim them all, verify no duplicates
