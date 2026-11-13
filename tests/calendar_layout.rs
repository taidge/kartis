use glory::render::{GraphicStroke, PathCommand as C, StrokeCap};
use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 720.,
        height: 420.,
        margin: Margin {
            left: 60.,
            right: 30.,
            top: 35.,
            bottom: 35.,
        },
        ..Default::default()
    }
}
fn options() -> CalendarLayoutOptions {
    CalendarLayoutOptions {
        start: Some((2024, 2, 27)),
        end: Some((2024, 3, 3)),
        first_weekday: CalendarWeekday::Monday,
        month_labels: false,
        weekday_ticks: vec![],
        day_border: None,
        month_border: None,
        square: false,
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
#[test]
fn compact_inclusive_dates_need_only_their_actual_weeks_and_keep_leap_day() {
    let s = calendar_layout(&config(), &[], &options()).unwrap();
    assert_eq!(s.days.len(), 6);
    assert_eq!(s.panels[0].weeks, 1);
    assert_eq!(s.days[0].weekday, 1);
    assert_eq!(s.days[5].weekday, 6);
    assert!(s.day((2024, 2, 29)).is_some());
    assert!(s.days.iter().all(|d| d.week == 0));
    let sunday = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            first_weekday: CalendarWeekday::Sunday,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(sunday.panels[0].weeks, 2);
    assert_eq!(sunday.days[5].weekday, 0);
    assert_eq!(sunday.days[5].week, 1);
    let single = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            start: Some((2024, 1, 3)),
            end: Some((2024, 1, 3)),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(single.days.len(), 1);
    assert_eq!(single.panels[0].weeks, 1);
    assert_eq!(single.days[0].weekday, 2);
}
#[test]
fn crossing_years_preserves_consecutive_weeks_and_exact_iso_identity() {
    let s = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            start: Some((2023, 12, 30)),
            end: Some((2024, 1, 2)),
            first_weekday: CalendarWeekday::Sunday,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(s.panels[0].weeks, 2);
    assert_eq!(
        s.days.iter().map(|d| d.id.as_str()).collect::<Vec<_>>(),
        ["2023-12-30", "2023-12-31", "2024-01-01", "2024-01-02"]
    );
    assert_eq!(
        s.days
            .iter()
            .map(|d| (d.week, d.weekday))
            .collect::<Vec<_>>(),
        [(0, 6), (1, 0), (1, 1), (1, 2)]
    );
    assert_eq!(s.months.len(), 2);
}
#[test]
fn weekdays_shift_names_and_ticks_for_all_seven_week_starts() {
    for first in [
        CalendarWeekday::Sunday,
        CalendarWeekday::Monday,
        CalendarWeekday::Tuesday,
        CalendarWeekday::Wednesday,
        CalendarWeekday::Thursday,
        CalendarWeekday::Friday,
        CalendarWeekday::Saturday,
    ] {
        let o = CalendarLayoutOptions {
            start: Some((2024, 1, 1)),
            end: Some((2024, 1, 7)),
            first_weekday: first,
            weekday_ticks: vec![0, 2, 6],
            ..options()
        };
        let s = calendar_layout(&config(), &[], &o).unwrap();
        assert_eq!(s.days[0].weekday, (8 - first.index()) % 7);
        assert_eq!(
            s.scene
                .labels
                .iter()
                .map(|l| l.text.clone())
                .collect::<Vec<_>>(),
            [0, 2, 6].map(|d| o.weekdays[(d + first.index()) % 7].clone())
        );
    }
}
#[test]
fn rectangular_and_square_cells_and_vertical_layout_are_real_geometry() {
    let c = config();
    let v = calendar_layout(
        &c,
        &[],
        &CalendarLayoutOptions {
            orientation: CalendarOrientation::Vertical,
            ..options()
        },
    )
    .unwrap();
    let canonical = ChartConfig {
        width: c.height,
        height: c.width,
        margin: Margin {
            left: c.margin.top,
            right: c.margin.bottom,
            top: c.margin.left,
            bottom: c.margin.right,
        },
        ..c.clone()
    };
    let h = calendar_layout(&canonical, &[], &options()).unwrap();
    for (a, b) in v.days.iter().zip(&h.days) {
        assert_eq!(a.id, b.id);
        close(a.rect.x, b.rect.y);
        close(a.rect.y, b.rect.x);
        close(a.rect.width, b.rect.height);
        close(a.rect.height, b.rect.width);
    }
    let square = calendar_layout(
        &c,
        &[],
        &CalendarLayoutOptions {
            square: true,
            ..options()
        },
    )
    .unwrap();
    for d in &square.days {
        close(d.rect.width, d.rect.height);
    }
    assert_ne!(h.days[0].rect.width, h.days[0].rect.height);
}
#[test]
fn automatic_range_is_actual_dates_and_null_is_distinct_from_zero_and_absence() {
    let data = [
        CalendarCell::new((2024, 2, 28), Some(0.)),
        CalendarCell::new((2024, 3, 1), None),
    ];
    let o = CalendarLayoutOptions {
        start: None,
        end: None,
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            unknown: Color(130, 140, 150, 255),
            colors: vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)],
            ..Default::default()
        },
        day_labels: CalendarDayLabel::Value,
        ..options()
    };
    let s = calendar_layout(&config(), &data, &o).unwrap();
    assert_eq!((s.start, s.end), ((2024, 2, 28), (2024, 3, 1)));
    assert_eq!(s.days.len(), 3);
    assert_eq!(s.days[0].value, Some(0.));
    assert!(s.days[0].provided);
    assert!(!s.days[1].provided);
    assert_eq!(s.days[1].value, None);
    assert!(s.days[2].provided);
    assert_eq!(s.days[2].value, None);
    assert_ne!(s.days[0].color, s.days[1].color);
    assert_eq!(s.days[1].color, s.days[2].color);
    assert_eq!(s.days[0].tooltip_datum().value, Some(0.));
    assert_eq!(s.days[1].tooltip_datum().value, None);
    let mut reversed = data.to_vec();
    reversed.reverse();
    let r = calendar_layout(&config(), &reversed, &o).unwrap();
    assert_eq!(s.scene, r.scene);
    assert_eq!(s.days, r.days);
}
#[test]
fn month_outline_is_a_true_staircase_not_its_bounding_rectangle() {
    let s = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            start: Some((2024, 2, 25)),
            end: Some((2024, 4, 6)),
            first_weekday: CalendarWeekday::Sunday,
            ..options()
        },
    )
    .unwrap();
    let march = s.months.iter().find(|m| m.month == 3).unwrap();
    let Geometry::Path(p) = &march.outline else {
        panic!()
    };
    assert!(p.len() > 5);
    let mut mask = config().scene().unwrap();
    mask.marks.push(Mark::new(
        "month",
        march.outline.clone(),
        Color(0, 0, 0, 255),
    ));
    for d in &s.days {
        let r = d.rect;
        let inside = mask
            .hit_test(r.x + r.width / 2., r.y + r.height / 2.)
            .is_some();
        assert_eq!(
            inside,
            d.month == 3,
            "month outline includes wrong civil date {}",
            d.id
        );
    }
    let partial = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            start: Some((2024, 3, 2)),
            end: Some((2024, 3, 3)),
            first_weekday: CalendarWeekday::Sunday,
            ..options()
        },
    )
    .unwrap();
    let Geometry::Path(p) = &partial.months[0].outline else {
        panic!()
    };
    assert_eq!(
        p.iter().filter(|c| matches!(c, C::Move(_))).count(),
        2,
        "two disconnected requested dates must not acquire a bounding-box bridge"
    );
}
#[test]
fn padding_moves_actual_month_edges_and_preserves_concave_shape() {
    let a = calendar_layout(&config(), &[], &options()).unwrap();
    let b = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            month_outline_padding: 1.,
            ..options()
        },
    )
    .unwrap();
    assert_eq!(a.days, b.days);
    for (a, b) in a.months.iter().zip(&b.months) {
        close(a.bounds.x - b.bounds.x, 1.);
        close(a.bounds.y - b.bounds.y, 1.);
        close(b.bounds.width - a.bounds.width, 2.);
        close(b.bounds.height - a.bounds.height, 2.);
        assert_ne!(a.outline, b.outline);
    }
    assert!(
        calendar_layout(
            &config(),
            &[],
            &CalendarLayoutOptions {
                month_outline_padding: 1000.,
                ..options()
            }
        )
        .is_err()
    );
}
#[test]
fn full_year_mode_expands_dates_and_keeps_54_week_leap_years_inside_grid() {
    let o = CalendarLayoutOptions {
        mode: CalendarLayoutMode::FullYears,
        start: Some((2000, 3, 1)),
        end: Some((2001, 4, 1)),
        first_weekday: CalendarWeekday::Sunday,
        year_gap: 10.,
        ..options()
    };
    let s = calendar_layout(&config(), &[], &o).unwrap();
    assert_eq!((s.start, s.end), ((2000, 1, 1), (2001, 12, 31)));
    assert_eq!(s.days.len(), 731);
    assert_eq!(s.panels.len(), 2);
    assert_eq!(s.panels[0].weeks, 54);
    assert_eq!(s.panels[1].weeks, 53);
    for d in &s.days {
        let panel = &s.panels[(d.year - 2000) as usize];
        assert!(
            d.rect.x >= panel.grid.x
                && d.rect.x + d.rect.width <= panel.grid.x + panel.grid.width + 1e-9
        );
        assert!(d.rect.y + d.rect.height <= panel.grid.y + panel.grid.height + 1e-9);
    }
    assert!(s.scene.labels.iter().any(|l| l.text == "2000"));
}
#[test]
fn hidden_year_data_and_owned_labels_do_not_remove_shared_month_weekday_labels() {
    let o = CalendarLayoutOptions {
        start: Some((2023, 12, 30)),
        end: Some((2024, 1, 3)),
        first_weekday: CalendarWeekday::Sunday,
        month_labels: true,
        weekday_ticks: vec![1, 3, 5],
        year_labels: Some(true),
        year_legend: true,
        day_labels: CalendarDayLabel::DayOfMonth,
        ..options()
    };
    let s = calendar_layout(&config(), &[], &o).unwrap();
    let shared = s
        .scene
        .labels
        .iter()
        .enumerate()
        .filter(|(i, _)| !s.scene.label_series.contains_key(i))
        .map(|(_, l)| l.clone())
        .collect::<Vec<_>>();
    let filtered = s.scene.without_series(&["year:2023".into()].into());
    assert!(!filtered.marks.iter().any(|m| m.series == "year:2023"));
    for l in &shared {
        assert!(filtered.labels.contains(l));
    }
    let h = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            hidden_years: [2023].into(),
            ..o
        },
    )
    .unwrap();
    assert!(h.days.iter().filter(|d| d.year == 2023).all(|d| !d.visible));
    for (a, b) in s.days.iter().zip(&h.days) {
        assert_eq!(a.geometry, b.geometry);
    }
}
#[test]
fn rounded_day_corners_gaps_and_month_strokes_are_not_data_hits() {
    let s = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            day_radius: 8.,
            day_spacing: 6.,
            month_border: Some(GraphicStroke {
                color: Color(0, 0, 0, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            ..options()
        },
    )
    .unwrap();
    let d = &s.days[1];
    let r = d.rect;
    assert_eq!(
        s.hit_test(r.x + r.width / 2., r.y + r.height / 2.)
            .unwrap()
            .id,
        d.id
    );
    assert!(s.hit_test(r.x + 0.1, r.y + 0.1).is_none());
    assert!(s.hit_test(r.x + r.width / 2., r.y - 3.).is_none());
}
#[test]
fn bordered_day_picking_retains_legacy_slop_but_fill_only_uses_rounded_geometry() {
    let o = CalendarLayoutOptions {
        day_radius: 2.,
        day_border: Some(GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        }),
        ..options()
    };
    let small = calendar_layout(&config(), &[], &o).unwrap();
    let r = small.days[0].rect;
    assert_eq!(
        small.hit_test(r.x + 0.1, r.y + 0.1).unwrap().id,
        small.days[0].id
    );
    let large = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            day_radius: 8.,
            ..o.clone()
        },
    )
    .unwrap();
    let r = large.days[0].rect;
    assert!(large.hit_test(r.x + 0.1, r.y + 0.1).is_none());
    let fill = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            day_border: None,
            ..o
        },
    )
    .unwrap();
    let r = fill.days[0].rect;
    assert!(fill.hit_test(r.x + 0.1, r.y + 0.1).is_none());
}
#[test]
fn invalid_dates_options_overflow_and_duplicate_sources_are_transactional() {
    let source = [CalendarCell::new((2024, 2, 29), Some(0.))];
    let before = source.clone();
    for date in [(2023, 2, 29), (2024, 0, 1), (0, 1, 1), (10000, 1, 1)] {
        assert!(
            calendar_layout(
                &config(),
                &[],
                &CalendarLayoutOptions {
                    start: Some(date),
                    end: Some(date),
                    ..options()
                }
            )
            .is_err()
        );
    }
    assert!(
        calendar_layout(
            &config(),
            &[source[0].clone(), source[0].clone()],
            &options()
        )
        .is_err()
    );
    assert!(
        calendar_layout(
            &config(),
            &[CalendarCell::new((2024, 2, 29), Some(f64::NAN))],
            &options()
        )
        .is_err()
    );
    assert!(
        calendar_layout(
            &config(),
            &source,
            &CalendarLayoutOptions {
                start: Some((2024, 3, 1)),
                end: Some((2024, 3, 3)),
                ..options()
            }
        )
        .is_err()
    );
    for v in [f64::NAN, f64::INFINITY, -1., 1000.] {
        assert!(
            calendar_layout(
                &config(),
                &source,
                &CalendarLayoutOptions {
                    day_spacing: v,
                    ..options()
                }
            )
            .is_err()
        );
    }
    assert!(
        calendar_layout(
            &config(),
            &[],
            &CalendarLayoutOptions {
                start: Some((2000, 1, 1)),
                end: Some((2032, 1, 1)),
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        calendar_layout(
            &config(),
            &[],
            &CalendarLayoutOptions {
                weekday_ticks: vec![1, 1],
                ..options()
            }
        )
        .is_err()
    );
    assert!(
        calendar_layout(
            &config(),
            &[],
            &CalendarLayoutOptions {
                start: None,
                end: None,
                ..options()
            }
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn zoned_layout_reuses_local_date_aggregation_and_rejects_sum_overflow() {
    let data = [
        CalendarInstant {
            timestamp_ms: -28_800_000,
            value: 2.,
        },
        CalendarInstant {
            timestamp_ms: 57_599_999,
            value: 3.,
        },
        CalendarInstant {
            timestamp_ms: 57_600_000,
            value: 99.,
        },
    ];
    let o = CalendarLayoutOptions {
        start: Some((1970, 1, 1)),
        end: Some((1970, 1, 1)),
        ..options()
    };
    let s = calendar_layout_zoned(
        &config(),
        &data,
        &TimeZone::fixed_offset(480).unwrap(),
        CalendarAggregation::Sum,
        &o,
    )
    .unwrap();
    assert_eq!(s.days.len(), 1);
    assert_eq!(s.days[0].value, Some(5.));
    let bad = [CalendarInstant {
        timestamp_ms: 0,
        value: f64::MAX,
    }; 2];
    assert!(
        calendar_layout_zoned(
            &config(),
            &bad,
            &TimeZone::Utc,
            CalendarAggregation::Sum,
            &o
        )
        .is_err()
    );
}
#[test]
fn padded_diagonal_month_contacts_are_unioned_without_false_internal_borders() {
    let o = CalendarLayoutOptions {
        start: Some((2024, 1, 1)),
        end: Some((2024, 1, 7)),
        first_weekday: CalendarWeekday::Tuesday,
        ..options()
    };
    let a = calendar_layout(&config(), &[], &o).unwrap();
    let b = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            month_outline_padding: 1.,
            ..o
        },
    )
    .unwrap();
    let count = |l: &CalendarLayout| {
        let Geometry::Path(p) = &l.months[0].outline else {
            panic!()
        };
        p.iter().filter(|c| matches!(c, C::Move(_))).count()
    };
    assert_eq!(count(&a), 2);
    assert_eq!(count(&b), 1);
    b.scene.validate().unwrap();
}
#[cfg(feature = "timezone")]
#[test]
fn named_zone_dst_jump_produces_civil_days_and_real_missing_dates() {
    let data = [
        CalendarInstant {
            timestamp_ms: 1_710_052_200_000,
            value: 2.,
        },
        CalendarInstant {
            timestamp_ms: 1_710_055_800_000,
            value: 3.,
        },
    ];
    let s = calendar_layout_zoned(
        &config(),
        &data,
        &TimeZone::iana("America/New_York").unwrap(),
        CalendarAggregation::Sum,
        &CalendarLayoutOptions {
            start: Some((2024, 3, 9)),
            end: Some((2024, 3, 11)),
            ..options()
        },
    )
    .unwrap();
    assert_eq!(s.days.len(), 3);
    assert_eq!(s.days[0].value, None);
    assert_eq!(s.days[1].value, Some(5.));
    assert_eq!(s.days[2].value, None);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_pixels_verify_missing_zero_rounding_border_and_nonrectangular_month_mask() {
    use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
    let fonts = FontRegistry::new();
    let o = CalendarLayoutOptions {
        start: Some((2024, 2, 27)),
        end: Some((2024, 3, 3)),
        day_spacing: 8.,
        day_radius: 7.,
        day_border: Some(GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 2.,
            cap: StrokeCap::Butt,
        }),
        colors: ColorScaleOptions {
            domain: Some([0., 10.]),
            colors: vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)],
            unknown: Color(120, 130, 140, 255),
            ..Default::default()
        },
        ..options()
    };
    let s = calendar_layout(
        &config(),
        &[
            CalendarCell::new((2024, 2, 28), Some(0.)),
            CalendarCell::new((2024, 3, 1), Some(10.)),
        ],
        &o,
    )
    .unwrap();
    let frame = render_raster(&s.scene, &fonts, &RasterOptions::default()).unwrap();
    let pixel = |x: f64, y: f64| {
        let i = (y.floor() as usize * frame.width as usize + x.floor() as usize) * 4;
        &frame.rgba[i..i + 4]
    };
    for (date, color) in [
        ((2024, 2, 28), [220, 20, 30, 255]),
        ((2024, 2, 29), [120, 130, 140, 255]),
        ((2024, 3, 1), [20, 40, 220, 255]),
    ] {
        let r = s.day(date).unwrap().rect;
        assert_eq!(pixel(r.x + r.width / 2., r.y + r.height / 2.), &color);
        assert_eq!(pixel(r.x + 0.1, r.y + 0.1), &[255, 255, 255, 255]);
        let p = pixel(r.x + r.width / 2., r.y);
        assert!(p[0] < 80 && p[1] < 80 && p[2] < 80);
    }
    let source = calendar_layout(
        &config(),
        &[],
        &CalendarLayoutOptions {
            start: Some((2024, 2, 25)),
            end: Some((2024, 4, 6)),
            first_weekday: CalendarWeekday::Sunday,
            ..options()
        },
    )
    .unwrap();
    let march = source.months.iter().find(|m| m.month == 3).unwrap();
    let mut mask = config().scene().unwrap();
    mask.marks.push(Mark::new(
        "month",
        march.outline.clone(),
        Color(0, 0, 0, 255),
    ));
    let raster = render_raster(&mask, &fonts, &RasterOptions::default()).unwrap();
    for d in &source.days {
        let r = d.rect;
        let i = ((r.y + r.height / 2.).floor() as usize * raster.width as usize
            + (r.x + r.width / 2.).floor() as usize)
            * 4;
        assert_eq!(
            &raster.rgba[i..i + 4],
            if d.month == 3 {
                &[0, 0, 0, 255]
            } else {
                &[255, 255, 255, 255]
            }
        );
    }
}
