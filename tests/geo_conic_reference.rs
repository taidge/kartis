//! Original-function conic reference; native fixtures remain unfiltered.
use std::collections::BTreeMap;

use kartis::*;
const ORDINARY: &str = include_str!("fixtures/geo-conic/fixtures/ordinary.csv");
const CRITICAL: &str = include_str!("fixtures/geo-conic/fixtures/critical.csv");
const ALBERS: &str = include_str!("fixtures/geo-conic/fixtures/albers-unit.csv");
const NATIVE_FORWARD: &str = include_str!("fixtures/geo-conic/fixtures/all-native-forward.csv");
fn number(c: &[&str], i: usize) -> f64 {
    c[i].parse().unwrap()
}
fn projection(c: &[&str]) -> GeoProjection {
    let p = GeoConicParallels::new([number(c, 1), number(c, 2)]).unwrap();
    match c[0] {
        "equalArea" => GeoProjection::ConicEqualArea(p),
        "equidistant" => GeoProjection::ConicEquidistant(p),
        "conformal" => GeoProjection::ConicConformal(p),
        _ => panic!("family"),
    }
}
fn vector([l, p]: [f64; 2]) -> [f64; 3] {
    let (l, p) = (l.to_radians(), p.to_radians());
    [p.cos() * l.cos(), p.cos() * l.sin(), p.sin()]
}
fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let (a, b) = (vector(a), vector(b));
    (a[0] - b[0]).hypot(a[1] - b[1]).hypot(a[2] - b[2])
}
fn context(p: GeoProjection) -> GeoProjectionContext {
    geographic_map_layout(
        &ChartConfig::default(),
        &[],
        &GeoMapOptions {
            projection: p,
            scale: Some(1.),
            translate: Some([0., 0.]),
            ..Default::default()
        },
    )
    .unwrap()
    .projection
}
fn inverse_original(ctx: &GeoProjectionContext, raw: [f64; 2]) -> [f64; 2] {
    let c = ctx.fitted_raw_center();
    let t = ctx.translate();
    let s = ctx.scale();
    ctx.invert([t[0] + (raw[0] - c[0]) * s, t[1] - (raw[1] - c[1]) * s])
        .unwrap()
        .expect("authored original query must recover source")
}
#[test]
fn ordinary_original_forward_and_context_inverse_fixed_1e12() {
    let mut contexts = BTreeMap::new();
    let mut count = 0;
    let mut max_forward: f64 = 0.;
    let mut max_inverse: f64 = 0.;
    for line in ORDINARY.lines() {
        let c = line.split(',').collect::<Vec<_>>();
        let p = projection(&c);
        let source = [number(&c, 3), number(&c, 4)];
        let expected = [number(&c, 7), number(&c, 8)];
        let actual = project_geo_point(
            source,
            &GeoMapOptions {
                projection: p,
                ..Default::default()
            },
        )
        .unwrap()
        .unwrap();
        let error = (actual[0] - expected[0])
            .abs()
            .max((actual[1] - expected[1]).abs());
        max_forward = max_forward.max(error);
        assert!(error <= 1e-12, "forward {line}: {actual:?}, error={error}");
        let key = format!("{}:{}:{}", c[0], c[1], c[2]);
        let ctx = contexts.entry(key).or_insert_with(|| context(p));
        let actual = inverse_original(ctx, expected);
        let error = distance(actual, source);
        max_inverse = max_inverse.max(error);
        assert!(error <= 1e-12, "inverse {line}: {actual:?}, error={error}");
        count += 1;
    }
    assert_eq!(count, 8208);
    eprintln!("ordinary={count} forward_max={max_forward:e} inverse_vector_max={max_inverse:e}");
}
#[test]
fn critical_authored_source_fixed_1e12_preserves_22_native_errors() {
    let mut count = 0;
    let mut original_bad = 0;
    let mut max: f64 = 0.;
    for line in CRITICAL.lines() {
        let c = line.split(',').collect::<Vec<_>>();
        let p = projection(&c);
        let source = [number(&c, 3).to_degrees(), number(&c, 4).to_degrees()];
        let actual = inverse_original(&context(p), [number(&c, 5), number(&c, 6)]);
        let error = distance(actual, source);
        max = max.max(error);
        assert!(error <= 1e-12, "critical {line}: {actual:?}, error={error}");
        count += 1;
        if c[7] == "original-sign-zero-inverse-error" {
            original_bad += 1;
        }
    }
    assert_eq!(count, 72);
    assert_eq!(original_bad, 22);
    eprintln!("critical={count} original_bad={original_bad} inverse_vector_max={max:e}");
}
#[test]
fn albers_intrinsic_frame_matches_original_unit_affine() {
    let options = GeoMapOptions {
        projection: GeoProjection::Albers,
        ..Default::default()
    };
    let ctx = context(GeoProjection::Albers);
    let mut count = 0;
    for line in ALBERS.lines() {
        let c = line.split(',').collect::<Vec<_>>();
        let source = [number(&c, 0), number(&c, 1)];
        let expected = [number(&c, 2), number(&c, 3)];
        let actual = project_geo_point(source, &options).unwrap().unwrap();
        assert!(
            (actual[0] - expected[0]).abs() <= 1e-12 && (actual[1] - expected[1]).abs() <= 1e-12,
            "Albers {line}: {actual:?}"
        );
        // Poles have nonunique longitude and lose inverse radial precision.
        if source[1].abs() < 85. {
            let inv = inverse_original(&ctx, expected);
            assert!(
                distance(inv, source) <= 1e-12,
                "Albers inverse {line}: {inv:?}"
            );
        }
        count += 1;
    }
    assert_eq!(count, 9);
}
#[test]
fn native_extremes_keep_explicit_precision_and_visibility_policies() {
    let mut precision = 0;
    let mut capped = 0;
    for line in NATIVE_FORWARD.lines() {
        let c = line.split(',').collect::<Vec<_>>();
        if c[0] == "equalArea"
            && number(&c, 1) == 5.72957796e-5
            && number(&c, 2) == 5.72957796e-5
            && number(&c, 3) == -180.
            && number(&c, 4) == -90.
        {
            let p = projection(&c);
            let ctx = context(p);
            let q = [number(&c, 7), number(&c, 8)];
            let center = ctx.fitted_raw_center();
            assert!(
                project_geo_point(
                    [-180., -90.],
                    &GeoMapOptions {
                        projection: p,
                        ..Default::default()
                    }
                )
                .unwrap()
                .is_some()
            );
            let err = ctx
                .invert([q[0] - center[0], center[1] - q[1]])
                .expect_err("small-n native raw residual does not establish source precision");
            assert!(err.to_string().contains("precision"), "{err}");
            precision += 1;
        }
        if c[0] == "conformal"
            && number(&c, 1) == 30.
            && number(&c, 2) == 30.
            && number(&c, 3) == 0.
            && number(&c, 4) == -90.
        {
            let p = projection(&c);
            let ctx = context(p);
            let q = [number(&c, 7), number(&c, 8)];
            let center = ctx.fitted_raw_center();
            assert!(
                q.iter().all(|v| v.is_finite()),
                "native many-to-one clamp stays in fixture"
            );
            assert!(
                project_geo_point(
                    [0., -90.],
                    &GeoMapOptions {
                        projection: p,
                        ..Default::default()
                    }
                )
                .unwrap()
                .is_none()
            );
            assert!(
                ctx.invert([q[0] - center[0], center[1] - q[1]])
                    .unwrap()
                    .is_none()
            );
            capped += 1;
        }
    }
    assert_eq!(precision, 1);
    assert_eq!(capped, 2); // native +0 and -0 longitude rows stay distinct
    let ctx = context(GeoProjection::conic_equal_area([29.5, 45.5]).unwrap());
    let center = ctx.fitted_raw_center();
    assert!(
        ctx.invert([-center[0], center[1] - 100.])
            .unwrap()
            .is_none(),
        "intrinsic outside footprint is None"
    );
    assert!(
        ctx.invert([f64::NAN, 0.]).is_err(),
        "nonfinite query is Err"
    );
}
