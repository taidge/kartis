//! Data-domain navigation for layouts with fixed plot axes.
//!
//! Like D3 zoom's rescaleX/rescaleY, navigation inverts the transformed pixel
//! range through the original continuous scale. It does not transform Scene
//! geometry. Coordinates in requests are plot-local, independent of plot.x/y.
//! Reference: E:/Repos/d3/docs/d3-zoom.md, transform.rescaleX/rescaleY.
//! Domains follow the existing chart builders' strictly ascending, nonconstant
//! continuous-scale profile. UTC output rounds to integral epoch milliseconds;
//! its continuous gesture window saturates at one original millisecond.
use crate::scene::ensure;
use crate::{Brush, ChartScale, ClipRect, Result, ScaleKind, ZoomTransform};
mod piecewise;
pub use piecewise::PiecewiseViewportDomains;
use piecewise::{PiecewiseMappings, resolve_axis};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportAxis {
    pub domain: [f64; 2],
    pub scale: ScaleKind,
}
impl ViewportAxis {
    pub fn new(domain: [f64; 2], scale: ScaleKind) -> Self {
        Self { domain, scale }
    }
    fn validate(self) -> Result<()> {
        ensure(
            self.domain[0] < self.domain[1],
            "viewport domains must be strictly ascending",
        )?;
        // The returned domains must be admissible by the chart builders too.
        ChartScale::new(self.scale, self.domain, [0., 1.])?;
        let t = self.transformed();
        ensure(
            t.iter().all(|v| v.is_finite()) && t[0] < t[1] && (t[1] - t[0]).is_finite(),
            "viewport transformed domain must have a representable positive span",
        )
    }
    fn transformed(self) -> [f64; 2] {
        self.domain.map(|v| match self.scale {
            ScaleKind::Linear | ScaleKind::Utc => v,
            ScaleKind::Log => self.domain[0].signum() * v.abs().ln(),
            ScaleKind::SymLog { constant } => crate::scales::symlog_transform(v, constant),
            ScaleKind::Pow { exponent } => crate::scales::power_transform(v, exponent),
            ScaleKind::Sqrt => crate::scales::power_transform(v, 0.5),
        })
    }
    fn value(self, fraction: f64) -> f64 {
        if fraction == 0. {
            return self.domain[0];
        }
        if fraction == 1. {
            return self.domain[1];
        }
        let [lo, hi] = self.transformed();
        let v = if (0. ..=1.).contains(&fraction) {
            lo * (1. - fraction) + hi * fraction
        } else {
            lo + (hi - lo) * fraction
        };
        match self.scale {
            ScaleKind::Linear | ScaleKind::Utc => v,
            ScaleKind::Log => {
                let sign = self.domain[0].signum();
                sign * (sign * v).exp()
            }
            ScaleKind::SymLog { constant } => crate::scales::symlog_inverse(v, constant),
            ScaleKind::Pow { exponent } => crate::scales::power_inverse(v, exponent),
            ScaleKind::Sqrt => crate::scales::power_inverse(v, 0.5),
        }
    }
    fn resolve(self, window: Window, vertical: bool, bounded: bool) -> Result<[f64; 2]> {
        let [a, b] = if vertical {
            [1. - window.start - window.span, 1. - window.start]
        } else {
            [window.start, window.start + window.span]
        };
        let mut domain = [self.value(a), self.value(b)];
        if matches!(self.scale, ScaleKind::Pow { exponent } if exponent < 0.) {
            ensure(
                domain
                    .iter()
                    .all(|v| *v != 0. && v.signum() == self.domain[0].signum()),
                "reciprocal viewport cannot cross its original zero singularity",
            )?;
        }
        if self.scale == ScaleKind::Utc {
            domain = domain.map(f64::round);
            // A sub-millisecond result cannot represent a usable time scale.
            // Preserve at least one millisecond, even at the timestamp limit.
            if domain[0] == domain[1] {
                if domain[1] < self.domain[1] || !bounded {
                    domain[1] += 1.;
                } else {
                    domain[0] -= 1.;
                }
            }
        }
        if bounded {
            domain[0] = domain[0].max(self.domain[0]);
            domain[1] = domain[1].min(self.domain[1]);
        }
        Self {
            domain,
            scale: self.scale,
        }
        .validate()?;
        Ok(domain)
    }
    fn max_zoom(self) -> f64 {
        if self.scale == ScaleKind::Utc {
            self.domain[1] - self.domain[0]
        } else {
            f64::INFINITY
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportAxes {
    pub x: ViewportAxis,
    pub left: Option<ViewportAxis>,
    pub right: Option<ViewportAxis>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ViewportSelection {
    X,
    Y,
    #[default]
    Both,
}
impl ViewportSelection {
    fn x(self) -> bool {
        self != Self::Y
    }
    fn y(self) -> bool {
        self != Self::X
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportLimits {
    /// Gesture zoom extent relative to the original transformed domain.
    /// Construction/reset always restore the exact original domain (zoom 1).
    pub min_zoom: f64,
    pub max_zoom: f64,
    /// Clamp selected windows to their original domains without changing their
    /// requested span when they fit. A wider window resolves to the original.
    pub bounded: bool,
}
impl Default for ViewportLimits {
    fn default() -> Self {
        Self {
            min_zoom: 1.,
            max_zoom: 100.,
            bounded: true,
        }
    }
}
impl ViewportLimits {
    pub(crate) fn validate(self) -> Result<()> {
        ensure(
            self.min_zoom.is_finite()
                && self.max_zoom.is_finite()
                && self.min_zoom >= 0.000001
                && self.max_zoom <= 1_000_000.
                && self.min_zoom <= self.max_zoom,
            "viewport zoom extent must satisfy 0.000001 <= min <= max <= 1000000",
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResolvedDomains {
    pub x: [f64; 2],
    pub left: Option<[f64; 2]>,
    pub right: Option<[f64; 2]>,
}
/// Screen-oriented fractions of the original plot: X left to right, Y top to
/// bottom. Both axes may have different zooms after an axis-selective gesture.
/// UTC domains round their endpoints; these windows retain continuous gesture
/// fractions so rounding does not accumulate across repeated input events.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportWindows {
    pub x: [f64; 2],
    pub y: [f64; 2],
}
/// Independent absolute, plot-local transforms of the original axes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportTransform {
    pub scale_x: f64,
    pub scale_y: f64,
    pub translate_x: f64,
    pub translate_y: f64,
}
/// A relative gesture separates anchored zoom from centroid/pointer movement.
/// This preserves pan when one independent data axis has reached a zoom limit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewportGesture {
    pub factor: f64,
    pub anchor: (f64, f64),
    pub pan: (f64, f64),
}
impl ViewportTransform {
    pub fn validate(self) -> Result<()> {
        ZoomTransform::new(self.scale_x, self.translate_x, 0.)?;
        ZoomTransform::new(self.scale_y, 0., self.translate_y)?;
        Ok(())
    }
}
/// Absolute original-frame transforms and incremental current-domain gestures
/// are separate variants to avoid accidentally applying a transform twice.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ViewportRequest {
    TransformXY {
        transform: ViewportTransform,
        axes: ViewportSelection,
    },
    Transform {
        transform: ZoomTransform,
        axes: ViewportSelection,
    },
    Zoom {
        factor: f64,
        anchor: (f64, f64),
        axes: ViewportSelection,
    },
    Pan {
        delta: (f64, f64),
        axes: ViewportSelection,
    },
    /// Brush coordinates refer to the current plot. Intersect with the plot;
    /// disjoint or zero-width selected dimensions reject without mutation.
    Brush {
        bounds: Brush,
        axes: ViewportSelection,
    },
    Reset,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct Window {
    start: f64,
    span: f64,
}
impl Default for Window {
    fn default() -> Self {
        Self {
            start: 0.,
            span: 1.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DomainViewport {
    axes: ViewportAxes,
    plot: ClipRect,
    limits: ViewportLimits,
    x: Window,
    y: Window,
    domains: ResolvedDomains,
    x_zoom_cap: Option<f64>,
    y_zoom_cap: Option<f64>,
    directions: ViewportDirections,
    piecewise: PiecewiseMappings,
}
/// Axis range reversals, independent for the two value axes. Windows and
/// gestures remain screen-oriented; returned domains remain ascending.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewportDirections {
    pub x: bool,
    pub left: bool,
    pub right: bool,
}
fn validate_plot(plot: ClipRect) -> Result<()> {
    ensure(
        [
            plot.x,
            plot.y,
            plot.width,
            plot.height,
            plot.x + plot.width,
            plot.y + plot.height,
        ]
        .iter()
        .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            && plot.width > 0.
            && plot.height > 0.,
        "viewport plot must be finite and have positive dimensions within the Scene coordinate budget",
    )
}
fn finite_pair(pair: (f64, f64)) -> Result<()> {
    ensure(
        pair.0.is_finite() && pair.1.is_finite(),
        "viewport coordinates must be finite",
    )
}
impl DomainViewport {
    pub fn new(axes: ViewportAxes, plot: ClipRect, limits: ViewportLimits) -> Result<Self> {
        validate_plot(plot)?;
        limits.validate()?;
        axes.x.validate()?;
        for axis in [axes.left, axes.right].into_iter().flatten() {
            axis.validate()?;
        }
        let domains = ResolvedDomains {
            x: axes.x.domain,
            left: axes.left.map(|a| a.domain),
            right: axes.right.map(|a| a.domain),
        };
        Ok(Self {
            axes,
            plot,
            limits,
            x: Window::default(),
            y: Window::default(),
            domains,
            x_zoom_cap: None,
            y_zoom_cap: None,
            directions: ViewportDirections::default(),
            piecewise: PiecewiseMappings::default(),
        })
    }
    pub fn with_directions(mut self, directions: ViewportDirections) -> Result<Self> {
        self.directions = directions;
        self.domains = self.resolved(self.x, self.y)?;
        Ok(self)
    }
    pub fn directions(&self) -> ViewportDirections {
        self.directions
    }
    /// Bound the independent X zoom without reducing the value-axis extent.
    /// Category controllers use the category count to retain at least one band.
    pub fn with_x_max_zoom(mut self, maximum: f64) -> Result<Self> {
        ensure(
            maximum.is_finite() && (1. ..=1_000_000.).contains(&maximum),
            "invalid independent X zoom cap",
        )?;
        self.x_zoom_cap = Some(maximum);
        // Installing an upper cap must not apply a gesture-only minimum to a
        // freshly constructed/reset viewport, whose exact original domain is
        // intentionally represented by zoom 1 even if min_zoom > 1.
        if 1. / self.x.span > maximum {
            self.x = self.constrained(self.x, false)?;
        }
        self.domains = self.resolved(self.x, self.y)?;
        Ok(self)
    }
    /// Bound the shared left/right zoom independently of X. Point-valued Y
    /// controllers use an ordinal span cap without limiting numeric X zoom.
    /// Construction/reset keep their exact domains even if min_zoom > 1.
    pub fn with_y_max_zoom(mut self, maximum: f64) -> Result<Self> {
        ensure(
            maximum.is_finite() && (1. ..=1_000_000.).contains(&maximum),
            "invalid independent Y zoom cap",
        )?;
        ensure(
            self.axes.left.is_some() || self.axes.right.is_some(),
            "viewport has no value axis",
        )?;
        self.y_zoom_cap = Some(maximum);
        if 1. / self.y.span > maximum {
            self.y = self.constrained(self.y, true)?;
        }
        self.domains = self.resolved(self.x, self.y)?;
        Ok(self)
    }
    pub fn original_axes(&self) -> ViewportAxes {
        self.axes
    }
    pub fn plot(&self) -> ClipRect {
        self.plot
    }
    pub fn limits(&self) -> ViewportLimits {
        self.limits
    }
    pub fn domains(&self) -> ResolvedDomains {
        self.domains
    }
    pub fn normalized_windows(&self) -> ViewportWindows {
        ViewportWindows {
            x: [self.x.start, self.x.start + self.x.span],
            y: [self.y.start, self.y.start + self.y.span],
        }
    }
    pub fn transform_xy(&self) -> ViewportTransform {
        let (scale_x, scale_y) = self.zoom();
        ViewportTransform {
            scale_x,
            scale_y,
            translate_x: -self.x.start * self.plot.width * scale_x,
            translate_y: -self.y.start * self.plot.height * scale_y,
        }
    }
    /// Return an absolute plot-local transform for a chosen interaction axis.
    /// A single isotropic transform cannot express unequal X/Y zooms; Both
    /// rejects that case so a host cannot silently change one current domain.
    pub fn transform_for(&self, axes: ViewportSelection) -> Result<ZoomTransform> {
        let (kx, ky) = self.zoom();
        let has_y = self.axes.left.is_some() || self.axes.right.is_some();
        ensure(
            axes != ViewportSelection::Y || has_y,
            "viewport has no value axis",
        )?;
        ensure(
            axes != ViewportSelection::Both || !has_y || kx == ky,
            "independent viewport axes cannot use one isotropic zoom transform",
        )?;
        let k = if axes == ViewportSelection::Y { ky } else { kx };
        ZoomTransform::new(
            k,
            if axes.x() {
                -self.x.start * self.plot.width * k
            } else {
                0.
            },
            if axes.y() && has_y {
                -self.y.start * self.plot.height * k
            } else {
                0.
            },
        )
    }
    /// Effective zoom in transformed data coordinates; left/right share Y.
    pub fn zoom(&self) -> (f64, f64) {
        (1. / self.x.span, 1. / self.y.span)
    }
    pub fn reset(&mut self) -> ResolvedDomains {
        self.x = Window::default();
        self.y = Window::default();
        self.domains = ResolvedDomains {
            x: self.axes.x.domain,
            left: self.axes.left.map(|a| a.domain),
            right: self.axes.right.map(|a| a.domain),
        };
        self.domains
    }
    /// A resize changes only the plot geometry. The existing numeric ranges
    /// stay bit-for-bit identical; future gestures use the new dimensions.
    pub fn resize(&mut self, plot: ClipRect) -> Result<ResolvedDomains> {
        validate_plot(plot)?;
        self.plot = plot;
        Ok(self.domains)
    }
    /// Evaluate a request without committing state (e.g. application approval).
    pub fn resolve(&self, request: ViewportRequest) -> Result<ResolvedDomains> {
        let mut proposal = self.clone();
        proposal.apply(request)
    }
    /// Apply a wheel/pan/pinch delta to the accepted domains. Unlike an absolute
    /// TransformXY, each axis clamps its zoom before preserving the anchor and
    /// applying pan. Captured pointers/centroids may lie outside the plot.
    pub fn apply_gesture(
        &mut self,
        gesture: ViewportGesture,
        axes: ViewportSelection,
    ) -> Result<ResolvedDomains> {
        ensure(
            gesture.factor.is_finite() && gesture.factor > 0.,
            "invalid relative viewport zoom factor",
        )?;
        finite_pair(gesture.anchor)?;
        finite_pair(gesture.pan)?;
        let has_y = self.axes.left.is_some() || self.axes.right.is_some();
        ensure(
            axes != ViewportSelection::Y || has_y,
            "viewport has no value axis",
        )?;
        let window =
            |old: Window, anchor: f64, pan: f64, extent: f64, vertical: bool| -> Result<Window> {
                let current = 1. / old.span;
                let (minimum, maximum) = self.zoom_extent(vertical);
                let zoom = if gesture.factor >= maximum / current {
                    maximum
                } else if gesture.factor <= minimum / current {
                    minimum
                } else {
                    current * gesture.factor
                };
                let span = 1. / zoom;
                self.constrained(
                    Window {
                        start: old.start + (old.span - span) * (anchor / extent)
                            - (pan / extent) * span,
                        span,
                    },
                    vertical,
                )
            };
        let x = if axes.x() {
            window(
                self.x,
                gesture.anchor.0,
                gesture.pan.0,
                self.plot.width,
                false,
            )?
        } else {
            self.x
        };
        let y = if axes.y() && has_y {
            window(
                self.y,
                gesture.anchor.1,
                gesture.pan.1,
                self.plot.height,
                true,
            )?
        } else {
            self.y
        };
        let domains = self.resolved(x, y)?;
        self.x = x;
        self.y = y;
        self.domains = domains;
        Ok(domains)
    }
    fn zoom_extent(&self, vertical: bool) -> (f64, f64) {
        let representable_max = if vertical {
            [self.axes.left, self.axes.right]
                .into_iter()
                .flatten()
                .map(ViewportAxis::max_zoom)
                .fold(f64::INFINITY, f64::min)
                .min(self.y_zoom_cap.unwrap_or(f64::INFINITY))
        } else {
            self.axes
                .x
                .max_zoom()
                .min(self.x_zoom_cap.unwrap_or(f64::INFINITY))
        };
        let maximum = self.limits.max_zoom.min(representable_max);
        let minimum = self.limits.min_zoom.min(maximum);
        (minimum, maximum)
    }
    fn constrained(&self, window: Window, vertical: bool) -> Result<Window> {
        ensure(
            (window.start.is_finite() || self.limits.bounded && !window.start.is_nan())
                && window.span.is_finite()
                && window.span > 0.,
            "viewport window overflow",
        )?;
        let (minimum, maximum) = self.zoom_extent(vertical);
        let zoom = (1. / window.span).clamp(minimum, maximum);
        let span = 1. / zoom;
        let mut start = window.start + (window.span - span) * 0.5;
        if self.limits.bounded {
            if span >= 1. {
                return Ok(Window::default());
            }
            start = start.clamp(0., 1. - span);
        }
        ensure(
            start.is_finite() && (start + span).is_finite(),
            "viewport window overflow",
        )?;
        Ok(Window { start, span })
    }
    fn resolved(&self, x: Window, y: Window) -> Result<ResolvedDomains> {
        Ok(ResolvedDomains {
            x: resolve_axis(
                self.axes.x,
                self.piecewise.x.as_ref(),
                x,
                self.directions.x,
                self.limits.bounded,
            )?,
            left: self
                .axes
                .left
                .map(|axis| {
                    resolve_axis(
                        axis,
                        self.piecewise.left.as_ref(),
                        y,
                        !self.directions.left,
                        self.limits.bounded,
                    )
                })
                .transpose()?,
            right: self
                .axes
                .right
                .map(|axis| {
                    resolve_axis(
                        axis,
                        self.piecewise.right.as_ref(),
                        y,
                        !self.directions.right,
                        self.limits.bounded,
                    )
                })
                .transpose()?,
        })
    }
    /// Commit only after all selected axes and their chart-compatible inverse
    /// domains validate. Failure leaves domains, plot and gesture state intact.
    pub fn apply(&mut self, request: ViewportRequest) -> Result<ResolvedDomains> {
        if request == ViewportRequest::Reset {
            return Ok(self.reset());
        }
        let (mut x, mut y) = (self.x, self.y);
        let selection = match request {
            ViewportRequest::TransformXY { transform, axes } => {
                transform.validate()?;
                if axes.x() {
                    x = Window {
                        start: -(transform.translate_x / transform.scale_x) / self.plot.width,
                        span: 1. / transform.scale_x,
                    };
                }
                if axes.y() {
                    y = Window {
                        start: -(transform.translate_y / transform.scale_y) / self.plot.height,
                        span: 1. / transform.scale_y,
                    };
                }
                axes
            }
            ViewportRequest::Transform { transform, axes } => {
                transform.validate()?;
                if axes.x() {
                    x = Window {
                        start: -(transform.translate_x / transform.scale) / self.plot.width,
                        span: 1. / transform.scale,
                    };
                }
                if axes.y() {
                    y = Window {
                        start: -(transform.translate_y / transform.scale) / self.plot.height,
                        span: 1. / transform.scale,
                    };
                }
                axes
            }
            ViewportRequest::Zoom {
                factor,
                anchor,
                axes,
            } => {
                ensure(
                    factor.is_finite() && factor > 0.,
                    "viewport zoom factor must be finite and positive",
                )?;
                finite_pair(anchor)?;
                ensure(
                    anchor.0 >= 0.
                        && anchor.0 <= self.plot.width
                        && anchor.1 >= 0.
                        && anchor.1 <= self.plot.height,
                    "viewport zoom anchor must be inside the local plot",
                )?;
                let zoom = |w: Window, pixel: f64, extent: f64, vertical: bool| -> Result<Window> {
                    // Clamp the target scale before multiplying, including
                    // extreme finite factors which would otherwise overflow.
                    let current = 1. / w.span;
                    let (minimum, maximum) = self.zoom_extent(vertical);
                    let target = if factor >= maximum / current {
                        maximum
                    } else if factor <= minimum / current {
                        minimum
                    } else {
                        current * factor
                    };
                    let span = 1. / target;
                    let anchor = pixel / extent;
                    self.constrained(
                        Window {
                            start: w.start + (w.span - span) * anchor,
                            span,
                        },
                        vertical,
                    )
                };
                if axes.x() {
                    x = zoom(x, anchor.0, self.plot.width, false)?;
                }
                if axes.y() {
                    y = zoom(y, anchor.1, self.plot.height, true)?;
                }
                axes
            }
            ViewportRequest::Pan { delta, axes } => {
                finite_pair(delta)?;
                if axes.x() {
                    x.start -= (delta.0 / self.plot.width) * x.span;
                }
                if axes.y() {
                    y.start -= (delta.1 / self.plot.height) * y.span;
                }
                axes
            }
            ViewportRequest::Brush { bounds, axes } => {
                let clipped = bounds
                    .clipped(self.plot.width, self.plot.height)?
                    .ok_or_else(|| {
                        crate::ChartError("viewport brush is disjoint from plot".into())
                    })?;
                if axes.x() {
                    x = Window {
                        start: x.start + x.span * (clipped.x0 / self.plot.width),
                        span: x.span * ((clipped.x1 - clipped.x0) / self.plot.width),
                    };
                }
                if axes.y() {
                    y = Window {
                        start: y.start + y.span * (clipped.y0 / self.plot.height),
                        span: y.span * ((clipped.y1 - clipped.y0) / self.plot.height),
                    };
                }
                axes
            }
            ViewportRequest::Reset => unreachable!(),
        };
        ensure(
            selection != ViewportSelection::Y
                || self.axes.left.is_some()
                || self.axes.right.is_some(),
            "viewport has no value axis",
        )?;
        if selection.x() {
            x = self.constrained(x, false)?;
        }
        if selection.y() && (self.axes.left.is_some() || self.axes.right.is_some()) {
            y = self.constrained(y, true)?;
        } else {
            y = self.y;
        }
        let domains = self.resolved(x, y)?;
        self.x = x;
        self.y = y;
        self.domains = domains;
        Ok(domains)
    }
}
