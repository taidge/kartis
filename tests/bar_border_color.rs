use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use std::cell::RefCell;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        ..Default::default()
    }
}
fn border(color: Color) -> GraphicStroke {
    GraphicStroke {
        color,
        width: 2.5,
        cap: StrokeCap::Round,
    }
}
fn categories() -> Vec<String> {
    ["positive", "zero", "missing", "negative"]
        .map(str::to_string)
        .to_vec()
}
fn data() -> Vec<BarSeries> {
    vec![
        BarSeries::new("z", vec![Some(3.), Some(0.), None, Some(-2.)]),
        BarSeries::new("a", vec![Some(5.), Some(0.), None, Some(-4.)]),
    ]
}
fn style() -> BarStyleOptions {
    BarStyleOptions {
        colors: [
            ("z".into(), Color(20, 40, 60, 160)),
            ("a".into(), Color(200, 180, 160, 220)),
        ]
        .into(),
        series_order: Some(vec!["z".into(), "a".into()]),
        labels: BarLabels {
            enabled: true,
            zero_extent: BarZeroExtentLabels::ValueAxis,
            color_profile: Some(InheritedColor::Fixed(Color(40, 50, 60, 101))),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn role() -> InheritedColor {
    InheritedColor::FromContext(vec![ColorModifier::Darker(1.), ColorModifier::Opacity(0.4)])
}
fn expected(
    c: &ChartConfig,
    fixed: GraphicStroke,
    fill: Color,
    profile: &InheritedColor,
) -> Result<GraphicStroke> {
    Ok(GraphicStroke {
        color: profile.resolve(InheritedColorContext {
            inherited: fill,
            surface: fill,
            theme: fixed.color,
            backdrop: c.theme.background,
        })?,
        ..fixed
    })
}
fn mark<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene.marks.iter().find(|m| m.id == id).unwrap()
}

#[test]
fn classic_bar_resolves_palette_and_theme_without_enabling_a_border() -> Result<()> {
    let c = config();
    let fixed = border(Color(7, 8, 9, 137));
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            let o = BarOptions {
                border: Some(fixed),
                horizontal,
                layout,
                corner_radius: 6.,
                ..Default::default()
            };
            let old = bar(&c, &categories(), &data(), &o)?;
            for profile in [
                InheritedColor::Theme,
                InheritedColor::Fixed(Color(1, 2, 3, 4)),
                role(),
                InheritedColor::Contrast {
                    dark: Color(0, 0, 0, 255),
                    light: Color(255, 255, 255, 255),
                },
            ] {
                let options = BarOptions {
                    border_color_profile: Some(profile.clone()),
                    ..o.clone()
                };
                let mut scene = bar(&c, &categories(), &data(), &options)?;
                let mut count = 0;
                for m in &mut scene.marks {
                    if m.id.starts_with("bar-") {
                        count += 1;
                        let fill = m.fill.unwrap();
                        assert_eq!(m.stroke, Some(expected(&c, fixed, fill, &profile)?));
                        m.stroke = Some(fixed);
                    }
                }
                assert_eq!(count, 6);
                assert_eq!(scene, old);
                assert_eq!(render::to_svg(&scene)?, render::to_svg(&old)?);
                assert_eq!(
                    render::to_canvas_script(&scene, "c")?,
                    render::to_canvas_script(&old, "c")?
                );
            }
            let none = BarOptions {
                border: None,
                border_color_profile: Some(role()),
                ..o.clone()
            };
            let legacy_none = BarOptions {
                border_color_profile: None,
                ..none.clone()
            };
            assert_eq!(
                bar(&c, &categories(), &data(), &none)?,
                bar(&c, &categories(), &data(), &legacy_none)?
            );
        }
    }
    Ok(())
}

