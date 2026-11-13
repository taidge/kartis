//! Local-calendar tick boundaries mapped back to UTC instants. Geometry remains
//! linear in epoch milliseconds. Calendar days are never fixed 24-hour durations.
//! References: E:/Repos/d3/docs/d3-time.md; Jiff's AmbiguousOffset and TZif
//! transition APIs, <https://docs.rs/jiff/latest/jiff/tz/index.html>.
use std::collections::BTreeSet;

use crate::datetime::{DateTimeFormat, TimeLocale, TimeZone};
use crate::scene::ensure;
use crate::time::{MAX_TIMESTAMP_MS, TimeTick, UtcTimeScale, civil_from_days, days_from_civil};
use crate::{ChartError, Result};

const DAY: i64 = 86_400_000;
pub const MAX_ZONED_TICKS: usize = 2000;
pub const MAX_CALENDAR_WORK: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Millisecond,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Year,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(i64)]
pub enum WeekStart {
    Sunday,
    #[default]
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeInterval {
    pub unit: TimeUnit,
    pub step: u32,
    pub week_start: WeekStart,
}
impl TimeInterval {
    pub fn new(unit: TimeUnit, step: u32) -> Result<Self> {
        let interval = Self {
            unit,
            step,
            week_start: WeekStart::Monday,
        };
        interval.validate()?;
        Ok(interval)
    }
    pub fn validate(self) -> Result<()> {
        ensure(
            (1..=1_000_000).contains(&self.step),
            "calendar interval step must be in 1..=1000000",
        )
    }
    fn duration(self) -> Option<i64> {
        Some(
            match self.unit {
                TimeUnit::Millisecond => 1,
                TimeUnit::Second => 1000,
                TimeUnit::Minute => 60_000,
                TimeUnit::Hour => 3_600_000,
                TimeUnit::Day => DAY,
                TimeUnit::Week => 7 * DAY,
                TimeUnit::Month | TimeUnit::Year => return None,
            } * i64::from(self.step),
        )
    }
    /// Greatest admitted interval boundary at/before an instant. With Fold::Both
    /// the later copy is used when it has already occurred. Returns None when
    /// no corresponding boundary is representable or a gap is explicitly skipped.
    pub fn floor(self, timestamp_ms: i64, calendar: &TimeCalendar) -> Result<Option<i64>> {
        self.validate()?;
        calendar.validate()?;
        let local = local_ms(timestamp_ms, &calendar.zone)?;
        Ok(resolve_boundary(self.floor_local(local), self, calendar)?
            .into_iter()
            .filter(|v| *v <= timestamp_ms)
            .max())
    }
    /// Move by civil units, preserving wall-clock fields. Month/year offsets
    /// clamp the day to the target month's last day. A fold may return two
    /// instants; a skipped gap returns none. No elapsed-duration approximation.
    pub fn offset(
        self,
        timestamp_ms: i64,
        steps: i32,
        calendar: &TimeCalendar,
    ) -> Result<Vec<i64>> {
        self.validate()?;
        calendar.validate()?;
        ensure(
            steps.unsigned_abs() <= 10_000,
            "calendar offset step budget exceeded",
        )?;
        resolve_boundary(
            self.offset_local(local_ms(timestamp_ms, &calendar.zone)?, i64::from(steps))?,
            self,
            calendar,
        )
    }
    fn floor_local(self, local: i64) -> i64 {
        let step = i64::from(self.step);
        match self.unit {
            TimeUnit::Month | TimeUnit::Year => {
                let (year, month, _) = civil_from_days(local.div_euclid(DAY));
                let (year, month) = if self.unit == TimeUnit::Year {
                    (year.div_euclid(step) * step, 1)
                } else {
                    (year, (month - 1).div_euclid(step) * step + 1)
                };
                days_from_civil(year, month, 1) * DAY
            }
            TimeUnit::Week => {
                let origin = -(4 - self.week_start as i64).rem_euclid(7) * DAY;
                let duration = self.duration().unwrap();
                (local - origin).div_euclid(duration) * duration + origin
            }
            TimeUnit::Day => {
                let (year, month, day) = civil_from_days(local.div_euclid(DAY));
                days_from_civil(year, month, (day - 1).div_euclid(step) * step + 1) * DAY
            }
            TimeUnit::Millisecond => local.div_euclid(step) * step,
            _ => {
                let (parent, unit) = match self.unit {
                    TimeUnit::Hour => (DAY, 3_600_000),
                    TimeUnit::Minute => (3_600_000, 60_000),
                    TimeUnit::Second => (60_000, 1000),
                    _ => unreachable!(),
                };
                local.div_euclid(parent) * parent
                    + local.rem_euclid(parent).div_euclid(unit * step) * (unit * step)
            }
        }
    }
    fn next_local(self, local: i64) -> Result<i64> {
        let next = self.offset_local(local, 1)?;
        let parent = match self.unit {
            TimeUnit::Second => Some(60_000),
            TimeUnit::Minute => Some(3_600_000),
            TimeUnit::Hour => Some(DAY),
            _ => None,
        };
        if let Some(parent) = parent {
            return Ok(next.min((local.div_euclid(parent) + 1) * parent));
        }
        if matches!(self.unit, TimeUnit::Day | TimeUnit::Month) {
            let (year, month, _) = civil_from_days(local.div_euclid(DAY));
            let boundary = if self.unit == TimeUnit::Day {
                days_from_civil(year + i64::from(month == 12), month % 12 + 1, 1) * DAY
            } else {
                days_from_civil(year + 1, 1, 1) * DAY
            };
            return Ok(next.min(boundary));
        }
        Ok(next)
    }
    fn offset_local(self, local: i64, steps: i64) -> Result<i64> {
        if let Some(duration) = self.duration() {
            return duration
                .checked_mul(steps)
                .and_then(|delta| local.checked_add(delta))
                .ok_or_else(|| ChartError("calendar offset overflow".into()));
        }
        let (year, month, day) = civil_from_days(local.div_euclid(DAY));
        let amount = i64::from(self.step) * steps;
        let serial = year * 12 + month - 1
            + if self.unit == TimeUnit::Year {
                amount * 12
            } else {
                amount
            };
        let year = serial.div_euclid(12);
        let month = serial.rem_euclid(12) + 1;
        let last_day = days_from_civil(year + (month == 12) as i64, month % 12 + 1, 1)
            - days_from_civil(year, month, 1);
        days_from_civil(year, month, day.min(last_day))
            .checked_mul(DAY)
            .and_then(|value| value.checked_add(local.rem_euclid(DAY)))
            .ok_or_else(|| ChartError("calendar offset overflow".into()))
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GapPolicy {
    Skip,
    /// First valid instant following the missing local boundary. Multiple local
    /// boundaries in one gap collapse to this one actual transition instant.
    #[default]
    NextValid,
    Reject,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FoldPolicy {
    /// Hour and smaller intervals include both copies; day/week/month/year
    /// choose the earlier copy, so a local date has a single start boundary.
    #[default]
    Calendar,
    /// Explicitly include both copies, including repeated midnight boundaries.
    Both,
    Earlier,
    Later,
    Reject,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TimeCalendar {
    pub zone: TimeZone,
    /// None selects an interval from the domain and requested tick count.
    pub interval: Option<TimeInterval>,
    pub gap: GapPolicy,
    pub fold: FoldPolicy,
}
impl TimeCalendar {
    pub fn new(zone: TimeZone) -> Self {
        Self {
            zone,
            ..Self::default()
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.zone.validate()?;
        if let Some(interval) = self.interval {
            interval.validate()?;
        }
        Ok(())
    }
}
fn local_ms(instant: i64, zone: &TimeZone) -> Result<i64> {
    Ok(instant + i64::from(zone.offset_seconds_at(instant)?) * 1000)
}
fn admitted(instant: i64) -> bool {
    (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&instant)
}
fn resolve_boundary(
    local: i64,
    interval: TimeInterval,
    calendar: &TimeCalendar,
) -> Result<Vec<i64>> {
    if calendar.fold == FoldPolicy::Calendar {
        let mut options = calendar.clone();
        options.fold = if matches!(
            interval.unit,
            TimeUnit::Day | TimeUnit::Week | TimeUnit::Month | TimeUnit::Year
        ) {
            FoldPolicy::Earlier
        } else {
            FoldPolicy::Both
        };
        return resolve_local(local, &options);
    }
    resolve_local(local, calendar)
}
#[cfg(feature = "timezone")]
fn timestamp(instant: i64) -> Result<jiff::Timestamp> {
    jiff::Timestamp::new(instant / 1000, (instant % 1000) as i32 * 1_000_000)
        .map_err(|_| ChartError("calendar instant outside Jiff Timestamp range".into()))
}
pub(crate) fn resolve_local(local: i64, calendar: &TimeCalendar) -> Result<Vec<i64>> {
    match &calendar.zone {
        TimeZone::Utc => Ok(admitted(local).then_some(local).into_iter().collect()),
        TimeZone::FixedOffset { minutes } => {
            let instant = local
                .checked_sub(i64::from(*minutes) * 60_000)
                .ok_or_else(|| ChartError("calendar local offset overflow".into()))?;
            Ok(admitted(instant).then_some(instant).into_iter().collect())
        }
        #[cfg(feature = "timezone")]
        TimeZone::Named(zone) => {
            use jiff::tz::AmbiguousOffset;
            let (year, month, day) = civil_from_days(local.div_euclid(DAY));
            if !(-9999..=9999).contains(&year) {
                return Ok(vec![]);
            }
            let clock = local.rem_euclid(DAY);
            let civil = jiff::civil::DateTime::new(
                year as i16,
                month as i8,
                day as i8,
                (clock / 3_600_000) as i8,
                (clock / 60_000 % 60) as i8,
                (clock / 1000 % 60) as i8,
                (clock % 1000) as i32 * 1_000_000,
            )
            .map_err(|error| ChartError(format!("invalid calendar civil boundary: {error}")))?;
            let ambiguous = zone.to_ambiguous_timestamp(civil);
            // A floored candidate can precede Timestamp::MIN even though both
            // domain endpoints are valid. Omit unrepresentable candidates,
            // matching the UTC/fixed-offset boundary behavior.
            let map = |offset: jiff::tz::Offset| {
                let instant = local - i64::from(offset.seconds()) * 1000;
                timestamp(instant).is_ok().then_some(instant)
            };
            match ambiguous.offset() {
                AmbiguousOffset::Unambiguous { offset } => Ok(map(offset).into_iter().collect()),
                AmbiguousOffset::Fold { before, after } => match calendar.fold {
                    FoldPolicy::Earlier | FoldPolicy::Calendar => {
                        Ok(map(before).into_iter().collect())
                    }
                    FoldPolicy::Later => Ok(map(after).into_iter().collect()),
                    FoldPolicy::Both => {
                        Ok([map(before), map(after)].into_iter().flatten().collect())
                    }
                    FoldPolicy::Reject => {
                        Err(ChartError("ambiguous folded calendar boundary".into()))
                    }
                },
                AmbiguousOffset::Gap { .. } => match calendar.gap {
                    GapPolicy::Skip => Ok(vec![]),
                    GapPolicy::Reject => Err(ChartError("nonexistent calendar boundary".into())),
                    GapPolicy::NextValid => {
                        let earlier = ambiguous
                            .earlier()
                            .map_err(|error| ChartError(error.to_string()))?;
                        let later = ambiguous
                            .later()
                            .map_err(|error| ChartError(error.to_string()))?;
                        let transition = zone
                            .following(earlier)
                            .next()
                            .ok_or_else(|| ChartError("missing calendar gap transition".into()))?
                            .timestamp();
                        ensure(
                            transition <= later,
                            "calendar gap transition lies outside ambiguity",
                        )?;
                        Ok(vec![transition.as_millisecond()])
                    }
                },
            }
        }
    }
}

/// Epoch-linear geometry with genuine local calendar ticks. Like D3 every,
/// seconds/minutes/hours align their local clock field, days align day-1 within
/// each month, months align month-1 within each year. Week steps anchor to the
/// chosen weekday preceding the local epoch, years to year zero.
#[derive(Clone, Debug, PartialEq)]
pub struct ZonedTimeScale {
    scale: UtcTimeScale,
    calendar: TimeCalendar,
}
impl ZonedTimeScale {
    pub fn new(domain: [i64; 2], range: [f64; 2], calendar: TimeCalendar) -> Result<Self> {
        calendar.validate()?;
        for instant in domain {
            calendar.zone.offset_seconds_at(instant)?;
        }
        Ok(Self {
            scale: UtcTimeScale::new(domain, range)?,
            calendar,
        })
    }
    pub fn domain(&self) -> [i64; 2] {
        self.scale.domain()
    }
    pub fn range(&self) -> [f64; 2] {
        self.scale.range()
    }
    pub fn calendar(&self) -> &TimeCalendar {
        &self.calendar
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.scale = self.scale.clamped(clamp);
        self
    }
    pub fn map(&self, instant: i64) -> f64 {
        self.scale.map(instant)
    }
    pub fn invert(&self, pixel: f64) -> Result<i64> {
        self.scale.invert(pixel)
    }
    pub fn ticks(&self, count: usize) -> Result<Vec<TimeTick>> {
        ensure(count <= 1000, "zoned tick request count exceeds 1000")?;
        if count == 0 {
            return Ok(vec![]);
        }
        let interval = self
            .calendar
            .interval
            .unwrap_or_else(|| choose_interval(self.domain(), count));
        self.ticks_every(interval)
    }
    pub fn ticks_every(&self, interval: TimeInterval) -> Result<Vec<TimeTick>> {
        let format = DateTimeFormat::new(
            "%Y-%m-%d %H:%M:%S.%L %Z",
            &TimeLocale::default(),
            self.calendar.zone.clone(),
        )?;
        self.timestamps_every(interval)?
            .into_iter()
            .map(|timestamp_ms| {
                Ok(TimeTick {
                    timestamp_ms,
                    label: format.format(timestamp_ms)?,
                })
            })
            .collect()
    }
    pub fn timestamps_every(&self, interval: TimeInterval) -> Result<Vec<i64>> {
        interval.validate()?;
        let [a, b] = self.domain();
        let (lo, hi) = (a.min(b), a.max(b));
        let wall_a = local_ms(lo, &self.calendar.zone)?;
        let wall_b = local_ms(hi, &self.calendar.zone)?;
        let (wall_lo, wall_hi) = (wall_a.min(wall_b), wall_a.max(wall_b));
        let mut work = 0usize;
        // Include local extrema at each transition. Endpoints alone miss e.g.
        // 01:00 EST when a range begins 01:45 EDT and ends 01:15 EST.
        #[cfg(feature = "timezone")]
        let (wall_lo, wall_hi) = {
            let (mut wall_lo, mut wall_hi) = (wall_lo, wall_hi);
            if let TimeZone::Named(zone) = &self.calendar.zone {
                let mut offset = zone.to_offset(timestamp(lo)?).seconds();
                for transition in zone.following(timestamp(lo)?) {
                    let instant = transition.timestamp().as_millisecond();
                    if instant > hi {
                        break;
                    }
                    work += 1;
                    ensure(
                        work <= MAX_CALENDAR_WORK,
                        "calendar transition work budget exceeded",
                    )?;
                    let old = instant - 1 + i64::from(offset) * 1000;
                    offset = zone.to_offset(transition.timestamp()).seconds();
                    let new = instant + i64::from(offset) * 1000;
                    wall_lo = wall_lo.min(old).min(new);
                    wall_hi = wall_hi.max(old).max(new);
                }
            }
            (wall_lo, wall_hi)
        };
        let mut local = interval.floor_local(wall_lo);
        let mut instants = BTreeSet::new();
        while local <= wall_hi {
            work += 1;
            ensure(
                work <= MAX_CALENDAR_WORK,
                "calendar boundary work budget exceeded",
            )?;
            for instant in resolve_boundary(local, interval, &self.calendar)? {
                if (lo..=hi).contains(&instant) {
                    instants.insert(instant);
                }
            }
            ensure(
                instants.len() <= MAX_ZONED_TICKS,
                "calendar tick output budget exceeded",
            )?;
            let next = interval.next_local(local)?;
            ensure(next > local, "calendar interval did not advance")?;
            local = next;
        }
        let mut result: Vec<_> = instants.into_iter().collect();
        if a > b {
            result.reverse();
        }
        Ok(result)
    }
}
fn choose_interval(domain: [i64; 2], count: usize) -> TimeInterval {
    let target = ((domain[1] as f64 - domain[0] as f64).abs() / count as f64).max(1.);
    let mut intervals = vec![];
    for (unit, steps, millis) in [
        (
            TimeUnit::Millisecond,
            &[1, 5, 10, 50, 100, 250, 500][..],
            1.,
        ),
        (TimeUnit::Second, &[1, 5, 15, 30][..], 1000.),
        (TimeUnit::Minute, &[1, 5, 15, 30][..], 60_000.),
        (TimeUnit::Hour, &[1, 3, 6, 12][..], 3_600_000.),
        (TimeUnit::Day, &[1, 2][..], DAY as f64),
        (TimeUnit::Week, &[1][..], 7. * DAY as f64),
        (TimeUnit::Month, &[1, 3, 6][..], 365.2425 / 12. * DAY as f64),
    ] {
        for step in steps {
            intervals.push(((*step as f64 * millis / target).ln().abs(), unit, *step));
        }
    }
    for step in [
        1, 2, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000, 50000, 100000, 200000,
        500000, 1000000,
    ] {
        intervals.push((
            (step as f64 * 365.2425 * DAY as f64 / target).ln().abs(),
            TimeUnit::Year,
            step,
        ));
    }
    let (_, unit, step) = intervals
        .into_iter()
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    TimeInterval {
        unit,
        step,
        week_start: WeekStart::Monday,
    }
}
