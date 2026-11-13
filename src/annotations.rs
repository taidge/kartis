//! Checked scene overlays and renderer-independent zoom/brush state.
//!
//! Applications convert browser pointer coordinates with
//! `interaction::pointer_coordinates`, update their transform or brush, and
//! publish the resulting scene through a Cage. No global listeners are installed.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand};
use glory::style::Color;

use crate::scene::*;

pub const MAX_ANNOTATIONS: usize = 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AnnotationLayer {
    /// Prepend to an explicit paint sequence, or to each default scene layer.
    Back,
    /// Append to an explicit paint sequence, or to each default scene layer.
    #[default]
    Front,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnnotationKind {
    Line {
        from: (f64, f64),
        to: (f64, f64),
        stroke: GraphicStroke,
    },
    Rect {
        x: f64,
        y: f64,
        width: f64,
        height: f64,
        fill: Option<Color>,
        stroke: Option<GraphicStroke>,
    },
    Circle {
        x: f64,
        y: f64,
        radius: f64,
        fill: Option<Color>,
        stroke: Option<GraphicStroke>,
    },
    Text(Label),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Annotation {
    /// Unique across both data marks and every annotation in the scene.
    pub id: String,
    /// Accessible metadata. A visible caption is an explicit Label.
    pub label: String,
    pub kind: AnnotationKind,
    pub caption: Option<Label>,
}
impl Annotation {
    pub fn new(id: impl Into<String>, kind: AnnotationKind) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            kind,
            caption: None,
        }
    }
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }
    pub fn caption(mut self, caption: Label) -> Self {
        self.caption = Some(caption);
        self
    }
}

/// Publish overlays only after the entire proposed scene validates. Rejections
/// leave every field and all drawing order unchanged. Text annotations reserve
/// their stable ID using a nonpainting mark. With an explicit paint sequence,
/// each annotation's geometry/text/caption stays together at the front/back.
/// An empty sequence preserves the default geometry-then-text layer contract.
pub fn annotate(
    scene: &mut Scene,
    annotations: &[Annotation],
    layer: AnnotationLayer,
) -> Result<()> {
    scene.validate()?;
    ensure(
        annotations.len() <= MAX_ANNOTATIONS,
        "annotation budget exceeded",
    )?;
    let added_labels = annotations
        .iter()
        .map(|a| {
            usize::from(matches!(a.kind, AnnotationKind::Text(_)))
                + usize::from(a.caption.is_some())
        })
        .sum::<usize>();
    ensure(
        scene
            .marks
            .len()
            .checked_add(annotations.len())
            .is_some_and(|n| n <= MAX_MARKS)
            && scene
                .labels
                .len()
                .checked_add(added_labels)
                .is_some_and(|n| n <= MAX_MARKS),
        "annotated scene budget exceeded",
    )?;
    let mut ids: BTreeSet<&str> = scene.marks.iter().map(|m| m.id.as_str()).collect();
    for annotation in annotations {
        ensure(
            !annotation.id.is_empty()
                && annotation.id.len() <= MAX_LABEL_BYTES
                && annotation.label.len() <= MAX_LABEL_BYTES
                && ids.insert(&annotation.id),
            "invalid or duplicate annotation identity",
        )?;
    }
    let mut marks = Vec::with_capacity(annotations.len());
    let mut labels = Vec::with_capacity(added_labels);
    let mut added_order = Vec::with_capacity(annotations.len() + added_labels);
    for annotation in annotations {
        added_order.push(DrawItem::Mark(marks.len()));
        let (geometry, fill, stroke) = match &annotation.kind {
            AnnotationKind::Line { from, to, stroke } => (
                Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(from.0, from.1)),
                    PathCommand::Line(GraphicPoint::new(to.0, to.1)),
                ]),
                None,
                Some(*stroke),
            ),
            AnnotationKind::Rect {
                x,
                y,
                width,
                height,
                fill,
                stroke,
            } => (
                Geometry::Rect {
                    x: *x,
                    y: *y,
                    width: *width,
                    height: *height,
                },
                *fill,
                *stroke,
            ),
            AnnotationKind::Circle {
                x,
                y,
                radius,
                fill,
                stroke,
            } => (
                Geometry::Circle {
                    x: *x,
                    y: *y,
                    radius: *radius,
                },
                *fill,
                *stroke,
            ),
            AnnotationKind::Text(label) => {
                added_order.push(DrawItem::Label(labels.len()));
                labels.push(label.clone());
                (Geometry::Path(Vec::new()), None, None)
            }
        };
        marks.push(Mark {
            clip: None,
            id: annotation.id.clone(),
            series: String::new(),
            label: annotation.label.clone(),
            value: 0.,
            geometry,
            fill,
            fill_paint: None,
            stroke,
            stroke_dash: None,
            clip_path: None,
            opacity: 1.,
        });
        if let Some(caption) = &annotation.caption {
            added_order.push(DrawItem::Label(labels.len()));
            labels.push(caption.clone());
        }
    }
    let mut next = scene.clone();
    match layer {
        AnnotationLayer::Front => {
            if !next.draw_order.is_empty() {
                next.draw_order
                    .extend(added_order.into_iter().map(|item| match item {
                        DrawItem::Mark(index) => DrawItem::Mark(index + next.marks.len()),
                        DrawItem::Label(index) => DrawItem::Label(index + next.labels.len()),
                    }));
            }
            next.marks.extend(marks);
            next.labels.extend(labels);
        }
        AnnotationLayer::Back => {
            next.typography.remap_labels(
                &(0..next.labels.len())
                    .map(|index| (index, index + labels.len()))
                    .collect(),
            );
            next.label_clips = next
                .label_clips
                .into_iter()
                .map(|(index, clip)| (index + labels.len(), clip))
                .collect();
            if !next.draw_order.is_empty() {
                added_order.extend(next.draw_order.iter().map(|item| match *item {
                    DrawItem::Mark(index) => DrawItem::Mark(index + marks.len()),
                    DrawItem::Label(index) => DrawItem::Label(index + labels.len()),
                }));
                next.draw_order = added_order;
            }
            next.label_series = next
                .label_series
                .into_iter()
                .map(|(index, series)| (index + labels.len(), series))
                .collect();
            marks.append(&mut next.marks);
            next.marks = marks;
            labels.append(&mut next.labels);
            next.labels = labels;
        }
    }
    next.validate()?;
    *scene = next;
    Ok(())
}

