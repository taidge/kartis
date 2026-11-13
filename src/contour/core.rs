//! Private marching-squares implementation, exposed through the checked parent API.
use std::collections::HashMap;
mod density;
pub use density::*;

#[derive(Clone, Debug, PartialEq)]
pub struct Error(pub String);
pub type Result<T> = std::result::Result<T, Error>;
fn ensure(value: bool, message: &str) -> Result<()> {
    if value {
        Ok(())
    } else {
        Err(Error(message.into()))
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub cells: usize,
    pub levels: usize,
    pub work: usize,
    pub edges: usize,
    pub points: usize,
    pub rings: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            cells: 1_048_576,
            levels: 256,
            work: 32_000_000,
            edges: 1_000_000,
            points: 1_000_000,
            rings: 20_000,
        }
    }
}
impl Limits {
    fn validate(self) -> Result<()> {
        let max = Self::default();
        ensure(
            self.cells <= max.cells
                && self.levels <= max.levels
                && self.work <= max.work
                && self.edges <= max.edges
                && self.points <= max.points
                && self.rings <= max.rings,
            "contour limits exceed fixed maximum profile",
        )
    }
}
#[derive(Clone, Copy, Debug)]
pub struct Grid<'a> {
    pub width: usize,
    pub height: usize,
    pub values: &'a [Option<f64>],
}
#[derive(Clone, Copy)]
enum Samples<'a> {
    Nullable(&'a [Option<f64>]),
    Dense(&'a [f32]),
}
#[derive(Clone, Copy)]
struct Field<'a> {
    width: usize,
    height: usize,
    samples: Samples<'a>,
}
impl Field<'_> {
    fn len(self) -> usize {
        match self.samples {
            Samples::Nullable(v) => v.len(),
            Samples::Dense(v) => v.len(),
        }
    }
    fn value(self, index: usize) -> Option<f64> {
        match self.samples {
            Samples::Nullable(v) => v[index],
            Samples::Dense(v) => Some(f64::from(v[index])),
        }
    }
    fn values(self) -> impl Iterator<Item = Option<f64>> {
        (0..self.len()).map(move |i| self.value(i))
    }
    fn get(self, x: isize, y: isize) -> Option<f64> {
        if x < 0 || y < 0 || x as usize >= self.width || y as usize >= self.height {
            return None;
        }
        self.value(y as usize * self.width + x as usize)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum Thresholds {
    Sturges,
    Count(usize),
    Values(Vec<f64>),
}
#[derive(Clone, Debug)]
pub struct Options {
    pub thresholds: Thresholds,
    pub smooth: bool,
    pub limits: Limits,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            thresholds: Thresholds::Sturges,
            smooth: true,
            limits: Limits::default(),
        }
    }
}
pub type Point = [f64; 2];
pub type Ring = Vec<Point>;
#[derive(Clone, Debug, PartialEq)]
pub struct Polygon {
    pub outer: Ring,
    pub holes: Vec<Ring>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Level {
    pub value: f64,
    pub polygons: Vec<Polygon>,
}
struct Budget {
    limits: Limits,
    work: usize,
    edges: usize,
    points: usize,
    rings: usize,
}
impl Budget {
    fn work(&mut self, count: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(count)
            .ok_or_else(|| Error("contour work overflow".into()))?;
        ensure(
            self.work <= self.limits.work,
            "contour work budget exceeded",
        )
    }
    fn edge(&mut self) -> Result<()> {
        ensure(
            self.edges < self.limits.edges,
            "contour edge budget exceeded",
        )?;
        self.edges += 1;
        Ok(())
    }
    fn point(&mut self) -> Result<()> {
        ensure(
            self.points < self.limits.points,
            "contour point budget exceeded",
        )?;
        self.points += 1;
        Ok(())
    }
    fn ring(&mut self) -> Result<()> {
        ensure(
            self.rings < self.limits.rings,
            "contour ring budget exceeded",
        )?;
        self.rings += 1;
        Ok(())
    }
}
impl Grid<'_> {
    fn validate(&self, limits: Limits) -> Result<()> {
        limits.validate()?;
        let count = self
            .width
            .checked_mul(self.height)
            .ok_or_else(|| Error("contour dimensions overflow".into()))?;
        ensure(
            self.width > 0 && self.height > 0 && count <= limits.cells,
            "invalid contour dimensions or cell budget",
        )?;
        ensure(self.values.len() == count, "contour sample count mismatch")?;
        ensure(
            self.values.iter().flatten().all(|v| v.is_finite()),
            "nonfinite contour sample",
        )
    }
}
impl<'a> Grid<'a> {
    fn field(self) -> Field<'a> {
        Field {
            width: self.width,
            height: self.height,
            samples: Samples::Nullable(self.values),
        }
    }
}

