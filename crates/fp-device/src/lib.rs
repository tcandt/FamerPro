use anyhow::{bail, Result};
use async_trait::async_trait;
use fp_domain::{ActionReceipt, DeviceAdapterKind, DeviceDriver, Frame, ScreenGeometry, SwipePath};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};
use tokio::sync::{mpsc, oneshot};

pub enum DeviceCommand {
    Snapshot {
        responder: oneshot::Sender<Result<Frame>>,
    },
    Geometry {
        responder: oneshot::Sender<Result<ScreenGeometry>>,
    },
    Tap {
        x: u32,
        y: u32,
        responder: oneshot::Sender<Result<ActionReceipt>>,
    },
    Swipe {
        path: SwipePath,
        responder: oneshot::Sender<Result<ActionReceipt>>,
    },
    KeyEvent {
        keycode: u32,
        responder: oneshot::Sender<Result<ActionReceipt>>,
    },
    LaunchApp {
        package: String,
        responder: oneshot::Sender<Result<ActionReceipt>>,
    },
    StopApp {
        package: String,
        responder: oneshot::Sender<Result<ActionReceipt>>,
    },
}

#[derive(Clone)]
pub struct DeviceActorHandle {
    pub device_id: String,
    pub adapter_kind: DeviceAdapterKind,
    sender: mpsc::Sender<DeviceCommand>,
    current_generation: Arc<AtomicU64>,
}

impl DeviceActorHandle {
    pub fn new(device_id: String, driver: Arc<dyn DeviceDriver>) -> Self {
        Self::with_adapter(device_id, DeviceAdapterKind::Synthetic, driver)
    }

