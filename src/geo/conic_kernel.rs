//! Resolved conic projection equations and checked inverse branches.
//! Adapted from d3-geo 3.1.1 (Mike Bostock), ISC; full text in
//! tests/fixtures/geo-inverse-d3-LICENSE.txt.
use std::f64::consts::{FRAC_PI_2, PI};
const D3_EPSILON: f64 = 1e-6;
/// Boundary roundoff allowance, separate from ordinary forward comparison.
const BOUNDARY_EPSILON: f64 = 1e-12;
/// Fixed raw-unit residual bound; not a guarantee in arbitrary Scene affine units.
pub const REPROJECT_LIMIT: f64 = 1e-7;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Parallels([u64; 2]);
impl Parallels {
    pub fn new(degrees: [f64; 2]) -> Result<Self, &'static str> {
        if !degrees.iter().all(|v| v.is_finite() && v.abs() < 90.) {
            return Err("parallels must be finite and strictly inside (-90,90)");
        }
        Ok(Self(
            degrees.map(|v| (if v == 0. { 0. } else { v }).to_bits()),
        ))
    }
    pub fn degrees(self) -> [f64; 2] {
        self.0.map(f64::from_bits)
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    EqualArea,
    Equidistant,
    Conformal,
}
#[derive(Clone, Copy, Debug)]
enum Formula {
    CylindricalEqualArea { cosine: f64 },
    Equirectangular,
    Mercator,
    EqualArea { n: f64, c: f64, r0: f64 },
    Equidistant { n: f64, g: f64 },
    Conformal { n: f64, f: f64 },
}
#[derive(Clone, Copy, Debug)]
pub struct Kernel {
    formula: Formula,
    parallels: Parallels,
}
fn checked_n(n: f64) -> Result<f64, &'static str> {
    // Rounded adjacent-standard-parallel difference quotients can leave the
    // mathematical |n|<=1 range. Preserve native diagnostics, but reject the
    // selected family rather than creating a multiple-turn, nonunique cone.
    if !n.is_finite() || n.abs() > 1. {
        return Err("conic coefficient n must be finite and within [-1,1]");
    }
    Ok(n)
}
// Cauchy mean-value theorem: EQD/CC n must lie between the sines
// of the authored standard latitudes. Eight adjacent binary64 values at each endpoint are an explicit
// engineering endpoint allowance for binary64/libm, not a separation cutoff.
fn checked_standard_interval(n: f64, a: f64, b: f64) -> Result<(), &'static str> {
    let sa = a.sin();
    let sb = b.sin();
    let mut lower = sa.min(sb);
    let mut upper = sa.max(sb);
    for _ in 0..8 {
        lower = lower.next_down();
        upper = upper.next_up();
    }
    if n < lower || n > upper {
        return Err("conic coefficient n violates the standard-latitude sine interval");
    }
    Ok(())
}
fn tany(y: f64) -> f64 {
    ((FRAC_PI_2 + y) / 2.).tan()
}
impl Kernel {
    pub fn new(kind: Kind, parallels: Parallels) -> Result<Self, &'static str> {
        let [a, b] = parallels.degrees().map(|d| d * (PI / 180.));
        let formula = match kind {
            Kind::EqualArea => {
                let sa = a.sin();
                let n = checked_n((sa + b.sin()) / 2.)?;
                checked_standard_interval(n, a, b)?;
                if n.abs() < D3_EPSILON {
                    Formula::CylindricalEqualArea { cosine: a.cos() }
                } else {
                    let c = 1. + sa * (2. * n - sa);
                    let r0 = c.sqrt() / n;
                    if !r0.is_finite() {
                        return Err("nonfinite equal-area coefficients");
                    }
                    Formula::EqualArea { n, c, r0 }
                }
            }
            Kind::Equidistant => {
                let ca = a.cos();
                let n = if a == b {
                    a.sin()
                } else {
                    (ca - b.cos()) / (b - a)
                };
                let n = checked_n(n)?;
                if n.abs() < D3_EPSILON {
                    checked_standard_interval(n, a, b)?;
                    Formula::Equirectangular
                } else {
                    let g = ca / n + a;
                    if !g.is_finite() {
                        return Err("nonfinite equidistant coefficients");
                    }
                    // The complete sphere must stay on the signed radial
                    // branch selected by n. A zero radius inside the latitude
                    // interval folds distinct latitudes/longitudes together.
                    if n.signum() * g < FRAC_PI_2 {
                        return Err("equidistant radial origin crosses the full-latitude domain");
                    }
                    checked_standard_interval(n, a, b)?;
                    Formula::Equidistant { n, g }
                }
            }
            Kind::Conformal => {
                let ca = a.cos();
                let n = if a == b {
                    a.sin()
                } else {
                    (ca / b.cos()).ln() / (tany(b) / tany(a)).ln()
                };
                let n = checked_n(n)?;
                checked_standard_interval(n, a, b)?;
                if n == 0. {
                    Formula::Mercator
                } else {
                    let f = ca * tany(a).powf(n) / n;
                    if !f.is_finite() {
                        return Err("nonfinite conformal coefficients");
                    }
                    Formula::Conformal { n, f }
                }
            }
        };
        Ok(Self { formula, parallels })
    }
    /// Only the equal-area standard-parallel preset; no D3 factory frame/center.
    pub fn albers_raw() -> Result<Self, &'static str> {
        Self::new(Kind::EqualArea, Parallels::new([29.5, 45.5])?)
    }
    pub fn branch(self) -> &'static str {
        match self.formula {
            Formula::CylindricalEqualArea { .. } => "cylindrical-equal-area",
            Formula::Equirectangular => "equirectangular",
            Formula::Mercator => "mercator",
            Formula::EqualArea { .. } => "conic-equal-area",
            Formula::Equidistant { .. } => "conic-equidistant",
            Formula::Conformal { .. } => "conic-conformal",
        }
    }
    pub fn n(self) -> Option<f64> {
        match self.formula {
            Formula::EqualArea { n, .. }
            | Formula::Equidistant { n, .. }
            | Formula::Conformal { n, .. } => Some(n),
            _ => None,
        }
    }
    /// Original raw forward arithmetic, including original nonfinite/pole behavior.
    pub fn raw_project(self, [lambda, mut phi]: [f64; 2]) -> [f64; 2] {
        match self.formula {
            Formula::CylindricalEqualArea { cosine } => [lambda * cosine, phi.sin() / cosine],
            Formula::Equirectangular => [lambda, phi],
            Formula::Mercator => [lambda, tany(phi).ln()],
            Formula::EqualArea { n, c, r0 } => {
                let r = (c - 2. * n * phi.sin()).sqrt() / n;
                let theta = lambda * n;
                [r * theta.sin(), r0 - r * theta.cos()]
            }
            Formula::Equidistant { n, g } => {
                let gy = g - phi;
                let theta = n * lambda;
                [gy * theta.sin(), g - gy * theta.cos()]
            }
            Formula::Conformal { n, f } => {
                if f > 0. {
                    if phi < -FRAC_PI_2 + D3_EPSILON {
                        phi = -FRAC_PI_2 + D3_EPSILON;
                    }
                } else if phi > FRAC_PI_2 - D3_EPSILON {
                    phi = FRAC_PI_2 - D3_EPSILON;
                }
                let r = f / tany(phi).powf(n);
                [r * (n * lambda).sin(), f - r * (n * lambda).cos()]
            }
        }
    }
    /// Canonical geometric inverse, qualified by domain and forward reprojection.
    /// At a pole longitude is a representative; Conformal clamped poles are
    /// represented at the clamp boundary and are not an exact authored inverse.
    pub fn invert(self, p: [f64; 2]) -> Result<Option<[f64; 2]>, &'static str> {
        if !p.iter().all(|v| v.is_finite()) {
            return Err("nonfinite inverse query");
        }
        let [x, y] = p;
        let mut source = match self.formula {
            Formula::CylindricalEqualArea { cosine } => {
                let s = y * cosine;
                if s.abs() > 1. + BOUNDARY_EPSILON {
                    return Ok(None);
                }
                [x / cosine, s.clamp(-1., 1.).asin()]
            }
            Formula::Equirectangular => [x, y],
            Formula::Mercator => [x, 2. * y.exp().atan() - FRAC_PI_2],
            Formula::EqualArea { n, c, r0 } => {
                let q = r0 - y;
                let radius = x.hypot(q);
                let s = (c - radius * radius * n * n) / (2. * n);
                if !s.is_finite() {
                    return Err("inverse radial expression overflow");
                }
                if s.abs() > 1. + BOUNDARY_EPSILON {
                    return Ok(None);
                }
                [
                    (x * n.signum()).atan2(q * n.signum()) / n,
                    s.clamp(-1., 1.).asin(),
                ]
            }
            Formula::Equidistant { n, g } => {
                let q = g - y;
                [
                    (x * n.signum()).atan2(q * n.signum()) / n,
                    g - n.signum() * x.hypot(q),
                ]
            }
            Formula::Conformal { n, f } => {
                let q = f - y;
                let r = n.signum() * x.hypot(q);
                [
                    (x * n.signum()).atan2(q * n.signum()) / n,
                    2. * (f / r).powf(1. / n).atan() - FRAC_PI_2,
                ]
            }
        };
        if !source.iter().all(|v| v.is_finite()) {
            return Ok(None);
        }
        if source[0].abs() > PI + BOUNDARY_EPSILON || source[1].abs() > FRAC_PI_2 + BOUNDARY_EPSILON
        {
            return Ok(None);
        }
        source[0] = source[0].clamp(-PI, PI);
        source[1] = source[1].clamp(-FRAC_PI_2, FRAC_PI_2);
        let again = self.raw_project(source);
        if !again.iter().all(|v| v.is_finite()) {
            return Ok(None);
        }
        let residual = (again[0] - x).abs().max((again[1] - y).abs());
        Ok((residual <= REPROJECT_LIMIT).then_some(source))
    }
}

