use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;
use trace::{FileVersion, Span, SpanId, SpanKind, SpanStatus, Trace, TraceId};

use crate::backend::{StorageBackend, StorageError};

// --- Migration system ---

const MIGRATIONS: &[&str] = &[
    // v1: core schema
    r#"
    CREATE TABLE IF NOT EXISTS spans (
        id TEXT PRIMARY KEY,
        trace_id TEXT NOT NULL,
        parent_id TEXT,
        name TEXT NOT NULL,
        kind_json TEXT NOT NULL,
        status TEXT NOT NULL,
        error TEXT,
        started_at TEXT NOT NULL,
        ended_at TEXT,
        input_json TEXT,
        output_json TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_spans_trace_id ON spans(trace_id);
    CREATE INDEX IF NOT EXISTS idx_spans_status ON spans(status);
    CREATE INDEX IF NOT EXISTS idx_spans_started_at ON spans(started_at);

    CREATE TABLE IF NOT EXISTS traces (
        id TEXT PRIMARY KEY,
        name TEXT,
        tags_json TEXT NOT NULL DEFAULT '[]',
        started_at TEXT NOT NULL,
        ended_at TEXT,
        machine_id TEXT
    );

    CREATE TABLE IF NOT EXISTS files (
        path TEXT NOT NULL,
        hash TEXT NOT NULL,
        size INTEGER NOT NULL,
        created_at TEXT NOT NULL,
        created_by_span TEXT,
        PRIMARY KEY (path, hash)
    );
    CREATE INDEX IF NOT EXISTS idx_files_path ON files(path);
    CREATE INDEX IF NOT EXISTS idx_files_hash ON files(hash);
    CREATE INDEX IF NOT EXISTS idx_files_created_by ON files(created_by_span);

    CREATE TABLE IF NOT EXISTS file_contents (
        hash TEXT PRIMARY KEY,
        content BLOB NOT NULL
    );
    "#,
];

fn run_migrations(conn: &Connection) -> Result<(), StorageError> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
        )",
    )?;

    let current_version: i64 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM migrations",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    for (i, migration) in MIGRATIONS.iter().enumerate() {
        let version = (i + 1) as i64;
        if version > current_version {
            conn.execute_batch(migration)?;
            conn.execute(
                "INSERT INTO migrations (version, applied_at) VALUES (?1, ?2)",
                params![version, Utc::now().to_rfc3339()],
            )?;
            tracing::info!(version, "applied migration");
        }
    }

    Ok(())
}

// --- SqliteBackend ---

pub struct SqliteBackend {
    conn: Mutex<Connection>,
}

impl SqliteBackend {
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn memory() -> Result<Self, StorageError> {
        let conn = Connection::open_in_memory()?;
        run_migrations(&conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    fn deserialize_span(
        id: &str,
        trace_id: &str,
        parent_id: Option<&str>,
        name: &str,
        kind_json: &str,
        status_str: &str,
        error: Option<&str>,
        started_at: &str,
        ended_at: Option<&str>,
        input_json: Option<&str>,
        output_json: Option<&str>,
    ) -> Result<Span, StorageError> {
        let id: SpanId = id
            .parse()
            .map_err(|e| StorageError::Database(format!("invalid span id: {}", e)))?;
        let trace_id: TraceId = trace_id
            .parse()
            .map_err(|e| StorageError::Database(format!("invalid trace id: {}", e)))?;
        let parent_id: Option<SpanId> = parent_id
            .map(|s| {
                s.parse()
                    .map_err(|e| StorageError::Database(format!("invalid parent id: {}", e)))
            })
            .transpose()?;
        let _kind: SpanKind = serde_json::from_str(kind_json)?;
        let _status = match status_str {
            "running" => SpanStatus::Running,
            "completed" => SpanStatus::Completed,
            "failed" => SpanStatus::Failed {
                error: error.unwrap_or_default().to_string(),
            },
            other => {
                return Err(StorageError::Database(format!(
                    "unknown status: {}",
                    other
                )))
            }
        };
        let started_at: DateTime<Utc> = DateTime::parse_from_rfc3339(started_at)
            .map_err(|e| StorageError::Database(format!("invalid started_at: {}", e)))?
            .with_timezone(&Utc);
        let ended_at: Option<DateTime<Utc>> = ended_at
            .map(|s| {
                DateTime::parse_from_rfc3339(s)
                    .map_err(|e| StorageError::Database(format!("invalid ended_at: {}", e)))
                    .map(|t| t.with_timezone(&Utc))
            })
            .transpose()?;
        let input: Option<serde_json::Value> =
            input_json.map(|s| serde_json::from_str(s)).transpose()?;
        let output: Option<serde_json::Value> =
            output_json.map(|s| serde_json::from_str(s)).transpose()?;

        // Reconstruct span via serde (Span fields are private)
        let span_value = serde_json::json!({
            "id": id,
            "trace_id": trace_id,
            "parent_id": parent_id,
            "name": name,
            "kind": serde_json::from_str::<serde_json::Value>(kind_json)?,
            "status": match status_str {
                "running" => serde_json::json!("running"),
                "completed" => serde_json::json!("completed"),
                _ => serde_json::json!({"failed": {"error": error.unwrap_or_default()}}),
            },
            "started_at": started_at,
            "ended_at": ended_at,
            "input": input,
            "output": output,
        });
        let span: Span = serde_json::from_value(span_value)?;
        Ok(span)
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    // --- Span operations ---

    async fn load_all_spans(&self) -> Result<Vec<Span>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, trace_id, parent_id, name, kind_json, status, error, started_at, ended_at, input_json, output_json FROM spans",
        )?;

        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let trace_id: String = row.get(1)?;
            let parent_id: Option<String> = row.get(2)?;
            let name: String = row.get(3)?;
            let kind_json: String = row.get(4)?;
            let status_str: String = row.get(5)?;
            let error: Option<String> = row.get(6)?;
            let started_at: String = row.get(7)?;
            let ended_at: Option<String> = row.get(8)?;
            let input_json: Option<String> = row.get(9)?;
            let output_json: Option<String> = row.get(10)?;
            Ok((
                id,
                trace_id,
                parent_id,
                name,
                kind_json,
                status_str,
                error,
                started_at,
                ended_at,
                input_json,
                output_json,
            ))
        })?;

