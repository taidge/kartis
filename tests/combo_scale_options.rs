use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::{
    surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions},
    *,
};

fn frame() -> ChartConfig {
    ChartConfig {
        width: 300.,
        height: 250.,
        margin: Margin {
            left: 40.25,
            right: 50.25,
            top: 35.25,
            bottom: 45.25,
        },
        ..Default::default()
    }
}

fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

fn tick(scene: &Scene, id: &str) -> [f64; 2] {
    let mark = scene.marks.iter().find(|mark| mark.id == id).unwrap();
    let Geometry::Path(path) = &mark.geometry else {
        panic!("expected a tick path")
    };
    let PathCommand::Move(p) = path[0] else {
        panic!("expected the start of a tick")
    };
    [p.x, p.y]
}

fn picking(out: &ComboLayout, mode: Picking) -> SurfaceModel {
    SurfaceModel::new_with_interaction(
        out.scene.clone(),
        SurfaceOptions {
            picking: mode,
            plot: Some(out.plot),
            ..Default::default()
        },
        Some(out.interaction_binding()),
    )
    .unwrap()
}

#[test]
fn numeric_round_reverse_and_nice_share_ticks_paths_labels_and_typed_sites() {
    let c = frame();
    let o = ComboOptions {
        x: ComboXAxis::Numeric {
            domain: Some([0.13, 9.87]),
            scale: ScaleKind::Linear,
        },
        x_axis: AxisOptions {
            tick_values: Some(vec![2.5]),
            ..Default::default()
        },
        left: YAxisOptions {
            domain: Some([0.13, 9.87]),
            axis: AxisOptions {
                tick_values: Some(vec![2.5]),
                ..Default::default()
            },
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([1.2, 7.8]),
            axis: AxisOptions {
                tick_values: Some(vec![2.]),
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    };
    let mut a = ComboSeries::new(
        "area",
        ComboKind::Area,
        vec![
            ComboPoint::new(7.5, 7.5).identified("end"),
            ComboPoint::new(2.5, 2.5).identified("start"),
        ],
    );
    a.style.point_labels = Some(ComboPointLabels {
        offset: [0., 0.],
        ..Default::default()
    });
    let mut b = ComboSeries::new(
        "log",
        ComboKind::Line,
        vec![
            ComboPoint::new(2.5, 2.).identified("log-start"),
            ComboPoint::new(7.5, 4.),
        ],
    );
    b.axis = YAxisSide::Right;
    let scales = ComboScaleOptions {
        x: Some(ContinuousScaleOptions {
            reverse: true,
            round: true,
            nice: ScaleNice::Count(5),
            ..Default::default()
        }),
        left: Some(ContinuousScaleOptions {
            round: true,
            nice: ScaleNice::Count(5),
            ..Default::default()
        }),
        right: Some(ContinuousScaleOptions {
            kind: ScaleKind::Log,
            log_base: 2.,
            reverse: true,
            round: true,
            nice: ScaleNice::Default,
            ..Default::default()
        }),
    };
    let out = combo_layout_with_scale_options(&c, &[a, b], &o, &scales).unwrap();
    let ResolvedComboXScale::Numeric(x) = &out.x_scale else {
        panic!()
    };
    assert_eq!(x.source_domain(), [0.13, 9.87]);
    assert_eq!(x.display_domain(), [0., 10.]);
    assert_eq!(out.left_scale.source_domain(), [0.13, 9.87]);
    assert_eq!(out.left_scale.display_domain(), [0., 10.]);
    assert_eq!(out.right_scale.source_domain(), [1.2, 7.8]);
    assert_eq!(out.right_scale.display_domain(), [1., 8.]);
    // Fractions 1/4 of the reversed x range, 1/4 of left linear y,
    // and 1/3 of the reversed base-2 logarithmic right range.
    assert_eq!(tick(&out.scene, "combo-axis-x-tick-0")[0], 197.);
    assert_eq!(tick(&out.scene, "combo-axis-left-tick-0")[1], 162.);
    assert_eq!(tick(&out.scene, "combo-axis-right-tick-0")[1], 92.);
    for (id, expected) in [("start", [197., 162.]), ("log-start", [197., 92.])] {
        let site = out
            .sites
            .iter()
            .find(|site| site.point_id.as_deref() == Some(id))
            .unwrap();
        assert_eq!(site.position, expected);
        assert_eq!(site.value_position, expected);
        assert_eq!(out.scene.marks[site.mark_index].id, site.mark_id);
        assert!(out.scene.marks.iter().any(|mark| mark.series == site.series_id
            && mark.id.contains(":path:2:")
            && matches!(&mark.geometry, Geometry::Path(path) if matches!(path[0], PathCommand::Move(p) if [p.x,p.y] == expected))));
    }
    assert_eq!(
        out.sites
            .iter()
            .find(|site| site.point_id.as_deref() == Some("start"))
            .unwrap()
            .source_index,
        1
    );
    assert!(
        out.scene
            .label_series
            .iter()
            .any(|(i, owner)| owner == "area"
                && (out.scene.labels[*i].x, out.scene.labels[*i].y) == (197., 162.))
    );
    assert!(
        out.sites
            .windows(2)
            .all(|p| p[0].mark_index < p[1].mark_index)
    );
    SceneInteraction::from(out).validate().unwrap();
}

#[test]
fn signed_percent_stacks_and_windowed_categories_keep_stable_slots() {
    let c = frame();
    let o = ComboOptions {
        x: ComboXAxis::Categories {
            values: ["A", "B", "C"].map(str::to_owned).to_vec(),
            padding: 0.,
        },
        category_window: Some([0.25, 0.75]),
        left: YAxisOptions {
            domain: Some([1., -1.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([-1., 1.]),
            ..Default::default()
        },
        bar_layout: BarLayout::Percent,
        ..Default::default()
    };
    let mut data: Vec<_> = [("a", 3.), ("b", 1.), ("c", -5.), ("d", -5.), ("other", 9.)]
        .into_iter()
        .map(|(id, v)| ComboSeries::new(id, ComboKind::Bar, vec![ComboPoint::new("B", v)]))
        .collect();
    data[4].axis = YAxisSide::Right;
    let scales = ComboScaleOptions {
        left: Some(ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 2. },
            reverse: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let out = combo_layout_with_scale_options(&c, &data, &o, &scales).unwrap();
    let ResolvedComboXScale::Categories(x) = &out.x_scale else {
        panic!()
    };
    assert_eq!(x.domain(), &["A", "B", "C"]);
    // A centered half-window doubles each of the original 69 5/6 px bands.
    near(x.bandwidth, 209.5 / 3. * 2.);
    near(out.x_scale.map(&"B".into()).unwrap(), 145.);
    let intervals = [[0., 0.75], [0.75, 1.], [0., -0.5], [-0.5, -1.], [0., 1.]];
    for (site, interval) in out.sites.iter().zip(intervals) {
        assert_eq!(site.stack_interval, Some(interval));
        assert_eq!(site.raw_y, data[site.series_index].data[0].y.unwrap());
        let expected = if site.axis == YAxisSide::Left {
            // Declared descending domain plus a reversed y range: +1 is top.
            let t = ((1.5_f64).ln() - interval[1].signum() * (interval[1].abs() / 2.).ln_1p())
                / (2. * (1.5_f64).ln());
            35.25 * (1. - t) + 204.75 * t
        } else {
            35.25
        };
        near(site.value_position[1], expected);
        near(
            site.position[0],
            if site.axis == YAxisSide::Left {
                145. - x.bandwidth * 0.2
            } else {
                145. + x.bandwidth * 0.2
            },
        );
        assert!(site.formatted_value.contains("(raw "));
    }
    let mut model = picking(&out, Picking::Geometry);
    let target = &out.sites[2];
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (target.position[0], target.position[1]),
        })
        .unwrap();
    assert_eq!(model.hovered(), std::slice::from_ref(&target.mark_id));
    assert_eq!(model.datum(&target.mark_id).unwrap().value, -5.);
}

#[test]
fn cropped_rounded_bar_uses_real_path_anchor_and_clipped_stack_endpoint() {
    let c = frame();
    let o = ComboOptions {
        x: ComboXAxis::Categories {
            values: vec!["Q".into()],
            padding: 0.,
        },
        left: YAxisOptions {
            domain: Some([2., 4.]),
            ..Default::default()
        },
        bar_corner_radius: 6.,
        bar_border: Some(GraphicStroke {
            color: Color(20, 20, 20, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        }),
        ..Default::default()
    };
    let rows = [ComboSeries::new(
        "bar",
        ComboKind::Bar,
        vec![ComboPoint::new("Q", 3.)],
    )];
    let scales = ComboScaleOptions {
        left: Some(ContinuousScaleOptions {
            clamp: true,
            reverse: true,
            round: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert!(combo(&c, &rows, &o).is_err());
    let out = combo_layout_with_scale_options(&c, &rows, &o, &scales).unwrap();
    let site = &out.sites[0];
    let mark = &out.scene.marks[site.mark_index];
    assert_eq!(site.stack_interval, Some([0., 3.]));
    assert_eq!(site.value_position, [145., 120.]);
    assert_eq!(mark.clip, Some(out.plot));
    let Geometry::Path(path) = &mark.geometry else {
        panic!("expected rounded bar")
    };
    assert!(matches!(path[0], PathCommand::Move(p) if p.y < out.plot.y - 6.));
    assert_eq!(
        out.anchors[&site.mark_id].resolve(&mark.geometry).unwrap(),
        site.position
    );
    let mut model = picking(&out, Picking::Nearest { distance: 1. });
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (site.position[0], site.position[1]),
        })
        .unwrap();
    assert_eq!(model.hovered(), std::slice::from_ref(&site.mark_id));
}

#[test]
fn cropped_interval_paths_bind_only_existing_ink_and_keep_raw_summary() {
    let c = frame();
    let o = ComboOptions {
        x: ComboXAxis::Categories {
            values: vec!["Q".into()],
            padding: 0.,
        },
        left: YAxisOptions {
            domain: Some([6., 9.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([6., 9.]),
            ..Default::default()
        },
        line_points: false,
        ..Default::default()
    };
    let b = ComboSeries::new(
        "box",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot("Q", [0., 1., 2., 3., 10.]).identified("summary")],
    );
    let mut e = ComboSeries::new(
        "errors",
        ComboKind::ErrorBars,
        vec![ComboPoint::new("Q", 2.).with_bounds(0., 8.)],
    );
    e.axis = YAxisSide::Right;
    e.style.error_cap_width = 0.;
    let mut line = ComboSeries::new("line", ComboKind::Line, vec![ComboPoint::new("Q", 7.)]);
    line.style.point_labels = Some(ComboPointLabels::default());
    let out = combo_layout_with_scale_options(&c, &[b, e, line], &o, &Default::default()).unwrap();
    assert_eq!(out.sites.len(), 2);
    assert_eq!(out.anchors.len(), 2);
    assert!(out.scene.label_series.values().any(|s| s == "line"));
    assert!(out.sites.iter().all(|s| s.series_id != "line"));
    for site in &out.sites {
        let mark = &out.scene.marks[site.mark_index];
        assert!(mark.fill.is_none() && mark.stroke.is_some());
        assert_eq!(site.raw_y, 2.);
        near(site.value_position[1], out.plot.y + out.plot.height);
        assert_eq!(
            out.anchors[&site.mark_id].resolve(&mark.geometry).unwrap(),
            site.position
        );
        assert_ne!(site.position, site.value_position);
    }
    let summary = out
        .sites
        .iter()
        .find(|site| site.kind == ComboKind::BoxPlot)
        .unwrap();
    assert_eq!(summary.point_id.as_deref(), Some("summary"));
    assert!(summary.label.contains("median 2"));
    let mut surface = picking(&out, Picking::Geometry);
    surface
        .apply(SurfaceInput::Move {
            id: 1,
            point: (summary.position[0], out.plot.y + 4.),
        })
        .unwrap();
    assert_eq!(surface.hovered(), std::slice::from_ref(&summary.mark_id));
}

#[test]
fn utc_x_uses_calendar_nice_for_real_bar_centers_and_retains_epoch_data() {
    let c = frame();
    let day = 86_400_000.;
    let leap = 1_709_164_800_000.;
    let o = ComboOptions {
        x: ComboXAxis::Numeric {
            domain: Some([leap - 0.8 * day, leap + 1.8 * day]),
            scale: ScaleKind::Linear,
        },
        x_axis: AxisOptions {
            tick_values: Some(vec![leap]),
            ..Default::default()
        },
        numeric_bar_width: Some(20.),
        ..Default::default()
    };
    let scales = ComboScaleOptions {
        x: Some(ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            nice: ScaleNice::Count(3),
            reverse: true,
            round: true,
            ..Default::default()
        }),
        ..Default::default()
    };
    let data = [ComboSeries::new(
        "daily",
        ComboKind::Bar,
        vec![ComboPoint::new(leap, 5.).identified("leap")],
    )];
    let out = combo_layout_with_scale_options(&c, &data, &o, &scales).unwrap();
    let ResolvedComboXScale::Numeric(x) = &out.x_scale else {
        panic!()
    };
    assert_eq!(x.display_domain(), [leap - day, leap + 2. * day]);
    let expected_x = (249.75_f64 * (2. / 3.) + 40.25 / 3.).round();
    assert_eq!(tick(&out.scene, "combo-axis-x-tick-0")[0], expected_x);
    assert_eq!(out.sites[0].position[0], expected_x);
    assert_eq!(out.sites[0].raw_x, ComboXValue::Number(leap));
    assert!(out.sites[0].label.contains("2024-02-29"));
}

#[test]
fn scale_constraints_reject_baselines_fractional_utc_and_category_override() {
    let c = frame();
    let o = ComboOptions {
        baseline: 1.,
        ..Default::default()
    };
    let area = [ComboSeries::new(
        "area",
        ComboKind::Area,
        vec![ComboPoint::new(0., 2.)],
    )];
    for kind in [ScaleKind::Log, ScaleKind::Utc] {
        let scales = ComboScaleOptions {
            left: Some(ContinuousScaleOptions {
                kind,
                clamp: true,
                ..Default::default()
            }),
            ..Default::default()
        };
        assert!(
            combo_layout_with_scale_options(&c, &area, &o, &scales)
                .unwrap_err()
                .0
                .contains("baseline")
        );
    }
    let data = [ComboSeries::new(
        "bad",
        ComboKind::Scatter,
        vec![ComboPoint::new(0.25, 5.)],
    )];
    let utc = ComboScaleOptions {
        x: Some(ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            ..Default::default()
        }),
        ..Default::default()
    };
    let hidden = ComboOptions {
        hidden: ["bad".into()].into(),
        ..Default::default()
    };
    assert!(combo_layout_with_scale_options(&c, &data, &hidden, &utc).is_err());
    let categorical = ComboOptions {
        x: ComboXAxis::Categories {
            values: vec!["A".into()],
            padding: 0.,
        },
        ..Default::default()
    };
    assert!(
        combo_layout_with_scale_options(&c, &[], &categorical, &utc)
            .unwrap_err()
            .0
            .contains("numeric x")
    );
}
