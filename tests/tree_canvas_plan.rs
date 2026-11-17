//! Cross-platform raw Canvas source/plan admission. No fake native context.
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 280.,
        ..Default::default()
    }
}
fn data() -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum {
            id: "leaf".into(),
            label: "raw leaf".into(),
            parent: Some("root".into()),
            value: 3.,
        },
        HierarchyDatum {
            id: "root".into(),
            label: "raw root".into(),
            parent: None,
            value: 0.,
        },
        HierarchyDatum {
            id: "zero".into(),
            label: "raw zero".into(),
            parent: Some("root".into()),
            value: -0.,
        },
    ]
}
fn frame() -> TreeCanvasFrame {
    let c = config();
    let layout = tree_layout(&c, &data(), &TreeOptions::default()).unwrap();
    TreeCanvasFrame::new(c, layout).unwrap()
}

#[test]
fn literal_repetitions_preserve_topology_order_custom_occurrences_and_empty_boundaries() {
    let f = frame();
    let layers = vec![
        TreeCanvasLayer::Custom("under".into()),
        TreeCanvasLayer::Nodes,
        TreeCanvasLayer::Links,
        TreeCanvasLayer::Nodes,
        TreeCanvasLayer::Custom("under".into()),
        TreeCanvasLayer::Mesh,
    ];
    let p = TreeCanvasPlan::new(&f, layers.clone(), TreeCanvasBudget::default()).unwrap();
    assert_eq!(
        p.layers()
            .iter()
            .map(|l| l.layer.clone())
            .collect::<Vec<_>>(),
        layers
    );
    assert_eq!(
        p.layers()[1].roles(),
        [
            TreeCanvasRole::Node(1),
            TreeCanvasRole::Node(0),
            TreeCanvasRole::Node(2)
        ]
    );
    assert_eq!(p.layers()[3].roles(), p.layers()[1].roles());
    assert_eq!(p.layers()[4].layer_index, 4);
    let c = config();
    let empty = TreeCanvasFrame::new(
        c.clone(),
        tree_layout(&c, &[], &TreeOptions::default()).unwrap(),
    )
    .unwrap();
    let p = TreeCanvasPlan::defaults(&empty).unwrap();
    assert_eq!(p.layers().len(), 4);
    assert_eq!(p.layers()[0].layer, TreeCanvasLayer::Links);
    assert!(p.layers()[0].roles().is_empty());
    assert_eq!(p.layers()[3].roles(), [TreeCanvasRole::Mesh]);
}

#[test]
fn frame_keeps_complete_original_source_theme_and_zero_bits_owned() {
    let mut raw = data();
    let c = config();
    let layout = tree_layout(&c, &raw, &TreeOptions::default()).unwrap();
    let f = TreeCanvasFrame::new(c.clone(), layout).unwrap();
    raw[2].value = 42.;
    assert_eq!(f.source()[2].value.to_bits(), (-0f64).to_bits());
    assert_eq!(f.node_by_id()["leaf"], 0);
    assert_eq!(f.node_context(1).unwrap().datum.value, 0.);
    assert_eq!(f.node_context(1).unwrap().node.value, 3.);
    assert_eq!(f.theme(), &c.theme);
    assert_eq!(
        f.label_qualification(),
        TreeCanvasLabelQualification::CurrentScene
    );
    let mut plus = data();
    plus[2].value = 0.;
    let g = TreeCanvasFrame::new(
        c.clone(),
        tree_layout(&c, &plus, &TreeOptions::default()).unwrap(),
    )
    .unwrap();
    assert_ne!(f, g);
    assert_eq!(f, f.clone());
}

