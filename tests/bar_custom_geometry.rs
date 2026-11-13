use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 25.,
            bottom: 25.,
        },
        ..Default::default()
    }
}
fn cats() -> Vec<String> {
    ["A", "B", "C"].map(str::to_string).to_vec()
}
fn data() -> Vec<BarSeries> {
    vec![
        BarSeries::new("z", vec![Some(0.), None, Some(-3.)]),
        BarSeries::new("a", vec![Some(2.), Some(4.), Some(6.)]),
    ]
}
fn path(x: f64, y: f64) -> Geometry {
    Geometry::Path(vec![
        C::Move(P::new(x + 12., y - 8.)),
        C::Line(P::new(x + 28., y - 8.)),
        C::Line(P::new(x + 28., y + 8.)),
        C::Line(P::new(x + 12., y + 8.)),
        C::Close,
    ])
}
#[test]
fn inherited_factories_preserve_default_scene_source_and_paint_exactly() -> Result<()> {
    for horizontal in [false, true] {
        for mode in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            for configured in [false, true] {
                for window in [None, Some([0.15, 0.85])] {
                    for corner_radius in [0., 8.] {
                        let o = BarOptions {
                            horizontal,
                            layout: mode,
                            category_window: window,
                            corner_radius,
                            ..Default::default()
                        };
                        let style = BarStyleOptions {
                            category_scale: configured.then(|| scales::BandScaleOptions {
                                round: true,
                                padding_inner: 0.2,
                                padding_outer: 0.2,
                                ..Default::default()
                            }),
                            labels: BarLabels {
                                enabled: true,
                                ..Default::default()
                            },
                            ..Default::default()
                        };
                        let plain = bar_layout_with_style(&config(), &cats(), &data(), &o, &style)?;
                        let mut calls = Vec::new();
                        let custom = bar_layout_with_bar_factory(
                            &config(),
                            &cats(),
                            &data(),
                            &o,
                            &style,
                            |c| {
                                assert_eq!(c.series.values[c.source_category], Some(c.raw_value));
                                assert_eq!(c.series.id, data()[c.source_series].id);
                                assert_eq!(c.category, cats()[c.source_category]);
                                assert_eq!(c.horizontal, horizontal);
                                calls.push(c.mark_id.to_string());
                                Ok(c.inherit(c.geometry.clone()))
                            },
                        )?;
                        assert_eq!(plain, custom.layout);
                        assert_eq!(calls.len(), plain.bars.len());
                        let binding = custom.interaction_binding()?;
                        assert_eq!(binding.sites.len(), plain.bars.len());
                        assert!(
                            !plain
                                .bars
                                .iter()
                                .any(|b| b.series == "z" && b.category == "B")
                        );
                        assert!(binding.sites.iter().any(|s| s.value == Some(0.)));
                        assert_eq!(
                            render::to_svg(&plain.scene)?,
                            render::to_svg(&custom.layout.scene)?
                        );
                        assert_eq!(
                            render::to_canvas_script(&plain.scene, "same")?,
                            render::to_canvas_script(&custom.layout.scene, "same")?
                        );
                    }
                }
            }
        }
    }
    let style = BarStyleOptions::default();
    let scaled = bar_layout_with_scale_options_and_bar_factory(
        &config(),
        &cats(),
        &data(),
        &Default::default(),
        &style,
        &Default::default(),
        |c| Ok(c.inherit(c.geometry.clone())),
    )?;
    scaled.layout.interaction_binding()?;
    Ok(())
}
#[test]
fn actual_offset_ink_keeps_logical_sites_and_rebuilds_on_resize_hide() -> Result<()> {
    let source = [BarSeries::new("s", vec![Some(4.)])];
    let categories = ["A".to_string()];
    let o = BarOptions::default();
    let make = |cfg: &ChartConfig, o: &BarOptions| {
        bar_layout_with_bar_factory(cfg, &categories, &source, o, &Default::default(), |c| {
            Ok(c.inherit(path(c.position[0], c.position[1])))
        })
    };
    let a = make(&config(), &o)?;
    let site = &a.interaction_binding()?.sites[0];
    let [x, y] = site.position;
    assert!(
        a.layout
            .scene
            .hit_test(x, y)
            .is_none_or(|m| m.id != site.mark_id)
    );
    assert_eq!(
        a.layout.scene.hit_test(x + 20., y).unwrap().id,
        site.mark_id
    );
    assert_eq!(site.value, Some(4.));
    let larger = ChartConfig {
        width: 480.,
        ..config()
    };
    let b = make(&larger, &o)?;
    let bs = b.interaction_binding()?;
    assert_eq!(site.mark_id, bs.sites[0].mark_id);
    assert_ne!(site.position, bs.sites[0].position);
    let mut hidden = o.clone();
    hidden.hidden.insert("s".into());
    let h = make(&config(), &hidden)?;
    assert!(h.interaction_binding()?.sites.is_empty());
    Ok(())
}
#[test]
fn hover_retains_custom_paint_owned_label_alpha_and_private_provenance() -> Result<()> {
    let style = BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            clip_to_plot: true,
            style: BarTextStyle {
                color: Some(Color(30, 40, 50, 137)),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut a = bar_layout_with_bar_factory(
        &config(),
        &cats(),
        &data(),
        &Default::default(),
        &style,
        |c| {
            let mut node = c.inherit(c.geometry.clone());
            node.opacity = 0.6;
            node.fill_paint = Some(FillPaint::Linear {
                start: [c.position[0] - 10., c.position[1]],
                end: [c.position[0] + 10., c.position[1]],
                stops: vec![
                    GradientStop::new(0., Color(230, 50, 20, 255)),
                    GradientStop::new(1., Color(20, 70, 220, 255)),
                ],
            });
            node.stroke = Some(GraphicStroke {
                color: Color(5, 10, 15, 255),
                width: 2.,
                cap: StrokeCap::Round,
            });
            node.stroke_dash = Some(StrokeDash {
                segments: vec![3., 2.],
                offset: 0.,
            });
            Ok(node)
        },
    )?;
    let before = a.layout.clone();
    let sites = a.interaction_binding()?.sites;
    let active = sites[0].mark_id.clone();
    a.set_active(Some(&active), 0.25)?;
    for b in &a.layout.bars {
        let mark = a.layout.scene.marks.iter().find(|m| m.id == b.id).unwrap();
        assert_eq!(mark.opacity, if b.id == active { 0.6 } else { 0.15 });
        if let Some(index) = b.label_index {
            assert_eq!(
                a.layout.scene.labels[index].color.3,
                (137. * mark.opacity).round() as u8
            );
            assert!(a.layout.scene.label_clips.contains_key(&index));
        }
    }
    a.interaction_binding()?;
    a.set_active(None, 0.25)?;
    assert_eq!(before, a.layout);
    for mutation in 0..8 {
        let mut bad = a.clone();
        match mutation {
            0 => bad.layout.bars[0].raw_value = 999.,
            1 => bad.layout.bars[0].source_series = 999,
            2 => bad.layout.scene.marks[0].opacity = 0.3,
            3 => bad.layout.scene.labels[0].text = "forged".into(),
            4 => {
                bad.layout
                    .scene
                    .marks
                    .iter_mut()
                    .find(|m| m.id == active)
                    .unwrap()
                    .fill_paint = None
            }
            5 => {
                let index = bad.layout.bars.iter().find_map(|b| b.label_index).unwrap();
                bad.layout.scene.label_series.remove(&index);
            }
            6 => {
                let index = bad.layout.bars.iter().find_map(|b| b.label_index).unwrap();
                bad.layout.scene.label_clips.remove(&index);
            }
            _ => bad.layout.scene.draw_order = vec![DrawItem::Mark(0)],
        };
        let frame = bad.layout.clone();
        assert!(bad.interaction_binding().is_err());
        assert!(bad.set_active(Some(&active), 0.25).is_err());
        assert_eq!(frame, bad.layout);
    }
    assert!(a.set_active(Some("missing"), 0.5).is_err());
    assert_eq!(a.layout, before);
    Ok(())
}
#[test]
fn bad_callbacks_geometry_paint_and_aggregate_budgets_reject() -> Result<()> {
    let source = data();
    let backup = source.clone();
    for kind in 0..7 {
        let result = bar_layout_with_bar_factory(
            &config(),
            &cats(),
            &source,
            &Default::default(),
            &Default::default(),
            |c| {
                let mut g = c.inherit(c.geometry.clone());
                match kind {
                    0 => return Err(ChartError("application rejected bar".into())),
                    1 => {
                        g.geometry = Geometry::Circle {
                            x: f64::NAN,
                            y: 0.,
                            radius: 2.,
                        }
                    }
                    2 => g.geometry = Geometry::Path(vec![C::Line(P::new(1., 1.))]),
                    3 => g.opacity = 1.1,
                    4 => {
                        g.stroke_dash = Some(StrokeDash {
                            segments: vec![-1.],
                            offset: 0.,
                        })
                    }
                    5 => {
                        g.fill_paint = Some(FillPaint::Linear {
                            start: [0., 0.],
                            end: [0., 0.],
                            stops: vec![],
                        })
                    }
                    _ => {
                        g.geometry = Geometry::Circle {
                            x: 0.,
                            y: 0.,
                            radius: 2.,
                        }
                    }
                }
                Ok(g)
            },
        );
        assert!(result.is_err(), "{kind}");
        assert_eq!(source, backup);
    }
    let categories = (0..40).map(|i| format!("C{i}")).collect::<Vec<_>>();
    let source = [BarSeries::new("s", vec![Some(1.); 40])];
    let mut calls = 0;
    let result = bar_layout_with_bar_factory(
        &config(),
        &categories,
        &source,
        &Default::default(),
        &Default::default(),
        |c| {
            calls += 1;
            let mut g = c.inherit(c.geometry.clone());
            g.geometry = Geometry::Path(
                std::iter::once(C::Move(P::new(c.position[0], c.position[1])))
                    .chain(std::iter::repeat_n(
                        C::Line(P::new(c.position[0], c.position[1])),
                        6000,
                    ))
                    .collect(),
            );
            Ok(g)
        },
    );
    assert!(result.is_err());
    assert!(calls < 40);
    let result = bar_layout_with_bar_factory(
        &config(),
        &categories,
        &source,
        &Default::default(),
        &Default::default(),
        |c| {
            let mut g = c.inherit(c.geometry.clone());
            g.fill_paint = Some(FillPaint::Linear {
                start: [0., 0.],
                end: [1., 0.],
                stops: (0..512)
                    .map(|i| GradientStop::new(i as f64 / 511., Color(5, 10, 15, 255)))
                    .collect(),
            });
            Ok(g)
        },
    );
    assert!(result.is_err());
    Ok(())
}

#[test]
fn hollow_circle_and_unrepresentable_uncropped_endpoints_keep_typed_source() -> Result<()> {
    let categories = ["A".to_string()];
    let data = [BarSeries::new("s", vec![Some(2.)])];
    let l = bar_layout_with_bar_factory(
        &config(),
        &categories,
        &data,
        &Default::default(),
        &Default::default(),
        |c| {
            let mut g = c.inherit(Geometry::Circle {
                x: c.position[0],
                y: c.position[1],
                radius: 12.,
            });
            g.fill = None;
            g.stroke = Some(GraphicStroke {
                color: Color(180, 20, 30, 255),
                width: 3.,
                cap: StrokeCap::Round,
            });
            Ok(g)
        },
    )?;
    let b = l.interaction_binding()?;
    let s = &b.sites[0];
    assert!(
        l.layout
            .scene
            .hit_test(s.position[0], s.position[1])
            .is_none_or(|m| m.id != s.mark_id)
    );
    assert_eq!(
        l.layout
            .scene
            .hit_test(s.position[0] + 12., s.position[1])
            .unwrap()
            .id,
        s.mark_id
    );
    let large = [BarSeries::new("s", vec![Some(f64::MAX)])];
    let o = BarOptions {
        value_domain: Some([0., 1.]),
        ..Default::default()
    };
    let mut observed = false;
    let result = bar_layout_with_bar_factory(
        &config(),
        &categories,
        &large,
        &o,
        &Default::default(),
        |c| {
            observed = c.endpoints[1].is_none() && c.original_rect.is_none();
            Ok(c.inherit(c.geometry.clone()))
        },
    )?;
    assert!(observed);
    assert_eq!(result.interaction_binding()?.sites[0].value, Some(f64::MAX));
    Ok(())
}

#[test]
fn dash_expansion_is_admitted_before_another_source_callback() {
    let mut calls = 0;
    let result = bar_layout_with_bar_factory(
        &config(),
        &cats(),
        &data(),
        &Default::default(),
        &Default::default(),
        |c| {
            calls += 1;
            let mut g = c.inherit(Geometry::Path(vec![
                C::Move(P::new(0., c.position[1])),
                C::Line(P::new(999_999., c.position[1])),
            ]));
            g.stroke = Some(GraphicStroke {
                color: Color(10, 20, 30, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            });
            g.stroke_dash = Some(StrokeDash {
                segments: vec![0.001, 0.001],
                offset: 0.,
            });
            Ok(g)
        },
    );
    assert!(result.is_err());
    assert_eq!(calls, 1);
}
