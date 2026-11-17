use std::cell::Cell;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 280.,
        title: "Funnel source / 漏斗".into(),
        ..Default::default()
    }
}
fn invisible_source() -> Vec<FunnelDatum> {
    let mut a = FunnelDatum::new("1", -0.);
    a.size = Some(0.);
    let mut b = FunnelDatum::new(1., 0.);
    b.size = Some(0.);
    vec![a, b]
}
fn invisible_options() -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        size: FunnelSize::DatumPixels,
        before: None,
        after: None,
        labels: None,
        border_width: 0.,
        border_opacity: 0.,
        fill_opacity: 0.,
        legend: false,
        ..Default::default()
    }
}

#[test]
fn absent_ink_retains_all_single_evaluation_style_results_and_typed_raw_source() {
    let colors = Cell::new(0);
    let formats = Cell::new(0);
    let borders = Cell::new(0);
    let labels = Cell::new(0);
    let mut color = |c: &FunnelColorContext<'_>| {
        colors.set(colors.get() + 1);
        Ok(Color(30 + c.source_index as u8, 50, 70, 255))
    };
    let mut format = |v: f64| {
        formats.set(formats.get() + 1);
        Ok(format!("{:016x}", v.to_bits()))
    };
    let mut border = |c: &FunnelPartStyleContext<'_>| {
        borders.set(borders.get() + 1);
        assert_eq!(c.fill_opacity, 0.);
        Ok(Color(10, 20, 30, 40))
    };
    let mut label = |c: &FunnelPartStyleContext<'_>| {
        labels.set(labels.get() + 1);
        assert_eq!(c.border_color, Color(10, 20, 30, 40));
        Ok(Color(90, 80, 70, 60))
    };
    let frame = FunnelSvgFrame::with_factories(
        &config(),
        &invisible_source(),
        &invisible_options(),
        &mut FunnelFactories {
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            label_color: Some(&mut label),
        },
    )
    .unwrap();
    assert!(frame.scene().marks.is_empty());
    assert!(frame.scene().labels.is_empty());
    assert_eq!(frame.source()[0].value.to_bits(), (-0f64).to_bits());
    assert_eq!(frame.part_by_id(&CategoryKey::from("1")).unwrap(), Some(0));
    assert_eq!(frame.part_by_id(&CategoryKey::from(1.)).unwrap(), Some(1));
    for i in 0..2 {
        assert!(frame.body_mark(i).unwrap().is_none());
        assert!(frame.label(i).unwrap().is_none());
        assert_eq!(frame.styles()[i].color, Color(30 + i as u8, 50, 70, 255));
        assert_eq!(frame.styles()[i].border_color, Color(10, 20, 30, 40));
        assert_eq!(frame.styles()[i].label_color, Some(Color(90, 80, 70, 60)));
    }
    let plan = FunnelSvgPlan::new(
        &frame,
        vec![
            FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts).unwrap(),
            FunnelSvgLayer::new("again", FunnelSvgLayerKind::Parts).unwrap(),
            FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels).unwrap(),
        ],
        FunnelSvgBudget::default(),
        FunnelSvgExtrasPolicy::RequireLayer,
    )
    .unwrap();
    assert!(plan.body_mark_indices().is_empty());
    assert!(plan.mark_indices().is_empty());
    for _ in 0..5 {
        let _ = frame.clone();
        let _ = frame.original_body_geometry(0).unwrap();
        let _ = frame.original_border_geometry(1).unwrap();
    }
    assert_eq!(
        [colors.get(), formats.get(), borders.get(), labels.get()],
        [2, 2, 2, 2]
    );
}

