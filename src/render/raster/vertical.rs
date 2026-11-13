//! Logical top-to-bottom OpenType shaping, columns and Unicode orientation.
use std::ops::Range;

use rustybuzz::{BufferFlags, Direction, Face, GlyphBuffer, UnicodeBuffer};
use unicode_segmentation::UnicodeSegmentation;

use super::text_runs::{ShapeRun, bidi_control, clusters, normalized, shape_runs};
use super::vertical_data::{self, VerticalOrientation};
use super::{
    Glyph, MAX_COORDINATE, MAX_SHAPED_GLYPHS, MissingGlyphPolicy, RasterTextColumn,
    RasterWritingMode, Text,
};
use crate::Result;
use crate::scene::ensure;

/// Orientation policy for complete extended grapheme clusters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RasterGlyphOrientation {
    /// Use OpenType top-to-bottom shaping for every grapheme.
    #[default]
    Upright,
    /// Unicode 16.0 UAX #50 U/Tu upright and R sideways. Tr uses a vertical
    /// alternate when available and otherwise rotates horizontal shaping.
    /// Combining enclosing marks keep their complete grapheme upright.
    /// This uses the orientation rules underlying CSS mixed text orientation.
    /// Column layout uses explicit breaks and font-metric centering of sideways
    /// runs. Ordering stays logical across runs, with native horizontal script
    /// shaping within sideways runs. Full CSS bidi, baseline tables, automatic
    /// line wrapping and text-combine-upright require a higher-level layout.
    Mixed,
}

/// Interpretation and progression of explicit vertical line separators.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RasterVerticalColumns {
    /// Preserve single-column whitespace normalization.
    #[default]
    Single,
    /// Advance subsequent explicit columns toward decreasing scene x.
    RightToLeft,
    /// Advance subsequent explicit columns toward increasing scene x.
    LeftToRight,
}

/// Opt-in vertical layout shared by measurement, raster and glyph outlines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterVerticalLayout {
    pub orientation: RasterGlyphOrientation,
    pub columns: RasterVerticalColumns,
    /// Distance between column centers, in em. Finite values in 0.25..=16.
    pub column_advance_em: f64,
}
impl Default for RasterVerticalLayout {
    fn default() -> Self {
        Self {
            orientation: RasterGlyphOrientation::Upright,
            columns: RasterVerticalColumns::Single,
            column_advance_em: 1.2,
        }
    }
}
impl RasterVerticalLayout {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.column_advance_em.is_finite() && (0.25..=16.).contains(&self.column_advance_em),
            "invalid raster vertical column advance",
        )
    }
}

const MAX_COLUMNS: usize = 256;

fn column_ranges(text: &str, mode: RasterVerticalColumns) -> Result<Vec<Range<usize>>> {
    if mode == RasterVerticalColumns::Single {
        return Ok(std::iter::once(0..text.len()).collect());
    }
    let mut columns = Vec::new();
    let mut start = 0;
    // CRLF is one extended grapheme and therefore introduces one column.
    for (offset, grapheme) in text.grapheme_indices(true) {
        if matches!(
            grapheme,
            "\r\n" | "\r" | "\n" | "\u{c}" | "\u{85}" | "\u{2028}" | "\u{2029}"
        ) {
            ensure(
                columns.len() + 1 < MAX_COLUMNS,
                "raster vertical column budget exceeded",
            )?;
            columns.push(start..offset);
            start = offset + grapheme.len();
        }
    }
    columns.push(start..text.len());
    Ok(columns)
}

fn shape_run(
    run: &ShapeRun,
    bytes: Range<usize>,
    line: &str,
    offsets: &[u32],
    face: &Face<'_>,
    direction: Option<Direction>,
    work: &mut usize,
) -> Result<GlyphBuffer> {
    let mut buffer = UnicodeBuffer::new();
    for (index, c) in line[bytes.clone()].char_indices() {
        buffer.add(c, offsets[bytes.start + index]);
    }
    let tag = ttf_parser::Tag::from_bytes_lossy(run.script.short_name().as_bytes());
    if let Some(script) = rustybuzz::Script::from_iso15924_tag(tag) {
        buffer.set_script(script);
    }
    if let Some(direction) = direction {
        buffer.set_direction(direction);
    }
    buffer.guess_segment_properties();
    buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);
    let shaped = rustybuzz::shape(face, &[], buffer);
    *work += shaped.len();
    ensure(
        shaped.len() <= MAX_SHAPED_GLYPHS && *work <= MAX_SHAPED_GLYPHS * 3,
        "shaped glyph work budget exceeded",
    )?;
    Ok(shaped)
}

