use super::geometry::{GeometryTrack, Shape};
use super::*;

#[derive(Clone, Debug)]
pub(super) enum Content {
    Mark {
        index: Option<usize>,
        part: Option<(usize, FunnelMotionPartRole)>,
        geometry: Geometry,
        scalar_path: bool,
        shape: Option<Shape>,
        colors: bool,
        fill: Option<Color>,
        stroke: Option<(Color, f64)>,
    },
    Label {
        index: usize,
        x: f64,
        y: f64,
        color: Option<Color>,
    },
}
pub(super) struct Input {
    pub key: String,
    pub content: Content,
}

fn color_values(color: Color) -> [f64; 4] {
    [
        color.0.into(),
        color.1.into(),
        color.2.into(),
        color.3.into(),
    ]
}
fn mark_values(c: &Content) -> Vec<(FunnelMotionMarkChannel, f64)> {
    use FunnelMotionMarkChannel::*;
    let mut out = vec![];
    if let Content::Mark {
        colors: true,
        fill,
        stroke,
        ..
    } = c
    {
        if let Some(color) = fill {
            out.extend(
                [FillRed, FillGreen, FillBlue, FillAlpha]
                    .into_iter()
                    .zip(color_values(*color)),
            );
        }
        if let Some((color, width)) = stroke {
            out.extend(
                [StrokeRed, StrokeGreen, StrokeBlue, StrokeAlpha]
                    .into_iter()
                    .zip(color_values(*color)),
            );
            out.push((StrokeWidth, *width));
        }
    }
    out
}
fn label_values(c: &Content) -> Vec<(FunnelMotionLabelChannel, f64)> {
    use FunnelMotionLabelChannel::*;
    let mut out = vec![];
    if let Content::Label { x, y, color, .. } = c {
        out.extend([(X, *x), (Y, *y)]);
        if let Some(c) = color {
            out.extend([Red, Green, Blue, Alpha].into_iter().zip(color_values(*c)));
        }
    }
    out
}

