//! Original d3-scale 4.0.2 repeated-domain oracle. Native values, checked-profile
//! rejections, special IEEE numbers and licenses are preserved by the producer.
//! Numerical and one-ULP half-integer policies were frozen before Rust execution.
#![cfg(feature = "geojson")]

use kartis::{
    ContinuousScaleOptions, PiecewiseScaleOptions, ResolvedContinuousScale, ScaleKind, ScaleNice,
};
use serde_json::Value;

fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn number(v: &Value) -> f64 {
    v.as_f64()
        .unwrap_or_else(|| v.as_str().unwrap().parse().unwrap())
}
fn values(v: &Value) -> Vec<f64> {
    array(v).iter().map(number).collect()
}
fn same(a: f64, b: f64) -> bool {
    a == b || a.is_nan() && b.is_nan()
}
fn close(a: f64, b: f64, context: &str) {
    assert!(
        same(a, b) || (a - b).abs() <= b.abs().max(1.) * 2e-12,
        "{context}: {a:?} != {b:?}; difference {}",
        (a - b).abs()
    );
}
fn equal(a: &[f64], b: &[f64], context: &str) {
    assert_eq!(a.len(), b.len(), "{context}");
    for (&a, &b) in a.iter().zip(b) {
        close(a, b, context);
    }
}
fn kind(c: &Value) -> ScaleKind {
    match c["kind"].as_str().unwrap() {
        "linear" => ScaleKind::Linear,
        "log" => ScaleKind::Log,
        "symlog" => ScaleKind::SymLog {
            constant: number(&c["parameter"]),
        },
        "pow" => ScaleKind::Pow {
            exponent: number(&c["parameter"]),
        },
        "sqrt" => ScaleKind::Sqrt,
        "utc" => ScaleKind::Utc,
        _ => panic!("unknown original scale"),
    }
}
fn options(c: &Value) -> PiecewiseScaleOptions {
    let o = &c["options"];
    let kind = kind(c);
    PiecewiseScaleOptions {
        domain: values(&c["domain"]),
        range: values(&c["range"]),
        continuous: ContinuousScaleOptions {
            kind,
            nice: match o["nice"].as_i64().unwrap() {
                -1 => ScaleNice::Off,
                0 => ScaleNice::Default,
                n => ScaleNice::Count(n as usize),
            },
            reverse: o["reverse"].as_bool().unwrap(),
            round: o["round"].as_bool().unwrap(),
            clamp: o["clamp"].as_bool().unwrap(),
            log_base: if kind == ScaleKind::Log {
                number(&c["parameter"])
            } else {
                10.
            },
        },
    }
}
fn inverse(actual: kartis::Result<f64>, expected: f64, utc: bool, context: &str) {
    if !expected.is_finite() {
        assert!(
            actual.is_err(),
            "{context}: native nonfinite inverse must reject"
        );
        return;
    }
    let actual = actual.unwrap_or_else(|e| panic!("{context}: unexpected inverse rejection: {e}"));
    if utc {
        assert_eq!(actual, expected, "{context}: original Date TimeClip");
    } else {
        close(actual, expected, context);
    }
}
fn rounded(a: f64, b: f64, raw: f64, native_raw: f64, context: &str) -> bool {
    if same(a, b) {
        if a == 0. && b == 0. {
            assert_eq!(
                a.is_sign_negative(),
                b.is_sign_negative(),
                "{context}: rounded zero sign"
            );
        }
        return false;
    }
    assert!(
        a.is_finite() && b.is_finite(),
        "{context}: rounded finite/nonfinite mismatch"
    );
    assert_eq!(
        (a - b).abs(),
        1.,
        "{context}: rounded disagreement must be exactly one pixel"
    );
    let half = native_raw.floor() + 0.5;
    let ulp = f64::from_bits(half.abs().to_bits() + 1) - half.abs();
    assert!(
        (raw - half).abs() <= ulp && (native_raw - half).abs() <= ulp,
        "{context}: rounded mismatch outside ONE ULP of the SAME half integer: raw {raw:?}/{native_raw:?}, half {half:?}, ULP {ulp}"
    );
    assert_eq!(a, (raw + 0.5).floor(), "{context}: actual round");
    assert_eq!(b, (native_raw + 0.5).floor(), "{context}: original round");
    true
}

