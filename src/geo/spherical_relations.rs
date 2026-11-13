//! Checked spherical centroid and containment over borrowed typed geometry.
//! Equations follow pinned d3-geo3.1.1 / d3-array3.2.5 (ISC, Mike Bostock).
//! Complete notices: tests/fixtures/geo-inverse-d3-LICENSE.txt.
use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

use super::{GeoSphericalGeometry as G, Result, ensure, valid_position};
const RADIANS: f64 = PI / 180.;
const EPS: f64 = 1e-6;
const EPS2: f64 = 1e-12;
#[derive(Default)]
struct Budget {
    points: usize,
    parts: usize,
}
impl Budget {
    fn part(&mut self) -> Result<()> {
        self.parts += 1;
        ensure(
            self.parts <= 4096,
            "spherical relation part budget exceeded",
        )
    }
    fn positions(&mut self, points: &[[f64; 2]], edges: bool) -> Result<()> {
        ensure(
            points.len() <= 100_000usize.saturating_sub(self.points),
            "spherical relation source point budget exceeded",
        )?;
        self.points += points.len();
        for p in points {
            valid_position(*p)?;
        }
        if edges {
            for p in points.windows(2) {
                ensure(
                    distance(p[0], p[1]) < PI - 1e-8,
                    "spherical relation antipodal edge is ambiguous",
                )?;
            }
        }
        Ok(())
    }
    fn polygon(&mut self, rings: &[Vec<[f64; 2]>]) -> Result<()> {
        for ring in rings {
            self.part()?;
            self.positions(ring, true)?;
            ensure(
                ring.len() >= 4 && ring.first() == ring.last(),
                "spherical relation rings require at least four positions and exact closure",
            )?;
        }
        Ok(())
    }
    fn geometry(&mut self, g: G<'_>, depth: usize) -> Result<()> {
        ensure(depth <= 32, "spherical relation nesting budget exceeded")?;
        self.part()?;
        match g {
            G::Empty | G::Sphere => {}
            G::Point(p) => self.positions(&[p], false)?,
            G::MultiPoint(p) => self.positions(p, false)?,
            G::LineString(p) => self.positions(p, true)?,
            G::MultiLineString(lines) => {
                for p in lines {
                    self.part()?;
                    self.positions(p, true)?;
                }
            }
            G::Polygon(rings) => self.polygon(rings)?,
            G::MultiPolygon(polygons) => {
                for p in polygons {
                    self.part()?;
                    self.polygon(p)?;
                }
            }
            G::GeometryCollection(parts) => {
                for p in parts {
                    self.geometry(*p, depth + 1)?;
                }
            }
        }
        Ok(())
    }
}
// Kept local so this isolated addition does not change the frozen measurement
// module's API or source bytes; both use the original bounded partial expansion.
// Original d3-array Adder's error-free partial expansion. Storage exhaustion is
// an explicit error rather than an out-of-bounds access or silent loss.
struct Adder {
    p: [f64; 32],
    n: usize,
}
impl Default for Adder {
    fn default() -> Self {
        Self { p: [0.; 32], n: 0 }
    }
}
impl Adder {
    fn add(&mut self, mut x: f64) -> Result<()> {
        let mut i = 0;
        for j in 0..self.n {
            let y = self.p[j];
            let hi = x + y;
            let lo = if x.abs() < y.abs() {
                x - (hi - y)
            } else {
                y - (hi - x)
            };
            if lo != 0. {
                ensure(i < 32, "spherical accumulator storage exhausted")?;
                self.p[i] = lo;
                i += 1;
            }
            x = hi;
        }
        ensure(
            i < 32 && x.is_finite(),
            "spherical accumulator is not representable",
        )?;
        self.p[i] = x;
        self.n = i + 1;
        Ok(())
    }
    fn value(&self) -> f64 {
        let mut n = self.n;
        let mut hi = 0.;
        let mut lo = 0.;
        if n > 0 {
            n -= 1;
            hi = self.p[n];
            while n > 0 {
                let x = hi;
                n -= 1;
                let y = self.p[n];
                hi = x + y;
                lo = y - (hi - x);
                if lo != 0. {
                    break;
                }
            }
            if n > 0 && ((lo < 0. && self.p[n - 1] < 0.) || (lo > 0. && self.p[n - 1] > 0.)) {
                let y = lo * 2.;
                let x = hi + y;
                if y == x - hi {
                    hi = x;
                }
            }
        }
        hi
    }
}