#[derive(Clone, Debug)]
pub(super) struct Track {
    content: Content,
    geometry: Option<GeometryTrack>,
    marks: BTreeMap<FunnelMotionMarkChannel, SpringValue>,
    labels: BTreeMap<FunnelMotionLabelChannel, SpringValue>,
}
impl Track {
    pub fn new(input: Input, config: SpringConfig, reduced: bool) -> Result<Self> {
        let geometry = if let Content::Mark {
            geometry,
            scalar_path,
            shape,
            ..
        } = &input.content
        {
            Some(GeometryTrack::new(
                geometry.clone(),
                config,
                *scalar_path,
                reduced,
                shape.clone(),
            )?)
        } else {
            None
        };
        let mut out = Self {
            geometry,
            marks: BTreeMap::new(),
            labels: BTreeMap::new(),
            content: input.content,
        };
        for (k, v) in mark_values(&out.content) {
            out.marks.insert(k, SpringValue::new(v, config)?);
        }
        for (k, v) in label_values(&out.content) {
            out.labels.insert(k, SpringValue::new(v, config)?);
        }
        out.set_reduced(reduced);
        Ok(out)
    }
    pub fn retarget(&mut self, input: Input, config: SpringConfig, reduced: bool) -> Result<()> {
        if let Content::Mark {
            geometry,
            scalar_path,
            shape,
            ..
        } = &input.content
        {
            if let Some(track) = &mut self.geometry {
                track.retarget(
                    geometry.clone(),
                    config,
                    *scalar_path,
                    reduced,
                    shape.clone(),
                )?;
            } else {
                self.geometry = Some(GeometryTrack::new(
                    geometry.clone(),
                    config,
                    *scalar_path,
                    reduced,
                    shape.clone(),
                )?);
            }
        }
        let mut marks = BTreeMap::new();
        for (k, v) in mark_values(&input.content) {
            let mut spring = if let Some(s) = self.marks.remove(&k) {
                s
            } else {
                SpringValue::new(v, config)?
            };
            spring.retarget(v)?;
            spring.set_reduced_motion(reduced);
            marks.insert(k, spring);
        }
        let mut labels = BTreeMap::new();
        for (k, v) in label_values(&input.content) {
            let mut spring = if let Some(s) = self.labels.remove(&k) {
                s
            } else {
                SpringValue::new(v, config)?
            };
            spring.retarget(v)?;
            spring.set_reduced_motion(reduced);
            labels.insert(k, spring);
        }
        self.marks = marks;
        self.labels = labels;
        self.content = input.content;
        Ok(())
    }
    pub fn settled(&self) -> bool {
        self.geometry.as_ref().is_none_or(GeometryTrack::settled)
            && self
                .marks
                .values()
                .chain(self.labels.values())
                .all(SpringValue::is_settled)
    }
    pub fn tick(&mut self, dt: f64) -> Result<()> {
        if let Some(g) = &mut self.geometry {
            g.tick(dt)?;
        }
        for s in self.marks.values_mut().chain(self.labels.values_mut()) {
            s.tick(dt)?;
        }
        Ok(())
    }
    pub fn set_config(&mut self, c: SpringConfig) -> Result<()> {
        if let Some(g) = &mut self.geometry {
            g.set_config(c)?;
        }
        for s in self.marks.values_mut().chain(self.labels.values_mut()) {
            s.set_config(c)?;
        }
        Ok(())
    }
    pub fn set_reduced(&mut self, r: bool) {
        if let Some(g) = &mut self.geometry {
            g.set_reduced(r);
        }
        for s in self.marks.values_mut().chain(self.labels.values_mut()) {
            s.set_reduced_motion(r);
        }
    }
    pub fn finish(&mut self) {
        if let Some(g) = &mut self.geometry {
            g.finish();
        }
        for s in self.marks.values_mut().chain(self.labels.values_mut()) {
            s.finish();
        }
    }
    pub fn paint(&self, out: &mut Snapshot) -> Result<()> {
        match &self.content {
            Content::Mark { index, part, .. } => {
                if let Some(part) = part {
                    for (&key, s) in &self.marks {
                        out.part_states.insert((part.0, part.1, key), state(s));
                    }
                    if let Some(g) = &self.geometry {
                        let geometry = g.scene()?;
                        super::geometry::validate(&geometry)?;
                        out.part_geometries.insert(*part, g.states());
                        if let Some(p) = g.progress() {
                            out.part_paths.insert(*part, p);
                        }
                    }
                }
                let Some(index) = index else {
                    return Ok(());
                };
                let mark = &mut out.scene.marks[*index];
                if let Some(g) = &self.geometry {
                    mark.geometry = g.scene()?;
                    out.geometries.insert(mark.id.clone(), g.states());
                    if let Some(p) = g.progress() {
                        out.paths.insert(mark.id.clone(), p);
                    }
                }
                let channel = |c| self.marks.get(&c).map(|v| v.value());
                use FunnelMotionMarkChannel::*;
                let c = |keys: [FunnelMotionMarkChannel; 4]| -> Option<Color> {
                    let [r, g, b, a] = keys.map(channel);
                    Some(Color(byte(r?), byte(g?), byte(b?), byte(a?)))
                };
                if mark.fill.is_some()
                    && let Some(color) = c([FillRed, FillGreen, FillBlue, FillAlpha])
                {
                    mark.fill = Some(color);
                }
                if channel(StrokeWidth).is_some_and(|width| width <= 0.) {
                    mark.stroke = None;
                } else if let Some(stroke) = &mut mark.stroke {
                    if let Some(color) = c([StrokeRed, StrokeGreen, StrokeBlue, StrokeAlpha]) {
                        stroke.color = color;
                    }
                    if let Some(width) = channel(StrokeWidth) {
                        stroke.width = width.clamp(0.000001, 10000.);
                    }
                }
                for (&key, s) in &self.marks {
                    out.mark_states.insert((mark.id.clone(), key), state(s));
                }
            }
            Content::Label { index, .. } => {
                let label = &mut out.scene.labels[*index];
                use FunnelMotionLabelChannel::*;
                label.x = self.labels[&X].value();
                label.y = self.labels[&Y].value();
                if let (Some(r), Some(g), Some(b), Some(a)) = (
                    self.labels.get(&Red),
                    self.labels.get(&Green),
                    self.labels.get(&Blue),
                    self.labels.get(&Alpha),
                ) {
                    label.color = Color(
                        byte(r.value()),
                        byte(g.value()),
                        byte(b.value()),
                        byte(a.value()),
                    );
                }
                for (&key, s) in &self.labels {
                    out.label_states.insert((*index, key), state(s));
                }
            }
        }
        Ok(())
    }
}
fn byte(v: f64) -> u8 {
    v.round().clamp(0., 255.) as u8
}

