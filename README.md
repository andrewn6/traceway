# Traceway
<img width="2870" height="1140" alt="853b4311ceb992ef099677d1ddd0696e56406a2f6b0e9ad8ca7f3a940b20da19" src="https://github.com/user-attachments/assets/ce066845-5a6a-44af-a48e-22d8c317c33e" />

Observability platform for LLM applications. Capture traces, spans, token usage, cost, and file I/O — then analyze everything through a web UI and REST API.

Runs locally with zero config, or deploy to the cloud with Turbopuffer + Postgres.

<!-- Add a screenshot: drag an image into a GitHub issue/PR to get a URL, then paste it here -->
<!-- ![Traceway UI](https://github.com/user-attachments/assets/your-screenshot.png) -->

## Features

- **Structured tracing** — Spans with typed kinds: `llm_call`, `fs_read`, `fs_write`, `custom`
- **Transparent LLM proxy** — Drop-in proxy for Ollama/OpenAI; traces captured automatically
- **Token & cost tracking** — Per-model input/output token counts and cost
- **File versioning** — Content-addressed snapshots of every file read/written during a trace
- **Analytics** — Aggregate cost, tokens, latency with flexible group-by queries and a configurable dashboard
- **Datasets** — Export spans to datasets, import CSVs, label via a review queue
- **Real-time events** — SSE stream for live trace/span updates
- **Web UI** — Trace explorer, span waterfall, query builder, analytics dashboard, file browser

## Quick Start (Local)

Single binary, SQLite storage, no dependencies.

```sh
git clone https://github.com/andrewn6/llm-fs.git
cd llm-fs

cargo build -p daemon --release
./target/release/daemon --foreground
```

API at `localhost:3000`, proxy at `localhost:3001`. Open `localhost:3000` for the UI.

```sh
curl http://localhost:3000/api/health
```

## Cloud Mode

Turbopuffer for trace storage, Postgres for auth, JWT sessions.

```sh
# .env (auto-loaded by the daemon)
STORAGE_BACKEND=turbopuffer
TURBOPUFFER_API_KEY=tpuf_xxx
TURBOPUFFER_NAMESPACE=traceway
DATABASE_URL=postgresql://user:pass@host/db
PORT=3000

# Build with cloud feature and run
cargo build -p daemon --release --features cloud
./target/release/daemon --cloud
```

Cloud mode adds:
- **Multi-tenant auth** — Signup/login, orgs, API keys, JWT sessions
- **Turbopuffer storage** — Namespaced vector-native storage for traces, spans, datasets
- **Postgres auth store** — Users, orgs, API keys, invites with auto-migrations

## SDKs

- **Python** — `sdk/python/`
- **TypeScript** — `sdk/typescript/`

```python
from traceway import Traceway

tw = Traceway()
with tw.trace("my-task") as trace:
    with trace.span("planning", kind="llm_call", model="gpt-4") as span:
        result = call_llm(...)
        span.complete(output=result)
```

## Architecture

```
┌─────────────┐      ┌─────────────┐
│  Your App   │      │  LLM Server │
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
│   SQLite (local) or Turbopuffer      │
└──────────────────────────────────────┘
```

## Development

```sh
# Backend
cargo check -p trace -p storage -p api -p proxy -p daemon
TRACEWAY_LOG=debug cargo run -p daemon -- --foreground

# UI
cd ui && npm install && npm run dev    # localhost:5173

# Cloud backend
cargo run -p daemon --features cloud -- --cloud
```

## License

MIT
