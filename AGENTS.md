# AGENTS.md

Canonical agent handbook for the Traceway monorepo.

The full operational guide — repo layout, quick commands, architecture, and all coding conventions — lives in [`CLAUDE.md`](./CLAUDE.md). Read that first.

## Agent entry points

| Task | Start here |
|---|---|
| Add a backend API endpoint | `.claude/skills/backend-endpoint/SKILL.md` |
| Add or modify a DB table | `.claude/skills/db-migration/SKILL.md` |
| Add or modify a UI page/component | `.claude/skills/ui-route/SKILL.md` |
| Modify Rust crates | `.claude/skills/rust-crate/SKILL.md` |
| Sync frontend types after API changes | `.claude/skills/type-generation/SKILL.md` |
| Validate changes before pushing | `.claude/skills/pre-push-checks/SKILL.md` |

## Key facts for agents

- **Product stack**: Encore.ts API (`backend/app/`) + SvelteKit UI (`ui/`). Run with `encore run` and `npm run dev`.
- **Rust daemon**: Optional. Only needed for OTLP ingest and infra paths.
- **Auth**: All endpoints use `auth: true`. Every query is scoped to `org_id` + `project_id`.
- **Frontend**: Svelte 5 runes only (`$state`, `$derived`, `$effect`). No Svelte 4 stores.
- **Checks before pushing**: `cd ui && npm run check`, `cd backend/app && npm run typecheck`, `cargo check -p trace -p storage -p daemon`.
- **Never** run `cargo check --workspace` or `cargo check -p memfs` — `memfs` requires macFUSE.
- **Commit style**: Short imperative messages. No emoji, no conventional-commit prefixes.
