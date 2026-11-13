//! Actual host line metrics for ordinary semantic Text.
//!
//! The host measures its real font, line box and alphabetic baseline. This
//! module only binds those results to their inputs and places them; it does not
//! infer font ascent, descent or glyph widths from the font size.
use crate::scene::ensure;
use crate::{Label, LayoutRect, MAX_LABEL_BYTES, ResolvedFont, Result, TextAnchor};

/// Resource identity and requested single-line line height. Bump `revision`
/// when a host font resource changes, even if its family and weight stay equal.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticTextProfile {
    /// Requested line height as a ratio of the label's font size.
    pub line_height: f64,
    pub revision: u64,
}

impl Default for SemanticTextProfile {
    fn default() -> Self {
        Self {
            line_height: 1.,
            revision: 0,
        }
    }
}

impl SemanticTextProfile {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.line_height.is_finite() && (0.5..=4.).contains(&self.line_height),
            "semantic text line-height ratio must be finite and in 0.5..=4",
        )
    }

    pub fn request(&self, label: &Label, font: &ResolvedFont) -> Result<SemanticTextRequest> {
        SemanticTextRequest::for_label(label, font, self)
    }
}

/// A complete measurement key. Anchor, position, paint and series ownership
/// deliberately stay outside this key, so moving the same text can reuse it.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticTextRequest {
    pub text: String,
    pub size: f64,
    pub font: ResolvedFont,
    /// Requested host line height in logical pixels, rather than a ratio.
    pub line_height: f64,
    pub revision: u64,
}

impl SemanticTextRequest {
    pub fn for_label(
        label: &Label,
        font: &ResolvedFont,
        profile: &SemanticTextProfile,
    ) -> Result<Self> {
        profile.validate()?;
        font.validate()?;
        ensure(
            label.text.len() <= MAX_LABEL_BYTES
                && label.size.is_finite()
                && (1. ..=256.).contains(&label.size),
            "invalid semantic text or font size",
        )?;
        let request = Self {
            text: label.text.clone(),
            size: label.size,
            font: font.clone(),
            line_height: label.size * profile.line_height,
            revision: profile.revision,
        };
        request.validate()?;
        Ok(request)
    }

    pub fn validate(&self) -> Result<()> {
        self.font.validate()?;
        ensure(
            self.text.len() <= MAX_LABEL_BYTES
                && self.size.is_finite()
                && (1. ..=256.).contains(&self.size),
            "invalid semantic text or font size",
        )?;
        ensure(
            self.line_height.is_finite()
                && (self.size * 0.5..=self.size * 4.).contains(&self.line_height),
            "invalid semantic text requested line height",
        )
    }

    /// Bind actual host measurements to these exact text/font/resource inputs.
    pub fn measured(&self, metrics: SemanticTextMetrics) -> Result<SemanticTextMeasurement> {
        self.validate()?;
        metrics.validate()?;
        Ok(SemanticTextMeasurement {
            request: self.clone(),
            metrics,
        })
    }
}

/// Measurements in logical pixels, before anchor adjustment and positioning.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticTextMetrics {
    pub advance: f64,
    /// Actual measured host line-box height. Hosts may quantize the requested
    /// line height, so this need not equal the request's CSS pixel value.
    pub line_height: f64,
    /// Alphabetic baseline measured from the top of the actual host line box.
    /// Negative leading can put this outside `0..line_height`.
    pub baseline_from_top: f64,
    /// Actual ink relative to physical Start and alphabetic baseline `(0, 0)`.
    /// Bearings may be negative; descenders may extend below the baseline.
    /// `None` represents no painted ink, including an empty or whitespace run.
    pub ink_bounds: Option<LayoutRect>,
}

impl SemanticTextMetrics {
    pub fn validate(&self) -> Result<()> {
        ensure(
            [self.advance, self.line_height]
                .into_iter()
                .all(|value| value.is_finite() && (0. ..=1_000_000.).contains(&value))
                && self.baseline_from_top.is_finite()
                && self.baseline_from_top.abs() <= 1_000_000.,
            "invalid semantic text host metrics",
        )?;
        if let Some(bounds) = self.ink_bounds {
            bounds.validate()?;
        }
        Ok(())
    }
}

