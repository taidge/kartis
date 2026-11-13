use kartis::layers::*;
use kartis::*;
use std::cell::Cell;
use std::collections::BTreeSet;

fn label(text: &str) -> Label {
    Label {
        rotation: 0.,
        x: 30.,
        y: 40.,
        text: text.into(),
        color: Color(1, 2, 3, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
    }
}
fn mark(id: &str) -> Mark {
    let mut mark = Mark::new(
        id,
        Geometry::Rect {
            x: 10.,
            y: 10.,
            width: 20.,
            height: 20.,
        },
        Color(11, 22, 33, 255),
    );
    mark.series = "data".into();
    mark
}
fn source() -> Scene {
    let mut scene = ChartConfig::default().scene().unwrap();
    scene.marks.push(mark("base"));
    scene.labels.push(label("base-label"));
    scene.label_series.insert(0, "data".into());
    scene
}
fn order(scene: &Scene) -> Vec<String> {
    scene
        .paint_items()
        .map(|item| match item {
            PaintItem::Mark(mark) => format!("mark:{}", mark.id),
            PaintItem::Label(label) => format!("text:{}", label.text),
        })
        .collect()
}

#[test]
fn named_layers_and_custom_geometry_text_interleave_in_actual_svg_order() {
    let base = source();
    let custom = CustomLayer::new(
        "callout",
        vec![
            LayerContent::Label {
                label: label("before-custom"),
                series: None,
            },
            LayerContent::Mark(mark("custom")),
            LayerContent::Label {
                label: label("after-custom"),
                series: Some("data".into()),
            },
        ],
    )
    .unwrap();
    let plan = LayerPlan::from_layers(vec![
        NamedLayer::new("under-text", vec![DrawItem::Label(0)])
            .unwrap()
            .into(),
        custom.into(),
        NamedLayer::new("data", vec![DrawItem::Mark(0)])
            .unwrap()
            .into(),
    ])
    .unwrap();
    let scene = plan.compose(&base).unwrap();
    assert_eq!(
        order(&scene),
        [
            "text:base-label",
            "text:before-custom",
            "mark:custom",
            "text:after-custom",
            "mark:base"
        ]
    );
    assert_eq!(scene.label_series.get(&2).map(String::as_str), Some("data"));
    let svg = kartis::render::to_svg(&scene).unwrap();
    let positions = [
        svg.find(">base-label</text>").unwrap(),
        svg.find(">before-custom</text>").unwrap(),
        svg.find("data-id=\"custom\"").unwrap(),
        svg.find(">after-custom</text>").unwrap(),
        svg.find("data-id=\"base\"").unwrap(),
    ];
    assert!(positions.windows(2).all(|p| p[0] < p[1]));
    assert!(base.draw_order.is_empty());
}
#[test]
fn custom_layer_computation_is_explicit_once_and_reads_immutable_context() {
    let base = source();
    let before = base.clone();
    let calls = Cell::new(0);
    let custom = CustomLayer::compute(&base, "computed", |scene| {
        calls.set(calls.get() + 1);
        let mut text = label("computed");
        text.x = scene.width / 2.;
        Ok(vec![LayerContent::Label {
            label: text,
            series: None,
        }])
    })
    .unwrap();
    assert_eq!(calls.get(), 1);
    let mut plan = LayerPlan::for_scene(&base).unwrap();
    plan.insert_before("chart", custom).unwrap();
    let first = plan.compose(&base).unwrap();
    let second = plan.compose(&base).unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(first, second);
    assert_eq!(base, before);
    assert_eq!(order(&first)[0], "text:computed");
    let failed = CustomLayer::compute(&base, "failure", |_| {
        Err(ChartError("application error".into()))
    });
    assert!(failed.is_err());
    assert_eq!(base, before);
    assert!(
        CustomLayer::compute(&base, " ", |_| {
            calls.set(calls.get() + 1);
            Ok(vec![])
        })
        .is_err()
    );
    let mut invalid = base.clone();
    invalid.width = f64::NAN;
    assert!(
        CustomLayer::compute(&invalid, "invalid-base", |_| {
            calls.set(calls.get() + 1);
            Ok(vec![])
        })
        .is_err()
    );
    assert_eq!(calls.get(), 1);
}
#[test]
fn plan_insertions_are_named_checked_and_preserve_existing_explicit_order() {
    let mut base = source();
    base.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    let mut plan = LayerPlan::for_scene(&base).unwrap();
    plan.insert_before(
        "chart",
        CustomLayer::new(
            "before",
            vec![LayerContent::Label {
                label: label("before"),
                series: None,
            }],
        )
        .unwrap(),
    )
    .unwrap();
    plan.insert_after(
        "chart",
        CustomLayer::new(
            "after",
            vec![LayerContent::Label {
                label: label("after"),
                series: None,
            }],
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        order(&plan.compose(&base).unwrap()),
        ["text:before", "text:base-label", "mark:base", "text:after"]
    );
    let before = plan.clone();
    assert!(
        plan.insert_before("missing", NamedLayer::new("empty", vec![]).unwrap())
            .is_err()
    );
    assert_eq!(plan, before);
    assert!(
        plan.push(NamedLayer::new("chart", vec![]).unwrap())
            .is_err()
    );
    assert_eq!(plan, before);
}
#[test]
fn missing_duplicate_and_out_of_frame_references_reject_before_publication() {
    let mut base = source();
    let before = base.clone();
    for items in [
        vec![DrawItem::Mark(0)],
        vec![DrawItem::Mark(0), DrawItem::Mark(0)],
        vec![DrawItem::Mark(1), DrawItem::Label(0)],
    ] {
        let plan =
            LayerPlan::from_layers(vec![NamedLayer::new("bad", items).unwrap().into()]).unwrap();
        assert!(plan.apply(&mut base).is_err());
        assert_eq!(base, before);
    }
    let plan = LayerPlan::from_layers(vec![
        NamedLayer::new("future", vec![DrawItem::Mark(1), DrawItem::Label(0)])
            .unwrap()
            .into(),
        CustomLayer::new("new", vec![LayerContent::Mark(mark("new"))])
            .unwrap()
            .into(),
    ])
    .unwrap();
    assert!(plan.apply(&mut base).is_err());
    assert_eq!(base, before);
}
#[test]
fn cross_layer_identity_geometry_and_name_failures_are_atomic() {
    let mut base = source();
    let before = base.clone();
    let mut duplicate = LayerPlan::for_scene(&base).unwrap();
    duplicate
        .push(CustomLayer::new("duplicate", vec![LayerContent::Mark(mark("base"))]).unwrap())
        .unwrap();
    assert!(duplicate.apply(&mut base).is_err());
    assert_eq!(base, before);
    let mut invalid = mark("invalid");
    invalid.opacity = f64::NAN;
    assert!(CustomLayer::new("bad", vec![LayerContent::Mark(invalid.clone())]).is_err());
    let mut bypass = LayerPlan::for_scene(&base).unwrap();
    bypass.layers.push(ChartLayer::Custom(CustomLayer {
        name: "bad".into(),
        content: vec![LayerContent::Mark(invalid)],
    }));
    assert!(bypass.apply(&mut base).is_err());
    assert_eq!(base, before);
    assert!(
        LayerPlan::from_layers(vec![
            NamedLayer::new("same", vec![]).unwrap().into(),
            NamedLayer::new("same", vec![]).unwrap().into()
        ])
        .is_err()
    );
    assert!(NamedLayer::new(" ", vec![]).is_err());
    assert!(
        CustomLayer::new(
            "bad-owner",
            vec![LayerContent::Label {
                label: label("text"),
                series: Some(String::new())
            }]
        )
        .is_err()
    );
}
#[test]
fn custom_layer_budgets_validate_before_scene_replacement() {
    let mut base = source();
    let before = base.clone();
    let excessive = CustomLayer {
        name: "excessive".into(),
        content: vec![
            LayerContent::Label {
                label: label("text"),
                series: None
            };
            MAX_MARKS + 1
        ],
    };
    assert!(CustomLayer::new(excessive.name.clone(), excessive.content.clone()).is_err());
    let mut plan = LayerPlan::for_scene(&base).unwrap();
    plan.layers.push(excessive.into());
    assert!(plan.apply(&mut base).is_err());
    assert_eq!(base, before);
    let mut plan = LayerPlan::default();
    for i in 0..MAX_CHART_LAYERS {
        plan.push(NamedLayer::new(format!("n{i}"), vec![]).unwrap())
            .unwrap();
    }
    assert!(
        plan.push(NamedLayer::new("one-more", vec![]).unwrap())
            .is_err()
    );
}
#[test]
fn series_filtering_remaps_custom_order_and_removes_only_owned_labels() {
    let base = source();
    let mut plan = LayerPlan::for_scene(&base).unwrap();
    plan.insert_before(
        "chart",
        CustomLayer::new(
            "labels",
            vec![
                LayerContent::Label {
                    label: label("owned"),
                    series: Some("data".into()),
                },
                LayerContent::Label {
                    label: label("shared"),
                    series: None,
                },
            ],
        )
        .unwrap(),
    )
    .unwrap();
    let scene = plan.compose(&base).unwrap();
    let filtered = scene.without_series(&BTreeSet::from(["data".into()]));
    filtered.validate().unwrap();
    assert_eq!(order(&filtered), ["text:shared"]);
    assert!(filtered.label_series.is_empty());
}
#[test]
fn radial_datum_labels_follow_series_while_radar_axes_stay_shared() {
    let config = ChartConfig::default();
    let data = vec![RadialDatum::new("a", 4.), RadialDatum::new("b", 3.)];
    for scene in [
        pie(&config, &data, &PieOptions::default()).unwrap(),
        funnel(&config, &data, &FunnelOptions::default()).unwrap(),
    ] {
        assert_eq!(scene.label_series.len(), scene.labels.len());
        let filtered = scene.without_series(&BTreeSet::from(["a".into()]));
        assert!(filtered.labels.iter().all(|l| l.text != "a"));
        assert!(filtered.labels.iter().any(|l| l.text == "b"));
    }
    let radar = radar(
        &config,
        &["x".into(), "y".into(), "z".into()],
        &[RadarSeries::new("a", vec![1., 2., 3.])],
        &RadarOptions::default(),
    )
    .unwrap();
    assert!(radar.label_series.is_empty());
    assert_eq!(
        radar.without_series(&BTreeSet::from(["a".into()])).labels,
        radar.labels
    );
    let hierarchy = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("a", Some("root".into()), 4.),
        HierarchyDatum::new("b", Some("root".into()), 3.),
    ];
    for scene in [
        treemap(&config, &hierarchy, &HierarchyOptions::default()).unwrap(),
        sunburst(&config, &hierarchy, &HierarchyOptions::default()).unwrap(),
        icicle(&config, &hierarchy, &HierarchyOptions::default()).unwrap(),
        tree(&config, &hierarchy, &HierarchyOptions::default()).unwrap(),
        tree_with_options(&config, &hierarchy, &TreeOptions::default()).unwrap(),
        circle_packing(&config, &hierarchy, &HierarchyOptions::default()).unwrap(),
    ] {
        assert!(!scene.labels.is_empty());
        assert_eq!(scene.label_series.len(), scene.labels.len());
        scene
            .without_series(&BTreeSet::from(["a".into()]))
            .validate()
            .unwrap();
    }
    let nodes = [FlowNode::new("a"), FlowNode::new("b")];
    let flow = sankey(
        &config,
        &nodes,
        &[FlowLink {
            source: "a".into(),
            target: "b".into(),
            value: 1.,
        }],
        &SankeyOptions::default(),
    )
    .unwrap();
    assert_eq!(flow.label_series.len(), 2);
}
