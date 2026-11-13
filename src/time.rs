//! UTC epoch-millisecond scale with Gregorian calendar-aligned ticks.
//!
//! Unlike fixed thirty-day approximations, month/quarter/year ticks always land
//! on the first of the calendar month. No host timezone, DST, I/O or time crate
//! enters the default dependency graph. Leap seconds are not represented.
use crate::scales::LinearScale;
use crate::scene::ensure;
use crate::{ChartError, Result};

const DAY: i64 = 86_400_000;
/// Matches the ECMAScript Date bound and stays below f64's exact-integer limit.
pub const MAX_TIMESTAMP_MS: i64 = 8_640_000_000_000_000;
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeTick {
    pub timestamp_ms: i64,
    pub label: String,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UtcTimeScale {
    domain: [i64; 2],
    scale: LinearScale,
}
impl UtcTimeScale {
    pub fn new(domain: [i64; 2], range: [f64; 2]) -> Result<Self> {
        ensure(
            domain
                .iter()
                .all(|v| (-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(v)),
            "timestamps must be within +/-8640000000000000 epoch milliseconds",
        )?;
        Ok(Self {
            domain,
            scale: LinearScale::new([domain[0] as f64, domain[1] as f64], range)?,
        })
    }
    pub fn domain(&self) -> [i64; 2] {
        self.domain
    }
    pub fn range(&self) -> [f64; 2] {
        self.scale.range
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.scale.clamp = clamp;
        self
    }
    pub fn map(&self, timestamp_ms: i64) -> f64 {
        self.scale.map(timestamp_ms as f64)
    }
    pub fn invert(&self, pixel: f64) -> Result<i64> {
        let timestamp = self.scale.invert(pixel);
        ensure(
            timestamp.is_finite() && timestamp.abs() <= MAX_TIMESTAMP_MS as f64,
            "time inverse lies outside timestamp range",
        )?;
        Ok(timestamp.round() as i64)
    }
    /// Requested count is approximate, bounded to 1,000. Smaller steps use UTC
    /// clock boundaries, weekly steps use Mondays, longer steps use Gregorian
    /// month/quarter/year boundaries. Reversed domains return reversed ticks.
    pub fn ticks(&self, count: usize) -> Vec<TimeTick> {
        if count == 0 {
            return vec![];
        }
        let lo = self.domain[0].min(self.domain[1]);
        let hi = self.domain[0].max(self.domain[1]);
        if lo == hi {
            return vec![TimeTick {
                timestamp_ms: lo,
                label: format_timestamp(lo),
            }];
        }
        let target = (hi as f64 - lo as f64) / count.min(1000) as f64;
        let mut candidates = vec![];
        for milliseconds in [
            1,
            5,
            10,
            50,
            100,
            250,
            500,
            1000,
            5000,
            15000,
            30000,
            60000,
            300000,
            900000,
            1800000,
            3600000,
            10800000,
            21600000,
            43200000,
            DAY,
            2 * DAY,
            7 * DAY,
        ] {
            candidates.push((milliseconds as f64, Interval::Duration(milliseconds)));
        }
        let month = 365.2425 * DAY as f64 / 12.;
        for months in [1, 3, 6] {
            candidates.push((month * months as f64, Interval::Months(months)));
        }
        let years = (target / (month * 12.)).max(1.);
        let power = 10_f64.powf(years.log10().floor());
        for factor in [1., 2., 5., 10.] {
            let years = (power * factor).max(1.).round() as i64;
            candidates.push((years as f64 * month * 12., Interval::Months(years * 12)));
        }
        let interval = candidates
            .into_iter()
            .min_by(|(a, _), (b, _)| (a / target).ln().abs().total_cmp(&(b / target).ln().abs()))
            .map(|(_, i)| i)
            .unwrap_or(Interval::Duration(DAY));
        let mut timestamps = vec![];
        match interval {
            Interval::Duration(step) => {
                let offset = if step == 7 * DAY { 4 * DAY } else { 0 };
                let value = lo - offset;
                let mut tick = value.div_euclid(step) * step + offset;
                if tick < lo {
                    tick += step;
                }
                while tick <= hi && timestamps.len() < 2000 {
                    timestamps.push(tick);
                    tick += step;
                }
            }
            Interval::Months(step) => {
                let (year, month, _) = civil_from_days(lo.div_euclid(DAY));
                let serial = year * 12 + month - 1;
                let mut aligned = serial.div_euclid(step) * step;
                loop {
                    let year = aligned.div_euclid(12);
                    let month = aligned.rem_euclid(12) + 1;
                    let tick = days_from_civil(year, month, 1) * DAY;
                    if tick > hi || timestamps.len() >= 2000 {
                        break;
                    }
                    if tick >= lo {
                        timestamps.push(tick);
                    }
                    aligned += step;
                }
            }
        }
        if self.domain[0] > self.domain[1] {
            timestamps.reverse();
        }
        timestamps
            .into_iter()
            .map(|timestamp_ms| TimeTick {
                timestamp_ms,
                label: format_timestamp(timestamp_ms),
            })
            .collect()
    }
}
#[derive(Clone, Copy)]
enum Interval {
    Duration(i64),
    Months(i64),
}
/// Full UTC label, including milliseconds. Extended years retain their sign.
pub fn format_utc(timestamp_ms: i64) -> Result<String> {
    if !(-MAX_TIMESTAMP_MS..=MAX_TIMESTAMP_MS).contains(&timestamp_ms) {
        return Err(ChartError("timestamp out of range".into()));
    }
    Ok(format_timestamp(timestamp_ms))
}
fn format_timestamp(timestamp_ms: i64) -> String {
    let (year, month, day) = civil_from_days(timestamp_ms.div_euclid(DAY));
    let time = timestamp_ms.rem_euclid(DAY);
    let hour = time / 3_600_000;
    let minute = time / 60_000 % 60;
    let second = time / 1000 % 60;
    let millis = time % 1000;
    let year = if year < 0 {
        format!("-{:04}", -year)
    } else if year > 9999 {
        format!("+{year:06}")
    } else {
        format!("{year:04}")
    };
    format!("{year}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}.{millis:03} UTC")
}
// Proleptic Gregorian era arithmetic, valid across the complete timestamp bound.
pub(crate) fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}
pub(crate) fn days_from_civil(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let yoe = year - era * 400;
    let mp = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn utc_labels_epoch_negative_leap_and_extended_years() {
        assert_eq!(format_utc(0).unwrap(), "1970-01-01 00:00:00.000 UTC");
        assert_eq!(format_utc(-1).unwrap(), "1969-12-31 23:59:59.999 UTC");
        assert_eq!(
            format_utc(days_from_civil(2000, 2, 29) * DAY).unwrap(),
            "2000-02-29 00:00:00.000 UTC"
        );
        assert!(format_utc(MAX_TIMESTAMP_MS + 1).is_err());
        for year in [-10000, -400, -1, 0, 1, 1900, 2000, 2400, 10000, 275000] {
            for month in 1..=12 {
                assert_eq!(
                    civil_from_days(days_from_civil(year, month, 1)),
                    (year, month, 1)
                );
            }
        }
    }
    #[test]
    fn utc_calendar_months_quarters_years_and_reverse() {
        let start = days_from_civil(2024, 1, 1) * DAY;
        let end = days_from_civil(2025, 1, 1) * DAY;
        let scale = UtcTimeScale::new([start, end], [0., 1000.]).unwrap();
        let monthly = scale.ticks(12);
        assert_eq!(monthly.len(), 13);
        for tick in monthly {
            let (_, _, day) = civil_from_days(tick.timestamp_ms / DAY);
            assert_eq!(day, 1);
            assert_eq!(tick.timestamp_ms.rem_euclid(DAY), 0);
        }
        let quarters = scale.ticks(4);
        assert_eq!(quarters.len(), 5);
        for tick in &quarters {
            let (_, month, day) = civil_from_days(tick.timestamp_ms / DAY);
            assert_eq!(day, 1);
            assert_eq!((month - 1) % 3, 0);
        }
        let reversed = UtcTimeScale::new([end, start], [0., 1000.]).unwrap();
        let mut values = reversed.ticks(4);
        values.reverse();
        assert_eq!(values, quarters);
        assert_eq!(
            scale.invert(scale.map(start + 12345)).unwrap(),
            start + 12345
        );
        assert_eq!(scale.clamped(true).invert(-1.).unwrap(), start);
        assert!(scale.invert(f64::INFINITY).is_err());
    }
    #[test]
    fn utc_days_weeks_and_full_bound_remain_aligned() {
        let start = days_from_civil(2024, 2, 27) * DAY;
        let end = days_from_civil(2024, 3, 2) * DAY;
        let daily = UtcTimeScale::new([start, end], [0., 1.]).unwrap().ticks(4);
        assert_eq!(daily.len(), 5);
        assert!(daily.iter().any(|t| t.label.starts_with("2024-02-29")));
        let weekly = UtcTimeScale::new([0, 28 * DAY], [0., 1.]).unwrap().ticks(4);
        assert!(
            weekly
                .iter()
                .all(|t| t.timestamp_ms.div_euclid(DAY).rem_euclid(7) == 4)
        );
        let large = UtcTimeScale::new([-MAX_TIMESTAMP_MS, MAX_TIMESTAMP_MS], [0., 1.])
            .unwrap()
            .ticks(10);
        assert!(!large.is_empty());
        assert!(large.len() <= 20);
        assert!(UtcTimeScale::new([i64::MIN, 0], [0., 1.]).is_err());
    }
}
