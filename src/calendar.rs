//! Inclusive multi-year Gregorian calendar ranges using the single-year layout
//! as the date/weekday authority. Date identities survive year reordering and
//! orientation changes; one color domain applies to the complete range.
use std::collections::{BTreeMap, BTreeSet};

use crate::scene::ensure;
use crate::{CalendarDay, ChartConfig, Color, Geometry, Label, Result, Scene, TextAnchor};

/// One observation at an absolute instant. Multiple observations on the same
/// local civil date are combined by the selected aggregation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CalendarInstant {
    pub timestamp_ms: i64,
    pub value: f64,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarAggregation {
    #[default]
    Sum,
    Count,
    Mean,
    Min,
    Max,
}
pub const MAX_CALENDAR_INSTANTS: usize = 100_000;

/// Convert an instant to a local date, preserving historical offset seconds.
/// Calendar drawing admits years 1..=9999, narrower than formatter UTC ranges.
pub fn calendar_date_at(
    timestamp_ms: i64,
    value: f64,
    zone: &crate::TimeZone,
) -> Result<CalendarDate> {
    ensure(value.is_finite(), "calendar instant value must be finite")?;
    let offset = zone.offset_seconds_at(timestamp_ms)?;
    let (year, month, day) = crate::time::civil_from_days(
        (timestamp_ms + i64::from(offset) * 1000).div_euclid(86_400_000),
    );
    ensure(
        (1..=9999).contains(&year),
        "local calendar year must be in 1..=9999",
    )?;
    Ok(CalendarDate::new(
        year as i32,
        month as u8,
        day as u8,
        value,
    ))
}
/// The local year accompanies CalendarDay, whose original type has no year.
pub fn calendar_day_at(
    timestamp_ms: i64,
    value: f64,
    zone: &crate::TimeZone,
) -> Result<(i32, CalendarDay)> {
    let date = calendar_date_at(timestamp_ms, value, zone)?;
    Ok((
        date.year,
        CalendarDay {
            month: date.month,
            day: date.day,
            value,
        },
    ))
}
/// Aggregate observations inside an inclusive LOCAL date range. Observations
/// outside the range are validated, then omitted. Sorting by instant/value
/// makes aggregation deterministic under source permutation. Missing dates are
/// absent; render with calendar_range to show the complete requested interval.
/// Sum overflow rejects; Mean uses bounded weighted terms instead of raw sums.
pub fn aggregate_calendar_instants(
    data: &[CalendarInstant],
    zone: &crate::TimeZone,
    start: (i32, u8, u8),
    end: (i32, u8, u8),
    aggregation: CalendarAggregation,
) -> Result<Vec<CalendarDate>> {
    ensure(
        data.len() <= MAX_CALENDAR_INSTANTS,
        "calendar instant input budget exceeded",
    )?;
    zone.validate()?;
    let validate = |date: (i32, u8, u8)| -> Result<()> {
        ensure(
            (1..=9999).contains(&date.0)
                && (1..=12).contains(&date.1)
                && (1..=31).contains(&date.2),
            "invalid local calendar range date",
        )?;
        let days =
            crate::time::days_from_civil(i64::from(date.0), i64::from(date.1), i64::from(date.2));
        ensure(
            crate::time::civil_from_days(days)
                == (i64::from(date.0), i64::from(date.1), i64::from(date.2)),
            "invalid local calendar range date",
        )
    };
    validate(start)?;
    validate(end)?;
    ensure(
        start <= end && end.0 - start.0 < 32,
        "local calendar interval must ascend and span at most 32 years",
    )?;
    let mut ordered: Vec<_> = data.iter().collect();
    ordered.sort_by(|a, b| {
        a.timestamp_ms
            .cmp(&b.timestamp_ms)
            .then_with(|| a.value.total_cmp(&b.value))
    });
    let mut groups = BTreeMap::<(i32, u8, u8), (usize, f64)>::new();
    for datum in ordered {
        let date = calendar_date_at(datum.timestamp_ms, datum.value, zone)?;
        let key = (date.year, date.month, date.day);
        if key < start || key > end {
            continue;
        }
        let entry = groups.entry(key).or_insert((0, 0.));
        entry.1 = match aggregation {
            CalendarAggregation::Sum => entry.1 + date.value,
            CalendarAggregation::Count => (entry.0 + 1) as f64,
            CalendarAggregation::Mean => {
                let difference = date.value - entry.1;
                if difference.is_finite() {
                    entry.1 + difference / (entry.0 + 1) as f64
                } else {
                    let scale = entry.1.abs().max(date.value.abs());
                    let normalized = entry.1 / scale * (entry.0 as f64 / (entry.0 + 1) as f64)
                        + (date.value / scale) / (entry.0 + 1) as f64;
                    scale * normalized.clamp(-1., 1.)
                }
            }
            CalendarAggregation::Min => {
                if entry.0 == 0 {
                    date.value
                } else {
                    entry.1.min(date.value)
                }
            }
            CalendarAggregation::Max => {
                if entry.0 == 0 {
                    date.value
                } else {
                    entry.1.max(date.value)
                }
            }
        };
        ensure(entry.1.is_finite(), "local calendar aggregation overflow")?;
        entry.0 += 1;
    }
    Ok(groups
        .into_iter()
        .map(|((year, month, day), (_, value))| CalendarDate::new(year, month, day, value))
        .collect())
}
/// Render a local-date inclusive interval from absolute instant observations.
/// The caller's range options must supply start/end or years explicitly; they
/// are interpreted in the chosen zone, never as UTC midnight durations.
pub fn calendar_range_zoned(
    c: &ChartConfig,
    data: &[CalendarInstant],
    zone: &crate::TimeZone,
    aggregation: CalendarAggregation,
    o: &CalendarRangeOptions,
) -> Result<Scene> {
    let (start, end) = match (o.start, o.end, o.years) {
        (Some(start), Some(end), None) => (start, end),
        (None, None, Some((first, last))) => ((first, 1, 1), (last, 12, 31)),
        _ => {
            return Err(crate::ChartError(
                "zoned calendar requires an explicit local interval".into(),
            ));
        }
    };
    let dates = aggregate_calendar_instants(data, zone, start, end, aggregation)?;
    calendar_range(c, &dates, o)
}