fn oriented_spans(
    run: &ShapeRun,
    line: &str,
    offsets: &[u32],
    face: &Face<'_>,
    orientation: RasterGlyphOrientation,
    work: &mut usize,
) -> Result<Vec<(Range<usize>, bool)>> {
    if orientation == RasterGlyphOrientation::Upright {
        return Ok(vec![(run.bytes.clone(), false)]);
    }
    let mut spans: Vec<(Range<usize>, bool)> = Vec::new();
    for (offset, grapheme) in line[run.bytes.clone()].grapheme_indices(true) {
        let start = run.bytes.start + offset;
        let bytes = start..start + grapheme.len();
        let property = if grapheme.chars().any(vertical_data::enclosing) {
            VerticalOrientation::U
        } else {
            vertical_data::orientation(grapheme.chars().next().unwrap())
        };
        let sideways = match property {
            VerticalOrientation::U | VerticalOrientation::Tu => false,
            VerticalOrientation::R => true,
            VerticalOrientation::Tr => {
                let vertical = shape_run(
                    run,
                    bytes.clone(),
                    line,
                    offsets,
                    face,
                    Some(Direction::TopToBottom),
                    work,
                )?;
                let horizontal = shape_run(run, bytes.clone(), line, offsets, face, None, work)?;
                vertical
                    .glyph_infos()
                    .iter()
                    .map(|g| g.glyph_id)
                    .eq(horizontal.glyph_infos().iter().map(|g| g.glyph_id))
            }
        };
        if let Some((last, previous)) = spans.last_mut()
            && *previous == sideways
        {
            last.end = bytes.end;
        } else {
            spans.push((bytes, sideways));
        }
    }
    Ok(spans)
}

