# ADR-002: Tauri v2, React Flow 13, and Tailwind CSS v4

## Status
Accepted

## Context
The visual workflow studio needs an n8n-like graph experience with pan, zoom, minimap, custom typed ports, and contextual node inspectors. It must fit desktop window breakpoints (1600x900 target, 1440x900, 1280x720 responsive fallbacks) without UI clipping.

## Decision
1. **Tauri v2**: Use Tauri v2 for the Windows packaging target (`FarmerPro.exe` and NSIS installer `FarmerPro_*-setup.exe`). It leverages Microsoft Edge WebView2, ensuring low memory footprint and high responsiveness.
2. **React Flow (`@xyflow/react`)**: Chosen as the node-graph engine. Custom nodes adhere to FarmerPro Design Tokens with port indicators for flow and data channels.
3. **Tailwind CSS v4 & FarmerPro Design Tokens**: Design tokens from `FARMERPRO_DESIGN_TOKENS.json` are exposed as CSS variables (`--color-bg-app: #101722`, `--color-accent: #1878F5`, etc.). Full dark navy aesthetic with high contrast (WCAG 2.2 AA compliant).
4. **Zustand & TanStack Query**: Zustand maintains client-side editor draft graph, dirty states, and selection; TanStack Query syncs device summaries, run timelines, and execution logs from Tauri commands.

## Consequences
- No heavyweight Electron or Node runtime bundled into the release installer.
- Custom node components have dedicated type checking for port connections.
- Flexible responsive layout with collapsible sidebar (64px/220px), collapsible palette (240px), collapsible inspector (312px), and resizable execution dock (160px–420px).
