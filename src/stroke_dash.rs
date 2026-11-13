//! Bounded SVG-style dash centerlines, without a renderer or scheduler.
//! Primary algorithm: SVG 2 painting, stroke-dasharray/dashoffset and dash
//! positions: <https://www.w3.org/TR/SVG2/painting.html#StrokeDashing>.
//! Lengths are logical user units (no percentages or pathLength normalization).
use std::borrow::Cow;

use glory::render::{GraphicPoint as P, PathCommand as C, StrokeCap};

use crate::scene::ensure;
use crate::{ChartError, Geometry, Result};

pub const MAX_DASH_SEGMENTS: usize = 64;
pub const MAX_DASH_COMMANDS: usize = 200_000;
pub const MAX_DASH_WORK: usize = 4_000_000;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct StrokeDash {
    pub segments: Vec<f64>,
    pub offset: f64,
}
impl StrokeDash {
    /// Normalize before handing values to browser SVG/Canvas engines, whose
    /// native dash storage can saturate at f32::MAX. Pure expansion retains
    /// its wider f64 input profile.
    pub fn native_normalized(&self) -> Result<Self> {
        let dash = self.normalized()?;
        ensure(
            dash.offset <= f64::from(f32::MAX)
                && dash.segments.iter().all(|v| *v <= f64::from(f32::MAX)),
            "native SVG/Canvas dash exceeds the f32 range; use pure geometry or CPU raster",
        )?;
        Ok(dash)
    }
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.segments.len() <= MAX_DASH_SEGMENTS
                && self.offset.is_finite()
                && self
                    .segments
                    .iter()
                    .all(|value| value.is_finite() && *value >= 0.),
            "invalid stroke dash pattern",
        )?;
        let sum = self.segments.iter().sum::<f64>();
        ensure(
            sum.is_finite() && (self.segments.len().is_multiple_of(2) || (sum * 2.).is_finite()),
            "stroke dash period overflow",
        )
    }
    /// Empty/all-zero means solid. Odd patterns repeat to even length, with
    /// positive/negative offsets reduced modulo the resulting period.
    pub fn normalized(&self) -> Result<Self> {
        self.validate()?;
        let mut segments = self.segments.clone();
        if segments.iter().all(|value| *value == 0.) {
            return Ok(Self::default());
        }
        if !segments.len().is_multiple_of(2) {
            segments.extend(self.segments.iter().copied());
        }
        let period = segments.iter().sum::<f64>();
        let offset = self.offset.rem_euclid(period);
        Ok(Self {
            segments,
            offset: if offset == period || offset == 0. {
                0.
            } else {
                offset
            },
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StrokeDashLimits {
    /// Maximum control-point distance to an accepted cubic leaf chord.
    pub tolerance: f64,
    /// Global upper bound on omitted cubic arc length, using the sum of
    /// control-polygon minus chord lengths; split among every source cubic.
    pub length_error: f64,
    pub max_output_commands: usize,
    pub max_flattened_segments: usize,
    pub max_work: usize,
}
impl Default for StrokeDashLimits {
    fn default() -> Self {
        Self {
            tolerance: 0.1,
            length_error: 0.01,
            max_output_commands: MAX_DASH_COMMANDS,
            max_flattened_segments: MAX_DASH_COMMANDS,
            max_work: MAX_DASH_WORK,
        }
    }
}
impl StrokeDashLimits {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.tolerance.is_finite()
                && (1e-9..=10000.).contains(&self.tolerance)
                && self.length_error.is_finite()
                && (1e-9..=10000.).contains(&self.length_error)
                && self.max_output_commands <= MAX_DASH_COMMANDS
                && self.max_flattened_segments <= MAX_DASH_COMMANDS
                && self.max_work > 0
                && self.max_work <= MAX_DASH_WORK,
            "invalid stroke dash limits",
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DashDot {
    pub point: P,
    /// Unit directions on either side; a completely zero-length source uses
    /// `[1,0]`. Retained for backends which orient square zero-dash cap shapes.
    pub incoming: [f64; 2],
    pub outgoing: [f64; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct DashedGeometry {
    /// Stroke-only geometry. Do not replace the original geometry's fill.
    pub geometry: Geometry,
    /// Zero-length painted runs also appear as Move+Line at the same point.
    /// That centerline suffices for round/butt caps; a square cap backend must
    /// use these source tangents rather than inventing an axis-aligned tangent.
    pub dots: Vec<DashDot>,
    pub length_error_bound: f64,
    pub flattened_segments: usize,
    pub work: usize,
}
impl DashedGeometry {
    /// Stroke runs with zero-length subpaths removed. Paint `dots` separately
    /// with `dash_dot_geometry` when a backend cannot orient their caps.
    pub fn stroke_geometry(&self) -> Geometry {
        let Geometry::Path(commands) = &self.geometry else {
            return self.geometry.clone();
        };
        let mut result = Vec::with_capacity(commands.len());
        let mut start = 0;
        while start < commands.len() {
            let end = (start + 1..commands.len())
                .find(|i| matches!(commands[*i], C::Move(_)))
                .unwrap_or(commands.len());
            let sub = &commands[start..end];
            let zero = matches!(sub, [C::Move(a), C::Line(b)] if a == b);
            if !zero {
                result.extend_from_slice(sub);
            }
            start = end;
        }
        Geometry::Path(result)
    }
}
/// Filled footprint of a zero dash for a backend which cannot stroke a
/// zero-length centerline. Square uses the original incoming/outgoing cap
/// directions; two half-rectangles form a square on a straight segment.
/// Footprints can extend beyond centerline coordinates by half the width, just
/// like any stroke; callers retain their existing viewport/mark clips.
pub fn dash_dot_geometry(dot: &DashDot, width: f64, cap: StrokeCap) -> Result<Geometry> {
    ensure(
        width.is_finite()
            && (0. ..=10000.).contains(&width)
            && [dot.point.x, dot.point.y]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1e6),
        "invalid zero dash footprint",
    )?;
    let unit = |t: [f64; 2]| -> Result<[f64; 2]> {
        let length = t[0].hypot(t[1]);
        ensure(
            t.iter().all(|v| v.is_finite()) && length.is_finite() && length > 0.,
            "invalid zero dash tangent",
        )?;
        Ok([t[0] / length, t[1] / length])
    };
    let incoming = unit(dot.incoming)?;
    let outgoing = unit(dot.outgoing)?;
    if width == 0. || cap == StrokeCap::Butt {
        return Ok(Geometry::Path(vec![]));
    }
    let half = width / 2.;
    if cap == StrokeCap::Round {
        return Ok(Geometry::Circle {
            x: dot.point.x,
            y: dot.point.y,
            radius: half,
        });
    }
    let mut path = Vec::with_capacity(10);
    for (t, sign) in [(outgoing, -1.), (incoming, 1.)] {
        let n = [-t[1], t[0]];
        let points = if sign < 0. {
            [
                [n[0], n[1]],
                [-t[0] + n[0], -t[1] + n[1]],
                [-t[0] - n[0], -t[1] - n[1]],
                [-n[0], -n[1]],
            ]
        } else {
            [
                [-n[0], -n[1]],
                [t[0] - n[0], t[1] - n[1]],
                [t[0] + n[0], t[1] + n[1]],
                [n[0], n[1]],
            ]
        };
        for (i, p) in points.into_iter().enumerate() {
            let p = P::new(dot.point.x + half * p[0], dot.point.y + half * p[1]);
            path.push(if i == 0 { C::Move(p) } else { C::Line(p) });
        }
        path.push(C::Close);
    }
    Ok(Geometry::Path(path))
}
struct Budget<'a> {
    limits: &'a StrokeDashLimits,
    work: usize,
    segments: usize,
    error: f64,
}
impl Budget<'_> {
    fn spend(&mut self, n: usize) -> Result<()> {
        self.work = self.work.saturating_add(n);
        ensure(
            self.work <= self.limits.max_work,
            "stroke dash work budget exceeded",
        )
    }
    fn segment(&mut self) -> Result<()> {
        self.segments = self.segments.saturating_add(1);
        ensure(
            self.segments <= self.limits.max_flattened_segments,
            "stroke dash flatten budget exceeded",
        )
    }
}
#[derive(Default)]
struct Contour {
    points: Vec<P>,
    closed: bool,
    drawable: bool,
}
fn distance(a: P, b: P) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn middle(a: P, b: P) -> P {
    P::new((a.x + b.x) / 2., (a.y + b.y) / 2.)
}
fn flatness(a: P, d: P, b: P) -> f64 {
    let length = distance(a, d);
    if length == 0. {
        distance(a, b)
    } else {
        ((d.x - a.x) * (a.y - b.y) - (a.x - b.x) * (d.y - a.y)).abs() / length
    }
}
fn cubic(
    control: [P; 4],
    allowance: f64,
    depth: usize,
    out: &mut Vec<P>,
    budget: &mut Budget<'_>,
) -> Result<()> {
    budget.spend(1)?;
    let [a, b, c, d] = control;
    let excess = (distance(a, b) + distance(b, c) + distance(c, d) - distance(a, d)).max(0.);
    if excess <= allowance && flatness(a, d, b).max(flatness(a, d, c)) <= budget.limits.tolerance {
        budget.segment()?;
        budget.error += excess;
        out.push(d);
        return Ok(());
    }
    ensure(depth < 24, "stroke dash cubic subdivision budget exceeded")?;
    let ab = middle(a, b);
    let bc = middle(b, c);
    let cd = middle(c, d);
    let abc = middle(ab, bc);
    let bcd = middle(bc, cd);
    let mid = middle(abc, bcd);
    cubic([a, ab, abc, mid], allowance / 2., depth + 1, out, budget)?;
    cubic([mid, bcd, cd, d], allowance / 2., depth + 1, out, budget)
}
fn source(geometry: &Geometry, max_work: usize) -> Result<Cow<'_, [C]>> {
    let coordinate = |v: f64| v.is_finite() && v.abs() <= 1e6;
    match geometry {
        Geometry::Path(path) => {
            ensure(
                path.len() <= MAX_DASH_COMMANDS,
                "stroke dash source command budget exceeded",
            )?;
            ensure(path.len() <= max_work, "stroke dash work budget exceeded")?;
            let mut active = false;
            for command in path {
                match *command {
                    C::Move(p) => {
                        ensure(
                            coordinate(p.x) && coordinate(p.y),
                            "invalid stroke dash point",
                        )?;
                        active = true;
                    }
                    C::Line(p) => ensure(
                        active && coordinate(p.x) && coordinate(p.y),
                        "invalid stroke dash segment",
                    )?,
                    C::Cubic { first, second, end } => ensure(
                        active
                            && [first, second, end]
                                .iter()
                                .all(|p| coordinate(p.x) && coordinate(p.y)),
                        "invalid stroke dash cubic",
                    )?,
                    C::Close => {
                        ensure(active, "invalid stroke dash close")?;
                        active = false;
                    }
                }
            }
            Ok(Cow::Borrowed(path))
        }
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => {
            ensure(
                [*x, *y, *width, *height, x + width, y + height]
                    .into_iter()
                    .all(coordinate)
                    && *width >= 0.
                    && *height >= 0.,
                "invalid stroke dash rectangle",
            )?;
            Ok(Cow::Owned(geometry.commands()))
        }
        Geometry::Circle { x, y, radius } => {
            ensure(
                [
                    *x,
                    *y,
                    *radius,
                    x - radius,
                    x + radius,
                    y - radius,
                    y + radius,
                ]
                .into_iter()
                .all(coordinate)
                    && *radius >= 0.,
                "invalid stroke dash circle",
            )?;
            Ok(Cow::Owned(geometry.commands()))
        }
    }
}
fn flatten(commands: &[C], budget: &mut Budget<'_>) -> Result<Vec<Contour>> {
    let cubics = commands
        .iter()
        .filter(|c| matches!(c, C::Cubic { .. }))
        .count();
    let allowance = budget.limits.length_error / cubics.max(1) as f64;
    let mut result = Vec::new();
    let mut contour = Contour::default();
    for command in commands {
        budget.spend(1)?;
        match *command {
            C::Move(point) => {
                if !contour.points.is_empty() {
                    result.push(std::mem::take(&mut contour));
                }
                contour.points.push(point);
            }
            C::Line(point) => {
                budget.segment()?;
                contour.points.push(point);
                contour.drawable = true;
            }
            C::Cubic { first, second, end } => {
                let start = *contour
                    .points
                    .last()
                    .ok_or_else(|| ChartError("dash cubic without Move".into()))?;
                cubic(
                    [start, first, second, end],
                    allowance,
                    0,
                    &mut contour.points,
                    budget,
                )?;
                contour.drawable = true;
            }
            C::Close => {
                budget.segment()?;
                contour.points.push(contour.points[0]);
                contour.closed = true;
                contour.drawable = true;
                result.push(std::mem::take(&mut contour));
            }
        }
    }
    if !contour.points.is_empty() {
        result.push(contour);
    }
    Ok(result)
}
struct Arc {
    points: Vec<P>,
    lengths: Vec<f64>,
    closed: bool,
}
impl Arc {
    fn new(contour: Contour) -> Result<Self> {
        let mut lengths = vec![0.];
        for pair in contour.points.windows(2) {
            let length = distance(pair[0], pair[1]);
            let old = *lengths.last().unwrap();
            let next = old + length;
            ensure(
                length == 0. || next > old,
                "stroke dash arc length loses numeric precision",
            )?;
            lengths.push(next);
        }
        Ok(Self {
            points: contour.points,
            lengths,
            closed: contour.closed,
        })
    }
    fn length(&self) -> f64 {
        *self.lengths.last().unwrap()
    }
    fn index(&self, d: f64, budget: &mut Budget<'_>) -> Result<usize> {
        budget.spend(self.lengths.len().ilog2() as usize + 1)?;
        Ok(self
            .lengths
            .partition_point(|length| *length < d)
            .min(self.lengths.len() - 1))
    }
    fn point(&self, d: f64, budget: &mut Budget<'_>) -> Result<P> {
        let index = self.index(d, budget)?;
        if index == 0 {
            return Ok(self.points[0]);
        }
        let a = self.points[index - 1];
        let b = self.points[index];
        let span = self.lengths[index] - self.lengths[index - 1];
        let t = if span == 0. {
            0.
        } else {
            (d - self.lengths[index - 1]) / span
        };
        Ok(P::new(a.x * (1. - t) + b.x * t, a.y * (1. - t) + b.y * t))
    }
    fn tangent(&self, d: f64, outgoing: bool, budget: &mut Budget<'_>) -> Result<[f64; 2]> {
        let end = self.index(d, budget)?;
        let mut index = if outgoing && self.lengths[end] == d {
            end
        } else {
            end.saturating_sub(1)
        };
        for _ in 0..self.points.len() {
            budget.spend(1)?;
            if index + 1 < self.points.len() {
                let a = self.points[index];
                let b = self.points[index + 1];
                let length = distance(a, b);
                if length > 0. {
                    return Ok([(b.x - a.x) / length, (b.y - a.y) / length]);
                }
            }
            if outgoing {
                if index + 1 < self.points.len() - 1 {
                    index += 1;
                } else if self.closed {
                    index = 0;
                } else {
                    break;
                }
            } else if index > 0 {
                index -= 1;
            } else if self.closed {
                index = self.points.len().saturating_sub(2);
            } else {
                break;
            }
        }
        Ok([1., 0.])
    }
}
fn intervals(length: f64, dash: &StrokeDash, budget: &mut Budget<'_>) -> Result<Vec<[f64; 2]>> {
    let mut sum = 0.;
    let mut index = 0;
    for (i, value) in dash.segments.iter().enumerate() {
        budget.spend(1)?;
        sum += value;
        index = i;
        if sum >= dash.offset {
            break;
        }
    }
    let mut remaining = (sum - dash.offset).min(length);
    let mut position = 0.;
    let mut output = Vec::new();
    loop {
        budget.spend(1)?;
        let next = (position + remaining).min(length);
        ensure(
            remaining == 0. || next > position,
            "stroke dash advance loses numeric precision",
        )?;
        if index % 2 == 0 {
            ensure(
                output.len() < budget.limits.max_output_commands / 2,
                "stroke dash output command budget exceeded",
            )?;
            output.push([position, next]);
        }
        position = next;
        if position >= length {
            break;
        }
        index = (index + 1) % dash.segments.len();
        remaining = dash.segments[index].min(length - position);
    }
    Ok(output)
}
fn push(path: &mut Vec<C>, command: C, budget: &mut Budget<'_>) -> Result<()> {
    budget.spend(1)?;
    ensure(
        path.len() < budget.limits.max_output_commands,
        "stroke dash output command budget exceeded",
    )?;
    path.push(command);
    Ok(())
}
fn append_run(
    arc: &Arc,
    start: f64,
    end: f64,
    path: &mut Vec<C>,
    move_to: bool,
    budget: &mut Budget<'_>,
) -> Result<()> {
    let first = arc.point(start, budget)?;
    if move_to {
        push(path, C::Move(first), budget)?;
    }
    let from = arc.index(start, budget)?;
    let to = arc.index(end, budget)?;
    for index in from..to {
        budget.spend(1)?;
        if arc.lengths[index] > start && arc.lengths[index] < end {
            push(path, C::Line(arc.points[index]), budget)?;
        }
    }
    push(path, C::Line(arc.point(end, budget)?), budget)
}
fn append_closed(arc: &Arc, path: &mut Vec<C>, budget: &mut Budget<'_>) -> Result<()> {
    push(path, C::Move(arc.points[0]), budget)?;
    for point in arc
        .points
        .iter()
        .skip(1)
        .take(arc.points.len().saturating_sub(2))
    {
        push(path, C::Line(*point), budget)?;
    }
    push(path, C::Close, budget)
}
pub(crate) struct PickingContours {
    pub contours: Vec<(Vec<P>, bool)>,
    pub work: usize,
    pub segments: usize,
}
/// Reuse the dash module's admitted adaptive centerline flattening without
/// inventing a dash pattern. Used only by opt-in painted-geometry picking.
pub(crate) fn picking_contours(
    geometry: &Geometry,
    limits: &StrokeDashLimits,
) -> Result<PickingContours> {
    limits.validate()?;
    // The caller has admitted the Scene, or obtained a renderer-normalized
    // point/dash footprint. Such derived Path vertices may extend beyond the
    // source coordinate envelope by half a valid stroke width. Do not reject
    // that genuine ink by reapplying the source-only coordinate bound.
    let commands = match geometry {
        Geometry::Path(path) => Cow::Borrowed(path.as_slice()),
        _ => source(geometry, limits.max_work)?,
    };
    ensure(
        commands.len() <= limits.max_output_commands,
        "paint picking source command budget exceeded",
    )?;
    let mut budget = Budget {
        limits,
        work: 0,
        segments: 0,
        error: 0.,
    };
    budget.spend(commands.len())?;
    let contours = flatten(&commands, &mut budget)?;
    let contours = contours
        .into_iter()
        .filter(|c| c.drawable)
        .map(|c| (c.points, c.closed))
        .collect();
    Ok(PickingContours {
        contours,
        work: budget.work,
        segments: budget.segments,
    })
}

/// Stroke-only centerline geometry. For zero-dash square caps use the detailed
/// result's source tangents; round/butt zero caps work directly on this path.
pub fn dash_geometry(
    geometry: &Geometry,
    dash: &StrokeDash,
    limits: &StrokeDashLimits,
) -> Result<Geometry> {
    Ok(dash_geometry_detailed(geometry, dash, limits)?.geometry)
}
pub fn dash_geometry_detailed(
    geometry: &Geometry,
    dash: &StrokeDash,
    limits: &StrokeDashLimits,
) -> Result<DashedGeometry> {
    limits.validate()?;
    let dash = dash.normalized()?;
    let commands = source(geometry, limits.max_work)?;
    let mut budget = Budget {
        limits,
        work: 0,
        segments: 0,
        error: 0.,
    };
    budget.spend(commands.len())?;
    if dash.segments.is_empty() {
        ensure(
            commands.len() <= limits.max_output_commands,
            "stroke dash output command budget exceeded",
        )?;
        return Ok(DashedGeometry {
            geometry: geometry.clone(),
            dots: vec![],
            length_error_bound: 0.,
            flattened_segments: 0,
            work: budget.work,
        });
    }
    let contours = flatten(&commands, &mut budget)?;
    let mut path = Vec::new();
    let mut dots = Vec::new();
    for contour in contours {
        if !contour.drawable {
            continue;
        }
        budget.spend(contour.points.len())?;
        let arc = Arc::new(contour)?;
        let length = arc.length();
        let mut runs = intervals(length, &dash, &mut budget)?;
        if arc.closed && runs.len() == 1 && runs[0] == [0., length] && length > 0. {
            append_closed(&arc, &mut path, &mut budget)?;
            continue;
        }
        if arc.closed
            && runs.len() > 1
            && runs[0][0] == 0.
            && runs.last().unwrap()[1] == length
            && runs[0][1] > 0.
            && runs.last().unwrap()[0] < length
        {
            let last = runs.pop().unwrap();
            let first = runs.remove(0);
            append_run(&arc, last[0], last[1], &mut path, true, &mut budget)?;
            append_run(&arc, first[0], first[1], &mut path, false, &mut budget)?;
        }
        for [start, end] in runs {
            if arc.closed && start == 0. && end == length && length > 0. {
                append_closed(&arc, &mut path, &mut budget)?;
                continue;
            }
            append_run(&arc, start, end, &mut path, true, &mut budget)?;
            if start == end {
                dots.push(DashDot {
                    point: arc.point(start, &mut budget)?,
                    incoming: arc.tangent(start, false, &mut budget)?,
                    outgoing: arc.tangent(start, true, &mut budget)?,
                });
            }
        }
    }
    Ok(DashedGeometry {
        geometry: Geometry::Path(path),
        dots,
        length_error_bound: budget.error,
        flattened_segments: budget.segments,
        work: budget.work,
    })
}
