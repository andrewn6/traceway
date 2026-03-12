# Decompose Eval Runs into Per-Datapoint Jobs

**Labels:** `enhancement`, `backend`, `PRD-07`
**Difficulty:** Hard
**PRD:** [PRD-07: Eval Job Queue](../../prds/PRD-07-eval-job-queue.md) — Phases 3-4
**Depends on:** #008

## Summary

Refactor the eval runner from a single monolithic `tokio::spawn` task into a series of jobs processed via the job queue. Each datapoint evaluation becomes an independent, retryable, resumable job.

## Why this is hard

- Must decompose `execute_eval_run()` (currently ~150 lines doing everything sequentially) into two job types: `EvalRun` (orchestrator) and `EvalDatapoint` (per-item worker)
- The orchestrator must enqueue N child jobs and the aggregate must update correctly as children complete concurrently
- Cancellation must propagate: if a run is cancelled, in-flight datapoint jobs must check and bail
- SSE events must still fire correctly so the UI shows real-time progress
- Must handle partial completion: if 47 of 200 datapoints are done and the server restarts, the remaining 153 should be picked up

## Current architecture

```
create_eval_run handler
  └── tokio::spawn(execute_eval_run)
       └── for each datapoint:
            ├── HTTP POST to LLM
            ├── Score
            ├── Save EvalResult
            └── Update EvalRun aggregate + emit SSE
```

## Target architecture

```
create_eval_run handler
  └── enqueue Job { type: EvalRun { run_id } }

Worker claims EvalRun job:
  ├── Load datapoints
  ├── Enqueue Job { type: EvalDatapoint { run_id, dp_id } } for each
  └── Update run status to "running"

Worker claims EvalDatapoint job:
  ├── Check run.status != cancelled
  ├── HTTP POST to LLM (with retry from #004)
  ├── Score result
  ├── Save EvalResult
  ├── Atomically update EvalRun.results aggregate
  └── Emit SSE event
  └── If this is the last datapoint: mark run completed
```

## What to do

1. Define `JobType` enum:
   ```rust
   pub enum JobType {
       EvalRun { run_id: EvalRunId },
       EvalDatapoint { run_id: EvalRunId, datapoint_id: DatapointId },
   }
   ```

2. Implement `process_eval_run_job()` — the orchestrator that enqueues child jobs

3. Implement `process_eval_datapoint_job()` — processes one datapoint

4. Add a worker loop that claims and dispatches jobs based on type

5. Handle the "last datapoint" detection: after saving a result, check if `completed + failed == total` and if so, finalize the run

6. Handle cancellation: before processing a datapoint, check if the run is cancelled

7. Wire into `create_eval_run` handler: instead of `tokio::spawn(execute_eval_run)`, call `job_queue.enqueue(EvalRun job)`

8. The worker loop can run as a `tokio::spawn` in the same process for local mode

## Files to modify

- `crates/api/src/lib.rs` — change `create_eval_run` handler, remove/refactor `execute_eval_run`
- `crates/api/src/jobs.rs` — add job types, worker loop, processing functions

## Acceptance criteria

- [ ] Eval runs are processed via job queue (not direct `tokio::spawn`)
- [ ] Each datapoint is an independent job that can be retried
- [ ] Runs survive API server restart (pending jobs are re-claimed)
- [ ] Cancellation stops processing within one job cycle
- [ ] SSE events fire correctly (UI shows real-time progress as before)
- [ ] Run aggregate (completed/failed/scores) is correct after all jobs finish
- [ ] `cargo check -p api` succeeds