pub(super) fn inputs(target: &FunnelSvgFrame, options: &FunnelMotionOptions) -> Result<Vec<Input>> {
    target.scene().validate()?;
    ensure(
        target.scene().marks.len() <= options.max_marks
            && target.scene().labels.len() <= options.max_labels,
        "Funnel motion role budget exceeded",
    )?;
    let mut numbers = 0usize;
    for mark in &target.scene().marks {
        numbers = numbers
            .checked_add(super::geometry::numbers(&mark.geometry).len())
            .ok_or_else(|| ChartError("Funnel motion geometry budget overflow".into()))?;
        ensure(
            numbers <= options.max_path_numbers,
            "Funnel motion geometry budget exceeded",
        )?;
    }
    let mut out = vec![];
    let mut body_slots = vec![None; target.source().len()];
    let mut border_slots = vec![None; target.source().len()];
    for (index, mark) in target.scene().marks.iter().enumerate() {
        let key = match target.mark_role(index)? {
            Some(FunnelSvgMarkRole::Body { part_index }) => {
                body_slots[part_index] = Some(index);
                continue;
            }
            Some(FunnelSvgMarkRole::Border { part_index }) => {
                border_slots[part_index] = Some(index);
                continue;
            }
            Some(FunnelSvgMarkRole::Separator { separator_index }) => {
                let s = &target.separators()[separator_index];
                format!(
                    "separator:{:?}:{:?}",
                    s.side,
                    s.part_id.as_ref().map(CategoryKey::identity).transpose()?
                )
            }
            None => continue,
        };
        out.push(Input {
            key,
            content: Content::Mark {
                index: Some(index),
                part: None,
                geometry: mark.geometry.clone(),
                scalar_path: false,
                shape: None,
                colors: false,
                fill: mark.fill,
                stroke: mark.stroke.map(|s| (s.color, s.width)),
            },
        });
    }
    let mut roles = target.scene().marks.len();
    for index in 0..target.source().len() {
        let capture = target.motion_capture(index)?;
        for (role, slot, latent) in [
            (
                FunnelMotionPartRole::Body,
                body_slots[index],
                capture.body.as_ref(),
            ),
            (
                FunnelMotionPartRole::Border,
                border_slots[index],
                Some(&capture.border),
            ),
        ] {
            if role == FunnelMotionPartRole::Body && slot.is_none() {
                ensure(
                    capture.origin != FunnelMotionGeometryOrigin::UncalledStageFactory,
                    "Funnel motion needs an all-source stage capture for an inactive custom body",
                )?;
            }
            let latent = latent.ok_or_else(|| {
                ChartError("Funnel motion source geometry descriptor is missing".into())
            })?;
            let (geometry, fill, stroke) = if let Some(slot) = slot {
                let mark = &target.scene().marks[slot];
                // Some actual edited paint is authoritative. Absent paint may
                // retain a numeric target only when the constructor explicitly
                // supplied one; this fallback never changes the paint mask.
                (
                    mark.geometry.clone(),
                    mark.fill.or(latent.fill),
                    mark.stroke.or(latent.stroke),
                )
            } else {
                roles = roles.checked_add(1).ok_or_else(|| {
                    ChartError("Funnel motion dormant role budget overflow".into())
                })?;
                numbers = numbers
                    .checked_add(super::geometry::numbers(&latent.geometry).len())
                    .ok_or_else(|| {
                        ChartError("Funnel motion dormant geometry budget overflow".into())
                    })?;
                (latent.geometry.clone(), latent.fill, latent.stroke)
            };
            ensure(
                roles <= options.max_marks,
                "Funnel motion live plus dormant role budget exceeded",
            )?;
            ensure(
                numbers <= options.max_path_numbers,
                "Funnel motion live plus dormant geometry budget exceeded",
            )?;
            super::geometry::validate(&geometry)?;
            out.push(Input {
                key: format!(
                    "{}:{}",
                    if role == FunnelMotionPartRole::Body {
                        "body"
                    } else {
                        "border"
                    },
                    target.source()[index].id.identity()?
                ),
                content: Content::Mark {
                    index: slot,
                    part: Some((index, role)),
                    geometry,
                    scalar_path: true,
                    shape: None,
                    colors: true,
                    fill,
                    stroke: stroke.map(|s| (s.color, s.width)),
                },
            });
        }
    }
    for (index, label) in target.scene().labels.iter().enumerate() {
        if let Some(part) = target.label_part_index(index)? {
            out.push(Input {
                key: format!("label:{}", target.source()[part].id.identity()?),
                content: Content::Label {
                    index,
                    x: label.x,
                    y: label.y,
                    color: Some(label.color),
                },
            });
        }
    }
    annotation_inputs(target, &mut out)?;
    let mut keys = BTreeSet::new();
    for i in &out {
        ensure(
            keys.insert(i.key.clone()),
            "Funnel motion duplicate role key",
        )?;
    }
    Ok(out)
}

