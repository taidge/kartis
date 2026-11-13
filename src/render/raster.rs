//! CPU Scene rasterization: tiny-skia paths and rustybuzz-shaped font outlines.
//! Fonts are application-supplied bytes, never implicit system files. Explicit
//! LTR runs support kerning, ligatures and combining marks. `FontRegistry`
//! also supports top-to-bottom OpenType shaping with explicit columns and
//! optional Unicode mixed glyph orientation. The `raster-bidi`
//! feature adds opt-in single-line horizontal bidirectional shaping.
#[cfg(feature = "raster-bidi")]
mod bidi;
mod blend_layer;
mod colr0;
mod font_selection;
mod text_runs;
mod vertical;
mod vertical_data;
use super::Canvas;
use crate::blend::{BlendMode, BlendPlan, MarkBlends};
use crate::scene::ensure;
use crate::{ChartError, ClipRect, Color, Label, Result, Scene, TextAnchor, ZoomTransform};
use base64::Engine as _;
#[cfg(feature = "raster-bidi")]
pub use bidi::BidiBaseDirection;
pub use font_selection::FaceDescriptor;
use glory::render::{GraphicStroke, StrokeCap};
use std::{collections::BTreeMap, sync::Arc};
use tiny_skia::{FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Rect, Stroke, Transform};
pub use vertical::{RasterGlyphOrientation, RasterVerticalColumns, RasterVerticalLayout};

pub const MAX_RASTER_PIXELS: u64 = 67_108_864;
pub const MAX_RASTER_OPERATIONS: usize = 1_000_000_000;
const MAX_COORDINATE: f64 = 16_777_216.;
const MAX_GLYPH_COMMANDS: usize = 8192;
const MAX_SHAPED_GLYPHS: usize = 8192;
const MAX_TEXT_PAINT_LAYERS: usize = 20_000;
const MAX_TEXT_PAINT_COMMANDS: usize = 200_000;
const MAX_PATTERN_CACHE_PIXELS: u64 = 16_777_216;
const MAX_PATTERN_CACHE_ENTRIES: usize = 1024;
const MAX_MASK_CACHE_PIXELS: u64 = 134_217_728;
const MAX_MASK_CACHE_ENTRIES: usize = 256;
const MAX_MASK_KEY_BYTES: usize = 8 * 1024 * 1024;
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClipKind {
    Rect,
    Path,
}
struct PhysicalClip {
    id: String,
    path: Option<Path>,
    rule: FillRule,
}
#[derive(Clone)]
struct ClipState {
    kind: ClipKind,
    rect: ClipRect,
    bounds: ClipRect,
    paths: Vec<Arc<PhysicalClip>>,
    key: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MissingGlyphPolicy {
    #[default]
    Error,
    Skip,
    Notdef,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RasterOptions {
    pub pixel_ratio: f64,
    pub view: ZoomTransform,
    pub max_pixels: u64,
    /// Approximate work units: path commands plus visited pixel bounding areas,
    /// mask allocations and shaped codepoints. Enforced before each paint call.
    pub max_operations: usize,
    pub missing_glyph: MissingGlyphPolicy,
}
impl Default for RasterOptions {
    fn default() -> Self {
        Self {
            pixel_ratio: 1.,
            view: ZoomTransform::default(),
            max_pixels: MAX_RASTER_PIXELS,
            max_operations: 250_000_000,
            missing_glyph: MissingGlyphPolicy::Error,
        }
    }
}
impl RasterOptions {
    pub fn validate(&self) -> Result<()> {
        self.view.validate()?;
        ensure(
            self.pixel_ratio.is_finite() && (0.25..=8.).contains(&self.pixel_ratio),
            "raster DPR must be in 0.25..=8",
        )?;
        ensure(
            self.max_pixels > 0
                && self.max_pixels <= MAX_RASTER_PIXELS
                && self.max_operations > 0
                && self.max_operations <= MAX_RASTER_OPERATIONS,
            "invalid raster pixel/operation budget",
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontId(pub usize);
#[derive(Clone, Debug)]
struct Font {
    bytes: Arc<[u8]>,
    index: u32,
}
/// Inline direction for native raster text and explicit glyph outlines.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RasterWritingMode {
    #[default]
    Horizontal,
    /// Shape top to bottom using the configured vertical layout.
    Vertical,
}
/// Explicit color-font selection. Fonts remain application-supplied bytes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ColorGlyphMode {
    #[default]
    Reject,
    /// OpenType COLR version 0 solid layers with the selected CPAL palette.
    /// Palette alpha is preserved. The special foreground entry uses the full
    /// label color, including alpha. Palette layers can therefore remain
    /// visible when the foreground is transparent.
    ColrV0 { palette: u16 },
}
#[derive(Clone, Debug, Default)]
pub struct FontRegistry {
    fonts: Vec<Font>,
    bindings: Vec<(FontId, FaceDescriptor)>,
    bytes: usize,
    writing_mode: RasterWritingMode,
    vertical_layout: RasterVerticalLayout,
    color_glyphs: ColorGlyphMode,
    #[cfg(feature = "raster-bidi")]
    bidi: Option<BidiBaseDirection>,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterTextMetrics {
    /// Inline advance in scene units along the configured writing direction.
    pub advance: f64,
    pub glyph_count: usize,
}
/// Axis-aligned bounds in label-local scene coordinates before label rotation.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RasterTextBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
/// One vertical column measured from its top-center origin.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterTextColumn {
    pub x: f64,
    pub advance: f64,
}
/// Detailed layout before anchoring or label rotation.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterTextLayoutMetrics {
    /// Maximum column advance for vertical text, horizontal advance otherwise.
    pub advance: f64,
    pub glyph_count: usize,
    /// Vertical columns in source order, including explicit empty columns.
    /// Horizontal layout leaves this empty.
    pub columns: Vec<RasterTextColumn>,
    /// Nominal em cells and their advances. Ink can extend beyond these cells.
    pub advance_bounds: RasterTextBounds,
    /// Union of visible glyph-layer bounding boxes after individual orientation.
    /// This measurement uses an opaque foreground color.
    pub ink_bounds: Option<RasterTextBounds>,
}
/// Complete shaped glyph paths in scene coordinates, including label anchor
/// and clockwise rotation around the anchor point. Each glyph preserves all
/// of its contours together so winding-rule holes survive path lowering.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterTextOutline {
    pub glyphs: Vec<Vec<glory::render::PathCommand>>,
    pub advance: f64,
}
/// One complete monochrome or COLR solid layer in scene coordinates.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterGlyphLayer {
    /// Source-order shaped glyph index, including preceding whitespace glyphs.
    pub glyph: usize,
    /// Paint-order layer index within the glyph.
    pub layer: usize,
    pub commands: Vec<glory::render::PathCommand>,
    pub color: Color,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RasterPaintedText {
    pub layers: Vec<RasterGlyphLayer>,
    pub advance: f64,
    /// Shaped glyph count, including glyphs with empty outlines.
    pub glyph_count: usize,
}
struct Glyph {
    font: usize,
    id: ttf_parser::GlyphId,
    #[cfg(test)]
    cluster: u32,
    x: f64,
    y: f64,
    size: f64,
    sideways: bool,
}
impl Glyph {
    fn point(&self, x: f32, y: f32, anchor: (f64, f64)) -> (f64, f64) {
        let (x, y) = if self.sideways { (y, x) } else { (x, -y) };
        (
            anchor.0 + self.x + f64::from(x) * self.size,
            anchor.1 + self.y + f64::from(y) * self.size,
        )
    }
}
struct Text {
    glyphs: Vec<Glyph>,
    advance: f64,
    columns: Vec<RasterTextColumn>,
}

struct LabelTransform {
    origin: (f64, f64),
    anchor: (f64, f64),
    sin: f64,
    cos: f64,
}

impl LabelTransform {
    fn new(label: &Label, writing_mode: RasterWritingMode, advance: f64) -> Self {
        let inline_anchor = match label.anchor {
            TextAnchor::Start => 0.,
            TextAnchor::Middle => -advance / 2.,
            TextAnchor::End => -advance,
        };
        let anchor = match writing_mode {
            RasterWritingMode::Horizontal => (inline_anchor, 0.),
            RasterWritingMode::Vertical => (0., inline_anchor),
        };
        let (sin, cos) = label.rotation.to_radians().sin_cos();
        Self {
            origin: (label.x, label.y),
            anchor,
            sin,
            cos,
        }
    }

