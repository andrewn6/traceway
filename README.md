# Traceway

A local-first observability platform for LLM applications. Capture traces, spans, file I/O, token usage, and cost — then query and analyze everything through a REST API and web UI.

Think of it as [Jaeger](https://www.jaegertracing.io/) or [Langfuse](https://langfuse.com/) but running entirely on your machine with zero cloud dependencies.

## Why

When building with LLMs, you need to understand what's happening: which files were read into context, what the model produced, how many tokens it consumed, and how much it cost. Existing tools either require a cloud account or are tightly coupled to a specific framework.

Traceway is a standalone daemon that any LLM application can send traces to — framework-agnostic, language-agnostic, local-first.

## Features

- **Structured tracing** — Traces contain spans with typed kinds: `llm_call`, `fs_read`, `fs_write`, `custom`
- **Transparent LLM proxy** — Point your app at the proxy instead of Ollama/OpenAI; traces are captured automatically
- **Token & cost tracking** — Input/output token counts and cost per LLM call
- **File versioning** — Content-addressed storage tracks every file read/written during a trace
- **Analytics API** — Aggregate cost, tokens, latency, and error rates with flexible group-by queries
- **Dataset management** — Export spans to datasets, import CSVs, label via a review queue
- **Real-time events** — SSE stream pushes span/trace lifecycle events to connected clients
- **Web UI** — Dashboard, trace explorer, span query builder, waterfall timelines, analysis tools
- **SQLite storage** — Everything persists in a single `~/.traceway/traces.db` file

## Quick Start

### Prerequisites

- Rust 1.75+ (for building the daemon)
- Node.js 18+ (for the UI, optional)

### Build & Run

```sh
# Clone
git clone https://github.com/andrewn6/llm-fs.git
cd llm-fs

# Build the daemon (includes API + proxy)
cargo build -p daemon --release

# Run in foreground
./target/release/daemon --foreground

# Or daemonize (background)
./target/release/daemon -d
```

The daemon starts two servers:

| Service | Default Address | Purpose |
|---------|----------------|---------|
| API     | `127.0.0.1:3000` | REST API + SSE events + Web UI |
| Proxy   | `127.0.0.1:3001` | Transparent LLM proxy (forwards to Ollama at `localhost:11434`) |

### Verify

```sh
curl http://localhost:3000/api/health
# {"status":"ok","uptime_secs":5}
```

### Send Your First Trace

```sh
# 1. Create a trace
TRACE_ID=$(curl -s http://localhost:3000/api/traces -X POST \
  -H 'Content-Type: application/json' \
  -d '{"name":"my-first-trace"}' | jq -r '.id')

echo "Trace: $TRACE_ID"

# 2. Create a span
SPAN_ID=$(curl -s http://localhost:3000/api/spans -X POST \
  -H 'Content-Type: application/json' \
  -d "{\"trace_id\":\"$TRACE_ID\",\"name\":\"hello-world\",\"kind\":{\"type\":\"custom\",\"kind\":\"task\",\"attributes\":{}}}" | jq -r '.id')

# 3. Complete the span
curl -s http://localhost:3000/api/spans/$SPAN_ID/complete -X POST \
  -H 'Content-Type: application/json' \
  -d '{"output":{"result":"done"}}'

# 4. View the trace
curl -s http://localhost:3000/api/traces/$TRACE_ID | jq
```

### Use the Proxy

Instead of calling your LLM directly, point your application at the proxy:

```sh
# Before (direct to Ollama)
curl http://localhost:11434/api/chat -d '{"model":"llama3","messages":[{"role":"user","content":"Hi"}]}'

# After (through Traceway proxy — automatically traced)
curl http://localhost:3001/api/chat -d '{"model":"llama3","messages":[{"role":"user","content":"Hi"}]}'
```

The proxy captures the request/response, extracts token counts, and creates a span — all transparently.

## Configuration

Config file: `~/.traceway/config.toml`

```toml
[api]
addr = "127.0.0.1:3000"

[proxy]
addr = "127.0.0.1:3001"
target = "http://localhost:11434"   # Ollama, vLLM, OpenAI-compatible, etc.

[storage]
db_path = "/custom/path/traces.db"  # default: ~/.traceway/traces.db

[logging]
level = "info"                      # trace, debug, info, warn, error
```

All settings can be overridden via CLI flags:

```sh
daemon --foreground \
  --api-addr 0.0.0.0:8080 \
  --proxy-addr 0.0.0.0:8081 \
  --target-url http://my-llm:11434 \
  --db-path /data/traces.db \
  --log-level debug
```

Environment variable: `TRACEWAY_LOG=debug` overrides the log level.

### File Locations

| File | Path |
|------|------|
| Config | `~/.traceway/config.toml` |
| Database | `~/.traceway/traces.db` |
| Logs | `~/.traceway/logs/daemon.log` |
| PID file | `~/.traceway/daemon.pid` |

## Architecture

```
┌─────────────┐      ┌─────────────┐
│  Your App   │      │  LLM Server │
│  (Python,   │      │  (Ollama,   │
│   TS, curl) │      │   vLLM)     │
└──────┬──────┘      └──────▲──────┘
       │                     │
       ▼                     │
┌──────────────┐    ┌────────┴───────┐
│   API :3000  │    │  Proxy :3001   │
│  (REST+SSE)  │    │  (transparent) │
└──────┬───────┘    └────────┬───────┘
       │                     │
       ▼                     ▼
┌──────────────────────────────────────┐
│           Storage Layer              │
│   In-memory SpanStore + SQLite       │
└──────────────────────────────────────┘
```

### Crate Structure

```
crates/
├── trace/     # Core types: Span, Trace, SpanKind, Analytics types
├── storage/   # SpanStore, SQLite backend, filtering, analytics computation
├── api/       # Axum REST API, SSE events, static UI serving
├── proxy/     # Transparent LLM proxy with automatic tracing
├── daemon/    # Binary entry point, config, PID management, supervision
└── memfs/     # FUSE filesystem (experimental, requires macFUSE)
```

### Data Model

**Trace** — A logical unit of work (e.g., one agent run, one chat turn).

**Span** — A single operation within a trace. Spans have a `kind`:

| Kind | Fields | Description |
|------|--------|-------------|
| `llm_call` | `model`, `provider`, `input_tokens`, `output_tokens`, `cost` | An LLM API call |
| `fs_read` | `path`, `bytes_read`, `file_version` | A file read into context |
| `fs_write` | `path`, `bytes_written`, `file_version` | A file written as output |
| `custom` | `kind`, `attributes` | Anything else (tool call, API request, etc.) |

Spans follow a lifecycle: `running` → `completed` or `failed`.

**FileVersion** — Content-addressed snapshot of a file at a point in time (SHA-256 hash).

**Dataset** — A collection of labeled datapoints, built from exported spans or CSV imports.

## API Reference

### Traces

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/traces` | List all traces |
| `POST` | `/api/traces` | Create a new trace |
| `GET` | `/api/traces/:id` | Get trace with its spans |
| `DELETE` | `/api/traces/:id` | Delete a trace and its spans |
| `DELETE` | `/api/traces` | Clear all traces |

### Spans

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/spans` | List/filter spans |
| `POST` | `/api/spans` | Create a span |
| `GET` | `/api/spans/:id` | Get a single span |
| `DELETE` | `/api/spans/:id` | Delete a span |
| `POST` | `/api/spans/:id/complete` | Mark span as completed |
| `POST` | `/api/spans/:id/fail` | Mark span as failed |

**Span filters** (query params on `GET /api/spans`):

```
?trace_id=...&status=running&kind=llm_call&model=gpt-4&provider=openai&name_contains=chat&since=2024-01-01T00:00:00Z&until=...&path=/src/...
```

### Files

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/files` | List tracked file paths |
| `GET` | `/api/files/*path` | Get all versions of a file |

### Analytics

| Method | Endpoint | Description |
|--------|----------|-------------|
| `POST` | `/api/analytics` | Flexible aggregation query |
| `GET` | `/api/analytics/summary` | Quick dashboard summary |

**Example: Cost by model**

```sh
curl -s http://localhost:3000/api/analytics -X POST \
  -H 'Content-Type: application/json' \
  -d '{
    "metrics": ["total_cost", "total_tokens", "span_count"],
    "group_by": ["model"],
    "filter": { "kind": "llm_call" }
  }' | jq
```

**Example: Dashboard summary**

```sh
curl -s http://localhost:3000/api/analytics/summary | jq
# {
#   "total_traces": 42,
#   "total_spans": 186,
#   "total_llm_calls": 89,
#   "total_cost": 1.23,
#   "total_input_tokens": 150000,
#   "total_output_tokens": 45000,
#   "avg_latency_ms": 1200,
#   "error_count": 3,
#   "models_used": ["gpt-4", "claude-3"],
#   "cost_by_model": [...],
#   "tokens_by_model": [...]
# }
```

Available metrics: `total_cost`, `total_input_tokens`, `total_output_tokens`, `total_tokens`, `avg_latency_ms`, `span_count`, `error_count`

Available group-by fields: `model`, `provider`, `kind`, `status`, `trace`, `day`, `hour`

### Datasets

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/datasets` | List datasets |
| `POST` | `/api/datasets` | Create a dataset |
| `GET` | `/api/datasets/:id` | Get dataset details |
| `PUT` | `/api/datasets/:id` | Update dataset |
| `DELETE` | `/api/datasets/:id` | Delete dataset |
| `GET` | `/api/datasets/:id/datapoints` | List datapoints |
| `POST` | `/api/datasets/:id/datapoints` | Add datapoint |
| `DELETE` | `/api/datasets/:id/datapoints/:dp_id` | Remove datapoint |
| `POST` | `/api/datasets/:id/export-span` | Export a span as a datapoint |
| `POST` | `/api/datasets/:id/import` | Import CSV file |
| `GET` | `/api/datasets/:id/queue` | List review queue |
| `POST` | `/api/datasets/:id/queue` | Enqueue datapoints for review |
| `POST` | `/api/queue/:item_id/claim` | Claim a queue item |
| `POST` | `/api/queue/:item_id/submit` | Submit review |

### Other

| Method | Endpoint | Description |
|--------|----------|-------------|
| `GET` | `/api/health` | Health check with uptime |
| `GET` | `/api/stats` | Trace/span counts |
| `GET` | `/api/export/json` | Export all data as JSON |
| `GET` | `/api/events` | SSE event stream |

## Web UI

The UI is a SvelteKit app in `ui/`.

```sh
cd ui
npm install
npm run dev
```

Opens at `http://localhost:5173` for development. In production, the UI is embedded in the daemon and served at `http://localhost:3000`. All API calls use the `/api/` prefix.

### Pages

- **Dashboard** — Overview cards (traces, spans, latency, error rate), model breakdown, recent activity
- **Traces** — Filterable trace list with span counts, status, duration. Create new traces from the UI.
- **Trace Detail** — Span tree, waterfall timeline, context in/out panels, span detail with input/output payloads
- **Query** — DSL-based span search (`kind:llm_call model:gpt-4 since:1h`) with structured filter dropdowns and query history
- **Files** — Browse content-addressed file versions
- **Analysis** — Trace diff, context diff, unused context detection, impact analysis
- **Settings** — Daemon status, storage stats, configuration

## Use Cases

### LLM Application Debugging
Trace exactly which files your AI agent reads, what prompts it constructs, and what it produces. When something goes wrong, replay the trace to see the full context.

### Cost Monitoring
Track per-model token usage and cost across all your LLM calls. Use the analytics API to break down spending by model, provider, or time period.

### Context Optimization
Identify unused context — files that were read but didn't influence the output. Reduce token waste by trimming unnecessary context.

### Dataset Curation
Export interesting spans (input/output pairs) directly into datasets. Import external data via CSV. Use the review queue for human labeling workflows.

### Proxy-Based Instrumentation
Drop the proxy in front of any OpenAI-compatible API and get tracing for free — no code changes required.

## Comparison with Similar Tools

| Feature | Traceway | Langfuse | Langsmith | Phoenix |
|---------|----------|----------|-----------|---------|
| Self-hosted | Local-only | Cloud + self-host | Cloud only | Self-host |
| Setup | Single binary | Docker compose | Account required | Docker |
| LLM Proxy | Built-in | No | No | No |
| File tracking | Content-addressed | No | No | No |
| Framework lock-in | None (REST API) | Python/JS SDK | LangChain | LlamaIndex |
| Analytics API | Flexible group-by | Dashboard only | Dashboard only | Limited |
| Dataset management | Built-in | Separate product | Separate product | No |
| Storage | SQLite | Postgres | Cloud | Postgres |

## Development

```sh
# Type-check all crates (skip memfs which needs macFUSE)
cargo check -p trace -p storage -p api -p proxy -p daemon

# Build
cargo build -p daemon

# Run with debug logging
TRACEWAY_LOG=debug cargo run -p daemon -- --foreground

# UI development
cd ui && npm run dev

# UI type-checking
cd ui && npx svelte-check
```

## License

MIT
