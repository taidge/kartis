//! Explicit-clock spring scene transitions. Applications call tick from their
//! host's animation loop; this module installs no scheduler or global state.
//! Compatible geometry interpolates by ID. Entering/exiting and incompatible
//! topology crossfade, with every intermediate frame validated before commit.
//! Departing items keep their position around surviving paint-order anchors.
//! If those anchors themselves reorder, complete ordered snapshots crossfade.
use crate::MarkBlends;
use crate::scene::*;
use glory::render::PathCommand;
use glory::style::Color;
use std::collections::{BTreeMap, BTreeSet};
pub mod geometry;
pub mod tracks;
pub use geometry::{
    ClosedAlignment, ContourBindings, ContourMorphOptions, GeometryTransitions, MorphContour,
    PathMorphOptions, sampled_contours, sampled_geometry,
};
pub use tracks::{
    FadeMode, PropertyMotion, PropertyMotionOptions, PropertyMotionStats, PropertyState,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringConfig {
    pub mass: f64,
    pub stiffness: f64,
    pub damping: f64,
    pub position_epsilon: f64,
    pub velocity_epsilon: f64,
}
impl Default for SpringConfig {
    fn default() -> Self {
        Self {
            mass: 1.,
            stiffness: 170.,
            damping: 26.,
            position_epsilon: 0.0001,
            velocity_epsilon: 0.0001,
        }
    }
}
impl SpringConfig {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.mass.is_finite()
                && (0.001..=1000.).contains(&self.mass)
                && self.stiffness.is_finite()
                && (0.001..=1_000_000.).contains(&self.stiffness)
                && self.damping.is_finite()
                && (0. ..=10000.).contains(&self.damping),
            "invalid spring configuration",
        )?;
        ensure(
            [self.position_epsilon, self.velocity_epsilon]
                .iter()
                .all(|v| v.is_finite() && (0.00000001..=0.1).contains(v)),
            "invalid spring rest threshold",
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum MotionKey {
    Mark(String),
    Label(usize),
}
#[derive(Clone)]
struct Plan {
    from: Scene,
    to: Scene,
    compatible_from: Scene,
    compatible_to: Scene,
    compatible_ids: BTreeSet<String>,
    compatible_labels: BTreeSet<usize>,
    exits: Vec<Mark>,
    exit_labels: Vec<usize>,
    draw_order: Vec<DrawItem>,
    paint_blends: MarkBlends,
    target_blends: MarkBlends,
}
fn motion_key(
    scene: &Scene,
    item: DrawItem,
    marks: &BTreeSet<String>,
    labels: &BTreeSet<usize>,
) -> Option<MotionKey> {
    match item {
        DrawItem::Mark(index) => marks
            .contains(&scene.marks[index].id)
            .then(|| MotionKey::Mark(scene.marks[index].id.clone())),
        DrawItem::Label(index) => labels.contains(&index).then_some(MotionKey::Label(index)),
    }
}
impl Plan {
    fn new(
        from: Scene,
        to: Scene,
        from_blends: &MarkBlends,
        target_blends: MarkBlends,
    ) -> Result<Self> {
        from.validate()?;
        to.validate()?;
        ensure(
            from.opacity_groups.is_empty() && to.opacity_groups.is_empty(),
            "spring topology motion does not support multi-mark opacity groups; use immediate replacement or Scene::interpolate",
        )?;
        let previous: BTreeMap<_, _> = from.marks.iter().map(|m| (m.id.as_str(), m)).collect();
        let mut compatible_ids: BTreeSet<_> = to
            .marks
            .iter()
            .filter(|mark| {
                previous.get(mark.id.as_str()).is_some_and(|old| {
                    compatible(old, mark)
                        && from_blends.mode(&old.id) == target_blends.mode(&mark.id)
                })
            })
            .map(|m| m.id.clone())
            .collect();
        let mut compatible_labels: BTreeSet<_> = to
            .labels
            .iter()
            .enumerate()
            .filter(|(i, next)| {
                from.labels
                    .get(*i)
                    .is_some_and(|old| old.text == next.text && old.anchor == next.anchor)
                    && from.label_series.get(i) == to.label_series.get(i)
                    && from.label_clips.get(i) == to.label_clips.get(i)
                    && from.resolved_label_font(*i) == to.resolved_label_font(*i)
            })
            .map(|(i, _)| i)
            .collect();
        let from_order = from.draw_items();
        let to_order = to.draw_items();
        let from_keys: Vec<_> = from_order
            .iter()
            .filter_map(|item| motion_key(&from, *item, &compatible_ids, &compatible_labels))
            .collect();
        let to_keys: Vec<_> = to_order
            .iter()
            .filter_map(|item| motion_key(&to, *item, &compatible_ids, &compatible_labels))
            .collect();
        // Paint-order permutations cannot interpolate. Crossfade two complete
        // snapshots, preserving each snapshot's actual mixed geometry/text order.
        let snapshots = from_keys.is_empty() || from_keys != to_keys;
        if snapshots {
            compatible_ids.clear();
            compatible_labels.clear();
        }
        let mut compatible_from = from.clone();
        compatible_from.marks.clear();
        compatible_from.labels.clear();
        compatible_from.draw_order.clear();
        compatible_from.label_series.clear();
        compatible_from.label_clips.clear();
        compatible_from.typography.labels.clear();
        let mut compatible_to = compatible_from.clone();
        for mark in &to.marks {
            if compatible_ids.contains(&mark.id) {
                compatible_from
                    .marks
                    .push(previous[mark.id.as_str()].clone());
                compatible_to.marks.push(mark.clone());
            }
        }
        let target_ids: BTreeSet<_> = to.marks.iter().map(|m| m.id.as_str()).collect();
        let mut reserved: BTreeSet<String> = from
            .marks
            .iter()
            .chain(&to.marks)
            .map(|m| m.id.clone())
            .collect();
        let mut exits = Vec::new();
        let mut exit_blends = Vec::new();
        let mut exit_slots = BTreeMap::new();
        let mut ghost_index = 0usize;
        for (index, mark) in from.marks.iter().enumerate() {
            if !compatible_ids.contains(&mark.id) && mark.opacity > 0. {
                let mut exit = mark.clone();
                if target_ids.contains(mark.id.as_str()) {
                    loop {
                        let id = format!("kartis-motion-exit-{ghost_index}");
                        ghost_index = ghost_index
                            .checked_add(1)
                            .ok_or_else(|| ChartError("motion identity budget exceeded".into()))?;
                        if reserved.insert(id.clone()) {
                            exit.id = id;
                            break;
                        }
                    }
                }
                // Keep series visibility ownership while retiring interaction metadata.
                exit.label.clear();
                let mode = from_blends.mode(&mark.id);
                if mode != crate::BlendMode::Normal {
                    exit_blends.push((exit.id.clone(), mode));
                }
                exit_slots.insert(index, exits.len());
                exits.push(exit);
            }
        }
        let exit_labels: Vec<_> = from
            .labels
            .iter()
            .enumerate()
            .filter(|(index, old)| old.color.3 > 0 && !compatible_labels.contains(index))
            .map(|(index, _)| index)
            .collect();
        ensure(
            to.marks
                .len()
                .checked_add(exits.len())
                .is_some_and(|n| n <= MAX_MARKS),
            "motion enter/exit mark budget exceeded",
        )?;
        ensure(
            to.labels
                .len()
                .checked_add(exit_labels.len())
                .is_some_and(|n| n <= MAX_MARKS),
            "motion label crossfade budget exceeded",
        )?;
        let exit_label_slots: BTreeMap<_, _> = exit_labels
            .iter()
            .enumerate()
            .map(|(new, old)| (*old, new))
            .collect();
        let old_slot = |item: DrawItem| match item {
            DrawItem::Mark(i) => exit_slots.get(&i).copied().map(DrawItem::Mark),
            DrawItem::Label(i) => exit_label_slots.get(&i).copied().map(DrawItem::Label),
        };
        let target_slot = |item: DrawItem| match item {
            DrawItem::Mark(i) => DrawItem::Mark(exits.len() + i),
            DrawItem::Label(i) => DrawItem::Label(exit_labels.len() + i),
        };
        let mut draw_order =
            Vec::with_capacity(exits.len() + to.marks.len() + exit_labels.len() + to.labels.len());
        if snapshots {
            draw_order.extend(from_order.iter().filter_map(|item| old_slot(*item)));
            draw_order.extend(to_order.iter().map(|item| target_slot(*item)));
        } else {
            // Preserve departing items around the next surviving origin anchor.
            // When survivors keep their order this reproduces origin paint at
            // progress zero and target paint at progress one without layer jumps.
            let mut pending = Vec::new();
            let mut before = BTreeMap::<MotionKey, Vec<DrawItem>>::new();
            for item in &from_order {
                if let Some(exit) = old_slot(*item) {
                    pending.push(exit);
                } else if let Some(key) =
                    motion_key(&from, *item, &compatible_ids, &compatible_labels)
                {
                    before.insert(key, std::mem::take(&mut pending));
                }
            }
            for item in &to_order {
                if let Some(key) = motion_key(&to, *item, &compatible_ids, &compatible_labels)
                    && let Some(mut exits) = before.remove(&key)
                {
                    draw_order.append(&mut exits);
                }
                draw_order.push(target_slot(*item));
            }
            draw_order.append(&mut pending);
        }
        let paint_blends = MarkBlends::from_motion_entries(
            exit_blends
                .into_iter()
                .chain(target_blends.iter().map(|(id, mode)| (id.to_owned(), mode))),
        )?;
        let plan = Self {
            from,
            to,
            compatible_from,
            compatible_to,
            compatible_ids,
            compatible_labels,
            exits,
            exit_labels,
            draw_order,
            paint_blends,
            target_blends,
        };
        plan.frame(0.)?;
        plan.frame(0.5)?;
        Ok(plan)
    }
    fn frame(&self, t: f64) -> Result<Scene> {
        let tween = self.compatible_from.interpolate(&self.compatible_to, t)?;
        let compatible: BTreeMap<_, _> =
            tween.marks.into_iter().map(|m| (m.id.clone(), m)).collect();
        let mut out = self.to.clone();
        out.width = lerp(self.from.width, self.to.width, t);
        out.height = lerp(self.from.height, self.to.height, t);
        out.background = color(self.from.background, self.to.background, t);
        out.marks.clear();
        for exit in &self.exits {
            let mut exit = exit.clone();
            exit.opacity *= 1. - t;
            out.marks.push(exit);
        }
        for mark in &self.to.marks {
            if self.compatible_ids.contains(&mark.id) {
                out.marks.push(compatible[&mark.id].clone());
            } else {
                let mut enter = mark.clone();
                enter.opacity *= t;
                out.marks.push(enter);
            }
        }
        out.labels.clear();
        out.label_series.clear();
        out.label_clips.clear();
        out.typography.labels.clear();
        for &index in &self.exit_labels {
            let mut label = self.from.labels[index].clone();
            label.color.3 = (f64::from(label.color.3) * (1. - t)).round() as u8;
            if let Some(owner) = self.from.label_series.get(&index) {
                out.label_series.insert(out.labels.len(), owner.clone());
            }
            if let Some(clip) = self.from.label_clips.get(&index) {
                out.label_clips.insert(out.labels.len(), clip.clone());
            }
            if self.from.typography.has_label_font(index) || !self.to.typography.default.is_empty()
            {
                out.typography.labels.insert(
                    out.labels.len(),
                    crate::FontOptions {
                        family: Some(self.from.resolved_label_font(index).family),
                        weight: Some(self.from.resolved_label_font(index).weight),
                    },
                );
            }
            out.labels.push(label);
        }
        for (index, next) in self.to.labels.iter().enumerate() {
            let mut label = next.clone();
            if self.compatible_labels.contains(&index) {
                let old = &self.from.labels[index];
                label.x = lerp(old.x, next.x, t);
                label.y = lerp(old.y, next.y, t);
                label.size = lerp(old.size, next.size, t);
                label.rotation = lerp(old.rotation, next.rotation, t);
                label.color = color(old.color, next.color, t);
            } else {
                label.color.3 = (f64::from(label.color.3) * t).round() as u8;
            }
            if let Some(owner) = self.to.label_series.get(&index) {
                out.label_series.insert(out.labels.len(), owner.clone());
            }
            if let Some(clip) = self.to.label_clips.get(&index) {
                out.label_clips.insert(out.labels.len(), clip.clone());
            }
            if self.to.typography.has_label_font(index) {
                out.typography
                    .labels
                    .insert(out.labels.len(), self.to.typography.label_options(index));
            }
            out.labels.push(label);
        }
        out.draw_order = self.draw_order.clone();
        out.validate()?;
        Ok(out)
    }
}
fn compatible(a: &Mark, b: &Mark) -> bool {
    let geometry = match (&a.geometry, &b.geometry) {
        (Geometry::Rect { .. }, Geometry::Rect { .. })
        | (Geometry::Circle { .. }, Geometry::Circle { .. }) => true,
        (Geometry::Path(a), Geometry::Path(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    matches!(
                        (a, b),
                        (PathCommand::Move(_), PathCommand::Move(_))
                            | (PathCommand::Line(_), PathCommand::Line(_))
                            | (PathCommand::Cubic { .. }, PathCommand::Cubic { .. })
                            | (PathCommand::Close, PathCommand::Close)
                    )
                })
        }
        _ => false,
    };
    geometry
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
fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a * (1. - t) + b * t
}
fn color(a: Color, b: Color, t: f64) -> Color {
    let c = |a: u8, b: u8| lerp(f64::from(a), f64::from(b), t).round() as u8;
    Color(c(a.0, b.0), c(a.1, b.1), c(a.2, b.2), c(a.3, b.3))
}

/// A normalized spring progress drives all geometry and fades. Spring dynamics
/// are integrated analytically, so dt subdivision is deterministic up to normal
/// floating-point rounding. Progress is clamped for painting to prevent negative
/// sizes/opacity during physical overshoot. Undamped springs do not settle.
#[derive(Clone)]
pub struct SpringMotion {
    current: Scene,
    plan: Option<Plan>,
    config: SpringConfig,
    position: f64,
    velocity: f64,
    reduced_motion: bool,
    retarget_policy: RetargetPolicy,
    blends: MarkBlends,
}
/// Scene springs have one normalized progress velocity. Projection preserves
/// the component of prior physical geometry velocity along the new trajectory.
/// Topology/paint-order/ownership fades reset it. Painting still clamps progress;
/// use SpringValue tracks for independent physical property velocities.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RetargetPolicy {
    Reset,
    #[default]
    PreserveProjectedVelocity,
}
impl SpringMotion {
    pub fn new(initial: Scene, config: SpringConfig) -> Result<Self> {
        Self::new_with_blends(initial, config, MarkBlends::default())
    }
    /// Attach the discrete artistic modes of the initial paint identities.
    pub fn new_with_blends(
        initial: Scene,
        config: SpringConfig,
        blends: MarkBlends,
    ) -> Result<Self> {
        initial.validate()?;
        blends.validate(&initial)?;
        config.validate()?;
        Ok(Self {
            current: initial,
            plan: None,
            config,
            position: 1.,
            velocity: 0.,
            reduced_motion: false,
            retarget_policy: RetargetPolicy::default(),
            blends,
        })
    }
    pub fn scene(&self) -> &Scene {
        &self.current
    }
    /// Modes keyed by the current physical Scene, including retiring ghosts.
    pub fn blends(&self) -> &MarkBlends {
        &self.blends
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn blend_admission_frames(&self) -> Result<Option<[Scene; 2]>> {
        let Some(plan) = self
            .plan
            .as_ref()
            .filter(|plan| !plan.paint_blends.is_empty())
        else {
            return Ok(None);
        };
        let mut frames = [self.current.clone(), plan.frame(1.)?];
        for frame in &mut frames {
            // Arm presence without inventing ink for authored zero opacity.
            for (mark, origin) in frame.marks.iter_mut().zip(&plan.exits) {
                mark.opacity = origin.opacity;
            }
            for (mark, target) in frame.marks[plan.exits.len()..]
                .iter_mut()
                .zip(&plan.to.marks)
            {
                if !plan.compatible_ids.contains(&mark.id) {
                    mark.opacity = target.opacity;
                }
            }
        }
        Ok(Some(frames))
    }
    pub fn is_settled(&self) -> bool {
        self.plan.is_none()
    }
    pub fn progress(&self) -> f64 {
        self.position.clamp(0., 1.)
    }
    pub fn velocity(&self) -> f64 {
        self.velocity
    }
    pub fn retarget_policy(&self) -> RetargetPolicy {
        self.retarget_policy
    }
    pub fn set_retarget_policy(&mut self, policy: RetargetPolicy) {
        self.retarget_policy = policy;
    }
    /// Retarget from the currently displayed frame. A rejected target or
    /// transient union budget leaves the previous animation entirely intact.
    pub fn retarget(&mut self, target: Scene) -> Result<()> {
        self.retarget_with_policy(target, self.retarget_policy)
    }
    pub fn retarget_with_policy(&mut self, target: Scene, policy: RetargetPolicy) -> Result<()> {
        self.retarget_with_blends_and_policy(target, MarkBlends::default(), policy)
    }
    /// An unchanged mode preserves the compatible track. A mode change keeps
    /// the old mode on retiring paint and uses the new mode on entering paint.
    pub fn retarget_with_blends(&mut self, target: Scene, blends: MarkBlends) -> Result<()> {
        self.retarget_with_blends_and_policy(target, blends, self.retarget_policy)
    }
    pub fn retarget_with_blends_and_policy(
        &mut self,
        target: Scene,
        blends: MarkBlends,
        policy: RetargetPolicy,
    ) -> Result<()> {
        target.validate()?;
        blends.validate(&target)?;
        if self.reduced_motion || self.current == target && self.blends == blends {
            self.current = target;
            self.blends = blends;
            self.plan = None;
            self.position = 1.;
            self.velocity = 0.;
            return Ok(());
        }
        let plan = Plan::new(self.current.clone(), target, &self.blends, blends)?;
        let frame = plan.frame(0.)?;
        let velocity = if policy == RetargetPolicy::PreserveProjectedVelocity {
            self.projected_velocity(&plan)?
        } else {
            0.
        };
        self.current = frame;
        self.blends = plan.paint_blends.clone();
        self.plan = Some(plan);
        self.position = 0.;
        self.velocity = velocity;
        Ok(())
    }
    fn projected_velocity(&self, next: &Plan) -> Result<f64> {
        let Some(previous) = &self.plan else {
            return Ok(0.);
        };
        let complete = |plan: &Plan| {
            plan.compatible_ids.len() == plan.from.marks.len()
                && plan.compatible_ids.len() == plan.to.marks.len()
                && plan.compatible_labels.len() == plan.from.labels.len()
                && plan.compatible_labels.len() == plan.to.labels.len()
        };
        if self.position <= 0. || self.position >= 1. || !complete(previous) || !complete(next) {
            return Ok(0.);
        }
        let before: BTreeMap<_, _> = previous
            .from
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect();
        let after: BTreeMap<_, _> = previous
            .to
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect();
        let now: BTreeMap<_, _> = next.from.marks.iter().map(|m| (m.id.as_str(), m)).collect();
        let mut numerator = 0.;
        let mut denominator = 0.;
        let mut add = |old_delta: f64, new_delta: f64| {
            numerator += old_delta * new_delta;
            denominator += new_delta * new_delta;
        };
        for mark in &next.to.marks {
            if let (Some(a), Some(b), Some(current)) = (
                before.get(mark.id.as_str()),
                after.get(mark.id.as_str()),
                now.get(mark.id.as_str()),
            ) {
                let a = components(a);
                let b = components(b);
                let c = components(current);
                let d = components(mark);
                if a.len() == b.len() && a.len() == c.len() && a.len() == d.len() {
                    for i in 0..a.len() {
                        add(b[i] - a[i], d[i] - c[i]);
                    }
                }
            }
        }
        for i in 0..next.to.labels.len() {
            if let (Some(a), Some(b), Some(c)) = (
                previous.from.labels.get(i),
                previous.to.labels.get(i),
                next.from.labels.get(i),
            ) {
                let d = &next.to.labels[i];
                for (old, new) in [
                    (b.x - a.x, d.x - c.x),
                    (b.y - a.y, d.y - c.y),
                    (b.size - a.size, d.size - c.size),
                    (b.rotation - a.rotation, d.rotation - c.rotation),
                ] {
                    add(old, new);
                }
            }
        }
        add(
            previous.to.width - previous.from.width,
            next.to.width - next.from.width,
        );
        add(
            previous.to.height - previous.from.height,
            next.to.height - next.from.height,
        );
        let velocity = if denominator > 0. {
            numerator / denominator * self.velocity
        } else {
            0.
        };
        ensure(
            velocity.is_finite(),
            "retarget velocity projection overflow",
        )?;
        Ok(velocity)
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced {
            self.finish();
        }
    }
    pub fn finish(&mut self) {
        if let Some(plan) = self.plan.take() {
            self.current = plan.to;
            self.blends = plan.target_blends;
        }
        self.position = 1.;
        self.velocity = 0.;
    }
    /// Advance by seconds, with a bounded explicit clock step in 0..=60.
    /// Rendering and spring state commit together only after frame validation.
    pub fn tick(&mut self, dt: f64) -> Result<&Scene> {
        ensure(
            dt.is_finite() && (0. ..=60.).contains(&dt),
            "motion dt must be finite and in 0..=60 seconds",
        )?;
        let Some(plan) = &self.plan else {
            return Ok(&self.current);
        };
        let (position, velocity) = integrate(self.position, self.velocity, dt, self.config);
        ensure(
            position.is_finite() && velocity.is_finite(),
            "spring integration overflow",
        )?;
        let resting = self.config.damping > 0.
            && (position - 1.).abs() <= self.config.position_epsilon
            && velocity.abs() <= self.config.velocity_epsilon;
        let frame = if resting {
            plan.to.clone()
        } else {
            plan.frame(position.clamp(0., 1.))?
        };
        self.current = frame;
        if resting {
            self.blends = plan.target_blends.clone();
        }
        self.position = if resting { 1. } else { position };
        self.velocity = if resting { 0. } else { velocity };
        if resting {
            self.plan = None;
        }
        Ok(&self.current)
    }
}
fn components(mark: &Mark) -> Vec<f64> {
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
    if let Some(clip) = mark.clip {
        values.extend([clip.x, clip.y, clip.width, clip.height]);
    }
    values
}
fn integrate(position: f64, velocity: f64, dt: f64, config: SpringConfig) -> (f64, f64) {
    integrate_about(position, velocity, 1., dt, config)
}
fn integrate_about(
    position: f64,
    velocity: f64,
    target: f64,
    dt: f64,
    config: SpringConfig,
) -> (f64, f64) {
    let y = position - target;
    let alpha = config.damping / (2. * config.mass);
    let omega2 = config.stiffness / config.mass;
    let difference = alpha * alpha - omega2;
    let (y, v) = if difference.abs() <= omega2 * 1e-10 {
        let decay = (-alpha * dt).exp();
        let c = velocity + alpha * y;
        (decay * (y + c * dt), decay * (velocity - alpha * c * dt))
    } else if difference < 0. {
        let omega = (-difference).sqrt();
        let decay = (-alpha * dt).exp();
        let (sin, cos) = (omega * dt).sin_cos();
        (
            decay * (y * cos + (velocity + alpha * y) / omega * sin),
            decay * (velocity * cos - (alpha * velocity + omega2 * y) / omega * sin),
        )
    } else {
        let root = difference.sqrt();
        let r1 = -omega2 / (alpha + root);
        let r2 = -alpha - root;
        let a = (velocity - r2 * y) / (r1 - r2);
        let b = y - a;
        let first = a * (r1 * dt).exp();
        let second = b * (r2 * dt).exp();
        (first + second, r1 * first + r2 * second)
    };
    (y + target, v)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScalarVelocityPolicy {
    Reset,
    #[default]
    Preserve,
}
/// An independent physical-units scalar spring. Compose one track per property
/// when preserving a full velocity vector across unrelated target changes is
/// required. No normalized projection or opacity/geometry clipping occurs here;
/// callers validate composed scene constraints before publishing.
#[derive(Clone, Debug)]
pub struct SpringValue {
    value: f64,
    target: f64,
    velocity: f64,
    config: SpringConfig,
    reduced_motion: bool,
    settled: bool,
}
impl SpringValue {
    pub fn new(value: f64, config: SpringConfig) -> Result<Self> {
        config.validate()?;
        ensure(value.is_finite(), "spring value must be finite")?;
        Ok(Self {
            value,
            target: value,
            velocity: 0.,
            config,
            reduced_motion: false,
            settled: true,
        })
    }
    pub fn value(&self) -> f64 {
        self.value
    }
    pub fn target(&self) -> f64 {
        self.target
    }
    pub fn velocity(&self) -> f64 {
        self.velocity
    }
    pub fn is_settled(&self) -> bool {
        self.settled
    }
    pub fn retarget(&mut self, target: f64) -> Result<()> {
        self.retarget_with_policy(target, ScalarVelocityPolicy::Preserve)
    }
    pub fn retarget_with_policy(
        &mut self,
        target: f64,
        policy: ScalarVelocityPolicy,
    ) -> Result<()> {
        ensure(
            target.is_finite() && (self.value - target).is_finite(),
            "spring target or displacement overflow",
        )?;
        let velocity = if policy == ScalarVelocityPolicy::Preserve {
            self.velocity
        } else {
            0.
        };
        self.target = target;
        self.velocity = velocity;
        self.settled = self.value == target && velocity == 0.;
        if self.reduced_motion {
            self.finish();
        }
        Ok(())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        if reduced {
            self.finish();
        }
    }
    pub fn finish(&mut self) {
        self.value = self.target;
        self.velocity = 0.;
        self.settled = true;
    }
    pub fn tick(&mut self, dt: f64) -> Result<f64> {
        ensure(
            dt.is_finite() && (0. ..=60.).contains(&dt),
            "spring dt must be finite and in 0..=60 seconds",
        )?;
        if self.settled || dt == 0. {
            return Ok(self.value);
        }
        let (value, velocity) =
            integrate_about(self.value, self.velocity, self.target, dt, self.config);
        ensure(
            value.is_finite() && velocity.is_finite() && (value - self.target).is_finite(),
            "scalar spring integration overflow",
        )?;
        let resting = self.config.damping > 0.
            && (value - self.target).abs() <= self.config.position_epsilon
            && velocity.abs() <= self.config.velocity_epsilon;
        self.value = if resting { self.target } else { value };
        self.velocity = if resting { 0. } else { velocity };
        self.settled = resting;
        Ok(self.value)
    }
}
