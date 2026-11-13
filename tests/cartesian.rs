use kartis::scales::*;
use kartis::*;

fn series() -> Vec<Series> {
    vec![Series::new(
        "first",
        vec![
            Point::new(0., 2.),
            Point::new(1., 3.),
            Point::gap(2.),
            Point::new(3., 1.),
        ],
    )]
}

#[test]
fn continuous_scales_roundtrip_reversed_clamped_and_constant() {
    let linear = LinearScale::new([-20., 40.], [200., 0.]).unwrap();
    for v in [-20., -1., 0., 12., 40.] {
        assert!((linear.invert(linear.map(v)) - v).abs() < 1e-10);
    }
    assert_eq!(linear.clamped(true).map(100.), 0.);
    assert_eq!(LinearScale::new([5., 5.], [0., 100.]).unwrap().map(9.), 50.);
    let log = LogScale::new([1., 1000.], [0., 1.]).unwrap();
    assert!((log.map(10.) - 1. / 3.).abs() < 1e-12);
    assert!((log.invert(log.map(24.)) - 24.).abs() < 1e-10);
    let negative = LogScale::new([-100., -1.], [1., 0.]).unwrap();
    assert!((negative.invert(negative.map(-13.)) + 13.).abs() < 1e-10);
    assert!(LogScale::new([-1., 1.], [0., 1.]).is_err());
    let symlog = SymLogScale::new([-100., 100.], [0., 1000.], 1.).unwrap();
    for v in [-100., -1., 0., 10., 100.] {
        assert!((symlog.invert(symlog.map(v)) - v).abs() < 1e-10);
    }
    assert!(LinearScale::new([-f64::MAX, f64::MAX], [0., 1.]).is_err());
    assert!(LinearScale::new([f64::NAN, 1.], [0., 1.]).is_err());
    assert!(ticks([10., 0.], 5).windows(2).all(|w| w[0] > w[1]));
}

#[test]
fn band_scales_roundtrip_reversed_and_reject_duplicates() {
    for range in [[0., 100.], [100., 0.]] {
        let band = BandScale::new(vec!["a".into(), "b".into()], range, 0.2).unwrap();
        for key in ["a", "b"] {
            let p = band.map(key).unwrap();
            assert_eq!(band.invert(p + band.bandwidth / 2.), Some(key));
        }
        assert!(band.invert(-1.).is_none());
    }
    assert!(BandScale::new(vec!["a".into(), "a".into()], [0., 100.], 0.).is_err());
    assert!(BandScale::new(vec![], [0., 100.], 1.).is_err());
}

#[test]
fn line_gaps_curves_areas_and_chunking_preserve_semantic_limits() {
    let config = ChartConfig::default();
    for curve in [Curve::Linear, Curve::Step, Curve::Monotone] {
        let scene = line(
            &config,
            &series(),
            &LineOptions {
                curve,
                area: true,
                ..LineOptions::default()
            },
        )
        .unwrap();
        let paths = scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("line-"))
            .collect::<Vec<_>>();
        assert_eq!(
            paths.len(),
            2,
            "a gap splits the area into separate subpaths"
        );
        assert_eq!(
            scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("point-"))
                .count(),
            3
        );
        for mark in scene.marks {
            assert!(mark.geometry.commands().len() <= 256);
            if mark.id.starts_with("area-") {
                assert!(
                    mark.stroke.is_none(),
                    "chunk boundaries must not have visible seam strokes"
                );
            }
            if mark.id.starts_with("line-") {
                assert!(mark.fill.is_none());
                let commands = mark.geometry.commands();
                if commands.contains(&glory::render::PathCommand::Close) {
                    // D3 closes a singleton run. No multi-point outline may
                    // acquire a synthetic closing edge across a gap/baseline.
                    assert!(matches!(
                        commands.as_slice(),
                        [
                            glory::render::PathCommand::Move(_),
                            glory::render::PathCommand::Close,
                        ]
                    ));
                }
            }
        }
    }
    let points = (0..600)
        .map(|i| Point::new(i as f64, (i as f64).sin()))
        .collect();
    let scene = line(
        &config,
        &[Series::new("many", points)],
        &LineOptions {
            curve: Curve::Step,
            area: true,
            ..LineOptions::default()
        },
    )
    .unwrap();
    assert!(
        scene
            .marks
            .iter()
            .all(|m| m.geometry.commands().len() <= 256)
    );
    assert!(
        line(
            &config,
            &[Series::new(
                "bad",
                vec![Point::new(1., 1.), Point::new(1., 2.)]
            )],
            &LineOptions {
                curve: Curve::Monotone,
                ..LineOptions::default()
            }
        )
        .is_err()
    );
    assert!(
        line(
            &config,
            &[Series::new("bad", vec![Point::new(0., f64::NAN)])],
            &LineOptions::default()
        )
        .is_err()
    );
    assert!(line(&config, &[], &LineOptions::default()).is_ok());
    let outside = [Series::new(
        "outside",
        vec![
            Point::new(-f64::MAX, -f64::MAX),
            Point::new(f64::MAX, f64::MAX),
        ],
    )];
    let clipped = line(
        &config,
        &outside,
        &LineOptions {
            x_domain: Some([0., 1.]),
            y_domain: Some([0., 1.]),
            ..LineOptions::default()
        },
    )
    .unwrap();
    assert!(
        clipped.marks.iter().all(|m| !m.id.starts_with("point-")),
        "explicit domains exclude samples before potentially overflowing scale extrapolation"
    );
}