fn annotation_inputs(target: &FunnelSvgFrame, out: &mut Vec<Input>) -> Result<()> {
    let Some(annotations) = target.annotations() else {
        return Ok(());
    };
    let scene = annotations.scene();
    let mark_base = target.annotation_source().scene().marks.len();
    let label_base = target.annotation_source().scene().labels.len();
    let order = scene.draw_items();
    let mut cursor = 0;
    for (binding_index, binding) in annotations.bindings().iter().enumerate() {
        let theme = annotations.theme();
        let mut roles = vec![];
        if theme.link.outline_width > 0. {
            roles.push(("link-halo", true));
        }
        if theme.outline.outline_width > 0. {
            roles.push(("shape-halo", false));
        }
        roles.extend([("shape", false), ("link", true)]);
        for (role, scalar_path) in roles {
            let Some(DrawItem::Mark(index)) = order.get(cursor).copied() else {
                return Err(ChartError(
                    "Funnel motion annotation role order changed".into(),
                ));
            };
            cursor += 1;
            let mark = &scene.marks[index];
            out.push(Input {
                key: format!("annotation:{binding_index}:{role}"),
                content: Content::Mark {
                    index: Some(mark_base + index),
                    part: None,
                    geometry: mark.geometry.clone(),
                    scalar_path,
                    shape: if scalar_path {
                        None
                    } else {
                        Some(match binding.kind {
                            FunnelAnnotationKind::Circle | FunnelAnnotationKind::Dot => {
                                Shape::Circle([
                                    binding.center[0],
                                    binding.center[1],
                                    binding.size / 2.,
                                ])
                            }
                            FunnelAnnotationKind::Rect => Shape::Rect {
                                values: [
                                    binding.center[0] - binding.width / 2.,
                                    binding.center[1] - binding.height / 2.,
                                    binding.width,
                                    binding.height,
                                ],
                                radius: binding.border_radius,
                            },
                        })
                    },
                    colors: false,
                    fill: mark.fill,
                    stroke: mark.stroke.map(|s| (s.color, s.width)),
                },
            });
        }
        if matches!(binding.note, FunnelAnnotationNote::Text(_)) {
            let count = 1 + usize::from(theme.text.outline_width > 0.);
            for note_index in 0..count {
                let Some(DrawItem::Label(index)) = order.get(cursor).copied() else {
                    return Err(ChartError(
                        "Funnel motion annotation text order changed".into(),
                    ));
                };
                cursor += 1;
                let label = &scene.labels[index];
                out.push(Input {
                    key: format!("annotation:{binding_index}:note:{note_index}"),
                    content: Content::Label {
                        index: label_base + index,
                        x: label.x,
                        y: label.y,
                        color: None,
                    },
                });
            }
        } else {
            // A custom note is a captured target Scene, not an animated native
            // note component. Skip exactly its authored mixed draw-order span.
            while let Some(item) = order.get(cursor) {
                if let DrawItem::Mark(i) = item
                    && !scene.marks[*i].id.contains(":note:")
                {
                    break;
                }
                cursor += 1;
            }
        }
    }
    ensure(
        cursor == order.len(),
        "Funnel motion annotation item mapping changed",
    )
}
