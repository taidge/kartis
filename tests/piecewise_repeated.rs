use kartis::scales::PiecewiseScale;
use kartis::{
    ContinuousScaleOptions, PiecewiseScaleOptions, ResolvedContinuousScale, ScaleKind, ScaleNice,
};

fn close(a: f64, b: f64) {
    assert!(
        a == b || (a - b).abs() <= b.abs().max(1.) * 2e-12,
        "{a:?} != {b:?}"
    );
}
fn raw(domain: &[f64], range: &[f64], kind: ScaleKind, clamp: bool) -> PiecewiseScale {
    PiecewiseScale::new(
        domain,
        range,
        &ContinuousScaleOptions {
            kind,
            clamp,
            ..Default::default()
        },
    )
    .unwrap()
}

#[test]
fn duplicate_ties_follow_native_reverse_then_bisect_right_in_both_directions() {
    for (kind, domain, at, expected) in [
        (ScaleKind::Linear, vec![0., 1., 1., 2.], 1., 30.),
        (ScaleKind::Linear, vec![2., 1., 1., 0.], 1., 20.),
        (ScaleKind::Log, vec![1., 2., 2., 4.], 2., 30.),
        (ScaleKind::Log, vec![4., 2., 2., 1.], 2., 20.),
        (
            ScaleKind::Pow { exponent: -1. },
            vec![1., 2., 2., 4.],
            2.,
            20.,
        ),
        (
            ScaleKind::Pow { exponent: -1. },
            vec![4., 2., 2., 1.],
            2.,
            30.,
        ),
        (
            ScaleKind::Pow { exponent: -2. },
            vec![-4., -2., -2., -1.],
            -2.,
            20.,
        ),
        (
            ScaleKind::Pow { exponent: -2. },
            vec![-1., -2., -2., -4.],
            -2.,
            30.,
        ),
    ] {
        for clamp in [false, true] {
            let scale = raw(&domain, &[10., 20., 30., 40.], kind, clamp);
            close(scale.map(at), expected);
            assert!(scale.is_invertible());
            assert_eq!(
                scale.invert(25.).unwrap(),
                at,
                "inverse selects a range branch; repeated domain is not a bijection"
            );
        }
    }
    let scale = raw(&[0., 0., 1.], &[0., 100., 200.], ScaleKind::Linear, false);
    assert_eq!(scale.map(-1.), 50.);
    assert_eq!(scale.map(0.), 100.);
    let scale = scale.clamped(true);
    assert_eq!(
        scale.map(-1.),
        100.,
        "clamp precedes repeated-knot selection across whole domain"
    );
    assert_eq!(scale.map(2.), 200.);
    let scale = raw(&[1., 0., 0.], &[0., 100., 200.], ScaleKind::Linear, true);
    assert_eq!(scale.map(-1.), 100.);
    assert_eq!(scale.map(2.), 0.);
}

#[test]
fn constant_domains_preserve_d3_bimap_and_polymap_half_value_rules() {
    for n in [2, 3, 4, 9] {
        let domain = vec![2.; n];
        let range = (0..n).map(|i| i as f64 * 100.).collect::<Vec<_>>();
        for clamp in [false, true] {
            for kind in [
                ScaleKind::Linear,
                ScaleKind::Log,
                ScaleKind::SymLog { constant: 2. },
                ScaleKind::Pow { exponent: -1. },
                ScaleKind::Pow { exponent: 2. },
                ScaleKind::Sqrt,
                ScaleKind::Utc,
            ] {
                let scale = raw(&domain, &range, kind, clamp);
                let first = 50.;
                let last = (n as f64 - 1.5) * 100.;
                close(scale.map(2.), last);
                let below = if n == 2 {
                    first
                } else if clamp || matches!(kind,ScaleKind::Pow{exponent} if exponent<0.) {
                    last
                } else {
                    first
                };
                let above =
                    if n == 2 || !clamp && matches!(kind,ScaleKind::Pow{exponent} if exponent<0.) {
                        first
                    } else {
                        last
                    };
                close(scale.map(1.), below);
                close(scale.map(3.), above);
                for p in [-100., 0., 50., 200., 1000.] {
                    close(scale.invert(p).unwrap(), 2.);
                }
                assert!(scale.is_invertible());
            }
        }
    }
}

#[test]
fn range_plateaus_and_turns_keep_explicit_inverse_branch_policy() {
    let plateau = raw(
        &[0., 1., 1., 2.],
        &[0., 10., 10., 20.],
        ScaleKind::Linear,
        false,
    );
    assert!(!plateau.is_invertible());
    assert!(plateau.invert(10.).is_err());
    assert!(plateau.invert_segment(1, 10.).is_err());
    assert_eq!(plateau.invert_segment(0, 5.).unwrap(), 0.5);
    let turning = raw(
        &[0., 1., 1., 2.],
        &[0., 10., -10., 0.],
        ScaleKind::Linear,
        true,
    );
    assert!(!turning.is_invertible());
    assert!(turning.invert(0.).is_err());
    assert_eq!(turning.invert_segment(1, 500.).unwrap(), 1.);
    assert_eq!(turning.invert_segment(0, -10.).unwrap(), 0.);
    assert!(turning.invert_segment(3, 0.).is_err());
}

