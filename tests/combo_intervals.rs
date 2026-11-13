use std::collections::BTreeSet;

use glory::render::{GraphicStroke, PathCommand, StrokeCap};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 360.,
        margin: Margin {
            left: 60.,
            right: 60.,
            top: 30.,
            bottom: 50.,
        },
        ..Default::default()
    }
}
fn categories() -> ComboOptions {
    ComboOptions {
        x: ComboXAxis::Categories {
            values: vec!["A".into(), "B".into()],
            padding: 0.,
        },
        left: YAxisOptions {
            domain: Some([0., 100.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([0., 1000.]),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn data<'a>(scene: &'a Scene, series: &str) -> Vec<&'a Mark> {
    scene.marks.iter().filter(|m| m.series == series).collect()
}
fn center(mark: &Mark) -> (f64, f64) {
    match mark.geometry {
        Geometry::Circle { x, y, .. } => (x, y),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => (x + width / 2., y + height / 2.),
        _ => panic!("not a point or rect"),
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}

#[test]
fn boxes_bars_errors_and_lines_share_x_and_independent_value_axes() {
    let mut boxplot = ComboSeries::new(
        "box",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot("A", [100., 300., 500., 700., 900.])],
    );
    boxplot.axis = YAxisSide::Right;
    boxplot.style.point_labels = Some(ComboPointLabels::default());
    let bars = ComboSeries::new("bar", ComboKind::Bar, vec![ComboPoint::new("A", 50.)]);
    let errors = ComboSeries::new(
        "errors",
        ComboKind::ErrorBars,
        vec![ComboPoint::new("A", 50.).with_bounds(20., 80.)],
    );
    let line = ComboSeries::new(
        "line",
        ComboKind::Line,
        vec![ComboPoint::new("A", 50.), ComboPoint::new("B", 75.)],
    );
    let scene = combo(&config(), &[boxplot, bars, errors, line], &categories()).unwrap();
    scene.validate().unwrap();
    let body = *data(&scene, "box")
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Rect { .. }))
        .unwrap();
    let bar = data(&scene, "bar")[0];
    let point = *data(&scene, "line")
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Circle { .. }))
        .unwrap();
    assert_eq!(center(body), (228., 170.));
    assert_eq!(center(bar).0, 132.);
    assert_eq!(center(point), (180., 170.));
    let Geometry::Rect {
        y, height, width, ..
    } = body.geometry
    else {
        panic!()
    };
    close(y, 114.);
    close(height, 112.);
    close(width, 96.);
    let error = data(&scene, "errors")[0];
    let Geometry::Path(commands) = &error.geometry else {
        panic!()
    };
    assert!(matches!(commands[0],PathCommand::Move(p) if p.x==180.&&p.y==254.));
    let PathCommand::Line(end) = commands[1] else {
        panic!()
    };
    close(end.x, 180.);
    close(end.y, 86.);
    let owned = *scene
        .label_series
        .iter()
        .find(|(_, owner)| *owner == "box")
        .unwrap()
        .0;
    assert_eq!((scene.labels[owned].x, scene.labels[owned].y), (228., 161.));
    assert!(body.label.contains("median 500") && body.label.contains("Q1 300"));
    let median = *data(&scene, "box")
        .iter()
        .find(|m| m.id.ends_with(":median"))
        .unwrap();
    assert_eq!(median.stroke.unwrap().color, config().theme.text);
    assert_ne!(Some(median.stroke.unwrap().color), body.fill);
    assert_eq!(
        scene
            .marks
            .iter()
            .filter(|m| m.id == "combo-axis-left")
            .count(),
        1
    );
    assert_eq!(
        scene
            .marks
            .iter()
            .filter(|m| m.id == "combo-axis-right")
            .count(),
        1
    );
}

