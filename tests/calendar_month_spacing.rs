#[cfg(feature = "geojson")]
use glory::render::PathCommand;
#[cfg(feature = "raster")]
use kartis::render::raster::{FontRegistry, RasterOptions, render_raster};
use kartis::*;
#[cfg(feature = "geojson")]
use serde_json::Value;

fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn options(first: i32, last: i32, spacing: f64) -> CalendarLayoutOptions {
    CalendarLayoutOptions {
        mode: CalendarLayoutMode::FullYears,
        start: Some((first, 1, 1)),
        end: Some((last, 12, 31)),
        weekday_space: 0.,
        weekday_ticks: vec![],
        day_spacing: 0.,
        month_labels: false,
        year_labels: Some(false),
        day_border: None,
        month_border: None,
        year_gap: 14.,
        month_spacing: spacing,
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[cfg(feature = "geojson")]
fn n(v: &Value) -> f64 {
    v.as_f64().unwrap()
}
#[cfg(feature = "geojson")]
fn points(g: &Geometry) -> Vec<[f64; 2]> {
    let Geometry::Path(commands) = g else {
        panic!("month outline must be its actual path")
    };
    commands
        .iter()
        .filter_map(|c| match c {
            PathCommand::Move(p) | PathCommand::Line(p) => Some([p.x, p.y]),
            PathCommand::Close => None,
            _ => panic!("calendar boundary is orthogonal"),
        })
        .collect()
}
#[cfg(feature = "geojson")]
fn boundary_distance(p: [f64; 2], ring: &[[f64; 2]]) -> f64 {
    ring.iter()
        .zip(ring.iter().cycle().skip(1))
        .take(ring.len())
        .map(|(a, b)| {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let d = dx * dx + dy * dy;
            let t = if d == 0. {
                0.
            } else {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / d).clamp(0., 1.)
            };
            (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
        })
        .fold(f64::INFINITY, f64::min)
}

#[cfg(feature = "geojson")]
#[test]
fn original_complete_calendar_functions_match_5120_days_and_168_months() {
    let oracle: Value =
        serde_json::from_str(include_str!("fixtures/calendar-month-spacing/values.json")).unwrap();
    let mut days = 0;
    let mut months = 0;
    for case in oracle["cases"].as_array().unwrap() {
        let first = case["first"].as_i64().unwrap() as i32;
        let last = case["last"].as_i64().unwrap() as i32;
        let c = config(n(&case["width"]), n(&case["height"]));
        let mut o = options(first, last, n(&case["monthSpacing"]));
        if case["direction"] == "vertical" {
            o.orientation = CalendarOrientation::Vertical;
        }
        let layout = calendar_layout(&c, &[], &o).unwrap();
        // Nivo reserves one extra empty week when Jan1 is Sunday. Kartis's
        // pre-existing default uses occupied columns. With this independently
        // authored height-limited10px cell size, the sole difference is the
        // exactly known half-empty-week centering translation, not a fit from
        // candidate geometry. Preserve the native raw coordinates in fixtures.
        let shift = n(&case["occupiedWeekOriginShift"]);
        let (dx, dy) = if case["direction"] == "vertical" {
            (0., shift)
        } else {
            (shift, 0.)
        };
        assert_eq!(
            layout.panels.iter().map(|p| p.weeks).max().unwrap() as f64,
            n(&case["occupiedWeekColumns"])
        );
        for d in case["days"].as_array().unwrap() {
            let date = d["date"].as_array().unwrap();
            let p = layout
                .day((
                    date[0].as_i64().unwrap() as i32,
                    date[1].as_u64().unwrap() as u8,
                    date[2].as_u64().unwrap() as u8,
                ))
                .unwrap();
            assert_eq!(p.id, d["day"].as_str().unwrap());
            close(p.rect.x, n(&d["x"]) + dx);
            close(p.rect.y, n(&d["y"]) + dy);
            close(p.rect.width, n(&d["size"]));
            close(p.rect.height, n(&d["size"]));
            days += 1;
        }
        for m in case["months"].as_array().unwrap() {
            let p = layout
                .months
                .iter()
                .find(|p| {
                    p.year == m["year"].as_i64().unwrap() as i32
                        && p.month == m["month"].as_u64().unwrap() as u8
                })
                .unwrap();
            close(p.bounds.x, n(&m["bbox"]["x"]) + dx);
            close(p.bounds.y, n(&m["bbox"]["y"]) + dy);
            close(p.bounds.width, n(&m["bbox"]["width"]));
            close(p.bounds.height, n(&m["bbox"]["height"]));
            let actual = points(&p.outline);
            let expected: Vec<_> = m["vertices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| [n(&v[0]) + dx, n(&v[1]) + dy])
                .collect();
            // Cyclic start and collinear vertex reduction are not a native
            // contract. Both actual boundary vertex sets must lie on the
            // other's real segments; no bounding-box substitute is used.
            for q in &actual {
                close(boundary_distance(*q, &expected), 0.);
            }
            for q in &expected {
                close(boundary_distance(*q, &actual), 0.);
            }
            months += 1;
        }
    }
    assert_eq!((days, months), (5120, 168));
}

#[test]
fn same_week_different_month_cells_separate_in_both_orientations() {
    for orientation in [
        CalendarOrientation::Horizontal,
        CalendarOrientation::Vertical,
    ] {
        let c = if orientation == CalendarOrientation::Horizontal {
            config(1000., 70.)
        } else {
            config(70., 1000.)
        };
        let l = calendar_layout(
            &c,
            &[],
            &CalendarLayoutOptions {
                orientation,
                ..options(2024, 2024, 8.)
            },
        )
        .unwrap();
        let a = l.day((2024, 1, 31)).unwrap();
        let b = l.day((2024, 2, 1)).unwrap();
        assert_eq!(a.week, b.week);
        close(
            if orientation == CalendarOrientation::Horizontal {
                b.rect.x - a.rect.x
            } else {
                b.rect.y - a.rect.y
            },
            8.,
        );
        assert_ne!(a.geometry, b.geometry);
    }
}

#[test]
fn source_holes_zero_null_leap_and_pick_keep_identity_after_translation() {
    let c = config(1000., 70.);
    let o = options(2024, 2024, 8.);
    let d = [
        CalendarCell::new((2024, 1, 31), Some(0.)),
        CalendarCell::new((2024, 2, 1), Some(7.)),
        CalendarCell::new((2024, 2, 29), None),
    ];
    let l = calendar_layout(&c, &d, &o).unwrap();
    assert_eq!(l.days.len(), 366);
    for (date, value, provided) in [
        ((2024, 1, 31), Some(0.), true),
        ((2024, 2, 1), Some(7.), true),
        ((2024, 2, 29), None, true),
        ((2024, 3, 1), None, false),
    ] {
        let day = l.day(date).unwrap();
        assert_eq!(day.value, value);
        assert_eq!(day.provided, provided);
        let r = day.rect;
        let hit = l.hit_test(r.x + r.width / 2., r.y + r.height / 2.).unwrap();
        assert_eq!(hit.id, day.id);
        let mark = l.scene.marks.iter().find(|m| m.id == day.id).unwrap();
        assert_eq!(mark.geometry, day.geometry);
    }
}

#[test]
fn all_week_origins_shift_by_real_month_while_hidden_years_keep_positions() {
    for first in [
        CalendarWeekday::Sunday,
        CalendarWeekday::Monday,
        CalendarWeekday::Tuesday,
        CalendarWeekday::Wednesday,
        CalendarWeekday::Thursday,
        CalendarWeekday::Friday,
        CalendarWeekday::Saturday,
    ] {
        let c = config(1000., 154.);
        let o = CalendarLayoutOptions {
            first_weekday: first,
            ..options(2023, 2024, 8.)
        };
        let l = calendar_layout(&c, &[], &o).unwrap();
        let hidden = calendar_layout(
            &c,
            &[],
            &CalendarLayoutOptions {
                hidden_years: [2023].into(),
                ..o
            },
        )
        .unwrap();
        for day in &l.days {
            let h = hidden.day(day.date).unwrap();
            assert_eq!(day.geometry, h.geometry);
            assert_eq!(day.rect, h.rect);
            assert_eq!(h.visible, day.year != 2023);
        }
        assert!(hidden.scene.marks.iter().all(|m| m.series != "year:2023"));
        assert_eq!(l.panels.len(), 2);
        assert!(l.day((2024, 2, 29)).is_some());
    }
}

#[test]
fn day_month_weekday_and_year_labels_keep_their_authored_anchor_policies() {
    let c = config(1000., 110.);
    let o = CalendarLayoutOptions {
        month_labels: true,
        month_style: CalendarTextStyle {
            size: Some(12.),
            ..Default::default()
        },
        month_label_offset: 8.,
        year_labels: Some(true),
        year_style: CalendarTextStyle {
            size: Some(10.),
            ..Default::default()
        },
        day_labels: CalendarDayLabel::DayOfMonth,
        weekday_ticks: vec![0],
        weekday_style: CalendarTextStyle {
            size: Some(12.),
            ..Default::default()
        },
        ..options(2024, 2024, 8.)
    };
    let l = calendar_layout(&c, &[], &o).unwrap();
    let d = l.day((2024, 2, 1)).unwrap();
    let day_label = l
        .scene
        .labels
        .iter()
        .find(|s| s.text == "1" && (s.x - (d.rect.x + d.rect.width / 2.)).abs() < 1e-9)
        .unwrap();
    close(day_label.x, d.rect.x + d.rect.width / 2.);
    let feb = l.months.iter().find(|m| m.month == 2).unwrap();
    let label = l.scene.labels.iter().find(|s| s.text == "Feb").unwrap();
    close(label.x, feb.bounds.x + feb.bounds.width / 2.);
    close(label.y, l.panels[0].grid.y - 8. + 4.);
    let weekday = l.scene.labels.iter().find(|s| s.text == "Sun").unwrap();
    close(weekday.x, l.panels[0].grid.x - o.weekday_offset);
    let year = l.scene.labels.iter().find(|s| s.text == "2024").unwrap();
    close(year.x, 0.);
    close(year.y, 5. + 10. / 3.);
}

#[test]
fn compact_positive_spacing_and_invalid_or_saturating_dimensions_reject() {
    let c = config(1000., 70.);
    for spacing in [f64::NAN, f64::INFINITY, -1., 1025.] {
        assert!(calendar_layout(&c, &[], &options(2024, 2024, spacing)).is_err());
    }
    assert!(
        calendar_layout(
            &c,
            &[],
            &CalendarLayoutOptions {
                mode: CalendarLayoutMode::Compact,
                ..options(2024, 2024, 1.)
            }
        )
        .is_err()
    );
    assert!(calendar_layout(&c, &[], &options(2024, 2024, 100.)).is_err());
    assert!(calendar_layout(&c, &[], &options(2024, 2024, -0.)).is_ok());
    assert!(
        calendar_layout(
            &c,
            &[],
            &CalendarLayoutOptions {
                mode: CalendarLayoutMode::Compact,
                start: Some((2024, 1, 1)),
                end: Some((2024, 1, 7)),
                ..options(2024, 2024, 0.)
            }
        )
        .is_ok()
    );
}

#[test]
fn outline_padding_is_independent_and_does_not_move_day_cells() {
    let c = config(1000., 70.);
    let o = options(2024, 2024, 8.);
    let a = calendar_layout(&c, &[], &o).unwrap();
    let b = calendar_layout(
        &c,
        &[],
        &CalendarLayoutOptions {
            month_outline_padding: 1.,
            ..o
        },
    )
    .unwrap();
    assert_eq!(a.days, b.days);
    assert_ne!(a.months[1].outline, b.months[1].outline);
    close(b.months[1].bounds.x, a.months[1].bounds.x - 1.);
}

#[cfg(feature = "raster")]
#[test]
fn actual_raster_uses_shifted_days_and_outlines_without_filling_month_gaps() {
    let c = config(1000., 70.);
    let mut o = options(2024, 2024, 8.);
    o.colors.domain = Some([0., 7.]);
    o.colors.colors = vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)];
    o.colors.unknown = Color(255, 255, 255, 255);
    let l = calendar_layout(
        &c,
        &[
            CalendarCell::new((2024, 1, 31), Some(0.)),
            CalendarCell::new((2024, 2, 1), Some(7.)),
        ],
        &o,
    )
    .unwrap();
    let image = render_raster(&l.scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    for (date, color) in [
        ((2024, 1, 31), [220, 20, 30, 255]),
        ((2024, 2, 1), [20, 40, 220, 255]),
    ] {
        let r = l.day(date).unwrap().rect;
        let i = ((r.y + r.height / 2.).floor() as usize * image.width as usize
            + (r.x + r.width / 2.).floor() as usize)
            * 4;
        assert_eq!(&image.rgba[i..i + 4], &color);
    }
    assert!(render::to_svg(&l.scene).unwrap().contains("2024-02-01"));
    assert!(
        !render::to_canvas_script(&l.scene, "calendar")
            .unwrap()
            .is_empty()
    );
}
