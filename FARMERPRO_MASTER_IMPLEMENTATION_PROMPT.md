# MASTER IMPLEMENTATION PROMPT — FARMERPRO

> **Copy toàn bộ prompt này vào Codex / Claude Code / Gemini CLI / một coding agent có quyền làm việc trong repo FarmerPro.**  
> **Nguồn sự thật:** `FARMERPRO_MASTER_BLUEPRINT_V1.0.md` trong cùng gói.  
> **Trạng thái:** đây là YÊU CẦU triển khai, không phải báo cáo mã nguồn đã tồn tại.  
> **Ngày đóng gói:** 20/11/2026.

---

## ROLE / AUTHORITY

Bạn là nhóm kỹ thuật đa chức năng gồm: Principal Rust Engineer, Senior Windows/Tauri Engineer, Product Design Director, Senior UI/UX Designer, React/TypeScript Tech Lead, Android Automation Engineer, QA Automation Lead và Security Engineer. Bạn phải đưa ra sản phẩm có thể build, kiểm thử và bàn giao, không chỉ mock giao diện.

**Không được đưa ra tuyên bố chưa kiểm chứng.** Báo cáo phân biệt rõ `implemented`, `tested`, `blocked`, `proposed`; khi chưa chạy lệnh/test trên máy cụ thể, phải viết `NOT TESTED` hoặc `BLOCKED`, không giả lập PASS.

## OBJECTIVE

Tạo mới toàn bộ sản phẩm **FarmerPro** chạy Windows, đóng gói bằng Tauri 2 thành chương trình `FarmerPro.exe` và bộ cài NSIS `FarmerPro_*-setup.exe`. Dùng **Rust làm lõi xử lý automation**; UI premium dark navy/blue bằng Vite + React + TypeScript + React Flow + shadcn/ui/Tailwind. UX tạo workflow kéo thả kiểu n8n; điều khiển Android 9–13 bằng ADB USB/TCP; OCR/image recognition/safe-region random tap; từng node được kiểm tra, workflow có verify/retry/checkpoint; có task catalog và đường mở rộng 20–50 thiết bị.

**ĐẶC BIỆT:** MVP **KHÔNG** làm streaming video, không WebRTC, không cloud phone, không APK Agent, không phỏng đoán API ẩn từ APK/game và không giả mạo dữ liệu kết quả.

## INPUTS

- Read the ENTIRE `FARMERPRO_MASTER_BLUEPRINT_V1.0.md` and `FARMERPRO_DESIGN_TOKENS.json`.
- Refer to user-approved screenshot of dark “AutoFarm Workflow Studio”: retain midnight navy + electric blue accents only; redesign shell/layout/visual hierarchy from scratch.
- `FARMERPRO_SAMPLE_WORKFLOW.json` is an illustrative workflow contract; migrate/adjust with explicit schema version; never silently treat its sample outputs as production results.
- StarCi Harness `https://github.com/tcandt/starci-harness` can be used to manage AI coding contracts/quality gates **if owner-approved and installed**, but it is not FarmerPro's runtime. Read its rules before invoking; do not modify harness repo as a shortcut.

## NON-NEGOTIABLE DESIGN RULES

1. Product visual quality must be comparable to polished commercial developer tools; avoid generic AI admin dashboard styling.
2. Matte dark navy, solid flat colors; accents limited to actionable/selected/status. No full-page gradients/glass, no garish neon, no noisy shadows.
3. Re-design Sidebar, App Header, Command Bar, Workflow Canvas, Inspector, Device List, Task Library, Runtime Monitor and Settings from first principles.
4. Grid and panels resize without clipping at 1600×900, 1440×900 and 1280×720. At smaller widths collapse inspector/palette into overlays.
5. React Flow canvas is the hero: pan/zoom/fit, edge handling, type-safe ports, custom nodes, keyboard shortcuts, Ctrl+S/Undo/Redo, selected state and viewport controls.
6. Node Library and Inspector are contextual, not permanent clutter. The bottom execution dock must be resizable/collapsible.
7. Use one primitive system (shadcn/ui/Radix ownership) and FarmerPro design tokens. Lucide icons consistently; accessible labels/focus states.
8. Never show synthetic metrics (e.g. “50 online”) as real; fixture mode labeled `DEMO`.
9. Before coding functional pages, create component gallery/stories and screenshot evidence for review. Must demonstrate responsive variants.
10. Design acceptance is a documented rubric score, not an empty claim of “10/10”.

## NON-NEGOTIABLE RUST & AUTOMATION RULES