    fn point(&self, glyph: &Glyph, x: f32, y: f32) -> (f64, f64) {
        let (x, y) = glyph.point(x, y, self.anchor);
        (
            self.origin.0 + x * self.cos - y * self.sin,
            self.origin.1 + x * self.sin + y * self.cos,
        )
    }
}
fn validate_label_transform(label: &Label) -> Result<()> {
    ensure(
        label.x.is_finite()
            && label.y.is_finite()
            && label.x.abs() <= 1_000_000.
            && label.y.abs() <= 1_000_000.
            && label.rotation.is_finite()
            && (-360. ..=360.).contains(&label.rotation),
        "invalid outline label transform",
    )
}
impl FontRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    /// Set the writing mode for measurement, CPU raster and explicit outlines.
    ///
    /// Vertical mode uses the font's OpenType vertical substitutions, origins
    /// and advances. Start and End anchor the top and bottom of the advance.
    /// Label rotation remains clockwise around the anchor point. The default
    /// layout uses upright glyphs in one logical-order column and maps line
    /// separators to spaces. `with_vertical_layout` configures explicit columns
    /// and Unicode mixed orientation. Directional formatting controls are
    /// unsupported in vertical mode.
    /// Fallback preserves complete graphemes and uses one face per script span
    /// for contextual scripts such as Mongolian or Arabic.
    pub fn with_writing_mode(mut self, mode: RasterWritingMode) -> Self {
        self.writing_mode = mode;
        self
    }
    pub fn writing_mode(&self) -> RasterWritingMode {
        self.writing_mode
    }
    /// Configure vertical columns and glyph orientation and select vertical mode.
    /// Invalid numeric options are rejected when measuring or rendering.
    pub fn with_vertical_layout(mut self, layout: RasterVerticalLayout) -> Self {
        self.vertical_layout = layout;
        self.writing_mode = RasterWritingMode::Vertical;
        self
    }
    pub fn vertical_layout(&self) -> RasterVerticalLayout {
        self.vertical_layout
    }
    pub fn with_color_glyphs(mut self, mode: ColorGlyphMode) -> Self {
        self.color_glyphs = mode;
        self
    }
    pub fn color_glyphs(&self) -> ColorGlyphMode {
        self.color_glyphs
    }
    /// Use horizontal single-line bidirectional ordering with explicit font bytes.
    /// Start and End anchors remain the physical left and right of the advance.
    ///
    /// Line separators and supported whitespace controls become spaces.
    /// Directional controls affect order without producing visible glyphs.
    /// Fallback uses complete graphemes for Latin, Greek, Cyrillic, Hebrew and
    /// common text. Other scripts, including Arabic, require one face per
    /// directional script span to preserve contextual forms. When coverage
    /// fails, the missing-glyph policy applies to the entire fallback unit.
    #[cfg(feature = "raster-bidi")]
    pub fn with_bidi(mut self, base_direction: BidiBaseDirection) -> Self {
        self.bidi = Some(base_direction);
        self
    }
    pub fn len(&self) -> usize {
        self.fonts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
    /// Register an unmodified TTF/OTF/TTC face, in deterministic fallback order.
    /// At most 32 faces, 16MiB each and 64MiB aggregate; rejection is atomic.
    pub fn register(&mut self, bytes: impl Into<Arc<[u8]>>, face_index: u32) -> Result<FontId> {
        let bytes = bytes.into();
        ensure(
            self.fonts.len() < 32
                && bytes.len() <= 16 * 1024 * 1024
                && self.bytes + bytes.len() <= 64 * 1024 * 1024,
            "raster font byte/count budget exceeded",
        )?;
        ttf_parser::Face::parse(&bytes, face_index)
            .map_err(|_| ChartError("invalid raster font face".into()))?;
        ensure(
            rustybuzz::Face::from_slice(&bytes, face_index).is_some(),
            "font cannot be shaped",
        )?;
        let id = FontId(self.fonts.len());
        self.bytes += bytes.len();
        self.fonts.push(Font {
            bytes,
            index: face_index,
        });
        Ok(id)
    }
    pub fn measure(
        &self,
        text: &str,
        size: f64,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextMetrics> {
        let text = self.shape(text, size, missing)?;
        Ok(RasterTextMetrics {
            advance: text.advance,
            glyph_count: text.glyphs.len(),
        })
    }
    /// Measure the same glyph placement used by CPU and outline rendering.
    /// Bounds are before the label's anchor and clockwise rotation. Vertical
    /// labels anchor the complete column group using its maximum inline advance.
    pub fn measure_layout(
        &self,
        text: &str,
        size: f64,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextLayoutMetrics> {
        let text = self.shape(text, size, missing)?;
        let advance_bounds = if text.glyphs.is_empty() && text.columns.is_empty() {
            RasterTextBounds::default()
        } else if self.writing_mode == RasterWritingMode::Vertical {
            let first = text.columns.first().unwrap().x;
            let last = text.columns.last().unwrap().x;
            RasterTextBounds {
                x: first.min(last) - size / 2.,
                y: 0.,
                width: (last - first).abs() + size,
                height: text.advance,
            }
        } else {
            RasterTextBounds {
                x: text.advance.min(0.),
                y: -size,
                width: text.advance.abs(),
                height: size,
            }
        };
        let ink_bounds = self.glyph_bounds(&text, None, Color(0, 0, 0, 255))?;
        Ok(RasterTextLayoutMetrics {
            advance: text.advance,
            glyph_count: text.glyphs.len(),
            columns: text.columns,
            advance_bounds,
            ink_bounds,
        })
    }
    /// Measure a conservative glyph-ink AABB in scene coordinates, including
    /// bearings, inline anchor, per-glyph orientation and label rotation.
    /// Whitespace and empty labels return `None`. Glyph boxes are transformed
    /// through the same placement as their complete CPU and vector outlines.
    pub fn measure_label(
        &self,
        label: &Label,
        missing: MissingGlyphPolicy,
    ) -> Result<Option<RasterTextBounds>> {
        validate_label_transform(label)?;
        let text = self.shape(&label.text, label.size, missing)?;
        let transform = LabelTransform::new(label, self.writing_mode, text.advance);
        self.glyph_bounds(&text, Some(&transform), label.color)
    }
    fn glyph_bounds(
        &self,
        text: &Text,
        transform: Option<&LabelTransform>,
        foreground: Color,
    ) -> Result<Option<RasterTextBounds>> {
        let mut extents: Option<(f64, f64, f64, f64)> = None;
        let limit = if transform.is_some() {
            1_000_000.
        } else {
            MAX_COORDINATE
        };
        for (index, layer) in self.glyph_layers(text, foreground, self.color_glyphs)? {
            if layer.color.3 != 0 || self.color_glyphs == ColorGlyphMode::Reject {
                let glyph = &text.glyphs[index];
                let bounds = layer.bounds;
                for (x, y) in [
                    (bounds.x_min, bounds.y_min),
                    (bounds.x_max, bounds.y_min),
                    (bounds.x_min, bounds.y_max),
                    (bounds.x_max, bounds.y_max),
                ] {
                    let (x, y) = match transform {
                        Some(transform) => transform.point(glyph, f32::from(x), f32::from(y)),
                        None => glyph.point(f32::from(x), f32::from(y), (0., 0.)),
                    };
                    ensure(
                        x.is_finite() && y.is_finite() && x.abs() <= limit && y.abs() <= limit,
                        "raster text bounds overflow",
                    )?;
                    extents = Some(match extents {
                        None => (x, y, x, y),
                        Some((left, top, right, bottom)) => {
                            (left.min(x), top.min(y), right.max(x), bottom.max(y))
                        }
                    });
                }
            }
        }
        Ok(extents.map(|(x, y, right, bottom)| RasterTextBounds {
            x,
            y,
            width: right - x,
            height: bottom - y,
        }))
    }
    /// Lower monochrome outlines. Colored glyphs require `painted_label`,
    /// which retains each font layer's actual color and alpha.
    pub fn outline_label(
        &self,
        label: &Label,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterTextOutline> {
        let painted = self.painted_label_mode(label, missing, ColorGlyphMode::Reject)?;
        Ok(RasterTextOutline {
            glyphs: painted
                .layers
                .into_iter()
                .map(|layer| layer.commands)
                .collect(),
            advance: painted.advance,
        })
    }
    /// Lower complete solid-color layers in shaped glyph and font paint order.
    /// Every layer retains all contours together. Transparent foreground color
    /// affects the foreground layer while palette layers retain their CPAL alpha.
    pub fn painted_label(
        &self,
        label: &Label,
        missing: MissingGlyphPolicy,
    ) -> Result<RasterPaintedText> {
        self.painted_label_mode(label, missing, self.color_glyphs)
    }
    fn glyph_layers(
        &self,
        text: &Text,
        foreground: Color,
        mode: ColorGlyphMode,
    ) -> Result<Vec<(usize, colr0::Layer)>> {
        let mut result = Vec::new();
        let mut layers_seen = 0;
        let mut commands_seen = 0;
        for (index, glyph) in text.glyphs.iter().enumerate() {
            let font = &self.fonts[glyph.font];
            let face = ttf_parser::Face::parse(&font.bytes, font.index).unwrap();
            let (count, layers) = colr0::layers(
                &face,
                glyph.id,
                mode,
                foreground,
                MAX_TEXT_PAINT_LAYERS - layers_seen,
                MAX_TEXT_PAINT_COMMANDS - commands_seen,
            )?;
            layers_seen += count;
            for layer in layers {
                commands_seen += layer.outline.commands.len();
                result.push((index, layer));
            }
        }
        Ok(result)
    }
    fn painted_label_mode(
        &self,
        label: &Label,
        missing: MissingGlyphPolicy,
        mode: ColorGlyphMode,
    ) -> Result<RasterPaintedText> {
        use glory::render::{GraphicPoint as P, PathCommand as C};
        validate_label_transform(label)?;
        let text = self.shape(&label.text, label.size, missing)?;
        let transform = LabelTransform::new(label, self.writing_mode, text.advance);
        let mut layers = Vec::new();
        for (index, layer) in self.glyph_layers(&text, label.color, mode)? {
            let glyph = &text.glyphs[index];
            let point = |x: f32, y: f32| -> Result<P> {
                let (x, y) = transform.point(glyph, x, y);
                let p = P::new(x, y);
                ensure(
                    p.x.is_finite()
                        && p.y.is_finite()
                        && p.x.abs() <= 1_000_000.
                        && p.y.abs() <= 1_000_000.,
                    "outlined glyph exceeds scene coordinate budget",
                )?;
                Ok(p)
            };
            let mut commands = Vec::new();
            let mut current = None;
            for command in layer.outline.commands {
                match command {
                    OutlineCommand::Move(x, y) => {
                        let p = point(x, y)?;
                        commands.push(C::Move(p));
                        current = Some(p);
                    }
                    OutlineCommand::Line(x, y) => {
                        let p = point(x, y)?;
                        commands.push(C::Line(p));
                        current = Some(p);
                    }
                    OutlineCommand::Quad(x, y, a, b) => {
                        let p = current.ok_or_else(|| {
                            ChartError("quadratic outline without current point".into())
                        })?;
                        let q = point(x, y)?;
                        let end = point(a, b)?;
                        commands.push(C::Cubic {
                            first: P::new(p.x + (q.x - p.x) * 2. / 3., p.y + (q.y - p.y) * 2. / 3.),
                            second: P::new(
                                end.x + (q.x - end.x) * 2. / 3.,
                                end.y + (q.y - end.y) * 2. / 3.,
                            ),
                            end,
                        });
                        current = Some(end);
                    }
                    OutlineCommand::Cubic(x, y, a, b, c, d) => {
                        let end = point(c, d)?;
                        commands.push(C::Cubic {
                            first: point(x, y)?,
                            second: point(a, b)?,
                            end,
                        });
                        current = Some(end);
                    }
                    OutlineCommand::Close => {
                        commands.push(C::Close);
                        current = None;
                    }
                }
            }
            if !commands.is_empty() {
                layers.push(RasterGlyphLayer {
                    glyph: index,
                    layer: layer.index,
                    commands,
                    color: layer.color,
                });
            }
        }
        Ok(RasterPaintedText {
            layers,
            advance: text.advance,
            glyph_count: text.glyphs.len(),
        })
    }
    fn shape(&self, text: &str, size: f64, missing: MissingGlyphPolicy) -> Result<Text> {
        ensure(
            text.len() <= crate::MAX_LABEL_BYTES
                && size.is_finite()
                && (1. ..=256.).contains(&size),
            "invalid raster text size/byte budget",
        )?;
        if self.writing_mode == RasterWritingMode::Vertical {
            self.vertical_layout.validate()?;
        }
        if text.is_empty() {
            return Ok(Text {
                glyphs: vec![],
                advance: 0.,
                columns: Vec::new(),
            });
        }
        ensure(
            !self.fonts.is_empty(),
            "raster text requires explicitly registered font bytes",
        )?;
        let faces = self
            .fonts
            .iter()
            .map(|font| rustybuzz::Face::from_slice(&font.bytes, font.index).unwrap())
            .collect::<Vec<_>>();
        if self.writing_mode == RasterWritingMode::Vertical {
            return vertical::shape(text, size, missing, &faces, self.vertical_layout);
        }
        #[cfg(feature = "raster-bidi")]
        if let Some(base_direction) = self.bidi {
            return bidi::shape(text, size, missing, base_direction, &faces);
        }
        let mut runs: Vec<(usize, String)> = vec![];
        for mut c in text.chars() {
            if matches!(c, '\n' | '\r' | '\t' | '\u{c}') {
                c = ' ';
            }
            ensure(
                !c.is_control()
                    && !matches!(c as u32,0x0590..=0x08ff|0x200e..=0x200f|0x202a..=0x202e|0x2066..=0x2069|0xfb1d..=0xfdff|0xfe70..=0xfeff|0x10800..=0x10fff|0x1e800..=0x1eeff),
                "raster text requires explicit LTR content; bidi/RTL text is unsupported",
            )?;
            let font = faces.iter().position(|face| face.glyph_index(c).is_some());
            let font = match font {
                Some(i) => i,
                None => match missing {
                    MissingGlyphPolicy::Error => {
                        return Err(ChartError(format!(
                            "no registered font contains U+{:04X}",
                            c as u32
                        )));
                    }
                    MissingGlyphPolicy::Skip => continue,
                    MissingGlyphPolicy::Notdef => 0,
                },
            };
            if let Some((last, run)) = runs.last_mut()
                && *last == font
            {
                run.push(c);
            } else {
                runs.push((font, c.to_string()));
            }
        }
        let mut glyphs = vec![];
        let mut width = 0.;
        for (font, run) in runs {
            let face = &faces[font];
            let scale = size / f64::from(face.units_per_em());
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(&run);
            buffer.guess_segment_properties();
            buffer.set_direction(rustybuzz::Direction::LeftToRight);
            let shaped = rustybuzz::shape(face, &[], buffer);
            ensure(
                glyphs.len() + shaped.len() <= MAX_SHAPED_GLYPHS,
                "shaped glyph budget exceeded",
            )?;
            for (info, p) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                glyphs.push(Glyph {
                    font,
                    id: ttf_parser::GlyphId(info.glyph_id as u16),
                    #[cfg(test)]
                    cluster: info.cluster,
                    x: width + f64::from(p.x_offset) * scale,
                    y: -f64::from(p.y_offset) * scale,
                    size: scale,
                    sideways: false,
                });
                width += f64::from(p.x_advance) * scale;
            }
        }
        ensure(
            width.is_finite() && width.abs() <= MAX_COORDINATE,
            "raster text advance overflow",
        )?;
        Ok(Text {
            glyphs,
            advance: width,
            columns: Vec::new(),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RenderedImage {
    pub width: u32,
    pub height: u32,
    pub logical_width: f64,
    pub logical_height: f64,
    pub pixel_ratio: f64,
    /// Straight (unpremultiplied), row-major RGBA8, without row padding.
    pub rgba: Vec<u8>,
}
impl RenderedImage {
    pub fn png(&self) -> Result<Vec<u8>> {
        ensure(
            u64::from(self.width) * u64::from(self.height) <= MAX_RASTER_PIXELS
                && self.width > 0
                && self.height > 0
                && self.rgba.len() == self.width as usize * self.height as usize * 4,
            "invalid raster image buffer",
        )?;
        let mut bytes = vec![];
        {
            let mut encoder = png::Encoder::new(&mut bytes, self.width, self.height);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder
                .write_header()
                .map_err(|e| ChartError(format!("PNG header: {e}")))?;
            writer
                .write_image_data(&self.rgba)
                .map_err(|e| ChartError(format!("PNG pixels: {e}")))?;
        }
        Ok(bytes)
    }
    pub fn data_url(&self) -> Result<String> {
        Ok(format!(
            "data:image/png;base64,{}",
            base64::engine::general_purpose::STANDARD.encode(self.png()?)
        ))
    }
}
struct Pending {
    pixels: Pixmap,
    logical_width: f64,
    logical_height: f64,
    path: PathBuilder,
    clips: Vec<ClipState>,
    masks: BTreeMap<String, Arc<Mask>>,
    mask_pixels: u64,
    mask_key_bytes: usize,
    patterns: BTreeMap<String, Arc<crate::PatternRasterTile>>,
    pattern_pixels: u64,
    used: usize,
    blend: Option<blend_layer::Layers>,
}
/// Canvas adapter retaining the last successfully completed frame. A failed
/// primitive discards its staged frame at finish; render() is transactional.
pub struct RasterCanvas {
    fonts: FontRegistry,
    options: RasterOptions,
    frame: Option<RenderedImage>,
    pending: Option<Pending>,
    failed: bool,
    prepared_blend: Option<blend_layer::Layers>,
}
impl RasterCanvas {
    pub fn new(fonts: FontRegistry, options: RasterOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            fonts,
            options,
            frame: None,
            pending: None,
            failed: false,
            prepared_blend: None,
        })
    }
    pub fn frame(&self) -> Option<&RenderedImage> {
        self.frame.as_ref()
    }
    pub fn options(&self) -> &RasterOptions {
        &self.options
    }
    pub fn set_options(&mut self, options: RasterOptions) -> Result<()> {
        options.validate()?;
        ensure(
            self.pending.is_none(),
            "cannot replace raster options during a frame",
        )?;
        self.options = options;
        Ok(())
    }
    pub fn render(&mut self, scene: &Scene) -> Result<&RenderedImage> {
        self.render_with_blends(scene, &MarkBlends::default())
    }
    pub fn render_with_blends(
        &mut self,
        scene: &Scene,
        blends: &MarkBlends,
    ) -> Result<&RenderedImage> {
        super::draw_with_blends(scene, self, blends)?;
        self.frame
            .as_ref()
            .ok_or_else(|| ChartError("raster frame not committed".into()))
    }
    fn fail<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.failed = true;
        }
        result
    }
    fn charge(&mut self, n: usize) -> Result<()> {
        let p = self
            .pending
            .as_mut()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?;
        p.used = p
            .used
            .checked_add(n)
            .ok_or_else(|| ChartError("raster operation budget overflow".into()))?;
        ensure(
            p.used <= self.options.max_operations,
            "raster operation budget exceeded",
        )
    }
    fn point(&self, x: f64, y: f64) -> Result<(f32, f32)> {
        let v = self.options.view;
        let ratio = self.options.pixel_ratio;
        let p = (
            (x * v.scale + v.translate_x) * ratio,
            (y * v.scale + v.translate_y) * ratio,
        );
        ensure(
            p.0.is_finite()
                && p.1.is_finite()
                && p.0.abs() <= MAX_COORDINATE
                && p.1.abs() <= MAX_COORDINATE,
            "raster transformed coordinate exceeds +/-16777216 physical pixels",
        )?;
        Ok((p.0 as f32, p.1 as f32))
    }
    fn viewport(&self) -> Result<ClipRect> {
        let p = self
            .pending
            .as_ref()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?;
        Ok(ClipRect {
            x: 0.,
            y: 0.,
            width: f64::from(p.pixels.width()),
            height: f64::from(p.pixels.height()),
        })
    }
    fn push_state(
        &mut self,
        kind: ClipKind,
        rect: ClipRect,
        bounds: ClipRect,
        paths: Vec<Arc<PhysicalClip>>,
    ) -> Result<()> {
        use std::fmt::Write;
        let p = self
            .pending
            .as_ref()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?;
        ensure(p.clips.len() < 16, "raster clip nesting budget exceeded")?;
        let bytes = 65 + paths.iter().map(|p| p.id.len() + 1).sum::<usize>();
        ensure(
            bytes + p.clips.iter().map(|p| p.key.len()).sum::<usize>() <= MAX_MASK_KEY_BYTES,
            "raster clip key budget exceeded",
        )?;
        self.charge(bytes)?;
        let mut key = String::with_capacity(bytes);
        key.push('r');
        for v in [rect.x, rect.y, rect.width, rect.height] {
            write!(key, "{:016x}", if v == 0. { 0 } else { v.to_bits() }).unwrap();
        }
        for path in &paths {
            key.push(';');
            key.push_str(&path.id);
        }
        self.pending.as_mut().unwrap().clips.push(ClipState {
            kind,
            rect,
            bounds,
            paths,
            key,
        });
        Ok(())
    }
    fn active_mask(&mut self) -> Result<Option<Arc<Mask>>> {
        let Some(state) = self.pending.as_ref().and_then(|p| p.clips.last()).cloned() else {
            return Ok(None);
        };
        if let Some(mask) = self.pending.as_ref().unwrap().masks.get(&state.key) {
            return Ok(Some(mask.clone()));
        }
        let (w, h) = (p_width(self), p_height(self));
        let pixels = u64::from(w) * u64::from(h);
        let p = self.pending.as_ref().unwrap();
        ensure(
            p.masks.len() < MAX_MASK_CACHE_ENTRIES
                && p.mask_pixels + pixels <= MAX_MASK_CACHE_PIXELS
                && p.mask_key_bytes + state.key.len() <= MAX_MASK_KEY_BYTES,
            "raster clip mask cache budget exceeded",
        )?;
        // Each intersection allocates/clears a temporary mask, fills it, then
        // multiplies its coverage. Include those full-frame visits up front.
        let visits = state.paths.len().max(1) * 3 + 2;
        self.charge(
            (pixels as usize)
                .checked_mul(visits)
                .ok_or_else(|| ChartError("raster mask work overflow".into()))?,
        )?;
        let mut mask = Mask::new(w, h)
            .ok_or_else(|| ChartError("raster clip mask allocation failed".into()))?;
        if state.paths.is_empty() {
            if let Some(rect) = Rect::from_xywh(
                state.rect.x as f32,
                state.rect.y as f32,
                state.rect.width as f32,
                state.rect.height as f32,
            ) {
                mask.fill_path(
                    &PathBuilder::from_rect(rect),
                    FillRule::Winding,
                    true,
                    Transform::identity(),
                );
            }
        } else {
            for (i, clip) in state.paths.iter().enumerate() {
                let Some(path) = &clip.path else {
                    mask.clear();
                    break;
                };
                if i == 0 {
                    mask.fill_path(path, clip.rule, true, Transform::identity());
                } else {
                    mask.intersect_path(path, clip.rule, true, Transform::identity());
                }
            }
            // Rectangle and path clips intersect as alpha masks. Do not stroke
            // the clipping boundary or multiply a path's own tight bounds into
            // its antialias coverage a second time.
            for (i, value) in mask.data_mut().iter_mut().enumerate() {
                let x = (i % w as usize) as f64;
                let y = (i / w as usize) as f64;
                let cx = ((x + 1.).min(state.rect.x + state.rect.width) - x.max(state.rect.x))
                    .clamp(0., 1.);
                let cy = ((y + 1.).min(state.rect.y + state.rect.height) - y.max(state.rect.y))
                    .clamp(0., 1.);
                *value = (f64::from(*value) * cx * cy).round() as u8;
            }
        }
        let mask = Arc::new(mask);
        let p = self.pending.as_mut().unwrap();
        p.mask_pixels += pixels;
        p.mask_key_bytes += state.key.len();
        p.masks.insert(state.key, mask.clone());
        Ok(Some(mask))
    }
    fn pop_kind(&mut self, kind: ClipKind) -> Result<()> {
        let result = (|| {
            self.charge(1)?;
            ensure(
                self.pending
                    .as_ref()
                    .unwrap()
                    .clips
                    .last()
                    .is_some_and(|c| c.kind == kind),
                "raster clip stack underflow or mismatched pop",
            )?;
            self.pending.as_mut().unwrap().clips.pop();
            Ok(())
        })();
        self.fail(result)
    }
    fn pattern_tile(
        &mut self,
        pattern: &crate::PatternFill,
    ) -> Result<Arc<crate::PatternRasterTile>> {
        let id = pattern.deterministic_id();
        if let Some(tile) = self
            .pending
            .as_ref()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?
            .patterns
            .get(&id)
        {
            return Ok(tile.clone());
        }
        let density = self.options.pixel_ratio * self.options.view.scale;
        let [w, h] = pattern.tile_pixels(density)?;
        let count = u64::from(w) * u64::from(h);
        ensure(
            self.pending.as_ref().unwrap().pattern_pixels + count <= MAX_PATTERN_CACHE_PIXELS
                && self.pending.as_ref().unwrap().patterns.len() < MAX_PATTERN_CACHE_ENTRIES,
            "raster pattern cache exceeds 16M tile pixels",
        )?;
        self.charge(pattern.tile_operation_cost(density)?)?;
        let tile = Arc::new(pattern.raster_tile(density)?);
        let p = self.pending.as_mut().unwrap();
        p.pattern_pixels += count;
        p.patterns.insert(id, tile.clone());
        Ok(tile)
    }
    fn paint(
        &mut self,
        path: &Path,
        color: Color,
        opacity: f64,
        stroke: Option<GraphicStroke>,
        fill_paint: Option<&crate::FillPaint>,
    ) -> Result<()> {
        ensure(
            opacity.is_finite() && (0. ..=1.).contains(&opacity),
            "invalid raster paint opacity",
        )?;
        let tile = if let Some(crate::FillPaint::Pattern(pattern)) = fill_paint {
            Some(self.pattern_tile(pattern)?)
        } else {
            None
        };
        let mut paint = Paint::default();
        paint.set_color(
            tiny_skia::Color::from_rgba(
                f32::from(color.0) / 255.,
                f32::from(color.1) / 255.,
                f32::from(color.2) / 255.,
                f32::from(color.3) / 255. * opacity as f32,
            )
            .unwrap(),
        );
        if let Some(fill) = fill_paint {
            let stops = fill
                .stops()
                .iter()
                .map(|s| {
                    let Color(r, g, b, a) = s.color;
                    tiny_skia::GradientStop::new(
                        s.offset as f32,
                        tiny_skia::Color::from_rgba8(r, g, b, a),
                    )
                })
                .collect();
            let shader = match fill {
                crate::FillPaint::Pattern(_) => {
                    let tile = tile.as_ref().unwrap();
                    let [a, b, c, d, e, f] = tile.transform;
                    // Equivalent periodic phase near the origin avoids f32
                    // precision loss for large but legal logical offsets.
                    let sx = tile.logical_size[0] / f64::from(tile.pixmap.width());
                    let (cos, sin) = (a / sx, b / sx);
                    let x = (cos * e + sin * f).rem_euclid(tile.logical_size[0]);
                    let y = (-sin * e + cos * f).rem_euclid(tile.logical_size[1]);
                    let (e, f) = (cos * x - sin * y, sin * x + cos * y);
                    let view = self.options.view;
                    let ratio = self.options.pixel_ratio;
                    let scale = view.scale * ratio;
                    let values = [
                        a * scale,
                        b * scale,
                        c * scale,
                        d * scale,
                        (e * view.scale + view.translate_x) * ratio,
                        (f * view.scale + view.translate_y) * ratio,
                    ];
                    ensure(
                        values
                            .iter()
                            .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE),
                        "raster pattern transform exceeds physical coordinate budget",
                    )?;
                    Some(tiny_skia::Pattern::new(
                        tile.pixmap.as_ref(),
                        tiny_skia::SpreadMode::Repeat,
                        tiny_skia::FilterQuality::Bilinear,
                        1.,
                        Transform::from_row(
                            values[0] as f32,
                            values[1] as f32,
                            values[2] as f32,
                            values[3] as f32,
                            values[4] as f32,
                            values[5] as f32,
                        ),
                    ))
                }
                crate::FillPaint::Linear { start, end, .. } => {
                    let a = self.point(start[0], start[1])?;
                    let b = self.point(end[0], end[1])?;
                    tiny_skia::LinearGradient::new(
                        tiny_skia::Point::from_xy(a.0, a.1),
                        tiny_skia::Point::from_xy(b.0, b.1),
                        stops,
                        tiny_skia::SpreadMode::Pad,
                        Transform::identity(),
                    )
                }
                crate::FillPaint::Radial { center, radius, .. } => {
                    let a = self.point(center[0], center[1])?;
                    let radius = radius * self.options.view.scale * self.options.pixel_ratio;
                    ensure(
                        radius.is_finite() && radius > 0. && radius <= MAX_COORDINATE,
                        "raster gradient radius exceeds physical budget",
                    )?;
                    tiny_skia::RadialGradient::new(
                        tiny_skia::Point::from_xy(a.0, a.1),
                        0.,
                        tiny_skia::Point::from_xy(a.0, a.1),
                        radius as f32,
                        stops,
                        tiny_skia::SpreadMode::Pad,
                        Transform::identity(),
                    )
                }
            }
            .ok_or_else(|| ChartError("raster gradient is not representable".into()))?;
            paint.shader = shader;
            paint.shader.apply_opacity(opacity as f32);
        }
        let width = stroke.map_or(0., |s| {
            s.width * self.options.view.scale * self.options.pixel_ratio
        });
        ensure(
            width.is_finite() && width <= MAX_COORDINATE,
            "raster stroke width exceeds physical coordinate budget",
        )?;
        let p = self
            .pending
            .as_ref()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?;
        let viewport = ClipRect {
            x: 0.,
            y: 0.,
            width: f64::from(p.pixels.width()),
            height: f64::from(p.pixels.height()),
        };
        let layer = p.blend.as_ref().and_then(blend_layer::Layers::active);
        let clip = layer.map_or_else(
            || p.clips.last().map(|c| c.bounds).unwrap_or(viewport),
            |region| ClipRect {
                x: f64::from(region.x),
                y: f64::from(region.y),
                width: f64::from(region.width),
                height: f64::from(region.height),
            },
        );
        if clip.width <= 0.
            || clip.height <= 0.
            || opacity == 0.
            || fill_paint.is_none() && color.3 == 0
        {
            return Ok(());
        }
        let bounds = path.bounds();
        let area = intersect(
            clip,
            ClipRect {
                x: f64::from(bounds.x()) - width,
                y: f64::from(bounds.y()) - width,
                width: f64::from(bounds.width()) + 2. * width,
                height: f64::from(bounds.height()) + 2. * width,
            },
        );
        self.charge((area.width.ceil() * area.height.ceil()) as usize + 1)?;
        let group_active = self
            .pending
            .as_ref()
            .and_then(|p| p.blend.as_ref())
            .is_some_and(blend_layer::Layers::group_active);
        let mask = if group_active {
            let mask = self.active_mask()?;
            if let (Some(mask), Some(region)) = (mask, layer) {
                self.charge(region.pixels() as usize)?;
                let mut cropped = Mask::new(region.width, region.height)
                    .ok_or_else(|| ChartError("raster group mask allocation failed".into()))?;
                for (row, destination) in cropped
                    .data_mut()
                    .chunks_exact_mut(region.width as usize)
                    .enumerate()
                {
                    let start =
                        (region.y as usize + row) * mask.width() as usize + region.x as usize;
                    destination.copy_from_slice(&mask.data()[start..start + region.width as usize]);
                }
                Some(Arc::new(cropped))
            } else {
                None
            }
        } else if layer.is_some() {
            None
        } else {
            self.active_mask()?
        };
        let pending = self.pending.as_mut().unwrap();
        let mask = mask.as_deref();
        let (mut target, transform) = if let Some(region) = layer {
            let target = pending
                .blend
                .as_mut()
                .and_then(blend_layer::Layers::source)
                .ok_or_else(|| ChartError("raster blend source unavailable".into()))?;
            (
                target,
                Transform::from_translate(-(region.x as f32), -(region.y as f32)),
            )
        } else {
            (pending.pixels.as_mut(), Transform::identity())
        };
        if let Some(s) = stroke {
            let stroke = Stroke {
                width: width as f32,
                line_cap: match s.cap {
                    StrokeCap::Butt => tiny_skia::LineCap::Butt,
                    StrokeCap::Round => tiny_skia::LineCap::Round,
                    StrokeCap::Square => tiny_skia::LineCap::Square,
                },
                ..Default::default()
            };
            target.stroke_path(path, &paint, &stroke, transform, mask);
        } else {
            target.fill_path(path, &paint, FillRule::Winding, transform, mask);
        }
        Ok(())
    }
    fn text_inner(&mut self, label: &Label) -> Result<()> {
        let cost = label.text.len() * 16 + 1;
        let cost = if self.fonts.writing_mode == RasterWritingMode::Vertical
            && self.fonts.vertical_layout.orientation == RasterGlyphOrientation::Mixed
        {
            // Tr alternate probes can shape each grapheme twice before final shaping.
            cost + label.text.len() * 32
        } else {
            cost
        };
        let run_fallback = self.fonts.writing_mode == RasterWritingMode::Vertical;
        #[cfg(feature = "raster-bidi")]
        let run_fallback = run_fallback || self.fonts.bidi.is_some();
        let cost = if run_fallback {
            // One complete fallback scan per input unit, plus run processing.
            cost + label.text.len() * self.fonts.len()
        } else {
            cost
        };
        self.charge(cost)?;
        if self.fonts.color_glyphs != ColorGlyphMode::Reject {
            let painted = self
                .fonts
                .painted_label(label, self.options.missing_glyph)?;
            return self.paint_text_layers(painted);
        }
        let text = self
            .fonts
            .shape(&label.text, label.size, self.options.missing_glyph)?;
        let transform = LabelTransform::new(label, self.fonts.writing_mode, text.advance);
        for glyph in text.glyphs {
            let font = &self.fonts.fonts[glyph.font];
            let face = ttf_parser::Face::parse(&font.bytes, font.index).unwrap();
            ensure(
                !face.is_color_glyph(glyph.id),
                "color glyph requires explicit COLR v0 mode",
            )?;
            let mut outline = Outline::default();
            let bounds = face.outline_glyph(glyph.id, &mut outline);
            ensure(
                !outline.overflow,
                "raster glyph outline command budget exceeded",
            )?;
            if bounds.is_none() {
                ensure(
                    !face.is_color_glyph(glyph.id)
                        && face.glyph_raster_image(glyph.id, 256).is_none()
                        && face.glyph_svg_image(glyph.id).is_none(),
                    "raster color/bitmap/SVG glyphs require an outline font",
                )?;
                continue;
            }
            self.charge(outline.commands.len())?;
            let mut path = PathBuilder::new();
            let point = |x: f32, y: f32| -> Result<(f32, f32)> {
                let (x, y) = transform.point(&glyph, x, y);
                self.point(x, y)
            };
            for command in outline.commands {
                match command {
                    OutlineCommand::Move(x, y) => {
                        let (x, y) = point(x, y)?;
                        path.move_to(x, y);
                    }
                    OutlineCommand::Line(x, y) => {
                        let (x, y) = point(x, y)?;
                        path.line_to(x, y);
                    }
                    OutlineCommand::Quad(x, y, a, b) => {
                        let (x, y) = point(x, y)?;
                        let (a, b) = point(a, b)?;
                        path.quad_to(x, y, a, b);
                    }
                    OutlineCommand::Cubic(x, y, a, b, c, d) => {
                        let (x, y) = point(x, y)?;
                        let (a, b) = point(a, b)?;
                        let (c, d) = point(c, d)?;
                        path.cubic_to(x, y, a, b, c, d);
                    }
                    OutlineCommand::Close => path.close(),
                }
            }
            if let Some(path) = path.finish() {
                self.paint(&path, label.color, 1., None, None)?;
            }
        }
        Ok(())
    }
    fn paint_text_layers(&mut self, text: RasterPaintedText) -> Result<()> {
        use glory::render::PathCommand as C;
        for layer in text.layers {
            self.charge(layer.commands.len())?;
            let mut path = PathBuilder::new();
            for command in layer.commands {
                match command {
                    C::Move(p) => {
                        let (x, y) = self.point(p.x, p.y)?;
                        path.move_to(x, y);
                    }
                    C::Line(p) => {
                        let (x, y) = self.point(p.x, p.y)?;
                        path.line_to(x, y);
                    }
                    C::Cubic { first, second, end } => {
                        let (x, y) = self.point(first.x, first.y)?;
                        let (a, b) = self.point(second.x, second.y)?;
                        let (c, d) = self.point(end.x, end.y)?;
                        path.cubic_to(x, y, a, b, c, d);
                    }
                    C::Close => path.close(),
                }
            }
            if let Some(path) = path.finish() {
                self.paint(&path, layer.color, 1., None, None)?;
            }
        }
        Ok(())
    }
}
fn p_width(c: &RasterCanvas) -> u32 {
    c.pending.as_ref().unwrap().pixels.width()
}
fn p_height(c: &RasterCanvas) -> u32 {
    c.pending.as_ref().unwrap().pixels.height()
}
fn intersect(a: ClipRect, b: ClipRect) -> ClipRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    ClipRect {
        x,
        y,
        width: (a.x + a.width).min(b.x + b.width).max(x) - x,
        height: (a.y + a.height).min(b.y + b.height).max(y) - y,
    }
}
#[derive(Default)]
struct Outline {
    commands: Vec<OutlineCommand>,
    overflow: bool,
}
enum OutlineCommand {
    Move(f32, f32),
    Line(f32, f32),
    Quad(f32, f32, f32, f32),
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}
impl Outline {
    fn push(&mut self, c: OutlineCommand) {
        if self.commands.len() >= MAX_GLYPH_COMMANDS {
            self.overflow = true;
        } else {
            self.commands.push(c);
        }
    }
}
impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.push(OutlineCommand::Move(x, y));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.push(OutlineCommand::Line(x, y));
    }
    fn quad_to(&mut self, x: f32, y: f32, a: f32, b: f32) {
        self.push(OutlineCommand::Quad(x, y, a, b));
    }
    fn curve_to(&mut self, x: f32, y: f32, a: f32, b: f32, c: f32, d: f32) {
        self.push(OutlineCommand::Cubic(x, y, a, b, c, d));
    }
    fn close(&mut self) {
        self.push(OutlineCommand::Close);
    }
}
impl Canvas for RasterCanvas {
    fn supports_group_opacity(&self) -> bool {
        true
    }
    fn prepare_mark_blends(&mut self, scene: &Scene, blends: &MarkBlends) -> Result<()> {
        self.prepared_blend = None;
        // Resolve every explicit request before a new frame allocates pixels.
        // Default typography retains the historical registration-order profile.
        for index in 0..scene.labels.len() {
            if scene.typography.has_label_font(index) {
                self.fonts.for_font(&scene.resolved_label_font(index))?;
            }
        }
        if crate::blend::has_isolated_marks(scene, blends) {
            self.options.validate()?;
            let ratio = self.options.pixel_ratio;
            let size = [scene.width, scene.height].map(|n| (n * ratio).ceil() as u32);
            ensure(
                u64::from(size[0]) * u64::from(size[1]) <= self.options.max_pixels,
                "raster pixel budget exceeded",
            )?;
            let plan = BlendPlan::new(scene, blends, size, ratio, self.options.view)?;
            self.prepared_blend = Some(blend_layer::Layers::new(plan)?);
        }
        Ok(())
    }
    fn begin_mark_blend(&mut self, index: usize) -> Result<bool> {
        let result = (|| {
            let blend = self
                .pending
                .as_ref()
                .and_then(|p| p.blend.as_ref())
                .ok_or_else(|| ChartError("raster blend plan unavailable".into()))?;
            let Some(region) = blend.region(index) else {
                return Ok(false);
            };
            // Count both scratch clearing and the composite pass. Source
            // fills/strokes and destination masks charge their own work.
            self.charge(region.pixels() as usize * 2)?;
            self.pending
                .as_mut()
                .unwrap()
                .blend
                .as_mut()
                .unwrap()
                .begin(region, index)?;
            Ok(true)
        })();
        self.fail(result)
    }
    fn end_mark_blend(&mut self, mode: BlendMode, opacity: f64, commit: bool) -> Result<()> {
        let result = (|| {
            let mask = if commit { self.active_mask()? } else { None };
            let pending = self
                .pending
                .as_mut()
                .ok_or_else(|| ChartError("raster frame not begun".into()))?;
            pending
                .blend
                .as_mut()
                .ok_or_else(|| ChartError("raster blend plan unavailable".into()))?
                .end(&mut pending.pixels, mask.as_deref(), mode, opacity, commit)
        })();
        self.fail(result)
    }
    fn supports_translucent_dash_dots(&self) -> bool {
        true
    }
    fn stroke_dashed(
        &mut self,
        geometry: &crate::Geometry,
        stroke: GraphicStroke,
        dash: &crate::StrokeDash,
        opacity: f64,
    ) -> Result<()> {
        let result = (|| {
            ensure(
                stroke.width.is_finite() && stroke.width > 0. && stroke.width <= 10000.,
                "invalid raster stroke width",
            )?;
            self.charge(1)?;
            let density = self.options.pixel_ratio * self.options.view.scale;
            let limits = crate::StrokeDashLimits {
                tolerance: 0.1 / density,
                length_error: 0.01 / density,
                max_work: (self.options.max_operations - self.pending.as_ref().unwrap().used)
                    .clamp(1, crate::MAX_DASH_WORK),
                ..Default::default()
            };
            let detailed = crate::dash_geometry_detailed(geometry, dash, &limits)?;
            self.charge(detailed.work)?;
            let runs = detailed.stroke_geometry();
            self.charge(runs.commands().len() * 32)?;
            super::replay_path(self, &runs)?;
            let physical = self.pending.as_ref().unwrap().path.clone().finish();
            let width = stroke.width * density;
            ensure(
                width.is_finite() && width <= MAX_COORDINATE,
                "raster stroke width exceeds physical coordinate budget",
            )?;
            let mut ink = PathBuilder::new();
            let mut commands = 0;
            if let Some(path) = physical
                && let Some(outline) = path.stroke(
                    &Stroke {
                        width: width as f32,
                        line_cap: match stroke.cap {
                            StrokeCap::Butt => tiny_skia::LineCap::Butt,
                            StrokeCap::Round => tiny_skia::LineCap::Round,
                            StrokeCap::Square => tiny_skia::LineCap::Square,
                        },
                        ..Default::default()
                    },
                    1.,
                )
            {
                commands = outline.segments().count();
                ensure(
                    commands <= 1_000_000,
                    "raster dashed outline command budget exceeded",
                )?;
                ink.push_path(&outline);
            }
            // Paint all positive runs and zero-dash cap footprints as one
            // nonzero-winding ink path. Separate stroke/fill calls would apply
            // translucent color twice wherever their cap regions overlap.
            if stroke.cap != StrokeCap::Butt {
                for dot in detailed.dots {
                    let geometry = crate::dash_dot_geometry(&dot, stroke.width, stroke.cap)?;
                    super::replay_path(self, &geometry)?;
                    if let Some(path) = self.pending.as_ref().unwrap().path.clone().finish() {
                        commands += path.segments().count();
                        ensure(
                            commands <= 1_000_000,
                            "raster dashed outline command budget exceeded",
                        )?;
                        ink.push_path(&path);
                    }
                }
            }
            self.charge(commands)?;
            if let Some(path) = ink.finish() {
                self.paint(&path, stroke.color, opacity, None, None)?;
            }
            Ok(())
        })();
        self.fail(result)
    }
    fn supports_fill_paint(&self) -> bool {
        true
    }
    fn fill_paint(&mut self, paint: &crate::FillPaint, opacity: f64) -> Result<()> {
        let r = (|| {
            paint.validate()?;
            self.charge(1 + paint.stops().len() * 128)?;
            let path = self
                .pending
                .as_ref()
                .ok_or_else(|| ChartError("raster frame not begun".into()))?
                .path
                .clone()
                .finish();
            if let Some(path) = path {
                self.paint(&path, Color(0, 0, 0, 255), opacity, None, Some(paint))?;
            }
            Ok(())
        })();
        self.fail(r)
    }
    fn supports_text_rotation(&self) -> bool {
        true
    }
    fn supports_clipping(&self) -> bool {
        true
    }
    fn begin(&mut self, width: f64, height: f64, background: Color) -> Result<()> {
        self.pending = None;
        self.failed = false;
        let blend = self.prepared_blend.take();
        let result = (|| {
            self.options.validate()?;
            ensure(
                width.is_finite() && height.is_finite() && width > 0. && height > 0.,
                "invalid raster logical dimensions",
            )?;
            let w = (width * self.options.pixel_ratio).ceil();
            let h = (height * self.options.pixel_ratio).ceil();
            ensure(
                w.is_finite()
                    && h.is_finite()
                    && w >= 1.
                    && h >= 1.
                    && w * h <= self.options.max_pixels as f64,
                "raster backing store exceeds pixel budget",
            )?;
            let used = (w * h) as usize
                + blend
                    .as_ref()
                    .map_or(0, blend_layer::Layers::allocation_pixels);
            ensure(
                used <= self.options.max_operations,
                "raster operation budget exceeded",
            )?;
            let mut pixels = Pixmap::new(w as u32, h as u32)
                .ok_or_else(|| ChartError("raster allocation failed".into()))?;
            pixels.fill(tiny_skia::Color::from_rgba8(
                background.0,
                background.1,
                background.2,
                background.3,
            ));
            self.pending = Some(Pending {
                pixels,
                logical_width: width,
                logical_height: height,
                path: PathBuilder::new(),
                clips: vec![],
                masks: BTreeMap::new(),
                mask_pixels: 0,
                mask_key_bytes: 0,
                patterns: BTreeMap::new(),
                pattern_pixels: 0,
                used,
                blend,
            });
            Ok(())
        })();
        self.fail(result)
    }
    fn begin_path(&mut self) -> Result<()> {
        let r = self.charge(1).map(|_| {
            self.pending.as_mut().unwrap().path = PathBuilder::new();
        });
        self.fail(r)
    }
    fn move_to(&mut self, x: f64, y: f64) -> Result<()> {
        let r = (|| {
            self.charge(1)?;
            let (x, y) = self.point(x, y)?;
            self.pending.as_mut().unwrap().path.move_to(x, y);
            Ok(())
        })();
        self.fail(r)
    }
    fn line_to(&mut self, x: f64, y: f64) -> Result<()> {
        let r = (|| {
            self.charge(1)?;
            let (x, y) = self.point(x, y)?;
            self.pending.as_mut().unwrap().path.line_to(x, y);
            Ok(())
        })();
        self.fail(r)
    }
    fn cubic_to(&mut self, x: f64, y: f64, a: f64, b: f64, c: f64, d: f64) -> Result<()> {
        let r = (|| {
            self.charge(1)?;
            let (x, y) = self.point(x, y)?;
            let (a, b) = self.point(a, b)?;
            let (c, d) = self.point(c, d)?;
            self.pending
                .as_mut()
                .unwrap()
                .path
                .cubic_to(x, y, a, b, c, d);
            Ok(())
        })();
        self.fail(r)
    }
    fn close_path(&mut self) -> Result<()> {
        let r = self
            .charge(1)
            .map(|_| self.pending.as_mut().unwrap().path.close());
        self.fail(r)
    }
    fn fill(&mut self, color: Color, opacity: f64) -> Result<()> {
        let r = (|| {
            self.charge(1)?;
            let path = self.pending.as_ref().unwrap().path.clone().finish();
            if let Some(path) = path {
                self.paint(&path, color, opacity, None, None)?;
            }
            Ok(())
        })();
        self.fail(r)
    }
    fn stroke(&mut self, stroke: GraphicStroke, opacity: f64) -> Result<()> {
        let r = (|| {
            ensure(
                stroke.width.is_finite() && stroke.width > 0. && stroke.width <= 10000.,
                "invalid raster stroke width",
            )?;
            self.charge(1)?;
            let path = self.pending.as_ref().unwrap().path.clone().finish();
            if let Some(path) = path {
                self.paint(&path, stroke.color, opacity, Some(stroke), None)?;
            }
            Ok(())
        })();
        self.fail(r)
    }
    fn push_clip(&mut self, clip: ClipRect) -> Result<()> {
        let result = (|| {
            clip.validate()?;
            self.charge(1)?;
            let (a, b) = self.point(clip.x, clip.y)?;
            let (c, d) = self.point(clip.x + clip.width, clip.y + clip.height)?;
            let physical = ClipRect {
                x: f64::from(a),
                y: f64::from(b),
                width: f64::from(c - a),
                height: f64::from(d - b),
            };
            let previous = self.pending.as_ref().unwrap().clips.last().cloned();
            let viewport = self.viewport()?;
            let rect = intersect(previous.as_ref().map_or(viewport, |c| c.rect), physical);
            let bounds = intersect(previous.as_ref().map_or(viewport, |c| c.bounds), physical);
            let paths = previous.map_or_else(Vec::new, |c| c.paths);
            self.push_state(ClipKind::Rect, rect, bounds, paths)
        })();
        self.fail(result)
    }
    fn pop_clip(&mut self) -> Result<()> {
        self.pop_kind(ClipKind::Rect)
    }
    fn supports_clip_paths(&self) -> bool {
        true
    }
    fn push_clip_path(&mut self, clip: &crate::ClipPath) -> Result<()> {
        let result = (|| {
            clip.validate()?;
            self.charge(1)?;
            ensure(
                self.pending.as_ref().unwrap().clips.len() < 16,
                "raster clip nesting budget exceeded",
            )?;
            super::replay_path(self, &clip.geometry)?;
            let path = self.pending.as_ref().unwrap().path.clone().finish();
            let previous = self.pending.as_ref().unwrap().clips.last().cloned();
            let viewport = self.viewport()?;
            let rect = previous.as_ref().map_or(viewport, |c| c.rect);
            let bounds = path.as_ref().map_or(
                ClipRect {
                    x: 0.,
                    y: 0.,
                    width: 0.,
                    height: 0.,
                },
                |p| {
                    let b = p.bounds();
                    ClipRect {
                        x: f64::from(b.x()),
                        y: f64::from(b.y()),
                        width: f64::from(b.width()),
                        height: f64::from(b.height()),
                    }
                },
            );
            let bounds = intersect(previous.as_ref().map_or(viewport, |c| c.bounds), bounds);
            let mut paths = previous.map_or_else(Vec::new, |c| c.paths);
            let id = clip.svg_id();
            // Set intersection is idempotent. Reapplying an identical clip
            // must not multiply antialiased edge coverage into a darker edge.
            if !paths.iter().any(|p| p.id == id) {
                paths.push(Arc::new(PhysicalClip {
                    id,
                    path,
                    rule: match clip.rule {
                        crate::ClipRule::NonZero => FillRule::Winding,
                        crate::ClipRule::EvenOdd => FillRule::EvenOdd,
                    },
                }));
            }
            self.push_state(ClipKind::Path, rect, bounds, paths)
        })();
        self.fail(result)
    }
    fn pop_clip_path(&mut self) -> Result<()> {
        self.pop_kind(ClipKind::Path)
    }
    fn text(&mut self, label: &Label) -> Result<()> {
        let r = self.text_inner(label);
        self.fail(r)
    }
    fn supports_text_fonts(&self) -> bool {
        true
    }
    fn text_with_font(&mut self, label: &Label, font: &crate::ResolvedFont) -> Result<()> {
        let selected = self.fonts.for_font(font);
        let result = match selected {
            Ok(selected) => {
                let original = std::mem::replace(&mut self.fonts, selected);
                let result = self.text_inner(label);
                self.fonts = original;
                result
            }
            Err(error) => Err(error),
        };
        self.fail(result)
    }
    fn finish(&mut self) -> Result<()> {
        let pending = self
            .pending
            .take()
            .ok_or_else(|| ChartError("raster frame not begun".into()))?;
        ensure(
            !self.failed
                && pending.clips.is_empty()
                && pending.blend.as_ref().is_none_or(|b| b.active().is_none()),
            "raster frame discarded after rejected paint or unbalanced clipping",
        )?;
        let width = pending.pixels.width();
        let height = pending.pixels.height();
        let mut rgba = pending.pixels.take();
        for pixel in rgba.as_chunks_mut::<4>().0 {
            let a = u32::from(pixel[3]);
            for v in &mut pixel[..3] {
                *v = (u32::from(*v) * 255 + a / 2)
                    .checked_div(a)
                    .unwrap_or(0)
                    .min(255) as u8;
            }
        }
        self.frame = Some(RenderedImage {
            width,
            height,
            logical_width: pending.logical_width,
            logical_height: pending.logical_height,
            pixel_ratio: self.options.pixel_ratio,
            rgba,
        });
        Ok(())
    }
}
pub fn render_raster(
    scene: &Scene,
    fonts: &FontRegistry,
    options: &RasterOptions,
) -> Result<RenderedImage> {
    render_raster_with_blends(scene, fonts, options, &MarkBlends::default())
}

/// Render an isolated mark blend profile to CPU RGBA and PNG.
pub fn render_raster_with_blends(
    scene: &Scene,
    fonts: &FontRegistry,
    options: &RasterOptions,
    blends: &MarkBlends,
) -> Result<RenderedImage> {
    let mut canvas = RasterCanvas::new(fonts.clone(), options.clone())?;
    canvas.render_with_blends(scene, blends)?;
    Ok(canvas.frame.take().unwrap())
}
