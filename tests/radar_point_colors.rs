use glory::render::StrokeCap;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 200.,
        height: 200.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        theme: Theme {
            colors: vec![Color(20, 40, 60, 255), Color(100, 120, 140, 255)],
            ..Default::default()
        },
        ..Default::default()
    }
}
fn axes() -> Vec<String> {
    ["north", "east", "south", "west"]
        .map(str::to_owned)
        .to_vec()
}
fn data() -> Vec<RadarSeries> {
    vec![
        RadarSeries::new("alpha", vec![1., 0.5, 0.75, 0.]),
        RadarSeries::new("beta", vec![0.5, 1., 0., 0.75]),
    ]
}
fn options() -> RadarLayoutOptions {
    let mut o = RadarLayoutOptions {
        domain: Some([0., 1.]),
        axis_labels: false,
        ..Default::default()
    };
    o.grid.line.enabled = false;
    o.grid.spokes.enabled = false;
    o.series.fill_enabled = false;
    o.series.border.enabled = false;
    o.points.size = 10.;
    o.points.border = RadarLineStyle {
        enabled: true,
        width: 2.,
        opacity: 0.5,
        dash: Some(StrokeDash {
            segments: vec![2., 1.],
            offset: 0.,
        }),
        ..Default::default()
    };
    o
}
fn roles(o: &mut RadarLayoutOptions) {
    o.points.fill_color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    o.points.border_color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Brighter(
            1.,
        )]));
}
fn point_marks(scene: &Scene) -> Vec<&Mark> {
    scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("radar-point:"))
        .collect()
}
fn setup_shape(o: &mut RadarLayoutOptions, kind: usize) {
    match kind {
        0 => {}
        1 => o.points.shape = RadarPointShape::Triangle,
        2 => {
            o.points.glyph = Some(GlyphOptions {
                shape: GlyphShape::Filled(FilledGlyph::Diamond),
                size: 64.,
                rotation: 0.2,
            })
        }
        _ => {
            o.points.glyph = Some(GlyphOptions {
                shape: GlyphShape::Stroked(StrokedGlyph::Plus),
                size: 64.,
                rotation: 0.2,
            })
        }
    }
}
fn contrast() -> InheritedColor {
    InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    }
}

#[test]
fn declarative_roles_supply_identical_paint_to_builtin_shapes_and_factory_context() -> Result<()> {
    for kind in 0..4 {
        let mut o = options();
        setup_shape(&mut o, kind);
        roles(&mut o);
        o.points.fill = Some(Color(255, 0, 255, 255));
        o.points.border.color = Some(Color(0, 255, 0, 255));
        let ordinary = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
        let mut calls = 0;
        let custom = radar_layout_with_point_factory(
            &config(),
            &axes(),
            &data(),
            &o,
            &Default::default(),
            |ctx| {
                calls += 1;
                let fill = if ctx.series_index == 0 {
                    Color(14, 28, 42, 255)
                } else {
                    Color(70, 84, 98, 255)
                };
                let border = if ctx.series_index == 0 {
                    Color(29, 57, 86, 128)
                } else {
                    Color(143, 171, 200, 128)
                };
                assert_eq!(ctx.color, config().theme.color(ctx.series_index));
                assert_eq!(ctx.fill, (kind != 3).then_some(fill));
                assert_eq!(ctx.stroke.unwrap().color, border);
                assert_eq!(ctx.stroke.unwrap().cap, StrokeCap::Butt);
                assert!(ctx.stroke_dash.is_some());
                let authored = ordinary
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == ctx.mark_id)
                    .unwrap();
                assert_eq!(authored.fill, ctx.fill);
                assert_eq!(authored.stroke, ctx.stroke);
                Ok(ctx.inherit(authored.geometry.clone()))
            },
        )?;
        assert_eq!(calls, 8);
        for mark in point_marks(&custom.scene) {
            let old = ordinary
                .scene
                .marks
                .iter()
                .find(|m| m.id == mark.id)
                .unwrap();
            assert_eq!(mark.fill, old.fill);
            assert_eq!(mark.stroke, old.stroke);
            assert_eq!(mark.stroke_dash, old.stroke_dash);
        }
    }
    Ok(())
}

