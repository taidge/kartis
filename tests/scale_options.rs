use kartis::{
    CategoryKey, ChartScale, ContinuousScaleOptions, PointScale, PointScaleOptions,
    ResolvedContinuousScale, ScaleKind, ScaleNice,
};
fn close(a: f64, b: f64) {
    let tolerance = if b == 0. {
        1e-12
    } else {
        (b.abs() * 2e-12).max(f64::from_bits(1) * 2.)
    };
    assert!(
        (a.is_nan() && b.is_nan()) || a == b || (a - b).abs() <= tolerance,
        "{a:?} != {b:?}"
    );
}
fn number(s: &str) -> f64 {
    s.parse().unwrap()
}
fn values(s: &str) -> Vec<f64> {
    if s.is_empty() {
        vec![]
    } else {
        s.split(',').map(number).collect()
    }
}
#[test]
fn actual_d3_continuous_and_point_oracle() {
    let mut rows = 0;
    for line in include_str!("fixtures/scale-options-d3.csv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        let c = line.split('|').collect::<Vec<_>>();
        if c[0] == "C" {
            let kind = match c[1] {
                "linear" => ScaleKind::Linear,
                "log" => ScaleKind::Log,
                "symlog" => ScaleKind::SymLog {
                    constant: number(c[2]),
                },
                "utc" => ScaleKind::Utc,
                _ => panic!(),
            };
            let o = ContinuousScaleOptions {
                kind,
                log_base: number(c[2]),
                nice: match c[7] {
                    "-1" => ScaleNice::Off,
                    "0" => ScaleNice::Default,
                    s => ScaleNice::Count(s.parse().unwrap()),
                },
                reverse: c[8] == "1",
                round: c[9] == "1",
                clamp: c[10] == "1",
            };
            // Only log uses the base; use the valid default for other kinds.
            let o = ContinuousScaleOptions {
                log_base: if kind == ScaleKind::Log {
                    o.log_base
                } else {
                    10.
                },
                ..o
            };
            let s = ResolvedContinuousScale::new(
                [number(c[3]), number(c[4])],
                [number(c[5]), number(c[6])],
                &o,
            )
            .unwrap();
            close(s.display_domain()[0], number(c[11]));
            close(s.display_domain()[1], number(c[12]));
            for (q, want) in values(c[13]).into_iter().zip(values(c[14])) {
                if want.is_finite() {
                    close(s.map(q), want);
                } else {
                    assert!(!s.map(q).is_finite());
                }
            }
            for (q, want) in values(c[15]).into_iter().zip(values(c[16])) {
                let a = s.invert(q).unwrap();
                assert!(
                    (a - want).abs() <= 1e-9_f64.max(want.abs() * 2e-12),
                    "inverse {q}: {a} != {want}; {line}"
                );
            }
            let actual = s.ticks(c[17].parse().unwrap());
            let expected = values(c[18]);
            assert_eq!(
                actual.len(),
                expected.len(),
                "tick count: {line}\nactual={actual:?}"
            );
            for (a, b) in actual.into_iter().zip(expected) {
                close(a, b);
            }
        } else if c[0] == "T" {
            let kind = match c[1] {
                "linear" => ScaleKind::Linear,
                "log" => ScaleKind::Log,
                "symlog" => ScaleKind::SymLog {
                    constant: number(c[2]),
                },
                "utc" => ScaleKind::Utc,
                _ => panic!(),
            };
            let s = ResolvedContinuousScale::new(
                [number(c[3]), number(c[4])],
                [0., 1.],
                &ContinuousScaleOptions {
                    kind,
                    log_base: if kind == ScaleKind::Log {
                        number(c[2])
                    } else {
                        10.
                    },
                    ..Default::default()
                },
            )
            .unwrap();
            let actual = s.ticks(c[5].parse().unwrap());
            let expected = values(c[6]);
            assert_eq!(
                actual.len(),
                expected.len(),
                "tick oracle {line}\nactual {actual:?}"
            );
            for (a, b) in actual.into_iter().zip(expected) {
                close(a, b);
            }
        } else {
            let n = c[1].parse::<usize>().unwrap();
            let s = PointScale::new(
                (0..n)
                    .map(|i| CategoryKey::String(format!("k{i}")))
                    .collect(),
                [number(c[6]), number(c[7])],
                &PointScaleOptions {
                    padding: number(c[2]),
                    align: number(c[3]),
                    round: c[4] == "1",
                    reverse: c[5] == "1",
                },
            )
            .unwrap();
            close(s.step(), number(c[8]));
            for (i, want) in values(c[9]).into_iter().enumerate() {
                close(s.at(i).unwrap(), want);
            }
            assert_eq!(s.at(n), None);
        }
        rows += 1;
    }
    assert_eq!(rows, 1132);
}
#[test]
fn round_negative_half_zero_and_unrounded_inverse() {
    let resolved = ResolvedContinuousScale::new(
        [0.13, 870.],
        [5., 205.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            nice: ScaleNice::Default,
            reverse: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(resolved.source_domain(), [0.13, 870.]);
    assert_eq!(resolved.display_domain(), [0.1, 1000.]);
    assert_eq!(resolved.range(), [205., 5.]);
    assert_eq!(resolved.map(0.1), 205.);
    let s = ChartScale::with_options(
        [0., 4.],
        [-1.5, 2.5],
        &ContinuousScaleOptions {
            round: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s.map(0.), -1.);
    assert_eq!(s.map(1.).to_bits(), (-0_f64).to_bits());
    assert_eq!(s.map(2.), 1.);
    close(s.invert(0.1).unwrap(), 1.6);
    // Old constructor still uses the old logarithmic transformed tick profile.
    let legacy = ChartScale::new(ScaleKind::Log, [1., 100.], [0., 1.]).unwrap();
    let current = ChartScale::with_options(
        [1., 100.],
        [0., 1.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            ..Default::default()
        },
    )
    .unwrap();
    assert_ne!(legacy.ticks(10), current.ticks(10));
    assert_eq!(current.ticks(10).len(), 19);
}
#[test]
fn categorical_identity_dedup_and_no_bandwidth() {
    let s = PointScale::new(
        vec![
            "1".into(),
            1_f64.into(),
            (-0_f64).into(),
            0_f64.into(),
            "1".into(),
            "".into(),
        ],
        [0., 90.],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(s.domain().len(), 4);
    assert_eq!(s.map(&"1".into()), Some(0.));
    assert_eq!(s.map(&1_f64.into()), Some(30.));
    assert_eq!(s.map(&0_f64.into()), Some(60.));
    assert_eq!(s.map(&"missing".into()), None);
    let single = PointScale::new(vec!["only".into()], [90., -10.], &Default::default()).unwrap();
    assert_eq!(single.at(0), Some(40.));
}
#[test]
fn invalid_inputs_rejected_and_outputs_bounded() {
    for d in [
        [f64::NAN, 1.],
        [f64::NEG_INFINITY, 0.],
        [-f64::MAX, f64::MAX],
    ] {
        assert!(ResolvedContinuousScale::new(d, [0., 1.], &Default::default()).is_err());
    }
    for base in [0., 1., f64::NAN, 1001.] {
        assert!(
            ResolvedContinuousScale::new(
                [1., 2.],
                [0., 1.],
                &ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    log_base: base,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    for d in [[0., 1.], [-1., 1.]] {
        assert!(
            ResolvedContinuousScale::new(
                d,
                [0., 1.],
                &ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(PointScale::new(vec![f64::NAN.into()], [0., 1.], &Default::default()).is_err());
    assert!(
        PointScale::new(
            vec![],
            [0., 1.],
            &PointScaleOptions {
                align: 1.1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        ResolvedContinuousScale::new(
            [0., 1.],
            [0., 1.],
            &ContinuousScaleOptions {
                nice: ScaleNice::Count(0),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        ResolvedContinuousScale::new(
            [0.5, 100.],
            [0., 1.],
            &ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                ..Default::default()
            }
        )
        .is_err()
    );
    let s = ResolvedContinuousScale::new(
        [1., 1000.],
        [0., 100.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(s.ticks(usize::MAX).len() <= 2000);
    assert!(s.invert(f64::INFINITY).is_err());
    assert!(s.map(-1.).is_nan());
}
#[test]
fn utc_nice_calendar_and_inverse_timeclip() {
    let day = 86400000.;
    let s = ResolvedContinuousScale::new(
        [day, day * 26.],
        [0., 100.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            nice: ScaleNice::Count(4),
            ..Default::default()
        },
    )
    .unwrap();
    // D3 UTC weeks begin Sunday (1970-01-04), unlike the legacy Monday ticks.
    assert_eq!(s.display_domain(), [-4. * day, 31. * day]);
    assert_eq!(
        s.ticks(4),
        vec![
            -4. * day,
            3. * day,
            10. * day,
            17. * day,
            24. * day,
            31. * day
        ]
    );
    let t = ResolvedContinuousScale::new(
        [-10., 10.],
        [0., 20.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(t.invert(9.4).unwrap(), 0.);
    assert_eq!(t.invert(9.4).unwrap().to_bits(), 0_f64.to_bits());
    assert_eq!(t.invert(8.4).unwrap(), -1.);
}
#[test]
fn extreme_symlog_and_log_tick_budgets_fail_atomically_on_axis() {
    let s = ResolvedContinuousScale::new(
        [-1e300, 1e300],
        [0., 100.],
        &ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 1e-300 },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(s.map(0.), 50.);
    close(s.invert(s.map(1e200)).unwrap(), 1e200);
    // A mathematically valid scale can have far more minor ticks than an axis
    // can admit. Reject before adding geometry, preserving the accepted Scene.
    let s = ResolvedContinuousScale::new(
        [1e-200, 1e200],
        [20., 500.],
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            log_base: 1000.,
            ..Default::default()
        },
    )
    .unwrap();
    let mut scene = kartis::ChartConfig::default().scene().unwrap();
    let before = scene.clone();
    assert!(
        kartis::axis(
            &mut scene,
            &Default::default(),
            s.chart_scale(),
            kartis::AxisSide::Bottom,
            &kartis::AxisOptions {
                tick_count: 1000,
                ..Default::default()
            },
            "test"
        )
        .is_err()
    );
    assert_eq!(scene, before);
}
#[test]
fn descending_axes_and_configured_clamping_share_real_tick_pixels() {
    let c = kartis::ChartConfig::default();
    let s = ResolvedContinuousScale::new(
        [8., -2.],
        [c.margin.left, c.width - c.margin.right],
        &ContinuousScaleOptions {
            round: true,
            clamp: true,
            nice: ScaleNice::Default,
            ..Default::default()
        },
    )
    .unwrap();
    let mut scene = c.scene().unwrap();
    kartis::axis(
        &mut scene,
        &c,
        s.chart_scale(),
        kartis::AxisSide::Bottom,
        &kartis::AxisOptions {
            tick_values: Some(vec![-2., 0., 8.]),
            ..Default::default()
        },
        "descending",
    )
    .unwrap();
    assert_eq!(scene.labels.len(), 3);
    for label in &scene.labels {
        let value = label.text.parse::<f64>().unwrap();
        assert_eq!(label.x, s.map(value));
    }
    for d in [[1., 100.], [-100., -1.], [100., 1.], [-1., -100.]] {
        let s = ResolvedContinuousScale::new(
            d,
            [10., 90.],
            &ContinuousScaleOptions {
                kind: ScaleKind::Log,
                clamp: true,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(s.chart_scale().accepts(0.));
        let boundary = if d[0].abs() < d[1].abs() { 10. } else { 90. };
        assert_eq!(s.map(0.), boundary);
    }
}
