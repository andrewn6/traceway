# Development Guide

LLM observability platform. Rust backend (Axum + SQLite) with SvelteKit frontend.

## Cursor Cloud specific instructions

### Services

| Service | Port | Start command |
|---------|------|---------------|
| Backend (daemon) | 3000 (API), 3001 (LLM proxy) | `cargo run --manifest-path crates/daemon/Cargo.toml -- --foreground` |
| UI (SvelteKit) | 5173 | `cd ui && npx vite dev --port 5173 --host 0.0.0.0` |
| Landing page (www) | 5174 | `cd www && npm run dev` |

The UI proxies `/api` requests to the daemon at `localhost:3000` via Vite config.

### Key gotchas

- **rust-embed requires `ui/build/` directory**: The `api` crate uses `rust-embed` to embed the UI build at compile time. If `ui/build/` doesn't exist, Rust compilation will fail. Run `mkdir -p ui/build` before building the backend if no UI production build has been done.
- **Missing `org_id` migration in `storage-sqlite`**: The `storage-sqlite` crate queries `org_id` from the `datasets` table but no migration adds this column. On first run, the daemon may error with "no such column: org_id". Fix by running: `sqlite3 ~/.<app-data-dir>/traces.db "ALTER TABLE datasets ADD COLUMN org_id TEXT;"` (the data dir name is redacted but discoverable from daemon startup logs).
- **Rust default toolchain**: The system may ship with an older Rust (1.83). Ensure `rustup default stable` is set so that the latest stable Rust is used. The project needs at least Rust 1.88+.
- **System deps**: `pkg-config`, `libssl-dev`, and `libfuse3-dev` are required for building the full workspace (including the `memfs` crate).

### Lint / Check / Test

See `Makefile` for all available targets. Key commands:
- `cargo clippy --workspace -- -W clippy::all` — Rust linting
- `cargo fmt --all --check` — Rust formatting check (note: repo has pre-existing format drift)
- `cd ui && npx svelte-check --tsconfig ./tsconfig.json` — UI type checking
- `cd www && npx svelte-check --tsconfig ./tsconfig.json` — Landing page type checking

### Package managers

- **Rust**: Cargo (workspace at root)
- **UI** (`/ui`): Bun (`bun.lock` present); the Makefile uses `npm` but Bun is the canonical choice per Dockerfile
- **Landing page** (`/www`): npm (`package-lock.json` present)