#[test]
fn theme_roles_keep_each_legacy_fallback_and_border_opacity_is_applied_once() -> Result<()> {
    for explicit in [false, true] {
        let mut o = options();
        o.points.fill_color_profile = Some(InheritedColor::Theme);
        o.points.border_color_profile = Some(InheritedColor::Theme);
        if explicit {
            o.points.fill = Some(Color(40, 50, 60, 101));
            o.points.border.color = Some(Color(70, 80, 90, 101));
        }
        let l = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
        for mark in point_marks(&l.scene) {
            let palette = config()
                .theme
                .color(if mark.series == "alpha" { 0 } else { 1 });
            assert_eq!(
                mark.fill,
                Some(if explicit {
                    Color(40, 50, 60, 101)
                } else {
                    palette
                })
            );
            let mut border = if explicit {
                Color(70, 80, 90, 101)
            } else {
                palette
            };
            border.3 = if explicit { 51 } else { 128 };
            assert_eq!(mark.stroke.unwrap().color, border);
            assert_eq!(mark.opacity, 1.);
        }
    }
    let mut o = options();
    o.points.fill_color_profile = Some(InheritedColor::Fixed(Color(40, 50, 60, 101)));
    o.points.border_color_profile =
        Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
            0.4,
        )]));
    let l = radar_layout_with_point_factory(
        &config(),
        &axes(),
        &data(),
        &o,
        &Default::default(),
        |ctx| {
            assert_eq!(ctx.stroke.unwrap().color.3, 51);
            let mut p = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            });
            p.opacity = 0.3;
            Ok(p)
        },
    )?;
    assert!(
        point_marks(&l.scene)
            .iter()
            .all(|m| m.fill == Some(Color(40, 50, 60, 101))
                && m.stroke.unwrap().color.3 == 51
                && m.opacity == 0.3)
    );
    Ok(())
}

#[test]
fn role_does_not_enable_border_or_invent_fill_for_stroked_glyphs() -> Result<()> {
    let mut o = options();
    roles(&mut o);
    o.points.border.enabled = false;
    let plain = radar_with_options(&config(), &axes(), &data(), &o)?;
    assert!(point_marks(&plain).iter().all(|m| m.stroke.is_none()));
    setup_shape(&mut o, 3);
    assert!(radar_with_options(&config(), &axes(), &data(), &o).is_err());
    o.points.border.enabled = true;
    let mut calls = 0;
    let l = radar_layout_with_point_factory(
        &config(),
        &axes(),
        &data(),
        &o,
        &Default::default(),
        |ctx| {
            calls += 1;
            assert_eq!(ctx.fill, None);
            // The explicit custom factory still may choose its own filled shape.
            let mut p = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            });
            p.fill = Some(Color(1, 2, 3, 255));
            Ok(p)
        },
    )?;
    assert_eq!(calls, 8);
    assert!(
        point_marks(&l.scene)
            .iter()
            .all(|m| m.fill == Some(Color(1, 2, 3, 255)))
    );
    Ok(())
}

#[test]
fn original_palette_is_independent_of_polygon_swatch_and_custom_paint_representative() -> Result<()>
{
    let mut o = options();
    roles(&mut o);
    o.series.fill_color = Some(Color(200, 20, 140, 255));
    o.series.border.color = Some(Color(30, 200, 70, 255));
    let mut observations = vec![];
    let custom = radar_layout_with_point_factory(
        &config(),
        &axes(),
        &data(),
        &o,
        &Default::default(),
        |ctx| {
            observations.push((ctx.series_index, ctx.axis_index, ctx.fill));
            assert_eq!(ctx.color, Color(200, 20, 140, 255));
            let mut p = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            });
            if ctx.axis_index == 0 {
                p.fill_paint = Some(FillPaint::Linear {
                    start: [0., 0.],
                    end: [200., 0.],
                    stops: vec![
                        GradientStop::new(0., Color(250, 10, 30, 255)),
                        GradientStop::new(1., Color(10, 250, 30, 255)),
                    ],
                });
            }
            Ok(p)
        },
    )?;
    assert_eq!(observations.len(), 8);
    for (i, _, fill) in observations {
        assert_eq!(
            fill,
            Some(if i == 0 {
                Color(14, 28, 42, 255)
            } else {
                Color(70, 84, 98, 255)
            })
        );
    }
    let ordinary = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
    assert!(
        ordinary
            .slices
            .iter()
            .flat_map(|s| &s.rows)
            .all(|r| r.color == Color(200, 20, 140, 255))
    );
    assert!(
        ordinary
            .scene
            .legend
            .iter()
            .all(|l| l.color == Color(200, 20, 140, 255))
    );
    assert!(
        custom.slices[0]
            .rows
            .iter()
            .all(|r| r.color == Color(250, 10, 30, 255))
    );
    assert!(custom.slices[1].rows.iter().all(|r| r.color
        == if r.series_index == 0 {
            Color(14, 28, 42, 255)
        } else {
            Color(70, 84, 98, 255)
        }));
    custom.interaction_binding().validate(&custom.scene)?;
    Ok(())
}

