use kartis::surface::*;
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 800.,
        height: 420.,
        margin: Margin {
            left: 64.,
            right: 24.,
            top: 32.,
            bottom: 32.,
        },
        ..Default::default()
    }
}
fn data() -> Vec<BumpSeries> {
    vec![
        BumpSeries::new(
            "Alpha",
            vec![BumpDatum::new("Q1", 1.), BumpDatum::new("Q2", 2.)],
        ),
        BumpSeries::new(
            "Beta",
            vec![BumpDatum::new("Q1", 2.), BumpDatum::new("Q2", 1.)],
        ),
    ]
}
fn options() -> BumpLayoutOptions {
    BumpLayoutOptions {
        axes: BumpAxes::none(),
        labels: BumpEndpointLabels {
            start: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn side(padding: Option<f64>, color: Option<InheritedColor>) -> Option<BumpEndpointLabelStyle> {
    Some(BumpEndpointLabelStyle { padding, color })
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-12, "{a} != {b}");
}

#[test]
fn original_nivo_hook_fixed_color_padding_coordinates_and_gaps() {
    let mut count = 0;
    for line in include_str!("fixtures/bump-endpoint/values.txt")
        .lines()
        .filter(|s| !s.starts_with('#'))
    {
        let f = line.split('|').collect::<Vec<_>>();
        let mut d = data();
        d.truncate(1);
        let mut o = options();
        o.x_scale.reverse = f[0] == "1";
        o.rank_scale.reverse = o.x_scale.reverse;
        o.style.opacity = f[1].parse().unwrap();
        if f[2] == "1" {
            d[0].data[0].rank = None;
        }
        // Preserve the two-rank domain when the first source endpoint is absent.
        if f[2] == "1" {
            d.push(BumpSeries::new("domain", vec![BumpDatum::new("Q1", 1.)]));
            o.hidden.insert("domain".into());
        }
        o.labels.start_style = side(Some(8.), Some(InheritedColor::Fixed(Color(255, 0, 0, 255))));
        o.labels.end_style = side(
            Some(24.),
            Some(InheritedColor::Fixed(Color(0, 0, 255, 255))),
        );
        let l = bump_layout(&frame(), &d, &o).unwrap();
        let index = usize::from(f[3] == "end" && f[2] == "0");
        let label = &l.scene.labels[index];
        close(label.x, f[4].parse().unwrap());
        close(label.y, f[5].parse().unwrap());
        assert_eq!(
            label.color,
            if f[6] == "red" {
                Color(255, 0, 0, (255. * o.style.opacity).round() as u8)
            } else {
                Color(0, 0, 255, (255. * o.style.opacity).round() as u8)
            }
        );
        assert_eq!(
            label.anchor,
            if f[7] == "end" {
                TextAnchor::End
            } else {
                TextAnchor::Start
            }
        );
        assert_eq!(label.text, f[9]);
        assert_eq!(l.scene.label_series[&index], f[8]);
        count += 1;
    }
    assert_eq!(count, 18);
}

#[test]
fn independent_fields_inherit_shared_values_and_preserve_every_nonlabel_field() {
    for reverse in [false, true] {
        for round in [false, true] {
            let mut c = frame();
            c.width += 0.75;
            let mut o = options();
            o.x_scale.reverse = reverse;
            o.rank_scale.reverse = reverse;
            o.x_scale.round = round;
            o.rank_scale.round = round;
            o.labels.color = InheritedColor::Fixed(Color(11, 22, 33, 101));
            o.style.opacity = 0.5;
            let before = bump_layout(&c, &data(), &o).unwrap();
            o.labels.start_style = side(Some(8.), None);
            o.labels.end_style = side(None, Some(InheritedColor::Theme));
            let after = bump_layout(&c, &data(), &o).unwrap();
            assert_eq!(before.series, after.series);
            assert_eq!(before.interaction, after.interaction);
            assert_eq!(before.x_scale, after.x_scale);
            assert_eq!(before.rank_scale, after.rank_scale);
            assert_eq!(before.scene.label_series, after.scene.label_series);
            let mut normalized = after.scene.clone();
            normalized.labels.clone_from(&before.scene.labels);
            assert_eq!(normalized, before.scene);
            for i in [0, 2] {
                close(after.scene.labels[i].x, before.scene.labels[i].x + 8.);
                assert_eq!(after.scene.labels[i].color, Color(11, 22, 33, 51));
            }
            for i in [1, 3] {
                assert_eq!(after.scene.labels[i].x, before.scene.labels[i].x);
                let mut expected = c.theme.text;
                expected.3 = (f64::from(expected.3) * 0.5).round() as u8;
                assert_eq!(after.scene.labels[i].color, expected);
            }
            after.clone().into_scene_interaction().validate().unwrap();
        }
    }
}

#[test]
fn modifiers_use_series_color_theme_surface_and_existing_typed_rgba_semantics() {
    let mut o = options();
    o.style.color = Some(Color(20, 40, 60, 0));
    o.style.opacity = 0.5;
    o.labels.start_style = side(
        Some(0.),
        Some(InheritedColor::FromContext(vec![
            ColorModifier::Darker(1.),
            ColorModifier::Opacity(0.4),
        ])),
    );
    o.labels.end_style = side(
        Some(16384.),
        Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
    );
    let l = bump_layout(&frame(), &data(), &o).unwrap();
    assert_eq!(l.scene.labels[0].color, Color(14, 28, 42, 51));
    assert_eq!(l.scene.labels[1].color, Color(40, 50, 60, 51));
    // The large authored padding keeps the established off-plot endpoint policy.
    assert!(l.scene.labels[1].x > frame().width);
    let mut c = frame();
    c.theme.background = Color(255, 255, 255, 255);
    o.labels.end_style = side(
        None,
        Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
    );
    let l = bump_layout(&c, &data(), &o).unwrap();
    assert_eq!(l.scene.labels[1].color, Color(0, 0, 0, 128));
}

#[test]
fn hidden_endpoint_gaps_singletons_and_custom_point_paint_keep_identity() {
    let mut d = data();
    d[0].data[0].rank = None;
    d.push(BumpSeries::new("single", vec![BumpDatum::new("Q2", 1.)]));
    d.push(BumpSeries::new("empty", vec![]));
    let mut o = options();
    o.hidden.insert("Beta".into());
    o.labels.start_style = side(Some(8.), Some(InheritedColor::Fixed(Color(9, 8, 7, 101))));
    let normal = bump_layout(&frame(), &d, &o).unwrap();
    let mut calls = 0;
    let custom = bump_layout_with_point_factory(&frame(), &d, &o, |ctx| {
        calls += 1;
        let mut p = ctx.inherit(Geometry::Circle {
            x: ctx.position[0],
            y: ctx.position[1],
            radius: 4.,
        });
        p.fill = Some(Color(240, 30, 180, 255));
        Ok(p)
    })
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(normal.scene.labels, custom.scene.labels);
    assert_eq!(normal.scene.label_series, custom.scene.label_series);
    assert_eq!(
        custom
            .scene
            .label_series
            .values()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["Alpha", "single", "single"]
    );
    assert_eq!(custom.series[0].source, d[0]);
    assert!(custom.series[1].hidden);
    assert_eq!(custom.interaction.sites.len(), 2);
}

#[test]
fn intrinsic_invalid_overrides_fail_even_without_emitted_labels_and_before_factory() {
    for invalid in [
        side(Some(f64::NAN), None),
        side(Some(f64::INFINITY), None),
        side(Some(-1.), None),
        side(Some(16384.1), None),
        side(
            None,
            Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                f64::NAN,
            )])),
        ),
    ] {
        for kind in 0..4 {
            for start in [true, false] {
                let mut o = options();
                if start {
                    o.labels.start_style = invalid.clone();
                } else {
                    o.labels.end_style = invalid.clone();
                }
                let d = if kind == 0 { vec![] } else { data() };
                if kind == 1 {
                    o.labels.start = false;
                    o.labels.end = false;
                }
                if kind == 2 {
                    o.hidden.extend(["Alpha".into(), "Beta".into()]);
                }
                let mut d = d;
                if kind == 3 {
                    for row in &mut d {
                        for p in &mut row.data {
                            p.rank = None;
                        }
                    }
                }
                let prior_d = d.clone();
                let prior_o = o.clone();
                let mut calls = 0;
                assert!(
                    bump_layout_with_point_factory(&frame(), &d, &o, |ctx| {
                        calls += 1;
                        Ok(ctx.inherit(Geometry::Circle {
                            x: ctx.position[0],
                            y: ctx.position[1],
                            radius: 3.,
                        }))
                    })
                    .is_err()
                );
                assert_eq!(calls, 0);
                assert_eq!(d, prior_d);
                // NaN is deliberately invalid and never equal to itself under PartialEq.
                assert_eq!(format!("{o:?}"), format!("{prior_o:?}"));
                for (after, before) in [
                    (&o.labels.start_style, &prior_o.labels.start_style),
                    (&o.labels.end_style, &prior_o.labels.end_style),
                ] {
                    assert_eq!(
                        after.as_ref().and_then(|s| s.padding).map(f64::to_bits),
                        before.as_ref().and_then(|s| s.padding).map(f64::to_bits)
                    );
                }
            }
        }
    }
}

