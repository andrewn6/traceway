# Traceway UI Design System

This is the shared visual language for the Traceway product UI.

## Signature feels

1. Grounded sidebar navigation
- Fixed left sidebar (240px) with logo, project switcher, nav, and settings.
- All navigation in the sidebar; no floating bottom rails.

2. Solid surfaces
- Opaque backgrounds with clean borders. No backdrop-filter blur or color-mix transparency.
- Consistent border radius (0.5rem panels, 0.375rem controls).

3. Quiet data plane
- Tables stay calm and information-dense.
- Minimal color noise, strong hierarchy, fast scanability.

4. Enterprise typography
- Inter font for all UI text. Monospace (JetBrains Mono) only for code and data values.
- Clean, readable at small sizes.

## Core primitives

Use these classes before adding new styling:

- Surfaces: `surface-panel`, `surface-command`, `surface-quiet`, `table-float`
- Sidebar: `sidebar-shell`, `sidebar-nav-item`, `sidebar-nav-item-active`, `sidebar-section-label`
- Shell: `app-shell-wide`, `app-toolbar-shell`
- Controls: `control-input`, `control-select`, `control-textarea`
- Buttons: `btn-primary`, `btn-secondary`, `btn-ghost`
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

- Sidebar is the primary navigation anchor.
- Use right docked panels for detail/edit flows with structured data.
- For tables, put filter/columns/search in a compact toolbar above the grid.
- Cmd+K opens search modal for quick navigation.

## Dark/light parity

- Dark theme is primary, light theme has full parity.
- New components must be legible in both themes.
- Use CSS custom properties (--color-*) for all colors.
- Preserve border contrast and focus visibility in light mode.

## Contribution checklist

- Reused existing primitives first
- Kept spacing and typography on system scale
- Confirmed keyboard behavior for overlays/panels
- Ran `cd ui && npm run check`
