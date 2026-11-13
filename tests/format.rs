use kartis::format::*;
use kartis::*;

#[test]
fn primary_d3_documentation_examples_match_supported_formats() {
    // Golden examples from https://d3js.org/d3-format#locale_format.
    let locale = FormatLocale::default();
    for (spec, value, expected) in [
        (".0%", 0.123, "12%"),
        ("+20", 42., "                 +42"),
        (".^20", 42., ".........42........."),
        (".2s", 42e6, "42M"),
        ("#x", 48879., "0xbeef"),
        (",.2r", 4223., "4,200"),
        ("~s", 1500., "1.5k"),
        ("s", 1500., "1.50000k"),
    ] {
        assert_eq!(
            format_number(spec, value, &locale).unwrap(),
            expected,
            "{spec}"
        );
    }
    let pound = FormatLocale {
        currency: ("£".into(), String::new()),
        ..Default::default()
    };
    assert_eq!(format_number("($.2f", -3.5, &pound).unwrap(), "(£3.50)");
}
#[test]
fn locale_currency_grouping_and_numeral_substitution_are_explicit() {
    let de = FormatLocale {
        decimal: ",".into(),
        thousands: ".".into(),
        currency: (String::new(), " €".into()),
        minus: "-".into(),
        ..Default::default()
    };
    assert_eq!(format_number("$,.2f", -1234.5, &de).unwrap(), "-1.234,50 €");
    let bn = FormatLocale {
        numerals: Some(['০', '১', '২', '৩', '৪', '৫', '৬', '৭', '৮', '৯'].map(|c| c.to_string())),
        ..Default::default()
    };
    assert_eq!(format_number(".1f", 12.5, &bn).unwrap(), "১২.৫");
    assert_eq!(
        format_number("+08.1f", 42., &FormatLocale::default()).unwrap(),
        "+00042.0"
    );
}
#[test]
fn exact_fixed_rounding_and_si_prefix_promotion_avoid_double_rounding() {
    let l = FormatLocale::default();
    assert_eq!(format_number(".1f", 1.25, &l).unwrap(), "1.3");
    assert_eq!(format_number(".2f", 2.675, &l).unwrap(), "2.67");
    assert_eq!(format_number(".2f", 1.005, &l).unwrap(), "1.00");
    assert_eq!(format_number(".3s", 999950., &l).unwrap(), "1.00M");
    assert_eq!(format_number(".2e", 42000., &l).unwrap(), "4.20e+4");
    assert_eq!(format_number(".2f", -0.0001, &l).unwrap(), "0.00");
    assert_eq!(format_number("+.2f", -0., &l).unwrap(), "−0.00");
}
#[test]
fn unsupported_or_unbounded_formats_and_values_reject_before_output() {
    let l = FormatLocale::default();
    for spec in ["nonsense", ".21f", ".22g", "257f", "\n.2f"] {
        assert!(NumberFormat::new(spec, &l).is_err(), "{spec:?}");
    }
    assert!(format_number("c", 42., &l).is_err());
    assert!(format_number(".2f", f64::NAN, &l).is_err());
    assert!(format_number(".2%", f64::MAX, &l).is_err());
    let invalid = FormatLocale {
        grouping: vec![0],
        ..Default::default()
    };
    assert!(NumberFormat::new("f", &invalid).is_err());
    assert_eq!(
        ValueFormat::specifier("$,.2f", &l)
            .unwrap()
            .format(1234.5)
            .unwrap(),
        "$1,234.50"
    );
}
