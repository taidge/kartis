//! Explicit logical-coordinate fill paints shared by SVG and Canvas backends.
use crate::{Color, Result, scene::ensure};
mod pattern;
pub use pattern::*;
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GradientStop {
    pub offset: f64,
    pub color: Color,
}
impl GradientStop {
    pub fn new(offset: f64, color: Color) -> Self {
        Self { offset, color }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum FillPaint {
    Pattern(PatternFill),
    Linear {
        start: [f64; 2],
        end: [f64; 2],
        stops: Vec<GradientStop>,
    },
    Radial {
        center: [f64; 2],
        radius: f64,
        stops: Vec<GradientStop>,
    },
}
pub const MAX_GRADIENT_STOPS: usize = 512;
pub const MAX_SCENE_GRADIENT_STOPS: usize = 16_384;
impl FillPaint {
    pub fn stops(&self) -> &[GradientStop] {
        match self {
            Self::Pattern(_) => &[],
            Self::Linear { stops, .. } | Self::Radial { stops, .. } => stops,
        }
    }
    pub fn validate(&self) -> Result<()> {
        let coordinate = |x: f64| x.is_finite() && x.abs() <= 1_000_000.;
        match self {
            Self::Pattern(pattern) => return pattern.validate(),
            Self::Linear { start, end, .. } => ensure(
                start.iter().chain(end).copied().all(coordinate) && start != end,
                "linear gradient needs distinct finite endpoints within Scene coordinates",
            )?,
            Self::Radial { center, radius, .. } => ensure(
                center.iter().copied().all(coordinate)
                    && coordinate(*radius)
                    && *radius > 0.
                    && [
                        center[0] - radius,
                        center[0] + radius,
                        center[1] - radius,
                        center[1] + radius,
                    ]
                    .into_iter()
                    .all(coordinate),
                "invalid radial gradient extent",
            )?,
        }
        let stops = self.stops();
        ensure(
            (2..=MAX_GRADIENT_STOPS).contains(&stops.len()),
            "gradient needs 2..=512 stops",
        )?;
        ensure(
            stops.first().unwrap().offset == 0.
                && stops.last().unwrap().offset == 1.
                && stops
                    .iter()
                    .all(|s| s.offset.is_finite() && (0. ..=1.).contains(&s.offset))
                && stops.windows(2).all(|s| s[0].offset <= s[1].offset),
            "gradient stops must be ordered from 0 to 1",
        )
    }
    /// Exact bit encoding avoids aliasing between different definitions in
    /// independent SVGs embedded in one document, without global/random IDs.
    pub(crate) fn svg_id(&self) -> String {
        use std::fmt::Write;
        let mut id = String::from("kartis-paint-");
        let values = match self {
            Self::Pattern(pattern) => return pattern.deterministic_id(),
            Self::Linear { start, end, .. } => {
                id.push('l');
                vec![start[0], start[1], end[0], end[1]]
            }
            Self::Radial { center, radius, .. } => {
                id.push('r');
                vec![center[0], center[1], *radius]
            }
        };
        for v in values {
            write!(id, "{:016x}", if v == 0. { 0 } else { v.to_bits() }).unwrap();
        }
        for s in self.stops() {
            let Color(r, g, b, a) = s.color;
            write!(
                id,
                "-{:016x}{r:02x}{g:02x}{b:02x}{a:02x}",
                if s.offset == 0. {
                    0
                } else {
                    s.offset.to_bits()
                }
            )
            .unwrap();
        }
        id
    }
}
