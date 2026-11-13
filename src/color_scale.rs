//! Validated data-to-color scales, independent of renderer fill paints.
//!
//! Continuous interpolation uses piecewise, unpremultiplied RGBA8 by default.
//! Other spaces share the same domains and authored palette anchors.
//! Quantize boundaries use right-bisect, as d3-scale/quantize.js;
//! equality belongs to the following bin. Descending domains reverse the
//! direction explicitly. Non-finite queries and missing values use `unknown`.
use crate::scene::ensure;
use crate::{Color, Result};

mod interpolation;
mod schemes;
mod sequential_quantile;
mod transform;
use interpolation::PreparedInterpolation;
pub use interpolation::{ColorInterpolation, HueInterpolation};
pub use schemes::ColorScheme;
pub use transform::ColorScaleTransform;

/// The source of colors for a resolved continuous scale.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorScaleProfile {
    Interpolate(ColorInterpolation),
    Scheme(ColorScheme),
}

impl From<ColorInterpolation> for ColorScaleProfile {
    fn from(profile: ColorInterpolation) -> Self {
        Self::Interpolate(profile)
    }
}

impl From<ColorScheme> for ColorScaleProfile {
    fn from(scheme: ColorScheme) -> Self {
        Self::Scheme(scheme)
    }
}

