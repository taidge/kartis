//! Independent accepted-source/geometry audit for typed Bar component roles.
use std::collections::BTreeMap;
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 340.,
        height: 230.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 19.,
            bottom: 27.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<String> {
    ["missing", "zero", "positive", "negative", "cropped"]
        .map(str::to_string)
        .to_vec()
}
fn source() -> Vec<BarSeries> {
    vec![
        BarSeries::new("zeta", vec![None, Some(-0.), Some(4.), Some(-3.), Some(8.)]),
        BarSeries::new("hidden", vec![Some(1.); 5]),
        BarSeries::new("alpha", vec![Some(1.), Some(0.), Some(2.), Some(-2.), None]),
    ]
}
fn options() -> BarOptions {
    let mut o = BarOptions {
        value_domain: Some([-5., 5.]),
        category_window: Some([0., 0.8]),
        ..Default::default()
    };
    o.hidden.insert("hidden".into());
    o.value_axis.visible = false;
    o.value_axis.grid = false;
    o.category_axis.visible = false;
    o.category_axis.grid = false;
    o
}
fn style() -> BarStyleOptions {
    BarStyleOptions {
        series_order: Some(vec!["alpha".into(), "hidden".into(), "zeta".into()]),
        labels: BarLabels {
            enabled: true,
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn build() -> CustomBarLayout {
    bar_layout_with_factories(
        &config(),
        &categories(),
        &source(),
        &options(),
        &style(),
        BarFactories::default(),
    )
    .unwrap()
}
fn measured(_: &BarWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-4., -3.],
        measured: TextSize {
            width: 8.,
            height: 6.,
        },
    }))
}

#[test]
fn default_factory_path_keeps_ordinary_bar_features_and_actual_zero_roles() {
    for horizontal in [false, true] {
        for mode in [BarLayout::Grouped, BarLayout::Stacked, BarLayout::Percent] {
            let mut o = options();
            o.horizontal = horizontal;
            o.layout = mode;
            if mode == BarLayout::Percent {
                o.value_domain = None;
            }
            let plain =
                bar_layout_with_style(&config(), &categories(), &source(), &o, &style()).unwrap();
            let layout = bar_layout_with_factories(
                &config(),
                &categories(),
                &source(),
                &o,
                &style(),
                BarFactories::default(),
            )
            .unwrap();
            assert_eq!(layout.layout, plain);
            let mut calls = vec![];
            let packet = bar_widgets(&layout, 12, |c| {
                assert!(std::ptr::eq(c.layout, &layout));
                assert_eq!(c.series, &source()[c.bar.source_series]);
                assert_eq!(c.category, categories()[c.bar.source_category]);
                assert_eq!(
                    c.series.values[c.bar.source_category].unwrap().to_bits(),
                    c.bar.raw_value.to_bits()
                );
                assert_eq!(c.horizontal, horizontal);
                calls.push(c.bar.id.clone());
                measured(c)
            })
            .unwrap();
            assert_eq!(
                calls,
                plain.bars.iter().map(|b| b.id.clone()).collect::<Vec<_>>()
            );
            assert!(packet.layout.slots.iter().all(|slot| slot.revision == 12));
            assert!(packet.data.values().any(|d| d.bar.raw_value == 0.));
            assert!(
                !packet
                    .data
                    .values()
                    .any(|d| d.bar.series == "hidden" || d.bar.category == "cropped")
            );
            let frame = bar_widget_frame(&layout).unwrap();
            assert_eq!(frame.scene, plain.scene);
            assert_eq!(frame.plot, Some(ClipRect::plot(&config())));
            assert_eq!(
                frame.interaction.as_ref().unwrap().picking_profile,
                PickingProfile::PaintedGeometry
            );
        }
    }
}

