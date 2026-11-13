//! Independent original d3-scale 4.0.2 SequentialQuantile records. No Node is
//! needed by these Rust tests; native JavaScript profiles are kept separate.
use kartis::*;
#[path = "sequential_quantile_common/mod.rs"]
mod common;
use common::*;

#[test]
fn every_original_csv_record_has_an_explicit_checked_profile() {
    let cases = cases();
    assert_eq!(cases.len(), 124);
    let mut finite = 0;
    let mut empty = 0;
    let mut singleton = 0;
    let mut native = 0;
    let mut queries = 0;
    let mut copies = 0;
    let mut statistics = 0;
    let mut range_values = 0;
    for case in &cases {
        match case.profile {
            "finite-domain-n-ge-2" => finite += 1,
            "empty-native-I-minus-zero" => empty += 1,
            "singleton-native-I-NaN" => singleton += 1,
            "native-domain-coercion-and-infinity"
            | "native-domain-infinity"
            | "native-domain-object-coercion" => native += 1,
            profile => panic!("Unaccounted profile: {profile}"),
        }
        queries += case.queries.len();
        copies += case.copy_queries.len();
        statistics += case.quantiles.len();
        range_values += numbers(case.range).len();
    }
    assert_eq!((finite, empty, singleton, native), (109, 1, 7, 7));
    assert_eq!(
        (queries, copies, statistics, range_values),
        (8481, 8481, 1116, 41717)
    );
}

#[test]
fn original_finite_109_distributions_7021_ranks_ranges_r7_getters_and_copies() {
    let mut configurations = 0;
    let mut queries = 0;
    let mut stats_finite = 0;
    let mut stats_overflow = 0;
    for case in cases()
        .into_iter()
        .filter(|case| case.profile == "finite-domain-n-ge-2")
    {
        configurations += 1;
        let source = numbers(case.source);
        assert!(source.len() >= 2 && source.iter().all(|value| value.is_finite()));
        let scale = resolve(&source).unwrap();
        assert!(!scale.kind().is_discrete());
        assert!(scale.bins().is_empty());
        assert!(scale.bin_extent(0).is_none());
        for (index, row) in case.queries.iter().enumerate() {
            queries += usize::from(query(&scale, row, &format!("{} rank {index}", case.id)));
        }
        let (finite, overflow) = statistics(&scale, &case);
        stats_finite += finite;
        stats_overflow += overflow;
        getter_and_copy(&scale, &case);
    }
    assert_eq!((configurations, queries), (109, 7021));
    assert_eq!(stats_finite + stats_overflow, 109 * 8);
    assert_eq!(
        stats_overflow, 0,
        "These frozen finite distributions have no overflowing adjacent R7 span; overflow is a separate original-function edge profile"
    );
    eprintln!(
        "original finite configurations={configurations}, exact ranks={queries}, finite R7={stats_finite}, original R7 overflow rejects={stats_overflow}"
    );
}

#[test]
fn empty_and_singleton_original_nan_minus_zero_profiles_follow_checked_contract() {
    let mut configurations = 0;
    for case in cases().into_iter().filter(|case| {
        matches!(
            case.profile,
            "empty-native-I-minus-zero" | "singleton-native-I-NaN"
        )
    }) {
        configurations += 1;
        let source = numbers(case.source);
        let scale = resolve(&source).unwrap();
        bits(
            scale.sample_domain().unwrap(),
            &numbers(case.domain),
            case.id,
        );
        for row in &case.queries {
            match token(row.input) {
                Token::Number(value) => {
                    assert_eq!(scale.rank(value), None);
                    assert_eq!(scale.map(Some(value)), options().unknown);
                    if value.is_nan() {
                        assert_eq!(row.output, "u");
                        assert!(row.arguments.is_empty());
                    } else {
                        assert_eq!(row.output, if source.is_empty() { "n:-0" } else { "n:NaN" });
                        assert_eq!(row.arguments, row.output);
                    }
                }
                Token::Missing => {
                    assert_eq!(row.output, "u");
                    assert!(row.arguments.is_empty());
                    assert_eq!(scale.map(None), options().unknown);
                }
                Token::JsOnly(_) => {
                    assert_eq!(row.profile, "native-query-coercion-or-infinity");
                    assert!(matches!(row.output, "u" | "n:-0" | "n:NaN"));
                }
            }
        }
        let ordinal = scale.rank_colors().unwrap();
        assert_eq!(ordinal.len(), source.len());
        assert!(ordinal.iter().all(|color| *color == options().unknown));
        for &(segments, raw) in &case.quantiles {
            if segments == 0 {
                assert!(scale.quantiles(0).is_err());
                continue;
            }
            if source.is_empty() {
                assert_eq!(scale.quantiles(segments).unwrap(), None);
                assert_eq!(raw.split(',').collect::<Vec<_>>(), vec!["u"; segments + 1]);
            } else {
                bits(
                    &scale.quantiles(segments).unwrap().unwrap(),
                    &numbers(raw),
                    "Singleton original R7",
                );
            }
        }
        getter_and_copy(&scale, &case);
    }
    assert_eq!(configurations, 8);
}