1. ADB/device/vision/workflow/scheduler/persistence core is Rust; Tauri IPC is a thin adapter; React must not spawn arbitrary shell commands or own automation semantics.
2. Use `tokio::process::Command` with separate argv args, timeout, cancellation and per-device action serialization. Do not interpolate untrusted user input into shell command strings.
3. DeviceDriver trait abstracts ADB USB/TCP; mock driver supports deterministic tests. Android 9–13 support must be verified on hardware, not assumed.
4. Screenshot on demand for Find Image/OCR/Verify; no live streaming for MVP.
5. No fixed screen click coordinates. `RandomSafeTap` only consumes a freshly observed bbox/polygon + geometry + exclusions, samples a valid point, avoids recent clicks when possible and verifies the result. Reject stale/low-confidence/unsafe targets.
6. ADB tap receipt does not mean the game accepted the action. Verify postcondition using a new observation. Unknown status after disconnect must not replay a non-idempotent action blindly.
7. Workflow graph typed/versioned, bounded loops, explicit branches, deadlines, retry policies, checkpoint only after verified state.
8. One active input/action workflow per device; account lease unique; multiple devices via asynchronous actors; bounded OCR/CPU concurrency and backpressure.
9. Credentials in Windows Credential Manager/OS keyring; logs redact secrets; destructive tasks like app-data clear require permission and confirmation.
10. Account/proxy support only in the phase scheduled by blueprint; never claim Android system proxy applies to every game. Request-node only for authorized/public APIs.
11. Local-first with SQLite migrations; no Docker, no permanent cloud requirement.
12. EXE target is NSIS Installer + compiled binary; document WebView2 dependency, ADB distribution strategy, code signing and clean-Windows smoke test.

## PHASE EXECUTION — DO NOT JUMP AHEAD

### Phase 0 — Inspection, architecture and visual skill

- Check actual repository state, host OS, toolchain, branch and existing user changes. Do not overwrite work.
- Create `docs/decisions/` for key Architecture Decision Records and `docs/design/` for tokens, page map, shell architecture, component inventory.
- Build or render a high-fidelity UI gallery: Shell, Workflow Studio, Task Library, Devices, Runtime Monitor. Show ready/loading/empty/error states.
- Screenshot 1600×900 / 1440×900 / 1280×720; report overflow and usability assessment (scored by rubric).
- Validate the primary product structure with static fixtures. No claim ADB works at this phase.

**STOP/GATE:** only proceed when UI design has approval or clear pre-approved phase criteria; if user isn't available, proceed with documented best-effort assumptions but mark approval pending.

### Phase 1 — Bootstrap & packaging proof

- Rust cargo workspace and modular crates from blueprint; Vite React TS app; Tauri 2; shadcn/ui; Tailwind v4; React Flow.
- Implement app shell, design token CSS variables and system status; connect a typed `app_get_status()` Tauri command; SQLite migration bootstrap.
- `cargo fmt`, `cargo clippy`, `cargo test`, UI lint/typecheck/build; validate that compiled Windows binary and NSIS installer appear.
- Installer smoke on a clean Windows VM when available; if unavailable, report `NOT TESTED ON CLEAN WINDOWS`.

### Phase 2 — Node workflow editor

- Build NodeRegistry and typed ports in frontend; palette, canvas, inspector; schema-driven config and validation; save/load/import/export.
- Demo workflow: Start → Launch App → Capture → Find Image → Random Safe Tap → Verify → Checkpoint → End.
- Edge validation, branch true/false, bounded loops, duplicate IDs, no nonexistent ports, versioning and dirty state.
- Implement persisted draft and immutable published snapshots.

### Phase 3 — ADB + screenshot + vision + action engine

- ADB discover/select/inspect and screenshot metadata. Avoid no-serial ambiguous commands when multiple devices connected.
- Matching engine: template matching first, OCR adapter next if configured and verified. Low confidence must fail closed.
- Coordinate transform across rotations/scales/insets. Safe random point sampling with exclusion masks.
- Verify on fresh image after action; logs include target bounding box, RNG point, geometry and evidence refs.
- Tests on synthetic fixtures first, then 3 real Android 9–13 devices when available.

### Phase 4 — Durable workflow executor and debugger

- Compile workflow into validated graph; executor with device actor, timeout/retry/backoff/verify, checkpoint, cancel/pause/resume.
- Runtime UI: exact node status, progress, timeline, structured logs, screenshot on demand, unknown-state handling.
- Task templates: beginner quest and auto-combat as configurable subworkflows using game-specific observable targets; never assume universal game action semantics.
- Test 100 runs on a controlled test application/game where permitted; report actual sample size and failure reasons.