#[test]
fn whole_per_series_point_override_keeps_existing_selection_semantics() -> Result<()> {
    let mut o = options();
    roles(&mut o);
    let override_points = RadarPointOptions {
        fill: Some(Color(8, 9, 10, 111)),
        border: o.points.border.clone(),
        ..Default::default()
    };
    o.series_styles.insert(
        "alpha".into(),
        RadarSeriesStyle {
            points: Some(override_points),
            ..o.series.clone()
        },
    );
    let l = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
    for m in point_marks(&l.scene) {
        assert_eq!(
            m.fill,
            Some(if m.series == "alpha" {
                Color(8, 9, 10, 111)
            } else {
                Color(70, 84, 98, 255)
            })
        );
    }
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    c.theme.colors = vec![Color(10, 20, 30, 128)];
    o.points.fill_color_profile = Some(contrast());
    for d in data() {
        o.series_styles.insert(
            d.id,
            RadarSeriesStyle {
                points: Some(RadarPointOptions::default()),
                ..o.series.clone()
            },
        );
    }
    // Global profile is intrinsically valid but unused by actual selected opts.
    radar_layout(&c, &axes(), &data(), &o, &Default::default())?;
    Ok(())
}

#[test]
fn alpha_zero_preserves_typed_roles_and_existing_custom_normalization_boundary() -> Result<()> {
    let mut o = options();
    o.points.fill_color_profile = Some(InheritedColor::Fixed(Color(40, 50, 60, 0)));
    o.points.border.enabled = false;
    o.series.fill_color = Some(Color(200, 20, 140, 255));
    let ordinary = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
    assert!(
        point_marks(&ordinary.scene)
            .iter()
            .all(|m| m.fill == Some(Color(40, 50, 60, 0)))
    );
    let custom = radar_layout_with_point_factory(
        &config(),
        &axes(),
        &data(),
        &o,
        &Default::default(),
        |ctx| {
            assert_eq!(ctx.fill, Some(Color(40, 50, 60, 0)));
            Ok(ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            }))
        },
    )?;
    assert!(point_marks(&custom.scene).iter().all(|m| m.fill.is_none()));
    assert!(
        custom
            .slices
            .iter()
            .flat_map(|s| &s.rows)
            .all(|r| r.color == Color(200, 20, 140, 255))
    );
    o.points.border.enabled = true;
    o.points.border_color_profile = Some(InheritedColor::Fixed(Color(70, 80, 90, 0)));
    setup_shape(&mut o, 3);
    let stroked = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
    assert!(
        point_marks(&stroked.scene)
            .iter()
            .all(|m| m.fill.is_none() && m.stroke.unwrap().color == Color(70, 80, 90, 0))
    );
    Ok(())
}

#[test]
fn intrinsic_invalid_roles_reject_before_callbacks_for_hidden_disabled_unused_and_empty() {
    for border in [false, true] {
        for kind in 0..5 {
            let mut o = options();
            let bad = Some(InheritedColor::FromContext(vec![ColorModifier::Darker(
                f64::NAN,
            )]));
            if border {
                o.points.border_color_profile = bad;
            } else {
                o.points.fill_color_profile = bad;
            }
            let mut source = data();
            match kind {
                0 => {}
                1 => {
                    o.hidden = source.iter().map(|s| s.id.clone()).collect();
                }
                2 => o.points.enabled = false,
                3 => source.clear(),
                _ => {
                    for d in &source {
                        o.series_styles.insert(
                            d.id.clone(),
                            RadarSeriesStyle {
                                points: Some(RadarPointOptions::default()),
                                ..o.series.clone()
                            },
                        );
                    }
                }
            }
            let mut calls = 0;
            assert!(
                radar_layout_with_point_factory(
                    &config(),
                    &axes(),
                    &source,
                    &o,
                    &Default::default(),
                    |ctx| {
                        calls += 1;
                        Ok(ctx.inherit(Geometry::Circle {
                            x: ctx.position[0],
                            y: ctx.position[1],
                            radius: 3.,
                        }))
                    }
                )
                .is_err()
            );
            assert_eq!(calls, 0);
        }
    }
}