#[test]
fn signed_stacks_use_independent_positive_negative_baselines() {
    let config = ChartConfig::default();
    let categories = vec!["A".into()];
    let data = vec![
        BarSeries::new("positive1", vec![Some(2.)]),
        BarSeries::new("negative1", vec![Some(-3.)]),
        BarSeries::new("positive2", vec![Some(4.)]),
        BarSeries::new("negative2", vec![Some(-1.)]),
    ];
    for horizontal in [false, true] {
        let scene = bar(
            &config,
            &categories,
            &data,
            &BarOptions {
                layout: BarLayout::Stacked,
                horizontal,
                ..BarOptions::default()
            },
        )
        .unwrap();
        let marks = scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("bar-"))
            .collect::<Vec<_>>();
        assert_eq!(marks.len(), 4);
        let length = |m: &Mark| match m.geometry {
            Geometry::Rect { width, height, .. } => {
                if horizontal {
                    width
                } else {
                    height
                }
            }
            _ => panic!("expected rectangle"),
        };
        assert!((length(marks[2]) / length(marks[0]) - 2.).abs() < 1e-12);
        assert!((length(marks[1]) / length(marks[3]) - 3.).abs() < 1e-12);
        assert!(marks.iter().all(|m| length(m) > 0.));
    }
    assert!(
        bar(
            &config,
            &categories,
            &[BarSeries::new("bad", vec![])],
            &BarOptions::default()
        )
        .is_err()
    );
    assert!(
        bar(
            &config,
            &categories,
            &[
                BarSeries::new("big", vec![Some(f64::MAX)]),
                BarSeries::new("big2", vec![Some(f64::MAX)])
            ],
            &BarOptions {
                layout: BarLayout::Stacked,
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn heatmap_duplicate_missing_square_and_domain() {
    let config = ChartConfig::default();
    let cells = vec![
        HeatCell::new("r1", "c1", 0.),
        HeatCell::new("r1", "c2", 10.),
        HeatCell {
            row: "r2".into(),
            column: "c1".into(),
            value: None,
        },
    ];
    let scene = heatmap(
        &config,
        &cells,
        &HeatmapOptions {
            square: true,
            labels: true,
            ..HeatmapOptions::default()
        },
    )
    .unwrap();
    assert_eq!(scene.marks.len(), 3);
    match scene.marks[0].geometry {
        Geometry::Rect { width, height, .. } => assert!((width - height).abs() < 1e-12),
        _ => panic!("expected rect"),
    };
    assert_eq!(scene.marks[2].fill, Some(HeatmapOptions::default().missing));
    assert_ne!(scene.marks[0].fill, scene.marks[1].fill);
    assert!(
        heatmap(
            &config,
            &[cells[0].clone(), cells[0].clone()],
            &HeatmapOptions::default()
        )
        .is_err()
    );
}

#[test]
fn stream_modes_and_bump_rank_validation() {
    let config = ChartConfig::default();
    let categories = vec!["a".into(), "b".into(), "c".into()];
    let data = vec![
        BarSeries::new("one", vec![Some(2.), None, Some(3.)]),
        BarSeries::new("two", vec![Some(4.), Some(1.), Some(5.)]),
    ];
    for offset in [
        StreamOffset::Zero,
        StreamOffset::Center,
        StreamOffset::Expand,
    ] {
        let scene = stream(
            &config,
            &categories,
            &data,
            &StreamOptions {
                offset,
                ..StreamOptions::default()
            },
        )
        .unwrap();
        assert_eq!(
            scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("stream-"))
                .count(),
            2
        );
    }
    let ranks = vec![Series::new(
        "rank",
        vec![Point::new(0., 1.), Point::new(1., 2.)],
    )];
    let scene = bump(&config, &ranks, &LineOptions::default()).unwrap();
    let positions = scene
        .marks
        .iter()
        .filter_map(|m| {
            if m.id.starts_with("point-") {
                match m.geometry {
                    Geometry::Circle { y, .. } => Some(y),
                    _ => None,
                }
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert!(positions[0] < positions[1]);
    assert!(
        bump(
            &config,
            &[Series::new("rank", vec![Point::new(0., 0.5)])],
            &LineOptions::default()
        )
        .is_err()
    );
}