#[test]
fn private_source_preserves_complete_nullable_rows_categories_and_negative_zero() {
    let mut rows = source();
    let mut cats = categories();
    let layout = bar_layout_with_factories(
        &config(),
        &cats,
        &rows,
        &options(),
        &style(),
        BarFactories::default(),
    )
    .unwrap();
    let original_rows = rows.clone();
    let original_categories = cats.clone();
    rows[0].values.fill(Some(999.));
    cats[0] = "caller changed".into();
    let packet = bar_widgets(&layout, 0, measured).unwrap();
    assert_eq!(packet.data.len(), 7);
    let mut calls = vec![];
    bar_widgets(&layout, 0, |c| {
        calls.push((c.bar.source_series, c.bar.source_category));
        Ok(None)
    })
    .unwrap();
    assert_eq!(
        calls,
        [(2, 0), (2, 1), (2, 2), (2, 3), (0, 1), (0, 2), (0, 3)]
    );
    let all = packet.data.values().collect::<Vec<_>>();
    for d in &all {
        assert_eq!(d.series.as_ref(), &original_rows[d.bar.source_series]);
        assert_eq!(d.categories.as_ref(), &original_categories);
        assert!(Arc::ptr_eq(&d.categories, &all[0].categories));
    }
    let zeta = all
        .iter()
        .filter(|d| d.bar.source_series == 0)
        .collect::<Vec<_>>();
    for d in &zeta {
        assert!(Arc::ptr_eq(&d.series, &zeta[0].series));
    }
    assert_eq!(zeta[0].series.values[0], None);
    assert_eq!(zeta[0].series.values[4], Some(8.));
    let zero = all
        .iter()
        .find(|d| d.bar.source_series == 0 && d.bar.source_category == 1)
        .unwrap();
    assert_eq!(zero.bar.raw_value.to_bits(), (-0_f64).to_bits());
    assert_eq!(zero.series.values[1].unwrap().to_bits(), (-0_f64).to_bits());
    let site = bar_widget_frame(&layout)
        .unwrap()
        .interaction
        .unwrap()
        .sites
        .into_iter()
        .find(|s| s.mark_id == zero.bar.id)
        .unwrap();
    assert_eq!(site.value.unwrap().to_bits(), (-0_f64).to_bits());
    let mut detached = packet.clone();
    let d = detached.data.values_mut().next().unwrap();
    Arc::make_mut(&mut d.series).values.fill(None);
    Arc::make_mut(&mut d.categories)[0] = "detached edit".into();
    assert_eq!(
        bar_widgets(&layout, 0, measured).unwrap(),
        packet,
        "mutating a returned owned copy cannot forge private source"
    );
}

