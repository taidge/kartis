//! Explicit-clock Funnel springs and actual Scene presentation.
//!
//! Paths use the target command template and ordinal numeric interpolation from
//! the previous committed target, as in `useAnimatedPath`/`interpolateString`.
//! They deliberately do not morph from the current sample on interruption.
//! New source parts start at target; removed source parts disappear immediately.
//! Dormant body/border roles retain their source-owned springs without ink. Extras
//! and custom annotation notes remain target-authored. This module schedules no
//! frames and grants no additional input eligibility.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::motion::{PropertyState, SpringConfig, SpringValue};
use crate::scene::ensure;
use crate::*;

mod geometry;
mod tracks;
use tracks::Track;

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelMotionOptions {
    pub spring: SpringConfig,
    /// Actual Scene Marks plus each source body/border without a paint slot.
    pub max_marks: usize,
    pub max_labels: usize,
    /// Target numeric geometry channels, including dormant role descriptors.
    pub max_path_numbers: usize,
}
impl Default for FunnelMotionOptions {
    fn default() -> Self {
        Self {
            spring: SpringConfig::default(),
            max_marks: MAX_MARKS,
            max_labels: MAX_MARKS,
            max_path_numbers: 1_200_000,
        }
    }
}
impl FunnelMotionOptions {
    pub fn validate(&self) -> Result<()> {
        self.spring.validate()?;
        ensure(
            self.max_marks <= MAX_MARKS
                && self.max_labels <= MAX_MARKS
                && self.max_path_numbers <= 1_200_000,
            "Funnel motion budget exceeds Scene limits",
        )
    }
}

/// Channels exposed by actual Mark springs. Missing paint channels remain
/// absent; a property value never creates a fill/stroke or business target.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunnelMotionMarkChannel {
    FillRed,
    FillGreen,
    FillBlue,
    FillAlpha,
    StrokeRed,
    StrokeGreen,
    StrokeBlue,
    StrokeAlpha,
    StrokeWidth,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunnelMotionLabelChannel {
    X,
    Y,
    Red,
    Green,
    Blue,
    Alpha,
}
/// A source-owned Part role, independent of whether a Scene Mark is present.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FunnelMotionPartRole {
    Body,
    Border,
}

