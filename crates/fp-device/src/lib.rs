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