#[test]
fn resolved_nice_reverse_round_ticks_and_utc_keep_all_source_knots() {
    let source = PiecewiseScaleOptions {
        domain: vec![0.2, 1., 1., 1.8],
        range: vec![-2.5, 1.5, 5.5, 8.5],
        continuous: ContinuousScaleOptions {
            reverse: true,
            round: true,
            nice: ScaleNice::Count(4),
            ..Default::default()
        },
    };
    let saved = source.clone();
    let scale = ResolvedContinuousScale::piecewise(&source).unwrap();
    assert_eq!(source, saved);
    assert_eq!(scale.source_domain_stops(), source.domain);
    assert_eq!(scale.display_domain_stops(), [0., 1., 1., 2.]);
    assert_eq!(scale.range_stops(), [8.5, 5.5, 1.5, -2.5]);
    assert_eq!(scale.map(1.), 2.);
    assert_eq!(scale.ticks(4), [0., 0.5, 1., 1.5, 2.]);
    close(scale.invert(3.5).unwrap(), 1.);
    let utc = ResolvedContinuousScale::piecewise(&PiecewiseScaleOptions {
        domain: vec![1000., 2000., 2000., 4000.],
        range: vec![0., 100., 200., 300.],
        continuous: ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            ..Default::default()
        },
    })
    .unwrap();
    assert_eq!(utc.map(2000.), 200.);
    assert_eq!(utc.invert(200.025).unwrap(), 2000.);
    assert!(utc.map(2000.5).is_nan());
    assert_eq!(utc.source_domain_stops(), [1000., 2000., 2000., 4000.]);
}

#[test]
fn true_turns_distinct_transform_collapse_and_invalid_branches_still_reject() {
    for domain in [
        vec![0., 1., 1., 0.],
        vec![2., 2., 1., 2.],
        vec![0., f64::NAN, 0.],
    ] {
        assert!(
            PiecewiseScale::new(&domain, &vec![0.; domain.len()], &Default::default()).is_err()
        );
    }
    for (kind, domain) in [
        (ScaleKind::Pow { exponent: 1e-300 }, vec![1., 2., 2., 3.]),
        (ScaleKind::Log, vec![-1., -1., 0.]),
        (ScaleKind::Pow { exponent: -1. }, vec![0., 0., 1.]),
        (ScaleKind::Utc, vec![1., 1., 1.5]),
    ] {
        assert!(
            PiecewiseScale::new(
                &domain,
                &vec![0.; domain.len()],
                &ContinuousScaleOptions {
                    kind,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        raw(&[1., 1., 1.], &[0., 1., 2.], ScaleKind::Log, true)
            .map(-10.)
            .is_finite()
    );
    assert!(
        raw(&[1., 1., 1.], &[0., 1., 2.], ScaleKind::Log, false)
            .map(-10.)
            .is_nan()
    );
}

#[test]
fn original_symlog_neighbor_discontinuity_and_nice_rebuild_keep_the_same_math() {
    // Captured from original d3-scale symlogish's transform closure, not a
    // transcription of its formula. MSVCRT ln_1p formerly merged q and knot.
    let knot = -9.3_f64;
    let q = f64::from_bits(knot.to_bits() + 1);
    let next = f64::from_bits(knot.to_bits() - 1);
    let source = PiecewiseScaleOptions {
        domain: vec![knot, knot, 17.6],
        range: vec![-10.5, 42.5, 95.5],
        continuous: ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 2. },
            ..Default::default()
        },
    };
    let scale = ResolvedContinuousScale::piecewise(&source).unwrap();
    assert_eq!(
        scale.chart_scale().transformed(q).to_bits(),
        0xbffbb4dc71e6464d
    );
    assert_eq!(
        scale.chart_scale().transformed(knot).to_bits(),
        0xbffbb4dc71e6464c
    );
    assert_eq!(
        scale.chart_scale().transformed(next).to_bits(),
        0xbffbb4dc71e6464c
    );
    assert_eq!(scale.map(q), 16.);
    assert_eq!(scale.map(knot), 42.5);
    assert_eq!(
        scale.map(next),
        42.5,
        "original next-up really coalesces; raw-domain bisect is incorrect"
    );
    let mut niced = source;
    niced.continuous.nice = ScaleNice::Default;
    niced.continuous.round = true;
    niced.continuous.clamp = true;
    let scale = ResolvedContinuousScale::piecewise(&niced).unwrap();
    assert_eq!(scale.display_domain_stops(), [-10., knot, 18.]);
    assert_eq!(scale.map(q), 42.);
    assert_eq!(scale.map(knot), 43.);
    assert_eq!(scale.map(next), 43.);
    niced.domain = scale.display_domain_stops().to_vec();
    niced.continuous.nice = ScaleNice::Off;
    let rebuilt = ResolvedContinuousScale::piecewise(&niced).unwrap();
    assert_eq!(
        rebuilt.map(q),
        scale.map(q),
        "strict domain after nice retains the explicit piecewise math contract"
    );
}
