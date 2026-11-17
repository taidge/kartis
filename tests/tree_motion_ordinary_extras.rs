//! Pure public-plan proofs. The actual private browser mount is covered separately.
use kartis::*;

fn frame(annotation: bool) -> TreeSvgFrame {
    let config = ChartConfig {
        title: "Tree title / 树标题".into(),
        ..Default::default()
    };
    let source = vec![
        HierarchyDatum::new("r", None, -0.),
        HierarchyDatum::new("a", Some("r".into()), 2.),
    ];
    let options = TreeOptions {
        hierarchy: HierarchyOptions {
            include_root: true,
            labels: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut layout = tree_layout(&config, &source, &options).unwrap();
    assert_eq!(layout.scene.title, config.title);
    if annotation {
        let index = layout.scene.labels.len();
        let mut title = layout.scene.labels[0].clone();
        title.text = "Actual unowned visible title Label".into();
        title.x = 100.;
        title.y = 20.;
        layout.scene.labels.push(title);
        if !layout.scene.draw_order.is_empty() {
            layout.scene.draw_order.push(DrawItem::Label(index));
        }
    }
    layout.scene.validate().unwrap();
    TreeSvgFrame::new(config, layout).unwrap()
}
fn literal(frame: &TreeSvgFrame, layers: Vec<TreeSvgLayer>) -> Vec<DrawItem> {
    TreeSvgPlan::new(
        frame,
        layers,
        TreeSvgBudget::default(),
        TreeSvgExtrasPolicy::RequireLayer,
    )
    .unwrap()
    .layers()
    .iter()
    .flat_map(|o| o.items().iter().copied())
    .collect()
}
fn ordinary_layers() -> Vec<TreeSvgLayer> {
    [
        ("links", TreeSvgLayerKind::Links),
        ("nodes", TreeSvgLayerKind::Nodes),
        ("labels", TreeSvgLayerKind::Labels),
        ("scene-extras", TreeSvgLayerKind::SceneExtras),
        ("mesh", TreeSvgLayerKind::Mesh),
    ]
    .into_iter()
    .map(|(key, kind)| TreeSvgLayer::new(key, kind).unwrap())
    .collect()
}
#[test]
fn title_metadata_alone_does_not_create_scene_extras() {
    let frame = frame(false);
    assert!(frame.extras().is_empty());
    let mut current = TreeSvgPlan::default_layers();
    current.push(TreeSvgLayer::new("scene-extras", TreeSvgLayerKind::SceneExtras).unwrap());
    assert_eq!(literal(&frame, current), frame.layout().scene.draw_items());
}
#[test]
fn ordinary_annotation_current_duplicate_fails_unique_suffix_preserves_actual_order() {
    let frame = frame(true);
    assert_eq!(frame.extras().len(), 1);
    let mut current = TreeSvgPlan::default_layers();
    current.push(TreeSvgLayer::new("scene-extras", TreeSvgLayerKind::SceneExtras).unwrap());
    let source = frame.layout().scene.draw_items();
    let old = literal(&frame, current);
    assert_eq!(old.len(), source.len() + 1);
    assert_ne!(old, source);
    assert_eq!(literal(&frame, ordinary_layers()), source);
}
#[test]
fn ordinary_extras_suffix_cannot_silently_reorder_mixed_source() {
    let original = frame(true);
    let config = original.config().clone();
    let mut layout = original.layout().clone();
    let mut order = layout.scene.draw_items();
    let extra = order.pop().unwrap();
    order.insert(1, extra);
    layout.scene.draw_order = order.clone();
    let frame = TreeSvgFrame::new(config, layout).unwrap();
    assert_ne!(literal(&frame, ordinary_layers()), order);
    assert_eq!(frame.layout().scene.draw_items(), order);
}
