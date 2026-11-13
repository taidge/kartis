//! Checked numeric piecewise interpolation. Segment arithmetic follows the
//! existing continuous scales; breakpoints are selected by binary search.
use crate::axes::{ChartScale, ScaleKind};
use crate::scale_options::ContinuousScaleOptions;
use crate::scene::{Result, ensure};

/// A continuous scale with matching domain/range stops. Domains must be strictly
/// monotonic. Ranges may turn or contain plateaus: these remain useful for
/// forward mapping, but have no unique global inverse. Use `invert_segment`
/// when an application explicitly chooses a branch.
#[derive(Clone, Debug)]
pub struct PiecewiseScale {
    domain: Vec<f64>,
    range: Vec<f64>,
    transformed: Vec<f64>,
    segments: Vec<ChartScale>,
    invertible: bool,
    clamp: bool,
}

impl PiecewiseScale {
    /// Construct from final display stops. Nice and reverse are resolved by
    /// `ResolvedContinuousScale::piecewise`; this low-level constructor does
    /// not apply them a second time. Two to 4096 stops are supported.
    pub fn new(domain: &[f64], range: &[f64], options: &ContinuousScaleOptions) -> Result<Self> {
        options.validate()?;
        ensure(
            (2..=4096).contains(&domain.len()) && domain.len() == range.len(),
            "piecewise scales require 2..=4096 matching domain and range stops",
        )?;
        ensure(
            domain.iter().chain(range).all(|v| v.is_finite()),
            "piecewise scale stops must be finite",
        )?;
        ensure(
            strictly_monotonic(domain),
            "piecewise domain must be strictly monotonic",
        )?;
        let segments = domain
            .windows(2)
            .zip(range.windows(2))
            .map(|(d, r)| {
                ChartScale::configured_raw(options.kind, [d[0], d[1]], [r[0], r[1]], options)
            })
            .collect::<Result<Vec<_>>>()?;
        let transformed = domain
            .iter()
            .map(|&v| segments[0].transformed(v))
            .collect::<Vec<_>>();
        ensure(
            transformed.iter().all(|v| v.is_finite()) && strictly_monotonic(&transformed),
            "piecewise transformed domain overflow or collapse",
        )?;
        // Also validate global spans; downstream axes and clipping expose the
        // two outer endpoints and require representable finite differences.
        ChartScale::configured_raw(
            options.kind,
            [domain[0], domain[domain.len() - 1]],
            [range[0], range[range.len() - 1]],
            options,
        )?;
        Ok(Self {
            domain: domain.to_vec(),
            range: range.to_vec(),
            transformed,
            segments,
            invertible: strictly_monotonic(range),
            clamp: options.clamp,
        })
    }
    pub fn domain(&self) -> &[f64] {
        &self.domain
    }
    pub fn range(&self) -> &[f64] {
        &self.range
    }
    pub fn is_invertible(&self) -> bool {
        self.invertible
    }
    pub fn is_clamped(&self) -> bool {
        self.clamp
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self.segments = self
            .segments
            .into_iter()
            .map(|s| s.clamped(clamp))
            .collect();
        self
    }
    pub fn accepts(&self, value: f64) -> bool {
        self.segments[0].accepts(value)
    }
    pub fn map(&self, value: f64) -> f64 {
        if !self.accepts(value) {
            return f64::NAN;
        }
        self.segments[segment(&self.domain, value)].map(value)
    }
    pub(crate) fn map_transformed(&self, value: f64) -> f64 {
        self.segments[segment(&self.transformed, value)].map_transformed(value)
    }
    /// Inverse of the unrounded numeric interpolation. A turning or plateau
    /// range is rejected rather than adopting D3's arbitrary bisect result.
    pub fn invert(&self, pixel: f64) -> Result<f64> {
        ensure(
            self.invertible,
            "piecewise range has no unique inverse; choose a segment",
        )?;
        ensure(pixel.is_finite(), "invalid inverse coordinate")?;
        let i = segment(&self.range, pixel);
        let kind = self.segments[0].kind();
        let direction = if let ScaleKind::Pow { exponent } = kind {
            exponent.signum()
        } else {
            1.
        };
        let t = crate::scale_options::interpolate(
            [self.range[i], self.range[i + 1]],
            [
                self.transformed[i] * direction,
                self.transformed[i + 1] * direction,
            ],
            pixel,
            false,
        );
        let mut value = match kind {
            ScaleKind::Log => self.domain[0].signum() * (self.domain[0].signum() * t).exp(),
            ScaleKind::SymLog { constant } => super::symlog_inverse(t, constant),
            ScaleKind::Pow { exponent } => super::power_inverse(t * direction, exponent),
            ScaleKind::Sqrt => super::power_inverse(t, 0.5),
            _ => t,
        };
        // D3 clamps AFTER inverse transformation to the WHOLE domain. Clamping
        // pixels or delegating to a clamped last segment changes reciprocal
        // extrapolation across a singularity (which may return the first stop).
        if self.clamp {
            value = value.clamp(
                self.domain[0].min(self.domain[self.domain.len() - 1]),
                self.domain[0].max(self.domain[self.domain.len() - 1]),
            );
        }
        if kind == ScaleKind::Utc {
            value = value.trunc();
            ensure(
                value.abs() <= crate::MAX_TIMESTAMP_MS as f64,
                "UTC inverse out of range",
            )?;
            if value == 0. {
                value = 0.;
            }
        }
        ensure(value.is_finite(), "inverse scale overflow")?;
        Ok(value)
    }
    /// Explicit branch inverse, including extrapolation when unclamped. With
    /// clamp enabled, the result is bounded to this segment's domain. A flat
    /// segment has an interval of solutions and is rejected even here.
    pub fn invert_segment(&self, index: usize, pixel: f64) -> Result<f64> {
        ensure(
            index < self.segments.len(),
            "piecewise inverse segment out of bounds",
        )?;
        ensure(
            self.range[index] != self.range[index + 1],
            "flat piecewise segment has no unique inverse",
        )?;
        self.segments[index].invert(pixel)
    }
}

