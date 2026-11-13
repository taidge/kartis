//! Checked D3-style signed power scales, including same-branch reciprocals.
use super::{LinearScale, ticks};
use crate::Result;
use crate::scene::ensure;

pub(crate) fn validate_power_exponent(exponent: f64) -> Result<()> {
    ensure(
        exponent.is_finite() && exponent != 0. && exponent.recip().is_finite(),
        "power exponent must be finite, nonzero and have a finite reciprocal",
    )
}

fn signed_power(value: f64, exponent: f64) -> f64 {
    if exponent == 1. {
        value
    } else if exponent == 0.5 {
        if value < 0. {
            -(-value).sqrt()
        } else {
            value.sqrt()
        }
    } else if exponent == 2. {
        if value < 0. {
            -(value * value)
        } else {
            value * value
        }
    } else if value < 0. {
        -(-value).powf(exponent)
    } else {
        value.powf(exponent)
    }
}

/// Normalize a reciprocal's decreasing D3 transform to increasing coordinates.
/// Multiplying both transformed endpoints by -1 leaves the scale unchanged,
/// and lets clipping, interval layout and viewport windows share one contract.
pub(crate) fn power_transform(value: f64, exponent: f64) -> f64 {
    signed_power(value, exponent) * exponent.signum()
}
pub(crate) fn power_inverse(value: f64, exponent: f64) -> f64 {
    signed_power(value * exponent.signum(), exponent.recip())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PowScale {
    domain: [f64; 2],
    range: [f64; 2],
    exponent: f64,
    transformed: [f64; 2],
    clamp: bool,
}
impl PowScale {
    /// Negative exponents require a same-sign nonzero domain. A nonconstant
    /// domain must retain a distinct, finite transformed span. Zero exponents
    /// are noninvertible and reject. Endpoints can otherwise be descending.
    pub fn new(domain: [f64; 2], range: [f64; 2], exponent: f64) -> Result<Self> {
        validate_power_exponent(exponent)?;
        LinearScale::new(domain, range)?;
        ensure(
            exponent > 0.
                || domain[0] != 0. && domain[1] != 0. && domain[0].signum() == domain[1].signum(),
            "negative power domains must have one nonzero sign",
        )?;
        let transformed = domain.map(|value| power_transform(value, exponent));
        ensure(
            transformed.iter().all(|v| v.is_finite())
                && (transformed[1] - transformed[0]).is_finite()
                && (domain[0] == domain[1]
                    || transformed[0] != transformed[1]
                        && (domain[1] > domain[0]) == (transformed[1] > transformed[0])),
            "power domain transform overflow or collapse",
        )?;
        Ok(Self {
            domain,
            range,
            exponent,
            transformed,
            clamp: false,
        })
    }
    pub fn sqrt(domain: [f64; 2], range: [f64; 2]) -> Result<Self> {
        Self::new(domain, range, 0.5)
    }
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    pub fn range(&self) -> [f64; 2] {
        self.range
    }
    pub fn exponent(&self) -> f64 {
        self.exponent
    }
    pub fn is_clamped(&self) -> bool {
        self.clamp
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
    pub(crate) fn accepts(&self, value: f64) -> bool {
        value.is_finite()
            && (self.clamp
                || (self.exponent > 0. || value != 0. && value.signum() == self.domain[0].signum())
                    && power_transform(value, self.exponent).is_finite())
    }
    pub fn map(&self, value: f64) -> f64 {
        if !self.accepts(value) {
            return f64::NAN;
        }
        let value = if self.clamp {
            value.clamp(
                self.domain[0].min(self.domain[1]),
                self.domain[0].max(self.domain[1]),
            )
        } else {
            value
        };
        let direction = self.exponent.signum();
        crate::scale_options::interpolate(
            self.transformed.map(|v| v * direction),
            self.range,
            power_transform(value, self.exponent) * direction,
            self.clamp,
        )
    }
    pub fn invert(&self, pixel: f64) -> f64 {
        if !pixel.is_finite() {
            return f64::NAN;
        }
        let direction = self.exponent.signum();
        let transformed = crate::scale_options::interpolate(
            self.range,
            self.transformed.map(|v| v * direction),
            pixel,
            false,
        );
        let value = power_inverse(transformed * direction, self.exponent);
        if self.clamp {
            value.clamp(
                self.domain[0].min(self.domain[1]),
                self.domain[0].max(self.domain[1]),
            )
        } else {
            value
        }
    }
    /// D3 power scales use ordinary numeric ticks in the source value domain.
    pub fn ticks(&self, count: usize) -> Vec<f64> {
        ticks(self.domain, count)
    }
    pub fn nice(&self, count: usize) -> Result<Self> {
        let domain = LinearScale::new(self.domain, [0., 1.])?.nice(count).domain;
        Ok(Self::new(domain, self.range, self.exponent)?.clamped(self.clamp))
    }
}
