use kartis::*;

fn utc(spec: &str) -> DateTimeFormat {
    DateTimeFormat::utc(spec, &TimeLocale::default()).unwrap()
}
const DAY: i64 = 86_400_000;

#[test]
fn calendar_and_week_golden_values_match_independent_python_strftime_fixtures() {
    // Precomputed with Python datetime/strftime, independently of this crate.
    let fixtures = [
        (-1, "1969-12-31 Wed 365 3 3 52 52 1970-01"),
        (0, "1970-01-01 Thu 001 4 4 00 00 1970-01"),
        (951827696789, "2000-02-29 Tue 060 2 2 09 09 2000-09"),
        (-2203891200000, "1900-03-01 Thu 060 4 4 08 09 1900-09"),
        (1451520000000, "2015-12-31 Thu 365 4 4 52 52 2015-53"),
        (1451606400000, "2016-01-01 Fri 001 5 5 00 00 2015-53"),
        (1451779200000, "2016-01-03 Sun 003 0 7 01 00 2015-53"),
        (1451865600000, "2016-01-04 Mon 004 1 1 01 01 2016-01"),
        (1609372800000, "2020-12-31 Thu 366 4 4 52 52 2020-53"),
        (1609459200000, "2021-01-01 Fri 001 5 5 00 00 2020-53"),
        (1672531200000, "2023-01-01 Sun 001 0 7 01 00 2022-52"),
        (1735516800000, "2024-12-30 Mon 365 1 1 52 53 2025-01"),
        (1709164800000, "2024-02-29 Thu 060 4 4 08 09 2024-09"),
    ];
    let format = utc("%Y-%m-%d %a %j %w %u %U %W %G-%V");
    for (timestamp, expected) in fixtures {
        assert_eq!(format.format(timestamp).unwrap(), expected);
    }
}

#[test]
fn negative_epoch_milliseconds_floor_seconds_and_keep_all_subsecond_fields() {
    let format = utc("%Y-%m-%d %H:%M:%S.%L %f %Q %s %Z");
    assert_eq!(
        format.format(-1).unwrap(),
        "1969-12-31 23:59:59.999 999000 -1 -1 +0000"
    );
    assert_eq!(
        format.format(-1001).unwrap(),
        "1969-12-31 23:59:58.999 999000 -1001 -2 +0000"
    );
    assert_eq!(
        format.format(1).unwrap(),
        "1970-01-01 00:00:00.001 001000 1 0 +0000"
    );
    assert_eq!(
        utc("%f|%-f|%_f|%0f").format(0).unwrap(),
        "000000|0000|  0000|000000"
    );
    assert_eq!(utc("%I %p %H").format(0).unwrap(), "12 AM 00");
    assert_eq!(utc("%I %p %H").format(12 * 3_600_000).unwrap(), "12 PM 12");
}

#[test]
fn locale_templates_names_padding_and_literals_are_explicit() {
    assert_eq!(utc("%c").format(0).unwrap(), "1/1/1970, 12:00:00 AM");
    assert_eq!(
        utc("%A %B %b %a %q %y %g").format(951827696789).unwrap(),
        "Tuesday February Feb Tue 1 00 00"
    );
    assert_eq!(
        utc("%d|%e|%-d|%_d|%0e|%%").format(0).unwrap(),
        "01| 1|1| 1|01|%"
    );
    let zh = DateTimeFormat::new(
        "%c %p %b",
        &TimeLocale::zh_cn(),
        TimeZone::fixed_offset(480).unwrap(),
    )
    .unwrap();
    assert_eq!(
        zh.format(0).unwrap(),
        "1970年1月1日 星期四 08:00:00 上午 1月"
    );
    let mut custom = TimeLocale::default();
    custom.months[0] = "一月 %Y".into(); // Names are literal, never recursive templates.
    custom.date = "%B %d %Y".into();
    custom.date_time = "[%x] %X".into();
    let format = DateTimeFormat::utc("日期 %c ✓", &custom).unwrap();
    assert_eq!(
        format.format(0).unwrap(),
        "日期 [一月 %Y 01 1970] 12:00:00 AM ✓"
    );
    custom.months[0] = "Changed later".into();
    assert!(format.format(0).unwrap().contains("一月 %Y"));
}

