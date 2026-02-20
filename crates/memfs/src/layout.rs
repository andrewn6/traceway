//! Memfs directory layout specification.
//!
//! This module defines the virtual filesystem tree that memfs exposes.
//! It serves as the canonical reference for path conventions, inode
//! allocation ranges, and file content formats.
//!
//! # Layout overview
//!
//! The mount point (default `~/.traceway/mem/`) contains two top-level
//! subtrees with very different semantics:
//!
//! ```text
//! ~/.traceway/mem/                  (mount root)
//! ├── traces/                       (read-only, auto-generated from span store)
//! │   ├── <trace-id>/               (one directory per trace)
//! │   │   ├── info.json             (trace metadata: name, tags, timestamps)
//! │   │   ├── spans/                (one file per span in this trace)
//! │   │   │   ├── <span-id>.json    (full span JSON including kind, status, timing)
//! │   │   │   └── ...
//! │   │   ├── tree.txt              (human-readable span tree with indentation)
//! │   │   └── summary.json          (aggregate: span count, duration, token totals, cost)
//! │   ├── _latest -> <trace-id>/    (symlink to most recent trace)
//! │   └── _active/                  (virtual dir: only traces with running spans)
//! │       └── <trace-id> -> ../...  (symlinks to parent trace dirs)
//! │
//! ├── workspace/                    (read-write, user-facing shared context)
//! │   └── ...                       (arbitrary user files/directories)
//! │                                 (every read() → FsRead span, write() → FsWrite span)
//! │
//! ├── stats.json                    (read-only, global AnalyticsSummary)
//! └── status.txt                    (read-only, daemon status: uptime, span count, etc.)
//! ```
//!
//! # Subtree semantics
//!
//! ## `traces/` — read-only observability view
//!
//! Auto-generated from the span store. Directories appear/disappear as traces
//! are created/deleted. Files are synthesized on read (not stored on disk).
//!
//! - **`info.json`**: Serialized `Trace` struct (id, name, tags, started_at, ended_at, machine_id)
//! - **`spans/<id>.json`**: Serialized `Span` struct for each span in the trace
//! - **`tree.txt`**: Human-readable ASCII tree showing span hierarchy with timing:
//!   ```text
//!   code-review [2.3s]
//!   ├── llm-call-0 (gpt-4o, 1234 tok, $0.02) [1.1s] ✓
//!   ├── read-main.py (1.2 KB) [0.01s] ✓
//!   ├── llm-call-1 (claude-sonnet-4-20250514, 890 tok) [0.9s] ✓
//!   └── write-output.csv (4.5 KB) [0.02s] ✓
//!   ```
//! - **`summary.json`**: Aggregated metrics for this trace (total tokens, cost, latency, error count)
//!
//! ## `workspace/` — read-write shared context (Phase 3)
//!
//! A real writable directory where applications store shared context. Every
//! filesystem operation is automatically instrumented:
//!
//! - `read()` → creates `SpanKind::FsRead { path, file_version, bytes_read }`
//! - `write()` → computes SHA256 hash, stores content (deduped), creates
//!   `SpanKind::FsWrite { path, file_version, bytes_written }`
//! - `create()`/`mkdir()`/`unlink()` → tracked but no span (metadata-only changes)
//!
//! Content is stored in a content-addressed object store at
//! `~/.traceway/objects/{hash[0:2]}/{hash[2:]}`. File metadata (path, inode,
//! size, timestamps, current version hash) lives in SQLite.
//!
//! ## Root-level virtual files
//!
//! - **`stats.json`**: Global `AnalyticsSummary` — total traces, spans, tokens, cost, error count
//! - **`status.txt`**: Human-readable daemon status:
//!   ```text
//!   traceway daemon
//!   uptime: 2h 13m 07s
//!   spans: 1,247 (42 active)
//!   traces: 89 (3 active)
//!   storage: 12.4 MB (traces.db)
//!   api: http://127.0.0.1:3000
//!   proxy: http://127.0.0.1:3001 -> http://localhost:11434
//!   ```

