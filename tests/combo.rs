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
fn category_options() -> ComboOptions {
    ComboOptions {
        x: ComboXAxis::Categories {
            values: vec!["A".into(), "B".into(), "C".into()],
            padding: 0.,
        },
        left: YAxisOptions {
            domain: Some([0., 100.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([0., 5000.]),
            axis: AxisOptions {
                format: ValueFormat::Affix {
                    prefix: "$".into(),
                    suffix: String::new(),
                    precision: 0,
                },
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
fn mixed() -> Vec<ComboSeries> {
    let mut scatter = ComboSeries::new(
        "Scatter",
        ComboKind::Scatter,
        vec![
            ComboPoint::new("A", 1000.),
            ComboPoint::new("B", 2000.),
            ComboPoint::new("C", 1500.),
        ],
    );
    scatter.axis = YAxisSide::Right;
    vec![
        ComboSeries::new(
            "Bars",
            ComboKind::Bar,
            vec![
                ComboPoint::new("A", 20.),
                ComboPoint::new("B", 40.),
                ComboPoint::new("C", 30.),
            ],
        ),
        ComboSeries::new(
            "Line",
            ComboKind::Line,
            vec![
                ComboPoint::new("A", 20.),
                ComboPoint::new("B", 40.),
                ComboPoint::new("C", 30.),
            ],
        ),
        ComboSeries::new(
            "Area",
            ComboKind::Area,
            vec![
                ComboPoint::new("A", 10.),
                ComboPoint::new("B", 30.),
                ComboPoint::new("C", 20.),
            ],
        ),
        scatter,
    ]
}
fn datum<'a>(scene: &'a Scene, series: &str, x: &str) -> &'a Mark {
    scene
        .marks
        .iter()
        .find(|m| m.series == series && m.id.ends_with(&format!("c{}:{x}", x.len())))
        .unwrap()
}
#[test]
fn four_kinds_share_categories_and_independent_value_axes_with_one_axis_set() {
    let c = config();
    let scene = combo(&c, &mixed(), &category_options()).unwrap();
    scene.validate().unwrap();
    assert_eq!(scene.legend.len(), 4);
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
    assert_eq!(
        scene
            .marks
            .iter()
            .filter(|m| m.id == "combo-category-axis")
            .count(),
        1
    );
    let bar = datum(&scene, "Bars", "A");
    let line = datum(&scene, "Line", "A");
    let scatter = datum(&scene, "Scatter", "A");
    let Geometry::Rect { x, y, width, .. } = bar.geometry else {
        panic!()
    };
    let Geometry::Circle { x: lx, y: ly, .. } = line.geometry else {
        panic!()
    };
    let Geometry::Circle { x: sx, y: sy, .. } = scatter.geometry else {
        panic!()
    };
    assert!((x + width / 2. - lx).abs() < 1e-10 && (lx - sx).abs() < 1e-10);
    assert!(
        (y - ly).abs() < 1e-10 && (ly - sy).abs() < 1e-10,
        "left20/100 and right1000/5000 must map to the same coordinate"
    );
    assert!(scatter.label.contains("$1000"));
    assert!(
        scene
            .marks
            .iter()
            .filter(|m| !m.series.is_empty())
            .all(|m| m.clip == Some(ClipRect::plot(&c)))
    );
    let area = scene
        .marks
        .iter()
        .position(|m| {
            m.series == "Area" && m.fill.is_some() && matches!(m.geometry, Geometry::Path(_))
        })
        .unwrap();
    let bars = scene.marks.iter().position(|m| m.series == "Bars").unwrap();
    let points = scene.marks.iter().position(|m| m.id == line.id).unwrap();
    assert!(area < bars && bars < points);
    glory::ssr::render(chart(scene.clone())).unwrap();
    render::to_svg(&scene).unwrap();
    render::to_canvas_script(&scene, "mixed").unwrap();
}
#[test]
fn source_reordering_and_hiding_keep_ids_colors_and_legend_identity() {
    let c = config();
    let input = mixed();
    let options = category_options();
    let first = combo(&c, &input, &options).unwrap();
    let mut reversed = input.clone();
    reversed.reverse();
    for s in &mut reversed {
        s.data.reverse();
    }
    let reordered = combo(&c, &reversed, &options).unwrap();
    assert_eq!(first, reordered);
    let hidden = combo(
        &c,
        &input,
        &ComboOptions {
            hidden: ["Bars".into()].into_iter().collect(),
            ..options
        },
    )
    .unwrap();
    assert_eq!(first.legend, hidden.legend);
    assert!(hidden.marks.iter().all(|m| m.series != "Bars"));
    let original = first
        .marks
        .iter()
        .filter(|m| m.series == "Line")
        .map(|m| (&m.id, m.fill, m.stroke))
        .collect::<Vec<_>>();
    let retained = hidden
        .marks
        .iter()
        .filter(|m| m.series == "Line")
        .map(|m| (&m.id, m.fill, m.stroke))
        .collect::<Vec<_>>();
    assert_eq!(original, retained);
}
#[test]
fn signed_stacks_and_distinct_value_axes_have_independent_slots() {
    let c = config();
    let mut options = category_options();
    options.bar_layout = BarLayout::Stacked;
    options.left.domain = Some([-10., 10.]);
    options.right.domain = Some([-100., 100.]);
    let mut data = vec![
        ComboSeries::new(
            "left-a",
            ComboKind::Bar,
            vec![ComboPoint::new("A", 2.), ComboPoint::new("B", -3.)],
        ),
        ComboSeries::new(
            "left-b",
            ComboKind::Bar,
            vec![ComboPoint::new("A", 4.), ComboPoint::new("B", -1.)],
        ),
        ComboSeries::new(
            "right",
            ComboKind::Bar,
            vec![ComboPoint::new("A", 20.), ComboPoint::new("B", -40.)],
        ),
    ];
    data[2].axis = YAxisSide::Right;
    let scene = combo(&c, &data, &options).unwrap();
    let ra = datum(&scene, "left-a", "A");
    let rb = datum(&scene, "left-b", "A");
    let rr = datum(&scene, "right", "A");
    let Geometry::Rect {
        x: xa,
        y: ya,
        width: wa,
        height: ha,
    } = ra.geometry
    else {
        panic!()
    };
    let Geometry::Rect {
        x: xb,
        y: yb,
        width: wb,
        height: hb,
    } = rb.geometry
    else {
        panic!()
    };
    let Geometry::Rect {
        x: xr, width: wr, ..
    } = rr.geometry
    else {
        panic!()
    };
    assert_eq!(xa, xb);
    assert_eq!(wa, wb);
    assert!((yb + hb - ya).abs() < 1e-10);
    assert!((hb / ha - 2.).abs() < 1e-10);
    assert!(xr >= xa + wa && wr > 0.);
    options.bar_layout = BarLayout::Percent;
    options.left.domain = Some([-1., 1.]);
    options.right.domain = Some([-1., 1.]);
    let percent = combo(&c, &data, &options).unwrap();
    assert!(
        datum(&percent, "left-a", "A")
            .label
            .contains("33.3% (raw 2)")
    );
    assert_eq!(datum(&percent, "left-a", "A").value, 2.);
}
#[test]
fn sparse_categories_are_gaps_but_numeric_lines_remain_irregular_samples() {
    let c = config();
    let categorical = [ComboSeries::new(
        "sparse",
        ComboKind::Line,
        vec![ComboPoint::new("A", 20.), ComboPoint::new("C", 40.)],
    )];
    let scene = combo(&c, &categorical, &category_options()).unwrap();
    let lines = scene
        .marks
        .iter()
        .filter(|m| m.series == "sparse" && matches!(m.geometry, Geometry::Path(_)))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(
        lines.iter().all(|m| matches!(
            m.geometry.commands().as_slice(),
            [
                glory::render::PathCommand::Move(_),
                glory::render::PathCommand::Close
            ]
        )),
        "each category remains a singleton run, with no segment bridging the missing category"
    );
    let numeric = [
        ComboSeries::new(
            "sparse",
            ComboKind::Line,
            vec![ComboPoint::new(0., 20.), ComboPoint::new(2., 40.)],
        ),
        ComboSeries::new(
            "bars",
            ComboKind::Bar,
            vec![
                ComboPoint::new(0., 10.),
                ComboPoint::new(1., 20.),
                ComboPoint::new(2., 10.),
            ],
        ),
    ];
    let scene = combo(&c, &numeric, &ComboOptions::default()).unwrap();
    let lines = scene
        .marks
        .iter()
        .filter(|m| m.series == "sparse" && matches!(m.geometry, Geometry::Path(_)))
        .collect::<Vec<_>>();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].geometry.commands().len(), 2);
}
#[test]
fn dense_scatter_does_not_shrink_numeric_bars_and_utc_clipping_works() {
    let c = config();
    let bars = ComboSeries::new(
        "bars",
        ComboKind::Bar,
        vec![ComboPoint::new(0., 10.), ComboPoint::new(10., 20.)],
    );
    let opts = ComboOptions {
        x: ComboXAxis::Numeric {
            domain: Some([-5., 15.]),
            scale: ScaleKind::Linear,
        },
        ..Default::default()
    };
    let a = combo(&c, std::slice::from_ref(&bars), &opts).unwrap();
    let cloud = ComboSeries::new(
        "cloud",
        ComboKind::Scatter,
        (0..100)
            .map(|i| ComboPoint::new(i as f64 / 10., 5.))
            .collect(),
    );
    let b = combo(&c, &[bars, cloud], &opts).unwrap();
    let width = |s: &Scene| {
        s.marks
            .iter()
            .filter(|m| m.series == "bars")
            .map(|m| match m.geometry {
                Geometry::Rect { width, .. } => width,
                _ => panic!(),
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(width(&a), width(&b));
    let start = 1709164800000.;
    let utc = [
        ComboSeries::new(
            "daily",
            ComboKind::Bar,
            vec![
                ComboPoint::new(start, 10.),
                ComboPoint::new(start + 86400000., 20.),
            ],
        ),
        ComboSeries::new(
            "line",
            ComboKind::Line,
            vec![
                ComboPoint::new(start - 86400000., 5.),
                ComboPoint::new(start + 2. * 86400000., 25.),
            ],
        ),
    ];
    let scene = combo(
        &c,
        &utc,
        &ComboOptions {
            x: ComboXAxis::Numeric {
                domain: Some([start, start + 86400000.]),
                scale: ScaleKind::Utc,
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        scene
            .labels
            .iter()
            .any(|l| l.text.starts_with("2024-02-29"))
    );
    assert!(
        scene
            .marks
            .iter()
            .filter(|m| m.series == "daily")
            .all(|m| m.label.contains("2024-"))
    );
}
#[test]
fn invalid_profiles_and_ambiguous_series_fail_explicitly() {
    let c = config();
    let duplicate = [ComboSeries::new(
        "bad",
        ComboKind::Line,
        vec![ComboPoint::new(-0., 1.), ComboPoint::new(0., 2.)],
    )];
    assert!(combo(&c, &duplicate, &ComboOptions::default()).is_err());
    let wrong = [ComboSeries::new(
        "bad",
        ComboKind::Line,
        vec![ComboPoint::new("unknown", 1.)],
    )];
    assert!(combo(&c, &wrong, &category_options()).is_err());
    let bars = [ComboSeries::new(
        "bad",
        ComboKind::Bar,
        vec![ComboPoint::new(1., 1.)],
    )];
    assert!(
        combo(
            &c,
            &bars,
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
    assert!(
        combo(
            &c,
            &bars,
            &ComboOptions {
                left: YAxisOptions {
                    scale: ScaleKind::Log,
                    domain: Some([1., 10.]),
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        combo(
            &c,
            &mixed(),
            &ComboOptions {
                series_order: vec!["missing".into()],
                ..category_options()
            }
        )
        .is_err()
    );
}

#[test]
fn category_rotation_and_explicit_order_are_kept_while_points_are_keyed_by_x() {
    let c = config();
    let mut options = category_options();
    options.series_order = vec![
        "Scatter".into(),
        "Line".into(),
        "Bars".into(),
        "Area".into(),
    ];
    options.category_axis.tick_rotation = -35.;
    options.x_side = AxisSide::Top;
    let scene = combo(&c, &mixed(), &options).unwrap();
    assert_eq!(
        scene
            .legend
            .iter()
            .map(|l| l.id.as_str())
            .collect::<Vec<_>>(),
        vec!["Scatter", "Line", "Bars", "Area"]
    );
    assert_eq!(
        scene
            .labels
            .iter()
            .find(|l| l.text == "A")
            .unwrap()
            .rotation,
        -35.
    );
    let original = datum(&scene, "Line", "C").id.clone();
    let mut input = mixed();
    input[1].data.remove(0);
    let next = combo(&c, &input, &options).unwrap();
    assert_eq!(
        datum(&next, "Line", "C").id,
        original,
        "removing an earlier category must not rename retained data"
    );
    let explicit = combo(
        &c,
        &[ComboSeries::new(
            "bar",
            ComboKind::Bar,
            vec![ComboPoint::new(1., 1.)],
        )],
        &ComboOptions {
            numeric_bar_width: Some(30.),
            bar_width_ratio: 0.,
            ..Default::default()
        },
    )
    .unwrap();
    let Geometry::Rect { width, .. } = explicit
        .marks
        .iter()
        .find(|m| m.series == "bar")
        .unwrap()
        .geometry
    else {
        panic!()
    };
    assert_eq!(width, 30.);
}

#[test]
fn scatter_observations_can_share_x_when_explicit_ids_disambiguate_them() {
    let c = config();
    let points = ComboSeries::new(
        "observations",
        ComboKind::Scatter,
        vec![
            ComboPoint::new("A", 20.).identified("first"),
            ComboPoint::new("A", 40.).identified("second"),
        ],
    );
    let first = combo(&c, std::slice::from_ref(&points), &category_options()).unwrap();
    let data = first
        .marks
        .iter()
        .filter(|m| m.series == "observations")
        .collect::<Vec<_>>();
    assert_eq!(data.len(), 2);
    assert_ne!(data[0].id, data[1].id);
    let (Geometry::Circle { x: a, y: ay, .. }, Geometry::Circle { x: b, y: by, .. }) =
        (&data[0].geometry, &data[1].geometry)
    else {
        panic!()
    };
    assert_eq!(a, b);
    assert_ne!(ay, by);
    let mut reversed = points.clone();
    reversed.data.reverse();
    assert_eq!(first, combo(&c, &[reversed], &category_options()).unwrap());
    let mut shifted = points;
    shifted.data[0].x = ComboXValue::Category("B".into());
    let moved = combo(&c, &[shifted], &category_options()).unwrap();
    assert_eq!(
        data.iter()
            .map(|m| &m.id)
            .collect::<std::collections::BTreeSet<_>>(),
        moved
            .marks
            .iter()
            .filter(|m| m.series == "observations")
            .map(|m| &m.id)
            .collect()
    );
    let outlier = ComboSeries::new(
        "scatter",
        ComboKind::Scatter,
        vec![ComboPoint::new(0.5, 5.), ComboPoint::new(f64::MAX, 5.)],
    );
    let rendered = combo(
        &c,
        &[outlier],
        &ComboOptions {
            curve: Curve::Natural,
            x: ComboXAxis::Numeric {
                domain: Some([0., 1.]),
                scale: ScaleKind::Linear,
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        rendered
            .marks
            .iter()
            .filter(|m| m.series == "scatter")
            .count(),
        1,
        "scatter points must not run irrelevant smoothing over huge outliers"
    );
}
