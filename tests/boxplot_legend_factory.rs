use std::cell::Cell;
use std::collections::BTreeMap;

use kartis::*;

fn chart() -> ChartConfig {
    let mut c = ChartConfig {
        width: 600.,
        height: 320.,
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(10, 20, 30, 255),
        Color(90, 100, 110, 160),
        Color(150, 160, 170, 255),
    ];
    c
}
fn data() -> Vec<BoxObservation> {
    let mut rows = Vec::new();
    for group in ["A", "B"] {
        for subgroup in ["x", "y"] {
            for (i, value) in [Some(10.), None, Some(0.)].into_iter().enumerate() {
                rows.push(BoxObservation::new(
                    format!("{group}/{subgroup}/{i}"),
                    group,
                    subgroup,
                    value,
                ));
            }
        }
    }
    rows
}
fn options() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        groups: Some(vec!["reserved".into(), "B".into(), "A".into()]),
        subgroups: Some(vec!["reserved".into(), "y".into(), "x".into()]),
        inner_padding: 0.,
        legend_mode: BoxPlotLegendMode::ByLabel,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        ..Default::default()
    }
}

#[test]
fn default_keys_keep_full_identity_and_debug_but_by_label_uses_real_summaries() {
    let c = chart();
    let d = data();
    let mut o = options();
    o.legend_mode = BoxPlotLegendMode::Keys;
    let old = boxplot_layout(&c, &d, &o).unwrap();
    assert_eq!(
        old.scene
            .legend
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        [
            "boxplot-series:8:reserved",
            "boxplot-series:1:y",
            "boxplot-series:1:x"
        ]
    );
    assert!(old.scene.legend_targets.is_empty());
    assert!(!format!("{o:?}").contains("legend_mode"));
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    let new = boxplot_layout(&c, &d, &o).unwrap();
    assert_eq!(
        new.scene
            .legend
            .iter()
            .map(|r| r.label.as_str())
            .collect::<Vec<_>>(),
        ["y", "x"]
    );
    assert_eq!(old.boxes, new.boxes);
    let mut scene = new.scene.clone();
    scene.legend = old.scene.legend.clone();
    scene.legend_targets.clear();
    assert_eq!(scene, old.scene);
}

#[test]
fn actual_summary_context_preserves_nullable_original_indices_and_full_palette_identity() {
    let c = chart();
    let d = data();
    let o = options();
    let mut seen = Vec::new();
    let layout = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        assert!(std::ptr::eq(ctx.data.as_ptr(), d.as_ptr()));
        let indices: Vec<_> = d
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.group == ctx.computed_box.group && r.subgroup == ctx.computed_box.subgroup
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(ctx.observation_indices, indices);
        assert_eq!(ctx.observation_indices.len(), 3);
        assert_eq!(ctx.computed_box.count, 2);
        assert!(
            ctx.observation_indices
                .iter()
                .any(|i| d[*i].value.is_none())
        );
        assert_eq!(ctx.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
        assert_eq!(ctx.computed_box.quantiles, [1., 2.5, 5., 7.5, 9.]);
        assert_eq!(ctx.cell_index, ctx.group_index * 3 + ctx.subgroup_index);
        assert_eq!(ctx.color_index, ctx.subgroup_index);
        assert_eq!(ctx.color, c.theme.color(ctx.color_index));
        assert_eq!(ctx.raw_color_key, ctx.computed_box.subgroup);
        assert_eq!(ctx.default_text, ctx.raw_color_key);
        seen.push((ctx.summary_index, ctx.group_index, ctx.subgroup_index));
        Ok(format!(
            "{}/{}",
            ctx.computed_box.group, ctx.computed_box.subgroup
        ))
    })
    .unwrap();
    assert_eq!(seen, [(0, 1, 1), (1, 1, 2), (2, 2, 1), (3, 2, 2)]);
    // Original getLegendData witness: uniq first, then stable full-index sort.
    assert_eq!(
        layout
            .scene
            .legend
            .iter()
            .map(|r| r.label.as_str())
            .collect::<Vec<_>>(),
        ["B/y", "A/y", "B/x", "A/x"]
    );
    let targets: Vec<_> = layout
        .scene
        .legend
        .iter()
        .map(|r| layout.scene.legend_targets[&r.id].clone())
        .collect();
    assert_eq!(
        targets,
        [
            LegendTarget::Series(boxplot_series_id("y")),
            LegendTarget::Series(boxplot_series_id("y")),
            LegendTarget::Series(boxplot_series_id("x")),
            LegendTarget::Series(boxplot_series_id("x"))
        ]
    );
    assert_eq!(
        layout
            .scene
            .legend
            .iter()
            .map(|r| &r.id)
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        4
    );
}

