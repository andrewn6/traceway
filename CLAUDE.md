# CLAUDE.md

Operational handbook for contributors and coding agents working in the Traceway monorepo.

## What this repo is now

Traceway is no longer daemon-only. The current product stack is:

- `backend/app`: Encore.ts API (primary product backend)
- `ui`: SvelteKit app (primary frontend)
- `crates/*`: Rust ingest/infra components (still important, but not the default path for most product changes)
- Postgres (source of truth) + Turbopuffer (secondary search index)

## Quick commands

### Run product stack (recommended)

```sh
# terminal 1
cd backend/app && encore run

# terminal 2
cd ui && npm install && npm run dev
```

### Checks

```sh
cd ui && npm run check
cd backend/app && npm run typecheck
cargo check
```

### Optional ingest daemon path

```sh
cargo run -p traceway --features cloud -- --cloud
```

## Architecture snapshot

1. SDKs/apps send traces/spans into Traceway.
2. Encore API (`backend/app`) handles auth, org/project scoping, traces/spans/files, datasets/review, analytics, billing.
3. Data persists in Postgres.
4. Search indexing is mirrored to Turbopuffer (best effort).
5. UI (`ui`) consumes the Encore API only (no direct DB coupling).
6. Rust services handle ingest/runtime infrastructure concerns where needed.

## Best practices (repo-specific)

### Product/API

- Fail closed for auth and API reachability; never show a fake unauthenticated shell in cloud mode.
- Keep browser-facing endpoints in Encore (`backend/app`), not in ad hoc side services.
- Preserve org/project boundaries in all new endpoints.

### Frontend

- Keep the current shell language consistent: floating top nav, dense controls, shared spacing scale.
- Reuse existing primitives (`table-float`, `app-shell-wide`, `query-chip`, etc.) before adding new styles.
- Prefer Svelte local state and derived values; avoid over-engineering state layers.
- Keep dark/light mode parity when changing major surfaces.

### Data and contracts

- Update OpenAPI/types whenever API shape changes.
- Maintain backward compatibility for SDK-facing endpoints where practical.
- Validate query/filter UX against existing DSL behavior before changing semantics.

### Rust side

- Keep ingest/infra concerns in Rust crates.
- Do not move product/API logic from Encore into Rust unless there is a clear runtime need.

## Tooling conventions

- Package manager: npm for `ui` and `backend/app`.
- API runtime: Encore CLI.
- Type checks: `npm run check` (UI), `npm run typecheck` (backend where available).
- Rust checks: `cargo check` / targeted crate checks.
- Docker files at repo root:
  - `Dockerfile.ingest` for daemon/ingest runtime
  - `Dockerfile.api` for Encore API container flow

## Where to look first

- Root overview: `README.md`
- Backend details: `backend/app/README.md`
- Contribution flow: `CONTRIBUTING.md`
