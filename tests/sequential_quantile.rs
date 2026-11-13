use kartis::*;
fn options() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 128, 0, 255)],
        unknown: Color(9, 8, 7, 6),
        ..Default::default()
    }
}
#[test]
fn complete_distribution_is_stable_shared_and_right_bisected() {
    let scale = options()
        .resolve([Some(10.), Some(0.), Some(-0.), None, Some(5.), Some(10.)])
        .unwrap();
    let samples = scale.sample_domain().unwrap();
    assert_eq!(samples.len(), 5);
    assert_eq!(samples[0].to_bits(), 0_f64.to_bits());
    assert_eq!(samples[1].to_bits(), (-0_f64).to_bits());
    assert_eq!(&samples[2..], &[5., 10., 10.]);
    let copy = scale.clone();
    assert!(std::ptr::eq(
        samples.as_ptr(),
        copy.sample_domain().unwrap().as_ptr()
    ));
    for (value, rank) in [
        (-100., 0.),
        (0., 0.25),
        (4., 0.25),
        (5., 0.5),
        (9., 0.5),
        (10., 1.),
        (100., 1.),
    ] {
        assert_eq!(scale.rank(value), Some(rank));
    }
    assert_eq!(scale.rank(f64::NAN), None);
    assert_eq!(scale.rank(f64::INFINITY), None);
    assert!(!scale.kind().is_discrete());
    assert!(scale.bins().is_empty());
    assert_eq!(scale.bin_extent(0), None);
    assert_eq!(scale.invert_extent(scale.palette()[0]), None);
}
#[test]
fn rank_colors_are_ordinal_not_duplicate_sample_queries() {
    let scale = ColorScaleOptions {
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        ..options()
    }
    .resolve([Some(0.), Some(0.), Some(10.)])
    .unwrap();
    assert_eq!(
        scale.rank_colors().unwrap(),
        vec![
            Color(0, 0, 0, 255),
            Color(128, 128, 128, 255),
            Color(255, 255, 255, 255)
        ]
    );
    assert_eq!(scale.map(Some(0.)), Color(128, 128, 128, 255));
    assert_ne!(scale.rank_colors().unwrap()[0], scale.map(Some(0.)));
    assert_eq!(scale.palette().len(), 2);
}
#[test]
fn finite_legend_window_is_independent_of_distribution_and_axis() {
    let scale = ColorScaleOptions {
        domain: Some([3., 9.]),
        clamp: false,
        ..options()
    }
    .resolve([Some(0.), Some(0.), Some(5.), Some(10.)])
    .unwrap();
    assert_eq!(scale.sample_domain().unwrap(), &[0., 0., 5., 10.]);
    assert_eq!(scale.sample_extent(), Some([0., 10.]));
    assert_eq!(scale.domain(), [3., 9.]);
    assert_eq!(scale.position(6.), Some(0.5));
    assert_eq!(scale.value_at(0.5).unwrap(), 6.);
    assert_eq!(scale.rank(6.), Some(2. / 3.));
    assert_eq!(scale.rank(-1.), Some(0.));
    assert_eq!(scale.map(Some(-1.)), scale.unknown());
    assert_eq!(scale.map(Some(10.)), scale.unknown());
    let saturated = ColorScaleOptions {
        clamp: true,
        ..ColorScaleOptions {
            domain: Some([3., 9.]),
            ..options()
        }
    }
    .resolve([Some(0.), Some(0.), Some(5.), Some(10.)])
    .unwrap();
    assert_eq!(saturated.map(Some(-1.)), saturated.palette()[0]);
    assert_eq!(saturated.map(Some(11.)), saturated.palette()[1]);
}
#[test]
fn empty_singleton_constant_and_other_kinds_remain_distinct() {
    let empty = options().resolve([None, None]).unwrap();
    assert_eq!(empty.sample_domain(), Some(&[][..]));
    assert_eq!(empty.domain(), [0., 1.]);
    assert_eq!(empty.rank_colors(), Some(vec![]));
    assert_eq!(empty.quantiles(2).unwrap(), None);
    let singleton = options().resolve([Some(-0.)]).unwrap();
    assert_eq!(singleton.rank(0.), None);
    assert_eq!(singleton.rank_colors(), Some(vec![singleton.unknown()]));
    assert!(
        singleton
            .quantiles(4)
            .unwrap()
            .unwrap()
            .iter()
            .all(|v| v.to_bits() == (-0_f64).to_bits())
    );
    for scale in [empty, singleton] {
        for value in [
            None,
            Some(-1.),
            Some(0.),
            Some(f64::MAX),
            Some(f64::NAN),
            Some(f64::NEG_INFINITY),
        ] {
            assert_eq!(scale.map(value), scale.unknown());
        }
        assert!(
            scale
                .stops(16)
                .unwrap()
                .iter()
                .all(|s| s.color == scale.unknown())
        );
    }
    let constant = options().resolve([Some(2.); 3]).unwrap();
    assert_eq!(constant.rank(1.), Some(0.));
    assert_eq!(constant.rank(2.), Some(1.));
    assert_eq!(constant.map(Some(2.)), constant.palette()[1]);
    assert!(
        constant
            .stops(2)
            .unwrap()
            .iter()
            .all(|s| s.color == constant.palette()[1])
    );
    let other = ColorScaleOptions::default()
        .resolve([Some(1.), Some(2.)])
        .unwrap();
    assert_eq!(other.rank(1.), None);
    assert_eq!(other.rank_colors(), None);
    assert_eq!(other.sample_domain(), None);
    assert_eq!(other.quantiles(2).unwrap(), None);
    assert!(other.quantiles(0).is_err());
}
#[test]
fn original_r7_statistics_preserve_zero_and_report_overflow() {
    let scale = options()
        .resolve([Some(10.), Some(2.), Some(8.), Some(4.)])
        .unwrap();
    assert_eq!(
        scale.quantiles(4).unwrap(),
        Some(vec![2., 3.5, 6., 8.5, 10.])
    );
    assert!(scale.quantiles(0).is_err());
    assert!(scale.quantiles(257).is_err());
    let zeros = options().resolve([Some(-0.), Some(0.)]).unwrap();
    let q = zeros.quantiles(2).unwrap().unwrap();
    assert_eq!(q[0].to_bits(), (-0_f64).to_bits());
    assert_eq!(q[1].to_bits(), 0_f64.to_bits());
    assert_eq!(q[2].to_bits(), (-0_f64).to_bits());
    let huge = options()
        .resolve([Some(-f64::MAX), Some(f64::MAX)])
        .unwrap();
    assert_eq!(huge.quantiles(1).unwrap(), Some(vec![-f64::MAX, f64::MAX]));
    assert!(huge.quantiles(2).is_err());
    assert_eq!(huge.sample_domain().unwrap(), &[-f64::MAX, f64::MAX]);
}
#[test]
fn every_source_entry_counts_and_invalid_profiles_do_not_invent_samples() {
    assert!(options().resolve(std::iter::repeat_n(None, 20_000)).is_ok());
    assert!(
        options()
            .resolve(std::iter::repeat_n(None, 20_001))
            .is_err()
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(options().resolve([None, Some(value)]).is_err());
    }
    for domain in [[2., 1.], [f64::NAN, 1.], [0., f64::INFINITY]] {
        assert!(
            ColorScaleOptions {
                domain: Some(domain),
                ..options()
            }
            .resolve([])
            .is_err()
        );
    }
    assert!(
        ColorScaleOptions {
            transform: ColorScaleTransform::Log,
            ..options()
        }
        .resolve([Some(1.), Some(2.)])
        .is_err()
    );
    assert!(
        ColorScaleOptions {
            thresholds: vec![1.],
            ..options()
        }
        .resolve([])
        .is_err()
    );
    assert!(
        ColorScaleOptions {
            colors: vec![Color(0, 0, 0, 255)],
            ..options()
        }
        .resolve([])
        .is_err()
    );
    assert!(
        ColorScaleOptions {
            clamp: false,
            ..options()
        }
        .resolve_with_profile([Some(1.), Some(2.)], ColorScheme::Viridis)
        .is_err()
    );
}
#[test]
fn existing_interpolation_and_named_scheme_paths_color_the_rank() {
    for profile in [
        ColorScaleProfile::from(ColorInterpolation::Srgb),
        ColorInterpolation::Lab.into(),
        ColorInterpolation::RgbGamma { gamma: 2.2 }.into(),
        ColorScheme::Viridis.into(),
        ColorScheme::Turbo.into(),
    ] {
        let scale = options()
            .resolve_with_profile([Some(2.), Some(2.), Some(7.), Some(20.)], profile)
            .unwrap();
        let unit = ColorScaleOptions {
            domain: Some([0., 1.]),
            kind: ColorScaleKind::Sequential,
            ..options()
        }
        .resolve_with_profile([], profile)
        .unwrap();
        for value in [-1., 2., 4., 7., 19., 20., 30.] {
            assert_eq!(scale.map(Some(value)), unit.map(scale.rank(value)));
        }
    }
}