/// A normalized, inclusive rectangle in scene coordinates. Zero-area brushes
/// are valid and can select centers lying exactly on their segment or point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Brush {
    pub x0: f64,
    pub y0: f64,
    pub x1: f64,
    pub y1: f64,
}
impl Brush {
    pub fn from_points(a: (f64, f64), b: (f64, f64)) -> Result<Self> {
        ensure(
            [a.0, a.1, b.0, b.1]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "brush coordinates must be finite and within +/-1000000",
        )?;
        Ok(Self {
            x0: a.0.min(b.0),
            y0: a.1.min(b.1),
            x1: a.0.max(b.0),
            y1: a.1.max(b.1),
        })
    }
    pub fn validate(&self) -> Result<()> {
        ensure(
            [self.x0, self.y0, self.x1, self.y1]
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                && self.x0 <= self.x1
                && self.y0 <= self.y1,
            "invalid normalized brush",
        )
    }
    /// Return None for a wholly disjoint drag, rather than projecting it onto
    /// an unrelated viewport edge.
    pub fn clipped(&self, width: f64, height: f64) -> Result<Option<Self>> {
        self.validate()?;
        ensure(
            [width, height]
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 1_000_000.),
            "invalid brush viewport",
        )?;
        if self.x1 < 0. || self.y1 < 0. || self.x0 > width || self.y0 > height {
            return Ok(None);
        }
        Ok(Some(Self {
            x0: self.x0.max(0.),
            y0: self.y0.max(0.),
            x1: self.x1.min(width),
            y1: self.y1.min(height),
        }))
    }
    pub fn contains(&self, point: (f64, f64)) -> bool {
        self.validate().is_ok()
            && point.0.is_finite()
            && point.1.is_finite()
            && point.0 >= self.x0
            && point.0 <= self.x1
            && point.1 >= self.y0
            && point.1 <= self.y1
    }
    /// Data marks have a nonempty series and label, visible paint and opacity.
    /// Selection follows scene order and uses exact rectangle/circle centers or
    /// the tight geometric bounding-box center of a cubic path.
    pub fn select(&self, scene: &Scene) -> Result<Vec<String>> {
        self.select_transformed(scene, &ZoomTransform::default())
    }
    pub fn select_transformed(
        &self,
        scene: &Scene,
        transform: &ZoomTransform,
    ) -> Result<Vec<String>> {
        self.validate()?;
        scene.validate()?;
        transform.validate()?;
        let mut ids = Vec::new();
        for mark in &scene.marks {
            if mark.series.is_empty()
                || mark.label.is_empty()
                || mark.opacity <= 0.
                || (!mark.has_fill() && mark.stroke.is_none())
            {
                continue;
            }
            if let Some(center) = geometry_center(&mark.geometry)
                && mark
                    .clip
                    .is_none_or(|clip| clip.contains(center.0, center.1))
                && mark
                    .clip_path
                    .as_ref()
                    .is_none_or(|clip| clip.contains(center.0, center.1))
                && self.contains(transform.project(center)?)
            {
                ids.push(mark.id.clone());
            }
        }
        Ok(ids)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomTransform {
    pub scale: f64,
    pub translate_x: f64,
    pub translate_y: f64,
}
impl Default for ZoomTransform {
    fn default() -> Self {
        Self {
            scale: 1.,
            translate_x: 0.,
            translate_y: 0.,
        }
    }
}
impl ZoomTransform {
    pub fn new(scale: f64, translate_x: f64, translate_y: f64) -> Result<Self> {
        let value = Self {
            scale,
            translate_x,
            translate_y,
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.scale.is_finite()
                && (0.000001..=1_000_000.).contains(&self.scale)
                && self.translate_x.is_finite()
                && self.translate_y.is_finite(),
            "invalid zoom transform",
        )
    }
    pub fn project(&self, point: (f64, f64)) -> Result<(f64, f64)> {
        self.validate()?;
        finite_point(point)?;
        let point = (
            point.0 * self.scale + self.translate_x,
            point.1 * self.scale + self.translate_y,
        );
        finite_point(point)?;
        Ok(point)
    }
    pub fn unproject(&self, point: (f64, f64)) -> Result<(f64, f64)> {
        self.validate()?;
        finite_point(point)?;
        let point = (
            (point.0 - self.translate_x) / self.scale,
            (point.1 - self.translate_y) / self.scale,
        );
        finite_point(point)?;
        Ok(point)
    }
    pub fn pan(&self, delta: (f64, f64)) -> Result<Self> {
        self.validate()?;
        finite_point(delta)?;
        Self::new(
            self.scale,
            self.translate_x + delta.0,
            self.translate_y + delta.1,
        )
    }
    /// Preserve the world coordinate under the pointer, subject to the global
    /// finite scale range. Use zoom_constrained for viewport/content limits.
    pub fn zoom_at(&self, factor: f64, pointer: (f64, f64)) -> Result<Self> {
        self.validate()?;
        ensure(
            factor.is_finite() && factor > 0.,
            "zoom factor must be finite and positive",
        )?;
        let world = self.unproject(pointer)?;
        let scale = self.scale * factor;
        Self::new(
            scale,
            pointer.0 - world.0 * scale,
            pointer.1 - world.1 * scale,
        )
    }
    pub fn pan_constrained(&self, delta: (f64, f64), limits: &ZoomConstraints) -> Result<Self> {
        limits.constrain(&limits.constrain(self)?.pan(delta)?)
    }
    pub fn zoom_constrained(
        &self,
        factor: f64,
        pointer: (f64, f64),
        limits: &ZoomConstraints,
    ) -> Result<Self> {
        limits.validate()?;
        ensure(
            factor.is_finite() && factor > 0.,
            "zoom factor must be finite and positive",
        )?;
        let current = limits.constrain(self)?;
        let scale = if factor >= limits.max_scale / current.scale {
            limits.max_scale
        } else if factor <= limits.min_scale / current.scale {
            limits.min_scale
        } else {
            current.scale * factor
        };
        let world = current.unproject(pointer)?;
        limits.constrain(&Self::new(
            scale,
            pointer.0 - world.0 * scale,
            pointer.1 - world.1 * scale,
        )?)
    }
}
fn finite_point(point: (f64, f64)) -> Result<()> {
    ensure(
        point.0.is_finite() && point.1.is_finite(),
        "nonfinite coordinate or transform overflow",
    )
}

/// The scaled world must cover the viewport where possible. If the world is
/// narrower than an axis of the viewport, that axis is centered.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ZoomConstraints {
    pub min_scale: f64,
    pub max_scale: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub world: Brush,
}
impl ZoomConstraints {
    pub fn validate(&self) -> Result<()> {
        self.world.validate()?;
        ensure(
            [self.min_scale, self.max_scale]
                .iter()
                .all(|v| v.is_finite() && (0.000001..=1_000_000.).contains(v))
                && self.min_scale <= self.max_scale,
            "invalid zoom scale extent",
        )?;
        ensure(
            [self.viewport_width, self.viewport_height]
                .iter()
                .all(|v| v.is_finite() && *v > 0. && *v <= 1_000_000.),
            "invalid zoom viewport",
        )
    }
    pub fn constrain(&self, transform: &ZoomTransform) -> Result<ZoomTransform> {
        self.validate()?;
        transform.validate()?;
        let scale = transform.scale.clamp(self.min_scale, self.max_scale);
        let axis = |value: f64, low: f64, high: f64, viewport: f64| {
            if (high - low) * scale < viewport {
                viewport / 2. - (low / 2. + high / 2.) * scale
            } else {
                value.clamp(viewport - high * scale, -low * scale)
            }
        };
        ZoomTransform::new(
            scale,
            axis(
                transform.translate_x,
                self.world.x0,
                self.world.x1,
                self.viewport_width,
            ),
            axis(
                transform.translate_y,
                self.world.y0,
                self.world.y1,
                self.viewport_height,
            ),
        )
    }
}

