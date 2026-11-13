//! Typed tree presentation, following local Nivo tree/src/types.ts, defaults.ts,
//! hooks.ts::useLinkGenerator and labelsHooks.ts. Radial links and labels extend
//! Nivo's four Cartesian layouts; this module emits real shared Scene geometry.
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use super::{TreeOrientation, arc_segment, point};
use crate::scene::ensure;
use crate::*;

/// Bump keeps the earlier default. Step variants route along layout depth
/// (including vertical layouts); radial steps use circular polar elbows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeLinkCurve {
    #[default]
    Bump,
    Straight,
    Step,
    StepBefore,
    StepAfter,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeLinkColor {
    #[default]
    Theme,
    Fixed(Color),
    Source,
    Target,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLinkOptions {
    pub enabled: bool,
    pub curve: TreeLinkCurve,
    pub width: f64,
    pub color: TreeLinkColor,
    pub opacity: f64,
    pub cap: StrokeCap,
    pub dash: Option<StrokeDash>,
}
impl Default for TreeLinkOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            curve: TreeLinkCurve::Bump,
            width: 1.5,
            color: TreeLinkColor::Theme,
            opacity: 1.,
            cap: StrokeCap::Round,
            dash: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeNodeShape {
    #[default]
    Circle,
    Square,
    Diamond,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeNodeFill {
    #[default]
    Palette,
    Fixed(Color),
    None,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeNodeStyle {
    pub shape: TreeNodeShape,
    /// None inherits TreeOptions.node_radius. Square uses this as half-side;
    /// diamond uses it as the distance from center to each vertex.
    pub radius: Option<f64>,
    pub fill: TreeNodeFill,
    pub border: Option<GraphicStroke>,
    pub opacity: f64,
}
impl Default for TreeNodeStyle {
    fn default() -> Self {
        Self {
            shape: TreeNodeShape::Circle,
            radius: None,
            fill: TreeNodeFill::Palette,
            border: None,
            opacity: 1.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeLabels {
    #[default]
    All,
    Leaves,
    Internal,
    None,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeLabelPosition {
    #[default]
    Above,
    Below,
    Left,
    Right,
    Center,
    Outward,
    Inward,
    Layout,
    LayoutOpposite,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TreeLabelRotation {
    Fixed(f64),
    AlongLayout,
}
impl Default for TreeLabelRotation {
    fn default() -> Self {
        Self::Fixed(0.)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLabelOptions {
    pub visibility: TreeLabels,
    pub position: TreeLabelPosition,
    pub offset: f64,
    pub rotation: TreeLabelRotation,
    pub color: Option<Color>,
    pub size: Option<f64>,
}
impl Default for TreeLabelOptions {
    fn default() -> Self {
        Self {
            visibility: TreeLabels::All,
            position: TreeLabelPosition::Above,
            offset: 5.,
            rotation: TreeLabelRotation::Fixed(0.),
            color: None,
            size: None,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TreeStyleOptions {
    pub links: TreeLinkOptions,
    pub nodes: TreeNodeStyle,
    /// Complete class style, replacing nodes; a None radius still inherits the
    /// legacy node_radius. Omitted class styles inherit nodes unchanged.
    pub leaves: Option<TreeNodeStyle>,
    pub internal: Option<TreeNodeStyle>,
    pub labels: TreeLabelOptions,
    /// Clip marks and labels to the same plot rectangle. Default preserves
    /// the earlier layout's overflowing labels and unmasked links.
    pub clip_plot: bool,
}
fn border_valid(border: Option<GraphicStroke>) -> bool {
    border.is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width))
}
impl TreeStyleOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.links.width.is_finite()
                && (0.000001..=10000.).contains(&self.links.width)
                && self.links.opacity.is_finite()
                && (0. ..=1.).contains(&self.links.opacity),
            "invalid tree link style",
        )?;
        if let Some(dash) = &self.links.dash {
            dash.validate()?;
        }
        for style in std::iter::once(&self.nodes)
            .chain(self.leaves.iter())
            .chain(self.internal.iter())
        {
            ensure(
                style
                    .radius
                    .is_none_or(|r| r.is_finite() && (0.000001..=16384.).contains(&r))
                    && border_valid(style.border)
                    && style.opacity.is_finite()
                    && (0. ..=1.).contains(&style.opacity),
                "invalid tree node style",
            )?;
        }
        ensure(
            self.labels.offset.is_finite()
                && (0. ..=16384.).contains(&self.labels.offset)
                && self
                    .labels
                    .size
                    .is_none_or(|size| size.is_finite() && (1. ..=256.).contains(&size))
                && match self.labels.rotation {
                    TreeLabelRotation::Fixed(angle) => {
                        angle.is_finite() && (-360. ..=360.).contains(&angle)
                    }
                    TreeLabelRotation::AlongLayout => true,
                },
            "invalid tree label style",
        )
    }
    pub(super) fn node(&self, leaf: bool) -> &TreeNodeStyle {
        if leaf {
            self.leaves.as_ref().unwrap_or(&self.nodes)
        } else {
            self.internal.as_ref().unwrap_or(&self.nodes)
        }
    }
}
impl TreeNodeStyle {
    pub(super) fn radius(&self, fallback: f64) -> f64 {
        self.radius.unwrap_or(fallback)
    }
    pub(super) fn extent(&self, fallback: f64) -> f64 {
        self.radius(fallback) + self.border.map_or(0., |s| s.width / 2.)
    }
    pub(super) fn color(&self, c: &ChartConfig, index: usize) -> Option<Color> {
        match self.fill {
            TreeNodeFill::Palette => Some(c.theme.color(index)),
            TreeNodeFill::Fixed(color) => Some(color),
            TreeNodeFill::None => None,
        }
    }
    pub(super) fn geometry(&self, p: P, fallback: f64) -> Geometry {
        let r = self.radius(fallback);
        match self.shape {
            TreeNodeShape::Circle => Geometry::Circle {
                x: p.x,
                y: p.y,
                radius: r,
            },
            TreeNodeShape::Square => Geometry::Rect {
                x: p.x - r,
                y: p.y - r,
                width: r * 2.,
                height: r * 2.,
            },
            TreeNodeShape::Diamond => Geometry::Path(vec![
                C::Move(P::new(p.x, p.y - r)),
                C::Line(P::new(p.x + r, p.y)),
                C::Line(P::new(p.x, p.y + r)),
                C::Line(P::new(p.x - r, p.y)),
                C::Close,
            ]),
        }
    }
    fn edge(&self, direction: [f64; 2], fallback: f64) -> f64 {
        let r = self.radius(fallback);
        let distance = match self.shape {
            TreeNodeShape::Circle => r,
            TreeNodeShape::Square => r / direction[0].abs().max(direction[1].abs()),
            TreeNodeShape::Diamond => r / (direction[0].abs() + direction[1].abs()),
        };
        distance + self.border.map_or(0., |s| s.width / 2.)
    }
}
pub(super) struct TreeLinkGeometry {
    pub source: P,
    pub target: P,
    pub source_polar: [f64; 2],
    pub target_polar: [f64; 2],
    pub center: [f64; 2],
    pub orientation: TreeOrientation,
}
pub(super) fn link_geometry(curve: TreeLinkCurve, g: TreeLinkGeometry) -> Geometry {
    let p = g.source;
    let q = g.target;
    let mut path = vec![C::Move(p)];
    if curve == TreeLinkCurve::Straight {
        path.push(C::Line(q));
        return Geometry::Path(path);
    }
    if g.orientation == TreeOrientation::Radial {
        let [rp, ap] = g.source_polar;
        let [rq, aq] = g.target_polar;
        let [cx, cy] = g.center;
        if curve == TreeLinkCurve::Bump {
            let middle = (rp + rq) / 2.;
            path.push(C::Cubic {
                first: point(cx, cy, middle, ap),
                second: point(cx, cy, middle, aq),
                end: q,
            });
        } else {
            // Polar elbows preserve radial/tangential segments; the angular
            // transition takes the shortest arc and uses bounded cubic arcs.
            let r = match curve {
                TreeLinkCurve::StepBefore => rp,
                TreeLinkCurve::StepAfter => rq,
                _ => (rp + rq) / 2.,
            };
            let delta = (aq - ap).sin().atan2((aq - ap).cos());
            path.push(C::Line(point(cx, cy, r, ap)));
            if r > 0. && delta != 0. {
                arc_segment(&mut path, cx, cy, r, ap, ap + delta);
            }
            if r == rq {
                // Set the polar arc endpoint to the exact node coordinate.
                // A redundant epsilon-length line after angle normalization
                // would otherwise lose arc-length precision when dashed.
                match path.last_mut().unwrap() {
                    C::Cubic { end, .. } => *end = q,
                    C::Line(end) => *end = q,
                    _ => unreachable!(),
                }
            } else {
                path.push(C::Line(q));
            }
        }
    } else {
        let horizontal = matches!(
            g.orientation,
            TreeOrientation::LeftToRight | TreeOrientation::RightToLeft
        );
        if curve == TreeLinkCurve::Bump {
            let (first, second) = if horizontal {
                let x = (p.x + q.x) / 2.;
                (P::new(x, p.y), P::new(x, q.y))
            } else {
                let y = (p.y + q.y) / 2.;
                (P::new(p.x, y), P::new(q.x, y))
            };
            path.push(C::Cubic {
                first,
                second,
                end: q,
            });
        } else if horizontal {
            let x = match curve {
                TreeLinkCurve::StepBefore => p.x,
                TreeLinkCurve::StepAfter => q.x,
                _ => (p.x + q.x) / 2.,
            };
            path.extend([C::Line(P::new(x, p.y)), C::Line(P::new(x, q.y)), C::Line(q)]);
        } else {
            let y = match curve {
                TreeLinkCurve::StepBefore => p.y,
                TreeLinkCurve::StepAfter => q.y,
                _ => (p.y + q.y) / 2.,
            };
            path.extend([C::Line(P::new(p.x, y)), C::Line(P::new(q.x, y)), C::Line(q)]);
        }
    }
    Geometry::Path(path)
}
pub(super) struct TreeLabelGeometry<'a> {
    pub position: P,
    pub orientation: TreeOrientation,
    pub angle: f64,
    pub leaf: bool,
    pub style: &'a TreeNodeStyle,
    pub fallback_radius: f64,
    pub text: &'a str,
}
pub(super) fn tree_label(
    c: &ChartConfig,
    options: &TreeLabelOptions,
    g: TreeLabelGeometry<'_>,
) -> Option<Label> {
    let TreeLabelGeometry {
        position: p,
        orientation,
        angle,
        leaf,
        style,
        fallback_radius,
        text,
    } = g;
    if matches!(options.visibility, TreeLabels::None)
        || options.visibility == TreeLabels::Leaves && !leaf
        || options.visibility == TreeLabels::Internal && leaf
    {
        return None;
    }
    let layout = match orientation {
        TreeOrientation::LeftToRight => [1., 0.],
        TreeOrientation::RightToLeft => [-1., 0.],
        TreeOrientation::TopToBottom => [0., 1.],
        TreeOrientation::BottomToTop => [0., -1.],
        TreeOrientation::Radial => [angle.sin(), -angle.cos()],
    };
    let direction = match options.position {
        TreeLabelPosition::Above => [0., -1.],
        TreeLabelPosition::Below => [0., 1.],
        TreeLabelPosition::Left => [-1., 0.],
        TreeLabelPosition::Right => [1., 0.],
        TreeLabelPosition::Center => [0., 0.],
        TreeLabelPosition::Layout => layout,
        TreeLabelPosition::LayoutOpposite => [-layout[0], -layout[1]],
        TreeLabelPosition::Outward => {
            if leaf {
                layout
            } else {
                [-layout[0], -layout[1]]
            }
        }
        TreeLabelPosition::Inward => {
            if leaf {
                [-layout[0], -layout[1]]
            } else {
                layout
            }
        }
    };
    let rotation = match options.rotation {
        TreeLabelRotation::Fixed(rotation) => rotation,
        TreeLabelRotation::AlongLayout => {
            let mut degrees = layout[1].atan2(layout[0]).to_degrees();
            if degrees > 90. {
                degrees -= 180.;
            } else if degrees < -90. {
                degrees += 180.;
            }
            degrees
        }
    };
    let size = options.size.unwrap_or(c.theme.font_size);
    let radians = rotation.to_radians();
    let along = direction[0] * radians.cos() + direction[1] * radians.sin();
    let anchor = if along > 1e-9 {
        TextAnchor::Start
    } else if along < -1e-9 {
        TextAnchor::End
    } else {
        TextAnchor::Middle
    };
    let spacing = if direction == [0., 0.] {
        0.
    } else {
        style.edge(direction, fallback_radius) + options.offset
    };
    let mut x = p.x + direction[0] * spacing;
    let mut y = p.y + direction[1] * spacing;
    if rotation == 0. {
        if direction[1] > 0.5 {
            y += size * 0.8;
        } else if direction[0].abs() > 0.5 || direction == [0., 0.] {
            y += size / 3.;
        }
    } else {
        x -= radians.sin() * size / 3.;
        y += radians.cos() * size / 3.;
    }
    Some(Label {
        x,
        y,
        text: text.into(),
        size,
        color: options.color.unwrap_or(c.theme.text),
        anchor,
        rotation,
    })
}
