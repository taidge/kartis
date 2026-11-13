//! Explicit text-to-path conversion for standalone SVG, Canvas and CPU Scenes.
use std::collections::{BTreeMap, BTreeSet};

use crate::render::raster::FontRegistry;
use crate::scene::{MAX_MARKS, ensure};
use crate::{ClipPath, DrawItem, Geometry, Mark, PaintedLabelOptions, Result, Scene};

// Match Scene::validate admission limits before multiplying owned clip paths.
const MAX_COMMANDS: usize = 200_000;
const MAX_CLIP_WORK: usize = 4_000_000;

#[derive(Default)]
struct ClipBudget {
    commands: usize,
    segments: usize,
    work: usize,
}
impl ClipBudget {
    fn include(&mut self, clip: &ClipPath, copies: usize) -> Result<()> {
        let stats = clip.picking_stats()?;
        let accumulate = |total: usize, each: usize, limit: usize| -> Result<usize> {
            let next = each
                .checked_mul(copies)
                .and_then(|added| total.checked_add(added));
            ensure(
                next.is_some_and(|next| next <= limit),
                "painted scene clip command/picking budget exceeded",
            )?;
            Ok(next.unwrap())
        };
        self.commands = accumulate(self.commands, clip.command_count(), MAX_COMMANDS)?;
        self.segments = accumulate(self.segments, stats.flattened_segments, MAX_COMMANDS)?;
        self.work = accumulate(self.work, stats.work, MAX_CLIP_WORK)?;
        Ok(())
    }
}

