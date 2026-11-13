use kartis::scales::PowScale;
use kartis::*;

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
fn kind(name: &str, exponent: f64) -> ScaleKind {
    if name == "sqrt" {
        ScaleKind::Sqrt
    } else {
        ScaleKind::Pow { exponent }
    }
}
fn close(actual: f64, expected: f64, context: &str) {
    assert!(
        actual == expected || (actual - expected).abs() <= expected.abs().max(1.) * 2e-12,
        "{actual:?} != {expected:?}: {context}"
    );
}
fn inverse(scale: &ResolvedContinuousScale, pixels: &[f64], expected: &[f64], context: &str) {
    assert_eq!(pixels.len(), expected.len());
    for (&pixel, &value) in pixels.iter().zip(expected) {
        if value.is_finite() {
            close(scale.invert(pixel).unwrap(), value, context);
        } else {
            assert!(scale.invert(pixel).is_err(), "{context}");
        }
    }
}

#[test]
fn independent_d3_power_mapping_inverse_nice_round_clamp_and_raw_ticks() {
    let mut configs = 0;
    let mut records = 0;
    for row in include_str!("fixtures/power-scale-d3.csv")
        .lines()
        .filter(|l| !l.starts_with('#'))
    {
        records += 1;
        let c = row.split('|').collect::<Vec<_>>();
        if c[0] == "Z" {
            let domain = [number(c[3]), number(c[4])];
            let kind = kind(c[1], number(c[2]));
            let mut v = DomainViewport::new(
                ViewportAxes {
                    x: ViewportAxis::new(domain, kind),
                    left: Some(ViewportAxis::new(domain, kind)),
                    right: None,
                },
                ClipRect {
                    x: 40.,
                    y: 30.,
                    width: 200.,
                    height: 200.,
                },
                ViewportLimits::default(),
            )
            .unwrap()
            .with_directions(ViewportDirections {
                x: c[5] == "1",
                left: c[6] == "1",
                right: false,
            })
            .unwrap();
            v.apply(ViewportRequest::Zoom {
                factor: 2.,
                anchor: (70., 90.),
                axes: ViewportSelection::Both,
            })
            .unwrap();
            if c[7] == "1" {
                v.apply(ViewportRequest::Pan {
                    delta: (20., -10.),
                    axes: ViewportSelection::Both,
                })
                .unwrap();
            }
            let d = v.domains();
            for (a, b) in [d.x[0], d.x[1], d.left.unwrap()[0], d.left.unwrap()[1]]
                .into_iter()
                .zip(&c[8..12])
            {
                close(a, number(b), row);
            }
        } else if c[0] == "I" {
            let s = ResolvedContinuousScale::new(
                [number(c[1]), number(c[2])],
                [0., 200.],
                &ContinuousScaleOptions {
                    kind: ScaleKind::Pow { exponent: -1. },
                    clamp: c[3] == "1",
                    ..Default::default()
                },
            )
            .unwrap();
            inverse(&s, &values(c[4]), &values(c[5]), row);
        } else if c[0] == "T" {
            let s = ResolvedContinuousScale::new(
                [number(c[3]), number(c[4])],
                [0., 1.],
                &ContinuousScaleOptions {
                    kind: kind(c[1], number(c[2])),
                    ..Default::default()
                },
            )
            .unwrap();
            let actual = s.ticks(c[5].parse().unwrap());
            let expected = values(c[6]);
            assert_eq!(actual.len(), expected.len(), "{row}");
            for (a, b) in actual.into_iter().zip(expected) {
                close(a, b, row);
            }
        } else {
            configs += 1;
            let o = ContinuousScaleOptions {
                kind: kind(c[1], number(c[2])),
                nice: match c[7] {
                    "-1" => ScaleNice::Off,
                    "0" => ScaleNice::Default,
                    s => ScaleNice::Count(s.parse().unwrap()),
                },
                reverse: c[8] == "1",
                round: c[9] == "1",
                clamp: c[10] == "1",
                ..Default::default()
            };
            let source = [number(c[3]), number(c[4])];
            let range = [number(c[5]), number(c[6])];
            let result = ResolvedContinuousScale::new(source, range, &o);
            if c[0] == "R" {
                assert!(result.is_err(), "{row}");
                continue;
            }
            let s = result.unwrap();
            assert_eq!(s.source_domain(), source);
            assert_eq!(s.display_domain(), [number(c[11]), number(c[12])]);
            let queries = values(c[13]);
            let expected = values(c[14]);
            assert_eq!(queries.len(), expected.len());
            for (q, want) in queries.iter().zip(&expected) {
                if o.round {
                    assert_eq!(s.map(*q), *want, "{row}; q={q}");
                } else {
                    close(s.map(*q), *want, row);
                }
            }
            inverse(&s, &values(c[15]), &values(c[16]), row);
            let actual = s.ticks(7);
            let expected_ticks = values(c[17]);
            assert_eq!(actual.len(), expected_ticks.len(), "{row}");
            for (a, b) in actual.into_iter().zip(expected_ticks) {
                close(a, b, row);
            }
            if o.nice == ScaleNice::Off && !o.round {
                let range = if o.reverse {
                    [range[1], range[0]]
                } else {
                    range
                };
                let direct = PowScale::new(source, range, number(c[2]))
                    .unwrap()
                    .clamped(o.clamp);
                for (q, want) in queries.iter().zip(expected) {
                    close(direct.map(*q), want, row);
                }
            }
        }
    }
    assert_eq!(configs, 736);
    assert_eq!(records, 895, "truncated oracle");
}

