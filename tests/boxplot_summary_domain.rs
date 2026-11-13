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
fn summary() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        orientation: StatisticsOrientation::Horizontal,
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-14 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}

#[test]
fn original_nivo_default_custom_and_timeclip_goldens_are_preserved_without_hiding_differences() {
    let mut count = 0;
    let mut exact = 0;
    let mut exceptions = 0;
    for line in include_str!("fixtures/boxplot-summary-domain.csv")
        .lines()
        .skip(1)
    {
        count += 1;
        let f: Vec<_> = line.split('|').collect();
        let nums = |i: usize| {
            if f[i].is_empty() {
                vec![]
            } else {
                f[i].split(',')
                    .map(|s| s.parse::<f64>().unwrap())
                    .collect::<Vec<_>>()
            }
        };
        let bound = |i: usize| {
            if f[i] == "auto" {
                None
            } else {
                Some(f[i].parse::<f64>().unwrap())
            }
        };
        let mut o = summary();
        o.quantiles = nums(3).try_into().unwrap();
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: bound(4),
            max: bound(5),
        });
        let mut data = samples(&nums(2));
        if f[0] == "hidden-other-series" {
            data.extend([
                BoxObservation::new("hidden1", "A", "hidden", Some(100.)),
                BoxObservation::new("hidden2", "A", "hidden", Some(200.)),
            ]);
            o.hidden.insert(boxplot_series_id("hidden"));
        }
        if f[0] == "all-hidden" {
            data = samples(&[1., 2., 3., 4.]);
            o.hidden.insert(boxplot_series_id(""));
        }
        let kind = match f[1] {
            "linear" => ScaleKind::Linear,
            "log" => ScaleKind::Log,
            "time" => ScaleKind::Utc,
            _ => panic!("unknown kind"),
        };
        let result = boxplot_layout_with_scale_options(
            &frame(),
            &data,
            &o,
            &ContinuousScaleOptions {
                kind,
                ..Default::default()
            },
        );
        if f[9] == "rust-raw-source-reject" {
            exceptions += 1;
            assert!(result.is_err());
            assert_eq!(f[7], "0.30000000000000004,2.7");
            continue;
        }
        let l = result.unwrap_or_else(|e| panic!("{}: {e:?}", f[0]));
        match f[9] {
            "exact" | "date-bound" => {
                exact += 1;
                for (a, b) in l.value_scale.source_domain().into_iter().zip(nums(7)) {
                    if kind == ScaleKind::Utc {
                        assert_eq!(a, b, "{} Date endpoint", f[0]);
                    } else {
                        near(a, b);
                    }
                }
                for (a, b) in l.layout.boxes[0].quantiles.into_iter().zip(nums(6)) {
                    near(a, b);
                }
                for (value, expected) in l.layout.boxes[0].quantiles.into_iter().zip(nums(8)) {
                    let actual = l.map_statistic(value).unwrap();
                    assert!(
                        (actual - expected).abs() < 1e-9,
                        "{} query {value}: {actual} != {expected}",
                        f[0]
                    );
                }
            }
            "rust-finite-fallback" => {
                exceptions += 1;
                assert_eq!(f[7], "inf,-inf");
                assert_eq!(l.layout.domain, [0., 1.]);
                assert!(l.layout.boxes.is_empty());
            }
            "rust-stable-quantile" => {
                exceptions += 1;
                assert!(f[6].contains("inf"));
                assert_eq!(l.layout.boxes[0].extrema, [-1e308, 1e308]);
                for v in l.layout.boxes[0].quantiles {
                    assert!(v.is_finite());
                }
                near(l.layout.domain[0], -8e307);
                near(l.layout.domain[1], 8e307);
            }
            "rust-zero-domain" => {
                exceptions += 1;
                assert_eq!(f[7], "946684800000,946771200000");
                assert_eq!(l.layout.domain, [0., 0.]);
                assert_eq!(l.map_statistic(0.).unwrap(), 180.);
            }
            _ => panic!("unclassified oracle difference"),
        }
        l.layout.scene.validate().unwrap();
    }
    assert_eq!((count, exact, exceptions), (24, 19, 5));
}

