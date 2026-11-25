use fp_domain::{Bounds, ScreenGeometry};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ElementError {
    #[error("XML parsing error: {0}")]
    XmlParse(String),

    #[error("Element not found with selector: {0}")]
    NotFound(String),

    #[error(
        "Ambiguous match: found {count} elements for selector '{selector}' (unique match required)"
    )]
    AmbiguousMatch { count: usize, selector: String },

    #[error("Element is offscreen or occluded (bounds: {0:?})")]
    OffscreenOrOccluded(Bounds),

    #[error("Snapshot is stale: age {age_ms}ms exceeds max allowed {max_age_ms}ms")]
    StaleSnapshot { age_ms: u64, max_age_ms: u64 },

    #[error(
        "Orientation mismatch: snapshot rotation {snapshot_deg}° != device rotation {device_deg}°"
    )]
    OrientationMismatch { snapshot_deg: u16, device_deg: u16 },

    #[error("Action failed: {0}")]
    ActionFailed(String),

    #[error("Invalid selector syntax: {0}")]
    InvalidSelector(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementNode {
    pub id: usize,
    pub parent_id: Option<usize>,
    pub children_ids: Vec<usize>,
    pub index: u32,
    pub text: String,
    pub resource_id: String,
    pub class_name: String,
    pub package: String,
    pub content_desc: String,
    pub checkable: bool,
    pub checked: bool,
    pub clickable: bool,
    pub enabled: bool,
    pub focusable: bool,
    pub focused: bool,
    pub scrollable: bool,
    pub long_clickable: bool,
    pub password: bool,
    pub selected: bool,
    pub bounds: Bounds,
    pub raw_bounds: Bounds,
    pub depth: usize,
    pub path: String,
    pub is_visible: bool,
}

impl ElementNode {
    pub fn display_label(&self) -> String {
        if !self.text.is_empty() {
            format!("\"{}\" ({})", self.text, self.short_class())
        } else if !self.content_desc.is_empty() {
            format!("[{}] ({})", self.content_desc, self.short_class())
        } else if !self.resource_id.is_empty() {
            format!("{} ({})", self.short_id(), self.short_class())
        } else {
            self.short_class()
        }
    }

    pub fn short_class(&self) -> String {
        self.class_name
            .rsplit('.')
            .next()
            .unwrap_or(&self.class_name)
            .to_string()
    }

    pub fn short_id(&self) -> String {
        self.resource_id
            .rsplit('/')
            .next()
            .unwrap_or(&self.resource_id)
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ElementSnapshot {
    pub id: String,
    pub timestamp_ms: u64,
    pub rotation_deg: u16,
    pub screen_width: u32,
    pub screen_height: u32,
    pub package_name: Option<String>,
    pub nodes: Vec<ElementNode>,
    pub root_ids: Vec<usize>,
}

impl ElementSnapshot {
    pub fn new(
        timestamp_ms: u64,
        rotation_deg: u16,
        screen_width: u32,
        screen_height: u32,
        package_name: Option<String>,
        nodes: Vec<ElementNode>,
        root_ids: Vec<usize>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp_ms,
            rotation_deg,
            screen_width,
            screen_height,
            package_name,
            nodes,
            root_ids,
        }
    }

    pub fn get_node(&self, id: usize) -> Option<&ElementNode> {
        self.nodes.get(id)
    }

    pub fn ancestors(&self, mut id: usize) -> Vec<usize> {
        let mut list = Vec::new();
        while let Some(node) = self.nodes.get(id) {
            if let Some(pid) = node.parent_id {
                list.push(pid);
                id = pid;
            } else {
                break;
            }
        }
        list
    }

    pub fn descendants(&self, id: usize) -> Vec<usize> {
        let mut list = Vec::new();
        let mut queue = vec![id];
        while let Some(curr_id) = queue.pop() {
            if let Some(node) = self.nodes.get(curr_id) {
                for &child_id in &node.children_ids {
                    list.push(child_id);
                    queue.push(child_id);
                }
            }
        }
        list
    }

    pub fn find_node_at_point(&self, x: f32, y: f32) -> Option<&ElementNode> {
        // Find all nodes containing the point, pick the deepest / smallest area
        let mut matches: Vec<&ElementNode> = self
            .nodes
            .iter()
            .filter(|n| n.is_visible && n.bounds.contains_point(x, y))
            .collect();

        if matches.is_empty() {
            return None;
        }

        // Sort by area ascending (smallest node first) then depth descending (deepest first)
        matches.sort_by(|a, b| {
            let area_a = a.bounds.area();
            let area_b = b.bounds.area();
            area_a
                .partial_cmp(&area_b)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| b.depth.cmp(&a.depth))
        });

        Some(matches[0])
    }

    pub fn validate_freshness(
        &self,
        max_age_ms: u64,
        current_geom: Option<&ScreenGeometry>,
    ) -> Result<(), ElementError> {
        let now_ms = chrono::Utc::now().timestamp_millis() as u64;
        let age_ms = now_ms.saturating_sub(self.timestamp_ms);

        if age_ms > max_age_ms {
            return Err(ElementError::StaleSnapshot { age_ms, max_age_ms });
        }

        if let Some(geom) = current_geom {
            if geom.rotation_deg != self.rotation_deg {
                return Err(ElementError::OrientationMismatch {
                    snapshot_deg: self.rotation_deg,
                    device_deg: geom.rotation_deg,
                });
            }
        }

        Ok(())
    }
}