#[test]
fn original_repeated_knots_map_global_inverse_branch_inverse_nice_ticks_round_clamp() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/piecewise-repeated/values.json")).unwrap();
    let cases = array(&fixture["cases"]);
    assert_eq!(cases.len(), 9496);
    let (
        mut accepted,
        mut rejected,
        mut mapped,
        mut unchecked,
        mut inverses,
        mut branch_inverses,
        mut round_boundaries,
        mut ticks,
    ) = (0, 0, 0, 0, 0, 0, 0, 0);
    for case in cases {
        let name = case["name"].as_str().unwrap();
        let native = &case["native"];
        let profile = options(case);
        let before = profile.clone();
        let result = ResolvedContinuousScale::piecewise(&profile);
        assert_eq!(profile, before, "{name}: input mutation");
        if case["checkedConstruction"] != "accept" {
            assert!(
                result.is_err(),
                "{name}: checked transform/nice profile must reject, native data is still retained"
            );
            rejected += 1;
            continue;
        }
        accepted += 1;
        let scale = result.unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(scale.is_piecewise());
        assert_eq!(
            scale
                .source_domain_stops()
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            profile
                .domain
                .iter()
                .map(|v| v.to_bits())
                .collect::<Vec<_>>(),
            "{name}: original signed source knots"
        );
        equal(
            scale.display_domain_stops(),
            &values(&native["display"]),
            name,
        );
        equal(scale.range_stops(), &values(&native["range"]), name);
        assert_eq!(
            scale.is_invertible(),
            native["strictRangeInverse"].as_bool().unwrap(),
            "{name}: inverse policy"
        );
        let mut raw_profile = profile.clone();
        raw_profile.continuous.round = false;
        let raw_scale = ResolvedContinuousScale::piecewise(&raw_profile).unwrap();
        for (query_index, query) in array(&native["queries"]).iter().enumerate() {
            let context = format!("{name}/query-{query_index} input {}", query["input"]);
            let input = number(&query["input"]);
            let actual = scale.map(input);
            let raw = raw_scale.map(input);
            if !query["checkedInput"].as_bool().unwrap() {
                unchecked += 1;
                assert!(
                    actual.is_nan() && raw.is_nan(),
                    "{context}: checked reciprocal/log input branch must reject"
                );
                continue;
            }
            mapped += 1;
            let expected = number(&query["mapped"]);
            let expected_raw = number(&query["unrounded"]);
            close(raw, expected_raw, &context);
            if profile.continuous.round {
                if rounded(actual, expected, raw, expected_raw, &context) {
                    round_boundaries += 1;
                    eprintln!(
                        "Retained one-ULP rounded boundary {context}: actual {actual}, native {expected}, raw {raw:?}/{expected_raw:?}"
                    );
                }
            } else {
                close(actual, expected, &context);
            }
        }
        for query in array(&native["inverse"]) {
            inverses += 1;
            let pixel = number(&query["pixel"]);
            let actual = scale.invert(pixel);
            if scale.is_invertible() {
                inverse(
                    actual,
                    number(&query["value"]),
                    profile.continuous.kind == ScaleKind::Utc,
                    &format!("{name}/invert {pixel}"),
                );
            } else {
                assert!(
                    actual.is_err(),
                    "{name}: original turning/plateau inverse retained but checked inverse rejects"
                );
            }
        }
        for branch in array(&native["branches"]) {
            let index = branch["index"].as_u64().unwrap() as usize;
            for query in array(&branch["pixels"]) {
                branch_inverses += 1;
                let pixel = number(&query["pixel"]);
                let actual = scale.invert_segment(index, pixel);
                if branch["flat"].as_bool().unwrap() {
                    assert!(actual.is_err(), "{name}: flat branch must reject");
                } else {
                    inverse(
                        actual,
                        number(&query["inverse"]),
                        profile.continuous.kind == ScaleKind::Utc,
                        &format!("{name}/branch-{index} {pixel}"),
                    );
                }
            }
        }
        let actual = scale.ticks(7);
        let expected = values(&native["ticks"]);
        ticks += actual.len();
        if profile.continuous.kind == ScaleKind::Utc {
            assert_eq!(actual, expected, "{name}: original UTC ticks");
        } else {
            equal(&actual, &expected, &format!("{name}/ticks"));
        }
    }
    assert_eq!((accepted, rejected), (8960, 536));
    eprintln!(
        "Original repeated-knot D3: {accepted} accepted/{rejected} retained checked rejections, {mapped} map queries, {unchecked} retained out-of-profile inputs, {inverses} global inverse checks, {branch_inverses} branch inverse checks, {ticks} ticks, {round_boundaries} fixed one-ULP half-boundary disagreements"
    );
}

#[test]
fn native_viewport_records_preserve_original_full_knots_and_true_turn_diagnostics() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/piecewise-repeated/values.json")).unwrap();
    let cases = array(&fixture["viewportCases"]);
    assert_eq!(cases.len(), 4128);
    let (
        mut gap,
        mut gap_constant,
        mut gap_variation,
        mut turns,
        mut initial_constant,
        mut rescaled_constant,
    ) = (0, 0, 0, 0, 0, 0);
    for case in cases {
        let native = &case["native"];
        let source = values(&case["source"]);
        let display = values(&native["display"]);
        let rescaled = values(&native["rescaled"]);
        let fractions = values(&case["rangeFractions"]);
        assert_eq!(source.len(), display.len());
        assert_eq!(source.len(), rescaled.len());
        assert_eq!(fractions.first(), Some(&0.));
        assert_eq!(fractions.last(), Some(&1.));
        assert!(fractions.windows(2).all(|p| p[0] < p[1]));
        let monotonic = rescaled.iter().all(|v| v.is_finite())
            && (rescaled.windows(2).all(|p| p[0] <= p[1])
                || rescaled.windows(2).all(|p| p[0] >= p[1]));
        assert_eq!(monotonic, native["rescaledMonotonic"].as_bool().unwrap());
        let constant = rescaled.iter().all(|v| *v == rescaled[0]);
        assert_eq!(constant, native["rescaledConstant"].as_bool().unwrap());
        let initial = source.first() == source.last();
        assert_eq!(initial, native["initialSourceConstant"].as_bool().unwrap());
        if !monotonic {
            turns += 1;
        }
        if initial {
            initial_constant += 1;
        }
        if constant {
            rescaled_constant += 1;
        }
        if case["window"]["duplicateGap"].as_bool().unwrap() {
            gap += 1;
            if constant {
                gap_constant += 1;
            } else {
                gap_variation += 1;
            }
        }
    }
    assert_eq!(
        (
            gap,
            gap_constant,
            gap_variation,
            turns,
            initial_constant,
            rescaled_constant
        ),
        (928, 872, 56, 32, 1280, 2040)
    );
}
