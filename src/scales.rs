//! Checked numeric and categorical scales. Architecture reference:
//! nivo/packages/scales/src/linearScale.ts and d3/src/index.js (d3-scale).
use crate::scene::{Result, ensure};
use std::collections::{BTreeMap, BTreeSet};
mod piecewise;
mod power;
pub use piecewise::PiecewiseScale;
pub use power::PowScale;
pub(crate) use power::{power_inverse, power_transform, validate_power_exponent};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LinearScale {
    pub domain: [f64; 2],
    pub range: [f64; 2],
    pub clamp: bool,
}
impl LinearScale {
    pub fn new(domain: [f64; 2], range: [f64; 2]) -> Result<Self> {
        ensure(
            domain.into_iter().chain(range).all(f64::is_finite),
            "scale endpoints must be finite",
        )?;
        ensure(
            (domain[1] - domain[0]).is_finite() && (range[1] - range[0]).is_finite(),
            "scale span overflow",
        )?;
        Ok(Self {
            domain,
            range,
            clamp: false,
        })
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
    pub fn map(&self, value: f64) -> f64 {
        let t = if self.domain[0] == self.domain[1] {
            0.5
        } else {
            (value - self.domain[0]) / (self.domain[1] - self.domain[0])
        };
        let t = if self.clamp { t.clamp(0., 1.) } else { t };
        self.range[0] * (1. - t) + self.range[1] * t
    }
    pub fn invert(&self, value: f64) -> f64 {
        let t = if self.range[0] == self.range[1] {
            0.5
        } else {
            (value - self.range[0]) / (self.range[1] - self.range[0])
        };
        let t = if self.clamp { t.clamp(0., 1.) } else { t };
        self.domain[0] * (1. - t) + self.domain[1] * t
    }
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        ticks(self.domain, count)
    }
    /// Extend the domain to stable decimal tick boundaries. As with `ticks`,
    /// requests are capped at 1000. Zero count, constant domains and boundaries
    /// that cannot be represented with a finite span retain the original domain.
    /// The ten-step convergence and reciprocal fractional increments follow
    /// d3-scale 4.0.2 / d3-array's linear nice algorithm (ISC; see notices).
    pub fn nice(mut self, count: usize) -> Self {
        if count == 0 || self.domain[0] == self.domain[1] {
            return self;
        }
        let reverse = self.domain[0] > self.domain[1];
        let mut lo = self.domain[0].min(self.domain[1]);
        let mut hi = self.domain[0].max(self.domain[1]);
        let mut previous = None;
        for _ in 0..10 {
            let Some(step) = nice_increment(lo, hi, count.min(1000)) else {
                return self;
            };
            if previous == Some(step) {
                self.domain = if reverse { [hi, lo] } else { [lo, hi] };
                return self;
            }
            let bounds = if step > 0. {
                [(lo / step).floor() * step, (hi / step).ceil() * step]
            } else {
                [(lo * step).ceil() / step, (hi * step).floor() / step]
            };
            if !bounds.iter().all(|v| v.is_finite())
                || !(bounds[1] - bounds[0]).is_finite()
                || bounds[0] > lo
                || bounds[1] < hi
            {
                return self;
            }
            [lo, hi] = bounds;
            previous = Some(step);
        }
        self
    }
}
// Negative increments encode the reciprocal of a fractional step, avoiding the
// rounding drift caused by subtracting adjacent generated ticks.
fn nice_increment(lo: f64, hi: f64, count: usize) -> Option<f64> {
    let raw = (hi - lo) / count as f64;
    if !raw.is_finite() || raw <= 0. {
        return None;
    }
    let power = raw.log10().floor();
    let error = raw / 10_f64.powf(power);
    let factor = if error >= 50_f64.sqrt() {
        10.
    } else if error >= 10_f64.sqrt() {
        5.
    } else if error >= 2_f64.sqrt() {
        2.
    } else {
        1.
    };
    let step = if power < 0. {
        -10_f64.powf(-power) / factor
    } else {
        10_f64.powf(power) * factor
    };
    if !step.is_finite() || step == 0. {
        return None;
    }
    // D3 retries a one-tick request at count two when its interior has no tick.
    let no_tick = if step > 0. {
        (lo / step).ceil() > (hi / step).floor()
    } else {
        (lo * -step).ceil() > (hi * -step).floor()
    };
    if count == 1 && no_tick {
        nice_increment(lo, hi, 2)
    } else {
        Some(step)
    }
}
pub fn ticks(domain: [f64; 2], count: usize) -> Vec<f64> {
    if count == 0 || !domain.iter().all(|v| v.is_finite()) {
        return vec![];
    }
    if domain[0] == domain[1] {
        return vec![domain[0]];
    }
    let lo = domain[0].min(domain[1]);
    let hi = domain[0].max(domain[1]);
    let raw = (hi - lo) / count.min(1000) as f64;
    if !raw.is_finite() || raw <= 0. {
        return vec![domain[0], domain[1]];
    }
    let power = 10_f64.powf(raw.log10().floor());
    let error = raw / power;
    let step = power
        * if error >= 50_f64.sqrt() {
            10.
        } else if error >= 10_f64.sqrt() {
            5.
        } else if error >= 2_f64.sqrt() {
            2.
        } else {
            1.
        };
    if step == 0. || !step.is_finite() {
        return vec![domain[0], domain[1]];
    }
    let first = (lo / step).ceil();
    let last = (hi / step).floor();
    if !first.is_finite() || !last.is_finite() || last - first > 2000. {
        return vec![domain[0], domain[1]];
    }
    let mut result = (0..=((last - first).max(0.) as usize))
        .map(|i| (first + i as f64) * step)
        .filter(|v| v.is_finite() && *v >= lo && *v <= hi)
        .collect::<Vec<_>>();
    if domain[0] > domain[1] {
        result.reverse();
    }
    result
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogScale {
    linear: LinearScale,
    sign: f64,
}
impl LogScale {
    pub fn new(domain: [f64; 2], range: [f64; 2]) -> Result<Self> {
        ensure(
            domain.iter().all(|v| v.is_finite() && *v != 0.)
                && domain[0].signum() == domain[1].signum(),
            "log domain must have one nonzero sign",
        )?;
        Ok(Self {
            linear: LinearScale::new([domain[0].abs().ln(), domain[1].abs().ln()], range)?,
            sign: domain[0].signum(),
        })
    }
    pub fn map(&self, value: f64) -> f64 {
        if value == 0. || value.signum() != self.sign {
            f64::NAN
        } else {
            self.linear.map(value.abs().ln())
        }
    }
    pub fn invert(&self, value: f64) -> f64 {
        self.sign * self.linear.invert(value).exp()
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.linear.clamp = clamp;
        self
    }
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        self.linear
            .ticks(count)
            .into_iter()
            .map(|v| self.sign * v.exp())
            .collect()
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SymLogScale {
    linear: LinearScale,
    constant: f64,
}
pub(crate) fn symlog_transform(value: f64, constant: f64) -> f64 {
    let ratio = value.abs() / constant;
    value.signum()
        * if ratio.is_finite() {
            ratio.ln_1p()
        } else {
            value.abs().ln() - constant.ln()
        }
}
pub(crate) fn symlog_inverse(value: f64, constant: f64) -> f64 {
    let direct = value.abs().exp_m1() * constant;
    value.signum()
        * if direct.is_finite() {
            direct
        } else {
            (value.abs() + constant.ln()).exp()
        }
}
impl SymLogScale {
    pub fn new(domain: [f64; 2], range: [f64; 2], constant: f64) -> Result<Self> {
        ensure(
            constant.is_finite() && constant > 0.,
            "symlog constant must be positive",
        )?;
        ensure(
            domain.iter().all(|v| v.is_finite()),
            "symlog domain must be finite",
        )?;
        let transform = |v: f64| symlog_transform(v, constant);
        Ok(Self {
            linear: LinearScale::new([transform(domain[0]), transform(domain[1])], range)?,
            constant,
        })
    }
    pub fn map(&self, value: f64) -> f64 {
        self.linear.map(symlog_transform(value, self.constant))
    }
    pub fn invert(&self, value: f64) -> f64 {
        let v = self.linear.invert(value);
        symlog_inverse(v, self.constant)
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.linear.clamp = clamp;
        self
    }
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        self.linear
            .ticks(count)
            .into_iter()
            .map(|v| symlog_inverse(v, self.constant))
            .collect()
    }
}
/// Checked D3 band parameters. Defaults mirror scaleBand, distinct from legacy
/// BandScale::new(padding). Align is a D3 extension beyond Nivo's round spec.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandScaleOptions {
    pub padding_inner: f64,
    pub padding_outer: f64,
    pub align: f64,
    pub round: bool,
}
impl Default for BandScaleOptions {
    fn default() -> Self {
        Self {
            padding_inner: 0.,
            padding_outer: 0.,
            align: 0.5,
            round: false,
        }
    }
}
impl BandScaleOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.padding_inner.is_finite()
                && (0. ..=1.).contains(&self.padding_inner)
                && self.padding_outer.is_finite()
                && (0. ..=4.).contains(&self.padding_outer)
                && self.align.is_finite()
                && (0. ..=1.).contains(&self.align),
            "invalid configured band options",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BandScale {
    options: Option<BandScaleOptions>,
    domain: Vec<String>,
    range: [f64; 2],
    pub bandwidth: f64,
    step: f64,
    offset: f64,
    window: [f64; 2],
    indices: BTreeMap<String, usize>,
}
impl BandScale {
    /// Resolve D3's complete band profile before any normalized window zoom.
    pub fn with_options(
        domain: Vec<String>,
        range: [f64; 2],
        options: &BandScaleOptions,
    ) -> Result<Self> {
        options.validate()?;
        let mut scale = Self::new(domain, range, 0.)?;
        let n = scale.domain.len() as f64;
        let span = (range[1] - range[0]).abs();
        let mut step = span / (n - options.padding_inner + 2. * options.padding_outer).max(1.);
        if options.round {
            step = step.floor();
        }
        let mut start =
            range[0].min(range[1]) + (span - step * (n - options.padding_inner)) * options.align;
        let mut bandwidth = step * (1. - options.padding_inner);
        if options.round {
            start = (start + 0.5).floor();
            bandwidth = (bandwidth + 0.5).floor();
        }
        let offset = if range[1] >= range[0] {
            start - range[0]
        } else {
            range[0] - start - (n - 1.) * step - bandwidth
        };
        ensure(
            [step, bandwidth, start, offset]
                .iter()
                .all(|v| v.is_finite()),
            "configured band projection overflow",
        )?;
        scale.step = step;
        scale.bandwidth = bandwidth;
        scale.offset = offset;
        scale.options = Some(*options);
        Ok(scale)
    }
    pub fn step(&self) -> f64 {
        self.step
    }
    pub fn options(&self) -> Option<&BandScaleOptions> {
        self.options.as_ref()
    }
    pub fn new(domain: Vec<String>, range: [f64; 2], padding: f64) -> Result<Self> {
        ensure(
            domain.len() <= 20_000
                && range.iter().all(|v| v.is_finite())
                && (range[1] - range[0]).is_finite(),
            "invalid band scale",
        )?;
        ensure(
            padding.is_finite() && (0. ..1.).contains(&padding),
            "band padding must be in 0..1",
        )?;
        let mut seen = BTreeSet::new();
        ensure(
            domain
                .iter()
                .all(|s| !s.is_empty() && s.len() <= 1024 && seen.insert(s)),
            "band ids must be unique, nonempty and bounded",
        )?;
        let span = (range[1] - range[0]).abs();
        let step = span / (domain.len() as f64 + padding).max(1.);
        let bandwidth = step * (1. - padding);
        let offset = step * padding;
        let indices = domain
            .iter()
            .enumerate()
            .map(|(i, key)| (key.clone(), i))
            .collect();
        Ok(Self {
            options: None,
            domain,
            range,
            bandwidth,
            step,
            offset,
            window: [0., 1.],
            indices,
        })
    }
    pub fn domain(&self) -> &[String] {
        &self.domain
    }
    /// Keep full category identities while projecting a normalized window of
    /// the original band range into the current output range.
    pub fn windowed(mut self, window: [f64; 2]) -> Result<Self> {
        ensure(
            window.iter().all(|v| v.is_finite())
                && window[0] >= 0.
                && window[1] <= 1.
                && window[0] < window[1],
            "band window must satisfy 0 <= start < end <= 1",
        )?;
        let span = window[1] - window[0];
        let original = (self.range[1] - self.range[0]).abs();
        let previous_span = self.window[1] - self.window[0];
        self.step = self.step * previous_span / span;
        self.bandwidth = self.bandwidth * previous_span / span;
        self.offset =
            (self.offset * previous_span + original * self.window[0] - original * window[0]) / span;
        self.window = window;
        ensure(
            [self.step, self.bandwidth, self.offset]
                .iter()
                .all(|v| v.is_finite()),
            "band window projection overflow",
        )?;
        Ok(self)
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn at(&self, index: usize) -> Option<f64> {
        if index >= self.domain.len() {
            None
        } else if self.range[1] >= self.range[0] {
            Some(self.range[0] + self.offset + index as f64 * self.step)
        } else {
            Some(self.range[0] - self.offset - index as f64 * self.step - self.bandwidth)
        }
    }
    pub fn map(&self, key: &str) -> Option<f64> {
        self.indices.get(key).and_then(|i| self.at(*i))
    }
    pub fn invert(&self, value: f64) -> Option<&str> {
        if !value.is_finite()
            || self.step == 0.
            || value < self.range[0].min(self.range[1])
            || value > self.range[0].max(self.range[1])
        {
            return None;
        }
        let forward = self.range[1] >= self.range[0];
        let delta = if forward {
            value - self.range[0] - self.offset
        } else {
            self.range[0] - self.offset - value
        };
        if delta < 0. {
            return None;
        }
        let index = (delta / self.step).floor() as usize;
        self.at(index)
            .filter(|x| value >= *x && value <= *x + self.bandwidth)
            .and_then(|_| self.domain.get(index).map(String::as_str))
    }
}