/// Return a text-to-path copy for existing standalone Scene renderers.
/// Selected labels become decorative solid-color path marks at the original
/// paint position, retaining series ownership and clips. Remaining Text slots
/// and their ownership/clip indices are remapped. The source Scene is retained
/// by the caller for text editing and accessibility of converted labels.
///
/// Namespaces use 1..=128 ASCII letters, digits, `_`, `-`, or `.`, beginning
/// with a letter or `_`. Generated IDs are stable for label/glyph/layer slots.
/// Collisions with source mark IDs reject the complete conversion.
///
/// Complete layers can exceed Glory Graphic's 256-command limit. Scene path,
/// mark and expanded clip budgets apply before any clip is copied per layer.
pub fn scene_with_painted_labels(
    scene: &Scene,
    fonts: &FontRegistry,
    options: &PaintedLabelOptions,
    namespace: &str,
) -> Result<Scene> {
    scene.validate()?;
    options.validate()?;
    ensure(
        !namespace.is_empty()
            && namespace.len() <= 128
            && namespace.bytes().enumerate().all(|(index, byte)| {
                byte.is_ascii_alphabetic()
                    || byte == b'_'
                    || index > 0 && (byte.is_ascii_digit() || byte == b'-' || byte == b'.')
            }),
        "invalid painted scene namespace",
    )?;
    let selected: Vec<_> = scene
        .labels
        .iter()
        .map(|label| !options.rotated_only || label.rotation != 0.)
        .collect();
    if !selected.iter().any(|selected| *selected) {
        return Ok(scene.clone());
    }
    let source_ids: BTreeSet<_> = scene.marks.iter().map(|mark| mark.id.as_str()).collect();
    let existing_commands: usize = scene
        .marks
        .iter()
        .map(|mark| match &mark.geometry {
            Geometry::Path(path) => path.len(),
            _ => 0,
        })
        .sum();
    let mut clip_budget = ClipBudget::default();
    for clip in scene
        .marks
        .iter()
        .filter_map(|mark| mark.clip_path.as_deref())
    {
        clip_budget.include(clip, 1)?;
    }
    for (&index, clip) in &scene.label_clips {
        if !selected[index] {
            clip_budget.include(clip, 1)?;
        }
    }

    let mut additions = Vec::new();
    let mut ranges = BTreeMap::new();
    let mut glyph_count = 0;
    let mut command_count = 0;
    for (index, label) in scene.labels.iter().enumerate() {
        if !selected[index] {
            continue;
        }
        let painted = if scene.typography.has_label_font(index) {
            fonts.painted_label_with_font(
                label,
                &scene.resolved_label_font(index),
                options.missing_glyph,
            )?
        } else {
            fonts.painted_label(label, options.missing_glyph)?
        };
        glyph_count += painted.glyph_count;
        ensure(
            glyph_count <= options.max_glyphs,
            "painted scene glyph budget exceeded",
        )?;
        ensure(
            additions.len() + painted.layers.len() <= options.max_layers
                && scene.marks.len() + additions.len() + painted.layers.len() <= MAX_MARKS,
            "painted scene mark/layer budget exceeded",
        )?;
        if let Some(clip) = scene.label_clips.get(&index) {
            clip_budget.include(clip, painted.layers.len())?;
        }
        let start = scene.marks.len() + additions.len();
        for layer in painted.layers {
            command_count += layer.commands.len();
            ensure(
                command_count <= options.max_commands
                    && existing_commands + command_count <= MAX_COMMANDS,
                "painted scene path command budget exceeded",
            )?;
            let id = format!(
                "kartis-painted-{namespace}:label:{index}:glyph:{}:layer:{}",
                layer.glyph, layer.layer
            );
            ensure(
                !source_ids.contains(id.as_str()),
                "painted scene generated mark ID collision",
            )?;
            let mut mark = Mark::new(id, Geometry::Path(layer.commands), layer.color);
            mark.label.clear();
            if let Some(owner) = scene.label_series.get(&index) {
                mark.series.clone_from(owner);
            }
            additions.push(mark);
        }
        ranges.insert(index, start..scene.marks.len() + additions.len());
    }

    // All final geometry and clip multipliers have been admitted. Only now
    // clone the input's owned masks, including one mask per generated layer.
    for (&index, range) in &ranges {
        if let Some(clip) = scene.label_clips.get(&index) {
            for mark in
                &mut additions[range.start - scene.marks.len()..range.end - scene.marks.len()]
            {
                mark.clip_path = Some(Box::new(clip.clone()));
            }
        }
    }
    let mut result = Scene {
        width: scene.width,
        height: scene.height,
        title: scene.title.clone(),
        background: scene.background,
        marks: scene.marks.clone(),
        opacity_groups: scene.opacity_groups.clone(),
        labels: Vec::new(),
        legend: scene.legend.clone(),
        legend_roles: scene.legend_roles.clone(),
        legend_targets: scene.legend_targets.clone(),
        draw_order: Vec::new(),
        label_series: BTreeMap::new(),
        label_clips: BTreeMap::new(),
        typography: scene.typography.clone(),
    };
    result.typography.labels.clear();
    result.marks.extend(additions);
    let mut remaining = BTreeMap::new();
    for (index, label) in scene.labels.iter().enumerate() {
        if selected[index] {
            continue;
        }
        let next = result.labels.len();
        remaining.insert(index, next);
        result.labels.push(label.clone());
        if let Some(font) = scene.typography.labels.get(&index) {
            result.typography.labels.insert(next, font.clone());
        }
        if let Some(owner) = scene.label_series.get(&index) {
            result.label_series.insert(next, owner.clone());
        }
        if let Some(clip) = scene.label_clips.get(&index) {
            result.label_clips.insert(next, clip.clone());
        }
    }
    for item in scene.draw_items() {
        match item {
            DrawItem::Mark(index) => result.draw_order.push(DrawItem::Mark(index)),
            DrawItem::Label(index) => {
                if let Some(range) = ranges.get(&index) {
                    result.draw_order.extend(range.clone().map(DrawItem::Mark));
                } else {
                    result.draw_order.push(DrawItem::Label(remaining[&index]));
                }
            }
        }
    }
    result.validate()?;
    Ok(result)
}
