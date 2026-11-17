//! The shared compiler is verified without mounting a host or reusing its
//! render loop. Host-level mixed Widget pixels and input are separate suites.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint, PathCommand};
use kartis::layers::{LayerPlan, NamedLayer};
use kartis::widget_paint_plan::*;
use kartis::*;

fn set(ids: &[&str]) -> BTreeSet<String> {
    ids.iter().map(|id| (*id).into()).collect()
}
fn source(ids: &[&str]) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (index, id) in ids.iter().enumerate() {
        let mut mark = Mark::new(
            *id,
            Geometry::Rect {
                x: 20. + index as f64,
                y: 20.,
                width: 30.,
                height: 25.,
            },
            Color(30, 90, 180, 255),
        );
        mark.label = (*id).into();
        mark.series = "row".into();
        mark.value = index as f64;
        scene.marks.push(mark);
    }
    scene
}
fn label(text: &str) -> Label {
    Label {
        x: 20.,
        y: 30.,
        text: text.into(),
        color: Color(20, 30, 40, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    }
}
fn slots(ids: &[&str]) -> PlotWidgetsLayout {
    PlotWidgetsLayout {
        plot: None,
        slots: ids
            .iter()
            .map(|id| PlotWidgetSlot {
                mark_id: (*id).into(),
                revision: 7,
                placement: PlotWidgetPlacement::Rect(LayoutRect {
                    x: 20.,
                    y: 20.,
                    width: 30.,
                    height: 25.,
                }),
            })
            .collect(),
    }
}
fn signature(nodes: &[WidgetPaintNode]) -> Vec<String> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            WidgetPaintNode::Widget(id) => result.push(format!("W:{id}")),
            WidgetPaintNode::Paint(run) => {
                for item in run.scene().paint_items() {
                    result.push(match item {
                        PaintItem::Mark(mark) => format!("M:{}", mark.id),
                        PaintItem::Label(label) => format!("L:{}", label.text),
                    });
                }
            }
            WidgetPaintNode::Group { id, children, .. } => {
                result.push(format!("G:{id}"));
                result.extend(signature(children));
                result.push("/G".into());
            }
        }
    }
    result
}
fn runs(nodes: &[WidgetPaintNode]) -> Vec<&WidgetPaintRun> {
    let mut result = Vec::new();
    for node in nodes {
        match node {
            WidgetPaintNode::Paint(run) => result.push(run),
            WidgetPaintNode::Group { children, .. } => result.extend(runs(children)),
            WidgetPaintNode::Widget(_) => {}
        }
    }
    result
}

#[test]
fn actual_layer_plan_order_drives_overlay_and_replacement_without_changing_source() {
    let mut original = source(&["a", "b", "c"]);
    original.labels = vec![label("title"), label("axis")];
    let authored = LayerPlan::from_layers(vec![
        NamedLayer::new("first", vec![DrawItem::Mark(1), DrawItem::Label(1)])
            .unwrap()
            .into(),
        NamedLayer::new("middle", vec![DrawItem::Mark(0), DrawItem::Mark(2)])
            .unwrap()
            .into(),
        NamedLayer::new("last", vec![DrawItem::Label(0)])
            .unwrap()
            .into(),
    ])
    .unwrap()
    .compose(&original)
    .unwrap();
    let measured = slots(&["a", "b"]); // Deliberately the opposite of source order.
    let before = authored.clone();
    let old_slots = measured.clone();
    let plan =
        widget_paint_plan(&authored, &measured, &set(&["b"]), &MarkBlends::default()).unwrap();
    assert_eq!(
        signature(plan.nodes()),
        ["W:b", "L:axis", "M:a", "W:a", "M:c", "L:title"]
    );
    assert_eq!(plan.widget_count(), 2);
    assert_eq!(plan.run_count(), 2);
    assert_eq!(plan.node_count(), 4);
    assert_eq!(
        (plan.width(), plan.height(), plan.background()),
        (240., 180., authored.background)
    );
    let batches = runs(plan.nodes());
    assert_eq!(
        batches[0].source_items(),
        [DrawItem::Label(1), DrawItem::Mark(0)]
    );
    assert_eq!(
        batches[1].source_items(),
        [DrawItem::Mark(2), DrawItem::Label(0)]
    );
    let svg = kartis::render::to_svg(batches[0].scene()).unwrap();
    assert!(svg.find(">axis</text>").unwrap() < svg.find("data-id=\"a\"").unwrap());
    assert!(!svg.contains("data-id=\"b\""));
    assert_eq!(authored, before);
    assert_eq!(measured, old_slots);
    assert!(original.draw_order.is_empty());
}

