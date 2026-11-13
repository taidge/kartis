use kartis::*;
use std::cell::{Cell, RefCell};

fn chart() -> ChartConfig {
    let mut c = ChartConfig {
        width: 600.,
        height: 320.,
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(10, 20, 30, 255),
        Color(40, 50, 60, 140),
        Color(70, 80, 90, 255),
    ];
    c
}
fn observations() -> Vec<BoxObservation> {
    let mut rows = Vec::new();
    for group in ["A", "B"] {
        for subgroup in ["x", "y"] {
            for (i, value) in [Some(0.), None, Some(10.)].into_iter().enumerate() {
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
fn source_color(ctx: &BoxPlotColorContext<'_>) -> Result<Color> {
    Ok(Color(
        100 + ctx.group_index as u8,
        120 + ctx.subgroup_index as u8,
        140,
        101,
    ))
}
fn mark<'a>(layout: &'a BoxPlotLayout, id: &str) -> &'a Mark {
    layout.scene.marks.iter().find(|m| m.id == id).unwrap()
}
fn exact(old: &BoxPlotLayout, new: &BoxPlotLayout) {
    assert_eq!(old, new);
    assert_eq!(format!("{old:?}"), format!("{new:?}"));
    assert_eq!(format!("{old:#?}"), format!("{new:#?}"));
    assert_eq!(
        render::to_svg(&old.scene).unwrap(),
        render::to_svg(&new.scene).unwrap()
    );
    assert_eq!(
        render::to_canvas_script(&old.scene, "test").unwrap(),
        render::to_canvas_script(&new.scene, "test").unwrap()
    );
}

#[test]
fn absent_factories_use_exact_legacy_builders_and_controller_debug() {
    let c = chart();
    let d = observations();
    for mode in [BoxPlotLegendMode::Keys, BoxPlotLegendMode::ByLabel] {
        let mut o = options();
        o.legend_mode = mode;
        let old = boxplot_layout(&c, &d, &o).unwrap();
        exact(
            &old,
            &boxplot_layout_with_factories(&c, &d, &o, BoxPlotFactories::default()).unwrap(),
        );
        let scale = ContinuousScaleOptions {
            reverse: true,
            round: true,
            ..Default::default()
        };
        let old_scaled = boxplot_layout_with_scale_options(&c, &d, &o, &scale).unwrap();
        let new_scaled = boxplot_layout_with_scale_options_and_factories(
            &c,
            &d,
            &o,
            &scale,
            BoxPlotFactories::default(),
        )
        .unwrap();
        exact(&old_scaled.layout, &new_scaled.layout);
        assert_eq!(
            format!("{:?}", old_scaled.value_scale),
            format!("{:?}", new_scaled.value_scale)
        );
        let interaction = BoxPlotInteractionOptions::default();
        let old = BoxPlotInteraction::new(&c, &d, &o, &interaction).unwrap();
        let new = BoxPlotInteraction::with_factories(
            &c,
            &d,
            &o,
            &interaction,
            BoxPlotFactories::default(),
        )
        .unwrap();
        assert_eq!(format!("{old:?}"), format!("{new:?}"));
        assert_eq!(format!("{old:#?}"), format!("{new:#?}"));
    }
}

#[test]
fn context_preserves_true_statistics_nullable_source_and_complete_domain_indices() {
    let c = chart();
    let d = observations();
    let o = options();
    let ordinary = boxplot_layout(&c, &d, &o).unwrap();
    let mut calls = 0;
    let mut factory = |ctx: &BoxPlotColorContext<'_>| {
        assert!(std::ptr::eq(ctx.data.as_ptr(), d.as_ptr()));
        assert_eq!(ctx.computed_box, &ordinary.boxes[calls]);
        let expected: Vec<_> = d
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.group == ctx.computed_box.group && r.subgroup == ctx.computed_box.subgroup
            })
            .map(|(i, _)| i)
            .collect();
        assert_eq!(ctx.observation_indices, expected);
        assert_eq!(expected.len(), 3);
        assert_eq!(ctx.computed_box.count, 2);
        assert_eq!(ctx.quantile_probabilities, [0.1, 0.25, 0.5, 0.75, 0.9]);
        assert_eq!(ctx.computed_box.quantiles, [1., 2.5, 5., 7.5, 9.]);
        assert_eq!(ctx.summary_index, calls);
        assert_eq!(ctx.cell_index, ctx.group_index * 3 + ctx.subgroup_index);
        assert_eq!(ctx.color_index, ctx.subgroup_index);
        assert_eq!(ctx.raw_color_key, ctx.computed_box.subgroup);
        assert_eq!(ctx.default_color, c.theme.color(ctx.color_index));
        calls += 1;
        source_color(ctx)
    };
    let custom = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut factory),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(calls, 4);
    assert_eq!(custom.boxes, ordinary.boxes);
    assert_eq!(custom.domain, ordinary.domain);
}

