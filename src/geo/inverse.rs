//! Checked inverse of the existing fitted spherical projections. D3-geo 3.1.1
//! supplies the raw inverse equations (ISC); the fit and camera are Kartis's
//! already accepted context, not an unrelated planar annotation transform.
use super::*;

impl GeoProjectionContext {
    /// Recover longitude/latitude from logical Scene pixels. Nonfinite input,
    /// Scene-budget violations and unrepresentable affine arithmetic are errors.
    /// Points outside the projection's intrinsic footprint or spherical cap
    /// return `None`; polygon coverage and the plot rectangle are not inferred.
    ///
    /// Like `project`, longitude is canonical within +/-180 degrees. Longitude
    /// at a pole and the two antimeridian representations are not unique;
    /// compare spherical positions rather than requiring one longitude there.
    pub fn invert(&self, point: [f64; 2]) -> Result<Option<[f64; 2]>> {
        inverse_input(point)?;
        let raw_point = [
            (point[0] - self.translate[0]) / self.scale + self.center[0],
            (self.translate[1] - point[1]) / self.scale + self.center[1],
        ];
        ensure(
            raw_point.iter().all(|v| v.is_finite()),
            "geographic inverse arithmetic is not representable",
        )?;
        let checked = if self.view.projection.conic_kernel().is_some() {
            self.view
                .projection
                .conic_inverse(raw_point)?
                .map(from_radians)
        } else {
            inverse_raw(raw_point, self.view.projection)
        };
        let Some(vector) = checked else {
            return Ok(None);
        };
        let position = inverse_camera(vector, &self.view);
        valid_position(position)?;
        // Reuse the actual forward visibility policy, including its deliberately
        // bounded Mercator and azimuthal tolerances. An algebraic inverse alone
        // must not resurrect the back hemisphere or a clipped singularity.
        if self.project(position)?.is_none() {
            return Ok(None);
        }
        Ok(Some(position))
    }

    /// Inverse for visible-window picking. The inclusive plot boundary follows
    /// `ClipRect::contains`; outside points return None before spherical work.
    /// This still does not claim that a feature polygon covers the queried pixel.
    pub fn invert_in_plot(&self, point: [f64; 2]) -> Result<Option<[f64; 2]>> {
        inverse_input(point)?;
        if !self.plot.contains(point[0], point[1]) {
            return Ok(None);
        }
        self.invert(point)
    }
}