#[test]
fn styled_category_and_series_roles_preserve_scale_source_and_clipping() -> Result<()> {
    let c = config();
    let fixed = border(Color(9, 8, 7, 123));
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            for category in [false, true] {
                for configured in [false, true] {
                    let mut s = style();
                    if category {
                        s.color_by = BarColorBy::Category;
                        s.colors = categories()
                            .into_iter()
                            .enumerate()
                            .map(|(i, id)| (id, Color(20 + i as u8 * 20, 50, 90, 180)))
                            .collect();
                    }
                    s.category_scale = Some(scales::BandScaleOptions {
                        round: true,
                        ..Default::default()
                    });
                    s.category_reverse = horizontal;
                    let old_options = BarOptions {
                        horizontal,
                        layout,
                        border: Some(fixed),
                        corner_radius: 7.,
                        category_window: Some([0.1, 0.9]),
                        value_domain: Some([-1., 4.]),
                        ..Default::default()
                    };
                    let options = BarOptions {
                        border_color_profile: Some(role()),
                        ..old_options.clone()
                    };
                    let scale = ContinuousScaleOptions {
                        round: true,
                        reverse: true,
                        kind: ScaleKind::SymLog { constant: 1. },
                        ..Default::default()
                    };
                    let build = |o: &BarOptions| -> Result<StyledBarLayout> {
                        if configured {
                            Ok(bar_layout_with_scale_options(
                                &c,
                                &categories(),
                                &data(),
                                o,
                                &s,
                                &scale,
                            )?
                            .layout)
                        } else {
                            bar_layout_with_style(&c, &categories(), &data(), o, &s)
                        }
                    };
                    let old = build(&old_options)?;
                    let mut actual = build(&options)?;
                    assert!(!actual.bars.is_empty());
                    assert!(actual.bars.iter().all(|b| b.source_category != 2));
                    assert!(actual.bars.iter().any(|b| b.raw_value == 0.));
                    for b in &actual.bars {
                        assert_eq!(data()[b.source_series].id, b.series);
                        let m = actual
                            .scene
                            .marks
                            .iter_mut()
                            .find(|m| m.id == b.id)
                            .unwrap();
                        assert_eq!(m.stroke, Some(expected(&c, fixed, b.color, &role())?));
                        m.stroke = Some(fixed);
                    }
                    assert_eq!(actual, old);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn both_factory_paths_receive_resolved_stroke_and_keep_explicit_override() -> Result<()> {
    let c = config();
    let s = style();
    let fixed = border(Color(9, 8, 7, 123));
    let override_stroke = border(Color(200, 3, 4, 79));
    let o = BarOptions {
        border: Some(fixed),
        border_color_profile: Some(role()),
        corner_radius: 5.,
        ..Default::default()
    };
    for configured in [false, true] {
        for with_label in [false, true] {
            for replace_stroke in [false, true] {
                let seen = RefCell::new(Vec::new());
                let text_count = RefCell::new(0);
                let mut text = |ctx: &BarLabelContext<'_>| {
                    *text_count.borrow_mut() += 1;
                    Ok(ctx.default_text.to_owned())
                };
                let mut geometry = |ctx: &BarDatumContext<'_>| {
                    assert_eq!(ctx.stroke, Some(expected(&c, fixed, ctx.color, &role())?));
                    seen.borrow_mut().push((
                        ctx.source_series,
                        ctx.source_category,
                        ctx.raw_value,
                        ctx.position,
                        ctx.stroke,
                    ));
                    let mut n = ctx.inherit(ctx.geometry.clone());
                    // Custom representative color must not feed back into the border role.
                    n.fill = Some(Color(0, 255, 0, 255));
                    n.opacity = 0.5;
                    if replace_stroke {
                        n.stroke = Some(override_stroke);
                    }
                    Ok(n)
                };
                let factories = BarFactories {
                    bar: Some(&mut geometry),
                    label: if with_label { Some(&mut text) } else { None },
                };
                let frame = if configured {
                    bar_layout_with_scale_options_and_factories(
                        &c,
                        &categories(),
                        &data(),
                        &o,
                        &s,
                        &ContinuousScaleOptions {
                            round: true,
                            reverse: true,
                            ..Default::default()
                        },
                        factories,
                    )?
                    .layout
                } else {
                    bar_layout_with_factories(&c, &categories(), &data(), &o, &s, factories)?
                };
                assert_eq!(seen.borrow().len(), 6);
                assert_eq!(*text_count.borrow(), if with_label { 6 } else { 0 });
                let binding = frame.interaction_binding()?;
                assert_eq!(binding.sites.len(), 6);
                assert_eq!(
                    frame
                        .layout
                        .bars
                        .iter()
                        .filter(|b| b.raw_value == 0.)
                        .count(),
                    2
                );
                for b in &frame.layout.bars {
                    let m = mark(&frame.layout.scene, &b.id);
                    assert_eq!(m.opacity, 0.5);
                    let original_fill = s.colors[&data()[b.source_series].id];
                    assert_eq!(
                        m.stroke,
                        Some(if replace_stroke {
                            override_stroke
                        } else {
                            expected(&c, fixed, original_fill, &role())?
                        })
                    );
                    // Source metadata retains its numeric palette. The prepared
                    // paint representative belongs to the interaction binding.
                    assert_eq!(b.color, original_fill);
                    assert_eq!(m.fill, Some(Color(0, 255, 0, 255)));
                    assert_eq!(
                        binding
                            .sites
                            .iter()
                            .find(|s| s.mark_id == b.id)
                            .unwrap()
                            .color,
                        Color(0, 255, 0, 255)
                    );
                    if b.raw_value == 0. {
                        assert_eq!(b.visible_rect.height, 0.);
                        assert!(matches!(m.geometry, Geometry::Rect { height: 0., .. }));
                    } else {
                        assert!(b.visible_rect.width > 0. && b.visible_rect.height > 0.);
                    }
                    assert_eq!(
                        b.raw_value,
                        data()[b.source_series].values[b.source_category].unwrap()
                    );
                }
            }
        }
    }
    Ok(())
}

#[test]
fn hover_keeps_border_rgba_and_owned_label_alpha_and_rejects_source_forgery() -> Result<()> {
    let fixed = border(Color(9, 8, 7, 123));
    let o = BarOptions {
        border: Some(fixed),
        border_color_profile: Some(role()),
        ..Default::default()
    };
    let mut geometry = |ctx: &BarDatumContext<'_>| {
        let mut n = ctx.inherit(ctx.geometry.clone());
        n.opacity = 0.5;
        Ok(n)
    };
    let text_calls = RefCell::new(0);
    let mut text = |ctx: &BarLabelContext<'_>| {
        *text_calls.borrow_mut() += 1;
        Ok(ctx.default_text.to_owned())
    };
    let mut frame = bar_layout_with_factories(
        &config(),
        &categories(),
        &data(),
        &o,
        &style(),
        BarFactories {
            bar: Some(&mut geometry),
            label: Some(&mut text),
        },
    )?;
    let base = frame.layout.clone();
    let count = *text_calls.borrow();
    let active = base.bars[0].id.clone();
    frame.set_active(Some(&active), 0.2)?;
    for b in &frame.layout.bars {
        let m = mark(&frame.layout.scene, &b.id);
        assert_eq!(m.stroke, mark(&base.scene, &b.id).stroke);
        assert_eq!(m.stroke.unwrap().color.3, 102);
        assert_eq!(m.opacity, if b.id == active { 0.5 } else { 0.1 });
        let i = b.label_index.unwrap();
        assert_eq!(
            frame.layout.scene.labels[i].color,
            Color(40, 50, 60, if b.id == active { 51 } else { 10 })
        );
    }
    frame.interaction_binding()?;
    frame.set_active(None, 0.2)?;
    assert_eq!(frame.layout, base);
    assert_eq!(*text_calls.borrow(), count);
    for mutation in 0..3 {
        let mut bad = frame.clone();
        let id = bad.layout.bars[0].id.clone();
        let m = bad
            .layout
            .scene
            .marks
            .iter_mut()
            .find(|m| m.id == id)
            .unwrap();
        match mutation {
            0 => m.stroke.as_mut().unwrap().color = Color(1, 2, 3, 4),
            1 => m.stroke.as_mut().unwrap().width = 4.,
            _ => {
                bad.layout
                    .scene
                    .label_series
                    .remove(&base.bars[0].label_index.unwrap());
            }
        }
        let observed = bad.layout.clone();
        assert!(bad.interaction_binding().is_err());
        assert!(bad.set_active(Some(&active), 0.2).is_err());
        assert_eq!(bad.layout, observed);
    }
    Ok(())
}

