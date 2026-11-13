//! Geographic projections and bounded polygon clipping.
//!
//! Projection equations match D3's equalEarthRaw/naturalEarth1Raw and standard
//! spherical Mercator/orthographic formulas. Coordinates and view angles use
//! degrees; projected raw units use radians (orthographic uses unit radius).
//! Edges follow minor great-circle arcs, tessellated at at most two degrees.
//! Antimeridian and azimuthal clip-circle intersections are solved on the sphere.
//! Nonconvex simple polygons can produce multiple visible contours; holes retain
//! their nonzero winding. Azimuthal polygons must fit a bounded open hemisphere
//! before clipping; antipodal, self-intersecting or ambiguous spherical topology
//! is rejected explicitly. Azimuthal clip angles are at most 90°; Gnomonic uses
//! 60° by default and requires less than 90°, matching its singularity.
use crate::scene::ensure;
use crate::{ChartConfig, ChartError, Color, GeoFeature, Geometry, Mark, Result, Scene};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};
use std::collections::BTreeSet;
use std::f64::consts::{FRAC_PI_2, PI, TAU};
const EPS: f64 = 1e-10;
const PRECISION: f64 = PI / 90.;
const MERCATOR_LIMIT: f64 = 1.4844222297453324;
const MAX_POINTS: usize = 100_000;

#[path = "geo/hemisphere.rs"]
mod hemisphere;
#[path = "geo_symbols.rs"]
mod symbols;
pub use symbols::*;
mod path;
pub use path::{GeoPathOptions, GeoPathStats};
mod stream;
pub use stream::*;
mod spherical;
pub use spherical::*;
mod spherical_relations;
pub use spherical_relations::*;
mod conic;
mod conic_kernel;
mod conic_mask;
mod conic_seam;
mod inverse;
pub use conic::GeoConicParallels;
type Ring = Vec<[f64; 2]>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoProjection {
    #[default]
    Equirectangular,
    Mercator,
    /// D3's spherical transverse Mercator, including its +90-degree frame.
    TransverseMercator,
    EqualEarth,
    NaturalEarth,
    Orthographic,
    /// True Lambert azimuthal equal-area, bounded to the front hemisphere.
    AzimuthalEqualArea,
    /// True azimuthal equidistant, bounded to the front hemisphere.
    AzimuthalEquidistant,
    /// True stereographic, bounded to the front hemisphere (radius <= 1).
    Stereographic,
    /// Great circles project to straight lines; default front cap is 60°.
    /// Configured clip angles must stay strictly below the 90° singularity.
    Gnomonic,
    ConicEqualArea(GeoConicParallels),
    ConicEquidistant(GeoConicParallels),
    /// Finite view bounded to rotated latitude +/-85.0511287798066 degrees.
    ConicConformal(GeoConicParallels),
    /// Equal-area [29.5,45.5], intrinsic longitude rotation +96 degrees and
    /// projected origin [-0.6,38.7]. Author camera remains independent.
    Albers,
}
impl GeoProjection {
    fn hemisphere(self) -> bool {
        matches!(
            self,
            Self::Orthographic
                | Self::AzimuthalEqualArea
                | Self::AzimuthalEquidistant
                | Self::Stereographic
                | Self::Gnomonic
        )
    }
    fn disk_radius(self, angle: f64) -> f64 {
        match self {
            Self::AzimuthalEqualArea => 2. * (angle / 2.).sin(),
            Self::AzimuthalEquidistant => angle,
            Self::Stereographic => (angle / 2.).tan(),
            Self::Gnomonic => angle.tan(),
            _ => angle.sin(),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum GeoFit {
    #[default]
    World,
    Data,
}
#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapOptions {
    pub projection: GeoProjection,
    /// Spherical azimuthal cap in degrees (0.01..=90). None means 90°, or
    /// 60° for Gnomonic. Non-azimuthal projections reject an explicit cap.
    pub clip_angle: Option<f64>,
    /// Spherical camera center, longitude/latitude in degrees.
    pub center: [f64; 2],
    /// Clockwise view roll in degrees, applied before projection.
    pub rotation: f64,
    pub fit: GeoFit,
    pub padding: f64,
    /// Logical pixels per raw unit; overrides automatic fit scaling.
    pub scale: Option<f64>,
    /// Logical pixel translation of the projection's fitted center.
    pub translate: Option<[f64; 2]>,
    pub fill: Option<Color>,
    pub border: Option<Color>,
    pub border_width: f64,
}

/// A fitted projection shared by a polygon map and any point overlays. Data
/// fitting uses the visible, clipped polygon bounds, never the overlay points.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoProjectionContext {
    view: GeoMapOptions,
    center: [f64; 2],
    scale: f64,
    translate: [f64; 2],
    dimensions: [f64; 2],
    plot: crate::ClipRect,
    path_options: Option<GeoPathOptions>,
}
/// The logical-pixel circular boundary of an azimuthal projection, after
/// fitting, scaling and translation. Non-azimuthal projections have no disk.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoProjectionDisk {
    pub center: [f64; 2],
    pub radius: f64,
}
impl GeoProjectionContext {
    pub fn projection_disk(&self) -> Option<GeoProjectionDisk> {
        self.view
            .projection
            .hemisphere()
            .then(|| GeoProjectionDisk {
                center: self.map_raw([0., 0.]),
                radius: self.scale * self.view.projection.disk_radius(clip_angle(&self.view)),
            })
    }
    /// Project longitude/latitude into the exact logical pixels used by the
    /// map. Returns None behind a hemisphere projection or outside Mercator's cap.
    /// Longitude wraps once about the camera through spherical atan2, just as
    /// the map's antimeridian clipping does; points are not duplicated at seams.
    pub fn project(&self, position: [f64; 2]) -> Result<Option<[f64; 2]>> {
        project_geo_point(position, &self.view)?
            .map(|point| {
                let point = self.map_raw(point);
                ensure(
                    point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                    "projected geographic point exceeds Scene coordinates",
                )?;
                Ok(point)
            })
            .transpose()
    }
    fn map_raw(&self, point: [f64; 2]) -> [f64; 2] {
        [
            self.translate[0] + (point[0] - self.center[0]) * self.scale,
            self.translate[1] - (point[1] - self.center[1]) * self.scale,
        ]
    }
    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn translate(&self) -> [f64; 2] {
        self.translate
    }
    /// Midpoint of the accepted bounds in projected raw units, not geographic
    /// longitude/latitude. Together with scale/translate it describes Data fit.
    pub fn fitted_raw_center(&self) -> [f64; 2] {
        self.center
    }
    /// Logical Scene dimensions used when this projection context was fitted.
    pub fn dimensions(&self) -> [f64; 2] {
        self.dimensions
    }
    pub fn plot(&self) -> crate::ClipRect {
        self.plot
    }
}