/// A measurement retains its private input identity. Editing a request never
/// changes a previously accepted measurement or makes it current again.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticTextMeasurement {
    request: SemanticTextRequest,
    metrics: SemanticTextMetrics,
}

impl SemanticTextMeasurement {
    pub fn request(&self) -> &SemanticTextRequest {
        &self.request
    }

    pub fn metrics(&self) -> &SemanticTextMetrics {
        &self.metrics
    }

    pub fn validate_for(&self, request: &SemanticTextRequest) -> Result<&SemanticTextMetrics> {
        request.validate()?;
        ensure(
            self.request == *request,
            "stale semantic text measurement: text, font, size, line height or resource revision changed",
        )?;
        self.metrics.validate()?;
        Ok(&self.metrics)
    }

    /// Place a horizontal ordinary Text line at the label's alphabetic anchor.
    /// Rotated labels require the existing explicit outline/path renderer.
    pub fn place_label(&self, label: &Label) -> Result<SemanticTextPlacement> {
        self.request.validate()?;
        self.metrics.validate()?;
        ensure(
            self.request.text == label.text && self.request.size == label.size,
            "semantic text placement differs from measured text or size",
        )?;
        ensure(
            label.rotation == 0.
                && [label.x, label.y]
                    .into_iter()
                    .all(|coordinate| coordinate.is_finite() && coordinate.abs() <= 1_000_000.),
            "semantic text placement requires finite horizontal label coordinates",
        )?;
        let start = label.x
            - self.metrics.advance
                * match label.anchor {
                    TextAnchor::Start => 0.,
                    TextAnchor::Middle => 0.5,
                    TextAnchor::End => 1.,
                };
        let placement = SemanticTextPlacement {
            line_box: LayoutRect {
                x: start,
                y: label.y - self.metrics.baseline_from_top,
                width: self.metrics.advance,
                height: self.metrics.line_height,
            },
            ink_bounds: self.metrics.ink_bounds.map(|ink| LayoutRect {
                x: start + ink.x,
                y: label.y + ink.y,
                ..ink
            }),
            alphabetic_anchor: [label.x, label.y],
        };
        placement.validate()?;
        Ok(placement)
    }
}

/// Separate host line-box placement and actual collision ink placement.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemanticTextPlacement {
    pub line_box: LayoutRect,
    pub ink_bounds: Option<LayoutRect>,
    pub alphabetic_anchor: [f64; 2],
}

impl SemanticTextPlacement {
    pub fn validate(&self) -> Result<()> {
        self.line_box.validate()?;
        if let Some(ink) = self.ink_bounds {
            ink.validate()?;
        }
        ensure(
            self.alphabetic_anchor
                .into_iter()
                .all(|coordinate| coordinate.is_finite() && coordinate.abs() <= 1_000_000.),
            "invalid semantic text alphabetic anchor",
        )
    }

    pub fn translated(&self, dx: f64, dy: f64) -> Result<Self> {
        self.validate()?;
        ensure(
            dx.is_finite() && dy.is_finite(),
            "invalid semantic text displacement",
        )?;
        let translated = Self {
            line_box: LayoutRect {
                x: self.line_box.x + dx,
                y: self.line_box.y + dy,
                ..self.line_box
            },
            ink_bounds: self.ink_bounds.map(|ink| LayoutRect {
                x: ink.x + dx,
                y: ink.y + dy,
                ..ink
            }),
            alphabetic_anchor: [
                self.alphabetic_anchor[0] + dx,
                self.alphabetic_anchor[1] + dy,
            ],
        };
        translated.validate()?;
        Ok(translated)
    }

    /// Apply the collision solver's ink translation to the original line box.
    /// The solver must retain the measured ink size; it cannot reshape text.
    pub fn with_placed_ink_bounds(&self, bounds: LayoutRect) -> Result<Self> {
        bounds.validate()?;
        let ink = self.ink_bounds.ok_or_else(|| {
            crate::ChartError("semantic text without ink has no collision rectangle".into())
        })?;
        ensure(
            ink.width == bounds.width && ink.height == bounds.height,
            "semantic text placement cannot resize measured ink",
        )?;
        let mut translated = self.translated(bounds.x - ink.x, bounds.y - ink.y)?;
        translated.ink_bounds = Some(bounds);
        Ok(translated)
    }
}