        let mut spans = Vec::new();
        for row_result in rows {
            let (
                id,
                trace_id,
                parent_id,
                name,
                kind_json,
                status_str,
                error,
                started_at,
                ended_at,
                input_json,
                output_json,
            ) = row_result?;

            let span = Self::deserialize_span(
                &id,
                &trace_id,
                parent_id.as_deref(),
                &name,
                &kind_json,
                &status_str,
                error.as_deref(),
                &started_at,
                ended_at.as_deref(),
                input_json.as_deref(),
                output_json.as_deref(),
            )?;
            spans.push(span);
        }

        tracing::debug!(count = spans.len(), "loaded spans from sqlite");
        Ok(spans)
    }

    async fn save_span(&self, span: &Span) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;

        let id = span.id().to_string();
        let trace_id = span.trace_id().to_string();
        let parent_id = span.parent_id().map(|id| id.to_string());
        let name = span.name().to_string();
        let kind_json = serde_json::to_string(span.kind())?;
        let (status_str, error) = match span.status() {
            SpanStatus::Running => ("running".to_string(), None),
            SpanStatus::Completed => ("completed".to_string(), None),
            SpanStatus::Failed { error } => ("failed".to_string(), Some(error.clone())),
        };
        let started_at = span.started_at().to_rfc3339();
        let ended_at = span.ended_at().map(|t| t.to_rfc3339());
        let input_json = span
            .input()
            .map(|v| serde_json::to_string(v))
            .transpose()?;
        let output_json = span
            .output()
            .map(|v| serde_json::to_string(v))
            .transpose()?;

        conn.execute(
            "INSERT OR REPLACE INTO spans (id, trace_id, parent_id, name, kind_json, status, error, started_at, ended_at, input_json, output_json) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![id, trace_id, parent_id, name, kind_json, status_str, error, started_at, ended_at, input_json, output_json],
        )?;

        tracing::trace!(span_id = %span.id(), "saved span to sqlite");
        Ok(())
    }

    async fn delete_span(&self, id: SpanId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute("DELETE FROM spans WHERE id = ?1", params![id.to_string()])?;
        Ok(deleted > 0)
    }

    async fn delete_trace_spans(&self, trace_id: TraceId) -> Result<usize, StorageError> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute(
            "DELETE FROM spans WHERE trace_id = ?1",
            params![trace_id.to_string()],
        )?;
        Ok(deleted)
    }

    async fn clear_spans(&self) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM spans", [])?;
        Ok(())
    }

    // --- Trace operations ---

    async fn load_all_traces(&self) -> Result<Vec<Trace>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT id, name, tags_json, started_at, ended_at, machine_id FROM traces")?;

        let rows = stmt.query_map([], |row| {
            let id_str: String = row.get(0)?;
            let name: Option<String> = row.get(1)?;
            let tags_json: String = row.get(2)?;
            let started_at_str: String = row.get(3)?;
            let ended_at_str: Option<String> = row.get(4)?;
            let machine_id: Option<String> = row.get(5)?;
            Ok((
                id_str,
                name,
                tags_json,
                started_at_str,
                ended_at_str,
                machine_id,
            ))
        })?;

        let mut traces = Vec::new();
        for row_result in rows {
            let (id_str, name, tags_json, started_at_str, ended_at_str, machine_id) = row_result?;

            let id: TraceId = id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid trace id: {}", e)))?;
            let started_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&started_at_str)
                .map_err(|e| StorageError::Database(format!("invalid started_at: {}", e)))?
                .with_timezone(&Utc);
            let ended_at: Option<DateTime<Utc>> = ended_at_str
                .as_ref()
                .map(|s| {
                    DateTime::parse_from_rfc3339(s)
                        .map_err(|e| StorageError::Database(format!("invalid ended_at: {}", e)))
                        .map(|t| t.with_timezone(&Utc))
                })
                .transpose()?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();

            traces.push(Trace {
                id,
                name,
                tags,
                started_at,
                ended_at,
                machine_id,
            });
        }

        Ok(traces)
    }

    async fn save_trace(&self, trace: &Trace) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        let tags_json = serde_json::to_string(&trace.tags)?;
        conn.execute(
            "INSERT OR REPLACE INTO traces (id, name, tags_json, started_at, ended_at, machine_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                trace.id.to_string(),
                trace.name,
                tags_json,
                trace.started_at.to_rfc3339(),
                trace.ended_at.map(|t| t.to_rfc3339()),
                trace.machine_id,
            ],
        )?;
        Ok(())
    }

    async fn delete_trace(&self, trace_id: TraceId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted =
            conn.execute("DELETE FROM traces WHERE id = ?1", params![trace_id.to_string()])?;
        conn.execute(
            "DELETE FROM spans WHERE trace_id = ?1",
            params![trace_id.to_string()],
        )?;
        Ok(deleted > 0)
    }

    // --- File operations ---

    async fn load_all_files(&self) -> Result<Vec<FileVersion>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt =
            conn.prepare("SELECT path, hash, size, created_at, created_by_span FROM files")?;

        let rows = stmt.query_map([], |row| {
            let path: String = row.get(0)?;
            let hash: String = row.get(1)?;
            let size: i64 = row.get(2)?;
            let created_at_str: String = row.get(3)?;
            let created_by_span_str: Option<String> = row.get(4)?;
            Ok((path, hash, size, created_at_str, created_by_span_str))
        })?;

        let mut files = Vec::new();
        for row_result in rows {
            let (path, hash, size, created_at_str, created_by_span_str) = row_result?;
            let created_at: DateTime<Utc> = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| StorageError::Database(format!("invalid created_at: {}", e)))?
                .with_timezone(&Utc);
            let created_by_span: Option<SpanId> = created_by_span_str
                .as_ref()
                .map(|s| {
                    s.parse()
                        .map_err(|e| StorageError::Database(format!("invalid span id: {}", e)))
                })
                .transpose()?;

            files.push(FileVersion {
                hash,
                path,
                size: size as u64,
                created_at,
                created_by_span,
            });
        }

        Ok(files)
    }

    async fn save_file_version(&self, version: &FileVersion) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO files (path, hash, size, created_at, created_by_span) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                version.path,
                version.hash,
                version.size as i64,
                version.created_at.to_rfc3339(),
                version.created_by_span.map(|id| id.to_string()),
            ],
        )?;
        Ok(())
    }

    async fn save_file_content(&self, hash: &str, content: &[u8]) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR IGNORE INTO file_contents (hash, content) VALUES (?1, ?2)",
            params![hash, content],
        )?;
        Ok(())
    }

    async fn load_file_content(&self, hash: &str) -> Result<Vec<u8>, StorageError> {
        let conn = self.conn.lock().await;
        conn.query_row(
            "SELECT content FROM file_contents WHERE hash = ?1",
            params![hash],
            |row| row.get(0),
        )
        .map_err(|e| match e {
            rusqlite::Error::QueryReturnedNoRows => StorageError::NotFound,
            other => StorageError::Database(other.to_string()),
        })
    }
}