#[test]
fn all_public_source_or_scene_edits_reject_before_any_callback() {
    type Change = fn(&mut StyledBarLayout);
    let cases: &[(&str, Change)] = &[
        ("bar id", |l| {
            l.bars.last_mut().unwrap().id = "forged".into()
        }),
        ("bar series", |l| {
            l.bars.last_mut().unwrap().series = "forged".into()
        }),
        ("bar category", |l| {
            l.bars.last_mut().unwrap().category = "forged".into()
        }),
        ("series index", |l| {
            l.bars.last_mut().unwrap().source_series = 1
        }),
        ("category index", |l| {
            l.bars.last_mut().unwrap().source_category = 0
        }),
        ("raw value", |l| l.bars.last_mut().unwrap().raw_value = 7.),
        ("displayed value", |l| {
            l.bars.last_mut().unwrap().displayed_value = 7.
        }),
        ("interval", |l| l.bars.last_mut().unwrap().interval[0] += 1.),
        ("band", |l| l.bars.last_mut().unwrap().band[0] += 1.),
        ("visible rect", |l| {
            l.bars.last_mut().unwrap().visible_rect.width += 1.
        }),
        ("computed color", |l| {
            l.bars.last_mut().unwrap().color = Color(1, 2, 3, 255)
        }),
        ("label index", |l| {
            l.bars.last_mut().unwrap().label_index = None
        }),
        ("reorder bars", |l| l.bars.swap(0, 1)),
        ("remove late bar", |l| {
            l.bars.pop();
        }),
        ("domain", |l| l.value_domain[1] += 1.),
        ("category step", |l| l.category_step += 1.),
        ("category bandwidth", |l| l.category_bandwidth += 1.),
        ("total", |l| l.totals[0].value += 1.),
        ("mark paint", |l| {
            l.scene.marks.last_mut().unwrap().fill = Some(Color(1, 2, 3, 255))
        }),
        ("mark opacity", |l| {
            l.scene.marks.last_mut().unwrap().opacity = 0.
        }),
        ("mark value", |l| {
            l.scene.marks.last_mut().unwrap().value = 999.
        }),
        ("mark owner", |l| {
            l.scene.marks.last_mut().unwrap().series = "forged".into()
        }),
        ("mark geometry", |l| {
            l.scene.marks.last_mut().unwrap().geometry = Geometry::Circle {
                x: 4.,
                y: 5.,
                radius: 2.,
            }
        }),
        ("mark clip", |l| {
            l.scene.marks.last_mut().unwrap().clip = None
        }),
        ("late invalid mark", |l| {
            l.scene.marks.last_mut().unwrap().opacity = f64::NAN
        }),
        ("remove actual mark", |l| {
            l.scene.marks.pop();
        }),
        ("owned text", |l| {
            l.scene.labels.last_mut().unwrap().text = "forged".into()
        }),
        ("owned label clip association", |l| {
            let index = l.bars.last().unwrap().label_index.unwrap();
            assert!(l.scene.label_clips.remove(&index).is_some());
        }),
        ("legend", |l| l.scene.legend[0].label = "forged".into()),
        ("background", |l| l.scene.background = Color(1, 2, 3, 255)),
    ];
    let mut accepted = vec![];
    for (name, change) in cases {
        let mut layout = build();
        change(&mut layout.layout);
        let mut calls = 0;
        if bar_widgets(&layout, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .is_ok()
        {
            accepted.push((*name, calls));
        } else {
            assert_eq!(calls, 0, "{name}");
        }
        assert!(
            bar_widget_frame(&layout).is_err(),
            "{name} frame bypassed preflight"
        );
    }
    assert!(
        accepted.is_empty(),
        "unexpected accepted edits {accepted:?}"
    );
    let mut normalized = build();
    let zero = normalized
        .layout
        .bars
        .iter_mut()
        .find(|b| b.source_series == 0 && b.source_category == 1)
        .unwrap();
    zero.raw_value = 0.;
    let mut calls = 0;
    assert!(
        bar_widgets(&normalized, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(calls, 0, "raw sign cannot normalize via public PartialEq");
}

#[test]
fn custom_ink_retains_factory_original_geometry_coordinates_and_path_anchor() {
    for horizontal in [false, true] {
        let mut o = options();
        o.horizontal = horizontal;
        o.value_domain = Some([-1., 2.]);
        let mut original = BTreeMap::new();
        let layout = bar_layout_with_scale_options_and_bar_factory(
            &config(),
            &categories(),
            &source(),
            &o,
            &style(),
            &ContinuousScaleOptions {
                kind: ScaleKind::SymLog { constant: 1. },
                nice: ScaleNice::Off,
                reverse: true,
                ..Default::default()
            },
            |c| {
                original.insert(
                    c.mark_id.to_string(),
                    (
                        c.position,
                        c.plot,
                        c.original_rect,
                        c.endpoints,
                        c.horizontal,
                    ),
                );
                let [x, y] = c.position;
                let mut node = c.inherit(Geometry::Path(vec![
                    C::Move(P::new(x + 10., y - 5.)),
                    C::Line(P::new(x + 22., y - 5.)),
                    C::Line(P::new(x + 22., y + 5.)),
                    C::Line(P::new(x + 10., y + 5.)),
                    C::Close,
                ]));
                if c.source_series == 0 && c.source_category == 1 {
                    node.opacity = 0.;
                }
                Ok(node)
            },
        )
        .unwrap()
        .layout;
        let frame = bar_widget_frame(&layout).unwrap();
        let binding = frame.interaction.as_ref().unwrap();
        let packet = bar_widgets(&layout, 0, |c| {
            assert_eq!(
                (
                    c.position,
                    c.plot,
                    c.original_rect,
                    c.endpoints,
                    c.horizontal
                ),
                original[&c.bar.id]
            );
            assert_eq!(binding.anchor_for(c.mark)?, Some(c.position));
            assert!(matches!(c.mark.geometry, Geometry::Path(_)));
            measured(c)
        })
        .unwrap();
        assert_eq!(packet.data.len(), original.len());
        assert!(
            packet
                .data
                .values()
                .any(|d| d.original_rect.is_some_and(|r| r != d.bar.visible_rect)),
            "unclipped source rect remains independent of crop"
        );
        for d in packet.data.values() {
            assert_eq!(
                (
                    d.position,
                    d.plot,
                    d.original_rect,
                    d.endpoints,
                    d.horizontal
                ),
                original[&d.bar.id]
            );
        }
        assert!(frame.scene.marks.iter().any(|m| m.opacity == 0.));
    }
}

#[test]
fn controlled_active_opacity_is_accepted_without_changing_typed_source_or_roles() {
    let mut layout = build();
    let before = bar_widgets(&layout, 0, measured).unwrap();
    let active = layout.layout.bars[0].id.clone();
    layout.set_active(Some(&active), 0.).unwrap();
    let after = bar_widgets(&layout, 0, |c| {
        assert_eq!(c.mark.opacity, if c.mark.id == active { 1. } else { 0. });
        measured(c)
    })
    .unwrap();
    assert_eq!(after.layout, before.layout);
    for (id, data) in &after.data {
        let original = &before.data[id];
        assert_eq!(
            (
                &data.bar,
                &data.series,
                &data.categories,
                data.position,
                data.plot,
                data.original_rect,
                data.endpoints,
                data.horizontal
            ),
            (
                &original.bar,
                &original.series,
                &original.categories,
                original.position,
                original.plot,
                original.original_rect,
                original.endpoints,
                original.horizontal
            )
        );
        assert_eq!(data.label_clip, original.label_clip);
        if let Some(index) = data.bar.label_index {
            assert_eq!(
                data.label.as_ref(),
                Some(&layout.layout.scene.labels[index])
            );
            let label = data.label.as_ref().unwrap();
            assert_eq!(label.text, original.label.as_ref().unwrap().text);
            assert_eq!(
                label.color.3,
                if id == &active {
                    original.label.as_ref().unwrap().color.3
                } else {
                    0
                }
            );
        } else {
            assert!(data.label.is_none());
            assert!(data.label_clip.is_none());
        }
    }
    bar_widget_frame(&layout).unwrap();
    let accepted = layout.layout.clone();
    assert!(layout.set_active(Some("unknown"), 0.5).is_err());
    assert_eq!(layout.layout, accepted);
    assert!(layout.set_active(Some(&active), f64::NAN).is_err());
    assert_eq!(layout.layout, accepted);
    layout.set_active(None, 0.).unwrap();
    assert_eq!(layout.layout, build().layout);
    let mut calls = 0;
    assert!(
        bar_widgets(&layout, 0, |c| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("audit refuses third Bar".into()))
            } else {
                measured(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
}

#[test]
fn label_factory_branches_keep_original_bar_roles_and_owned_text_separate() {
    for scaled in [false, true] {
        let mut label_calls = BTreeMap::new();
        let mut label = |c: &BarLabelContext<'_>| {
            label_calls.insert(
                c.mark_id.to_string(),
                (c.position, c.plot, c.raw_value.to_bits()),
            );
            Ok(if c.raw_value < 0. {
                String::new()
            } else {
                format!("source label {}", c.category)
            })
        };
        let factories = BarFactories {
            bar: None,
            label: Some(&mut label),
        };
        let layout = if scaled {
            bar_layout_with_scale_options_and_factories(
                &config(),
                &categories(),
                &source(),
                &options(),
                &style(),
                &ContinuousScaleOptions {
                    kind: ScaleKind::SymLog { constant: 1. },
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
                factories,
            )
            .unwrap()
            .layout
        } else {
            bar_layout_with_factories(
                &config(),
                &categories(),
                &source(),
                &options(),
                &style(),
                factories,
            )
            .unwrap()
        };
        let before = layout.layout.scene.clone();
        let packet = bar_widgets(&layout, 0, |c| {
            if let Some(index) = c.bar.label_index {
                assert!(std::ptr::eq(
                    c.label.unwrap(),
                    &layout.layout.scene.labels[index]
                ));
                assert!(std::ptr::eq(
                    c.label_clip.unwrap(),
                    &layout.layout.scene.label_clips[&index]
                ));
            } else {
                assert!(c.label.is_none());
                assert!(c.label_clip.is_none());
            }
            measured(c)
        })
        .unwrap();
        assert_eq!(
            packet.data.len(),
            7,
            "label eligibility does not remove bar roles"
        );
        for data in packet.data.values() {
            if let Some((position, plot, raw_bits)) = label_calls.get(&data.bar.id) {
                assert_eq!(
                    (*position, *plot, *raw_bits),
                    (data.position, data.plot, data.bar.raw_value.to_bits())
                );
                let owned = &layout.layout.scene.labels[data.bar.label_index.unwrap()];
                assert_eq!(owned.text.is_empty(), data.bar.raw_value < 0.);
                assert_eq!(data.label.as_ref(), Some(owned));
                assert_eq!(
                    data.label_clip.as_ref(),
                    layout
                        .layout
                        .scene
                        .label_clips
                        .get(&data.bar.label_index.unwrap())
                );
            }
        }
        assert!(
            packet
                .data
                .values()
                .any(|data| data.bar.raw_value == 0. && data.bar.label_index.is_none())
        );
        assert_eq!(bar_widget_frame(&layout).unwrap().scene, before);
    }
}

#[test]
fn empty_sources_keep_exact_plot_and_selection_budget_counts_only_selected_roles() {
    for (cats, rows) in [
        (categories(), vec![]),
        (vec![], vec![]),
        (categories(), vec![BarSeries::new("none", vec![None; 5])]),
    ] {
        let layout = bar_layout_with_factories(
            &config(),
            &cats,
            &rows,
            &BarOptions::default(),
            &BarStyleOptions::default(),
            BarFactories::default(),
        )
        .unwrap();
        let mut calls = 0;
        let packet = bar_widgets(&layout, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .unwrap();
        assert_eq!(calls, 0);
        assert!(packet.data.is_empty());
        assert_eq!(packet.layout.plot, Some(ClipRect::plot(&config())));
        assert_eq!(
            bar_widget_frame(&layout).unwrap().plot,
            Some(ClipRect::plot(&config()))
        );
    }
    let cats = (0..129).map(|i| format!("column {i}")).collect::<Vec<_>>();
    let rows = [BarSeries::new("zero", vec![Some(0.); 129])];
    let layout = bar_layout_with_factories(
        &config(),
        &cats,
        &rows,
        &BarOptions::default(),
        &BarStyleOptions::default(),
        BarFactories::default(),
    )
    .unwrap();
    let mut calls = 0;
    assert!(
        bar_widgets(&layout, 0, |c| {
            calls += 1;
            measured(c)
        })
        .is_err()
    );
    assert_eq!(calls, 129);
    let selected = bar_widgets(&layout, 0, |c| {
        if c.bar.source_category == 0 {
            Ok(None)
        } else {
            measured(c)
        }
    })
    .unwrap();
    assert_eq!(selected.data.len(), 128);
}
