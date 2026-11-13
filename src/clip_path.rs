//! Explicit fill-rule clip paths; drawing and picking share logical coordinates.
use glory::render::{GraphicPoint as P, PathCommand as C};

use crate::scene::ensure;
use crate::*;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClipRule {
    #[default]
    NonZero,
    EvenOdd,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ClipPath {
    pub geometry: Geometry,
    pub rule: ClipRule,
}
pub const MAX_CLIP_COMMANDS: usize = 4096;
pub const CLIP_PICK_TOLERANCE: f64 = 0.05;
pub const MAX_CLIP_PICK_SEGMENTS: usize = 65_536;
pub const MAX_CLIP_PICK_WORK: usize = 262_144;
const MAX_CLIP_PICK_DEPTH: usize = 20;
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ClipPickingStats {
    pub flattened_segments: usize,
    pub work: usize,
    pub max_error_bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RectCoverage {
    Outside,
    Partial,
    Full,
}
fn coordinate(value: f64) -> bool {
    value.is_finite() && value.abs() <= 1e6
}
fn point_valid(point: P) -> bool {
    coordinate(point.x) && coordinate(point.y)
}
fn distance_to_segment(point: P, a: P, b: P) -> f64 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let squared = dx * dx + dy * dy;
    let t = if squared == 0. {
        0.
    } else {
        ((point.x - a.x) * dx + (point.y - a.y) * dy) / squared
    }
    .clamp(0., 1.);
    (point.x - a.x - t * dx).hypot(point.y - a.y - t * dy)
}
fn clip_spend(stats: &mut ClipPickingStats) -> Result<()> {
    stats.work += 1;
    ensure(
        stats.work <= MAX_CLIP_PICK_WORK,
        "clip picking work budget exceeded",
    )
}
fn line(
    a: P,
    b: P,
    error: f64,
    stats: &mut ClipPickingStats,
    visit: &mut impl FnMut(P, P, f64),
) -> Result<()> {
    clip_spend(stats)?;
    stats.flattened_segments += 1;
    ensure(
        stats.flattened_segments <= MAX_CLIP_PICK_SEGMENTS,
        "clip picking flatten budget exceeded",
    )?;
    stats.max_error_bound = stats.max_error_bound.max(error);
    visit(a, b, error);
    Ok(())
}
fn cubic(
    points: [P; 4],
    depth: usize,
    stats: &mut ClipPickingStats,
    visit: &mut impl FnMut(P, P, f64),
) -> Result<()> {
    clip_spend(stats)?;
    let [a, b, c, d] = points;
    let error = distance_to_segment(b, a, d).max(distance_to_segment(c, a, d));
    if error <= CLIP_PICK_TOLERANCE / 2. {
        return line(a, d, error, stats, visit);
    }
    ensure(
        depth < MAX_CLIP_PICK_DEPTH,
        "clip picking subdivision budget exceeded",
    )?;
    let mid = |a: P, b: P| P::new((a.x + b.x) / 2., (a.y + b.y) / 2.);
    let ab = mid(a, b);
    let bc = mid(b, c);
    let cd = mid(c, d);
    let abc = mid(ab, bc);
    let bcd = mid(bc, cd);
    let m = mid(abc, bcd);
    cubic([a, ab, abc, m], depth + 1, stats, visit)?;
    cubic([m, bcd, cd, d], depth + 1, stats, visit)
}
fn visit_path(path: &[C], mut visit: impl FnMut(P, P, f64)) -> Result<ClipPickingStats> {
    ensure(
        path.len() <= MAX_CLIP_COMMANDS,
        "clip path command budget exceeded",
    )?;
    let mut stats = ClipPickingStats::default();
    let mut open = false;
    let (mut current, mut first) = (P::new(0., 0.), P::new(0., 0.));
    for command in path {
        clip_spend(&mut stats)?;
        match *command {
            C::Move(point) => {
                ensure(
                    !open && point_valid(point),
                    "clip contours must start finite and explicitly close",
                )?;
                open = true;
                current = point;
                first = point;
            }
            C::Line(point) => {
                ensure(open && point_valid(point), "invalid clip path segment")?;
                line(current, point, 0., &mut stats, &mut visit)?;
                current = point;
            }
            C::Cubic {
                first: a,
                second: b,
                end,
            } => {
                ensure(
                    open && [a, b, end].into_iter().all(point_valid),
                    "invalid clip path cubic",
                )?;
                cubic([current, a, b, end], 0, &mut stats, &mut visit)?;
                current = end;
            }
            C::Close => {
                ensure(open, "clip close without open contour")?;
                line(current, first, 0., &mut stats, &mut visit)?;
                current = first;
                open = false;
            }
        }
    }
    ensure(!open, "clip contours must be explicitly closed")?;
    Ok(stats)
}

fn winding_at(point: P, a: P, b: P) -> i32 {
    let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);
    if a.y <= point.y && b.y > point.y && cross > 0. {
        1
    } else if a.y > point.y && b.y <= point.y && cross < 0. {
        -1
    } else {
        0
    }
}

fn filled(rule: ClipRule, winding: i32) -> bool {
    match rule {
        ClipRule::NonZero => winding != 0,
        ClipRule::EvenOdd => winding.abs() % 2 == 1,
    }
}

