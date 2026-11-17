//! Shared chart scales, numeric formatting and configurable Cartesian axes.
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::scales::{LinearScale, LogScale, PiecewiseScale, PowScale, SymLogScale};
use crate::scene::ensure;
use crate::{
    ChartConfig, Geometry, Label, MAX_LABEL_BYTES, MAX_TIMESTAMP_MS, Mark, Result, Scene,
    TextAnchor, UtcTimeScale,
};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScaleKind {
    #[default]
    Linear,
    Log,
    SymLog {
        constant: f64,
    },
    Utc,
    /// Signed power mapping; negative exponents require a same-sign nonzero domain.
    Pow {
        exponent: f64,
    },
    Sqrt,
}
impl ScaleKind {
    pub(crate) fn requires_nonzero_domain(self) -> bool {
        matches!(self, Self::Log) || matches!(self, Self::Pow { exponent } if exponent < 0.)
    }
    pub(crate) fn supports_zero_baseline(self) -> bool {
        matches!(self, Self::Linear | Self::SymLog { .. } | Self::Sqrt)
            || matches!(self, Self::Pow { exponent } if exponent > 0.)
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ValueFormat {
    #[default]
    Auto,
    Fixed(usize),
    Scientific(usize),
    Percent(usize),
    Compact(usize),
    Grouped(usize),
    Utc,
    UtcDate,
    UtcTime,
    UtcYearMonth,
    UtcYear,
    Affix {
        prefix: String,
        suffix: String,
        precision: usize,
    },
    Specifier(Box<crate::format::NumberFormat>),
    Prefix(Box<crate::format::PrefixFormat>),
    DateTime(Box<crate::datetime::DateTimeFormat>),
}
impl ValueFormat {
    /// Explicit locale and wall-clock offset for integral epoch milliseconds.
    pub fn date_time(
        specifier: &str,
        locale: &crate::datetime::TimeLocale,
        zone: crate::datetime::TimeZone,
    ) -> Result<Self> {
        Ok(Self::DateTime(Box::new(
            crate::datetime::DateTimeFormat::new(specifier, locale, zone)?,
        )))
    }
    pub fn specifier(spec: &str, locale: &crate::format::FormatLocale) -> Result<Self> {
        let format = crate::format::NumberFormat::new(spec, locale)?;
        ensure(!format.is_text(), "numeric axes cannot use string format c")?;
        Ok(Self::Specifier(Box::new(format)))
    }
    pub fn prefix(
        spec: &str,
        reference: f64,
        locale: &crate::format::FormatLocale,
    ) -> Result<Self> {
        Ok(Self::Prefix(Box::new(crate::format::PrefixFormat::new(
            spec, reference, locale,
        )?)))
    }
    pub fn validate(&self) -> Result<()> {
        if let Self::Specifier(format) = self {
            ensure(!format.is_text(), "numeric axes cannot use string format c")?;
        }
        let precision = match self {
            Self::Fixed(p)
            | Self::Scientific(p)
            | Self::Percent(p)
            | Self::Compact(p)
            | Self::Grouped(p) => *p,
            Self::Affix {
                prefix,
                suffix,
                precision,
            } => {
                ensure(
                    prefix.len() + suffix.len() <= 512,
                    "format affix budget exceeded",
                )?;
                *precision
            }
            _ => 0,
        };
        ensure(precision <= 20, "format precision must be at most 20")
    }
    pub fn format(&self, value: f64) -> Result<String> {
        self.validate()?;
        ensure(value.is_finite(), "cannot format a nonfinite number")?;
        let value = if value == 0. { 0. } else { value };
        let numeric = |spec: &str, number: f64| {
            crate::format::format_number(
                spec,
                number,
                &crate::format::FormatLocale {
                    minus: "-".into(),
                    ..Default::default()
                },
            )
        };
        let out = match self {
            Self::DateTime(format) => {
                ensure(
                    value.fract() == 0. && value.abs() <= MAX_TIMESTAMP_MS as f64,
                    "date-time formatting needs bounded integral epoch milliseconds",
                )?;
                format.format(value as i64)?
            }
            Self::Specifier(format) => format.format(value)?,
            Self::Prefix(format) => format.format(value)?,
            Self::Auto => {
                if value == 0. {
                    "0".into()
                } else if value.abs() >= 1e6 || value.abs() < 0.001 {
                    numeric(".2e", value)?
                } else {
                    numeric(".3~f", value)?
                }
            }
            Self::Fixed(p) => numeric(&format!(".{p}f"), value)?,
            Self::Scientific(p) => numeric(&format!(".{p}e"), value)?,
            Self::Percent(p) => numeric(&format!(".{p}%"), value)?,
            Self::Compact(p) => {
                let (factor, suffix) = if value.abs() >= 1e12 {
                    (1e12, "T")
                } else if value.abs() >= 1e9 {
                    (1e9, "B")
                } else if value.abs() >= 1e6 {
                    (1e6, "M")
                } else if value.abs() >= 1e3 {
                    (1e3, "k")
                } else {
                    (1., "")
                };
                format!("{}{suffix}", numeric(&format!(".{p}f"), value / factor)?)
            }
            Self::Grouped(p) => numeric(&format!(",.{p}f"), value)?,
            Self::Utc | Self::UtcDate | Self::UtcTime | Self::UtcYearMonth | Self::UtcYear => {
                ensure(
                    value.fract() == 0. && value.abs() <= MAX_TIMESTAMP_MS as f64,
                    "UTC formatting needs bounded integral epoch milliseconds",
                )?;
                let full = crate::format_utc(value as i64)?;
                let (date, time) = full.split_once(' ').unwrap();
                match self {
                    Self::UtcDate => date.into(),
                    Self::UtcTime => time.strip_suffix(" UTC").unwrap_or(time).into(),
                    Self::UtcYearMonth => date.rsplit_once('-').unwrap().0.into(),
                    Self::UtcYear => date
                        .rsplit_once('-')
                        .unwrap()
                        .0
                        .rsplit_once('-')
                        .unwrap()
                        .0
                        .into(),
                    _ => full,
                }
            }
            Self::Affix {
                prefix,
                suffix,
                precision: p,
            } => format!("{prefix}{}{suffix}", numeric(&format!(".{p}f"), value)?),
        };
        ensure(
            out.len() <= MAX_LABEL_BYTES,
            "formatted label budget exceeded",
        )?;
        Ok(out)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct AxisOptions {
    pub tick_rotation: f64,
    pub visible: bool,
    pub grid: bool,
    pub tick_count: usize,
    pub tick_values: Option<Vec<f64>>,
    /// Genuine local-calendar boundaries on ScaleKind::Utc geometry. Explicit
    /// tick_values and a calendar are mutually exclusive. Auto labels use the
    /// calendar's zone; an explicit ValueFormat controls labels independently.
    pub tick_calendar: Option<crate::zoned_time::TimeCalendar>,
    pub format: ValueFormat,
    pub legend: Option<String>,
    pub tick_size: f64,
    pub tick_padding: f64,
}
impl Default for AxisOptions {
    fn default() -> Self {
        Self {
            tick_rotation: 0.,
            visible: true,
            grid: false,
            tick_count: 6,
            tick_values: None,
            tick_calendar: None,
            format: ValueFormat::Auto,
            legend: None,
            tick_size: 4.,
            tick_padding: 6.,
        }
    }
}
impl AxisOptions {
    pub fn validate(&self) -> Result<()> {
        self.format.validate()?;
        if let Some(calendar) = &self.tick_calendar {
            calendar.validate()?;
            ensure(
                self.tick_values.is_none(),
                "calendar ticks and explicit ticks are mutually exclusive",
            )?;
        }
        ensure(
            self.tick_count <= 1000
                && self
                    .tick_values
                    .as_ref()
                    .is_none_or(|v| v.len() <= 1000 && v.iter().all(|v| v.is_finite())),
            "axis tick budget exceeded or invalid ticks",
        )?;
        ensure(
            self.tick_rotation.is_finite()
                && (-360. ..=360.).contains(&self.tick_rotation)
                && [self.tick_size, self.tick_padding]
                    .into_iter()
                    .all(|v| v.is_finite() && (0. ..=1000.).contains(&v))
                && self
                    .legend
                    .as_ref()
                    .is_none_or(|v| v.len() <= MAX_LABEL_BYTES),
            "invalid axis geometry or legend",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct CartesianAxes {
    pub x: AxisOptions,
    pub y: AxisOptions,
    pub x_side: AxisSide,
    pub y_side: AxisSide,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryAxisOptions {
    pub tick_rotation: f64,
    pub visible: bool,
    pub grid: bool,
    pub tick_count: usize,
    pub tick_values: Option<Vec<String>>,
    pub legend: Option<String>,
    pub tick_padding: f64,
}
impl Default for CategoryAxisOptions {
    fn default() -> Self {
        Self {
            tick_rotation: 0.,
            visible: true,
            grid: false,
            tick_count: 40,
            tick_values: None,
            legend: None,
            tick_padding: 10.,
        }
    }
}
impl CategoryAxisOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.tick_count <= 1000
                && self.tick_rotation.is_finite()
                && (-360. ..=360.).contains(&self.tick_rotation)
                && self.tick_padding.is_finite()
                && (0. ..=1000.).contains(&self.tick_padding)
                && self
                    .legend
                    .as_ref()
                    .is_none_or(|s| s.len() <= MAX_LABEL_BYTES)
                && self.tick_values.as_ref().is_none_or(|v| {
                    v.len() <= 1000 && v.iter().all(|s| s.len() <= MAX_LABEL_BYTES)
                }),
            "invalid category axis",
        )
    }
}
impl Default for CartesianAxes {
    fn default() -> Self {
        Self {
            x: AxisOptions::default(),
            y: AxisOptions {
                grid: true,
                tick_count: 5,
                ..Default::default()
            },
            x_side: AxisSide::Bottom,
            y_side: AxisSide::Left,
        }
    }
}
impl CartesianAxes {
    pub fn validate(&self) -> Result<()> {
        self.x.validate()?;
        self.y.validate()?;
        ensure(
            matches!(self.x_side, AxisSide::Top | AxisSide::Bottom)
                && matches!(self.y_side, AxisSide::Left | AxisSide::Right),
            "Cartesian x axis must be top/bottom and y axis left/right",
        )
    }
}
#[derive(Clone, Debug)]
enum Scale {
    Linear(LinearScale),
    Log(LogScale),
    SymLog(SymLogScale),
    Utc(UtcTimeScale),
    Pow(PowScale),
    Piecewise(PiecewiseScale),
}
#[derive(Clone, Debug)]
pub struct ChartScale {
    kind: ScaleKind,
    domain: [f64; 2],
    range: [f64; 2],
    scale: Scale,
    configured: Option<crate::scale_options::ContinuousScaleOptions>,
    clamp: bool,
}
impl ChartScale {
    pub fn new(kind: ScaleKind, domain: [f64; 2], range: [f64; 2]) -> Result<Self> {
        ensure(domain[0] <= domain[1], "chart domain must be ascending")?;
        Self::construct(kind, domain, range)
    }
    fn construct(kind: ScaleKind, domain: [f64; 2], range: [f64; 2]) -> Result<Self> {
        let scale = match kind {
            ScaleKind::Linear => Scale::Linear(LinearScale::new(domain, range)?),
            ScaleKind::Pow { exponent } => Scale::Pow(PowScale::new(domain, range, exponent)?),
            ScaleKind::Sqrt => Scale::Pow(PowScale::sqrt(domain, range)?),
            ScaleKind::Log => Scale::Log(LogScale::new(domain, range)?),
            ScaleKind::SymLog { constant } => {
                Scale::SymLog(SymLogScale::new(domain, range, constant)?)
            }
            ScaleKind::Utc => {
                ensure(
                    domain.iter().all(|v| {
                        v.is_finite() && v.fract() == 0. && v.abs() <= MAX_TIMESTAMP_MS as f64
                    }),
                    "UTC domain must contain bounded integral epoch milliseconds",
                )?;
                Scale::Utc(UtcTimeScale::new(
                    [domain[0] as i64, domain[1] as i64],
                    range,
                )?)
            }
        };
        Ok(Self {
            kind,
            domain,
            range,
            scale,
            configured: None,
            clamp: false,
        })
    }
    pub fn with_options(
        domain: [f64; 2],
        range: [f64; 2],
        options: &crate::scale_options::ContinuousScaleOptions,
    ) -> Result<Self> {
        Ok(
            crate::scale_options::ResolvedContinuousScale::new(domain, range, options)?
                .into_chart_scale(),
        )
    }
    /// Resolve an explicit numeric piecewise profile, preserving every stop.
    pub fn with_piecewise_options(
        options: &crate::scale_options::PiecewiseScaleOptions,
    ) -> Result<Self> {
        Ok(crate::scale_options::ResolvedContinuousScale::piecewise(options)?.into_chart_scale())
    }
    pub(crate) fn piecewise_raw(
        domain: &[f64],
        range: &[f64],
        options: &crate::scale_options::ContinuousScaleOptions,
    ) -> Result<Self> {
        let scale = PiecewiseScale::new(domain, range, options)?;
        Ok(Self {
            kind: options.kind,
            domain: [domain[0], domain[domain.len() - 1]],
            range: [range[0], range[range.len() - 1]],
            scale: Scale::Piecewise(scale),
            configured: Some(*options),
            clamp: options.clamp,
        })
    }
    pub fn is_piecewise(&self) -> bool {
        matches!(self.scale, Scale::Piecewise(_))
    }
    /// Full stops, rather than only the outer endpoints returned by `domain`.
    pub fn domain_stops(&self) -> &[f64] {
        if let Scale::Piecewise(s) = &self.scale {
            s.domain()
        } else {
            &self.domain
        }
    }
    pub fn range_stops(&self) -> &[f64] {
        if let Scale::Piecewise(s) = &self.scale {
            s.range()
        } else {
            &self.range
        }
    }
    pub fn is_invertible(&self) -> bool {
        if let Scale::Piecewise(s) = &self.scale {
            s.is_invertible()
        } else {
            self.range[0] != self.range[1]
        }
    }
    /// Choose an explicit branch for a turning numeric piecewise range.
    pub fn invert_segment(&self, index: usize, pixel: f64) -> Result<f64> {
        if let Scale::Piecewise(s) = &self.scale {
            return s.invert_segment(index, pixel);
        }
        ensure(index == 0, "inverse segment out of bounds")?;
        self.invert(pixel)
    }
    pub(crate) fn configured_raw(
        kind: ScaleKind,
        domain: [f64; 2],
        range: [f64; 2],
        options: &crate::scale_options::ContinuousScaleOptions,
    ) -> Result<Self> {
        let mut out = Self::construct(kind, domain, range)?.clamped(options.clamp);
        out.configured = Some(*options);
        Ok(out)
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        if let Some(o) = &mut self.configured {
            o.clamp = clamp;
        }
        self.scale = match self.scale {
            Scale::Linear(s) => Scale::Linear(s.clamped(clamp)),
            Scale::Log(s) => Scale::Log(s.clamped(clamp)),
            Scale::SymLog(s) => Scale::SymLog(s.clamped(clamp)),
            Scale::Utc(s) => Scale::Utc(s.clamped(clamp)),
            Scale::Pow(s) => Scale::Pow(s.clamped(clamp)),
            Scale::Piecewise(s) => Scale::Piecewise(s.clamped(clamp)),
        };
        self
    }
    pub fn kind(&self) -> ScaleKind {
        self.kind
    }
    pub(crate) fn configured_options(
        &self,
    ) -> Option<crate::scale_options::ContinuousScaleOptions> {
        self.configured
    }
    pub fn is_clamped(&self) -> bool {
        self.clamp
    }
    pub fn is_rounded(&self) -> bool {
        self.configured.is_some_and(|o| o.round)
    }
    /// Project a coordinate already expressed in `transformed` domain units.
    pub(crate) fn map_transformed(&self, value: f64) -> f64 {
        if let Scale::Piecewise(s) = &self.scale {
            return s.map_transformed(value);
        }
        // Reciprocal clipping coordinates increase with source values. Restore
        // D3's raw decreasing transform before bimap, preserving its arithmetic
        // order at rounding boundaries and the sign of zero at singularities.
        let direction = if let ScaleKind::Pow { exponent } = self.kind {
            exponent.signum()
        } else {
            1.
        };
        let domain = self.domain.map(|v| self.transformed(v) * direction);
        let value = value * direction;
        let out = crate::scale_options::interpolate(domain, self.range, value, self.is_clamped());
        if self.is_rounded() {
            crate::scale_options::js_round(out)
        } else {
            out
        }
    }
    pub fn transformed(&self, value: f64) -> f64 {
        if let Scale::Piecewise(s) = &self.scale {
            return s.transformed(value);
        }
        match self.kind {
            ScaleKind::Log => value.abs().ln() * self.domain[0].signum(),
            ScaleKind::SymLog { constant } => crate::scales::symlog_transform(value, constant),
            ScaleKind::Pow { exponent } => crate::scales::power_transform(value, exponent),
            ScaleKind::Sqrt => crate::scales::power_transform(value, 0.5),
            _ => value,
        }
    }
    pub fn map(&self, value: f64) -> f64 {
        if let Scale::Piecewise(s) = &self.scale {
            return s.map(value);
        }
        if self.configured.is_some() {
            if !self.accepts(value) {
                return f64::NAN;
            }
            let value = if self.is_clamped() {
                value.clamp(
                    self.domain[0].min(self.domain[1]),
                    self.domain[0].max(self.domain[1]),
                )
            } else {
                value
            };
            return self.map_transformed(self.transformed(value));
        }
        let value = match &self.scale {
            Scale::Linear(s) => s.map(value),
            Scale::Log(s) => s.map(value),
            Scale::SymLog(s) => s.map(value),
            Scale::Pow(s) => s.map(value),
            Scale::Piecewise(s) => s.map(value),
            Scale::Utc(s) => {
                if value.fract() != 0. || value.abs() > MAX_TIMESTAMP_MS as f64 {
                    f64::NAN
                } else {
                    s.map(value as i64)
                }
            }
        };
        if self.configured.is_some_and(|o| o.round) {
            crate::scale_options::js_round(value)
        } else {
            value
        }
    }
    pub fn invert(&self, pixel: f64) -> Result<f64> {
        ensure(pixel.is_finite(), "invalid inverse coordinate")?;
        if let Scale::Piecewise(s) = &self.scale {
            return s.invert(pixel);
        }
        if self.configured.is_some() {
            let direction = if let ScaleKind::Pow { exponent } = self.kind {
                exponent.signum()
            } else {
                1.
            };
            let v = crate::scale_options::interpolate(
                self.range,
                self.domain.map(|x| self.transformed(x) * direction),
                pixel,
                false,
            );
            let mut value = match self.kind {
                ScaleKind::Log => self.domain[0].signum() * (self.domain[0].signum() * v).exp(),
                ScaleKind::SymLog { constant } => crate::scales::symlog_inverse(v, constant),
                ScaleKind::Pow { exponent } => {
                    crate::scales::power_inverse(v * direction, exponent)
                }
                ScaleKind::Sqrt => crate::scales::power_inverse(v, 0.5),
                _ => v,
            };
            if self.is_clamped() {
                value = value.clamp(
                    self.domain[0].min(self.domain[1]),
                    self.domain[0].max(self.domain[1]),
                );
            }
            if self.kind == ScaleKind::Utc {
                value = value.trunc();
                if value == 0. {
                    value = 0.;
                }
                ensure(
                    value.abs() <= MAX_TIMESTAMP_MS as f64,
                    "UTC inverse out of range",
                )?;
            }
            ensure(value.is_finite(), "inverse scale overflow")?;
            return Ok(value);
        }
        let value = match &self.scale {
            Scale::Linear(s) => s.invert(pixel),
            Scale::Log(s) => s.invert(pixel),
            Scale::SymLog(s) => s.invert(pixel),
            Scale::Pow(s) => s.invert(pixel),
            Scale::Piecewise(s) => s.invert(pixel)?,
            Scale::Utc(s) => s.invert(pixel)? as f64,
        };
        ensure(value.is_finite(), "inverse scale overflow")?;
        Ok(value)
    }
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        if let Some(o) = self.configured {
            return crate::scale_options::configured_ticks(
                self.kind,
                self.domain,
                count,
                o.log_base,
            );
        }
        match &self.scale {
            Scale::Linear(s) => s.ticks(count),
            Scale::Log(s) => s.ticks(count),
            Scale::SymLog(s) => s.ticks(count),
            Scale::Pow(s) => s.ticks(count),
            Scale::Piecewise(_) => unreachable!("piecewise scales are configured"),
            Scale::Utc(s) => s
                .ticks(count)
                .into_iter()
                .map(|t| t.timestamp_ms as f64)
                .collect(),
        }
    }
    pub fn accepts(&self, value: f64) -> bool {
        if let Scale::Piecewise(s) = &self.scale {
            return s.accepts(value);
        }
        if let Scale::Pow(s) = &self.scale {
            return s.accepts(value);
        }
        value.is_finite()
            && match self.kind {
                ScaleKind::Log => {
                    (self.configured.is_some() && self.is_clamped())
                        || (value != 0. && value.signum() == self.domain[0].signum())
                }
                ScaleKind::Utc => value.fract() == 0. && value.abs() <= MAX_TIMESTAMP_MS as f64,
                _ => true,
            }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisSide {
    Top,
    Right,
    Bottom,
    Left,
}

pub(crate) fn calendar_axis_ticks(
    scale: &ChartScale,
    options: &AxisOptions,
) -> Result<Option<Vec<crate::TimeTick>>> {
    let Some(calendar) = &options.tick_calendar else {
        return Ok(None);
    };
    ensure(
        scale.kind == ScaleKind::Utc,
        "calendar ticks require a UTC epoch-millisecond scale",
    )?;
    let ticks = crate::zoned_time::ZonedTimeScale::new(
        scale.domain.map(|value| value as i64),
        scale.range,
        calendar.clone(),
    )?
    .ticks(options.tick_count)?;
    ensure(ticks.len() <= 1000, "calendar axis tick budget exceeded")?;
    Ok(Some(ticks))
}

/// Append an axis to a checked proposal; invalid input leaves the scene intact.
pub fn axis(
    scene: &mut Scene,
    c: &ChartConfig,
    scale: &ChartScale,
    side: AxisSide,
    o: &AxisOptions,
    id: &str,
) -> Result<()> {
    c.validate()?;
    o.validate()?;
    ensure(!id.is_empty() && id.len() <= 256, "invalid axis ID")?;
    let mut next = scene.clone();
    let horizontal = matches!(side, AxisSide::Top | AxisSide::Bottom);
    let position = match side {
        AxisSide::Top => c.margin.top,
        AxisSide::Bottom => c.height - c.margin.bottom,
        AxisSide::Left => c.margin.left,
        AxisSide::Right => c.width - c.margin.right,
    };
    let direction = if matches!(side, AxisSide::Top | AxisSide::Left) {
        -1.
    } else {
        1.
    };
    let calendar_ticks = calendar_axis_ticks(scale, o)?;
    let ticks = calendar_ticks
        .as_ref()
        .map(|ticks| ticks.iter().map(|t| t.timestamp_ms as f64).collect())
        .unwrap_or_else(|| {
            o.tick_values
                .clone()
                .unwrap_or_else(|| scale.ticks(o.tick_count))
        });
    ensure(ticks.len() <= 2000, "axis automatic tick budget exceeded")?;
    let mut seen = std::collections::BTreeSet::new();
    for (i, value) in ticks.into_iter().enumerate() {
        let value = if value == 0. { 0. } else { value };
        ensure(scale.accepts(value), "tick is invalid for selected scale")?;
        if value < scale.domain[0].min(scale.domain[1])
            || value > scale.domain[0].max(scale.domain[1])
            || !seen.insert(value.to_bits())
        {
            continue;
        }
        let pixel = scale.map(value);
        ensure(pixel.is_finite(), "scaled tick overflow")?;
        if o.grid {
            let geometry = if horizontal {
                vec![
                    C::Move(P::new(pixel, c.margin.top)),
                    C::Line(P::new(pixel, c.height - c.margin.bottom)),
                ]
            } else {
                vec![
                    C::Move(P::new(c.margin.left, pixel)),
                    C::Line(P::new(c.width - c.margin.right, pixel)),
                ]
            };
            let mut mark = Mark::new(
                format!("{id}-grid-{i}"),
                Geometry::Path(geometry),
                c.theme.grid,
            );
            mark.fill = None;
            mark.label.clear();
            mark.stroke = Some(GraphicStroke {
                color: c.theme.grid,
                width: 1.,
                cap: StrokeCap::Butt,
            });
            next.marks.push(mark);
        }
        if o.visible {
            let geometry = if horizontal {
                vec![
                    C::Move(P::new(pixel, position)),
                    C::Line(P::new(pixel, position + direction * o.tick_size)),
                ]
            } else {
                vec![
                    C::Move(P::new(position, pixel)),
                    C::Line(P::new(position + direction * o.tick_size, pixel)),
                ]
            };
            let mut mark = Mark::new(
                format!("{id}-tick-{i}"),
                Geometry::Path(geometry),
                c.theme.text,
            );
            mark.fill = None;
            mark.label.clear();
            mark.stroke = Some(GraphicStroke {
                color: c.theme.text,
                width: 1.,
                cap: StrokeCap::Butt,
            });
            next.marks.push(mark);
            let format = if scale.kind == ScaleKind::Utc && o.format == ValueFormat::Auto {
                &ValueFormat::Utc
            } else {
                &o.format
            };
            let text = if o.format == ValueFormat::Auto {
                calendar_ticks
                    .as_ref()
                    .and_then(|ticks| ticks.iter().find(|tick| tick.timestamp_ms as f64 == value))
                    .map(|tick| Ok(tick.label.clone()))
                    .unwrap_or_else(|| format.format(value))?
            } else {
                format.format(value)?
            };
            let (x, y, anchor) = if horizontal {
                (
                    pixel,
                    position
                        + direction * (o.tick_size + o.tick_padding)
                        + if direction > 0. {
                            c.theme.font_size
                        } else {
                            0.
                        },
                    TextAnchor::Middle,
                )
            } else {
                (
                    position + direction * (o.tick_size + o.tick_padding),
                    pixel + c.theme.font_size / 3.,
                    if direction < 0. {
                        TextAnchor::End
                    } else {
                        TextAnchor::Start
                    },
                )
            };
            next.labels.push(Label {
                rotation: o.tick_rotation,
                x,
                y,
                text,
                color: c.theme.text,
                size: c.theme.font_size,
                anchor,
            });
        }
    }
    if o.visible {
        let commands = if horizontal {
            vec![
                C::Move(P::new(scale.range[0], position)),
                C::Line(P::new(scale.range[1], position)),
            ]
        } else {
            vec![
                C::Move(P::new(position, scale.range[0])),
                C::Line(P::new(position, scale.range[1])),
            ]
        };
        let mut mark = Mark::new(id, Geometry::Path(commands), c.theme.text);
        mark.fill = None;
        mark.label.clear();
        mark.stroke = Some(GraphicStroke {
            color: c.theme.text,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        next.marks.push(mark);
        if let Some(text) = &o.legend {
            let (x, y, anchor) = if horizontal {
                (
                    (scale.range[0] + scale.range[1]) / 2.,
                    position + direction * (o.tick_size + o.tick_padding + c.theme.font_size * 2.5),
                    TextAnchor::Middle,
                )
            } else {
                (
                    position,
                    scale.range[1] - c.theme.font_size,
                    TextAnchor::Middle,
                )
            };
            next.labels.push(Label {
                rotation: 0.,
                x,
                y,
                text: text.clone(),
                color: c.theme.text,
                size: c.theme.font_size,
                anchor,
            });
        }
    }
    if !next.draw_order.is_empty() {
        next.draw_order
            .extend((scene.marks.len()..next.marks.len()).map(crate::DrawItem::Mark));
        next.draw_order
            .extend((scene.labels.len()..next.labels.len()).map(crate::DrawItem::Label));
    }
    next.validate()?;
    *scene = next;
    Ok(())
}
pub(crate) fn cartesian_axes(
    scene: &mut Scene,
    c: &ChartConfig,
    x: &ChartScale,
    y: &ChartScale,
    o: &CartesianAxes,
) -> Result<()> {
    o.validate()?;
    axis(scene, c, x, o.x_side, &o.x, "axis-x")?;
    axis(scene, c, y, o.y_side, &o.y, "axis-y")
}