#[test]
fn one_color_cache_feeds_body_outlier_roles_binding_and_first_representative() {
    let c = chart();
    let d = observations();
    let mut o = options();
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    o.border_color_profile = Some(InheritedColor::FromContext(vec![]));
    o.median_color_profile = Some(InheritedColor::FromContext(vec![]));
    o.whisker_color_profile = Some(InheritedColor::FromContext(vec![]));
    o.fill_opacity = 0.3;
    o.border = Some(o.median);
    let colors_done = Cell::new(0);
    let labels_done = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        colors_done.set(colors_done.get() + 1);
        source_color(ctx)
    };
    let mut label = |ctx: &BoxPlotLegendContext<'_>| {
        assert_eq!(colors_done.get(), 4);
        assert_eq!(
            ctx.color,
            Color(
                100 + ctx.group_index as u8,
                120 + ctx.subgroup_index as u8,
                140,
                101
            )
        );
        labels_done.set(labels_done.get() + 1);
        Ok("same caption".into())
    };
    let controller = BoxPlotInteraction::with_factories(
        &c,
        &d,
        &o,
        &BoxPlotInteractionOptions::default(),
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(colors_done.get(), 4);
    assert_eq!(labels_done.get(), 4);
    let l = &controller.layout;
    let b = &l.boxes[0];
    let expected = Color(101, 121, 140, 101);
    assert_eq!(l.scene.legend.len(), 1);
    assert_eq!(l.scene.legend[0].color, expected);
    assert_eq!(
        l.scene.legend_target(&l.scene.legend[0].id),
        Some(LegendTarget::Series(boxplot_series_id("y")))
    );
    for b in &l.boxes {
        let gi = l.groups.iter().position(|g| g == &b.group).unwrap();
        let si = l.subgroups.iter().position(|s| s == &b.subgroup).unwrap();
        let rgba = Color(100 + gi as u8, 120 + si as u8, 140, 101);
        assert_eq!(mark(l, &b.id).fill, Some(rgba));
        assert_eq!(mark(l, &b.id).opacity, 0.3);
        for part in ["median", "whiskers", "caps"] {
            assert_eq!(
                mark(l, &format!("{}:{part}", b.id)).stroke.unwrap().color,
                rgba
            );
        }
        if o.border.is_some() {
            assert_eq!(
                mark(l, &format!("{}:border", b.id)).stroke.unwrap().color,
                rgba
            );
        }
        for outlier in &b.outliers {
            let id = format!(
                "{}:outlier:{}:{}",
                b.id,
                outlier.source_id.len(),
                outlier.source_id
            );
            assert_eq!(mark(l, &id).fill, Some(rgba));
        }
    }
    assert_eq!(
        controller.interaction_binding().unwrap().sites[0].color,
        expected
    );
    assert_eq!(mark(l, &b.id).value, b.quantiles[2]);
}

#[test]
fn keys_use_first_actual_summary_but_reserved_palette_rows_have_no_callback() {
    let c = chart();
    let d = observations();
    let o = options();
    let mut calls = 0;
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        calls += 1;
        source_color(ctx)
    };
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(calls, 4);
    assert_eq!(l.scene.legend.len(), 3);
    assert_eq!(l.scene.legend[0].color, c.theme.color(0));
    assert_eq!(l.scene.legend[1].color, Color(101, 121, 140, 101));
    assert_eq!(l.scene.legend[2].color, Color(101, 122, 140, 101));
}

