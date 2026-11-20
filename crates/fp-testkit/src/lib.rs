use anyhow::Result;
use async_trait::async_trait;
use fp_domain::{ActionReceipt, Bounds, DeviceDriver, Frame, Point, ScreenGeometry, SwipePath};
use image::{ImageFormat, Rgb, RgbImage};
use std::io::Cursor;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct FakeDeviceDriver {
    pub serial: String,
    pub width: u32,
    pub height: u32,
    pub tap_count: AtomicU32,
    pub swipe_count: AtomicU32,
    pub launched_package: Arc<tokio::sync::Mutex<Option<String>>>,
    pub should_fail_next_action: AtomicBool,
    pub current_screen_state: Arc<tokio::sync::Mutex<String>>,
}

impl FakeDeviceDriver {
    pub fn new(serial: &str, width: u32, height: u32) -> Self {
        Self {
            serial: serial.to_string(),
            width,
            height,
            tap_count: AtomicU32::new(0),
            swipe_count: AtomicU32::new(0),
            launched_package: Arc::new(tokio::sync::Mutex::new(None)),
            should_fail_next_action: AtomicBool::new(false),
            current_screen_state: Arc::new(tokio::sync::Mutex::new("home".to_string())),
        }
    }

    pub fn generate_synthetic_screen_png(
        width: u32,
        height: u32,
        button_bounds: Option<Bounds>,
    ) -> Vec<u8> {
        let mut img = RgbImage::new(width, height);

        for pixel in img.pixels_mut() {
            *pixel = Rgb([16, 23, 34]);
        }

        if let Some(b) = button_bounds {
            let bx = (b.x as u32).min(width);
            let by = (b.y as u32).min(height);
            let bw = (b.width as u32).min(width.saturating_sub(bx));
            let bh = (b.height as u32).min(height.saturating_sub(by));

            for y in by..(by + bh) {
                for x in bx..(bx + bw) {
                    img.put_pixel(x, y, Rgb([24, 120, 245]));
                }
            }
        }

        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);
        img.write_to(&mut cursor, ImageFormat::Png).unwrap();
        buf
    }

    pub fn generate_template_png(width: u32, height: u32) -> Vec<u8> {
        let mut img = RgbImage::new(width, height);
        for pixel in img.pixels_mut() {
            *pixel = Rgb([24, 120, 245]);
        }
        let mut buf = Vec::new();
        let mut cursor = Cursor::new(&mut buf);
        img.write_to(&mut cursor, ImageFormat::Png).unwrap();
        buf
    }
}

#[async_trait]
impl DeviceDriver for FakeDeviceDriver {
    async fn snapshot(&self) -> Result<Frame> {
        let state = self.current_screen_state.lock().await.clone();
        let button = if state == "quest_marker_visible" {
            Some(Bounds::new(200.0, 400.0, 120.0, 60.0))
        } else if state == "quest_dialog_open" {
            Some(Bounds::new(100.0, 800.0, 400.0, 200.0))
        } else {
            None
        };

        let png_bytes = Self::generate_synthetic_screen_png(self.width, self.height, button);
        Ok(Frame {
            bytes: png_bytes,
            geometry: ScreenGeometry::new(self.width, self.height, 0),
            timestamp_ms: chrono::Utc::now().timestamp_millis() as u64,
        })
    }

    async fn geometry(&self) -> Result<ScreenGeometry> {
        Ok(ScreenGeometry::new(self.width, self.height, 0))
    }

    async fn tap(&self, x: u32, y: u32) -> Result<ActionReceipt> {
        if self.should_fail_next_action.swap(false, Ordering::SeqCst) {
            anyhow::bail!("Simulated hardware tap failure");
        }
        self.tap_count.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {
            success: true,
            duration_ms: 12,
            error: None,
            point_tapped: Some(Point {
                x: x as f32,
                y: y as f32,
            }),
            verify_passed: false,
        })
    }

    async fn swipe(&self, _path: SwipePath) -> Result<ActionReceipt> {
        self.swipe_count.fetch_add(1, Ordering::SeqCst);
        Ok(ActionReceipt {
            success: true,
            duration_ms: 150,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn launch_app(&self, package: &str) -> Result<ActionReceipt> {
        let mut pkg = self.launched_package.lock().await;
        *pkg = Some(package.to_string());
        let mut state = self.current_screen_state.lock().await;
        *state = "quest_marker_visible".to_string();

        Ok(ActionReceipt {
            success: true,
            duration_ms: 80,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }

    async fn stop_app(&self, _package: &str) -> Result<ActionReceipt> {
        let mut pkg = self.launched_package.lock().await;
        *pkg = None;
        let mut state = self.current_screen_state.lock().await;
        *state = "home".to_string();

        Ok(ActionReceipt {
            success: true,
            duration_ms: 40,
            error: None,
            point_tapped: None,
            verify_passed: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fake_driver_flow() {
        let driver = FakeDeviceDriver::new("fake-01", 1080, 1920);
        let receipt = driver.launch_app("com.example.game").await.unwrap();
        assert!(receipt.success);

        let snap = driver.snapshot().await.unwrap();
        assert!(!snap.bytes.is_empty());
        assert_eq!(snap.geometry.width_px, 1080);

        let tap_rec = driver.tap(250, 420).await.unwrap();
        assert!(tap_rec.success);
        assert_eq!(driver.tap_count.load(Ordering::SeqCst), 1);
    }
}