// Directed D3 marching-squares topology, in half-cell integer coordinates.
// Original reference: d3-contour 4.0.2, ISC; see THIRD-PARTY-NOTICES.txt.
const CASES: [&[[[isize; 2]; 2]]; 16] = [
    &[],
    &[[[2, 3], [1, 2]]],
    &[[[3, 2], [2, 3]]],
    &[[[3, 2], [1, 2]]],
    &[[[2, 1], [3, 2]]],
    &[[[2, 3], [1, 2]], [[2, 1], [3, 2]]],
    &[[[2, 1], [2, 3]]],
    &[[[2, 1], [1, 2]]],
    &[[[1, 2], [2, 1]]],
    &[[[2, 3], [2, 1]]],
    &[[[1, 2], [2, 1]], [[3, 2], [2, 3]]],
    &[[[3, 2], [2, 1]]],
    &[[[1, 2], [3, 2]]],
    &[[[2, 3], [3, 2]]],
    &[[[1, 2], [2, 3]]],
    &[],
];
type Vertex = [usize; 2];
struct Link {
    point: Vertex,
    next: Option<usize>,
}
struct Fragment {
    start: Vertex,
    end: Vertex,
    first: usize,
    last: usize,
}
#[derive(Default)]
struct Stitcher {
    starts: HashMap<Vertex, usize>,
    ends: HashMap<Vertex, usize>,
    fragments: Vec<Option<Fragment>>,
    vacant: Vec<usize>,
    links: Vec<Link>,
    rings: Vec<Ring>,
}
impl Stitcher {
    fn vertex(&mut self, point: Vertex, next: Option<usize>, budget: &mut Budget) -> Result<usize> {
        budget.point()?;
        let index = self.links.len();
        self.links.push(Link { point, next });
        Ok(index)
    }
    fn segment(&mut self, start: Vertex, end: Vertex, budget: &mut Budget) -> Result<()> {
        budget.edge()?;
        let left = self.ends.remove(&start);
        let right = self.starts.remove(&end);
        match (left, right) {
            (Some(f), Some(g)) if f == g => {
                let fragment = self.fragments[f].take().unwrap();
                self.vacant.push(f);
                let last = self.vertex(end, None, budget)?;
                self.links[fragment.last].next = Some(last);
                budget.ring()?;
                let mut ring = Vec::new();
                let mut current = Some(fragment.first);
                while let Some(index) = current {
                    budget.work(1)?;
                    let link = &self.links[index];
                    ring.push([link.point[0] as f64 / 2., link.point[1] as f64 / 2.]);
                    current = link.next;
                }
                ensure(
                    ring.len() >= 4 && ring.first() == ring.last(),
                    "invalid stitched contour ring",
                )?;
                self.rings.push(ring);
            }
            (Some(f), Some(g)) => {
                let left = self.fragments[f].take().unwrap();
                let right = self.fragments[g].take().unwrap();
                self.links[left.last].next = Some(right.first);
                self.starts.insert(left.start, f);
                self.ends.insert(right.end, f);
                self.fragments[f] = Some(Fragment {
                    start: left.start,
                    end: right.end,
                    first: left.first,
                    last: right.last,
                });
                self.vacant.push(g);
            }
            (Some(f), None) => {
                let last = self.vertex(end, None, budget)?;
                let fragment = self.fragments[f].as_mut().unwrap();
                self.links[fragment.last].next = Some(last);
                fragment.end = end;
                fragment.last = last;
                ensure(self.ends.insert(end, f).is_none(), "ambiguous contour end")?;
            }
            (None, Some(g)) => {
                let next = self.fragments[g].as_ref().unwrap().first;
                let first = self.vertex(start, Some(next), budget)?;
                let fragment = self.fragments[g].as_mut().unwrap();
                fragment.start = start;
                fragment.first = first;
                ensure(
                    self.starts.insert(start, g).is_none(),
                    "ambiguous contour start",
                )?;
            }
            (None, None) => {
                let last = self.vertex(end, None, budget)?;
                let first = self.vertex(start, Some(last), budget)?;
                let fragment = Fragment {
                    start,
                    end,
                    first,
                    last,
                };
                let index = if let Some(index) = self.vacant.pop() {
                    self.fragments[index] = Some(fragment);
                    index
                } else {
                    self.fragments.push(Some(fragment));
                    self.fragments.len() - 1
                };
                ensure(
                    self.starts.insert(start, index).is_none()
                        && self.ends.insert(end, index).is_none(),
                    "ambiguous contour fragment",
                )?;
            }
        }
        Ok(())
    }
}

