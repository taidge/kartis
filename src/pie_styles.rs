//! Pie presentation follows Nivo pie/src/props.ts and arcs/src/{centers,
//! arc_link_labels/compute}.ts (local checkout). Circular fillet constraints
//! agree with d3-shape v3.2.0 src/arc.js: corner <= half ring thickness,
//! outer <= R sin(halfAngle)/(1+sin(halfAngle)), inner analogous with minus.
//! Fillets are actual tangent circular arcs, represented by bounded cubics;
//! no masks or radius metadata substitute for their geometry.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{FRAC_PI_2, PI, TAU};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use super::{RadialDatum, arc_segment, point, sector};
use crate::scene::*;
use crate::{Color, ValueFormat};

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PieLabelText {
    #[default]
    Label,
    Id,
    Value(ValueFormat),
    /// The formatter receives value / visible total; Percent(p) appends %.
    Percent(ValueFormat),
}
impl PieLabelText {
    pub(crate) fn validate(&self) -> Result<()> {
        match self {
            Self::Value(f) | Self::Percent(f) => f.validate(),
            _ => Ok(()),
        }
    }
    pub(crate) fn text(&self, d: &RadialDatum, fraction: f64) -> Result<String> {
        match self {
            Self::Label => Ok(d.label.clone()),
            Self::Id => Ok(d.id.clone()),
            Self::Value(f) => f.format(d.value),
            Self::Percent(f) => f.format(fraction),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
}
impl Default for PieTextStyle {
    fn default() -> Self {
        Self {
            color: None,
            size: None,
            rotation: 0.,
        }
    }
}
impl PieTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && self.rotation.is_finite()
                && (-360. ..=360.).contains(&self.rotation),
            "invalid pie text style",
        )
    }
}
/// Radius quantity compared with an arc label's `skip_radius` threshold.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArcLabelRadiusMeasure {
    /// Absolute outer radius in chart pixels; preserves the original profiles.
    #[default]
    OuterRadius,
    /// Absolute ring thickness in chart pixels, as in Nivo ArcLabelsLayer.
    RingThickness,
}
impl ArcLabelRadiusMeasure {
    pub fn measure(self, inner_radius: f64, outer_radius: f64) -> f64 {
        match self {
            Self::OuterRadius => outer_radius.abs(),
            Self::RingThickness => (outer_radius - inner_radius).abs(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieArcLabels {
    pub enabled: bool,
    pub text: PieLabelText,
    /// Radians, measured after the existing symmetric angular gap.
    pub skip_angle: f64,
    /// Suppress labels whose selected radius measure is smaller than this.
    pub skip_radius: f64,
    pub radius_measure: ArcLabelRadiusMeasure,
    /// 0 is inner radius, 1 outer radius, 0.5 annular centroid.
    pub radius_offset: f64,
    /// Preserve the earlier extent * radius > font_size * 1.5 admission gate.
    pub legacy_fit: bool,
    pub style: PieTextStyle,
}
impl Default for PieArcLabels {
    fn default() -> Self {
        Self {
            enabled: true,
            text: PieLabelText::Label,
            skip_angle: 0.,
            skip_radius: 0.,
            radius_measure: Default::default(),
            radius_offset: 0.5,
            legacy_fit: true,
            style: Default::default(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PieLinkColor {
    #[default]
    Theme,
    Slice,
    Fixed(Color),
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieLinkLabels {
    pub enabled: bool,
    pub text: PieLabelText,
    pub skip_angle: f64,
    /// Radial displacement from the actual outer radius. May be negative.
    pub offset: f64,
    pub diagonal_length: f64,
    pub straight_length: f64,
    pub thickness: f64,
    pub color: PieLinkColor,
    pub text_offset: f64,
    pub style: PieTextStyle,
}
impl Default for PieLinkLabels {
    fn default() -> Self {
        Self {
            enabled: false,
            text: PieLabelText::Id,
            skip_angle: 0.,
            offset: 0.,
            diagonal_length: 16.,
            straight_length: 24.,
            thickness: 1.,
            color: Default::default(),
            text_offset: 6.,
            style: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieStyleOptions {
    pub corner_radius: f64,
    pub border: Option<GraphicStroke>,
    /// Fill alpha multiplier; the border keeps its own color alpha.
    pub fill_opacity: f64,
    pub active_ids: BTreeSet<String>,
    /// Positive values reduce the hole, capped at zero; negative values enlarge it.
    pub active_inner_radius_offset: f64,
    pub active_outer_radius_offset: f64,
    pub arc_labels: PieArcLabels,
    pub link_labels: PieLinkLabels,
}
impl Default for PieStyleOptions {
    fn default() -> Self {
        Self {
            corner_radius: 0.,
            border: None,
            fill_opacity: 1.,
            active_ids: Default::default(),
            active_inner_radius_offset: 0.,
            active_outer_radius_offset: 0.,
            arc_labels: Default::default(),
            link_labels: Default::default(),
        }
    }
}
fn bounded(v: f64) -> bool {
    v.is_finite() && (0. ..=16384.).contains(&v)
}
impl PieStyleOptions {
    pub(crate) fn validate(&self, data: &[RadialDatum]) -> Result<()> {
        ensure(
            bounded(self.corner_radius)
                && self.fill_opacity.is_finite()
                && (0. ..=1.).contains(&self.fill_opacity)
                && self
                    .border
                    .is_none_or(|b| b.width.is_finite() && (0.000001..=10000.).contains(&b.width))
                && [
                    self.active_inner_radius_offset,
                    self.active_outer_radius_offset,
                ]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 16384.)
                && self.active_ids.len() <= 256
                && self
                    .active_ids
                    .iter()
                    .all(|id| data.iter().any(|d| &d.id == id)),
            "invalid pie slice style or active identity",
        )?;
        let a = &self.arc_labels;
        let l = &self.link_labels;
        ensure(
            a.skip_angle.is_finite()
                && (0. ..=TAU).contains(&a.skip_angle)
                && bounded(a.skip_radius)
                && a.radius_offset.is_finite()
                && (0. ..=1.).contains(&a.radius_offset)
                && l.skip_angle.is_finite()
                && (0. ..=TAU).contains(&l.skip_angle)
                && l.offset.is_finite()
                && l.offset.abs() <= 16384.
                && [
                    l.diagonal_length,
                    l.straight_length,
                    l.thickness,
                    l.text_offset,
                ]
                .into_iter()
                .all(bounded),
            "invalid pie label geometry",
        )?;
        a.style.validate()?;
        l.style.validate()?;
        a.text.validate()?;
        l.text.validate()
    }
}
pub(super) fn leader_ids(data: &[RadialDatum]) -> BTreeMap<String, String> {
    let mut occupied = data.iter().map(|d| d.id.clone()).collect::<BTreeSet<_>>();
    let mut result = BTreeMap::new();
    for id in data.iter().map(|d| &d.id).collect::<BTreeSet<_>>() {
        let base = format!("pie-leader:{}:{id}", id.len());
        let mut candidate = base.clone();
        let mut salt = 0;
        while occupied.contains(&candidate) {
            salt += 1;
            candidate = format!("{base}:{salt}");
        }
        occupied.insert(candidate.clone());
        result.insert(id.clone(), candidate);
    }
    result
}
pub(super) struct PieLabelGeometry<'a> {
    pub center: [f64; 2],
    pub inner: f64,
    pub outer: f64,
    pub middle: f64,
    pub extent: f64,
    pub fraction: f64,
    pub arc_enabled: bool,
    pub leader_id: &'a str,
    pub slice_color: Color,
}
pub(super) fn append_labels(
    s: &mut Scene,
    c: &ChartConfig,
    d: &RadialDatum,
    o: &PieStyleOptions,
    g: PieLabelGeometry<'_>,
) -> Result<()> {
    let a = &o.arc_labels;
    let l = &o.link_labels;
    let [cx, cy] = g.center;
    if g.arc_enabled
        && a.enabled
        && g.extent >= a.skip_angle
        && a.radius_measure.measure(g.inner, g.outer) >= a.skip_radius
        && (!a.legacy_fit || g.extent * g.outer > a.style.size.unwrap_or(c.theme.font_size) * 1.5)
    {
        let p = point(
            cx,
            cy,
            g.inner + (g.outer - g.inner) * a.radius_offset,
            g.middle,
        );
        let index = s.labels.len();
        s.labels.push(Label {
            x: p.x,
            y: p.y,
            text: a.text.text(d, g.fraction)?,
            size: a.style.size.unwrap_or(c.theme.font_size),
            color: a.style.color.unwrap_or(c.theme.text),
            anchor: TextAnchor::Middle,
            rotation: a.style.rotation,
        });
        s.label_series.insert(index, d.id.clone());
    }
    if l.enabled && g.extent >= l.skip_angle {
        let radius = g.outer + l.offset;
        ensure(radius >= 0., "pie leader offset crosses the origin")?;
        let first = point(cx, cy, radius, g.middle);
        let elbow = point(cx, cy, radius + l.diagonal_length, g.middle);
        let side = if g.middle.sin() >= 0. { 1. } else { -1. };
        let end = P::new(elbow.x + side * l.straight_length, elbow.y);
        if l.thickness > 0. && (l.diagonal_length > 0. || l.straight_length > 0.) {
            let color = match l.color {
                PieLinkColor::Theme => c.theme.grid,
                PieLinkColor::Slice => g.slice_color,
                PieLinkColor::Fixed(color) => color,
            };
            let mut mark = Mark::new(
                g.leader_id,
                Geometry::Path(vec![C::Move(first), C::Line(elbow), C::Line(end)]),
                color,
            );
            mark.fill = None;
            mark.stroke = Some(GraphicStroke {
                color,
                width: l.thickness,
                cap: StrokeCap::Butt,
            });
            mark.label.clear();
            mark.series = d.id.clone();
            s.marks.push(mark);
        }
        let index = s.labels.len();
        s.labels.push(Label {
            x: end.x + side * l.text_offset,
            y: end.y,
            text: l.text.text(d, g.fraction)?,
            size: l.style.size.unwrap_or(c.theme.font_size),
            color: l.style.color.unwrap_or(c.theme.text),
            anchor: if side > 0. {
                TextAnchor::Start
            } else {
                TextAnchor::End
            },
            rotation: l.style.rotation,
        });
        s.label_series.insert(index, d.id.clone());
    }
    Ok(())
}

// Circular paths use the same <=90 degree cubic representation as the shared
// sector helper. The angular approximation is at most 0.000273 * radius.
fn circular(out: &mut Vec<C>, center: P, r: f64, start: f64, end: f64, direction: f64) {
    let mut delta = end - start;
    if direction > 0. {
        while delta < 0. {
            delta += TAU;
        }
    } else {
        while delta > 0. {
            delta -= TAU;
        }
    }
    if delta.abs() < 1e-14 {
        return;
    }
    arc_segment(
        out,
        center.x,
        center.y,
        r,
        start + FRAC_PI_2,
        start + delta + FRAC_PI_2,
    );
}
fn fillet(out: &mut Vec<C>, center: P, r: f64, from: P, to: P, direction: f64) {
    circular(
        out,
        center,
        r,
        (from.y - center.y).atan2(from.x - center.x),
        (to.y - center.y).atan2(to.x - center.x),
        direction,
    );
    if let Some(C::Cubic { end, .. }) = out.last_mut() {
        *end = to;
    }
}
/// Fillet construction for unpadded radial boundaries; existing Pie angular
/// padding is applied before this call, preserving that API's gap semantics.
pub(crate) fn rounded_sector(
    cx: f64,
    cy: f64,
    inner: f64,
    outer: f64,
    start: f64,
    end: f64,
    corner: f64,
) -> Geometry {
    let span = (end - start).abs();
    let dir = (end - start).signum();
    if span >= TAU - 1e-12 {
        let mut out = vec![C::Move(point(cx, cy, outer, start))];
        arc_segment(&mut out, cx, cy, outer, start, start + dir * TAU);
        out.push(C::Close);
        if inner > 0. {
            out.push(C::Move(point(cx, cy, inner, start + dir * TAU)));
            arc_segment(&mut out, cx, cy, inner, start + dir * TAU, start);
            out.push(C::Close);
        }
        return Geometry::Path(out);
    }
    let limit = corner.min((outer - inner) / 2.);
    if limit <= 1e-12 {
        return sector(cx, cy, inner, outer, start, end);
    }
    let sin = (span / 2.).sin();
    let co = if span < PI {
        limit.min(outer * sin / (1. + sin))
    } else {
        limit
    };
    let ci = if inner <= 0. {
        0.
    } else if span < PI {
        limit.min(inner * sin / (1. - sin))
    } else {
        limit
    };
    let od = (co / (outer - co)).clamp(0., 1.).asin();
    let ot = (outer * (outer - 2. * co)).max(0.).sqrt();
    let a = start + dir * od;
    let b = end - dir * od;
    let os = point(cx, cy, ot, start);
    let oe = point(cx, cy, ot, end);
    let ca = point(cx, cy, outer - co, a);
    let cb = point(cx, cy, outer - co, b);
    let oa = point(cx, cy, outer, a);
    let ob = point(cx, cy, outer, b);
    let mut out = vec![C::Move(os)];
    fillet(&mut out, ca, co, os, oa, dir);
    if span - 2. * od > 1e-12 {
        arc_segment(&mut out, cx, cy, outer, a, b);
    }
    fillet(&mut out, cb, co, ob, oe, dir);
    if ci > 1e-12 {
        let delta = (ci / (inner + ci)).clamp(0., 1.).asin();
        let t = (inner * (inner + 2. * ci)).sqrt();
        let a = end - dir * delta;
        let b = start + dir * delta;
        let first = point(cx, cy, t, end);
        let last = point(cx, cy, t, start);
        let ca = point(cx, cy, inner + ci, a);
        let cb = point(cx, cy, inner + ci, b);
        let ia = point(cx, cy, inner, a);
        let ib = point(cx, cy, inner, b);
        out.push(C::Line(first));
        fillet(&mut out, ca, ci, first, ia, dir);
        if span - 2. * delta > 1e-12 {
            arc_segment(&mut out, cx, cy, inner, a, b);
        }
        fillet(&mut out, cb, ci, ib, last, dir);
    } else {
        out.push(C::Line(point(cx, cy, inner, end)));
        if inner > 0. {
            arc_segment(&mut out, cx, cy, inner, end, start);
        }
    }
    out.push(C::Close);
    Geometry::Path(out)
}
