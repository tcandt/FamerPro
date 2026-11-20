use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowDocument {
    pub schema_version: u32,
    pub workflow_id: String,
    pub name: String,
    pub status: String,
    pub graph: WorkflowGraph,
    pub runtime_policy: Option<RuntimePolicy>,
    #[serde(default)]
    pub disclaimer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowNode {
    pub id: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub position: NodePosition,
    #[serde(default)]
    pub config: serde_json::Value,
    pub timeout_ms: Option<u64>,
    pub retry_policy: Option<NodeRetryPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodePosition {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowEdge {
    pub id: String,
    pub source: String,
    pub source_port: String,
    pub target: String,
    pub target_port: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NodeRetryPolicy {
    pub max_attempts: u32,
    pub backoff_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimePolicy {
    pub one_active_input_workflow_per_device: bool,
    pub retry: Option<NodeRetryPolicy>,
    pub on_unknown_input_result: Option<String>,
    pub on_failure: Option<String>,
    pub end_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationReport {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub struct WorkflowValidator;

impl WorkflowValidator {
    pub fn validate(workflow: &WorkflowDocument) -> ValidationReport {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Check schema version
        if workflow.schema_version == 0 {
            errors.push("Invalid schemaVersion: must be >= 1".to_string());
        }

        // 2. Check duplicate node IDs
        let mut node_ids = HashSet::new();
        let mut start_count = 0;
        let mut end_count = 0;

        for node in &workflow.graph.nodes {
            if !node_ids.insert(node.id.clone()) {
                errors.push(format!("Duplicate node id detected: '{}'", node.id));
            }

            if node.node_type == "flow.start" {
                start_count += 1;
            } else if node.node_type == "flow.end" {
                end_count += 1;
            }

            // Bounded loop verification
            if node.node_type == "logic.loop" {
                if let Some(max_iter) = node.config.get("maxIterations") {
                    if max_iter.as_u64().unwrap_or(0) == 0 {
                        errors.push(format!(
                            "Loop node '{}' must have maxIterations > 0",
                            node.id
                        ));
                    }
                } else {
                    errors.push(format!(
                        "Loop node '{}' missing maxIterations configuration",
                        node.id
                    ));
                }
            }
        }

        if start_count != 1 {
            errors.push(format!(
                "Workflow must have exactly one 'flow.start' node (found {})",
                start_count
            ));
        }

        if end_count < 1 {
            errors.push("Workflow must have at least one 'flow.end' node".to_string());
        }

        // 3. Check edges and port references
        let mut edge_ids = HashSet::new();
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();

        for edge in &workflow.graph.edges {
            if !edge_ids.insert(edge.id.clone()) {
                errors.push(format!("Duplicate edge id detected: '{}'", edge.id));
            }

            if !node_ids.contains(&edge.source) {
                errors.push(format!(
                    "Edge '{}' references nonexistent source node '{}'",
                    edge.id, edge.source
                ));
            }
            if !node_ids.contains(&edge.target) {
                errors.push(format!(
                    "Edge '{}' references nonexistent target node '{}'",
                    edge.id, edge.target
                ));
            }

            adj.entry(&edge.source).or_default().push(&edge.target);
        }

        // 4. Reachability from Start
        if let Some(start_node) = workflow
            .graph
            .nodes
            .iter()
            .find(|n| n.node_type == "flow.start")
        {
            let mut visited = HashSet::new();
            let mut queue = vec![start_node.id.as_str()];
            visited.insert(start_node.id.as_str());

            while let Some(curr) = queue.pop() {
                if let Some(neighbors) = adj.get(curr) {
                    for &nbr in neighbors {
                        if visited.insert(nbr) {
                            queue.push(nbr);
                        }
                    }
                }
            }

            for node in &workflow.graph.nodes {
                if !visited.contains(node.id.as_str()) {
                    warnings.push(format!(
                        "Node '{}' ({}) is unreachable from Start",
                        node.id, node.node_type
                    ));
                }
            }
        }

        ValidationReport {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    pub fn parse_and_validate(json_str: &str) -> Result<WorkflowDocument> {
        let doc: WorkflowDocument = serde_json::from_str(json_str)?;
        let report = Self::validate(&doc);
        if !report.is_valid {
            bail!("Workflow validation failed: {:?}", report.errors);
        }
        Ok(doc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_sample_workflow_json() {
        let sample_json = r#"{
            "schemaVersion": 1,
            "workflowId": "test-flow",
            "name": "Test Flow",
            "status": "draft",
            "graph": {
                "nodes": [
                    { "id": "n-start", "type": "flow.start", "position": { "x": 0.0, "y": 0.0 } },
                    { "id": "n-end", "type": "flow.end", "position": { "x": 100.0, "y": 0.0 } }
                ],
                "edges": [
                    { "id": "e-1", "source": "n-start", "sourcePort": "flow", "target": "n-end", "targetPort": "flow" }
                ]
            }
        }"#;

        let doc = WorkflowValidator::parse_and_validate(sample_json).unwrap();
        assert_eq!(doc.workflow_id, "test-flow");
        assert_eq!(doc.graph.nodes.len(), 2);
    }

    #[test]
    fn test_missing_start_node() {
        let invalid_json = r#"{
            "schemaVersion": 1,
            "workflowId": "no-start",
            "name": "No Start",
            "status": "draft",
            "graph": {
                "nodes": [
                    { "id": "n-end", "type": "flow.end", "position": { "x": 100.0, "y": 0.0 } }
                ],
                "edges": []
            }
        }"#;

        let res = WorkflowValidator::parse_and_validate(invalid_json);
        assert!(res.is_err());
    }
}
