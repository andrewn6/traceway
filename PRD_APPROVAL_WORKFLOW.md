# Traceway Approval Workflow PRD

## Goal

Make Traceway fit real AI engineering workflows where teams review, approve, and audit agent outputs the same way they inspect traces.

The target experience is:

1. Observe execution in Traces.
2. Convert interesting spans/outputs into reviewable artifacts.
3. Approve or edit decisions in a queue.
4. Keep decision history linked back to traces, spans, and files.

## Problem

Current data primitives exist (`datasets`, `datapoints`, `queue`), but the workflow is not yet cohesive enough for day-to-day evaluator and reviewer loops.

Gaps:

- Auth mismatch for headless agents: traces can be API-key driven, approvals historically required session cookies.
- Review UX is generic and not strongly trace-linked.
- No dedicated operator surface for decisions across datasets tied to span context.

## Product Principles

- Keep Traceway visual language and shell; do not clone competitor copy.
- Fail closed on auth and scope boundaries.
- Preserve trace-first context: every approval should be attributable to source execution.
- Keep flow usable both by humans in UI and by agents/CI via SDK/API.

## Primary Users

- AI engineer reviewing agent behavior changes.
- QA/research reviewer triaging low-confidence outputs.
- Team lead auditing approval decisions for reliability and compliance.

## User Stories

- As an engineer, I can send a span result into a review queue in one action.
- As a reviewer, I can claim an item, inspect source trace context, edit if needed, and submit.
- As an agent platform owner, I can run the same flow programmatically with API keys.
- As a lead, I can audit who approved what and from which trace/span/file context.

## Functional Requirements

### 1) API-key compatible approval APIs

- Public dataset/datapoint/queue endpoints accept scoped API auth (not session-only).
- Queue list supports filtering by status and optional dataset ID.
- Export-and-enqueue remains one action from span to queue item.

### 2) SDK headless loop

- SDK can create dataset, create datapoints, enqueue items, claim, and submit using API key mode.
- SDK gracefully handles response-shape differences across backend versions.

### 3) Approval workbench UI

- Dedicated page for approval operations with two-pane pattern:
  - Left: dense queue list with filters and ownership/status metadata.
  - Right: detail panel with editable payload and trace/span linkage.
- Fast actions: claim, approve-as-is, submit-edited.

### 4) Trace-linked context

- Queue item detail exposes dataset, datapoint, and source span linkage when present.
- Reviewer can jump to trace detail directly.

## Non-Goals (for this phase)

- Full policy engine (multi-step approver rules, SLA routing).
- Cross-org workflow orchestration.
- Automatic model retraining pipelines.

## Success Metrics

- Time-to-first-approval from span capture < 2 minutes in local setup.
- 100% of approval actions carry org/project scope + queue item provenance.
- API-key-only flow works end-to-end without browser session.

## Risks

- Scope leakage risk if auth handling is inconsistent across endpoints.
- Reviewer friction if trace context is too buried in the approval view.
- Backward compatibility issues across mixed old/new endpoint response shapes.

## Rollout Plan

1. Auth + API compatibility upgrades.
2. SDK compatibility and example flow refresh.
3. Approval workbench UI with trace links.
4. Iterate with usage feedback and add deeper decision analytics.
