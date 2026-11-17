//! Shared, bounded font selection for chart text and renderer-independent measurement.
//!
//! A named family is a CSS string, distinct from a generic family. Font data and
//! native face selection remain explicit application resources.
use std::borrow::Cow;
use std::collections::BTreeMap;

use glory::style::Color;
pub use glory::style::Family as FontFamily;

use crate::scene::ensure;
use crate::{Label, MAX_MARKS, Result};

/// Native glyph stroke, painted after the label fill with round joins.
/// Width is the full centered stroke width in Scene units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LabelStroke {
    pub color: Color,
    pub width: f64,
}
impl LabelStroke {
    pub fn validate(self) -> Result<()> {
        ensure(
            self.width.is_finite() && (0.0..=4096.0).contains(&self.width),
            "invalid label stroke width",
        )
    }
}

/// Explicit single-line whitespace behavior. Legacy leaves each host's old
/// behavior unchanged; Preserve retains spaces and maps each line/control
/// separator to one space, matching the existing native shaper's normalization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LabelWhitespace {
    #[default]
    Legacy,
    Preserve,
}

impl LabelWhitespace {
    pub(crate) fn measurement_signature(self, font: &ResolvedFont) -> String {
        match self {
            Self::Legacy => font.signature(),
            Self::Preserve => format!("preserve:{}", font.signature()),
        }
    }

    pub fn normalize(self, text: &str) -> Cow<'_, str> {
        let separator = |c| {
            matches!(
                c,
                '\t' | '\n' | '\r' | '\u{b}' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}'
            )
        };
        if self == Self::Legacy || !text.chars().any(separator) {
            Cow::Borrowed(text)
        } else {
            Cow::Owned(
                text.chars()
                    .map(|c| if separator(c) { ' ' } else { c })
                    .collect(),
            )
        }
    }

    pub fn drawable_label(self, label: &Label) -> Cow<'_, Label> {
        match self.normalize(&label.text) {
            Cow::Borrowed(_) => Cow::Borrowed(label),
            Cow::Owned(text) => {
                let mut drawable = label.clone();
                drawable.text = text;
                Cow::Owned(drawable)
            }
        }
    }
}

/// Optional declarations, inherited independently for family and weight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FontOptions {
    pub family: Option<Vec<FontFamily>>,
    /// Absolute CSS weight, including fractional variable-font weights.
    pub weight: Option<f64>,
}

impl FontOptions {
    pub fn is_empty(&self) -> bool {
        self.family.is_none() && self.weight.is_none()
    }

    pub fn validate(&self) -> Result<()> {
        if let Some(family) = &self.family {
            validate_families(family)?;
        }
        ensure(
            self.weight
                .is_none_or(|weight| weight.is_finite() && (1.0..=1000.0).contains(&weight)),
            "font weight must be finite and in 1..=1000",
        )
    }

    /// Resolve declarations over an already resolved parent. Validate before
    /// publishing the resulting font to a host or measuring with it.
    pub fn resolve(&self, parent: &ResolvedFont) -> ResolvedFont {
        ResolvedFont {
            family: self.family.clone().unwrap_or_else(|| parent.family.clone()),
            weight: self.weight.unwrap_or(parent.weight),
        }
    }

    /// Overlay this declaration on an optional parent without losing whether
    /// the application actually requested a font.
    pub fn inherit(&self, parent: &Self) -> Self {
        Self {
            family: self.family.clone().or_else(|| parent.family.clone()),
            weight: self.weight.or(parent.weight),
        }
    }
}

/// The identical family list and weight supplied to drawing and measurement.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFont {
    pub family: Vec<FontFamily>,
    pub weight: f64,
}

impl Default for ResolvedFont {
    fn default() -> Self {
        Self {
            family: vec![FontFamily::Sans],
            weight: 400.0,
        }
    }
}

impl ResolvedFont {
    pub fn validate(&self) -> Result<()> {
        validate_families(&self.family)?;
        ensure(
            self.weight.is_finite() && (1.0..=1000.0).contains(&self.weight),
            "font weight must be finite and in 1..=1000",
        )
    }

