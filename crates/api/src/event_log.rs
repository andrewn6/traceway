//! SQLite-backed durable event log for SSE replay.
//!
//! Stores `SystemEvent`s in an append-only table with auto-incrementing sequence
//! numbers. Supports reading events after a given sequence (for replay on reconnect)
//! and trimming old events by age.

use async_trait::async_trait;
use chrono::Utc;
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::{error, info};

use crate::events::{EventLog, EventLogError, StoredEvent};
use crate::SystemEvent;

/// SQLite-backed event log.
pub struct SqliteEventLog {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteEventLog {
    /// Open (or create) the event log database at the given path.
    pub fn open(path: &Path) -> Result<Self, EventLogError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| EventLogError::Storage(e.to_string()))?;
        }
        let conn = Connection::open(path)
            .map_err(|e| EventLogError::Storage(e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| EventLogError::Storage(e.to_string()))?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Create an in-memory event log (for tests).
    pub fn memory() -> Result<Self, EventLogError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| EventLogError::Storage(e.to_string()))?;
        Self::run_migrations(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    fn run_migrations(conn: &Connection) -> Result<(), EventLogError> {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS event_log (
                sequence INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                org_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_event_log_org_seq ON event_log(org_id, sequence);
            CREATE INDEX IF NOT EXISTS idx_event_log_created ON event_log(created_at);
            "#,
        )
        .map_err(|e| EventLogError::Storage(e.to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl EventLog for SqliteEventLog {
    async fn append(&self, org_id: &str, event: &SystemEvent) -> Result<u64, EventLogError> {
        let event_data = serde_json::to_string(event)
            .map_err(|e| EventLogError::Serialization(e.to_string()))?;

        // Extract event type from the serde tag
        let event_type = event_type_name(event);
        let now = Utc::now().to_rfc3339();
        let org_id = org_id.to_string();

        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            conn.execute(
                "INSERT INTO event_log (event_type, event_data, org_id, created_at) VALUES (?1, ?2, ?3, ?4)",
                params![event_type, event_data, org_id, now],
            )
            .map_err(|e| EventLogError::Storage(e.to_string()))?;
            let seq = conn.last_insert_rowid() as u64;
            Ok(seq)
        })
        .await
        .map_err(|e| EventLogError::Storage(format!("spawn_blocking join error: {e}")))?
    }

    async fn read_after(&self, after_sequence: u64, limit: usize) -> Result<Vec<StoredEvent>, EventLogError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let mut stmt = conn
                .prepare(
                    "SELECT sequence, event_data, org_id, created_at FROM event_log WHERE sequence > ?1 ORDER BY sequence ASC LIMIT ?2",
                )
                .map_err(|e| EventLogError::Storage(e.to_string()))?;

            let rows = stmt
                .query_map(params![after_sequence as i64, limit as i64], |row| {
                    let sequence: i64 = row.get(0)?;
                    let event_data: String = row.get(1)?;
                    let org_id: String = row.get(2)?;
                    let created_at: String = row.get(3)?;
                    Ok((sequence as u64, event_data, org_id, created_at))
                })
                .map_err(|e| EventLogError::Storage(e.to_string()))?;

            let mut events = Vec::new();
            for row in rows {
                let (sequence, event_data, org_id, created_at) =
                    row.map_err(|e| EventLogError::Storage(e.to_string()))?;
                let event: SystemEvent = serde_json::from_str(&event_data)
                    .map_err(|e| EventLogError::Serialization(e.to_string()))?;
                let timestamp = chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|dt| dt.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now());
                events.push(StoredEvent {
                    sequence,
                    event,
                    timestamp,
                    org_id,
                });
            }
            Ok(events)
        })
        .await
        .map_err(|e| EventLogError::Storage(format!("spawn_blocking join error: {e}")))?
    }

    async fn trim(&self, max_age: Duration) -> Result<usize, EventLogError> {
        let cutoff = Utc::now() - chrono::Duration::from_std(max_age)
            .unwrap_or_else(|_| chrono::Duration::hours(24));
        let cutoff_str = cutoff.to_rfc3339();

        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let deleted = conn
                .execute(
                    "DELETE FROM event_log WHERE created_at < ?1",
                    params![cutoff_str],
                )
                .map_err(|e| EventLogError::Storage(e.to_string()))?;
            Ok(deleted)
        })
        .await
        .map_err(|e| EventLogError::Storage(format!("spawn_blocking join error: {e}")))?
    }

    async fn latest_sequence(&self) -> Result<u64, EventLogError> {
        let conn = self.conn.clone();
        tokio::task::spawn_blocking(move || {
            let conn = conn.blocking_lock();
            let seq: i64 = conn
                .query_row(
                    "SELECT COALESCE(MAX(sequence), 0) FROM event_log",
                    [],
                    |row| row.get(0),
                )
                .map_err(|e| EventLogError::Storage(e.to_string()))?;
            Ok(seq as u64)
        })
        .await
        .map_err(|e| EventLogError::Storage(format!("spawn_blocking join error: {e}")))?
    }
}

/// Extract the serde tag name for a SystemEvent variant.
fn event_type_name(event: &SystemEvent) -> &'static str {
    match event {
        SystemEvent::SpanCreated { .. } => "span_created",
        SystemEvent::SpanCompleted { .. } => "span_completed",
        SystemEvent::SpanFailed { .. } => "span_failed",
        SystemEvent::TraceCreated { .. } => "trace_created",
        SystemEvent::TraceCompleted { .. } => "trace_completed",
        SystemEvent::FileVersionCreated { .. } => "file_version_created",
        SystemEvent::SpanDeleted { .. } => "span_deleted",
        SystemEvent::TraceDeleted { .. } => "trace_deleted",
        SystemEvent::DatasetCreated { .. } => "dataset_created",
        SystemEvent::DatasetDeleted { .. } => "dataset_deleted",
        SystemEvent::DatapointCreated { .. } => "datapoint_created",
        SystemEvent::QueueItemUpdated { .. } => "queue_item_updated",
        SystemEvent::EvalRunCreated { .. } => "eval_run_created",
        SystemEvent::EvalRunUpdated { .. } => "eval_run_updated",
        SystemEvent::EvalRunCompleted { .. } => "eval_run_completed",
        SystemEvent::CaptureRuleFired { .. } => "capture_rule_fired",
        SystemEvent::Cleared => "cleared",
    }
}

/// Spawn a background task that trims old events periodically.
pub fn spawn_event_log_trimmer(event_log: Arc<dyn EventLog>, retention: Duration) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300)); // every 5 min
        loop {
            interval.tick().await;
            match event_log.trim(retention).await {
                Ok(0) => {}
                Ok(n) => info!(deleted = n, "trimmed old events from event log"),
                Err(e) => error!("failed to trim event log: {e}"),
            }
        }
    });
}