#[test]
fn unused_cache_differs_without_changing_legacy_layout_equality_or_debug() {
    let build = |paint| {
        let mut label = |_: &FunnelPartStyleContext<'_>| Ok(paint);
        funnel_layout_with_factories(
            &config(),
            &invisible_source(),
            &invisible_options(),
            &mut FunnelFactories {
                label_color: Some(&mut label),
                ..Default::default()
            },
        )
        .unwrap()
    };
    let a = build(Color(1, 2, 3, 4));
    let b = build(Color(5, 6, 7, 8));
    assert_eq!(a, b);
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    assert_ne!(a.computed_part_styles(), b.computed_part_styles());
    let expected = format!(
        "FunnelLayout {{ scene: {:?}, parts: {:?}, separators: {:?}, value_domain: {:?}, size_range: {:?}, bandwidth: {:?}, colors: {:?}, plot: {:?}, interaction: {:?} }}",
        a.scene,
        a.parts,
        a.separators,
        a.value_domain,
        a.size_range,
        a.bandwidth,
        a.colors,
        a.plot(),
        a.interaction_binding()
    );
    assert_eq!(format!("{a:?}"), expected);
    let fa = FunnelSvgFrame::from_layout(a).unwrap();
    let fb = FunnelSvgFrame::from_layout(b).unwrap();
    assert_ne!(fa, fb);
    assert_eq!(fa, fa.clone());
}

#[test]
fn composed_size_styles_and_custom_stage_keep_actual_ink_distinct_from_profile() {
    let size_calls = Cell::new(0);
    let color_calls = Cell::new(0);
    let stage_calls = Cell::new(0);
    let source = [FunnelDatum::new("a", 4.), FunnelDatum::new("b", 2.)];
    let options = FunnelLayoutOptions {
        labels: Some(FunnelLabelOptions {
            color: InheritedColor::FromContext(vec![]),
            ..Default::default()
        }),
        ..Default::default()
    };
    let mut size = |c: FunnelSizeContext<'_>| {
        size_calls.set(size_calls.get() + 1);
        Ok(if c.source_index == 0 { 80. } else { 0. })
    };
    let mut color = |_: &FunnelColorContext<'_>| {
        color_calls.set(color_calls.get() + 1);
        Ok(Color(20, 40, 60, 255))
    };
    let mut stage = |c: &FunnelStageContext<'_>| {
        stage_calls.set(stage_calls.get() + 1);
        let mut p = c.inherit(Geometry::Circle {
            x: c.part.center[0],
            y: c.part.center[1],
            radius: 10.,
        });
        p.body.fill = Some(Color(180, 70, 50, 255));
        p.retain_profile_border = false;
        Ok(p)
    };
    let frame = FunnelSvgFrame::with_size_factories_and_stage_factory(
        &config(),
        &source,
        &options,
        &mut size,
        &mut FunnelFactories {
            color: Some(&mut color),
            ..Default::default()
        },
        &mut stage,
    )
    .unwrap();
    assert_eq!(
        [size_calls.get(), color_calls.get(), stage_calls.get()],
        [2, 2, 1]
    );
    assert_eq!(frame.options(), &options);
    assert_eq!(frame.styles()[0].color, Color(20, 40, 60, 255));
    assert_eq!(frame.styles()[0].label_color, Some(Color(20, 40, 60, 255)));
    assert_eq!(frame.parts()[0].color, Color(180, 70, 50, 255));
    assert_eq!(
        frame.label(0).unwrap().unwrap().color,
        Color(180, 70, 50, 255)
    );
    assert!(matches!(
        frame.body_mark(0).unwrap().unwrap().geometry,
        Geometry::Circle { .. }
    ));
    assert!(matches!(
        frame.original_body_geometry(0).unwrap(),
        Some(Geometry::Path(_))
    ));
    assert!(frame.border_mark(0).unwrap().is_none());
    assert!(frame.body_mark(1).unwrap().is_none());
    let _ = FunnelSvgPlan::defaults(&frame).unwrap();
    let _ = frame.original_body_geometry(0).unwrap();
    assert_eq!(
        [size_calls.get(), color_calls.get(), stage_calls.get()],
        [2, 2, 1]
    );
}