#[test]
fn interval_endpoints_drive_auto_domains_on_both_axes() {
    let left = ComboSeries::new(
        "left",
        ComboKind::ErrorBars,
        vec![ComboPoint::new(0., 50.).with_bounds(0., 100.)],
    );
    let mut right = ComboSeries::new(
        "right",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot(0., [0., 2500., 5000., 7500., 10000.])],
    );
    right.axis = YAxisSide::Right;
    let middle = ComboSeries::new("middle", ComboKind::Scatter, vec![ComboPoint::new(0., 50.)]);
    let scene = combo(&config(), &[left, right, middle], &ComboOptions::default()).unwrap();
    let error = data(&scene, "left")[0];
    let Geometry::Path(commands) = &error.geometry else {
        panic!()
    };
    assert!(matches!(commands[0],PathCommand::Move(p) if p.y==310.));
    assert!(matches!(commands[1],PathCommand::Line(p) if p.y==30.));
    assert_eq!(center(data(&scene, "middle")[0]).1, 170.);
    let body = *data(&scene, "right")
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Rect { .. }))
        .unwrap();
    assert_eq!(center(body).1, 170.);
    let Geometry::Rect { height, .. } = body.geometry else {
        panic!()
    };
    close(height, 140.);
}

#[test]
fn categorical_and_explicit_numeric_gaps_split_linear_range_envelopes() {
    let series = ComboSeries::new(
        "range",
        ComboKind::RangeBand,
        vec![
            ComboPoint::range("A", 10., 30.),
            ComboPoint::range("B", 20., 40.),
            ComboPoint::range("D", 30., 60.),
            ComboPoint::range("E", 40., 70.),
        ],
    );
    let mut options = categories();
    options.x = ComboXAxis::Categories {
        values: ["A", "B", "C", "D", "E"].map(str::to_owned).to_vec(),
        padding: 0.,
    };
    let scene = combo(&config(), std::slice::from_ref(&series), &options).unwrap();
    let bands = data(&scene, "range")
        .into_iter()
        .filter(|m| m.id.contains(":band:"))
        .collect::<Vec<_>>();
    assert_eq!(bands.len(), 2);
    for mark in bands {
        let Geometry::Path(commands) = &mark.geometry else {
            panic!()
        };
        assert_eq!(commands.len(), 5);
        assert_eq!(commands.last(), Some(&PathCommand::Close));
        assert_eq!(mark.opacity, 0.35);
        assert_eq!(mark.clip, Some(ClipRect::plot(&config())));
    }
    let mut reordered = series.clone();
    reordered.data.reverse();
    assert_eq!(scene, combo(&config(), &[reordered], &options).unwrap());
    let numeric = ComboSeries::new(
        "n",
        ComboKind::RangeBand,
        vec![
            ComboPoint::range(0., 10., 30.),
            ComboPoint::range(1., 10., 40.),
            ComboPoint::gap(2.),
            ComboPoint::range(3., 10., 50.),
            ComboPoint::range(4., 10., 60.),
        ],
    );
    let scene = combo(&config(), &[numeric], &ComboOptions::default()).unwrap();
    assert_eq!(
        data(&scene, "n")
            .into_iter()
            .filter(|m| m.id.contains(":band:"))
            .count(),
        2
    );
}

#[test]
fn error_caps_are_not_invented_at_clipped_domain_edges() {
    let series = ComboSeries::new(
        "errors",
        ComboKind::ErrorBars,
        vec![ComboPoint::new("A", 50.).with_bounds(-100., 150.)],
    );
    let scene = combo(&config(), &[series], &categories()).unwrap();
    let error = data(&scene, "errors")[0];
    let Geometry::Path(commands) = &error.geometry else {
        panic!()
    };
    assert_eq!(
        commands.len(),
        2,
        "only the visible vertical interval, no false endpoint caps"
    );
    assert!(matches!(commands[0],PathCommand::Move(p) if p.y==310.));
    assert!(matches!(commands[1],PathCommand::Line(p) if p.y==30.));
    assert!(
        scene
            .hit_test(180., 170.)
            .is_some_and(|m| m.series == "errors")
    );
}

