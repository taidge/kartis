use kartis::*;

fn number(value: &str) -> f64 {
    match value {
        "nan" => f64::NAN,
        "inf" => f64::INFINITY,
        "-inf" => f64::NEG_INFINITY,
        _ => value.parse().unwrap(),
    }
}
fn options(c: &[&str]) -> ColorScaleOptions {
    ColorScaleOptions {
        kind: if c[2] == "diverging" {
            ColorScaleKind::Diverging {
                midpoint: number(c[7]),
            }
        } else {
            ColorScaleKind::Sequential
        },
        transform: match c[3] {
            "linear" => ColorScaleTransform::Linear,
            "log" => ColorScaleTransform::Log,
            "pow" => ColorScaleTransform::Pow {
                exponent: number(c[4]),
            },
            "sqrt" => ColorScaleTransform::Sqrt,
            "symlog" => ColorScaleTransform::SymLog {
                constant: number(c[4]),
            },
            _ => panic!("unknown transform"),
        },
        domain: Some([number(c[5]), number(c[6])]),
        colors: c[9].split(';').map(color).collect(),
        clamp: c[8] == "1",
        unknown: Color(123, 45, 67, 255),
        ..Default::default()
    }
}
fn color(value: &str) -> Color {
    let c: Vec<u8> = value.split(':').map(|v| v.parse().unwrap()).collect();
    Color(c[0], c[1], c[2], c[3])
}
fn close(a: f64, b: f64, row: &str) {
    assert!(
        (a - b).abs() <= 5e-12 * b.abs().max(1.),
        "{a} != {b}: {row}"
    );
}
#[test]
fn original_d3_queries_and_axis_inverses() {
    let (mut queries, mut inverses, mut nonfinite) = (0, 0, 0);
    for row in include_str!("fixtures/nonlinear-color-d3.csv")
        .lines()
        .filter(|r| !r.starts_with('#'))
    {
        let c: Vec<_> = row.split('|').collect();
        let o = options(&c);
        let scale = o.resolve([]).unwrap();
        assert_eq!(scale.transform(), o.transform);
        if c[0] == "I" {
            close(scale.value_at(number(c[10])).unwrap(), number(c[11]), row);
            inverses += 1;
            continue;
        }
        let value = (c[10] != "missing").then(|| number(c[10]));
        let expected = if c[14] == "nonfinite-parameter-unknown" {
            nonfinite += 1;
            o.unknown
        } else {
            color(c[13])
        };
        let actual = scale.map(value);
        // D3 interpolates normalized floating alpha; the existing RGBA8
        // contract interpolates byte alpha. Their final rounding can differ by one.
        assert_eq!(
            (actual.0, actual.1, actual.2),
            (expected.0, expected.1, expected.2),
            "{row}"
        );
        assert!(
            actual.3.abs_diff(expected.3) <= 1,
            "alpha {actual:?} != {expected:?}: {row}"
        );
        if let Some(value) = value.filter(|x| x.is_finite()) {
            let position = number(c[12]);
            if position.is_finite() {
                close(scale.position(value).unwrap(), position, row);
            }
        }
        queries += 1;
    }
    assert_eq!((queries, inverses, nonfinite), (6400, 1792, 498));
}
#[test]
fn domains_singularities_and_nonfinite_queries_are_checked() {
    let old = ColorScaleOptions::default().resolve([]).unwrap();
    let invalid = [
        (ColorScaleTransform::Log, [-1., 1.]),
        (ColorScaleTransform::Log, [0., 1.]),
        (ColorScaleTransform::Pow { exponent: -1. }, [-1., 1.]),
        (ColorScaleTransform::Pow { exponent: -1. }, [0., 1.]),
        (ColorScaleTransform::Pow { exponent: 0. }, [1., 2.]),
        (ColorScaleTransform::Pow { exponent: f64::NAN }, [1., 2.]),
        (ColorScaleTransform::Pow { exponent: 1000. }, [1., 10.]),
        (
            ColorScaleTransform::Pow { exponent: 1. },
            [-f64::MAX, f64::MAX],
        ),
        (ColorScaleTransform::Pow { exponent: 1e-320 }, [1., 2.]),
        (ColorScaleTransform::SymLog { constant: 1e-320 }, [1., 2.]),
        (
            ColorScaleTransform::Log,
            [1e308, f64::from_bits(1e308_f64.to_bits() + 1)],
        ),
        (ColorScaleTransform::SymLog { constant: 0. }, [-1., 1.]),
        (
            ColorScaleTransform::SymLog {
                constant: f64::INFINITY,
            },
            [-1., 1.],
        ),
    ];
    for (transform, domain) in invalid {
        assert!(
            ColorScaleOptions {
                transform,
                domain: Some(domain),
                ..Default::default()
            }
            .resolve([])
            .is_err()
        );
    }
    assert!(
        ColorScaleOptions {
            transform: ColorScaleTransform::Log,
            ..Default::default()
        }
        .resolve([])
        .is_err()
    );
    assert!(
        ColorScaleOptions {
            transform: ColorScaleTransform::Log,
            kind: ColorScaleKind::Quantize,
            ..Default::default()
        }
        .resolve([])
        .is_err()
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(old.map(Some(value)), old.unknown());
        assert_eq!(old.position(value), None);
        assert!(old.value_at(value).is_err());
    }
    assert!(old.value_at(-0.1).is_err());
    assert!(old.value_at(1.1).is_err());
}
#[test]
fn logarithmic_zero_and_reciprocal_opposite_sign_queries_keep_d3_finite_results() {
    for (domain, zero) in [([1., 100.], 0.), ([-100., -1.], 1.)] {
        let s = ColorScaleOptions {
            transform: ColorScaleTransform::Log,
            domain: Some(domain),
            ..Default::default()
        }
        .resolve([])
        .unwrap();
        assert_eq!(
            s.map(Some(0.)),
            s.map(Some(if zero == 0. { domain[0] } else { domain[1] }))
        );
        assert_eq!(s.map(Some(-domain[0])), s.unknown());
    }
    let s = ColorScaleOptions {
        transform: ColorScaleTransform::Pow { exponent: -1. },
        domain: Some([1., 100.]),
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    assert_eq!(s.map(Some(-1.)), s.map(Some(100.)));
    assert_eq!(s.map(Some(0.)), s.map(Some(1.)));
    let unclamped = ColorScaleOptions {
        transform: ColorScaleTransform::Pow { exponent: -1. },
        domain: Some([1., 100.]),
        clamp: false,
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    close(
        unclamped.position(-1.).unwrap(),
        2.0202020202020203,
        "reciprocal opposite-sign",
    );
    assert_eq!(unclamped.map(Some(0.)), unclamped.unknown());
}

#[test]
fn axis_inverse_rejects_underflow_outside_the_accepted_domain() {
    for domain in [[1e-200, 1e-100], [-1e-200, -1e-100]] {
        let scale = ColorScaleOptions {
            transform: ColorScaleTransform::Pow { exponent: 2. },
            domain: Some(domain),
            ..Default::default()
        }
        .resolve([])
        .unwrap();
        assert!(scale.value_at(f64::from_bits(1)).is_err());
        assert_eq!(scale.value_at(0.).unwrap(), domain[0]);
        assert_eq!(scale.value_at(1.).unwrap(), domain[1]);
        let interior = scale.value_at(0.5).unwrap();
        assert!((domain[0].min(domain[1])..=domain[0].max(domain[1])).contains(&interior));
    }
}