#[test]
fn js_domain_and_query_profiles_are_retained_without_implicit_rust_coercion() {
    let mut cases_seen = 0;
    let mut rejected_source = 0;
    let mut explicit_numeric_domain = 0;
    let mut rows = 0;
    for case in cases()
        .into_iter()
        .filter(|case| case.profile.starts_with("native-"))
    {
        cases_seen += 1;
        // Domain() below is the original already-coerced getter, not implicit
        // coercion performed by the typed Rust resolver.
        let original_domain = numbers(case.domain);
        let typed = resolve(&original_domain);
        if original_domain.iter().any(|value| !value.is_finite()) {
            rejected_source += 1;
            assert!(typed.is_err(), "{} present infinity must reject", case.id);
        } else {
            explicit_numeric_domain += 1;
            let scale = typed.as_ref().unwrap();
            bits(
                scale.sample_domain().unwrap(),
                &original_domain,
                "Explicit caller-supplied numeric domain",
            );
            for (i, row) in case.queries.iter().enumerate() {
                query(scale, row, &format!("{} pre-coerced {i}", case.id));
            }
            statistics(scale, &case);
            getter_and_copy(scale, &case);
        }
        for row in &case.queries {
            rows += 1;
            assert!(matches!(
                row.profile,
                "finite-number" | "native-missing" | "native-query-coercion-or-infinity"
            ));
            assert!(row.output == "u" || matches!(token(row.output), Token::Number(_)));
        }
        for &(segments, raw) in &case.quantiles {
            assert_eq!(raw.split(',').count(), segments + 1);
        }
        assert!(!case.source.is_empty());
    }
    assert_eq!(
        (cases_seen, rejected_source, explicit_numeric_domain),
        (7, 6, 1)
    );
    assert!(rows > 0);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(
            options()
                .resolve([Some(0.), Some(value), Some(1.)])
                .is_err()
        );
    }
    let missing = options().resolve([None, Some(0.), None, Some(1.)]).unwrap();
    bits(
        missing.sample_domain().unwrap(),
        &[0., 1.],
        "Missing source contributes no rank",
    );
}