#[derive(Clone, Debug)]
pub struct FunnelMotionPart {
    pub key: CategoryKey,
    pub source_index: usize,
    pub body_mark_index: Option<usize>,
    pub border_mark_index: Option<usize>,
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelMotionSeparatorKey {
    pub side: FunnelSeparatorSide,
    pub part_id: Option<CategoryKey>,
}
#[derive(Clone, Debug)]
pub struct FunnelMotionSeparator {
    pub key: FunnelMotionSeparatorKey,
    pub separator_index: usize,
    pub mark_index: Option<usize>,
}
#[derive(Clone, Debug)]
pub struct FunnelMotionLabel {
    pub key: CategoryKey,
    pub source_index: usize,
    pub label_index: usize,
}

#[derive(Clone, Debug)]
struct Snapshot {
    target: FunnelSvgFrame,
    scene: Scene,
    target_serial: u64,
    sample_serial: u64,
    settled: bool,
    parts: Vec<FunnelMotionPart>,
    separators: Vec<FunnelMotionSeparator>,
    labels: Vec<FunnelMotionLabel>,
    mark_states: BTreeMap<(String, FunnelMotionMarkChannel), PropertyState>,
    label_states: BTreeMap<(usize, FunnelMotionLabelChannel), PropertyState>,
    paths: BTreeMap<String, PropertyState>,
    geometries: BTreeMap<String, Vec<PropertyState>>,
    part_states: BTreeMap<(usize, FunnelMotionPartRole, FunnelMotionMarkChannel), PropertyState>,
    part_paths: BTreeMap<(usize, FunnelMotionPartRole), PropertyState>,
    part_geometries: BTreeMap<(usize, FunnelMotionPartRole), Vec<PropertyState>>,
    bodies: BTreeSet<usize>,
}
/// One atomically validated target/sample cohort. Equality is capture identity,
/// never legacy float equality. Scene pools retain target slots and paint order.
#[derive(Clone, Debug)]
pub struct FunnelMotionFrame(Arc<Snapshot>);
impl PartialEq for FunnelMotionFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for FunnelMotionFrame {}
pub type FunnelMotionPaintFrame = FunnelMotionFrame;
pub fn funnel_motion_paint(frame: &FunnelMotionFrame) -> Result<FunnelMotionPaintFrame> {
    frame.scene().validate()?;
    Ok(frame.clone())
}
impl FunnelMotionFrame {
    pub fn target(&self) -> &FunnelSvgFrame {
        &self.0.target
    }
    pub fn scene(&self) -> &Scene {
        &self.0.scene
    }
    pub fn target_serial(&self) -> u64 {
        self.0.target_serial
    }
    pub fn sample_serial(&self) -> u64 {
        self.0.sample_serial
    }
    pub fn is_settled(&self) -> bool {
        self.0.settled
    }
    pub fn parts(&self) -> &[FunnelMotionPart] {
        &self.0.parts
    }
    pub fn separators(&self) -> &[FunnelMotionSeparator] {
        &self.0.separators
    }
    pub fn labels(&self) -> &[FunnelMotionLabel] {
        &self.0.labels
    }
    pub fn body_mark_indices(&self) -> &BTreeSet<usize> {
        &self.0.bodies
    }
    pub fn mark_role(&self, index: usize) -> Result<Option<FunnelSvgMarkRole>> {
        self.target().mark_role(index)
    }
    pub fn label_part_index(&self, index: usize) -> Result<Option<usize>> {
        self.target().label_part_index(index)
    }
    pub fn target_item(&self, item: DrawItem) -> Option<DrawItem> {
        let valid = match item {
            DrawItem::Mark(i) => i < self.scene().marks.len(),
            DrawItem::Label(i) => i < self.scene().labels.len(),
        };
        valid.then_some(item)
    }
    pub fn source_item(&self, item: DrawItem) -> Option<DrawItem> {
        self.target_item(item)
    }
    pub fn mark_property(
        &self,
        id: &str,
        channel: FunnelMotionMarkChannel,
    ) -> Option<PropertyState> {
        self.0.mark_states.get(&(id.to_owned(), channel)).copied()
    }
    pub fn label_property(
        &self,
        slot: usize,
        channel: FunnelMotionLabelChannel,
    ) -> Option<PropertyState> {
        self.0.label_states.get(&(slot, channel)).copied()
    }
    pub fn path_progress(&self, id: &str) -> Option<PropertyState> {
        self.0.paths.get(id).copied()
    }
    /// Actual geometry channels. Path geometry exposes one progress spring;
    /// Circle/Rect and separator coordinates expose independent numeric springs.
    pub fn geometry_states(&self, id: &str) -> Option<&[PropertyState]> {
        self.0.geometries.get(id).map(Vec::as_slice)
    }
    pub fn part_property(
        &self,
        index: usize,
        role: FunnelMotionPartRole,
        channel: FunnelMotionMarkChannel,
    ) -> Result<Option<PropertyState>> {
        ensure(
            index < self.target().source().len(),
            "Funnel motion part index is missing",
        )?;
        Ok(self.0.part_states.get(&(index, role, channel)).copied())
    }
    pub fn part_path_progress(
        &self,
        index: usize,
        role: FunnelMotionPartRole,
    ) -> Result<Option<PropertyState>> {
        ensure(
            index < self.target().source().len(),
            "Funnel motion part index is missing",
        )?;
        Ok(self.0.part_paths.get(&(index, role)).copied())
    }
    pub fn part_geometry_states(
        &self,
        index: usize,
        role: FunnelMotionPartRole,
    ) -> Result<Option<&[PropertyState]>> {
        ensure(
            index < self.target().source().len(),
            "Funnel motion part index is missing",
        )?;
        Ok(self
            .0
            .part_geometries
            .get(&(index, role))
            .map(Vec::as_slice))
    }
    pub fn part_geometry_origin(&self, index: usize) -> Result<FunnelMotionGeometryOrigin> {
        Ok(self.target().motion_capture(index)?.origin)
    }
}

#[derive(Clone, Debug)]
pub struct FunnelMotion {
    options: FunnelMotionOptions,
    reduced: bool,
    tracks: BTreeMap<String, Track>,
    frame: FunnelMotionFrame,
}
impl FunnelMotion {
    pub fn new(target: FunnelSvgFrame, options: FunnelMotionOptions) -> Result<Self> {
        options.validate()?;
        let input = tracks::inputs(&target, &options)?;
        let tracks = input
            .into_iter()
            .map(|i| Ok((i.key.clone(), Track::new(i, options.spring, false)?)))
            .collect::<Result<_>>()?;
        let mut model = Self {
            options,
            reduced: false,
            tracks,
            frame: FunnelMotionFrame(Arc::new(Snapshot {
                scene: target.scene().clone(),
                target,
                target_serial: 0,
                sample_serial: 0,
                settled: true,
                parts: vec![],
                separators: vec![],
                labels: vec![],
                mark_states: BTreeMap::new(),
                label_states: BTreeMap::new(),
                paths: BTreeMap::new(),
                geometries: BTreeMap::new(),
                part_states: BTreeMap::new(),
                part_paths: BTreeMap::new(),
                part_geometries: BTreeMap::new(),
                bodies: BTreeSet::new(),
            })),
        };
        model.rebuild(0, 0)?;
        Ok(model)
    }
    pub fn frame(&self) -> &FunnelMotionFrame {
        &self.frame
    }
    pub fn target(&self) -> &FunnelSvgFrame {
        self.frame.target()
    }
    pub fn options(&self) -> &FunnelMotionOptions {
        &self.options
    }
    pub fn is_settled(&self) -> bool {
        self.tracks.values().all(Track::settled)
    }
    pub fn reduced_motion(&self) -> bool {
        self.reduced
    }
    pub fn retarget(&mut self, target: FunnelSvgFrame) -> Result<&FunnelMotionFrame> {
        let input = tracks::inputs(&target, &self.options)?;
        let serial = next(self.frame.target_serial())?;
        let mut candidate = self.clone();
        let mut previous = std::mem::take(&mut candidate.tracks);
        for input in input {
            let key = input.key.clone();
            let track = if let Some(mut track) = previous.remove(&key) {
                track.retarget(input, candidate.options.spring, candidate.reduced)?;
                track
            } else {
                Track::new(input, candidate.options.spring, candidate.reduced)?
            };
            candidate.tracks.insert(key, track);
        }
        Arc::make_mut(&mut candidate.frame.0).target = target;
        candidate.rebuild(serial, self.frame.sample_serial())?;
        *self = candidate;
        Ok(self.frame())
    }
    pub fn tick(&mut self, seconds: f64) -> Result<&FunnelMotionFrame> {
        ensure(
            seconds.is_finite() && (0. ..=60.).contains(&seconds),
            "Funnel motion dt must be in 0..=60 seconds",
        )?;
        if seconds == 0. || self.is_settled() {
            return Ok(self.frame());
        }
        let mut candidate = self.clone();
        for track in candidate.tracks.values_mut() {
            track.tick(seconds)?;
        }
        candidate.rebuild(
            self.frame.target_serial(),
            next(self.frame.sample_serial())?,
        )?;
        *self = candidate;
        Ok(self.frame())
    }
    pub fn set_options(&mut self, options: FunnelMotionOptions) -> Result<()> {
        options.validate()?;
        tracks::inputs(self.target(), &options)?;
        let mut candidate = self.clone();
        for track in candidate.tracks.values_mut() {
            track.set_config(options.spring)?;
        }
        candidate.options = options;
        candidate.rebuild(self.frame.target_serial(), self.frame.sample_serial())?;
        *self = candidate;
        Ok(())
    }
    pub fn finish(&mut self) -> Result<()> {
        if self.is_settled() {
            return Ok(());
        }
        let mut candidate = self.clone();
        for track in candidate.tracks.values_mut() {
            track.finish();
        }
        candidate.rebuild(
            self.frame.target_serial(),
            next(self.frame.sample_serial())?,
        )?;
        *self = candidate;
        Ok(())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> Result<()> {
        let mut candidate = self.clone();
        candidate.reduced = reduced;
        let was_settled = candidate.is_settled();
        for track in candidate.tracks.values_mut() {
            track.set_reduced(reduced);
        }
        let sample = if reduced && !was_settled {
            next(self.frame.sample_serial())?
        } else {
            self.frame.sample_serial()
        };
        candidate.rebuild(self.frame.target_serial(), sample)?;
        *self = candidate;
        Ok(())
    }
    fn rebuild(&mut self, target_serial: u64, sample_serial: u64) -> Result<()> {
        let target = self.target().clone();
        let mut out = Snapshot {
            scene: target.scene().clone(),
            target,
            target_serial,
            sample_serial,
            settled: self.is_settled(),
            parts: vec![],
            separators: vec![],
            labels: vec![],
            mark_states: BTreeMap::new(),
            label_states: BTreeMap::new(),
            paths: BTreeMap::new(),
            geometries: BTreeMap::new(),
            part_states: BTreeMap::new(),
            part_paths: BTreeMap::new(),
            part_geometries: BTreeMap::new(),
            bodies: BTreeSet::new(),
        };
        for track in self.tracks.values() {
            track.paint(&mut out)?;
        }
        out.parts = out
            .target
            .parts()
            .iter()
            .map(|part| FunnelMotionPart {
                key: part.datum.id.clone(),
                source_index: part.source_index,
                body_mark_index: None,
                border_mark_index: None,
                label_index: None,
            })
            .collect();
        out.separators = out
            .target
            .separators()
            .iter()
            .enumerate()
            .map(|(index, separator)| FunnelMotionSeparator {
                key: FunnelMotionSeparatorKey {
                    side: separator.side,
                    part_id: separator.part_id.clone(),
                },
                separator_index: index,
                mark_index: None,
            })
            .collect();
        for (i, _) in out.scene.marks.iter().enumerate() {
            match out.target.mark_role(i)? {
                Some(FunnelSvgMarkRole::Body { part_index }) => {
                    out.parts[part_index].body_mark_index = Some(i);
                    out.bodies.insert(i);
                }
                Some(FunnelSvgMarkRole::Border { part_index }) => {
                    out.parts[part_index].border_mark_index = Some(i)
                }
                Some(FunnelSvgMarkRole::Separator { separator_index }) => {
                    out.separators[separator_index].mark_index = Some(i)
                }
                None => {}
            }
        }
        for i in 0..out.scene.labels.len() {
            if let Some(index) = out.target.label_part_index(i)? {
                let row = &mut out.parts[index];
                row.label_index = Some(i);
                out.labels.push(FunnelMotionLabel {
                    key: row.key.clone(),
                    source_index: row.source_index,
                    label_index: i,
                });
            }
        }
        out.scene.validate()?;
        self.frame = FunnelMotionFrame(Arc::new(out));
        Ok(())
    }
}
fn next(value: u64) -> Result<u64> {
    value
        .checked_add(1)
        .ok_or_else(|| ChartError("Funnel motion serial exhausted".into()))
}
fn state(value: &SpringValue) -> PropertyState {
    PropertyState {
        value: value.value(),
        target: value.target(),
        velocity: value.velocity(),
    }
}
