//! Named continuous palettes from Nivo's interpolator catalog.
//!
//! The names follow Nivo's colors package. The interpolators and packed color
//! data follow d3-scale-chromatic 3.1.0, d3-interpolate 3.0.1, and d3-color 3.1.0.
//! The upstream ISC and ColorBrewer attribution is in the neighboring schemes
//! directory. No JavaScript or runtime color table loading is required.
#[path = "schemes/data.rs"]
mod data;
#[path = "schemes/trig.rs"]
mod trig;

use std::f64::consts::PI;
use std::fmt;
use std::ops::RangeInclusive;
use std::str::FromStr;

use crate::scene::ensure;
use crate::{ChartError, Color, Result};

enum Profile {
    Brewer(&'static [&'static [u32]]),
    Lookup(&'static [u32; 256]),
    Turbo,
    Cividis,
    Cubehelix { from: [f64; 3], to: [f64; 3] },
    Rainbow,
    Sinebow,
}

macro_rules! named_schemes {
    ($($variant:ident => ($id:literal, $profile:expr)),+ $(,)?) => {
        /// A named Nivo/D3 continuous color interpolator.
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub enum ColorScheme {
            $($variant),+
        }
        impl ColorScheme {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// The case-sensitive Nivo interpolator ID.
            pub const fn id(self) -> &'static str {
                match self { $(Self::$variant => $id),+ }
            }

            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $($id => Some(Self::$variant),)+
                    _ => None,
                }
            }

            fn profile(self) -> Profile {
                match self { $(Self::$variant => $profile),+ }
            }
        }
    };
}

named_schemes! {
    Blues => ("blues", Profile::Brewer(data::BLUES)),
    Greens => ("greens", Profile::Brewer(data::GREENS)),
    Greys => ("greys", Profile::Brewer(data::GREYS)),
    Oranges => ("oranges", Profile::Brewer(data::ORANGES)),
    Purples => ("purples", Profile::Brewer(data::PURPLES)),
    Reds => ("reds", Profile::Brewer(data::REDS)),
    BlueGreen => ("blue_green", Profile::Brewer(data::BLUE_GREEN)),
    BluePurple => ("blue_purple", Profile::Brewer(data::BLUE_PURPLE)),
    GreenBlue => ("green_blue", Profile::Brewer(data::GREEN_BLUE)),
    OrangeRed => ("orange_red", Profile::Brewer(data::ORANGE_RED)),
    PurpleBlueGreen => ("purple_blue_green", Profile::Brewer(data::PURPLE_BLUE_GREEN)),
    PurpleBlue => ("purple_blue", Profile::Brewer(data::PURPLE_BLUE)),
    PurpleRed => ("purple_red", Profile::Brewer(data::PURPLE_RED)),
    RedPurple => ("red_purple", Profile::Brewer(data::RED_PURPLE)),
    YellowGreenBlue => ("yellow_green_blue", Profile::Brewer(data::YELLOW_GREEN_BLUE)),
    YellowGreen => ("yellow_green", Profile::Brewer(data::YELLOW_GREEN)),
    YellowOrangeBrown => ("yellow_orange_brown", Profile::Brewer(data::YELLOW_ORANGE_BROWN)),
    YellowOrangeRed => ("yellow_orange_red", Profile::Brewer(data::YELLOW_ORANGE_RED)),
    BrownBlueGreen => ("brown_blueGreen", Profile::Brewer(data::BROWN_BLUE_GREEN)),
    PurpleRedGreen => ("purpleRed_green", Profile::Brewer(data::PURPLE_RED_GREEN)),
    PinkYellowGreen => ("pink_yellowGreen", Profile::Brewer(data::PINK_YELLOW_GREEN)),
    PurpleOrange => ("purple_orange", Profile::Brewer(data::PURPLE_ORANGE)),
    RedBlue => ("red_blue", Profile::Brewer(data::RED_BLUE)),
    RedGrey => ("red_grey", Profile::Brewer(data::RED_GREY)),
    RedYellowBlue => ("red_yellow_blue", Profile::Brewer(data::RED_YELLOW_BLUE)),
    RedYellowGreen => ("red_yellow_green", Profile::Brewer(data::RED_YELLOW_GREEN)),
    Spectral => ("spectral", Profile::Brewer(data::SPECTRAL)),
    Viridis => ("viridis", Profile::Lookup(data::VIRIDIS)),
    Inferno => ("inferno", Profile::Lookup(data::INFERNO)),
    Magma => ("magma", Profile::Lookup(data::MAGMA)),
    Plasma => ("plasma", Profile::Lookup(data::PLASMA)),
    Turbo => ("turbo", Profile::Turbo),
    Cividis => ("cividis", Profile::Cividis),
    Warm => ("warm", Profile::Cubehelix {
        from: [-100., 0.75, 0.35],
        to: [80., 1.5, 0.8],
    }),
    Cool => ("cool", Profile::Cubehelix {
        from: [260., 0.75, 0.35],
        to: [80., 1.5, 0.8],
    }),
    CubehelixDefault => ("cubehelixDefault", Profile::Cubehelix {
        from: [300., 0.5, 0.],
        to: [-240., 0.5, 1.],
    }),
    Rainbow => ("rainbow", Profile::Rainbow),
    Sinebow => ("sinebow", Profile::Sinebow),
}

impl ColorScheme {
    /// Sample the named interpolator. Finite inputs are clamped to [0, 1]
    /// as in Nivo's domain scales. Both cyclical schemes cover one turn.
    pub fn sample(self, t: f64) -> Result<Color> {
        ensure(t.is_finite(), "color scheme parameter must be finite")?;
        Ok(self.sample_unit(t.clamp(0., 1.)))
    }

