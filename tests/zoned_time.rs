use kartis::*;
const DAY: i64 = 86_400_000;
fn interval(unit: TimeUnit, step: u32) -> TimeInterval {
    TimeInterval::new(unit, step).unwrap()
}
fn scale(domain: [i64; 2], zone: TimeZone) -> ZonedTimeScale {
    ZonedTimeScale::new(domain, [0., 100.], TimeCalendar::new(zone)).unwrap()
}
fn ordered(ticks: &[i64]) {
    assert!(ticks.windows(2).all(|w| w[0] < w[1]));
}

#[test]
fn field_alignment_restarts_days_months_hours_minutes_at_parent_boundary() {
    let s = scale([1706572800000, 1707091200000], TimeZone::Utc); // Jan30..Feb5
    let f = DateTimeFormat::utc("%m-%d", &TimeLocale::default()).unwrap();
    let labels: Vec<_> = s
        .timestamps_every(interval(TimeUnit::Day, 2))
        .unwrap()
        .into_iter()
        .map(|v| f.format(v).unwrap())
        .collect();
    assert_eq!(labels, ["01-31", "02-01", "02-03", "02-05"]);
    let s = scale([0, 2 * DAY], TimeZone::fixed_offset(330).unwrap());
    let f = DateTimeFormat::new(
        "%H:%M",
        &TimeLocale::default(),
        TimeZone::fixed_offset(330).unwrap(),
    )
    .unwrap();
    let labels: Vec<_> = s
        .timestamps_every(interval(TimeUnit::Hour, 5))
        .unwrap()
        .into_iter()
        .map(|v| f.format(v).unwrap())
        .collect();
    assert_eq!(
        &labels[..6],
        ["10:00", "15:00", "20:00", "00:00", "05:00", "10:00"]
    );
    let s = scale([0, 3_600_000], TimeZone::Utc);
    let labels: Vec<_> = s
        .ticks_every(interval(TimeUnit::Minute, 7))
        .unwrap()
        .into_iter()
        .map(|v| v.timestamp_ms / 60_000)
        .collect();
    assert_eq!(labels, [0, 7, 14, 21, 28, 35, 42, 49, 56, 60]);
    let s = scale([1722470400000, 1735689600000], TimeZone::Utc); // Aug1..Jan1
    let f = DateTimeFormat::utc("%Y-%m", &TimeLocale::default()).unwrap();
    let labels: Vec<_> = s
        .timestamps_every(interval(TimeUnit::Month, 5))
        .unwrap()
        .into_iter()
        .map(|v| f.format(v).unwrap())
        .collect();
    assert_eq!(labels, ["2024-11", "2025-01"]);
}
#[test]
fn fixed_offset_local_midnight_month_quarter_year_week_and_negative_floor() {
    let zone = TimeZone::fixed_offset(345).unwrap();
    let s = scale([-DAY, 3 * DAY], zone.clone());
    let ticks = s.timestamps_every(interval(TimeUnit::Day, 1)).unwrap();
    ordered(&ticks);
    assert!(
        ticks
            .iter()
            .all(|v| (v + 345 * 60_000).rem_euclid(DAY) == 0)
    );
    assert_eq!(
        interval(TimeUnit::Day, 1)
            .floor(-1, &TimeCalendar::new(zone))
            .unwrap(),
        Some(-20_700_000)
    );
    let s = scale([1704067200000, 1735689600000], TimeZone::Utc);
    assert_eq!(
        s.timestamps_every(interval(TimeUnit::Month, 3)).unwrap(),
        [
            1704067200000,
            1711929600000,
            1719792000000,
            1727740800000,
            1735689600000
        ]
    );
    assert_eq!(
        s.timestamps_every(interval(TimeUnit::Year, 1)).unwrap(),
        [1704067200000, 1735689600000]
    );
    let weeks = scale([0, 14 * DAY], TimeZone::Utc)
        .timestamps_every(interval(TimeUnit::Week, 1))
        .unwrap();
    assert_eq!(weeks, [4 * DAY, 11 * DAY]);
    assert_eq!(s.invert(s.map(1706745600123)).unwrap(), 1706745600123);
}
#[test]
fn civil_offset_clamps_month_day_and_budgets_are_explicit() {
    let c = TimeCalendar::default();
    assert_eq!(
        interval(TimeUnit::Month, 1)
            .offset(1706659200000, 1, &c)
            .unwrap(),
        [1709164800000]
    ); // Jan31->Feb29
    assert!(TimeInterval::new(TimeUnit::Day, 0).is_err());
    assert!(interval(TimeUnit::Day, 1).offset(0, 10001, &c).is_err());
    let s = scale([0, 10 * DAY], TimeZone::Utc);
    assert!(
        s.timestamps_every(interval(TimeUnit::Millisecond, 1))
            .is_err()
    );
    assert!(s.ticks(1001).is_err());
    assert!(s.ticks(0).unwrap().is_empty());
    let mut reverse = scale([10 * DAY, 0], TimeZone::Utc)
        .timestamps_every(interval(TimeUnit::Day, 1))
        .unwrap();
    reverse.reverse();
    assert_eq!(
        reverse,
        s.timestamps_every(interval(TimeUnit::Day, 1)).unwrap()
    );
    for bound in [-MAX_TIMESTAMP_MS, MAX_TIMESTAMP_MS] {
        scale([bound, bound], TimeZone::Utc).ticks(1).unwrap();
    }
}
fn axis_options(calendar: TimeCalendar) -> AxisOptions {
    AxisOptions {
        tick_calendar: Some(calendar),
        ..Default::default()
    }
}
#[test]
fn calendar_ticks_reach_line_dual_axis_combo_and_rejection_is_atomic() {
    let mut c = TimeCalendar::new(TimeZone::fixed_offset(480).unwrap());
    c.interval = Some(interval(TimeUnit::Day, 1));
    let axis = axis_options(c);
    let data = vec![Point::new(0., 1.), Point::new(2. * DAY as f64, 2.)];
    let mut options = LineOptions {
        x_scale: ScaleKind::Utc,
        ..Default::default()
    };
    options.axes.x = axis.clone();
    let a = line(
        &ChartConfig::default(),
        &[Series::new("A", data.clone())],
        &options,
    )
    .unwrap();
    let b = dual_axis(
        &ChartConfig::default(),
        &[DualAxisSeries::new(Series::new("A", data), YAxisSide::Left)],
        &DualAxisOptions {
            x_scale: ScaleKind::Utc,
            x_axis: axis.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    let d = combo(
        &ChartConfig::default(),
        &[ComboSeries::new(
            "A",
            ComboKind::Line,
            vec![
                ComboPoint::new(0., 1.),
                ComboPoint::new(2. * DAY as f64, 2.),
            ],
        )],
        &ComboOptions {
            x: ComboXAxis::Numeric {
                domain: None,
                scale: ScaleKind::Utc,
            },
            x_axis: axis.clone(),
            ..Default::default()
        },
    )
    .unwrap();
    for scene in [a, b, d] {
        assert!(
            scene
                .labels
                .iter()
                .any(|l| l.text == "1970-01-02 00:00:00.000 +0800")
        );
    }
    let mut scene = ChartConfig::default().scene().unwrap();
    let old = scene.clone();
    assert!(
        axes::axis(
            &mut scene,
            &ChartConfig::default(),
            &ChartScale::new(ScaleKind::Linear, [0., 1.], [0., 100.]).unwrap(),
            AxisSide::Bottom,
            &axis,
            "bad"
        )
        .is_err()
    );
    assert_eq!(scene, old);
    let mut bad = axis;
    bad.tick_values = Some(vec![0.]);
    assert!(bad.validate().is_err());
}

#[cfg(feature = "timezone")]
fn named(name: &str) -> TimeZone {
    TimeZone::iana(name).unwrap()
}
#[cfg(feature = "timezone")]
#[test]
fn new_york_days_are_23_or_25_hours_and_hours_skip_or_repeat() {
    let spring = scale([1709960400000, 1710216000000], named("America/New_York"));
    assert_eq!(
        spring.timestamps_every(interval(TimeUnit::Day, 1)).unwrap(),
        [1709960400000, 1710046800000, 1710129600000, 1710216000000]
    );
    let fall = scale([1730520000000, 1730782800000], named("America/New_York"));
    assert_eq!(
        fall.timestamps_every(interval(TimeUnit::Day, 1)).unwrap(),
        [1730520000000, 1730606400000, 1730696400000, 1730782800000]
    );
    let hours = scale([1730606400000, 1730624400000], named("America/New_York"))
        .ticks_every(interval(TimeUnit::Hour, 1))
        .unwrap();
    assert!(hours.iter().any(|t| t.label.contains("01:00:00.000 -0400")));
    assert!(hours.iter().any(|t| t.label.contains("01:00:00.000 -0500")));
    ordered(&hours.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>());
    let hours = scale([1710046800000, 1710061200000], named("America/New_York"))
        .ticks_every(interval(TimeUnit::Hour, 1))
        .unwrap();
    assert!(!hours.iter().any(|t| t.label.contains("02:00")));
    ordered(&hours.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>());
    let narrow = scale([1730612700000, 1730614500000], named("America/New_York"));
    assert_eq!(
        narrow
            .timestamps_every(interval(TimeUnit::Hour, 1))
            .unwrap(),
        [1730613600000]
    );
}
#[cfg(feature = "timezone")]
#[test]
fn half_hour_folds_skipped_dates_and_midnight_gaps_are_real_boundaries() {
    let lord = scale([1712412000000, 1712422800000], named("Australia/Lord_Howe"));
    let ticks = lord.ticks_every(interval(TimeUnit::Minute, 30)).unwrap();
    assert!(ticks.iter().any(|t| t.label.contains("01:30:00.000 +1100")));
    assert!(ticks.iter().any(|t| t.label.contains("01:30:00.000 +1030")));
    ordered(&ticks.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>());
    let apia = scale([1325152800000, 1325325600000], named("Pacific/Apia"));
    let ticks = apia.ticks_every(interval(TimeUnit::Day, 1)).unwrap();
    assert_eq!(ticks.len(), 3);
    assert!(!ticks.iter().any(|t| t.label.starts_with("2011-12-30")));
    ordered(&ticks.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>());
    let sao = scale([1541300400000, 1541383200000], named("America/Sao_Paulo"));
    let ticks = sao.ticks_every(interval(TimeUnit::Day, 1)).unwrap();
    assert!(ticks[0].label.starts_with("2018-11-04 01:00"));
    assert_eq!(ticks.len(), 2);
}
#[cfg(feature = "timezone")]
#[test]
fn explicit_gap_fold_policies_offset_and_floor_are_observable() {
    let mut calendar = TimeCalendar::new(named("America/New_York"));
    calendar.interval = Some(interval(TimeUnit::Hour, 1));
    let domain = [1730606400000, 1730624400000];
    calendar.fold = FoldPolicy::Earlier;
    let early = ZonedTimeScale::new(domain, [0., 1.], calendar.clone())
        .unwrap()
        .ticks(5)
        .unwrap();
    assert!(!early.iter().any(|t| t.label.contains("01:00:00.000 -0500")));
    calendar.fold = FoldPolicy::Later;
    let late = ZonedTimeScale::new(domain, [0., 1.], calendar.clone())
        .unwrap()
        .ticks(5)
        .unwrap();
    assert!(!late.iter().any(|t| t.label.contains("01:00:00.000 -0400")));
    calendar.fold = FoldPolicy::Reject;
    assert!(
        ZonedTimeScale::new(domain, [0., 1.], calendar.clone())
            .unwrap()
            .ticks(5)
            .is_err()
    );
    calendar.fold = FoldPolicy::Both;
    assert_eq!(
        interval(TimeUnit::Hour, 1)
            .floor(1730614500000, &calendar)
            .unwrap(),
        Some(1730613600000)
    );
    calendar.gap = GapPolicy::Skip;
    assert!(
        interval(TimeUnit::Day, 1)
            .offset(
                1325152800000,
                1,
                &TimeCalendar {
                    zone: named("Pacific/Apia"),
                    gap: GapPolicy::Skip,
                    ..Default::default()
                }
            )
            .unwrap()
            .is_empty()
    );
    calendar.gap = GapPolicy::Reject;
    assert!(
        ZonedTimeScale::new([1710046800000, 1710061200000], [0., 1.], calendar)
            .unwrap()
            .ticks(5)
            .is_err()
    );
}

#[cfg(feature = "timezone")]
#[test]
fn midnight_fold_has_one_date_start_by_default_and_both_only_when_requested() {
    let domain = [1730602800000, 1730700000000];
    let mut calendar = TimeCalendar::new(named("America/Havana"));
    let s = ZonedTimeScale::new(domain, [0., 100.], calendar.clone()).unwrap();
    let ticks = s.ticks_every(interval(TimeUnit::Day, 1)).unwrap();
    let midnight: Vec<_> = ticks
        .iter()
        .filter(|t| t.label.starts_with("2024-11-03 00:00"))
        .collect();
    assert_eq!(midnight.len(), 1);
    assert!(midnight[0].label.ends_with("-0400"));
    calendar.fold = FoldPolicy::Both;
    let both = ZonedTimeScale::new(domain, [0., 100.], calendar)
        .unwrap()
        .ticks_every(interval(TimeUnit::Day, 1))
        .unwrap();
    assert_eq!(
        both.iter()
            .filter(|t| t.label.starts_with("2024-11-03 00:00"))
            .count(),
        2
    );
    ordered(&both.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>());
}

#[cfg(feature = "timezone")]
#[test]
fn automatic_calendar_ticks_and_axis_geometry_follow_true_dst_midnights() {
    let mut calendar = TimeCalendar::new(named("America/New_York"));
    let domain = [1709960400000, 1710216000000];
    let s = ZonedTimeScale::new(domain, [0., 100.], calendar.clone()).unwrap();
    let auto = s.ticks(3).unwrap();
    assert_eq!(
        auto.iter().map(|t| t.timestamp_ms).collect::<Vec<_>>(),
        [1709960400000, 1710046800000, 1710129600000, 1710216000000]
    );
    calendar.interval = Some(interval(TimeUnit::Day, 1));
    let config = ChartConfig::default();
    let scale = ChartScale::new(
        ScaleKind::Utc,
        domain.map(|v| v as f64),
        [config.margin.left, config.width - config.margin.right],
    )
    .unwrap();
    let mut scene = config.scene().unwrap();
    axes::axis(
        &mut scene,
        &config,
        &scale,
        AxisSide::Bottom,
        &axis_options(calendar),
        "local",
    )
    .unwrap();
    assert!(
        scene
            .labels
            .iter()
            .any(|label| label.text == "2024-03-11 00:00:00.000 -0400")
    );
    let x: Vec<_> = scene.labels.iter().map(|l| l.x).collect();
    assert!(((x[1] - x[0]) / (x[2] - x[1]) - 24. / 23.).abs() < 1e-10);
}

#[cfg(feature = "timezone")]
#[test]
fn named_timestamp_endpoints_and_historical_second_offsets_stay_bounded() {
    for bound in iana_timestamp_bounds() {
        let s = scale([bound, bound], named("UTC"));
        s.timestamps_every(interval(TimeUnit::Day, 1)).unwrap();
    }
    let s = scale(
        [-2_208_988_800_000, -2_208_816_000_000],
        named("Europe/Paris"),
    );
    let ticks = s.timestamps_every(interval(TimeUnit::Day, 1)).unwrap();
    assert!(ticks.iter().all(|v| (v + 561000).rem_euclid(DAY) == 0));
    ordered(&ticks);
}
