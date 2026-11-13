use kartis::*;
fn format(spec: &str) -> DateTimeFormat {
    DateTimeFormat::utc(spec, &TimeLocale::default()).unwrap()
}
#[test]
fn locale_directory_uses_primary_d3_names_and_full_template_roundtrips() {
    for name in TimeLocale::NAMES {
        let locale = TimeLocale::for_name(name).unwrap();
        let f = DateTimeFormat::new(
            "%c | %x | %X | %Z",
            &locale,
            TimeZone::fixed_offset(345).unwrap(),
        )
        .unwrap();
        let text = f.format(951827696000).unwrap();
        assert_eq!(f.parse(&text).unwrap(), 951827696000, "{name}: {text}");
    }
    assert_eq!(
        TimeLocale::for_name("zh-CN").unwrap().short_months[0],
        "一月"
    );
    assert!(TimeLocale::for_name("missing").is_err());
    assert!(TimeLocale::for_name("en-us").is_err());
}
#[test]
fn negative_epoch_extended_years_fractional_seconds_and_zone_roundtrip() {
    let f = format("%Y-%m-%d %H:%M:%S.%L %f %Q %s %Z");
    for instant in [
        -MAX_TIMESTAMP_MS,
        -62198755200000,
        -1001,
        -1,
        0,
        1,
        951827696789,
        MAX_TIMESTAMP_MS,
    ] {
        let text = f.format(instant).unwrap();
        assert_eq!(f.parse(&text).unwrap(), instant, "{text}");
    }
    assert_eq!(format("%s.%L").parse("-1.999").unwrap(), -1);
    assert_eq!(format("%Y%m%d").parse("20240229").unwrap(), 1709164800000);
    assert_eq!(
        format("%Y-%m-%d %H:%M %Z")
            .parse("1970-01-01 05:45 +05:45")
            .unwrap(),
        0
    );
    assert_eq!(
        format("%Y-%m-%d %H:%M %Z")
            .parse("1970-01-01 00:00 Z")
            .unwrap(),
        0
    );
    assert_eq!(format("%y").parse("68").unwrap(), 3092601600000);
    assert_eq!(format("%y").parse("69").unwrap(), -31536000000);
    let f = format("%Y %y %m %d %G %g %V %u");
    for instant in [-62198755200000, 0, 951782400000, 1609459200000] {
        let text = f.format(instant).unwrap();
        assert_eq!(f.parse(&text).unwrap(), instant, "{text}");
    }
}
#[test]
fn iso_ordinal_weekday_quarter_and_sunday_monday_weeks_are_real_dates() {
    assert_eq!(
        format("%G-%V-%u").parse("2020-53-5").unwrap(),
        1609459200000
    );
    assert_eq!(format("%Y-%j").parse("2024-060").unwrap(), 1709164800000);
    for spec in ["%Y-%m-%d %A %j %q %U %W %G-%V-%u", "%Y %U %w", "%Y %W %u"] {
        let f = format(spec);
        for instant in [
            0,
            1451606400000,
            1451779200000,
            1451865600000,
            1735516800000,
        ] {
            let text = f.format(instant).unwrap();
            assert_eq!(f.parse(&text).unwrap(), instant, "{spec}: {text}");
        }
    }
    assert_eq!(format("%Y Q%q").parse("2024 Q2").unwrap(), 1711929600000);
}
#[test]
fn names_padding_period_and_repeated_fields_are_checked() {
    let f = format("%B %e %Y %I:%M %p");
    assert_eq!(f.parse("fEbRuArY 29 2024 12:00 PM").unwrap(), 1709208000000);
    assert_eq!(format("%Y %Y").parse("1970 1970").unwrap(), 0);
    assert!(format("%Y %Y").parse("1970 1971").is_err());
    assert!(format("%H %I %p").parse("13 01 AM").is_err());
    assert_eq!(format("%Q %p").parse("0 AM").unwrap(), 0);
    assert!(format("%Q %p").parse("0 PM").is_err());
}

#[test]
fn calendar_fields_roundtrip_across_a_full_gregorian_cycle() {
    let f = format("%Y-%m-%d %a %j %q %U %W %G-%g-%V-%u");
    for day in (-146097..146097).step_by(113) {
        let instant = day * 86_400_000;
        let text = f.format(instant).unwrap();
        assert_eq!(f.parse(&text).unwrap(), instant, "{text}");
    }
}
#[test]
fn invalid_input_conflicts_dates_and_unconsumed_text_reject() {
    for text in [
        "2023-02-29",
        "2024-02-30",
        "2024-13-01",
        "2024-00-01",
        "2024-01-00",
        "2024-01-32",
        "2024-01-01 ",
        "2024-01-01suffix",
    ] {
        assert!(format("%Y-%m-%d").parse(text).is_err(), "{text}");
    }
    for (spec, text) in [
        ("%Y-%m-%d %a", "2024-02-29 Fri"),
        ("%Y-%m-%d %j", "2024-02-29 061"),
        ("%G-%V-%u", "2021-53-1"),
        ("%Y-%j", "2023-366"),
        ("%Y-%m Q%q", "2024-01 Q2"),
        ("%Q %s", "1000 2"),
        ("%Q %Y", "0 2000"),
        ("%f", "123456"),
        ("%H:%M:%S", "24:00:00"),
        ("%Z", "+2460"),
        ("%Z", "+1"),
        ("%Z", "+01:2"),
        ("%p", "AM"),
    ] {
        assert!(format(spec).parse(text).is_err(), "{spec}: {text}");
    }
    assert!(format("%Q").parse(&"9".repeat(1025)).is_err());
}
#[cfg(feature = "timezone")]
#[test]
fn iana_parsing_requires_explicit_gap_fold_choices_and_offset_overrides_zone() {
    let f = DateTimeFormat::new(
        "%Y-%m-%d %H:%M",
        &TimeLocale::default(),
        TimeZone::iana("America/New_York").unwrap(),
    )
    .unwrap();
    assert!(f.parse("2024-11-03 01:30").is_err());
    let early = f
        .parse_with(
            "2024-11-03 01:30",
            DateTimeParseOptions {
                fold: ParseFold::Earlier,
                ..Default::default()
            },
        )
        .unwrap();
    let late = f
        .parse_with(
            "2024-11-03 01:30",
            DateTimeParseOptions {
                fold: ParseFold::Later,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(late - early, 3600000);
    assert_eq!(early, 1730611800000);
    assert!(f.parse("2024-03-10 02:30").is_err());
    assert_eq!(
        f.parse_with(
            "2024-03-10 02:30",
            DateTimeParseOptions {
                gap: ParseGap::NextValid,
                ..Default::default()
            }
        )
        .unwrap(),
        1710054000000
    );
    let offset = DateTimeFormat::new(
        "%Y-%m-%d %H:%M %Z",
        &TimeLocale::default(),
        TimeZone::iana("America/New_York").unwrap(),
    )
    .unwrap();
    assert_eq!(offset.parse("2024-11-03 01:30 -0500").unwrap(), late);
}
