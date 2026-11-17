//! Checked numeric piecewise interpolation. Segment arithmetic follows the
//! D3 continuous scales; breakpoints are selected by binary search. Explicit
//! piecewise logarithmic primitives use fixed libm implementations, avoiding
//! host ln_1p coalescence at a discontinuity. General signed power uses browser
//! Math.pow in canvas/web WASM builds and the platform Rust powf path otherwise.
use crate::axes::{ChartScale, ScaleKind};
use crate::scale_options::ContinuousScaleOptions;
use crate::scene::{Result, ensure};

/// A numeric piecewise scale with matching domain/range stops. Domains are monotonic
/// and may repeat, including an entirely constant domain. Duplicate ties use
/// D3's reverse-then-bisectRight rule in native transformed coordinates. Ranges
/// may turn or contain plateaus, requiring an explicit `invert_segment` branch.
#[derive(Clone, Debug)]
pub struct PiecewiseScale {
    domain: Vec<f64>,
    range: Vec<f64>,
    transformed: Vec<f64>,
    native_transformed: Vec<f64>,
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
        ensure(monotonic(domain), "piecewise domain must be monotonic")?;
        // Clamp belongs to the whole raw domain before forward transformation.
        // Segment-local clamping would make a repeated outer knot act like an
        // endpoint plateau and select the wrong interpolation at a clamped tie.
        let unbounded = ContinuousScaleOptions {
            clamp: false,
            ..*options
        };
        let segments = domain
            .windows(2)
            .zip(range.windows(2))
            .map(|(d, r)| {
                ChartScale::configured_raw(options.kind, [d[0], d[1]], [r[0], r[1]], &unbounded)
            })
            .collect::<Result<Vec<_>>>()?;
        let transformed = domain
            .iter()
            .map(|&v| piecewise_transform(options.kind, domain[0], v))
            .collect::<Vec<_>>();
        ensure(
            transformed.iter().all(|v| v.is_finite())
                && monotonic(&transformed)
                && domain
                    .windows(2)
                    .zip(transformed.windows(2))
                    .all(|(raw, t)| raw[0] == raw[1] || t[0] != t[1]),
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
            native_transformed: transformed
                .iter()
                .map(|t| t * direction(options.kind))
                .collect(),
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
    /// Whether the strictly monotonic range selects a unique inverse branch.
    /// A repeated domain can still map several outputs to one input or jump;
    /// this property does not promise a bijection or a map/invert round trip.
    pub fn is_invertible(&self) -> bool {
        self.invertible
    }
    pub fn is_clamped(&self) -> bool {
        self.clamp
    }
    pub fn clamped(mut self, clamp: bool) -> Self {
        self.clamp = clamp;
        self
    }
    pub fn accepts(&self, value: f64) -> bool {
        self.segments[0].clone().clamped(self.clamp).accepts(value)
    }
    pub fn map(&self, value: f64) -> f64 {
        if !self.accepts(value) {
            return f64::NAN;
        }
        let value = if self.clamp {
            value.clamp(
                self.domain[0].min(self.domain[self.domain.len() - 1]),
                self.domain[0].max(self.domain[self.domain.len() - 1]),
            )
        } else {
            value
        };
        self.map_transformed(self.transformed(value))
    }
    /// The same numeric profile is used for every explicit piecewise scale,
    /// including strict domains produced when nice/viewport removes repetition.
    /// This does not promise identical transcendental bits in every runtime.
    pub(crate) fn transformed(&self, value: f64) -> f64 {
        piecewise_transform(self.segments[0].kind(), self.domain[0], value)
    }
    pub(crate) fn map_transformed(&self, value: f64) -> f64 {
        let value = if self.clamp {
            value.clamp(
                self.transformed[0].min(self.transformed[self.transformed.len() - 1]),
                self.transformed[0].max(self.transformed[self.transformed.len() - 1]),
            )
        } else {
            value
        };
        let index = segment(
            &self.native_transformed,
            value * direction(self.segments[0].kind()),
        );
        let output = crate::scale_options::interpolate(
            [
                self.native_transformed[index],
                self.native_transformed[index + 1],
            ],
            [self.range[index], self.range[index + 1]],
            value * direction(self.segments[0].kind()),
            false,
        );
        if self.segments[0].is_rounded() {
            crate::scale_options::js_round(output)
        } else {
            output
        }
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
        let t = crate::scale_options::interpolate(
            [self.range[i], self.range[i + 1]],
            [self.native_transformed[i], self.native_transformed[i + 1]],
            pixel,
            false,
        );
        let mut value = piecewise_inverse(kind, self.domain[0], t);
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
    /// range segment is rejected even here. Repeated domain knots can yield
    /// a constant algebraic inverse without promising a forward round trip.
    pub fn invert_segment(&self, index: usize, pixel: f64) -> Result<f64> {
        ensure(
            index < self.segments.len(),
            "piecewise inverse segment out of bounds",
        )?;
        ensure(
            self.range[index] != self.range[index + 1],
            "flat piecewise segment has no unique inverse",
        )?;
        ensure(pixel.is_finite(), "invalid inverse coordinate")?;
        let t = crate::scale_options::interpolate(
            [self.range[index], self.range[index + 1]],
            [
                self.native_transformed[index],
                self.native_transformed[index + 1],
            ],
            pixel,
            false,
        );
        let mut value = piecewise_inverse(self.segments[0].kind(), self.domain[0], t);
        if self.clamp {
            value = value.clamp(
                self.domain[index].min(self.domain[index + 1]),
                self.domain[index].max(self.domain[index + 1]),
            );
        }
        if self.segments[0].kind() == ScaleKind::Utc {
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
        -platform_power(-value, exponent)
    } else {
        platform_power(value, exponent)
    }
}
fn platform_power(value: f64, exponent: f64) -> f64 {
    // D3 uses the current JavaScript engine's Math.pow. Its result may differ
    // by an ULP from the standalone WASM libm, which is material at a repeated
    // transformed knot. Browser builds use their actual host primitive;
    // native and standalone core builds keep the platform's Rust powf path.
    #[cfg(all(target_arch = "wasm32", feature = "canvas"))]
    {
        #[wasm_bindgen::prelude::wasm_bindgen]
        extern "C" {
            #[wasm_bindgen(js_namespace = Math, js_name = pow)]
            fn browser_power(value: f64, exponent: f64) -> f64;
        }
        browser_power(value, exponent)
    }
    #[cfg(not(all(target_arch = "wasm32", feature = "canvas")))]
    value.powf(exponent)
}
fn piecewise_transform(kind: ScaleKind, origin: f64, value: f64) -> f64 {
    match kind {
        ScaleKind::Log => origin.signum() * libm::log(value.abs()),
        ScaleKind::SymLog { constant } => {
            let ratio = value.abs() / constant;
            value.signum()
                * if ratio.is_finite() {
                    libm::log1p(ratio)
                } else {
                    libm::log(value.abs()) - libm::log(constant)
                }
        }
        ScaleKind::Pow { exponent } => signed_power(value, exponent) * exponent.signum(),
        ScaleKind::Sqrt => signed_power(value, 0.5),
        _ => value,
    }
}
// Input is the native (not normalized reciprocal) transformed coordinate.
fn piecewise_inverse(kind: ScaleKind, origin: f64, value: f64) -> f64 {
    match kind {
        ScaleKind::Log => origin.signum() * libm::exp(origin.signum() * value),
        ScaleKind::SymLog { constant } => {
            let direct = libm::expm1(value.abs()) * constant;
            value.signum()
                * if direct.is_finite() {
                    direct
                } else {
                    libm::exp(value.abs() + libm::log(constant))
                }
        }
        ScaleKind::Pow { exponent } => signed_power(value, exponent.recip()),
        ScaleKind::Sqrt => signed_power(value, 2.),
        _ => value,
    }
}

fn direction(kind: ScaleKind) -> f64 {
    if let ScaleKind::Pow { exponent } = kind {
        exponent.signum()
    } else {
        1.
    }
}

fn monotonic(values: &[f64]) -> bool {
    values.windows(2).all(|v| v[1] >= v[0]) || values.windows(2).all(|v| v[1] <= v[0])
}

fn strictly_monotonic(values: &[f64]) -> bool {
    let ascending = values[1] > values[0];
    values
        .windows(2)
        .all(|v| if ascending { v[1] > v[0] } else { v[1] < v[0] })
}

fn segment(stops: &[f64], value: f64) -> usize {
    // D3 polymap first reverses both arrays if the transformed last stop is
    // below the first. Bisect right over the ascending view's interior stops;
    // using >= on the original descending array chooses the opposite tie.
    let reversed = stops[stops.len() - 1] < stops[0];
    let mut lo = 1;
    let mut hi = stops.len() - 1;
    while lo < hi {
        let middle = (lo + hi) / 2;
        let stop = if reversed {
            stops[stops.len() - 1 - middle]
        } else {
            stops[middle]
        };
        if value < stop {
            hi = middle;
        } else {
            lo = middle + 1;
        }
    }
    if reversed {
        stops.len() - 1 - lo
    } else {
        lo - 1
    }
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
                        let actual = s.map_transformed(s.transformed(value));
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

    #[test]
    fn repeated_and_constant_knots_use_native_transform_direction_and_whole_clamp() {
        for (kind, stops) in [
            (ScaleKind::Linear, vec![0., 1., 1., 2.]),
            (ScaleKind::Log, vec![1., 2., 2., 4.]),
            (ScaleKind::SymLog { constant: 2. }, vec![-2., 0., 0., 4.]),
            (ScaleKind::Pow { exponent: -1. }, vec![1., 2., 2., 4.]),
            (ScaleKind::Pow { exponent: -2. }, vec![-4., -2., -2., -1.]),
            (ScaleKind::Pow { exponent: 2. }, vec![-2., 0., 0., 4.]),
            (ScaleKind::Sqrt, vec![-2., 0., 0., 4.]),
            (ScaleKind::Utc, vec![0., 1000., 1000., 4000.]),
        ] {
            for reverse in [false, true] {
                for clamp in [false, true] {
                    for constant in [false, true] {
                        let mut domain = if constant {
                            vec![stops[1]; 4]
                        } else {
                            stops.clone()
                        };
                        if reverse {
                            domain.reverse();
                        }
                        let scale = PiecewiseScale::new(
                            &domain,
                            &[10., 20., 30., 40.],
                            &ContinuousScaleOptions {
                                kind,
                                clamp,
                                ..Default::default()
                            },
                        )
                        .unwrap();
                        for value in stops
                            .iter()
                            .copied()
                            .chain([domain[0] * 0.5, domain[3] * 2.])
                        {
                            if !scale.accepts(value) {
                                continue;
                            }
                            assert_eq!(
                                scale.map_transformed(scale.transformed(value)),
                                scale.map(value),
                                "{kind:?}/{domain:?}/{value}/{clamp}"
                            );
                        }
                    }
                }
            }
        }
    }
}
