//! Composition and actual renderer checks. This does not run a browser owner.
use kartis::*;
fn layout() -> FunnelLayout {
    let config = ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 24.,
            right: 20.,
            top: 18.,
            bottom: 24.,
        },
        title: "Annotations and extras / 注解".into(),
        ..Default::default()
    };
    funnel_layout(
        &config,
        &[FunnelDatum::new("a", 12.), FunnelDatum::new("b", 4.)],
        &FunnelLayoutOptions {
            legend: false,
            ..Default::default()
        },
    )
    .unwrap()
}
fn recipe() -> FunnelAnnotationSpec {
    FunnelAnnotationSpec::new(
        FunnelAnnotationMatch::Id("a".into()),
        FunnelAnnotationKind::Rect,
        FunnelAnnotationNote::Text("Bound".into()),
        FunnelAnnotationPosition::Relative(25.),
        FunnelAnnotationPosition::Relative(-20.),
    )
}
fn extra_frame() -> FunnelSvgFrame {
    let mut source = layout();
    source.scene.labels.push(Label {
        rotation: 0.,
        x: 12.,
        y: 24.,
        text: "authored extra".into(),
        color: Color(40, 70, 90, 255),
        size: 13.,
        anchor: TextAnchor::Start,
    });
    FunnelSvgFrame::from_layout(source).unwrap()
}
fn layer(key: &str, kind: FunnelSvgLayerKind) -> FunnelSvgLayer {
    FunnelSvgLayer::new(key, kind).unwrap()
}
#[test]
fn annotations_are_a_distinct_literal_family_not_scene_extras_or_business_ink() {
    let base = extra_frame();
    let frame = base
        .with_annotations(&[recipe()], &mut Default::default(), &Default::default())
        .unwrap();
    assert_eq!(frame.extras(), base.extras());
    assert_eq!(frame.extras().len(), 1);
    let plan = FunnelSvgPlan::defaults(&frame).unwrap();
    let annotations = plan
        .layers()
        .iter()
        .find(|o| o.layer.kind == FunnelSvgLayerKind::Annotations)
        .unwrap();
    assert_eq!(annotations.items().len(), 6);
    for item in annotations.items() {
        assert!(!frame.extras().contains(item));
        match *item {
            DrawItem::Mark(i) => {
                assert!(frame.mark_role(i).unwrap().is_none());
                assert!(!plan.body_mark_indices().contains(&i));
                assert!(frame.scene().marks[i].series.is_empty());
            }
            DrawItem::Label(i) => assert!(frame.label_part_index(i).unwrap().is_none()),
        }
    }
    assert_eq!(frame.parts(), base.parts());
    assert_eq!(frame.source(), base.source());
    let without_annotations = FunnelSvgPlan::new(
        &frame,
        vec![
            layer("parts", FunnelSvgLayerKind::Parts),
            layer("extras", FunnelSvgLayerKind::SceneExtras),
        ],
        Default::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert!(annotations.items().iter().all(|item| {
        !without_annotations
            .layers()
            .iter()
            .any(|o| o.items().contains(item))
    }));
    assert!(
        FunnelSvgPlan::new(
            &frame,
            vec![layer("annotations", FunnelSvgLayerKind::Annotations)],
            Default::default(),
            FunnelSvgExtrasPolicy::RequireLayer
        )
        .is_err()
    );
}
#[test]
fn repeated_annotations_keep_literal_position_and_charge_real_work_and_resources() {
    let base = extra_frame();
    let frame = base
        .with_annotations(&[recipe()], &mut Default::default(), &Default::default())
        .unwrap();
    let layers = vec![
        layer("under", FunnelSvgLayerKind::Annotations),
        layer("body", FunnelSvgLayerKind::Parts),
        layer("extras", FunnelSvgLayerKind::SceneExtras),
        layer("over", FunnelSvgLayerKind::Annotations),
    ];
    let plan = FunnelSvgPlan::new(
        &frame,
        layers.clone(),
        Default::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert_eq!(plan.layers()[0].items(), plan.layers()[3].items());
    assert_eq!(plan.layers()[0].index, 0);
    assert_eq!(plan.layers()[3].index, 3);
    let annotation_mark = plan.layers()[0]
        .items()
        .iter()
        .find_map(|i| {
            if let DrawItem::Mark(i) = i {
                Some(*i)
            } else {
                None
            }
        })
        .unwrap();
    assert_eq!(
        plan.mark_indices()
            .iter()
            .filter(|i| **i == annotation_mark)
            .count(),
        2
    );
    assert!(
        FunnelSvgPlan::new(
            &frame,
            layers.clone(),
            FunnelSvgBudget {
                work: plan.work() - 1,
                ..Default::default()
            },
            FunnelSvgExtrasPolicy::RequireLayer
        )
        .is_err()
    );
    assert!(
        FunnelSvgPlan::new(
            &frame,
            layers.clone(),
            FunnelSvgBudget {
                layers: 3,
                ..Default::default()
            },
            FunnelSvgExtrasPolicy::RequireLayer
        )
        .is_err()
    );
    assert!(
        FunnelSvgPlan::new(
            &frame,
            layers,
            FunnelSvgBudget {
                source_commands: 1,
                ..Default::default()
            },
            FunnelSvgExtrasPolicy::RequireLayer
        )
        .is_err()
    );
}
#[test]
fn an_empty_annotation_family_preserves_legacy_scene_svg_and_canvas_commands() {
    let base = FunnelSvgFrame::from_layout(layout()).unwrap();
    let annotated = base
        .with_annotations(&[], &mut Default::default(), &Default::default())
        .unwrap();
    assert_eq!(annotated.scene().marks, base.scene().marks);
    assert_eq!(annotated.scene().labels, base.scene().labels);
    assert_eq!(
        render::to_svg(annotated.scene()).unwrap(),
        render::to_svg(base.scene()).unwrap()
    );
    assert_eq!(
        render::to_canvas_script(annotated.scene(), "canvas").unwrap(),
        render::to_canvas_script(base.scene(), "canvas").unwrap()
    );
    let plan = FunnelSvgPlan::defaults(&annotated).unwrap();
    assert!(
        plan.layers()
            .iter()
            .find(|o| o.layer.kind == FunnelSvgLayerKind::Annotations)
            .unwrap()
            .items()
            .is_empty()
    );
}
#[test]
fn actual_note_strokes_serialize_in_both_svg_and_canvas_with_a_zero_width_guard() {
    let base = FunnelSvgFrame::from_layout(layout()).unwrap();
    let frame = base
        .with_annotations(&[recipe()], &mut Default::default(), &Default::default())
        .unwrap();
    let svg = render::to_svg(frame.scene()).unwrap();
    assert!(svg.contains("stroke-linejoin=\"round\""));
    assert!(svg.contains("stroke-width=\"4\""));
    let commands = render::to_canvas_script(frame.scene(), "canvas").unwrap();
    assert!(commands.contains("ctx.lineJoin='round'"));
    assert!(commands.contains("if(4>0)"));
    assert!(commands.contains("ctx.strokeText(\"Bound\",0,0)"));
    let mut scene = frame.annotations().unwrap().scene().clone();
    scene.typography.label_strokes.get_mut(&0).unwrap().width = 0.;
    let zero = render::to_canvas_script(&scene, "canvas").unwrap();
    assert!(
        zero.contains("if(0>0)"),
        "zero native width must not become Canvas's previous/default hairline"
    );
}
#[test]
fn source_opacity_group_ids_cannot_collide_with_custom_note_group_names() {
    let mut source = layout();
    source.scene.opacity_groups.push(MarkOpacityGroup {
        id: "funnel:annotation:0:note:0:group:0".into(),
        members: vec![source.scene.marks[0].id.clone()],
        opacity: 0.8,
    });
    let frame = FunnelSvgFrame::from_layout(source).unwrap();
    let mut spec = recipe();
    spec.note = FunnelAnnotationNote::Custom("group".into());
    let mut note = |c: FunnelAnnotationNoteContext<'_>| {
        let mut s = c.scene()?;
        s.marks.push(Mark::new(
            "note",
            Geometry::Circle {
                x: c.x,
                y: c.y,
                radius: 6.,
            },
            Color(10, 70, 120, 255),
        ));
        s.opacity_groups.push(MarkOpacityGroup {
            id: "local".into(),
            members: vec!["note".into()],
            opacity: 0.4,
        });
        Ok(s)
    };
    let merged = frame
        .with_annotations(
            &[spec],
            &mut FunnelAnnotationFactories {
                matcher: None,
                note: Some(&mut note),
            },
            &Default::default(),
        )
        .unwrap();
    merged.scene().validate().unwrap();
    assert_eq!(merged.scene().opacity_groups.len(), 2);
    assert_eq!(
        merged.scene().opacity_groups[0].id,
        "funnel:annotation:0:note:0:group:0"
    );
    assert_ne!(
        merged.scene().opacity_groups[0].id,
        merged.scene().opacity_groups[1].id
    );
}
#[cfg(feature = "raster")]
mod pixels {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};

    use super::*;
    fn fonts() -> FontRegistry {
        let mut fonts = FontRegistry::new();
        fonts
            .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
            .unwrap();
        fonts
    }
    fn glyph_scene() -> Scene {
        let mut s = ChartConfig {
            width: 240.,
            height: 150.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        s.background = Color(0, 0, 0, 0);
        s.labels.push(Label {
            rotation: 0.,
            x: 35.,
            y: 105.,
            text: "HH".into(),
            color: Color(0, 0, 0, 0),
            size: 72.,
            anchor: TextAnchor::Start,
        });
        s
    }
    #[test]
    fn transparent_fill_with_real_glyph_stroke_paints_cpu_outline_pixels() {
        let fonts = fonts();
        let mut scene = glyph_scene();
        scene.typography.label_strokes.insert(
            0,
            LabelStroke {
                color: Color(220, 30, 70, 255),
                width: 5.,
            },
        );
        let image = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
        let count = image.rgba.chunks_exact(4).filter(|p| p[3] > 0).count();
        assert!(
            count > 100,
            "real glyph outline must paint nontransparent pixels"
        );
        assert!(
            image
                .rgba
                .chunks_exact(4)
                .filter(|p| p[3] == 255)
                .any(|p| p[0] == 220 && p[1] == 30 && p[2] == 70)
        );
        scene.typography.label_strokes.get_mut(&0).unwrap().width = 0.;
        let zero = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
        assert!(zero.rgba.chunks_exact(4).all(|p| p[3] == 0));
        scene.typography.label_strokes.clear();
        let none = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
        assert_eq!(zero.rgba, none.rgba);
    }
    #[test]
    fn svg_annotation_two_fill_alpha_order_has_observable_cpu_source_over() {
        let frame = FunnelSvgFrame::from_layout(layout()).unwrap();
        let mut spec = recipe();
        spec.note = FunnelAnnotationNote::Text("HH".into());
        let theme = FunnelAnnotationTheme {
            text: FunnelAnnotationTextStyle {
                color: Color(20, 70, 130, 128),
                font_size: 72.,
                outline_color: Color(0, 0, 0, 0),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = funnel_annotations(&frame, &[spec], &mut Default::default(), &theme).unwrap();
        let mut scene = glyph_scene();
        scene.labels = vec![
            result.scene().labels[0].clone(),
            result.scene().labels[1].clone(),
        ];
        for label in &mut scene.labels {
            label.x = 35.;
            label.y = 105.;
        }
        scene.typography.label_strokes = result.scene().typography.label_strokes.clone();
        let fonts = fonts();
        let twice = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
        scene.labels.remove(0);
        scene.typography.label_strokes.clear();
        let once = render_raster(&scene, &fonts, &RasterOptions::default()).unwrap();
        let mut interiors = 0;
        for (a, b) in once.rgba.chunks_exact(4).zip(twice.rgba.chunks_exact(4)) {
            if a[3] == 128 {
                interiors += 1;
                assert!(
                    (190..=194).contains(&b[3]),
                    "two real half-alpha fills source-over, not one scalar metadata echo"
                );
            }
        }
        assert!(interiors > 100);
        assert_ne!(once.rgba, twice.rgba);
    }
}