/// Polygon map and its already-fitted projection. Keep this context when
/// adding symbols so a Data fit cannot drift between the two layers.
#[derive(Clone, Debug, PartialEq)]
pub struct GeoMapLayout {
    pub scene: Scene,
    pub projection: GeoProjectionContext,
}
impl Default for GeoMapOptions {
    fn default() -> Self {
        Self {
            projection: GeoProjection::Equirectangular,
            clip_angle: None,
            center: [0., 0.],
            rotation: 0.,
            fit: GeoFit::World,
            padding: 0.,
            scale: None,
            translate: None,
            fill: None,
            border: Some(Color(255, 255, 255, 255)),
            border_width: 0.75,
        }
    }
}
fn valid_position(p: [f64; 2]) -> Result<()> {
    ensure(
        p.iter().all(|v| v.is_finite())
            && (-180. ..=180.).contains(&p[0])
            && (-90. ..=90.).contains(&p[1]),
        "invalid geographic longitude/latitude",
    )
}
fn options(o: &GeoMapOptions) -> Result<()> {
    o.projection.validate_conic()?;
    valid_position(o.center)?;
    ensure(
        o.clip_angle.is_none_or(|v| {
            o.projection.hemisphere() && v.is_finite() && (0.01..=90.).contains(&v)
        }),
        "geographic clip_angle requires an azimuthal projection and 0.01..=90 degrees",
    )?;
    ensure(
        o.projection != GeoProjection::Gnomonic || clip_angle(o) < FRAC_PI_2,
        "gnomonic clip_angle must be less than 90 degrees",
    )?;
    ensure(
        o.rotation.is_finite()
            && o.rotation.abs() <= 360_000.
            && o.padding.is_finite()
            && o.padding >= 0.
            && o.border_width.is_finite()
            && (0. ..=10000.).contains(&o.border_width),
        "invalid geographic view options",
    )?;
    ensure(
        o.scale
            .is_none_or(|v| v.is_finite() && v > 0. && v <= 1_000_000.),
        "invalid geographic scale",
    )?;
    ensure(
        o.translate
            .is_none_or(|p| p.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.)),
        "invalid geographic translation",
    )
}
fn clip_angle(o: &GeoMapOptions) -> f64 {
    o.clip_angle
        .unwrap_or(if o.projection == GeoProjection::Gnomonic {
            60.
        } else {
            90.
        })
        .to_radians()
}
#[derive(Clone, Copy, Debug)]
struct V {
    x: f64,
    y: f64,
    z: f64,
}
impl V {
    fn dot(self, b: Self) -> f64 {
        self.x * b.x + self.y * b.y + self.z * b.z
    }
    fn cross(self, b: Self) -> Self {
        Self {
            x: self.y * b.z - self.z * b.y,
            y: self.z * b.x - self.x * b.z,
            z: self.x * b.y - self.y * b.x,
        }
    }
    fn norm(self) -> f64 {
        self.x.hypot(self.y).hypot(self.z)
    }
    fn normalized(self) -> Result<Self> {
        let n = self.norm();
        ensure(n > EPS && n.is_finite(), "ambiguous spherical direction")?;
        Ok(Self {
            x: self.x / n,
            y: self.y / n,
            z: self.z / n,
        })
    }
}
fn rotate(p: [f64; 2], o: &GeoMapOptions) -> V {
    let l = (p[0] - o.center[0]).to_radians();
    let f = p[1].to_radians();
    let center = o.center[1].to_radians();
    let roll = o.rotation.to_radians();
    let x = f.cos() * l.cos() * center.cos() + f.sin() * center.sin();
    let east = f.cos() * l.sin();
    let north = f.sin() * center.cos() - f.cos() * l.cos() * center.sin();
    let v = V {
        x,
        y: east * roll.cos() + north * roll.sin(),
        z: -east * roll.sin() + north * roll.cos(),
    };
    if o.projection == GeoProjection::TransverseMercator {
        // Compose the 90-degree frame with roll before trig, as D3 does.
        // Its finite cos(pi/2) also fixes the selected side of exact seams.
        let gamma = FRAC_PI_2 - roll;
        V {
            x,
            y: east * gamma.cos() - north * gamma.sin(),
            z: north * gamma.cos() + east * gamma.sin(),
        }
    } else if o.projection == GeoProjection::Albers {
        let a = 96_f64.to_radians();
        V {
            x: v.x * a.cos() - v.y * a.sin(),
            y: v.x * a.sin() + v.y * a.cos(),
            z: v.z,
        }
    } else {
        v
    }
}
fn spherical(v: V) -> [f64; 2] {
    [v.y.atan2(v.x), v.z.atan2(v.x.hypot(v.y))]
}
fn from_radians(p: [f64; 2]) -> V {
    V {
        x: p[1].cos() * p[0].cos(),
        y: p[1].cos() * p[0].sin(),
        z: p[1].sin(),
    }
}
fn raw(p: [f64; 2], projection: GeoProjection) -> [f64; 2] {
    let [l, f] = p;
    match projection {
        GeoProjection::ConicEqualArea(_)
        | GeoProjection::ConicEquidistant(_)
        | GeoProjection::ConicConformal(_)
        | GeoProjection::Albers => projection.conic_project(p),
        GeoProjection::Equirectangular => [l, f],
        GeoProjection::Mercator => [l, (PI / 4. + f / 2.).tan().ln()],
        GeoProjection::TransverseMercator => [f.sin().atanh(), -l],
        GeoProjection::EqualEarth => {
            let m = 3_f64.sqrt() / 2.;
            let t = (m * f.sin()).asin();
            let t2 = t * t;
            let t6 = t2 * t2 * t2;
            [
                l * t.cos()
                    / (m * (1.340264 - 3. * 0.081106 * t2
                        + t6 * (7. * 0.000893 + 9. * 0.003796 * t2))),
                t * (1.340264 - 0.081106 * t2 + t6 * (0.000893 + 0.003796 * t2)),
            ]
        }
        GeoProjection::NaturalEarth => {
            let f2 = f * f;
            let f4 = f2 * f2;
            [
                l * (0.8707 - 0.131979 * f2
                    + f4 * (-0.013791 + f4 * (0.003971 * f2 - 0.001529 * f4))),
                f * (1.007226 + f2 * (0.015085 + f4 * (-0.044475 + 0.028874 * f2 - 0.005916 * f4))),
            ]
        }
        GeoProjection::Orthographic => [f.cos() * l.sin(), f.sin()],
        GeoProjection::AzimuthalEqualArea
        | GeoProjection::AzimuthalEquidistant
        | GeoProjection::Stereographic
        | GeoProjection::Gnomonic => {
            // d3-geo/src/projection/{azimuthalEqualArea,azimuthalEquidistant,
            // stereographic,gnomonic}.js. Center/roll already rotate the sphere.
            let cos_c = (f.cos() * l.cos()).clamp(-1., 1.);
            let k = match projection {
                GeoProjection::AzimuthalEqualArea => (2. / (1. + cos_c)).sqrt(),
                GeoProjection::AzimuthalEquidistant => {
                    let angle = cos_c.acos();
                    if angle < 1e-10 {
                        1.
                    } else {
                        angle / angle.sin()
                    }
                }
                GeoProjection::Gnomonic => 1. / cos_c,
                _ => 1. / (1. + cos_c),
            };
            [k * f.cos() * l.sin(), k * f.sin()]
        }
    }
}
fn raw_vector(v: V, projection: GeoProjection) -> [f64; 2] {
    if projection == GeoProjection::TransverseMercator {
        return [v.z.atanh(), -v.y.atan2(v.x)];
    }
    // Keep Cartesian X: round-tripping longitude/latitude loses significant
    // relative precision near the gnomonic 90-degree singularity.
    let k = match projection {
        GeoProjection::Orthographic => 1.,
        GeoProjection::AzimuthalEqualArea => (2. / (1. + v.x)).sqrt(),
        GeoProjection::AzimuthalEquidistant => {
            let radius = v.y.hypot(v.z);
            if radius < 1e-14 {
                1.
            } else {
                radius.atan2(v.x) / radius
            }
        }
        GeoProjection::Stereographic => 1. / (1. + v.x),
        GeoProjection::Gnomonic => 1. / v.x,
        _ => return raw(spherical(v), projection),
    };
    [k * v.y, k * v.z]
}
/// Project one point into raw units after spherical center/roll. Hidden
/// hemisphere-projection points and points beyond Mercator's ±85.05113° cap
/// return None. Use the map functions for automatic fitting to a ChartConfig.
pub fn project_geo_point(point: [f64; 2], o: &GeoMapOptions) -> Result<Option<[f64; 2]>> {
    options(o)?;
    valid_position(point)?;
    let v = rotate(point, o);
    let p = spherical(v);
    if o.projection.hemisphere() {
        let angle = clip_angle(o);
        let tolerance = EPS * angle.sin().max(1e-4);
        let tolerance = if o.projection == GeoProjection::Gnomonic {
            tolerance.min(angle.cos() * 1e-6)
        } else {
            tolerance
        };
        if v.x < angle.cos() - tolerance {
            return Ok(None);
        }
    }
    if o.projection.latitude_bounded() && p[1].abs() > MERCATOR_LIMIT + EPS {
        return Ok(None);
    }
    let q = raw_vector(v, o.projection);
    if o.projection.conic_kernel().is_some() {
        ensure(
            q.iter().all(|v| v.is_finite()),
            "conic forward arithmetic is not representable",
        )?;
    }
    Ok(Some(q))
}
fn interpolate_arc(a: V, b: V, t: f64) -> Result<V> {
    let theta = a.cross(b).norm().atan2(a.dot(b).clamp(-1., 1.));
    ensure(theta < PI - 1e-8, "antipodal geographic edge is ambiguous")?;
    if theta == 0. {
        return Ok(a);
    }
    if theta < 1e-6 {
        let v = V {
            x: a.x * (1. - t) + b.x * t,
            y: a.y * (1. - t) + b.y * t,
            z: a.z * (1. - t) + b.z * t,
        };
        let n = v.norm();
        return Ok(V {
            x: v.x / n,
            y: v.y / n,
            z: v.z / n,
        });
    }
    let u = ((1. - t) * theta).sin() / theta.sin();
    let v = (t * theta).sin() / theta.sin();
    Ok(V {
        x: a.x * u + b.x * v,
        y: a.y * u + b.y * v,
        z: a.z * u + b.z * v,
    })
}
fn visible_vector(v: V, o: &GeoMapOptions) -> bool {
    if o.projection.hemisphere() {
        let angle = clip_angle(o);
        let tolerance = if o.projection == GeoProjection::Gnomonic {
            (EPS * angle.sin().max(1e-4)).min(angle.cos() * 1e-6)
        } else {
            EPS * angle.sin().max(1e-4)
        };
        v.x >= angle.cos() - tolerance
    } else if o.projection.latitude_bounded() {
        spherical(v)[1].abs() <= MERCATOR_LIMIT + EPS
    } else {
        true
    }
}
fn slerp(a: V, b: V, t: f64) -> Result<V> {
    let angle = a.dot(b).clamp(-1., 1.).acos();
    ensure(angle < PI - 1e-8, "antipodal geographic edge is ambiguous")?;
    if angle < EPS {
        return Ok(a);
    }
    let s = angle.sin();
    Ok(V {
        x: (a.x * ((1. - t) * angle).sin() + b.x * (t * angle).sin()) / s,
        y: (a.y * ((1. - t) * angle).sin() + b.y * (t * angle).sin()) / s,
        z: (a.z * ((1. - t) * angle).sin() + b.z * (t * angle).sin()) / s,
    })
}
fn resample(a: V, b: V, out: &mut Vec<V>) -> Result<()> {
    let angle = a.dot(b).clamp(-1., 1.).acos();
    ensure(angle < PI - 1e-8, "antipodal geographic edge is ambiguous")?;
    let count = (angle / PRECISION).ceil().max(1.) as usize;
    let mut fractions = (1..=count)
        .map(|i| i as f64 / count as f64)
        .collect::<Vec<_>>();
    if angle > EPS {
        let basis = (b.z - a.z * angle.cos()) / angle.sin();
        let mut extremum = basis.atan2(a.z);
        if extremum < 0. {
            extremum += PI;
        }
        if extremum > EPS && extremum < angle - EPS {
            fractions.push(extremum / angle);
            fractions.sort_by(f64::total_cmp);
        }
    }
    ensure(
        out.len() + fractions.len() <= MAX_POINTS,
        "geographic resampling budget exceeded",
    )?;
    for t in fractions {
        out.push(slerp(a, b, t)?);
    }
    Ok(())
}
fn vectors(ring: &[[f64; 2]], o: &GeoMapOptions) -> Result<Vec<V>> {
    ensure(
        (3..=4096).contains(&ring.len()),
        "geographic rings require 3..=4096 points",
    )?;
    for p in ring {
        valid_position(*p)?;
    }
    let mut points = ring.iter().map(|p| rotate(*p, o)).collect::<Vec<_>>();
    if points
        .first()
        .zip(points.last())
        .is_some_and(|(a, b)| a.dot(*b) > 1. - 1e-14)
    {
        points.pop();
    }
    ensure(
        points.len() >= 3,
        "geographic ring has too few distinct points",
    )?;
    for i in 0..points.len() {
        ensure(
            points[i].dot(points[(i + 1) % points.len()]) > -1. + 1e-10,
            "antipodal geographic edge is ambiguous",
        )?;
    }
    Ok(points)
}
fn sampled(points: &[V]) -> Result<Vec<V>> {
    let mut out = vec![points[0]];
    for i in 0..points.len() {
        resample(points[i], points[(i + 1) % points.len()], &mut out)?;
    }
    Ok(out)
}
fn unwrap_ring(points: &[V]) -> Result<Ring> {
    let samples = sampled(points)?;
    let mut out = vec![];
    let mut previous = 0.;
    for (i, v) in samples.into_iter().enumerate() {
        let mut p = spherical(v);
        if i > 0 {
            while p[0] - previous > PI {
                p[0] -= TAU;
            }
            while p[0] - previous < -PI {
                p[0] += TAU;
            }
        }
        previous = p[0];
        out.push(p);
    }
    ensure(
        (out.last().unwrap()[0] - out[0][0]).abs() < 1e-6,
        "pole-enclosing geographic rings require spherical cap topology",
    )?;
    out.pop();
    let min = out.iter().map(|p| p[0]).fold(f64::INFINITY, f64::min);
    let max = out.iter().map(|p| p[0]).fold(f64::NEG_INFINITY, f64::max);
    ensure(
        max - min < TAU - EPS,
        "geographic ring spans multiple longitude turns",
    )?;
    Ok(out)
}
fn winding(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            ring[i][0] * ring[(i + 1) % ring.len()][1] - ring[(i + 1) % ring.len()][0] * ring[i][1]
        })
        .sum::<f64>()
        / 2.
}
fn inside(p: [f64; 2], ring: &[[f64; 2]]) -> bool {
    let mut hit = false;
    for i in 0..ring.len() {
        let a = ring[i];
        let b = ring[(i + 1) % ring.len()];
        let cross = (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        if cross.abs() < EPS
            && p[0] >= a[0].min(b[0]) - EPS
            && p[0] <= a[0].max(b[0]) + EPS
            && p[1] >= a[1].min(b[1]) - EPS
            && p[1] <= a[1].max(b[1]) + EPS
        {
            return true;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            hit = !hit;
        }
    }
    hit
}
fn seam_intersection(a: [f64; 2], b: [f64; 2], boundary: f64) -> Result<[f64; 2]> {
    let n = from_radians(a).cross(from_radians(b));
    let mut p = V {
        x: -n.z,
        y: 0.,
        z: n.x,
    }
    .normalized()?;
    if p.x > 0. {
        p = V {
            x: -p.x,
            y: 0.,
            z: -p.z,
        };
    }
    Ok([boundary, p.z.atan2(p.x.abs())])
}
fn latitude_intersection(a: [f64; 2], b: [f64; 2], boundary: f64) -> Result<[f64; 2]> {
    let av = from_radians(a);
    let bv = from_radians(b);
    let mut lo = 0.;
    let mut hi = 1.;
    let increasing = b[1] > a[1];
    for _ in 0..45 {
        let t = (lo + hi) / 2.;
        let p = spherical(slerp(av, bv, t)?);
        if (p[1] < boundary) == increasing {
            lo = t;
        } else {
            hi = t;
        }
    }
    let mut p = spherical(slerp(av, bv, (lo + hi) / 2.)?);
    let target = (a[0] + b[0]) / 2.;
    while p[0] - target > PI {
        p[0] -= TAU;
    }
    while p[0] - target < -PI {
        p[0] += TAU;
    }
    p[1] = boundary;
    Ok(p)
}
fn clip(ring: &[[f64; 2]], axis: usize, boundary: f64, greater: bool) -> Result<Ring> {
    if ring.is_empty() {
        return Ok(vec![]);
    }
    let mut out = vec![];
    let mut previous = *ring.last().unwrap();
    let mut pi = if greater {
        previous[axis] >= boundary - EPS
    } else {
        previous[axis] <= boundary + EPS
    };
    for &current in ring {
        let ci = if greater {
            current[axis] >= boundary - EPS
        } else {
            current[axis] <= boundary + EPS
        };
        if ci != pi {
            out.push(if axis == 0 {
                seam_intersection(previous, current, boundary)?
            } else {
                latitude_intersection(previous, current, boundary)?
            });
        }
        if ci {
            out.push(current);
        }
        previous = current;
        pi = ci;
    }
    Ok(out)
}
fn planar_polygon(
    rings: &[Vec<V>],
    projection: GeoProjection,
    mut adaptive: Option<&mut path::Sampler<'_>>,
    fit_bounds: &mut [f64; 4],
) -> Result<Vec<Vec<Ring>>> {
    let mut unwrapped = vec![];
    let mut samples = 0usize;
    for ring in rings {
        let ring = unwrap_ring(ring)?;
        samples += ring.len();
        ensure(
            samples <= 180_000,
            "geographic polygon sampling budget exceeded",
        )?;
        unwrapped.push(ring);
    }
    let mut containment_work = 0usize;
    let center = unwrapped[0].iter().map(|p| p[0]).sum::<f64>() / unwrapped[0].len() as f64;
    for i in 1..unwrapped.len() {
        let mean = unwrapped[i].iter().map(|p| p[0]).sum::<f64>() / unwrapped[i].len() as f64;
        let shift = ((center - mean) / TAU).round() * TAU;
        for p in &mut unwrapped[i] {
            p[0] += shift;
        }
        containment_work =
            containment_work.saturating_add(unwrapped[i].len().saturating_mul(unwrapped[0].len()));
        ensure(
            containment_work <= 20_000_000,
            "geographic hole containment budget exceeded",
        )?;
        ensure(
            unwrapped[i].iter().all(|p| inside(*p, &unwrapped[0])),
            "geographic hole lies outside its exterior",
        )?;
    }
    let min = unwrapped[0]
        .iter()
        .map(|p| p[0])
        .fold(f64::INFINITY, f64::min);
    let max = unwrapped[0]
        .iter()
        .map(|p| p[0])
        .fold(f64::NEG_INFINITY, f64::max);
    let first = ((min + PI) / TAU).floor() as i32;
    let last = ((max + PI) / TAU).floor() as i32;
    ensure(last - first <= 2, "too many antimeridian windows")?;
    if projection.conic_kernel().is_some() {
        return conic_seam::polygon(unwrapped, projection, adaptive, fit_bounds);
    }
    let mut pieces = vec![];
    for window in first..=last {
        let shift = window as f64 * TAU;
        let mut clipped_rings = vec![];
        for (index, ring) in unwrapped.iter().enumerate() {
            let mut ring = clip(&clip(ring, 0, -PI + shift, true)?, 0, PI + shift, false)?;
            if projection.latitude_bounded() {
                ring = clip(
                    &clip(&ring, 1, -MERCATOR_LIMIT, true)?,
                    1,
                    MERCATOR_LIMIT,
                    false,
                )?;
            }
            if ring.len() < 3 || winding(&ring).abs() < 1e-12 {
                if index == 0 {
                    break;
                }
                continue;
            }
            for p in &mut ring {
                p[0] -= shift;
            }
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                let parallel = projection.latitude_bounded()
                    && (a[1] - b[1]).abs() < EPS
                    && (a[1].abs() - MERCATOR_LIMIT).abs() < EPS;
                if let Some(bounds) = adaptive
                    .is_none()
                    .then(|| projection.conic_arc_bounds(a, b, parallel))
                    .flatten()
                {
                    ensure(
                        bounds.iter().all(|v| v.is_finite()),
                        "conic edge fit bounds are not representable",
                    )?;
                    merge_geo_bounds(fit_bounds, bounds);
                }
            }
            if let Some(sampler) = adaptive.as_deref_mut() {
                let mut baseline = vec![];
                for i in 0..ring.len() {
                    let a = ring[i];
                    let b = ring[(i + 1) % ring.len()];
                    baseline.push(a);
                    if (a[0].abs() - PI).abs() < EPS && (b[0] - a[0]).abs() < EPS {
                        let n = ((b[1] - a[1]).abs() / PRECISION).ceil().max(1.) as usize;
                        for j in 1..n {
                            baseline.push([a[0], a[1] + (b[1] - a[1]) * j as f64 / n as f64]);
                        }
                    }
                }
                clipped_rings.push(sampler.planar_ring(&baseline)?);
                continue;
            }
            let mut projected = vec![];
            for i in 0..ring.len() {
                let a = ring[i];
                let b = ring[(i + 1) % ring.len()];
                projected.push(raw(a, projection));
                if (a[0].abs() - PI).abs() < EPS && (b[0] - a[0]).abs() < EPS {
                    let steps = ((b[1] - a[1]).abs() / PRECISION).ceil().max(1.) as usize;
                    for j in 1..steps {
                        projected.push(raw(
                            [a[0], a[1] + (b[1] - a[1]) * j as f64 / steps as f64],
                            projection,
                        ));
                    }
                }
            }
            clipped_rings.push(projected);
        }
        if !clipped_rings.is_empty() {
            pieces.push(clipped_rings);
        }
    }
    Ok(pieces)
}
fn hemisphere_center(ring: &[V]) -> Result<V> {
    let center = ring
        .iter()
        .fold(
            V {
                x: 0.,
                y: 0.,
                z: 0.,
            },
            |a, b| V {
                x: a.x + b.x,
                y: a.y + b.y,
                z: a.z + b.z,
            },
        )
        .normalized()?;
    ensure(
        ring.iter().all(|p| center.dot(*p) > EPS),
        "orthographic polygons must fit an open hemisphere",
    )?;
    Ok(center)
}
fn merge_geo_bounds(a: &mut [f64; 4], b: [f64; 4]) {
    if a.iter().any(|v| v.is_nan()) || b.iter().any(|v| !v.is_finite()) {
        *a = [f64::NAN; 4];
        return;
    }
    *a = [
        a[0].min(b[0]),
        a[1].min(b[1]),
        a[2].max(b[2]),
        a[3].max(b[3]),
    ];
}
fn world(o: &GeoMapOptions) -> [f64; 4] {
    let projection = o.projection;
    if let Some(bounds) = projection.conic_world() {
        return bounds;
    }
    if projection.hemisphere() {
        let r = projection.disk_radius(clip_angle(o));
        return [-r, -r, r, r];
    }
    match projection {
        GeoProjection::Orthographic => [-1., -1., 1., 1.],
        GeoProjection::Mercator | GeoProjection::TransverseMercator => [-PI, -PI, PI, PI],
        GeoProjection::Equirectangular => [-PI, -FRAC_PI_2, PI, FRAC_PI_2],
        _ => {
            let x = raw([PI, 0.], projection)[0];
            let y = raw([0., FRAC_PI_2], projection)[1];
            [-x, -y, x, y]
        }
    }
}
fn fitted_context(
    c: &ChartConfig,
    o: &GeoMapOptions,
    mut bounds: [f64; 4],
    path_options: Option<&GeoPathOptions>,
) -> Result<GeoProjectionContext> {
    if o.fit == GeoFit::World || bounds[0] == f64::INFINITY {
        bounds = world(o);
    }
    if o.projection.conic_kernel().is_some() {
        ensure(
            bounds.iter().all(|v| v.is_finite()),
            "conic finite fit bounds are not representable",
        )?;
    }
    let dx = bounds[2] - bounds[0];
    let dy = bounds[3] - bounds[1];
    let scale = o.scale.unwrap_or_else(|| {
        if path_options.is_some() {
            let x = if dx > 0. {
                (c.inner_width() - 2. * o.padding) / dx
            } else {
                f64::INFINITY
            };
            let y = if dy > 0. {
                (c.inner_height() - 2. * o.padding) / dy
            } else {
                f64::INFINITY
            };
            x.min(y)
        } else {
            ((c.inner_width() - 2. * o.padding) / dx.max(EPS))
                .min((c.inner_height() - 2. * o.padding) / dy.max(EPS))
        }
    });
    ensure(
        scale.is_finite() && scale > 0.,
        "geographic fitted scale is not representable",
    )?;
    let center = [(bounds[0] + bounds[2]) / 2., (bounds[1] + bounds[3]) / 2.];
    let translate = o.translate.unwrap_or([
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ]);
    Ok(GeoProjectionContext {
        view: o.clone(),
        center,
        scale,
        translate,
        dimensions: [c.width, c.height],
        plot: crate::ClipRect::plot(c),
        path_options: path_options.cloned(),
    })
}
fn render(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
    choropleth: bool,
) -> Result<GeoMapLayout> {
    render_adaptive(c, features, o, choropleth, None)
}
fn render_adaptive(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
    choropleth: bool,
    adaptive: Option<(&GeoPathOptions, f64)>,
) -> Result<GeoMapLayout> {
    let mut scene = c.scene()?;
    options(o)?;
    ensure(
        features.len() <= 4096 && o.padding * 2. < c.inner_width().min(c.inner_height()),
        "geographic feature/padding budget exceeded",
    )?;
    let mut ids = BTreeSet::new();
    let mut count = 0usize;
    let mut generated = 0usize;
    let mut hemisphere_clipper = hemisphere::Clipper::new(clip_angle(o));
    if adaptive.is_some() {
        hemisphere_clipper.capture_boundary_edges();
    }
    let mut sampler = adaptive
        .map(|(p, s)| path::Sampler::new(p, s, o.projection))
        .transpose()?;
    if let Some(sampler) = &mut sampler {
        sampler.capture_fit = o.projection.conic_kernel().is_some();
    }
    let mut data = vec![];
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut values = [f64::INFINITY, f64::NEG_INFINITY];
    for feature in features {
        ensure(
            !feature.id.trim().is_empty()
                && feature.id.len() <= 1000
                && feature.label.len() <= 1024
                && ids.insert(&feature.id)
                && feature.value.is_none_or(f64::is_finite),
            "invalid geographic feature identity/value",
        )?;
        if let Some(v) = feature.value {
            values[0] = values[0].min(v);
            values[1] = values[1].max(v);
        }
        let mut projected = vec![];
        for polygon in &feature.polygons {
            ensure(
                !polygon.is_empty(),
                "geographic polygon needs an exterior ring",
            )?;
            for ring in polygon {
                count = count
                    .checked_add(ring.len())
                    .ok_or_else(|| ChartError("geographic point count overflow".into()))?;
                ensure(
                    count <= MAX_POINTS,
                    "geographic source point budget exceeded",
                )?;
            }
            let rings = polygon
                .iter()
                .map(|r| vectors(r, o))
                .collect::<Result<Vec<_>>>()?;
            let pieces = if o.projection.hemisphere() {
                // Preserve all three spherical coordinates through clipping;
                // reconstructing X from the orthographic disk loses precision
                // near a gnomonic singularity even when clip_angle is <90°.
                hemisphere_clipper
                    .polygon(&rings)?
                    .into_iter()
                    .map(|piece| {
                        piece
                            .into_iter()
                            .map(|ring| {
                                if let Some(sampler) = &mut sampler {
                                    return sampler.cap_ring(&ring, clip_angle(o), &|a, b| {
                                        hemisphere_clipper.is_circle_edge(a, b)
                                    });
                                }
                                Ok(ring
                                    .into_iter()
                                    .map(|v| raw_vector(v, o.projection))
                                    .collect::<Ring>())
                            })
                            .collect::<Result<Vec<_>>>()
                    })
                    .collect::<Result<Vec<_>>>()?
            } else {
                planar_polygon(&rings, o.projection, sampler.as_mut(), &mut bounds)?
            };
            for piece in pieces {
                for ring in &piece {
                    generated += ring.len();
                    ensure(
                        generated <= 180_000,
                        "geographic generated point budget exceeded",
                    )?;
                    for p in ring {
                        bounds[0] = bounds[0].min(p[0]);
                        bounds[1] = bounds[1].min(p[1]);
                        bounds[2] = bounds[2].max(p[0]);
                        bounds[3] = bounds[3].max(p[1]);
                    }
                }
                projected.push(piece);
            }
        }
        data.push(projected);
    }
    ensure(
        values[0] == f64::INFINITY || (values[1] - values[0]).is_finite(),
        "geographic color domain span overflow",
    )?;
    if let Some(sampler) = &sampler
        && sampler.capture_fit
    {
        merge_geo_bounds(&mut bounds, sampler.fit_bounds);
    }
    let projection = fitted_context(c, o, bounds, adaptive.map(|(o, _)| o))?;
    for (feature, polygons) in features.iter().zip(data) {
        let mut commands = vec![];
        for polygon in polygons {
            for (index, mut ring) in polygon.into_iter().enumerate() {
                for p in &mut ring {
                    *p = projection.map_raw(*p);
                }
                if o.projection.conic_kernel().is_none()
                    && ((index == 0 && winding(&ring) < 0.) || (index > 0 && winding(&ring) > 0.))
                {
                    ring.reverse();
                }
                if let Some(first) = ring.first() {
                    commands.push(Cmd::Move(P::new(first[0], first[1])));
                    commands.extend(ring.iter().skip(1).map(|p| Cmd::Line(P::new(p[0], p[1]))));
                    commands.push(Cmd::Close);
                }
            }
        }
        if commands.is_empty() {
            continue;
        }
        let fill = if choropleth {
            feature
                .value
                .map(|v| {
                    let t = if values[0] == values[1] {
                        0.5
                    } else {
                        (v - values[0]) / (values[1] - values[0])
                    };
                    Color(
                        (230. - 190. * t) as u8,
                        (240. - 120. * t) as u8,
                        (250. - 60. * t) as u8,
                        255,
                    )
                })
                .or(o.fill)
                .unwrap_or(c.theme.grid)
        } else {
            o.fill.unwrap_or(c.theme.grid)
        };
        let mut mark = Mark::new(
            format!("map:{}", feature.id),
            Geometry::Path(commands),
            fill,
        );
        mark.series = feature.id.clone();
        mark.label = feature.label.clone();
        mark.value = feature.value.unwrap_or(0.);
        if let Some(color) = o.border
            && o.border_width > 0.
        {
            mark.stroke = Some(GraphicStroke {
                color,
                width: o.border_width,
                cap: StrokeCap::Butt,
            });
        }
        scene.marks.push(mark);
    }
    scene.validate()?;
    Ok(GeoMapLayout { scene, projection })
}
pub fn geographic_map(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
) -> Result<Scene> {
    Ok(geographic_map_layout(c, features, o)?.scene)
}
pub fn choropleth_with_options(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
) -> Result<Scene> {
    Ok(choropleth_layout(c, features, o)?.scene)
}

