use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 360.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    }
}
fn samples(values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(i.to_string(), "A", "", Some(*v)))
        .collect()
}
fn options(min: Option<f64>, max: Option<f64>) -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        domain_bounds: Some(BoxPlotDomainBounds { min, max }),
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() <= 1e-10, "{a} != {b}");
}

#[test]
fn original_nivo_independent_endpoints_and_projections_match_48_profiles() {
    let mut count = 0;
    for line in include_str!("fixtures/boxplot-domain-bounds.csv")
        .lines()
        .skip(1)
    {
        count += 1;
        let row: Vec<_> = line.split('|').collect();
        let nums = |i: usize| {
            row[i]
                .split(',')
                .map(|v| v.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        };
        let bound = |i: usize| {
            if row[i] == "auto" {
                None
            } else {
                Some(row[i].parse().unwrap())
            }
        };
        let mut o = options(bound(1), bound(2));
        o.orientation = if row[3] == "1" {
            StatisticsOrientation::Vertical
        } else {
            StatisticsOrientation::Horizontal
        };
        o.quantiles = [0., 0.25, 0.5, 0.75, 1.];
        let nice = row[6].parse::<i32>().unwrap();
        let profile = ContinuousScaleOptions {
            reverse: row[4] == "1",
            round: row[5] == "1",
            nice: if nice < 0 {
                ScaleNice::Off
            } else {
                ScaleNice::Count(nice as usize)
            },
            ..Default::default()
        };
        let l =
            boxplot_layout_with_scale_options(&frame(), &samples(&nums(0)), &o, &profile).unwrap();
        assert_eq!(l.value_scale.source_domain().as_slice(), nums(7));
        assert_eq!(l.layout.domain.as_slice(), nums(8));
        for (actual, expected) in l.layout.boxes[0].quantiles.into_iter().zip(nums(9)) {
            near(actual, expected);
        }
        for (value, expected) in l.layout.boxes[0].quantiles.into_iter().zip(nums(10)) {
            near(l.map_statistic(value).unwrap(), expected);
        }
        // Endpoint selection must feed the existing complete-domain geometry path.
        let mut full = o.clone();
        full.domain_bounds = None;
        full.domain = Some(l.value_scale.source_domain());
        let expected =
            boxplot_layout_with_scale_options(&frame(), &samples(&nums(0)), &full, &profile)
                .unwrap();
        assert_eq!(l.layout, expected.layout);
    }
    assert_eq!(count, 48);
}

#[test]
fn automatic_endpoint_tracks_updates_hidden_samples_and_nullable_slots() {
    for (values, domain) in [([2., 4., 8.], [2., 10.]), ([4., 6., 9.], [4., 10.])] {
        let mut data = samples(&values);
        data.push(BoxObservation::new("null", "A", "", None));
        data.push(BoxObservation::new("hidden", "A", "hidden", Some(-100.)));
        let mut o = options(None, Some(10.));
        o.hidden.insert(boxplot_series_id("hidden"));
        let l = boxplot_layout(&frame(), &data, &o).unwrap();
        assert_eq!(l.domain, domain);
        assert_eq!(l.boxes.len(), 1);
        assert_eq!(l.boxes[0].count, 3);
        assert_eq!(l.boxes[0].extrema, [values[0], values[2]]);
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: Some(0.),
            max: None,
        });
        assert_eq!(
            boxplot_layout(&frame(), &data, &o).unwrap().domain,
            [0., values[2]]
        );
        o.hidden.clear();
        assert_eq!(
            boxplot_layout(&frame(), &data, &o).unwrap().domain,
            [0., values[2]]
        );
    }
}