fn inverse_input(point: [f64; 2]) -> Result<()> {
    ensure(
        point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "inverse geographic point exceeds Scene coordinates",
    )
}
fn bounded(value: f64, limit: f64) -> Option<f64> {
    let tolerance = 16. * f64::EPSILON * limit.abs().max(1.);
    (value.abs() <= limit + tolerance).then(|| value.clamp(-limit, limit))
}
fn inverse_raw([x, y]: [f64; 2], projection: GeoProjection) -> Option<V> {
    let spherical_inverse = |l: f64, f: f64| Some(from_radians([l, f]));
    match projection {
        GeoProjection::ConicEqualArea(_)
        | GeoProjection::ConicEquidistant(_)
        | GeoProjection::ConicConformal(_)
        | GeoProjection::Albers => None,
        GeoProjection::Equirectangular => {
            spherical_inverse(bounded(x, PI)?, bounded(y, FRAC_PI_2)?)
        }
        GeoProjection::Mercator => {
            let l = bounded(x, PI)?;
            let f = y.sinh().atan();
            (f.abs() <= MERCATOR_LIMIT + EPS).then(|| from_radians([l, f]))
        }
        GeoProjection::TransverseMercator => {
            let l = -bounded(y, PI)?;
            let f = x.tanh().asin();
            (f.abs() <= MERCATOR_LIMIT + EPS).then(|| from_radians([l, f]))
        }
        GeoProjection::EqualEarth => {
            let m = 3_f64.sqrt() / 2.;
            let limit = m.asin();
            let target = bounded(y, equal_earth_y(limit))?;
            let t = solve(target, limit, equal_earth_y, equal_earth_derivative);
            let l = bounded(m * x * equal_earth_derivative(t) / t.cos(), PI)?;
            let f = (t.sin() / m).clamp(-1., 1.).asin();
            spherical_inverse(l, f)
        }
        GeoProjection::NaturalEarth => {
            let target = bounded(y, raw([0., FRAC_PI_2], projection)[1])?;
            let f = solve(
                target,
                FRAC_PI_2,
                |p| raw([0., p], projection)[1],
                natural_earth_derivative,
            );
            let l = bounded(x / raw([1., f], projection)[0], PI)?;
            spherical_inverse(l, f)
        }
        GeoProjection::Orthographic => {
            let r = x.hypot(y);
            if !r.is_finite() || r > 1. + 16. * f64::EPSILON {
                return None;
            }
            let factor = if r > 1. { 1. / r } else { 1. };
            Some(V {
                x: (1. - r.min(1.).powi(2)).max(0.).sqrt(),
                y: x * factor,
                z: y * factor,
            })
        }
        GeoProjection::AzimuthalEqualArea => {
            let r = x.hypot(y);
            if !r.is_finite() || r > 2. + 32. * f64::EPSILON {
                return None;
            }
            let r = r.min(2.);
            let k = (1. - r * r / 4.).max(0.).sqrt();
            Some(V {
                x: 1. - r * r / 2.,
                y: x * k,
                z: y * k,
            })
        }
        GeoProjection::AzimuthalEquidistant => {
            let r = bounded(x.hypot(y), PI)?;
            let k = if r == 0. { 1. } else { r.sin() / r };
            Some(V {
                x: r.cos(),
                y: x * k,
                z: y * k,
            })
        }
        GeoProjection::Stereographic => {
            let r = x.hypot(y);
            if !r.is_finite() || r > 1. + 2. * EPS {
                return None;
            }
            let denominator = 1. + r * r;
            Some(V {
                x: (1. - r * r) / denominator,
                y: 2. * x / denominator,
                z: 2. * y / denominator,
            })
        }
        GeoProjection::Gnomonic => {
            let norm = 1_f64.hypot(x.hypot(y));
            if !norm.is_finite() {
                return None;
            }
            Some(V {
                x: 1. / norm,
                y: x / norm,
                z: y / norm,
            })
        }
    }
}
fn inverse_camera(mut v: V, view: &GeoMapOptions) -> [f64; 2] {
    if view.projection == GeoProjection::Albers {
        let a = 96_f64.to_radians();
        v = V {
            x: v.x * a.cos() + v.y * a.sin(),
            y: -v.x * a.sin() + v.y * a.cos(),
            z: v.z,
        };
    }
    let roll = view.rotation.to_radians();
    let (east, north) = if view.projection == GeoProjection::TransverseMercator {
        let gamma = FRAC_PI_2 - roll;
        (
            v.y * gamma.cos() + v.z * gamma.sin(),
            -v.y * gamma.sin() + v.z * gamma.cos(),
        )
    } else {
        (
            v.y * roll.cos() - v.z * roll.sin(),
            v.y * roll.sin() + v.z * roll.cos(),
        )
    };
    let center = view.center[1].to_radians();
    let local_x = v.x * center.cos() - north * center.sin();
    let global_z = v.x * center.sin() + north * center.cos();
    let mut longitude = east.atan2(local_x) + view.center[0].to_radians();
    if longitude > PI {
        longitude -= TAU;
    } else if longitude < -PI {
        longitude += TAU;
    }
    [
        longitude.to_degrees(),
        global_z.atan2(local_x.hypot(east)).to_degrees(),
    ]
}
fn equal_earth_y(t: f64) -> f64 {
    let t2 = t * t;
    let t6 = t2 * t2 * t2;
    t * (1.340264 - 0.081106 * t2 + t6 * (0.000893 + 0.003796 * t2))
}
fn equal_earth_derivative(t: f64) -> f64 {
    let t2 = t * t;
    let t6 = t2 * t2 * t2;
    1.340264 - 3. * 0.081106 * t2 + t6 * (7. * 0.000893 + 9. * 0.003796 * t2)
}
fn natural_earth_derivative(p: f64) -> f64 {
    let p2 = p * p;
    let p4 = p2 * p2;
    1.007226
        + p2 * (3. * 0.015085 + p4 * (-7. * 0.044475 + 9. * 0.028874 * p2 - 11. * 0.005916 * p4))
}
/// Safeguarded Newton on a bounded monotonic branch. The bracket avoids the
/// unrelated polynomial roots that unrestricted Newton may find outside Earth.
fn solve(
    target: f64,
    limit: f64,
    value: impl Fn(f64) -> f64,
    derivative: impl Fn(f64) -> f64,
) -> f64 {
    if target == value(limit) {
        return limit;
    }
    if target == value(-limit) {
        return -limit;
    }
    let (mut lo, mut hi) = (-limit, limit);
    let mut current = (target / value(limit) * limit).clamp(lo, hi);
    let mut best = current;
    let mut residual = f64::INFINITY;
    for _ in 0..64 {
        let error = value(current) - target;
        if error.abs() < residual {
            best = current;
            residual = error.abs();
        }
        if error == 0. {
            break;
        }
        if error < 0. {
            lo = current;
        } else {
            hi = current;
        }
        let next = current - error / derivative(current);
        let next = if next.is_finite() && next > lo && next < hi {
            next
        } else {
            lo + (hi - lo) / 2.
        };
        if next == current {
            break;
        }
        current = next;
    }
    best
}
