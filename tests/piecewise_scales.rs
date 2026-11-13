use kartis::{
    AxisOptions, AxisSide, ChartConfig, ChartScale, ContinuousScaleOptions, Geometry,
    PiecewiseScaleOptions, ResolvedContinuousScale, ScaleKind, ScaleNice, axis,
};

fn values(s: &str) -> Vec<f64> {
    if s.is_empty() {
        vec![]
    } else {
        s.split(',').map(|v| v.parse().unwrap()).collect()
    }
}
fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        actual == expected
            || actual.is_nan() && expected.is_nan()
            || (actual - expected).abs() <= expected.abs().max(1.) * 2e-12,
        "{actual:?} != {expected:?}: {context}"
    );
}
fn equal(actual: &[f64], expected: &[f64], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}");
    for (&a, &b) in actual.iter().zip(expected) {
        close(a, b, context);
    }
}

#[test]
fn original_d3_piecewise_oracle_maps_inverse_nice_ticks_round_clamp_and_branches() {
    let mut checked = 0;
    let mut rejected = 0;
    let mut rounding_boundaries = 0;
    let differences = include_str!("fixtures/piecewise-scale-rounding-differences.csv")
        .lines()
        .filter(|r| !r.starts_with('#'))
        .map(|r| r.split('|').collect::<Vec<_>>())
        .collect::<Vec<_>>();
    for (record, row) in include_str!("fixtures/piecewise-scale-d3.csv")
        .lines()
        .filter(|r| !r.starts_with('#'))
        .enumerate()
    {
        let c = row.split('|').collect::<Vec<_>>();
        let param = c[2].parse().unwrap();
        let kind = match c[1] {
            "linear" => ScaleKind::Linear,
            "log" => ScaleKind::Log,
            "symlog" => ScaleKind::SymLog { constant: param },
            "pow" => ScaleKind::Pow { exponent: param },
            "sqrt" => ScaleKind::Sqrt,
            "utc" => ScaleKind::Utc,
            _ => panic!("unknown oracle scale"),
        };
        let options = PiecewiseScaleOptions {
            domain: values(c[3]),
            range: values(c[4]),
            continuous: ContinuousScaleOptions {
                kind,
                nice: match c[5] {
                    "-1" => ScaleNice::Off,
                    "0" => ScaleNice::Default,
                    n => ScaleNice::Count(n.parse().unwrap()),
                },
                reverse: c[6] == "1",
                round: c[7] == "1",
                clamp: c[8] == "1",
                log_base: if kind == ScaleKind::Log { param } else { 10. },
            },
        };
        let resolved = ResolvedContinuousScale::piecewise(&options);
        if c[0] == "R" {
            assert!(resolved.is_err(), "{row}");
            rejected += 1;
            continue;
        }
        let s = resolved.unwrap_or_else(|e| panic!("{e:?}: {row}"));
        assert!(s.is_piecewise());
        equal(s.source_domain_stops(), &options.domain, row);
        equal(s.display_domain_stops(), &values(c[9]), row);
        equal(s.range_stops(), &values(c[10]), row);
        let mut unrounded_options = options.clone();
        unrounded_options.continuous.round = false;
        let unrounded = ResolvedContinuousScale::piecewise(&unrounded_options).unwrap();
        for (query_index, ((q, expected), expected_raw)) in values(c[11])
            .into_iter()
            .zip(values(c[12]))
            .zip(values(c[19]))
            .enumerate()
        {
            let actual = s.map(q);
            let raw = unrounded.map(q);
            close(raw, expected_raw, row);
            if options.continuous.round && actual != expected {
                let difference = differences.iter().find(|d| d[0].parse::<usize>().unwrap() == record && d[1].parse::<usize>().unwrap() == query_index)
                    .unwrap_or_else(|| panic!("unlisted cross-runtime rounded difference at record {record}, query {query_index}: {row}"));
                assert_eq!(
                    expected_raw.to_bits(),
                    u64::from_str_radix(difference[5], 16).unwrap(),
                    "original D3 raw bits changed"
                );
                assert_eq!(
                    expected,
                    difference[4].parse::<f64>().unwrap(),
                    "original D3 rounded value changed"
                );
                assert_eq!(q, difference[6].parse::<f64>().unwrap());
                assert_eq!(actual, difference[2].parse::<f64>().unwrap());
                // Cross-runtime transcendental functions may straddle a half
                // pixel by one ULP. Require independent raw agreement and both
                // values within one ULP of the SAME half; never permit a
                // generic one-pixel tolerance or alter the original oracle.
                let half = expected_raw.floor() + 0.5;
                let ulp = f64::from_bits(half.abs().to_bits() + 1) - half.abs();
                assert!(
                    (raw - half).abs() <= ulp && (expected_raw - half).abs() <= ulp,
                    "rounded disagreement away from a one-ULP half boundary: {actual} != {expected}, raw {raw} != {expected_raw}: {row}"
                );
                assert_eq!((actual - expected).abs(), 1., "{row}");
                assert_eq!(actual, (raw + 0.5).floor(), "{row}");
                assert_eq!(expected, (expected_raw + 0.5).floor(), "{row}");
                rounding_boundaries += 1;
            } else if options.continuous.round {
                assert_eq!(actual, expected, "{row}");
            } else {
                close(actual, expected, row);
            }
        }
        assert_eq!(s.is_invertible(), c[18] == "1", "{row}");
        for (pixel, expected) in values(c[13]).into_iter().zip(values(c[14])) {
            if s.is_invertible() && expected.is_finite() {
                close(
                    s.invert(pixel).unwrap_or_else(|e| panic!("{e:?}: {row}")),
                    expected,
                    row,
                );
            } else {
                assert!(s.invert(pixel).is_err(), "{row}");
            }
        }
        equal(&s.ticks(7), &values(c[15]), row);
        for (index, (pixel, expected)) in values(c[16]).into_iter().zip(values(c[17])).enumerate() {
            let inverse = s.invert_segment(index, pixel);
            if s.range_stops()[index] == s.range_stops()[index + 1] {
                assert!(inverse.is_err(), "{row}");
            } else {
                close(inverse.unwrap(), expected, row);
            }
        }
        checked += 1;
    }
    assert_eq!((checked, rejected), (2400, 160));
    println!(
        "D3 configurations: {checked} accepted, {rejected} checked-profile rejections; {rounding_boundaries} explicitly checked one-ULP rounding boundaries"
    );
}

