pub mod action;
pub mod executor;
pub mod generator;
pub mod model;
pub mod parser;
pub mod selectors;

pub use action::*;
pub use executor::*;
pub use generator::*;
pub use model::*;
pub use parser::*;
pub use selectors::*;

#[cfg(test)]
mod tests {
    use super::*;
    use fp_domain::{Bounds, ScreenGeometry};

    const SAMPLE_SETTINGS_XML: &str = r#"<?xml version='1.0' encoding='UTF-8' standalone='yes' ?>
<hierarchy rotation="0">
  <node index="0" text="" resource-id="" class="android.widget.FrameLayout" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,0][540,960]">
    <node index="0" text="" resource-id="com.android.settings:id/search_action_bar" class="android.view.ViewGroup" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[24,60][516,132]">
      <node index="0" text="Search settings" resource-id="com.android.settings:id/search_action_bar_title" class="android.widget.TextView" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[117,79][282,112]" />
    </node>
    <node index="1" text="" resource-id="" class="android.widget.LinearLayout" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="true" enabled="true" focusable="true" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[0,860][540,960]">
      <node index="0" text="" resource-id="android:id/icon" class="android.widget.ImageView" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[24,888][78,942]" />
      <node index="1" text="Display" resource-id="android:id/title" class="android.widget.TextView" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[108,884][185,917]" />
      <node index="2" text="Styles, wallpapers" resource-id="android:id/summary" class="android.widget.TextView" package="com.android.settings" content-desc="" checkable="false" checked="false" clickable="false" enabled="true" focusable="false" focused="false" scrollable="false" long-clickable="false" password="false" selected="false" bounds="[108,917][512,946]" />
    </node>
  </node>
