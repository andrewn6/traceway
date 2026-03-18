# AGENTS-IMPROVEMENT-SPEC.md

Audit of agent-facing documentation and skills in the Traceway monorepo.
Produced by reviewing: `CLAUDE.md`, `CONTRIBUTING.md`, `DESIGN_SYSTEM.md`,
`README.md`, `PRD_APPROVAL_WORKFLOW.md`, `Makefile`, `Cargo.toml`,
`.devcontainer/devcontainer.json`, `.github/workflows/`, and all skills
under `.claude/skills/`.

---

## What's good

### CLAUDE.md
- Accurate repo layout table with language annotations.
- Rust crate table is complete and correct (matches `Cargo.toml`).
- Quick-command section covers the full dev loop.
- Conventions are specific and actionable (no `unwrap()`, UUIDv7, Svelte 5 runes only, etc.).
- "Where to look first" section gives agents a clear entry point.

### Skills
- `backend-endpoint`: Full step-by-step with code templates, naming conventions, and post-creation checklist. High quality.
- `db-migration`: Column type reference table is useful. Covers the generate → verify loop.
- `pre-push-checks`: Covers all three stacks (UI, backend, Rust) with common-error fixes. Actionable.
- `rust-crate`: Covers types, StorageBackend, error handling, async patterns, and build commands. Solid.
- `type-generation`: Three-option workflow (file / daemon / full sync) is clear.
- `ui-route`: (Not fully retrieved but present and referenced in system prompt.)

### DESIGN_SYSTEM.md
- Lists all primitive class names agents should reuse.
- Density rules and dark/light parity requirements are explicit.

---

## What's missing

### 1. AGENTS.md does not exist
There is no `AGENTS.md` at the repo root. Agents that look for this file (e.g., Codex, Amp, some CI pipelines) find nothing. `CLAUDE.md` fills this role for Claude-based agents but is not universally discovered.

### 2. CONTRIBUTING.md references a stale repo name
`CONTRIBUTING.md` still references `llm-fs` (the old repo name) in clone URLs and directory names. Agents following setup instructions will clone the wrong repo or use wrong paths.

### 3. CONTRIBUTING.md architecture section is outdated
The architecture section in `CONTRIBUTING.md` describes the old Rust-only stack (`crates/api/`, `crates/proxy/`, `crates/daemon/`). The `api` and `proxy` crates no longer exist; the product API is now Encore.ts in `backend/app/`. Agents reading this will look for files that don't exist.

### 4. No skill for the `sdk` packages
There are Python and TypeScript SDKs under `sdk/`. No skill covers how to extend them, add new trace/span methods, or test SDK changes. Agents asked to modify SDK code have no guidance.

### 5. No skill for `docs/` or `www/`
The docs site (Next.js + Fumadocs) and marketing site (SvelteKit) have no skill. Agents asked to update documentation or the landing page have no workflow to follow.

### 6. devcontainer.json has no automations
`.devcontainer/devcontainer.json` uses the universal image but defines no `postCreateCommand`, no `features`, and no port forwarding. Agents that spin up a fresh environment must manually install Encore CLI, run `npm install`, etc. There is no `automations.yaml` to automate service startup.

### 7. No environment variable documentation for agents
`.env.example` exists in `backend/app/` but is not referenced in `CLAUDE.md` or any skill. Agents setting up the backend for the first time don't know which variables are required vs optional, or what values to use locally.

### 8. Issues directory structure is undocumented
`CONTRIBUTING.md` references `issues/` as a flat list but the actual directory has two subdirectories (`customer-exp/`, `infra/`) plus loose files. The index in `CONTRIBUTING.md` is incomplete (missing `cx-*` and `inf-*` issues). Agents looking for work items will miss most of them.

### 9. `check-all-crates` Makefile target is misleading
`make check-all-crates` runs `cargo check --workspace`, which includes `memfs`. `CLAUDE.md` says to skip `memfs`. The target name implies it's safe to run but it will fail in environments without macFUSE. The correct command (`cargo check -p trace -p storage -p daemon`) is documented in `CLAUDE.md` but the Makefile target contradicts it.

### 10. No guidance on the Encore auth middleware pattern
`CLAUDE.md` mentions the handler pattern (`auth::Auth(ctx)` → `require_scope()` → `store_for_org()`) but doesn't explain what `Auth` is, where it comes from, or how to add a new scope. Agents writing new endpoints copy the pattern without understanding it, leading to auth bugs.

### 11. No guidance on the `shared/` service
`backend/app/shared/` is listed in the service table but has no description. Agents don't know what utilities live there or when to use them vs writing new code.

### 12. `superdesign` skill fetches from an external URL at runtime
The `superdesign` skill instructs agents to fetch guidelines from a raw GitHub URL before proceeding. This creates a network dependency and means the skill content is not version-controlled with the repo. If the URL changes or is unavailable, the skill silently fails.

---

## What's wrong

### W1. CONTRIBUTING.md PR checklist is for the old Rust-only stack
The checklist says `cargo check -p api` specifically, but the `api` crate no longer exists. Running this command will fail. The correct check is `cd backend/app && npm run typecheck` for the Encore backend.

### W2. CONTRIBUTING.md setup instructions are broken
The setup section tells contributors to run `cargo build` and `cargo run -p daemon -- --foreground` as the primary dev workflow. The actual product stack requires Encore CLI (`encore run`) and `npm run dev`. Agents following these instructions will not get a working environment.