### Phase 5 — task planner and multi-device

- Task catalog, selected task set, priorities, prerequisites, skip/wait semantics, per-account task state, end-policy.
- Device/account leases + fencing, fairness, bounded Vision pool, queue backpressure; staged load test 5 → 20 → 50.
- Clearly distinguish simulated 50-device load vs measured 50 physical devices.

### Phase 6 — accounts, network, security and release

- Secret references, account verification and switching wizard, network profile capabilities and actual connection checks.
- Destructive operations require explicit confirmation and audit. No reverse-engineered private game API bypass.
- Installer tests, signed/unsigned policy, hash, user docs, uninstall behavior, final acceptance report.

### Phase 7 — future only

- Android Agent, cloud control, streaming/WebRTC, distributed gateway: **out of MVP**. Produce RFCs only if asked.

## DELIVERY FORMAT AFTER EVERY PHASE

Report in Vietnamese using this structure:

```text
PHASE N — <Title>
Result: PASS / PARTIAL / BLOCKED
Files changed: <exact paths>
Commit: <hash or NOT COMMITTED>
Commands executed: <commands + actual exit codes>
Tests: <pass/fail/skip + reasons>
UI Evidence: <screenshots with actual paths>
Rust/ADB Evidence: <device / simulator logs, redacted>
Windows EXE Evidence: <exact binary and installer paths / not built>
Known limitations: <specific>
Contract compliance: <requirement IDs fulfilled>
Next steps: <smallest verified increment>
```

Do not claim you executed commands not run. Do not report mock screenshots as application screenshots. Do not push to GitHub/publish packages unless current authorization and the selected harness's approval rules permit; where permission is insufficient, commit locally and show exact pending action.

## QUALITY / VERIFICATION MATRIX

- Rust: formatting, clippy warnings as errors, unit/integration/property-based tests, no unsafe shell expansion.
- Frontend: TypeScript strict, lint, component tests, React Flow graph tests, keyboard accessibility tests.
- UI: keyboard-focus visible, color-contrast review, 3 viewport screenshots, usable panel resize, polished empty/error states.
- Storage: migrations/upgrades and corrupt-run recovery; secrets redaction and no sensitive screenshot leakage.
- Device: Android 9/11/13 (or explain missing physical device), orientation/brand differences, ADB unauthorized/offline handling.
- Workflow: deterministic test driver, no repeat of unknown non-idempotent action, no unbounded loop, one active action per device.
- Release: Windows `.exe` binary plus NSIS `-setup.exe`, WebView2 prerequisites documented, real install/uninstall smoke.

## PRODUCT PAGE SPEC SUMMARY

1. **Overview**: real KPI/attention queue/current jobs/recent runs, no fake charts.
2. **Devices**: sortable/virtualized table + contextual inspector + screenshot on demand.
3. **Workflow Library**: status and versioned templates; create/duplicate/import/archive.
4. **Workflow Studio**: editable graph canvas + palette + inspector + execution dock.
5. **Tasks**: selected task set, drag priorities, ready/wait/skip states, plan preview.
6. **Runs**: timeline and exact step logs with verify evidence, pause/resume/stop.
7. **Accounts**: alias + secret references + safe assignment.
8. **Network/Proxy**: per-device network capability, health check, limitations.
9. **Settings/Diagnostics**: theme/paths/ADB/storage/audit/about.

## TECHNICAL SOURCE RULE

When uncertain about framework APIs (especially React Flow 13, Tauri 2, Tailwind 4), consult CURRENT OFFICIAL documentation and pin a compatible release; do not blindly paste outdated snippets. Cite URLs in the architectural decision notes. Never use an outdated Tauri 1 config to produce an alleged Tauri 2 app.

## DEFINITION OF DONE FOR THE FIRST DEMO

A clean Windows desktop launch with brand-new FarmerPro UI; editable React Flow graph; real Tauri IPC to Rust; SQLite save/load; ability to list ADB devices and capture PNG from one authorized Android; Find Image returns bbox/confidence or clear failure; Random Safe Tap chooses a new valid point for the recognized target; step-by-step verify/retry/checkpoint; runtime log shows evidence. Windows setup executable builds and can be installed on a test machine. No video streaming.

If an essential external prerequisite is missing (Android device, ADB binary, Windows build OS, credentials), implement and run deterministic mock integration tests for that slice, report the hardware part `BLOCKED`, and do not make unsupported success claims.

**START WITH PHASE 0 NOW.** First inspect sources/environment and describe actual findings; then follow the phase gates. Make tangible progress, not only a plan.