#[test]
fn offsets_roll_calendar_week_and_clock_without_changing_epoch_fields() {
    let west = DateTimeFormat::new(
        "%Y-%m-%d %H:%M:%S.%L %A %j %G-%V %Z %Q %s",
        &TimeLocale::default(),
        TimeZone::fixed_offset(-330).unwrap(),
    )
    .unwrap();
    assert_eq!(
        west.format(0).unwrap(),
        "1969-12-31 18:30:00.000 Wednesday 365 1970-01 -0530 0 0"
    );
    let east = DateTimeFormat::new(
        "%Y-%m-%d %H:%M %Z",
        &TimeLocale::default(),
        TimeZone::fixed_offset(345).unwrap(),
    )
    .unwrap();
    assert_eq!(east.format(-1).unwrap(), "1970-01-01 05:44 +0545");
    for minutes in [-1439, 1439] {
        let zone = TimeZone::fixed_offset(minutes).unwrap();
        assert_eq!(zone.offset_seconds_at(0).unwrap(), i32::from(minutes) * 60);
        let format =
            DateTimeFormat::new("%Y-%m-%d %H:%M:%S.%L %Z", &TimeLocale::default(), zone).unwrap();
        for timestamp in [-MAX_TIMESTAMP_MS, MAX_TIMESTAMP_MS] {
            assert!(format.format(timestamp).is_ok());
        }
    }
}

#[test]
fn extended_years_and_full_bound_keep_the_same_calendar_as_existing_utc() {
    let format = utc("%Y-%m-%d %H:%M:%S.%L UTC");
    for timestamp in [
        -MAX_TIMESTAMP_MS,
        -MAX_TIMESTAMP_MS + 1,
        -62_167_219_200_000,
        -62_135_596_800_000,
        -1,
        0,
        253_402_300_799_999,
        253_402_300_800_000,
        MAX_TIMESTAMP_MS - 1,
        MAX_TIMESTAMP_MS,
    ] {
        assert_eq!(
            format.format(timestamp).unwrap(),
            format_utc(timestamp).unwrap()
        );
    }
    assert_eq!(
        format.format(MAX_TIMESTAMP_MS).unwrap(),
        "+275760-09-13 00:00:00.000 UTC"
    );
    assert_eq!(
        format.format(-MAX_TIMESTAMP_MS).unwrap(),
        "-271821-04-20 00:00:00.000 UTC"
    );
    assert_eq!(
        utc("%Y %y").format(-62_198_755_200_000).unwrap(),
        "-0001 -01"
    );
}

#[test]
fn entire_gregorian_cycle_has_valid_weeks_days_and_repeats_after_400_years() {
    let format = utc("%a %b %d %j %w %u %U %W %V");
    // A complete Gregorian cycle, sampled across every month and both signs.
    for day in (-146_097..146_097).step_by(29) {
        let timestamp = day * DAY;
        assert_eq!(
            format.format(timestamp).unwrap(),
            format.format(timestamp + 146_097 * DAY).unwrap()
        );
    }
}