#[test]
fn conflicts_and_nonfinite_fixed_endpoints_reject_even_without_ink() {
    for data in [vec![], samples(&[2., 4., 8.])] {
        let mut o = options(None, None);
        o.domain = Some([0., 10.]);
        assert!(
            boxplot_layout(&frame(), &data, &o)
                .unwrap_err()
                .0
                .contains("conflict")
        );
        assert!(
            boxplot_layout_with_scale_options(&frame(), &data, &o, &Default::default()).is_err()
        );
        o.domain = None;
        o.hidden.insert(boxplot_series_id(""));
        for bad in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            for bounds in [(Some(bad), None), (None, Some(bad)), (Some(1.), Some(bad))] {
                o.domain_bounds = Some(BoxPlotDomainBounds {
                    min: bounds.0,
                    max: bounds.1,
                });
                assert!(
                    boxplot_layout(&frame(), &data, &o)
                        .unwrap_err()
                        .0
                        .contains("finite")
                );
            }
        }
    }
    let data = samples(&[2., 4., 8.]);
    assert_eq!(
        boxplot_layout(&frame(), &data, &options(None, None)).unwrap(),
        boxplot_layout(&frame(), &data, &Default::default()).unwrap()
    );
}

#[test]
fn empty_nonzero_fallback_uses_fixed_sign_only_without_real_source_or_ticks() {
    for kind in [
        ScaleKind::Log,
        ScaleKind::Pow { exponent: -1. },
        ScaleKind::Pow { exponent: -0.5 },
    ] {
        let p = ContinuousScaleOptions {
            kind,
            ..Default::default()
        };
        for (bounds, expected) in [
            ((Some(-20.), None), [-20., -1.]),
            ((None, Some(-0.5)), [-10., -0.5]),
            ((Some(2.), None), [2., 10.]),
            ((None, Some(20.)), [1., 20.]),
        ] {
            let mut o = options(bounds.0, bounds.1);
            o.scale = kind;
            for data in [vec![], vec![BoxObservation::new("null", "A", "", None)]] {
                let l = boxplot_layout_with_scale_options(&frame(), &data, &o, &p).unwrap();
                assert_eq!(l.value_scale.source_domain(), expected);
                assert!(l.layout.boxes.is_empty());
                assert_eq!(
                    boxplot_layout(&frame(), &data, &o).unwrap().domain,
                    expected
                );
            }
        }
        let mut o = options(None, Some(-0.5));
        o.hidden.insert(boxplot_series_id(""));
        for bad in [vec![0.], vec![-1., 1.], vec![-1., 0.]] {
            assert!(boxplot_layout_with_scale_options(&frame(), &samples(&bad), &o, &p).is_err());
        }
        // Real source/ticks override sign hints, so a contradictory fixed endpoint rejects.
        assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[2., 8.]), &o, &p).is_err());
        o.value_axis.tick_values = Some(vec![1., 2.]);
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &p).is_err());
        o.value_axis.tick_values = Some(vec![-8., -2.]);
        assert_eq!(
            boxplot_layout_with_scale_options(&frame(), &[], &o, &p)
                .unwrap()
                .value_scale
                .source_domain(),
            [-10., -0.5]
        );
        o.value_axis.tick_values = Some(vec![0.]);
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &p).is_err());
        o.value_axis.tick_values = None;
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: Some(0.),
            max: None,
        });
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &p).is_err());
        // Complete explicit endpoints bypass empty source sign inference like old domain.
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: Some(-10.),
            max: Some(-1.),
        });
        assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 1.]), &o, &p).is_ok());
    }
}