#[test]
fn whole_groups_contain_widgets_and_source_ink_with_alpha_applied_only_at_parent() {
    for opacity in [0., 0.5, 1.] {
        let mut scene = source(&["a", "b", "c", "d"]);
        scene.marks[0].opacity = 0.75;
        scene.labels = vec![label("axis-before"), label("title-after")];
        scene.draw_order = vec![
            DrawItem::Label(0),
            DrawItem::Mark(0),
            DrawItem::Mark(1),
            DrawItem::Mark(2),
            DrawItem::Mark(3),
            DrawItem::Label(1),
        ];
        scene.opacity_groups.push(MarkOpacityGroup {
            id: "source-group".into(),
            opacity,
            members: vec!["a".into(), "b".into(), "c".into()],
        });
        let plan = widget_paint_plan(
            &scene,
            &slots(&["c", "a", "b"]),
            &set(&["b"]),
            &Default::default(),
        )
        .unwrap();
        assert_eq!(
            signature(plan.nodes()),
            [
                "L:axis-before",
                "G:source-group",
                "M:a",
                "W:a",
                "W:b",
                "M:c",
                "W:c",
                "/G",
                "M:d",
                "L:title-after"
            ]
        );
        let WidgetPaintNode::Group {
            id,
            opacity: actual,
            children,
        } = &plan.nodes()[1]
        else {
            panic!("one shared group expected")
        };
        assert_eq!(id, "source-group");
        assert_eq!(*actual, opacity);
        assert!(
            children
                .iter()
                .all(|n| !matches!(n, WidgetPaintNode::Group { .. }))
        );
        assert_eq!(runs(children)[0].scene().marks[0].opacity, 0.75);
        assert_eq!(plan.run_count(), 4);
        assert_eq!(plan.widget_count(), 3);
        assert_eq!(plan.node_count(), 8);
        for run in runs(plan.nodes()) {
            assert_eq!(run.scene().background, Color(0, 0, 0, 0));
            assert!(run.scene().opacity_groups.is_empty());
            assert_eq!(run.blend(), BlendMode::Normal);
            run.scene().validate().unwrap();
        }
    }
}