#[test]
fn summary_domain_uses_quantiles_independent_of_whiskers_raw_extrema_and_outliers() {
    let data = samples(&[0., 1., 2., 3., 4., 100.]);
    for whiskers in [
        BoxWhiskers::Quantiles,
        BoxWhiskers::Tukey { factor: 1.5 },
        BoxWhiskers::Extrema,
    ] {
        let mut o = summary();
        o.quantiles = [0.25, 0.4, 0.5, 0.6, 0.75];
        o.whiskers = whiskers;
        let l = boxplot_layout(&frame(), &data, &o).unwrap();
        assert_eq!(l.domain, [1.25, 3.75]);
        assert_eq!(l.boxes[0].extrema, [0., 100.]);
        assert_eq!(l.boxes[0].count, 6);
        assert_eq!(l.boxes[0].source_ids.len(), 6);
        for m in l.scene.marks.iter().filter(|m| m.id.starts_with("box:")) {
            assert_eq!(m.clip, Some(ClipRect::plot(&frame())));
        }
        if whiskers == BoxWhiskers::Quantiles {
            assert!(l.boxes[0].outliers.iter().any(|v| v.value == 100.));
            assert!(
                l.scene
                    .marks
                    .iter()
                    .any(|m| m.id.contains(":outlier:") && m.value == 100.)
            );
        }
    }
}

#[test]
fn summary_bounds_feed_same_nice_reverse_round_clamp_geometry_path() {
    for kind in [
        ScaleKind::Linear,
        ScaleKind::Log,
        ScaleKind::Pow { exponent: -1. },
        ScaleKind::SymLog { constant: 0.4 },
        ScaleKind::Utc,
    ] {
        let data = if kind == ScaleKind::Utc {
            samples(&[0., 10.])
        } else {
            samples(&[1., 2., 3., 4.])
        };
        for (min, max) in [(None, None), (Some(2.), None), (None, Some(3.))] {
            for horizontal in [false, true] {
                for reverse in [false, true] {
                    for round in [false, true] {
                        for clamp in [false, true] {
                            let mut o = summary();
                            o.orientation = if horizontal {
                                StatisticsOrientation::Horizontal
                            } else {
                                StatisticsOrientation::Vertical
                            };
                            o.domain_bounds = Some(BoxPlotDomainBounds { min, max });
                            let p = ContinuousScaleOptions {
                                kind,
                                nice: ScaleNice::Count(5),
                                reverse,
                                round,
                                clamp,
                                ..Default::default()
                            };
                            let l =
                                boxplot_layout_with_scale_options(&frame(), &data, &o, &p).unwrap();
                            let mut full = o.clone();
                            full.domain_bounds = None;
                            full.domain = Some(l.value_scale.source_domain());
                            // Full domains bypass inference, but keep the selected native summary arithmetic.
                            let expected =
                                boxplot_layout_with_scale_options(&frame(), &data, &full, &p)
                                    .unwrap();
                            assert_eq!(l.layout, expected.layout);
                            assert_eq!(l.value_scale.range(), expected.value_scale.range());
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn utc_clips_only_automatic_endpoints_preserving_fractional_statistics_and_fixed_strictness() {
    let p = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    for values in [vec![0., 1.], vec![-1., 0.], vec![-1., 1.]] {
        let o = summary();
        let l = boxplot_layout_with_scale_options(&frame(), &samples(&values), &o, &p).unwrap();
        assert_eq!(l.layout.domain.map(f64::to_bits), [0u64, 0u64]);
        assert_ne!(l.layout.boxes[0].quantiles[0], 0.);
        assert_eq!(
            l.map_statistic(l.layout.boxes[0].quantiles[1]).unwrap(),
            180.
        );
        let mut sample = o.clone();
        sample.auto_domain_source = BoxPlotAutoDomainSource::SampleExtent;
        let old =
            boxplot_layout_with_scale_options(&frame(), &samples(&values), &sample, &p).unwrap();
        assert_eq!(old.layout.boxes[0].quantiles, l.layout.boxes[0].quantiles);
    }
    let mut o = summary();
    o.domain_bounds = Some(BoxPlotDomainBounds {
        min: Some(-2.),
        max: None,
    });
    assert_eq!(
        boxplot_layout_with_scale_options(&frame(), &samples(&[-1., 1.]), &o, &p)
            .unwrap()
            .layout
            .domain,
        [-2., 0.]
    );
    o.domain_bounds = Some(BoxPlotDomainBounds {
        min: None,
        max: Some(2.),
    });
    assert_eq!(
        boxplot_layout_with_scale_options(&frame(), &samples(&[-1., 1.]), &o, &p)
            .unwrap()
            .layout
            .domain,
        [0., 2.]
    );
    for bad in [0.5, MAX_TIMESTAMP_MS as f64 + 1.] {
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: None,
            max: Some(bad),
        });
        assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 1.]), &o, &p).is_err());
    }
    o.domain_bounds = None;
    o.domain = Some([0.5, 1.]);
    assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 1.]), &o, &p).is_err());
    o.domain = None;
    assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 0.5]), &o, &p).is_err());
    o.scale = ScaleKind::Utc;
    assert!(boxplot_layout(&frame(), &samples(&[0., 1.]), &o).is_err());
}