#[test]
fn invalid_specs_locales_offsets_and_epoch_inputs_reject_boundedly() {
    let locale = TimeLocale::default();
    for spec in ["%", "%0", "%_", "%-", "%z", "%n", "%t", "%04Y", "%年"] {
        assert!(DateTimeFormat::utc(spec, &locale).is_err(), "{spec}");
    }
    assert!(DateTimeFormat::utc(&"x".repeat(MAX_TIME_FORMAT_BYTES + 1), &locale).is_err());
    assert!(DateTimeFormat::utc(&"%c".repeat(512), &locale).is_err());
    for minutes in [i16::MIN, -1440, 1440, i16::MAX] {
        assert!(TimeZone::fixed_offset(minutes).is_err());
        assert!(DateTimeFormat::new("%Y", &locale, TimeZone::FixedOffset { minutes }).is_err());
    }
    for timestamp in [
        i64::MIN,
        -MAX_TIMESTAMP_MS - 1,
        MAX_TIMESTAMP_MS + 1,
        i64::MAX,
    ] {
        assert!(utc("%Y").format(timestamp).is_err());
    }
    let mut cyclic = locale.clone();
    cyclic.date = "%c".into();
    assert!(DateTimeFormat::utc("unreferenced", &cyclic).is_err());
    let mut mutual = locale.clone();
    mutual.date = "%X".into();
    mutual.time = "%x".into();
    assert!(mutual.validate().is_err());
    let mut oversized = locale.clone();
    oversized.days[0] = "界".repeat(86);
    assert!(oversized.validate().is_err());
    let mut unknown = locale.clone();
    unknown.time = "%z".into();
    assert!(unknown.validate().is_err());
    let mut empty = locale;
    empty.months[0].clear();
    assert!(empty.validate().is_err());
}

#[test]
fn output_limits_are_bytes_and_failure_does_not_change_reusable_formatter() {
    let mut locale = TimeLocale::default();
    locale.months[0] = "界".repeat(85);
    let formatter = DateTimeFormat::utc("%B%B%B%B%B", &locale).unwrap();
    assert!(formatter.format(0).is_err());
    assert_eq!(formatter.format(31 * DAY).unwrap(), "February".repeat(5));
    assert_eq!(
        DateTimeFormat::utc(&"界".repeat(341), &TimeLocale::default())
            .unwrap()
            .format(0)
            .unwrap()
            .len(),
        1023
    );
}

#[test]
fn value_format_axes_and_tooltip_adapter_accept_only_integral_bounded_instants() {
    let format = ValueFormat::date_time(
        "%m月%d日 %H:%M %Z",
        &TimeLocale::zh_cn(),
        TimeZone::fixed_offset(480).unwrap(),
    )
    .unwrap();
    assert_eq!(format.format(0.).unwrap(), "01月01日 08:00 +0800");
    for value in [0.1, f64::NAN, f64::INFINITY, (MAX_TIMESTAMP_MS + 1) as f64] {
        assert!(format.format(value).is_err());
    }
    let mut options = LineOptions {
        x_scale: ScaleKind::Utc,
        ..Default::default()
    };
    options.axes.x.format = format;
    options.axes.x.tick_values = Some(vec![0., DAY as f64]);
    let scene = line(
        &ChartConfig::default(),
        &[Series::new(
            "dates",
            vec![Point::new(0., 1.), Point::new(DAY as f64, 2.)],
        )],
        &options,
    )
    .unwrap();
    assert!(
        scene
            .labels
            .iter()
            .any(|l| l.text == "01月01日 08:00 +0800")
    );
    assert!(
        scene
            .labels
            .iter()
            .any(|l| l.text == "01月02日 08:00 +0800")
    );
}

#[cfg(feature = "timezone")]
#[test]
fn iana_new_york_spring_gap_and_fall_repeat_use_exact_instant_offsets() {
    let zone = TimeZone::iana("America/New_York").unwrap();
    let format =
        DateTimeFormat::new("%Y-%m-%d %H:%M:%S.%L %Z", &TimeLocale::default(), zone).unwrap();
    for (timestamp, expected) in [
        (1710053999999, "2024-03-10 01:59:59.999 -0500"),
        (1710054000000, "2024-03-10 03:00:00.000 -0400"),
        (1730613599999, "2024-11-03 01:59:59.999 -0400"),
        (1730613600000, "2024-11-03 01:00:00.000 -0500"),
    ] {
        assert_eq!(format.format(timestamp).unwrap(), expected);
    }
}

