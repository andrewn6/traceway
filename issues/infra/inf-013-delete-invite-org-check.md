# Add Org Ownership Check to delete_invite

**Labels:** `bug`, `backend`
**Difficulty:** Easy
**Priority:** P2

## Summary

The `DELETE /api/invites/:id` endpoint deletes an invite by ID without verifying that the invite belongs to the authenticated user's organization. A user from org A could delete an invite from org B if they know the invite ID.

## What to do

1. In the delete invite handler, after looking up the invite by ID, verify that `invite.org_id == ctx.org_id`

2. If the org doesn't match, return 404 (not 403 — don't reveal that the invite exists)

3. Pattern to follow — most other delete handlers already do this check. This one was missed.

## Files to modify

- `crates/api/src/auth_routes.rs` — Add org_id check in the delete invite handler

## Acceptance criteria

- [ ] Deleting an invite from another org returns 404
- [ ] Deleting an invite from your own org still works
- [ ] `cargo check -p api` passes
