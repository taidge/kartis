//! Fixed three-region Albers USA frame, following d3-geo 3.1.1 albersUsa.js.
//! The existing checked conic kernels supply projection and inverse equations.
use super::*;
use crate::ClipRect;

const CLIP_EPSILON: f64 = 1e-6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GeoUsaRegion {
    Lower48,
    Alaska,
    Hawaii,
}

impl GeoUsaRegion {
    /// Forward priority and stream multiplex order used by the original factory.
    pub const ALL: [Self; 3] = [Self::Lower48, Self::Alaska, Self::Hawaii];

    fn index(self) -> usize {
        match self {
            Self::Lower48 => 0,
            Self::Alaska => 1,
            Self::Hawaii => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoUsaProjectedPoint {
    /// Logical Scene pixels.
    pub position: [f64; 2],
    pub region: GeoUsaRegion,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoUsaInverse {
    /// Longitude and latitude in degrees.
    pub position: [f64; 2],
    pub region: GeoUsaRegion,
}

/// The immutable fixed-region composite at an explicit scale and translation.
/// Its regions have independent post-projection clips; no author camera,
/// standard parallels or clip-angle controls are introduced.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoAlbersUsaProjection {
    scale: f64,
    translate: [f64; 2],
    // Preserve the actual upper edges, rather than reconstructing them from a
    // rounded width during point clipping. Public rectangles share these bounds.
    clips: [[f64; 4]; 3],
}

impl Default for GeoAlbersUsaProjection {
    fn default() -> Self {
        Self::new(1070., [480., 250.]).expect("finite Albers USA default frame")
    }
}

/// A standard child context can use this camera and raw origin directly. The
/// offset is in normalized screen coordinates (positive Y points downward).
#[derive(Clone, Copy, Debug)]
pub(super) struct UsaComponentSpec {
    pub projection: GeoProjection,
    pub center: [f64; 2],
    pub raw_origin: [f64; 2],
    pub ratio: f64,
    pub offset: [f64; 2],
    native_inverse: [f64; 3],
}

pub(super) fn component_spec(region: GeoUsaRegion) -> UsaComponentSpec {
    static SPECS: std::sync::OnceLock<[UsaComponentSpec; 3]> = std::sync::OnceLock::new();
    SPECS.get_or_init(|| {
        let make = |parallels: [f64; 2],
                    longitude: f64,
                    origin: [f64; 2],
                    ratio: f64,
                    offset: [f64; 2]| {
            // Use generic CEA for all three children: Albers would apply its
            // intrinsic +96-degree frame and raw-origin subtraction a second time.
            let projection = GeoProjection::conic_equal_area(parallels)
                .expect("finite fixed Albers USA parallels");
            let kernel = projection.conic_kernel().unwrap();
            let n = kernel.n().unwrap();
            let sin_first = parallels[0].to_radians().sin();
            let c = 1. + sin_first * (2. * n - sin_first);
            UsaComponentSpec {
                projection,
                center: [-longitude, 0.],
                raw_origin: kernel.raw_project(origin.map(f64::to_radians)),
                ratio,
                offset,
                native_inverse: [n, c, c.sqrt() / n],
            }
        };
        [
            make([29.5, 45.5], 96., [-0.6, 38.7], 1., [0., 0.]),
            make([55., 65.], 154., [-2., 58.5], 0.35, [-0.307, 0.201]),
            make([8., 18.], 157., [-3., 19.9], 1., [-0.205, 0.212]),
        ]
    })[region.index()]
}

impl GeoAlbersUsaProjection {
    /// Scale is in pixels per composite raw unit. All three positive-width
    /// clips must remain representable within the ordinary Scene coordinate
    /// budget; scales that collapse the epsilon-inset windows reject.
    pub fn new(scale: f64, translate: [f64; 2]) -> Result<Self> {
        ensure(
            scale.is_finite() && scale > 0. && scale <= 1_000_000.,
            "invalid Albers USA scale",
        )?;
        ensure(
            translate
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "invalid Albers USA translation",
        )?;
        let [x, y] = translate;
        let e = CLIP_EPSILON;
        let clips = [
            [
                x - 0.455 * scale,
                y - 0.238 * scale,
                x + 0.455 * scale,
                y + 0.238 * scale,
            ],
            [
                x - 0.425 * scale + e,
                y + 0.120 * scale + e,
                x - 0.214 * scale - e,
                y + 0.234 * scale - e,
            ],
            [
                x - 0.214 * scale + e,
                y + 0.166 * scale + e,
                x - 0.115 * scale - e,
                y + 0.234 * scale - e,
            ],
        ];
        for [x0, y0, x1, y1] in clips {
            ensure(
                x0 < x1 && y0 < y1,
                "Albers USA region clip collapsed at this scale or translation",
            )?;
            ClipRect {
                x: x0,
                y: y0,
                width: x1 - x0,
                height: y1 - y0,
            }
            .validate()?;
        }
        Ok(Self {
            scale,
            translate,
            clips,
        })
    }

    pub fn scale(&self) -> f64 {
        self.scale
    }
    pub fn translate(&self) -> [f64; 2] {
        self.translate
    }

    pub(super) fn component_scale(&self, region: GeoUsaRegion) -> f64 {
        self.scale * component_spec(region).ratio
    }

    pub(super) fn component_translate(&self, region: GeoUsaRegion) -> [f64; 2] {
        let offset = component_spec(region).offset;
        [
            self.translate[0] + self.scale * offset[0],
            self.translate[1] + self.scale * offset[1],
        ]
    }

    fn component_affine(&self, region: GeoUsaRegion) -> [f64; 3] {
        let spec = component_spec(region);
        let scale = self.component_scale(region);
        let translate = self.component_translate(region);
        // D3 projection.recenter constructs scaleTranslate with the projected
        // center already multiplied. Keep its operation order at clip edges.
        [
            translate[0] - scale * spec.raw_origin[0],
            translate[1] + scale * spec.raw_origin[1],
            scale,
        ]
    }

    pub fn region_clip(&self, region: GeoUsaRegion) -> ClipRect {
        let [x0, y0, x1, y1] = self.clips[region.index()];
        ClipRect {
            x: x0,
            y: y0,
            width: x1 - x0,
            height: y1 - y0,
        }
    }

    fn contains(&self, region: GeoUsaRegion, [x, y]: [f64; 2]) -> bool {
        let [x0, y0, x1, y1] = self.clips[region.index()];
        x >= x0 && x <= x1 && y >= y0 && y <= y1
    }

    /// Select the first visible region in Lower48, Alaska, Hawaii order.
    pub fn project(&self, position: [f64; 2]) -> Result<Option<GeoUsaProjectedPoint>> {
        valid_position(position)?;
        for region in GeoUsaRegion::ALL {
            if let Some(position) = self.project_in_region(region, position)? {
                return Ok(Some(GeoUsaProjectedPoint { position, region }));
            }
        }
        Ok(None)
    }

    /// Project through one known component and its own rectangle. This retains
    /// source-region identity when composite image windows overlap.
    pub fn project_in_region(
        &self,
        region: GeoUsaRegion,
        position: [f64; 2],
    ) -> Result<Option<[f64; 2]>> {
        let pixel = self.project_unclipped(region, position)?;
        if !self.contains(region, pixel) {
            return Ok(None);
        }
        inverse_input(pixel)?;
        Ok(Some(pixel))
    }

    fn project_unclipped(&self, region: GeoUsaRegion, position: [f64; 2]) -> Result<[f64; 2]> {
        valid_position(position)?;
        let spec = component_spec(region);
        // These fixed components use longitude-only rotation. Match the
        // original rotateLambda directly; a Cartesian round-trip adds avoidable
        // trigonometric rounding before exact rectangle clipping.
        let longitude = wrapped_longitude(position[0].to_radians() - spec.center[0].to_radians());
        let q = spec
            .projection
            .conic_kernel()
            .unwrap()
            .raw_project([longitude, position[1].to_radians()]);
        ensure(
            q.iter().all(|v| v.is_finite()),
            "Albers USA forward arithmetic is not representable",
        )?;
        let [dx, dy, scale] = self.component_affine(region);
        let pixel = [dx + scale * q[0], dy - scale * q[1]];
        ensure(
            pixel.iter().all(|v| v.is_finite()),
            "Albers USA pixel arithmetic is not representable",
        )?;
        Ok(pixel)
    }

    /// D3's inset-first inverse selection, qualified by this library's visible
    /// footprint contract. Composite image overlaps prevent a global source
    /// round-trip guarantee; use a known region when provenance is available.
    pub fn invert(&self, pixel: [f64; 2]) -> Result<Option<GeoUsaInverse>> {
        inverse_input(pixel)?;
        let x = (pixel[0] - self.translate[0]) / self.scale;
        let y = (pixel[1] - self.translate[1]) / self.scale;
        let region = if (0.120..0.234).contains(&y) && (-0.425..-0.214).contains(&x) {
            GeoUsaRegion::Alaska
        } else if (0.166..0.234).contains(&y) && (-0.214..-0.115).contains(&x) {
            GeoUsaRegion::Hawaii
        } else {
            GeoUsaRegion::Lower48
        };
        Ok(self
            .invert_in_region(region, pixel)?
            .map(|position| GeoUsaInverse { position, region }))
    }

    /// Invert within the specified region only. The requested pixel must be
    /// inside that region's exact rectangle. Acceptance then checks unclipped
    /// same-region reprojection. At an exact edge the recovered binary64 source
    /// can itself forward-clip a rounding ulp outside; the input domain does not
    /// depend on that secondary visibility decision. No other region is tried.
    pub fn invert_in_region(
        &self,
        region: GeoUsaRegion,
        pixel: [f64; 2],
    ) -> Result<Option<[f64; 2]>> {
        inverse_input(pixel)?;
        if !self.contains(region, pixel) {
            return Ok(None);
        }
        let spec = component_spec(region);
        let [dx, dy, scale] = self.component_affine(region);
        let raw = [(pixel[0] - dx) / scale, (dy - pixel[1]) / scale];
        ensure(
            raw.iter().all(|v| v.is_finite()),
            "Albers USA inverse arithmetic is not representable",
        )?;
        let Some(source) = inverse_source(spec, raw)? else {
            return Ok(None);
        };
        let longitude = wrapped_longitude(source[0] + spec.center[0].to_radians());
        let position = [longitude.to_degrees(), source[1].to_degrees()];
        valid_position(position)?;
        let again = self.project_unclipped(region, position)?;
        let residual = ((again[0] - pixel[0]) / scale)
            .abs()
            .max(((again[1] - pixel[1]) / scale).abs());
        Ok((residual <= super::conic_kernel::REPROJECT_LIMIT).then_some(position))
    }
}

fn inverse_source(spec: UsaComponentSpec, raw: [f64; 2]) -> Result<Option<[f64; 2]>> {
    // Keep the existing checked kernel's finite-domain and uncertainty admission.
    // Once admitted, use the original CEA expression to choose its floating-point
    // representative. Squaring hypot instead of x*x+y*y changes exact clip-edge
    // round trips by an ulp even though the spherical positions agree.
    if spec.projection.conic_inverse(raw)?.is_none() {
        return Ok(None);
    }
    let [n, c, r0] = spec.native_inverse;
    let [x, y] = raw;
    let q = r0 - y;
    let sign = |v: f64| if v == 0. { v } else { v.signum() };
    let mut longitude = x.atan2(q.abs()) * sign(q);
    if q * n < 0. {
        longitude -= PI * sign(x) * sign(q);
    }
    let source = [
        longitude / n,
        ((c - (x * x + q * q) * n * n) / (2. * n))
            .clamp(-1., 1.)
            .asin(),
    ];
    if !source.iter().all(|v| v.is_finite()) {
        return Ok(None);
    }
    let again = spec.projection.conic_kernel().unwrap().raw_project(source);
    let residual = (again[0] - x).abs().max((again[1] - y).abs());
    Ok((residual <= super::conic_kernel::REPROJECT_LIMIT).then_some(source))
}

fn inverse_input(point: [f64; 2]) -> Result<()> {
    ensure(
        point.iter().all(|v| v.is_finite() && v.abs() <= 1_000_000.),
        "Albers USA point exceeds Scene coordinates",
    )
}

fn wrapped_longitude(mut longitude: f64) -> f64 {
    if longitude.abs() > PI {
        longitude -= (longitude / TAU).round() * TAU;
    }
    longitude
}