    /// CSS family list. Named faces are always quoted and escaped, including
    /// names which happen to be spelled like a generic family.
    pub fn css_family(&self) -> String {
        self.family
            .iter()
            .map(|family| match family {
                FontFamily::Named(name) => {
                    let mut quoted = String::with_capacity(name.len() + 2);
                    quoted.push('"');
                    for ch in name.chars() {
                        if matches!(ch, '"' | '\\') {
                            quoted.push('\\');
                        }
                        quoted.push(ch);
                    }
                    quoted.push('"');
                    quoted
                }
                FontFamily::Serif => "serif".into(),
                FontFamily::Sans => "sans-serif".into(),
                FontFamily::Mono => "monospace".into(),
                FontFamily::System => "system-ui".into(),
                FontFamily::Cursive => "cursive".into(),
                FontFamily::Fantasy => "fantasy".into(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// Canvas font shorthand. The ordinary 400 weight may be omitted without
    /// changing its meaning, preserving the existing default font string.
    pub fn canvas_font(&self, size: f64) -> String {
        if self.weight == 400.0 {
            format!("{size}px {}", self.css_family())
        } else {
            format!("{} {size}px {}", self.weight, self.css_family())
        }
    }

    /// Bounded, lossless font component for measured-layout signatures. CSS
    /// quoting preserves printable Unicode directly, avoiding Debug's expanded
    /// escapes for format/private-use characters in otherwise legal face names.
    pub(crate) fn signature(&self) -> String {
        let families = self.css_family();
        format!(
            "{}:{families}:{:016x}",
            families.len(),
            self.weight.to_bits()
        )
    }
}

fn validate_families(families: &[FontFamily]) -> Result<()> {
    ensure(
        !families.is_empty() && families.len() <= 16,
        "font family list must contain 1..=16 entries",
    )?;
    let mut bytes = 0;
    for family in families {
        if let FontFamily::Named(name) = family {
            ensure(
                !name.trim().is_empty() && name.len() <= 256 && !name.chars().any(char::is_control),
                "named font family must contain 1..=256 bytes without controls",
            )?;
            bytes += name.len();
        }
    }
    ensure(bytes <= 2048, "font family name budget exceeded")
}

/// Font declarations travel with a Scene, including retained updates, exports,
/// hiding and animation. Label keys refer to the corresponding Scene label slot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SceneTypography {
    pub default: FontOptions,
    pub labels: BTreeMap<usize, FontOptions>,
    /// Sparse opt-in label modes; absent slots retain legacy host behavior.
    pub label_whitespace: BTreeMap<usize, LabelWhitespace>,
    /// Sparse native glyph strokes; absent slots retain their original paint.
    pub label_strokes: BTreeMap<usize, LabelStroke>,
    pub legend: FontOptions,
    pub tooltip: FontOptions,
}

impl SceneTypography {
    pub fn validate(&self, label_count: usize) -> Result<()> {
        self.default.validate()?;
        self.legend.validate()?;
        self.tooltip.validate()?;
        ensure(
            self.labels.len() <= label_count && self.labels.len() <= MAX_MARKS,
            "label font budget exceeded",
        )?;
        for (index, font) in &self.labels {
            ensure(*index < label_count, "label font references an absent slot")?;
            font.validate()?;
        }
        ensure(
            self.label_strokes.len() <= label_count && self.label_strokes.len() <= MAX_MARKS,
            "label stroke budget exceeded",
        )?;
        for (&index, stroke) in &self.label_strokes {
            ensure(
                index < label_count,
                "label stroke references an absent slot",
            )?;
            stroke.validate()?;
        }
        ensure(
            self.label_whitespace.len() <= label_count && self.label_whitespace.len() <= MAX_MARKS,
            "label whitespace budget exceeded",
        )?;
        ensure(
            self.label_whitespace
                .keys()
                .all(|index| *index < label_count),
            "label whitespace references an absent slot",
        )?;
        Ok(())
    }

    pub fn label_whitespace(&self, index: usize) -> LabelWhitespace {
        self.label_whitespace
            .get(&index)
            .copied()
            .unwrap_or_default()
    }

    pub fn drawable_label<'a>(&self, index: usize, label: &'a Label) -> Cow<'a, Label> {
        self.label_whitespace(index).drawable_label(label)
    }

    pub fn label_options(&self, index: usize) -> FontOptions {
        self.labels
            .get(&index)
            .map_or_else(|| self.default.clone(), |font| font.inherit(&self.default))
    }

    /// False allows existing explicit-byte raster registries to retain their
    /// registration-order fallback when no font declaration was authored.
    pub fn has_label_font(&self, index: usize) -> bool {
        !self.default.is_empty() || self.labels.get(&index).is_some_and(|font| !font.is_empty())
    }

    pub fn label_font(&self, index: usize) -> ResolvedFont {
        self.label_options(index).resolve(&ResolvedFont::default())
    }

    /// Role inheritance is local declarations, then Scene role declarations,
    /// then Scene default declarations, then the legacy presentation weight.
    pub fn legend_font(&self, local: &FontOptions, legacy_weight: f64) -> ResolvedFont {
        self.role_font(&self.legend, local, legacy_weight)
    }

    pub fn tooltip_font(&self, local: &FontOptions, legacy_weight: f64) -> ResolvedFont {
        self.role_font(&self.tooltip, local, legacy_weight)
    }

    fn role_font(
        &self,
        role: &FontOptions,
        local: &FontOptions,
        legacy_weight: f64,
    ) -> ResolvedFont {
        local
            .inherit(&role.inherit(&self.default))
            .resolve(&ResolvedFont {
                weight: legacy_weight,
                ..ResolvedFont::default()
            })
    }

    /// Remap surviving labels after filtering, expansion, or removal. The map
    /// is from old slots to new slots; declarations for removed labels disappear.
    pub fn remap_labels(&mut self, slots: &BTreeMap<usize, usize>) {
        self.label_strokes = self
            .label_strokes
            .iter()
            .filter_map(|(old, stroke)| slots.get(old).map(|new| (*new, *stroke)))
            .collect();
        self.labels = self
            .labels
            .iter()
            .filter_map(|(old, font)| slots.get(old).map(|new| (*new, font.clone())))
            .collect();
        self.label_whitespace = self
            .label_whitespace
            .iter()
            .filter_map(|(old, mode)| slots.get(old).map(|new| (*new, *mode)))
            .collect();
    }
}
