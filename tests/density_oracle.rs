use kartis::{
    ContourLimits as Limits, DensityOptions, DensityPoint as WeightedPoint, DensityThresholds,
    estimate_density as density,
};
#[path = "contour_common/oracle.rs"]
mod common;
use common::{compare, parse_levels};

#[test]
fn actual_d3_float32_density_fields_threshold_units_and_paths() {
    let mut configs = 0;
    let mut cells = 0;
    for row in include_str!("fixtures/density-candidate.csv")
        .lines()
        .filter(|r| !r.starts_with('#'))
    {
        configs += 1;
        let c = row.split('|').collect::<Vec<_>>();
        let points = if c[7].is_empty() {
            vec![]
        } else {
            c[7].split(';')
                .map(|p| {
                    let v = p
                        .split(',')
                        .map(|v| v.parse::<f64>().unwrap())
                        .collect::<Vec<_>>();
                    WeightedPoint {
                        position: [v[0], v[1]],
                        weight: v[2],
                    }
                })
                .collect()
        };
        let field = density(
            &points,
            DensityOptions {
                size: [c[2].parse().unwrap(), c[3].parse().unwrap()],
                bandwidth: c[4].parse().unwrap(),
                cell_size: c[5].parse().unwrap(),
                ..Default::default()
            },
        )
        .unwrap();
        let expected = c[8]
            .split(',')
            .map(|v| u32::from_str_radix(v, 16).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            field.values().len(),
            expected.len(),
            "{} {}/{} dimensions",
            c[1],
            c[4],
            c[5]
        );
        for (index, (actual, expected)) in field.values().iter().zip(expected).enumerate() {
            cells += 1;
            assert_eq!(
                actual.to_bits(),
                expected,
                "{} bandwidth={} cell={} grid[{index}]: {} vs {}",
                c[1],
                c[4],
                c[5],
                actual,
                f32::from_bits(expected)
            );
        }
        assert_eq!(
            field.max(),
            c[9].parse::<f64>().unwrap(),
            "{} maximum density",
            c[1]
        );
        let thresholds = if let Some(v) = c[6].strip_prefix("count:") {
            DensityThresholds::Count(v.parse().unwrap())
        } else {
            DensityThresholds::Values(
                c[6].strip_prefix("list:")
                    .unwrap()
                    .split(',')
                    .map(|v| v.parse().unwrap())
                    .collect(),
            )
        };
        let actual = field.contours(&thresholds, Limits::default()).unwrap();
        let expected = parse_levels(c[10]);
        assert_eq!(actual.len(), expected.len(), "{} density levels", c[1]);
        for (a, b) in actual.into_iter().zip(expected) {
            compare(
                a,
                b,
                &format!("{} density bandwidth={} cell={}", c[1], c[4], c[5]),
            );
        }
        for expected in parse_levels(c[11]) {
            let actual = field.contour(expected.value, Limits::default()).unwrap();
            compare(
                actual,
                expected,
                &format!("{} cached density bandwidth={} cell={}", c[1], c[4], c[5]),
            );
        }
    }
    assert_eq!(configs, 500);
    assert_eq!(cells, 844_260);
}

#[test]
fn invalid_density_inputs_and_budgets_reject_without_capturing_business_points() {
    let options = DensityOptions::default();
    for options in [
        DensityOptions {
            bandwidth: f64::NAN,
            ..options
        },
        DensityOptions {
            cell_size: 0.5,
            ..options
        },
        DensityOptions {
            size: [0., 10.],
            ..options
        },
        DensityOptions {
            bandwidth: 100_000.,
            cell_size: 1.,
            ..options
        },
        DensityOptions {
            limits: Limits {
                work: 1,
                ..Default::default()
            },
            ..options
        },
    ] {
        assert!(density(&[], options).is_err());
    }
    assert!(
        density(
            &[WeightedPoint {
                position: [f64::INFINITY, 0.],
                weight: 1.
            }],
            options
        )
        .is_err()
    );
    assert!(
        density(
            &[WeightedPoint {
                position: [10., 10.],
                weight: f64::MAX
            }],
            options
        )
        .is_err()
    );
    let field = {
        let points = vec![WeightedPoint {
            position: [10., 10.],
            weight: 1.,
        }];
        density(&points, options).unwrap()
    };
    assert!(field.contour(f64::INFINITY, Limits::default()).is_err());
    assert!(
        field
            .contours(
                &DensityThresholds::Values(vec![f64::NAN]),
                Limits::default()
            )
            .is_err()
    );
    assert!(
        field
            .contours(&DensityThresholds::Count(257), Limits::default())
            .is_err()
    );
    assert!(
        field
            .contour(
                0.1,
                Limits {
                    cells: 1,
                    ..Default::default()
                }
            )
            .is_err()
    );
    field.contour(0.001, Limits::default()).unwrap();
}

#[test]
fn signed_negative_maximum_and_unsorted_levels_keep_geometry_bound_to_its_value() {
    let points = (0..16)
        .map(|i| WeightedPoint {
            position: [(i % 4) as f64 + 0.5, (i / 4) as f64 + 0.5],
            weight: -1.,
        })
        .collect::<Vec<_>>();
    let field = density(
        &points,
        DensityOptions {
            size: [4., 4.],
            bandwidth: 0.,
            cell_size: 1.,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(field.max(), -1.);
    assert!(field.values().iter().all(|v| *v == -1.));
    let levels = field
        .contours(&DensityThresholds::Count(4), Limits::default())
        .unwrap();
    let expected = parse_levels(include_str!("fixtures/negative-maximum-single-level.csv").trim());
    assert_eq!(levels.len(), expected.len());
    for (a, b) in levels.into_iter().zip(expected) {
        compare(
            a,
            b,
            "negative maximum uses independent D3 single-level geometry",
        );
    }
    let explicit = field
        .contours(
            &DensityThresholds::Values(vec![0., -1., -0.5, -1.]),
            Limits::default(),
        )
        .unwrap();
    assert_eq!(
        explicit.iter().map(|v| v.value).collect::<Vec<_>>(),
        [-1., -1., -0.5, 0.]
    );
    for level in explicit {
        assert_eq!(
            level,
            field.contour(level.value, Limits::default()).unwrap()
        );
    }
    assert!(
        field
            .contour(
                -1.,
                Limits {
                    levels: 0,
                    ..Default::default()
                }
            )
            .is_err()
    );
}