#[test]
fn complete_bounds_reuse_descending_constant_nice_round_and_reverse_scale_path() {
    let mut accepted = 0;
    let mut rejected = 0;
    for (kind, values, domains) in [
        (
            ScaleKind::Linear,
            vec![2., 4., 8.],
            vec![[0., 10.], [10., 0.], [4., 4.]],
        ),
        (
            ScaleKind::Log,
            vec![2., 4., 8.],
            vec![[1., 10.], [10., 1.], [4., 4.]],
        ),
        (
            ScaleKind::Pow { exponent: -1. },
            vec![-8., -4., -2.],
            vec![[-10., -1.], [-1., -10.], [-4., -4.]],
        ),
        (
            ScaleKind::SymLog { constant: 0.4 },
            vec![-8., 0., 4.],
            vec![[-9., 7.], [7., -9.], [0., 0.]],
        ),
    ] {
        for domain in domains {
            for reverse in [false, true] {
                for round in [false, true] {
                    let o = options(Some(domain[0]), Some(domain[1]));
                    let p = ContinuousScaleOptions {
                        kind,
                        nice: ScaleNice::Count(5),
                        round,
                        reverse,
                        clamp: true,
                        ..Default::default()
                    };
                    let mut old = o.clone();
                    old.domain_bounds = None;
                    old.domain = Some(domain);
                    let a = boxplot_layout_with_scale_options(&frame(), &samples(&values), &o, &p);
                    let b =
                        boxplot_layout_with_scale_options(&frame(), &samples(&values), &old, &p);
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            accepted += 1;
                            assert_eq!(a.layout, b.layout);
                            assert_eq!(a.value_scale.source_domain(), domain);
                            assert_eq!(a.value_scale.range(), b.value_scale.range());
                        }
                        (Err(a), Err(b)) => {
                            rejected += 1;
                            // Existing linear nice can extend inverse-Pow to
                            // zero; the shared strict guard must stay.
                            assert_eq!(kind, ScaleKind::Pow { exponent: -1. });
                            assert!(a.0.contains("one nonzero sign"));
                            assert_eq!(a, b);
                        }
                        (a, b) => panic!("independent and full domains differ: {a:?} / {b:?}"),
                    }
                }
            }
        }
    }
    assert_eq!((accepted, rejected), (40, 8));
    let o = options(Some(10.), Some(0.));
    assert!(boxplot_layout(&frame(), &samples(&[2., 4.]), &o).is_err());
}

#[test]
fn one_sided_crop_preserves_real_outliers_statistics_clips_and_saturated_extremes() {
    for horizontal in [false, true] {
        for clamp in [false, true] {
            let data = samples(&[-0.1, 2., 4., 6., 8., 10.1]);
            let mut o = options(None, Some(10.));
            o.orientation = if horizontal {
                StatisticsOrientation::Horizontal
            } else {
                StatisticsOrientation::Vertical
            };
            o.outlier_radius = 8.;
            let p = ContinuousScaleOptions {
                clamp,
                ..Default::default()
            };
            let l = boxplot_layout_with_scale_options(&frame(), &data, &o, &p).unwrap();
            assert_eq!(l.value_scale.source_domain(), [-0.1, 10.]);
            assert_eq!(l.layout.boxes[0].extrema, [-0.1, 10.1]);
            assert_eq!(l.layout.boxes[0].source_ids.len(), 6);
            let outliers: Vec<_> = l
                .layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.contains(":outlier:"))
                .collect();
            assert_eq!(outliers.len(), 2);
            assert!(
                outliers
                    .iter()
                    .all(|m| m.clip == Some(ClipRect::plot(&frame())))
            );
            assert!(outliers.iter().any(|m| m.value == 10.1));
            let mut old = o.clone();
            old.domain_bounds = None;
            old.domain = Some([-0.1, 10.]);
            assert_eq!(
                l.layout,
                boxplot_layout_with_scale_options(&frame(), &data, &old, &p)
                    .unwrap()
                    .layout
            );
        }
    }
    let data = samples(&[-1e308, -5e307, 0., 5e307, 1e308]);
    let o = options(Some(0.), Some(1.));
    let l = boxplot_layout_with_scale_options(&frame(), &data, &o, &Default::default()).unwrap();
    assert_eq!(l.layout.boxes[0].extrema, [-1e308, 1e308]);
    l.layout.scene.validate().unwrap();
}

#[test]
fn utc_bounds_use_integral_source_domain_while_quantiles_stay_fractional() {
    let data = samples(&[0., 1.]);
    let p = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let o = options(None, Some(2.));
    let l = boxplot_layout_with_scale_options(&frame(), &data, &o, &p).unwrap();
    assert_eq!(l.value_scale.source_domain(), [0., 2.]);
    assert_eq!(l.layout.boxes[0].quantiles[2], 0.5);
    assert!(l.map_statistic(0.5).unwrap().is_finite());
    for bound in [0.5, MAX_TIMESTAMP_MS as f64 + 1.] {
        assert!(
            boxplot_layout_with_scale_options(&frame(), &data, &options(None, Some(bound)), &p)
                .is_err()
        );
    }
    assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 0.5]), &o, &p).is_err());
    assert!(
        boxplot_layout(
            &frame(),
            &data,
            &BoxPlotLayoutOptions {
                scale: ScaleKind::Utc,
                ..o
            }
        )
        .is_err()
    );
}
