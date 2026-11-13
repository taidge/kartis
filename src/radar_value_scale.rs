//! Numeric radius scales for the configured Radar layout.
//!
//! Nivo Radar uses a zero-based linear radius scale. The profiles here are
//! additive and use Kartis's D3-compatible continuous numeric scales.
use crate::scene::ensure;
use crate::{ContinuousScaleOptions, ResolvedContinuousScale, Result, ScaleKind, ScaleNice};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum RadarValueScaleKind {
    #[default]
    Linear,
    /// A same-sign, strictly nonzero domain. Base configures ticks and nice.
    Log {
        base: f64,
    },
    /// A signed logarithmic scale with a continuous neighborhood around zero.
    SymLog {
        constant: f64,
    },
    Pow {
        exponent: f64,
    },
    Sqrt,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadarGridTickMode {
    /// Divide the available radius evenly and label rings with inverse values.
    #[default]
    EqualRadius,
    /// Use numeric scale ticks. Logarithmic ticks can produce more than the
    /// requested count. Use explicit grid values to select a bounded subset.
    ScaleTicks,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RadarValueScaleOptions {
    pub kind: RadarValueScaleKind,
    pub nice: ScaleNice,
    /// Used when RadarGridOptions.values is None.
    pub grid_ticks: RadarGridTickMode,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RadarValueTick {
    pub value: f64,
    pub radius: f64,
}

/// The shared value-to-radius map used by the configured grid, series and
/// interaction sites. The display minimum is the center and the maximum is
/// the outer radius. Negative values always map outwards along their own
/// axis, including in a negative log domain. No value changes axis direction.
#[derive(Clone, Debug)]
pub struct RadarValueScale {
    inner: ResolvedContinuousScale,
}
impl RadarValueScale {
    pub(super) fn new(
        domain: [f64; 2],
        radius: f64,
        options: &RadarValueScaleOptions,
    ) -> Result<Self> {
        ensure(
            radius.is_finite() && radius > 0.,
            "radar scale requires a representable positive outer radius",
        )?;
        let (kind, log_base) = match options.kind {
            RadarValueScaleKind::Linear => (ScaleKind::Linear, 10.),
            RadarValueScaleKind::Log { base } => (ScaleKind::Log, base),
            RadarValueScaleKind::SymLog { constant } => (ScaleKind::SymLog { constant }, 10.),
            RadarValueScaleKind::Pow { exponent } => (ScaleKind::Pow { exponent }, 10.),
            RadarValueScaleKind::Sqrt => (ScaleKind::Sqrt, 10.),
        };
        let inner = ResolvedContinuousScale::new(
            domain,
            [0., radius],
            &ContinuousScaleOptions {
                kind,
                log_base,
                nice: options.nice,
                ..Default::default()
            },
        )?;
        let [lo, hi] = inner.display_domain();
        let chart = inner.chart_scale();
        let span = chart.transformed(hi) - chart.transformed(lo);
        ensure(
            lo <= domain[0] && hi >= domain[1] && span.is_finite() && span > 0.,
            "radar scale display domain must contain the source with a representable transformed span",
        )?;
        Ok(Self { inner })
    }

    pub fn source_domain(&self) -> [f64; 2] {
        self.inner.source_domain()
    }

    pub fn display_domain(&self) -> [f64; 2] {
        self.inner.display_domain()
    }

    pub fn outer_radius(&self) -> f64 {
        self.inner.range()[1]
    }

    /// Checked radius for a value within the display domain.
    pub fn radius_at(&self, value: f64) -> Result<f64> {
        let [lo, hi] = self.display_domain();
        ensure(
            value >= lo && value <= hi,
            "radar value must lie inside the scale display domain",
        )?;
        let radius = self.inner.map(value);
        ensure(
            (0. ..=self.outer_radius()).contains(&radius),
            "radar value has no representable radius",
        )?;
        Ok(radius)
    }

    /// Checked inverse within the plotted radial interval.
    pub fn value_at(&self, radius: f64) -> Result<f64> {
        let [lo, hi] = self.display_domain();
        let outer = self.outer_radius();
        ensure(
            (0. ..=outer).contains(&radius),
            "radar inverse radius must lie inside the plot",
        )?;
        if radius == 0. {
            return Ok(lo);
        }
        if radius == outer {
            return Ok(hi);
        }
        let value = self.inner.invert(radius)?;
        ensure(
            (lo..=hi).contains(&value),
            "radar inverse value is outside the scale display domain",
        )?;
        Ok(value)
    }

    /// Zero is absent from a log domain. In a signed domain it can be an
    /// interior ring, independent of the minimum at the center.
    pub fn zero_radius(&self) -> Option<f64> {
        self.radius_at(0.).ok()
    }

    pub(super) fn ticks(&self, mode: RadarGridTickMode, levels: usize) -> Result<Vec<f64>> {
        match mode {
            RadarGridTickMode::EqualRadius => (1..=levels)
                .map(|i| self.value_at(self.outer_radius() * (i as f64 / levels as f64)))
                .collect(),
            RadarGridTickMode::ScaleTicks => {
                let [lo, hi] = self.display_domain();
                let mut ticks = self.inner.ticks(levels);
                ticks.retain(|v| (lo..=hi).contains(v));
                ticks.dedup();
                Ok(ticks)
            }
        }
    }
}
