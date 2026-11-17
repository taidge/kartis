//! Optional measured Sunburst label placement, extending its Nivo-style labels.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::PathCommand;

use super::{SunburstArc, SunburstLayout, SunburstLayoutNode};
use crate::clip_path::RectCoverage;
use crate::scene::ensure;
use crate::{
    ChartError, ClipPath, ClipRule, DrawItem, Geometry, Label, LabelCandidate,
    LabelCollisionOptions, LayoutRect, MAX_LABEL_BYTES, Result, Scene, layout_labels,
};

pub const MAX_SUNBURST_LABEL_MASK_WORK: usize = 16_000_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SunburstLabelClip {
    /// Keep the measured ink inside the plot, scene and collision viewport.
    #[default]
    Plot,
    /// Retain bounds intersecting the arc and clip the text to its actual Path.
    /// The solver still uses the complete measured AABB for collision checks.
    ClipToArc,
    /// Admit only AABBs fully inside the actual padded and rounded arc Path.
    /// This conservative rule can omit rotated text whose ink fits the arc
    /// while an empty corner of its AABB crosses the boundary or inner hole.
    RequireFullInkBounds,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLabelCandidate {
    pub id: String,
    pub node_id: String,
    pub label: Label,
    pub font: crate::ResolvedFont,
    pub whitespace: crate::LabelWhitespace,
    pub arc: SunburstArc,
    /// The actual padded and rounded D3 arc, including its inner hole. Fill
    /// contours are explicitly closed to satisfy the shared ClipPath contract.
    pub mask: ClipPath,
    context: String,
}

impl SunburstLabelCandidate {
    fn signature(&self) -> String {
        let label = &self.label;
        format!(
            "{}:{}:{}:{}:{}:{}:{:x}:{:x}:{:x}:{:x}:{:?}:{:?}:{:?}:{:?}:{}:{}",
            self.id.len(),
            self.id,
            self.node_id.len(),
            self.node_id,
            label.text.len(),
            label.text,
            label.x.to_bits(),
            label.y.to_bits(),
            label.size.to_bits(),
            label.rotation.to_bits(),
            label.anchor,
            label.color,
            self.arc,
            self.mask,
            self.context,
            self.whitespace.measurement_signature(&self.font),
        )
    }

    /// Supply actual ink AABB relative to this label's anchor, including font
    /// bearings and rotation. Whitespace can use a zero-size rectangle.
    pub fn measured(&self, bounds: LayoutRect) -> SunburstMeasuredLabel {
        SunburstMeasuredLabel {
            id: self.id.clone(),
            signature: self.signature(),
            bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstMeasuredLabel {
    pub id: String,
    pub signature: String,
    pub bounds: LayoutRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstLabelPlacementOptions {
    pub collision: LabelCollisionOptions,
    pub clip: SunburstLabelClip,
    /// At most sixteen scene-coordinate alternatives after the authored anchor.
    pub offsets: Vec<(f64, f64)>,
    /// Larger priorities are placed first, then stable node ID order.
    pub priorities: BTreeMap<String, i32>,
    /// Hide only these nodes' labels, retaining candidates for later restoration.
    pub hidden: BTreeSet<String>,
    /// Cumulative adaptive path work for mask preparation and coverage queries.
    /// Scene validation separately enforces the shared rendering clip budgets.
    pub max_mask_work: usize,
}

impl Default for SunburstLabelPlacementOptions {
    fn default() -> Self {
        Self {
            collision: Default::default(),
            clip: Default::default(),
            offsets: vec![],
            priorities: BTreeMap::new(),
            hidden: BTreeSet::new(),
            max_mask_work: 4_000_000,
        }
    }
}

impl SunburstLabelPlacementOptions {
    pub fn for_layout(layout: &SunburstLayout) -> Self {
        let plot = layout.plot();
        Self {
            collision: LabelCollisionOptions {
                viewport: LayoutRect {
                    x: plot.x,
                    y: plot.y,
                    width: plot.width,
                    height: plot.height,
                },
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn validate(&self) -> Result<()> {
        self.collision.validate()?;
        ensure(
            self.offsets.len() <= 16
                && self.offsets.iter().all(|(x, y)| {
                    x.is_finite() && y.is_finite() && x.abs() <= 1e6 && y.abs() <= 1e6
                }),
            "invalid Sunburst label displacement budget",
        )?;
        ensure(
            self.priorities.len() <= 8000
                && self.hidden.len() <= 8000
                && self
                    .priorities
                    .keys()
                    .chain(&self.hidden)
                    .all(|id| !id.trim().is_empty() && id.len() <= 1000),
            "invalid Sunburst label priority or hidden identity budget",
        )?;
        ensure(
            (1..=MAX_SUNBURST_LABEL_MASK_WORK).contains(&self.max_mask_work),
            "invalid Sunburst label mask work budget",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SunburstPlacedLabel {
    pub id: String,
    pub node_id: String,
    pub bounds: Option<LayoutRect>,
    pub label_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SunburstLabelPlacementStats {
    pub mask_work: usize,
    pub mask_queries: usize,
}

impl SunburstLabelPlacementStats {
    fn charge(&mut self, work: usize, limit: usize) -> Result<()> {
        let total = self
            .mask_work
            .checked_add(work)
            .ok_or_else(|| ChartError("Sunburst label mask work overflow".into()))?;
        ensure(total <= limit, "Sunburst label mask work budget exceeded")?;
        self.mask_work = total;
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
struct LabelBinding {
    slot: Option<usize>,
    label: Label,
    clip: Option<ClipPath>,
    font: Option<crate::FontOptions>,
    whitespace: Option<crate::LabelWhitespace>,
    stroke: Option<crate::LabelStroke>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SunburstLabelState {
    candidates: Vec<SunburstLabelCandidate>,
    bindings: Vec<LabelBinding>,
    placements: Vec<SunburstPlacedLabel>,
    stats: SunburstLabelPlacementStats,
}

fn source_context(
    layout: &SunburstLayout,
    node: &SunburstLayoutNode,
    geometry: &Geometry,
) -> String {
    let parent = node.data.parent.as_deref().unwrap_or("");
    format!(
        "{}:{}:{}:{}:{}:{:x}:{:x}:{:?}:{:?}:{:?}:{:?}:{:?}",
        usize::from(node.data.parent.is_some()),
        parent.len(),
        parent,
        node.data.label.len(),
        node.data.label,
        node.data.value.to_bits(),
        node.value.to_bits(),
        node.arc,
        node.color,
        layout.frame,
        layout.plot(),
        geometry,
    )
}

fn arc_mask(geometry: &Geometry) -> Result<ClipPath> {
    let Geometry::Path(commands) = geometry else {
        return Err(ChartError(
            "Sunburst label mask requires an arc Path".into(),
        ));
    };
    let mut closed = Vec::with_capacity(commands.len() + 2);
    let mut open = false;
    for command in commands {
        match command {
            PathCommand::Move(_) => {
                if open {
                    closed.push(PathCommand::Close);
                }
                open = true;
            }
            PathCommand::Close => open = false,
            _ => {}
        }
        closed.push(*command);
    }
    if open {
        closed.push(PathCommand::Close);
    }
    Ok(ClipPath {
        geometry: Geometry::Path(closed),
        rule: ClipRule::NonZero,
    })
}

impl SunburstLabelState {
    pub(super) fn capture(layout: &SunburstLayout) -> Result<Self> {
        let mut rows = Vec::new();
        for node in &layout.nodes {
            let Some(slot) = node.label_index else {
                continue;
            };
            let label = layout
                .scene
                .labels
                .get(slot)
                .ok_or_else(|| ChartError("missing authored Sunburst label".into()))?;
            let mark = node
                .mark_index
                .and_then(|slot| layout.scene.marks.get(slot))
                .ok_or_else(|| ChartError("missing Sunburst label arc".into()))?;
            rows.push((
                SunburstLabelCandidate {
                    id: format!("sunburst-label:{}", node.data.id),
                    node_id: node.data.id.clone(),
                    label: label.clone(),
                    font: layout.scene.resolved_label_font(slot),
                    whitespace: layout.scene.typography.label_whitespace(slot),
                    arc: node.arc,
                    mask: arc_mask(&mark.geometry)?,
                    context: source_context(layout, node, &mark.geometry),
                },
                LabelBinding {
                    slot: Some(slot),
                    label: label.clone(),
                    clip: layout.scene.label_clips.get(&slot).cloned(),
                    font: layout.scene.typography.labels.get(&slot).cloned(),
                    whitespace: layout.scene.typography.label_whitespace.get(&slot).copied(),
                    stroke: layout.scene.typography.label_strokes.get(&slot).copied(),
                },
            ));
        }
        rows.sort_by(|a, b| a.0.node_id.cmp(&b.0.node_id));
        let (candidates, bindings) = rows.into_iter().unzip();
        Ok(Self {
            candidates,
            bindings,
            ..Default::default()
        })
    }
}

fn intersection(a: LayoutRect, b: LayoutRect) -> LayoutRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    LayoutRect {
        x,
        y,
        width: ((a.x + a.width).min(b.x + b.width) - x).max(0.),
        height: ((a.y + a.height).min(b.y + b.height) - y).max(0.),
    }
}

fn push_label(
    scene: &mut Scene,
    label: Label,
    owner: Option<&str>,
    clip: Option<&ClipPath>,
) -> usize {
    let slot = scene.labels.len();
    scene.labels.push(label);
    if let Some(owner) = owner {
        scene.label_series.insert(slot, owner.into());
    }
    if let Some(clip) = clip {
        scene.label_clips.insert(slot, clip.clone());
    }
    slot
}

impl SunburstLayout {
    /// Select single-line whitespace before obtaining ink measurements.
    pub fn with_label_whitespace(&self, mode: crate::LabelWhitespace) -> Result<Self> {
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            self.label_layout.placements.is_empty(),
            "select label whitespace before measured placement",
        )?;
        let mut result = self.clone();
        for binding in &self.label_layout.bindings {
            if let Some(slot) = binding.slot {
                result.scene.typography.label_whitespace.insert(slot, mode);
            }
        }
        result.scene.validate()?;
        result.label_layout = SunburstLabelState::capture(&result)?;
        Ok(result)
    }

    /// Select fonts for authored arc labels before measuring their ink.
    /// Additional Scene text and independently inherited declarations survive.
    pub fn with_label_font(&self, font: crate::FontOptions) -> Result<Self> {
        font.validate()?;
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            self.label_layout.placements.is_empty(),
            "select Sunburst label fonts before measured placement",
        )?;
        let mut result = self.clone();
        for binding in &self.label_layout.bindings {
            if let Some(slot) = binding.slot {
                result.scene.typography.labels.insert(
                    slot,
                    font.inherit(&binding.font.clone().unwrap_or_default()),
                );
            }
        }
        result.scene.validate()?;
        result.label_layout = SunburstLabelState::capture(&result)?;
        Ok(result)
    }

    pub fn label_candidates(&self) -> &[SunburstLabelCandidate] {
        &self.label_layout.candidates
    }

    pub fn label_placements(&self) -> &[SunburstPlacedLabel] {
        &self.label_layout.placements
    }

    pub fn label_placement_stats(&self) -> SunburstLabelPlacementStats {
        self.label_layout.stats
    }

    /// Rotate the authored labels around their visual center before measuring.
    /// Call before measured placement. The default Sunburst stays horizontal.
    pub fn with_label_rotation(&self, degrees: f64) -> Result<Self> {
        ensure(
            degrees.is_finite() && (-360. ..=360.).contains(&degrees),
            "invalid Sunburst label rotation",
        )?;
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            self.label_layout.placements.is_empty(),
            "rotate Sunburst labels before measured placement",
        )?;
        let mut result = self.clone();
        for (index, binding) in self.label_layout.bindings.iter().enumerate() {
            let Some(slot) = binding.slot else { continue };
            let label = &mut result.scene.labels[slot];
            if label.rotation == degrees {
                continue;
            }
            let old = label.rotation.to_radians();
            let next = degrees.to_radians();
            let baseline = label.size * 0.35;
            label.x += (old.sin() - next.sin()) * baseline;
            label.y += (next.cos() - old.cos()) * baseline;
            label.rotation = degrees;
            result.label_layout.candidates[index].label = label.clone();
            result.label_layout.bindings[index].label = label.clone();
        }
        result.scene.validate()?;
        result.validate_managed_labels()?;
        Ok(result)
    }

    /// Optional measured placement. Every current candidate must be measured
    /// once, including candidates hidden by these options. The greedy solver
    /// uses the supplied rotated ink AABBs and preserves label style and owners.
    /// Arc strategies use the actual rounded/padded Path, with bounded adaptive
    /// boundary error. Strict full-AABB containment can omit rotated text whose
    /// actual glyphs fit. Plot mode preserves the original unclipped arc-label
    /// behavior inside the chosen plot viewport. Managed edits and stale
    /// measurements return errors. Valid custom labels, clips and paint order
    /// are retained, and restored native labels append to the existing order.
    pub fn place_labels(
        &self,
        measurements: &[SunburstMeasuredLabel],
        options: &SunburstLabelPlacementOptions,
    ) -> Result<Self> {
        options.validate()?;
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            measurements.len() == self.label_layout.candidates.len(),
            "Sunburst measurements must cover all current label candidates exactly",
        )?;
        let mut supplied = BTreeMap::new();
        for measured in measurements {
            ensure(
                measured.id.len() <= MAX_LABEL_BYTES && measured.signature.len() <= 32_768,
                "Sunburst label measurement identity budget exceeded",
            )?;
            measured.bounds.validate()?;
            ensure(
                supplied.insert(measured.id.as_str(), measured).is_none(),
                "duplicate Sunburst label measurement",
            )?;
        }
        let plot = self.plot();
        let plot_rect = LayoutRect {
            x: plot.x,
            y: plot.y,
            width: plot.width,
            height: plot.height,
        };
        let mut collision = options.collision.clone();
        collision.viewport = intersection(
            intersection(collision.viewport, plot_rect),
            LayoutRect {
                x: 0.,
                y: 0.,
                width: self.scene.width,
                height: self.scene.height,
            },
        );
        let mut stats = SunburstLabelPlacementStats::default();
        let mut inputs = Vec::new();
        let mut authored = BTreeMap::new();
        for candidate in &self.label_layout.candidates {
            let measured = supplied
                .get(candidate.id.as_str())
                .ok_or_else(|| ChartError("missing current Sunburst label measurement".into()))?;
            ensure(
                measured.signature == candidate.signature(),
                "stale Sunburst label measurement",
            )?;
            let bounds = LayoutRect {
                x: candidate.label.x + measured.bounds.x,
                y: candidate.label.y + measured.bounds.y,
                ..measured.bounds
            };
            bounds.validate()?;
            authored.insert(candidate.id.as_str(), bounds);
            if options.hidden.contains(&candidate.node_id)
                || bounds.width == 0.
                || bounds.height == 0.
            {
                continue;
            }
            let mask_work = if options.clip == SunburstLabelClip::Plot {
                0
            } else {
                let cost = candidate.mask.picking_stats()?.work;
                stats.charge(cost, options.max_mask_work)?;
                cost
            };
            let mut alternatives = Vec::new();
            for (dx, dy) in std::iter::once((0., 0.)).chain(options.offsets.iter().copied()) {
                let proposed = LayoutRect {
                    x: bounds.x + dx,
                    y: bounds.y + dy,
                    ..bounds
                };
                proposed.validate()?;
                if !collision.viewport.contains(proposed) {
                    continue;
                }
                let accepted = if options.clip == SunburstLabelClip::Plot {
                    true
                } else {
                    stats.charge(mask_work, options.max_mask_work)?;
                    stats.mask_queries += 1;
                    let coverage = candidate.mask.rect_coverage(proposed)?;
                    match options.clip {
                        SunburstLabelClip::ClipToArc => coverage != RectCoverage::Outside,
                        SunburstLabelClip::RequireFullInkBounds => coverage == RectCoverage::Full,
                        SunburstLabelClip::Plot => unreachable!(),
                    }
                };
                if accepted {
                    alternatives.push(proposed);
                }
            }
            let Some(first) = alternatives.first().copied() else {
                continue;
            };
            inputs.push(LabelCandidate {
                id: candidate.id.clone(),
                rect: first,
                priority: options
                    .priorities
                    .get(&candidate.node_id)
                    .copied()
                    .unwrap_or(0),
                offsets: alternatives
                    .iter()
                    .skip(1)
                    .map(|rect| (rect.x - first.x, rect.y - first.y))
                    .collect(),
            });
        }
        let placed = layout_labels(&inputs, &collision)?
            .into_iter()
            .filter_map(|placement| Some((placement.id, placement.rect?)))
            .collect::<BTreeMap<_, _>>();
        self.apply_label_placements(&placed, &authored, options.clip, stats)
    }

    fn validate_managed_labels(&self) -> Result<()> {
        ensure(
            self.nodes.len() <= 8000
                && self.label_layout.candidates.len() <= 8000
                && self.label_layout.candidates.len() == self.label_layout.bindings.len(),
            "Sunburst label metadata budget exceeded",
        )?;
        let mut slots = BTreeSet::new();
        for node in &self.nodes {
            if let Some(slot) = node.label_index {
                ensure(
                    slot < self.scene.labels.len()
                        && slots.insert(slot)
                        && self.scene.label_series.get(&slot) == Some(&node.data.id),
                    "invalid Sunburst node label ownership or slot",
                )?;
            }
        }
        for (candidate, binding) in self
            .label_layout
            .candidates
            .iter()
            .zip(&self.label_layout.bindings)
        {
            let font = binding.font.clone().unwrap_or_default().resolve(
                &self
                    .scene
                    .typography
                    .default
                    .resolve(&crate::ResolvedFont::default()),
            );
            ensure(
                font == candidate.font
                    && binding.whitespace.unwrap_or_default() == candidate.whitespace,
                "Sunburst label font was changed before placement",
            )?;
            let node = self
                .node(&candidate.node_id)
                .ok_or_else(|| ChartError("missing Sunburst label owner".into()))?;
            let mark = node
                .mark_index
                .and_then(|slot| self.scene.marks.get(slot))
                .ok_or_else(|| ChartError("missing Sunburst label arc".into()))?;
            ensure(
                node.label_index == binding.slot
                    && node.mark_id == mark.id
                    && mark.series == candidate.node_id
                    && mark.value == node.value
                    && mark.clip.is_none()
                    && mark.clip_path.is_none()
                    && candidate.arc == node.arc
                    && candidate.context == source_context(self, node, &mark.geometry),
                "Sunburst label arc or business metadata was changed",
            )?;
            if let Some(slot) = binding.slot {
                ensure(
                    self.scene.labels.get(slot) == Some(&binding.label)
                        && self.scene.label_series.get(&slot) == Some(&candidate.node_id)
                        && self.scene.typography.label_strokes.get(&slot)
                            == binding.stroke.as_ref()
                        && self.scene.typography.labels.get(&slot) == binding.font.as_ref()
                        && self.scene.typography.label_whitespace.get(&slot)
                            == binding.whitespace.as_ref()
                        && self.scene.label_clips.get(&slot) == binding.clip.as_ref(),
                    "Sunburst managed label or clip was changed before placement",
                )?;
            }
        }
        Ok(())
    }

    fn apply_label_placements(
        &self,
        placed: &BTreeMap<String, LayoutRect>,
        authored: &BTreeMap<&str, LayoutRect>,
        strategy: SunburstLabelClip,
        stats: SunburstLabelPlacementStats,
    ) -> Result<Self> {
        let plot = self.plot();
        let plot_clip = ClipPath {
            geometry: Geometry::Rect {
                x: plot.x,
                y: plot.y,
                width: plot.width,
                height: plot.height,
            },
            rule: ClipRule::NonZero,
        };
        let targets = self
            .label_layout
            .candidates
            .iter()
            .map(|candidate| {
                let bounds = placed.get(&candidate.id)?;
                let original = authored[candidate.id.as_str()];
                Some((
                    Label {
                        x: candidate.label.x + (bounds.x - original.x),
                        y: candidate.label.y + (bounds.y - original.y),
                        ..candidate.label.clone()
                    },
                    if strategy == SunburstLabelClip::Plot {
                        &plot_clip
                    } else {
                        &candidate.mask
                    },
                ))
            })
            .collect::<Vec<_>>();
        let managed = self
            .label_layout
            .bindings
            .iter()
            .enumerate()
            .filter_map(|(index, binding)| Some((binding.slot?, index)))
            .collect::<BTreeMap<_, _>>();
        let mut result = self.clone();
        result.scene.labels.clear();
        result.scene.label_series.clear();
        result.scene.label_clips.clear();
        result.scene.typography.labels.clear();
        result.scene.typography.label_whitespace.clear();
        result.scene.typography.label_strokes.clear();
        let mut slots = vec![None; self.scene.labels.len()];
        let mut next_slots = vec![None; targets.len()];
        for (old_slot, label) in self.scene.labels.iter().enumerate() {
            let slot = if let Some(&index) = managed.get(&old_slot) {
                let Some((label, clip)) = &targets[index] else {
                    continue;
                };
                let candidate = &self.label_layout.candidates[index];
                let slot = push_label(
                    &mut result.scene,
                    label.clone(),
                    Some(&candidate.node_id),
                    Some(clip),
                );
                next_slots[index] = Some(slot);
                slot
            } else {
                push_label(
                    &mut result.scene,
                    label.clone(),
                    self.scene.label_series.get(&old_slot).map(String::as_str),
                    self.scene.label_clips.get(&old_slot),
                )
            };
            slots[old_slot] = Some(slot);
            if let Some(mode) = self.scene.typography.label_whitespace.get(&old_slot) {
                result.scene.typography.label_whitespace.insert(slot, *mode);
            }
            if let Some(stroke) = self.scene.typography.label_strokes.get(&old_slot) {
                result.scene.typography.label_strokes.insert(slot, *stroke);
            }
            if let Some(font) = self.scene.typography.labels.get(&old_slot) {
                result.scene.typography.labels.insert(slot, font.clone());
            }
        }
        let mut restored = Vec::new();
        for (index, binding) in self.label_layout.bindings.iter().enumerate() {
            if binding.slot.is_none()
                && let Some((label, clip)) = &targets[index]
            {
                let slot = push_label(
                    &mut result.scene,
                    label.clone(),
                    Some(&self.label_layout.candidates[index].node_id),
                    Some(clip),
                );
                next_slots[index] = Some(slot);
                restored.push(slot);
                if let Some(stroke) = binding.stroke {
                    result.scene.typography.label_strokes.insert(slot, stroke);
                }
                if let Some(mode) = binding.whitespace {
                    result.scene.typography.label_whitespace.insert(slot, mode);
                }
                if let Some(font) = &binding.font {
                    result.scene.typography.labels.insert(slot, font.clone());
                }
            }
        }
        if !self.scene.draw_order.is_empty() {
            result.scene.draw_order = self
                .scene
                .draw_order
                .iter()
                .filter_map(|item| match item {
                    DrawItem::Mark(index) => Some(DrawItem::Mark(*index)),
                    DrawItem::Label(index) => slots[*index].map(DrawItem::Label),
                })
                .chain(restored.into_iter().map(DrawItem::Label))
                .collect();
        }
        for node in &mut result.nodes {
            node.label_index = node.label_index.and_then(|slot| slots[slot]);
        }
        result.label_layout.placements.clear();
        result.label_layout.stats = stats;
        for (index, candidate) in self.label_layout.candidates.iter().enumerate() {
            let slot = next_slots[index];
            result.nodes[result.by_id[&candidate.node_id]].label_index = slot;
            result.label_layout.bindings[index] = LabelBinding {
                slot,
                font: self.label_layout.bindings[index].font.clone(),
                whitespace: self.label_layout.bindings[index].whitespace,
                stroke: self.label_layout.bindings[index].stroke,
                label: slot
                    .map(|slot| result.scene.labels[slot].clone())
                    .unwrap_or_else(|| candidate.label.clone()),
                clip: slot.and_then(|slot| result.scene.label_clips.get(&slot).cloned()),
            };
            result.label_layout.placements.push(SunburstPlacedLabel {
                id: candidate.id.clone(),
                node_id: candidate.node_id.clone(),
                bounds: placed.get(&candidate.id).copied(),
                label_index: slot,
            });
        }
        result.scene.validate()?;
        result.validate_managed_labels()?;
        Ok(result)
    }
}