#[test]
fn empty_nonzero_and_complete_explicit_domains_preserve_original_inference_rules() {
    for kind in [ScaleKind::Log, ScaleKind::Pow { exponent: -1. }] {
        let p = ContinuousScaleOptions {
            kind,
            ..Default::default()
        };
        let mut o = summary();
        let l = boxplot_layout_with_scale_options(&frame(), &[], &o, &p).unwrap();
        assert_eq!(l.layout.domain, [1., 10.]);
        assert!(l.layout.boxes.is_empty());
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: None,
            max: Some(-1.),
        });
        assert_eq!(
            boxplot_layout_with_scale_options(&frame(), &[], &o, &p)
                .unwrap()
                .layout
                .domain,
            [-10., -1.]
        );
        o.hidden.insert(boxplot_series_id(""));
        assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 1.]), &o, &p).is_err());
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: Some(1.),
            max: Some(10.),
        });
        assert!(boxplot_layout_with_scale_options(&frame(), &samples(&[0., 1.]), &o, &p).is_ok());
        o.domain = Some([1., 10.]);
        assert!(boxplot_layout_with_scale_options(&frame(), &[], &o, &p).is_err());
    }
    let data = samples(&[1., 2., 3., 4.]);
    for domain in [[1., 4.], [2., 3.], [3., 3.]] {
        let mut o = summary();
        o.domain = Some(domain);
        let mut sample = o.clone();
        sample.auto_domain_source = BoxPlotAutoDomainSource::SampleExtent;
        assert_eq!(
            boxplot_layout(&frame(), &data, &o).unwrap(),
            boxplot_layout(&frame(), &data, &sample).unwrap()
        );
        o.domain = None;
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: Some(domain[0]),
            max: Some(domain[1]),
        });
        assert_eq!(
            boxplot_layout(&frame(), &data, &o).unwrap(),
            boxplot_layout(&frame(), &data, &sample).unwrap()
        );
    }
}