fn geometry_center(geometry: &Geometry) -> Option<(f64, f64)> {
    match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some((x + width / 2., y + height / 2.)),
        Geometry::Circle { x, y, .. } => Some((*x, *y)),
        Geometry::Path(commands) => {
            let mut bounds: Option<(f64, f64, f64, f64)> = None;
            let mut include = |p: GraphicPoint| match &mut bounds {
                Some((x0, y0, x1, y1)) => {
                    *x0 = x0.min(p.x);
                    *y0 = y0.min(p.y);
                    *x1 = x1.max(p.x);
                    *y1 = y1.max(p.y);
                }
                None => bounds = Some((p.x, p.y, p.x, p.y)),
            };
            let mut current = GraphicPoint::new(0., 0.);
            let mut start = current;
            for command in commands {
                match *command {
                    PathCommand::Move(point) => {
                        include(point);
                        current = point;
                        start = point;
                    }
                    PathCommand::Line(point) => {
                        include(point);
                        current = point;
                    }
                    PathCommand::Close => {
                        include(start);
                        current = start;
                    }
                    PathCommand::Cubic { first, second, end } => {
                        include(end);
                        for axis in [true, false] {
                            let get = |p: GraphicPoint| if axis { p.x } else { p.y };
                            let p0 = get(current);
                            let p1 = get(first);
                            let p2 = get(second);
                            let p3 = get(end);
                            let a = -p0 + 3. * p1 - 3. * p2 + p3;
                            let b = 2. * (p0 - 2. * p1 + p2);
                            let c = p1 - p0;
                            let mut roots = [f64::NAN; 2];
                            if a.abs() < 1e-12 * (b.abs() + c.abs() + 1.) {
                                if b != 0. {
                                    roots[0] = -c / b;
                                }
                            } else {
                                let d = b * b - 4. * a * c;
                                if d >= 0. {
                                    let q = -0.5 * (b + d.sqrt().copysign(b));
                                    if q != 0. {
                                        roots = [q / a, c / q];
                                    } else {
                                        roots[0] = -b / (2. * a);
                                    }
                                }
                            }
                            for t in roots {
                                if t > 0. && t < 1. {
                                    let u = 1. - t;
                                    include(GraphicPoint::new(
                                        u * u * u * current.x
                                            + 3. * u * u * t * first.x
                                            + 3. * u * t * t * second.x
                                            + t * t * t * end.x,
                                        u * u * u * current.y
                                            + 3. * u * u * t * first.y
                                            + 3. * u * t * t * second.y
                                            + t * t * t * end.y,
                                    ));
                                }
                            }
                        }
                        current = end;
                    }
                }
            }
            bounds.map(|(x0, y0, x1, y1)| ((x0 + x1) / 2., (y0 + y1) / 2.))
        }
    }
}