pub fn geographic_map_layout(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
) -> Result<GeoMapLayout> {
    render(c, features, o, false)
}
pub fn choropleth_layout(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
) -> Result<GeoMapLayout> {
    render(c, features, o, true)
}
fn conic_sampling_scale(c: &ChartConfig, o: &GeoMapOptions, baseline: &GeoMapLayout) -> f64 {
    if o.projection.conic_kernel().is_none() || o.scale.is_some() {
        return baseline.projection.scale;
    }
    let mut bounds = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for mark in &baseline.scene.marks {
        if mark.id.starts_with("map:")
            && let Geometry::Path(path) = &mark.geometry
        {
            for cmd in path {
                if let Cmd::Move(p) | Cmd::Line(p) = cmd {
                    merge_geo_bounds(&mut bounds, [p.x, p.y, p.x, p.y]);
                }
            }
        }
    }
    let dx = (bounds[2] - bounds[0]) / baseline.projection.scale;
    let dy = (bounds[3] - bounds[1]) / baseline.projection.scale;
    let upper =
        ((c.inner_width() - 2. * o.padding) / dx).min((c.inner_height() - 2. * o.padding) / dy);
    if upper.is_finite() && upper > 0. {
        (upper * (1. + 64. * f64::EPSILON)).max(baseline.projection.scale)
    } else {
        baseline.projection.scale
    }
}
/// Opt-in projected-pixel refinement. Existing projections retain their
/// legacy conservative fit. Conic projections fit refined leaf envelopes,
/// using a sampling-scale upper bound so a tighter final fit preserves the
/// measured chord tolerance.
pub fn geographic_map_layout_with_path_options(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
    path: &GeoPathOptions,
) -> Result<GeoMapLayout> {
    path.validate()?;
    let baseline = render(c, features, o, false)?;
    render_adaptive(
        c,
        features,
        o,
        false,
        Some((path, conic_sampling_scale(c, o, &baseline))),
    )
}
pub fn choropleth_layout_with_path_options(
    c: &ChartConfig,
    features: &[GeoFeature],
    o: &GeoMapOptions,
    path: &GeoPathOptions,
) -> Result<GeoMapLayout> {
    path.validate()?;
    let baseline = render(c, features, o, true)?;
    render_adaptive(
        c,
        features,
        o,
        true,
        Some((path, conic_sampling_scale(c, o, &baseline))),
    )
}