#[derive(Clone, Debug, PartialEq)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
    pub value: f64,
}
impl CalendarDate {
    pub fn new(year: i32, month: u8, day: u8, value: f64) -> Self {
        Self {
            year,
            month,
            day,
            value,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CalendarOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CalendarRangeOptions {
    pub start: Option<(i32, u8, u8)>,
    pub end: Option<(i32, u8, u8)>,
    /// Inclusive first/last year; mutually exclusive with start/end dates.
    pub years: Option<(i32, i32)>,
    /// Horizontal weeks with vertically stacked years, or transposed calendars.
    pub orientation: CalendarOrientation,
    pub year_gap: f64,
    pub show_year_labels: bool,
}
impl Default for CalendarRangeOptions {
    fn default() -> Self {
        Self {
            start: None,
            end: None,
            years: None,
            orientation: CalendarOrientation::Horizontal,
            year_gap: 24.,
            show_year_labels: true,
        }
    }
}
fn validate_date(c: &ChartConfig, date: (i32, u8, u8)) -> Result<()> {
    crate::specialized::calendar(
        c,
        date.0,
        &[CalendarDay {
            month: date.1,
            day: date.2,
            value: 0.,
        }],
    )
    .map(|_| ())
}
fn date_id(date: (i32, u8, u8)) -> String {
    format!("{:04}-{:02}-{:02}", date.0, date.1, date.2)
}
fn color(value: f64, maximum: f64) -> Color {
    let t = (value / maximum).clamp(-1., 1.);
    if t >= 0. {
        Color(
            (230. - 170. * t) as u8,
            (245. - 100. * t) as u8,
            (235. - 150. * t) as u8,
            255,
        )
    } else {
        Color(230, (235. + 150. * t) as u8, (230. + 130. * t) as u8, 255)
    }
}
/// Render an inclusive date interval, retaining the usual full-year weekday
/// grid positions. Dates outside the requested interval are omitted; supplied
/// data outside that interval is rejected. At most 32 years are admitted.
/// With no explicit interval, infer complete first/last years from input data.
pub fn calendar_range(
    c: &ChartConfig,
    data: &[CalendarDate],
    o: &CalendarRangeOptions,
) -> Result<Scene> {
    let mut output = c.scene()?;
    ensure(data.len() <= 12_000, "calendar range exceeds data budget")?;
    ensure(
        o.year_gap.is_finite() && (0. ..=256.).contains(&o.year_gap),
        "calendar year gap must be in 0..=256",
    )?;
    let (start, end) = match (o.start, o.end, o.years) {
        (Some(start), Some(end), None) => (start, end),
        (None, None, Some((first, last))) => ((first, 1, 1), (last, 12, 31)),
        (None, None, None) => {
            let first = data.iter().map(|d| d.year).min().ok_or_else(|| {
                crate::ChartError("empty calendar input requires an explicit interval".into())
            })?;
            let last = data.iter().map(|d| d.year).max().unwrap_or(first);
            ((first, 1, 1), (last, 12, 31))
        }
        _ => {
            return Err(crate::ChartError(
                "provide either both calendar endpoints or a year interval".into(),
            ));
        }
    };
    validate_date(c, start)?;
    validate_date(c, end)?;
    ensure(
        start <= end && end.0 - start.0 < 32,
        "calendar interval must be ascending and span at most 32 years",
    )?;
    let mut grouped = BTreeMap::<i32, Vec<CalendarDay>>::new();
    let mut values = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut maximum = 1_f64;
    for d in data {
        let date = (d.year, d.month, d.day);
        ensure(
            date >= start && date <= end,
            "calendar datum lies outside interval",
        )?;
        ensure(d.value.is_finite(), "calendar values must be finite")?;
        ensure(seen.insert(date), "duplicate calendar date")?;
        grouped.entry(d.year).or_default().push(CalendarDay {
            month: d.month,
            day: d.day,
            value: d.value,
        });
        values.insert(date_id(date), d.value);
        maximum = maximum.max(d.value.abs());
    }
    let years = (end.0 - start.0 + 1) as usize;
    let horizontal = o.orientation == CalendarOrientation::Horizontal;
    let span = if horizontal {
        c.inner_height()
    } else {
        c.inner_width()
    };
    let slot = (span - o.year_gap * years.saturating_sub(1) as f64) / years as f64;
    let heading = if o.show_year_labels {
        c.theme.font_size * 1.7 + 4.
    } else {
        c.theme.font_size + 6.
    };
    ensure(
        slot > if horizontal { heading } else { 0. },
        "calendar years and gaps leave no cell area",
    )?;
    let min_id = date_id(start);
    let max_id = date_id(end);
    for (index, year) in (start.0..=end.0).enumerate() {
        let empty = vec![];
        let dates = grouped.get(&year).unwrap_or(&empty);
        let scene = crate::specialized::calendar(c, year, dates)?;
        let (left, top, width, height) = if horizontal {
            (
                c.margin.left,
                c.margin.top + index as f64 * (slot + o.year_gap) + heading,
                c.inner_width(),
                slot - heading,
            )
        } else {
            ensure(
                c.inner_height() > heading,
                "calendar headings leave no vertical space",
            )?;
            (
                c.margin.left + index as f64 * (slot + o.year_gap),
                c.margin.top + heading,
                slot,
                c.inner_height() - heading,
            )
        };
        for mut mark in scene.marks {
            if mark.id < min_id || mark.id > max_id {
                continue;
            }
            if let Geometry::Rect {
                x,
                y,
                width: mw,
                height: mh,
            } = mark.geometry
            {
                let nx = (x - c.margin.left) / c.inner_width();
                let ny = (y - c.margin.top) / c.inner_height();
                mark.geometry = if horizontal {
                    Geometry::Rect {
                        x: left + nx * width,
                        y: top + ny * height,
                        width: mw / c.inner_width() * width,
                        height: mh / c.inner_height() * height,
                    }
                } else {
                    Geometry::Rect {
                        x: left + ny * width,
                        y: top + nx * height,
                        width: mh / c.inner_height() * width,
                        height: mw / c.inner_width() * height,
                    }
                };
            }
            if let Some(v) = values.get(&mark.id) {
                mark.fill = Some(color(*v, maximum));
            }
            mark.series = format!("year:{year}");
            output.marks.push(mark);
        }
        for mut label in scene.labels {
            let nx = (label.x - c.margin.left) / c.inner_width();
            let ny = (label.y - c.margin.top) / c.inner_height();
            if horizontal {
                label.x = if label.anchor == TextAnchor::End {
                    left - 6.
                } else {
                    left + nx * width
                };
                label.y = if label.y < c.margin.top {
                    top - 4.
                } else {
                    top + ny * height
                };
            } else {
                let month = label.y < c.margin.top;
                label.x = if month { left - 4. } else { left + ny * width };
                label.y = if month { top + nx * height } else { top - 6. };
                label.anchor = if month {
                    TextAnchor::End
                } else {
                    TextAnchor::Middle
                };
            }
            output.labels.push(label);
        }
        if o.show_year_labels {
            output.labels.push(Label {
                rotation: 0.,
                x: if horizontal {
                    left - 8.
                } else {
                    left + width / 2.
                },
                y: if horizontal {
                    top - 4.
                } else {
                    c.margin.top + c.theme.font_size
                },
                text: year.to_string(),
                color: c.theme.text,
                size: c.theme.font_size,
                anchor: if horizontal {
                    TextAnchor::End
                } else {
                    TextAnchor::Middle
                },
            });
        }
    }
    output.validate()?;
    Ok(output)
}