#[test]
fn duplicate_and_empty_text_keep_first_complete_representative_not_union() {
    let c = chart();
    let d = data();
    for text in ["same", ""] {
        for by in [BoxPlotColorBy::SubGroup, BoxPlotColorBy::Group] {
            let mut o = options();
            o.color_by = by;
            let l = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| Ok(text.into()))
                .unwrap();
            assert_eq!(l.scene.legend.len(), 1);
            let row = &l.scene.legend[0];
            assert_eq!(row.label, text);
            assert_eq!(row.color, c.theme.color(1));
            assert_eq!(
                l.scene.legend_targets[&row.id],
                if by == BoxPlotColorBy::Group {
                    LegendTarget::Category("B".into())
                } else {
                    LegendTarget::Series(boxplot_series_id("y"))
                }
            );
            assert!(row.id.contains("box:1:B:1:y"));
        }
    }
}

#[test]
fn by_label_map_fallback_empty_subgroup_and_hidden_reserved_qualification() {
    let c = chart();
    let d = vec![
        BoxObservation::new("none", "A", "", None),
        BoxObservation::new("zero", "A", "", Some(0.)),
    ];
    let mut o = BoxPlotLayoutOptions {
        legend_mode: BoxPlotLegendMode::ByLabel,
        ..Default::default()
    };
    assert_eq!(
        boxplot_layout(&c, &d, &o).unwrap().scene.legend[0].label,
        ""
    );
    o.legend_mode = BoxPlotLegendMode::Keys;
    assert_eq!(
        boxplot_layout(&c, &d, &o).unwrap().scene.legend[0].label,
        "Samples"
    );
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    o.legend_labels = Some(BTreeMap::from([(String::new(), "Caption".into())]));
    let mut calls = 0;
    let l = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        calls += 1;
        assert_eq!(ctx.default_text, "Caption");
        assert_eq!(ctx.observation_indices, [0, 1]);
        Ok(ctx.default_text.into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(l.scene.legend[0].label, "Caption");
    o.hidden.insert(boxplot_series_id(""));
    calls = 0;
    assert!(
        boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| {
            calls += 1;
            Ok("bad".into())
        })
        .unwrap()
        .scene
        .legend
        .is_empty()
    );
    assert_eq!(calls, 0);
}

#[test]
fn summary_eligibility_is_not_ink_eligibility_and_legend_false_calls_nothing() {
    let c = chart();
    let d = vec![
        BoxObservation::new("constant", "A", "x", Some(0.)),
        BoxObservation::new("null", "B", "x", None),
    ];
    let mut o = options();
    o.groups = None;
    o.subgroups = None;
    o.fill_opacity = 0.;
    o.median.width = 0.;
    o.whisker.width = 0.;
    o.outlier_radius = 0.;
    o.domain = Some([10., 20.]);
    let mut calls = 0;
    let l = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        calls += 1;
        assert_eq!(ctx.computed_box.quantiles, [0.; 5]);
        Ok("zero".into())
    })
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(l.scene.legend.len(), 1);
    o.legend = false;
    calls = 0;
    assert!(
        boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| {
            calls += 1;
            Ok("x".into())
        })
        .unwrap()
        .scene
        .legend
        .is_empty()
    );
    assert_eq!(calls, 0);
}

