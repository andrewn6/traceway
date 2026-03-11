# CLAUDE.md

Operational handbook for contributors and coding agents working in the Traceway monorepo.

## What Traceway is

Traceway is an observability platform for LLM applications. SDKs instrument traces/spans in user apps; the backend stores, indexes, and surfaces that data through a web UI.

## Repo layout

| Path | What it is | Language |
|---|---|---|
| `backend/app/` | Encore.ts API — primary product backend | TypeScript |
| `ui/` | SvelteKit 2 + Svelte 5 — primary frontend | Svelte/TS |
| `crates/*` | Rust ingest/infra (daemon, storage backends) | Rust |
| `sdk/python/` | Python SDK (`traceway`) | Python |
| `sdk/typescript/` | TypeScript SDK (`traceway`) | TypeScript |
| `www/` | Marketing landing page (SvelteKit) | Svelte |
| `docs/` | Documentation site (Next.js + Fumadocs) | MDX/React |
| `scripts/` | OpenAPI sync, seed data | Shell |

### Backend services (`backend/app/`)

`auth`, `billing`, `capture_rules`, `core` (schema/migrations), `datasets`, `email`, `evals`, `files`, `provider_connections`, `queue`, `search`, `shared`, `system`, `tracing`, `workflows`

Uses Drizzle ORM for Postgres migrations. Encore handles routing, service discovery, and auth middleware.

### Rust crates (`crates/`)

| Crate | Purpose |
|---|---|
| `trace` | Core types (Span, Trace, SpanKind, EvalRun) — foundational |
| `storage` | StorageBackend trait + PersistentStore wrapper |
| `storage-sqlite` | SQLite backend (local/dev default) |
| `storage-postgres` | Postgres backend (cloud mode) |
| `storage-turbopuffer` | Turbopuffer vector search (secondary index) |
| `auth` | JWT + API key middleware for Axum |
| `daemon` (bin: `traceway`) | Ingestor daemon — OTLP ingest, local daemon lifecycle, wires storage/API/auth |
| `memfs` | FUSE filesystem — **skip in builds** (needs macFUSE) |

## Quick commands

### Run product stack

```sh
# terminal 1 — API
cd backend/app && encore run

# terminal 2 — UI
cd ui && npm install && npm run dev
```

API at `http://localhost:4000`, Encore dashboard at `http://localhost:9400`, UI at `http://localhost:5173`.

### Checks (run before pushing)

```sh
cd ui && npm run check               # Svelte typecheck
cd backend/app && npm run typecheck   # Encore typecheck
cargo check -p trace -p storage -p daemon  # Rust (skip memfs)
```

### Type generation

```sh
cd ui && npm run generate-types   # Regenerate api-types.ts from OpenAPI spec
./scripts/sync-openapi.sh         # Sync openapi.json from running daemon
```

### Makefile shortcuts

`make dev` (daemon), `make dev-ui`, `make dev-all`, `make check`, `make check-all-crates`, `make lint`, `make fmt`, `make sync-openapi`, `make generate-types`

### Optional: Rust daemon

```sh
cargo run -p traceway -- --foreground          # local mode
cargo run -p traceway --features cloud -- --cloud  # cloud mode
```

## Architecture

1. SDKs/apps send traces and spans into Traceway.
2. Encore API (`backend/app`) handles auth, org/project scoping, traces, spans, files, datasets, evals, billing, email.
3. Postgres is source of truth.
4. Turbopuffer mirrors data for vector search (best effort).
5. UI (`ui`) consumes the Encore API only — no direct DB coupling.
6. Rust ingestor daemon handles OTLP ingestion, local daemon lifecycle, and cloud infra processing. Optional for most product UI/API work.

## Conventions

### Product / API

- Fail closed on auth — never show an unauthenticated shell in cloud mode.
- All browser-facing endpoints live in Encore (`backend/app`), not ad hoc side services.
- Preserve org/project boundaries in every new endpoint.
- API handler pattern: `auth::Auth(ctx)` -> `require_scope()` -> `store_for_org()`.

### Frontend (SvelteKit)

- **Svelte 5 runes only** (`$state`, `$derived`, `$effect`) — no Svelte 4 stores.
- Tailwind CSS v4 classes in markup — no CSS modules.
- Dark theme is primary; maintain dark/light parity on major surfaces.
- Reuse design system primitives (`table-float`, `app-shell-wide`, `query-chip`, `surface-panel`, `btn-primary`, etc.) before adding new styles. See `DESIGN_SYSTEM.md` for the full list.
- Floating top nav, dense controls, shared spacing scale.
- Types live in `src/lib/api-types.ts` (auto-generated) and `src/lib/api.ts` (fetch helpers).
- Vite proxies `/api` to `http://localhost:4000`.

### Rust

- Async everywhere (Tokio runtime).
- `thiserror` for error types, `tracing` crate for logging (not `println!`).
- No `unwrap()` in production paths — use `?` or proper error handling.
- Private span fields with read-only accessors (immutability guarantee).
- SpanBuilder for construction; `.complete(output)` / `.fail(error)` consume and produce new spans.
- UUIDv7 (`Uuid::now_v7()`) for time-sortable IDs.
- 409 Conflict returned when modifying terminal spans.
- Skip `memfs` in builds (needs macFUSE).

### Data and contracts

- Regenerate OpenAPI types whenever API shape changes (`npm run generate-types` in `ui/`).
- Maintain backward compatibility for SDK-facing endpoints.
- Drizzle handles Postgres migrations: `npx drizzle-kit generate`.

### Commits and PRs

- Short imperative commit messages. No emoji, no conventional-commit prefixes.
- `cargo check` must pass (all crates except memfs).
- `npm run check` must pass (UI).
- New public Rust types need `Serialize`/`Deserialize`.
- No secrets committed.

## Tooling

- **Package manager:** npm for `ui`, `backend/app`, `docs`, `www`.
- **API runtime:** Encore CLI (`encore run`).
- **Rust build:** Cargo workspace with feature flags (`cloud`, `metrics`).
- **Deployment:** Fly.io (daemon), Vercel (UI/docs/www), Docker multi-stage builds.
- **Dockerfiles:** `Dockerfile` (daemon + UI), `Dockerfile.api` (Encore container), `Dockerfile.ingest` (daemon-only).

## Where to look first

- This file and `DESIGN_SYSTEM.md` for conventions
- `backend/app/README.md` for API details
- `CONTRIBUTING.md` for PR requirements and code style
- `openapi.json` at repo root for the full API spec
