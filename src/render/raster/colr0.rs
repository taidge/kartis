//! Bounded COLR v0 solid-layer collection before any destination is painted.
use super::{Color, ColorGlyphMode, Outline};
use crate::{ChartError, Result, scene::ensure};
use ttf_parser::{Face, GlyphId, Rect, RgbaColor, Tag, colr};

const MAX_GLYPH_LAYERS: usize = 256;

pub(super) struct Layer {
    pub index: usize,
    pub outline: Outline,
    pub bounds: Rect,
    pub color: Color,
}

fn invalid() -> ChartError {
    ChartError("malformed COLR v0/CPAL glyph data".into())
}
fn u16_at(data: &[u8], at: usize) -> Result<u16> {
    let bytes = data
        .get(at..at.checked_add(2).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
}
fn u32_at(data: &[u8], at: usize) -> Result<usize> {
    let bytes = data
        .get(at..at.checked_add(4).ok_or_else(invalid)?)
        .ok_or_else(invalid)?;
    Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize)
}
fn array(data: &[u8], offset: usize, count: usize, stride: usize) -> Result<&[u8]> {
    let length = count.checked_mul(stride).ok_or_else(invalid)?;
    data.get(offset..offset.checked_add(length).ok_or_else(invalid)?)
        .ok_or_else(invalid)
}

