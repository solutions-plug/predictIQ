# Design System

This document describes the visual language and component conventions used across the trading terminal frontend.

## Principles

- **Density over decoration.** Traders need to scan a lot of information quickly. Favor compact layouts, tight spacing, and minimal chrome.
- **Data first.** Numbers, prices, and statuses are the primary content. UI elements should recede.
- **Consistency.** Colors, spacing, and typography are shared across all panels and views.
- **Dark by default.** The terminal is designed for long sessions in low-light environments.

## Color

Colors are defined as CSS custom properties in `frontend/src/styles/tokens.css`.

| Token | Usage |
| --- | --- |
| `--color-bg` | Application background |
| `--color-surface` | Panel and card backgrounds |
| `--color-border` | Dividers and outlines |
| `--color-text` | Primary text |
| `--color-text-muted` | Secondary and label text |
| `--color-positive` | Gains, buy side, healthy status |
| `--color-negative` | Losses, sell side, error status |
| `--color-warning` | Warnings and degraded status |
| `--color-accent` | Interactive highlights and focus |

## Typography

- **UI text:** system sans-serif stack, defined by `--font-ui`.
- **Numeric and tabular data:** monospace stack, defined by `--font-mono`. Always use tabular figures for prices, quantities, and timestamps so columns align.
- **Scale:** `--text-xs` through `--text-xl`. Body copy is `--text-sm`; dense tables use `--text-xs`.

## Spacing

Spacing follows a 4px base unit exposed as `--space-1` through `--space-8`. Prefer the smallest spacing that preserves legibility.

## Components

### Panels

Panels are the primary container. They have a surface background, a 1px border, and a compact header with a title and optional actions.

### Tables

Data tables use sticky headers, zebra-free rows separated by borders, and right-aligned numeric columns. Row height is fixed to keep scanning predictable.

### Status indicators

Status is conveyed with a colored dot plus a text label. Never rely on color alone.

## Representative components

The following components exemplify the trading-terminal style and are good references when building new UI:

| Component | Path | Why it's representative |
| --- | --- | --- |
| App shell chrome | `frontend/src/components/AppShell.tsx` | Defines the top bar, side navigation, and panel layout that frame every view. |
| Data table | `frontend/src/components/DataTable.tsx` | Sticky headers, right-aligned numeric columns, and fixed row heights for dense scanning. |
| Status indicator | `frontend/src/components/StatusIndicator.tsx` | Colored dot plus text label, demonstrating the color-plus-label status convention. |
| Panel | `frontend/src/components/Panel.tsx` | The standard surface container with compact header and optional actions. |
| Price cell | `frontend/src/components/PriceCell.tsx` | Monospace tabular figures with positive/negative coloring for numeric data. |

## Accessibility

- Maintain a minimum contrast ratio of 4.5:1 for text.
- Ensure all interactive elements are reachable and operable via keyboard.
- Pair color with text or iconography so status is never conveyed by color alone.
- Respect `prefers-reduced-motion` for any animation.