#[test]
fn intrinsic_and_real_source_errors_precede_all_callbacks_including_hidden_disabled() -> Result<()>
{
    let invalid = [
        InheritedColor::FromContext(vec![ColorModifier::Darker(f64::NAN)]),
        InheritedColor::FromContext(vec![ColorModifier::Opacity(1.1)]),
        InheritedColor::FromContext(vec![ColorModifier::Brighter(1.); 33]),
    ];
    for profile in invalid {
        for mode in 0..5 {
            let (cats, source) = if mode == 0 {
                (vec![], vec![])
            } else {
                (
                    vec!["A".into()],
                    vec![BarSeries::new(
                        "s",
                        vec![if mode == 2 {
                            None
                        } else if mode == 3 {
                            Some(0.)
                        } else {
                            Some(2.)
                        }],
                    )],
                )
            };
            let mut o = BarOptions {
                border_color_profile: Some(profile.clone()),
                ..Default::default()
            };
            if mode == 1 {
                o.hidden.insert("s".into());
            }
            assert!(bar(&config(), &cats, &source, &o).is_err());
            for with_label in [false, true] {
                let calls = RefCell::new(0);
                let mut text = |c: &BarLabelContext<'_>| {
                    *calls.borrow_mut() += 1;
                    Ok(c.default_text.to_owned())
                };
                let mut geometry = |c: &BarDatumContext<'_>| {
                    *calls.borrow_mut() += 1;
                    Ok(c.inherit(c.geometry.clone()))
                };
                assert!(
                    bar_layout_with_factories(
                        &config(),
                        &cats,
                        &source,
                        &o,
                        &style(),
                        BarFactories {
                            bar: Some(&mut geometry),
                            label: if with_label { Some(&mut text) } else { None }
                        }
                    )
                    .is_err()
                );
                assert_eq!(*calls.borrow(), 0);
            }
        }
    }
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    c.theme.colors = vec![Color(10, 20, 30, 0)];
    let contrast = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    for hidden in [false, true] {
        for border_enabled in [false, true] {
            let mut o = BarOptions {
                border: border_enabled.then(|| border(Color(9, 8, 7, 6))),
                border_color_profile: Some(contrast.clone()),
                value_domain: Some([2., 4.]),
                ..Default::default()
            };
            if hidden {
                o.hidden.insert("s".into());
            }
            let source = [BarSeries::new("s", vec![Some(0.)])];
            let mut s = style();
            s.colors.clear();
            s.series_order = None;
            s.labels.enabled = false;
            assert!(
                bar(&c, &["A".into()], &source, &o)
                    .unwrap_err()
                    .to_string()
                    .contains("opaque")
            );
            let calls = RefCell::new(0);
            let mut text = |ctx: &BarLabelContext<'_>| {
                *calls.borrow_mut() += 1;
                Ok(ctx.default_text.to_owned())
            };
            let mut geometry = |ctx: &BarDatumContext<'_>| {
                *calls.borrow_mut() += 1;
                Ok(ctx.inherit(ctx.geometry.clone()))
            };
            assert!(
                bar_layout_with_factories(
                    &c,
                    &["A".into()],
                    &source,
                    &o,
                    &s,
                    BarFactories {
                        bar: Some(&mut geometry),
                        label: Some(&mut text)
                    }
                )
                .unwrap_err()
                .to_string()
                .contains("opaque")
            );
            assert_eq!(*calls.borrow(), 0);
            let absent = [BarSeries::new("s", vec![None])];
            assert!(bar(&c, &["A".into()], &absent, &o).is_ok());
            assert!(bar_layout_with_style(&c, &["A".into()], &absent, &o, &s).is_ok());
        }
    }
    Ok(())
}

