use kartis::mesh::*;
use kartis::*;

#[test]
fn clipping_preserves_tiny_axis_values_beside_maximal_other_axis() {
    let y = 1e-308;
    let clipped = clipping::clip_segment(
        (-f64::MAX, y),
        (f64::MAX, y),
        ClipRect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 2. * y,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(clipped, [(0., y), (1., y)]);
    let transposed = clipping::clip_segment(
        (y, -f64::MAX),
        (y, f64::MAX),
        ClipRect {
            x: 0.,
            y: 0.,
            width: 2. * y,
            height: 1.,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(transposed, [(y, 0.), (y, 1.)]);
    let c = ChartConfig::default();
    let scene = line(
        &c,
        &[Series::new(
            "tiny",
            vec![Point::new(-f64::MAX, y), Point::new(f64::MAX, y)],
        )],
        &LineOptions {
            x_domain: Some([0., 1.]),
            y_domain: Some([0., 2. * y]),
            ..Default::default()
        },
    )
    .unwrap();
    let path = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-"))
        .unwrap()
        .geometry
        .commands();
    for command in path {
        if let glory::render::PathCommand::Move(p) | glory::render::PathCommand::Line(p) = command {
            assert!((p.y - (c.margin.top + c.inner_height() / 2.)).abs() < 1e-9);
        }
    }
}

#[test]
fn conditional_mesh_clips_cannot_silently_export_incorrect_voronoi_cells() {
    let mesh = PointMesh::new(vec![
        MeshPoint {
            id: "a".into(),
            x: 0.,
            y: 0.,
            clip: None,
        },
        MeshPoint {
            id: "b".into(),
            x: 10.,
            y: 0.,
            clip: Some(ClipRect {
                x: 10.,
                y: 0.,
                width: 10.,
                height: 10.,
            }),
        },
    ])
    .unwrap();
    assert_eq!(mesh.nearest(9., 0., f64::INFINITY).unwrap().id, "a");
    assert!(
        mesh.voronoi(ClipRect {
            x: 0.,
            y: 0.,
            width: 20.,
            height: 10.
        })
        .is_err(),
        "a nearer but inactive competitor must not steal another site's cell"
    );
}

#[test]
fn voronoi_normalizes_subnormal_site_separation() {
    let unit = 1e-308;
    let mesh = PointMesh::new(vec![
        MeshPoint {
            id: "a".into(),
            x: unit,
            y: 0.,
            clip: None,
        },
        MeshPoint {
            id: "b".into(),
            x: 3. * unit,
            y: 0.,
            clip: None,
        },
    ])
    .unwrap();
    let cells = mesh
        .voronoi(ClipRect {
            x: 0.,
            y: 0.,
            width: 4. * unit,
            height: unit,
        })
        .unwrap();
    let right = cells[0]
        .polygon
        .iter()
        .map(|p| p.0)
        .fold(f64::NEG_INFINITY, f64::max);
    assert!(
        (right / unit - 2.).abs() < 1e-12,
        "the bisector must remain at the midpoint of tiny sites"
    );
    assert_eq!(mesh.nearest(1.5 * unit, 0., f64::INFINITY).unwrap().id, "a");
}

#[test]
fn signed_zero_ticks_share_one_label_and_extended_year_formats_keep_signs() {
    let c = ChartConfig::default();
    let mut scene = c.scene().unwrap();
    let scale = ChartScale::new(
        ScaleKind::Linear,
        [-1., 1.],
        [c.margin.left, c.width - c.margin.right],
    )
    .unwrap();
    axis(
        &mut scene,
        &c,
        &scale,
        AxisSide::Bottom,
        &AxisOptions {
            tick_values: Some(vec![-0., 0.]),
            ..Default::default()
        },
        "zero",
    )
    .unwrap();
    assert_eq!(scene.labels.iter().filter(|l| l.text == "0").count(), 1);
    let bc = -62_167_219_200_000_f64;
    assert_eq!(ValueFormat::UtcYear.format(bc).unwrap(), "0000");
    assert_eq!(ValueFormat::UtcYearMonth.format(bc).unwrap(), "0000-01");
}

#[test]
fn utc_value_axis_and_point_tooltips_use_the_same_date_format() {
    let c = ChartConfig::default();
    let scene = line(
        &c,
        &[Series::new(
            "dated",
            vec![
                Point::new(0., 1709164800000.),
                Point::new(1., 1709251200000.),
            ],
        )],
        &LineOptions {
            y_scale: ScaleKind::Utc,
            ..Default::default()
        },
    )
    .unwrap();
    let point = scene.marks.iter().find(|m| m.id == "point-0-0").unwrap();
    assert!(point.label.contains("2024-02-29"));
    assert!(
        scene
            .labels
            .iter()
            .any(|l| l.text.starts_with("2024-02-29"))
    );
}

#[test]
fn explicit_heatmap_color_domain_can_clamp_extreme_values() {
    let c = ChartConfig::default();
    let options = HeatmapOptions {
        domain: Some([0., 1.]),
        ..Default::default()
    };
    let scene = heatmap(&c, &[HeatCell::new("row", "column", f64::MAX)], &options).unwrap();
    assert_eq!(scene.marks[0].fill, Some(options.high));
}

#[test]
fn bump_tick_labels_follow_theme_font_baselines_after_rank_reversal() {
    let mut c = ChartConfig::default();
    c.theme.font_size = 30.;
    let scene = bump(
        &c,
        &[Series::new(
            "ranks",
            vec![Point::new(0., 1.), Point::new(1., 2.)],
        )],
        &LineOptions {
            axes: CartesianAxes {
                y: AxisOptions {
                    tick_values: Some(vec![1., 2.]),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let point = scene.marks.iter().find(|m| m.id == "point-0-0").unwrap();
    let Geometry::Circle { y, .. } = point.geometry else {
        panic!()
    };
    let tick = scene
        .labels
        .iter()
        .find(|l| l.text == "1" && l.anchor == TextAnchor::End)
        .unwrap();
    assert!((tick.y - y - c.theme.font_size / 3.).abs() < 1e-9);
}

#[test]
fn huge_opposite_endpoints_clip_without_losing_a_unit_diagonal() {
    let result = clipping::clip_segment(
        (-f64::MAX, -f64::MAX),
        (f64::MAX, f64::MAX),
        ClipRect {
            x: 0.,
            y: 0.,
            width: 1.,
            height: 1.,
        },
    )
    .unwrap()
    .unwrap();
    assert_eq!(result, [(0., 0.), (1., 1.)]);
    let c = ChartConfig::default();
    let s = line(
        &c,
        &[Series::new(
            "huge",
            vec![
                Point::new(-f64::MAX, -f64::MAX),
                Point::new(f64::MAX, f64::MAX),
            ],
        )],
        &LineOptions {
            x_domain: Some([0., 1.]),
            y_domain: Some([0., 1.]),
            ..Default::default()
        },
    )
    .unwrap();
    let Geometry::Path(path) = &s
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-"))
        .unwrap()
        .geometry
    else {
        panic!()
    };
    assert_eq!(path.len(), 2);
}

#[test]
fn category_ticks_and_symlog_bar_axes_are_configurable() {
    let c = ChartConfig::default();
    let s = bar(
        &c,
        &["A".into(), "B".into(), "C".into()],
        &[BarSeries::new("v", vec![Some(-100.), Some(0.), Some(100.)])],
        &BarOptions {
            value_scale: ScaleKind::SymLog { constant: 1. },
            value_axis: AxisOptions {
                format: ValueFormat::Affix {
                    prefix: "$".into(),
                    suffix: String::new(),
                    precision: 0,
                },
                grid: false,
                tick_values: Some(vec![-100., 0., 100.]),
                ..Default::default()
            },
            category_axis: CategoryAxisOptions {
                tick_values: Some(vec!["A".into(), "C".into()]),
                legend: Some("Categories".into()),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.labels.iter().any(|l| l.text == "$100"));
    assert!(!s.labels.iter().any(|l| l.text == "B"));
    assert!(s.labels.iter().any(|l| l.text == "Categories"));
    assert!(s.marks.iter().all(|m| !m.id.contains("grid")));
    assert_eq!(ValueFormat::UtcDate.format(0.).unwrap(), "1970-01-01");
    assert_eq!(ValueFormat::UtcTime.format(0.).unwrap(), "00:00:00.000");
}

#[test]
fn shared_scales_feed_actual_chart_axes_and_tooltips() {
    let c = ChartConfig::default();
    let data = [Series::new(
        "power",
        vec![
            Point::new(1., -9.),
            Point::new(10., 0.),
            Point::new(100., 9.),
        ],
    )];
    let scene = line(
        &c,
        &data,
        &LineOptions {
            x_scale: ScaleKind::Log,
            y_scale: ScaleKind::SymLog { constant: 1. },
            axes: CartesianAxes {
                x: AxisOptions {
                    tick_values: Some(vec![1., 10., 100.]),
                    format: ValueFormat::Affix {
                        prefix: "x=".into(),
                        suffix: String::new(),
                        precision: 0,
                    },
                    ..Default::default()
                },
                y: AxisOptions {
                    format: ValueFormat::Fixed(2),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert!(scene.labels.iter().any(|l| l.text == "x=10"));
    let point = scene.marks.iter().find(|m| m.id == "point-0-1").unwrap();
    let Geometry::Circle { x, y, .. } = point.geometry else {
        panic!()
    };
    assert!((x - (c.margin.left + c.inner_width() / 2.)).abs() < 1e-9);
    assert!((y - (c.margin.top + c.inner_height() / 2.)).abs() < 1e-9);
    assert!(point.label.contains("0.00"));
    let mut bad = data.to_vec();
    bad[0].data[0].x = 0.;
    assert!(
        line(
            &c,
            &bad,
            &LineOptions {
                x_scale: ScaleKind::Log,
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn utc_ticks_and_custom_format_limits_are_enforced() {
    let c = ChartConfig::default();
    let data = [Series::new(
        "time",
        vec![
            Point::new(1709164800000., 1.),
            Point::new(1709251200000., 2.),
        ],
    )];
    let scene = line(
        &c,
        &data,
        &LineOptions {
            x_scale: ScaleKind::Utc,
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
    assert_eq!(
        ValueFormat::Grouped(2).format(-1234567.25).unwrap(),
        "-1,234,567.25"
    );
    assert_eq!(ValueFormat::Percent(1).format(0.123).unwrap(), "12.3%");
    assert_eq!(ValueFormat::Compact(1).format(12345.).unwrap(), "12.3k");
    assert!(ValueFormat::Fixed(100000).format(3.).is_err());
}
#[test]
fn domain_crossing_line_is_retained_and_clipped_in_every_backend() {
    let c = ChartConfig::default();
    let scene = line(
        &c,
        &[Series::new(
            "crossing",
            vec![Point::new(-1., 0.5), Point::new(2., 0.5)],
        )],
        &LineOptions {
            x_domain: Some([0., 1.]),
            y_domain: Some([0., 1.]),
            ..Default::default()
        },
    )
    .unwrap();
    let line = scene
        .marks
        .iter()
        .find(|m| m.id.starts_with("line-"))
        .unwrap();
    assert_eq!(line.clip, Some(ClipRect::plot(&c)));
    let Geometry::Path(commands) = &line.geometry else {
        panic!()
    };
    assert_eq!(commands.len(), 2);
    assert!(scene.marks.iter().all(|m| !m.id.starts_with("point-")));
    assert!(
        render::to_svg(&scene)
            .unwrap()
            .contains("viewBox=\"64 32 712 356\"")
    );
    let script = render::to_canvas_script(&scene, "plot").unwrap();
    assert!(script.contains("ctx.rect(64,32,712,356);ctx.clip();"));
    assert!(glory::ssr::render(chart(scene)).is_ok());
}
#[test]
fn clip_controls_hit_and_brush_selection() {
    let mut s = ChartConfig::default().scene().unwrap();
    let mut m = Mark::new(
        "clipped",
        Geometry::Circle {
            x: 30.,
            y: 30.,
            radius: 20.,
        },
        Color(1, 2, 3, 255),
    );
    m.series = "s".into();
    m.clip = Some(ClipRect {
        x: 30.,
        y: 0.,
        width: 30.,
        height: 100.,
    });
    s.marks.push(m);
    assert!(s.hit_test(20., 30.).is_none());
    assert!(s.hit_test(40., 30.).is_some());
    s.marks[0].clip.as_mut().unwrap().x = 40.;
    assert!(
        annotations::Brush::from_points((0., 0.), (100., 100.))
            .unwrap()
            .select(&s)
            .unwrap()
            .is_empty()
    );
}
#[test]
fn nearest_mesh_matches_bruteforce_and_voronoi_partitions_area() {
    let points: Vec<_> = (0..200)
        .map(|i| MeshPoint {
            id: format!("{i:03}"),
            x: ((i * 37) % 101) as f64,
            y: ((i * 61) % 97) as f64,
            clip: None,
        })
        .collect();
    let mesh = PointMesh::new(points.clone()).unwrap();
    for i in 0..80 {
        let (x, y) = ((i * 13 % 100) as f64, (i * 17 % 100) as f64);
        let expected = points
            .iter()
            .min_by(|a, b| {
                (a.x - x)
                    .hypot(a.y - y)
                    .total_cmp(&(b.x - x).hypot(b.y - y))
                    .then_with(|| a.id.cmp(&b.id))
            })
            .unwrap();
        assert_eq!(mesh.nearest(x, y, f64::INFINITY).unwrap().id, expected.id);
    }
    let mesh = PointMesh::new(vec![
        MeshPoint {
            id: "a".into(),
            x: 25.,
            y: 50.,
            clip: None,
        },
        MeshPoint {
            id: "b".into(),
            x: 75.,
            y: 50.,
            clip: None,
        },
    ])
    .unwrap();
    let cells = mesh
        .voronoi(ClipRect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        })
        .unwrap();
    for cell in cells {
        let area: f64 = cell
            .polygon
            .iter()
            .zip(cell.polygon.iter().cycle().skip(1))
            .map(|(a, b)| a.0 * b.1 - b.0 * a.1)
            .sum::<f64>()
            .abs()
            / 2.;
        assert!((area - 5000.).abs() < 1e-9);
    }
    let slices = SliceIndex::new(&mesh, SliceAxis::X);
    assert_eq!(slices.nearest(50., 100.).unwrap().ids, vec!["a"]);
}
#[test]
fn all_curve_variants_keep_path_budget_and_missing_segments() {
    let c = ChartConfig::default();
    for curve in [
        Curve::Linear,
        Curve::Step,
        Curve::StepBefore,
        Curve::StepAfter,
        Curve::Monotone,
        Curve::Cardinal,
        Curve::Natural,
    ] {
        let scene = line(
            &c,
            &[Series::new(
                "curve",
                (0..700)
                    .map(|i| {
                        if i == 350 {
                            Point::gap(i as f64)
                        } else {
                            Point::new(i as f64, (i as f64 * 0.03).sin())
                        }
                    })
                    .collect(),
            )],
            &LineOptions {
                curve,
                area: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(
            scene
                .marks
                .iter()
                .all(|m| m.geometry.commands().len() <= 256)
        );
        assert!(glory::ssr::render(chart(scene)).is_ok());
    }
}
