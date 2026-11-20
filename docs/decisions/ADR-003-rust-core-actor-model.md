# ADR-003: Rust Automation Core and Per-Device Actor Model

## Status
Accepted

## Context
When controlling multiple physical or emulated Android devices, race conditions and concurrent command interleaving on the same device must be prevented. If two threads send `input tap` simultaneously, game UI states desynchronize, causing erratic automation behavior.

## Decision
1. **DeviceActor**: Each connected device has exactly one `DeviceActor` task holding an exclusive command channel (`tokio::sync::mpsc`). All input actions (`tap`, `swipe`, `launch_app`) and captures are executed strictly sequentially for that device.
2. **DeviceLease & Fencing**: When a job starts, it acquires a `DeviceLease` with an incrementing `generation` token. If a lease expires or a disconnection occurs, subsequent actions with a stale generation are rejected.
3. **Driver Abstraction**: Defined `DeviceDriver` trait in `fp-domain`:
   - `snapshot(&self) -> Result<Frame>`
   - `geometry(&self) -> Result<ScreenGeometry>`
   - `tap(&self, x: u32, y: u32) -> Result<ActionReceipt>`
   - `swipe(&self, path: SwipePath) -> Result<ActionReceipt>`
   - `launch_app(&self, package: &str) -> Result<ActionReceipt>`
   Implemented by `AdbDeviceDriver` in `fp-adb` and `FakeDeviceDriver` in `fp-testkit`.

## Consequences
- Single active input workflow per device invariant is guaranteed at the architectural level.
- Multi-device concurrency is scaled horizontally by multiplexing Tokio actor tasks across devices.
- Unit and integration tests run headlessly and deterministically with `FakeDeviceDriver`.
