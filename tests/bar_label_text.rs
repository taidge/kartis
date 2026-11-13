use std::cell::RefCell;
use std::collections::BTreeMap;

use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
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
fn style(zero: bool) -> BarStyleOptions {
    BarStyleOptions {
        labels: BarLabels {
            enabled: true,
            zero_extent: if zero {
                BarZeroExtentLabels::ValueAxis
            } else {
                BarZeroExtentLabels::Skip
            },
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn empty_and_identity_factories_preserve_complete_layout_paint_and_old_contexts() -> Result<()> {
    let mut count = 0;
    for horizontal in [false, true] {
        for mode in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            for corner_radius in [0., 8.] {
                for window in [None, Some([0.15, 0.85])] {
                    for band in [false, true] {
                        for zero in [false, true] {
                            let options = BarOptions {
                                horizontal,
                                layout: mode,
                                corner_radius,
                                category_window: window,
                                ..Default::default()
                            };
                            let mut s = style(zero);
                            s.category_scale = band.then(|| scales::BandScaleOptions {
                                round: true,
                                padding_inner: 0.2,
                                ..Default::default()
                            });
                            let plain =
                                bar_layout_with_style(&config(), &cats(), &data(), &options, &s)?;
                            let empty = bar_layout_with_factories(
                                &config(),
                                &cats(),
                                &data(),
                                &options,
                                &s,
                                BarFactories::default(),
                            )?;
                            assert_eq!(plain, empty.layout);
                            let mut old_contexts = Vec::new();
                            let old = bar_layout_with_bar_factory(
                                &config(),
                                &cats(),
                                &data(),
                                &options,
                                &s,
                                |c| {
                                    old_contexts.push(format!("{c:?}"));
                                    Ok(c.inherit(c.geometry.clone()))
                                },
                            )?;
                            let mut new_contexts = Vec::new();
                            let events = RefCell::new(Vec::new());
                            let mut text = |c: &BarLabelContext<'_>| {
                                events.borrow_mut().push(format!("label:{}", c.mark_id));
                                assert_eq!(c.series.values[c.source_category], Some(c.raw_value));
                                assert_eq!(c.category, cats()[c.source_category]);
                                assert_eq!(c.series.id, data()[c.source_series].id);
                                Ok(c.default_text.to_string())
                            };
                            let mut bar = |c: &BarDatumContext<'_>| {
                                events.borrow_mut().push(format!("bar:{}", c.mark_id));
                                new_contexts.push(format!("{c:?}"));
                                Ok(c.inherit(c.geometry.clone()))
                            };
                            let new = bar_layout_with_factories(
                                &config(),
                                &cats(),
                                &data(),
                                &options,
                                &s,
                                BarFactories {
                                    bar: Some(&mut bar),
                                    label: Some(&mut text),
                                },
                            )?;
                            assert_eq!(new.layout, old.layout);
                            assert_eq!(new_contexts, old_contexts);
                            let expected = plain
                                .bars
                                .iter()
                                .flat_map(|b| {
                                    let mut v = Vec::new();
                                    if b.label_index.is_some() {
                                        v.push(format!("label:{}", b.id));
                                    }
                                    v.push(format!("bar:{}", b.id));
                                    v
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(*events.borrow(), expected);
                            assert_eq!(
                                render::to_svg(&plain.scene)?,
                                render::to_svg(&new.layout.scene)?
                            );
                            assert_eq!(
                                render::to_canvas_script(&plain.scene, "same")?,
                                render::to_canvas_script(&new.layout.scene, "same")?
                            );
                            new.interaction_binding()?;
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(count, 96);
    Ok(())
}

#[test]
fn configured_scale_contexts_match_legacy_for_reverse_round_clamp_nonlinear_and_window()
-> Result<()> {
    let mut cases = 0;
    for horizontal in [false, true] {
        for kind in [
            ScaleKind::Linear,
            ScaleKind::SymLog { constant: 1. },
            ScaleKind::Sqrt,
            ScaleKind::Pow { exponent: 2. },
        ] {
            for reverse in [false, true] {
                for round in [false, true] {
                    for clamp in [false, true] {
                        let options = BarOptions {
                            horizontal,
                            corner_radius: 8.,
                            category_window: Some([0.2, 0.8]),
                            value_domain: Some([-2., 4.]),
                            ..Default::default()
                        };
                        let scale = ContinuousScaleOptions {
                            kind,
                            reverse,
                            round,
                            clamp,
                            ..Default::default()
                        };
                        let mut old_contexts = Vec::new();
                        let old = bar_layout_with_scale_options_and_bar_factory(
                            &config(),
                            &cats(),
                            &data(),
                            &options,
                            &style(true),
                            &scale,
                            |c| {
                                old_contexts.push(format!("{c:?}"));
                                Ok(c.inherit(c.geometry.clone()))
                            },
                        )?;
                        let mut contexts = Vec::new();
                        let mut text = |c: &BarLabelContext<'_>| Ok(c.default_text.to_string());
                        let mut bar = |c: &BarDatumContext<'_>| {
                            contexts.push(format!("{c:?}"));
                            Ok(c.inherit(c.geometry.clone()))
                        };
                        let new = bar_layout_with_scale_options_and_factories(
                            &config(),
                            &cats(),
                            &data(),
                            &options,
                            &style(true),
                            &scale,
                            BarFactories {
                                bar: Some(&mut bar),
                                label: Some(&mut text),
                            },
                        )?;
                        assert_eq!(contexts, old_contexts);
                        assert_eq!(new.layout.layout, old.layout.layout);
                        assert_eq!(
                            new.value_scale.display_domain(),
                            old.value_scale.display_domain()
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert_eq!(cases, 64);
    Ok(())
}

#[test]
fn rounded_zero_placeholder_uses_authored_center_and_nonrepresentable_endpoints_stay_none()
-> Result<()> {
    let categories = cats();
    let zero = [BarSeries::new("z", vec![Some(0.); 3])];
    let options = BarOptions {
        corner_radius: 8.,
        category_window: Some([0.2, 0.8]),
        value_domain: Some([-1., 1.]),
        ..Default::default()
    };
    let mut authored_witness = false;
    let mut contexts = Vec::new();
    let old =
        bar_layout_with_bar_factory(&config(), &categories, &zero, &options, &style(true), |c| {
            contexts.push(format!("{c:?}"));
            authored_witness |= c.position[0] != c.visible_rect.x + c.visible_rect.width / 2.;
            Ok(c.inherit(c.geometry.clone()))
        })?;
    assert!(authored_witness);
    let mut new_contexts = Vec::new();
    let mut text = |c: &BarLabelContext<'_>| Ok(c.default_text.to_string());
    let mut bar = |c: &BarDatumContext<'_>| {
        new_contexts.push(format!("{c:?}"));
        Ok(c.inherit(c.geometry.clone()))
    };
    let new = bar_layout_with_factories(
        &config(),
        &categories,
        &zero,
        &options,
        &style(true),
        BarFactories {
            bar: Some(&mut bar),
            label: Some(&mut text),
        },
    )?;
    assert_eq!(new_contexts, contexts);
    assert_eq!(old.layout, new.layout);
    let large = [BarSeries::new("huge", vec![Some(1e308)])];
    let options = BarOptions {
        value_domain: Some([-1e-20, 1e-20]),
        ..Default::default()
    };
    let mut old_context = Vec::new();
    bar_layout_with_bar_factory(
        &config(),
        &["A".into()],
        &large,
        &options,
        &style(true),
        |c| {
            assert!(c.endpoints[1].is_none());
            assert!(c.original_rect.is_none());
            old_context.push(format!("{c:?}"));
            Ok(c.inherit(c.geometry.clone()))
        },
    )?;
    let mut new_context = Vec::new();
    let mut label_calls = 0;
    let mut text = |c: &BarLabelContext<'_>| {
        label_calls += 1;
        Ok(c.default_text.to_string())
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        new_context.push(format!("{c:?}"));
        Ok(c.inherit(c.geometry.clone()))
    };
    bar_layout_with_factories(
        &config(),
        &["A".into()],
        &large,
        &options,
        &style(true),
        BarFactories {
            bar: Some(&mut bar),
            label: Some(&mut text),
        },
    )?;
    assert_eq!(new_context, old_context);
    assert_eq!(label_calls, 0);
    Ok(())
}

#[test]
fn only_actual_owned_labels_call_text_and_empty_retains_slot() -> Result<()> {
    for kind in 0..7 {
        let mut options = BarOptions {
            value_domain: Some([-5., 5.]),
            ..Default::default()
        };
        let mut s = style(true);
        match kind {
            0 => s.labels.enabled = false,
            1 => {
                options.hidden.insert("z".into());
            }
            2 => options.value_domain = Some([1., 2.]),
            3 => s.labels.skip_height = 1000.,
            4 => s.labels.zero_extent = BarZeroExtentLabels::Skip,
            5 => {
                s.category_scale = Some(scales::BandScaleOptions {
                    padding_inner: 1.,
                    ..Default::default()
                })
            }
            _ => {}
        }
        let source = [BarSeries::new("z", vec![Some(0.), None])];
        let categories = ["A".into(), "B".into()];
        let mut calls = 0;
        let mut text = |_c: &BarLabelContext<'_>| {
            calls += 1;
            Ok(String::new())
        };
        let new = bar_layout_with_factories(
            &config(),
            &categories,
            &source,
            &options,
            &s,
            BarFactories {
                label: Some(&mut text),
                ..Default::default()
            },
        )?;
        assert_eq!(calls, usize::from(kind == 6));
        if kind == 6 {
            let i = new.layout.bars[0].label_index.unwrap();
            assert_eq!(new.layout.scene.labels[i].text, "");
            assert!(new.layout.scene.label_series.contains_key(&i));
            new.interaction_binding()?;
        }
    }
    Ok(())
}

#[test]
fn custom_text_keeps_format_tooltip_palette_alpha_hover_and_private_source() -> Result<()> {
    let mut s = style(true);
    s.colors = BTreeMap::from([
        ("a".into(), Color(20, 40, 60, 160)),
        ("z".into(), Color(20, 40, 60, 160)),
    ]);
    s.labels.color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]));
    s.labels.format = ValueFormat::Fixed(2);
    s.tooltip_format = Some(ValueFormat::Fixed(3));
    let count = RefCell::new(0);
    let mut text = |c: &BarLabelContext<'_>| {
        *count.borrow_mut() += 1;
        assert_eq!(c.default_text, format!("{:.2}", c.raw_value));
        Ok(format!("{} <&> {}", c.category, c.default_text))
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        let mut n = c.inherit(c.geometry.clone());
        n.fill = Some(Color(240, 30, 180, 255));
        n.opacity = 0.5;
        Ok(n)
    };
    let mut new = bar_layout_with_factories(
        &config(),
        &cats(),
        &data(),
        &Default::default(),
        &s,
        BarFactories {
            bar: Some(&mut bar),
            label: Some(&mut text),
        },
    )?;
    let base = new.layout.clone();
    let before = *count.borrow();
    let binding = new.interaction_binding()?;
    assert!(binding.sites.iter().any(|s| s.value == Some(0.)));
    for b in &base.bars {
        let m = base.scene.marks.iter().find(|m| m.id == b.id).unwrap();
        assert!(m.label.ends_with(&format!("{:.3}", b.raw_value)));
        if let Some(i) = b.label_index {
            assert_eq!(base.scene.labels[i].color, Color(14, 28, 42, 80));
        }
    }
    assert!(render::to_svg(&base.scene)?.contains("&lt;&amp;&gt;"));
    let active = base.bars[0].id.clone();
    new.set_active(Some(&active), 0.2)?;
    for b in &new.layout.bars {
        if let Some(i) = b.label_index {
            assert_eq!(
                new.layout.scene.labels[i].color,
                Color(14, 28, 42, if b.id == active { 80 } else { 16 })
            );
        }
    }
    new.set_active(None, 0.2)?;
    assert_eq!(new.layout, base);
    assert_eq!(*count.borrow(), before);
    for mutation in 0..5 {
        let mut bad = new.clone();
        let i = bad.layout.bars.iter().find_map(|b| b.label_index).unwrap();
        match mutation {
            0 => bad.layout.scene.labels[i].text = "forged".into(),
            1 => {
                bad.layout.scene.label_series.remove(&i);
            }
            2 => {
                bad.layout.scene.label_clips.remove(&i);
            }
            3 => bad.layout.bars[0].raw_value = 999.,
            _ => bad.layout.scene.labels[i].color = Color(0, 0, 0, 0),
        }
        let frame = bad.layout.clone();
        assert!(bad.interaction_binding().is_err());
        assert!(bad.set_active(Some(&active), 0.2).is_err());
        assert_eq!(bad.layout, frame);
    }
    Ok(())
}

#[test]
fn output_byte_limits_errors_and_no_external_side_effect_rollback_claim() -> Result<()> {
    let categories = ["A".into()];
    let source = [BarSeries::new("s", vec![Some(2.)])];
    let backup = source.clone();
    for (text, valid) in [
        (String::new(), true),
        ("é".repeat(512), true),
        ("é".repeat(513), false),
    ] {
        let events = RefCell::new(Vec::new());
        let mut label = |_c: &BarLabelContext<'_>| {
            events.borrow_mut().push("label");
            Ok(text.clone())
        };
        let mut bar = |c: &BarDatumContext<'_>| {
            events.borrow_mut().push("bar");
            Ok(c.inherit(c.geometry.clone()))
        };
        let result = bar_layout_with_factories(
            &config(),
            &categories,
            &source,
            &Default::default(),
            &style(false),
            BarFactories {
                bar: Some(&mut bar),
                label: Some(&mut label),
            },
        );
        assert_eq!(result.is_ok(), valid);
        assert_eq!(
            *events.borrow(),
            if valid {
                vec!["label", "bar"]
            } else {
                vec!["label"]
            }
        );
    }
    let calls = RefCell::new(Vec::new());
    let mut label = |_c: &BarLabelContext<'_>| {
        calls.borrow_mut().push("effect");
        Err(ChartError("application rejected text".into()))
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        calls.borrow_mut().push("bar");
        Ok(c.inherit(c.geometry.clone()))
    };
    assert!(
        bar_layout_with_factories(
            &config(),
            &categories,
            &source,
            &Default::default(),
            &style(false),
            BarFactories {
                bar: Some(&mut bar),
                label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(*calls.borrow(), vec!["effect"]);
    assert_eq!(source, backup);
    Ok(())
}

#[test]
fn all_actual_labels_including_axes_and_totals_are_admitted_before_closures() -> Result<()> {
    let categories = (0..1000).map(|i| format!("C{i}")).collect::<Vec<_>>();
    let source = (0..18)
        .map(|i| BarSeries::new(format!("S{i}"), vec![Some(1.); 1000]))
        .collect::<Vec<_>>();
    let options = BarOptions {
        value_domain: Some([-1., 20.]),
        category_axis: CategoryAxisOptions {
            tick_count: 1000,
            ..Default::default()
        },
        ..Default::default()
    };
    let mut s = style(false);
    s.totals.enabled = true;
    let calls = RefCell::new(0);
    let mut label = |c: &BarLabelContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.default_text.to_string())
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.inherit(c.geometry.clone()))
    };
    let result = bar_layout_with_factories(
        &config(),
        &categories,
        &source,
        &options,
        &s,
        BarFactories {
            bar: Some(&mut bar),
            label: Some(&mut label),
        },
    );
    assert!(result.is_err());
    assert_eq!(*calls.borrow(), 0);
    let mut sparse = source.clone();
    for row in &mut sparse {
        row.values.fill(None);
    }
    sparse[0].values[0] = Some(1.);
    let small = bar_layout_with_factories(
        &config(),
        &categories,
        &sparse,
        &options,
        &s,
        BarFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )?;
    assert_eq!(small.layout.bars.len(), 1);
    assert_eq!(*calls.borrow(), 1);
    assert!(small.layout.scene.labels.len() > 1000);
    Ok(())
}

#[test]
fn no_label_factory_keeps_legacy_deferred_error_and_callback_order() {
    let categories = ["A".into()];
    let source = [
        BarSeries::new("a", vec![Some(1e308)]),
        BarSeries::new("b", vec![Some(1e308)]),
    ];
    let options = BarOptions {
        value_domain: Some([-1., 1.]),
        ..Default::default()
    };
    let mut s = style(false);
    s.totals.enabled = true;
    let mut old_calls = Vec::new();
    let old = bar_layout_with_bar_factory(&config(), &categories, &source, &options, &s, |c| {
        old_calls.push(c.mark_id.to_string());
        Ok(c.inherit(c.geometry.clone()))
    });
    let mut new_calls = Vec::new();
    let mut bar = |c: &BarDatumContext<'_>| {
        new_calls.push(c.mark_id.to_string());
        Ok(c.inherit(c.geometry.clone()))
    };
    let new = bar_layout_with_factories(
        &config(),
        &categories,
        &source,
        &options,
        &s,
        BarFactories {
            bar: Some(&mut bar),
            label: None,
        },
    );
    assert_eq!(old.unwrap_err().to_string(), new.unwrap_err().to_string());
    assert_eq!(old_calls, new_calls);
    assert_eq!(old_calls.len(), 2);
    let calls = RefCell::new(0);
    let mut bar = |c: &BarDatumContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.inherit(c.geometry.clone()))
    };
    let mut label = |c: &BarLabelContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.default_text.to_string())
    };
    assert!(
        bar_layout_with_factories(
            &config(),
            &categories,
            &source,
            &options,
            &s,
            BarFactories {
                bar: Some(&mut bar),
                label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(*calls.borrow(), 0);
}

#[test]
fn source_text_budget_is_checked_before_new_callbacks_and_displayed_text_is_independent()
-> Result<()> {
    let categories = ["A".repeat(256)];
    let source = [BarSeries::new("S".repeat(256), vec![Some(2.)])];
    let mut s = style(false);
    s.tooltip_format = Some(ValueFormat::Affix {
        prefix: "p".repeat(512),
        suffix: String::new(),
        precision: 0,
    });
    let calls = RefCell::new(0);
    let mut label = |c: &BarLabelContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.default_text.to_string())
    };
    let mut bar = |c: &BarDatumContext<'_>| {
        *calls.borrow_mut() += 1;
        Ok(c.inherit(c.geometry.clone()))
    };
    assert!(
        bar_layout_with_factories(
            &config(),
            &categories,
            &source,
            &Default::default(),
            &s,
            BarFactories {
                bar: Some(&mut bar),
                label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(*calls.borrow(), 0);
    let categories = ["A".into()];
    let source = [
        BarSeries::new("a", vec![Some(2.)]),
        BarSeries::new("b", vec![Some(6.)]),
    ];
    let mut s = style(true);
    s.labels.value = BarLabelValue::Displayed;
    s.labels.format = ValueFormat::Percent(1);
    let mut actual = Vec::new();
    let mut text = |c: &BarLabelContext<'_>| {
        actual.push((c.raw_value, c.displayed_value, c.default_text.to_string()));
        Ok(c.default_text.to_string())
    };
    let frame = bar_layout_with_factories(
        &config(),
        &categories,
        &source,
        &BarOptions {
            layout: BarLayout::Percent,
            ..Default::default()
        },
        &s,
        BarFactories {
            label: Some(&mut text),
            ..Default::default()
        },
    )?;
    assert_eq!(
        actual,
        vec![(2., 0.25, "25.0%".into()), (6., 0.75, "75.0%".into())]
    );
    assert!(
        frame
            .interaction_binding()?
            .sites
            .iter()
            .any(|site| site.value == Some(6.))
    );
    Ok(())
}
