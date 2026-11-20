use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScreenGeometry {
    pub width_px: u32,
    pub height_px: u32,
    pub rotation_deg: u16, // 0, 90, 180, 270
    pub capture_ts_ms: u64,
}

impl ScreenGeometry {
    pub fn new(width_px: u32, height_px: u32, rotation_deg: u16) -> Self {
        Self {
            width_px,
            height_px,
            rotation_deg,
            capture_ts_ms: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Bounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Bounds {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }

    pub fn area(&self) -> f32 {
        if self.width <= 0.0 || self.height <= 0.0 {
            0.0
        } else {
            self.width * self.height
        }
    }

    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        px >= self.x && px <= self.right() && py >= self.y && py <= self.bottom()
    }

    pub fn intersect(&self, other: &Bounds) -> Option<Bounds> {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 > x1 && y2 > y1 {
            Some(Bounds {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VisionTarget {
    pub bounds: Bounds,
    pub confidence: f32,
    pub geometry: ScreenGeometry,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub bytes: Vec<u8>,
    pub geometry: ScreenGeometry,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionReceipt {
    pub success: bool,
    pub duration_ms: u64,
    pub error: Option<String>,
    pub point_tapped: Option<Point>,
    pub verify_passed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwipePath {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionState {
    Disconnected,
    Unauthorized,
    Authorizing,
    Ready,
    Busy,
    Recovering,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceSummary {
    pub id: String,
    pub adb_serial: String,
    pub alias: String,
    pub android_api: Option<u32>,
    pub model: Option<String>,
    pub width_px: u32,
    pub height_px: u32,
    pub rotation_deg: u16,
    pub connection_state: ConnectionState,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceDetail {
    pub summary: DeviceSummary,
    pub battery_level: Option<u8>,
    pub is_screen_on: bool,
    pub ip_address: Option<String>,
    pub installed_packages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Ready,
    Running,
    Waiting,
    Verifying,
    Succeeded,
    Failed,
    Paused,
    Unknown,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StepEventType {
    CommandSent,
    DeviceAck,
    VerificationPassed,
    VerificationFailed,
    Unknown,
    CheckpointSaved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEvent {
    pub id: String,
    pub job_id: String,
    pub node_id: String,
    pub event_type: StepEventType,
    pub attempt_no: u32,
    pub at: DateTime<Utc>,
    pub payload_json: String,
}

#[async_trait]
pub trait DeviceDriver: Send + Sync {
    async fn snapshot(&self) -> anyhow::Result<Frame>;
    async fn geometry(&self) -> anyhow::Result<ScreenGeometry>;
    async fn tap(&self, x: u32, y: u32) -> anyhow::Result<ActionReceipt>;
    async fn swipe(&self, path: SwipePath) -> anyhow::Result<ActionReceipt>;
    async fn launch_app(&self, package: &str) -> anyhow::Result<ActionReceipt>;
    async fn stop_app(&self, package: &str) -> anyhow::Result<ActionReceipt>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bounds_intersection() {
        let b1 = Bounds::new(10.0, 10.0, 100.0, 100.0);
        let b2 = Bounds::new(50.0, 50.0, 100.0, 100.0);
        let inter = b1.intersect(&b2).expect("Must intersect");
        assert_eq!(inter.x, 50.0);
        assert_eq!(inter.y, 50.0);
        assert_eq!(inter.width, 60.0);
        assert_eq!(inter.height, 60.0);
        assert!(inter.contains_point(60.0, 60.0));
        assert!(!inter.contains_point(20.0, 20.0));
    }

    #[test]
    fn test_bounds_no_intersection() {
        let b1 = Bounds::new(0.0, 0.0, 10.0, 10.0);
        let b2 = Bounds::new(20.0, 20.0, 10.0, 10.0);
        assert!(b1.intersect(&b2).is_none());
    }
}
