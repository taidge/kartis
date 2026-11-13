use kartis::{
    ContourGrid as Grid, ContourLimits as Limits, ContourOptions as Options,
    ContourThresholds as Thresholds, contour, contours,
};
#[path = "contour_common/oracle.rs"]
mod common;
use common::{compare, parse_levels};

#[test]
fn actual_d3_thresholds_topology_smoothing_missing_and_holes() {
    let mut configs = 0;
    let mut levels = 0;
    for row in include_str!("fixtures/grid-candidate.csv")
        .lines()
        .chain(include_str!("fixtures/nested-ownership-candidate.csv").lines())
        .filter(|r| !r.starts_with('#'))
    {
        let c = row.split('|').collect::<Vec<_>>();
        let thresholds = if c[5] == "default" {
            Thresholds::Sturges
        } else if let Some(count) = c[5].strip_prefix("count:") {
            Thresholds::Count(count.parse().unwrap())
        } else {
            Thresholds::Values(
                c[5].strip_prefix("list:")
                    .unwrap()
                    .split(',')
                    .map(|v| v.parse().unwrap())
                    .collect(),
            )
        };
        configs += 1;
        let values = c[6]
            .split(',')
            .map(|v| {
                if v == "_" {
                    None
                } else {
                    Some(v.parse().unwrap())
                }
            })
            .collect::<Vec<_>>();
        let grid = Grid {
            width: c[2].parse().unwrap(),
            height: c[3].parse().unwrap(),
            values: &values,
        };
        let actual = contours(
            grid,
            &Options {
                thresholds,
                smooth: c[4] == "1",
                ..Default::default()
            },
        )
        .unwrap();
        let expected = parse_levels(c[7]);
        assert_eq!(actual.len(), expected.len(), "{}", c[1]);
        for (a, b) in actual.into_iter().zip(expected) {
            levels += 1;
            let context = format!("{} smooth={} threshold={}", c[1], c[4], b.value);
            compare(a, b, &context);
        }
    }
    assert_eq!(configs, 546);
    assert!(levels > 816);
}

#[test]
fn invalid_data_thresholds_and_cumulative_budgets_reject() {
    let values = vec![Some(1.); 4];
    let grid = Grid {
        width: 2,
        height: 2,
        values: &values,
    };
    for limits in [
        Limits {
            cells: 3,
            ..Default::default()
        },
        Limits {
            work: 8,
            ..Default::default()
        },
        Limits {
            edges: 2,
            ..Default::default()
        },
        Limits {
            points: 2,
            ..Default::default()
        },
        Limits {
            rings: 0,
            ..Default::default()
        },
    ] {
        assert!(contour(grid, 0.5, true, limits).is_err());
    }
    assert!(
        contours(
            grid,
            &Options {
                thresholds: Thresholds::Values(vec![0.5, 0.5]),
                limits: Limits {
                    work: 20,
                    ..Default::default()
                },
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(contour(grid, f64::NAN, true, Limits::default()).is_err());
    assert!(
        contour(
            Grid {
                width: usize::MAX,
                height: 2,
                values: &[]
            },
            0.,
            true,
            Limits::default()
        )
        .is_err()
    );
    assert!(
        contour(
            Grid {
                width: 0,
                height: 0,
                values: &[]
            },
            0.,
            true,
            Limits::default()
        )
        .is_err()
    );
    assert!(
        contour(
            Grid {
                width: 1,
                height: 1,
                values: &[Some(f64::INFINITY)]
            },
            0.,
            true,
            Limits::default()
        )
        .is_err()
    );
    assert!(
        contour(
            Grid {
                width: 2,
                height: 2,
                values: &[]
            },
            0.,
            true,
            Limits::default()
        )
        .is_err()
    );
}
