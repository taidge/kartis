//! Additive D3 continuous and point-scale profiles. Legacy scale constructors
//! retain their historical tick and rounding behavior.
//! Algorithms follow d3-scale 4.0.2 log.js, linear.js and band.js (ISC).
//! Local Nivo 0.99.0 references: `packages/scales/src/{linearScale,logScale,
//! symlogScale,pointScale}.ts`. This API takes explicit endpoints; it does not
//! reproduce Nivo's automatic data-domain inference. Nice defaults to Off.
//! UTC inputs are bounded integral epoch milliseconds; UTC inverse uses Date's
//! truncating TimeClip profile. Linear/SymLog ticks use the original value
//! domain, not transformed units. Counts are capped at 1000; logarithmic minor
//! ticks can exceed the requested count (bounded by base <= 1000). Axes reject
//! more than 2000 ticks transactionally instead of silently dropping minors.
use crate::axes::{ChartScale, ScaleKind};
use crate::scales::LinearScale;
use crate::scene::{Result, ensure};
mod utc;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScaleNice {
    #[default]
    Off,
    Default,
    Count(usize),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ContinuousScaleOptions {
    pub kind: ScaleKind,
    /// Reverse the output range, independently of the domain's direction.
    pub reverse: bool,
    pub nice: ScaleNice,
    /// ECMAScript Math.round interpolation; inverse remains unrounded.
    /// Transcendental transforms can differ across floating-point runtimes by
    /// an ULP; a result at a half-integer can consequently round differently.
    pub round: bool,
    pub clamp: bool,
    /// Logarithmic tick/nice base. Mapping is base independent. Supported 1 < base <= 1000.
    pub log_base: f64,
}
impl Default for ContinuousScaleOptions {
    fn default() -> Self {
        Self {
            kind: ScaleKind::Linear,
            reverse: false,
            nice: ScaleNice::Off,
            round: false,
            clamp: false,
            log_base: 10.,
        }
    }
}
impl ContinuousScaleOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.log_base.is_finite() && self.log_base > 1. && self.log_base <= 1000.,
            "log base must satisfy 1 < base <= 1000",
        )?;
        if let ScaleNice::Count(n) = self.nice {
            ensure((1..=1000).contains(&n), "nice count must be 1..=1000")?;
        }
        if let ScaleKind::SymLog { constant } = self.kind {
            ensure(
                constant.is_finite() && constant > 0.,
                "symlog constant must be positive",
            )?;
        }
        if let ScaleKind::Pow { exponent } = self.kind {
            crate::scales::validate_power_exponent(exponent)?;
        }
        Ok(())
    }
}
/// Explicit numeric breakpoints and their actual output coordinates. This
/// additive profile leaves `ContinuousScaleOptions` small and `Copy`.
///
/// Domain stops must be monotonic, ascending or descending, with repeated knots
/// and fully constant domains accepted. Ties follow D3's bisect-right after
/// orienting the native transformed domain. Range stops may turn or form
/// plateaus; these require an explicit `invert_segment` branch. A strict range
/// selects one inverse branch even when a repeated domain is not bijective.
/// Explicit piecewise log/SymLog transforms and inverse primitives use fixed
/// libm implementations, including after nice or viewport removes repetition.
/// Signed powers retain the platform powf path. Transcendental results are not
/// promised to have identical bits in every runtime; duplicate knots can make
/// such a difference select different discontinuous branches.
/// `reverse` reverses all range stops. Nice extends only the outer domain
/// stops, preserving the interior stops exactly, as in D3. Ticks cover those
/// outer endpoints in source value units. UTC stops follow the same bounded
/// integral epoch-millisecond and truncating inverse policy as two-stop UTC.
///
/// A two-endpoint-only viewport cannot describe this mapping: navigation must
/// retain the full profile when relaying domains. Range stops are actual output
/// coordinates, not implicit fractions of a plot rectangle.
#[derive(Clone, Debug, PartialEq)]
pub struct PiecewiseScaleOptions {
    pub domain: Vec<f64>,
    pub range: Vec<f64>,
    pub continuous: ContinuousScaleOptions,
}
impl Default for PiecewiseScaleOptions {
    fn default() -> Self {
        Self {
            domain: vec![0., 1.],
            range: vec![0., 1.],
            continuous: ContinuousScaleOptions::default(),
        }
    }
}
impl PiecewiseScaleOptions {
    pub fn validate(&self) -> Result<()> {
        crate::scales::PiecewiseScale::new(&self.domain, &self.range, &self.continuous)?;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ResolvedContinuousScale {
    source: [f64; 2],
    source_stops: Option<Vec<f64>>,
    chart: ChartScale,
}
impl ResolvedContinuousScale {
    pub fn new(domain: [f64; 2], mut range: [f64; 2], o: &ContinuousScaleOptions) -> Result<Self> {
        o.validate()?;
        // Validate the source even if nice would otherwise conceal an invalid endpoint.
        ChartScale::configured_raw(o.kind, domain, range, o)?;
        if o.reverse {
            range.reverse();
        }
        let mut display = domain;
        if o.nice != ScaleNice::Off {
            let n = if let ScaleNice::Count(n) = o.nice {
                n
            } else {
                10
            };
            display = match o.kind {
                ScaleKind::Log => log_nice(domain, o.log_base)?,
                ScaleKind::Utc => utc::nice(domain, n)?,
                _ => LinearScale::new(domain, [0., 1.])?.nice(n).domain,
            };
        }
        Ok(Self {
            source: domain,
            source_stops: None,
            chart: ChartScale::configured_raw(o.kind, display, range, o)?,
        })
    }
    /// Resolve all breakpoints without collapsing them to the two outer
    /// endpoints. Invalid source or niced stops leave the options untouched.
    pub fn piecewise(profile: &PiecewiseScaleOptions) -> Result<Self> {
        profile.validate()?;
        let o = &profile.continuous;
        let mut range = profile.range.clone();
        if o.reverse {
            range.reverse();
        }
        let mut display = profile.domain.clone();
        let last = display.len() - 1;
        let endpoints = [display[0], display[last]];
        let resolved = Self::new(endpoints, [range[0], range[last]], o)?;
        let niced = resolved.display_domain();
        display[0] = niced[0];
        display[last] = niced[1];
        Ok(Self {
            source: endpoints,
            source_stops: Some(profile.domain.clone()),
            chart: ChartScale::piecewise_raw(&display, &range, o)?,
        })
    }
    pub fn source_domain(&self) -> [f64; 2] {
        self.source
    }
    pub fn display_domain(&self) -> [f64; 2] {
        self.chart.domain()
    }
    pub fn source_domain_stops(&self) -> &[f64] {
        self.source_stops.as_deref().unwrap_or(&self.source)
    }
    pub fn display_domain_stops(&self) -> &[f64] {
        self.chart.domain_stops()
    }
    pub fn range_stops(&self) -> &[f64] {
        self.chart.range_stops()
    }
    pub fn is_piecewise(&self) -> bool {
        self.chart.is_piecewise()
    }
    /// Whether the range selects one global inverse branch. Repeated domain
    /// knots may still make the forward mapping non-bijective.
    pub fn is_invertible(&self) -> bool {
        self.chart.is_invertible()
    }
    pub fn invert_segment(&self, index: usize, value: f64) -> Result<f64> {
        self.chart.invert_segment(index, value)
    }
    pub fn range(&self) -> [f64; 2] {
        self.chart.range()
    }
    pub fn map(&self, value: f64) -> f64 {
        self.chart.map(value)
    }
    pub fn invert(&self, value: f64) -> Result<f64> {
        self.chart.invert(value)
    }
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        self.chart.ticks(count)
    }
    pub fn chart_scale(&self) -> &ChartScale {
        &self.chart
    }
    pub fn into_chart_scale(self) -> ChartScale {
        self.chart
    }
}
/// ECMAScript round, including the negative zero produced by `[-0.5,0)`.
pub(crate) fn js_round(x: f64) -> f64 {
    if x == 0. || !x.is_finite() {
        return x;
    }
    let floor = x.floor();
    let r = if x - floor < 0.5 { floor } else { floor + 1. };
    if r == 0. && x < 0. { -0. } else { r }
}
/// d3 continuous.bimap swaps a descending domain before interpolation. This
/// ordering matters at integral TimeClip boundaries and negative half pixels.
pub(crate) fn interpolate(mut domain: [f64; 2], mut range: [f64; 2], x: f64, clamp: bool) -> f64 {
    if domain[1] < domain[0] {
        domain.reverse();
        range.reverse();
    }
    let t = if domain[0] == domain[1] {
        0.5
    } else {
        (x - domain[0]) / (domain[1] - domain[0])
    };
    let t = if clamp { t.clamp(0., 1.) } else { t };
    range[0] * (1. - t) + range[1] * t
}
fn log_value(x: f64, b: f64) -> f64 {
    let v = if b == 10. {
        x.abs().log10()
    } else if b == 2. {
        x.abs().log2()
    } else {
        x.abs().ln() / b.ln()
    };
    v * x.signum()
}
fn power(b: f64, e: f64) -> f64 {
    // D3's base-10 pows deliberately parses decimal powers instead of pow:
    // powi(23) can otherwise move an exact endpoint and lose a minor tick.
    if b == 10. && e.is_finite() && e.fract() == 0. {
        format!("1e{e:.0}").parse().unwrap_or(f64::INFINITY)
    } else if b == std::f64::consts::E {
        e.exp()
    } else {
        b.powf(e)
    }
}
fn log_power(x: f64, b: f64, sign: f64) -> f64 {
    sign * power(b, sign * x)
}
fn log_nice(d: [f64; 2], b: f64) -> Result<[f64; 2]> {
    let reverse = d[0] > d[1];
    let s = d[0].signum();
    let lo = d[0].min(d[1]);
    let hi = d[0].max(d[1]);
    let a = log_power(log_value(lo, b).floor(), b, s);
    let z = log_power(log_value(hi, b).ceil(), b, s);
    ensure(
        a.is_finite() && z.is_finite() && a != 0. && z != 0.,
        "log nice boundary overflow",
    )?;
    Ok(if reverse { [z, a] } else { [a, z] })
}
pub(crate) fn configured_ticks(kind: ScaleKind, d: [f64; 2], count: usize, base: f64) -> Vec<f64> {
    if count == 0 {
        return vec![];
    }
    let count = count.min(1000);
    match kind {
        ScaleKind::Log => log_ticks(d, count, base),
        ScaleKind::Utc => utc::ticks(d, count),
        _ => numeric_ticks(d, count as f64),
    }
}
// Reciprocal increments preserve decimal tick boundaries, as d3-array 3.2.5.
pub(crate) fn numeric_ticks(d: [f64; 2], count: f64) -> Vec<f64> {
    if count <= 0. {
        return vec![];
    }
    if d[0] == d[1] {
        return vec![d[0]];
    }
    let reverse = d[0] > d[1];
    let lo = d[0].min(d[1]);
    let hi = d[0].max(d[1]);
    let Some((first, last, inc)) = tick_spec(lo, hi, count) else {
        return vec![];
    };
    if last < first || last - first > 2000. {
        return vec![];
    }
    let mut r = (0..=(last - first) as usize)
        .map(|i| {
            if inc < 0. {
                (first + i as f64) / -inc
            } else {
                (first + i as f64) * inc
            }
        })
        .collect::<Vec<_>>();
    if reverse {
        r.reverse();
    }
    r
}
fn tick_spec(lo: f64, hi: f64, count: f64) -> Option<(f64, f64, f64)> {
    let step = (hi - lo) / count;
    if step <= 0. || !step.is_finite() {
        return None;
    }
    let power = step.log10().floor();
    let error = step / 10_f64.powf(power);
    let factor = if error >= 50_f64.sqrt() {
        10.
    } else if error >= 10_f64.sqrt() {
        5.
    } else if error >= 2_f64.sqrt() {
        2.
    } else {
        1.
    };
    let (mut first, mut last, inc) = if power < 0. {
        let k = 10_f64.powf(-power) / factor;
        let mut a = js_round(lo * k);
        let mut b = js_round(hi * k);
        if a / k < lo {
            a += 1.;
        }
        if b / k > hi {
            b -= 1.;
        }
        (a, b, -k)
    } else {
        let k = 10_f64.powf(power) * factor;
        let mut a = js_round(lo / k);
        let mut b = js_round(hi / k);
        if a * k < lo {
            a += 1.;
        }
        if b * k > hi {
            b -= 1.;
        }
        (a, b, k)
    };
    if !first.is_finite() || !last.is_finite() || !inc.is_finite() || inc == 0. {
        return None;
    }
    if last < first && (0.5..2.).contains(&count) {
        return tick_spec(lo, hi, count * 2.);
    }
    // Canonical tick index zero does not need the Math.round sign.
    if first == 0. {
        first = 0.;
    }
    if last == 0. {
        last = 0.;
    }
    Some((first, last, inc))
}
pub(crate) fn tick_step(lo: f64, hi: f64, count: f64) -> f64 {
    tick_spec(lo, hi, count).map_or(1., |(_, _, i)| if i < 0. { 1. / -i } else { i })
}
fn log_ticks(d: [f64; 2], count: usize, base: f64) -> Vec<f64> {
    let reverse = d[0] > d[1];
    let u = d[0].min(d[1]);
    let v = d[0].max(d[1]);
    let sign = u.signum();
    let i = log_value(u, base);
    let j = log_value(v, base);
    let n = count as f64;
    let mut out = vec![];
    if base.fract() == 0. && j - i < n {
        let first = i.floor() as i32;
        let last = j.ceil() as i32;
        // At most 1000 ticks requested; integer minors are bounded by base 1000.
        if (last - first) as usize * (base as usize - 1) > 1_000_000 {
            return vec![];
        }
        for exponent in first..=last {
            let magnitude = power(base, f64::from(exponent).abs());
            for slot in 1..base as usize {
                let k = if sign > 0. {
                    slot
                } else {
                    base as usize - slot
                } as f64;
                let t = if sign > 0. {
                    if exponent < 0 {
                        k / magnitude
                    } else {
                        k * magnitude
                    }
                } else if exponent > 0 {
                    -k / magnitude
                } else {
                    -k * magnitude
                };
                if t >= u && t <= v && t.is_finite() {
                    out.push(t);
                }
            }
        }
        if out.len() * 2 < count {
            out = numeric_ticks([u, v], n);
        }
    } else {
        out = numeric_ticks([i, j], (j - i).min(n))
            .into_iter()
            .map(|x| log_power(x, base, sign))
            .filter(|x| x.is_finite() && *x != 0.)
            .collect();
    }
    if reverse {
        out.reverse();
    }
    out
}

/// Numeric and textual categories are distinct. Numeric -0 has the same identity as +0.
#[derive(Clone, Debug, PartialEq)]
pub enum CategoryKey {
    String(String),
    Number(f64),
}
impl From<&str> for CategoryKey {
    fn from(s: &str) -> Self {
        Self::String(s.into())
    }
}
impl From<String> for CategoryKey {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}
impl From<f64> for CategoryKey {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}
impl CategoryKey {
    pub fn validate(&self) -> Result<()> {
        self.identity().map(|_| ())
    }
    /// Stable typed key for layout ownership/IDs; numeric signed zeros share
    /// one key, and a string never aliases a number with the same display text.
    pub fn identity(&self) -> Result<String> {
        match self {
            Self::String(s) => {
                ensure(s.len() <= 1024, "point category text budget exceeded")?;
                Ok(format!("s{s}"))
            }
            Self::Number(n) => {
                ensure(n.is_finite(), "point category number must be finite")?;
                Ok(format!("n{:x}", if *n == 0. { 0 } else { n.to_bits() }))
            }
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointScaleOptions {
    pub padding: f64,
    pub align: f64,
    pub round: bool,
    pub reverse: bool,
}
impl Default for PointScaleOptions {
    fn default() -> Self {
        Self {
            padding: 0.,
            align: 0.5,
            round: false,
            reverse: false,
        }
    }
}
impl PointScaleOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.padding.is_finite()
                && (0. ..=1e6).contains(&self.padding)
                && self.align.is_finite()
                && (0. ..=1.).contains(&self.align),
            "invalid point padding/align",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PointScale {
    domain: Vec<CategoryKey>,
    range: [f64; 2],
    positions: Vec<f64>,
    step: f64,
    indices: std::collections::BTreeMap<String, usize>,
}
impl PointScale {
    pub fn new(
        input: Vec<CategoryKey>,
        mut range: [f64; 2],
        o: &PointScaleOptions,
    ) -> Result<Self> {
        ensure(
            input.len() <= 20_000
                && range.iter().all(|x| x.is_finite())
                && (range[1] - range[0]).is_finite(),
            "point scale budget/endpoints invalid",
        )?;
        o.validate()?;
        let mut domain = vec![];
        let mut indices = std::collections::BTreeMap::new();
        for mut key in input {
            let id = key.identity()?;
            if let std::collections::btree_map::Entry::Vacant(e) = indices.entry(id) {
                if let CategoryKey::Number(n) = &mut key
                    && *n == 0.
                {
                    *n = 0.;
                }
                e.insert(domain.len());
                domain.push(key);
            }
        }
        if o.reverse {
            range.reverse();
        }
        let rev = range[1] < range[0];
        let start = range[0].min(range[1]);
        let span = (range[1] - range[0]).abs();
        let n = domain.len() as f64;
        let mut step = span / (n - 1. + o.padding * 2.).max(1.);
        if o.round {
            step = step.floor();
        }
        let mut first = start + (span - step * (n - 1.)) * o.align;
        if o.round {
            first = js_round(first);
        }
        let mut positions = (0..domain.len())
            .map(|i| first + step * i as f64)
            .collect::<Vec<_>>();
        if rev {
            positions.reverse();
        }
        Ok(Self {
            domain,
            range,
            positions,
            step,
            indices,
        })
    }
    pub fn domain(&self) -> &[CategoryKey] {
        &self.domain
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn step(&self) -> f64 {
        self.step
    }
    pub fn at(&self, i: usize) -> Option<f64> {
        self.positions.get(i).copied()
    }
    pub fn map(&self, key: &CategoryKey) -> Option<f64> {
        self.indices
            .get(&key.identity().ok()?)
            .and_then(|i| self.at(*i))
    }
}