#[test]
fn intrinsic_scale_geometry_and_key_mode_errors_precede_all_callbacks() {
    let c = chart();
    let d = data();
    let calls = Cell::new(0);
    let mut callback = |_: &BoxPlotLegendContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("caption".into())
    };
    let mut o = options();
    o.legend_mode = BoxPlotLegendMode::Keys;
    assert!(boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut callback).is_err());
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    o.median.width = f64::NAN;
    assert!(boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut callback).is_err());
    o = options();
    o.domain = Some([0., 0.]);
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Log,
        ..Default::default()
    };
    assert!(
        boxplot_layout_with_scale_options_and_legend_label_factory(
            &c,
            &d,
            &o,
            &scale,
            &mut callback
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn utf8_limit_and_callback_error_stop_at_the_actual_offending_summary() {
    let c = chart();
    let d = data();
    let o = options();
    let exact = "北".repeat(341) + "x";
    assert_eq!(exact.len(), 1024);
    assert_eq!(
        boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| Ok(exact.clone()))
            .unwrap()
            .scene
            .legend
            .len(),
        1
    );
    let calls = Cell::new(0);
    let err = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| {
        calls.set(calls.get() + 1);
        Ok("北".repeat(341) + "xx")
    })
    .unwrap_err();
    assert!(err.0.contains("byte budget"));
    assert_eq!(calls.get(), 1);
    calls.set(0);
    let err = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| {
        calls.set(calls.get() + 1);
        if calls.get() == 2 {
            Err(ChartError("actual callback stop".into()))
        } else {
            Ok("first".into())
        }
    })
    .unwrap_err();
    assert_eq!(err.0, "actual callback stop");
    assert_eq!(calls.get(), 2);
}

#[test]
fn budget_counts_unique_rows_after_callbacks_not_candidate_summaries() {
    let c = chart();
    let mut o = options();
    o.groups = None;
    o.subgroups = None;
    let d: Vec<_> = (0..256)
        .flat_map(|g| {
            (0..8).map(move |s| {
                BoxObservation::new(
                    format!("{g}/{s}"),
                    format!("g{g:03}"),
                    format!("s{s}"),
                    Some(0.),
                )
            })
        })
        .collect();
    let mut calls = 0;
    let l = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |_| {
        calls += 1;
        Ok("one".into())
    })
    .unwrap();
    assert_eq!(calls, 2048);
    assert_eq!(l.boxes.len(), 2048);
    assert_eq!(l.scene.legend.len(), 1);
    calls = 0;
    let err = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        calls += 1;
        Ok(ctx.computed_box.id.clone())
    })
    .unwrap_err();
    assert_eq!(calls, 257);
    assert!(err.0.contains("distinct legend"));
}

#[test]
fn configured_utc_preserves_fractional_statistics_and_exposes_probabilities_separately() {
    let c = chart();
    let d = vec![
        BoxObservation::new("a", "A", "x", Some(1.)),
        BoxObservation::new("b", "A", "x", Some(4.)),
    ];
    let mut o = options();
    o.groups = None;
    o.subgroups = None;
    o.auto_domain_source = BoxPlotAutoDomainSource::SummaryQuantiles;
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let mut calls = 0;
    let l = boxplot_layout_with_scale_options_and_legend_label_factory(
        &c,
        &d,
        &o,
        &scale,
        &mut |ctx| {
            calls += 1;
            assert_eq!(ctx.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
            assert!((ctx.computed_box.quantiles[0] - 1.3).abs() < 1e-12);
            Ok("UTC".into())
        },
    )
    .unwrap();
    assert_eq!(calls, 1);
    assert_eq!(l.layout.domain, [1., 3.]);
    assert!(l.map_statistic(1.3).unwrap().is_finite());
}

#[test]
fn raw_unicode_and_separator_keys_have_injective_caption_independent_row_ids() {
    let c = chart();
    let d = vec![
        BoxObservation::new("one", "北:/A", "x/y:北", Some(0.)),
        BoxObservation::new("two", "A", "x/y:北", Some(1.)),
    ];
    let o = BoxPlotLayoutOptions {
        legend_mode: BoxPlotLegendMode::ByLabel,
        ..Default::default()
    };
    let first = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        Ok(ctx.computed_box.group.clone())
    })
    .unwrap();
    let second = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut |ctx| {
        Ok(format!("caption {}", ctx.computed_box.group))
    })
    .unwrap();
    assert_eq!(first.scene.legend.len(), 2);
    assert_ne!(first.scene.legend[0].id, first.scene.legend[1].id);
    assert_eq!(
        first.scene.legend.iter().map(|r| &r.id).collect::<Vec<_>>(),
        second
            .scene
            .legend
            .iter()
            .map(|r| &r.id)
            .collect::<Vec<_>>()
    );
    for item in &first.scene.legend {
        assert_eq!(
            first.scene.legend_targets[&item.id],
            LegendTarget::Series(boxplot_series_id("x/y:北"))
        );
    }
}
