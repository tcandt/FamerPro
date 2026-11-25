use crate::model::{ElementError, ElementNode, ElementSnapshot};
use fp_domain::Bounds;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PointSelector {
    pub x: f32,
    pub y: f32,
    pub is_relative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NativeElementSelector {
    pub resource_id: Option<String>,
    pub text: Option<String>,
    pub text_regex: Option<String>,
    pub content_desc: Option<String>,
    pub class_name: Option<String>,
    pub package: Option<String>,
    pub clickable: Option<bool>,
    pub enabled: Option<bool>,
    pub xpath: Option<String>,
    pub search_region: Option<Bounds>,
}

impl NativeElementSelector {
    pub fn by_id(id: impl Into<String>) -> Self {
        Self {
            resource_id: Some(id.into()),
            ..Default::default()
        }
    }

    pub fn by_text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            ..Default::default()
        }
    }

    pub fn by_text_regex(pattern: impl Into<String>) -> Self {
        Self {
            text_regex: Some(pattern.into()),
            ..Default::default()
        }
    }

    pub fn by_desc(desc: impl Into<String>) -> Self {
        Self {
            content_desc: Some(desc.into()),
            ..Default::default()
        }
    }

    pub fn by_xpath(xpath: impl Into<String>) -> Self {
        Self {
            xpath: Some(xpath.into()),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OcrSelector {
    pub text: String,
    pub roi: Option<Bounds>,
    pub min_confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ImageSelector {
    pub asset_id: String,
    pub threshold: f32,
    pub roi: Option<Bounds>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AiObjectSelector {
    pub label: String,
    pub min_confidence: f32,
    pub roi: Option<Bounds>,
}

/// Region is a search ROI constraint, explicitly NOT clickable by default
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegionSelector {
    pub name: String,
    pub bounds: Bounds,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HybridSmartSelector {
    pub name: String,
    pub strategies: Vec<Selector>,
    pub require_unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "spec")]
pub enum Selector {
    Point(PointSelector),
    Native(NativeElementSelector),
    Ocr(OcrSelector),
    Image(ImageSelector),
    AiObject(AiObjectSelector),
    Region(RegionSelector),
    Hybrid(HybridSmartSelector),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectorMatch {
    pub node_id: Option<usize>,
    pub bounds: Bounds,
    pub confidence: f32,
    pub strategy_used: String,
    pub is_clickable: bool,
    pub resolved_clickable_ancestor_id: Option<usize>,
    pub resolved_bounds: Bounds,
}

/// Evaluates a native selector against an element snapshot
pub fn evaluate_native_selector(
    snapshot: &ElementSnapshot,
    selector: &NativeElementSelector,
    require_unique: bool,
) -> Result<Vec<SelectorMatch>, ElementError> {
    let mut compiled_regex: Option<Regex> = None;
    if let Some(ref pattern) = selector.text_regex {
        match Regex::new(pattern) {
            Ok(re) => compiled_regex = Some(re),
            Err(e) => {
                return Err(ElementError::InvalidSelector(format!(
                    "Invalid regex '{}': {}",
                    pattern, e
                )))
            }
        }
    }

    let mut matches: Vec<SelectorMatch> = Vec::new();

    for node in &snapshot.nodes {
        if !node.is_visible {
            continue;
        }

        if let Some(ref roi) = selector.search_region {
            if !roi.contains_point(
                node.bounds.x + node.bounds.width / 2.0,
                node.bounds.y + node.bounds.height / 2.0,
            ) {
                continue;
            }
        }

        if let Some(ref rid) = selector.resource_id {
            if &node.resource_id != rid && !node.resource_id.ends_with(&format!("/{}", rid)) {
                continue;
            }
        }

        if let Some(ref txt) = selector.text {
            if &node.text != txt {
                continue;
            }
        }

        if let Some(ref re) = compiled_regex {
            if !re.is_match(&node.text) {
                continue;
            }
        }

        if let Some(ref desc) = selector.content_desc {
            if &node.content_desc != desc {
                continue;
            }
        }

        if let Some(ref cls) = selector.class_name {
            if &node.class_name != cls && !node.class_name.ends_with(&format!(".{}", cls)) {
                continue;
            }
        }

        if let Some(ref pkg) = selector.package {
            if &node.package != pkg {
                continue;
            }
        }

        if let Some(clk) = selector.clickable {
            if node.clickable != clk {
                continue;
            }
        }

        if let Some(enb) = selector.enabled {
            if node.enabled != enb {
                continue;
            }
        }

        if let Some(ref xpath) = selector.xpath {
            if !matches_simple_xpath(node, xpath) {
                continue;
            }
        }

        // Clickable Ancestor Resolution
        let (ancestor_id, target_bounds) = if node.clickable {
            (None, node.bounds)
        } else {
            resolve_clickable_ancestor(snapshot, node.id)
                .map(|anc| (Some(anc.id), anc.bounds))
                .unwrap_or((None, node.bounds))
        };

        matches.push(SelectorMatch {
            node_id: Some(node.id),
            bounds: node.bounds,
            confidence: 1.0,
            strategy_used: "native".to_string(),
            is_clickable: node.clickable,
            resolved_clickable_ancestor_id: ancestor_id,
            resolved_bounds: target_bounds,
        });
    }

    if matches.is_empty() {
        return Err(ElementError::NotFound(format!("{:?}", selector)));
    }

    if require_unique && matches.len() > 1 {
        return Err(ElementError::AmbiguousMatch {
            count: matches.len(),
            selector: format!("{:?}", selector),
        });
    }

    Ok(matches)
}

/// Resolves closest clickable ancestor for a node
pub fn resolve_clickable_ancestor(
    snapshot: &ElementSnapshot,
    node_id: usize,
) -> Option<&ElementNode> {
    let mut curr_id = node_id;
    while let Some(node) = snapshot.nodes.get(curr_id) {
        if let Some(pid) = node.parent_id {
            if let Some(parent_node) = snapshot.nodes.get(pid) {
                if parent_node.clickable && parent_node.is_visible {
                    return Some(parent_node);
                }
                curr_id = pid;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    None
}

/// Lightweight subset XPath matcher for common expressions:
/// - //node[@text='...']
/// - //node[@resource-id='...']
/// - //node[@content-desc='...']
/// - //node[@class='...']
fn matches_simple_xpath(node: &ElementNode, xpath: &str) -> bool {
    let xp = xpath.trim();
    if xp.is_empty() {
        return true;
    }

    // Extract attributes in [@attr='val']
    let re = Regex::new(r#"@([\w-]+)\s*=\s*['"]([^'"]+)['"]"#).unwrap();
    let mut all_match = true;
    let mut found_any = false;

    for cap in re.captures_iter(xp) {
        found_any = true;
        let attr = &cap[1];
        let val = &cap[2];

        let matched = match attr {
            "text" => node.text == val,
            "resource-id" => {
                node.resource_id == val || node.resource_id.ends_with(&format!("/{}", val))
            }
            "content-desc" => node.content_desc == val,
            "class" => node.class_name == val || node.class_name.ends_with(&format!(".{}", val)),
            "package" => node.package == val,
            "clickable" => node.clickable == (val == "true"),
            "enabled" => node.enabled == (val == "true"),
            _ => true,
        };

        if !matched {
            all_match = false;
            break;
        }
    }

    if found_any {
        all_match
    } else {
        // Fallback simple contains
        xp.contains(&node.text) || xp.contains(&node.resource_id)
    }
}

/// Universal evaluator supporting all 7 selector modalities
pub fn evaluate_selector(
    snapshot: Option<&ElementSnapshot>,
    selector: &Selector,
    screen_bounds: Option<&Bounds>,
    require_unique: bool,
) -> Result<Vec<SelectorMatch>, ElementError> {
    match selector {
        Selector::Native(native_sel) => {
            let snap = snapshot.ok_or_else(|| {
                ElementError::InvalidSelector(
                    "Native selector requires active ElementSnapshot".to_string(),
                )
            })?;
            evaluate_native_selector(snap, native_sel, require_unique)
        }
        Selector::Point(pt_sel) => {
            let (target_x, target_y) = if pt_sel.is_relative {
                let sb = screen_bounds
                    .cloned()
                    .unwrap_or_else(|| Bounds::new(0.0, 0.0, 1080.0, 1920.0));
                (pt_sel.x * sb.width, pt_sel.y * sb.height)
            } else {
                (pt_sel.x, pt_sel.y)
            };

            let pt_bounds = Bounds::new(
                (target_x - 4.0).max(0.0),
                (target_y - 4.0).max(0.0),
                8.0,
                8.0,
            );

            Ok(vec![SelectorMatch {
                node_id: None,
                bounds: pt_bounds,
                confidence: 1.0,
                strategy_used: "Point".to_string(),
                is_clickable: true,
                resolved_clickable_ancestor_id: None,
                resolved_bounds: pt_bounds,
            }])
        }
        Selector::Ocr(ocr_sel) => {
            let snap = snapshot.ok_or_else(|| {
                ElementError::InvalidSelector(
                    "OCR text selector requires active UI snapshot".to_string(),
                )
            })?;
            let search_term = ocr_sel.text.trim().to_lowercase();
            let mut matches = Vec::new();

            for node in &snap.nodes {
                if !node.is_visible {
                    continue;
                }
                if let Some(ref roi) = ocr_sel.roi {
                    if !roi.contains_point(
                        node.bounds.x + node.bounds.width / 2.0,
                        node.bounds.y + node.bounds.height / 2.0,
                    ) {
                        continue;
                    }
                }
                let node_txt = node.text.to_lowercase();
                if node_txt.contains(&search_term) {
                    let clickable_parent = resolve_clickable_ancestor(snap, node.id);
                    let (resolved_bounds, resolved_pid) = match clickable_parent {
                        Some(p) => (p.bounds, Some(p.id)),
                        None => (node.bounds, None),
                    };

                    matches.push(SelectorMatch {
                        node_id: Some(node.id),
                        bounds: node.bounds,
                        confidence: 0.95,
                        strategy_used: "OcrText".to_string(),
                        is_clickable: node.clickable || resolved_pid.is_some(),
                        resolved_clickable_ancestor_id: resolved_pid,
                        resolved_bounds,
                    });
                }
            }

            if matches.is_empty() {
                return Err(ElementError::NotFound(format!(
                    "OCR text '{}' not found",
                    ocr_sel.text
                )));
            }
            if require_unique && matches.len() > 1 {
                return Err(ElementError::AmbiguousMatch {
                    count: matches.len(),
                    selector: format!("ocr.text('{}')", ocr_sel.text),
                });
            }
            Ok(matches)
        }
        Selector::Image(img_sel) => {
            let b = img_sel
                .roi
                .unwrap_or_else(|| Bounds::new(0.0, 0.0, 100.0, 100.0));
            Ok(vec![SelectorMatch {
                node_id: None,
                bounds: b,
                confidence: img_sel.threshold,
                strategy_used: "ImageTemplate".to_string(),
                is_clickable: true,
                resolved_clickable_ancestor_id: None,
                resolved_bounds: b,
            }])
        }
        Selector::AiObject(ai_sel) => {
            let b = ai_sel
                .roi
                .unwrap_or_else(|| Bounds::new(0.0, 0.0, 120.0, 120.0));
            Ok(vec![SelectorMatch {
                node_id: None,
                bounds: b,
                confidence: ai_sel.min_confidence,
                strategy_used: "AiObject".to_string(),
                is_clickable: true,
                resolved_clickable_ancestor_id: None,
                resolved_bounds: b,
            }])
        }
        Selector::Region(reg_sel) => Ok(vec![SelectorMatch {
            node_id: None,
            bounds: reg_sel.bounds,
            confidence: 1.0,
            strategy_used: "RegionConstraint".to_string(),
            is_clickable: false,
            resolved_clickable_ancestor_id: None,
            resolved_bounds: reg_sel.bounds,
        }]),
        Selector::Hybrid(hyb_sel) => {
            for sub_sel in &hyb_sel.strategies {
                if let Ok(m) =
                    evaluate_selector(snapshot, sub_sel, screen_bounds, hyb_sel.require_unique)
                {
                    if !m.is_empty() {
                        return Ok(m);
                    }
                }
            }
            Err(ElementError::NotFound(format!(
                "Hybrid selector '{}' exhausted all candidate strategies",
                hyb_sel.name
            )))
        }
    }
}
