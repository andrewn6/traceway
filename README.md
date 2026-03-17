# Traceway

Observability for LLM apps: traces, spans, tokens, cost, datasets, review queues, and analytics.

## README Navigation

- [Quick Start](#quick-start)
- [Running](#running)
- [Contributing](#contributing)
- [Architecture](#architecture)
- [Traceway API (Encore)](#traceway-api-encore)
- [Ingestor Daemon (Rust)](#ingestor-daemon-rust)
- [Full System Architecture](#full-system-architecture)

## Quick Start

### 1) Install prerequisites

- Node.js 20+
- npm
- Encore CLI (required for the new API): <https://encore.dev/docs/ts/install>

Quick check:

```sh
encore version
node -v
```

### 2) Clone and configure

```sh
git clone https://github.com/andrewn6/traceway.git
cd traceway
cp backend/app/.env.example backend/app/.env
cp ui/.env.example ui/.env
```

### 3) Run backend API (Encore)

```sh
cd backend/app
encore run
```

### 4) Run UI (new terminal)

```sh
cd ui
npm install
npm run dev
```

Local endpoints:

- API: `http://localhost:4000`
- Encore dashboard: `http://localhost:9400`
- UI: `http://localhost:5173`

Smoke test:

```sh
curl http://localhost:4000/health
```

## Running

### Product stack (recommended)

```sh
# Terminal 1
cd backend/app && encore run

# Terminal 2
cd ui && npm run dev
```

### Optional: Rust ingestor/infra daemon

```sh
cargo run -p traceway --features cloud -- --cloud
```

Use this when working on ingest/infra paths (not required for normal UI + API product development).

## Contributing

- Start with [CONTRIBUTING.md](./CONTRIBUTING.md)
- Backend service docs: [`backend/app/README.md`](./backend/app/README.md)
- UI docs: [`ui/README.md`](./ui/README.md)

## Architecture

Traceway now runs with an **Encore-first product API**.

- `backend/app` (Encore.ts): browser-facing API, auth, org/project domains, traces/spans/files, datasets/queue/evals, analytics, billing.
- `ui` (SvelteKit): primary web app.
- Rust components: ingest + infra responsibilities.
- Postgres: source of truth.
- Turbopuffer: secondary search index (best-effort mirror).

## Traceway API (Encore)

Location: `backend/app`

Core responsibilities:

- Auth and session flows
- Trace/span CRUD and live updates (SSE)
- File-version and dataset/review workflows
- Analytics and dashboard queries
- Provider connection and billing endpoints

Dev workflow:

```sh
cd backend/app
encore run
```

Generate OpenAPI + UI types (from repo root):

```sh
encore gen client --lang openapi --output "openapi.json"
npx openapi-typescript "openapi.json" -o "ui/src/lib/api-types.ts"
```

## Ingestor Daemon (Rust)

The Rust side is moving toward focused ingest/infra duties:

- OTLP and ingestion-related runtime paths
- local daemon lifecycle/config utilities
- infrastructure-specific processing

It is optional for most product UI/API changes.

## Full System Architecture

```text
Your App / SDKs
      |
      v
Traceway API (Encore, backend/app)  <---->  UI (SvelteKit, ui/)
      |
      +----> Postgres (primary source of truth)
      |
      +----> Turbopuffer (secondary search index)
      |
      +----> Optional Rust ingest/infra services
```

## SDKs

- Python: `sdk/python/`
- TypeScript: `sdk/typescript/`

## License

MIT
