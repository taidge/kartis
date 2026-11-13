//! Explicit hierarchy label placement with application-measured ink bounds.
use super::HierarchyLayout;
use crate::{
    ChartError, ClipPath, ClipRect, DrawItem, Geometry, Label, LabelCandidate,
    LabelCollisionOptions, LayoutRect, MAX_LABEL_BYTES, Result, Scene, layout_labels,
    scene::ensure,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyLabelCandidate {
    /// Stable across hierarchy focus, ordering and text updates for this node.
    pub id: String,
    pub node_id: String,
    pub label: Label,
    /// Use this exact selection when measuring the candidate's ink.
    pub font: crate::ResolvedFont,
    /// The authored node, parent-header or plot clip, in scene coordinates.
    pub clip: Option<ClipPath>,
}

impl HierarchyLabelCandidate {
    fn signature(&self) -> String {
        let label = &self.label;
        format!(
            "{}:{}:{}:{}:{}:{}:{:?}:{:x}:{:x}:{:x}:{:x}:{:?}:{}",
            self.id.len(),
            self.id,
            self.node_id.len(),
            self.node_id,
            label.text.len(),
            label.text,
            label.anchor,
            label.size.to_bits(),
            label.rotation.to_bits(),
            label.x.to_bits(),
            label.y.to_bits(),
            self.clip,
            self.font.signature(),
        )
    }

    /// Actual ink AABB relative to the authored text anchor. Include the
    /// selected font's bearings, text anchor and the label's rotation.
    pub fn measured(&self, bounds: LayoutRect) -> HierarchyMeasuredLabel {
        HierarchyMeasuredLabel {
            id: self.id.clone(),
            signature: self.signature(),
            bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyMeasuredLabel {
    pub id: String,
    pub signature: String,
    pub bounds: LayoutRect,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HierarchyLabelPlacementOptions {
    pub collision: LabelCollisionOptions,
    /// At most sixteen scene-coordinate alternatives after the authored anchor.
    pub offsets: Vec<(f64, f64)>,
    /// Larger priorities are placed first. Equal priorities use node ID order.
    pub priorities: BTreeMap<String, i32>,
    /// Omit only these nodes' labels. Unknown IDs may survive data replacement.
    pub hidden: BTreeSet<String>,
}

impl HierarchyLabelPlacementOptions {
    pub fn for_layout(layout: &HierarchyLayout) -> Self {
        Self {
            collision: LabelCollisionOptions {
                viewport: rect(layout.plot()),
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
            "invalid hierarchy label displacement budget",
        )?;
        ensure(
            self.priorities.len() <= 8000
                && self.hidden.len() <= 8000
                && self
                    .priorities
                    .keys()
                    .chain(&self.hidden)
                    .all(|id| !id.trim().is_empty() && id.len() <= 1000),
            "invalid hierarchy label priority or hidden identity budget",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HierarchyPlacedLabel {
    pub id: String,
    pub node_id: String,
    pub bounds: Option<LayoutRect>,
    pub label_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
struct LabelBinding {
    slot: Option<usize>,
    label: Label,
    font: Option<crate::FontOptions>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct HierarchyLabelState {
    candidates: Vec<HierarchyLabelCandidate>,
    bindings: Vec<LabelBinding>,
    placements: Vec<HierarchyPlacedLabel>,
}

impl HierarchyLabelState {
    pub(super) fn capture(layout: &HierarchyLayout) -> Result<Self> {
        let mut rows = Vec::new();
        for node in &layout.nodes {
            ensure(
                node.label_slots.len() <= 1,
                "hierarchy label placement supports one authored label per node",
            )?;
            for &slot in &node.label_slots {
                let label = layout
                    .scene
                    .labels
                    .get(slot)
                    .ok_or_else(|| ChartError("missing authored hierarchy label".into()))?;
                let candidate = HierarchyLabelCandidate {
                    id: format!("hierarchy-label:{}", node.id),
                    node_id: node.id.clone(),
                    label: label.clone(),
                    font: layout.scene.resolved_label_font(slot),
                    clip: layout.scene.label_clips.get(&slot).cloned(),
                };
                rows.push((
                    candidate,
                    LabelBinding {
                        slot: Some(slot),
                        label: label.clone(),
                        font: layout.scene.typography.labels.get(&slot).cloned(),
                    },
                ));
            }
        }
        rows.sort_by(|a, b| a.0.node_id.cmp(&b.0.node_id));
        let (candidates, bindings) = rows.into_iter().unzip();
        Ok(Self {
            candidates,
            bindings,
            placements: vec![],
        })
    }
}

fn rect(clip: ClipRect) -> LayoutRect {
    LayoutRect {
        x: clip.x,
        y: clip.y,
        width: clip.width,
        height: clip.height,
    }
}

fn scene_rect(scene: &Scene) -> LayoutRect {
    LayoutRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
    }
}

fn intersect(a: LayoutRect, b: LayoutRect) -> LayoutRect {
    let x = a.x.max(b.x);
    let y = a.y.max(b.y);
    LayoutRect {
        x,
        y,
        width: ((a.x + a.width).min(b.x + b.width) - x).max(0.),
        height: ((a.y + a.height).min(b.y + b.height) - y).max(0.),
    }
}

/// Conservative containment of measured rotated ink in the authored clip.
fn clip_contains(clip: Option<&ClipPath>, bounds: LayoutRect) -> Result<bool> {
    Ok(match clip.map(|clip| &clip.geometry) {
        None => true,
        Some(Geometry::Rect {
            x,
            y,
            width,
            height,
        }) => LayoutRect {
            x: *x,
            y: *y,
            width: *width,
            height: *height,
        }
        .contains(bounds),
        Some(Geometry::Circle { x, y, radius }) => [
            (bounds.x, bounds.y),
            (bounds.x + bounds.width, bounds.y),
            (bounds.x, bounds.y + bounds.height),
            (bounds.x + bounds.width, bounds.y + bounds.height),
        ]
        .iter()
        .all(|(cx, cy)| (cx - x).hypot(cy - y) <= *radius),
        Some(_) => {
            return Err(ChartError(
                "hierarchy label placement requires Rect or Circle clips".into(),
            ));
        }
    })
}

impl HierarchyLayout {
    /// Select fonts for the authored node labels before measuring them.
    /// Per-label declarations merge independently; extra Scene text is retained.
    pub fn with_label_font(&self, font: crate::FontOptions) -> Result<Self> {
        font.validate()?;
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            self.label_layout.placements.is_empty(),
            "select hierarchy label fonts before measured placement",
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
        result.label_layout = HierarchyLabelState::capture(&result)?;
        Ok(result)
    }

    pub fn label_candidates(&self) -> &[HierarchyLabelCandidate] {
        &self.label_layout.candidates
    }

    /// Empty until explicit measured placement is requested.
    pub fn label_placements(&self) -> &[HierarchyPlacedLabel] {
        &self.label_layout.placements
    }

    /// Place every authored candidate from actual rotated ink measurements.
    /// The shared greedy solver tries permitted offsets, honors node priorities
    /// and keeps all ink inside its original Rect/Circle clip and scene viewport.
    /// Unplaceable or hidden labels are omitted and can be restored on another
    /// call. Measurement coverage must be exact, including hidden candidates.
    /// Managed label, ownership or clip edits are rejected. Other valid Scene
    /// items and their current paint order are retained. Restored labels append
    /// to that order. This operation returns a new layout after full validation.
    pub fn place_labels(
        &self,
        measurements: &[HierarchyMeasuredLabel],
        options: &HierarchyLabelPlacementOptions,
    ) -> Result<Self> {
        options.validate()?;
        self.scene.validate()?;
        self.validate_managed_labels()?;
        ensure(
            measurements.len() == self.label_layout.candidates.len(),
            "hierarchy measurements must cover all current label candidates exactly",
        )?;
        let mut supplied = BTreeMap::new();
        for measured in measurements {
            ensure(
                measured.id.len() <= MAX_LABEL_BYTES && measured.signature.len() <= 8192,
                "hierarchy label measurement identity budget exceeded",
            )?;
            measured.bounds.validate()?;
            ensure(
                supplied.insert(measured.id.as_str(), measured).is_none(),
                "duplicate hierarchy label measurement",
            )?;
        }
        let mut collision = options.collision.clone();
        collision.viewport = intersect(collision.viewport, scene_rect(&self.scene));
        let mut inputs = Vec::new();
        let mut authored = BTreeMap::new();
        for candidate in &self.label_layout.candidates {
            let measured = supplied
                .get(candidate.id.as_str())
                .ok_or_else(|| ChartError("missing current hierarchy label measurement".into()))?;
            ensure(
                measured.signature == candidate.signature(),
                "stale hierarchy label measurement",
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
            let mut alternatives = Vec::new();
            for (dx, dy) in std::iter::once((0., 0.)).chain(options.offsets.iter().copied()) {
                let proposed = LayoutRect {
                    x: bounds.x + dx,
                    y: bounds.y + dy,
                    ..bounds
                };
                proposed.validate()?;
                if collision.viewport.contains(proposed)
                    && clip_contains(candidate.clip.as_ref(), proposed)?
                {
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
                    .map(|r| (r.x - first.x, r.y - first.y))
                    .collect(),
            });
        }
        let placed = layout_labels(&inputs, &collision)?
            .into_iter()
            .filter_map(|placement| Some((placement.id, placement.rect?)))
            .collect::<BTreeMap<_, _>>();
        self.apply_label_placements(&placed, &authored)
    }

    fn validate_managed_labels(&self) -> Result<()> {
        ensure(
            self.nodes.len() <= 8000
                && self.label_layout.candidates.len() <= 8000
                && self.label_layout.candidates.len() == self.label_layout.bindings.len(),
            "hierarchy label metadata budget exceeded",
        )?;
        let mut slots = BTreeSet::new();
        for node in &self.nodes {
            ensure(
                node.label_slots.len() <= self.scene.labels.len()
                    && node.label_slots.iter().all(|slot| {
                        *slot < self.scene.labels.len()
                            && slots.insert(*slot)
                            && self.scene.label_series.get(slot) == Some(&node.id)
                    }),
                "invalid hierarchy node label ownership or slot",
            )?;
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
                font == candidate.font,
                "hierarchy label font was changed before placement",
            )?;
            let node = self
                .node(&candidate.node_id)
                .ok_or_else(|| ChartError("missing hierarchy label owner".into()))?;
            ensure(
                node.visible && node.label_slots == binding.slot.into_iter().collect::<Vec<_>>(),
                "hierarchy label owner or label slots were changed",
            )?;
            if let Some(slot) = binding.slot {
                ensure(
                    self.scene.labels.get(slot) == Some(&binding.label)
                        && self.scene.label_series.get(&slot) == Some(&candidate.node_id)
                        && self.scene.typography.labels.get(&slot) == binding.font.as_ref()
                        && self.scene.label_clips.get(&slot) == candidate.clip.as_ref(),
                    "hierarchy managed label or clip was changed before placement",
                )?;
            }
        }
        Ok(())
    }

    fn apply_label_placements(
        &self,
        placed: &BTreeMap<String, LayoutRect>,
        authored: &BTreeMap<&str, LayoutRect>,
    ) -> Result<Self> {
        let mut result = self.clone();
        let managed = self
            .label_layout
            .bindings
            .iter()
            .enumerate()
            .filter_map(|(index, binding)| Some((binding.slot?, index)))
            .collect::<BTreeMap<_, _>>();
        let mut slots = vec![None; self.scene.labels.len()];
        let mut new_slots = vec![None; self.label_layout.candidates.len()];
        result.scene.labels.clear();
        result.scene.label_series.clear();
        result.scene.label_clips.clear();
        result.scene.typography.labels.clear();
        for (old_slot, label) in self.scene.labels.iter().enumerate() {
            let (label, owner, clip) = if let Some(&index) = managed.get(&old_slot) {
                let candidate = &self.label_layout.candidates[index];
                let Some(bounds) = placed.get(&candidate.id) else {
                    continue;
                };
                let source = authored[candidate.id.as_str()];
                let label = Label {
                    x: candidate.label.x + (bounds.x - source.x),
                    y: candidate.label.y + (bounds.y - source.y),
                    ..candidate.label.clone()
                };
                new_slots[index] = Some(result.scene.labels.len());
                (
                    label,
                    Some(candidate.node_id.clone()),
                    candidate.clip.clone(),
                )
            } else {
                (
                    label.clone(),
                    self.scene.label_series.get(&old_slot).cloned(),
                    self.scene.label_clips.get(&old_slot).cloned(),
                )
            };
            let slot = result.scene.labels.len();
            slots[old_slot] = Some(slot);
            result.scene.labels.push(label);
            if let Some(owner) = owner {
                result.scene.label_series.insert(slot, owner);
            }
            if let Some(clip) = clip {
                result.scene.label_clips.insert(slot, clip);
            }
            if let Some(font) = self.scene.typography.labels.get(&old_slot) {
                result.scene.typography.labels.insert(slot, font.clone());
            }
        }
        let mut restored = Vec::new();
        for (index, (candidate, binding)) in self
            .label_layout
            .candidates
            .iter()
            .zip(&self.label_layout.bindings)
            .enumerate()
        {
            if binding.slot.is_none()
                && let Some(bounds) = placed.get(&candidate.id)
            {
                let source = authored[candidate.id.as_str()];
                let slot = result.scene.labels.len();
                result.scene.labels.push(Label {
                    x: candidate.label.x + (bounds.x - source.x),
                    y: candidate.label.y + (bounds.y - source.y),
                    ..candidate.label.clone()
                });
                result
                    .scene
                    .label_series
                    .insert(slot, candidate.node_id.clone());
                if let Some(clip) = &candidate.clip {
                    result.scene.label_clips.insert(slot, clip.clone());
                }
                if let Some(font) = &binding.font {
                    result.scene.typography.labels.insert(slot, font.clone());
                }
                new_slots[index] = Some(slot);
                restored.push(slot);
            }
        }
        if !result.scene.draw_order.is_empty() {
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
            node.label_slots = node
                .label_slots
                .iter()
                .filter_map(|slot| slots[*slot])
                .collect();
        }
        result.label_layout.placements.clear();
        for (index, candidate) in self.label_layout.candidates.iter().enumerate() {
            let slot = new_slots[index];
            let owner = result.by_id[&candidate.node_id];
            result.nodes[owner].label_slots = slot.into_iter().collect();
            result.label_layout.bindings[index] = LabelBinding {
                slot,
                font: self.label_layout.bindings[index].font.clone(),
                label: slot
                    .map(|slot| result.scene.labels[slot].clone())
                    .unwrap_or_else(|| candidate.label.clone()),
            };
            result.label_layout.placements.push(HierarchyPlacedLabel {
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
