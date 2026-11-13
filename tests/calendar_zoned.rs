use kartis::*;
fn observation(timestamp_ms: i64, value: f64) -> CalendarInstant {
    CalendarInstant {
        timestamp_ms,
        value,
    }
}
#[test]
fn fixed_offsets_negative_epoch_and_inclusive_local_range_are_dates_not_durations() {
    let zone = TimeZone::fixed_offset(480).unwrap();
    let date = calendar_date_at(-1, 3., &zone).unwrap();
    assert_eq!((date.year, date.month, date.day), (1970, 1, 1));
    let (year, day) = calendar_day_at(0, 4., &TimeZone::fixed_offset(-300).unwrap()).unwrap();
    assert_eq!((year, day.month, day.day), (1969, 12, 31));
    let data = [
        observation(-28_800_001, 99.),
        observation(-28_800_000, 2.),
        observation(57_599_999, 3.),
        observation(57_600_000, 99.),
    ];
    let dates = aggregate_calendar_instants(
        &data,
        &zone,
        (1970, 1, 1),
        (1970, 1, 1),
        CalendarAggregation::Sum,
    )
    .unwrap();
    assert_eq!(dates, vec![CalendarDate::new(1970, 1, 1, 5.)]);
}
#[test]
fn aggregation_profiles_permutation_extremes_and_empty_days_are_explicit() {
    let mut data = [
        observation(0, 2.),
        observation(10, 4.),
        observation(20, -3.),
    ];
    for (aggregation, value) in [
        (CalendarAggregation::Sum, 3.),
        (CalendarAggregation::Mean, 1.),
        (CalendarAggregation::Min, -3.),
        (CalendarAggregation::Max, 4.),
        (CalendarAggregation::Count, 3.),
    ] {
        let result = aggregate_calendar_instants(
            &data,
            &TimeZone::Utc,
            (1970, 1, 1),
            (1970, 1, 2),
            aggregation,
        )
        .unwrap();
        assert_eq!(result, vec![CalendarDate::new(1970, 1, 1, value)]);
        data.reverse();
        assert_eq!(
            result,
            aggregate_calendar_instants(
                &data,
                &TimeZone::Utc,
                (1970, 1, 1),
                (1970, 1, 2),
                aggregation
            )
            .unwrap()
        );
    }
    let extreme = [observation(0, f64::MAX), observation(1, f64::MAX)];
    assert!(
        aggregate_calendar_instants(
            &extreme,
            &TimeZone::Utc,
            (1970, 1, 1),
            (1970, 1, 1),
            CalendarAggregation::Sum
        )
        .is_err()
    );
    assert_eq!(
        aggregate_calendar_instants(
            &extreme,
            &TimeZone::Utc,
            (1970, 1, 1),
            (1970, 1, 1),
            CalendarAggregation::Mean
        )
        .unwrap()[0]
            .value,
        f64::MAX
    );
    let repeated: Vec<_> = (0..1000).map(|i| observation(i, f64::MAX)).collect();
    assert_eq!(
        aggregate_calendar_instants(
            &repeated,
            &TimeZone::Utc,
            (1970, 1, 1),
            (1970, 1, 1),
            CalendarAggregation::Mean
        )
        .unwrap()[0]
            .value,
        f64::MAX
    );
}
#[test]
fn rendered_calendar_uses_local_date_identity_and_invalid_proposal_preserves_old_frame() {
    let options = CalendarRangeOptions {
        start: Some((1969, 12, 31)),
        end: Some((1970, 1, 1)),
        ..Default::default()
    };
    let config = ChartConfig::default();
    let data = [observation(0, 2.), observation(10, 4.)];
    let scene = calendar_range_zoned(
        &config,
        &data,
        &TimeZone::fixed_offset(-300).unwrap(),
        CalendarAggregation::Sum,
        &options,
    )
    .unwrap();
    let mark = scene.marks.iter().find(|m| m.id == "1969-12-31").unwrap();
    assert_eq!(mark.value, 6.);
    assert_eq!(scene.marks.len(), 2);
    let old = scene.clone();
    assert!(
        calendar_range_zoned(
            &config,
            &[observation(0, f64::NAN)],
            &TimeZone::Utc,
            CalendarAggregation::Sum,
            &options
        )
        .is_err()
    );
    assert_eq!(scene, old);
    assert!(
        calendar_range_zoned(
            &config,
            &[],
            &TimeZone::Utc,
            CalendarAggregation::Sum,
            &Default::default()
        )
        .is_err()
    );
}
#[test]
fn bad_dates_instants_ranges_and_budgets_reject() {
    for (start, end) in [
        ((2023, 2, 29), (2023, 3, 1)),
        ((2024, 1, 2), (2024, 1, 1)),
        ((1970, 1, 1), (2002, 1, 1)),
        ((0, 1, 1), (1, 1, 1)),
    ] {
        assert!(
            aggregate_calendar_instants(&[], &TimeZone::Utc, start, end, CalendarAggregation::Sum)
                .is_err()
        );
    }
    for instant in [-MAX_TIMESTAMP_MS, MAX_TIMESTAMP_MS, i64::MAX] {
        assert!(calendar_date_at(instant, 1., &TimeZone::Utc).is_err());
    }
    assert!(
        aggregate_calendar_instants(
            &vec![observation(0, 1.); MAX_CALENDAR_INSTANTS + 1],
            &TimeZone::Utc,
            (1970, 1, 1),
            (1970, 1, 1),
            CalendarAggregation::Count
        )
        .is_err()
    );
    // Values outside the requested date interval are still validated.
    assert!(
        aggregate_calendar_instants(
            &[observation(0, f64::NAN)],
            &TimeZone::Utc,
            (2000, 1, 1),
            (2000, 1, 1),
            CalendarAggregation::Count
        )
        .is_err()
    );
}
#[cfg(feature = "timezone")]
#[test]
fn new_york_repeated_hours_aggregate_same_date_and_23_hour_day_ends_at_local_midnight() {
    let zone = TimeZone::iana("America/New_York").unwrap();
    let data = [
        observation(1730611800000, 2.),
        observation(1730615400000, 3.),
    ];
    let dates = aggregate_calendar_instants(
        &data,
        &zone,
        (2024, 11, 3),
        (2024, 11, 3),
        CalendarAggregation::Sum,
    )
    .unwrap();
    assert_eq!(dates, vec![CalendarDate::new(2024, 11, 3, 5.)]);
    let data = [
        observation(1710046799999, 99.),
        observation(1710046800000, 2.),
        observation(1710129599999, 3.),
        observation(1710129600000, 99.),
    ];
    assert_eq!(
        aggregate_calendar_instants(
            &data,
            &zone,
            (2024, 3, 10),
            (2024, 3, 10),
            CalendarAggregation::Sum
        )
        .unwrap(),
        vec![CalendarDate::new(2024, 3, 10, 5.)]
    );
}