#[test]
fn native_exception_and_invalid_quantile_count_records_are_preserved_separately() {
    let original = include_str!("fixtures/sequential-quantile-d3.json");
    assert_eq!(original.matches("\"nativeExceptions\"").count(), 1);
    let (_, tail) = original.rsplit_once("\"nativeExceptions\"").unwrap();
    let tail = tail.trim_start().strip_prefix(':').unwrap();
    let actual = native_json(&format!("{{\"nativeExceptions\":{tail}"));
    let error = |operation, input, message| {
        format!(
            r#"{{"operation":"{operation}","input":"{input}","error":{{"name":"TypeError","message":"{message}"}}}}"#
        )
    };
    let errors = [
        error(
            "domain",
            "symbol",
            "Cannot convert a Symbol value to a number",
        ),
        error(
            "query",
            "symbol",
            "Cannot convert a Symbol value to a number",
        ),
        error(
            "domain",
            "bigint",
            "Cannot convert a BigInt value to a number",
        ),
        error(
            "query",
            "bigint",
            "Cannot convert a BigInt value to a number",
        ),
    ];
    let mut coerced = vec!["n:0", "n:5.5"];
    coerced.extend(std::iter::repeat_n("n:100", 19));
    let native_counts = [
        ("n:-1", vec![]),
        ("n:-0.5", vec![]),
        ("n:1.5", vec!["n:0", "n:10"]),
        ("n:NaN", vec![]),
        ("z", vec!["u"]),
        ("u", vec![]),
        ("s:32", coerced),
    ];
    let count_rows=native_counts.iter().map(|(input,values)|format!(r#"{{"input":"{input}","values":[{}],"profile":"native-count-coercion-not-rust-contract"}}"#,values.iter().map(|value|format!("\"{value}\"")).collect::<Vec<_>>().join(","))).collect::<Vec<_>>();
    let expected = native_json(&format!(
        r#"{{"nativeExceptions":[{}],"nativeCounts":[{}]}}"#,
        errors.join(","),
        count_rows.join(",")
    ));
    assert_eq!(actual, expected);
    let scale = resolve(&[0., 1., 10., 100.]).unwrap();
    for segments in [0, 257, usize::MAX] {
        assert!(scale.quantiles(segments).is_err());
    }
}

#[test]
fn authored_window_does_not_replace_distribution_and_other_kinds_have_no_rank_api() {
    let samples = [0., 0., 5., 10., 10.];
    let mut option = options();
    option.domain = Some([2., 8.]);
    let scale = option.resolve(samples.map(Some)).unwrap();
    bits(
        scale.sample_domain().unwrap(),
        &samples,
        "Complete samples independent of window",
    );
    assert_eq!(scale.rank(0.), Some(0.25));
    assert_eq!(scale.map(Some(0.)), gray(0.25));
    assert_eq!(scale.rank(10.), Some(1.));
    assert_eq!(scale.map(Some(10.)), gray(1.));
    assert_eq!(scale.position(5.), Some(0.5));
    assert_eq!(scale.value_at(0.5).unwrap(), 5.);
    option.clamp = false;
    let windowed = option.resolve(samples.map(Some)).unwrap();
    assert_eq!(windowed.rank(0.), Some(0.25));
    assert_eq!(windowed.map(Some(0.)), option.unknown);
    assert_eq!(windowed.map(Some(10.)), option.unknown);
    assert_eq!(windowed.map(Some(5.)), gray(0.5));
    let ordinal = scale.rank_colors().unwrap();
    assert_ne!(ordinal[0], scale.map(Some(samples[0])));
    assert_eq!(ordinal[0], gray(0.));
    for kind in [
        ColorScaleKind::Sequential,
        ColorScaleKind::Quantile,
        ColorScaleKind::Quantize,
    ] {
        let mut o = options();
        o.kind = kind;
        let other = o.resolve(samples.map(Some)).unwrap();
        assert_eq!(other.sample_domain(), None);
        assert_eq!(other.rank(5.), None);
        assert_eq!(other.rank_colors(), None);
        assert_eq!(other.quantiles(4).unwrap(), None);
        assert!(other.quantiles(0).is_err());
    }
}

#[test]
fn original_finite_input_r7_overflow_is_separate_from_valid_rank_mapping() {
    let mut scale = None;
    let mut configurations = 0;
    let mut queries = 0;
    let mut finite_statistics = 0;
    let mut overflow_statistics = 0;
    let mut requests_in_case = 0;
    for line in include_str!("fixtures/sequential-quantile-overflow-d3.csv").lines() {
        if line.starts_with('#') {
            continue;
        }
        let fields = line.split('|').collect::<Vec<_>>();
        match fields[0] {
            "C" => {
                assert_eq!(fields.len(), 4);
                if configurations > 0 {
                    assert_eq!(requests_in_case, 7);
                }
                requests_in_case = 0;
                configurations += 1;
                let source = numbers(fields[2]);
                assert!(source.iter().all(|value| value.is_finite()));
                let current = resolve(&source).unwrap();
                bits(
                    current.sample_domain().unwrap(),
                    &numbers(fields[3]),
                    fields[1],
                );
                scale = Some(current);
            }
            "Q" => {
                assert_eq!(fields.len(), 5);
                queries += 1;
                let current = scale.as_ref().unwrap();
                match token(fields[3]) {
                    Token::Number(value) if value.is_finite() => {
                        assert_eq!(fields[2], "finite-number");
                        let expected = numbers(fields[4])[0];
                        assert_eq!(current.rank(value).unwrap().to_bits(), expected.to_bits());
                        assert_eq!(current.map(Some(value)), gray(expected));
                    }
                    Token::Number(value) => {
                        assert_eq!(fields[2], "native-query");
                        assert_eq!(current.rank(value), None);
                        assert_eq!(current.map(Some(value)), options().unknown);
                        assert!(fields[4] == "u" || matches!(fields[4], "n:0" | "n:1"));
                    }
                    Token::Missing => {
                        assert_eq!(fields[2], "native-query");
                        assert_eq!(fields[4], "u");
                        assert_eq!(current.map(None), options().unknown);
                    }
                    Token::JsOnly(_) => panic!("Unexpected supplement coercion query"),
                }
            }
            "P" => {
                assert_eq!(fields.len(), 5);
                requests_in_case += 1;
                let segments = fields[2].parse::<usize>().unwrap();
                let expected = numbers(fields[4]);
                assert_eq!(expected.len(), segments + 1);
                let actual = scale.as_ref().unwrap().quantiles(segments);
                match fields[3] {
                    "finite-r7" => {
                        finite_statistics += 1;
                        assert!(expected.iter().all(|value| value.is_finite()));
                        bits(&actual.unwrap().unwrap(), &expected, fields[1]);
                    }
                    "native-nonfinite-r7" => {
                        overflow_statistics += 1;
                        assert!(expected.iter().any(|value| !value.is_finite()));
                        assert!(
                            actual.is_err(),
                            "{} original overflow must reject",
                            fields[1]
                        );
                    }
                    profile => panic!("Unclassified statistics profile: {profile}"),
                }
            }
            kind => panic!("Unclassified supplement record: {kind}"),
        }
    }
    assert_eq!(requests_in_case, 7);
    assert_eq!(
        (
            configurations,
            queries,
            finite_statistics,
            overflow_statistics
        ),
        (6, 62, 18, 24)
    );
}
