//! Bounded planar rectangle postclipping, adapted from d3-geo 3.1.1
//! clip/{rectangle,line,buffer,rejoin}.js (ISC, Copyright 2010-2024 Mike Bostock).
//! This is a planar stream operation, not geographic latitude/seam clipping.
use std::cmp::Ordering;

use crate::scene::ensure;
use crate::{ChartError, ClipRect, Result};

const EPSILON: f64 = 1e-6;
const COORDINATE_LIMIT: f64 = 1e9;
type Point = [f64; 2];
type Contour = Vec<Point>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoRectClipOptions {
    pub max_points: usize,
    pub max_work: usize,
}
impl Default for GeoRectClipOptions {
    fn default() -> Self {
        Self {
            max_points: 200_000,
            max_work: 4_000_000,
        }
    }
}
impl GeoRectClipOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            (1..=200_000).contains(&self.max_points) && (1..=4_000_000).contains(&self.max_work),
            "invalid rectangle clip budget",
        )
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeoRectClipStats {
    pub input_points: usize,
    pub buffered_points: usize,
    pub output_points: usize,
    pub work: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeoRectClipResult {
    /// Native nonzero-winding contours. Rejoined output can repeat its closing
    /// vertex or contain degenerate boundary contacts, as the D3 stream does.
    /// No exterior/hole role is inferred from position in this list.
    pub rings: Vec<Contour>,
    pub stats: GeoRectClipStats,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeoRectLineClipResult {
    /// Independent open fragments. Native tangent/point-contact fragments can
    /// be degenerate; consumers decide whether their paint can use these.
    pub lines: Vec<Contour>,
    pub stats: GeoRectClipStats,
}

struct Budget<'a> {
    options: &'a GeoRectClipOptions,
    stats: GeoRectClipStats,
}
impl Budget<'_> {
    fn charge(&mut self, amount: usize) -> Result<()> {
        self.stats.work = self
            .stats
            .work
            .checked_add(amount)
            .ok_or_else(|| ChartError("rectangle clip work overflow".into()))?;
        ensure(
            self.stats.work <= self.options.max_work,
            "rectangle clip work budget exceeded",
        )
    }
    fn input(&mut self, point: Point) -> Result<()> {
        self.charge(1)?;
        ensure(
            point.iter().all(|v| v.is_finite()),
            "rectangle clip input must be finite",
        )?;
        self.stats.input_points += 1;
        Ok(())
    }
    fn buffered(&mut self) -> Result<()> {
        self.charge(1)?;
        self.stats.buffered_points += 1;
        // Closing duplicates and separate intersection endpoints can use up
        // to twice the final-point allowance. This bounds intermediate storage.
        ensure(
            self.stats.buffered_points <= self.options.max_points * 2,
            "rectangle clip intermediate point budget exceeded",
        )
    }
    fn output(&mut self, ring: &mut Contour, point: Point) -> Result<()> {
        self.charge(1)?;
        self.stats.output_points += 1;
        ensure(
            self.stats.output_points <= self.options.max_points,
            "rectangle clip output point budget exceeded",
        )?;
        ring.push(point);
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Rectangle {
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
}
impl Rectangle {
    fn new(rect: ClipRect) -> Result<Self> {
        rect.validate()?;
        ensure(
            rect.width > 0.
                && rect.height > 0.
                && rect.x + rect.width > rect.x
                && rect.y + rect.height > rect.y,
            "rectangle clip requires positive representable extents",
        )?;
        Ok(Self {
            x0: rect.x,
            y0: rect.y,
            x1: rect.x + rect.width,
            y1: rect.y + rect.height,
        })
    }
    fn visible(self, p: Point) -> bool {
        self.x0 <= p[0] && p[0] <= self.x1 && self.y0 <= p[1] && p[1] <= self.y1
    }
    fn corner(self, p: Point, direction: i32) -> usize {
        if (p[0] - self.x0).abs() < EPSILON {
            if direction > 0 { 0 } else { 3 }
        } else if (p[0] - self.x1).abs() < EPSILON {
            if direction > 0 { 2 } else { 1 }
        } else if (p[1] - self.y0).abs() < EPSILON {
            if direction > 0 { 1 } else { 0 }
        } else if direction > 0 {
            3
        } else {
            2
        }
    }
    fn compare(self, a: Point, b: Point) -> Ordering {
        let ca = self.corner(a, 1);
        let cb = self.corner(b, 1);
        if ca != cb {
            return ca.cmp(&cb);
        }
        let (a, b) = match ca {
            0 => (b[1], a[1]),
            1 => (a[0], b[0]),
            2 => (a[1], b[1]),
            _ => (b[0], a[0]),
        };
        a.partial_cmp(&b).expect("finite rectangle intersections")
    }
    fn interpolate(
        self,
        from: Option<Point>,
        to: Option<Point>,
        direction: i32,
        out: &mut Contour,
        budget: &mut Budget<'_>,
    ) -> Result<()> {
        let (mut a, a1) = match (from, to) {
            (Some(a), Some(b)) => (self.corner(a, direction), self.corner(b, direction)),
            _ => (0, 0),
        };
        let wraps = from
            .zip(to)
            .is_some_and(|(a, b)| (self.compare(a, b) == Ordering::Less) ^ (direction > 0));
        if from.is_none() || a != a1 || wraps {
            loop {
                budget.output(
                    out,
                    [
                        if a == 0 || a == 3 { self.x0 } else { self.x1 },
                        if a > 1 { self.y1 } else { self.y0 },
                    ],
                )?;
                a = ((a as i32 + direction + 4) % 4) as usize;
                if a == a1 {
                    break;
                }
            }
        } else {
            budget.output(out, to.expect("boundary interpolation endpoint"))?;
        }
        Ok(())
    }
    fn line(self, a: Point, b: Point, budget: &mut Budget<'_>) -> Result<Option<(Point, Point)>> {
        let dx = b[0] - a[0];
        let dy = b[1] - a[1];
        let mut t0: f64 = 0.;
        let mut t1: f64 = 1.;
        for (numerator, delta, lower) in [
            (self.x0 - a[0], dx, true),
            (self.x1 - a[0], dx, false),
            (self.y0 - a[1], dy, true),
            (self.y1 - a[1], dy, false),
        ] {
            budget.charge(1)?;
            if delta == 0. {
                if (lower && numerator > 0.) || (!lower && numerator < 0.) {
                    return Ok(None);
                }
                continue;
            }
            let ratio = numerator / delta;
            let enters = (delta > 0.) == lower;
            if enters {
                if ratio > t1 {
                    return Ok(None);
                }
                t0 = t0.max(ratio);
            } else {
                if ratio < t0 {
                    return Ok(None);
                }
                t1 = t1.min(ratio);
            }
        }
        let p = if t0 > 0. {
            [a[0] + t0 * dx, a[1] + t0 * dy]
        } else {
            a
        };
        let q = if t1 < 1. {
            [a[0] + t1 * dx, a[1] + t1 * dy]
        } else {
            b
        };
        Ok(Some((p, q)))
    }
}

struct LineBuffer {
    rectangle: Rectangle,
    first: Option<Point>,
    previous: Option<Point>,
    previous_visible: bool,
    lines: Vec<Contour>,
    clean: bool,
}
impl LineBuffer {
    fn new(rectangle: Rectangle) -> Self {
        Self {
            rectangle,
            first: None,
            previous: None,
            previous_visible: false,
            lines: vec![],
            clean: true,
        }
    }
    fn push(&mut self, point: Point, start: bool, budget: &mut Budget<'_>) -> Result<()> {
        budget.buffered()?;
        if start {
            self.lines.push(vec![]);
        }
        self.lines
            .last_mut()
            .expect("active clipped line")
            .push(point);
        Ok(())
    }
    fn point(&mut self, mut p: Point, budget: &mut Budget<'_>) -> Result<()> {
        budget.charge(1)?;
        let visible = self.rectangle.visible(p);
        if let Some(previous) = self.previous {
            if visible && self.previous_visible {
                self.push(p, false, budget)?;
            } else {
                let previous = previous.map(|v| v.clamp(-COORDINATE_LIMIT, COORDINATE_LIMIT));
                p = p.map(|v| v.clamp(-COORDINATE_LIMIT, COORDINATE_LIMIT));
                if let Some((a, b)) = self.rectangle.line(previous, p, budget)? {
                    if !self.previous_visible {
                        self.push(a, true, budget)?;
                    }
                    self.push(b, false, budget)?;
                    self.clean = false;
                } else if visible {
                    self.push(p, true, budget)?;
                    self.clean = false;
                }
            }
        } else {
            self.first = Some(p);
            if visible {
                self.push(p, true, budget)?;
            }
        }
        self.previous = Some(p);
        self.previous_visible = visible;
        Ok(())
    }
    fn close(&mut self, budget: &mut Budget<'_>) -> Result<()> {
        if let Some(first) = self.first {
            self.point(first, budget)?;
            if self.rectangle.visible(first) && self.previous_visible && self.lines.len() > 1 {
                budget.charge(self.lines.len())?;
                let mut last = self.lines.pop().unwrap();
                let first = self.lines.remove(0);
                budget.charge(first.len())?;
                last.extend(first);
                self.lines.push(last);
            }
        }
        Ok(())
    }
}

fn unclosed(ring: &[Point]) -> &[Point] {
    if ring.len() > 1 && ring.first() == ring.last() {
        &ring[..ring.len() - 1]
    } else {
        ring
    }
}

fn winding_at_corner(
    rings: &[Contour],
    rectangle: Rectangle,
    budget: &mut Budget<'_>,
) -> Result<i64> {
    let mut winding = 0i64;
    for ring in rings {
        budget.charge(1)?;
        let ring = unclosed(ring);
        let mut a = *ring.last().expect("validated polygon ring");
        for &b in ring {
            budget.charge(1)?;
            if (a[1] <= rectangle.y1 && b[1] > rectangle.y1)
                || (a[1] > rectangle.y1 && b[1] <= rectangle.y1)
            {
                let cross = (b[0] - a[0]) * (rectangle.y1 - a[1]);
                let right = (b[1] - a[1]) * (rectangle.x0 - a[0]);
                ensure(
                    cross.is_finite() && right.is_finite(),
                    "rectangle clip polygon winding arithmetic is not representable",
                )?;
                if a[1] <= rectangle.y1 && cross > right {
                    winding += 1;
                } else if a[1] > rectangle.y1 && cross < right {
                    winding -= 1;
                }
            }
            a = b;
        }
    }
    Ok(winding)
}

#[derive(Clone)]
struct Intersection {
    point: Point,
    segment: Option<usize>,
    other: usize,
    entry: bool,
    visited: bool,
    next: usize,
    previous: usize,
}

fn link(ids: &[usize], nodes: &mut [Intersection], budget: &mut Budget<'_>) -> Result<()> {
    for (index, &id) in ids.iter().enumerate() {
        budget.charge(1)?;
        nodes[id].next = ids[(index + 1) % ids.len()];
        nodes[id].previous = ids[(index + ids.len() - 1) % ids.len()];
    }
    Ok(())
}

fn sort_intersections(
    ids: &mut Vec<usize>,
    nodes: &[Intersection],
    rectangle: Rectangle,
    budget: &mut Budget<'_>,
) -> Result<()> {
    let n = ids.len();
    let mut temp = vec![0; n];
    let mut width = 1;
    while width < n {
        for first in (0..n).step_by(width * 2) {
            let middle = (first + width).min(n);
            let last = (first + width * 2).min(n);
            let (mut a, mut b) = (first, middle);
            for slot in &mut temp[first..last] {
                budget.charge(1)?;
                let left = if a == middle {
                    false
                } else if b == last {
                    true
                } else {
                    budget.charge(1)?;
                    rectangle.compare(nodes[ids[a]].point, nodes[ids[b]].point) != Ordering::Greater
                };
                if left {
                    *slot = ids[a];
                    a += 1;
                } else {
                    *slot = ids[b];
                    b += 1;
                }
            }
        }
        std::mem::swap(ids, &mut temp);
        width *= 2;
    }
    Ok(())
}

fn rejoin(
    segments: &[Contour],
    rectangle: Rectangle,
    mut start_inside: bool,
    budget: &mut Budget<'_>,
) -> Result<Vec<Contour>> {
    let mut output = vec![];
    let mut nodes = Vec::new();
    let mut subject = vec![];
    let mut boundary = vec![];
    for (segment_index, segment) in segments.iter().enumerate() {
        budget.charge(1)?;
        if segment.len() < 2 {
            continue;
        }
        let p0 = segment[0];
        let p1 = *segment.last().unwrap();
        if (p0[0] - p1[0]).abs() < EPSILON && (p0[1] - p1[1]).abs() < EPSILON {
            // Rectangle buffers do not carry the spherical clipper's third
            // coordinate degeneracy marker, so D3's marked-endpoint jitter
            // branch does not apply here.
            let mut ring = vec![];
            for &point in &segment[..segment.len() - 1] {
                budget.output(&mut ring, point)?;
            }
            output.push(ring);
            continue;
        }
        for (point, entry) in [(p0, true), (p1, false)] {
            budget.charge(2)?;
            let id = nodes.len();
            nodes.push(Intersection {
                point,
                segment: Some(segment_index),
                other: id + 1,
                entry,
                visited: false,
                next: 0,
                previous: 0,
            });
            nodes.push(Intersection {
                point,
                segment: None,
                other: id,
                entry: !entry,
                visited: false,
                next: 0,
                previous: 0,
            });
            subject.push(id);
            boundary.push(id + 1);
        }
    }
    if subject.is_empty() {
        return Ok(output);
    }
    sort_intersections(&mut boundary, &nodes, rectangle, budget)?;
    link(&subject, &mut nodes, budget)?;
    link(&boundary, &mut nodes, budget)?;
    for &id in &boundary {
        budget.charge(1)?;
        start_inside = !start_inside;
        nodes[id].entry = start_inside;
    }
    let mut next_start = 0;
    loop {
        while next_start < subject.len() && nodes[subject[next_start]].visited {
            budget.charge(1)?;
            next_start += 1;
        }
        if next_start == subject.len() {
            break;
        }
        let mut current = subject[next_start];
        let mut is_subject = true;
        let mut ring = vec![];
        loop {
            budget.charge(1)?;
            let other = nodes[current].other;
            nodes[current].visited = true;
            nodes[other].visited = true;
            if nodes[current].entry {
                if is_subject {
                    let segment = &segments[nodes[current].segment.unwrap()];
                    for &point in segment {
                        budget.output(&mut ring, point)?;
                    }
                } else {
                    rectangle.interpolate(
                        Some(nodes[current].point),
                        Some(nodes[nodes[current].next].point),
                        1,
                        &mut ring,
                        budget,
                    )?;
                }
                current = nodes[current].next;
            } else {
                if is_subject {
                    let previous = nodes[current].previous;
                    let segment = &segments[nodes[previous].segment.unwrap()];
                    for &point in segment.iter().rev() {
                        budget.output(&mut ring, point)?;
                    }
                } else {
                    rectangle.interpolate(
                        Some(nodes[current].point),
                        Some(nodes[nodes[current].previous].point),
                        -1,
                        &mut ring,
                        budget,
                    )?;
                }
                current = nodes[current].previous;
            }
            current = nodes[current].other;
            is_subject = !is_subject;
            if nodes[current].visited {
                break;
            }
        }
        output.push(ring);
    }
    Ok(output)
}

/// Clip a planar nonzero-winding polygon stream to a positive rectangle.
/// Inputs may explicitly repeat their first vertex; results retain native
/// streamed contours, including closing duplicates or degenerate contacts,
/// not an inferred exterior/hole hierarchy. Coordinates must be finite; crossing
/// line endpoints use D3's +/-1e9 numerical clamp. Unrepresentable polygon winding
/// arithmetic rejects instead of silently evaluating overflowing determinants.
/// Final points and a two-times-sized intermediate buffer are bounded by
/// max_points; source scanning, sorting and graph traversal consume max_work.
/// Errors return no partial result. Shared consumer budgets remain caller-owned.
pub fn clip_geo_planar_polygon(
    rings: &[Contour],
    rect: ClipRect,
    options: &GeoRectClipOptions,
) -> Result<GeoRectClipResult> {
    options.validate()?;
    let rectangle = Rectangle::new(rect)?;
    let mut budget = Budget {
        options,
        stats: Default::default(),
    };
    let mut segments = vec![];
    let mut clean = true;
    for ring in rings {
        budget.charge(1)?;
        let ring = unclosed(ring);
        ensure(
            ring.len() >= 3,
            "rectangle polygon ring needs at least three vertices",
        )?;
        let mut buffer = LineBuffer::new(rectangle);
        for &point in ring {
            budget.input(point)?;
            buffer.point(point, &mut budget)?;
        }
        buffer.close(&mut budget)?;
        clean &= buffer.clean;
        segments.extend(buffer.lines);
    }
    let start_inside = winding_at_corner(rings, rectangle, &mut budget)? != 0;
    let mut result = vec![];
    if clean && start_inside {
        let mut ring = vec![];
        rectangle.interpolate(None, None, 1, &mut ring, &mut budget)?;
        result.push(ring);
    }
    result.extend(rejoin(&segments, rectangle, start_inside, &mut budget)?);
    Ok(GeoRectClipResult {
        rings: result,
        stats: budget.stats,
    })
}

/// Clip independent open planar lines without introducing connecting bridges.
/// Native point/tangent contacts remain separate degenerate fragments, as in
/// D3. Coordinate and work/storage bounds match clip_geo_planar_polygon.
pub fn clip_geo_planar_lines(
    lines: &[Contour],
    rect: ClipRect,
    options: &GeoRectClipOptions,
) -> Result<GeoRectLineClipResult> {
    options.validate()?;
    let rectangle = Rectangle::new(rect)?;
    let mut budget = Budget {
        options,
        stats: Default::default(),
    };
    let mut result = vec![];
    for line in lines {
        budget.charge(1)?;
        let mut buffer = LineBuffer::new(rectangle);
        for &point in line {
            budget.input(point)?;
            buffer.point(point, &mut budget)?;
        }
        for line in buffer.lines {
            let mut accepted = vec![];
            for point in line {
                budget.output(&mut accepted, point)?;
            }
            result.push(accepted);
        }
    }
    Ok(GeoRectLineClipResult {
        lines: result,
        stats: budget.stats,
    })
}
