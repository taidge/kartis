//! Bounded projected-chord refinement. Primary D3 3.1.1 reference:
//! projection/resample.js (ISC, Copyright 2010-2024 Mike Bostock).
//! Unlike D3's depth-limited best effort, an unresolved leaf rejects here.
use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct GeoPathOptions {
    /// Logical-pixel deviation of projected quarter/mid/three-quarter samples
    /// from each accepted chord. This is a sampled adaptive error criterion,
    /// not a proof of Hausdorff error for arbitrary user projection functions.
    pub precision: f64,
    pub max_depth: u8,
    pub max_points: usize,
    /// Adaptive interior projection evaluations; coarse/source and topology
    /// budgets remain separately bounded.
    pub max_work: usize,
}
impl Default for GeoPathOptions {
    fn default() -> Self {
        Self {
            precision: 0.5,
            max_depth: 20,
            max_points: 180_000,
            max_work: 4_000_000,
        }
    }
}
impl GeoPathOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.precision.is_finite()
                && (0.0001..=64.).contains(&self.precision)
                && (1..=24).contains(&self.max_depth)
                && (2..=180_000).contains(&self.max_points)
                && (1..=4_000_000).contains(&self.max_work),
            "invalid adaptive geographic precision or budget",
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GeoPathStats {
    pub points: usize,
    pub work: usize,
    pub deepest: u8,
}
type GeographicCurve<'a> = (&'a dyn Fn(f64) -> Result<[f64; 2]>, bool);
pub(super) struct Sampler<'a> {
    pub options: &'a GeoPathOptions,
    pub scale: f64,
    pub projection: GeoProjection,
    pub stats: GeoPathStats,
    pub capture_fit: bool,
    pub fit_bounds: [f64; 4],
}
impl<'a> Sampler<'a> {
    pub fn new(options: &'a GeoPathOptions, scale: f64, projection: GeoProjection) -> Result<Self> {
        options.validate()?;
        ensure(
            scale.is_finite() && scale > 0.,
            "invalid geographic sampling scale",
        )?;
        Ok(Self {
            options,
            scale,
            projection,
            stats: Default::default(),
            capture_fit: false,
            fit_bounds: [
                f64::INFINITY,
                f64::INFINITY,
                f64::NEG_INFINITY,
                f64::NEG_INFINITY,
            ],
        })
    }
    fn point(&mut self, out: &mut Ring, p: [f64; 2]) -> Result<()> {
        self.stats.points += 1;
        ensure(
            self.stats.points <= self.options.max_points,
            "adaptive geographic point budget exceeded",
        )?;
        out.push(p);
        Ok(())
    }
    pub fn start(&mut self, out: &mut Ring, p: [f64; 2]) -> Result<()> {
        self.point(out, p)
    }
    pub fn segment(
        &mut self,
        a: [f64; 2],
        b: [f64; 2],
        curve: &impl Fn(f64) -> Result<[f64; 2]>,
        angular_span: f64,
        out: &mut Ring,
    ) -> Result<()> {
        self.recursive([a, b], curve, [0., 1.], (angular_span, 0), out, None)
    }
    pub fn segment_geo(
        &mut self,
        a: [f64; 2],
        b: [f64; 2],
        geo: &dyn Fn(f64) -> Result<[f64; 2]>,
        parallel: bool,
        span: f64,
        out: &mut Ring,
    ) -> Result<()> {
        let projection = self.projection;
        let curve = |t| Ok(raw(geo(t)?, projection));
        let capture = self.capture_fit.then_some((geo, parallel));
        self.recursive([a, b], &curve, [0., 1.], (span, 0), out, capture)
    }
    fn recursive(
        &mut self,
        endpoints: [[f64; 2]; 2],
        curve: &impl Fn(f64) -> Result<[f64; 2]>,
        range: [f64; 2],
        span_depth: (f64, u8),
        out: &mut Ring,
        geo: Option<GeographicCurve<'_>>,
    ) -> Result<()> {
        let [a, b] = endpoints;
        let (span, depth) = span_depth;
        ensure(
            self.stats.work + 3 <= self.options.max_work,
            "adaptive geographic work budget exceeded",
        )?;
        self.stats.work += 3;
        self.stats.deepest = self.stats.deepest.max(depth);
        let t = |f: f64| range[0] + (range[1] - range[0]) * f;
        let samples = [curve(t(0.25))?, curve(t(0.5))?, curve(t(0.75))?];
        ensure(
            a.iter()
                .chain(b.iter())
                .chain(samples.iter().flatten())
                .all(|v| v.is_finite()),
            "nonfinite projected geographic curve",
        )?;
        let error = samples
            .iter()
            .map(|p| distance_segment(*p, a, b) * self.scale)
            .fold(0., f64::max);
        let concentrated = if self.projection.conic_kernel().is_some() {
            let dx = b[0] - a[0];
            let dy = b[1] - a[1];
            let chord2 = dx * dx + dy * dy;
            if chord2 > 0. && chord2.sqrt() * self.scale > 2. * self.options.precision {
                let progress = ((samples[1][0] - a[0]) * dx + (samples[1][1] - a[1]) * dy) / chord2;
                ensure(
                    progress.is_finite(),
                    "conic midpoint progress arithmetic is not representable",
                )?;
                // D3 resample.js also detects midpoint progress near an end.
                // This catches narrow polar turns missed by perpendicular
                // quarter samples without changing any projection samples.
                (progress - 0.5).abs() > 0.3
            } else {
                false
            }
        } else {
            false
        };
        if error <= self.options.precision && span <= PI / 6. && !concentrated {
            if let Some((geo, parallel)) = geo {
                // Four subintervals use the same quarter positions already
                // checked above. Each rectangle costs at most fourteen raw
                // evaluations; charge all fifty-six before doing any work.
                ensure(
                    self.stats.work + 56 <= self.options.max_work,
                    "conic fit envelope work budget exceeded",
                )?;
                self.stats.work += 56;
                let mut previous = geo(range[0])?;
                for fraction in [0.25, 0.5, 0.75, 1.] {
                    let point = geo(t(fraction))?;
                    let bounds = self
                        .projection
                        .conic_arc_bounds(previous, point, parallel)
                        .expect("conic fit kernel");
                    ensure(
                        bounds.iter().all(|v| v.is_finite()),
                        "conic leaf fit bounds are not representable",
                    )?;
                    merge_geo_bounds(&mut self.fit_bounds, bounds);
                    previous = point;
                }
            }
            return self.point(out, b);
        }
        ensure(
            depth < self.options.max_depth,
            "adaptive geographic precision cannot be met within max_depth",
        )?;
        let mid = t(0.5);
        self.recursive(
            [a, samples[1]],
            curve,
            [range[0], mid],
            (span / 2., depth + 1),
            out,
            geo,
        )?;
        self.recursive(
            [samples[1], b],
            curve,
            [mid, range[1]],
            (span / 2., depth + 1),
            out,
            geo,
        )
    }
    pub fn planar_ring(&mut self, ring: &Ring) -> Result<Ring> {
        let mut out = vec![];
        if ring.is_empty() {
            return Ok(out);
        }
        self.start(&mut out, raw(ring[0], self.projection))?;
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let projection = self.projection;
            let parallel = (projection.latitude_bounded())
                && (a[1] - b[1]).abs() < EPS
                && (a[1].abs() - MERCATOR_LIMIT).abs() < EPS;
            let av = from_radians(a);
            let bv = from_radians(b);
            let span = av.cross(bv).norm().atan2(av.dot(bv));
            let geo_curve = |t: f64| -> Result<[f64; 2]> {
                let p = if parallel {
                    [a[0] + (b[0] - a[0]) * t, a[1]]
                } else {
                    let mut p = spherical(interpolate_arc(av, bv, t)?);
                    let hint = a[0] + (b[0] - a[0]) * t;
                    while p[0] - hint > PI {
                        p[0] -= TAU;
                    }
                    while p[0] - hint < -PI {
                        p[0] += TAU;
                    }
                    p
                };
                Ok(p)
            };
            if projection.conic_kernel().is_some() {
                self.segment_geo(
                    raw(a, projection),
                    raw(b, projection),
                    &geo_curve,
                    parallel,
                    span,
                    &mut out,
                )?;
                continue;
            }
            let curve = |t| Ok(raw(geo_curve(t)?, projection));
            self.segment(
                raw(a, projection),
                raw(b, projection),
                &curve,
                span,
                &mut out,
            )?;
        }
        out.pop();
        Ok(out)
    }
    pub fn cap_ring(
        &mut self,
        ring: &[V],
        angle: f64,
        is_circle: &impl Fn(V, V) -> bool,
    ) -> Result<Ring> {
        let mut out = vec![];
        if ring.is_empty() {
            return Ok(out);
        }
        let projection = self.projection;
        self.start(&mut out, raw_vector(ring[0], projection))?;
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            let circle = is_circle(a, b);
            let start = a.z.atan2(a.y);
            let mut delta = (b.z.atan2(b.y) - start).rem_euclid(TAU);
            if delta > PI {
                delta -= TAU;
            }
            let curve = |t: f64| -> Result<[f64; 2]> {
                let p = if circle {
                    let v = start + delta * t;
                    V {
                        x: angle.cos(),
                        y: angle.sin() * v.cos(),
                        z: angle.sin() * v.sin(),
                    }
                } else {
                    interpolate_arc(a, b, t)?
                };
                Ok(raw_vector(p, projection))
            };
            self.segment(
                raw_vector(a, projection),
                raw_vector(b, projection),
                &curve,
                if circle {
                    delta.abs()
                } else {
                    a.cross(b).norm().atan2(a.dot(b))
                },
                &mut out,
            )?;
        }
        out.pop();
        Ok(out)
    }
}
pub(super) fn distance_segment(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let length = dx.hypot(dy);
    if length == 0. {
        return (p[0] - a[0]).hypot(p[1] - a[1]);
    }
    let ux = dx / length;
    let uy = dy / length;
    let s = ((p[0] - a[0]) * ux + (p[1] - a[1]) * uy).clamp(0., length);
    (p[0] - a[0] - ux * s).hypot(p[1] - a[1] - uy * s)
}