#[test]
fn label_fonts_whitespace_series_and_clips_remap_only_to_their_own_run() {
    let mut scene = source(&["split", "kept"]);
    scene.labels = vec![label("zero"), label("one"), label("two\tline")];
    scene.typography.default = FontOptions {
        family: Some(vec![FontFamily::Serif]),
        weight: Some(450.),
    };
    scene.typography.legend = FontOptions {
        family: Some(vec![FontFamily::Mono]),
        weight: Some(900.),
    };
    for i in 0..3 {
        scene.typography.labels.insert(
            i,
            FontOptions {
                weight: Some(500. + i as f64),
                ..Default::default()
            },
        );
        scene.label_series.insert(i, format!("row-{i}"));
        scene.label_clips.insert(
            i,
            ClipPath {
                geometry: Geometry::Rect {
                    x: 0.,
                    y: 0.,
                    width: 100. + i as f64,
                    height: 100.,
                },
                rule: ClipRule::NonZero,
            },
        );
    }
    scene
        .typography
        .label_whitespace
        .insert(2, LabelWhitespace::Preserve);
    scene.draw_order = vec![
        DrawItem::Label(2),
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Label(1),
    ];
    let plan = widget_paint_plan(
        &scene,
        &slots(&["split"]),
        &set(&["split"]),
        &Default::default(),
    )
    .unwrap();
    let batches = runs(plan.nodes());
    assert_eq!(batches.len(), 2);
    let first = batches[0].scene();
    let second = batches[1].scene();
    assert_eq!(batches[0].source_items(), [DrawItem::Label(2)]);
    assert_eq!(first.draw_order, [DrawItem::Label(0)]);
    assert_eq!(first.typography.default, scene.typography.default);
    assert_eq!(first.typography.labels.len(), 1);
    assert_eq!(first.label_clips.len(), 1);
    assert_eq!(first.typography.labels[&0], scene.typography.labels[&2]);
    assert_eq!(
        first.typography.label_whitespace[&0],
        LabelWhitespace::Preserve
    );
    assert_eq!(first.label_clips[&0], scene.label_clips[&2]);
    assert_eq!(first.label_series[&0], "row-2");
    assert_eq!(first.resolved_label_font(0), scene.resolved_label_font(2));
    assert_eq!(
        second.draw_order,
        [DrawItem::Label(0), DrawItem::Mark(0), DrawItem::Label(1)]
    );
    assert_eq!(second.typography.labels.len(), 2);
    assert!(second.typography.label_whitespace.is_empty());
    assert_eq!(second.label_clips[&1], scene.label_clips[&1]);
    for run in batches {
        assert!(run.scene().legend.is_empty());
        assert!(run.scene().typography.legend.is_empty());
        assert!(run.scene().typography.tooltip.is_empty());
    }
}

#[test]
fn each_artistic_mark_gets_its_own_plane_and_replaced_modes_do_not_style_widgets() {
    let mut scene = source(&["a", "b", "c", "d"]);
    scene.labels.push(label("end"));
    let blends = MarkBlends::new(
        &scene,
        [
            ("a", BlendMode::Screen),
            ("b", BlendMode::Multiply),
            ("c", BlendMode::Multiply),
        ],
    )
    .unwrap();
    let plan = widget_paint_plan(&scene, &slots(&["a", "b"]), &set(&["a"]), &blends).unwrap();
    assert_eq!(
        signature(plan.nodes()),
        ["W:a", "M:b", "W:b", "M:c", "M:d", "L:end"]
    );
    let batches = runs(plan.nodes());
    assert_eq!(batches.len(), 3);
    assert_eq!(
        batches.iter().map(|r| r.blend()).collect::<Vec<_>>(),
        [BlendMode::Multiply, BlendMode::Multiply, BlendMode::Normal]
    );
    assert_eq!(batches[0].source_items(), [DrawItem::Mark(1)]);
    assert_eq!(batches[1].source_items(), [DrawItem::Mark(2)]);
    assert_eq!(
        batches[2].source_items(),
        [DrawItem::Mark(3), DrawItem::Label(0)]
    );
    assert_eq!(scene.marks.len(), 4);
    assert_eq!(scene.marks[0].id, "a");
}

