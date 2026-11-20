# FarmerPro — Android Workflow Automation Studio

FarmerPro is a professional desktop automation studio for Android (supporting Android 9–13) built with a high-performance **Rust 2024** core engine and a **Tauri v2 + React 19 + TypeScript + React Flow** visual graph interface.

## Highlights
- **Visual Node Canvas**: Drag-and-drop workflow editor inspired by n8n, tailored for mobile automation.
- **Dynamic Safe Tap**: No hardcoded coordinates. Targets are dynamically located via template matching, shrunk by safe margins, subtracted by exclusion zones, and sampled with recent-point avoidance.
- **Verification-First Execution**: Commands verify post-conditions with fresh screen captures before confirming success or advancing checkpoints.
- **Robust Scheduling**: Single active serialized action queue per device, fencing generations, bounded vision workers, and multi-device support.
- **Local-First & Secure**: SQLite WAL persistence, secrets stored in OS credential store, redacted log traces, no required cloud logins.

## Architecture
See `docs/decisions/` and `docs/design/` for architecture decision records and design specifications.
- `apps/desktop`: Tauri v2 Windows shell with Vite + React + TypeScript + Tailwind CSS v4.
- `apps/cli`: CLI runner for headless workflows, device diagnostics, and testing.
- `crates/`: Modular Rust crates for domain, adb, device actor, vision, actions, workflow schema, runtime, scheduler, storage, and security.

## Building
```powershell
# Prerequisites: Rust 1.96+, Node 24+, pnpm 11+, Android Platform Tools (adb)
# Build frontend
cd apps/desktop
pnpm install
pnpm build

# Run Rust tests
cd ../..
cargo test --workspace

# Build Tauri desktop app
cd apps/desktop
pnpm tauri build
```