    pub fn with_adapter(
        device_id: String,
        adapter_kind: DeviceAdapterKind,
        driver: Arc<dyn DeviceDriver>,
    ) -> Self {
        let (sender, mut receiver) = mpsc::channel::<DeviceCommand>(64);

        tokio::spawn(async move {
            while let Some(cmd) = receiver.recv().await {
                match cmd {
                    DeviceCommand::Snapshot { responder } => {
                        let res = driver.snapshot().await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::Geometry { responder } => {
                        let res = driver.geometry().await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::Tap { x, y, responder } => {
                        let res = driver.tap(x, y).await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::Swipe { path, responder } => {
                        let res = driver.swipe(path).await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::KeyEvent { keycode, responder } => {
                        let res = driver.key_event(keycode).await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::LaunchApp { package, responder } => {
                        let res = driver.launch_app(&package).await;
                        let _ = responder.send(res);
                    }
                    DeviceCommand::StopApp { package, responder } => {
                        let res = driver.stop_app(&package).await;
                        let _ = responder.send(res);
                    }
                }
            }
        });

        Self {
            device_id,
            adapter_kind,
            sender,
            current_generation: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn generation(&self) -> u64 {
        self.current_generation.load(Ordering::SeqCst)
    }

    pub fn bump_generation(&mut self) -> u64 {
        self.current_generation.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Sets the active lease generation to at least `min_gen` when coordinating with `MultiDeviceScheduler`.
    pub fn sync_generation(&self, new_gen: u64) {
        self.current_generation.store(new_gen, Ordering::SeqCst);
    }

    fn verify_generation(&self, generation: u64) -> Result<()> {
        let cur = self.generation();
        if generation != cur {
            bail!(
                "STALE_DEVICE_LEASE_GENERATION: Stale device lease generation: {} vs active {}",
                generation,
                cur
            );
        }
        Ok(())
    }

    pub async fn snapshot(&self) -> Result<Frame> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::Snapshot { responder: tx })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn snapshot_fenced(&self, generation: u64) -> Result<Frame> {
        self.verify_generation(generation)?;
        self.snapshot().await
    }

    pub async fn geometry(&self) -> Result<ScreenGeometry> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::Geometry { responder: tx })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn tap(&self, x: u32, y: u32, generation: u64) -> Result<ActionReceipt> {
        self.verify_generation(generation)?;
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::Tap {
                x,
                y,
                responder: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn swipe(&self, path: SwipePath, generation: u64) -> Result<ActionReceipt> {
        self.verify_generation(generation)?;
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::Swipe {
                path,
                responder: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn key_event(&self, keycode: u32, generation: u64) -> Result<ActionReceipt> {
        self.verify_generation(generation)?;
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::KeyEvent {
                keycode,
                responder: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn launch_app(&self, package: &str) -> Result<ActionReceipt> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::LaunchApp {
                package: package.to_string(),
                responder: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn launch_app_fenced(&self, package: &str, generation: u64) -> Result<ActionReceipt> {
        self.verify_generation(generation)?;
        self.launch_app(package).await
    }

    pub async fn stop_app(&self, package: &str) -> Result<ActionReceipt> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::StopApp {
                package: package.to_string(),
                responder: tx,
            })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
    }

    pub async fn stop_app_fenced(&self, package: &str, generation: u64) -> Result<ActionReceipt> {
        self.verify_generation(generation)?;
        self.stop_app(package).await
    }
}

/// Generation-fenced `DeviceDriver` wrapper backed by a single-writer `DeviceActorHandle`.
/// Rejects EVERY command immediately if the device's active lease generation has rotated or been fenced.
pub struct FencedDeviceDriver {
    actor: DeviceActorHandle,
    pinned_generation: u64,
}

impl FencedDeviceDriver {
    pub fn new(actor: DeviceActorHandle, pinned_generation: u64) -> Self {
        Self {
            actor,
            pinned_generation,
        }
    }
}

#[async_trait]
impl DeviceDriver for FencedDeviceDriver {
    async fn snapshot(&self) -> Result<Frame> {
        self.actor.snapshot_fenced(self.pinned_generation).await
    }

    async fn geometry(&self) -> Result<ScreenGeometry> {
        self.actor.verify_generation(self.pinned_generation)?;
        self.actor.geometry().await
    }

    async fn tap(&self, x: u32, y: u32) -> Result<ActionReceipt> {
        self.actor.tap(x, y, self.pinned_generation).await
    }

    async fn swipe(&self, path: SwipePath) -> Result<ActionReceipt> {
        self.actor.swipe(path, self.pinned_generation).await
    }

    async fn key_event(&self, keycode: u32) -> Result<ActionReceipt> {
        self.actor.key_event(keycode, self.pinned_generation).await
    }

    async fn launch_app(&self, package: &str) -> Result<ActionReceipt> {
        self.actor
            .launch_app_fenced(package, self.pinned_generation)
            .await
    }

    async fn stop_app(&self, package: &str) -> Result<ActionReceipt> {
        self.actor
            .stop_app_fenced(package, self.pinned_generation)
            .await
    }
}

/// Manages single-writer `DeviceActorHandle` instances across multiple real/synthetic devices.
#[derive(Clone, Default)]
pub struct DeviceRegistry {
    actors: Arc<RwLock<HashMap<String, DeviceActorHandle>>>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_or_get(
        &self,
        device_id: &str,
        adapter_kind: DeviceAdapterKind,
        driver_factory: impl FnOnce() -> Arc<dyn DeviceDriver>,
    ) -> DeviceActorHandle {
        if let Some(existing) = self.actors.read().unwrap().get(device_id).cloned() {
            return existing;
        }
        let mut map = self.actors.write().unwrap();
        map.entry(device_id.to_string())
            .or_insert_with(|| {
                DeviceActorHandle::with_adapter(
                    device_id.to_string(),
                    adapter_kind,
                    driver_factory(),
                )
            })
            .clone()
    }

    pub fn get(&self, device_id: &str) -> Option<DeviceActorHandle> {
        self.actors.read().unwrap().get(device_id).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fp_testkit::FakeDeviceDriver;

    #[tokio::test]
    async fn test_device_actor_operations() {
        let driver = Arc::new(FakeDeviceDriver::new("fake-01", 1080, 1920));
        let handle = DeviceActorHandle::new("fake-01".to_string(), driver);

        let geom = handle.geometry().await.unwrap();
        assert_eq!(geom.width_px, 1080);
        assert_eq!(geom.height_px, 1920);

        let snap = handle.snapshot().await.unwrap();
        assert!(!snap.bytes.is_empty());

        let launch = handle.launch_app("com.game.test").await.unwrap();
        assert!(launch.success);

        let tap = handle.tap(250, 450, 1).await.unwrap();
        assert!(tap.success);

        let key = handle.key_event(4, 1).await.unwrap();
        assert!(key.success);

        let stop = handle.stop_app("com.game.test").await.unwrap();
        assert!(stop.success);
    }

    #[tokio::test]
    async fn test_generation_fencing() {
        let driver = Arc::new(FakeDeviceDriver::new("fake-02", 1080, 1920));
        let mut handle = DeviceActorHandle::new("fake-02".to_string(), driver);
        let fenced_gen1 = FencedDeviceDriver::new(handle.clone(), 1);

        assert_eq!(handle.generation(), 1);
        assert!(fenced_gen1.tap(100, 100).await.is_ok());

        // Bump generation to 2 -> fenced_gen1 MUST fail immediately across clones!
        let new_gen = handle.bump_generation();
        assert_eq!(new_gen, 2);

        let stale_tap = fenced_gen1.tap(100, 100).await;
        assert!(stale_tap.is_err());
        assert!(stale_tap
            .unwrap_err()
            .to_string()
            .contains("STALE_DEVICE_LEASE_GENERATION"));

        let valid_tap = handle.tap(100, 100, 2).await;
        assert!(valid_tap.is_ok());
    }

    #[tokio::test]
    async fn test_device_actor_hardware_error_resilience() {
        let driver = Arc::new(FakeDeviceDriver::new("fake-03", 1080, 1920));
        let handle = DeviceActorHandle::new("fake-03".to_string(), driver.clone());

        driver.should_fail_next_action.store(true, Ordering::SeqCst);

        let failed_tap = handle.tap(100, 100, 1).await;
        assert!(failed_tap.is_err());

        let recovered_tap = handle.tap(100, 100, 1).await;
        assert!(recovered_tap.is_ok());
    }
}
