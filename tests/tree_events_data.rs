use std::cell::Cell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 320.,
        margin: Margin {
            left: 31.,
            top: 23.,
            right: 47.,
            bottom: 37.,
        },
        ..Default::default()
    }
}

fn source(zero: f64) -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("leaf", Some("branch".into()), zero),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("branch", Some("root".into()), 0.),
        HierarchyDatum::new("other", Some("root".into()), 5.),
    ]
}

#[test]
fn shared_event_pool_preserves_original_order_raw_values_and_computed_endpoints() {
    let c = config();
    let raw = source(-0.);
    let l = tree_layout(&c, &raw, &TreeOptions::default()).unwrap();
    let f = TreeEventFrame::new(c.clone(), l.clone()).unwrap();
    let root = f.find_node("root").unwrap();
    assert_eq!(root.source_index(), 1);
    assert_eq!(root.datum().value.to_bits(), (-0_f64).to_bits());
    assert_eq!(root.node().value, 5.);
    assert_eq!(root.frame().source().len(), 4);
    assert_eq!(root.frame().config(), &c);
    assert_eq!(root.frame().theme(), &c.theme);
    assert_eq!(f.plot(), ClipRect::plot(&c));
    assert_eq!(f.scene(), &l.scene);
    let leaf_link = f.link(2).unwrap();
    assert_eq!(leaf_link.link_index(), 2);
    assert_eq!(leaf_link.source_index(), 2);
    assert_eq!(leaf_link.target_index(), 0);
    assert_eq!(leaf_link.source_datum().id, "branch");
    assert_eq!(leaf_link.target_datum().id, "leaf");
    assert_eq!(leaf_link.target_datum().value.to_bits(), (-0_f64).to_bits());
    assert_eq!(leaf_link.source_node().children, vec![0]);
    assert_eq!(leaf_link.target_node().depth, 2);
    assert_eq!(leaf_link.link().source_position, f.nodes()[2].position);
    assert_eq!(leaf_link.link().target_position, f.nodes()[0].position);
    assert_eq!(leaf_link.is_active(), None);
    assert_eq!(root.is_active(), None);
    assert!(std::ptr::eq(
        root.frame().source(),
        leaf_link.frame().source()
    ));
    assert!(std::ptr::eq(root.frame().nodes(), f.nodes()));
    assert!(std::ptr::eq(leaf_link.frame().links(), f.links()));
    assert!(std::ptr::eq(
        leaf_link.style(),
        &f.computed_link_styles()[2]
    ));
    assert!(f.find_node("node:root").is_none());
    assert!(f.find_node("missing").is_none());
    assert!(f.node(usize::MAX).is_err());
    assert!(f.link(usize::MAX).is_err());
}

#[test]
fn target_clones_share_immutable_data_after_original_frame_is_dropped() {
    let c = config();
    let f = TreeEventFrame::new(
        c.clone(),
        tree_layout(&c, &source(-0.), &TreeOptions::default()).unwrap(),
    )
    .unwrap();
    let leaf = f.node(0).unwrap();
    let link = f.link(2).unwrap();
    let copied = leaf.clone();
    let source_address = f.source().as_ptr();
    let styles_address = f.computed_link_styles().as_ptr();
    drop(f);
    drop(leaf);
    for _ in 0..1000 {
        let node_copy = copied.clone();
        let link_copy = link.clone();
        assert_eq!(node_copy.frame().source().as_ptr(), source_address);
        assert_eq!(
            link_copy.frame().computed_link_styles().as_ptr(),
            styles_address
        );
        assert_eq!(node_copy.datum().value.to_bits(), (-0_f64).to_bits());
        assert!(std::ptr::eq(node_copy.node(), link_copy.target_node()));
    }
}

#[test]
fn nonpainted_roles_remain_data_without_fabricated_marks_or_controls() {
    let c = config();
    let mut raw = source(-0.);
    raw[0].label.clear();
    let mut o = TreeOptions::default();
    o.style.nodes.radius = Some(0.);
    o.style.nodes.opacity = 0.;
    o.style.links.enabled = false;
    o.hierarchy.labels = false;
    let l = tree_layout(&c, &raw, &o).unwrap();
    let original = l.scene.clone();
    let f = TreeEventFrame::new(c, l).unwrap();
    let node = f.node(0).unwrap();
    assert_eq!(node.datum().label, "");
    assert_eq!(node.node().radius, 0.);
    assert_eq!(node.node().raw_value.to_bits(), (-0_f64).to_bits());
    for i in 0..f.links().len() {
        let link = f.link(i).unwrap();
        assert!(link.link().mark_id.is_none());
        assert!(link.mark().is_none());
        assert_eq!(link.source_node().position, link.link().source_position);
        assert_eq!(link.target_node().position, link.link().target_position);
    }
    assert_eq!(
        f.scene(),
        &original,
        "data capture cannot create paint or controls"
    );
    assert!(f.scene().labels.is_empty());
}

