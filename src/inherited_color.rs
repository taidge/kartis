//! Resolve typed inherited paint using the ordered color modifiers used by Nivo.
use crate::{Color, Result, scene::ensure};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColorModifier {
    Brighter(f64),
    Darker(f64),
    /// Replace the alpha with a fraction in [0, 1].
    Opacity(f64),
}

/// Paint inputs known at the point where a role is drawn.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InheritedColorContext {
    pub inherited: Color,
    pub theme: Color,
    /// The layer immediately under the role, such as a filled pie slice.
    pub surface: Color,
    /// The chart background beneath that layer.
    pub backdrop: Color,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum InheritedColor {
    #[default]
    Theme,
    Fixed(Color),
    FromContext(Vec<ColorModifier>),
    /// Choose the candidate with higher relative luminance contrast. Both
    /// candidates are evaluated after compositing their alpha on the surface.
    /// Ties use `dark`. A translucent surface needs an opaque backdrop.
    Contrast {
        dark: Color,
        light: Color,
    },
}

impl InheritedColor {
    pub fn validate(&self) -> Result<()> {
        if let Self::FromContext(modifiers) = self {
            ensure(modifiers.len() <= 32, "too many inherited color modifiers")?;
            for modifier in modifiers {
                match *modifier {
                    ColorModifier::Brighter(amount) | ColorModifier::Darker(amount) => {
                        ensure(
                            amount.is_finite() && amount.abs() <= 32.,
                            "color brightness amount must be finite and in -32..=32",
                        )?;
                    }
                    ColorModifier::Opacity(alpha) => {
                        ensure(
                            (0. ..=1.).contains(&alpha),
                            "color opacity must be in [0, 1]",
                        )?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn resolve(&self, context: InheritedColorContext) -> Result<Color> {
        self.validate()?;
        self.resolve_validated(context)
    }

    pub(crate) fn resolve_validated(&self, context: InheritedColorContext) -> Result<Color> {
        match self {
            Self::Theme => Ok(context.theme),
            Self::Fixed(color) => Ok(*color),
            Self::FromContext(modifiers) => Ok(modify(context.inherited, modifiers)),
            Self::Contrast { dark, light } => {
                ensure(
                    context.surface.3 == 255 || context.backdrop.3 == 255,
                    "color contrast requires an opaque effective background",
                )?;
                let background = over(context.surface, rgb(context.backdrop));
                let background_luminance = luminance(background);
                let score = |color| {
                    let foreground = luminance(over(color, background));
                    (foreground.max(background_luminance) + 0.05)
                        / (foreground.min(background_luminance) + 0.05)
                };
                Ok(if score(*light) > score(*dark) {
                    *light
                } else {
                    *dark
                })
            }
        }
    }
}

fn modify(Color(r, g, b, alpha): Color, modifiers: &[ColorModifier]) -> Color {
    let mut rgb = [r, g, b].map(f64::from);
    let mut alpha = f64::from(alpha);
    for modifier in modifiers {
        let multiplier = match *modifier {
            ColorModifier::Brighter(amount) => (1. / 0.7_f64).powf(amount),
            ColorModifier::Darker(amount) => 0.7_f64.powf(amount),
            ColorModifier::Opacity(fraction) => {
                alpha = fraction * 255.;
                continue;
            }
        };
        rgb = rgb.map(|channel| channel * multiplier);
    }
    let byte = |value: f64| value.round().clamp(0., 255.) as u8;
    let [r, g, b] = rgb.map(byte);
    Color(r, g, b, byte(alpha))
}

fn rgb(Color(r, g, b, _): Color) -> [f64; 3] {
    [r, g, b].map(|channel| f64::from(channel) / 255.)
}

fn over(color: Color, backdrop: [f64; 3]) -> [f64; 3] {
    let rgb = rgb(color);
    let alpha = f64::from(color.3) / 255.;
    std::array::from_fn(|i| rgb[i] * alpha + backdrop[i] * (1. - alpha))
}

fn luminance(rgb: [f64; 3]) -> f64 {
    let [r, g, b] = rgb.map(|channel| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    });
    0.2126 * r + 0.7152 * g + 0.0722 * b
}
