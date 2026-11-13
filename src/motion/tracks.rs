//! Independent physical property springs with stable mark/label identities.
//!
//! Painting constrains sizes, coordinates and paint channels to Scene's finite
//! domain; it never writes those constraints back into physical spring state.
//! Data values, text, anchors and legends are target metadata. Text/path/style
//! topology changes crossfade instead of inventing interpolation between them.
use super::geometry::{self, ContourBindings, GeometryTransitions};
use super::{SpringConfig, SpringValue};
use crate::scene::{ensure, *};
use crate::{BlendMode, MarkBlends};
use glory::render::{GraphicPoint, GraphicStroke, PathCommand};
use glory::style::Color;
use std::collections::{BTreeMap, BTreeSet};

const COORDINATE: f64 = 1_000_000.;
const COMMANDS: usize = 200_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FadeMode {
    Immediate,
    #[default]
    Fade,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PropertyMotionOptions {
    pub geometry: GeometryTransitions,
    pub spring: SpringConfig,
    pub presence_spring: SpringConfig,
    pub enter: FadeMode,
    pub exit: FadeMode,
    pub topology: FadeMode,
    pub shortest_rotation: bool,
    pub max_transient_marks: usize,
    pub max_transient_labels: usize,
    pub max_path_commands: usize,
}
impl Default for PropertyMotionOptions {
    fn default() -> Self {
        Self {
            geometry: GeometryTransitions::default(),
            spring: SpringConfig::default(),
            presence_spring: SpringConfig::default(),
            enter: FadeMode::Fade,
            exit: FadeMode::Fade,
            topology: FadeMode::Fade,
            shortest_rotation: true,
            max_transient_marks: MAX_MARKS,
            max_transient_labels: MAX_MARKS,
            max_path_commands: COMMANDS,
        }
    }
}
impl PropertyMotionOptions {
    pub fn validate(&self) -> Result<()> {
        self.spring.validate()?;
        self.presence_spring.validate()?;
        self.geometry.validate()?;
        ensure(
            self.max_transient_marks <= MAX_MARKS
                && self.max_transient_labels <= MAX_MARKS
                && self.max_path_commands <= COMMANDS,
            "property motion budget exceeds Scene limits",
        )
    }
    fn admit(&self, marks: usize, labels: usize, commands: usize) -> Result<()> {
        ensure(
            marks <= self.max_transient_marks
                && labels <= self.max_transient_labels
                && commands <= self.max_path_commands,
            "property motion transient union budget exceeded",
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PropertyState {
    pub value: f64,
    pub target: f64,
    pub velocity: f64,
}
impl From<&SpringValue> for PropertyState {
    fn from(value: &SpringValue) -> Self {
        Self {
            value: value.value(),
            target: value.target(),
            velocity: value.velocity(),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PropertyMotionStats {
    pub marks: usize,
    pub labels: usize,
    pub exiting_marks: usize,
    pub exiting_labels: usize,
    pub scalar_tracks: usize,
}

#[derive(Clone)]
struct Scalars(Vec<SpringValue>);
impl Scalars {
    fn new(values: Vec<f64>, config: SpringConfig) -> Result<Self> {
        Ok(Self(
            values
                .into_iter()
                .map(|value| SpringValue::new(value, config))
                .collect::<Result<_>>()?,
        ))
    }
    fn retarget(&mut self, values: Vec<f64>) -> Result<()> {
        ensure(
            self.0.len() == values.len(),
            "property motion scalar topology mismatch",
        )?;
        for (track, target) in self.0.iter_mut().zip(values) {
            track.retarget(target)?;
        }
        Ok(())
    }
    fn tick(&mut self, dt: f64) -> Result<()> {
        for track in &mut self.0 {
            track.tick(dt)?;
        }
        Ok(())
    }
    fn finish(&mut self) {
        for track in &mut self.0 {
            track.finish();
        }
    }
    fn settled(&self) -> bool {
        self.0.iter().all(SpringValue::is_settled)
    }
}
struct Reader<'a> {
    values: &'a [SpringValue],
    index: usize,
}
impl<'a> Reader<'a> {
    fn new(values: &'a Scalars) -> Self {
        Self {
            values: &values.0,
            index: 0,
        }
    }
    fn next(&mut self) -> f64 {
        let value = self.values[self.index].value();
        self.index += 1;
        value
    }
    fn point(&mut self) -> GraphicPoint {
        GraphicPoint::new(coordinate(self.next()), coordinate(self.next()))
    }
    fn color(&mut self) -> Color {
        Color(
            channel(self.next()),
            channel(self.next()),
            channel(self.next()),
            channel(self.next()),
        )
    }
}
fn channel(value: f64) -> u8 {
    value.clamp(0., 255.).round() as u8
}
fn coordinate(value: f64) -> f64 {
    value.clamp(-COORDINATE, COORDINATE)
}
fn paint_box(x: f64, y: f64, width: f64, height: f64) -> (f64, f64, f64, f64) {
    let x = coordinate(x);
    let y = coordinate(y);
    (
        x,
        y,
        width.clamp(0., COORDINATE).min((COORDINATE - x).max(0.)),
        height.clamp(0., COORDINATE).min((COORDINATE - y).max(0.)),
    )
}
fn angle(value: f64) -> f64 {
    if (-360. ..=360.).contains(&value) {
        value
    } else {
        (value + 360.).rem_euclid(720.) - 360.
    }
}
fn color_values(color: Color) -> [f64; 4] {
    [
        color.0.into(),
        color.1.into(),
        color.2.into(),
        color.3.into(),
    ]
}
fn mark_values(mark: &Mark) -> Vec<f64> {
    let mut values = match &mark.geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => vec![*x, *y, *width, *height],
        Geometry::Circle { x, y, radius } => vec![*x, *y, *radius],
        Geometry::Path(path) => {
            let mut values = Vec::new();
            for command in path {
                match command {
                    PathCommand::Move(p) | PathCommand::Line(p) => values.extend([p.x, p.y]),
                    PathCommand::Cubic { first, second, end } => {
                        values.extend([first.x, first.y, second.x, second.y, end.x, end.y])
                    }
                    PathCommand::Close => {}
                }
            }
            values
        }
    };
    if let Some(fill) = mark.fill {
        values.extend(color_values(fill));
    }
    if let Some(stroke) = mark.stroke {
        values.extend(color_values(stroke.color));
        values.push(stroke.width);
    }
    values.push(mark.opacity);
    if let Some(clip) = mark.clip {
        values.extend([clip.x, clip.y, clip.width, clip.height]);
    }
    values
}
fn mark_paths(mark: &Mark) -> Vec<String> {
    let mut paths: Vec<String> = match &mark.geometry {
        Geometry::Rect { .. } => [
            "geometry.x",
            "geometry.y",
            "geometry.width",
            "geometry.height",
        ]
        .into_iter()
        .map(str::to_string)
        .collect(),
        Geometry::Circle { .. } => ["geometry.x", "geometry.y", "geometry.radius"]
            .into_iter()
            .map(str::to_string)
            .collect(),
        Geometry::Path(commands) => {
            let mut paths = Vec::new();
            for (index, command) in commands.iter().enumerate() {
                match command {
                    PathCommand::Move(_) | PathCommand::Line(_) => {
                        paths.push(format!("geometry.path.{index}.x"));
                        paths.push(format!("geometry.path.{index}.y"));
                    }
                    PathCommand::Cubic { .. } => {
                        for point in ["first", "second", "end"] {
                            for axis in ["x", "y"] {
                                paths.push(format!("geometry.path.{index}.{point}.{axis}"));
                            }
                        }
                    }
                    PathCommand::Close => {}
                }
            }
            paths
        }
    };
    if mark.fill.is_some() {
        paths.extend(["fill.r", "fill.g", "fill.b", "fill.a"].map(str::to_string));
    }
    if mark.stroke.is_some() {
        paths.extend(
            [
                "stroke.r",
                "stroke.g",
                "stroke.b",
                "stroke.a",
                "stroke.width",
            ]
            .map(str::to_string),
        );
    }
    paths.push("opacity".into());
    if mark.clip.is_some() {
        paths.extend(["clip.x", "clip.y", "clip.width", "clip.height"].map(str::to_string));
    }
    paths
}
fn mark_compatible(a: &Mark, b: &Mark) -> bool {
    let shape = match (&a.geometry, &b.geometry) {
        (Geometry::Rect { .. }, Geometry::Rect { .. })
        | (Geometry::Circle { .. }, Geometry::Circle { .. }) => true,
        (Geometry::Path(a), Geometry::Path(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| std::mem::discriminant(a) == std::mem::discriminant(b))
        }
        _ => false,
    };
    shape
        && a.fill.is_some() == b.fill.is_some()
        && a.fill_paint == b.fill_paint
        && a.stroke_dash == b.stroke_dash
        && a.clip_path == b.clip_path
        && a.clip.is_some() == b.clip.is_some()
        && match (a.stroke, b.stroke) {
            (None, None) => true,
            (Some(a), Some(b)) => a.cap == b.cap,
            _ => false,
        }
}
fn command_count(mark: &Mark) -> usize {
    match &mark.geometry {
        Geometry::Path(path) => path.len(),
        _ => 0,
    }
}
fn presence(mode: FadeMode, config: SpringConfig) -> Result<SpringValue> {
    let mut value = SpringValue::new(if mode == FadeMode::Fade { 0. } else { 1. }, config)?;
    value.retarget(1.)?;
    Ok(value)
}
fn needs_fade(value: &SpringValue) -> bool {
    value.value() != 0. || value.velocity() != 0.
}

#[derive(Clone)]
struct MarkTrack {
    logical_id: String,
    metadata: Mark,
    values: Scalars,
    presence: SpringValue,
    exiting: bool,
    sampled: bool,
    blend: BlendMode,
}
impl MarkTrack {
    fn new(
        mark: Mark,
        mode: FadeMode,
        options: &PropertyMotionOptions,
        blend: BlendMode,
    ) -> Result<Self> {
        Ok(Self {
            logical_id: mark.id.clone(),
            values: Scalars::new(mark_values(&mark), options.spring)?,
            metadata: mark,
            presence: presence(mode, options.presence_spring)?,
            exiting: false,
            sampled: false,
            blend,
        })
    }
    fn retarget(&mut self, mark: &Mark) -> Result<()> {
        self.values.retarget(mark_values(mark))?;
        self.presence.retarget(1.)?;
        self.metadata = mark.clone();
        self.exiting = false;
        Ok(())
    }
    fn entering(
        mark: Mark,
        mode: FadeMode,
        options: &PropertyMotionOptions,
        work: &mut usize,
        blend: BlendMode,
    ) -> Result<Self> {
        let mut track = Self::new(mark.clone(), mode, options, blend)?;
        track.sampled = options.geometry.sampled(&mark.id);
        if let Some(origin) = options.geometry.enter.get(&mark.id) {
            let mut start = mark.clone();
            start.geometry = origin.clone();
            let start = geometry::prepare(&start, &options.geometry, Some(&mark.geometry), work)?;
            ensure(
                mark_compatible(&start, &mark),
                "custom enter geometry topology requires compatible commands or morph opt-in",
            )?;
            track.values = Scalars::new(mark_values(&start), options.spring)?;
            track.values.retarget(mark_values(&mark))?;
        }
        Ok(track)
    }
    fn exit_to(&mut self, options: &PropertyMotionOptions, work: &mut usize) -> Result<()> {
        if let Some(destination) = options.geometry.exit.get(&self.logical_id) {
            let mut target = self.metadata.clone();
            target.geometry = destination.clone();
            let actual_id = target.id.clone();
            target.id = self.logical_id.clone();
            let mut target = geometry::prepare(
                &target,
                &options.geometry,
                Some(&self.paint().geometry),
                work,
            )?;
            target.id = actual_id;
            ensure(
                mark_compatible(&self.metadata, &target),
                "custom exit geometry topology requires compatible commands or morph opt-in",
            )?;
            self.values.retarget(mark_values(&target))?;
            self.metadata = target;
        }
        self.exit()
    }
    fn can_exit(&self, mode: FadeMode) -> bool {
        mode == FadeMode::Fade
            && (self.metadata.has_fill() || self.metadata.stroke.is_some())
            && needs_fade(&self.presence)
    }
    fn exit(&mut self) -> Result<()> {
        self.exiting = true;
        self.metadata.label.clear();
        self.presence.retarget(0.)
    }
    fn tick(&mut self, dt: f64) -> Result<()> {
        self.values.tick(dt)?;
        self.presence.tick(dt)?;
        Ok(())
    }
    fn settled(&self) -> bool {
        self.values.settled() && self.presence.is_settled()
    }
    fn paint(&self) -> Mark {
        self.paint_with_presence(self.presence.value().clamp(0., 1.))
    }
    fn paint_with_presence(&self, presence: f64) -> Mark {
        let mut mark = self.metadata.clone();
        let mut r = Reader::new(&self.values);
        mark.geometry = match &self.metadata.geometry {
            Geometry::Rect { .. } => {
                let (x, y, width, height) = paint_box(r.next(), r.next(), r.next(), r.next());
                Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                }
            }
            Geometry::Circle { .. } => {
                let x = coordinate(r.next());
                let y = coordinate(r.next());
                let radius = r
                    .next()
                    .clamp(0., COORDINATE)
                    .min(COORDINATE - x.abs())
                    .min(COORDINATE - y.abs());
                Geometry::Circle { x, y, radius }
            }
            Geometry::Path(path) => Geometry::Path(
                path.iter()
                    .map(|command| match command {
                        PathCommand::Move(_) => PathCommand::Move(r.point()),
                        PathCommand::Line(_) => PathCommand::Line(r.point()),
                        PathCommand::Cubic { .. } => PathCommand::Cubic {
                            first: r.point(),
                            second: r.point(),
                            end: r.point(),
                        },
                        PathCommand::Close => PathCommand::Close,
                    })
                    .collect(),
            ),
        };
        mark.fill = self.metadata.fill.map(|_| r.color());
        mark.stroke = self.metadata.stroke.map(|stroke| GraphicStroke {
            color: r.color(),
            width: r.next().clamp(0.000001, 10000.),
            cap: stroke.cap,
        });
        mark.opacity = r.next().clamp(0., 1.) * presence;
        mark.clip = self.metadata.clip.map(|_| {
            let (x, y, width, height) = paint_box(r.next(), r.next(), r.next(), r.next());
            ClipRect {
                x,
                y,
                width,
                height,
            }
        });
        debug_assert_eq!(r.index, self.values.0.len());
        mark
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum LabelKey {
    Slot(usize),
    Ghost(u64),
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Item {
    Mark(String),
    Label(LabelKey),
}
fn label_values(label: &Label) -> Vec<f64> {
    let mut values = vec![label.x, label.y, label.size, label.rotation];
    values.extend(color_values(label.color));
    values
}
#[derive(Clone)]
struct LabelTrack {
    metadata: Label,
    font: crate::FontOptions,
    owner: Option<String>,
    clip: Option<crate::ClipPath>,
    values: Scalars,
    presence: SpringValue,
    exiting: bool,
}
impl LabelTrack {
    fn new(
        label: Label,
        font: crate::FontOptions,
        owner: Option<String>,
        clip: Option<crate::ClipPath>,
        mode: FadeMode,
        options: &PropertyMotionOptions,
    ) -> Result<Self> {
        Ok(Self {
            values: Scalars::new(label_values(&label), options.spring)?,
            metadata: label,
            font,
            owner,
            clip,
            presence: presence(mode, options.presence_spring)?,
            exiting: false,
        })
    }
    fn compatible(
        &self,
        label: &Label,
        font: &crate::FontOptions,
        owner: Option<&String>,
        clip: Option<&crate::ClipPath>,
    ) -> bool {
        self.metadata.text == label.text
            && self.font.resolve(&crate::ResolvedFont::default())
                == font.resolve(&crate::ResolvedFont::default())
            && self.metadata.anchor == label.anchor
            && self.owner.as_ref() == owner
            && self.clip.as_ref() == clip
    }
    fn retarget(
        &mut self,
        label: &Label,
        font: crate::FontOptions,
        owner: Option<String>,
        clip: Option<crate::ClipPath>,
        shortest: bool,
    ) -> Result<()> {
        let mut values = label_values(label);
        if shortest {
            let current = self.values.0[3].value();
            values[3] = current + (label.rotation - current + 180.).rem_euclid(360.) - 180.;
        }
        self.values.retarget(values)?;
        self.presence.retarget(1.)?;
        self.metadata = label.clone();
        self.font = font;
        self.owner = owner;
        self.clip = clip;
        self.exiting = false;
        Ok(())
    }
    fn can_exit(&self, mode: FadeMode) -> bool {
        mode == FadeMode::Fade && needs_fade(&self.presence)
    }
    fn exit(&mut self) -> Result<()> {
        self.exiting = true;
        self.presence.retarget(0.)
    }
    fn tick(&mut self, dt: f64) -> Result<()> {
        self.values.tick(dt)?;
        self.presence.tick(dt)?;
        Ok(())
    }
    fn settled(&self) -> bool {
        self.values.settled() && self.presence.is_settled()
    }
    fn paint(&self) -> Label {
        let mut label = self.metadata.clone();
        let mut r = Reader::new(&self.values);
        label.x = coordinate(r.next());
        label.y = coordinate(r.next());
        label.size = r.next().clamp(1., 256.);
        label.rotation = angle(r.next());
        label.color = r.color();
        label.color.3 =
            (f64::from(label.color.3) * self.presence.value().clamp(0., 1.)).round() as u8;
        label
    }
}

/// Complete per-property scene engine. Stable compatible identities preserve
/// independent physical velocities on retarget, including interrupted enters.
/// `tick`/`retarget` validate a candidate frame before committing any state.
#[derive(Clone)]
pub struct PropertyMotion {
    current: Scene,
    target: Scene,
    options: PropertyMotionOptions,
    marks: BTreeMap<String, MarkTrack>,
    labels: BTreeMap<LabelKey, LabelTrack>,
    globals: Scalars,
    order: Vec<Item>,
    target_order: Vec<Item>,
    next_ghost: u64,
    reduced_motion: bool,
    contour_bindings: ContourBindings,
    current_blends: MarkBlends,
    target_blends: MarkBlends,
}
impl PropertyMotion {
    pub fn new(initial: Scene, options: PropertyMotionOptions) -> Result<Self> {
        Self::new_with_contours(initial, options, ContourBindings::new())
    }
    /// Explicit input contour identities for multi-contour profiles. Missing
    /// binding entries use the profile's complete declared contour order.
    pub fn new_with_contours(
        initial: Scene,
        options: PropertyMotionOptions,
        bindings: ContourBindings,
    ) -> Result<Self> {
        Self::new_with_contours_and_blends(initial, options, bindings, MarkBlends::default())
    }
    pub fn new_with_blends(
        initial: Scene,
        options: PropertyMotionOptions,
        blends: MarkBlends,
    ) -> Result<Self> {
        Self::new_with_contours_and_blends(initial, options, ContourBindings::new(), blends)
    }
    pub fn new_with_contours_and_blends(
        initial: Scene,
        options: PropertyMotionOptions,
        bindings: ContourBindings,
        blends: MarkBlends,
    ) -> Result<Self> {
        initial.validate()?;
        ensure(
            initial.opacity_groups.is_empty(),
            "property motion does not support multi-mark opacity groups",
        )?;
        blends.validate(&initial)?;
        options.validate()?;
        geometry::validate_bindings(&initial, &options.geometry, &bindings)?;
        options.admit(
            initial.marks.len(),
            initial.labels.len(),
            initial
                .marks
                .iter()
                .map(|mark| geometry::command_count(mark, &options.geometry))
                .sum(),
        )?;
        let mut work = 0;
        let prepared = initial
            .marks
            .iter()
            .map(|mark| {
                geometry::prepare_bound(
                    mark,
                    &options.geometry,
                    bindings.get(&mark.id).map(Vec::as_slice),
                    None,
                    &mut work,
                )
            })
            .collect::<Result<Vec<_>>>()?;
        options.admit(
            prepared.len(),
            initial.labels.len(),
            prepared.iter().map(command_count).sum(),
        )?;
        let marks = prepared
            .into_iter()
            .map(|mark| {
                Ok((mark.id.clone(), {
                    let sampled = options.geometry.sampled(&mark.id);
                    let mode = blends.mode(&mark.id);
                    let mut track = MarkTrack::new(mark, FadeMode::Immediate, &options, mode)?;
                    track.sampled = sampled;
                    track
                }))
            })
            .collect::<Result<_>>()?;
        let labels = initial
            .labels
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, label)| {
                Ok((
                    LabelKey::Slot(index),
                    LabelTrack::new(
                        label,
                        initial.typography.label_options(index),
                        initial.label_series.get(&index).cloned(),
                        initial.label_clips.get(&index).cloned(),
                        FadeMode::Immediate,
                        &options,
                    )?,
                ))
            })
            .collect::<Result<_>>()?;
        let globals = Scalars::new(global_values(&initial), options.spring)?;
        let order = scene_order(&initial);
        Ok(Self {
            current: initial.clone(),
            target: initial,
            options,
            marks,
            labels,
            globals,
            order: order.clone(),
            target_order: order,
            next_ghost: 0,
            reduced_motion: false,
            contour_bindings: bindings,
            current_blends: blends.clone(),
            target_blends: blends,
        })
    }
    pub fn scene(&self) -> &Scene {
        &self.current
    }
    /// Discrete modes indexed by the actual live and retiring paint identities.
    pub fn blends(&self) -> &MarkBlends {
        &self.current_blends
    }
    fn paint_blends(&self) -> Result<MarkBlends> {
        MarkBlends::from_motion_entries(
            self.marks
                .values()
                .filter(|track| track.blend != BlendMode::Normal)
                .map(|track| (track.metadata.id.clone(), track.blend)),
        )
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn blend_admission_frames(&self) -> Result<Option<[Scene; 2]>> {
        if self.current_blends.is_empty() || self.is_settled() {
            return Ok(None);
        }
        // Include resident exit tracks at their actual geometry targets even
        // though their final presence would ordinarily be zero.
        let mut start = self.current.clone();
        let mut end = self.current.clone();
        for (start, end) in start.marks.iter_mut().zip(&mut end.marks) {
            if self.current_blends.mode(&start.id) == BlendMode::Normal {
                continue;
            }
            let mut track = self
                .marks
                .get(&start.id)
                .ok_or_else(|| ChartError("blend track is absent from the physical frame".into()))?
                .clone();
            *start = track.paint_with_presence(1.);
            track.values.finish();
            *end = track.paint_with_presence(1.);
        }
        end.width = self.target.width;
        end.height = self.target.height;
        Ok(Some([start, end]))
    }
    pub fn target(&self) -> &Scene {
        &self.target
    }
    pub fn options(&self) -> &PropertyMotionOptions {
        &self.options
    }
    /// Bindings accepted with the current exact target. Empty mark entries and
    /// absent mark entries have distinct collapse/default-order semantics.
    pub fn contour_bindings(&self) -> &ContourBindings {
        &self.contour_bindings
    }
    pub fn is_settled(&self) -> bool {
        self.globals.settled()
            && self.marks.values().all(|m| !m.exiting && m.settled())
            && self.labels.values().all(|l| !l.exiting && l.settled())
    }
    pub fn stats(&self) -> PropertyMotionStats {
        PropertyMotionStats {
            marks: self.marks.len(),
            labels: self.labels.len(),
            exiting_marks: self.marks.values().filter(|m| m.exiting).count(),
            exiting_labels: self.labels.values().filter(|l| l.exiting).count(),
            scalar_tracks: self.globals.0.len()
                + self
                    .marks
                    .values()
                    .map(|m| m.values.0.len() + 1)
                    .sum::<usize>()
                + self
                    .labels
                    .values()
                    .map(|l| l.values.0.len() + 1)
                    .sum::<usize>(),
        }
    }
    /// Raw physical state, before paint clamping/quantization. Example paths:
    /// geometry.x, geometry.path.2.first.x, fill.r, stroke.width, clip.x, presence.
    pub fn mark_property(&self, id: &str, property: &str) -> Option<PropertyState> {
        let track = self.marks.get(id)?;
        if property == "presence" {
            return Some((&track.presence).into());
        }
        let index = mark_paths(&track.metadata)
            .iter()
            .position(|path| path == property)?;
        Some((&track.values.0[index]).into())
    }
    pub fn mark_velocity(&self, id: &str, property: &str) -> Option<f64> {
        self.mark_property(id, property).map(|value| value.velocity)
    }
    pub fn label_property(&self, slot: usize, property: &str) -> Option<PropertyState> {
        let track = self.labels.get(&LabelKey::Slot(slot))?;
        if property == "presence" {
            return Some((&track.presence).into());
        }
        let index = [
            "x", "y", "size", "rotation", "color.r", "color.g", "color.b", "color.a",
        ]
        .iter()
        .position(|path| *path == property)?;
        Some((&track.values.0[index]).into())
    }
    pub fn label_velocity(&self, slot: usize, property: &str) -> Option<f64> {
        self.label_property(slot, property)
            .map(|value| value.velocity)
    }
    pub fn frame_property(&self, property: &str) -> Option<PropertyState> {
        let index = [
            "width",
            "height",
            "background.r",
            "background.g",
            "background.b",
            "background.a",
        ]
        .iter()
        .position(|path| *path == property)?;
        Some((&self.globals.0[index]).into())
    }
    pub fn retarget(&mut self, target: Scene) -> Result<()> {
        self.retarget_with_contours(target, ContourBindings::new())
    }
    /// Keys identify each input Move contour; omitted slots animate to their
    /// declared collapse point. Correspondence/validation changes are atomic.
    pub fn retarget_with_contours(
        &mut self,
        target: Scene,
        bindings: ContourBindings,
    ) -> Result<()> {
        self.retarget_with_contours_and_blends(target, bindings, MarkBlends::default())
    }
    /// Blend modes are discrete paint topology. Compatible tracks preserve
    /// their velocity, while entering/retiring tracks each own their mode.
    pub fn retarget_with_blends(&mut self, target: Scene, blends: MarkBlends) -> Result<()> {
        self.retarget_with_contours_and_blends(target, ContourBindings::new(), blends)
    }
    pub fn retarget_with_contours_and_blends(
        &mut self,
        target: Scene,
        bindings: ContourBindings,
        blends: MarkBlends,
    ) -> Result<()> {
        target.validate()?;
        ensure(
            target.opacity_groups.is_empty(),
            "property motion does not support multi-mark opacity groups",
        )?;
        blends.validate(&target)?;
        geometry::validate_bindings(&target, &self.options.geometry, &bindings)?;
        self.options.admit(
            target.marks.len(),
            target.labels.len(),
            target
                .marks
                .iter()
                .map(|mark| geometry::command_count(mark, &self.options.geometry))
                .sum(),
        )?;
        if target == self.target
            && bindings == self.contour_bindings
            && blends == self.target_blends
        {
            return Ok(());
        }
        if self.reduced_motion {
            let mut next =
                Self::new_with_contours_and_blends(target, self.options.clone(), bindings, blends)?;
            next.next_ghost = self.next_ghost;
            next.reduced_motion = true;
            *self = next;
            return Ok(());
        }
        let mut next = self.clone();
        next.retarget_inner(target, bindings, blends)?;
        next.prune();
        next.current = next.paint()?;
        next.current_blends = next.paint_blends()?;
        if next.is_settled() {
            next.finish();
        }
        *self = next;
        Ok(())
    }
    pub fn tick(&mut self, dt: f64) -> Result<&Scene> {
        ensure(
            dt.is_finite() && (0. ..=60.).contains(&dt),
            "property motion dt must be finite and in 0..=60 seconds",
        )?;
        if self.is_settled() || dt == 0. {
            return Ok(&self.current);
        }
        let mut next = self.clone();
        next.globals.tick(dt)?;
        for mark in next.marks.values_mut() {
            mark.tick(dt)?;
        }
        for label in next.labels.values_mut() {
            label.tick(dt)?;
        }
        next.prune();
        next.current = next.paint()?;
        next.current_blends = next.paint_blends()?;
        if next.is_settled() {
            next.finish();
        }
        *self = next;
        Ok(&self.current)
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced {
            self.finish();
        }
    }
    pub fn finish(&mut self) {
        self.globals.finish();
        self.marks.retain(|_, m| !m.exiting);
        for mark in self.marks.values_mut() {
            mark.values.finish();
            mark.presence.finish();
        }
        // Exact target geometry/metadata is restored after sampled motion.
        // New tracks are settled and have no retained velocity to discard.
        for mark in &self.target.marks {
            if let Some(track) = self.marks.get_mut(&mark.id) {
                track.metadata = mark.clone();
                track.sampled = false;
                track.values = Scalars::new(mark_values(mark), self.options.spring)
                    .expect("validated finite target scalars");
            }
        }
        self.labels.retain(|_, l| !l.exiting);
        for label in self.labels.values_mut() {
            label.values.finish();
            label.presence.finish();
        }
        self.order = self.target_order.clone();
        self.current = self.target.clone();
        self.current_blends = self.target_blends.clone();
    }
    fn prune(&mut self) {
        self.marks
            .retain(|_, m| !m.exiting || !m.presence.is_settled() || m.presence.value() != 0.);
        self.labels
            .retain(|_, l| !l.exiting || !l.presence.is_settled() || l.presence.value() != 0.);
        let marks = &self.marks;
        let labels = &self.labels;
        self.order.retain(|item| match item {
            Item::Mark(id) => marks.contains_key(id),
            Item::Label(key) => labels.contains_key(key),
        });
    }
    fn ghost(&mut self, reserved: &mut BTreeSet<String>) -> Result<(u64, String)> {
        loop {
            let number = self.next_ghost;
            self.next_ghost = self
                .next_ghost
                .checked_add(1)
                .ok_or_else(|| ChartError("property motion ghost identity exhausted".into()))?;
            let id = format!("kartis-property-ghost-{number}");
            if reserved.insert(id.clone()) {
                return Ok((number, id));
            }
        }
    }
    fn rename_mark(&mut self, id: &str, reserved: &mut BTreeSet<String>) -> Result<()> {
        let mut track = self.marks.remove(id).expect("existing property track");
        let (_, ghost) = self.ghost(reserved)?;
        track.metadata.id = ghost.clone();
        self.marks.insert(ghost.clone(), track);
        for item in &mut self.order {
            if matches!(item,Item::Mark(key)if key==id) {
                *item = Item::Mark(ghost.clone());
            }
        }
        Ok(())
    }
    fn rename_label(&mut self, key: LabelKey, reserved: &mut BTreeSet<String>) -> Result<()> {
        let (number, _) = self.ghost(reserved)?;
        let ghost = LabelKey::Ghost(number);
        let track = self.labels.remove(&key).expect("existing label track");
        self.labels.insert(ghost, track);
        for item in &mut self.order {
            if *item == Item::Label(key) {
                *item = Item::Label(ghost);
            }
        }
        Ok(())
    }
    fn retarget_inner(
        &mut self,
        target: Scene,
        bindings: ContourBindings,
        blends: MarkBlends,
    ) -> Result<()> {
        self.prune();
        let original_target = target;
        let mut target = original_target.clone();
        let mut work = 0;
        // A settled exact target may have its original topology. Re-establish
        // sampled correspondence only for opted-in IDs; active sample tracks
        // retain their raw scalar state and velocities.
        for (id, track) in &mut self.marks {
            if !track.exiting && !track.sampled && self.options.geometry.sampled(id) {
                let painted = track.paint();
                let normalized = geometry::prepare_bound(
                    &painted,
                    &self.options.geometry,
                    self.contour_bindings.get(id).map(Vec::as_slice),
                    None,
                    &mut work,
                )?;
                track.values = Scalars::new(mark_values(&normalized), self.options.spring)?;
                track.metadata = normalized;
                track.sampled = true;
            }
        }
        for mark in &mut target.marks {
            let reference = self.marks.get(&mark.id).map(MarkTrack::paint);
            *mark = geometry::prepare_bound(
                mark,
                &self.options.geometry,
                bindings.get(&mark.id).map(Vec::as_slice),
                reference.as_ref().map(|m| &m.geometry),
                &mut work,
            )?;
        }
        let target_order = scene_order(&target);
        let target_ids: BTreeSet<_> = target.marks.iter().map(|m| m.id.clone()).collect();
        let reuse_marks: BTreeSet<_> = target
            .marks
            .iter()
            .filter(|mark| {
                self.marks.get(&mark.id).is_some_and(|old| {
                    old.logical_id == mark.id
                        && old.blend == blends.mode(&mark.id)
                        && mark_compatible(&old.metadata, mark)
                })
            })
            .map(|mark| mark.id.clone())
            .collect();
        let reuse_labels: BTreeSet<_> = target
            .labels
            .iter()
            .enumerate()
            .filter(|(index, label)| {
                self.labels.get(&LabelKey::Slot(*index)).is_some_and(|old| {
                    old.compatible(
                        label,
                        &target.typography.label_options(*index),
                        target.label_series.get(index),
                        target.label_clips.get(index),
                    )
                })
            })
            .map(|(index, _)| LabelKey::Slot(index))
            .collect();
        let survivors: BTreeSet<Item> = reuse_marks
            .iter()
            .cloned()
            .map(Item::Mark)
            .chain(reuse_labels.iter().copied().map(Item::Label))
            .collect();
        let from_survivors: Vec<_> = self
            .order
            .iter()
            .filter(|key| survivors.contains(*key))
            .cloned()
            .collect();
        let to_survivors: Vec<_> = target_order
            .iter()
            .filter(|key| survivors.contains(*key))
            .cloned()
            .collect();
        let snapshot = self.options.topology == FadeMode::Fade && from_survivors != to_survivors;
        self.preflight(&target, &reuse_marks, &reuse_labels, snapshot)?;
        let old_scene = self.current.clone();
        let mut reserved: BTreeSet<_> = self
            .marks
            .keys()
            .cloned()
            .chain(target_ids.iter().cloned())
            .collect();
        let old_keys: Vec<_> = self.marks.keys().cloned().collect();
        for id in old_keys {
            if !target_ids.contains(&id) && !self.marks[&id].exiting {
                if self.marks[&id].can_exit(self.options.exit) {
                    self.marks
                        .get_mut(&id)
                        .expect("live track")
                        .exit_to(&self.options, &mut work)?;
                } else {
                    self.marks.remove(&id);
                }
            }
        }
        for mark in &target.marks {
            if reuse_marks.contains(&mark.id) {
                self.marks
                    .get_mut(&mark.id)
                    .expect("matching property track")
                    .retarget(mark)?;
                continue;
            }
            let mismatch = self.marks.contains_key(&mark.id);
            if mismatch {
                if self.marks[&mark.id].exiting
                    || self.marks[&mark.id].can_exit(self.options.topology)
                {
                    self.marks
                        .get_mut(&mark.id)
                        .expect("live track")
                        .exit_to(&self.options, &mut work)?;
                    self.rename_mark(&mark.id, &mut reserved)?;
                } else {
                    self.marks.remove(&mark.id);
                }
            }
            let mode = if mismatch {
                self.options.topology
            } else {
                self.options.enter
            };
            self.marks.insert(
                mark.id.clone(),
                MarkTrack::entering(
                    mark.clone(),
                    mode,
                    &self.options,
                    &mut work,
                    blends.mode(&mark.id),
                )?,
            );
        }
        let old_labels: Vec<_> = self.labels.keys().copied().collect();
        for key in old_labels {
            if matches!(key,LabelKey::Slot(index)if index>=target.labels.len())
                && !self.labels[&key].exiting
            {
                if self.labels[&key].can_exit(self.options.exit) {
                    self.labels.get_mut(&key).expect("live label").exit()?;
                } else {
                    self.labels.remove(&key);
                }
            }
        }
        for (index, label) in target.labels.iter().enumerate() {
            let key = LabelKey::Slot(index);
            let owner = target.label_series.get(&index).cloned();
            let clip = target.label_clips.get(&index).cloned();
            if reuse_labels.contains(&key) {
                self.labels
                    .get_mut(&key)
                    .expect("matching label")
                    .retarget(
                        label,
                        target.typography.label_options(index),
                        owner,
                        clip,
                        self.options.shortest_rotation,
                    )?;
                continue;
            }
            let mismatch = self.labels.contains_key(&key);
            if mismatch {
                if self.labels[&key].exiting || self.labels[&key].can_exit(self.options.topology) {
                    self.labels.get_mut(&key).expect("live label").exit()?;
                    self.rename_label(key, &mut reserved)?;
                } else {
                    self.labels.remove(&key);
                }
            }
            let mode = if mismatch {
                self.options.topology
            } else {
                self.options.enter
            };
            self.labels.insert(
                key,
                LabelTrack::new(
                    label.clone(),
                    target.typography.label_options(index),
                    owner,
                    clip,
                    mode,
                    &self.options,
                )?,
            );
        }
        self.globals.retarget(global_values(&target))?;
        if snapshot {
            self.snapshot_order(&old_scene, &target_order, &mut reserved)?;
        } else {
            self.weave_order(&target_order);
        }
        self.target = original_target;
        self.target_blends = blends;
        self.contour_bindings = bindings;
        self.target_order = target_order;
        Ok(())
    }
    fn preflight(
        &self,
        target: &Scene,
        reuse_marks: &BTreeSet<String>,
        reuse_labels: &BTreeSet<LabelKey>,
        snapshot: bool,
    ) -> Result<()> {
        if snapshot {
            let old_marks: Vec<_> = self
                .current
                .marks
                .iter()
                .filter(|m| m.opacity > 0. && (m.has_fill() || m.stroke.is_some()))
                .collect();
            let old_labels = self.current.labels.iter().filter(|l| l.color.3 > 0).count();
            return self.options.admit(
                target.marks.len() + old_marks.len(),
                target.labels.len() + old_labels,
                target.marks.iter().map(command_count).sum::<usize>()
                    + old_marks.iter().map(|m| command_count(m)).sum::<usize>(),
            );
        }
        let target_ids: BTreeSet<_> = target.marks.iter().map(|m| m.id.as_str()).collect();
        let kept: Vec<_> = self
            .marks
            .iter()
            .filter(|(id, old)| {
                !reuse_marks.contains(*id)
                    && (old.exiting
                        || old.can_exit(if target_ids.contains(id.as_str()) {
                            self.options.topology
                        } else {
                            self.options.exit
                        }))
            })
            .map(|(_, m)| m)
            .collect();
        let kept_labels = self
            .labels
            .iter()
            .filter(|(key, old)| {
                !reuse_labels.contains(key)
                    && (old.exiting
                        || old.can_exit(
                            if matches!(key,LabelKey::Slot(index)if *index<target.labels.len()) {
                                self.options.topology
                            } else {
                                self.options.exit
                            },
                        ))
            })
            .count();
        self.options.admit(
            target.marks.len() + kept.len(),
            target.labels.len() + kept_labels,
            target.marks.iter().map(command_count).sum::<usize>()
                + kept
                    .iter()
                    .map(|m| command_count(&m.metadata))
                    .sum::<usize>(),
        )
    }
    fn exists(&self, item: &Item) -> bool {
        match item {
            Item::Mark(id) => self.marks.contains_key(id),
            Item::Label(key) => self.labels.contains_key(key),
        }
    }
    fn weave_order(&mut self, target: &[Item]) {
        let target_set: BTreeSet<_> = target.iter().cloned().collect();
        let survivors: BTreeSet<_> = self
            .order
            .iter()
            .filter(|item| target_set.contains(*item))
            .cloned()
            .collect();
        if survivors.is_empty() {
            let mut order: Vec<_> = self
                .order
                .iter()
                .filter(|item| self.exists(item) && !target_set.contains(*item))
                .cloned()
                .collect();
            order.extend_from_slice(target);
            self.order = order;
            return;
        }
        let mut before = BTreeMap::<Item, Vec<Item>>::new();
        let mut pending = Vec::new();
        for item in &self.order {
            if !self.exists(item) {
                continue;
            }
            if target_set.contains(item) {
                before.insert(item.clone(), std::mem::take(&mut pending));
            } else {
                pending.push(item.clone());
            }
        }
        let mut order = Vec::new();
        for item in target {
            if let Some(mut old) = before.remove(item) {
                order.append(&mut old);
            }
            order.push(item.clone());
        }
        order.append(&mut pending);
        self.order = order;
    }
    fn snapshot_order(
        &mut self,
        old: &Scene,
        target: &[Item],
        reserved: &mut BTreeSet<String>,
    ) -> Result<()> {
        self.marks.retain(|_, m| !m.exiting);
        self.labels.retain(|_, l| !l.exiting);
        for mark in self.marks.values_mut() {
            mark.presence = presence(FadeMode::Fade, self.options.presence_spring)?;
        }
        for label in self.labels.values_mut() {
            label.presence = presence(FadeMode::Fade, self.options.presence_spring)?;
        }
        let mut old_marks = BTreeMap::new();
        let mut old_labels = BTreeMap::new();
        for (index, mark) in old.marks.iter().enumerate() {
            if mark.opacity <= 0. || (!mark.has_fill() && mark.stroke.is_none()) {
                continue;
            }
            let (_, id) = self.ghost(reserved)?;
            let mut track = MarkTrack::new(
                mark.clone(),
                FadeMode::Immediate,
                &self.options,
                self.current_blends.mode(&mark.id),
            )?;
            track.metadata.id = id.clone();
            track.exit()?;
            self.marks.insert(id.clone(), track);
            old_marks.insert(index, Item::Mark(id));
        }
        for (index, label) in old.labels.iter().enumerate() {
            if label.color.3 == 0 {
                continue;
            }
            let (number, _) = self.ghost(reserved)?;
            let key = LabelKey::Ghost(number);
            let mut track = LabelTrack::new(
                label.clone(),
                old.typography.label_options(index),
                old.label_series.get(&index).cloned(),
                old.label_clips.get(&index).cloned(),
                FadeMode::Immediate,
                &self.options,
            )?;
            track.exit()?;
            self.labels.insert(key, track);
            old_labels.insert(index, Item::Label(key));
        }
        self.order = old
            .draw_items()
            .into_iter()
            .filter_map(|item| match item {
                DrawItem::Mark(index) => old_marks.get(&index).cloned(),
                DrawItem::Label(index) => old_labels.get(&index).cloned(),
            })
            .chain(target.iter().cloned())
            .collect();
        Ok(())
    }
    fn paint(&self) -> Result<Scene> {
        let mut scene = self.target.clone();
        let mut r = Reader::new(&self.globals);
        scene.width = r.next().clamp(1., 16384.);
        scene.height = r.next().clamp(1., 16384.);
        scene.background = r.color();
        scene.marks.clear();
        scene.labels.clear();
        scene.label_series.clear();
        scene.label_clips.clear();
        scene.typography.labels.clear();
        let mut marks = BTreeMap::new();
        let mut labels = BTreeMap::new();
        for (id, track) in &self.marks {
            marks.insert(id.clone(), scene.marks.len());
            scene.marks.push(track.paint());
        }
        for (key, track) in &self.labels {
            labels.insert(*key, scene.labels.len());
            let mut font = track.font.clone();
            if font.is_empty() && !scene.typography.default.is_empty() {
                let resolved = crate::ResolvedFont::default();
                font.family = Some(resolved.family);
                font.weight = Some(resolved.weight);
            }
            if !font.is_empty() {
                scene.typography.labels.insert(scene.labels.len(), font);
            }
            if let Some(owner) = &track.owner {
                scene.label_series.insert(scene.labels.len(), owner.clone());
            }
            if let Some(clip) = &track.clip {
                scene.label_clips.insert(scene.labels.len(), clip.clone());
            }
            scene.labels.push(track.paint());
        }
        scene.draw_order = self
            .order
            .iter()
            .map(|item| match item {
                Item::Mark(id) => DrawItem::Mark(marks[id]),
                Item::Label(key) => DrawItem::Label(labels[key]),
            })
            .collect();
        self.options.admit(
            scene.marks.len(),
            scene.labels.len(),
            scene.marks.iter().map(command_count).sum(),
        )?;
        scene.validate()?;
        Ok(scene)
    }
}
fn global_values(scene: &Scene) -> Vec<f64> {
    let mut values = vec![scene.width, scene.height];
    values.extend(color_values(scene.background));
    values
}
fn scene_order(scene: &Scene) -> Vec<Item> {
    scene
        .draw_items()
        .into_iter()
        .map(|item| match item {
            DrawItem::Mark(index) => Item::Mark(scene.marks[index].id.clone()),
            DrawItem::Label(index) => Item::Label(LabelKey::Slot(index)),
        })
        .collect()
}

impl crate::clock::SceneMotion for PropertyMotion {
    fn scene(&self) -> &Scene {
        PropertyMotion::scene(self)
    }
    fn is_settled(&self) -> bool {
        PropertyMotion::is_settled(self)
    }
    fn retarget(&mut self, scene: Scene) -> Result<()> {
        PropertyMotion::retarget(self, scene)
    }
    fn tick(&mut self, dt: f64) -> Result<()> {
        PropertyMotion::tick(self, dt).map(|_| ())
    }
    fn set_reduced_motion(&mut self, reduced: bool) {
        PropertyMotion::set_reduced_motion(self, reduced);
    }
}