### W3. README.md clone URL is wrong
`README.md` step 2 says `git clone https://github.com/andrewn6/llm-fs.git` — the old repo name. The correct URL is `https://github.com/andrewn6/traceway.git`.

### W4. `CLAUDE.md` and `CONTRIBUTING.md` give conflicting check commands
`CLAUDE.md` says `cargo check -p trace -p storage -p daemon` (correct).
`CONTRIBUTING.md` says `cargo check` (runs workspace, includes memfs) and `cargo check -p api` (crate doesn't exist).
Agents reading both files will be confused about which is authoritative.

### W5. `Makefile` `check` target only checks daemon, not backend
`make check` runs `check-daemon` (cargo check on daemon) and `check-ui` (svelte-check). It does not run `cd backend/app && npm run typecheck`. Agents running `make check` before pushing will miss backend type errors.

---

## Improvement spec

### Priority 1 — Fix broken instructions (correctness)

**P1-A: Create `AGENTS.md` at repo root**
- Mirror the content of `CLAUDE.md` (or symlink/reference it).
- Add a header noting it is the canonical agent handbook.
- This ensures agents that look for `AGENTS.md` find the right file.

**P1-B: Fix `CONTRIBUTING.md` stale references**
- Replace all `llm-fs` references with `traceway`.
- Replace the architecture section with the current Encore + Rust split.
- Replace the PR checklist item `cargo check -p api` with `cd backend/app && npm run typecheck`.
- Replace the setup section with the correct Encore-first workflow.

**P1-C: Fix `README.md` clone URL**
- Change `https://github.com/andrewn6/llm-fs.git` to `https://github.com/andrewn6/traceway.git`.

**P1-D: Fix `Makefile` `check` target**
- Add `cd backend/app && npm run typecheck` to the `check` target (or create a `check-backend` target and include it).
- Rename or annotate `check-all-crates` to warn that it includes `memfs`.

### Priority 2 — Fill critical gaps (agent effectiveness)

**P2-A: Add Encore auth pattern explanation to `CLAUDE.md`**
- Explain what `auth::Auth(ctx)` is (Encore middleware, injects caller identity).
- Explain `require_scope()` (validates org/project membership).
- Explain `store_for_org()` (returns a DB handle scoped to the org).
- Add a minimal annotated example showing all three in sequence.

**P2-B: Document `shared/` service in `CLAUDE.md`**
- Add a row to the backend services table for `shared/`.
- Describe what utilities it exports (e.g., `validateScope`, `newId`, common error helpers).

**P2-C: Add `sdk` skill**
- Create `.claude/skills/sdk/SKILL.md`.
- Cover: SDK structure, how to add a new method, how to run the Python/TS test suites, backward compatibility rules.

**P2-D: Update issues index in `CONTRIBUTING.md`**
- Add the `customer-exp/` and `infra/` subdirectory issues to the index.
- Or replace the static index with a pointer to the `issues/` directory and its subdirectory structure.

**P2-E: Add devcontainer automations**
- Add `postCreateCommand` to `.devcontainer/devcontainer.json` to install Encore CLI and run `npm install` in `ui/` and `backend/app/`.
- Add port forwarding for 4000 (API), 5173 (UI), 9400 (Encore dashboard).
- Consider adding an `automations.yaml` for service startup.

### Priority 3 — Quality improvements (agent reliability)

**P3-A: Pin `superdesign` skill content locally**
- Copy the fetched guidelines into `.claude/skills/superdesign/SUPERDESIGN.md`.
- Remove the runtime fetch instruction.
- Add a note on how to update it manually.

**P3-B: Add `.env` setup to `CLAUDE.md` quick-start**
- Add `cp backend/app/.env.example backend/app/.env` and `cp ui/.env.example ui/.env` to the "Run product stack" section.
- Note which variables are required for local dev vs optional cloud features.

**P3-C: Add `docs/` and `www/` skills**
- Create `.claude/skills/docs-page/SKILL.md` for adding/editing documentation pages.
- Create `.claude/skills/www-page/SKILL.md` for marketing site changes.
- Both should cover: dev server command, build command, content conventions.

---

## Summary table

| ID | Category | Severity | File(s) affected |
|---|---|---|---|
| W1 | Wrong | High | `CONTRIBUTING.md` |
| W2 | Wrong | High | `CONTRIBUTING.md` |
| W3 | Wrong | Medium | `README.md` |
| W4 | Wrong | Medium | `CLAUDE.md`, `CONTRIBUTING.md` |
| W5 | Wrong | Medium | `Makefile` |
| M1 | Missing | High | (new) `AGENTS.md` |
| M4 | Missing | High | (new) `.claude/skills/sdk/SKILL.md` |
| M10 | Missing | High | `CLAUDE.md` |
| M6 | Missing | Medium | `.devcontainer/devcontainer.json` |
| M7 | Missing | Medium | `CLAUDE.md` |
| M8 | Missing | Medium | `CONTRIBUTING.md` |
| M11 | Missing | Medium | `CLAUDE.md` |
| M9 | Missing | Low | `Makefile` |
| M5 | Missing | Low | (new) skills for `docs/`, `www/` |
| M12 | Missing | Low | `.claude/skills/superdesign/SKILL.md` |
