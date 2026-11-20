use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use fp_domain::{
    ActionReceipt, ConnectionState, DeviceDriver, DeviceSummary, Frame, ScreenGeometry, SwipePath,
};
use std::time::Duration;
use tokio::process::Command;

pub struct AdbRunner {
    pub adb_path: String,
    pub timeout: Duration,
}

impl Default for AdbRunner {
    fn default() -> Self {
        Self {
            adb_path: "adb".to_string(),
            timeout: Duration::from_secs(15),
        }
    }
}

impl AdbRunner {
    pub fn new(adb_path: Option<String>) -> Self {
        Self {
            adb_path: adb_path.unwrap_or_else(|| "adb".to_string()),
            timeout: Duration::from_secs(15),
        }
    }

    pub async fn run_args(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.adb_path);
        cmd.args(args);

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .context("ADB command timed out")?
            .context("Failed to execute ADB command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!(
                "ADB error (exit code {:?}): {}",
                output.status.code(),
                stderr.trim()
            );
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub async fn run_raw_bytes(&self, args: &[&str]) -> Result<Vec<u8>> {
        let mut cmd = Command::new(&self.adb_path);
        cmd.args(args);

        let output = tokio::time::timeout(self.timeout, cmd.output())
            .await
            .context("ADB command timed out")?
            .context("Failed to execute ADB command")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("ADB error: {}", stderr.trim());
        }

        Ok(output.stdout)
    }

    pub async fn list_devices(&self) -> Result<Vec<DeviceSummary>> {
        let output = self.run_args(&["devices", "-l"]).await?;
        Self::parse_devices_output(&output)
    }

    pub fn parse_devices_output(output: &str) -> Result<Vec<DeviceSummary>> {
        let mut devices = Vec::new();

        for line in output.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.is_empty() {
                continue;
            }

            let serial = parts[0].to_string();
            let state_str = if parts.len() > 1 { parts[1] } else { "offline" };

            let connection_state = match state_str {
                "device" => ConnectionState::Ready,
                "unauthorized" => ConnectionState::Unauthorized,
                "authorizing" => ConnectionState::Authorizing,
                "offline" => ConnectionState::Disconnected,
                _ => ConnectionState::Blocked,
            };

            let mut model = None;
            for part in &parts[2..] {
                if let Some(m) = part.strip_prefix("model:") {
                    model = Some(m.to_string());
                }
            }

            let alias = model.clone().unwrap_or_else(|| serial.clone());

            devices.push(DeviceSummary {
                id: serial.clone(),
                adb_serial: serial,
                alias,
                android_api: None,
                model,
                width_px: 1080,
                height_px: 1920,
                rotation_deg: 0,
                connection_state,
                updated_at: chrono::Utc::now(),
            });
        }

        Ok(devices)
    }
}

pub struct AdbDeviceDriver {
    pub serial: String,
    pub runner: AdbRunner,
}

impl AdbDeviceDriver {
    pub fn new(serial: String, runner: AdbRunner) -> Self {
        Self { serial, runner }
    }
}

#[async_trait]
impl DeviceDriver for AdbDeviceDriver {
    async fn snapshot(&self) -> Result<Frame> {
        let bytes = self
            .runner
            .run_raw_bytes(&["-s", &self.serial, "exec-out", "screencap", "-p"])
            .await?;
        let geom = self.geometry().await?;
        Ok(Frame {
            bytes,
            geometry: geom,
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    async fn geometry(&self) -> Result<ScreenGeometry> {
        let wm_size = self
            .runner
            .run_args(&["-s", &self.serial, "shell", "wm", "size"])
            .await
            .unwrap_or_default();

        let mut width = 1080;
        let mut height = 1920;

        for line in wm_size.lines() {
            if let Some(pos) = line.find(':') {
                let res_str = line[pos + 1..].trim();
                let dims: Vec<&str> = res_str.split('x').collect();
                if dims.len() == 2 {
                    if let (Ok(w), Ok(h)) = (dims[0].trim().parse(), dims[1].trim().parse()) {
                        width = w;
                        height = h;
                    }
                }
            }
        }

        Ok(ScreenGeometry::new(width, height, 0))
    }

    async fn tap(&self, x: u32, y: u32) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let x_str = x.to_string();
        let y_str = y.to_string();
        self.runner
            .run_args(&["-s", &self.serial, "shell", "input", "tap", &x_str, &y_str])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: Some(fp_domain::Point {
                x: x as f32,
                y: y as f32,
            }),
            verify_passed: false,
        })
    }

    async fn swipe(&self, path: SwipePath) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        let s_x = path.start_x.to_string();
        let s_y = path.start_y.to_string();
        let e_x = path.end_x.to_string();
        let e_y = path.end_y.to_string();
        let dur = path.duration_ms.to_string();

        self.runner
            .run_args(&[
                "-s",
                &self.serial,
                "shell",
                "input",
                "swipe",
                &s_x,
                &s_y,
                &e_x,
                &e_y,
                &dur,
            ])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn launch_app(&self, package: &str) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        self.runner
            .run_args(&[
                "-s",
                &self.serial,
                "shell",
                "monkey",
                "-p",
                package,
                "-c",
                "android.intent.category.LAUNCHER",
                "1",
            ])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn stop_app(&self, package: &str) -> Result<ActionReceipt> {
        let start = std::time::Instant::now();
        self.runner
            .run_args(&["-s", &self.serial, "shell", "am", "force-stop", package])
            .await?;

        Ok(ActionReceipt {
            success: true,
            duration_ms: start.elapsed().as_millis() as u64,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_adb_devices_output() {
        let mock_out = r#"
List of devices attached
emulator-5554          device product:sdk_gphone64_x86_64 model:Pixel_6 device:emu64x transport_id:1
192.168.1.50:5555      unauthorized transport_id:2
        "#;
        let devices = AdbRunner::parse_devices_output(mock_out).unwrap();
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].adb_serial, "emulator-5554");
        assert_eq!(devices[0].model.as_deref(), Some("Pixel_6"));
        assert_eq!(devices[0].connection_state, ConnectionState::Ready);
        assert_eq!(devices[1].adb_serial, "192.168.1.50:5555");
        assert_eq!(devices[1].connection_state, ConnectionState::Unauthorized);
    }
}
