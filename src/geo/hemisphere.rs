//! Directed boundary rejoining for a spherical cap, at most a hemisphere.
//!
//! Primary reference: d3-geo/src/clip/{rejoin,circle}.js. Within the declared
//! open-hemisphere polygon profile a gnomonic frame sends great circles and
//! great-circle boundaries to straight lines. We retain directed subject edges,
//! sort unmatched endpoints along the clip circle, and reconnect only its interior
//! intervals. This produces separate contours rather than a false chord
//! between disconnected visible lobes. Rendering samples the spherical arcs.
use super::{
    ChartError, EPS, MAX_POINTS, Result, Ring, V, ensure, hemisphere_center, inside, resample,
    winding,
};

const MAX_WORK: usize = 16_000_000;
const TOL: f64 = 1e-11;
pub(super) struct Clipper {
    work: usize,
    angle: f64,
    capture_edges: bool,
    circle_edges: std::collections::BTreeSet<([u64; 3], [u64; 3])>,
}
impl Clipper {
    pub(super) fn new(angle: f64) -> Self {
        Self {
            work: 0,
            angle,
            capture_edges: false,
            circle_edges: Default::default(),
        }
    }
    pub(super) fn capture_boundary_edges(&mut self) {
        self.capture_edges = true;
    }
    fn record_circle_edge(&mut self, a: V, b: V) {
        if self.capture_edges {
            self.circle_edges.insert((
                [a.x.to_bits(), a.y.to_bits(), a.z.to_bits()],
                [b.x.to_bits(), b.y.to_bits(), b.z.to_bits()],
            ));
        }
    }
    pub(super) fn is_circle_edge(&self, a: V, b: V) -> bool {
        self.circle_edges.contains(&(
            [a.x.to_bits(), a.y.to_bits(), a.z.to_bits()],
            [b.x.to_bits(), b.y.to_bits(), b.z.to_bits()],
        ))
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or_else(|| ChartError("spherical clip work overflow".into()))?;
        ensure(
            self.work <= MAX_WORK,
            "spherical clip topology work budget exceeded",
        )
    }
    pub(super) fn polygon(&mut self, rings: &[Vec<V>]) -> Result<Vec<Vec<Vec<V>>>> {
        self.circle_edges.clear();
        let frame = Frame::new(&rings[0])?;
        let mut planar = Vec::with_capacity(rings.len());
        for ring in rings {
            let points = ring
                .iter()
                .map(|p| frame.project(*p))
                .collect::<Result<Vec<_>>>()?;
            self.simple(&points)?;
            planar.push(points);
        }
        for i in 1..rings.len() {
            self.charge(planar[0].len() * planar[i].len())?;
            ensure(
                !boundaries_cross(&planar[0], &planar[i]),
                "spherical hole touches or crosses exterior",
            )?;
            self.charge(planar[0].len() * planar[i].len())?;
            ensure(
                planar[i].iter().all(|p| inside(*p, &planar[0])),
                "spherical hole lies outside exterior",
            )?;
            for j in 1..i {
                self.charge(planar[i].len() * planar[j].len() + planar[i].len() + planar[j].len())?;
                ensure(
                    !boundaries_cross(&planar[i], &planar[j])
                        && !inside(planar[i][0], &planar[j])
                        && !inside(planar[j][0], &planar[i]),
                    "spherical holes overlap, nest or touch",
                )?;
            }
        }
        let contours = self.rings(rings, &planar, &frame)?;
        let mut exteriors = contours
            .iter()
            .filter(|r| disk_winding(r) > 0.)
            .map(|r| vec![r.clone()])
            .collect::<Vec<_>>();
        for hole in contours.into_iter().filter(|r| disk_winding(r) < 0.) {
            self.charge(exteriors.iter().map(|p| p[0].len()).sum())?;
            let owners = exteriors
                .iter()
                .enumerate()
                .filter(|(_, p)| {
                    inside(
                        [hole[0].y, hole[0].z],
                        &p[0].iter().map(|v| [v.y, v.z]).collect::<Vec<_>>(),
                    )
                })
                .map(|(i, _)| i)
                .collect::<Vec<_>>();
            ensure(
                owners.len() == 1,
                "ambiguous visible spherical hole ownership",
            )?;
            exteriors[owners[0]].push(hole);
        }
        Ok(exteriors)
    }
    fn simple(&mut self, p: &[[f64; 2]]) -> Result<()> {
        let n = p.len();
        self.charge(n * n)?;
        ensure(winding(p).abs() > TOL, "degenerate spherical ring area")?;
        for i in 0..n {
            let a = p[i];
            let b = p[(i + 1) % n];
            let c = p[(i + 2) % n];
            ensure(
                (a[0] - b[0]).hypot(a[1] - b[1]) > TOL,
                "duplicate spherical ring vertex",
            )?;
            if orient(a, b, c).abs() < TOL {
                ensure(
                    (a[0] - b[0]) * (c[0] - b[0]) + (a[1] - b[1]) * (c[1] - b[1]) <= 0.,
                    "backtracking spherical boundary",
                )?;
            }
            for j in i + 1..n {
                if j == i + 1 || i == 0 && j == n - 1 {
                    continue;
                }
                ensure(
                    !segments_cross(a, b, p[j], p[(j + 1) % n]),
                    "self-intersecting or touching spherical ring",
                )?;
            }
        }
        Ok(())
    }
    fn rings(
        &mut self,
        rings: &[Vec<V>],
        projected: &[Ring],
        frame: &Frame,
    ) -> Result<Vec<Vec<V>>> {
        use std::f64::consts::TAU;
        let count = rings.iter().map(Vec::len).sum::<usize>();
        self.charge(count * 8)?;
        let cr = self.angle.cos();
        let sr = self.angle.sin();
        let tolerance = EPS * sr.max(1e-4);
        let tolerance = if self.angle < std::f64::consts::FRAC_PI_2 {
            tolerance.min(cr * 1e-6)
        } else {
            tolerance
        };
        let mut vertices = vec![];
        let mut ranges = vec![];
        for (index, (ring, planar)) in rings.iter().zip(projected).enumerate() {
            let start = vertices.len();
            if (winding(planar) > 0.) == (index == 0) {
                vertices.extend_from_slice(ring);
            } else {
                vertices.extend(ring.iter().rev().copied());
            }
            ranges.push(start..vertices.len());
        }
        for p in &mut vertices {
            if (p.x - cr).abs() <= tolerance {
                p.x = cr;
                let norm = p.y.hypot(p.z);
                ensure(norm > EPS, "ambiguous clip circle vertex")?;
                p.y *= sr / norm;
                p.z *= sr / norm;
            }
        }
        let contains = |p: V| -> bool {
            frame.project(p).is_ok_and(|p| {
                inside(p, &projected[0]) && !projected.iter().skip(1).any(|r| inside(p, r))
            })
        };
        let mut edges = vec![];
        for range in ranges {
            for i in range.clone() {
                let j = if i + 1 == range.end {
                    range.start
                } else {
                    i + 1
                };
                let a = vertices[i];
                let b = vertices[j];
                let cosine = a.dot(b).clamp(-1., 1.);
                let theta = a.cross(b).norm().atan2(cosine);
                ensure(
                    theta > EPS && theta < std::f64::consts::PI - 1e-8,
                    "ambiguous spherical clip edge",
                )?;
                let sine = theta.sin();
                let tangent = V {
                    x: (b.x - a.x * cosine) / sine,
                    y: (b.y - a.y * cosine) / sine,
                    z: (b.z - a.z * cosine) / sine,
                };
                let at = |t: f64| V {
                    x: a.x * t.cos() + tangent.x * t.sin(),
                    y: a.y * t.cos() + tangent.y * t.sin(),
                    z: a.z * t.cos() + tangent.z * t.sin(),
                };
                let mut cuts = vec![(0., i), (theta, j)];
                let radius = a.x.hypot(tangent.x);
                if radius > tolerance && cr <= radius + tolerance {
                    let phase = tangent.x.atan2(a.x);
                    let offset = (cr / radius).clamp(-1., 1.).acos();
                    for sign in [-1., 1.] {
                        for turn in -1..=1 {
                            let t = phase + sign * offset + f64::from(turn) * TAU;
                            if t > EPS
                                && t < theta - EPS
                                && !cuts.iter().any(|(u, _)| (t - u).abs() < EPS)
                            {
                                let mut p = at(t);
                                p.x = cr;
                                let norm = p.y.hypot(p.z);
                                p.y *= sr / norm;
                                p.z *= sr / norm;
                                let id = vertices.len();
                                vertices.push(p);
                                cuts.push((t, id));
                            }
                        }
                    }
                }
                cuts.sort_by(|a, b| a.0.total_cmp(&b.0));
                for pair in cuts.windows(2) {
                    let mid = at((pair[0].0 + pair[1].0) / 2.);
                    if mid.x < cr - tolerance {
                        continue;
                    }
                    // At a 90-degree cap an entire edge may coincide with the
                    // circle. Retain it only when its left-hand interior is visible.
                    if self.angle == std::f64::consts::FRAC_PI_2
                        && a.x == cr
                        && b.x == cr
                        && a.y * b.z - a.z * b.y <= 0.
                    {
                        continue;
                    }
                    edges.push((pair[0].1, pair[1].1));
                }
            }
        }
        let mut next = vec![None; vertices.len()];
        let mut incoming = vec![0usize; vertices.len()];
        for (a, b) in edges {
            ensure(
                next[a].replace((b, false)).is_none(),
                "ambiguous spherical clip branching",
            )?;
            incoming[b] += 1;
            ensure(incoming[b] <= 1, "ambiguous spherical clip junction")?;
        }
        let mut boundary = (0..vertices.len())
            .filter(|i| next[*i].is_some() != (incoming[*i] == 1))
            .collect::<Vec<_>>();
        let angle = |i: usize| vertices[i].z.atan2(vertices[i].y).rem_euclid(TAU);
        boundary.sort_by(|a, b| angle(*a).total_cmp(&angle(*b)));
        ensure(
            boundary.len() % 2 == 0,
            "odd spherical clip intersection topology",
        )?;
        for i in 0..boundary.len() {
            let a = boundary[i];
            let b = boundary[(i + 1) % boundary.len()];
            let delta = (angle(b) - angle(a)).rem_euclid(TAU);
            if delta < EPS {
                continue;
            }
            let mid = angle(a) + delta / 2.;
            self.charge(count)?;
            if !contains(V {
                x: cr,
                y: sr * mid.cos(),
                z: sr * mid.sin(),
            }) {
                continue;
            }
            ensure(
                next[a].is_none() && incoming[a] == 1 && next[b].is_some() && incoming[b] == 0,
                "ambiguous spherical clip circle endpoint order",
            )?;
            next[a] = Some((b, true));
            incoming[b] = 1;
        }
        let mut visited = vec![false; vertices.len()];
        let mut contours = vec![];
        for start in 0..vertices.len() {
            if visited[start] || next[start].is_none() {
                continue;
            }
            let mut current = start;
            let mut out = vec![];
            loop {
                ensure(!visited[current], "spherical clip path failed to close")?;
                visited[current] = true;
                let (end, circle) = next[current]
                    .ok_or_else(|| ChartError("open spherical clip contour".into()))?;
                out.push(vertices[current]);
                if circle {
                    let begin = angle(current);
                    let delta = (angle(end) - begin).rem_euclid(TAU);
                    let count = (delta / super::PRECISION).ceil().max(1.) as usize;
                    self.charge(count)?;
                    let mut previous = vertices[current];
                    for i in 1..count {
                        let a = begin + delta * i as f64 / count as f64;
                        let point = V {
                            x: cr,
                            y: sr * a.cos(),
                            z: sr * a.sin(),
                        };
                        self.record_circle_edge(previous, point);
                        previous = point;
                        out.push(point);
                    }
                    self.record_circle_edge(previous, vertices[end]);
                } else {
                    let mut samples = vec![];
                    resample(vertices[current], vertices[end], &mut samples)?;
                    self.charge(samples.len() + 1)?;
                    samples.pop();
                    out.extend(samples);
                }
                ensure(
                    out.len() <= MAX_POINTS,
                    "spherical clip sampled contour budget exceeded",
                )?;
                current = end;
                if current == start {
                    break;
                }
            }
            if out.len() >= 3 && disk_winding(&out).abs() > 1e-14 {
                contours.push(out);
            }
        }
        if boundary.is_empty() {
            self.charge(count * 3)?;
            // If the entire clip circle lies inside the polygon, it remains
            // an exterior even when enclosed subject holes already produced
            // clockwise contours. Conversely a cap hidden wholly by a hole
            // emits nothing, including no phantom outline.
            if (0..3).all(|i| {
                let a = TAU * (i as f64 / 3. + 0.07123456789);
                contains(V {
                    x: cr,
                    y: sr * a.cos(),
                    z: sr * a.sin(),
                })
            }) {
                let count = (TAU / super::PRECISION).ceil() as usize;
                self.charge(count)?;
                let circle = (0..count)
                    .map(|i| {
                        let a = TAU * i as f64 / count as f64;
                        V {
                            x: cr,
                            y: sr * a.cos(),
                            z: sr * a.sin(),
                        }
                    })
                    .collect::<Vec<_>>();
                for i in 0..circle.len() {
                    self.record_circle_edge(circle[i], circle[(i + 1) % circle.len()]);
                }
                contours.push(circle);
            }
        }
        Ok(contours)
    }
}
struct Frame {
    c: V,
    u: V,
    v: V,
}
impl Frame {
    fn new(ring: &[V]) -> Result<Self> {
        let c = hemisphere_center(ring)?;
        let axis = if c.z.abs() < 0.8 {
            V {
                x: 0.,
                y: 0.,
                z: 1.,
            }
        } else {
            V {
                x: 0.,
                y: 1.,
                z: 0.,
            }
        };
        let u = axis.cross(c).normalized()?;
        let v = c.cross(u);
        Ok(Self { c, u, v })
    }
    fn project(&self, p: V) -> Result<[f64; 2]> {
        let d = p.dot(self.c);
        ensure(
            d > 1e-8,
            "spherical polygon must fit a numerically bounded open hemisphere",
        )?;
        let q = [p.dot(self.u) / d, p.dot(self.v) / d];
        ensure(
            q.iter().all(|x| x.is_finite()),
            "gnomonic topology projection overflow",
        )?;
        Ok(q)
    }
}
fn orient(a: [f64; 2], b: [f64; 2], c: [f64; 2]) -> f64 {
    let [x, y] = [b[0] - a[0], b[1] - a[1]];
    let [u, v] = [c[0] - a[0], c[1] - a[1]];
    let scale = x.abs().max(y.abs()).max(u.abs()).max(v.abs()).max(1.);
    (x / scale) * (v / scale) - (y / scale) * (u / scale)
}
fn segments_cross(a: [f64; 2], b: [f64; 2], c: [f64; 2], d: [f64; 2]) -> bool {
    if a[0].max(b[0]) < c[0].min(d[0]) - TOL
        || c[0].max(d[0]) < a[0].min(b[0]) - TOL
        || a[1].max(b[1]) < c[1].min(d[1]) - TOL
        || c[1].max(d[1]) < a[1].min(b[1]) - TOL
    {
        return false;
    }
    let (ab_c, ab_d, cd_a, cd_b) = (
        orient(a, b, c),
        orient(a, b, d),
        orient(c, d, a),
        orient(c, d, b),
    );
    ab_c * ab_d <= TOL * TOL && cd_a * cd_b <= TOL * TOL
}
fn boundaries_cross(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
    (0..a.len()).any(|i| {
        (0..b.len()).any(|j| segments_cross(a[i], a[(i + 1) % a.len()], b[j], b[(j + 1) % b.len()]))
    })
}

fn disk_winding(ring: &[V]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a.y * b.z - b.y * a.z
        })
        .sum::<f64>()
        / 2.
}