    /// The official ColorBrewer palette lengths, when present.
    pub fn palette_sizes(self) -> Option<RangeInclusive<usize>> {
        match self.profile() {
            Profile::Brewer(sizes) => Some(3..=sizes.len() + 2),
            _ => None,
        }
    }

    /// Return an official discrete palette at this exact size.
    /// Continuous-only schemes and unavailable lengths return an error.
    pub fn palette(self, size: usize) -> Result<Vec<Color>> {
        let Profile::Brewer(sizes) = self.profile() else {
            return Err(ChartError(
                "color scheme has no official discrete palette".into(),
            ));
        };
        let colors = size.checked_sub(3).and_then(|i| sizes.get(i));
        colors
            .map(|colors| colors.iter().copied().map(packed).collect())
            .ok_or_else(|| ChartError("unsupported discrete color scheme size".into()))
    }

    pub(crate) fn lookup_len(self) -> Option<usize> {
        match self.profile() {
            Profile::Lookup(colors) => Some(colors.len()),
            _ => None,
        }
    }

    pub(crate) fn sample_unit(self, t: f64) -> Color {
        debug_assert!((0. ..=1.).contains(&t));
        match self.profile() {
            Profile::Brewer(sizes) => basis(sizes[sizes.len() - 1], t),
            Profile::Lookup(colors) => packed(colors[((t * 256.) as usize).min(255)]),
            Profile::Turbo => polynomial(&TURBO, t),
            Profile::Cividis => polynomial(&CIVIDIS, t),
            Profile::Cubehelix { from, to } => {
                let [h, s, l] = std::array::from_fn(|i| from[i] + t * (to[i] - from[i]));
                cubehelix(h, s, l)
            }
            Profile::Rainbow => {
                let distance = (t - 0.5).abs();
                cubehelix(360. * t - 100., 1.5 - 1.5 * distance, 0.8 - 0.9 * distance)
            }
            Profile::Sinebow => {
                let angle = (0.5 - t) * PI;
                rgb([0., PI / 3., 2. * PI / 3.].map(|offset| {
                    let sine = trig::sin(angle + offset);
                    255. * sine * sine
                }))
            }
        }
    }
}

impl fmt::Display for ColorScheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}
impl FromStr for ColorScheme {
    type Err = ChartError;

    fn from_str(value: &str) -> Result<Self> {
        Self::from_name(value).ok_or_else(|| ChartError("unknown color scheme name".into()))
    }
}

fn packed(value: u32) -> Color {
    Color((value >> 16) as u8, (value >> 8) as u8, value as u8, 255)
}

fn rgb([r, g, b]: [f64; 3]) -> Color {
    let channel = |value: f64| value.round().clamp(0., 255.) as u8;
    Color(channel(r), channel(g), channel(b), 255)
}

// D3's open uniform cubic basis uses extrapolated end controls. The largest
// official Brewer palette supplies the controls and the endpoint colors.
fn basis(colors: &[u32], t: f64) -> Color {
    let intervals = colors.len() - 1;
    let n = intervals as f64;
    let i = ((t * n) as usize).min(intervals - 1);
    let u = (t - i as f64 / n) * n;
    let u2 = u * u;
    let u3 = u2 * u;
    let weights = [
        1. - 3. * u + 3. * u2 - u3,
        4. - 6. * u2 + 3. * u3,
        1. + 3. * u + 3. * u2 - 3. * u3,
        u3,
    ];
    rgb([16, 8, 0].map(|shift| {
        let channel = |index: usize| f64::from(((colors[index] >> shift) & 255) as u8);
        let v1 = channel(i);
        let v2 = channel(i + 1);
        let v0 = if i == 0 { 2. * v1 - v2 } else { channel(i - 1) };
        let v3 = if i == intervals - 1 {
            2. * v2 - v1
        } else {
            channel(i + 2)
        };
        (weights[0] * v0 + weights[1] * v1 + weights[2] * v2 + weights[3] * v3) / 6.
    }))
}

const TURBO: [[f64; 6]; 3] = [
    [34.61, 1172.33, -10793.56, 33300.12, -38394.49, 14825.05],
    [23.31, 557.33, 1225.33, -3574.96, 1073.77, 707.56],
    [27.2, 3211.1, -15327.97, 27814., -22569.18, 6838.66],
];
const CIVIDIS: [[f64; 6]; 3] = [
    [-4.54, -35.34, 2381.73, -6402.7, 7024.72, -2710.57],
    [32.49, 170.73, 52.82, -131.46, 176.58, -67.37],
    [81.24, 442.36, -2482.43, 6167.24, -6614.94, 2475.67],
];
fn polynomial(coefficients: &[[f64; 6]; 3], t: f64) -> Color {
    rgb(coefficients.map(|channel| channel.into_iter().rev().fold(0., |value, c| c + t * value)))
}

fn cubehelix(hue: f64, saturation: f64, lightness: f64) -> Color {
    let angle = (hue + 120.) * (PI / 180.);
    let (sin, cos) = trig::sin_cos(angle);
    let amplitude = saturation * lightness * (1. - lightness);
    rgb([
        255. * (lightness + amplitude * (-0.14861 * cos + 1.78277 * sin)),
        255. * (lightness + amplitude * (-0.29227 * cos - 0.90649 * sin)),
        255. * (lightness + amplitude * (1.97294 * cos)),
    ])
}
