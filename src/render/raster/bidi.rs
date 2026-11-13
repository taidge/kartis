//! UAX #9 visual runs and per-run OpenType shaping. The Unicode order uses
//! logical byte ranges, so the shaping engine still receives logical text.
use super::text_runs::{bidi_control, clusters, normalized, shape_runs};
use super::{
    Glyph, MAX_COORDINATE, MAX_SHAPED_GLYPHS, MissingGlyphPolicy, RasterWritingMode, Text,
};
use crate::Result;
use crate::scene::ensure;
use rustybuzz::{BufferFlags, Direction, Face, UnicodeBuffer};
use std::ops::Range;
use unicode_bidi::{BidiInfo, Level};

/// Paragraph direction for one native raster or outlined label.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BidiBaseDirection {
    /// Use the first strong character outside isolates, or LTR if none exists.
    #[default]
    Auto,
    LeftToRight,
    RightToLeft,
}

impl BidiBaseDirection {
    fn level(self) -> Option<Level> {
        match self {
            Self::Auto => None,
            Self::LeftToRight => Some(Level::ltr()),
            Self::RightToLeft => Some(Level::rtl()),
        }
    }
}

pub(super) fn shape(
    text: &str,
    size: f64,
    missing: MissingGlyphPolicy,
    base: BidiBaseDirection,
    faces: &[Face<'_>],
) -> Result<Text> {
    let (line, offsets) = normalized(text)?;
    let mut result = Text {
        glyphs: Vec::new(),
        advance: 0.,
        columns: Vec::new(),
    };
    for (bytes, rtl) in visual_runs(&line, base) {
        let clusters = clusters(&line, bytes);
        let mut runs = shape_runs(
            &line,
            &clusters,
            faces,
            missing,
            RasterWritingMode::Horizontal,
        )?;
        if rtl {
            runs.reverse();
        }
        for run in runs {
            let mut buffer = UnicodeBuffer::new();
            for (index, c) in line[run.bytes.clone()].char_indices() {
                if !bidi_control(c) {
                    buffer.add(c, offsets[run.bytes.start + index]);
                }
            }
            buffer.set_direction(if rtl {
                Direction::RightToLeft
            } else {
                Direction::LeftToRight
            });
            let tag = ttf_parser::Tag::from_bytes_lossy(run.script.short_name().as_bytes());
            if let Some(script) = rustybuzz::Script::from_iso15924_tag(tag) {
                buffer.set_script(script);
            }
            buffer.set_flags(BufferFlags::REMOVE_DEFAULT_IGNORABLES);
            let face = &faces[run.font];
            let scale = size / f64::from(face.units_per_em());
            let shaped = rustybuzz::shape(face, &[], buffer);
            ensure(
                result.glyphs.len() + shaped.len() <= MAX_SHAPED_GLYPHS,
                "shaped glyph budget exceeded",
            )?;
            for (glyph, position) in shaped.glyph_infos().iter().zip(shaped.glyph_positions()) {
                result.glyphs.push(Glyph {
                    font: run.font,
                    id: ttf_parser::GlyphId(glyph.glyph_id as u16),
                    #[cfg(test)]
                    cluster: glyph.cluster,
                    x: result.advance + f64::from(position.x_offset) * scale,
                    y: -f64::from(position.y_offset) * scale,
                    size: scale,
                    sideways: false,
                });
                result.advance += f64::from(position.x_advance) * scale;
            }
        }
    }
    ensure(
        result.advance.is_finite() && result.advance.abs() <= MAX_COORDINATE,
        "raster text advance overflow",
    )?;
    Ok(result)
}

fn visual_runs(line: &str, base: BidiBaseDirection) -> Vec<(Range<usize>, bool)> {
    let info = BidiInfo::new(line, base.level());
    // Normalization makes this one paragraph. Iterating also handles the
    // empty input defensively.
    let mut visual = Vec::new();
    for paragraph in &info.paragraphs {
        let (levels, runs) = info.visual_runs(paragraph, paragraph.range.clone());
        visual.extend(runs.into_iter().map(|bytes| {
            let rtl = levels[bytes.start].is_rtl();
            (bytes, rtl)
        }));
    }
    visual
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::raster::FontRegistry;

    fn direction(value: &str) -> BidiBaseDirection {
        match value {
            "0" | "ltr" => BidiBaseDirection::LeftToRight,
            "1" | "rtl" => BidiBaseDirection::RightToLeft,
            "2" | "auto" => BidiBaseDirection::Auto,
            _ => panic!("invalid oracle base direction"),
        }
    }

    fn text(value: &str) -> String {
        value
            .split_ascii_whitespace()
            .map(|c| char::from_u32(u32::from_str_radix(c, 16).unwrap()).unwrap())
            .collect()
    }

    fn cases(data: &str) -> impl Iterator<Item = &str> {
        data.lines()
            .filter(|line| !line.is_empty() && !line.starts_with('#'))
    }

    #[test]
    fn unicode_16_visual_runs_match_uax9_conformance_cases() {
        let data = include_str!("../../../tests/fixtures/bidi-unicode-16.txt");
        let mut count = 0;
        for row in cases(data) {
            let parts: Vec<_> = row.split(';').collect();
            let input = text(parts[0]);
            let starts: Vec<_> = input.char_indices().map(|(i, _)| i).collect();
            let removed: Vec<_> = parts[3]
                .split_ascii_whitespace()
                .map(|s| s == "x")
                .collect();
            let mut actual = Vec::new();
            for (range, rtl) in visual_runs(&input, direction(parts[1])) {
                let mut chars: Vec<_> = input[range.clone()]
                    .char_indices()
                    .map(|(i, _)| starts.binary_search(&(i + range.start)).unwrap())
                    .filter(|i| !removed[*i])
                    .collect();
                if rtl {
                    chars.reverse();
                }
                actual.extend(chars);
            }
            let expected: Vec<usize> = parts[4]
                .split_ascii_whitespace()
                .map(|v| v.parse().unwrap())
                .collect();
            assert_eq!(actual, expected, "UAX #9 row {row}");
            count += 1;
        }
        assert_eq!(count, 96);
    }

    #[test]
    fn native_harfbuzz_oracle_covers_glyphs_logical_clusters_and_positions() {
        let mut registry = FontRegistry::new();
        for bytes in [
            include_bytes!("../../../tests/fonts/ABeeZee-Regular.ttf").as_slice(),
            include_bytes!("../../../tests/fonts/Lato-Regular.ttf").as_slice(),
            include_bytes!("../../../tests/fonts/KartisFixtureHebrew-Regular.ttf").as_slice(),
            include_bytes!("../../../tests/fonts/KartisFixtureArabic-Regular.ttf").as_slice(),
        ] {
            registry.register(bytes, 0).unwrap();
        }
        let data = include_str!("../../../tests/fixtures/bidi-harfbuzz.txt");
        let mut count = 0;
        for row in cases(data) {
            let parts: Vec<_> = row.split(';').collect();
            let input = text(parts[1]);
            let shaped = registry
                .clone()
                .with_bidi(direction(parts[0]))
                .shape(&input, 1., MissingGlyphPolicy::Error)
                .unwrap();
            let width: f64 = parts[2].parse().unwrap();
            assert!((shaped.advance - width).abs() < 1e-12, "{input:?}");
            let records: Vec<_> = parts[3].split_ascii_whitespace().collect();
            assert_eq!(shaped.glyphs.len(), records.len(), "{input:?}");
            for (glyph, record) in shaped.glyphs.iter().zip(records) {
                let fields: Vec<_> = record.split(',').collect();
                let x: f64 = fields[3].parse().unwrap();
                let y: f64 = fields[4].parse().unwrap();
                assert_eq!(glyph.font, fields[0].parse::<usize>().unwrap(), "{input:?}");
                assert_eq!(glyph.id.0, fields[1].parse::<u16>().unwrap(), "{input:?}");
                assert_eq!(
                    glyph.cluster,
                    fields[2].parse::<u32>().unwrap(),
                    "{input:?}"
                );
                assert!((glyph.x - x).abs() < 1e-12, "{input:?}");
                assert!((glyph.y - y).abs() < 1e-12, "{input:?}");
            }
            count += 1;
        }
        assert_eq!(count, 10);
    }
}