impl PartialEq for Kernel {
    fn eq(&self, b: &Self) -> bool {
        self.parallels == b.parallels && self.branch() == b.branch()
    }
}
impl Eq for Kernel {}

impl Kernel {
    /// Conditional engineering estimate; fixed coefficients, gamma8 basic
    /// arithmetic plus eight-ulp libm. Includes cancelled large intermediates.
    pub fn estimated_inverse_uncertainty(self, [x, y]: [f64; 2], source: [f64; 2]) -> f64 {
        let u = f64::EPSILON / 2.;
        let gamma = 8. * u / (1. - 8. * u);
        let (origin, n) = match self.formula {
            Formula::EqualArea { n, r0, .. } => (r0, n),
            Formula::Equidistant { n, g } => (g, n),
            Formula::Conformal { n, f } => (f, n),
            Formula::Equirectangular => return gamma * (x.abs() + y.abs()),
            Formula::Mercator => return gamma * (x.abs() + y.abs().max(1.)),
            Formula::CylindricalEqualArea { cosine } => {
                let e = (gamma + 8. * u) * y.abs();
                let lo = ((y - e) * cosine).clamp(-1., 1.).asin();
                let hi = ((y + e) * cosine).clamp(-1., 1.).asin();
                return (source[1] - lo).abs().max((hi - source[1]).abs())
                    + gamma * x.abs() / cosine.abs();
            }
        };
        let q = origin - y;
        let r = x.hypot(q);
        let ex = gamma * x.abs() + 8. * u * r;
        let ey = gamma * (origin.abs() + r) + 8. * u * r;
        let delta = ex.hypot(ey + 4. * u * q.abs());
        let dl = if delta < r {
            (delta / r).min(1.).asin() / n.abs()
        } else {
            f64::INFINITY
        };
        let radial_phi = |radius: f64| match self.formula {
            Formula::EqualArea { n, c, .. } => ((c - radius * radius * n * n) / (2. * n))
                .clamp(-1., 1.)
                .asin(),
            Formula::Equidistant { n, g } => g - n.signum() * radius,
            Formula::Conformal { n, f } => 2. * (f.abs() / radius).powf(1. / n).atan() - FRAC_PI_2,
            _ => unreachable!(),
        };
        // Factor two widens this estimate for omitted correlated basic ops.
        let a = radial_phi((r - delta).max(0.));
        let b = radial_phi(r + delta);
        let dp = (source[1] - a).abs().max((source[1] - b).abs()) + 16. * u;
        2. * (dp + dl * (source[1].cos().abs() + dp).min(1.))
    }
}

#[cfg(test)]
mod interval_checks {
    use super::*;
    #[test]
    fn zero_quotient_is_not_a_valid_negative_adjacent_standard_fallback() {
        let [a, b] = [-0.9585319170638371, -0.9585319170638373].map(f64::to_radians);
        assert!(checked_standard_interval(0., a, b).is_err());
        assert!(checked_standard_interval(a.sin(), a, b).is_ok());
    }
    #[test]
    fn equal_symmetric_and_nearby_controls() {
        for a in [-75., -53.13008026016052, -0.9585319170638371, 0., 30., 75.] {
            for pair in [[a, a], [a, -a], [a, a + 1e-5], [a + 1e-5, a]] {
                for k in [Kind::EqualArea, Kind::Equidistant, Kind::Conformal] {
                    assert!(
                        Kernel::new(k, Parallels::new(pair).unwrap()).is_ok(),
                        "{pair:?} {k:?}"
                    );
                }
            }
        }
    }
}
