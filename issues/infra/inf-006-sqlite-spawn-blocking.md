# SQLite: Use spawn_blocking for Synchronous I/O

**Labels:** `enhancement`, `backend`
**Difficulty:** Easy
**Priority:** P1

## Summary

The SQLite backend performs synchronous disk I/O (via `rusqlite`) directly on the Tokio async runtime. This blocks the executor thread during every database operation. Under load, this starves other async tasks (SSE events, HTTP handlers) because Tokio's thread pool is occupied by blocked SQLite calls.

## Where

`crates/storage-sqlite/src/lib.rs` — every method that calls `self.conn.lock()` and executes SQL.

The current pattern:

```rust
async fn save_trace(&self, trace: &Trace) -> Result<()> {
    let conn = self.conn.lock().unwrap(); // Mutex::lock, not async
    conn.execute("INSERT INTO traces ...", params![...])?; // sync I/O
    Ok(())
}
```

## What to do

Wrap synchronous SQLite calls in `tokio::task::spawn_blocking`:

```rust
async fn save_trace(&self, trace: &Trace) -> Result<()> {
    let conn = self.conn.clone(); // Arc<Mutex<Connection>>
    let trace = trace.clone();
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        conn.execute("INSERT INTO traces ...", params![...])?;
        Ok(())
    }).await?
}
```

This moves the blocking I/O to Tokio's blocking thread pool, freeing the async executor.

Apply this pattern to all methods in the SQLite backend. Consider extracting a helper:

```rust
async fn with_conn<F, R>(&self, f: F) -> Result<R>
where
    F: FnOnce(&Connection) -> Result<R> + Send + 'static,
    R: Send + 'static,
{
    let conn = self.conn.clone();
    tokio::task::spawn_blocking(move || {
        let conn = conn.lock().unwrap();
        f(&conn)
    }).await?
}
```

## Acceptance criteria

- [ ] No synchronous SQLite I/O on the async runtime
- [ ] All SQLite backend methods use `spawn_blocking` or equivalent
- [ ] SSE events are not delayed by concurrent SQLite operations
- [ ] `cargo check -p storage-sqlite` passes
