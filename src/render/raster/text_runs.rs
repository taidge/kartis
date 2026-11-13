//! Shared Unicode preprocessing and font fallback for script-aware text.
use std::ops::Range;

use rustybuzz::Face;
use unicode_script::{Script, ScriptExtension, UnicodeScript};
use unicode_segmentation::UnicodeSegmentation;

use super::{MissingGlyphPolicy, RasterWritingMode};
use crate::scene::ensure;
use crate::{ChartError, Result};

pub(super) fn normalized(text: &str) -> Result<(String, Vec<u32>)> {
    let mut line = String::with_capacity(text.len());
    let mut offsets = Vec::with_capacity(text.len());
    for (offset, c) in text.char_indices() {
        let c = match c {
            '\t' | '\n' | '\r' | '\u{c}' | '\u{85}' | '\u{2028}' | '\u{2029}' => ' ',
            _ => c,
        };
        ensure(!c.is_control(), "control character in raster text")?;
        line.push(c);
        offsets.resize(line.len(), offset as u32);
    }
    Ok((line, offsets))
}

pub(super) fn bidi_control(c: char) -> bool {
    matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

// Shaper controls which need neither a cmap entry nor an outline. Keep
// them in the buffer for joining, normalization and variation selection.
fn shaping_control(c: char) -> bool {
    matches!(c, '\u{ad}' | '\u{34f}' | '\u{180b}'..='\u{180d}' | '\u{180f}' | '\u{200b}'..='\u{200d}' | '\u{2060}' | '\u{fe00}'..='\u{fe0f}' | '\u{feff}' | '\u{e0100}'..='\u{e01ef}')
}

#[derive(Clone)]
pub(super) struct Cluster {
    bytes: Range<usize>,
    script: Script,
    extensions: ScriptExtension,
}

fn specific(script: Script) -> bool {
    !matches!(script, Script::Common | Script::Inherited | Script::Unknown)
}

pub(super) fn clusters(line: &str, run: Range<usize>) -> Vec<Cluster> {
    let mut clusters = Vec::new();
    for (offset, grapheme) in line[run.clone()].grapheme_indices(true) {
        let chars = || grapheme.chars().filter(|c| !bidi_control(*c));
        if chars().next().is_none() {
            continue;
        }
        let script = chars()
            .map(|c| c.script())
            .find(|script| specific(*script))
            .unwrap_or(Script::Common);
        let extensions = chars().fold(Script::Common.into(), |ext: ScriptExtension, c| {
            ext.intersection(c.script_extension())
        });
        let start = run.start + offset;
        clusters.push(Cluster {
            bytes: start..start + grapheme.len(),
            script,
            extensions,
        });
    }
    let mut following = vec![None; clusters.len()];
    let mut next = None;
    for (cluster, right) in clusters.iter().zip(&mut following).rev() {
        *right = next;
        if specific(cluster.script) {
            next = Some(cluster.script);
        }
    }
    let mut previous = None;
    for (cluster, right) in clusters.iter_mut().zip(following) {
        if !specific(cluster.script) {
            cluster.script = previous
                .filter(|s| cluster.extensions.contains_script(*s))
                .or_else(|| right.filter(|s| cluster.extensions.contains_script(*s)))
                .or_else(|| {
                    let mut scripts = cluster.extensions.iter().filter(|s| specific(*s));
                    let first = scripts.next();
                    if scripts.next().is_none() {
                        first
                    } else {
                        None
                    }
                })
                .unwrap_or(Script::Common);
        }
        previous = specific(cluster.script).then_some(cluster.script);
    }
    clusters
}

// These scripts can change faces at an extended grapheme boundary. For
// contextual scripts (including Arabic), require one face for the whole
// resolved script span to keep joining and OpenType substitutions intact.
fn per_grapheme_fallback(script: Script, mode: RasterWritingMode) -> bool {
    matches!(
        script,
        Script::Common | Script::Latin | Script::Greek | Script::Cyrillic | Script::Hebrew
    ) || (mode == RasterWritingMode::Vertical
        && matches!(
            script,
            Script::Han | Script::Hiragana | Script::Katakana | Script::Hangul
        ))
}

pub(super) struct ShapeRun {
    pub(super) bytes: Range<usize>,
    pub(super) script: Script,
    pub(super) font: usize,
}

fn fallback(
    line: &str,
    bytes: Range<usize>,
    faces: &[Face<'_>],
    missing: MissingGlyphPolicy,
    mode: RasterWritingMode,
) -> Result<Option<usize>> {
    let text = &line[bytes];
    let needs_font = || {
        text.chars()
            .filter(|&c| !bidi_control(c) && !shaping_control(c))
    };
    if let Some(font) = faces
        .iter()
        .position(|face| needs_font().all(|c| face.glyph_index(c).is_some()))
    {
        return Ok(Some(font));
    }
    match missing {
        MissingGlyphPolicy::Error => Err(ChartError(
            match mode {
                RasterWritingMode::Horizontal => {
                    "no registered font covers a raster bidi grapheme or contextual script span"
                }
                RasterWritingMode::Vertical => {
                    "no registered font covers a raster vertical grapheme or contextual script span"
                }
            }
            .into(),
        )),
        MissingGlyphPolicy::Skip => Ok(None),
        MissingGlyphPolicy::Notdef => Ok(Some(0)),
    }
}

pub(super) fn shape_runs(
    line: &str,
    clusters: &[Cluster],
    faces: &[Face<'_>],
    missing: MissingGlyphPolicy,
    mode: RasterWritingMode,
) -> Result<Vec<ShapeRun>> {
    let mut runs: Vec<ShapeRun> = Vec::new();
    let mut boundary = false;
    for span in clusters.chunk_by(|a, b| a.script == b.script) {
        let script = span[0].script;
        if !per_grapheme_fallback(script, mode) {
            let bytes = span[0].bytes.start..span[span.len() - 1].bytes.end;
            if let Some(font) = fallback(line, bytes.clone(), faces, missing, mode)? {
                runs.push(ShapeRun {
                    bytes,
                    script,
                    font,
                });
            }
            boundary = true;
            continue;
        }
        for cluster in span {
            let Some(font) = fallback(line, cluster.bytes.clone(), faces, missing, mode)? else {
                boundary = true;
                continue;
            };
            if !boundary
                && let Some(last) = runs.last_mut()
                && last.font == font
                && last.script == script
            {
                last.bytes.end = cluster.bytes.end;
            } else {
                runs.push(ShapeRun {
                    bytes: cluster.bytes.clone(),
                    script,
                    font,
                });
            }
            boundary = false;
        }
    }
    Ok(runs)
}