#[cfg(feature = "timezone")]
#[test]
fn iana_shanghai_historical_dst_kolkata_and_second_precision_offsets() {
    let shanghai = TimeZone::iana("Asia/Shanghai").unwrap();
    let format =
        DateTimeFormat::new("%Y-%m-%d %H:%M:%S %Z", &TimeLocale::default(), shanghai).unwrap();
    // 1991-04-14 02:00 local jumps to 03:00; 1991-09-15 02:00 repeats 01:00.
    for (timestamp, expected) in [
        (671565599000, "1991-04-14 01:59:59 +0800"),
        (671565600000, "1991-04-14 03:00:00 +0900"),
        (684867599000, "1991-09-15 01:59:59 +0900"),
        (684867600000, "1991-09-15 01:00:00 +0800"),
    ] {
        assert_eq!(format.format(timestamp).unwrap(), expected);
    }
    let kolkata = DateTimeFormat::new(
        "%Y-%m-%d %H:%M:%S %Z",
        &TimeLocale::default(),
        TimeZone::iana("Asia/Kolkata").unwrap(),
    )
    .unwrap();
    assert_eq!(kolkata.format(0).unwrap(), "1970-01-01 05:30:00 +0530");
    let paris = TimeZone::iana("Europe/Paris").unwrap();
    let timestamp = -2_208_988_800_000; // 1900-01-01 UTC; Paris mean time +00:09:21.
    assert_eq!(paris.offset_seconds_at(timestamp).unwrap(), 561);
    let historic = DateTimeFormat::new("%H:%M:%S %Z", &TimeLocale::default(), paris).unwrap();
    assert_eq!(historic.format(timestamp).unwrap(), "00:09:21 +0009");
    assert!(iana_time_database_version().is_some());
}

#[cfg(feature = "timezone")]
#[test]
fn named_zone_validation_and_dependency_civil_bounds_are_explicit() {
    for name in ["", "Invalid/Zone", "america/new_york", "UTC "] {
        assert!(TimeZone::iana(name).is_err());
    }
    assert!(TimeZone::iana(&"a".repeat(129)).is_err());
    let format =
        DateTimeFormat::new("%c", &TimeLocale::default(), TimeZone::iana("UTC").unwrap()).unwrap();
    for instant in [-MAX_TIMESTAMP_MS, MAX_TIMESTAMP_MS] {
        assert!(format.format(instant).is_err());
    }
    assert!(
        DateTimeFormat::utc("%c", &TimeLocale::default())
            .unwrap()
            .format(MAX_TIMESTAMP_MS)
            .is_ok()
    );
    let [min, max] = iana_timestamp_bounds();
    assert!(format.format(min).is_ok());
    assert!(
        format.format(max).is_ok(),
        "max={max}: {:?}",
        format.format(max)
    );
    assert!(format.format(min - 1).is_err());
    assert!(format.format(max + 1).is_err());
}

#[cfg(feature = "timezone")]
#[test]
fn tzif_posix_tail_rules_keep_future_northern_and_southern_dst_past_2100() {
    let ny = DateTimeFormat::new(
        "%Y-%m-%d %H:%M %Z",
        &TimeLocale::default(),
        TimeZone::iana("America/New_York").unwrap(),
    )
    .unwrap();
    assert_eq!(
        ny.format(4_135_190_400_000).unwrap(),
        "2101-01-14 19:00 -0500"
    );
    assert_eq!(
        ny.format(4_150_828_800_000).unwrap(),
        "2101-07-14 20:00 -0400"
    );
    let sydney = DateTimeFormat::new(
        "%Y-%m-%d %H:%M %Z",
        &TimeLocale::default(),
        TimeZone::iana("Australia/Sydney").unwrap(),
    )
    .unwrap();
    assert_eq!(
        sydney.format(4_133_894_400_000 + 13 * 3_600_000).unwrap(),
        "2101-01-01 00:00 +1100"
    );
    assert_eq!(
        sydney.format(4_148_236_800_000).unwrap(),
        "2101-06-15 10:00 +1000"
    );
    assert_eq!(
        sydney.format(4_150_828_800_000).unwrap(),
        "2101-07-15 10:00 +1000"
    );
}