#[test]
fn hidden_null_only_and_empty_never_call_but_legend_false_cropped_zero_summary_does() {
    let c = chart();
    let mut o = options();
    o.groups = None;
    o.subgroups = None;
    o.legend = false;
    o.fill_opacity = 0.;
    o.domain = Some([10., 20.]);
    let d = [
        BoxObservation::new("zero", "A", "x", Some(0.)),
        BoxObservation::new("none", "B", "x", None),
    ];
    let count = Cell::new(0);
    let mut f = |ctx: &BoxPlotColorContext<'_>| {
        count.set(count.get() + 1);
        assert_eq!(ctx.computed_box.quantiles, [0.; 5]);
        Ok(Color(1, 2, 3, 0))
    };
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut f),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(count.get(), 1);
    assert!(l.scene.legend.is_empty());
    assert_eq!(mark(&l, &l.boxes[0].id).fill, Some(Color(1, 2, 3, 0)));
    o.hidden.insert(boxplot_series_id("x"));
    boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut f),
            legend_label: None,
        },
    )
    .unwrap();
    boxplot_layout_with_factories(
        &c,
        &[],
        &o,
        BoxPlotFactories {
            color: Some(&mut f),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(count.get(), 1);
}

#[test]
fn intrinsic_errors_precede_callbacks_and_contrast_uses_returned_color_not_palette() {
    let mut c = chart();
    let d = observations();
    let mut o = options();
    let count = Cell::new(0);
    let mut color = |_: &BoxPlotColorContext<'_>| {
        count.set(count.get() + 1);
        Ok(Color(3, 4, 5, 255))
    };
    o.median.width = f64::NAN;
    assert!(
        boxplot_layout_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    o = options();
    o.border_color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
        f64::NAN,
    )]));
    assert!(
        boxplot_layout_with_factories(
            &c,
            &[],
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    o = options();
    let mut bad = d.clone();
    bad[0].id = bad[1].id.clone();
    assert!(
        boxplot_layout_with_factories(
            &c,
            &bad,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    let invalid_scale = ContinuousScaleOptions {
        log_base: f64::NAN,
        ..Default::default()
    };
    assert!(
        boxplot_layout_with_scale_options_and_factories(
            &c,
            &d,
            &o,
            &invalid_scale,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    assert_eq!(count.get(), 0);
    c.theme.background = Color(1, 2, 3, 0);
    c.theme.colors = vec![Color(9, 8, 7, 0)];
    o.median_color_profile = Some(InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    });
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(count.get(), 4);
    assert_eq!(mark(&l, &l.boxes[0].id).fill, Some(Color(3, 4, 5, 255)));
    let failed = Cell::new(0);
    let mut transparent = |_: &BoxPlotColorContext<'_>| {
        failed.set(failed.get() + 1);
        Ok(Color(3, 4, 5, 0))
    };
    assert!(
        boxplot_layout_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut transparent),
                legend_label: None
            }
        )
        .is_err()
    );
    assert_eq!(failed.get(), 4); // Context-dependent failures do not undo callback side effects.
}

#[test]
fn cached_hover_and_atomic_replacement_require_every_previously_used_factory() {
    let c = chart();
    let d = observations();
    let mut o = options();
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    let colors = Cell::new(0);
    let captions = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        colors.set(colors.get() + 1);
        source_color(ctx)
    };
    let mut label = |ctx: &BoxPlotLegendContext<'_>| {
        captions.set(captions.get() + 1);
        Ok(ctx.default_text.into())
    };
    let mut x = BoxPlotInteraction::with_factories(
        &c,
        &d,
        &o,
        &Default::default(),
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut label),
        },
    )
    .unwrap();
    let base = x.layout.clone();
    let active = BoxPlotKey::new("A", "x");
    x.set_active(Some(active.clone())).unwrap();
    x.set_active(None).unwrap();
    exact(&base, &x.layout);
    assert_eq!((colors.get(), captions.get()), (4, 4));
    x.set_active(Some(active.clone())).unwrap();
    let accepted = x.layout.clone();
    assert!(x.replace_source(&c, &d, &o).is_err());
    assert!(
        x.replace_source_with_legend_label_factory(&c, &d, &o, &mut label)
            .is_err()
    );
    assert!(
        x.replace_source_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    assert!(
        x.replace_source_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: None,
                legend_label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!((colors.get(), captions.get()), (4, 4));
    assert_eq!(accepted, x.layout);
    let mut fail = |_: &BoxPlotColorContext<'_>| Err(ChartError("actual callback stop".into()));
    assert!(
        x.replace_source_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut fail),
                legend_label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(accepted, x.layout);
    assert_eq!(x.active(), Some(&active));
    x.replace_source_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(x.active(), Some(&active));
    assert_eq!((colors.get(), captions.get()), (8, 8));
    o.hidden.insert(boxplot_series_id("x"));
    x.replace_source_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: Some(&mut label),
        },
    )
    .unwrap();
    assert_eq!(x.active(), None);
}