fn strictly_monotonic(values: &[f64]) -> bool {
    let ascending = values[1] > values[0];
    values
        .windows(2)
        .all(|v| if ascending { v[1] > v[0] } else { v[1] < v[0] })
}

fn segment(stops: &[f64], value: f64) -> usize {
    let ascending = stops[stops.len() - 1] > stops[0];
    let index = stops.partition_point(|&stop| {
        if ascending {
            stop <= value
        } else {
            stop >= value
        }
    });
    index.saturating_sub(1).min(stops.len() - 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transformed_clipping_uses_the_selected_segment_for_every_numeric_kind() {
        for (kind, domain) in [
            (ScaleKind::Linear, vec![-9., 1., 40.]),
            (ScaleKind::Log, vec![1., 10., 1000.]),
            (ScaleKind::Log, vec![-1000., -10., -1.]),
            (ScaleKind::SymLog { constant: 2. }, vec![-100., 0., 20.]),
            (ScaleKind::Pow { exponent: 2. }, vec![-9., 1., 40.]),
            (ScaleKind::Pow { exponent: -1. }, vec![1., 4., 100.]),
            (ScaleKind::Pow { exponent: -2. }, vec![-100., -4., -1.]),
            (ScaleKind::Sqrt, vec![-9., 1., 40.]),
            (ScaleKind::Utc, vec![0., 3600000., 90000000.]),
        ] {
            for descending in [false, true] {
                for clamp in [false, true] {
                    let mut domain = domain.clone();
                    if descending {
                        domain.reverse();
                    }
                    let s = PiecewiseScale::new(
                        &domain,
                        &[20., 80., 500.],
                        &ContinuousScaleOptions {
                            kind,
                            clamp,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                    let queries = [
                        domain[0],
                        domain[1],
                        domain[2],
                        (domain[0] + domain[1]) / 2.,
                        (domain[1] + domain[2]) / 2.,
                    ];
                    for value in queries {
                        let actual = s.map_transformed(s.segments[0].transformed(value));
                        assert_eq!(
                            actual,
                            s.map(value),
                            "{kind:?}, {value}, descending={descending}, clamp={clamp}"
                        );
                    }
                }
            }
        }
    }
}