#[test]
fn all_real_contrast_contexts_are_preflighted_before_first_factory_without_empty_fake_context()
-> Result<()> {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    c.theme.colors = vec![Color(0, 0, 0, 255), Color(40, 50, 60, 128)];
    for hidden in [false, true] {
        let mut o = options();
        o.points.fill_color_profile = Some(contrast());
        if hidden {
            o.hidden.insert("beta".into());
        } else {
            o.points.enabled = false;
        }
        let mut calls = 0;
        assert!(
            radar_layout_with_point_factory(&c, &axes(), &data(), &o, &Default::default(), |ctx| {
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
    }
    let mut o = options();
    o.points.border_color_profile = Some(contrast());
    o.points.border.enabled = false;
    assert!(radar_with_options(&c, &axes(), &data(), &o).is_err());
    radar_with_options(&c, &axes(), &[], &o)?;
    c.theme.colors = vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)];
    o.points.border.enabled = true;
    let l = radar_layout(&c, &axes(), &data(), &o, &Default::default())?;
    for m in point_marks(&l.scene) {
        assert_eq!(
            m.stroke.unwrap().color,
            if m.series == "alpha" {
                Color(255, 255, 255, 128)
            } else {
                Color(0, 0, 0, 128)
            }
        );
    }
    Ok(())
}

#[test]
fn roles_preserve_scene_source_binding_and_surface_contract_without_new_freeze_claim() -> Result<()>
{
    use kartis::surface::{SurfaceInput, SurfaceModel};
    let o = options();
    let ordinary = radar_layout(&config(), &axes(), &data(), &o, &Default::default())?;
    let mut with = o.clone();
    roles(&mut with);
    let colored = radar_layout(&config(), &axes(), &data(), &with, &Default::default())?;
    assert_eq!(colored.slices, ordinary.slices);
    assert_eq!(colored.polar, ordinary.polar);
    assert_eq!(colored.domain, ordinary.domain);
    assert_eq!(colored.scene.legend, ordinary.scene.legend);
    assert_eq!(
        colored.interaction_binding(),
        ordinary.interaction_binding()
    );
    let mut restored = colored.scene.clone();
    for m in &mut restored.marks {
        if m.id.starts_with("radar-point:") {
            let old = ordinary.scene.marks.iter().find(|p| p.id == m.id).unwrap();
            m.fill = old.fill;
            m.stroke = old.stroke;
        }
    }
    assert_eq!(restored, ordinary.scene);
    let row = &colored.slices[0].rows[0];
    let mut surface = SurfaceModel::new_with_interaction(
        colored.scene.clone(),
        Default::default(),
        Some(colored.interaction_binding()),
    )?;
    surface.apply(SurfaceInput::Move {
        id: 1,
        point: (row.position[0], row.position[1]),
    })?;
    surface.apply(SurfaceInput::Leave)?;
    assert_eq!(surface.scene(), &colored.scene);
    let mut bad = colored.clone();
    bad.slices[0].rows[0].raw_value = 99.;
    assert!(bad.interaction_binding().validate(&bad.scene).is_err());
    let mut bad = colored.clone();
    bad.scene.marks.retain(|m| m.id != row.point_mark_id);
    assert!(bad.interaction_binding().validate(&bad.scene).is_err());
    // Public Radar layout still has its existing checked candidate contract,
    // not a new private business-source/frozen-label protocol.
    Ok(())
}

#[test]
fn scaled_counterclockwise_factory_uses_same_roles_and_error_keeps_accepted_frame() -> Result<()> {
    let source = [RadarSeries::new("alpha", vec![1., 10., 100., 1000.])];
    let mut o = options();
    roles(&mut o);
    o.domain = None;
    let scale = RadarValueScaleOptions {
        kind: RadarValueScaleKind::Log { base: 10. },
        ..Default::default()
    };
    let interaction = RadarInteractionOptions {
        direction: PolarSliceDirection::CounterClockwise,
        ..Default::default()
    };
    let accepted =
        radar_layout_with_value_scale(&config(), &axes(), &source, &o, &scale, &interaction)?;
    let before = accepted.layout.scene.clone();
    let source_before = source.clone();
    let mut calls = vec![];
    let l = radar_layout_with_value_scale_and_point_factory(
        &config(),
        &axes(),
        &source,
        &o,
        &scale,
        &interaction,
        |ctx| {
            calls.push(ctx.axis_index);
            assert_eq!(ctx.fill, Some(Color(14, 28, 42, 255)));
            Ok(ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            }))
        },
    )?;
    assert_eq!(calls, [0, 3, 2, 1]);
    assert_eq!(
        l.value_scale.source_domain(),
        accepted.value_scale.source_domain()
    );
    assert_eq!(
        l.value_scale.display_domain(),
        accepted.value_scale.display_domain()
    );
    assert_eq!(
        l.value_scale.outer_radius(),
        accepted.value_scale.outer_radius()
    );
    for value in [1., 10., 100., 1000.] {
        assert_eq!(
            l.value_scale.radius_at(value)?,
            accepted.value_scale.radius_at(value)?
        );
    }
    assert_eq!(source, source_before);
    let mut effects = 0;
    assert!(
        radar_layout_with_value_scale_and_point_factory(
            &config(),
            &axes(),
            &source,
            &o,
            &scale,
            &interaction,
            |_| {
                effects += 1;
                Err(ChartError("caller rejected".into()))
            }
        )
        .is_err()
    );
    assert_eq!(effects, 1);
    assert_eq!(accepted.layout.scene, before);
    Ok(())
}