</hierarchy>"#;

    #[test]
    fn test_parse_ui_hierarchy() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        assert_eq!(snapshot.screen_width, 540);
        assert_eq!(snapshot.screen_height, 960);
        assert_eq!(
            snapshot.package_name,
            Some("com.android.settings".to_string())
        );
        assert_eq!(snapshot.nodes.len(), 7);

        // Find "Display"
        let display_node = snapshot
            .nodes
            .iter()
            .find(|n| n.text == "Display")
            .expect("Display node not found");
        assert_eq!(display_node.resource_id, "android:id/title");
        assert_eq!(display_node.class_name, "android.widget.TextView");
        assert_eq!(display_node.clickable, false);
        assert_eq!(display_node.bounds.x, 108.0);
        assert_eq!(display_node.bounds.y, 884.0);
        assert_eq!(display_node.bounds.width, 77.0);
        assert_eq!(display_node.bounds.height, 33.0);
    }

    #[test]
    fn test_native_selectors_and_clickable_ancestor() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        // 1. Exact text selector
        let sel = NativeElementSelector::by_text("Display");
        let matches = evaluate_native_selector(&snapshot, &sel, true).expect("evaluation failed");
        assert_eq!(matches.len(), 1);

        let m = &matches[0];
        assert_eq!(m.is_clickable, false);
        assert!(m.resolved_clickable_ancestor_id.is_some());
        // Parent LinearLayout bounds
        assert_eq!(m.resolved_bounds.x, 0.0);
        assert_eq!(m.resolved_bounds.y, 860.0);
        assert_eq!(m.resolved_bounds.width, 540.0);
        assert_eq!(m.resolved_bounds.height, 100.0);

        // 2. XPath selector
        let xpath_sel = NativeElementSelector::by_xpath("//node[@text='Search settings']");
        let xpath_matches =
            evaluate_native_selector(&snapshot, &xpath_sel, true).expect("xpath evaluation failed");
        assert_eq!(xpath_matches.len(), 1);
        assert_eq!(xpath_matches[0].is_clickable, false);
        assert!(xpath_matches[0].resolved_clickable_ancestor_id.is_some());
    }

    #[test]
    fn test_ambiguity_enforcement() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        // TextView matches 3 nodes in the sample XML
        let mut ambiguous_sel = NativeElementSelector::default();
        ambiguous_sel.class_name = Some("android.widget.TextView".to_string());

        // When require_unique == true, MUST return AmbiguousMatch error
        let err = evaluate_native_selector(&snapshot, &ambiguous_sel, true)
            .expect_err("should have failed on ambiguity");
        match err {
            ElementError::AmbiguousMatch { count, .. } => {
                assert_eq!(count, 3);
            }
            _ => panic!("Expected AmbiguousMatch error, got {:?}", err),
        }

        // When require_unique == false, returns all 3
        let matches = evaluate_native_selector(&snapshot, &ambiguous_sel, false)
            .expect("non-unique query should succeed");
        assert_eq!(matches.len(), 3);
    }

    #[test]
    fn test_selector_generator() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        let display_node = snapshot.nodes.iter().find(|n| n.text == "Display").unwrap();
        let candidates = generate_selectors_for_node(&snapshot, display_node.id);

        assert!(!candidates.is_empty());
        // Exact text "Display" is unique in this sample, should have score 100
        let text_cand = candidates
            .iter()
            .find(|c| c.strategy_name == "Exact Text")
            .unwrap();
        assert_eq!(text_cand.uniqueness_score, 100);
        assert_eq!(text_cand.match_count, 1);

        // android:id/title is unique in this sample, should have score 100
        let id_cand = candidates
            .iter()
            .find(|c| c.strategy_name == "Resource ID")
            .unwrap();
        assert_eq!(id_cand.uniqueness_score, 100);
    }

    #[test]
    fn test_action_planning_safe_tap() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        let sel = NativeElementSelector::by_text("Display");
        let matches = evaluate_native_selector(&snapshot, &sel, true).unwrap();
        let m = &matches[0];

        let plan = plan_element_action(
            m,
            ElementActionKind::Tap,
            ActionExecutionMode::ClickableParent,
            None,
        )
        .unwrap();

        assert!(plan.is_clickable_parent_used);
        // Tap point should be within parent bounds: [0,860][540,960]
        assert!(plan.safe_point.x >= 0.0 && plan.safe_point.x <= 540.0);
        assert!(plan.safe_point.y >= 860.0 && plan.safe_point.y <= 960.0);
    }

    #[test]
    fn test_find_node_at_point() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        // Click at (150, 900) - inside "Display" TextView [108,884][185,917]
        let found = snapshot
            .find_node_at_point(150.0, 900.0)
            .expect("point not found");
        assert_eq!(found.text, "Display");
    }

    #[test]
    fn test_parse_real_device_xml_settings() {
        let xml_path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../artifacts/element_inspector_poc/settings_hierarchy.xml"
        );
        if let Ok(real_xml) = std::fs::read_to_string(xml_path) {
            let geom = ScreenGeometry::new(540, 960, 0);
            let snapshot = parse_ui_hierarchy(&real_xml, &geom).expect("real XML parse failed");
            assert!(snapshot.nodes.len() > 20);
            assert_eq!(
                snapshot.package_name,
                Some("com.android.settings".to_string())
            );

            // Verify finding "Display" in real XML
            let sel = NativeElementSelector::by_text("Display");
            let matches = evaluate_native_selector(&snapshot, &sel, true)
                .expect("real display search failed");
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0].bounds.width, 77.0);
            assert_eq!(matches[0].bounds.height, 33.0);
            assert!(matches[0].resolved_clickable_ancestor_id.is_some());
        }
    }

    #[tokio::test]
    async fn test_unified_element_executor_flow() {
        let geom = ScreenGeometry::new(540, 960, 0);
        let snapshot = parse_ui_hierarchy(SAMPLE_SETTINGS_XML, &geom).expect("parse failed");

        // 1. Test Detection (Virtual Selector - Native)
        let sel = Selector::Native(NativeElementSelector::by_text("Display"));
        let detect = UnifiedElementExecutor::detect(Some(&snapshot), &sel, None, true)
            .expect("detect failed");
        assert_eq!(detect.candidate_count, 1);
        assert!(!detect.is_clickable); // Node itself is static label
        assert!(detect.resolved_clickable_ancestor_id.is_some()); // Clickable parent resolved!

        // 2. Test Planning
        let plan = UnifiedElementExecutor::plan(
            &detect.matches[0],
            ElementActionKind::Tap,
            ActionExecutionMode::ClickableParent,
            None,
        )
        .expect("plan failed");
        assert!(plan.is_clickable_parent_used);
        assert!(plan.safe_point.x >= 0.0 && plan.safe_point.x <= 540.0);
        assert!(plan.safe_point.y >= 860.0 && plan.safe_point.y <= 960.0);

        // 3. Test Execution via DeviceDriver (FakeDeviceDriver)
        let driver = fp_testkit::FakeDeviceDriver::new("test-device", 540, 960);
        let receipt = UnifiedElementExecutor::execute(&driver, &plan)
            .await
            .expect("execute failed");
        assert!(receipt.success);
        assert_eq!(
            driver.tap_count.load(std::sync::atomic::Ordering::SeqCst),
            1
        );

        // 4. Test Detection on Point Selector
        let pt_sel = Selector::Point(PointSelector {
            x: 0.5,
            y: 0.5,
            is_relative: true,
        });
        let screen_bounds = Bounds::new(0.0, 0.0, 540.0, 960.0);
        let pt_detect = UnifiedElementExecutor::detect(None, &pt_sel, Some(&screen_bounds), true)
            .expect("point detect failed");
        assert_eq!(pt_detect.target_bounds.x, 266.0); // 270 - 4
        assert_eq!(pt_detect.target_bounds.y, 476.0); // 480 - 4
    }
}
