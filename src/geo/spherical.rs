//! Checked, allocation-free spherical measurement over borrowed typed geometry.
//! Equations and compensated addition follow d3-geo3.1.1 / d3-array3.2.5 (ISC,
//! Mike Bostock); complete original notices: tests/fixtures/geo-inverse-d3-LICENSE.txt.
use super::{GeoFeature, Result, ensure, valid_position};
use std::f64::consts::{FRAC_PI_4, PI, TAU};
const MAX_SOURCE: usize = 100_000;
const MAX_PARTS: usize = 4096;
/// Borrowed spherical geometry. Rings must be exactly closed with >=4 positions.
/// Measurements preserve authored orientation and algebraic winding; they do
/// not validate simple topology, hole containment, geographic union or picking.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GeoSphericalGeometry<'a> {
    Empty,
    Sphere,
    Point([f64; 2]),
    MultiPoint(&'a [[f64; 2]]),
    LineString(&'a [[f64; 2]]),
    MultiLineString(&'a [Vec<[f64; 2]>]),
    Polygon(&'a [Vec<[f64; 2]>]),
    MultiPolygon(&'a [Vec<Vec<[f64; 2]>>]),
    GeometryCollection(&'a [GeoSphericalGeometry<'a>]),
}
impl<'a> From<&'a GeoFeature> for GeoSphericalGeometry<'a> {
    fn from(feature: &'a GeoFeature) -> Self {
        Self::MultiPolygon(&feature.polygons)
    }
}
#[derive(Default)]
struct Budget {
    points: usize,
    parts: usize,
}
impl Budget {
    fn part(&mut self) -> Result<()> {
        self.parts += 1;
        ensure(
            self.parts <= MAX_PARTS,
            "spherical geometry part budget exceeded",
        )
    }
    fn points(&mut self, p: &[[f64; 2]]) -> Result<()> {
        ensure(
            p.len() <= MAX_SOURCE.saturating_sub(self.points),
            "spherical source point budget exceeded",
        )?;
        self.points += p.len();
        for p in p {
            valid_position(*p)?;
        }
        Ok(())
    }
}
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
    let [la, pa] = a.map(|d| d * (PI / 180.));
    let [lb, pb] = b.map(|d| d * (PI / 180.));
    let delta = (lb - la).abs();
    let (sa, ca) = (pa.sin(), pa.cos());
    let (sb, cb) = (pb.sin(), pb.cos());
    let x = cb * delta.sin();
    let y = ca * sb - sa * cb * delta.cos();
    let z = sa * sb + ca * cb * delta.cos();
    (x * x + y * y).sqrt().atan2(z)
}
/// Minor great-circle angular distance in radians, including antipodal pi.
/// Longitude at a pole is immaterial; no route or unique antipodal arc is implied.
pub fn geo_spherical_distance(a: [f64; 2], b: [f64; 2]) -> Result<f64> {
    valid_position(a)?;
    valid_position(b)?;
    Ok(distance(a, b))
}
#[derive(Clone, Copy)]
enum Measure {
    Length,
    Area,
}
fn line_length(points: &[[f64; 2]], sum: &mut Adder) -> Result<()> {
    for edge in points.windows(2) {
        sum.add(distance(edge[0], edge[1]))?;
    }
    Ok(())
}
fn polygon(
    rings: &[Vec<[f64; 2]>],
    mode: Measure,
    budget: &mut Budget,
    total: &mut Adder,
) -> Result<()> {
    let mut area = Adder::default();
    for ring in rings {
        budget.part()?;
        budget.points(ring)?;
        ensure(
            ring.len() >= 4 && ring.first() == ring.last(),
            "spherical rings require at least four positions and exact closure",
        )?;
        let points = &ring[..ring.len() - 1];
        if matches!(mode, Measure::Length) {
            line_length(points, total)?;
            continue;
        }
        for i in 0..points.len() {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            ensure(
                distance(a, b) < PI - 1e-8,
                "spherical area antipodal edge is ambiguous",
            )?;
            let la = a[0] * (PI / 180.);
            let lb = b[0] * (PI / 180.);
            let pa = a[1] * (PI / 180.) / 2. + FRAC_PI_4;
            let pb = b[1] * (PI / 180.) / 2. + FRAC_PI_4;
            let delta = lb - la;
            let sign = if delta >= 0. { 1. } else { -1. };
            let ad = sign * delta;
            let k = pa.sin() * pb.sin();
            area.add((k * sign * ad.sin()).atan2(pa.cos() * pb.cos() + k * ad.cos()))?;
        }
    }
    if matches!(mode, Measure::Area) {
        let a = area.value();
        total.add(if a < 0. { TAU + a } else { a })?;
    }
    Ok(())
}
fn visit(
    g: GeoSphericalGeometry<'_>,
    mode: Measure,
    depth: usize,
    budget: &mut Budget,
    sum: &mut Adder,
) -> Result<()> {
    ensure(depth <= 32, "spherical geometry nesting budget exceeded")?;
    budget.part()?;
    match g {
        GeoSphericalGeometry::Empty => {}
        GeoSphericalGeometry::Sphere => {
            if matches!(mode, Measure::Area) {
                sum.add(TAU)?;
            }
        }
        GeoSphericalGeometry::Point(p) => budget.points(&[p])?,
        GeoSphericalGeometry::MultiPoint(p) => budget.points(p)?,
        GeoSphericalGeometry::LineString(p) => {
            budget.points(p)?;
            if matches!(mode, Measure::Length) {
                line_length(p, sum)?;
            }
        }
        GeoSphericalGeometry::MultiLineString(lines) => {
            for p in lines {
                budget.part()?;
                budget.points(p)?;
                if matches!(mode, Measure::Length) {
                    line_length(p, sum)?;
                }
            }
        }
        GeoSphericalGeometry::Polygon(p) => polygon(p, mode, budget, sum)?,
        GeoSphericalGeometry::MultiPolygon(polygons) => {
            for p in polygons {
                budget.part()?;
                polygon(p, mode, budget, sum)?;
            }
        }
        GeoSphericalGeometry::GeometryCollection(parts) => {
            for p in parts {
                visit(*p, mode, depth + 1, budget, sum)?;
            }
        }
    }
    Ok(())
}
/// D3-compatible streamed angular length in radians. Polygon streams omit the
/// duplicated closing vertex and do not add a closing edge: this is the original
/// geoLength behavior, NOT an automatically closed geometric perimeter.
pub fn geo_spherical_length(geometry: GeoSphericalGeometry<'_>) -> Result<f64> {
    let mut sum = Adder::default();
    visit(
        geometry,
        Measure::Length,
        0,
        &mut Budget::default(),
        &mut sum,
    )?;
    Ok(sum.value())
}
/// D3-compatible oriented spherical area in steradians. Clockwise small outer
/// rings and opposite holes select their intended region; reversed winding can
/// select its complement. Overlapping features add; no union/topology repair is
/// performed. Near-antipodal polygon edges (within1e-8rad) reject explicitly.
pub fn geo_spherical_area(geometry: GeoSphericalGeometry<'_>) -> Result<f64> {
    let mut sum = Adder::default();
    visit(geometry, Measure::Area, 0, &mut Budget::default(), &mut sum)?;
    let a = sum.value() * 2.;
    ensure(a.is_finite(), "spherical area overflow")?;
    Ok(a)
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoCircleOptions {
    pub center: [f64; 2],
    pub radius_degrees: f64,
    /// Angular sampling step (0,90] degrees; not projected-pixel precision.
    pub precision_degrees: f64,
    /// Includes every native sample and any appended exact closing position.
    pub max_points: usize,
}
impl Default for GeoCircleOptions {
    fn default() -> Self {
        Self {
            center: [0., 0.],
            radius_degrees: 90.,
            precision_degrees: 2.,
            max_points: MAX_SOURCE,
        }
    }
}
/// Return D3 circle samples in degrees with exact closure. When the last
/// native sample is within1e-12 radians of the first, canonicalize that sample
/// to the first to avoid a tiny duplicate closing edge; otherwise append it. Degenerate radius0/180 retain native
/// samples; no nondegenerate renderer/GeoJSON-ingestion guarantee is implied.
pub fn geo_circle(o: &GeoCircleOptions) -> Result<Vec<[f64; 2]>> {
    valid_position(o.center)?;
    ensure(
        o.radius_degrees.is_finite()
            && (0. ..=180.).contains(&o.radius_degrees)
            && o.precision_degrees.is_finite()
            && o.precision_degrees > 0.
            && o.precision_degrees <= 90.
            && (4..=MAX_SOURCE).contains(&o.max_points),
        "invalid spherical circle options",
    )?;
    let radius = o.radius_degrees * (PI / 180.);
    let step = o.precision_degrees * (PI / 180.);
    ensure(
        (360. / o.precision_degrees).floor() + 1. <= o.max_points as f64,
        "spherical circle point budget exceeded",
    )?;
    let (cr, sr) = (radius.cos(), radius.sin());
    let l = -o.center[0] * (PI / 180.);
    let f = -o.center[1] * (PI / 180.);
    let (cf, sf) = (f.cos(), f.sin());
    let mut t = radius + TAU;
    let end = radius - step / 2.;
    let mut points = Vec::new();
    while t > end {
        ensure(
            points.len() < o.max_points,
            "spherical circle point budget exceeded",
        )?;
        let x = cr;
        let y = -sr * t.cos();
        let z = -sr * t.sin();
        let phi = z.clamp(-1., 1.).asin();
        let lambda = y.atan2(x);
        let (longitude, latitude) = if f == 0. {
            (if l == 0. { lambda } else { lambda - l }, phi)
        } else {
            let x = lambda.cos() * phi.cos();
            let y = lambda.sin() * phi.cos();
            let z = phi.sin();
            (
                {
                    let angle = y.atan2(x * cf + z * sf);
                    if l == 0. { angle } else { angle - l }
                },
                (z * cf - x * sf).clamp(-1., 1.).asin(),
            )
        };
        let mut longitude = longitude;
        if longitude > PI {
            longitude -= TAU;
        } else if longitude < -PI {
            longitude += TAU;
        }
        let p = [longitude * (180. / PI), latitude * (180. / PI)];
        valid_position(p)?;
        points.push(p);
        let next = t - step;
        ensure(next < t, "spherical circle precision cannot advance")?;
        t = next;
    }
    if distance(points[0], *points.last().expect("circle has samples")) <= 1e-12 {
        *points.last_mut().expect("circle has samples") = points[0];
    } else {
        ensure(
            points.len() < o.max_points,
            "spherical circle closure budget exceeded",
        )?;
        points.push(points[0]);
    }
    Ok(points)
}
