# FarmerPro Design System Tokens

## 1. Color System (Dark Midnight Precision)
Derived from `FARMERPRO_DESIGN_TOKENS.json`.

| Token Name | CSS Variable | Hex Value | Semantic Usage |
|---|---|---|---|
| Background App | `--bg-app` | `#101722` | Deepest root canvas/window background |
| Background Sidebar | `--bg-sidebar` | `#161E29` | Navigation rail / sidebar background |
| Background Surface | `--bg-surface` | `#192230` | Cards, toolbar, table headers |
| Background Elevated | `--bg-elevated` | `#202B3B` | Inspector, dialogs, popovers, drawers |
| Canvas Background | `--bg-canvas` | `#121A26` | React Flow graph background |
| Border Subtle | `--border-subtle` | `#2A3647` | Default dividing lines and card strokes |
| Focus Ring | `--border-focus` | `#4B9CFF` | Accessible focus state, active node stroke |
| Foreground Primary | `--fg-primary` | `#EDF3FC` | High-contrast headings and active labels |
| Foreground Secondary | `--fg-secondary` | `#A9B8CC` | Descriptions and body text |
| Foreground Muted | `--fg-muted` | `#74869D` | Metadata, timestamps, inactive icons |
| Accent Primary | `--accent-primary` | `#1878F5` | Primary action button, selected tabs |
| Accent Hover | `--accent-hover` | `#2F8CFF` | Hover state for accent buttons |
| Accent Soft | `--accent-soft` | `#173355` | Active chips, selected table row highlight |
| Success | `--color-success` | `#3BC38D` | Verification passed, ready device, job complete |
| Warning | `--color-warning` | `#E7B15B` | Retry, wait, authorizing state |
| Danger | `--color-danger` | `#F27883` | Stop, failure, disconnected, destructive action |
| Vision Target | `--color-vision` | `#B291F7` | Image matching nodes, OCR nodes, bbox highlights |
| Node Neutral | `--color-node-neutral` | `#7BA6D6` | Flow control and condition nodes |

## 2. Typography
- **Font Stack**: `Inter, "Segoe UI Variable", "Segoe UI", sans-serif`
- **Body**: 13px / line-height 19px (400 regular, 500 medium)
- **Table Data**: 12px / line-height 16px (tabular numbers enabled)
- **Metadata/Badges**: 11px / line-height 14px (600 semi-bold)
- **Headings**:
  - H1: 22px / line-height 28px (700 bold)
  - H2: 17px / line-height 22px (600 semi-bold)
  - H3: 14px / line-height 20px (600 semi-bold)

## 3. Spacing & Dimensions
- Base 4px grid: `4px, 8px, 12px, 16px, 20px, 24px, 32px`
- **Corner Radii**:
  - Control (Button, Input): `6px`
  - Panel / Card: `9px`
  - Dialog / Modal: `12px`
  - Graph Node: `10px`
- **Layout Measurements**:
  - Title Bar: `38px`
  - Command Bar: `52px`
  - Navigation Rail: `64px` (collapsed), `220px` (expanded)
  - Node Palette: `240px`
  - Inspector: `312px`
  - Execution Dock: `220px` (resizable between 160px and 420px)
  - Target Resolution: `1600×900` (min: `1120×700`)
