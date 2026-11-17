//! Font-defined line metrics and real glyph advances for the shared wrapper.
use super::*;
use crate::{
    LayoutRect, ResolvedFont, SemanticTextMeasurement, SemanticTextMetrics, SemanticTextRequest,
    TextWrapOptions, WrappedLabelLines,
};

impl FontRegistry {
    fn wrap_measurement(
        &self,
        request: &SemanticTextRequest,
        missing: MissingGlyphPolicy,
        explicit_font: bool,
    ) -> Result<SemanticTextMeasurement> {
        ensure(
            self.writing_mode == RasterWritingMode::Horizontal,
            "automatic wrapping requires horizontal raster text",
        )?;
        request.validate()?;
        let selected = if explicit_font {
            self.for_font(&request.font)?
        } else {
            self.clone()
        };
        let layout = selected.measure_layout(
            &request.whitespace.normalize(&request.text),
            request.size,
            missing,
        )?;
        let primary = selected
            .fonts
            .first()
            .ok_or_else(|| ChartError("wrapping needs a registered font face".into()))?;
        let face = ttf_parser::Face::parse(&primary.bytes, primary.index)
            .map_err(|_| ChartError("invalid wrapping font face".into()))?;
        let scale = request.size / f64::from(face.units_per_em());
        let ascent = f64::from(face.ascender()) * scale;
        let descent = -f64::from(face.descender()) * scale;
        request.measured(SemanticTextMetrics {
            advance: layout.advance,
            line_height: request.line_height,
            baseline_from_top: ascent + (request.line_height - ascent - descent) / 2.,
            ink_bounds: layout.ink_bounds.map(|bounds| LayoutRect {
                x: bounds.x,
                y: bounds.y,
                width: bounds.width,
                height: bounds.height,
            }),
        })
    }

    /// Wrap using real shaped advances and the same bound faces used by rendering.
    /// Horizontal lines are reshaped independently, including explicit bidi mode.
    /// Metrics use the primary bound face's ascent/descent and requested leading;
    /// they are not browser/CSS line-box measurements. Vertical registries reject.
    pub fn wrap_label(
        &self,
        label: &Label,
        font: &ResolvedFont,
        options: &TextWrapOptions,
        missing: MissingGlyphPolicy,
    ) -> Result<WrappedLabelLines> {
        ensure(
            self.writing_mode == RasterWritingMode::Horizontal,
            "automatic wrapping requires horizontal raster text",
        )?;
        crate::wrap_label(label, font, options, &mut |request| {
            self.wrap_measurement(request, missing, true)
        })
    }

    /// Replace one label only after complete measurement, reindexing and validation.
    /// Ownership, typography, clip and explicit draw order follow every new line.
    pub fn wrap_scene_label(
        &self,
        scene: &mut Scene,
        index: usize,
        options: &TextWrapOptions,
        missing: MissingGlyphPolicy,
    ) -> Result<WrappedLabelLines> {
        ensure(
            self.writing_mode == RasterWritingMode::Horizontal,
            "automatic wrapping requires horizontal raster text",
        )?;
        let explicit_font = scene.typography.has_label_font(index);
        crate::wrap_scene_label(scene, index, options, &mut |request| {
            self.wrap_measurement(request, missing, explicit_font)
        })
    }
}
