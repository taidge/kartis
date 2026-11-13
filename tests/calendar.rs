use kartis::*;
#[test]
fn calendar_range_gregorian_years_shared_colors_and_identities() {
    let c = ChartConfig {
        height: 800.,
        ..Default::default()
    };
    let data = vec![
        CalendarDate::new(2023, 12, 31, 4.),
        CalendarDate::new(2024, 2, 29, 4.),
    ];
    let scene = calendar_range(
        &c,
        &data,
        &CalendarRangeOptions {
            years: Some((2023, 2024)),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(scene.marks.len(), 731);
    let first = scene.marks.iter().find(|m| m.id == "2023-12-31").unwrap();
    let second = scene.marks.iter().find(|m| m.id == "2024-02-29").unwrap();
    assert_eq!(first.fill, second.fill);
    assert_eq!(first.series, "year:2023");
    assert_eq!(second.series, "year:2024");
    let vertical = calendar_range(
        &c,
        &data,
        &CalendarRangeOptions {
            years: Some((2023, 2024)),
            orientation: CalendarOrientation::Vertical,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        scene.marks.iter().map(|m| &m.id).collect::<Vec<_>>(),
        vertical.marks.iter().map(|m| &m.id).collect::<Vec<_>>()
    );
    vertical.validate().unwrap();
}
#[test]
fn date_intervals_are_inclusive_and_reject_invalid_days_and_duplicates() {
    let c = ChartConfig::default();
    let options = CalendarRangeOptions {
        start: Some((2023, 12, 30)),
        end: Some((2024, 1, 2)),
        ..Default::default()
    };
    let scene = calendar_range(&c, &[], &options).unwrap();
    assert_eq!(
        scene
            .marks
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        vec!["2023-12-30", "2023-12-31", "2024-01-01", "2024-01-02"]
    );
    assert!(calendar_range(&c, &[CalendarDate::new(2024, 1, 3, 1.)], &options).is_err());
    assert!(
        calendar_range(
            &c,
            &[],
            &CalendarRangeOptions {
                start: Some((1900, 2, 29)),
                end: Some((1900, 3, 1)),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        calendar_range(
            &c,
            &[],
            &CalendarRangeOptions {
                start: Some((2000, 2, 28)),
                end: Some((2000, 3, 1)),
                ..Default::default()
            }
        )
        .unwrap()
        .marks
        .len(),
        3
    );
    assert!(
        calendar_range(
            &c,
            &[
                CalendarDate::new(2024, 1, 1, 1.),
                CalendarDate::new(2024, 1, 1, 2.)
            ],
            &CalendarRangeOptions::default()
        )
        .is_err()
    );
    assert!(
        calendar_range(
            &c,
            &[CalendarDate::new(2024, 2, 30, 1.)],
            &CalendarRangeOptions::default()
        )
        .is_err()
    );
    assert!(
        calendar_range(
            &c,
            &[],
            &CalendarRangeOptions {
                years: Some((2024, 2023)),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(calendar_range(&c, &[], &CalendarRangeOptions::default()).is_err());
}
