use glory::render::{GraphicPoint as P, PathCommand as Cmd};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
#[test]
fn axis_rotation_reaches_real_tick_labels_and_rejects_invalid_proposals() {
    let c = ChartConfig::default();
    let data = [Series::new(
        "s",
        vec![Point::new(0., 0.), Point::new(1., 1.)],
    )];
    let mut options = LineOptions::default();
    options.axes.x.tick_rotation = -45.;
    options.axes.y.tick_rotation = 30.;
    let s = line(&c, &data, &options).unwrap();
    assert!(s.labels.iter().any(|label| label.rotation == -45.));
    assert!(s.labels.iter().any(|label| label.rotation == 30.));
    options.axes.x.tick_rotation = f64::NAN;
    assert!(line(&c, &data, &options).is_err());
    let mut bars = BarOptions::default();
    bars.category_axis.tick_rotation = 60.;
    let categorical = bar(
        &c,
        &["January".into()],
        &[BarSeries::new("s", vec![Some(3.)])],
        &bars,
    )
    .unwrap();
    assert!(
        categorical
            .labels
            .iter()
            .any(|label| label.text == "January" && label.rotation == 60.)
    );
}
fn paths(curve: Curve, values: &[(f64, f64)]) -> Vec<Cmd> {
    let scene = line(
        &config(),
        &[Series::new(
            "s",
            values.iter().map(|(x, y)| Point::new(*x, *y)).collect(),
        )],
        &LineOptions {
            curve,
            points: false,
            x_domain: Some([0., 100.]),
            y_domain: Some([0., 100.]),
            ..Default::default()
        },
    )
    .unwrap();
    scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("line-"))
        .flat_map(|m| m.geometry.commands())
        .collect()
}
#[test]
fn selected_axis_sides_place_real_geometry_and_validate_orientation() {
    let c = ChartConfig::default();
    let o = LineOptions {
        axes: CartesianAxes {
            x_side: AxisSide::Top,
            y_side: AxisSide::Right,
            ..Default::default()
        },
        ..Default::default()
    };
    let scene = line(
        &c,
        &[Series::new(
            "s",
            vec![Point::new(0., 0.), Point::new(1., 1.)],
        )],
        &o,
    )
    .unwrap();
    let x = scene
        .marks
        .iter()
        .find(|m| m.id == "axis-x")
        .unwrap()
        .geometry
        .commands();
    assert!(
        x.iter()
            .all(|p| matches!(p,Cmd::Move(q)|Cmd::Line(q) if q.y==c.margin.top))
    );
    let y = scene
        .marks
        .iter()
        .find(|m| m.id == "axis-y")
        .unwrap()
        .geometry
        .commands();
    assert!(
        y.iter()
            .all(|p| matches!(p,Cmd::Move(q)|Cmd::Line(q) if q.x==c.width-c.margin.right))
    );
    let bad = LineOptions {
        axes: CartesianAxes {
            x_side: AxisSide::Left,
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(line(&c, &[], &bad).is_err());
    let ranks = bump(
        &c,
        &[Series::new(
            "ranks",
            vec![Point::new(0., 1.), Point::new(1., 2.)],
        )],
        &LineOptions {
            axes: CartesianAxes {
                y_side: AxisSide::Right,
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
    let point = ranks.marks.iter().find(|m| m.id == "point-0-0").unwrap();
    let Geometry::Circle { y, .. } = point.geometry else {
        panic!()
    };
    let label = ranks
        .labels
        .iter()
        .find(|l| l.text == "1" && l.anchor == TextAnchor::Start)
        .unwrap();
    assert!((label.y - y - c.theme.font_size / 3.).abs() < 1e-9);
}
#[test]
fn dual_axes_share_x_but_map_values_independently_and_keep_series_identity() {
    let c = ChartConfig::default();
    let data = vec![
        DualAxisSeries::new(
            Series::new(
                "temperature",
                vec![Point::new(0., 0.), Point::new(10., 100.)],
            ),
            YAxisSide::Left,
        ),
        DualAxisSeries::new(
            Series::new(
                "revenue",
                vec![Point::new(0., 1000.), Point::new(10., 2000.)],
            ),
            YAxisSide::Right,
        ),
    ];
    let o = DualAxisOptions {
        left: YAxisOptions {
            domain: Some([0., 100.]),
            ..Default::default()
        },
        right: YAxisOptions {
            domain: Some([1000., 2000.]),
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
    };
    let scene = dual_axis(&c, &data, &o).unwrap();
    let points = scene
        .marks
        .iter()
        .filter(|m| matches!(m.geometry, Geometry::Circle { .. }))
        .collect::<Vec<_>>();
    assert_eq!(points.len(), 4);
    assert_eq!(points[0].geometry, points[2].geometry);
    assert_eq!(points[1].geometry, points[3].geometry);
    assert!(points[2].label.contains("$1000"));
    assert_ne!(points[0].fill, points[2].fill);
    assert_eq!(scene.legend[0].id, "temperature");
    let mut swapped = data.clone();
    swapped[0].axis = YAxisSide::Right;
    swapped[1].axis = YAxisSide::Left;
    let swapped = dual_axis(&c, &swapped, &DualAxisOptions::default()).unwrap();
    let ids = scene
        .marks
        .iter()
        .filter(|m| !m.series.is_empty())
        .map(|m| m.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    let other = swapped
        .marks
        .iter()
        .filter(|m| !m.series.is_empty())
        .map(|m| m.id.clone())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(ids, other);
}
#[test]
fn basis_and_catmull_rom_match_primary_d3_control_point_golden_values() {
    // D3's basis and Catmull-Rom source equations; projected points are (0,0),(6,6),(12,0).
    let input = [(0., 100.), (6., 94.), (12., 100.)];
    let basis = paths(Curve::Basis, &input);
    let golden = vec![
        Cmd::Move(P::new(0., 0.)),
        Cmd::Line(P::new(1., 1.)),
        Cmd::Cubic {
            first: P::new(2., 2.),
            second: P::new(4., 4.),
            end: P::new(6., 4.),
        },
        Cmd::Cubic {
            first: P::new(8., 4.),
            second: P::new(10., 2.),
            end: P::new(11., 1.),
        },
        Cmd::Line(P::new(12., 0.)),
    ];
    assert_eq!(basis.len(), golden.len());
    let close = |a: P, b: P| assert!((a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12);
    for (actual, expected) in basis.iter().zip(golden) {
        match (*actual, expected) {
            (Cmd::Move(a), Cmd::Move(b)) | (Cmd::Line(a), Cmd::Line(b)) => close(a, b),
            (
                Cmd::Cubic {
                    first: a,
                    second: b,
                    end: c,
                },
                Cmd::Cubic {
                    first: x,
                    second: y,
                    end: z,
                },
            ) => {
                close(a, x);
                close(b, y);
                close(c, z);
            }
            _ => panic!("curve command topology differs from the D3 golden path"),
        }
    }
    let cat = paths(Curve::CatmullRom { alpha: 0.5 }, &input);
    assert_eq!(cat.len(), 3);
    let Cmd::Cubic { first, second, end } = cat[1] else {
        panic!()
    };
    assert!((first.x - 0.).abs() < 1e-12 && first.y.abs() < 1e-12);
    assert!((second.x - 4.).abs() < 1e-12 && (second.y - 6.).abs() < 1e-12);
    assert!((end.x - 6.).abs() < 1e-12 && (end.y - 6.).abs() < 1e-12);
    assert_eq!(
        paths(Curve::CatmullRom { alpha: 0. }, &input),
        paths(Curve::Cardinal, &input)
    );
    assert_eq!(paths(Curve::Bundle { beta: 1. }, &input), basis);
    let bundled = paths(Curve::Bundle { beta: 0. }, &input);
    assert!(bundled.iter().all(|p| {
        match p {
            Cmd::Move(p) | Cmd::Line(p) => p.y.abs() < 1e-12,
            Cmd::Cubic { first, second, end } => [first.y, second.y, end.y]
                .into_iter()
                .all(|y| y.abs() < 1e-12),
            Cmd::Close => false,
        }
    }));
}
#[test]
fn new_curves_keep_gaps_and_exact_chunk_topology_with_semantic_budgets() {
    let c = ChartConfig::default();
    for curve in [
        Curve::Basis,
        Curve::CatmullRom { alpha: 0.5 },
        Curve::Bundle { beta: 0.85 },
    ] {
        let data = Series::new(
            "large",
            (0..900)
                .map(|i| {
                    if i == 450 {
                        Point::gap(i as f64)
                    } else {
                        Point::new(i as f64, (i as f64 * 0.1).sin())
                    }
                })
                .collect(),
        );
        let scene = line(
            &c,
            &[data],
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
        assert!(scene.marks.iter().any(|m| m.id.starts_with("line-0-1-")));
        glory::ssr::render(chart(scene)).unwrap();
    }
    assert!(
        line(
            &c,
            &[],
            &LineOptions {
                curve: Curve::CatmullRom { alpha: f64::NAN },
                ..Default::default()
            }
        )
        .is_err()
    );
}