impl ClipPath {
    pub fn circle(x: f64, y: f64, radius: f64) -> Self {
        Self {
            geometry: Geometry::Circle { x, y, radius },
            rule: ClipRule::NonZero,
        }
    }
    pub fn validate(&self) -> Result<()> {
        self.picking_stats().map(|_| ())
    }
    /// Source SVG-equivalent command count, without cloning a Path vector.
    pub fn command_count(&self) -> usize {
        match &self.geometry {
            Geometry::Path(path) => path.len(),
            Geometry::Circle { .. } => 6,
            Geometry::Rect { .. } => 5,
        }
    }
    /// Validate the complete mask and pre-admit the exact adaptive picking
    /// workload. Circle/rect picking remains analytic without flattening.
    pub fn picking_stats(&self) -> Result<ClipPickingStats> {
        match &self.geometry {
            Geometry::Path(path) => visit_path(path, |_, _, _| {}),
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
                    "invalid clip circle",
                )?;
                Ok(ClipPickingStats {
                    work: 1,
                    ..Default::default()
                })
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
                    "invalid clip rectangle",
                )?;
                Ok(ClipPickingStats {
                    work: 1,
                    ..Default::default()
                })
            }
        }
    }

    /// Conservative admission for a complete interactive overlay rectangle.
    /// The curved boundary's error band counts as partial coverage.
    pub(crate) fn rect_coverage(&self, rect: LayoutRect) -> Result<RectCoverage> {
        use RectCoverage::{Full, Outside, Partial};
        let right = rect.x + rect.width;
        let bottom = rect.y + rect.height;
        match &self.geometry {
            Geometry::Circle { x, y, radius } => {
                let distance = (x - x.clamp(rect.x, right)).hypot(y - y.clamp(rect.y, bottom));
                let far_x = (rect.x - x).abs().max((right - x).abs());
                let far_y = (rect.y - y).abs().max((bottom - y).abs());
                Ok(if distance >= *radius {
                    Outside
                } else if far_x.hypot(far_y) <= *radius {
                    Full
                } else {
                    Partial
                })
            }
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => {
                let mask = LayoutRect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                };
                Ok(if mask.overlap(rect) == 0. {
                    Outside
                } else if mask.contains(rect) {
                    Full
                } else {
                    Partial
                })
            }
            Geometry::Path(path) => {
                let middle = P::new(rect.x + rect.width / 2., rect.y + rect.height / 2.);
                let mut wind = 0;
                let mut touches = false;
                visit_path(path, |a, b, error| {
                    if touches {
                        return;
                    }
                    wind += winding_at(middle, a, b);
                    let band = error + 1e-9;
                    let expanded = ClipRect {
                        x: rect.x - band,
                        y: rect.y - band,
                        width: rect.width + 2. * band,
                        height: rect.height + 2. * band,
                    };
                    if a.x.max(b.x) >= expanded.x
                        && a.x.min(b.x) <= expanded.x + expanded.width
                        && a.y.max(b.y) >= expanded.y
                        && a.y.min(b.y) <= expanded.y + expanded.height
                    {
                        touches = !matches!(
                            crate::clipping::clip_segment((a.x, a.y), (b.x, b.y), expanded),
                            Ok(None)
                        );
                    }
                })?;
                Ok(if touches {
                    Partial
                } else if filled(self.rule, wind) {
                    Full
                } else {
                    Outside
                })
            }
        }
    }
    /// Fill-rule picking with a <=0.05 logical-unit uncertainty band around
    /// cubic boundaries (up to floating-point roundoff). Cubic leaf chords have
    /// <=0.025 positional error; their error-aware boundary band preserves
    /// actual curve points. Invalid/over-budget masks return false.
    pub fn contains(&self, x: f64, y: f64) -> bool {
        if !x.is_finite() || !y.is_finite() {
            return false;
        }
        match &self.geometry {
            Geometry::Circle {
                x: cx,
                y: cy,
                radius,
            } => self.picking_stats().is_ok() && *radius > 0. && (x - cx).hypot(y - cy) <= *radius,
            Geometry::Rect {
                x: cx,
                y: cy,
                width,
                height,
            } => {
                self.picking_stats().is_ok()
                    && *width > 0.
                    && *height > 0.
                    && x >= *cx
                    && x <= cx + width
                    && y >= *cy
                    && y <= cy + height
            }
            Geometry::Path(path) => {
                let mut wind = 0i32;
                let mut boundary = false;
                let point = P::new(x, y);
                let result = visit_path(path, |a, b, error| {
                    boundary |= distance_to_segment(point, a, b) <= error + 1e-9;
                    wind += winding_at(point, a, b);
                });
                result.is_ok() && (boundary || filled(self.rule, wind))
            }
        }
    }
    pub(crate) fn svg_id(&self) -> String {
        use std::fmt::Write;
        let commands = match &self.geometry {
            Geometry::Path(path) => std::borrow::Cow::Borrowed(path.as_slice()),
            _ => std::borrow::Cow::Owned(self.geometry.commands()),
        };
        let prefix = if self.rule == ClipRule::NonZero {
            "kartis-clip-n"
        } else {
            "kartis-clip-e"
        };
        let words = commands
            .iter()
            .map(|command| match command {
                C::Move(_) | C::Line(_) => 3,
                C::Cubic { .. } => 7,
                C::Close => 1,
            })
            .sum::<usize>();
        let mut id = String::with_capacity(prefix.len() + words * 16);
        id.push_str(prefix);
        let mut number = |v: f64| {
            write!(id, "{:016x}", if v == 0. { 0 } else { v.to_bits() }).unwrap();
        };
        // Commands preserve contour boundaries and rule; no randomized/global IDs.
        for c in commands.iter() {
            match *c {
                C::Move(p) => {
                    number(1.);
                    number(p.x);
                    number(p.y)
                }
                C::Line(p) => {
                    number(2.);
                    number(p.x);
                    number(p.y)
                }
                C::Cubic { first, second, end } => {
                    number(3.);
                    for p in [first, second, end] {
                        number(p.x);
                        number(p.y)
                    }
                }
                C::Close => number(4.),
            }
        }
        id
    }
}