#[test]
fn current_mark_edits_do_not_rewrite_source_centers_or_resolved_link_styles() {
    let c = config();
    let mut l = tree_layout(&c, &source(-0.), &TreeOptions::default()).unwrap();
    let original_center = l.nodes[0].position;
    let original_style = l.computed_link_styles()[2];
    let node_id = l.nodes[0].mark_id.clone().unwrap();
    let link_id = l.links[2].mark_id.clone().unwrap();
    let offset_path = Geometry::Path(vec![
        glory::render::PathCommand::Move(glory::render::GraphicPoint::new(2., 3.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(9., 3.)),
        glory::render::PathCommand::Line(glory::render::GraphicPoint::new(9., 14.)),
        glory::render::PathCommand::Close,
    ]);
    l.scene
        .marks
        .iter_mut()
        .find(|m| m.id == node_id)
        .unwrap()
        .geometry = offset_path.clone();
    l.scene
        .marks
        .iter_mut()
        .find(|m| m.id == link_id)
        .unwrap()
        .stroke
        .as_mut()
        .unwrap()
        .color = Color(9, 8, 7, 255);
    let f = TreeEventFrame::new(c, l).unwrap();
    let node = f.node(0).unwrap();
    let link = f.link(2).unwrap();
    assert_eq!(node.node().position, original_center);
    assert_eq!(node.mark().unwrap().geometry, offset_path);
    assert_eq!(link.style(), &original_style);
    assert_eq!(
        link.mark().unwrap().stroke.unwrap().color,
        Color(9, 8, 7, 255)
    );
    assert_eq!(link.target_node().position, original_center);
    assert!(std::ptr::eq(
        node.mark().unwrap(),
        f.source_mark(&node_id).unwrap()
    ));
}

#[test]
fn activity_and_unpainted_function_results_are_captured_without_replaying_callbacks() {
    let c = config();
    let raw = source(-0.);
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let calls = Cell::new(0);
    let mut width = |ctx: &TreeLinkFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(ctx.link.link_index as f64 + 2.)
    };
    let mut color = |ctx: &TreeLinkFunctionContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(Color(ctx.link.link_index as u8, 35, 70, 0))
    };
    let mut on = |ctx: &TreeLinkModifierContext<'_>| Ok(ctx.thickness + 10.);
    let mut off = |_: &TreeLinkModifierContext<'_>| Ok(-0.);
    let l = tree_layout_with_activity(
        &c,
        &raw,
        &o,
        &mut TreeFactories {
            link_thickness: Some(&mut width),
            link_color: Some(&mut color),
            ..Default::default()
        },
        &TreeActivityOptions {
            focus_id: Some("leaf".into()),
            ..Default::default()
        },
        &mut TreeActivityFactories {
            active_link_thickness: Some(&mut on),
            inactive_link_thickness: Some(&mut off),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 6);
    let f = TreeEventFrame::with_activity(c, l).unwrap();
    assert_eq!(f.activity().unwrap().focus_id.as_deref(), Some("leaf"));
    assert_eq!(f.node(0).unwrap().is_active(), Some(true));
    assert_eq!(f.node(3).unwrap().is_active(), Some(false));
    assert_eq!(f.link(0).unwrap().style().thickness, 12.);
    assert_eq!(
        f.link(1).unwrap().style().thickness.to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(f.link(2).unwrap().style().thickness, 14.);
    for i in 0..3 {
        assert_eq!(f.link(i).unwrap().style().color, Color(i as u8, 35, 70, 0));
        assert_eq!(
            f.link(i).unwrap().is_active(),
            [Some(true), Some(false), Some(true)][i]
        );
        assert!(f.link(i).unwrap().mark().is_none());
    }
    assert_eq!(calls.get(), 6);
}

#[test]
fn source_zero_and_disabled_style_differences_are_not_lost_to_legacy_layout_equality() {
    let c = config();
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let build = |zero, color| {
        let mut width = |_: &TreeLinkFunctionContext<'_>| Ok(zero);
        let mut fill = |_: &TreeLinkFunctionContext<'_>| Ok(color);
        tree_layout_with_factories(
            &c,
            &source(-0.),
            &o,
            &mut TreeFactories {
                link_thickness: Some(&mut width),
                link_color: Some(&mut fill),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = build(-0., Color(1, 2, 3, 0));
    let b = build(0., Color(7, 8, 9, 0));
    assert_eq!(
        a, b,
        "legacy equality intentionally omits unpainted style caches"
    );
    let a = TreeEventFrame::new(c.clone(), a).unwrap();
    let b = TreeEventFrame::new(c.clone(), b).unwrap();
    assert_ne!(a.link(0).unwrap().style(), b.link(0).unwrap().style());
    assert_eq!(
        a.link(0).unwrap().style().thickness.to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(
        b.link(0).unwrap().style().thickness.to_bits(),
        0_f64.to_bits()
    );
    let positive =
        TreeEventFrame::new(c.clone(), tree_layout(&c, &source(0.), &o).unwrap()).unwrap();
    assert_eq!(
        a.node(0).unwrap().datum().value.to_bits(),
        (-0_f64).to_bits()
    );
    assert_eq!(
        positive.node(0).unwrap().datum().value.to_bits(),
        0_f64.to_bits()
    );
}

#[test]
fn invalid_capture_provenance_dimensions_and_activity_are_rejected() {
    let c = config();
    let raw = source(-0.);
    let l = tree_layout(&c, &raw, &TreeOptions::default()).unwrap();
    let mut wrong_config = c.clone();
    wrong_config.width += 1.;
    assert!(TreeEventFrame::new(wrong_config, l.clone()).is_err());
    let mut wrong_index = l.clone();
    wrong_index.nodes[0].source_index = 3;
    assert!(TreeEventFrame::new(c.clone(), wrong_index).is_err());
    let mut wrong_link = l;
    wrong_link.links[0].target_index = 0;
    assert!(TreeEventFrame::new(c.clone(), wrong_link).is_err());
    let mut a = tree_layout_with_activity(
        &c,
        &raw,
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    std::sync::Arc::make_mut(&mut a.activity).focus_id = Some("leaf".into());
    assert!(TreeEventFrame::with_activity(c, a).is_err());
}