#[test]
fn per_series_line_point_fill_opacity_and_labels_reach_actual_svg() {
    let mut a = ComboSeries::new(
        "area",
        ComboKind::Area,
        vec![ComboPoint::new("A", 25.), ComboPoint::new("B", 75.)],
    );
    a.color = Some(Color(200, 30, 40, 255));
    a.style.line_width = 5.;
    a.style.point_radius = Some(9.);
    a.style.point_color = Some(Color(20, 140, 80, 255));
    a.style.point_border = Some(GraphicStroke {
        color: Color(1, 2, 3, 255),
        width: 3.,
        cap: StrokeCap::Round,
    });
    a.style.area_opacity = 0.2;
    a.style.opacity = 0.5;
    a.style.point_labels = Some(ComboPointLabels {
        format: ValueFormat::Fixed(1),
        offset: [2., -12.],
        color: Some(Color(7, 8, 9, 255)),
        size: Some(18.),
    });
    let scene = combo(&config(), &[a], &categories()).unwrap();
    let marks = data(&scene, "area");
    let point = *marks
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Circle { .. }))
        .unwrap();
    assert!(matches!(
        point.geometry,
        Geometry::Circle { radius: 9., .. }
    ));
    assert_eq!(point.fill, Some(Color(20, 140, 80, 255)));
    assert_eq!(point.stroke.unwrap().width, 3.);
    assert_eq!(point.opacity, 0.5);
    let line = *marks.iter().find(|m| m.id.contains(":path:2:")).unwrap();
    assert_eq!(line.stroke.unwrap().width, 5.);
    assert_eq!(line.stroke.unwrap().cap, StrokeCap::Round);
    let area = *marks.iter().find(|m| m.id.contains(":path:0:")).unwrap();
    assert_eq!(area.opacity, 0.1);
    let (&slot, _) = scene
        .label_series
        .iter()
        .find(|(slot, _)| scene.labels[**slot].text == "25.0")
        .unwrap();
    let label = &scene.labels[slot];
    assert_eq!((label.x, label.y, label.size), (182., 228., 18.));
    assert_eq!(label.color, Color(7, 8, 9, 255));
    let svg = kartis::render::to_svg(&scene).unwrap();
    assert!(
        svg.contains("stroke-width=\"5\"")
            && svg.contains("opacity=\"0.1\"")
            && svg.contains(">25.0</text>")
    );
    let hidden = scene.without_series(&BTreeSet::from(["area".into()]));
    assert!(hidden.label_series.is_empty());
    assert!(!hidden.labels.iter().any(|l| l.text == "25.0"));
    assert!(hidden.labels.iter().any(|l| l.text == "A"));
}

#[test]
fn stacked_bar_labels_use_actual_stack_endpoint_and_group_center() {
    let mut a = ComboSeries::new("a", ComboKind::Bar, vec![ComboPoint::new("A", 20.)]);
    a.style.point_labels = Some(ComboPointLabels {
        offset: [0., 0.],
        ..Default::default()
    });
    let mut b = ComboSeries::new("b", ComboKind::Bar, vec![ComboPoint::new("A", 30.)]);
    b.style.point_labels = a.style.point_labels.clone();
    let scene = combo(
        &config(),
        &[a, b],
        &ComboOptions {
            bar_layout: BarLayout::Stacked,
            ..categories()
        },
    )
    .unwrap();
    let slots = scene
        .label_series
        .iter()
        .filter(|(_, v)| *v == "b")
        .map(|(i, _)| *i)
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 1);
    assert_eq!(
        (scene.labels[slots[0]].x, scene.labels[slots[0]].y),
        (180., 170.)
    );
    assert_eq!(scene.labels[slots[0]].text, "30");
}