#[test]
fn constructor_raw_metadata_tampering_is_rejected_but_real_scene_decoration_is_allowed() {
    let original = funnel_layout(&config(), &invisible_source(), &invisible_options()).unwrap();
    let mut changed = original.clone();
    changed.parts[0].datum.value = 0.;
    assert_eq!(
        changed.parts, original.parts,
        "legacy float equality alone cannot guard the raw source"
    );
    assert!(FunnelSvgFrame::from_layout(changed).is_err());
    let mut decorated = original;
    let index = decorated.scene.labels.len();
    decorated.scene.labels.push(Label {
        x: 25.,
        y: 25.,
        text: "actual decoration".into(),
        size: 12.,
        color: Color(3, 4, 5, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    let frame = FunnelSvgFrame::from_layout(decorated).unwrap();
    assert_eq!(frame.extras(), [DrawItem::Label(index)]);
    assert!(frame.styles().iter().all(|s| s.label_color.is_none()));
    assert!(frame.body_mark(usize::MAX).is_err());
    assert!(frame.separator_mark(usize::MAX).is_err());
    assert!(frame.mark_role(0).is_err());
}

#[test]
fn illegal_config_and_source_fail_before_any_application_role() {
    let called = Cell::new(0);
    let mut color = |_: &FunnelColorContext<'_>| {
        called.set(called.get() + 1);
        Ok(Color(1, 2, 3, 255))
    };
    let invalid = ChartConfig {
        width: f64::NAN,
        ..config()
    };
    assert!(
        FunnelSvgFrame::with_factories(
            &invalid,
            &invisible_source(),
            &invisible_options(),
            &mut FunnelFactories {
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    let duplicate = [FunnelDatum::new(0., 0.), FunnelDatum::new(-0., 0.)];
    assert!(
        FunnelSvgFrame::with_factories(
            &config(),
            &duplicate,
            &invisible_options(),
            &mut FunnelFactories {
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(called.get(), 0);
}

#[test]
fn static_validation_does_not_reject_transparent_defaults_repaired_by_real_color_function() {
    let mut config = config();
    config.theme.background = Color(0, 0, 0, 0);
    let mut source = [FunnelDatum::new("a", 2.), FunnelDatum::new("b", 1.)];
    for datum in &mut source {
        datum.color = Some(Color(0, 0, 0, 0));
    }
    let options = FunnelLayoutOptions {
        labels: None,
        border_color: InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        },
        ..Default::default()
    };
    assert!(
        FunnelSvgFrame::new(&config, &source, &options).is_err(),
        "default transparent paint genuinely cannot resolve contrast"
    );
    funnel_svg_validate_inputs(&config, &source, &options).unwrap();
    let calls = Cell::new(0);
    let mut color = |_: &FunnelColorContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(Color(20, 30, 40, 255))
    };
    let frame = FunnelSvgFrame::with_factories(
        &config,
        &source,
        &options,
        &mut FunnelFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 2);
    assert_eq!(frame.parts()[0].color, Color(20, 30, 40, 255));
}

fn paint_bits(paint: &FillPaint) -> Vec<u64> {
    match paint {
        FillPaint::Linear { start, end, stops } => start
            .iter()
            .chain(end)
            .copied()
            .chain(stops.iter().map(|s| s.offset))
            .map(f64::to_bits)
            .collect(),
        FillPaint::Radial {
            center,
            radius,
            stops,
        } => center
            .iter()
            .copied()
            .chain([*radius])
            .chain(stops.iter().map(|s| s.offset))
            .map(f64::to_bits)
            .collect(),
        FillPaint::Pattern(p) => {
            let size = match p.shape {
                PatternShape::Dots { diameter, .. } => diameter,
                PatternShape::Squares { size, .. } => size,
                PatternShape::Lines { width } => width,
            };
            [size, p.spacing, p.rotation, p.offset[0], p.offset[1]]
                .map(f64::to_bits)
                .to_vec()
        }
    }
}
fn paint_case(profile: usize, zero: f64) -> FillPaint {
    let stops = vec![
        GradientStop::new(
            if profile == 4 || profile == 7 {
                zero
            } else {
                0.
            },
            Color(10, 20, 30, 255),
        ),
        GradientStop::new(1., Color(80, 90, 100, 255)),
    ];
    match profile {
        0..=4 => FillPaint::Linear {
            start: [
                if profile == 0 { zero } else { 20. },
                if profile == 1 { zero } else { 10. },
            ],
            end: [
                if profile == 2 { zero } else { 140. },
                if profile == 3 { zero } else { 90. },
            ],
            stops,
        },
        5..=7 => FillPaint::Radial {
            center: [
                if profile == 5 { zero } else { 40. },
                if profile == 6 { zero } else { 40. },
            ],
            radius: 30.,
            stops,
        },
        _ => FillPaint::Pattern(PatternFill {
            shape: match profile {
                8 => PatternShape::Dots {
                    diameter: zero,
                    stagger: true,
                },
                9 => PatternShape::Squares {
                    size: zero,
                    stagger: true,
                },
                10 => PatternShape::Lines { width: zero },
                _ => PatternShape::Dots {
                    diameter: 2.,
                    stagger: false,
                },
            },
            rotation: if profile == 11 { zero } else { 0. },
            offset: [
                if profile == 12 { zero } else { 0. },
                if profile == 13 { zero } else { 0. },
            ],
            ..Default::default()
        }),
    }
}

#[test]
fn nested_paint_source_bits_cannot_be_forged_and_legitimate_rebuilds_notify_widgets() {
    let config = config();
    let options = FunnelLayoutOptions {
        before: None,
        after: None,
        labels: None,
        border_width: 0.,
        legend: false,
        ..Default::default()
    };
    for profile in 0..14 {
        let negative = paint_case(profile, -0.);
        let positive = paint_case(profile, 0.);
        negative.validate().unwrap();
        positive.validate().unwrap();
        assert_eq!(
            negative, positive,
            "ordinary paint equality is intentionally numerical, profile {profile}"
        );
        assert_ne!(
            paint_bits(&negative),
            paint_bits(&positive),
            "authored nested bits differ, profile {profile}"
        );
        let mut datum = FunnelDatum::new("part", 1.);
        datum.paint = Some(negative.clone());
        let original_source = [datum.clone()];
        let original = funnel_layout(&config, &original_source, &options).unwrap();
        let mut forged = original.clone();
        forged.parts[0].datum.paint = Some(positive.clone());
        assert!(
            FunnelSvgFrame::from_layout(forged).is_err(),
            "capture must reject nested paint forgery, profile {profile}"
        );

        datum.paint = Some(positive.clone());
        let rebuilt_source = [datum];
        let rejected_calls = Cell::new(0);
        assert!(
            funnel_widgets(&original, &rebuilt_source, 0, |_| {
                rejected_calls.set(rejected_calls.get() + 1);
                Ok(None)
            })
            .is_err(),
            "Widget source mismatch must reject before placement, profile {profile}"
        );
        assert_eq!(rejected_calls.get(), 0);
        let rebuilt = funnel_layout(&config, &rebuilt_source, &options).unwrap();
        assert_eq!(
            original.scene, rebuilt.scene,
            "no claimed paint geometry change, profile {profile}"
        );
        let old = FunnelSvgFrame::from_layout(original.clone()).unwrap();
        let new = FunnelSvgFrame::from_layout(rebuilt.clone()).unwrap();
        assert_eq!(
            paint_bits(old.source()[0].paint.as_ref().unwrap()),
            paint_bits(&negative)
        );
        assert_eq!(
            paint_bits(new.source()[0].paint.as_ref().unwrap()),
            paint_bits(&positive)
        );
        let packet = |layout: &FunnelLayout, source: &[FunnelDatum]| {
            funnel_widgets(layout, source, 0, |c| {
                Ok(
                    (c.role == FunnelWidgetRole::Body).then_some(PlotWidgetPlacement::Rect(
                        LayoutRect {
                            x: c.position[0] - 4.,
                            y: c.position[1] - 4.,
                            width: 8.,
                            height: 8.,
                        },
                    )),
                )
            })
            .unwrap()
        };
        let a = packet(&original, &original_source);
        let b = packet(&rebuilt, &rebuilt_source);
        assert_eq!(
            a.layout, b.layout,
            "slot identity and placement stay stable"
        );
        assert_eq!(a.data.len(), 1);
        assert_ne!(
            a.data, b.data,
            "actual typed Widget data detects new paint source, profile {profile}"
        );
    }
}