fn rings(grid: Field<'_>, threshold: f64, smooth: bool, budget: &mut Budget) -> Result<Vec<Ring>> {
    let width = isize::try_from(grid.width).map_err(|_| Error("contour width overflow".into()))?;
    let height =
        isize::try_from(grid.height).map_err(|_| Error("contour height overflow".into()))?;
    let cell_work = grid
        .width
        .checked_add(1)
        .and_then(|w| grid.height.checked_add(1).and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| Error("contour scan overflow".into()))?;
    budget.work(cell_work)?;
    let mut stitcher = Stitcher::default();
    for y in -1..height {
        for x in -1..width {
            let above = |x, y| grid.get(x, y).is_some_and(|v| v >= threshold);
            let index = usize::from(above(x, y + 1))
                | (usize::from(above(x + 1, y + 1)) * 2)
                | (usize::from(above(x + 1, y)) * 4)
                | (usize::from(above(x, y)) * 8);
            for segment in CASES[index] {
                let mut points = [[0; 2]; 2];
                for (point, local) in points.iter_mut().zip(segment) {
                    let px = 2 * x + local[0];
                    let py = 2 * y + local[1];
                    ensure(px >= 0 && py >= 0, "negative contour vertex")?;
                    *point = [px as usize, py as usize];
                }
                stitcher.segment(points[0], points[1], budget)?;
            }
        }
    }
    ensure(
        stitcher.starts.is_empty() && stitcher.ends.is_empty(),
        "open contour fragments",
    )?;
    if smooth {
        for ring in &mut stitcher.rings {
            smooth_ring(grid, ring, threshold)?;
        }
    }
    Ok(stitcher.rings)
}
fn smooth_ring(grid: Field<'_>, ring: &mut Ring, threshold: f64) -> Result<()> {
    for point in ring {
        let [x, y] = *point;
        let xt = x as isize;
        let yt = y as isize;
        let v1 = grid.get(xt, yt).unwrap_or(f64::NEG_INFINITY);
        if x > 0. && x < grid.width as f64 && xt as f64 == x {
            point[0] = smooth_value(
                x,
                grid.get(xt - 1, yt).unwrap_or(f64::NEG_INFINITY),
                v1,
                threshold,
            );
        }
        if y > 0. && y < grid.height as f64 && yt as f64 == y {
            point[1] = smooth_value(
                y,
                grid.get(xt, yt - 1).unwrap_or(f64::NEG_INFINITY),
                v1,
                threshold,
            );
        }
        ensure(
            point.iter().all(|p| p.is_finite()),
            "nonfinite smoothed contour",
        )?;
    }
    Ok(())
}
fn smooth_value(position: f64, v0: f64, v1: f64, threshold: f64) -> f64 {
    let a = threshold - v0;
    let b = v1 - v0;
    let d = if a.is_finite() || b.is_finite() {
        a / b
    } else {
        a.signum() / b.signum()
    };
    if d.is_nan() {
        position
    } else {
        position + d - 0.5
    }
}
fn signed_area(ring: &Ring) -> f64 {
    ring.windows(2)
        .map(|p| p[0][1] * p[1][0] - p[0][0] * p[1][1])
        .sum::<f64>()
        / 2.
}
fn contains(ring: &Ring, point: Point, budget: &mut Budget) -> Result<i8> {
    budget.work(ring.len())?;
    let mut inside = false;
    for i in 0..ring.len() {
        let [a, b] = [ring[i], ring[(i + ring.len() - 1) % ring.len()]];
        let collinear = (b[0] - a[0]) * (point[1] - a[1]) == (point[0] - a[0]) * (b[1] - a[1]);
        let axis = usize::from(a[0] == b[0]);
        if collinear && point[axis] >= a[axis].min(b[axis]) && point[axis] <= a[axis].max(b[axis]) {
            return Ok(0);
        }
        if (a[1] > point[1]) != (b[1] > point[1])
            && point[0] < (b[0] - a[0]) * (point[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    Ok(if inside { 1 } else { -1 })
}
fn level(grid: Field<'_>, threshold: f64, smooth: bool, budget: &mut Budget) -> Result<Level> {
    let mut polygons = Vec::new();
    let mut holes = Vec::new();
    for ring in rings(grid, threshold, smooth, budget)? {
        let area = signed_area(&ring);
        if area > 0. {
            polygons.push(Polygon {
                outer: ring,
                holes: Vec::new(),
            });
        } else {
            holes.push(ring);
        }
    }
    for hole in holes {
        let mut owner = None;
        for (index, polygon) in polygons.iter().enumerate() {
            let mut state = 0;
            for &point in &hole {
                state = contains(&polygon.outer, point, budget)?;
                if state != 0 {
                    break;
                }
            }
            if state != -1 {
                owner = Some(index);
                break;
            }
        }
        if let Some(owner) = owner {
            polygons[owner].holes.push(hole);
        }
    }
    Ok(Level {
        value: threshold,
        polygons,
    })
}

/// Explicit one-level contour. Missing observations always lie below threshold.
pub fn contour(grid: Grid<'_>, threshold: f64, smooth: bool, limits: Limits) -> Result<Level> {
    grid.validate(limits)?;
    ensure(limits.levels > 0, "contour level budget exceeded")?;
    ensure(threshold.is_finite(), "nonfinite contour threshold")?;
    let mut budget = Budget {
        limits,
        work: 0,
        edges: 0,
        points: 0,
        rings: 0,
    };
    level(grid.field(), threshold, smooth, &mut budget)
}

fn tick_spec(start: f64, stop: f64, count: f64) -> (f64, f64, f64) {
    let step = (stop - start) / count;
    let power = step.log10().floor();
    let unit = 10_f64.powf(power);
    let error = step / unit;
    let factor = if error >= 50_f64.sqrt() {
        10.
    } else if error >= 10_f64.sqrt() {
        5.
    } else if error >= 2_f64.sqrt() {
        2.
    } else {
        1.
    };
    let (mut first, mut last, increment) = if power < 0. {
        let increment = 10_f64.powf(-power) / factor;
        let mut first = (start * increment + 0.5).floor();
        let mut last = (stop * increment + 0.5).floor();
        if first / increment < start {
            first += 1.;
        }
        if last / increment > stop {
            last -= 1.;
        }
        (first, last, -increment)
    } else {
        let increment = unit * factor;
        let mut first = (start / increment + 0.5).floor();
        let mut last = (stop / increment + 0.5).floor();
        if first * increment < start {
            first += 1.;
        }
        if last * increment > stop {
            last -= 1.;
        }
        (first, last, increment)
    };
    if last < first && (0.5..2.).contains(&count) {
        return tick_spec(start, stop, count * 2.);
    }
    // Keep the return values finite check in callers; a degenerate tick request
    // has the same nonfinite increment as the independent reference.
    if first == 0. {
        first = 0.;
    }
    if last == 0. {
        last = 0.;
    }
    (first, last, increment)
}
fn numeric_ticks(start: f64, stop: f64, count: usize, limit: usize) -> Result<Vec<f64>> {
    if count == 0 {
        return Ok(vec![]);
    }
    if start == stop {
        return Ok(vec![start]);
    }
    let reverse = stop < start;
    let (first, last, increment) = if reverse {
        tick_spec(stop, start, count as f64)
    } else {
        tick_spec(start, stop, count as f64)
    };
    if last < first || first.is_nan() || last.is_nan() {
        return Ok(vec![]);
    }
    ensure(
        first.is_finite() && last.is_finite() && increment.is_finite() && increment != 0.,
        "unrepresentable contour threshold ticks",
    )?;
    let n = last - first + 1.;
    ensure(
        n >= 0. && n <= limit as f64,
        "contour threshold budget exceeded",
    )?;
    Ok((0..n as usize)
        .map(|i| {
            let index = if reverse {
                last - i as f64
            } else {
                first + i as f64
            };
            if increment < 0. {
                index / -increment
            } else {
                index * increment
            }
        })
        .collect())
}
fn automatic_thresholds(grid: Field<'_>, count: usize, limit: usize) -> Result<Vec<f64>> {
    if count == 0 {
        return Ok(vec![]);
    }
    let mut samples = grid.values().flatten();
    let Some(first) = samples.next() else {
        return Ok(vec![]);
    };
    let (mut low, mut high) = (first, first);
    for value in samples {
        low = low.min(value);
        high = high.max(value);
    }
    if low == high {
        return Ok(vec![]);
    }
    ensure(
        (high - low).is_finite(),
        "contour automatic threshold span overflow",
    )?;
    let (extent_low, extent_high) = (low, high);
    let mut previous = None;
    let mut stable = false;
    for _ in 0..32 {
        let (_, _, step) = tick_spec(low, high, count as f64);
        if previous == Some(step) || step == 0. || !step.is_finite() {
            stable = true;
            break;
        }
        if step > 0. {
            low = (low / step).floor() * step;
            high = (high / step).ceil() * step;
        } else {
            low = (low * step).ceil() / step;
            high = (high * step).floor() / step;
        }
        previous = Some(step);
    }
    ensure(
        stable && low.is_finite() && high.is_finite(),
        "contour automatic thresholds failed to converge",
    )?;
    let mut ticks = numeric_ticks(low, high, count, limit.saturating_add(2))?;
    while ticks.last().is_some_and(|v| *v >= extent_high) {
        ticks.pop();
    }
    let mut drop = 0;
    while drop + 1 < ticks.len() && ticks[drop + 1] < extent_low {
        drop += 1;
    }
    ticks.drain(..drop);
    ensure(ticks.len() <= limit, "contour threshold budget exceeded")?;
    Ok(ticks)
}

/// Compute explicit or D3-style automatic thresholds with cumulative limits.
pub fn contours(grid: Grid<'_>, options: &Options) -> Result<Vec<Level>> {
    grid.validate(options.limits)?;
    let grid = grid.field();
    let mut values = match &options.thresholds {
        Thresholds::Values(values) => {
            ensure(
                values.len() <= options.limits.levels,
                "contour threshold budget exceeded",
            )?;
            ensure(
                values.iter().all(|v| v.is_finite()),
                "nonfinite contour threshold",
            )?;
            values.clone()
        }
        Thresholds::Count(count) => {
            ensure(
                *count <= options.limits.levels,
                "contour threshold count budget exceeded",
            )?;
            automatic_thresholds(grid, *count, options.limits.levels)?
        }
        Thresholds::Sturges => {
            let count = grid.values().flatten().count();
            let levels = if count == 0 {
                1
            } else {
                (count as f64).log2().ceil() as usize + 1
            };
            automatic_thresholds(grid, levels, options.limits.levels)?
        }
    };
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut budget = Budget {
        limits: options.limits,
        work: 0,
        edges: 0,
        points: 0,
        rings: 0,
    };
    values
        .into_iter()
        .map(|value| level(grid, value, options.smooth, &mut budget))
        .collect()
}
