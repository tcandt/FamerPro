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
pub enum DeviceAdapterKind {
    LdPlayer,
    Nox,
    BlueStacks,
    PhysicalAdb,
    Synthetic,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DevicePortMapping {
    pub direction: String, // "forward" | "reverse"
    pub serial: String,
    pub local: String,
    pub remote: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHealthReport {
    pub serial: String,
    pub reachable: bool,
    pub connection_state: ConnectionState,
    pub width_px: u32,
    pub height_px: u32,
    pub rotation_deg: u16,
    pub latency_ms: u64,
    pub error: Option<String>,
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
    Blocked,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VariableSource {
    LiveAdbOcr,
    LiveState,
    TestkitMock,
    LegacyUnverified,
    #[default]
    Unverified,
}

impl VariableSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::LiveAdbOcr => "LIVE_ADB_OCR",
            Self::LiveState => "LIVE_STATE",
            Self::TestkitMock => "TESTKIT_MOCK",
            Self::LegacyUnverified => "LEGACY_UNVERIFIED",
            Self::Unverified => "UNVERIFIED",
        }
    }

    pub fn is_live(&self) -> bool {
        matches!(self, Self::LiveAdbOcr | Self::LiveState)
    }

    pub fn is_simulated(&self) -> bool {
        matches!(self, Self::TestkitMock)
    }

    pub fn display_label(&self, verified: bool) -> &'static str {
        match self {
            Self::LiveAdbOcr | Self::LiveState => {
                if verified {
                    "LIVE — Đã xác minh"
                } else {
                    "LIVE — Chưa xác minh"
                }
            }
            Self::TestkitMock => "TESTKIT — Mô phỏng",
            Self::LegacyUnverified => "LEGACY — Chưa xác minh",
            Self::Unverified => "Chưa xác minh",
        }
    }
}

pub fn infer_json_value_type(val: &serde_json::Value) -> String {
    match val {
        serde_json::Value::String(_) => "string".to_string(),
        serde_json::Value::Number(_) => "number".to_string(),
        serde_json::Value::Bool(_) => "boolean".to_string(),
        serde_json::Value::Null => "null".to_string(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => "json".to_string(),
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConfidenceSource {
    Measured,
    Heuristic,
    Simulated,
    Uncalibrated,
}

impl ConfidenceSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Measured => "MEASURED",
            Self::Heuristic => "HEURISTIC",
            Self::Simulated => "SIMULATED",
            Self::Uncalibrated => "UNCALIBRATED",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s.trim() {
            "MEASURED" => Some(Self::Measured),
            "HEURISTIC" => Some(Self::Heuristic),
            "SIMULATED" => Some(Self::Simulated),
            "UNCALIBRATED" => Some(Self::Uncalibrated),
            _ => None,
        }
    }
}

/// Canonical provenance-tracked Runtime Variable envelope (P5.1 / Gate 2 OCR Provenance).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVariableEnvelope {
    pub project_id: String,
    pub session_id: String,
    pub job_id: String,
    pub name: String,
    pub value: serde_json::Value,
    pub value_type: String,
    pub source: VariableSource,
    pub verified: bool,
    #[serde(default)]
    pub confidence: Option<f32>,
    #[serde(default)]
    pub confidence_source: Option<ConfidenceSource>,
    #[serde(default)]
    pub measured_confidence: Option<f32>,
    #[serde(default)]
    pub rule_matched: Option<bool>,
    #[serde(default)]
    pub device_id: Option<String>,
    pub updated_at_utc: String,
    pub workflow_version: String,
    #[serde(default)]
    pub seq: u64,
    #[serde(default)]
    pub redacted: bool,
}

/// Typed delta stream payload emitted over Tauri Event (`mini://runtime-var-delta`)
/// and persisted in SQLite `runtime_var_events` for gap/reconnect recovery.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeVarDeltaEvent {
    pub stream_id: String,
    pub scope_key: String,
    pub project_id: String,
    pub session_id: String,
    pub job_id: String,
    pub seq: u64,
    pub workflow_version: String,
    pub emitted_at_utc: String,
    pub deltas: Vec<RuntimeVariableEnvelope>,
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
    async fn key_event(&self, keycode: u32) -> anyhow::Result<ActionReceipt>;
    async fn launch_app(&self, package: &str) -> anyhow::Result<ActionReceipt>;
    async fn stop_app(&self, package: &str) -> anyhow::Result<ActionReceipt>;
    async fn get_ui_tree(&self) -> anyhow::Result<String> {
        anyhow::bail!("get_ui_tree not supported on this driver")
    }
    async fn input_text(&self, _text: &str) -> anyhow::Result<ActionReceipt> {
        anyhow::bail!("input_text not supported on this driver")
    }
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
