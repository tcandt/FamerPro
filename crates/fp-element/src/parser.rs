use crate::model::{ElementError, ElementNode, ElementSnapshot};
use fp_domain::{Bounds, ScreenGeometry};
use quick_xml::events::Event;
use quick_xml::Reader;

/// Parses standard Android UIAutomator XML hierarchy output into an ElementSnapshot
pub fn parse_ui_hierarchy(
    xml_content: &str,
    geometry: &ScreenGeometry,
) -> Result<ElementSnapshot, ElementError> {
    let mut reader = Reader::from_str(xml_content);
    reader.config_mut().trim_text(true);

    let screen_w = geometry.width_px as f32;
    let screen_h = geometry.height_px as f32;
    let screen_bounds = Bounds::new(0.0, 0.0, screen_w, screen_h);

    let mut nodes: Vec<ElementNode> = Vec::new();
    let mut root_ids: Vec<usize> = Vec::new();
    let mut stack: Vec<usize> = Vec::new(); // stack of parent node ids
    let mut rotation_deg = geometry.rotation_deg;
    let mut top_package: Option<String> = None;

    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let qname = e.name();
                let tag_name = qname.as_ref();

                if tag_name == "hierarchy" {
                    for attr in e.attributes().flatten() {
                        let key_qname = attr.key;
                        if key_qname.as_ref() == "rotation" {
                            let val = attr.value.as_ref();
                            if let Ok(rot_code) = val.parse::<u16>() {
                                rotation_deg = match rot_code {
                                    1 => 90,
                                    2 => 180,
                                    3 => 270,
                                    _ => 0,
                                };
                            }
                        }
                    }
                    continue;
                }

                if tag_name == "node" {
                    let node_id = nodes.len();
                    let parent_id = stack.last().copied();
                    let depth = stack.len();

                    let node = parse_node_attributes(
                        e,
                        node_id,
                        parent_id,
                        depth,
                        &nodes,
                        &screen_bounds,
                        &mut top_package,
                    );
                    nodes.push(node);

                    if let Some(pid) = parent_id {
                        nodes[pid].children_ids.push(node_id);
                    } else {
                        root_ids.push(node_id);
                    }

                    stack.push(node_id);
                }
            }
            Ok(Event::Empty(ref e)) => {
                let qname = e.name();
                let tag_name = qname.as_ref();

                if tag_name == "node" {
                    let node_id = nodes.len();
                    let parent_id = stack.last().copied();
                    let depth = stack.len();

                    let node = parse_node_attributes(
                        e,
                        node_id,
                        parent_id,
                        depth,
                        &nodes,
                        &screen_bounds,
                        &mut top_package,
                    );
                    nodes.push(node);

                    if let Some(pid) = parent_id {
                        nodes[pid].children_ids.push(node_id);
                    } else {
                        root_ids.push(node_id);
                    }
                    // Self-closing: do not push to stack
                }
            }
            Ok(Event::End(ref e)) => {
                let qname = e.name();
                let tag_name = qname.as_ref();
                if tag_name == "node" {
                    stack.pop();
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(ElementError::XmlParse(format!(
                    "XML parse error at position {}: {:?}",
                    reader.buffer_position(),
                    e
                )))
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(ElementSnapshot::new(
        chrono::Utc::now().timestamp_millis() as u64,
        rotation_deg,
        geometry.width_px,
        geometry.height_px,
        top_package,
        nodes,
        root_ids,
    ))
}

fn parse_node_attributes(
    e: &quick_xml::events::BytesStart,
    node_id: usize,
    parent_id: Option<usize>,
    depth: usize,
    nodes: &[ElementNode],
    screen_bounds: &Bounds,
    top_package: &mut Option<String>,
) -> ElementNode {
    let mut index: u32 = 0;
    let mut text = String::new();
    let mut resource_id = String::new();
    let mut class_name = String::new();
    let mut package = String::new();
    let mut content_desc = String::new();
    let mut checkable = false;
    let mut checked = false;
    let mut clickable = false;
    let mut enabled = true;
    let mut focusable = false;
    let mut focused = false;
    let mut scrollable = false;
    let mut long_clickable = false;
    let mut password = false;
    let mut selected = false;
    let mut raw_bounds = Bounds::new(0.0, 0.0, 0.0, 0.0);

    for attr in e.attributes().flatten() {
        let key_qname = attr.key;
        let key = key_qname.as_ref();
        let val = attr.value.as_ref();

        match key {
            "index" => index = val.parse().unwrap_or(0),
            "text" => text = val.to_string(),
            "resource-id" => resource_id = val.to_string(),
            "class" => class_name = val.to_string(),
            "package" => {
                if top_package.is_none() && !val.is_empty() {
                    *top_package = Some(val.to_string());
                }
                package = val.to_string();
            }
            "content-desc" => content_desc = val.to_string(),
            "checkable" => checkable = val == "true",
            "checked" => checked = val == "true",
            "clickable" => clickable = val == "true",
            "enabled" => enabled = val != "false",
            "focusable" => focusable = val == "true",
            "focused" => focused = val == "true",
            "scrollable" => scrollable = val == "true",
            "long-clickable" => long_clickable = val == "true",
            "password" => password = val == "true",
            "selected" => selected = val == "true",
            "bounds" => {
                raw_bounds = parse_bounds_string(val);
            }
            _ => {}
        }
    }

    let clipped_bounds = screen_bounds
        .intersect(&raw_bounds)
        .unwrap_or_else(|| Bounds::new(0.0, 0.0, 0.0, 0.0));

    let is_visible =
        clipped_bounds.area() > 0.0 && clipped_bounds.width > 1.0 && clipped_bounds.height > 1.0;

    let path = if let Some(pid) = parent_id {
        let parent_path = &nodes[pid].path;
        format!("{}/{}", parent_path, index)
    } else {
        format!("{}", index)
    };

    ElementNode {
        id: node_id,
        parent_id,
        children_ids: Vec::new(),
        index,
        text,
        resource_id,
        class_name,
        package,
        content_desc,
        checkable,
        checked,
        clickable,
        enabled,
        focusable,
        focused,
        scrollable,
        long_clickable,
        password,
        selected,
        bounds: clipped_bounds,
        raw_bounds,
        depth,
        path,
        is_visible,
    }
}

/// Parses "[x1,y1][x2,y2]" into Bounds
pub fn parse_bounds_string(s: &str) -> Bounds {
    let trimmed = s.trim();
    if !trimmed.starts_with('[') {
        return Bounds::new(0.0, 0.0, 0.0, 0.0);
    }

    let parts: Vec<&str> = trimmed
        .trim_matches(|c| c == '[' || c == ']')
        .split("][")
        .collect();

    if parts.len() != 2 {
        return Bounds::new(0.0, 0.0, 0.0, 0.0);
    }

    let p1: Vec<&str> = parts[0].split(',').collect();
    let p2: Vec<&str> = parts[1].split(',').collect();

    if p1.len() != 2 || p2.len() != 2 {
        return Bounds::new(0.0, 0.0, 0.0, 0.0);
    }

    let x1: f32 = p1[0].trim().parse().unwrap_or(0.0);
    let y1: f32 = p1[1].trim().parse().unwrap_or(0.0);
    let x2: f32 = p2[0].trim().parse().unwrap_or(0.0);
    let y2: f32 = p2[1].trim().parse().unwrap_or(0.0);

    let width = (x2 - x1).max(0.0);
    let height = (y2 - y1).max(0.0);

    Bounds::new(x1, y1, width, height)
}
