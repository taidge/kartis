//! Bounded, measurement-driven wrapping into ordinary drawable Scene labels.
//! This is an explicit whitespace/grapheme profile, not CSS or full UAX #14.
use std::collections::BTreeMap;

use unicode_segmentation::UnicodeSegmentation;

use crate::scene::ensure;
use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrapMode {
    #[default]
    Words,
    Graphemes,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrapWhitespace {
    #[default]
    Collapse,
    Preserve,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrapLongWord {
    #[default]
    BreakGraphemes,
    Overflow,
    Error,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextWrapOverflow {
    #[default]
    Error,
    Clip,
    Ellipsis,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextWrapOptions {
    /// Maximum measured advance, not a bound on glyph ink or rotated AABBs.
    pub max_width: f64,
    pub line_height: f64,
    pub revision: u64,
    pub max_lines: usize,
    pub max_measurements: usize,
    /// Sum of UTF-8 bytes submitted to measurement, bounding repeated shaping.
    pub max_measured_bytes: usize,
    pub mode: TextWrapMode,
    pub whitespace: TextWrapWhitespace,
    pub long_word: TextWrapLongWord,
    pub overflow: TextWrapOverflow,
}
impl Default for TextWrapOptions {
    fn default() -> Self {
        Self {
            max_width: 160.,
            line_height: 1.2,
            revision: 0,
            max_lines: 64,
            max_measurements: 2048,
            max_measured_bytes: 1_048_576,
            mode: Default::default(),
            whitespace: Default::default(),
            long_word: Default::default(),
            overflow: Default::default(),
        }
    }
}
impl TextWrapOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.max_width.is_finite() && self.max_width > 0. && self.max_width <= 1_000_000.,
            "invalid wrap width",
        )?;
        ensure(
            (1..=256).contains(&self.max_lines)
                && (1..=65_536).contains(&self.max_measurements)
                && (1..=16_777_216).contains(&self.max_measured_bytes),
            "invalid wrap work budget",
        )?;
        SemanticTextProfile {
            line_height: self.line_height,
            revision: self.revision,
        }
        .validate()
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct WrappedLabelLines {
    pub labels: Vec<Label>,
    pub measurements: Vec<SemanticTextMeasurement>,
    pub truncated: bool,
}

fn space(g: &str) -> bool {
    matches!(g, " " | "\t" | "\u{000b}" | "\u{000c}")
}

struct Measure<'a, F> {
    font: &'a ResolvedFont,
    size: f64,
    options: &'a TextWrapOptions,
    callback: &'a mut F,
    cache: BTreeMap<String, SemanticTextMeasurement>,
    bytes: usize,
}
impl<F: FnMut(&SemanticTextRequest) -> Result<SemanticTextMeasurement>> Measure<'_, F> {
    fn get(&mut self, text: &str) -> Result<SemanticTextMeasurement> {
        if let Some(value) = self.cache.get(text) {
            return Ok(value.clone());
        }
        ensure(
            self.cache.len() < self.options.max_measurements,
            "wrap measurement budget exceeded",
        )?;
        self.bytes = self
            .bytes
            .checked_add(text.len())
            .ok_or_else(|| ChartError("wrap byte budget overflow".into()))?;
        ensure(
            self.bytes <= self.options.max_measured_bytes,
            "wrap measured-byte budget exceeded",
        )?;
        let request = SemanticTextRequest {
            whitespace: LabelWhitespace::Preserve,
            text: text.into(),
            size: self.size,
            font: self.font.clone(),
            line_height: self.size * self.options.line_height,
            revision: self.options.revision,
        };
        request.validate()?;
        let value = (self.callback)(&request)?;
        value.validate_for(&request)?;
        self.cache.insert(text.into(), value.clone());
        Ok(value)
    }
    fn fits(&mut self, text: &str) -> Result<bool> {
        Ok(self.get(text)?.metrics().advance <= self.options.max_width)
    }
}

/// Measure each candidate using the exact resolved font and resource revision.
/// Callbacks are borrowed and never retained. Their external side effects are
/// not rolled back on failure. Lines retain the original anchor and rotation;
/// baseline offsets follow the rotated block axis about the original anchor.
pub fn wrap_label(
    label: &Label,
    font: &ResolvedFont,
    options: &TextWrapOptions,
    measurer: &mut impl FnMut(&SemanticTextRequest) -> Result<SemanticTextMeasurement>,
) -> Result<WrappedLabelLines> {
    options.validate()?;
    font.validate()?;
    ensure(
        label.text.len() <= MAX_LABEL_BYTES
            && label.x.is_finite()
            && label.y.is_finite()
            && label.rotation.is_finite()
            && (-360. ..=360.).contains(&label.rotation),
        "invalid wrap label",
    )?;
    let profile = SemanticTextProfile {
        line_height: options.line_height,
        revision: options.revision,
    };
    profile.request(label, font)?;
    let mut measure = Measure {
        font,
        size: label.size,
        options,
        callback: measurer,
        cache: BTreeMap::new(),
        bytes: 0,
    };
    let normalized = label
        .text
        .replace("\r\n", "\n")
        .replace(['\r', '\u{2028}', '\u{2029}'], "\n");
    let mut strings = Vec::new();
    let mut truncated = false;
    'paragraphs: for paragraph in normalized.split('\n') {
        let mut text = String::new();
        let mut pending = false;
        for g in paragraph.graphemes(true) {
            if options.whitespace == TextWrapWhitespace::Collapse && space(g) {
                pending = !text.is_empty();
            } else {
                if pending {
                    text.push(' ');
                }
                pending = false;
                text.push_str(g);
            }
        }
        let mut rest = text.as_str();
        loop {
            if strings.len() == options.max_lines {
                truncated = true;
                break 'paragraphs;
            }
            if rest.is_empty() {
                strings.push(String::new());
                break;
            }
            let graphemes = rest.grapheme_indices(true).collect::<Vec<_>>();
            let mut fitting = None;
            let mut fitting_word = None;
            let mut first_word_end = None;
            for (i, (start, g)) in graphemes.iter().enumerate() {
                let end = start + g.len();
                if first_word_end.is_none() && space(g) {
                    first_word_end = Some(if *start == 0 { end } else { *start });
                }
                // Admission must measure the text that will actually be drawn;
                // collapsed trailing separators are consumed, not painted.
                let candidate = &rest[..end];
                let candidate = if options.whitespace == TextWrapWhitespace::Collapse {
                    candidate.trim_end_matches([' ', '\t', '\u{000b}', '\u{000c}'])
                } else {
                    candidate
                };
                let fit = measure.fits(candidate)?;
                if fit {
                    fitting = Some(end);
                }
                if fit && (space(g) || i + 1 == graphemes.len()) {
                    fitting_word = Some(end);
                }
            }
            let end = if options.mode == TextWrapMode::Graphemes {
                fitting
            } else {
                fitting_word.or_else(|| {
                    (options.long_word == TextWrapLongWord::BreakGraphemes)
                        .then_some(fitting)
                        .flatten()
                })
            };
            let end = match end {
                Some(end) => end,
                None if options.long_word == TextWrapLongWord::Overflow => {
                    if options.mode == TextWrapMode::Graphemes {
                        graphemes[0].0 + graphemes[0].1.len()
                    } else {
                        first_word_end.unwrap_or(rest.len())
                    }
                }
                None => {
                    return Err(ChartError(
                        "wrap unbreakable grapheme or word exceeds width".into(),
                    ));
                }
            };
            let line = &rest[..end];
            strings.push(if options.whitespace == TextWrapWhitespace::Collapse {
                line.trim_end_matches([' ', '\t', '\u{000b}', '\u{000c}'])
                    .into()
            } else {
                line.into()
            });
            rest = &rest[end..];
            if options.whitespace == TextWrapWhitespace::Collapse {
                rest = rest.trim_start_matches([' ', '\t', '\u{000b}', '\u{000c}']);
            }
            if rest.is_empty() {
                break;
            }
        }
    }
    if truncated {
        match options.overflow {
            TextWrapOverflow::Error => return Err(ChartError("wrap line budget exceeded".into())),
            TextWrapOverflow::Clip => {}
            TextWrapOverflow::Ellipsis => {
                ensure(measure.fits("…")?, "wrap ellipsis exceeds width")?;
                let last = strings.last_mut().expect("positive line budget");
                let mut best = String::from("…");
                for (start, g) in last.grapheme_indices(true) {
                    let trial = format!("{}…", &last[..start + g.len()]);
                    if trial.len() <= MAX_LABEL_BYTES && measure.fits(&trial)? {
                        best = trial;
                    }
                }
                *last = best;
            }
        }
    }
    let mut labels = Vec::new();
    let mut measurements = Vec::new();
    let (sin, cos) = label.rotation.to_radians().sin_cos();
    for (i, text) in strings.into_iter().enumerate() {
        let measurement = measure.get(&text)?;
        let mut line = label.clone();
        line.text = text;
        let offset = i as f64 * label.size * options.line_height;
        line.x = label.x - sin * offset;
        line.y = label.y + cos * offset;
        ensure(
            line.x.is_finite()
                && line.y.is_finite()
                && line.x.abs() <= 1_000_000.
                && line.y.abs() <= 1_000_000.,
            "wrapped label position overflow",
        )?;
        labels.push(line);
        measurements.push(measurement);
    }
    Ok(WrappedLabelLines {
        labels,
        measurements,
        truncated,
    })
}

