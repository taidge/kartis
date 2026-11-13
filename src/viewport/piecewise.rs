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
    normalized: ChartScale,
    source_descending: bool,
}

impl PartialEq for PiecewiseMapping {
    fn eq(&self, other: &Self) -> bool {
        self.profile == other.profile && self.source_descending == other.source_descending
    }
}

impl PiecewiseMapping {
    fn new(axis: ViewportAxis, scale: &ChartScale) -> Result<Self> {
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
        let mut domain = scale.domain_stops().to_vec();
        let mut range = scale.range_stops().to_vec();
        let source_descending = domain[0] > domain[domain.len() - 1];
        if source_descending {
            domain.reverse();
            range.reverse();
        }
        ensure(
            [domain[0], domain[domain.len() - 1]] == axis.domain,
            "piecewise viewport endpoints differ from the original axis",
        )?;
        let first = range[0];
        let span = range[range.len() - 1] - first;
        ensure(
            span.is_finite() && span != 0.,
            "piecewise viewport range has no finite span",
        )?;
        for value in &mut range {
            *value = (*value - first) / span;
        }
        let profile = PiecewiseScaleOptions {
            domain,
            range,
            continuous: ContinuousScaleOptions {
                kind: axis.scale,
                ..Default::default()
            },
        };
        let normalized = ResolvedContinuousScale::piecewise(&profile)?.into_chart_scale();
        Ok(Self {
            profile,
            normalized,
            source_descending,
        })
    }

    fn stops(&self, window: Window, reverse: bool, bounded: bool) -> Result<Vec<f64>> {
        let mut domain = if window.start == 0. && window.span == 1. {
            self.profile.domain.clone()
        } else {
            let start = if reverse {
                1. - window.start - window.span
            } else {
                window.start
            };
            let low = self.profile.domain[0];
            let high = self.profile.domain[self.profile.domain.len() - 1];
            let values = self
                .profile
                .range
                .iter()
                .map(|&position| {
                    let value = self.normalized.invert(start + window.span * position)?;
                    Ok(if bounded {
                        value.clamp(low, high)
                    } else {
                        value
                    })
                })
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
        if self.source_descending {
            domain.reverse();
        }
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
        let stops = mapping.stops(window, reverse, bounded)?;
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
                    PiecewiseMapping::new(axis, scale)
                })
                .transpose()
        };
        self.piecewise = PiecewiseMappings {
            x: x.map(|scale| PiecewiseMapping::new(self.axes.x, scale))
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
                .map(|m| m.stops(self.x, self.directions.x, self.limits.bounded))
                .transpose()?,
            left: self
                .piecewise
                .left
                .as_ref()
                .map(|m| m.stops(self.y, !self.directions.left, self.limits.bounded))
                .transpose()?,
            right: self
                .piecewise
                .right
                .as_ref()
                .map(|m| m.stops(self.y, !self.directions.right, self.limits.bounded))
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
