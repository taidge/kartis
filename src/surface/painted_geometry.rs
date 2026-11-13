//! Solid-paint geometry for the explicit PaintedGeometry profile only.
//! Lines/rectangles/circles use actual dimensions; cubic centerlines reuse the
//! bounded dash flattener (0.0001 logical positional tolerance). Existing clip
//! paths retain their separately admitted ClipPath picking error profile.
use super::*;
use crate::{ChartError, Color, Geometry, InteractionBinding, StrokeDashLimits};
use glory::render::{GraphicPoint as P, GraphicStroke, StrokeCap};

const TOLERANCE: f64 = 0.0001;
const LENGTH_ERROR: f64 = 0.000001;
const MAX_WORK: usize = crate::MAX_DASH_WORK;
const MAX_SEGMENTS: usize = crate::MAX_DASH_COMMANDS;

#[derive(Default)]
struct Budget {
    work: usize,
    segments: usize,
}
impl Budget {
    fn charge(&mut self, work: usize, segments: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(work)
            .ok_or_else(|| ChartError("paint picking work overflow".into()))?;
        self.segments = self
            .segments
            .checked_add(segments)
            .ok_or_else(|| ChartError("paint picking segment overflow".into()))?;
        ensure(
            self.work <= MAX_WORK && self.segments <= MAX_SEGMENTS,
            "paint picking cumulative budget exceeded",
        )
    }
    fn limits(&self) -> StrokeDashLimits {
        StrokeDashLimits {
            tolerance: TOLERANCE,
            length_error: LENGTH_ERROR,
            max_work: MAX_WORK.saturating_sub(self.work).max(1),
            max_flattened_segments: MAX_SEGMENTS.saturating_sub(self.segments),
            ..Default::default()
        }
    }
    fn stroke_limits(&self, width: f64) -> StrokeDashLimits {
        StrokeDashLimits {
            tolerance: TOLERANCE.min(width / 16.),
            length_error: LENGTH_ERROR.min(width / 16.),
            ..self.limits()
        }
    }
    fn stroke_contours(&mut self, geometry: &Geometry, width: f64) -> Result<Vec<(Vec<P>, bool)>> {
        let result = crate::stroke_dash::picking_contours(geometry, &self.stroke_limits(width))?;
        self.charge(result.work, result.segments)?;
        Ok(result.contours)
    }
    fn contours(&mut self, geometry: &Geometry) -> Result<Vec<(Vec<P>, bool)>> {
        let result = crate::stroke_dash::picking_contours(geometry, &self.limits())?;
        self.charge(result.work, result.segments)?;
        Ok(result.contours)
    }
}
struct PhysicalInk {
    physical: String,
    owner: String,
    series: String,
    fill: Option<Shape>,
    stroke: Option<StrokeInk>,
    clip: Option<ClipRect>,
    clip_path: Option<Box<crate::ClipPath>>,
}
enum Shape {
    Rect(ClipRect),
    Circle { center: P, radius: f64 },
    Path(Vec<(Vec<P>, bool)>),
}
impl Shape {
    fn prepare(geometry: &Geometry, budget: &mut Budget) -> Result<Self> {
        budget.charge(1, 0)?;
        Ok(match geometry {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => Self::Rect(ClipRect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            }),
            Geometry::Circle { x, y, radius } => Self::Circle {
                center: P::new(*x, *y),
                radius: *radius,
            },
            Geometry::Path(_) => Self::Path(budget.contours(geometry)?),
        })
    }
    fn potential(&self) -> bool {
        match self {
            Self::Rect(r) => r.width > 0. && r.height > 0.,
            Self::Circle { radius, .. } => *radius > 0.,
            Self::Path(contours) => contours.iter().any(|(points, _)| noncollinear(points)),
        }
    }
    fn contains(&self, p: P) -> bool {
        match self {
            Self::Rect(r) => r.width > 0. && r.height > 0. && r.contains(p.x, p.y),
            Self::Circle { center, radius } => *radius > 0. && distance(p, *center) <= *radius,
            Self::Path(contours) => {
                let mut winding = 0i64;
                for (points, _) in contours {
                    if !noncollinear(points) {
                        continue;
                    }
                    for (a, b) in points
                        .iter()
                        .zip(points.iter().cycle().skip(1))
                        .take(points.len())
                    {
                        let cross = (b.x - a.x) * (p.y - a.y) - (p.x - a.x) * (b.y - a.y);
                        if cross == 0.
                            && p.x >= a.x.min(b.x)
                            && p.x <= a.x.max(b.x)
                            && p.y >= a.y.min(b.y)
                            && p.y <= a.y.max(b.y)
                        {
                            return true;
                        }
                        if a.y <= p.y {
                            if b.y > p.y && cross > 0. {
                                winding += 1;
                            }
                        } else if b.y <= p.y && cross < 0. {
                            winding -= 1;
                        }
                    }
                }
                winding != 0
            }
        }
    }
}
fn noncollinear(points: &[P]) -> bool {
    let Some(a) = points.first() else {
        return false;
    };
    let Some(b) = points.iter().find(|p| *p != a) else {
        return false;
    };
    points
        .iter()
        .any(|c| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x) != 0.)
}
fn distance(a: P, b: P) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn direction(a: P, b: P) -> [f64; 2] {
    let d = distance(a, b);
    [(b.x - a.x) / d, (b.y - a.y) / d]
}
fn rectangle(p: P, origin: P, t: [f64; 2], lo: f64, hi: f64, half: f64) -> bool {
    let dx = p.x - origin.x;
    let dy = p.y - origin.y;
    let along = dx * t[0] + dy * t[1];
    let across = (-dx * t[1] + dy * t[0]).abs();
    along >= lo && along <= hi && across <= half
}
fn polygon(p: P, points: &[P]) -> bool {
    let mut positive = false;
    let mut negative = false;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        let cross = (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
        positive |= cross > 0.;
        negative |= cross < 0.;
    }
    positive != negative
}
fn join(a: P, b: P, c: P, half: f64, p: P) -> bool {
    let u = direction(a, b);
    let v = direction(b, c);
    let cross = u[0] * v[1] - u[1] * v[0];
    if cross == 0. {
        return false;
    }
    let side = if cross > 0. { -1. } else { 1. };
    let n = [-u[1] * side, u[0] * side];
    let m = [-v[1] * side, v[0] * side];
    let left = P::new(b.x + n[0] * half, b.y + n[1] * half);
    let right = P::new(b.x + m[0] * half, b.y + m[1] * half);
    let denominator = 1. + (u[0] * v[0] + u[1] * v[1]).clamp(-1., 1.);
    if denominator > 0. && (2. / denominator).sqrt() <= 4. {
        let tip = P::new(
            b.x + (n[0] + m[0]) * half / denominator,
            b.y + (n[1] + m[1]) * half / denominator,
        );
        polygon(p, &[b, left, tip, right])
    } else {
        polygon(p, &[b, left, right])
    }
}
fn run_hit(points: &[P], closed: bool, width: f64, cap: StrokeCap, p: P) -> bool {
    let points = if closed && points.len() > 1 && points.first() == points.last() {
        &points[..points.len() - 1]
    } else {
        points
    };
    let Some(first) = points.first().copied() else {
        return false;
    };
    let half = width / 2.;
    if points.len() == 1 {
        return !closed
            && match cap {
                StrokeCap::Butt => false,
                StrokeCap::Round => distance(p, first) <= half,
                StrokeCap::Square => (p.x - first.x).abs() <= half && (p.y - first.y).abs() <= half,
            };
    }
    for pair in points.windows(2) {
        if rectangle(
            p,
            pair[0],
            direction(pair[0], pair[1]),
            0.,
            distance(pair[0], pair[1]),
            half,
        ) {
            return true;
        }
    }
    let last = points.len() - 1;
    if closed
        && rectangle(
            p,
            points[last],
            direction(points[last], first),
            0.,
            distance(points[last], first),
            half,
        )
    {
        return true;
    }
    for triple in points.windows(3) {
        if join(triple[0], triple[1], triple[2], half, p) {
            return true;
        }
    }
    if closed {
        return join(points[last - 1], points[last], first, half, p)
            || join(points[last], first, points[1], half, p);
    }
    match cap {
        StrokeCap::Butt => false,
        StrokeCap::Round => distance(p, first) <= half || distance(p, points[last]) <= half,
        StrokeCap::Square => {
            rectangle(p, first, direction(first, points[1]), -half, 0., half)
                || rectangle(
                    p,
                    points[last],
                    direction(points[last - 1], points[last]),
                    0.,
                    half,
                    half,
                )
        }
    }
}
fn clean_contours(mut contours: Vec<(Vec<P>, bool)>) -> Vec<(Vec<P>, bool)> {
    for (points, _) in &mut contours {
        points.dedup();
    }
    contours
}
enum StrokeInk {
    Circle {
        center: P,
        radius: f64,
        width: f64,
    },
    Runs {
        contours: Vec<(Vec<P>, bool)>,
        stroke: GraphicStroke,
        dots: Vec<Shape>,
    },
}
impl StrokeInk {
    fn prepare(mark: &Mark, budget: &mut Budget) -> Result<Self> {
        let stroke = mark.stroke.expect("admitted positive solid stroke");
        if let Some(dash) = &mark.stroke_dash
            && !dash.normalized()?.segments.is_empty()
        {
            let dashed = crate::dash_geometry_detailed(
                &mark.geometry,
                dash,
                &budget.stroke_limits(stroke.width),
            )?;
            budget.charge(dashed.work, dashed.flattened_segments)?;
            let contours =
                clean_contours(budget.stroke_contours(&dashed.stroke_geometry(), stroke.width)?);
            let mut dots = Vec::new();
            for dot in &dashed.dots {
                let shape = Shape::prepare(
                    &crate::dash_dot_geometry(dot, stroke.width, stroke.cap)?,
                    budget,
                )?;
                if shape.potential() {
                    dots.push(shape);
                }
            }
            return Ok(Self::Runs {
                contours,
                stroke,
                dots,
            });
        }
        if let Geometry::Circle { x, y, radius } = mark.geometry
            && radius > 0.
        {
            budget.charge(1, 0)?;
            return Ok(Self::Circle {
                center: P::new(x, y),
                radius,
                width: stroke.width,
            });
        }
        Ok(Self::Runs {
            contours: clean_contours(budget.stroke_contours(&mark.geometry, stroke.width)?),
            stroke,
            dots: vec![],
        })
    }
    fn potential(&self) -> bool {
        match self {
            Self::Circle { radius, width, .. } => *radius > 0. && *width > 0.,
            Self::Runs {
                contours,
                stroke,
                dots,
            } => {
                !dots.is_empty()
                    || contours.iter().any(|(points, closed)| {
                        points.windows(2).any(|p| p[0] != p[1])
                            || (!closed && points.len() == 1 && stroke.cap != StrokeCap::Butt)
                    })
            }
        }
    }
    fn contains(&self, p: P) -> bool {
        match self {
            Self::Circle {
                center,
                radius,
                width,
            } => (distance(p, *center) - radius).abs() <= width / 2.,
            Self::Runs {
                contours,
                stroke,
                dots,
            } => {
                dots.iter().any(|d| d.contains(p))
                    || contours.iter().any(|(points, closed)| {
                        run_hit(points, *closed, stroke.width, stroke.cap, p)
                    })
            }
        }
    }
}
pub(super) struct PreparedPainted {
    entries: Vec<PhysicalInk>,
    owners: BTreeSet<String>,
}
impl PreparedPainted {
    pub(super) fn prepare(scene: &Scene, binding: &InteractionBinding) -> Result<Self> {
        let sites: BTreeSet<_> = binding.sites.iter().map(|s| s.mark_id.as_str()).collect();
        let mut budget = Budget::default();
        let mut entries = Vec::new();
        let mut owners = BTreeSet::new();
        let groups = scene.mark_group_opacities();
        for original in scene.paint_marks() {
            let owner = if sites.contains(original.id.as_str()) {
                &original.id
            } else if let Some(owner) = binding.ink_owners.get(&original.id) {
                owner
            } else {
                continue;
            };
            ensure(
                original.fill_paint.is_none(),
                "PaintedGeometry currently requires solid eligible fills",
            )?;
            if original.opacity * groups.get(&original.id).copied().unwrap_or(1.) <= 0. {
                continue;
            }
            if original.clip.is_some() {
                budget.charge(1, 0)?;
            }
            if let Some(clip) = &original.clip_path {
                let stats = clip.picking_stats()?;
                budget.charge(stats.work, stats.flattened_segments)?;
            }
            let normalized = crate::render::paint_mark(original)?;
            let mark = normalized.as_ref();
            let fill = if mark.fill.is_some_and(|Color(_, _, _, a)| a > 0) {
                let shape = Shape::prepare(&mark.geometry, &mut budget)?;
                shape.potential().then_some(shape)
            } else {
                None
            };
            let stroke = if mark.stroke.is_some_and(|s| s.color.3 > 0 && s.width > 0.) {
                let stroke = StrokeInk::prepare(mark, &mut budget)?;
                stroke.potential().then_some(stroke)
            } else {
                None
            };
            if fill.is_none() && stroke.is_none() {
                continue;
            }
            owners.insert(owner.clone());
            entries.push(PhysicalInk {
                physical: mark.id.clone(),
                owner: owner.clone(),
                series: mark.series.clone(),
                fill,
                stroke,
                clip: mark.clip,
                clip_path: mark.clip_path.clone(),
            });
        }
        Ok(Self { entries, owners })
    }
    pub(super) fn has_owner(&self, id: &str) -> bool {
        self.owners.contains(id)
    }
    pub(super) fn hit(&self, point: (f64, f64), hidden: &BTreeSet<String>) -> Option<(&str, &str)> {
        let p = P::new(point.0, point.1);
        self.entries
            .iter()
            .rev()
            .find(|e| {
                !hidden.contains(&e.series)
                    && e.clip
                        .is_none_or(|c| c.width > 0. && c.height > 0. && c.contains(p.x, p.y))
                    && e.clip_path.as_ref().is_none_or(|c| c.contains(p.x, p.y))
                    && (e.fill.as_ref().is_some_and(|s| s.contains(p))
                        || e.stroke.as_ref().is_some_and(|s| s.contains(p)))
            })
            .map(|e| (e.physical.as_str(), e.owner.as_str()))
    }
}
