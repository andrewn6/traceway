# Fix SQLite Datasets Migration Missing org_id Column

**Labels:** `bug`, `backend`
**Difficulty:** Easy
**Priority:** P0 — Critical

## Summary

The SQLite `datasets` table migration does not include an `org_id` column, but the `save_dataset` query references `org_id` in its INSERT statement. This causes a runtime SQL error when creating datasets in local mode.

## Where

`crates/storage-sqlite/src/lib.rs` — the migration SQL for the `datasets` table and the `save_dataset` method.

## What to do

1. Add `org_id TEXT NOT NULL` to the `datasets` table CREATE statement in the migrations
2. Verify the column is referenced consistently in all dataset queries (INSERT, SELECT, WHERE clauses)
3. If there are existing databases in the wild, add an ALTER TABLE migration to add the column with a default value

## Acceptance criteria

- [ ] `datasets` table has `org_id` column
- [ ] `save_dataset` INSERT succeeds without SQL errors
- [ ] Existing databases are migrated cleanly
- [ ] `cargo check -p storage-sqlite` passes
