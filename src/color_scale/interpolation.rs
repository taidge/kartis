//! Interpolation in the D3 color spaces. The typed RGBA endpoints preserve
//! their authored components even at zero alpha.
use crate::scene::ensure;
use crate::{Color, Result};

/// How a polar color space moves between two defined hues.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HueInterpolation {
    #[default]
    Shortest,
    /// Interpolate the two authored hue coordinates directly, as in D3 Long.
    Direct,
}

/// Piecewise interpolation for continuous data-to-color scales.
/// Alpha remains linear in every space. Output is clipped to RGBA8.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ColorInterpolation {
    #[default]
    Srgb,
    /// D3-style power interpolation on the three RGB channels.
    RgbGamma {
        gamma: f64,
    },
    /// D50 CIELAB, as in d3-color.
    Lab,
    Hcl {
        hue: HueInterpolation,
    },
    Hsl {
        hue: HueInterpolation,
    },
    /// D3 Cubehelix, with gamma applied to the lightness parameter only.
    Cubehelix {
        hue: HueInterpolation,
        gamma: f64,
    },
}

type Coordinates = [Option<f64>; 3];

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PreparedInterpolation {
    pub(super) profile: ColorInterpolation,
    coordinates: Vec<Coordinates>,
}

impl PreparedInterpolation {
    pub(super) fn new(profile: ColorInterpolation, palette: &[Color]) -> Result<Self> {
        if let ColorInterpolation::RgbGamma { gamma }
        | ColorInterpolation::Cubehelix { gamma, .. } = profile
        {
            ensure(
                gamma.is_finite() && (0.01..=100.).contains(&gamma),
                "color interpolation gamma must be in 0.01..=100",
            )?;
        }
        let coordinates = match profile {
            ColorInterpolation::Srgb => Vec::new(),
            ColorInterpolation::RgbGamma { gamma } => palette
                .iter()
                .map(|&color| rgb_unit(color).map(|channel| Some(channel.powf(gamma))))
                .collect(),
            ColorInterpolation::Lab => palette
                .iter()
                .map(|&color| to_lab(color).map(Some))
                .collect(),
            ColorInterpolation::Hcl { .. } => palette.iter().map(|&color| to_hcl(color)).collect(),
            ColorInterpolation::Hsl { .. } => palette.iter().map(|&color| to_hsl(color)).collect(),
            ColorInterpolation::Cubehelix { .. } => {
                palette.iter().map(|&color| to_cubehelix(color)).collect()
            }
        };
        Ok(Self {
            profile,
            coordinates,
        })
    }

    pub(super) fn map(&self, palette: &[Color], index: usize, t: f64) -> Color {
        let a = palette[index];
        let b = palette[index + 1];
        if t == 0. || a == b {
            return a;
        }
        if t == 1. {
            return b;
        }
        if self.profile == ColorInterpolation::Srgb
            || self.profile == (ColorInterpolation::RgbGamma { gamma: 1. })
        {
            return super::interpolate(a, b, t);
        }
        let [a0, a1, a2] = self.coordinates[index];
        let [b0, b1, b2] = self.coordinates[index + 1];
        let alpha = byte(lerp(f64::from(a.3), f64::from(b.3), t));
        let mix = |left, right| mix_optional(left, right, t).unwrap_or_default();
        let rgb = match self.profile {
            ColorInterpolation::Srgb => unreachable!(),
            ColorInterpolation::RgbGamma { gamma } => {
                [mix(a0, b0), mix(a1, b1), mix(a2, b2)].map(|value| 255. * value.powf(1. / gamma))
            }
            ColorInterpolation::Lab => from_lab([mix(a0, b0), mix(a1, b1), mix(a2, b2)]),
            ColorInterpolation::Hcl { hue } => {
                let h = mix_hue(a0, b0, t, hue);
                let c = mix(a1, b1);
                let [aa, bb] = h.map_or([0., 0.], |h| {
                    let h = h.to_radians();
                    [c * h.cos(), c * h.sin()]
                });
                from_lab([mix(a2, b2), aa, bb])
            }
            ColorInterpolation::Hsl { hue } => {
                from_hsl(mix_hue(a0, b0, t, hue), mix(a1, b1), mix(a2, b2))
            }
            ColorInterpolation::Cubehelix { hue, gamma } => from_cubehelix(
                mix_hue(a0, b0, t, hue),
                mix(a1, b1),
                mix_optional(a2, b2, t.powf(gamma)).unwrap_or_default(),
            ),
        };
        let [r, g, b] = rgb.map(byte);
        Color(r, g, b, alpha)
    }
}

fn byte(value: f64) -> u8 {
    value.round().clamp(0., 255.) as u8
}

fn lerp(a: f64, b: f64, t: f64) -> f64 {
    if a == b { a } else { a + (b - a) * t }
}

fn mix_optional(a: Option<f64>, b: Option<f64>, t: f64) -> Option<f64> {
    match (a, b) {
        (Some(a), Some(b)) => Some(lerp(a, b, t)),
        _ => a.or(b),
    }
}

fn mix_hue(a: Option<f64>, b: Option<f64>, t: f64, mode: HueInterpolation) -> Option<f64> {
    let (Some(a), Some(b)) = (a, b) else {
        return a.or(b);
    };
    let delta = b - a;
    let delta = if mode == HueInterpolation::Shortest && delta.abs() > 180. {
        delta - delta.signum() * 360.
    } else {
        delta
    };
    Some(if delta == 0. { a } else { a + delta * t })
}

fn rgb_unit(Color(r, g, b, _): Color) -> [f64; 3] {
    [r, g, b].map(|c| f64::from(c) / 255.)
}