pub(super) fn shape(
    text: &str,
    size: f64,
    missing: MissingGlyphPolicy,
    faces: &[Face<'_>],
    layout: RasterVerticalLayout,
) -> Result<Text> {
    let ranges = column_ranges(text, layout.columns)?;
    let progression = match layout.columns {
        RasterVerticalColumns::RightToLeft => -1.,
        RasterVerticalColumns::Single | RasterVerticalColumns::LeftToRight => 1.,
    };
    let mut result = Text {
        glyphs: Vec::new(),
        advance: 0.,
        columns: Vec::new(),
    };
    let mut work = 0;
    for (index, bytes) in ranges.into_iter().enumerate() {
        let (line, mut offsets) = normalized(&text[bytes.clone()])?;
        for offset in &mut offsets {
            *offset += bytes.start as u32;
        }
        ensure(
            !line.chars().any(bidi_control),
            "directional formatting controls are unsupported in raster vertical text",
        )?;
        let clusters = clusters(&line, 0..line.len());
        let runs = shape_runs(
            &line,
            &clusters,
            faces,
            missing,
            RasterWritingMode::Vertical,
        )?;
        let column_x = progression * index as f64 * size * layout.column_advance_em;
        let mut x = column_x;
        let mut y = 0.;
        for run in runs {
            let face = &faces[run.font];
            let scale = size / f64::from(face.units_per_em());
            let spans = oriented_spans(&run, &line, &offsets, face, layout.orientation, &mut work)?;
            for (bytes, sideways) in spans {
                let direction = (!sideways).then_some(Direction::TopToBottom);
                let shaped = shape_run(&run, bytes, &line, &offsets, face, direction, &mut work)?;
                ensure(
                    result.glyphs.len() + shaped.len() <= MAX_SHAPED_GLYPHS,
                    "shaped glyph budget exceeded",
                )?;
                let center = if sideways {
                    (f64::from(face.ascender()) + f64::from(face.descender())) * scale / 2.
                } else {
                    0.
                };
                for (glyph, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                    let (glyph_x, glyph_y) = if sideways {
                        (
                            x - center + f64::from(position.y_offset) * scale,
                            y + f64::from(position.x_offset) * scale,
                        )
                    } else {
                        (
                            x + f64::from(position.x_offset) * scale,
                            y - f64::from(position.y_offset) * scale,
                        )
                    };
                    result.glyphs.push(Glyph {
                        font: run.font,
                        id: ttf_parser::GlyphId(glyph.glyph_id as u16),
                        #[cfg(test)]
                        cluster: glyph.cluster,
                        x: glyph_x,
                        y: glyph_y,
                        size: scale,
                        sideways,
                    });
                    if sideways {
                        x += f64::from(position.y_advance) * scale;
                        y += f64::from(position.x_advance) * scale;
                    } else {
                        x += f64::from(position.x_advance) * scale;
                        y -= f64::from(position.y_advance) * scale;
                    }
                }
            }
        }
        ensure(
            x.is_finite()
                && x.abs() <= MAX_COORDINATE
                && y.is_finite()
                && (0. ..=MAX_COORDINATE).contains(&y),
            "raster text advance overflow",
        )?;
        result.advance = result.advance.max(y);
        result.columns.push(RasterTextColumn {
            x: column_x,
            advance: y,
        });
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::raster::FontRegistry;

    #[test]
    fn unicode_16_orientation_table_matches_every_scalar_and_enclosing_mark() {
        use VerticalOrientation::{R, Tr, Tu, U};
        let mut expected = vec![R; 0x110000];
        let source = include_str!("../../../tests/fixtures/vertical-orientation-unicode-16.txt");
        for line in source.lines() {
            let line = line.split('#').next().unwrap().trim();
            if line.is_empty() {
                continue;
            }
            let (span, property) = line.split_once(';').unwrap();
            let mut range = span.trim().split("..");
            let start = usize::from_str_radix(range.next().unwrap(), 16).unwrap();
            let end = range
                .next()
                .map(|end| usize::from_str_radix(end, 16).unwrap())
                .unwrap_or(start);
            let property = match property.trim() {
                "U" => U,
                "R" => R,
                "Tu" => Tu,
                "Tr" => Tr,
                _ => panic!("unknown property"),
            };
            expected[start..=end].fill(property);
        }
        let mut enclosing = vec![false; 0x110000];
        let source = include_str!("../../../tests/fixtures/vertical-enclosing-unicode-16.txt");
        for line in source.lines().filter(|line| !line.starts_with('#')) {
            let (span, _) = line.split_once(';').unwrap();
            let (start, end) = span.split_once("..").unwrap();
            let start = usize::from_str_radix(start, 16).unwrap();
            let end = usize::from_str_radix(end, 16).unwrap();
            enclosing[start..=end].fill(true);
        }
        for (code, expected) in expected.into_iter().enumerate() {
            if let Some(c) = char::from_u32(code as u32) {
                assert_eq!(vertical_data::orientation(c), expected, "U+{code:X}");
                assert_eq!(vertical_data::enclosing(c), enclosing[code], "U+{code:X}");
            }
        }
    }

    #[test]
    fn mixed_columns_match_native_harfbuzz_positions_metrics_and_ink_boxes() {
        let cases = include!("../../../tests/fixtures/vertical-mixed-harfbuzz.rs");
        let fonts = [
            include_bytes!("../../../tests/fonts/KartisFixtureVertical-Regular.ttf").as_slice(),
            include_bytes!("../../../tests/fonts/KartisFixtureVerticalFallback-Regular.ttf")
                .as_slice(),
        ];
        let size = 40.;
        let scale = size / 1000.;
        for &(font, right_to_left, scalars, expected_columns, expected_glyphs, expected_bounds) in
            cases
        {
            let columns = if right_to_left {
                RasterVerticalColumns::RightToLeft
            } else {
                RasterVerticalColumns::LeftToRight
            };
            let mut registry = FontRegistry::new().with_vertical_layout(RasterVerticalLayout {
                orientation: RasterGlyphOrientation::Mixed,
                columns,
                column_advance_em: 1.5,
            });
            registry.register(fonts[font], 0).unwrap();
            let text: String = scalars
                .iter()
                .map(|&code| char::from_u32(code).unwrap())
                .collect();
            let shaped = registry
                .shape(&text, size, MissingGlyphPolicy::Error)
                .unwrap();
            assert_eq!(shaped.glyphs.len(), expected_glyphs.len(), "{text}");
            for (glyph, &(id, cluster, x, y, sideways)) in shaped.glyphs.iter().zip(expected_glyphs)
            {
                assert_eq!(glyph.id.0, id, "{text}");
                assert_eq!(glyph.cluster, cluster, "{text}");
                assert!(
                    (glyph.x - x * scale).abs() < 1e-10,
                    "{text} x={} expected={}",
                    glyph.x,
                    x * scale
                );
                assert!((glyph.y - y * scale).abs() < 1e-10, "{text} y");
                assert_eq!(glyph.sideways, sideways, "{text}");
            }
            let metrics = registry
                .measure_layout(&text, size, MissingGlyphPolicy::Error)
                .unwrap();
            assert_eq!(metrics.columns.len(), expected_columns.len());
            for (column, &(x, advance)) in metrics.columns.iter().zip(expected_columns) {
                assert!((column.x - x * scale).abs() < 1e-10);
                assert!((column.advance - advance * scale).abs() < 1e-10);
            }
            let bounds = metrics.ink_bounds.unwrap();
            for (actual, expected) in [bounds.x, bounds.y, bounds.width, bounds.height]
                .into_iter()
                .zip(expected_bounds)
            {
                assert!(
                    (actual - expected * scale).abs() < 1e-10,
                    "{text} ink {actual} vs {}",
                    expected * scale
                );
            }
        }
    }

    #[test]
    fn vertical_layout_matches_native_harfbuzz_glyph_ids_clusters_and_origins() {
        let mut fonts = FontRegistry::new();
        fonts
            .register(
                include_bytes!("../../../tests/fonts/KartisFixtureVertical-Regular.ttf").as_slice(),
                0,
            )
            .unwrap();
        let vertical = fonts.clone().with_writing_mode(RasterWritingMode::Vertical);
        let data = include_str!("../../../tests/fixtures/vertical-harfbuzz.txt");
        let size = 40.;
        let scale = size / 1000.;
        for line in data.lines().filter(|line| !line.starts_with('#')) {
            let mut fields = line.split(';');
            let text: String = fields
                .next()
                .unwrap()
                .split_ascii_whitespace()
                .map(|c| char::from_u32(u32::from_str_radix(c, 16).unwrap()).unwrap())
                .collect();
            let advance: f64 = fields.next().unwrap().parse().unwrap();
            let expected = fields.next().unwrap().split_ascii_whitespace();
            let shaped = vertical
                .shape(&text, size, MissingGlyphPolicy::Error)
                .unwrap();
            assert!((shaped.advance - advance * scale).abs() < 1e-12, "{text}");
            assert_eq!(shaped.glyphs.len(), expected.clone().count(), "{text}");
            for (glyph, expected) in shaped.glyphs.iter().zip(expected) {
                let tuple: Vec<i32> = expected.split(',').map(|n| n.parse().unwrap()).collect();
                assert_eq!(i32::from(glyph.id.0), tuple[0], "{text}");
                assert_eq!(glyph.cluster as i32, tuple[1], "{text}");
                assert!(
                    (glyph.x - f64::from(tuple[2]) * scale).abs() < 1e-12,
                    "{text}"
                );
                assert!(
                    (glyph.y - f64::from(tuple[3]) * scale).abs() < 1e-12,
                    "{text}"
                );
            }
        }
        // The alternate punctuation glyph exists in the font and is selected
        // in vertical writing even when there is no surrounding strong script.
        let punctuation = |registry: &FontRegistry| {
            registry
                .shape("。", size, MissingGlyphPolicy::Error)
                .unwrap()
                .glyphs[0]
                .id
        };
        assert_ne!(punctuation(&vertical), punctuation(&fonts));
    }
}