#[test]
fn nullable_hidden_and_explicit_sample_profile_keep_legacy_extent_without_fabricated_data() {
    let mut data = samples(&[1., 2., 3., 4.]);
    data.push(BoxObservation::new("null", "A", "", None));
    let mut o = summary();
    let l = boxplot_layout(&frame(), &data, &o).unwrap();
    assert_eq!(l.domain, [1.3, 3.7]);
    assert_eq!(l.boxes[0].count, 4);
    o.auto_domain_source = BoxPlotAutoDomainSource::SampleExtent;
    let l = boxplot_layout(&frame(), &data, &o).unwrap();
    assert_eq!(l.domain, [1., 4.]);
    assert_eq!(
        l,
        boxplot_layout(
            &frame(),
            &data,
            &BoxPlotLayoutOptions {
                orientation: StatisticsOrientation::Horizontal,
                ..Default::default()
            }
        )
        .unwrap()
    );
    o.auto_domain_source = BoxPlotAutoDomainSource::SummaryQuantiles;
    for q in [
        [0.5; 5],
        [0., 0.5, 0.25, 0.75, 1.],
        [0., 0.25, f64::NAN, 0.75, 1.],
    ] {
        o.quantiles = q;
        if q == [0.5; 5] {
            assert_eq!(
                boxplot_layout(&frame(), &data, &o).unwrap().domain,
                [2.5, 2.5]
            );
        } else {
            assert!(boxplot_layout(&frame(), &data, &o).is_err());
        }
    }
}

#[test]
fn summary_equal_endpoint_identity_handles_local_duplicates_zero_singletons_and_large_values() {
    let o = summary();
    for values in [
        vec![7., 7., 7.],
        vec![7.],
        vec![f64::MAX, f64::MAX],
        vec![-1e308, -1e308],
    ] {
        let l =
            boxplot_layout_with_scale_options(&frame(), &samples(&values), &o, &Default::default())
                .unwrap();
        assert!(
            l.layout.boxes[0]
                .quantiles
                .iter()
                .all(|v| v.to_bits() == values[0].to_bits())
        );
        assert_eq!(l.layout.domain, [values[0], values[0]]);
        assert_eq!(l.map_statistic(values[0]).unwrap(), 180.);
    }
    for full_domain in [false, true] {
        let mut fixed = o.clone();
        if full_domain {
            fixed.domain = Some([1., 10.]);
        } else {
            fixed.domain_bounds = Some(BoxPlotDomainBounds {
                min: Some(1.),
                max: Some(10.),
            });
        }
        let l = boxplot_layout(&frame(), &samples(&[7., 7., 7.]), &fixed).unwrap();
        assert_eq!(l.domain, [1., 10.]);
        assert!(
            l.boxes[0]
                .quantiles
                .iter()
                .all(|v| v.to_bits() == 7f64.to_bits())
        );
    }
    let mut duplicate = o.clone();
    duplicate.quantiles = [0., 0.1, 0.25, 0.5, 1.];
    let l = boxplot_layout(&frame(), &samples(&[7., 7., 7., 10.]), &duplicate).unwrap();
    assert_eq!(l.boxes[0].quantiles, [7., 7., 7., 7., 10.]);
    let raw = samples(&[-0., 0.]);
    let l = boxplot_layout_with_scale_options(
        &frame(),
        &raw,
        &o,
        &ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            ..Default::default()
        },
    )
    .unwrap();
    // Native interpolation produces positive zero from these ordered endpoints;
    // the original raw negative-zero observation is still unchanged.
    assert!(
        l.layout.boxes[0]
            .quantiles
            .iter()
            .all(|v| v.to_bits() == 0u64)
    );
    assert_eq!(l.layout.domain.map(f64::to_bits), [0u64, 0u64]);
    assert_eq!(raw[0].value.unwrap().to_bits(), (-0f64).to_bits());
    let mut old = o.clone();
    old.auto_domain_source = BoxPlotAutoDomainSource::SampleExtent;
    let old = boxplot_layout(&frame(), &samples(&[7., 7., 7.]), &old).unwrap();
    assert!(
        old.boxes[0]
            .quantiles
            .iter()
            .any(|v| v.to_bits() != 7f64.to_bits())
    );
}
