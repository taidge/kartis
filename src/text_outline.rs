//! Explicit font-outline lowering for semantic Glory labels. It preserves the
//! original Scene and label slots, metadata, owner series and paint strata.
//! Text uses the raster font registry's writing mode and shaping profile
//! with application-supplied outline fonts.
use std::collections::BTreeMap;

use crate::render::raster::{
    FontRegistry, MissingGlyphPolicy, RasterPaintedText, RasterTextOutline,
};
use crate::scene::ensure;
use crate::{Result, Scene};
#[derive(Clone, Debug, PartialEq)]
pub struct PaintedLabelOptions {
    /// Convert every label by default. Set `true` to convert only rotated
    /// labels and retain zero-rotation labels as host-rendered Text.
    pub rotated_only: bool,
    pub missing_glyph: MissingGlyphPolicy,
    pub max_glyphs: usize,
    pub max_layers: usize,
    pub max_commands: usize,
}
impl Default for PaintedLabelOptions {
    fn default() -> Self {
        Self {
            rotated_only: false,
            missing_glyph: MissingGlyphPolicy::Error,
            max_glyphs: 20_000,
            max_layers: 20_000,
            max_commands: 200_000,
        }
    }
}
impl PaintedLabelOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            (1..=20_000).contains(&self.max_glyphs)
                && (1..=20_000).contains(&self.max_layers)
                && (1..=200_000).contains(&self.max_commands),
            "invalid painted-label glyph/layer/command budget",
        )
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintedLabels {
    pub labels: BTreeMap<usize, RasterPaintedText>,
    pub glyph_count: usize,
    pub layer_count: usize,
    pub command_count: usize,
}
/// Lower explicit solid-color glyph layers without discarding transparent
/// foreground labels whose font palette can still contain visible layers.
pub fn painted_labels(
    scene: &Scene,
    fonts: &FontRegistry,
    options: &PaintedLabelOptions,
) -> Result<PaintedLabels> {
    scene.validate()?;
    options.validate()?;
    let mut result = PaintedLabels::default();
    for (index, label) in scene.labels.iter().enumerate() {
        if options.rotated_only && label.rotation == 0. {
            continue;
        }
        let text = if scene.typography.has_label_font(index) {
            fonts.painted_label_with_font(
                label,
                &scene.resolved_label_font(index),
                options.missing_glyph,
            )?
        } else {
            fonts.painted_label(label, options.missing_glyph)?
        };
        result.glyph_count += text.glyph_count;
        ensure(
            result.glyph_count <= options.max_glyphs,
            "painted-label glyph budget exceeded",
        )?;
        for layer in &text.layers {
            ensure(
                layer.commands.len() <= glory::render::MAX_GRAPHIC_COMMANDS,
                "a complete font layer exceeds Glory Graphic's 256-command budget; use direct raster",
            )?;
            result.layer_count += 1;
            result.command_count += layer.commands.len();
            ensure(
                result.layer_count <= options.max_layers
                    && result.command_count <= options.max_commands,
                "painted-label layer/command budget exceeded",
            )?;
        }
        result.labels.insert(index, text);
    }
    Ok(result)
}
#[derive(Clone, Debug, PartialEq)]
pub struct OutlineLabelOptions {
    /// Leave zero-rotation labels on the existing semantic Text path.
    /// Set this to `false` to outline upright vertical labels.
    pub rotated_only: bool,
    pub missing_glyph: MissingGlyphPolicy,
    pub max_glyphs: usize,
    /// Budget for generated outline commands, separate from Scene mark paths.
    pub max_commands: usize,
}
impl Default for OutlineLabelOptions {
    fn default() -> Self {
        Self {
            rotated_only: true,
            missing_glyph: MissingGlyphPolicy::Error,
            max_glyphs: 20_000,
            max_commands: 200_000,
        }
    }
}
impl OutlineLabelOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            (1..=20_000).contains(&self.max_glyphs) && (1..=200_000).contains(&self.max_commands),
            "invalid outline-label glyph/command budget",
        )
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OutlinedLabels {
    pub labels: BTreeMap<usize, RasterTextOutline>,
    pub glyph_count: usize,
    pub command_count: usize,
}
pub fn outline_labels(
    scene: &Scene,
    fonts: &FontRegistry,
    options: &OutlineLabelOptions,
) -> Result<OutlinedLabels> {
    scene.validate()?;
    options.validate()?;
    let mut result = OutlinedLabels::default();
    for (index, label) in scene.labels.iter().enumerate() {
        if options.rotated_only && label.rotation == 0. {
            continue;
        }
        let outline = if label.color.3 == 0 {
            RasterTextOutline {
                glyphs: Vec::new(),
                advance: 0.,
            }
        } else if scene.typography.has_label_font(index) {
            fonts.outline_label_with_font(
                label,
                &scene.resolved_label_font(index),
                options.missing_glyph,
            )?
        } else {
            fonts.outline_label(label, options.missing_glyph)?
        };
        for glyph in &outline.glyphs {
            ensure(
                glyph.len() <= glory::render::MAX_GRAPHIC_COMMANDS,
                "a complete font glyph exceeds Glory Graphic's 256-command budget; use direct SVG/Canvas/raster",
            )?;
            result.glyph_count += 1;
            result.command_count += glyph.len();
            ensure(
                result.glyph_count <= options.max_glyphs
                    && result.command_count <= options.max_commands,
                "outline-label resource budget exceeded",
            )?;
        }
        result.labels.insert(index, outline);
    }
    Ok(result)
}
