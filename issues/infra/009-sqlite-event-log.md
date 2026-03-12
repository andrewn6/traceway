# Implement SQLite Event Log for Durable Events

**Labels:** `enhancement`, `backend`, `PRD-10`
**Difficulty:** Medium
**PRD:** [PRD-10: Durable Event Log](../../prds/PRD-10-durable-event-log.md) — Phases 1-2

## Summary

Create an `EventLog` trait and a SQLite implementation that stores `SystemEvent`s in an append-only table with auto-incrementing sequence numbers. This enables event replay on SSE reconnect and lays the groundwork for CDC consumers.

## Context

Today, events are broadcast via `tokio::broadcast::channel` — ephemeral, in-memory, dropped if no one's listening. This issue adds a durable log so events survive restarts and can be replayed.

## What to do

1. Define the `EventLog` trait in `crates/api/src/events.rs`:
   ```rust
   #[async_trait]
   pub trait EventLog: Send + Sync {
       async fn append(&self, org_id: &str, event: &SystemEvent) -> Result<u64, EventError>;
       async fn read_from(&self, org_id: &str, sequence: u64, limit: usize) -> Result<Vec<StoredEvent>, EventError>;
       async fn trim(&self, older_than: Duration) -> Result<usize, EventError>;
   }

   pub struct StoredEvent {
       pub sequence: u64,
       pub event: SystemEvent,
       pub timestamp: DateTime<Utc>,
       pub org_id: String,
   }
   ```

2. Add SQLite migration for the `event_log` table:
   ```sql
   CREATE TABLE IF NOT EXISTS event_log (
       sequence INTEGER PRIMARY KEY AUTOINCREMENT,
       event_type TEXT NOT NULL,
       event_data TEXT NOT NULL,
       org_id TEXT NOT NULL,
       created_at TEXT NOT NULL
   );
   CREATE INDEX IF NOT EXISTS idx_event_log_org_seq ON event_log(org_id, sequence);
   CREATE INDEX IF NOT EXISTS idx_event_log_created ON event_log(created_at);
   ```

3. Implement `SqliteEventLog` with:
   - `append`: INSERT and return the AUTOINCREMENT sequence
   - `read_from`: SELECT WHERE sequence > ? LIMIT ?
   - `trim`: DELETE WHERE created_at < ?

4. Wire it into the API: after `event_tx.send()`, also call `event_log.append()`.

## Files to modify

- `crates/api/src/events.rs` — add trait + `StoredEvent` type
- `crates/storage-sqlite/src/lib.rs` — add migration
- New file `crates/api/src/event_log.rs` for `SqliteEventLog` implementation
- `crates/api/src/lib.rs` — wire append calls into handlers that emit events

## Acceptance criteria

- [ ] Events are persisted to SQLite `event_log` table
- [ ] `read_from(seq)` returns events after the given sequence number
- [ ] `trim(duration)` deletes old events
- [ ] Existing SSE/broadcast behavior is unchanged (dual-write: broadcast + log)
- [ ] `cargo check -p api` succeeds
- [ ] Unit test: append 5 events, read_from(0) returns all 5, read_from(3) returns last 2
