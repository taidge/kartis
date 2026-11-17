//! Full-breakpoint numeric navigation, using the original scale's inverse.
use super::*;
use crate::{ContinuousScaleOptions, PiecewiseScaleOptions, ResolvedContinuousScale};

/// Authored-order domain stops for the configured piecewise axes. Ordinary
/// two-endpoint axes remain None and continue to use ResolvedDomains.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PiecewiseViewportDomains {
    pub x: Option<Vec<f64>>,
    pub left: Option<Vec<f64>>,
    pub right: Option<Vec<f64>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct PiecewiseMappings {
    pub x: Option<PiecewiseMapping>,
    pub left: Option<PiecewiseMapping>,
    pub right: Option<PiecewiseMapping>,
}

#[derive(Clone, Debug)]
pub(super) struct PiecewiseMapping {
    profile: PiecewiseScaleOptions,
    pixels: ChartScale,
    extent: f64,
}

impl PartialEq for PiecewiseMapping {
    fn eq(&self, other: &Self) -> bool {
        self.profile == other.profile && self.extent == other.extent
    }
}

impl PiecewiseMapping {
    fn new(axis: ViewportAxis, scale: &ChartScale, origin: f64, extent: f64) -> Result<Self> {
        ensure(
            scale.is_piecewise(),
            "viewport mapping must be a piecewise scale",
        )?;
        ensure(
            scale.is_invertible(),
            "piecewise viewport requires a strictly monotonic range",
        )?;
        ensure(
            axis.scale == scale.kind(),
            "piecewise viewport kind differs from axis",
        )?;
        let domain = scale.domain_stops().to_vec();
        let range = scale
            .range_stops()
            .iter()
            .map(|v| v - origin)
            .collect::<Vec<_>>();
        let first_domain = domain[0];
        let last_domain = domain[domain.len() - 1];
        ensure(
            [first_domain.min(last_domain), first_domain.max(last_domain)] == axis.domain,
            "piecewise viewport endpoints differ from the original axis",
        )?;
        let span = range[range.len() - 1] - range[0];
        ensure(
            span.is_finite() && span != 0.,
            "piecewise viewport range has no finite span",
        )?;
        let profile = PiecewiseScaleOptions {
            domain,
            range,
            continuous: ContinuousScaleOptions {
                kind: axis.scale,
                clamp: scale.is_clamped(),
                ..Default::default()
            },
        };
        let pixels = ResolvedContinuousScale::piecewise(&profile)?.into_chart_scale();
        Ok(Self {
            profile,
            pixels,
            extent,
        })
    }

    fn stops(&self, window: Window) -> Result<Vec<f64>> {
        let domain = if window.start == 0. && window.span == 1. {
            self.profile.domain.clone()
        } else {
            // Keep D3 Transform's pixel arithmetic and order of operations.
            // Normalizing to [0,1] and simplifying this affine expression can
            // perturb constant inverse branches by an ULP or UTC TimeClip by
            // one millisecond, then select an entirely different output part.
            let k = 1. / window.span;
            let translation = -window.start * self.extent * k;
            let values = self
                .profile
                .range
                .iter()
                // Bounded navigation already constrains the pixel window.
                // Clamping the inverse again would erase native weighted
                // interpolation and transcendental rounding at repeated ends.
                .map(|&position| self.pixels.invert((position - translation) / k))
                .collect::<Result<Vec<_>>>()?;
            // Every knot must remain usable, not merely the two outer ones.
            let proposal = PiecewiseScaleOptions {
                domain: values.clone(),
                range: self.profile.range.clone(),
                continuous: self.profile.continuous,
            };
            proposal.validate()?;
            values
        };
        Ok(domain)
    }
}

pub(super) fn resolve_axis(
    axis: ViewportAxis,
    mapping: Option<&PiecewiseMapping>,
    window: Window,
    reverse: bool,
    bounded: bool,
) -> Result<[f64; 2]> {
    if let Some(mapping) = mapping {
        let stops = mapping.stops(window)?;
        let last = stops[stops.len() - 1];
        Ok([stops[0].min(last), stops[0].max(last)])
    } else {
        axis.resolve(window, reverse, bounded)
    }
}

impl DomainViewport {
    /// Install complete displayed mappings for any piecewise axes. Their
    /// endpoint domains and kinds must match original_axes. Passing None keeps
    /// that axis on its existing ordinary two-endpoint profile.
    ///
    /// Like D3 rescaleX/rescaleY, a request inverts every original range knot.
    /// Ranges that turn or contain plateaus can be drawn, but cannot supply a
    /// unique viewport inverse and are rejected here.
    pub fn with_piecewise_scales(
        mut self,
        x: Option<&ChartScale>,
        left: Option<&ChartScale>,
        right: Option<&ChartScale>,
    ) -> Result<Self> {
        let value_axis = |axis: Option<ViewportAxis>, scale: Option<&ChartScale>| {
            scale
                .map(|scale| {
                    let axis = axis.ok_or_else(|| {
                        crate::ChartError("piecewise mapping has no value axis".into())
                    })?;
                    PiecewiseMapping::new(axis, scale, self.plot.y, self.plot.height)
                })
                .transpose()
        };
        self.piecewise = PiecewiseMappings {
            x: x.map(|scale| {
                PiecewiseMapping::new(self.axes.x, scale, self.plot.x, self.plot.width)
            })
            .transpose()?,
            left: value_axis(self.axes.left, left)?,
            right: value_axis(self.axes.right, right)?,
        };
        self.domains = self.resolved(self.x, self.y)?;
        Ok(self)
    }

    /// Full accepted breakpoints, retaining authored domain order. Layout
    /// callbacks must use these for piecewise axes instead of replacing their
    /// profile with the two outer ResolvedDomains endpoints.
    pub fn piecewise_domains(&self) -> Result<PiecewiseViewportDomains> {
        Ok(PiecewiseViewportDomains {
            x: self
                .piecewise
                .x
                .as_ref()
                .map(|m| m.stops(self.x))
                .transpose()?,
            left: self
                .piecewise
                .left
                .as_ref()
                .map(|m| m.stops(self.y))
                .transpose()?,
            right: self
                .piecewise
                .right
                .as_ref()
                .map(|m| m.stops(self.y))
                .transpose()?,
        })
    }

    /// Preview all breakpoint domains without committing the request.
    pub fn resolve_piecewise(&self, request: ViewportRequest) -> Result<PiecewiseViewportDomains> {
        let mut proposal = self.clone();
        proposal.apply(request)?;
        proposal.piecewise_domains()
    }
}