// D50 white point and the CIELAB piecewise transfer constants.
const WHITE_X: f64 = 0.96422;
const WHITE_Z: f64 = 0.82521;
const LAB_OFFSET: f64 = 4. / 29.;
const LAB_LINEAR_LIMIT: f64 = 6. / 29.;
const LAB_SLOPE: f64 = 3. * LAB_LINEAR_LIMIT * LAB_LINEAR_LIMIT;
const LAB_POWER_LIMIT: f64 = LAB_LINEAR_LIMIT * LAB_LINEAR_LIMIT * LAB_LINEAR_LIMIT;

fn to_lab(color: Color) -> [f64; 3] {
    let [r, g, b] = rgb_unit(color).map(|channel| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    let xyz = |channel: f64| {
        if channel > LAB_POWER_LIMIT {
            channel.cbrt()
        } else {
            channel / LAB_SLOPE + LAB_OFFSET
        }
    };
    let y = xyz(0.2225045 * r + 0.7168786 * g + 0.0606169 * b);
    let (x, z) = if r == g && g == b {
        (y, y)
    } else {
        (
            xyz((0.4360747 * r + 0.3850649 * g + 0.1430804 * b) / WHITE_X),
            xyz((0.0139322 * r + 0.0971045 * g + 0.7141733 * b) / WHITE_Z),
        )
    };
    [116. * y - 16., 500. * (x - y), 200. * (y - z)]
}

fn from_lab([light, a, b]: [f64; 3]) -> [f64; 3] {
    let xyz = |channel: f64| {
        if channel > LAB_LINEAR_LIMIT {
            channel * channel * channel
        } else {
            LAB_SLOPE * (channel - LAB_OFFSET)
        }
    };
    let yy = (light + 16.) / 116.;
    let x = WHITE_X * xyz(yy + a / 500.);
    let y = xyz(yy);
    let z = WHITE_Z * xyz(yy - b / 200.);
    [
        3.1338561 * x - 1.6168667 * y - 0.4906146 * z,
        -0.9787684 * x + 1.9161415 * y + 0.0334540 * z,
        0.0719453 * x - 0.2289914 * y + 1.4052427 * z,
    ]
    .map(|channel| {
        255. * if channel <= 0.0031308 {
            12.92 * channel
        } else {
            1.055 * channel.powf(1. / 2.4) - 0.055
        }
    })
}

fn to_hcl(color: Color) -> Coordinates {
    let [l, a, b] = to_lab(color);
    if a == 0. && b == 0. {
        [None, (l > 0. && l < 100.).then_some(0.), Some(l)]
    } else {
        [
            Some(b.atan2(a).to_degrees().rem_euclid(360.)),
            Some((a * a + b * b).sqrt()),
            Some(l),
        ]
    }
}

fn to_hsl(color: Color) -> Coordinates {
    let [r, g, b] = rgb_unit(color);
    let low = r.min(g).min(b);
    let high = r.max(g).max(b);
    let range = high - low;
    let light = (high + low) / 2.;
    if range == 0. {
        return [None, (light > 0. && light < 1.).then_some(0.), Some(light)];
    }
    let hue = if r == high {
        (g - b) / range + if g < b { 6. } else { 0. }
    } else if g == high {
        (b - r) / range + 2.
    } else {
        (r - g) / range + 4.
    };
    let saturation = range
        / if light < 0.5 {
            high + low
        } else {
            2. - high - low
        };
    [Some(hue * 60.), Some(saturation), Some(light)]
}

fn from_hsl(hue: Option<f64>, saturation: f64, light: f64) -> [f64; 3] {
    let Some(hue) = hue.filter(|h| h.is_finite()) else {
        return [255. * light; 3];
    };
    let m2 = light + if light < 0.5 { light } else { 1. - light } * saturation;
    let m1 = 2. * light - m2;
    [120., 0., 240.].map(|offset| {
        let h = (hue + offset).rem_euclid(360.);
        255. * if h < 60. {
            m1 + (m2 - m1) * h / 60.
        } else if h < 180. {
            m2
        } else if h < 240. {
            m1 + (m2 - m1) * (240. - h) / 60.
        } else {
            m1
        }
    })
}

// Cubehelix coefficients from Dave Green's RGB helix and d3-color.
const HELIX_A: f64 = -0.14861;
const HELIX_B: f64 = 1.78277;
const HELIX_C: f64 = -0.29227;
const HELIX_D: f64 = -0.90649;
const HELIX_E: f64 = 1.97294;

fn to_cubehelix(color: Color) -> Coordinates {
    let [r, g, b] = rgb_unit(color);
    let ed = HELIX_E * HELIX_D;
    let eb = HELIX_E * HELIX_B;
    let cross = HELIX_B * HELIX_C - HELIX_D * HELIX_A;
    let light = (cross * b + ed * r - eb * g) / (cross + ed - eb);
    let blue = b - light;
    let k = (HELIX_E * (g - light) - HELIX_C * blue) / HELIX_D;
    let saturation = (k * k + blue * blue).sqrt() / (HELIX_E * light * (1. - light));
    let hue = (saturation.is_finite() && saturation != 0.)
        .then(|| (k.atan2(blue).to_degrees() - 120.).rem_euclid(360.));
    [
        hue,
        saturation.is_finite().then_some(saturation),
        Some(light),
    ]
}

fn from_cubehelix(hue: Option<f64>, saturation: f64, light: f64) -> [f64; 3] {
    let theta = hue.map_or(0., |h| (h + 120.).to_radians());
    let (sin, cos) = theta.sin_cos();
    let amount = saturation * light * (1. - light);
    [
        HELIX_A * cos + HELIX_B * sin,
        HELIX_C * cos + HELIX_D * sin,
        HELIX_E * cos,
    ]
    .map(|v| 255. * (light + amount * v))
}
