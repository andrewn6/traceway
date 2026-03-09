# Traceway UI Design System

This is the shared visual language for the Traceway product UI.

## Signature feels

1. Floating command layer
- Bottom command rails are detached from page edges.
- Dense controls + one clear primary action.

2. Detached utility rails
- Modal-like workflows use right floating panels.
- Panels support compact/default/wide sizing.

3. Quiet data plane
- Tables stay calm and information-dense.
- Minimal color noise, strong hierarchy, fast scanability.

## Core primitives

Use these classes before adding new styling:

- Surfaces: `surface-panel`, `surface-command`, `surface-quiet`, `table-float`
- Shells: `app-shell-wide`, `app-toolbar-shell`, `app-page-shell`
- Controls: `control-input`, `control-select`, `control-textarea`
- Buttons: `btn-primary`, `btn-secondary`, `btn-ghost`
- Command input: `command-input-shell`, `command-input`
- Chips: `query-chip`, `query-chip-active`
- Labels: `label-micro`, `table-head-compact`
- Alerts: `alert-danger`, `alert-success`, `alert-warning`
- Auth cards: `auth-card`

## Density rules

- Micro labels: 11px (`label-micro`)
- Body controls/content: 13px baseline
- Strong values/headings: 14-16px
- Avoid introducing new ad-hoc 10px styles unless strictly metadata

## Interaction rules

- Use right floating panel for create/edit/detail flows with structured data.
- Keep command bars anchored and detached; do not dock full-width edge-to-edge.
- For tables, put filter/columns/search in a compact toolbar above the grid.

## Dark/light parity

- New components must be legible in both themes.
- Preserve border contrast and focus visibility in light mode.

## Contribution checklist

- Reused existing primitives first
- Kept spacing and typography on system scale
- Confirmed keyboard behavior for overlays/panels
- Ran `cd ui && npm run check`
