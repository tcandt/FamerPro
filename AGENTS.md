# AGENTS.md — FARMERPRO DEVELOPMENT RULES & CONTRACTS

## Core Principles
1. **Source of Truth**: `FARMERPRO_MASTER_BLUEPRINT_V1.0.md` and `FARMERPRO_DESIGN_TOKENS.json`.
2. **Never Mock Success**: Do not state that real ADB actions occurred unless physically tested on real connected hardware. Use `implemented`, `tested`, `blocked`, `proposed` labels explicitly.
3. **No Raw Shell Interpolation**: All ADB commands MUST go through `tokio::process::Command` with explicit arguments vector. Never concatenate shell strings.
4. **Rust Automation Core**: The desktop frontend (Tauri 2 + React + Vite) is an inspector/canvas client. The core workflow engine, device drivers, scheduler, safe tap calculations, and database persist in Rust crates.
5. **Safe Random Tap Policy**: Never hardcode click coordinates. Every interaction target is calculated dynamically inside an observed bounding box with margins applied and exclusion masks subtracted.
6. **No Live Video Streaming in MVP**: Frame capture is strictly on-demand (PNG snapshots for vision detection and verification).
7. **Local First**: Persistence uses SQLite in WAL mode with structured versioned migrations.

## Workspace Layout
- `crates/fp-domain`: Core data structures, identifiers, errors, driver traits.
- `crates/fp-adb`: Tokio-based ADB runner, parsing, USB/TCP device discovery.
- `crates/fp-device`: Device actor abstraction, geometry metadata, orientation transforms.
- `crates/fp-vision`: Frame metadata, template matching, bounding box normalization.
- `crates/fp-actions`: Safe random tap policy, exclusion masks, swipe generator.
- `crates/fp-workflow-schema`: Versioned workflow JSON graph validation.
- `crates/fp-workflow-runtime`: State machine executor, retry, checkpoints, pause/resume.
- `crates/fp-scheduler`: Multi-device leases, priority queue, worker token limits.
- `crates/fp-storage`: SQLite migrations, CRUD for workflows, jobs, step events.
- `crates/fp-security`: Secrets redaction, OS keyring abstraction, destructive confirmations.
- `crates/fp-testkit`: Synthetic fake device driver and golden fixtures for deterministic testing.
- `apps/desktop`: Tauri v2 + React 19 + TypeScript + Vite + Tailwind CSS v4 + React Flow canvas.
- `apps/cli`: Standalone CLI diagnostic runner for testing without GUI.