// Painter callbacks cannot short-circuit. Validate the selected v0 record and
// palette row first so malformed data and oversized layer lists are rejected
// before the parser starts invoking callbacks.
fn preflight(face: &Face<'_>, glyph: GlyphId, palette: u16, remaining: usize) -> Result<usize> {
    let table = face.tables().colr.ok_or_else(invalid)?;
    ensure(
        table.is_simple(),
        "color glyph requires COLR v0; COLR v1 is unsupported",
    )?;
    let data = face
        .raw_face()
        .table(Tag::from_bytes(b"COLR"))
        .ok_or_else(invalid)?;
    let count = usize::from(u16_at(data, 2)?);
    let records = array(data, u32_at(data, 4)?, count, 6)?;
    let mut low = 0;
    let mut high = count;
    while low < high {
        let middle = low + (high - low) / 2;
        if u16_at(records, middle * 6)? < glyph.0 {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    ensure(
        low < count && u16_at(records, low * 6)? == glyph.0,
        "missing COLR v0 base glyph",
    )?;
    let first = usize::from(u16_at(records, low * 6 + 2)?);
    let layers = usize::from(u16_at(records, low * 6 + 4)?);
    ensure(
        layers <= MAX_GLYPH_LAYERS && layers <= remaining,
        "color glyph layer budget exceeded",
    )?;
    let total = usize::from(u16_at(data, 12)?);
    ensure(first + layers <= total, "invalid COLR v0 layer range")?;
    let records = array(data, u32_at(data, 8)?, total, 4)?;
    let records = &records[first * 4..(first + layers) * 4];
    let cpal = face
        .raw_face()
        .table(Tag::from_bytes(b"CPAL"))
        .ok_or_else(invalid)?;
    let entries = u16_at(cpal, 2)?;
    let palettes = face.color_palettes().ok_or_else(invalid)?.get();
    ensure(palette < palettes, "color glyph palette index out of range")?;
    ensure(entries > 0, "empty color glyph palette")?;
    let palette_start = usize::from(u16_at(cpal, 12 + usize::from(palette) * 2)?);
    ensure(
        palette_start + usize::from(entries) <= usize::from(u16_at(cpal, 6)?),
        "invalid CPAL palette row",
    )?;
    for record in records.as_chunks::<4>().0 {
        let id = u16_at(record, 0)?;
        let color = u16_at(record, 2)?;
        ensure(id < face.number_of_glyphs(), "invalid COLR v0 layer glyph")?;
        // ttf-parser's CPAL accessor bounds-checks the whole color array.
        // Checking the palette's own width also prevents crossing into the
        // next palette on a malformed layer entry.
        ensure(
            color == 0xFFFF || color < entries,
            "COLR v0 palette entry out of range",
        )?;
    }
    Ok(layers)
}

pub(super) fn layers(
    face: &Face<'_>,
    glyph: GlyphId,
    mode: ColorGlyphMode,
    foreground: Color,
    remaining_layers: usize,
    remaining_commands: usize,
) -> Result<(usize, Vec<Layer>)> {
    if face.is_color_glyph(glyph) {
        let ColorGlyphMode::ColrV0 { palette } = mode else {
            return Err(ChartError(
                "color glyph requires explicit COLR v0 mode and painted_label".into(),
            ));
        };
        let count = preflight(face, glyph, palette, remaining_layers)?;
        let mut painter = Collector {
            face,
            current: None,
            layers: Vec::new(),
            index: 0,
            commands: 0,
            remaining_commands,
            error: None,
        };
        let Color(r, g, b, a) = foreground;
        let painted =
            face.paint_color_glyph(glyph, palette, RgbaColor::new(r, g, b, a), &mut painter);
        if let Some(error) = painter.error {
            return Err(ChartError(error.into()));
        }
        ensure(
            painted.is_some() && painter.current.is_none() && painter.index == count,
            "incomplete COLR v0 glyph paint",
        )?;
        return Ok((count, painter.layers));
    }
    if mode != ColorGlyphMode::Reject
        && face.raw_face().table(Tag::from_bytes(b"COLR")).is_some()
        && face.raw_face().table(Tag::from_bytes(b"CPAL")).is_some()
        && face.tables().colr.is_none()
    {
        return Err(invalid());
    }
    let mut outline = Outline::default();
    let bounds = face.outline_glyph(glyph, &mut outline);
    ensure(
        !outline.overflow && outline.commands.len() <= remaining_commands,
        "font glyph outline command budget exceeded",
    )?;
    if let Some(bounds) = bounds {
        ensure(remaining_layers > 0, "text paint layer budget exceeded")?;
        Ok((
            1,
            vec![Layer {
                index: 0,
                outline,
                bounds,
                color: foreground,
            }],
        ))
    } else {
        ensure(
            face.glyph_raster_image(glyph, 256).is_none() && face.glyph_svg_image(glyph).is_none(),
            "bitmap/SVG glyph requires an outline font or supported COLR v0 font",
        )?;
        Ok((0, Vec::new()))
    }
}

struct Collector<'a, 'font> {
    face: &'a Face<'font>,
    current: Option<(Outline, Option<Rect>)>,
    layers: Vec<Layer>,
    index: usize,
    commands: usize,
    remaining_commands: usize,
    error: Option<&'static str>,
}
impl Collector<'_, '_> {
    fn reject(&mut self) {
        self.error
            .get_or_insert("unsupported callback in COLR v0 glyph");
    }
}
impl<'font> colr::Painter<'font> for Collector<'_, 'font> {
    fn outline_glyph(&mut self, glyph: GlyphId) {
        if self.error.is_some() {
            return;
        }
        if self.current.is_some() {
            self.reject();
            return;
        }
        let mut outline = Outline::default();
        let bounds = self.face.outline_glyph(glyph, &mut outline);
        self.commands += outline.commands.len();
        if outline.overflow || self.commands > self.remaining_commands {
            self.error = Some("color glyph outline command budget exceeded");
            return;
        }
        if bounds.is_none() && !outline.commands.is_empty() {
            self.error = Some("incomplete color glyph layer outline");
            return;
        }
        self.current = Some((outline, bounds));
    }
    fn paint(&mut self, paint: colr::Paint<'font>) {
        if self.error.is_some() {
            return;
        }
        let colr::Paint::Solid(color) = paint else {
            self.reject();
            return;
        };
        let Some((outline, bounds)) = self.current.take() else {
            self.reject();
            return;
        };
        if let Some(bounds) = bounds {
            self.layers.push(Layer {
                index: self.index,
                outline,
                bounds,
                color: Color(color.red, color.green, color.blue, color.alpha),
            });
        }
        self.index += 1;
    }
    fn push_clip(&mut self) {
        self.reject();
    }
    fn push_clip_box(&mut self, _: colr::ClipBox) {
        self.reject();
    }
    fn pop_clip(&mut self) {
        self.reject();
    }
    fn push_layer(&mut self, _: colr::CompositeMode) {
        self.reject();
    }
    fn pop_layer(&mut self) {
        self.reject();
    }
    fn push_transform(&mut self, _: ttf_parser::Transform) {
        self.reject();
    }
    fn pop_transform(&mut self) {
        self.reject();
    }
}