#[derive(Clone, Debug, PartialEq)]
enum ResolvedColorProfile {
    Interpolate(PreparedInterpolation),
    Scheme(ColorScheme),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorScaleKind {
    #[default]
    Sequential,
    /// Empirical right-ranks mapped through a continuous color profile.
    SequentialQuantile,
    /// Explicit data-space center, mapped to the middle of three colors.
    Diverging {
        midpoint: f64,
    },
    Quantize,
    /// R7 sample quantiles; missing observations do not enter the distribution.
    Quantile,
    /// Explicit ascending breaks, with unbounded first and last buckets.
    Threshold,
}
impl ColorScaleKind {
    pub fn is_discrete(self) -> bool {
        matches!(self, Self::Quantize | Self::Quantile | Self::Threshold)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorScaleOptions {
    pub kind: ColorScaleKind,
    /// Data-to-color input transform. Discrete and SequentialQuantile scales require Linear.
    pub transform: ColorScaleTransform,
    /// None infers the finite extent; empty/all-missing input uses [0, 1],
    /// or the first/last authored threshold when present. For Quantile,
    /// SequentialQuantile and Threshold this is a finite legend window,
    /// independent of the full distribution or authored breaks. Those kinds
    /// require ascending endpoints.
    /// A constant extent is retained, rather than expanded arbitrarily.
    pub domain: Option<[f64; 2]>,
    /// Sequential/SequentialQuantile: 2..=256 palette anchors; Diverging: exactly 3;
    /// Quantize/Quantile: 1..=256 bins. Threshold: breaks.len() + 1 colors.
    /// No hidden palette/scheme or nice-domain changes.
    pub colors: Vec<Color>,
    pub unknown: Color,
    /// Continuous scales extrapolate saturated RGBA8 when false, except
    /// SequentialQuantile: like discrete scales, it returns unknown outside
    /// the finite legend window. True preserves saturating empirical ranks
    /// and D3's Quantile/Threshold behavior for finite data.
    pub clamp: bool,
    /// Threshold only: ascending finite breaks (duplicates retain empty bins).
    /// Exactly one more color than breaks is required. Other kinds require [].
    pub thresholds: Vec<f64>,
}
impl Default for ColorScaleOptions {
    fn default() -> Self {
        Self {
            kind: ColorScaleKind::Sequential,
            transform: ColorScaleTransform::Linear,
            domain: None,
            colors: vec![Color(235, 244, 251, 255), Color(31, 99, 171, 255)],
            unknown: Color(238, 238, 238, 255),
            clamp: true,
            thresholds: Vec::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ColorScale {
    kind: ColorScaleKind,
    transform: ColorScaleTransform,
    domain: [f64; 2],
    colors: Vec<Color>,
    unknown: Color,
    clamp: bool,
    thresholds: Vec<f64>,
    profile: ResolvedColorProfile,
    sample_extent: Option<[f64; 2]>,
    samples: Option<std::sync::Arc<[f64]>>,
}
/// An inverse bucket interval. None denotes an unbounded threshold tail.
/// Adjacent repeated breaks describe an empty interval. Endpoints describe
/// the partition; equality at an internal break belongs to the following bin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScaleExtent {
    pub start: Option<f64>,
    pub end: Option<f64>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScaleStop {
    pub offset: f64,
    pub value: f64,
    pub color: Color,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorScaleBin {
    pub start: f64,
    pub end: f64,
    pub color: Color,
}
// Avoid overflow for extents such as [-f64::MAX, f64::MAX].
pub(crate) fn color_fraction(value: f64, a: f64, b: f64) -> f64 {
    if a == b {
        return 0.5;
    }
    if value == a {
        return 0.;
    }
    if value == b {
        return 1.;
    }
    let span = b - a;
    let delta = value - a;
    if span.is_finite() && delta.is_finite() {
        delta / span
    } else {
        (value / 2. - a / 2.) / (b / 2. - a / 2.)
    }
}
pub(crate) fn color_value(a: f64, b: f64, t: f64) -> f64 {
    if a == b || t == 0. {
        a
    } else if t == 1. {
        b
    } else if (b - a).is_finite() {
        a + (b - a) * t
    } else {
        a * (1. - t) + b * t
    }
}
fn interpolate(a: Color, b: Color, t: f64) -> Color {
    let channel = |a: u8, b: u8| {
        if a == b {
            a
        } else {
            (f64::from(a) + (f64::from(b) - f64::from(a)) * t)
                .round()
                .clamp(0., 255.) as u8
        }
    };
    Color(
        channel(a.0, b.0),
        channel(a.1, b.1),
        channel(a.2, b.2),
        channel(a.3, b.3),
    )
}
impl ColorScaleOptions {
    pub fn resolve(&self, values: impl IntoIterator<Item = Option<f64>>) -> Result<ColorScale> {
        self.resolve_with_interpolation(values, ColorInterpolation::Srgb)
    }

    /// Resolve the domain and piecewise color interpolation together.
    /// Quantized scales use discrete bins and accept the default profile.
    pub fn resolve_with_interpolation(
        &self,
        values: impl IntoIterator<Item = Option<f64>>,
        profile: ColorInterpolation,
    ) -> Result<ColorScale> {
        self.resolve_with_profile(values, profile)
    }

    /// Use either an authored palette with an interpolation space or a named
    /// Nivo/D3 ramp. A named ramp supplies its own colors and requires a
    /// clamped continuous scale. For discrete bins, supply `scheme.palette(n)`.
    pub fn resolve_with_profile(
        &self,
        values: impl IntoIterator<Item = Option<f64>>,
        profile: impl Into<ColorScaleProfile>,
    ) -> Result<ColorScale> {
        ensure(
            self.thresholds.len() <= 255
                && self.thresholds.iter().all(|v| v.is_finite())
                && self.thresholds.windows(2).all(|w| w[0] <= w[1]),
            "color-scale breaks must be ascending, finite and at most 255",
        )?;
        ensure(
            self.kind == ColorScaleKind::Threshold || self.thresholds.is_empty(),
            "explicit breaks require a threshold color scale",
        )?;
        ensure(
            !self.kind.is_discrete() || self.transform == ColorScaleTransform::Linear,
            "discrete color scales require a linear input transform",
        )?;
        ensure(
            self.kind != ColorScaleKind::SequentialQuantile
                || self.transform == ColorScaleTransform::Linear,
            "sequential quantile colors require a linear data axis",
        )?;
        let (colors, profile) = match profile.into() {
            ColorScaleProfile::Interpolate(profile) => {
                let valid_palette = match self.kind {
                    ColorScaleKind::Sequential | ColorScaleKind::SequentialQuantile => {
                        (2..=256).contains(&self.colors.len())
                    }
                    ColorScaleKind::Diverging { .. } => self.colors.len() == 3,
                    ColorScaleKind::Quantize | ColorScaleKind::Quantile => {
                        (1..=256).contains(&self.colors.len())
                    }
                    ColorScaleKind::Threshold => self.colors.len() == self.thresholds.len() + 1,
                };
                ensure(valid_palette, "invalid color-scale palette length")?;
                ensure(
                    !self.kind.is_discrete() || profile == ColorInterpolation::Srgb,
                    "discrete colors do not interpolate between bins",
                )?;
                let prepared = PreparedInterpolation::new(profile, &self.colors)?;
                (
                    self.colors.clone(),
                    ResolvedColorProfile::Interpolate(prepared),
                )
            }
            ColorScaleProfile::Scheme(scheme) => {
                ensure(
                    self.clamp && !self.kind.is_discrete(),
                    "named color ramps require a clamped continuous scale",
                )?;
                let mut colors = vec![scheme.sample_unit(0.)];
                if matches!(self.kind, ColorScaleKind::Diverging { .. }) {
                    colors.push(scheme.sample_unit(0.5));
                }
                colors.push(scheme.sample_unit(1.));
                (colors, ResolvedColorProfile::Scheme(scheme))
            }
        };
        let (mut minimum, mut maximum) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut samples = Vec::new();
        for (index, value) in values.into_iter().enumerate() {
            ensure(
                index < crate::scene::MAX_MARKS,
                "color-scale data budget exceeded",
            )?;
            if let Some(value) = value {
                ensure(
                    value.is_finite(),
                    "color-scale source values must be finite",
                )?;
                minimum = minimum.min(value);
                maximum = maximum.max(value);
                if matches!(
                    self.kind,
                    ColorScaleKind::Quantile | ColorScaleKind::SequentialQuantile
                ) {
                    samples.push(value);
                }
            }
        }
        let domain = self.domain.unwrap_or(if minimum.is_finite() {
            [minimum, maximum]
        } else if let (Some(first), Some(last)) = (self.thresholds.first(), self.thresholds.last())
        {
            [*first, *last]
        } else {
            [0., 1.]
        });
        ensure(
            domain.iter().all(|v| v.is_finite()),
            "color-scale domain must be finite",
        )?;
        ensure(
            !matches!(
                self.kind,
                ColorScaleKind::Quantile
                    | ColorScaleKind::SequentialQuantile
                    | ColorScaleKind::Threshold
            ) || domain[0] <= domain[1],
            "quantile and threshold legend domains must be ascending",
        )?;
        let sample_extent = (matches!(
            self.kind,
            ColorScaleKind::Quantile | ColorScaleKind::SequentialQuantile
        ) && minimum.is_finite())
        .then_some([minimum, maximum]);
        if let ColorScaleKind::Diverging { midpoint } = self.kind {
            ensure(
                midpoint.is_finite()
                    && midpoint >= domain[0].min(domain[1])
                    && midpoint <= domain[0].max(domain[1]),
                "diverging midpoint is outside its domain",
            )?;
        }
        self.transform.validate(domain, self.kind)?;
        let mut thresholds: Vec<f64> = if self.kind == ColorScaleKind::Quantize {
            (1..colors.len())
                .map(|i| {
                    let count = colors.len() as f64;
                    let index = i as f64;
                    // Preserve D3's ordinary finite rounding at bin boundaries.
                    // Equivalent lerp forms can differ by one ulp, changing a
                    // right-bisect result immediately below a true threshold.
                    let numerator = index * domain[1] - (index - count) * domain[0];
                    if numerator.is_finite() {
                        numerator / count
                    } else {
                        color_value(domain[0], domain[1], index / count)
                    }
                })
                .collect()
        } else if self.kind == ColorScaleKind::Threshold {
            self.thresholds.clone()
        } else if self.kind == ColorScaleKind::Quantile && !samples.is_empty() {
            samples.sort_by(f64::total_cmp);
            (1..colors.len())
                .map(|i| {
                    let index = (samples.len() - 1) as f64 * (i as f64 / colors.len() as f64);
                    let lower = index.floor() as usize;
                    color_value(
                        samples[lower],
                        samples[(lower + 1).min(samples.len() - 1)],
                        index - lower as f64,
                    )
                })
                .collect()
        } else {
            Vec::new()
        };
        if self.kind == ColorScaleKind::Quantize
            && (thresholds.iter().any(|v| {
                !v.is_finite() || *v < domain[0].min(domain[1]) || *v > domain[0].max(domain[1])
            }) || thresholds.windows(2).any(|v| {
                if domain[0] <= domain[1] {
                    v[0] > v[1]
                } else {
                    v[0] < v[1]
                }
            }))
        {
            // Weighted threshold products can lose their sorted order in a
            // domain only a few ulps wide. Bisect requires a true partition;
            // retain the finite domain with monotone stable interpolation.
            thresholds = (1..colors.len())
                .map(|i| color_value(domain[0], domain[1], i as f64 / colors.len() as f64))
                .collect();
        }
        let samples = if self.kind == ColorScaleKind::SequentialQuantile {
            // Stable numeric ordering retains the original order of equal ±0.
            samples.sort_by(|a, b| a.partial_cmp(b).expect("finite samples"));
            Some(samples.into())
        } else {
            None
        };
        Ok(ColorScale {
            kind: self.kind,
            transform: self.transform,
            domain,
            colors,
            unknown: self.unknown,
            clamp: self.clamp,
            thresholds,
            profile,
            sample_extent,
            samples,
        })
    }
}
impl ColorScale {
    pub fn domain(&self) -> [f64; 2] {
        self.domain
    }
    pub fn transform(&self) -> ColorScaleTransform {
        self.transform
    }
    /// Position on the transformed data axis, independent of a diverging
    /// palette's midpoint and without clamping. Non-finite results return None.
    pub fn position(&self, value: f64) -> Option<f64> {
        if !value.is_finite() {
            return None;
        }
        let [a, b] = self.domain;
        let offset = if self.transform == ColorScaleTransform::Linear {
            color_fraction(value, a, b)
        } else {
            self.transform.fraction(value, a, b, a)
        };
        offset.is_finite().then_some(offset)
    }
    /// Sample the data axis at a finite position in [0, 1]. This is an axis
    /// inverse, not an inverse of rounded/interpolated colors.
    pub fn value_at(&self, offset: f64) -> Result<f64> {
        ensure(
            offset.is_finite() && (0. ..=1.).contains(&offset),
            "invalid color-axis offset",
        )?;
        let [a, b] = self.domain;
        let value = if offset == 0. || a == b {
            a
        } else if offset == 1. {
            b
        } else if self.transform == ColorScaleTransform::Linear {
            color_value(a, b, offset)
        } else {
            self.transform.inverse(
                color_value(
                    self.transform.apply(a, a),
                    self.transform.apply(b, a),
                    offset,
                ),
                a,
            )
        };
        ensure(
            value.is_finite() && (a.min(b)..=a.max(b)).contains(&value),
            "color-axis inverse is outside its representable domain",
        )?;
        Ok(value)
    }
    pub fn kind(&self) -> ColorScaleKind {
        self.kind
    }
    /// Authored palette, or the two endpoints and optional midpoint of a named ramp.
    pub fn palette(&self) -> &[Color] {
        &self.colors
    }
    pub fn profile(&self) -> ColorScaleProfile {
        match &self.profile {
            ResolvedColorProfile::Interpolate(interpolation) => interpolation.profile.into(),
            ResolvedColorProfile::Scheme(scheme) => (*scheme).into(),
        }
    }
    pub fn unknown(&self) -> Color {
        self.unknown
    }
    pub fn clamped(&self) -> bool {
        self.clamp
    }
    /// Boundaries in domain direction (always ascending for Quantile/Threshold).
    /// Equality moves forward. An empty quantile distribution has no cuts and
    /// maps every value to unknown, rather than inventing a distribution.
    pub fn thresholds(&self) -> &[f64] {
        &self.thresholds
    }
    /// Quantile's observed extent, independently of the optional legend domain.
    /// Quantile resolution retains only this extent and its cuts, not samples.
    pub fn sample_extent(&self) -> Option<[f64; 2]> {
        self.sample_extent
    }
    /// Inverse by palette index; unlike inverse-by-color, duplicate colors
    /// remain independently addressable. Continuous scales have no buckets.
    pub fn bin_extent(&self, index: usize) -> Option<ColorScaleExtent> {
        if !self.kind.is_discrete() || index >= self.colors.len() {
            return None;
        }
        let domain = if self.kind == ColorScaleKind::Quantile {
            self.sample_extent?
        } else {
            self.domain
        };
        Some(ColorScaleExtent {
            start: index
                .checked_sub(1)
                .map(|i| self.thresholds[i])
                .or_else(|| (self.kind != ColorScaleKind::Threshold).then_some(domain[0])),
            end: self
                .thresholds
                .get(index)
                .copied()
                .or_else(|| (self.kind != ColorScaleKind::Threshold).then_some(domain[1])),
        })
    }
    /// D3 inverse convention: the first matching palette entry wins.
    pub fn invert_extent(&self, color: Color) -> Option<ColorScaleExtent> {
        self.bin_extent(self.colors.iter().position(|c| *c == color)?)
    }
    pub fn map(&self, value: Option<f64>) -> Color {
        let Some(value) = value.filter(|v| v.is_finite()) else {
            return self.unknown;
        };
        let [a, b] = self.domain;
        if self.kind == ColorScaleKind::SequentialQuantile {
            if !self.clamp && (value < a || value > b) {
                return self.unknown;
            }
            return self
                .rank(value)
                .map_or(self.unknown, |rank| self.color_at_parameter(rank));
        }
        if self.kind == ColorScaleKind::Quantile && self.sample_extent.is_none() {
            return self.unknown;
        }
        if self.kind.is_discrete() {
            if !self.clamp && (value < a.min(b) || value > a.max(b)) {
                return self.unknown;
            }
            let index = if a <= b {
                self.thresholds.partition_point(|t| *t <= value)
            } else {
                self.thresholds.partition_point(|t| *t >= value)
            };
            return self.colors[index];
        }
        let mut t = if self.transform != ColorScaleTransform::Linear {
            self.transform.parameter(value, self.domain, self.kind)
        } else {
            match self.kind {
                ColorScaleKind::Diverging { midpoint } => {
                    if value == midpoint {
                        0.5
                    } else if (b >= a && value < midpoint) || (b < a && value > midpoint) {
                        if a == midpoint {
                            0.5
                        } else {
                            0.5 * color_fraction(value, a, midpoint)
                        }
                    } else if midpoint == b {
                        0.5
                    } else {
                        0.5 + 0.5 * color_fraction(value, midpoint, b)
                    }
                }
                _ => color_fraction(value, a, b),
            }
        };
        if self.clamp {
            t = t.clamp(0., 1.);
        }
        if self.transform != ColorScaleTransform::Linear && !t.is_finite() {
            return self.unknown;
        }
        self.color_at_parameter(t)
    }
    fn color_at_parameter(&self, t: f64) -> Color {
        let interpolation = match &self.profile {
            ResolvedColorProfile::Interpolate(interpolation) => interpolation,
            ResolvedColorProfile::Scheme(scheme) => return scheme.sample_unit(t),
        };
        let n = self.colors.len() - 1;
        let i = if t <= 0. {
            0
        } else if t >= 1. {
            n - 1
        } else {
            (t * n as f64).floor() as usize
        };
        interpolation.map(&self.colors, i, t * n as f64 - i as f64)
    }
    pub fn bins(&self) -> Vec<ColorScaleBin> {
        if !self.kind.is_discrete() {
            return Vec::new();
        }
        if self.domain[0] == self.domain[1] {
            return vec![ColorScaleBin {
                start: self.domain[0],
                end: self.domain[1],
                color: self.map(Some(self.domain[0])),
            }];
        }
        if matches!(
            self.kind,
            ColorScaleKind::Quantile | ColorScaleKind::Threshold
        ) {
            if self.kind == ColorScaleKind::Quantile && self.sample_extent.is_none() {
                return vec![ColorScaleBin {
                    start: self.domain[0],
                    end: self.domain[1],
                    color: self.unknown,
                }];
            }
            let [a, b] = self.domain;
            return self
                .colors
                .iter()
                .enumerate()
                .filter_map(|(i, color)| {
                    let start = i.checked_sub(1).map_or(a, |i| self.thresholds[i].max(a));
                    let end = self.thresholds.get(i).map_or(b, |t| t.min(b));
                    (start < end).then_some(ColorScaleBin {
                        start,
                        end,
                        color: *color,
                    })
                })
                .collect();
        }
        self.colors
            .iter()
            .enumerate()
            .map(|(i, color)| ColorScaleBin {
                start: if i == 0 {
                    self.domain[0]
                } else {
                    self.thresholds[i - 1]
                },
                end: self.thresholds.get(i).copied().unwrap_or(self.domain[1]),
                color: *color,
            })
            .collect()
    }
    /// Ordered value/color samples plus every interpolation kink. Quantize
    /// and lookup-table schemes retain their exact bins by using duplicate
    /// boundary offsets so consumers can draw hard edges. SequentialQuantile
    /// retains exact effective data jumps regardless of the valid sample count;
    /// more than 512 stops or collapsed f64/f32 jump positions return an error.
    pub fn stops(&self, steps: usize) -> Result<Vec<ColorScaleStop>> {
        ensure(
            (2..=256).contains(&steps),
            "color-scale stop sample budget must be 2..=256",
        )?;
        if self.kind == ColorScaleKind::SequentialQuantile {
            return self.sequential_quantile_stops();
        }
        let [a, b] = self.domain;
        if a == b {
            let color = self.map(Some(a));
            return Ok(vec![
                ColorScaleStop {
                    offset: 0.,
                    value: a,
                    color,
                },
                ColorScaleStop {
                    offset: 1.,
                    value: a,
                    color,
                },
            ]);
        }
        if self.transform != ColorScaleTransform::Linear {
            return self.nonlinear_stops(steps);
        }
        if self.kind.is_discrete() {
            return Ok(self
                .bins()
                .into_iter()
                .flat_map(|bin| {
                    [
                        ColorScaleStop {
                            offset: color_fraction(bin.start, a, b),
                            value: bin.start,
                            color: bin.color,
                        },
                        ColorScaleStop {
                            offset: color_fraction(bin.end, a, b),
                            value: bin.end,
                            color: bin.color,
                        },
                    ]
                })
                .collect());
        }
        if let ResolvedColorProfile::Scheme(scheme) = self.profile
            && let Some(len) = scheme.lookup_len()
        {
            let midpoint = match self.kind {
                ColorScaleKind::Diverging { midpoint } => Some(color_fraction(midpoint, a, b)),
                _ => None,
            };
            let offset = |t: f64| match midpoint {
                Some(middle) if t <= 0.5 => 2. * t * middle,
                Some(middle) => color_value(middle, 1., 2. * t - 1.),
                None => t,
            };
            let stop = |offset, color| ColorScaleStop {
                offset,
                value: color_value(a, b, offset),
                color,
            };
            let mut stops = Vec::with_capacity(2 * len);
            let unit = 1. / len as f64;
            for index in 0..len {
                let left = offset(index as f64 * unit);
                let right = offset((index + 1) as f64 * unit);
                if left < right {
                    let color = scheme.sample_unit((index as f64 + 0.5) * unit);
                    stops.extend([stop(left, color), stop(right, color)]);
                }
            }
            if midpoint == Some(1.) {
                stops.push(stop(1., scheme.sample_unit(0.5)));
            }
            return Ok(stops);
        }
        let mut offsets = (0..steps)
            .map(|i| i as f64 / (steps - 1) as f64)
            .collect::<Vec<_>>();
        match self.kind {
            ColorScaleKind::Sequential => offsets
                .extend((0..self.colors.len()).map(|i| i as f64 / (self.colors.len() - 1) as f64)),
            ColorScaleKind::Diverging { midpoint } => offsets.push(color_fraction(midpoint, a, b)),
            _ => {}
        }
        offsets.sort_by(f64::total_cmp);
        offsets.dedup();
        Ok(offsets
            .into_iter()
            .map(|offset| {
                let value = color_value(a, b, offset);
                ColorScaleStop {
                    offset,
                    value,
                    color: self.map(Some(value)),
                }
            })
            .collect())
    }
}