#[test]
fn invalid_breakpoints_and_transform_collapse_are_rejected() {
    for (domain, range) in [
        (vec![], vec![]),
        (vec![0.], vec![0.]),
        (vec![0., 1., 2.], vec![0., 1.]),
        (vec![0., 1., 1.], vec![0., 1., 2.]),
        (vec![0., 2., 1.], vec![0., 1., 2.]),
        (vec![0., f64::NAN, 2.], vec![0., 1., 2.]),
        (vec![0., 1., 2.], vec![0., f64::INFINITY, 2.]),
        (vec![-f64::MAX, 0., f64::MAX], vec![0., 1., 2.]),
        (
            (0..4097).map(f64::from).collect(),
            (0..4097).map(f64::from).collect(),
        ),
    ] {
        assert!(
            ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
                domain,
                range,
                ..Default::default()
            })
            .is_err()
        );
    }
    for (kind, domain) in [
        (ScaleKind::Log, vec![-2., -1., 1.]),
        (ScaleKind::Pow { exponent: -1. }, vec![-2., -1., 1.]),
        (ScaleKind::Pow { exponent: 1e300 }, vec![1., 2., 3.]),
        (ScaleKind::Pow { exponent: 1e-300 }, vec![1., 2., 3.]),
        (ScaleKind::Utc, vec![0., 1.5, 3.]),
        (ScaleKind::Utc, vec![0., 1., 9e15]),
    ] {
        assert!(
            ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
                domain,
                range: vec![0., 1., 2.],
                continuous: ContinuousScaleOptions {
                    kind,
                    ..Default::default()
                },
            })
            .is_err()
        );
    }
}

#[test]
fn nonmonotonic_range_has_checked_branch_inverses_and_valid_forward_axes() {
    let scale = ChartScale::with_piecewise_options(&PiecewiseScaleOptions {
        domain: vec![0., 10., 20.],
        range: vec![40., 240., 40.],
        ..Default::default()
    })
    .unwrap();
    close(scale.map(5.), 140., "first branch");
    close(scale.map(15.), 140., "second branch");
    assert!(scale.invert(140.).is_err());
    close(
        scale.invert_segment(0, 140.).unwrap(),
        5.,
        "chosen first inverse",
    );
    close(
        scale.invert_segment(1, 140.).unwrap(),
        15.,
        "chosen second inverse",
    );
    assert!(scale.invert_segment(2, 140.).is_err());
    assert!(scale.invert_segment(0, f64::NAN).is_err());
    let mut scene = ChartConfig::default().scene().unwrap();
    axis(
        &mut scene,
        &ChartConfig::default(),
        &scale,
        AxisSide::Bottom,
        &AxisOptions {
            tick_values: Some(vec![0., 5., 10., 15., 20.]),
            ..Default::default()
        },
        "piecewise",
    )
    .unwrap();
    let xs = scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("piecewise-tick-"))
        .map(|m| {
            if let Geometry::Path(path) = &m.geometry
                && let glory::render::PathCommand::Move(p) = path[0]
            {
                return p.x;
            }
            panic!("tick path missing")
        })
        .collect::<Vec<_>>();
    equal(
        &xs,
        &[40., 140., 240., 140., 40.],
        "actual shared axis positions",
    );
}

#[test]
fn nice_preserves_interior_stops_and_reverse_changes_every_range_stop() {
    let options = PiecewiseScaleOptions {
        domain: vec![-0.71, -0.1, 0.01],
        range: vec![0.2, 0.7, 1.9],
        continuous: ContinuousScaleOptions {
            nice: ScaleNice::Default,
            reverse: true,
            ..Default::default()
        },
    };
    let s = ResolvedContinuousScale::piecewise(&options).unwrap();
    assert_eq!(s.display_domain_stops(), &[-0.8, -0.1, 0.1]);
    assert_eq!(s.range_stops(), &[1.9, 0.7, 0.2]);
    assert_eq!(s.source_domain_stops(), &[-0.71, -0.1, 0.01]);
    assert_eq!(options.range, vec![0.2, 0.7, 1.9]);
}
