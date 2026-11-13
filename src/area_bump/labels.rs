//! Measured layout of the start and end labels authored by AreaBump.
use std::collections::{BTreeMap, BTreeSet};
use std::mem;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C};

use super::{AreaBumpLayout, alpha};
use crate::scene::ensure;
use crate::{
    ChartError, DrawItem, Geometry, Label, LabelCandidate, LabelCollisionOptions, LayoutRect, Mark,
    Result, Scene, layout_labels,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AreaBumpLabelSide {
    Start,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpLabelCandidate {
    pub id: String,
    pub series: String,
    pub side: AreaBumpLabelSide,
    /// Midpoint of the terminal band edge, before applying the label offsets.
    pub endpoint: [f64; 2],
    /// The original label anchor, rotation, text, and authored paint.
    pub label: Label,
    pub font: crate::ResolvedFont,
}

impl AreaBumpLabelCandidate {
    pub(super) fn new(
        series: &str,
        side: AreaBumpLabelSide,
        endpoint: [f64; 2],
        label: Label,
        font: crate::ResolvedFont,
    ) -> Self {
        let id = format!(
            "area-bump-endpoint:{}:{series}:{}",
            series.len(),
            match side {
                AreaBumpLabelSide::Start => "start",
                AreaBumpLabelSide::End => "end",
            },
        );
        Self {
            id,
            series: series.into(),
            side,
            endpoint,
            label,
            font,
        }
    }

    fn signature(&self) -> String {
        let label = &self.label;
        format!(
            "{}:{:?}:{}:{}:{:?}:{:x}:{:x}:{:x}:{:x}:{:x}:{:x}:{}",
            self.id,
            self.side,
            label.text.len(),
            label.text,
            label.anchor,
            label.size.to_bits(),
            label.rotation.to_bits(),
            label.x.to_bits(),
            label.y.to_bits(),
            self.endpoint[0].to_bits(),
            self.endpoint[1].to_bits(),
            self.font.signature(),
        )
    }

    /// Supply actual ink bounds relative to the text anchor, including bearings
    /// and rotation. Measure using the font that will render the chart.
    pub fn measured(&self, bounds: LayoutRect) -> AreaBumpMeasuredLabel {
        AreaBumpMeasuredLabel {
            id: self.id.clone(),
            signature: self.signature(),
            bounds,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpMeasuredLabel {
    pub id: String,
    pub signature: String,
    pub bounds: LayoutRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpLabelPlacementOptions {
    pub collision: LabelCollisionOptions,
    /// Ordered vertical alternatives after the original anchor. The horizontal
    /// anchor stays fixed. An empty list only allows the original position.
    pub y_offsets: Vec<f64>,
    /// Optional decorative connector from the band edge to the measured label.
    /// Collision constraints apply to the text bounds.
    pub leader: Option<GraphicStroke>,
}

impl Default for AreaBumpLabelPlacementOptions {
    fn default() -> Self {
        Self {
            collision: LabelCollisionOptions::default(),
            y_offsets: (1..=8)
                .flat_map(|i| [-8. * f64::from(i), 8. * f64::from(i)])
                .collect(),
            leader: None,
        }
    }
}

impl AreaBumpLabelPlacementOptions {
    pub fn for_scene(scene: &Scene) -> Self {
        Self {
            collision: LabelCollisionOptions {
                viewport: viewport(scene),
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AreaBumpPlacedLabel {
    pub id: String,
    pub bounds: Option<LayoutRect>,
    pub label_index: Option<usize>,
    pub leader_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct LabelBinding {
    slot: Option<usize>,
    anchor: [f64; 2],
    opacity: f64,
    leader: Option<LeaderBinding>,
    font: Option<crate::FontOptions>,
}

#[derive(Clone, Debug, PartialEq)]
struct LeaderBinding {
    slot: usize,
    authored: Mark,
}

impl LabelBinding {
    pub(super) fn new(slot: usize, label: &Label, opacity: f64) -> Self {
        Self {
            slot: Some(slot),
            anchor: [label.x, label.y],
            opacity,
            leader: None,
            font: None,
        }
    }
}

fn viewport(scene: &Scene) -> LayoutRect {
    LayoutRect {
        x: 0.,
        y: 0.,
        width: scene.width,
        height: scene.height,
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

fn leader_mark(
    candidate: &AreaBumpLabelCandidate,
    rect: LayoutRect,
    stroke: GraphicStroke,
) -> Option<Mark> {
    let [x, y] = candidate.endpoint;
    let edge = match candidate.side {
        AreaBumpLabelSide::Start => rect.x + rect.width,
        AreaBumpLabelSide::End => rect.x,
    };
    let center = rect.y + rect.height / 2.;
    if [x, y] == [edge, center] {
        return None;
    }
    let mut mark = Mark::new(
        format!("{}:leader", candidate.id),
        Geometry::Path(vec![
            C::Move(P::new(x, y)),
            C::Line(P::new((x + edge) / 2., center)),
            C::Line(P::new(edge, center)),
        ]),
        stroke.color,
    );
    mark.label.clear();
    mark.series.clone_from(&candidate.series);
    mark.fill = None;
    mark.stroke = Some(stroke);
    Some(mark)
}

impl AreaBumpLayout {
    /// Select fonts for native endpoint labels before obtaining ink measurements.
    pub fn with_label_font(&self, font: crate::FontOptions) -> Result<Self> {
        font.validate()?;
        self.scene.validate()?;
        let any = self.active_ids.iter().any(|id| self.visible.contains(id));
        self.validate_label_slots(any)?;
        ensure(
            self.endpoint_placements.is_empty(),
            "select AreaBump label fonts before measured placement",
        )?;
        let mut result = self.clone();
        for (candidate, binding) in result.endpoint_labels.iter_mut().zip(&mut result.labels) {
            let Some(slot) = binding.slot else { continue };
            let options = font.inherit(&binding.font.clone().unwrap_or_default());
            result.scene.typography.labels.insert(slot, options.clone());
            binding.font = Some(options);
            candidate.font = result.scene.resolved_label_font(slot);
        }
        result.scene.validate()?;
        result.validate_label_slots(any)?;
        Ok(result)
    }

    pub fn endpoint_label_candidates(&self) -> &[AreaBumpLabelCandidate] {
        &self.endpoint_labels
    }

    /// Empty until measured placement is requested. A candidate dropped by the
    /// solver has no bounds, label slot or connector slot.
    pub fn endpoint_label_placements(&self) -> &[AreaBumpPlacedLabel] {
        &self.endpoint_placements
    }

    /// Reflow the Nivo start/end labels with actual text measurements. Supply
    /// every current candidate exactly once. The shared collision solver tries
    /// the original position and the configured vertical offsets in order,
    /// with priority by side and series ID. Labels stay on their endpoint's
    /// side and inside both the scene and collision viewport. Unplaceable ink
    /// and its connector are omitted. Call again to restore previously omitted
    /// candidates. Edits to managed labels or connectors are rejected. Other
    /// valid scene items and their explicit paint order are preserved.
    pub fn place_endpoint_labels(
        &self,
        measurements: &[AreaBumpMeasuredLabel],
        options: &AreaBumpLabelPlacementOptions,
    ) -> Result<Self> {
        ensure(
            measurements.len() == self.endpoint_labels.len(),
            "AreaBump measurements must cover the current label candidates exactly",
        )?;
        options.collision.validate()?;
        ensure(
            options.y_offsets.len() <= 16
                && options
                    .y_offsets
                    .iter()
                    .all(|y| y.is_finite() && y.abs() <= 1_000_000.),
            "invalid AreaBump label displacement options",
        )?;
        ensure(
            options
                .leader
                .is_none_or(|s| (0.000001..=10_000.).contains(&s.width)),
            "invalid AreaBump label leader width",
        )?;
        self.scene.validate()?;
        let any = self.active_ids.iter().any(|id| self.visible.contains(id));
        self.validate_label_slots(any)?;
        let mut supplied = BTreeMap::new();
        for m in measurements {
            ensure(
                m.id.len() <= crate::MAX_LABEL_BYTES && m.signature.len() <= 8192,
                "AreaBump label measurement identity budget exceeded",
            )?;
            m.bounds.validate()?;
            ensure(
                supplied.insert(m.id.as_str(), m).is_none(),
                "duplicate AreaBump label measurement",
            )?;
        }
        let mut inputs = Vec::with_capacity(measurements.len());
        let mut authored_y = Vec::with_capacity(measurements.len());
        let offsets: Vec<_> = options.y_offsets.iter().map(|&y| (0., y)).collect();
        for c in &self.endpoint_labels {
            let m = supplied
                .get(c.id.as_str())
                .ok_or_else(|| ChartError("missing current AreaBump label measurement".into()))?;
            ensure(
                c.signature() == m.signature,
                "stale AreaBump label measurement",
            )?;
            let rect = LayoutRect {
                x: c.label.x + m.bounds.x,
                y: c.label.y + m.bounds.y,
                ..m.bounds
            };
            rect.validate()?;
            authored_y.push(rect.y);
            let on_side = match c.side {
                AreaBumpLabelSide::Start => rect.x + rect.width <= c.endpoint[0],
                AreaBumpLabelSide::End => rect.x >= c.endpoint[0],
            };
            if on_side && rect.width > 0. && rect.height > 0. {
                inputs.push((
                    c.side,
                    c.series.as_str(),
                    LabelCandidate {
                        id: c.id.clone(),
                        rect,
                        priority: 0,
                        offsets: offsets.clone(),
                    },
                ));
            }
        }
        inputs.sort_by_key(|&(side, series, _)| (side, series));
        let inputs: Vec<_> = inputs.into_iter().map(|(_, _, label)| label).collect();
        let mut collision = options.collision.clone();
        collision.viewport = intersection(collision.viewport, viewport(&self.scene));
        let placed: BTreeMap<_, _> = layout_labels(&inputs, &collision)?
            .into_iter()
            .filter_map(|p| Some((p.id, p.rect?)))
            .collect();
        self.apply_endpoint_placements(&placed, &authored_y, options.leader, any)
    }

    fn label_opacity(
        &self,
        binding: &LabelBinding,
        ids: &BTreeSet<String>,
        any: bool,
        series: &str,
    ) -> f64 {
        if !any {
            binding.opacity
        } else if ids.contains(series) {
            self.active_style.fill_opacity
        } else {
            self.inactive_style.fill_opacity
        }
    }

    pub(super) fn update_label_paint(
        &self,
        scene: &mut Scene,
        ids: &BTreeSet<String>,
        any: bool,
    ) -> Result<()> {
        for (candidate, binding) in self.endpoint_labels.iter().zip(&self.labels) {
            let font = binding.font.clone().unwrap_or_default().resolve(
                &self
                    .scene
                    .typography
                    .default
                    .resolve(&crate::ResolvedFont::default()),
            );
            ensure(
                font == candidate.font,
                "AreaBump label font was changed before placement",
            )?;
            let Some(slot) = binding.slot else { continue };
            ensure(
                scene.label_series.get(&slot) == Some(&candidate.series),
                "AreaBump label ownership was changed",
            )?;
            let label = scene
                .labels
                .get_mut(slot)
                .ok_or_else(|| ChartError("AreaBump label slots were changed".into()))?;
            let opacity = self.label_opacity(binding, ids, any, &candidate.series);
            label.color = alpha(candidate.label.color, opacity);
            if let Some(leader) = &binding.leader {
                let mark = scene
                    .marks
                    .get_mut(leader.slot)
                    .ok_or_else(|| ChartError("AreaBump label leader slots were changed".into()))?;
                ensure(
                    mark.id == leader.authored.id && mark.series == candidate.series,
                    "AreaBump label leader ownership was changed",
                )?;
                mark.opacity = opacity;
            }
        }
        Ok(())
    }

    fn validate_label_slots(&self, any: bool) -> Result<()> {
        for (candidate, binding) in self.endpoint_labels.iter().zip(&self.labels) {
            let font = binding.font.clone().unwrap_or_default().resolve(
                &self
                    .scene
                    .typography
                    .default
                    .resolve(&crate::ResolvedFont::default()),
            );
            ensure(
                font == candidate.font,
                "AreaBump label font was changed before placement",
            )?;
            let Some(slot) = binding.slot else { continue };
            let opacity = self.label_opacity(binding, &self.active_ids, any, &candidate.series);
            let expected = Label {
                x: binding.anchor[0],
                y: binding.anchor[1],
                color: alpha(candidate.label.color, opacity),
                ..candidate.label.clone()
            };
            ensure(
                self.scene.labels.get(slot) == Some(&expected)
                    && self.scene.typography.labels.get(&slot) == binding.font.as_ref()
                    && self.scene.label_series.get(&slot) == Some(&candidate.series)
                    && !self.scene.label_clips.contains_key(&slot),
                "AreaBump managed label was changed before placement",
            )?;
            if let Some(leader) = &binding.leader {
                let expected = Mark {
                    opacity,
                    ..leader.authored.clone()
                };
                ensure(
                    self.scene.marks.get(leader.slot) == Some(&expected),
                    "AreaBump managed label leader was changed before placement",
                )?;
            }
        }
        Ok(())
    }

    fn apply_endpoint_placements(
        &self,
        placed: &BTreeMap<String, LayoutRect>,
        authored_y: &[f64],
        leader: Option<GraphicStroke>,
        any: bool,
    ) -> Result<Self> {
        let mut result = self.clone();
        let (mark_slots, mut label_slots) = retain_unmanaged(&mut result.scene, &self.labels);
        for binding in &mut result.paints {
            binding.slot = mark_slots
                .get(binding.slot)
                .copied()
                .flatten()
                .filter(|&slot| {
                    result.scene.marks[slot].id == binding.id
                        && result.scene.marks[slot].series == binding.series
                })
                .ok_or_else(|| ChartError("AreaBump band slots were changed".into()))?;
        }
        let mut leaders = BTreeMap::new();
        let mut restored = Vec::new();
        result.endpoint_placements.clear();
        for (index, (c, b)) in result
            .endpoint_labels
            .iter()
            .zip(&mut result.labels)
            .enumerate()
        {
            let previous = b.slot.take();
            b.leader = None;
            let bounds = placed.get(&c.id).copied();
            if let Some(rect) = bounds {
                let opacity = self.label_opacity(b, &self.active_ids, any, &c.series);
                let y = c.label.y + (rect.y - authored_y[index]);
                let slot = result.scene.labels.len();
                b.slot = Some(slot);
                b.anchor = [c.label.x, y];
                result.scene.label_series.insert(slot, c.series.clone());
                if let Some(font) = &b.font {
                    result.scene.typography.labels.insert(slot, font.clone());
                }
                result.scene.labels.push(Label {
                    y,
                    color: alpha(c.label.color, opacity),
                    ..c.label.clone()
                });
                if let Some(previous) = previous {
                    label_slots[previous] = Some(slot);
                } else {
                    restored.push(slot);
                }
                if let Some(mark) = leader.and_then(|stroke| leader_mark(c, rect, stroke)) {
                    let leader_slot = result.scene.marks.len();
                    result.scene.marks.push(Mark {
                        opacity,
                        ..mark.clone()
                    });
                    b.leader = Some(LeaderBinding {
                        slot: leader_slot,
                        authored: mark,
                    });
                    leaders.insert(slot, leader_slot);
                }
            }
            result.endpoint_placements.push(AreaBumpPlacedLabel {
                id: c.id.clone(),
                bounds,
                label_index: b.slot,
                leader_index: b.leader.as_ref().map(|leader| leader.slot),
            });
        }
        if !self.scene.draw_order.is_empty() {
            let mut draw_order = Vec::new();
            let draw_label = |draw_order: &mut Vec<_>, slot| {
                if let Some(&leader) = leaders.get(&slot) {
                    draw_order.push(DrawItem::Mark(leader));
                }
                draw_order.push(DrawItem::Label(slot));
            };
            for item in &self.scene.draw_order {
                match *item {
                    DrawItem::Mark(slot) => {
                        if let Some(slot) = mark_slots[slot] {
                            draw_order.push(DrawItem::Mark(slot));
                        }
                    }
                    DrawItem::Label(slot) => {
                        if let Some(slot) = label_slots[slot] {
                            draw_label(&mut draw_order, slot);
                        }
                    }
                }
            }
            for slot in restored {
                draw_label(&mut draw_order, slot);
            }
            result.scene.draw_order = draw_order;
        }
        result.scene.validate()?;
        Ok(result)
    }
}

fn retain_unmanaged(
    scene: &mut Scene,
    bindings: &[LabelBinding],
) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let managed_labels: BTreeSet<_> = bindings.iter().filter_map(|b| b.slot).collect();
    let managed_marks: BTreeSet<_> = bindings
        .iter()
        .filter_map(|b| Some(b.leader.as_ref()?.slot))
        .collect();
    let marks = mem::take(&mut scene.marks);
    let mark_slots = marks
        .into_iter()
        .enumerate()
        .map(|(i, mark)| {
            if managed_marks.contains(&i) {
                return None;
            }
            let slot = scene.marks.len();
            scene.marks.push(mark);
            Some(slot)
        })
        .collect();
    let labels = mem::take(&mut scene.labels);
    let mut series = mem::take(&mut scene.label_series);
    let mut clips = mem::take(&mut scene.label_clips);
    let mut fonts = mem::take(&mut scene.typography.labels);
    let label_slots = labels
        .into_iter()
        .enumerate()
        .map(|(i, label)| {
            if managed_labels.contains(&i) {
                return None;
            }
            let slot = scene.labels.len();
            scene.labels.push(label);
            if let Some(series) = series.remove(&i) {
                scene.label_series.insert(slot, series);
            }
            if let Some(clip) = clips.remove(&i) {
                scene.label_clips.insert(slot, clip);
            }
            if let Some(font) = fonts.remove(&i) {
                scene.typography.labels.insert(slot, font);
            }
            Some(slot)
        })
        .collect();
    (mark_slots, label_slots)
}