fn distance(a: [f64; 2], b: [f64; 2]) -> f64 {
    let [la, pa] = a.map(|x| x * RADIANS);
    let [lb, pb] = b.map(|x| x * RADIANS);
    let delta = (lb - la).abs();
    let (sa, ca) = (pa.sin(), pa.cos());
    let (sb, cb) = (pb.sin(), pb.cos());
    let x = cb * delta.sin();
    let y = ca * sb - sa * cb * delta.cos();
    let z = sa * sb + ca * cb * delta.cos();
    (x * x + y * y).sqrt().atan2(z)
}
fn cartesian([lambda, phi]: [f64; 2]) -> [f64; 3] {
    let c = phi.cos();
    [c * lambda.cos(), c * lambda.sin(), phi.sin()]
}
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn norm(a: [f64; 3]) -> f64 {
    a[0].hypot(a[1]).hypot(a[2])
}
fn asin(x: f64) -> f64 {
    if x > 1. {
        FRAC_PI_2
    } else if x < -1. {
        -FRAC_PI_2
    } else {
        x.asin()
    }
}
#[derive(Default)]
struct Centroid {
    count: f64,
    line_weight: f64,
    point: [f64; 3],
    line: [f64; 3],
    area: [Adder; 3],
    stable_area: [Adder; 3],
    area_roundoff: f64,
}
impl Centroid {
    // In a frame whose first axis is the ring's first point, form delta=v-origin
    // directly with half-angle identities. Closed-ring telescoping gives
    // sum(v_i cross v_j)=sum(delta_i cross delta_j), avoiding cancellation of
    // O(edge) terms when the actual surface moment is O(edge^2).
    // The remaining asin(m)/m-1 term is evaluated by its convergent even series
    // for m<1/8; twelve correction terms leave a remainder below f64 rounding.
    fn stable_ring_area(&mut self, points: &[[f64; 2]]) -> Result<()> {
        let origin = points[0];
        let phi = origin[1] * RADIANS;
        let lambda = origin[0] * RADIANS;
        let (cp, sp, cl, sl) = (phi.cos(), phi.sin(), lambda.cos(), lambda.sin());
        let delta = |p: [f64; 2]| {
            let dl = (p[0] - origin[0]) * RADIANS;
            let dp = (p[1] - origin[1]) * RADIANS;
            let half = 2. * (dl / 2.).sin().powi(2);
            let c = (p[1] * RADIANS).cos();
            [
                -2. * (dp / 2.).sin().powi(2) - c * cp * half,
                c * dl.sin(),
                dp.sin() + c * sp * half,
            ]
        };
        let mut sums: [Adder; 3] = Default::default();
        for i in 0..points.len() {
            let da = delta(points[i]);
            let db = delta(points[(i + 1) % points.len()]);
            let a = [1. + da[0], da[1], da[2]];
            let b = [1. + db[0], db[1], db[2]];
            let c = cross(a, b);
            let m = norm(c);
            let correction = if m < 0.125 {
                let t = m * m;
                let mut term = t / 6.;
                let mut sum = term;
                for n in 1..12 {
                    let n = n as f64;
                    term *= t * (2. * n + 1.).powi(2) / (2. * (n + 1.) * (2. * n + 3.));
                    sum += term;
                }
                sum
            } else {
                // asin(sin(theta)) is mathematically atan2(sin(theta),
                // abs(cos(theta))). The latter avoids the derivative blowup
                // of asin when the edge angle is close to 90 degrees, while
                // preserving the original >90-degree folded-angle semantics.
                let dot = a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
                m.atan2(dot.abs()) / m - 1.
            };
            let base = cross(da, db);
            for j in 0..3 {
                sums[j].add(-base[j])?;
                sums[j].add(-correction * c[j])?;
            }
        }
        let a = sums.each_ref().map(Adder::value);
        let global = [
            cp * cl * a[0] - sl * a[1] - sp * cl * a[2],
            cp * sl * a[0] + cl * a[1] - sp * sl * a[2],
            sp * a[0] + cp * a[2],
        ];
        for (sum, v) in self.stable_area.iter_mut().zip(global) {
            sum.add(v)?;
        }
        Ok(())
    }

