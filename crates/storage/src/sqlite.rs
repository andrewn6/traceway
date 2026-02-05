use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;
use trace::{Span, SpanId, SpanMetadata, SpanStatus, TraceId};

use crate::backend::{StorageBackend, StorageError};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS spans (
    id TEXT PRIMARY KEY,
    trace_id TEXT NOT NULL,
    parent_id TEXT,
    name TEXT NOT NULL,
    status TEXT NOT NULL,
    started_at TEXT NOT NULL,
    ended_at TEXT,
    error TEXT,
    model TEXT,
    input_tokens INTEGER,
    output_tokens INTEGER
);

CREATE INDEX IF NOT EXISTS idx_trace_id ON spans(trace_id);
CREATE INDEX IF NOT EXISTS idx_status ON spans(status);
CREATE INDEX IF NOT EXISTS idx_model ON spans(model);
CREATE INDEX IF NOT EXISTS idx_started_at ON spans(started_at);
"#;

pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
    /// Open a SQLite database at the given path.
    /// Creates the file and parent directories if they don't exist.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        let backend = Self {
            conn: Mutex::new(conn),
        };
        // Run synchronously since we're in constructor
        backend.init_schema_sync()?;
        Ok(backend)
    }

    /// Create an in-memory database (useful for testing).
    pub fn memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        let backend = Self {
            conn: Mutex::new(conn),
        };
        backend.init_schema_sync()?;
        Ok(backend)
    }

    fn init_schema_sync(&self) -> Result<(), StorageError> {
        // We need to block here since this is called from constructors
        let conn = self.conn.blocking_lock();
        conn.execute_batch(SCHEMA)?;
        Ok(())
    }

    fn span_to_row(span: &Span) -> SpanRow {
        let (status, started_at, ended_at, error) = match &span.status {
            SpanStatus::Running { started_at } => ("running", *started_at, None, None),
            SpanStatus::Completed { started_at, ended_at } => {
                ("completed", *started_at, Some(*ended_at), None)
            }
            SpanStatus::Failed {
                started_at,
                ended_at,
                error,
            } => ("failed", *started_at, Some(*ended_at), Some(error.clone())),
        };

        SpanRow {
            id: span.id.to_string(),
            trace_id: span.trace_id.to_string(),
            parent_id: span.parent_id.map(|id| id.to_string()),
            name: span.name.clone(),
            status: status.to_string(),
            started_at: started_at.to_rfc3339(),
            ended_at: ended_at.map(|t| t.to_rfc3339()),
            error,
            model: span.metadata.model.clone(),
            input_tokens: span.metadata.input_tokens.map(|t| t as i64),
            output_tokens: span.metadata.output_tokens.map(|t| t as i64),
        }
    }

    fn row_to_span(row: &SpanRow) -> Result<Span, StorageError> {
        let id: SpanId = row
            .id
            .parse()
            .map_err(|e| StorageError::Database(format!("invalid span id: {}", e)))?;
        let trace_id: TraceId = row
            .trace_id
            .parse()
            .map_err(|e| StorageError::Database(format!("invalid trace id: {}", e)))?;
        let parent_id: Option<SpanId> = row
            .parent_id
            .as_ref()
            .map(|s| {
                s.parse()
                    .map_err(|e| StorageError::Database(format!("invalid parent id: {}", e)))
            })
            .transpose()?;

        let started_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&row.started_at)
            .map_err(|e| StorageError::Database(format!("invalid started_at: {}", e)))?
            .with_timezone(&Utc);

        let ended_at: Option<DateTime<Utc>> = row
            .ended_at
            .as_ref()
            .map(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map_err(|e| StorageError::Database(format!("invalid ended_at: {}", e)))
                    .map(|t| t.with_timezone(&Utc))
            })
            .transpose()?;

        let status = match row.status.as_str() {
            "running" => SpanStatus::Running { started_at },
            "completed" => SpanStatus::Completed {
                started_at,
                ended_at: ended_at.unwrap_or(started_at),
            },
            "failed" => SpanStatus::Failed {
                started_at,
                ended_at: ended_at.unwrap_or(started_at),
                error: row.error.clone().unwrap_or_default(),
            },
            other => {
                return Err(StorageError::Database(format!(
                    "unknown status: {}",
                    other
                )))
            }
        };

        Ok(Span {
            id,
            trace_id,
            parent_id,
            name: row.name.clone(),
            status,
            metadata: SpanMetadata {
                model: row.model.clone(),
                input_tokens: row.input_tokens.map(|t| t as u64),
                output_tokens: row.output_tokens.map(|t| t as u64),
            },
        })
    }
}

struct SpanRow {
    id: String,
    trace_id: String,
    parent_id: Option<String>,
    name: String,
    status: String,
    started_at: String,
    ended_at: Option<String>,
    error: Option<String>,
    model: Option<String>,
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    async fn load_all(&self) -> Result<Vec<Span>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, trace_id, parent_id, name, status, started_at, ended_at, error, model, input_tokens, output_tokens FROM spans",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(SpanRow {
                id: row.get(0)?,
                trace_id: row.get(1)?,
                parent_id: row.get(2)?,
                name: row.get(3)?,
                status: row.get(4)?,
                started_at: row.get(5)?,
                ended_at: row.get(6)?,
                error: row.get(7)?,
                model: row.get(8)?,
                input_tokens: row.get(9)?,
                output_tokens: row.get(10)?,
            })
        })?;

        let mut spans = Vec::new();
        for row_result in rows {
            let row = row_result?;
            spans.push(Self::row_to_span(&row)?);
        }

        tracing::debug!(count = spans.len(), "loaded spans from sqlite");
        Ok(spans)
    }

    async fn save_span(&self, span: &Span) -> Result<(), StorageError> {
        let row = Self::span_to_row(span);
        let conn = self.conn.lock().await;

        conn.execute(
            "INSERT OR REPLACE INTO spans (id, trace_id, parent_id, name, status, started_at, ended_at, error, model, input_tokens, output_tokens) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                row.id,
                row.trace_id,
                row.parent_id,
                row.name,
                row.status,
                row.started_at,
                row.ended_at,
                row.error,
                row.model,
                row.input_tokens,
                row.output_tokens,
            ],
        )?;

        tracing::trace!(span_id = %span.id, "saved span to sqlite");
        Ok(())
    }

    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute("DELETE FROM spans WHERE id = ?1", params![id.to_string()])?;
        Ok(deleted > 0)
    }

    async fn delete_trace(&self, trace_id: TraceId) -> Result<usize, StorageError> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute(
            "DELETE FROM spans WHERE trace_id = ?1",
            params![trace_id.to_string()],
        )?;
        Ok(deleted)
    }

    async fn clear(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM spans", [])?;
        Ok(())
    }
}