#[test]
fn empty_color_factory_policy_is_private_and_public_source_tampering_cannot_refresh_it() {
    let c = chart();
    let d = observations();
    let mut o = options();
    o.legend = false;
    let count = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        count.set(count.get() + 1);
        source_color(ctx)
    };
    let mut x = BoxPlotInteraction::with_factories(
        &c,
        &[],
        &o,
        &Default::default(),
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(count.get(), 0);
    assert!(format!("{x:?}").contains("color_factory_built: true"));
    assert!(x.replace_source(&c, &d, &o).is_err());
    assert!(
        x.replace_source_with_factories(&c, &d, &o, BoxPlotFactories::default())
            .is_err()
    );
    x.replace_source_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    let accepted = x.layout.clone();
    x.layout.scene.marks[0].fill = Some(Color(0, 0, 0, 255));
    assert!(x.interaction_binding().is_err());
    assert!(x.set_active(None).is_err());
    assert!(
        x.replace_source_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: None
            }
        )
        .is_err()
    );
    assert_eq!(count.get(), 4);
    x.layout = accepted;
    x.set_active(None).unwrap();
}

#[test]
fn configured_scale_and_group_colors_keep_source_geometry_and_category_targets() {
    let c = chart();
    let d = observations();
    let mut o = options();
    o.color_by = BoxPlotColorBy::Group;
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    let scale = ContinuousScaleOptions {
        reverse: true,
        round: true,
        ..Default::default()
    };
    let baseline = boxplot_layout_with_scale_options(&c, &d, &o, &scale).unwrap();
    let calls = RefCell::new(Vec::new());
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        assert_eq!(ctx.color_index, ctx.group_index);
        assert_eq!(ctx.raw_color_key, ctx.computed_box.group);
        calls.borrow_mut().push(ctx.computed_box.id.clone());
        source_color(ctx)
    };
    let mut x = BoxPlotInteraction::with_scale_options_and_factories(
        &c,
        &d,
        &o,
        &scale,
        &Default::default(),
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(x.layout.boxes, baseline.layout.boxes);
    assert_eq!(calls.borrow().len(), 4);
    for row in &x.layout.scene.legend {
        assert!(matches!(
            x.layout.scene.legend_target(&row.id),
            Some(LegendTarget::Category(_))
        ));
    }
    x.replace_source_with_scale_options_and_factories(
        &c,
        &d,
        &o,
        &scale,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    assert_eq!(calls.borrow().len(), 8);
}

#[test]
fn fixed_outlier_and_theme_strokes_stay_independent_of_source_color_alpha() {
    let c = chart();
    let d = observations();
    let mut o = options();
    o.outlier_fill = Some(Color(3, 9, 27, 123));
    o.median_color_profile = Some(InheritedColor::Theme);
    o.whisker_color_profile = Some(InheritedColor::FromContext(vec![ColorModifier::Opacity(
        0.5,
    )]));
    let mut color = |_: &BoxPlotColorContext<'_>| Ok(Color(180, 120, 60, 0));
    let l = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: Some(&mut color),
            legend_label: None,
        },
    )
    .unwrap();
    for b in &l.boxes {
        assert_eq!(mark(&l, &b.id).fill, Some(Color(180, 120, 60, 0)));
        assert_eq!(
            mark(&l, &format!("{}:median", b.id)).stroke.unwrap().color,
            o.median.color
        );
        assert_eq!(
            mark(&l, &format!("{}:whiskers", b.id))
                .stroke
                .unwrap()
                .color,
            Color(180, 120, 60, 128)
        );
        assert!(!b.outliers.is_empty());
        for outlier in &b.outliers {
            let id = format!(
                "{}:outlier:{}:{}",
                b.id,
                outlier.source_id.len(),
                outlier.source_id
            );
            assert_eq!(mark(&l, &id).fill, o.outlier_fill);
        }
    }
}

#[test]
fn legend_only_delegation_is_exact_and_keys_rejects_before_both_factories() {
    let c = chart();
    let d = observations();
    let mut o = options();
    let calls = Cell::new(0);
    let mut color = |ctx: &BoxPlotColorContext<'_>| {
        calls.set(calls.get() + 1);
        source_color(ctx)
    };
    let mut label = |ctx: &BoxPlotLegendContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(format!("{}:{}", ctx.computed_box.group, ctx.default_text))
    };
    assert!(
        boxplot_layout_with_factories(
            &c,
            &d,
            &o,
            BoxPlotFactories {
                color: Some(&mut color),
                legend_label: Some(&mut label)
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    o.legend_mode = BoxPlotLegendMode::ByLabel;
    let old = boxplot_layout_with_legend_label_factory(&c, &d, &o, &mut label).unwrap();
    let new = boxplot_layout_with_factories(
        &c,
        &d,
        &o,
        BoxPlotFactories {
            color: None,
            legend_label: Some(&mut label),
        },
    )
    .unwrap();
    exact(&old, &new);
    assert_eq!(calls.get(), 8);
}
