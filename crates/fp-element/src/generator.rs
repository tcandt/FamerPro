use crate::model::ElementSnapshot;
use crate::selectors::{evaluate_native_selector, NativeElementSelector};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GeneratedSelector {
    pub strategy_name: String,
    pub selector: NativeElementSelector,
    pub uniqueness_score: u32, // 0 - 100
    pub match_count: usize,
    pub expression: String,
    pub is_recommended: bool,
}

/// Generates and ranks selectors for a given node in the snapshot
pub fn generate_selectors_for_node(
    snapshot: &ElementSnapshot,
    node_id: usize,
) -> Vec<GeneratedSelector> {
    let node = match snapshot.get_node(node_id) {
        Some(n) => n,
        None => return Vec::new(),
    };

    let mut candidates: Vec<(String, NativeElementSelector, String)> = Vec::new();

    // 1. By Resource ID (if present)
    if !node.resource_id.is_empty() {
        candidates.push((
            "Resource ID".to_string(),
            NativeElementSelector::by_id(&node.resource_id),
            format!("element.id(\"{}\")", node.resource_id),
        ));
    }

    // 2. By Text (if present)
    if !node.text.is_empty() {
        candidates.push((
            "Exact Text".to_string(),
            NativeElementSelector::by_text(&node.text),
            format!("element.text(\"{}\")", node.text),
        ));
    }

    // 3. By Content Description (if present)
    if !node.content_desc.is_empty() {
        candidates.push((
            "Content Description".to_string(),
            NativeElementSelector::by_desc(&node.content_desc),
            format!("element.desc(\"{}\")", node.content_desc),
        ));
    }

    // 4. Compound: Resource ID + Text
    if !node.resource_id.is_empty() && !node.text.is_empty() {
        let mut compound = NativeElementSelector::by_id(&node.resource_id);
        compound.text = Some(node.text.clone());
        candidates.push((
            "ID + Text".to_string(),
            compound,
            format!(
                "element.id(\"{}\").text(\"{}\")",
                node.resource_id, node.text
            ),
        ));
    }

    // 5. Compound: Resource ID + Class
    if !node.resource_id.is_empty() && !node.class_name.is_empty() {
        let mut compound = NativeElementSelector::by_id(&node.resource_id);
        compound.class_name = Some(node.class_name.clone());
        candidates.push((
            "ID + Class".to_string(),
            compound,
            format!(
                "element.id(\"{}\").class(\"{}\")",
                node.resource_id,
                node.short_class()
            ),
        ));
    }

    // 6. XPath
    let xpath_expr = if !node.resource_id.is_empty() && !node.text.is_empty() {
        format!(
            "//node[@resource-id='{}' and @text='{}']",
            node.resource_id, node.text
        )
    } else if !node.resource_id.is_empty() {
        format!("//node[@resource-id='{}']", node.resource_id)
    } else if !node.text.is_empty() {
        format!("//node[@text='{}']", node.text)
    } else {
        format!("//node[@class='{}']", node.class_name)
    };

    candidates.push((
        "XPath".to_string(),
        NativeElementSelector::by_xpath(&xpath_expr),
        format!("element.xpath(\"{}\")", xpath_expr),
    ));

    // Evaluate uniqueness for each candidate
    let mut ranked: Vec<GeneratedSelector> = Vec::new();

    for (name, sel, expr) in candidates {
        let match_count = match evaluate_native_selector(snapshot, &sel, false) {
            Ok(matches) => matches.len(),
            Err(_) => 0,
        };

        let uniqueness_score = if match_count == 1 {
            100
        } else if match_count == 2 {
            50
        } else if match_count > 2 {
            (100 / match_count).max(10) as u32
        } else {
            0
        };

        ranked.push(GeneratedSelector {
            strategy_name: name,
            selector: sel,
            uniqueness_score,
            match_count,
            expression: expr,
            is_recommended: false,
        });
    }

    // Sort by uniqueness score descending, then priority (Resource ID > Text > Compound > XPath)
    ranked.sort_by_key(|b| std::cmp::Reverse(b.uniqueness_score));

    // Mark the first 100-score candidate as recommended
    if let Some(first_unique) = ranked.iter_mut().find(|s| s.uniqueness_score == 100) {
        first_unique.is_recommended = true;
    } else if let Some(first) = ranked.first_mut() {
        first.is_recommended = true;
    }

    ranked
}
