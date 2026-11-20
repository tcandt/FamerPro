# ADR-001: Architecture and Modular Cargo Workspace

## Status
Accepted

## Context
FarmerPro requires an Android automation workstation that scales from single-device interactive visual debugging to multi-device (20-50 devices) queue processing. Automation execution must be strictly deterministic, reliable, and separated from UI rendering so that crashes in the UI layer cannot corrupt automation states or leak non-idempotent taps into devices.

## Decision
1. Organize the repository as a Cargo workspace with distinct, single-responsibility crates:
   - `fp-domain`: Core types, identifiers, error handling, driver traits.
   - `fp-adb`: ADB subprocess runner using `tokio::process::Command` without shell string formatting.
   - `fp-device`: Device actor abstraction, device geometry and orientation state.
   - `fp-vision`: On-demand frame analysis, normalized cross-correlation template matching, bbox normalizer.
   - `fp-actions`: Safe random tap policy, exclusion mask processing, coordinate transformations.
   - `fp-workflow-schema`: Versioned JSON schema, acyclic graph validation, bounded loop constraints.
   - `fp-workflow-runtime`: State-machine graph executor with checkpointing, verification and retry.
   - `fp-scheduler`: Multi-device leases, account leases, fencing generations, priority dispatch.
   - `fp-storage`: SQLite WAL storage with forward-only versioned migrations.
   - `fp-security`: Secrets redaction, destructive confirmation guards, OS credential bridge.
   - `fp-testkit`: Synthetic fake device and screenshot fixtures for headless verification.
2. Put the UI in `apps/desktop` powered by Tauri v2 and React 19 + TypeScript.
3. Put the headless automation diagnostic tool in `apps/cli`.

## Consequences
- Business logic is strictly contained in Rust crates and independently unit/integration tested.
- Tauri IPC commands simply dispatch into domain services.
- The UI can be run in headless test environments or mocked browser environments for rapid visual regression testing.
