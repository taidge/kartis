//! Input transforms adapted from d3-scale 4.0.2 sequential.js, diverging.js,
//! log.js, pow.js and symlog.js (Copyright 2010–2021 Mike Bostock, ISC).
//! The complete notice is included in docs/third-party-notices.md.
use super::{ColorScaleKind, color_fraction};
use crate::Result;
use crate::scene::ensure;

/// Continuous input transforms, independent of RGB gamma or interpolation space.
/// Domains must remain finite and invertible. Log and negative powers require
/// one nonzero sign branch. Queries follow D3 before optional parameter clamping;
/// an unrepresentable final color parameter maps to the unknown color.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorScaleTransform {
    #[default]
    Linear,
    Log,
    Pow {
        exponent: f64,
    },
    Sqrt,
    SymLog {
        constant: f64,
    },
}
impl ColorScaleTransform {
    pub(super) fn apply(self, value: f64, first: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log if first < 0. => -(-value).ln(),
            Self::Log => value.ln(),
            Self::Pow { exponent } => {
                crate::scales::power_transform(value, exponent) * exponent.signum()
            }
            Self::Sqrt => crate::scales::power_transform(value, 0.5),
            Self::SymLog { constant } => value.signum() * (value.abs() / constant).ln_1p(),
        }
    }
    pub(super) fn inverse(self, value: f64, first: f64) -> f64 {
        match self {
            Self::Linear => value,
            Self::Log if first < 0. => -(-value).exp(),
            Self::Log => value.exp(),
            Self::Pow { exponent } => {
                crate::scales::power_inverse(value * exponent.signum(), exponent)
            }
            Self::Sqrt => crate::scales::power_inverse(value, 0.5),
            Self::SymLog { constant } => value.signum() * value.abs().exp_m1() * constant,
        }
    }
    pub(super) fn validate(self, domain: [f64; 2], kind: ColorScaleKind) -> Result<()> {
        if self == Self::Linear {
            return Ok(());
        }
        if let Self::Pow { exponent } = self {
            crate::scales::validate_power_exponent(exponent)?;
        }
        if let Self::SymLog { constant } = self {
            ensure(
                constant.is_finite() && constant > 0.,
                "color symlog constant must be finite and positive",
            )?;
        }
        let branch = self == Self::Log || matches!(self, Self::Pow { exponent } if exponent < 0.);
        ensure(
            !branch
                || (domain[0] != 0. && domain[1] != 0. && domain[0].signum() == domain[1].signum()),
            "log and negative-power color domains require one nonzero sign branch",
        )?;
        let [a, b] = domain.map(|v| self.apply(v, domain[0]));
        ensure(
            a.is_finite()
                && b.is_finite()
                && (b - a).is_finite()
                && (domain[0] == domain[1] || a != b && (b - a).recip().is_finite()),
            "color input transform collapses or overflows its domain",
        )?;
        if let ColorScaleKind::Diverging { midpoint } = kind {
            let m = self.apply(midpoint, domain[0]);
            ensure(
                m.is_finite()
                    && (midpoint == domain[0] || m != a)
                    && (midpoint == domain[1] || m != b)
                    && (m - a).is_finite()
                    && (b - m).is_finite()
                    && (m == a || (0.5 / (m - a)).is_finite())
                    && (m == b || (0.5 / (b - m)).is_finite()),
                "color midpoint transform collapses or overflows",
            )?;
        }
        Ok(())
    }
    pub(super) fn fraction(self, value: f64, a: f64, b: f64, first: f64) -> f64 {
        let (a, b) = (self.apply(a, first), self.apply(b, first));
        if a == b {
            return 0.5;
        }
        let value = self.apply(value, first);
        let span = b - a;
        // Preserve D3's reciprocal multiplication at ordinary round boundaries.
        if span.is_finite() && span.recip().is_finite() {
            (value - a) * span.recip()
        } else {
            color_fraction(value, a, b)
        }
    }
    pub(super) fn parameter(self, value: f64, domain: [f64; 2], kind: ColorScaleKind) -> f64 {
        let [a, b] = domain;
        if let ColorScaleKind::Diverging { midpoint } = kind {
            let (a, m, b, x) = (
                self.apply(a, a),
                self.apply(midpoint, a),
                self.apply(b, a),
                self.apply(value, a),
            );
            let sign = if m < a { -1. } else { 1. };
            let (start, end) = if sign * x < sign * m { (a, m) } else { (m, b) };
            let k = if start == end {
                0.
            } else {
                0.5 / (end - start)
            };
            0.5 + (x - m) * k
        } else {
            self.fraction(value, a, b, a)
        }
    }
}

impl super::ColorScale {
    pub(super) fn nonlinear_stops(&self, steps: usize) -> Result<Vec<super::ColorScaleStop>> {
        // D3 chooses its diverging side from t1 < t0, even when t1 == t0.
        // A first-node midpoint with a descending transform therefore maps
        // the entire interior to 0.5. Preserve that behavior in lookup ink too.
        if let ColorScaleKind::Diverging { midpoint } = self.kind
            && midpoint == self.domain[0]
            && self.transform.apply(self.domain[1], self.domain[0])
                < self.transform.apply(self.domain[0], self.domain[0])
        {
            let color = self.map(Some(midpoint));
            return Ok(vec![
                super::ColorScaleStop {
                    offset: 0.,
                    value: self.domain[0],
                    color,
                },
                super::ColorScaleStop {
                    offset: 1.,
                    value: self.domain[1],
                    color,
                },
            ]);
        }
        let middle = match self.kind {
            ColorScaleKind::Diverging { midpoint } => self.position(midpoint),
            _ => None,
        };
        let axis_offset = |t: f64| match middle {
            Some(m) if t <= 0.5 => super::color_value(0., m, 2. * t),
            Some(m) => super::color_value(m, 1., 2. * t - 1.),
            None => t,
        };
        let value_at = |offset: f64| {
            if let ColorScaleKind::Diverging { midpoint } = self.kind
                && middle == Some(offset)
            {
                return Ok(midpoint);
            }
            self.value_at(offset)
        };
        if let super::ResolvedColorProfile::Scheme(scheme) = self.profile
            && let Some(len) = scheme.lookup_len()
        {
            let mut stops = Vec::with_capacity(2 * len + 1);
            for index in 0..len {
                let left = axis_offset(index as f64 / len as f64);
                let right = axis_offset((index + 1) as f64 / len as f64);
                if left < right {
                    let color = scheme.sample_unit((index as f64 + 0.5) / len as f64);
                    for offset in [left, right] {
                        stops.push(super::ColorScaleStop {
                            offset,
                            value: value_at(offset)?,
                            color,
                        });
                    }
                }
            }
            if middle == Some(1.) {
                stops.push(super::ColorScaleStop {
                    offset: 1.,
                    value: self.domain[1],
                    color: scheme.sample_unit(0.5),
                });
            }
            return Ok(stops);
        }
        let mut offsets: Vec<_> = (0..steps).map(|i| i as f64 / (steps - 1) as f64).collect();
        if matches!(self.kind, ColorScaleKind::Sequential) {
            offsets
                .extend((0..self.colors.len()).map(|i| i as f64 / (self.colors.len() - 1) as f64));
        } else if let Some(middle) = middle {
            offsets.push(middle);
        }
        offsets.sort_by(f64::total_cmp);
        offsets.dedup();
        offsets
            .into_iter()
            .map(|offset| {
                let value = value_at(offset)?;
                Ok(super::ColorScaleStop {
                    offset,
                    value,
                    color: self.map(Some(value)),
                })
            })
            .collect()
    }
}