#[test]
fn frame_compares_hidden_computed_link_style_bits_despite_legacy_layout_equality() {
    let c = config();
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let build = |width: f64| {
        let mut thickness = move |_: &TreeLinkFunctionContext<'_>| Ok(width);
        tree_layout_with_factories(
            &c,
            &data(),
            &o,
            &mut TreeFactories {
                link_thickness: Some(&mut thickness),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = build(-0.);
    let b = build(0.);
    assert_eq!(a, b);
    assert!(a.links.iter().all(|l| l.mark_id.is_none()));
    assert_ne!(a.computed_link_styles(), b.computed_link_styles());
    let a = TreeCanvasFrame::new(c.clone(), a).unwrap();
    let b = TreeCanvasFrame::new(c, b).unwrap();
    assert_ne!(a, b);
    assert_eq!(
        TreeCanvasPlan::defaults(&a).unwrap().layers()[0]
            .roles()
            .len(),
        2,
        "disabled paint still has complete computed edge metadata"
    );
}

#[test]
fn no_widget_slot_limit_is_imposed_but_repeated_source_work_is_checked() {
    let c = config();
    let mut raw = vec![HierarchyDatum {
        id: "r".into(),
        label: "r".into(),
        parent: None,
        value: 0.,
    }];
    for i in 0..129 {
        raw.push(HierarchyDatum {
            id: format!("n{i}"),
            label: format!("n{i}"),
            parent: Some("r".into()),
            value: 1.,
        });
    }
    let f = TreeCanvasFrame::new(
        c.clone(),
        tree_layout(&c, &raw, &TreeOptions::default()).unwrap(),
    )
    .unwrap();
    let p = TreeCanvasPlan::new(
        &f,
        vec![TreeCanvasLayer::Nodes],
        TreeCanvasBudget::default(),
    )
    .unwrap();
    assert_eq!(p.work(), 130);
    assert!(
        TreeCanvasPlan::new(
            &f,
            vec![TreeCanvasLayer::Nodes],
            TreeCanvasBudget {
                work: 129,
                ..Default::default()
            }
        )
        .is_err()
    );
    let cost = f
        .layout()
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("node:"))
        .map(|m| m.geometry.commands().len())
        .sum::<usize>();
    assert!(
        TreeCanvasPlan::new(
            &f,
            vec![TreeCanvasLayer::Nodes, TreeCanvasLayer::Nodes],
            TreeCanvasBudget {
                source_commands: cost,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn public_layout_and_domain_conflicts_reject_before_native_execution_exists() {
    let c = config();
    let mut layout = tree_layout(&c, &data(), &TreeOptions::default()).unwrap();
    layout.nodes[0].position[0] += 1.;
    assert!(TreeCanvasFrame::new(c.clone(), layout).is_err());
    let layout = tree_layout(&c, &data(), &TreeOptions::default()).unwrap();
    let mut other = c.clone();
    other.width += 1.;
    assert!(TreeCanvasFrame::new(other, layout).is_err());
    let mut activity = tree_layout_with_activity(
        &c,
        &data(),
        &TreeOptions::default(),
        &mut TreeFactories::default(),
        &TreeActivityOptions::default(),
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    std::sync::Arc::make_mut(&mut activity.activity).nodes[0] = Some(true);
    assert!(TreeCanvasFrame::with_activity(c, activity).is_err());
}

#[test]
fn arbitrary_endpoint_generator_is_configurable_and_never_guesses_current_bounds() {
    let mut g = TreeCanvasLinkGenerator::new(TreeLinkCurve::Bump, TreeOrientation::LeftToRight);
    assert_eq!(
        g.svg_path([2., 3.], [10., 15.]).unwrap(),
        "M2 3C6 3 6 15 10 15"
    );
    assert_eq!(
        g.generate([2., 3.], [10., 15.]).unwrap(),
        Some("M2 3C6 3 6 15 10 15".into())
    );
    g.set_curve(TreeLinkCurve::StepBefore);
    g.set_orientation(TreeOrientation::TopToBottom);
    assert_eq!(
        g.svg_path([2., 3.], [10., 15.]).unwrap(),
        "M2 3L2 3L10 3L10 15"
    );
    assert!(g.geometry([f64::NAN, 0.], [0., 0.]).is_err());
    assert!(g.geometry([1e7, 0.], [0., 0.]).is_err());
    g.set_orientation(TreeOrientation::Radial);
    assert!(g.geometry([0., 0.], [1., 1.]).is_err());
    assert!(g.polar_geometry([2., 0.], [3., 1.], [10., 10.]).is_ok());
}

#[test]
fn backing_budget_includes_pending_seal_and_reserved_accepted_pixels() {
    let f = frame();
    let budget = TreeCanvasBudget::default();
    let mut o = TreeCanvasOptions::default();
    assert_eq!(o.backing_size(&f, budget, 2).unwrap(), [400, 280]);
    o.reserved_pixels = budget.live_pixels - 400 * 280;
    assert!(o.backing_size(&f, budget, 2).is_err());
    o.reserved_pixels = 0;
    o.pixel_ratio = f64::NAN;
    assert!(o.backing_size(&f, budget, 2).is_err());
    assert!(TreeCanvasPlan::new(&f, vec![TreeCanvasLayer::Custom(String::new())], budget).is_err());
}

#[test]
fn authoritative_all_computed_pool_is_not_invented_or_replayed_from_scene() {
    let mut c = config();
    c.title = "raw label computation".into();
    let mut o = TreeOptions::default();
    o.hierarchy.labels = false;
    let mut calls = 0;
    let mut label = |ctx: &TreeLabelFunctionContext<'_>| {
        calls += 1;
        Ok(format!("computed-{}", ctx.datum.id))
    };
    let layout = tree_canvas_layout_with_factories(
        &c,
        &data(),
        &o,
        &mut TreeFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(layout.scene.labels.is_empty());
    assert_eq!(calls, 3);
    let f = TreeCanvasFrame::from_all_computed(c.clone(), layout).unwrap();
    assert_eq!(f.labels().len(), 3);
    assert_eq!(
        f.label_qualification(),
        TreeCanvasLabelQualification::AllComputed
    );
    assert!(
        TreeCanvasPlan::defaults(&f).unwrap().layers()[2]
            .roles()
            .is_empty()
    );
    let label = f.label_context(0).unwrap();
    assert_eq!(label.label.text, "computed-root");
    assert_eq!(label.source_index, Some(1));
    assert!(!label.eligible);
    assert_eq!(label.scene_index, None);
    assert_eq!(calls, 3, "Frame/plan access never replays a formatter");
    let old = tree_layout(&c, &data(), &o).unwrap();
    assert!(TreeCanvasFrame::from_all_computed(c, old).is_err());
}

#[test]
fn all_computed_original_label_and_legally_edited_scene_label_are_distinct_views() {
    let c = config();
    let mut layout = tree_canvas_layout(&c, &data(), &TreeOptions::default()).unwrap();
    let original = layout.computed_canvas_labels().unwrap()[0]
        .label
        .text
        .clone();
    layout.scene.labels[0].text = "current Scene paint text".into();
    let raw = TreeCanvasFrame::from_all_computed(c.clone(), layout.clone()).unwrap();
    let ctx = raw.label_context(0).unwrap();
    assert_eq!(ctx.label.text, original);
    assert_eq!(ctx.scene_label.unwrap().text, "current Scene paint text");
    assert_eq!(ctx.scene_index, Some(0));
    let current = TreeCanvasFrame::new(c, layout).unwrap();
    assert_eq!(
        current.label_context(0).unwrap().label.text,
        "current Scene paint text"
    );
}

#[test]
fn raw_link_hidden_eligibility_uses_both_original_owners_including_disabled_marks() {
    let c = config();
    let mut o = TreeOptions::default();
    o.style.links.enabled = false;
    let f = TreeCanvasFrame::new(c.clone(), tree_layout(&c, &data(), &o).unwrap()).unwrap();
    let p = TreeCanvasPlan::defaults(&f).unwrap();
    let count = |ids: &[&str]| {
        let hidden = ids.iter().map(|s| s.to_string()).collect();
        p.layers()[0]
            .roles()
            .iter()
            .filter(|&&role| !f.role_is_hidden(role, &hidden).unwrap())
            .count()
    };
    assert_eq!(
        count(&[]),
        2,
        "disabled Marks do not remove raw computed callback roles"
    );
    assert_eq!(count(&["leaf"]), 1);
    assert_eq!(count(&["zero"]), 1);
    assert_eq!(count(&["root"]), 0);
    assert!(
        f.role_is_hidden(TreeCanvasRole::Node(usize::MAX), &Default::default())
            .is_err()
    );
    assert!(
        f.role_is_hidden(TreeCanvasRole::Link(usize::MAX), &Default::default())
            .is_err()
    );
    assert!(
        f.role_is_hidden(TreeCanvasRole::Label(usize::MAX), &Default::default())
            .is_err()
    );
}
