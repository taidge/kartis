//! UTC interval selection from d3-time 3.x ticker(utcYear, utcMonth,
//! utcSunday, unixDay, utcHour, utcMinute). No local timezone or host I/O.
use super::tick_step;
use crate::scene::{Result, ensure};
use crate::time::{MAX_TIMESTAMP_MS, civil_from_days, days_from_civil};
const DAY: i64 = 86_400_000;
#[derive(Clone, Copy)]
enum Interval {
    Millis(i64),
    Months(i64),
    Years(i64),
}
fn interval(d: [f64; 2], n: usize) -> Interval {
    let lo = d[0].min(d[1]);
    let hi = d[0].max(d[1]);
    let target = (hi - lo) / n as f64;
    let durations = [
        1000.,
        5000.,
        15000.,
        30000.,
        60000.,
        300000.,
        900000.,
        1800000.,
        3600000.,
        10800000.,
        21600000.,
        43200000.,
        86400000.,
        172800000.,
        604800000.,
        2592000000.,
        7776000000.,
        31536000000.,
    ];
    let i = durations.partition_point(|v| *v <= target);
    if i == 0 {
        return Interval::Millis(tick_step(lo, hi, n as f64).max(1.) as i64);
    }
    if i == durations.len() {
        return Interval::Years(
            tick_step(lo / 31536000000., hi / 31536000000., n as f64).max(1.) as i64,
        );
    }
    let j = if target / durations[i - 1] < durations[i] / target {
        i - 1
    } else {
        i
    };
    match j {
        15 => Interval::Months(1),
        16 => Interval::Months(3),
        17 => Interval::Years(1),
        _ => Interval::Millis(durations[j] as i64),
    }
}
fn floor(t: i64, i: Interval) -> i64 {
    match i {
        Interval::Millis(step) => {
            let offset = if step == 7 * DAY { 3 * DAY } else { 0 };
            (t - offset).div_euclid(step) * step + offset
        }
        Interval::Months(step) => {
            let (y, m, _) = civil_from_days(t.div_euclid(DAY));
            let serial = y * 12 + m - 1;
            let s = serial.div_euclid(step) * step;
            days_from_civil(s.div_euclid(12), s.rem_euclid(12) + 1, 1) * DAY
        }
        Interval::Years(step) => {
            let (y, ..) = civil_from_days(t.div_euclid(DAY));
            days_from_civil(y.div_euclid(step) * step, 1, 1) * DAY
        }
    }
}
fn next(t: i64, i: Interval) -> i64 {
    match i {
        Interval::Millis(step) => t + step,
        Interval::Months(step) => {
            let (y, m, _) = civil_from_days(t.div_euclid(DAY));
            let s = y * 12 + m - 1 + step;
            days_from_civil(s.div_euclid(12), s.rem_euclid(12) + 1, 1) * DAY
        }
        Interval::Years(step) => {
            let (y, ..) = civil_from_days(t.div_euclid(DAY));
            days_from_civil(y + step, 1, 1) * DAY
        }
    }
}
pub(super) fn ticks(d: [f64; 2], n: usize) -> Vec<f64> {
    if n == 0 {
        return vec![];
    }
    let i = interval(d, n);
    let lo = d[0].min(d[1]) as i64;
    let hi = d[0].max(d[1]) as i64;
    // D3 interval.range starts at interval.ceil(start): that operation first
    // constructs Date(start-1), which TimeClips at the exact lower Date bound.
    if lo == -MAX_TIMESTAMP_MS {
        return vec![];
    }
    let mut t = floor(lo, i);
    if t < lo {
        t = next(t, i);
    }
    let mut out = vec![];
    while t <= hi && out.len() < 2001 {
        if t.abs() <= MAX_TIMESTAMP_MS {
            out.push(t as f64);
        }
        let z = next(t, i);
        if z <= t {
            break;
        }
        t = z;
    }
    if d[0] > d[1] {
        out.reverse();
    }
    out
}
pub(super) fn nice(d: [f64; 2], n: usize) -> Result<[f64; 2]> {
    let i = interval(d, n);
    let lo = d[0].min(d[1]) as i64;
    let hi = d[0].max(d[1]) as i64;
    let a = floor(lo, i);
    let z = floor(hi, i);
    let b = if z < hi { next(z, i) } else { z };
    ensure(
        a.abs() <= MAX_TIMESTAMP_MS && b.abs() <= MAX_TIMESTAMP_MS,
        "UTC nice exceeds timestamp bounds",
    )?;
    Ok(if d[0] > d[1] {
        [b as f64, a as f64]
    } else {
        [a as f64, b as f64]
    })
}