/// Replace one Scene label transactionally, including all index-based metadata.
/// Existing font declarations, series ownership and clipping are copied to each
/// generated line. Explicit paint order expands the old label slot in place.
pub fn wrap_scene_label(
    scene: &mut Scene,
    index: usize,
    options: &TextWrapOptions,
    measurer: &mut impl FnMut(&SemanticTextRequest) -> Result<SemanticTextMeasurement>,
) -> Result<WrappedLabelLines> {
    scene.validate()?;
    let label = scene
        .labels
        .get(index)
        .ok_or_else(|| ChartError("wrap label index out of range".into()))?;
    let wrapped = wrap_label(label, &scene.resolved_label_font(index), options, measurer)?;
    ensure(
        scene.labels.len() - 1 + wrapped.labels.len() <= MAX_MARKS,
        "wrapped Scene label budget exceeded",
    )?;
    let count = wrapped.labels.len();
    // Admit the final duplicated clip cost before cloning any path payload.
    let mut clip_cost = [0usize; 3];
    for (clip, copies) in scene
        .label_clips
        .iter()
        .map(|(slot, clip)| (clip, if *slot == index { count } else { 1 }))
        .chain(
            scene
                .marks
                .iter()
                .filter_map(|mark| mark.clip_path.as_deref().map(|clip| (clip, 1))),
        )
    {
        let stats = clip.picking_stats()?;
        for (i, cost) in [clip.command_count(), stats.flattened_segments, stats.work]
            .into_iter()
            .enumerate()
        {
            clip_cost[i] = cost
                .checked_mul(copies)
                .and_then(|cost| clip_cost[i].checked_add(cost))
                .ok_or_else(|| ChartError("wrapped clip budget overflow".into()))?;
        }
    }
    ensure(
        clip_cost[0] <= 200_000 && clip_cost[1] <= 200_000 && clip_cost[2] <= 4_000_000,
        "wrapped Scene clip budget exceeded",
    )?;
    let mut candidate = scene.clone();
    candidate
        .labels
        .splice(index..=index, wrapped.labels.clone());
    fn remap<T: Clone>(old: &BTreeMap<usize, T>, index: usize, count: usize) -> BTreeMap<usize, T> {
        let mut new = BTreeMap::new();
        for (slot, value) in old {
            if *slot < index {
                new.insert(*slot, value.clone());
            } else if *slot == index {
                for i in 0..count {
                    new.insert(index + i, value.clone());
                }
            } else {
                new.insert(slot - 1 + count, value.clone());
            }
        }
        new
    }
    candidate.typography.labels = remap(&scene.typography.labels, index, count);
    candidate.typography.label_strokes = remap(&scene.typography.label_strokes, index, count);
    candidate.typography.label_whitespace = remap(&scene.typography.label_whitespace, index, count);
    for slot in index..index + count {
        candidate
            .typography
            .label_whitespace
            .insert(slot, LabelWhitespace::Preserve);
    }
    candidate.label_series = remap(&scene.label_series, index, count);
    candidate.label_clips = remap(&scene.label_clips, index, count);
    candidate.draw_order = scene
        .draw_order
        .iter()
        .flat_map(|item| match item {
            DrawItem::Label(slot) if *slot == index => (0..count)
                .map(|i| DrawItem::Label(index + i))
                .collect::<Vec<_>>(),
            DrawItem::Label(slot) if *slot > index => vec![DrawItem::Label(slot - 1 + count)],
            other => vec![*other],
        })
        .collect();
    candidate.validate()?;
    *scene = candidate;
    Ok(wrapped)
}
