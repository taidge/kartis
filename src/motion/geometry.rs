//! Explicit geometric correspondence, not semantic correspondence between data.
//! Contours are flattened and resampled by arc length. Multiple contours need
//! explicit caller keys; holes and semantic correspondence are never inferred.
use std::collections::BTreeMap;

use glory::render::{GraphicPoint as P, PathCommand as C};

use crate::scene::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ClosedAlignment {
    #[default]
    PreserveStart,
    Rotate,
    /// May reverse winding. Use only when the caller permits that correspondence.
    RotateAndReverse,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PathMorphOptions {
    pub samples: usize,
    pub tolerance: f64,
    pub alignment: ClosedAlignment,
}
impl Default for PathMorphOptions {
    fn default() -> Self {
        Self {
            samples: 64,
            tolerance: 0.25,
            alignment: ClosedAlignment::PreserveStart,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GeometryTransitions {
    /// Per-ID consent to arc-length correspondence. Rectangles/circles become
    /// closed sampled paths. Open/closed changes still use topology Fade.
    pub morphs: BTreeMap<String, PathMorphOptions>,
    /// Explicit, stable contour slots for multi-contour paths. Input order is
    /// declared by ContourBindings, or by the full slot order when omitted.
    pub contour_morphs: BTreeMap<String, ContourMorphOptions>,
    /// Explicit enter origins and exit destinations; style/ownership comes from
    /// the actual mark. Without morph opt-in command topology must match.
    pub enter: BTreeMap<String, Geometry>,
    pub exit: BTreeMap<String, Geometry>,
}
/// Input path Move order, keyed by mark ID. Missing declared slots collapse.
/// An empty list explicitly binds an empty Path; absent mark entries use all
/// declared slots in profile order. These keys do not infer outer/hole roles.
pub type ContourBindings = BTreeMap<String, Vec<String>>;

#[derive(Clone, Debug, PartialEq)]
pub struct MorphContour {
    pub key: String,
    pub closed: bool,
    /// Fixed point for this contour when absent from an input geometry.
    pub collapse: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContourMorphOptions {
    pub sampling: PathMorphOptions,
    pub contours: Vec<MorphContour>,
}
pub const MAX_MORPH_CONTOURS: usize = 32;
pub const MAX_MORPH_SAMPLES: usize = 256;
pub const MAX_MORPH_WORK: usize = 4_000_000;
const MAX_FLAT_POINTS: usize = 8192;
impl PathMorphOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            (2..=MAX_MORPH_SAMPLES).contains(&self.samples)
                && self.tolerance.is_finite()
                && (0.000001..=10000.).contains(&self.tolerance),
            "invalid path morph sampling options",
        )
    }
}
impl GeometryTransitions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.morphs.len() + self.contour_morphs.len() + self.enter.len() + self.exit.len()
                <= 256,
            "geometry transition identity budget exceeded",
        )?;
        for id in self
            .morphs
            .keys()
            .chain(self.contour_morphs.keys())
            .chain(self.enter.keys())
            .chain(self.exit.keys())
        {
            ensure(
                !id.is_empty() && id.len() <= MAX_LABEL_BYTES,
                "invalid geometry transition mark ID",
            )?;
        }
        for options in self.morphs.values() {
            options.validate()?;
        }
        for (id, options) in &self.contour_morphs {
            ensure(
                !self.morphs.contains_key(id),
                "single and multi contour morph profiles overlap",
            )?;
            options.validate()?;
        }
        let mut commands = 0;
        for geometry in self.enter.values().chain(self.exit.values()) {
            commands += geometry.commands().len();
            ensure(
                commands <= 200000,
                "geometry transition command budget exceeded",
            )?;
            validate_geometry(geometry)?;
        }
        Ok(())
    }
    pub(super) fn sampled(&self, id: &str) -> bool {
        self.morphs.contains_key(id) || self.contour_morphs.contains_key(id)
    }
}
impl ContourMorphOptions {
    pub fn validate(&self) -> Result<()> {
        self.sampling.validate()?;
        ensure(
            !self.contours.is_empty() && self.contours.len() <= MAX_MORPH_CONTOURS,
            "multi contour slot budget exceeded",
        )?;
        ensure(
            self.sampling.alignment != ClosedAlignment::RotateAndReverse,
            "multi contour morph must preserve contour winding",
        )?;
        let mut keys = std::collections::BTreeSet::new();
        for slot in &self.contours {
            ensure(
                !slot.key.is_empty()
                    && slot.key.len() <= 256
                    && keys.insert(&slot.key)
                    && slot
                        .collapse
                        .iter()
                        .all(|v| v.is_finite() && v.abs() <= 1e6)
                    && (!slot.closed || self.sampling.samples >= 3),
                "invalid multi contour slot",
            )?;
        }
        Ok(())
    }
}
pub(super) fn validate_bindings(
    scene: &Scene,
    config: &GeometryTransitions,
    bindings: &ContourBindings,
) -> Result<()> {
    ensure(
        bindings.len() <= 256,
        "contour binding mark budget exceeded",
    )?;
    for (id, order) in bindings {
        let profile = config
            .contour_morphs
            .get(id)
            .ok_or_else(|| ChartError("contour binding has no multi contour profile".into()))?;
        ensure(
            scene.marks.iter().any(|mark| &mark.id == id),
            "contour binding mark is absent",
        )?;
        ensure(
            order.len() <= profile.contours.len(),
            "contour binding slot budget exceeded",
        )?;
        let mut seen = std::collections::BTreeSet::new();
        for key in order {
            ensure(
                seen.insert(key) && profile.contours.iter().any(|slot| &slot.key == key),
                "unknown or duplicate contour binding key",
            )?;
        }
    }
    Ok(())
}
fn validate_geometry(geometry: &Geometry) -> Result<()> {
    let mut scene = ChartConfig::default().scene()?;
    scene.marks.push(Mark::new(
        "geometry-validation",
        geometry.clone(),
        glory::style::Color(0, 0, 0, 255),
    ));
    scene.validate()
}
fn spend(work: &mut usize, amount: usize) -> Result<()> {
    *work = work.saturating_add(amount);
    ensure(
        *work <= MAX_MORPH_WORK,
        "path morph frame work budget exceeded",
    )
}
fn distance(a: P, b: P) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}
fn midpoint(a: P, b: P) -> P {
    P::new(a.x / 2. + b.x / 2., a.y / 2. + b.y / 2.)
}
fn flatness(a: P, b: P, p: P) -> f64 {
    let length = distance(a, b);
    if length == 0. {
        distance(a, p)
    } else {
        ((b.x - a.x) * (a.y - p.y) - (a.x - p.x) * (b.y - a.y)).abs() / length
    }
}
fn cubic(
    control: [P; 4],
    depth: usize,
    o: &PathMorphOptions,
    points: &mut Vec<P>,
    work: &mut usize,
) -> Result<()> {
    let [a, b, c, d] = control;
    spend(work, 1)?;
    if flatness(a, d, b).max(flatness(a, d, c)) <= o.tolerance
        && distance(a, b) + distance(b, c) + distance(c, d) - distance(a, d) <= o.tolerance
    {
        ensure(
            points.len() < MAX_FLAT_POINTS,
            "path flatten point budget exceeded",
        )?;
        points.push(d);
        return Ok(());
    }
    ensure(
        depth < 16,
        "path morph flatten tolerance exceeds subdivision budget",
    )?;
    let ab = midpoint(a, b);
    let bc = midpoint(b, c);
    let cd = midpoint(c, d);
    let abc = midpoint(ab, bc);
    let bcd = midpoint(bc, cd);
    let mid = midpoint(abc, bcd);
    cubic([a, ab, abc, mid], depth + 1, o, points, work)?;
    cubic([mid, bcd, cd, d], depth + 1, o, points, work)
}
fn flatten(geometry: &Geometry, o: &PathMorphOptions, work: &mut usize) -> Result<(Vec<P>, bool)> {
    let mut points = vec![];
    let mut closed = false;
    match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => {
            points = vec![
                P::new(*x, *y),
                P::new(x + width, *y),
                P::new(x + width, y + height),
                P::new(*x, y + height),
            ];
            closed = true;
        }
        Geometry::Circle { x, y, radius } => {
            // Sagitta bound for chords: r*(1-cos(pi/n)) <= tolerance.
            let count = if *radius == 0. {
                4
            } else {
                (std::f64::consts::PI / (1. - (o.tolerance / radius).min(1.)).acos())
                    .ceil()
                    .max(8.) as usize
            };
            ensure(
                count <= MAX_FLAT_POINTS,
                "circle morph flatten point budget exceeded",
            )?;
            for i in 0..count {
                let angle = std::f64::consts::TAU * i as f64 / count as f64;
                points.push(P::new(x + radius * angle.cos(), y + radius * angle.sin()));
            }
            closed = true;
        }
        Geometry::Path(path) => {
            ensure(
                path.len() <= MAX_FLAT_POINTS,
                "path morph source command budget exceeded",
            )?;
            let mut current = None;
            for command in path {
                spend(work, 1)?;
                match *command {
                    C::Move(point) => {
                        ensure(current.is_none(), "path morph supports one contour only")?;
                        points.push(point);
                        current = Some(point);
                    }
                    C::Line(point) => {
                        ensure(!closed && current.is_some(), "invalid morph path contour")?;
                        points.push(point);
                        current = Some(point);
                    }
                    C::Cubic { first, second, end } => {
                        ensure(!closed, "invalid morph path after close")?;
                        let start = current
                            .ok_or_else(|| ChartError("morph path has no starting point".into()))?;
                        cubic([start, first, second, end], 0, o, &mut points, work)?;
                        current = Some(end);
                    }
                    C::Close => {
                        ensure(current.is_some() && !closed, "invalid morph close")?;
                        closed = true;
                    }
                }
                ensure(
                    points.len() <= MAX_FLAT_POINTS,
                    "path flatten point budget exceeded",
                )?;
            }
        }
    }
    ensure(!points.is_empty(), "cannot morph an empty contour")?;
    spend(work, points.len())?;
    if closed && points.len() > 1 && distance(points[0], *points.last().unwrap()) == 0. {
        points.pop();
    }
    Ok((points, closed))
}
fn resample(points: &[P], closed: bool, count: usize) -> Vec<P> {
    let mut lengths = vec![0.];
    let edges = if closed {
        points.len()
    } else {
        points.len() - 1
    };
    for i in 0..edges {
        lengths.push(lengths[i] + distance(points[i], points[(i + 1) % points.len()]));
    }
    let total = *lengths.last().unwrap();
    if total == 0. {
        return vec![points[0]; count];
    }
    let mut result = Vec::with_capacity(count);
    let mut edge = 0;
    for i in 0..count {
        let target = total * i as f64 / if closed { count } else { count - 1 } as f64;
        while edge + 1 < edges && lengths[edge + 1] < target {
            edge += 1;
        }
        let t = if lengths[edge + 1] == lengths[edge] {
            0.
        } else {
            (target - lengths[edge]) / (lengths[edge + 1] - lengths[edge])
        };
        let a = points[edge];
        let b = points[(edge + 1) % points.len()];
        result.push(P::new(a.x * (1. - t) + b.x * t, a.y * (1. - t) + b.y * t));
    }
    result
}
/// Sample one contour. Approximation error includes bounded cubic flattening
/// and the caller-chosen sample density. Does not claim semantic equivalence.
pub fn sampled_geometry(geometry: &Geometry, options: &PathMorphOptions) -> Result<Geometry> {
    options.validate()?;
    validate_geometry(geometry)?;
    sample(geometry, options, None, &mut 0)
}
/// Sample explicit contour slots, preserving their input winding. `order`
/// describes the keys of input contours in Move order; missing keys collapse.
/// This does not perform hole classification or guarantee intermediate topology.
pub fn sampled_contours(
    geometry: &Geometry,
    options: &ContourMorphOptions,
    order: Option<&[String]>,
) -> Result<Geometry> {
    options.validate()?;
    validate_geometry(geometry)?;
    sample_contours(geometry, options, order, None, &mut 0)
}
fn split_contours(geometry: &Geometry) -> Result<Vec<Geometry>> {
    let Geometry::Path(path) = geometry else {
        return Ok(vec![geometry.clone()]);
    };
    ensure(
        path.len() <= MAX_MORPH_CONTOURS * (MAX_MORPH_SAMPLES + 1),
        "multi contour command budget exceeded",
    )?;
    let mut result = Vec::new();
    let mut current = Vec::new();
    for command in path {
        if matches!(command, C::Move(_)) && !current.is_empty() {
            result.push(Geometry::Path(std::mem::take(&mut current)));
        }
        current.push(*command);
        ensure(
            result.len() < MAX_MORPH_CONTOURS,
            "multi contour source count budget exceeded",
        )?;
    }
    if !current.is_empty() {
        result.push(Geometry::Path(current));
    }
    Ok(result)
}
fn signed_area(geometry: &Geometry) -> f64 {
    let Geometry::Path(path) = geometry else {
        return 0.;
    };
    let points: Vec<_> = path
        .iter()
        .filter_map(|command| match command {
            C::Move(p) | C::Line(p) => Some(*p),
            _ => None,
        })
        .collect();
    if points.is_empty() {
        return 0.;
    }
    let origin = points[0];
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(a, b)| (a.x - origin.x) * (b.y - origin.y) - (b.x - origin.x) * (a.y - origin.y))
        .sum()
}
fn sample_contours(
    geometry: &Geometry,
    options: &ContourMorphOptions,
    order: Option<&[String]>,
    reference: Option<&Geometry>,
    work: &mut usize,
) -> Result<Geometry> {
    if let Geometry::Path(path) = geometry {
        ensure(
            path.len() <= MAX_FLAT_POINTS,
            "multi contour source command budget exceeded",
        )?;
    }
    let input = split_contours(geometry)?;
    let order: Vec<&str> = order
        .map(|keys| keys.iter().map(String::as_str).collect())
        .unwrap_or_else(|| {
            options
                .contours
                .iter()
                .map(|slot| slot.key.as_str())
                .collect()
        });
    ensure(
        order.len() == input.len(),
        "contour binding count differs from input contours",
    )?;
    let mut seen = std::collections::BTreeSet::new();
    for key in &order {
        ensure(
            seen.insert(*key) && options.contours.iter().any(|slot| slot.key == *key),
            "unknown or duplicate contour binding key",
        )?;
    }
    let references = reference.map(split_contours).transpose()?;
    if let Some(references) = &references {
        ensure(
            references.len() == options.contours.len(),
            "multi contour reference is not canonical",
        )?;
    }
    let mut path = Vec::with_capacity(options.contours.len() * (options.sampling.samples + 1));
    for (i, slot) in options.contours.iter().enumerate() {
        let reference = references.as_ref().map(|references| &references[i]);
        let sampled = if let Some(index) = order.iter().position(|key| *key == slot.key) {
            let closed = match &input[index] {
                Geometry::Path(commands) => commands.last() == Some(&C::Close),
                _ => true,
            };
            ensure(
                closed == slot.closed,
                "contour closed/open topology differs from its declared slot",
            )?;
            let sampled = sample(&input[index], &options.sampling, reference, work)?;
            if slot.closed
                && let Some(reference) = reference
            {
                let a = signed_area(reference);
                let b = signed_area(&sampled);
                ensure(
                    a.abs() <= 1e-8 || b.abs() <= 1e-8 || a.signum() == b.signum(),
                    "multi contour winding changes require a separate correspondence",
                )?;
            }
            sampled
        } else {
            spend(work, options.sampling.samples)?;
            let p = P::new(slot.collapse[0], slot.collapse[1]);
            let mut commands = vec![C::Move(p)];
            commands.extend((1..options.sampling.samples).map(|_| C::Line(p)));
            if slot.closed {
                commands.push(C::Close);
            }
            Geometry::Path(commands)
        };
        let Geometry::Path(commands) = sampled else {
            unreachable!()
        };
        path.extend(commands);
    }
    Ok(Geometry::Path(path))
}
pub(super) fn sample(
    geometry: &Geometry,
    o: &PathMorphOptions,
    reference: Option<&Geometry>,
    work: &mut usize,
) -> Result<Geometry> {
    let (points, closed) = flatten(geometry, o, work)?;
    ensure(
        !closed || o.samples >= 3,
        "closed morph contours require at least three samples",
    )?;
    spend(work, o.samples)?;
    let mut points = resample(&points, closed, o.samples);
    if closed
        && o.alignment != ClosedAlignment::PreserveStart
        && let Some(Geometry::Path(path)) = reference
    {
        let reference: Vec<_> = path
            .iter()
            .filter_map(|c| match c {
                C::Move(p) | C::Line(p) => Some(*p),
                _ => None,
            })
            .collect();
        if path.last() == Some(&C::Close) && reference.len() == points.len() {
            let n = points.len();
            let mut best = (f64::INFINITY, 0, false);
            for reverse in [false, true] {
                if reverse && o.alignment != ClosedAlignment::RotateAndReverse {
                    continue;
                }
                for shift in 0..n {
                    spend(work, n)?;
                    let score = (0..n)
                        .map(|i| {
                            let p = points[if reverse {
                                (shift + n - i) % n
                            } else {
                                (shift + i) % n
                            }];
                            let d = distance(p, reference[i]);
                            d * d
                        })
                        .sum::<f64>();
                    if score < best.0 {
                        best = (score, shift, reverse);
                    }
                }
            }
            points = (0..n)
                .map(|i| {
                    points[if best.2 {
                        (best.1 + n - i) % n
                    } else {
                        (best.1 + i) % n
                    }]
                })
                .collect();
        }
    }
    let mut path = vec![C::Move(points[0])];
    path.extend(points.into_iter().skip(1).map(C::Line));
    if closed {
        path.push(C::Close);
    }
    Ok(Geometry::Path(path))
}
pub(super) fn command_count(mark: &Mark, config: &GeometryTransitions) -> usize {
    if let Some(options) = config.contour_morphs.get(&mark.id) {
        options
            .contours
            .iter()
            .map(|slot| options.sampling.samples + usize::from(slot.closed))
            .sum()
    } else if let Some(options) = config.morphs.get(&mark.id) {
        let closed = match &mark.geometry {
            Geometry::Path(path) => path.last() == Some(&C::Close),
            _ => true,
        };
        options.samples + usize::from(closed)
    } else {
        match &mark.geometry {
            Geometry::Path(path) => path.len(),
            _ => 0,
        }
    }
}
pub(super) fn prepare(
    mark: &Mark,
    config: &GeometryTransitions,
    reference: Option<&Geometry>,
    work: &mut usize,
) -> Result<Mark> {
    prepare_bound(mark, config, None, reference, work)
}
pub(super) fn prepare_bound(
    mark: &Mark,
    config: &GeometryTransitions,
    order: Option<&[String]>,
    reference: Option<&Geometry>,
    work: &mut usize,
) -> Result<Mark> {
    let mut mark = mark.clone();
    if let Some(options) = config.morphs.get(&mark.id) {
        mark.geometry = sample(&mark.geometry, options, reference, work)?;
    } else if let Some(options) = config.contour_morphs.get(&mark.id) {
        mark.geometry = sample_contours(&mark.geometry, options, order, reference, work)?;
    }
    Ok(mark)
}
