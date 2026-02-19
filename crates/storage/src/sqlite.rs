use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use tokio::sync::Mutex;
use trace::{
    Datapoint, DatapointId, Dataset, DatasetId, FileVersion, QueueItem, QueueItemId, Span, SpanId,
    SpanKind, SpanStatus, Trace, TraceId,
};

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
    // v2: datasets, datapoints, queue_items
    r#"
    CREATE TABLE IF NOT EXISTS datasets (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        description TEXT,
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS datapoints (
        id TEXT PRIMARY KEY,
        dataset_id TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
        kind_json TEXT NOT NULL,
        source TEXT NOT NULL,
        source_span_id TEXT,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_datapoints_dataset_id ON datapoints(dataset_id);
    CREATE INDEX IF NOT EXISTS idx_datapoints_created_at ON datapoints(created_at);

    CREATE TABLE IF NOT EXISTS queue_items (
        id TEXT PRIMARY KEY,
        dataset_id TEXT NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
        datapoint_id TEXT NOT NULL REFERENCES datapoints(id) ON DELETE CASCADE,
        status TEXT NOT NULL DEFAULT 'pending',
        claimed_by TEXT,
        claimed_at TEXT,
        original_data_json TEXT,
        edited_data_json TEXT,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_queue_items_dataset_id ON queue_items(dataset_id);
    CREATE INDEX IF NOT EXISTS idx_queue_items_status ON queue_items(status);
    CREATE INDEX IF NOT EXISTS idx_queue_items_created_at ON queue_items(created_at);
    "#,
    // v3: multi-tenancy - add org_id columns and auth tables
    r#"
    -- Add org_id to existing tables (nullable for backward compatibility)
    ALTER TABLE spans ADD COLUMN org_id TEXT;
    ALTER TABLE traces ADD COLUMN org_id TEXT;
    ALTER TABLE datasets ADD COLUMN org_id TEXT;

    CREATE INDEX IF NOT EXISTS idx_spans_org_id ON spans(org_id);
    CREATE INDEX IF NOT EXISTS idx_traces_org_id ON traces(org_id);
    CREATE INDEX IF NOT EXISTS idx_datasets_org_id ON datasets(org_id);

    -- Organizations
    CREATE TABLE IF NOT EXISTS organizations (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        slug TEXT NOT NULL UNIQUE,
        plan TEXT NOT NULL DEFAULT 'free',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_organizations_slug ON organizations(slug);

    -- Users
    CREATE TABLE IF NOT EXISTS users (
        id TEXT PRIMARY KEY,
        email TEXT NOT NULL UNIQUE,
        name TEXT,
        org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        role TEXT NOT NULL DEFAULT 'member',
        created_at TEXT NOT NULL,
        updated_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_users_email ON users(email);
    CREATE INDEX IF NOT EXISTS idx_users_org_id ON users(org_id);

    -- API Keys
    CREATE TABLE IF NOT EXISTS api_keys (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        key_prefix TEXT NOT NULL,
        key_hash TEXT NOT NULL,
        scopes_json TEXT NOT NULL DEFAULT '[]',
        created_at TEXT NOT NULL,
        last_used_at TEXT,
        expires_at TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_api_keys_org_id ON api_keys(org_id);
    CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys(key_prefix);

    -- Invites
    CREATE TABLE IF NOT EXISTS invites (
        id TEXT PRIMARY KEY,
        org_id TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
        email TEXT NOT NULL,
        role TEXT NOT NULL DEFAULT 'member',
        invited_by TEXT NOT NULL REFERENCES users(id),
        token_hash TEXT NOT NULL,
        expires_at TEXT NOT NULL,
        created_at TEXT NOT NULL
    );
    CREATE INDEX IF NOT EXISTS idx_invites_email ON invites(email);
    CREATE INDEX IF NOT EXISTS idx_invites_org_id ON invites(org_id);
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
                org_id: None, // Loaded from DB if present via v3 migration
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

    // --- Dataset operations ---

    async fn load_all_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt =
            conn.prepare("SELECT id, name, description, created_at, updated_at FROM datasets")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let name: String = row.get(1)?;
            let description: Option<String> = row.get(2)?;
            let created_at: String = row.get(3)?;
            let updated_at: String = row.get(4)?;
            Ok((id, name, description, created_at, updated_at))
        })?;

        let mut datasets = Vec::new();
        for row_result in rows {
            let (id_str, name, description, created_at_str, updated_at_str) = row_result?;
            let id: DatasetId = id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid dataset id: {}", e)))?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| StorageError::Database(format!("invalid created_at: {}", e)))?
                .with_timezone(&Utc);
            let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| StorageError::Database(format!("invalid updated_at: {}", e)))?
                .with_timezone(&Utc);
            datasets.push(Dataset {
                id,
                org_id: None, // Loaded from DB if present via v3 migration
                name,
                description,
                created_at,
                updated_at,
            });
        }
        Ok(datasets)
    }

    async fn save_dataset(&self, dataset: &Dataset) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO datasets (id, name, description, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                dataset.id.to_string(),
                dataset.name,
                dataset.description,
                dataset.created_at.to_rfc3339(),
                dataset.updated_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn delete_dataset(&self, id: DatasetId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted =
            conn.execute("DELETE FROM datasets WHERE id = ?1", params![id.to_string()])?;
        Ok(deleted > 0)
    }

    // --- Datapoint operations ---

    async fn load_all_datapoints(&self) -> Result<Vec<Datapoint>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, dataset_id, kind_json, source, source_span_id, created_at FROM datapoints",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let dataset_id: String = row.get(1)?;
            let kind_json: String = row.get(2)?;
            let source: String = row.get(3)?;
            let source_span_id: Option<String> = row.get(4)?;
            let created_at: String = row.get(5)?;
            Ok((id, dataset_id, kind_json, source, source_span_id, created_at))
        })?;

        let mut datapoints = Vec::new();
        for row_result in rows {
            let (id_str, dataset_id_str, kind_json, source_str, source_span_id_str, created_at_str) =
                row_result?;
            let id: DatapointId = id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid datapoint id: {}", e)))?;
            let dataset_id: DatasetId = dataset_id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid dataset id: {}", e)))?;
            let kind = serde_json::from_str(&kind_json)?;
            let source = serde_json::from_value(serde_json::Value::String(source_str))?;
            let source_span_id: Option<SpanId> = source_span_id_str
                .map(|s| {
                    s.parse()
                        .map_err(|e| StorageError::Database(format!("invalid span id: {}", e)))
                })
                .transpose()?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| StorageError::Database(format!("invalid created_at: {}", e)))?
                .with_timezone(&Utc);
            datapoints.push(Datapoint {
                id,
                dataset_id,
                kind,
                source,
                source_span_id,
                created_at,
            });
        }
        Ok(datapoints)
    }

    async fn save_datapoint(&self, dp: &Datapoint) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        let kind_json = serde_json::to_string(&dp.kind)?;
        let source_str = serde_json::to_value(&dp.source)?;
        let source_str = source_str.as_str().unwrap_or("manual");
        conn.execute(
            "INSERT OR REPLACE INTO datapoints (id, dataset_id, kind_json, source, source_span_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                dp.id.to_string(),
                dp.dataset_id.to_string(),
                kind_json,
                source_str,
                dp.source_span_id.map(|id| id.to_string()),
                dp.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn delete_datapoint(&self, id: DatapointId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted =
            conn.execute("DELETE FROM datapoints WHERE id = ?1", params![id.to_string()])?;
        Ok(deleted > 0)
    }

    async fn delete_dataset_datapoints(
        &self,
        dataset_id: DatasetId,
    ) -> Result<usize, StorageError> {
        let conn = self.conn.lock().await;
        let deleted = conn.execute(
            "DELETE FROM datapoints WHERE dataset_id = ?1",
            params![dataset_id.to_string()],
        )?;
        Ok(deleted)
    }

    // --- Queue operations ---

    async fn load_all_queue_items(&self) -> Result<Vec<QueueItem>, StorageError> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare(
            "SELECT id, dataset_id, datapoint_id, status, claimed_by, claimed_at, original_data_json, edited_data_json, created_at FROM queue_items",
        )?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let dataset_id: String = row.get(1)?;
            let datapoint_id: String = row.get(2)?;
            let status: String = row.get(3)?;
            let claimed_by: Option<String> = row.get(4)?;
            let claimed_at: Option<String> = row.get(5)?;
            let original_data_json: Option<String> = row.get(6)?;
            let edited_data_json: Option<String> = row.get(7)?;
            let created_at: String = row.get(8)?;
            Ok((
                id,
                dataset_id,
                datapoint_id,
                status,
                claimed_by,
                claimed_at,
                original_data_json,
                edited_data_json,
                created_at,
            ))
        })?;

        let mut items = Vec::new();
        for row_result in rows {
            let (
                id_str,
                dataset_id_str,
                datapoint_id_str,
                status_str,
                claimed_by,
                claimed_at_str,
                original_data_json,
                edited_data_json,
                created_at_str,
            ) = row_result?;
            let id: QueueItemId = id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid queue item id: {}", e)))?;
            let dataset_id: DatasetId = dataset_id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid dataset id: {}", e)))?;
            let datapoint_id: DatapointId = datapoint_id_str
                .parse()
                .map_err(|e| StorageError::Database(format!("invalid datapoint id: {}", e)))?;
            let status = serde_json::from_value(serde_json::Value::String(status_str))?;
            let claimed_at = claimed_at_str
                .map(|s| {
                    DateTime::parse_from_rfc3339(&s)
                        .map_err(|e| StorageError::Database(format!("invalid claimed_at: {}", e)))
                        .map(|t| t.with_timezone(&Utc))
                })
                .transpose()?;
            let original_data: Option<serde_json::Value> =
                original_data_json.map(|s| serde_json::from_str(&s)).transpose()?;
            let edited_data: Option<serde_json::Value> =
                edited_data_json.map(|s| serde_json::from_str(&s)).transpose()?;
            let created_at = DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| StorageError::Database(format!("invalid created_at: {}", e)))?
                .with_timezone(&Utc);
            items.push(QueueItem {
                id,
                dataset_id,
                datapoint_id,
                status,
                claimed_by,
                claimed_at,
                original_data,
                edited_data,
                created_at,
            });
        }
        Ok(items)
    }

    async fn save_queue_item(&self, item: &QueueItem) -> Result<(), StorageError> {
        let conn = self.conn.lock().await;
        let original_data_json = item
            .original_data
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()?;
        let edited_data_json = item
            .edited_data
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()?;
        conn.execute(
            "INSERT OR REPLACE INTO queue_items (id, dataset_id, datapoint_id, status, claimed_by, claimed_at, original_data_json, edited_data_json, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                item.id.to_string(),
                item.dataset_id.to_string(),
                item.datapoint_id.to_string(),
                item.status.as_str(),
                item.claimed_by,
                item.claimed_at.map(|t| t.to_rfc3339()),
                original_data_json,
                edited_data_json,
                item.created_at.to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    async fn delete_queue_item(&self, id: QueueItemId) -> Result<bool, StorageError> {
        let conn = self.conn.lock().await;
        let deleted =
            conn.execute("DELETE FROM queue_items WHERE id = ?1", params![id.to_string()])?;
        Ok(deleted > 0)
    }

    // --- Methods required by new trait interface ---

    async fn get_trace(&self, id: TraceId) -> Result<Option<Trace>, StorageError> {
        let traces = self.load_all_traces().await?;
        Ok(traces.into_iter().find(|t| t.id == id))
    }

    async fn list_traces(&self, _filter: &crate::filter::TraceFilter) -> Result<Vec<Trace>, StorageError> {
        self.load_all_traces().await
    }

    async fn get_span(&self, id: SpanId) -> Result<Option<Span>, StorageError> {
        let spans = self.load_all_spans().await?;
        Ok(spans.into_iter().find(|s| s.id() == id))
    }

    async fn list_spans(&self, _filter: &crate::filter::SpanFilter) -> Result<Vec<Span>, StorageError> {
        self.load_all_spans().await
    }

    async fn get_dataset(&self, id: DatasetId) -> Result<Option<Dataset>, StorageError> {
        let datasets = self.load_all_datasets().await?;
        Ok(datasets.into_iter().find(|d| d.id == id))
    }

    async fn list_datasets(&self) -> Result<Vec<Dataset>, StorageError> {
        self.load_all_datasets().await
    }

    async fn get_datapoint(&self, id: DatapointId) -> Result<Option<Datapoint>, StorageError> {
        let datapoints = self.load_all_datapoints().await?;
        Ok(datapoints.into_iter().find(|d| d.id == id))
    }

    async fn list_datapoints(&self, dataset_id: DatasetId) -> Result<Vec<Datapoint>, StorageError> {
        let all = self.load_all_datapoints().await?;
        Ok(all.into_iter().filter(|d| d.dataset_id == dataset_id).collect())
    }

    async fn list_datapoints_all(&self) -> Result<Vec<Datapoint>, StorageError> {
        self.load_all_datapoints().await
    }

    async fn get_queue_item(&self, id: QueueItemId) -> Result<Option<QueueItem>, StorageError> {
        let items = self.load_all_queue_items().await?;
        Ok(items.into_iter().find(|q| q.id == id))
    }

    async fn list_queue_items(&self, dataset_id: DatasetId) -> Result<Vec<QueueItem>, StorageError> {
        let all = self.load_all_queue_items().await?;
        Ok(all.into_iter().filter(|q| q.dataset_id == dataset_id).collect())
    }

    async fn list_queue_items_all(&self) -> Result<Vec<QueueItem>, StorageError> {
        self.load_all_queue_items().await
    }

    async fn list_file_versions(&self) -> Result<Vec<FileVersion>, StorageError> {
        self.load_all_files().await
    }

    fn backend_type(&self) -> &'static str {
        "sqlite"
    }
}