    fn point(&mut self, p: [f64; 3]) {
        self.count += 1.;
        for (mean, v) in self.point.iter_mut().zip(p) {
            *mean += (v - *mean) / self.count;
        }
    }
    fn line(&mut self, positions: &[[f64; 2]], ring: bool) -> Result<()> {
        let Some(first) = positions.first() else {
            return Ok(());
        };
        let mut previous = cartesian(first.map(|x| x * RADIANS));
        self.point(previous);
        for source in positions.iter().skip(1).chain(ring.then_some(first)) {
            let next = cartesian(source.map(|x| x * RADIANS));
            let c = cross(previous, next);
            let w = if ring {
                let m = norm(c);
                let w = asin(m);
                let v = if m == 0. { 0. } else { -w / m };
                for (sum, value) in self.area.iter_mut().zip(c) {
                    sum.add(v * value)?;
                }
                let products = (previous[1] * next[2]).abs()
                    + (previous[2] * next[1]).abs()
                    + (previous[2] * next[0]).abs()
                    + (previous[0] * next[2]).abs()
                    + (previous[0] * next[1]).abs()
                    + (previous[1] * next[0]).abs();
                let slope = (1. - m * m).max(f64::EPSILON).sqrt();
                self.area_roundoff += 128. * f64::EPSILON * products * v.abs() / slope;

                w
            } else {
                let m = (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt();
                m.atan2(previous[0] * next[0] + previous[1] * next[1] + previous[2] * next[2])
            };
            self.line_weight += w;
            for (sum, (a, b)) in self.line.iter_mut().zip(previous.into_iter().zip(next)) {
                *sum += w * (a + b);
            }
            previous = next;
            self.point(previous);
        }
        Ok(())
    }
    fn polygon(&mut self, rings: &[Vec<[f64; 2]>]) -> Result<()> {
        for ring in rings {
            self.stable_ring_area(&ring[..ring.len() - 1])?;
            self.line(&ring[..ring.len() - 1], true)?;
        }
        Ok(())
    }
    fn geometry(&mut self, g: G<'_>) -> Result<()> {
        match g {
            G::Empty | G::Sphere => {}
            G::Point(p) => self.point(cartesian(p.map(|x| x * RADIANS))),
            G::MultiPoint(points) => {
                for p in points {
                    self.point(cartesian(p.map(|x| x * RADIANS)));
                }
            }
            G::LineString(p) => self.line(p, false)?,
            G::MultiLineString(lines) => {
                for p in lines {
                    self.line(p, false)?;
                }
            }
            G::Polygon(rings) => self.polygon(rings)?,
            G::MultiPolygon(polygons) => {
                for p in polygons {
                    self.polygon(p)?;
                }
            }
            G::GeometryCollection(parts) => {
                for p in parts {
                    self.geometry(*p)?;
                }
            }
        }
        Ok(())
    }
    fn value(&self) -> Result<Option<[f64; 2]>> {
        let mut vector = self.area.each_ref().map(Adder::value);
        let mut m = norm(vector);
        // The operation-product gamma128 estimate is an engineering trigger,
        // not a certified bound on every platform's libm. Its ratio to moment
        // norm estimates angular sensitivity in radians (1e-12 rad here).
        // Exact axial poles retain the original longitude convention; all
        // EPS2 area/line/point fallback policy remains unchanged.
        if m > 0. && (vector[0] != 0. || vector[1] != 0.) && self.area_roundoff / m > 1e-12 {
            vector = self.stable_area.each_ref().map(Adder::value);
            m = norm(vector);
        }
        if m < EPS2 {
            vector = self.line;
            if self.line_weight < EPS {
                vector = self.point;
            }
            m = norm(vector);
            if m < EPS2 {
                return Ok(None);
            }
        }
        let p = [
            vector[1].atan2(vector[0]) / RADIANS,
            asin(vector[2] / m) / RADIANS,
        ];
        ensure(
            p.iter().all(|v| v.is_finite()),
            "spherical centroid is not representable",
        )?;
        Ok(Some(p))
    }
}
/// D3-compatible area-, then length-, then point-weighted centroid in degrees.
/// Empty, Sphere alone, or a directionless cancellation returns None. Collections
/// accumulate authored contributions, not a geographic union. Every source is
/// validated before evaluation, including collection members after a Sphere.
pub fn geo_spherical_centroid(geometry: G<'_>) -> Result<Option<[f64; 2]>> {
    Budget::default().geometry(geometry, 0)?;
    let mut c = Centroid::default();
    c.geometry(geometry)?;
    c.value()
}
fn contains_line(points: &[[f64; 2]], point: [f64; 2]) -> bool {
    let mut ao = 0.;
    for (i, p) in points.iter().enumerate() {
        let bo = distance(*p, point);
        if bo == 0. {
            return true;
        }
        if i > 0 {
            let ab = distance(*p, points[i - 1]);
            if ab > 0.
                && ao <= ab
                && bo <= ab
                && (ao + bo - ab) * (1. - ((ao - bo) / ab).powi(2)) < EPS2 * ab
            {
                return true;
            }
        }
        ao = bo;
    }
    false
}
fn normalize(a: [f64; 3]) -> Option<[f64; 3]> {
    // Original polygonContains normalizes via sqrt(dot), not Math.hypot.
    let m = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
    if m == 0. {
        None
    } else {
        Some(a.map(|v| v / m))
    }
}
fn stable_source_cross(a: [f64; 2], b: [f64; 2]) -> [f64; 3] {
    // Analytic edge cross in the midpoint-longitude frame. Latitude and
    // longitude differences are formed in authored degrees before conversion,
    // rather than subtracting almost equal global Cartesian products.
    let half = (b[0] - a[0]) * (RADIANS / 2.);
    let mid = (a[0] + b[0]) * (RADIANS / 2.);
    let x = -half.sin() * ((a[1] + b[1]) * RADIANS).sin();
    let y = -half.cos() * ((b[1] - a[1]) * RADIANS).sin();
    let z = (a[1] * RADIANS).cos() * (b[1] * RADIANS).cos() * (2. * half).sin();
    [
        x * mid.cos() - y * mid.sin(),
        x * mid.sin() + y * mid.cos(),
        z,
    ]
}
fn contains_polygon(rings: &[Vec<[f64; 2]>], point: [f64; 2]) -> Result<bool> {
    let [lambda, mut phi] = point.map(|x| x * RADIANS);
    let sin_phi = phi.sin();
    let vertex = rings.iter().flatten().any(|p| *p == point);
    let normal = [lambda.sin(), -lambda.cos(), 0.];
    let (mut angle, mut winding) = (0., 0i32);
    let mut sum = Adder::default();
    if sin_phi == 1. {
        phi = FRAC_PI_2 + EPS;
    } else if sin_phi == -1. {
        phi = -FRAC_PI_2 - EPS;
    }
    for ring in rings {
        let points = &ring[..ring.len() - 1];
        let mut previous_source = points[points.len() - 1];
        let mut p0 = previous_source.map(|v| v * RADIANS);
        let mut lambda0 = p0[0];
        let phi0 = p0[1] / 2. + FRAC_PI_4;
        let (mut sin_phi0, mut cos_phi0) = (phi0.sin(), phi0.cos());
        for source in points {
            let p1 = source.map(|v| v * RADIANS);
            let lambda1 = p1[0];
            let phi1 = p1[1] / 2. + FRAC_PI_4;
            let (sin_phi1, cos_phi1) = (phi1.sin(), phi1.cos());
            let delta = lambda1 - lambda0;
            let sign = if delta >= 0. { 1. } else { -1. };
            let abs_delta = sign * delta;
            let anti = abs_delta > PI;
            let k = sin_phi0 * sin_phi1;
            sum.add((k * sign * abs_delta.sin()).atan2(cos_phi0 * cos_phi1 + k * abs_delta.cos()))?;
            angle += if anti { delta + sign * TAU } else { delta };
            if anti ^ (lambda0 >= lambda) ^ (lambda1 >= lambda) {
                // A zero cross/intersection produces NaN in original D3 and
                // contributes no crossing. Preserve that duplicate-edge case
                // without emitting nonfinite public coordinates.
                let raw = cross(cartesian(p0), cartesian(p1));
                let positive = anti ^ (delta >= 0.);
                let mut use_stable = false;
                if !vertex
                    && let Some(a) = normalize(raw)
                    && let Some(i) = normalize(cross(normal, a))
                {
                    let old_phi = (if positive { -1. } else { 1. }) * asin(i[2]);
                    let bound = 128. * f64::EPSILON
                        / (norm(raw) * norm(cross(normal, a)) * old_phi.cos().abs().max(EPS));
                    use_stable = (phi - old_phi).abs() <= bound;
                }
                if let Some(arc) = normalize(if use_stable {
                    stable_source_cross(previous_source, *source)
                } else {
                    raw
                }) && let Some(intersection) = normalize(cross(normal, arc))
                {
                    let positive = anti ^ (delta >= 0.);
                    let phi_arc = (if positive { -1. } else { 1. }) * asin(intersection[2]);
                    if phi > phi_arc || phi == phi_arc && (arc[0] != 0. || arc[1] != 0.) {
                        winding += if positive { 1 } else { -1 };
                    }
                }
            }
            p0 = p1;
            previous_source = *source;
            lambda0 = lambda1;
            sin_phi0 = sin_phi1;
            cos_phi0 = cos_phi1;
        }
    }
    Ok((angle < -EPS || angle < EPS && sum.value() < -EPS2) ^ (winding & 1 != 0))
}
fn contains_geometry(g: G<'_>, point: [f64; 2]) -> Result<bool> {
    match g {
        G::Empty => Ok(false),
        G::Sphere => Ok(true),
        G::Point(p) => Ok(distance(p, point) == 0.),
        G::MultiPoint(p) => Ok(p.iter().any(|p| distance(*p, point) == 0.)),
        G::LineString(p) => Ok(contains_line(p, point)),
        G::MultiLineString(lines) => Ok(lines.iter().any(|p| contains_line(p, point))),
        G::Polygon(rings) => contains_polygon(rings, point),
        G::MultiPolygon(polygons) => {
            for p in polygons {
                if contains_polygon(p, point)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        G::GeometryCollection(parts) => {
            for g in parts {
                if contains_geometry(*g, point)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
    }
}
/// Original D3 spherical containment, including its exact Point distance and
/// epsilon-based Line/Polygon boundary rules. This is not planar Scene picking,
/// a uniform boundary-inclusive predicate, or topology repair. All coordinates
/// are finite/canonical; line and ring edges within 1e-8rad of antipodal reject.
pub fn geo_spherical_contains(geometry: G<'_>, point: [f64; 2]) -> Result<bool> {
    valid_position(point)?;
    Budget::default().geometry(geometry, 0)?;
    contains_geometry(geometry, point)
}
