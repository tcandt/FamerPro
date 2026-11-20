use anyhow::{bail, Result};
use fp_domain::{ActionReceipt, DeviceDriver, Frame, ScreenGeometry, SwipePath};
use std::sync::Arc;
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
    sender: mpsc::Sender<DeviceCommand>,
    current_generation: u64,
}

impl DeviceActorHandle {
    pub fn new(device_id: String, driver: Arc<dyn DeviceDriver>) -> Self {
        let (sender, mut receiver) = mpsc::channel::<DeviceCommand>(32);

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
            sender,
            current_generation: 1,
        }
    }

    pub fn generation(&self) -> u64 {
        self.current_generation
    }

    pub fn bump_generation(&mut self) -> u64 {
        self.current_generation += 1;
        self.current_generation
    }

    pub async fn snapshot(&self) -> Result<Frame> {
        let (tx, rx) = oneshot::channel();
        self.sender
            .send(DeviceCommand::Snapshot { responder: tx })
            .await
            .map_err(|_| anyhow::anyhow!("Device actor closed"))?;
        rx.await.map_err(|_| anyhow::anyhow!("Canceled"))?
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
        if generation != self.current_generation {
            bail!(
                "Stale device lease generation: {} vs {}",
                generation,
                self.current_generation
            );
        }
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
        if generation != self.current_generation {
            bail!(
                "Stale device lease generation: {} vs {}",
                generation,
                self.current_generation
            );
        }
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

        let stop = handle.stop_app("com.game.test").await.unwrap();
        assert!(stop.success);
    }

    #[tokio::test]
    async fn test_generation_fencing() {
        let driver = Arc::new(FakeDeviceDriver::new("fake-02", 1080, 1920));
        let mut handle = DeviceActorHandle::new("fake-02".to_string(), driver);

        assert_eq!(handle.generation(), 1);

        // Tap with valid generation 1
        let tap_ok = handle.tap(100, 100, 1).await;
        assert!(tap_ok.is_ok());

        // Bump generation to 2
        let new_gen = handle.bump_generation();
        assert_eq!(new_gen, 2);

        // Tap with stale generation 1 must fail
        let stale_tap = handle.tap(100, 100, 1).await;
        assert!(stale_tap.is_err());
        let err_msg = stale_tap.unwrap_err().to_string();
        assert!(err_msg.contains("Stale device lease generation"));

        // Tap with current generation 2 succeeds
        let valid_tap = handle.tap(100, 100, 2).await;
        assert!(valid_tap.is_ok());
    }

    #[tokio::test]
    async fn test_device_actor_hardware_error_resilience() {
        let driver = Arc::new(FakeDeviceDriver::new("fake-03", 1080, 1920));
        let handle = DeviceActorHandle::new("fake-03".to_string(), driver.clone());

        // Force next action to fail
        driver
            .should_fail_next_action
            .store(true, std::sync::atomic::Ordering::SeqCst);

        let failed_tap = handle.tap(100, 100, 1).await;
        assert!(failed_tap.is_err());

        // Subsequent call must succeed (actor remains alive and responsive)
        let recovered_tap = handle.tap(100, 100, 1).await;
        assert!(recovered_tap.is_ok());
    }
}
