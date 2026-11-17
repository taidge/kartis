use std::cell::Cell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 480.,
        height: 320.,
        title: "Tree title".into(),
        margin: Margin {
            top: 31.,
            right: 23.,
            bottom: 47.,
            left: 37.,
        },
        ..Default::default()
    }
}
fn source(zero: f64) -> Vec<HierarchyDatum> {
    vec![
        HierarchyDatum::new("leaf", Some("root".into()), zero),
        HierarchyDatum::new("root", None, -0.),
        HierarchyDatum::new("other", Some("root".into()), 5.),
    ]
}
fn options() -> TreeOptions {
    TreeOptions {
        hierarchy: HierarchyOptions {
            include_root: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn layer(key: &str, kind: TreeSvgLayerKind) -> TreeSvgLayer {
    TreeSvgLayer::new(key, kind).unwrap()
}
fn frame() -> TreeSvgFrame {
    let config = config();
    let layout = tree_layout(&config, &source(-0.), &options()).unwrap();
    TreeSvgFrame::new(config, layout).unwrap()
}

#[test]
fn literal_order_repeats_reference_one_original_business_scene() {
    let frame = frame();
    let original = frame.layout().scene.clone();
    let plan = TreeSvgPlan::new(
        &frame,
        vec![
            layer("nodes-first", TreeSvgLayerKind::Nodes),
            layer("custom-a", TreeSvgLayerKind::Custom("same factory".into())),
            layer("links", TreeSvgLayerKind::Links),
            layer("nodes-again", TreeSvgLayerKind::Nodes),
            layer("custom-b", TreeSvgLayerKind::Custom("same factory".into())),
            layer("labels", TreeSvgLayerKind::Labels),
            layer("extras", TreeSvgLayerKind::SceneExtras),
        ],
        TreeSvgBudget::default(),
        TreeSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert_eq!(plan.layers()[0].items(), plan.layers()[3].items());
    assert_eq!(plan.layers()[1].layer.kind, plan.layers()[4].layer.kind);
    assert!(plan.layers()[1].items().is_empty());
    assert_eq!(plan.layers()[4].index, 4);
    let expected: Vec<_> = [0, 2, 3]
        .into_iter()
        .flat_map(|i| plan.layers()[i].items())
        .filter_map(|item| {
            if let DrawItem::Mark(index) = item {
                Some(*index)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(plan.mark_indices(), expected);
    let ids: Vec<_> = plan.layers()[0]
        .items()
        .iter()
        .map(|item| {
            let DrawItem::Mark(index) = item else {
                panic!("node must be a Mark")
            };
            frame.layout().scene.marks[*index].series.as_str()
        })
        .collect();
    assert_eq!(
        ids,
        ["root", "leaf", "other"],
        "computed BFS differs from original source order"
    );
    assert_eq!(plan.frame().layout().scene, original);
    assert_eq!(plan.frame().source().len(), 3);
    let empty = TreeSvgPlan::new(
        &frame,
        vec![],
        TreeSvgBudget::default(),
        TreeSvgExtrasPolicy::Omit,
    )
    .unwrap();
    assert!(empty.mark_indices().is_empty());
    assert_eq!(empty.frame().source().len(), 3);
}

#[test]
fn metadata_keys_extras_and_repeated_work_are_checked_before_execution() {
    let mut frame = frame();
    if frame.extras().is_empty() {
        let mut layout = frame.layout().clone();
        layout.scene.marks.push(Mark::new(
            "annotation",
            Geometry::Circle {
                x: 40.,
                y: 40.,
                radius: 2.,
            },
            Color(1, 2, 3, 255),
        ));
        layout.scene.draw_order.clear();
        frame = TreeSvgFrame::new(config(), layout).unwrap();
    }
    assert!(!frame.extras().is_empty());
    let node = layer("nodes", TreeSvgLayerKind::Nodes);
    assert!(
        TreeSvgPlan::new(
            &frame,
            vec![node.clone()],
            TreeSvgBudget::default(),
            TreeSvgExtrasPolicy::RequireLayer
        )
        .is_err()
    );
    assert!(
        TreeSvgPlan::new(
            &frame,
            vec![node.clone(), node.clone()],
            TreeSvgBudget::default(),
            TreeSvgExtrasPolicy::Omit
        )
        .is_err()
    );
    assert!(TreeSvgLayer::new(" ", TreeSvgLayerKind::Nodes).is_err());
    assert!(TreeSvgLayer::new("x", TreeSvgLayerKind::Custom("".into())).is_err());
    let budget = TreeSvgBudget {
        work: 3,
        ..Default::default()
    };
    assert!(
        TreeSvgPlan::new(
            &frame,
            vec![node, layer("again", TreeSvgLayerKind::Nodes)],
            budget,
            TreeSvgExtrasPolicy::Omit
        )
        .is_err()
    );
    let budget = TreeSvgBudget {
        components: 1,
        ..Default::default()
    };
    assert!(
        TreeSvgPlan::new(
            &frame,
            vec![
                layer("a", TreeSvgLayerKind::Custom("f".into())),
                layer("b", TreeSvgLayerKind::Custom("f".into()))
            ],
            budget,
            TreeSvgExtrasPolicy::Omit
        )
        .is_err()
    );
    assert!(
        TreeSvgPlan::new(
            &frame,
            vec![layer("only", TreeSvgLayerKind::Nodes)],
            TreeSvgBudget {
                source_commands: 0,
                ..Default::default()
            },
            TreeSvgExtrasPolicy::Omit
        )
        .is_err()
    );
}

#[test]
fn frame_preserves_raw_bits_activity_and_real_plot_coordinates() {
    let config = config();
    let activity = TreeActivityOptions {
        focus_id: Some("leaf".into()),
        ..Default::default()
    };
    let layout = tree_layout_with_activity(
        &config,
        &source(-0.),
        &options(),
        &mut TreeFactories::default(),
        &activity,
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    let frame = TreeSvgFrame::with_activity(config.clone(), layout).unwrap();
    assert_eq!(frame.source()[0].value.to_bits(), (-0_f64).to_bits());
    assert_eq!(frame.activity().unwrap().focus_id.as_deref(), Some("leaf"));
    assert_eq!(frame.node_by_id()["root"], 1);
    assert_eq!(frame.nodes()[1].value, 5.);
    assert_eq!(frame.inner_size(), [420., 242.]);
    assert_eq!(frame.inner_position([37., 31.]).unwrap(), [0., 0.]);
    assert!(frame.inner_position([f64::NAN, 0.]).is_err());
    let other = tree_layout_with_activity(
        &config,
        &source(0.),
        &options(),
        &mut TreeFactories::default(),
        &activity,
        &mut TreeActivityFactories::default(),
    )
    .unwrap();
    assert_ne!(
        frame,
        TreeSvgFrame::with_activity(config.clone(), other).unwrap()
    );
    let mut layout = frame.layout().clone();
    layout.nodes[0].position[0] += 1.;
    assert!(TreeSvgFrame::new(config.clone(), layout).is_err());
    let mut wrong = config.clone();
    wrong.width += 1.;
    assert!(TreeSvgFrame::new(wrong, frame.layout().clone()).is_err());
}

#[test]
fn authoritative_labels_follow_occurrence_gate_once_and_preserve_source_options() {
    let config = config();
    let raw = source(1.);
    let options = options();
    let count = Cell::new(0);
    let mut getter = |c: &TreeLabelFunctionContext<'_>| {
        count.set(count.get() + 1);
        Ok(format!("computed:{}", c.datum.id))
    };
    let no_labels = vec![layer("nodes", TreeSvgLayerKind::Nodes)];
    let hidden = tree_svg_layout_with_factories(
        &config,
        &raw,
        &options,
        &no_labels,
        &mut TreeFactories {
            label: Some(&mut getter),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(count.get(), 0);
    assert!(hidden.scene.label_series.is_empty());
    assert_eq!(hidden.options(), &options);
    tree_widget_frame(&hidden).unwrap();
    let layers = vec![
        layer("labels1", TreeSvgLayerKind::Labels),
        layer("labels2", TreeSvgLayerKind::Labels),
    ];
    let visible = tree_svg_layout_with_factories(
        &config,
        &raw,
        &options,
        &layers,
        &mut TreeFactories {
            label: Some(&mut getter),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        count.get(),
        raw.len(),
        "repeated Labels must not replay getter"
    );
    assert!(
        visible
            .scene
            .label_series
            .keys()
            .all(|index| visible.scene.labels[*index].text.starts_with("computed:"))
    );
    let mut untouched = |_: &TreeLabelFunctionContext<'_>| -> Result<String> {
        panic!("invalid layer must preflight first")
    };
    assert!(
        tree_svg_layout_with_factories(
            &config,
            &raw,
            &options,
            &[
                layer("x", TreeSvgLayerKind::Nodes),
                layer("x", TreeSvgLayerKind::Links)
            ],
            &mut TreeFactories {
                label: Some(&mut untouched),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn svg_generator_uses_actual_endpoints_without_a_canvas_context() {
    let frame = frame();
    let mut generator = frame.link_generator();
    for curve in [
        TreeLinkCurve::Straight,
        TreeLinkCurve::Bump,
        TreeLinkCurve::Step,
        TreeLinkCurve::StepBefore,
        TreeLinkCurve::StepAfter,
    ] {
        generator.set_curve(curve);
        let geometry = generator.geometry([13., 27.], [98., 121.]).unwrap();
        assert!(
            !generator
                .svg_path([13., 27.], [98., 121.])
                .unwrap()
                .is_empty()
        );
        let mut scene = config().scene().unwrap();
        scene
            .marks
            .push(Mark::new("generated", geometry, Color(0, 0, 0, 255)));
        scene.validate().unwrap();
    }
    assert!(generator.svg_path([f64::INFINITY, 0.], [0., 0.]).is_err());
}

#[test]
fn cross_family_opacity_groups_reject_instead_of_changing_compositing() {
    let frame = frame();
    let mut layout = frame.layout().clone();
    let pair = layout
        .scene
        .draw_items()
        .windows(2)
        .find_map(|items| match items {
            [DrawItem::Mark(a), DrawItem::Mark(b)]
                if layout.scene.marks[*a].id.starts_with("link:")
                    && layout.scene.marks[*b].id.starts_with("node:") =>
            {
                Some(vec![
                    layout.scene.marks[*a].id.clone(),
                    layout.scene.marks[*b].id.clone(),
                ])
            }
            _ => None,
        })
        .unwrap();
    layout.scene.opacity_groups.push(MarkOpacityGroup {
        id: "cross".into(),
        members: pair,
        opacity: 0.4,
    });
    layout.scene.validate().unwrap();
    let frame = TreeSvgFrame::new(config(), layout).unwrap();
    assert!(TreeSvgPlan::defaults(&frame).is_err());
    let custom_only = TreeSvgPlan::new(
        &frame,
        vec![layer("control", TreeSvgLayerKind::Custom("panel".into()))],
        TreeSvgBudget::default(),
        TreeSvgExtrasPolicy::Omit,
    );
    assert!(
        custom_only.is_ok(),
        "entire omitted groups have no paint to split"
    );
}

#[test]
fn frame_equality_observes_disabled_computed_link_colors() {
    let config = config();
    let mut options = options();
    options.style.links.enabled = false;
    let make = |color| {
        let mut getter = |_: &TreeLinkFunctionContext<'_>| Ok(color);
        tree_layout_with_factories(
            &config,
            &source(0.),
            &options,
            &mut TreeFactories {
                link_color: Some(&mut getter),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = make(Color(1, 2, 3, 255));
    let b = make(Color(4, 5, 6, 255));
    assert_eq!(
        a, b,
        "legacy layout equality deliberately omits computed cache"
    );
    assert_ne!(
        TreeSvgFrame::new(config.clone(), a).unwrap(),
        TreeSvgFrame::new(config, b).unwrap()
    );
}
