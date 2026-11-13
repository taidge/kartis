use kartis::*;
use std::collections::BTreeMap;

#[cfg(feature = "geojson")]
#[test]
fn original_nivo_eligibility_and_label_layout_match_on_checked_category_profile() -> Result<()> {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "fixtures/bar-zero-extent-label/nivo-original.json"
    ))
    .unwrap();
    let cfg = ChartConfig {
        width: 40.,
        height: 40.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    };
    let mut matched = 0;
    let mut native_outside_checked = 0;
    let mut native_anchor_outside_plot = 0;
    for row in fixture["records"].as_array().unwrap() {
        let horizontal = row["horizontal"].as_bool().unwrap();
        let width = row["width"].as_f64().unwrap();
        let height = row["height"].as_f64().unwrap();
        if if horizontal {
            height == 0.
        } else {
            width == 0.
        } {
            native_outside_checked += 1;
            continue;
        }
        let value = if if horizontal {
            width == 0.
        } else {
            height == 0.
        } {
            0.
        } else {
            1.
        };
        let mut s = style();
        s.labels.enabled = row["enabled"].as_bool().unwrap();
        s.labels.skip_width = row["labelSkipWidth"].as_f64().unwrap();
        s.labels.skip_height = row["labelSkipHeight"].as_f64().unwrap();
        s.labels.offset = 3.;
        s.labels.position = match row["position"].as_str().unwrap() {
            "start" => BarLabelPosition::Start,
            "end" => BarLabelPosition::End,
            _ => BarLabelPosition::Middle,
        };
        let options = BarOptions {
            horizontal,
            padding: 0.,
            value_domain: Some([0., 1.]),
            ..Default::default()
        };
        let frame = bar_layout_with_style(
            &cfg,
            &["A".into()],
            &[BarSeries::new("s", vec![Some(value)])],
            &options,
            &s,
        )?;
        let b = &frame.bars[0];
        assert_eq!(
            [b.visible_rect.width, b.visible_rect.height],
            [width, height]
        );
        let x = b.visible_rect.x + row["nativePosition"]["labelX"].as_f64().unwrap();
        let y = b.visible_rect.y + row["nativePosition"]["labelY"].as_f64().unwrap();
        let native = row["nativeShouldRender"].as_bool().unwrap();
        let in_plot = ClipRect::plot(&cfg).contains(x, y);
        if native && !in_plot {
            native_anchor_outside_plot += 1;
        }
        assert_eq!(b.label_index.is_some(), native && in_plot, "{row}");
        if let Some(i) = b.label_index {
            let label = &frame.scene.labels[i];
            assert_eq!(
                label.x,
                b.visible_rect.x + row["nativePosition"]["labelX"].as_f64().unwrap()
            );
            assert_eq!(
                label.y,
                b.visible_rect.y + row["nativePosition"]["labelY"].as_f64().unwrap()
            );
        }
        matched += 1;
    }
    assert_eq!((matched, native_outside_checked), (48, 48));
    assert_eq!(native_anchor_outside_plot, 4);
    Ok(())
}

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        ..Default::default()
    }
}
fn style() -> BarStyleOptions {
    BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            zero_extent: BarZeroExtentLabels::ValueAxis,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn source() -> Vec<BarSeries> {
    vec![
        BarSeries::new("a", vec![Some(2.)]),
        BarSeries::new("z", vec![Some(0.)]),
        BarSeries::new("missing", vec![None]),
    ]
}
fn zero(layout: &StyledBarLayout) -> &StyledBarDatum {
    layout.bars.iter().find(|b| b.series == "z").unwrap()
}

#[test]
fn actual_zero_matrix_preserves_marks_and_uses_original_anchors() -> Result<()> {
    let mut cases = 0;
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            for corner_radius in [0., 8.] {
                for band in [false, true] {
                    for position in [
                        BarLabelPosition::Start,
                        BarLabelPosition::Middle,
                        BarLabelPosition::End,
                    ] {
                        let options = BarOptions {
                            horizontal,
                            layout,
                            corner_radius,
                            value_domain: Some(if layout == BarLayout::Percent {
                                [-2., 2.]
                            } else {
                                [-5., 5.]
                            }),
                            ..Default::default()
                        };
                        let mut s = style();
                        s.category_scale = band.then(|| scales::BandScaleOptions {
                            round: true,
                            ..Default::default()
                        });
                        s.labels.position = position;
                        s.labels.offset = 3.;
                        let candidate = bar_layout_with_scale_options(
                            &config(),
                            &["A".into()],
                            &source(),
                            &options,
                            &s,
                            &Default::default(),
                        )?;
                        s.labels.zero_extent = BarZeroExtentLabels::Skip;
                        let old = bar_layout_with_scale_options(
                            &config(),
                            &["A".into()],
                            &source(),
                            &options,
                            &s,
                            &Default::default(),
                        )?;
                        assert_eq!(candidate.layout.scene.marks, old.layout.scene.marks);
                        assert_eq!(candidate.layout.bars.len(), 2);
                        assert!(zero(&old.layout).label_index.is_none());
                        let b = zero(&candidate.layout);
                        let label = &candidate.layout.scene.labels[b.label_index.unwrap()];
                        assert_eq!(b.raw_value, 0.);
                        assert_eq!(b.interval[0], b.interval[1]);
                        if layout != BarLayout::Grouped {
                            assert_ne!(b.interval[0], 0.);
                        }
                        let value = candidate.value_scale.map(b.interval[0]);
                        let category = b.band[0] + b.band[1] / 2.;
                        assert_eq!(
                            [label.x, label.y],
                            if horizontal {
                                [value + 3., category]
                            } else {
                                [category, value - 3.]
                            }
                        );
                        assert_eq!(label.text, "0");
                        assert_eq!(
                            candidate.layout.scene.label_series[&b.label_index.unwrap()],
                            "z"
                        );
                        assert!(
                            candidate
                                .layout
                                .scene
                                .label_clips
                                .contains_key(&b.label_index.unwrap())
                        );
                        candidate.layout.scene.validate()?;
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 72);
    Ok(())
}

#[test]
fn skip_dimensions_remain_real_thresholds() -> Result<()> {
    for horizontal in [false, true] {
        let options = BarOptions {
            horizontal,
            value_domain: Some([-2., 2.]),
            ..Default::default()
        };
        for width in [0., 0.001] {
            for height in [0., 0.001] {
                let mut s = style();
                s.labels.skip_width = width;
                s.labels.skip_height = height;
                let frame = bar_layout_with_style(
                    &config(),
                    &["A".into()],
                    &[BarSeries::new("z", vec![Some(0.)])],
                    &options,
                    &s,
                )?;
                assert_eq!(
                    zero(&frame).label_index.is_some(),
                    if horizontal {
                        width == 0.
                    } else {
                        height == 0.
                    }
                );
            }
        }
    }
    Ok(())
}

#[test]
fn original_scale_round_collapse_is_eligible_without_forging_raw_zero() -> Result<()> {
    for horizontal in [false, true] {
        let options = BarOptions {
            horizontal,
            value_domain: Some([-10., 10.]),
            ..Default::default()
        };
        let data = [BarSeries::new("tiny", vec![Some(0.000001)])];
        let rounded = bar_layout_with_scale_options(
            &config(),
            &["A".into()],
            &data,
            &options,
            &style(),
            &ContinuousScaleOptions {
                round: true,
                ..Default::default()
            },
        )?;
        let b = &rounded.layout.bars[0];
        assert_eq!(b.raw_value, 0.000001);
        assert!(b.label_index.is_some());
        assert_eq!(
            if horizontal {
                b.visible_rect.width
            } else {
                b.visible_rect.height
            },
            0.
        );
        let mut skip = style();
        skip.labels.zero_extent = BarZeroExtentLabels::Skip;
        let old = bar_layout_with_scale_options(
            &config(),
            &["A".into()],
            &data,
            &options,
            &skip,
            &ContinuousScaleOptions {
                round: true,
                ..Default::default()
            },
        )?;
        assert!(old.layout.bars[0].label_index.is_none());
        let unrounded = bar_layout_with_scale_options(
            &config(),
            &["A".into()],
            &data,
            &options,
            &skip,
            &Default::default(),
        )?;
        assert!(unrounded.layout.bars[0].label_index.is_some());
    }
    Ok(())
}

#[test]
fn missing_hidden_crop_and_zero_category_breadth_do_not_gain_labels() -> Result<()> {
    for horizontal in [false, true] {
        for kind in 0..5 {
            let mut options = BarOptions {
                horizontal,
                value_domain: Some([-5., 5.]),
                ..Default::default()
            };
            let mut s = style();
            let mut data = vec![BarSeries::new("z", vec![Some(0.)])];
            match kind {
                0 => data[0].values[0] = None,
                1 => {
                    options.hidden.insert("z".into());
                }
                2 => options.value_domain = Some([1., 2.]),
                3 => {
                    s.category_scale = Some(scales::BandScaleOptions {
                        padding_inner: 1.,
                        ..Default::default()
                    })
                }
                _ => s.labels.offset = 1000.,
            }
            let frame = bar_layout_with_style(&config(), &["A".into()], &data, &options, &s)?;
            assert!(frame.bars.iter().all(|b| b.label_index.is_none()));
            if kind < 4 {
                assert!(frame.bars.is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn partial_category_crop_retains_uncropped_anchor_and_cannot_revive_it_outside_plot() -> Result<()>
{
    let categories = ["A".into(), "B".into(), "C".into()];
    let data = [BarSeries::new("z", vec![Some(0.); 3])];
    for horizontal in [false, true] {
        let options = BarOptions {
            horizontal,
            category_window: Some([0.2, 0.8]),
            value_domain: Some([-1., 1.]),
            ..Default::default()
        };
        let frame = bar_layout_with_style(&config(), &categories, &data, &options, &style())?;
        for b in &frame.bars {
            let original = b.band[0] + b.band[1] / 2.;
            let plot = ClipRect::plot(&config());
            let category_in_plot = if horizontal {
                original >= plot.y && original <= plot.y + plot.height
            } else {
                original >= plot.x && original <= plot.x + plot.width
            };
            assert_eq!(b.label_index.is_some(), category_in_plot);
            if let Some(i) = b.label_index {
                let label = &frame.scene.labels[i];
                assert_eq!(if horizontal { label.y } else { label.x }, original);
            }
        }
    }
    Ok(())
}

#[test]
fn custom_zero_placeholder_keeps_typed_sites_role_alpha_hover_and_freeze() -> Result<()> {
    let mut s = style();
    s.colors = BTreeMap::from([
        ("z".into(), Color(20, 40, 60, 160)),
        ("other".into(), Color(20, 40, 60, 160)),
    ]);
    s.labels.color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    let data = [
        BarSeries::new("z", vec![Some(0.)]),
        BarSeries::new("other", vec![Some(0.)]),
    ];
    for horizontal in [false, true] {
        for corner_radius in [0., 8.] {
            let options = BarOptions {
                horizontal,
                corner_radius,
                value_domain: Some([-5., 5.]),
                ..Default::default()
            };
            let mut calls = Vec::new();
            let mut custom =
                bar_layout_with_bar_factory(&config(), &["A".into()], &data, &options, &s, |c| {
                    calls.push((c.mark_id.to_string(), c.position, c.raw_value));
                    let mut node = c.inherit(c.geometry.clone());
                    node.fill = Some(Color(240, 30, 180, 255));
                    node.opacity = 0.5;
                    Ok(node)
                })?;
            let base = custom.layout.clone();
            let binding = custom.interaction_binding()?;
            assert_eq!(binding.sites.len(), 2);
            for site in &binding.sites {
                let call = calls.iter().find(|v| v.0 == site.mark_id).unwrap();
                assert_eq!(site.position, call.1);
                assert_eq!(site.value, Some(0.));
            }
            for b in &base.bars {
                assert_eq!(
                    base.scene.labels[b.label_index.unwrap()].color,
                    Color(14, 28, 42, 80)
                );
            }
            let active = base.bars[0].id.clone();
            custom.set_active(Some(&active), 0.2)?;
            for b in &custom.layout.bars {
                assert_eq!(
                    custom.layout.scene.labels[b.label_index.unwrap()].color,
                    Color(14, 28, 42, if b.id == active { 80 } else { 16 })
                );
            }
            custom.set_active(None, 0.2)?;
            assert_eq!(custom.layout, base);
            for mutation in 0..4 {
                let mut bad = custom.clone();
                let i = bad.layout.bars[0].label_index.unwrap();
                match mutation {
                    0 => bad.layout.scene.labels[i].text = "forged".into(),
                    1 => {
                        bad.layout.scene.label_series.remove(&i);
                    }
                    2 => {
                        bad.layout.scene.label_clips.remove(&i);
                    }
                    _ => bad.layout.bars[0].raw_value = 10.,
                }
                let before = bad.layout.clone();
                assert!(bad.interaction_binding().is_err());
                assert!(bad.set_active(Some(&active), 0.2).is_err());
                assert_eq!(before, bad.layout);
            }
            let before = custom.layout.clone();
            assert!(custom.set_active(Some("absent"), 0.2).is_err());
            assert!(custom.set_active(None, f64::NAN).is_err());
            assert_eq!(custom.layout, before);
        }
    }
    Ok(())
}

#[test]
fn invalid_configuration_and_callback_reject_before_publishing() -> Result<()> {
    let data = source();
    let backup = data.clone();
    for kind in 0..4 {
        let mut s = style();
        match kind {
            0 => s.labels.skip_height = f64::NAN,
            1 => s.labels.offset = f64::INFINITY,
            2 => {
                s.labels.color_profile =
                    Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
                        f64::NAN,
                    )]))
            }
            _ => s.labels.style.size = Some(0.),
        }
        let mut called = false;
        assert!(
            bar_layout_with_bar_factory(
                &config(),
                &["A".into()],
                &data,
                &Default::default(),
                &s,
                |c| {
                    called = true;
                    Ok(c.inherit(c.geometry.clone()))
                }
            )
            .is_err()
        );
        assert!(!called);
    }
    let error = bar_layout_with_bar_factory(
        &config(),
        &["A".into()],
        &data,
        &Default::default(),
        &style(),
        |_c| Err(ChartError("application rejected".into())),
    );
    assert!(error.is_err());
    assert_eq!(data, backup);
    Ok(())
}

#[test]
fn zero_custom_ink_picks_raw_source_without_moving_logical_label_or_anchor() -> Result<()> {
    use glory::render::{GraphicPoint as P, PathCommand as C};
    let source = [BarSeries::new("z", vec![Some(0.)])];
    let categories = ["A".into()];
    let options = BarOptions {
        value_domain: Some([-1., 1.]),
        ..Default::default()
    };
    let make = |cfg: &ChartConfig, options: &BarOptions| {
        bar_layout_with_bar_factory(cfg, &categories, &source, options, &style(), |c| {
            let [x, y] = c.position;
            Ok(c.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 12., y - 6.)),
                C::Line(P::new(x + 24., y - 6.)),
                C::Line(P::new(x + 24., y + 6.)),
                C::Line(P::new(x + 12., y + 6.)),
                C::Close,
            ])))
        })
    };
    let frame = make(&config(), &options)?;
    let binding = frame.interaction_binding()?;
    let site = &binding.sites[0];
    let [x, y] = site.position;
    let hit = frame.layout.scene.hit_test(x + 18., y).unwrap();
    assert_eq!(hit.id, site.mark_id);
    assert_eq!(site.value, Some(0.));
    assert_eq!(binding.anchor_for(hit)?, Some([x, y]));
    let label = &frame.layout.scene.labels[frame.layout.bars[0].label_index.unwrap()];
    assert_eq!([label.x, label.y], [x, y]);
    assert!(
        frame
            .layout
            .scene
            .hit_test(x, y)
            .is_none_or(|m| m.id != site.mark_id)
    );
    let resized = make(
        &ChartConfig {
            width: 480.,
            ..config()
        },
        &options,
    )?;
    let new_binding = resized.interaction_binding()?;
    assert_eq!(new_binding.sites[0].mark_id, site.mark_id);
    assert_ne!(new_binding.sites[0].position, site.position);
    let mut hidden = options;
    hidden.hidden.insert("z".into());
    assert!(
        make(&config(), &hidden)?
            .interaction_binding()?
            .sites
            .is_empty()
    );
    Ok(())
}

#[test]
fn all_zero_percent_uses_existing_formatter_and_does_not_promote_none() -> Result<()> {
    for horizontal in [false, true] {
        let categories = ["A".into(), "B".into()];
        let source = [BarSeries::new("z", vec![Some(0.), None])];
        let mut s = style();
        s.labels.offset = 3.;
        s.labels.value = BarLabelValue::Displayed;
        s.labels.format = ValueFormat::Fixed(2);
        let frame = bar_layout_with_style(
            &config(),
            &categories,
            &source,
            &BarOptions {
                horizontal,
                layout: BarLayout::Percent,
                ..Default::default()
            },
            &s,
        )?;
        assert_eq!(frame.bars.len(), 1);
        let b = &frame.bars[0];
        assert_eq!(b.interval, [0., 0.]);
        assert_eq!(b.raw_value, 0.);
        assert_eq!(frame.scene.labels[b.label_index.unwrap()].text, "0.00");
    }
    Ok(())
}
