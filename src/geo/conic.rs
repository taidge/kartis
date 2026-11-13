//! Finite checked conic parameters, cached kernels and intrinsic projection frame.
use super::conic_kernel::{Kernel, Kind, Parallels};
use super::*;
/// A finite pair of authored standard latitudes inside (-90,90). Signed zero is
/// canonicalized. Kernels are resolved once; adaptive samples do no coefficient work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GeoConicParallels {
    degrees: Parallels,
    kernels: [std::result::Result<Kernel, &'static str>; 3],
}
impl GeoConicParallels {
    pub fn new(degrees: [f64; 2]) -> Result<Self> {
        let degrees = Parallels::new(degrees).map_err(|s| ChartError(s.into()))?;
        let kernel = |kind| Kernel::new(kind, degrees);
        Ok(Self {
            degrees,
            kernels: [
                kernel(Kind::EqualArea),
                kernel(Kind::Equidistant),
                kernel(Kind::Conformal),
            ],
        })
    }
    pub fn degrees(self) -> [f64; 2] {
        self.degrees.degrees()
    }
}
impl GeoProjection {
    pub fn conic_equal_area(parallels: [f64; 2]) -> Result<Self> {
        let projection = Self::ConicEqualArea(GeoConicParallels::new(parallels)?);
        projection.validate_conic()?;
        Ok(projection)
    }
    pub fn conic_equidistant(parallels: [f64; 2]) -> Result<Self> {
        let projection = Self::ConicEquidistant(GeoConicParallels::new(parallels)?);
        projection.validate_conic()?;
        Ok(projection)
    }
    pub fn conic_conformal(parallels: [f64; 2]) -> Result<Self> {
        let projection = Self::ConicConformal(GeoConicParallels::new(parallels)?);
        projection.validate_conic()?;
        Ok(projection)
    }
    pub fn conic_equal_area_default() -> Self {
        Self::conic_equal_area([0., 60.]).expect("finite defaults")
    }
    pub fn conic_equidistant_default() -> Self {
        Self::conic_equidistant([0., 60.]).expect("finite defaults")
    }
    pub fn conic_conformal_default() -> Self {
        Self::conic_conformal([30., 30.]).expect("finite defaults")
    }
    pub(super) fn validate_conic(self) -> Result<()> {
        let kernel = match self {
            Self::ConicEqualArea(p) => p.kernels[0],
            Self::ConicEquidistant(p) => p.kernels[1],
            Self::ConicConformal(p) => p.kernels[2],
            _ => return Ok(()),
        };
        kernel.map(|_| ()).map_err(|s| ChartError(s.into()))
    }
    pub(super) fn latitude_bounded(self) -> bool {
        matches!(
            self,
            Self::Mercator | Self::TransverseMercator | Self::ConicConformal(_)
        )
    }
    pub(super) fn conic_kernel(self) -> Option<Kernel> {
        match self {
            Self::ConicEqualArea(p) => p.kernels[0].ok(),
            Self::ConicEquidistant(p) => p.kernels[1].ok(),
            Self::ConicConformal(p) => p.kernels[2].ok(),
            Self::Albers => {
                static K: std::sync::OnceLock<Kernel> = std::sync::OnceLock::new();
                Some(*K.get_or_init(|| Kernel::albers_raw().expect("finite Albers defaults")))
            }
            _ => None,
        }
    }
    fn conic_origin(self) -> [f64; 2] {
        if self == Self::Albers {
            {
                static O: std::sync::OnceLock<[f64; 2]> = std::sync::OnceLock::new();
                *O.get_or_init(|| {
                    self.conic_kernel()
                        .unwrap()
                        .raw_project([-0.6_f64.to_radians(), 38.7_f64.to_radians()])
                })
            }
        } else {
            [0., 0.]
        }
    }
    pub(super) fn conic_project(self, p: [f64; 2]) -> [f64; 2] {
        let q = self.conic_kernel().unwrap().raw_project(p);
        let o = self.conic_origin();
        [q[0] - o[0], q[1] - o[1]]
    }
    /// Cone-sector extrema over a longitude/latitude rectangle. Radial
    /// endpoints and sin/cos stationary angles cover the full finite rectangle.
    fn conic_rectangle(self, longitude: [f64; 2], latitude: [f64; 2]) -> Option<[f64; 4]> {
        let k = self.conic_kernel()?;
        let mut longitudes = vec![longitude[0], longitude[1]];
        if let Some(n) = k.n() {
            for i in -2..=2 {
                let l = f64::from(i) * FRAC_PI_2 / n;
                if l >= longitude[0] && l <= longitude[1] {
                    longitudes.push(l);
                }
            }
        }
        let mut b = [
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ];
        for l in longitudes {
            for f in latitude {
                let q = self.conic_project([l, f]);
                if q.iter().any(|v| !v.is_finite()) {
                    return Some([f64::NAN; 4]);
                }
                b = [
                    b[0].min(q[0]),
                    b[1].min(q[1]),
                    b[2].max(q[0]),
                    b[3].max(q[1]),
                ];
            }
        }
        Some(b)
    }
    pub(super) fn conic_world(self) -> Option<[f64; 4]> {
        let limit = if self.latitude_bounded() {
            MERCATOR_LIMIT
        } else {
            FRAC_PI_2
        };
        self.conic_rectangle([-PI, PI], [-limit, limit])
    }
    /// Conservative curved-edge fit envelope. Longitude is monotonic along a
    /// nonpolar minor great circle; exact z extrema bound latitude. Projecting
    /// this small rectangle includes the entire edge without adding fake path
    /// vertices. Existing two-degree topology sampling keeps excess fit small.
    pub(super) fn conic_arc_bounds(
        self,
        a: [f64; 2],
        b: [f64; 2],
        parallel: bool,
    ) -> Option<[f64; 4]> {
        self.conic_kernel()?;
        let mut latitude = [a[1].min(b[1]), a[1].max(b[1])];
        let mut longitude = [a[0].min(b[0]), a[0].max(b[0])];
        if !parallel {
            let av = from_radians(a);
            let bv = from_radians(b);
            let theta = av.cross(bv).norm().atan2(av.dot(bv));
            if theta > 1e-14 {
                let tangent = (bv.z - av.z * theta.cos()) / theta.sin();
                let phase = tangent.atan2(av.z);
                for turn in -2..=2 {
                    let t = phase + f64::from(turn) * PI;
                    if t > 0. && t < theta {
                        let f = (av.z * t.cos() + tangent * t.sin()).clamp(-1., 1.).asin();
                        latitude = [latitude[0].min(f), latitude[1].max(f)];
                    }
                }
            }
            if latitude[0] <= -FRAC_PI_2 + 1e-10 || latitude[1] >= FRAC_PI_2 - 1e-10 {
                longitude = [-PI, PI];
            }
        }
        self.conic_rectangle(longitude, latitude)
    }
    pub(super) fn conic_inverse(self, point: [f64; 2]) -> Result<Option<[f64; 2]>> {
        let o = self.conic_origin();
        let point = [point[0] + o[0], point[1] + o[1]];
        let k = self.conic_kernel().unwrap();
        let Some(source) = k.invert(point).map_err(|s| ChartError(s.into()))? else {
            return Ok(None);
        };
        if self.latitude_bounded() && source[1].abs() > MERCATOR_LIMIT + EPS {
            return Ok(None);
        }
        // Engineering estimate, conditional on fixed resolved coefficients and
        // <=8 ulp libm error; not a certified all-input source error theorem.
        let estimate = k.estimated_inverse_uncertainty(point, source);
        ensure(
            estimate.is_finite() && estimate <= 1e-7,
            "conic inverse precision estimate exceeds 1e-7 spherical-vector units",
        )?;
        Ok(Some(source))
    }
}