#[test]
fn zero_opacity_zero_size_and_offscreen_slots_keep_the_same_plan_identity() {
    let mut scene = source(&["zero", "clipped"]);
    scene.marks[0].opacity = 0.;
    scene.marks[0].geometry = Geometry::Circle {
        x: 30.,
        y: 30.,
        radius: 0.,
    };
    let mut measured = slots(&["zero", "clipped"]);
    measured.slots[1].placement = PlotWidgetPlacement::Rect(LayoutRect {
        x: 500.,
        y: 500.,
        width: 40.,
        height: 20.,
    });
    let plan = widget_paint_plan(
        &scene,
        &measured,
        &set(&["zero", "clipped"]),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(signature(plan.nodes()), ["W:zero", "W:clipped"]);
    assert_eq!(plan.run_count(), 0);
    assert_eq!(scene.marks[0].opacity, 0.);
    assert_eq!(scene.marks[0].label, "zero");
}

#[test]
fn malformed_keys_placements_source_and_stale_blends_return_no_plan() {
    let scene = source(&["a", "b"]);
    let before = scene.clone();
    let mut duplicate = slots(&["a", "a"]);
    assert!(widget_paint_plan(&scene, &duplicate, &set(&["a"]), &Default::default()).is_err());
    duplicate.slots[1].mark_id = "missing".into();
    assert!(widget_paint_plan(&scene, &duplicate, &BTreeSet::new(), &Default::default()).is_err());
    for replacements in [set(&["b"]), set(&["unknown"])] {
        assert!(
            widget_paint_plan(&scene, &slots(&["a"]), &replacements, &Default::default()).is_err()
        );
    }
    for placement in [
        PlotWidgetPlacement::Rect(LayoutRect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 20.,
        }),
        PlotWidgetPlacement::AtDatum {
            offset: [f64::NAN, 0.],
            measured: TextSize {
                width: 30.,
                height: 20.,
            },
        },
    ] {
        let mut bad = slots(&["a"]);
        bad.slots[0].placement = placement;
        assert!(widget_paint_plan(&scene, &bad, &set(&["a"]), &Default::default()).is_err());
    }
    let mut decorative = scene.clone();
    decorative.marks[0].label.clear();
    let decorative_plan = widget_paint_plan(
        &decorative,
        &slots(&["a"]),
        &BTreeSet::new(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(decorative_plan.widget_count(), 1);
    assert!(decorative.marks[0].label.is_empty());
    let blends = MarkBlends::new(&scene, [("b", BlendMode::Multiply)]).unwrap();
    let mut removed = scene.clone();
    removed.marks.pop();
    assert!(widget_paint_plan(&removed, &slots(&["a"]), &BTreeSet::new(), &blends).is_err());
    let mut broken = scene.clone();
    broken.draw_order = vec![DrawItem::Mark(0), DrawItem::Mark(0)];
    assert!(
        widget_paint_plan(
            &broken,
            &PlotWidgetsLayout::default(),
            &BTreeSet::new(),
            &Default::default()
        )
        .is_err()
    );
    assert_eq!(scene, before);
}

fn many(count: usize) -> Scene {
    let names = (0..count).map(|i| format!("item-{i}")).collect::<Vec<_>>();
    source(&names.iter().map(String::as_str).collect::<Vec<_>>())
}

#[test]
fn run_budget_accepts_its_exact_boundary_and_batches_large_normal_sources() {
    let scene = many(MAX_WIDGET_PAINT_RUNS);
    let blends = MarkBlends::new(
        &scene,
        scene
            .marks
            .iter()
            .map(|m| (m.id.clone(), BlendMode::Multiply)),
    )
    .unwrap();
    let plan = widget_paint_plan(
        &scene,
        &PlotWidgetsLayout::default(),
        &BTreeSet::new(),
        &blends,
    )
    .unwrap();
    assert_eq!(plan.run_count(), MAX_WIDGET_PAINT_RUNS);
    let too_many = many(MAX_WIDGET_PAINT_RUNS + 1);
    let blends = MarkBlends::new(
        &too_many,
        too_many
            .marks
            .iter()
            .map(|m| (m.id.clone(), BlendMode::Multiply)),
    )
    .unwrap();
    assert!(
        widget_paint_plan(
            &too_many,
            &PlotWidgetsLayout::default(),
            &BTreeSet::new(),
            &blends
        )
        .unwrap_err()
        .0
        .contains("run budget")
    );
    let normal = widget_paint_plan(
        &too_many,
        &PlotWidgetsLayout::default(),
        &BTreeSet::new(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(normal.run_count(), 1);
    assert_eq!(normal.nodes().len(), 1);
}

#[test]
fn cumulative_node_budget_and_widget_budget_are_independent_of_run_count() {
    let mut scene = many(MAX_WIDGET_PAINT_NODES / 2);
    scene.opacity_groups = scene
        .marks
        .iter()
        .map(|m| MarkOpacityGroup {
            id: format!("group-{}", m.id),
            opacity: 0.5,
            members: vec![m.id.clone()],
        })
        .collect();
    let plan = widget_paint_plan(
        &scene,
        &PlotWidgetsLayout::default(),
        &BTreeSet::new(),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(plan.node_count(), MAX_WIDGET_PAINT_NODES);
    assert_eq!(plan.run_count(), MAX_WIDGET_PAINT_RUNS);
    assert!(
        widget_paint_plan(
            &scene,
            &slots(&["item-0"]),
            &BTreeSet::new(),
            &Default::default()
        )
        .unwrap_err()
        .0
        .contains("node budget")
    );
    let mut small = many(MAX_PLOT_WIDGET_SLOTS);
    let names = small
        .marks
        .iter()
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        widget_paint_plan(
            &small,
            &slots(&names),
            &BTreeSet::new(),
            &Default::default()
        )
        .unwrap()
        .widget_count(),
        MAX_PLOT_WIDGET_SLOTS
    );
    let mut extra = small.marks[0].clone();
    extra.id = "extra".into();
    small.marks.push(extra);
    let names = small
        .marks
        .iter()
        .map(|m| m.id.as_str())
        .collect::<Vec<_>>();
    assert!(
        widget_paint_plan(
            &small,
            &slots(&names),
            &BTreeSet::new(),
            &Default::default()
        )
        .unwrap_err()
        .0
        .contains("slot budget")
    );
}

#[test]
fn global_source_command_budget_includes_primitive_expansion_and_replaced_ink() {
    let mut scene = source(&["path", "primitive"]);
    scene.marks[0].geometry = Geometry::Path(vec![
        PathCommand::Move(GraphicPoint::new(0., 0.));
        MAX_WIDGET_PAINT_SOURCE_COMMANDS - 5
    ]);
    let plan = widget_paint_plan(
        &scene,
        &slots(&["path"]),
        &set(&["path"]),
        &Default::default(),
    )
    .unwrap();
    assert_eq!(plan.source_commands(), MAX_WIDGET_PAINT_SOURCE_COMMANDS);
    assert_eq!(plan.run_count(), 1);
    assert_eq!(runs(plan.nodes())[0].scene().marks.len(), 1);
    scene.marks[1].geometry = Geometry::Circle {
        x: 20.,
        y: 20.,
        radius: 5.,
    };
    scene.validate().unwrap(); // Ordinary Scene has a separate Path-only cap.
    assert!(
        widget_paint_plan(
            &scene,
            &slots(&["path"]),
            &set(&["path"]),
            &Default::default()
        )
        .unwrap_err()
        .0
        .contains("source command budget")
    );
}

#[test]
fn real_label_components_follow_authored_order_and_replace_no_node_ink() {
    let mut scene = source(&["node", "link"]);
    scene.labels = vec![label("annotation"), label("")];
    scene.labels[1].rotation = 37.;
    scene.labels[1].color = Color(0, 0, 0, 0);
    scene.label_series.insert(1, "row".into());
    scene.draw_order = vec![
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Label(1),
        DrawItem::Mark(0),
    ];
    let slots = SceneWidgetsLayout {
        plot: None,
        slots: vec![
            SceneWidgetSlot {
                key: "stable-label".into(),
                target: SceneWidgetTarget::Label(1),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-4., -8.],
                    measured: TextSize {
                        width: 0.,
                        height: 0.,
                    },
                },
            },
            SceneWidgetSlot {
                key: "stable-node".into(),
                target: SceneWidgetTarget::Mark("node".into()),
                revision: 0,
                placement: PlotWidgetPlacement::Rect(LayoutRect {
                    x: 10.,
                    y: 10.,
                    width: 10.,
                    height: 10.,
                }),
            },
        ],
    };
    let before = scene.clone();
    let replaced = scene_widget_paint_plan(
        &scene,
        &slots,
        &set(&["stable-label"]),
        &MarkBlends::default(),
    )
    .unwrap();
    assert_eq!(
        signature(replaced.nodes()),
        [
            "L:annotation",
            "M:link",
            "W:stable-label",
            "M:node",
            "W:stable-node"
        ]
    );
    assert_eq!(scene, before);
    assert_eq!(replaced.widget_count(), 2);
    let overlay =
        scene_widget_paint_plan(&scene, &slots, &BTreeSet::new(), &MarkBlends::default()).unwrap();
    assert_eq!(
        signature(overlay.nodes()),
        [
            "L:annotation",
            "M:link",
            "L:",
            "W:stable-label",
            "M:node",
            "W:stable-node"
        ]
    );
    // Reordering the label pool must change only the target ordinal, not the key.
    scene.labels.swap(0, 1);
    scene.label_series.clear();
    scene.label_series.insert(0, "row".into());
    scene.draw_order = vec![
        DrawItem::Label(1),
        DrawItem::Mark(1),
        DrawItem::Label(0),
        DrawItem::Mark(0),
    ];
    let mut retargeted = slots;
    retargeted.slots[0].target = SceneWidgetTarget::Label(0);
    let after = scene_widget_paint_plan(
        &scene,
        &retargeted,
        &set(&["stable-label"]),
        &MarkBlends::default(),
    )
    .unwrap();
    assert_eq!(signature(after.nodes()), signature(replaced.nodes()));
}

#[test]
fn scene_component_plan_rejects_bad_source_or_key_before_returning_any_plan() {
    let mut scene = source(&["node"]);
    scene.labels.push(label("text"));
    let good = SceneWidgetsLayout {
        plot: None,
        slots: vec![SceneWidgetSlot {
            key: "label-slot".into(),
            target: SceneWidgetTarget::Label(0),
            revision: 0,
            placement: PlotWidgetPlacement::AtDatum {
                offset: [0., 0.],
                measured: TextSize {
                    width: 0.,
                    height: 0.,
                },
            },
        }],
    };
    let compile = |layout: &SceneWidgetsLayout, replacement: &BTreeSet<String>| {
        scene_widget_paint_plan(&scene, layout, replacement, &MarkBlends::default())
    };
    assert!(compile(&good, &set(&["label-slot"])).is_ok());
    assert!(compile(&good, &set(&["node"])).is_err()); // Replacements are slot keys, not mark IDs.
    let mut invalid = good.clone();
    invalid.slots[0].target = SceneWidgetTarget::Label(1);
    assert!(compile(&invalid, &BTreeSet::new()).is_err());
    invalid = good.clone();
    invalid.slots[0].key.clear();
    assert!(compile(&invalid, &BTreeSet::new()).is_err());
    invalid = good.clone();
    let mut alias = good.slots[0].clone();
    alias.key = "second-key".into();
    invalid.slots.push(alias);
    assert!(compile(&invalid, &BTreeSet::new()).is_err());
    invalid = good.clone();
    invalid.slots[0].placement = PlotWidgetPlacement::AtDatum {
        offset: [f64::NAN, 0.],
        measured: TextSize::default(),
    };
    assert!(compile(&invalid, &BTreeSet::new()).is_err());
    invalid = good;
    invalid.slots[0].target = SceneWidgetTarget::Mark("node".into());
    assert!(compile(&invalid, &BTreeSet::new()).is_err()); // The existing positive mark-size contract stays strict.
}