/// Default mount point relative to the Traceway data directory.
pub const DEFAULT_MOUNT_SUBDIR: &str = "mem";

/// Well-known directory and file names within the mount.
pub mod paths {
    /// Top-level read-only trace observation directory.
    pub const TRACES_DIR: &str = "traces";

    /// Top-level read-write workspace directory (Phase 3).
    pub const WORKSPACE_DIR: &str = "workspace";

    /// Symlink name pointing to the most recently started trace.
    pub const LATEST_LINK: &str = "_latest";

    /// Virtual directory containing symlinks to traces with running spans.
    pub const ACTIVE_DIR: &str = "_active";

    /// Per-trace metadata file.
    pub const TRACE_INFO: &str = "info.json";

    /// Per-trace subdirectory containing span files.
    pub const SPANS_DIR: &str = "spans";

    /// Per-trace human-readable span tree.
    pub const TREE_TXT: &str = "tree.txt";

    /// Per-trace aggregated metrics.
    pub const SUMMARY_JSON: &str = "summary.json";

    /// Root-level global analytics summary.
    pub const STATS_JSON: &str = "stats.json";

    /// Root-level daemon status file.
    pub const STATUS_TXT: &str = "status.txt";
}

/// Reserved inode ranges for the virtual filesystem.
///
/// FUSE requires stable inode numbers. We partition the inode space:
/// - 1: root directory
/// - 2-9: well-known top-level entries
/// - 10-99: reserved for future well-known entries
/// - 100-999: per-trace directories (allocated dynamically)
/// - 1000+: span files and workspace entries (allocated dynamically)
pub mod inodes {
    pub const ROOT: u64 = 1;
    pub const TRACES_DIR: u64 = 2;
    pub const WORKSPACE_DIR: u64 = 3;
    pub const STATS_JSON: u64 = 4;
    pub const STATUS_TXT: u64 = 5;
    pub const ACTIVE_DIR: u64 = 6;
    pub const LATEST_LINK: u64 = 7;

    /// First dynamically allocated inode (for trace dirs, span files, etc.)
    pub const DYNAMIC_START: u64 = 100;
}

/// File extensions used in the virtual filesystem.
pub mod extensions {
    pub const JSON: &str = ".json";
    pub const TXT: &str = ".txt";
}

/// Content-addressed object store path conventions.
pub mod objects {
    /// Default subdirectory under the data dir for content-addressed objects.
    pub const OBJECTS_SUBDIR: &str = "objects";

    /// Given a hex-encoded SHA256 hash, return the relative storage path.
    /// Uses a 2-character prefix directory for fanout (e.g., "ab/cdef01234...").
    pub fn object_path(hash: &str) -> String {
        if hash.len() < 4 {
            return hash.to_string();
        }
        format!("{}/{}", &hash[..2], &hash[2..])
    }
}

/// Construct a trace directory name from a trace ID.
/// Uses the full UUID string (hyphenated, lowercase).
pub fn trace_dir_name(trace_id: &uuid::Uuid) -> String {
    trace_id.to_string()
}

/// Construct a span file name from a span ID.
/// Format: `<span-id>.json`
pub fn span_file_name(span_id: &uuid::Uuid) -> String {
    format!("{}.json", span_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn object_path_fanout() {
        assert_eq!(
            objects::object_path("abcdef0123456789"),
            "ab/cdef0123456789"
        );
    }

    #[test]
    fn object_path_short_hash() {
        assert_eq!(objects::object_path("ab"), "ab");
    }

    #[test]
    fn span_file_name_format() {
        let id = uuid::Uuid::nil();
        assert_eq!(
            span_file_name(&id),
            "00000000-0000-0000-0000-000000000000.json"
        );
    }

    #[test]
    fn trace_dir_name_format() {
        let id = uuid::Uuid::nil();
        assert_eq!(trace_dir_name(&id), "00000000-0000-0000-0000-000000000000");
    }
}