#[test]
fn reciprocal_navigation_rejects_crossing_an_asymptote_without_mutation() {
    let mut v = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([1., 4.], ScaleKind::Pow { exponent: -1. }),
            left: None,
            right: None,
        },
        ClipRect {
            x: 0.,
            y: 0.,
            width: 200.,
            height: 200.,
        },
        ViewportLimits {
            bounded: false,
            ..Default::default()
        },
    )
    .unwrap();
    let accepted = v.clone();
    assert!(
        v.apply(ViewportRequest::Pan {
            delta: (-400., 0.),
            axes: ViewportSelection::X
        })
        .is_err()
    );
    assert_eq!(v, accepted);
    v.apply(ViewportRequest::Zoom {
        factor: 2.,
        anchor: (70., 90.),
        axes: ViewportSelection::X,
    })
    .unwrap();
    let zoomed = v.clone();
    assert!(
        v.apply(ViewportRequest::Pan {
            delta: (-1000., 0.),
            axes: ViewportSelection::X
        })
        .is_err()
    );
    assert_eq!(v, zoomed);
    v.apply(ViewportRequest::Reset).unwrap();
    assert_eq!(v, accepted);
}

#[test]
fn finite_profiles_reject_noninvertible_or_singular_domains_and_keep_valid_constants() {
    for exponent in [
        0.,
        -0.,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::from_bits(1),
    ] {
        assert!(PowScale::new([1., 4.], [0., 100.], exponent).is_err());
        assert!(
            ContinuousScaleOptions {
                kind: ScaleKind::Pow { exponent },
                ..Default::default()
            }
            .validate()
            .is_err()
        );
    }
    for domain in [[-1., 1.], [0., 4.], [-4., 0.], [0., 0.]] {
        assert!(PowScale::new(domain, [0., 100.], -1.).is_err());
    }
    assert!(PowScale::new([1e200, 2e200], [0., 1.], 2.).is_err());
    assert!(PowScale::new([1e-200, 2e-200], [0., 1.], 2.).is_err());
    assert!(PowScale::new([1., 2.], [0., 1.], 1e-300).is_err());
    for exponent in [0.5, 1., 2., -1.] {
        let s = PowScale::new([3., 3.], [0., 100.], exponent).unwrap();
        close(s.map(2.), 50., "constant");
        close(s.invert(1000.), 3., "constant inverse");
        assert_eq!(s.ticks(5), vec![3.]);
    }
    let reciprocal = PowScale::new([1., 4.], [0., 100.], -1.).unwrap();
    assert!(reciprocal.map(0.).is_nan());
    assert!(reciprocal.map(-2.).is_nan());
    assert_eq!(reciprocal.clamped(true).map(-2.), 0.);
    assert!(
        PowScale::new([1., 16.], [0., 100.], -1.)
            .unwrap()
            .nice(3)
            .is_err()
    );
    let sqrt = PowScale::sqrt([-9., 16.], [0., 100.]).unwrap();
    close(sqrt.map(0.), 300. / 7., "signed sqrt crosses zero");
    assert_eq!(sqrt.domain(), [-9., 16.]);
    assert_eq!(sqrt.range(), [0., 100.]);
    assert_eq!(sqrt.exponent(), 0.5);
}