#[test]
fn contrast_context_still_only_resolves_for_enabled_visible_ungapped_endpoints() {
    let mut c = frame();
    c.theme.background = Color(0, 0, 0, 0);
    let mut o = options();
    o.style.color = Some(Color(0, 0, 0, 0));
    o.labels.start_style = side(
        None,
        Some(InheritedColor::Contrast {
            dark: Color(0, 0, 0, 255),
            light: Color(255, 255, 255, 255),
        }),
    );
    assert!(bump_layout(&c, &[], &o).is_ok());
    o.labels.start = false;
    assert!(bump_layout(&c, &data(), &o).is_ok());
    o.labels.start = true;
    let mut d = data();
    for s in &mut d {
        s.data[0].rank = None;
    }
    assert!(bump_layout(&c, &d, &o).is_ok());
    assert!(bump_layout(&c, &data(), &o).is_err());
}

#[test]
fn controlled_relation_dim_restores_owned_side_colors_without_compounding_alpha() {
    let mut o = options();
    o.style.opacity = 0.5;
    o.labels.start_style = side(
        Some(8.),
        Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
    );
    o.labels.end_style = side(
        Some(24.),
        Some(InheritedColor::Fixed(Color(70, 80, 90, 201))),
    );
    let l = bump_layout(&frame(), &data(), &o).unwrap();
    let nodes = l
        .series
        .iter()
        .map(|s| {
            let mut n = RelationNode::new(&s.source.id);
            n.mark_ids = l
                .scene
                .marks
                .iter()
                .filter(|m| m.series == s.source.id)
                .map(|m| m.id.clone())
                .collect();
            n.label_indices = l
                .scene
                .label_series
                .iter()
                .filter(|(_, id)| **id == s.source.id)
                .map(|(i, _)| *i)
                .collect();
            n
        })
        .collect();
    let mut ro = RelationOptions::default();
    ro.inactive_node.label_opacity = 0.2;
    let mut controller = RelationLayout::new(
        l.scene.clone(),
        RelationGraph::new(nodes, vec![]).unwrap(),
        ro,
    )
    .unwrap();
    controller
        .set_focus(Some(RelationFocus::Node("Alpha".into())))
        .unwrap();
    assert_eq!(controller.scene().labels[0].color, Color(40, 50, 60, 51));
    assert_eq!(controller.scene().labels[2].color, Color(40, 50, 60, 10));
    assert_eq!(controller.scene().labels[3].color, Color(70, 80, 90, 20));
    controller
        .set_focus(Some(RelationFocus::Node("Beta".into())))
        .unwrap();
    assert_eq!(controller.scene().labels[2].color, Color(40, 50, 60, 51));
    controller.set_focus(None).unwrap();
    assert_eq!(controller.scene(), &l.scene);
}

#[test]
fn surface_window_navigation_and_invalid_replacement_preserve_source_and_owned_labels() {
    let mut o = options();
    o.labels.start_style = side(Some(8.), None);
    o.labels.end_style = side(Some(24.), None);
    let l = bump_layout(&frame(), &data(), &o).unwrap();
    let mut surface = SurfaceModel::new_with_interaction(
        l.scene.clone(),
        SurfaceOptions {
            plot: Some(l.plot),
            ..Default::default()
        },
        Some(l.interaction.clone()),
    )
    .unwrap();
    surface
        .apply(SurfaceInput::Wheel {
            point: (400., 200.),
            delta: -150.,
        })
        .unwrap();
    assert!(surface.view().scale > 1.);
    assert_eq!(surface.scene(), &l.scene);
    assert_eq!(surface.interaction_data(), Some(&l.interaction));
    let prior = surface.scene().clone();
    let mut invalid = l.clone().into_scene_interaction();
    invalid.scene.labels[0].x = f64::NAN;
    assert!(surface.set_layout(invalid).is_err());
    assert_eq!(surface.scene(), &prior);
    surface.apply(SurfaceInput::Reset).unwrap();
    assert_eq!(surface.scene(), &l.scene);
}