#[test]
fn invalid_spreads_styles_and_unsupported_profiles_are_transactionally_rejected() {
    let c = config();
    let o = categories();
    let invalid = [
        ComboSeries::new("a", ComboKind::ErrorBars, vec![ComboPoint::new("A", 50.)]),
        ComboSeries::new("a", ComboKind::Line, vec![ComboPoint::range("A", 0., 100.)]),
        ComboSeries::new(
            "a",
            ComboKind::BoxPlot,
            vec![ComboPoint::box_plot("A", [0., 80., 50., 60., 100.])],
        ),
        ComboSeries::new(
            "a",
            ComboKind::ErrorBars,
            vec![ComboPoint::new("A", 110.).with_bounds(0., 100.)],
        ),
        ComboSeries::new(
            "a",
            ComboKind::RangeBand,
            vec![ComboPoint::range("A", f64::NAN, 100.)],
        ),
    ];
    for series in invalid {
        assert!(combo(&c, &[series], &o).is_err());
    }
    let mut series = ComboSeries::new("a", ComboKind::Line, vec![ComboPoint::new("A", 50.)]);
    series.style.area_opacity = f64::NAN;
    assert!(combo(&c, std::slice::from_ref(&series), &o).is_err());
    series.style.area_opacity = 0.3;
    series.style.point_labels = Some(ComboPointLabels {
        offset: [f64::INFINITY, 0.],
        ..Default::default()
    });
    assert!(combo(&c, &[series], &o).is_err());
    let boxplot = ComboSeries::new(
        "a",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot(1., [1., 2., 3., 4., 5.])],
    );
    assert!(
        combo(
            &c,
            &[boxplot],
            &ComboOptions {
                x: ComboXAxis::Numeric {
                    domain: Some([1., 10.]),
                    scale: ScaleKind::Log
                },
                ..Default::default()
            }
        )
        .is_err()
    );
    let huge = ComboSeries::new(
        "a",
        ComboKind::RangeBand,
        vec![
            ComboPoint::range(-f64::MAX, 0., 100.),
            ComboPoint::range(f64::MAX, 0., 100.),
        ],
    );
    assert!(
        combo(
            &c,
            &[huge],
            &ComboOptions {
                x: ComboXAxis::Numeric {
                    domain: Some([0., 1.]),
                    scale: ScaleKind::Linear
                },
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn hiding_intervals_reflows_auto_domains_without_renaming_or_recoloring_data() {
    let intervals = ComboSeries::new(
        "interval",
        ComboKind::ErrorBars,
        vec![ComboPoint::new(0., 50.).with_bounds(0., 100.)],
    );
    let scatter = ComboSeries::new(
        "scatter",
        ComboKind::Scatter,
        vec![
            ComboPoint::new(0., 25.).identified("a"),
            ComboPoint::new(1., 75.).identified("b"),
        ],
    );
    let source = vec![intervals, scatter];
    let first = combo(&config(), &source, &ComboOptions::default()).unwrap();
    let hidden = combo(
        &config(),
        &source,
        &ComboOptions {
            hidden: BTreeSet::from(["interval".into()]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(first.legend, hidden.legend);
    assert!(data(&hidden, "interval").is_empty());
    let before = data(&first, "scatter")[0];
    let after = data(&hidden, "scatter")[0];
    assert_eq!(before.id, after.id);
    assert_eq!(before.fill, after.fill);
    close(center(before).1, 240.);
    close(center(after).1, 310.);
    let mut reversed = source.clone();
    reversed.reverse();
    for s in &mut reversed {
        s.data.reverse();
    }
    assert_eq!(
        first,
        combo(&config(), &reversed, &ComboOptions::default()).unwrap()
    );
}

#[test]
fn point_visibility_override_is_independent_of_owned_labels() {
    let mut line = ComboSeries::new(
        "line",
        ComboKind::Line,
        vec![ComboPoint::new("A", 20.), ComboPoint::new("B", 30.)],
    );
    line.style.points = Some(false);
    line.style.line_width = 0.;
    line.style.point_labels = Some(ComboPointLabels::default());
    let scene = combo(&config(), &[line], &categories()).unwrap();
    assert!(
        !data(&scene, "line")
            .iter()
            .any(|m| matches!(m.geometry, Geometry::Circle { .. }))
    );
    assert!(data(&scene, "line").iter().all(|m| m.stroke.is_none()));
    assert_eq!(scene.label_series.len(), 2);
    let mut range = ComboSeries::new(
        "range",
        ComboKind::RangeBand,
        vec![
            ComboPoint::range("A", 10., 30.),
            ComboPoint::range("B", 20., 50.),
        ],
    );
    range.style.points = Some(true);
    let scene = combo(
        &config(),
        &[range],
        &ComboOptions {
            line_points: false,
            ..categories()
        },
    )
    .unwrap();
    assert_eq!(
        data(&scene, "range")
            .iter()
            .filter(|m| matches!(m.geometry, Geometry::Circle { .. }))
            .count(),
        2
    );
}

#[test]
fn boxes_with_outside_centers_keep_real_visible_extent_and_independent_part_widths() {
    let mut series = ComboSeries::new(
        "box",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot(-0.05, [0., 20., 50., 80., 100.])],
    );
    series.style.line_width = 1.;
    series.style.median_width = 5.;
    series.style.median_color = Some(Color(10, 20, 200, 255));
    series.style.whisker_width = 3.;
    let scene = combo(
        &config(),
        &[series],
        &ComboOptions {
            x: ComboXAxis::Numeric {
                domain: Some([0., 1.]),
                scale: ScaleKind::Linear,
            },
            numeric_bar_width: Some(80.),
            left: YAxisOptions {
                domain: Some([0., 100.]),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let marks = data(&scene, "box");
    let body = *marks
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Rect { .. }))
        .unwrap();
    assert_eq!(center(body).0, 36.);
    assert_eq!(body.stroke.unwrap().width, 1.);
    assert_eq!(
        marks
            .iter()
            .find(|m| m.id.ends_with(":median"))
            .unwrap()
            .stroke
            .unwrap()
            .width,
        5.
    );
    assert_eq!(
        marks
            .iter()
            .find(|m| m.id.ends_with(":median"))
            .unwrap()
            .stroke
            .unwrap()
            .color,
        Color(10, 20, 200, 255)
    );
    assert_eq!(
        marks
            .iter()
            .find(|m| m.id.ends_with(":whiskers"))
            .unwrap()
            .stroke
            .unwrap()
            .width,
        3.
    );
    assert!(scene.hit_test(65., 170.).is_some_and(|m| m.series == "box"));
}

#[test]
fn degenerate_subnormal_summary_and_log_value_axis_remain_valid() {
    let tiny = f64::from_bits(1);
    let point = ComboPoint::range("A", tiny, tiny);
    assert_eq!(point.y, Some(tiny));
    let series = ComboSeries::new("range", ComboKind::RangeBand, vec![point]);
    assert!(combo(&config(), &[series], &categories()).is_ok());
    let series = ComboSeries::new(
        "box",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot(
            "A",
            [-10000., -1000., -100., -10., -1.],
        )],
    );
    let scene = combo(
        &config(),
        &[series],
        &ComboOptions {
            left: YAxisOptions {
                scale: ScaleKind::Log,
                domain: Some([-10000., -1.]),
                ..Default::default()
            },
            ..categories()
        },
    )
    .unwrap();
    scene.validate().unwrap();
    let body = *data(&scene, "box")
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Rect { .. }))
        .unwrap();
    close(center(body).1, 170.);
}

#[test]
fn clipped_box_keeps_true_quartile_edges_instead_of_drawing_false_domain_borders() {
    let series = ComboSeries::new(
        "box",
        ComboKind::BoxPlot,
        vec![ComboPoint::box_plot("A", [-100., -50., 20., 80., 150.])],
    );
    let scene = combo(&config(), &[series], &categories()).unwrap();
    let body = *data(&scene, "box")
        .iter()
        .find(|m| matches!(m.geometry, Geometry::Rect { .. }))
        .unwrap();
    let Geometry::Rect { y, height, .. } = body.geometry else {
        panic!()
    };
    close(y, 86.);
    close(y + height, 450.);
    assert_eq!(body.clip, Some(ClipRect::plot(&config())));
}