#[cfg(feature = "geojson")]
#[test]
fn pinned_original_nivo_role_results_and_alpha_zero_boundary_reach_border() -> Result<()> {
    let rows: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/bar-label-color/values.json")).unwrap();
    let mut ordinary = 0;
    let mut boundary = 0;
    for row in rows.as_array().unwrap() {
        let bytes = |v: &serde_json::Value| {
            Color(
                v[0].as_u64().unwrap() as u8,
                v[1].as_u64().unwrap() as u8,
                v[2].as_u64().unwrap() as u8,
                v[3].as_u64().unwrap() as u8,
            )
        };
        let modifiers = row["modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|m| {
                let a = m[1].as_f64().unwrap();
                match m[0].as_str().unwrap() {
                    "darker" => ColorModifier::Darker(a),
                    "brighter" => ColorModifier::Brighter(a),
                    _ => ColorModifier::Opacity(a),
                }
            })
            .collect();
        let s = BarStyleOptions {
            colors: [("s".into(), bytes(&row["input"]))].into(),
            ..Default::default()
        };
        let o = BarOptions {
            border: Some(border(Color(1, 2, 3, 4))),
            border_color_profile: Some(InheritedColor::FromContext(modifiers)),
            ..Default::default()
        };
        let frame = bar_layout_with_style(
            &config(),
            &["A".into()],
            &[BarSeries::new("s", vec![Some(2.)])],
            &o,
            &s,
        )?;
        let stroke = mark(&frame.scene, &frame.bars[0].id).stroke.unwrap();
        assert_eq!(stroke.color, bytes(&row["nativeTypedRGBA"]));
        if row["profile"] == "ordinary-exact" {
            assert_eq!(stroke.color, bytes(&row["nativeNivoCSS"]));
            ordinary += 1;
        } else {
            assert_ne!(row["nativeTypedRGBA"], row["nativeNivoCSS"]);
            boundary += 1;
        }
    }
    assert_eq!((ordinary, boundary), (36, 6));
    Ok(())
}
