use std::collections::BTreeMap;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use super::*;
struct Painter {
    scene: Scene,
    budget: FunnelAnnotationBudget,
    commands: usize,
    prefix: String,
}
impl Painter {
    fn mark(
        &mut self,
        geometry: Geometry,
        style: FunnelAnnotationPaint,
        halo: bool,
        dot: bool,
    ) -> Result<()> {
        let width = if halo {
            (if dot { 0. } else { style.stroke_width }) + 2. * style.outline_width
        } else {
            style.stroke_width
        };
        let color = if halo {
            Some(style.outline_color)
        } else {
            style.stroke
        };
        let fill = if halo { None } else { style.fill };
        let mut mark = Mark::new(
            format!("{}{}", self.prefix, self.scene.marks.len()),
            geometry,
            Color(0, 0, 0, 0),
        );
        mark.label.clear();
        mark.fill = fill;
        mark.opacity = if halo {
            style.outline_opacity
        } else {
            style.opacity
        };
        mark.stroke = color.filter(|_| width > 0.).map(|color| GraphicStroke {
            color,
            width,
            cap: if halo && !dot {
                StrokeCap::Square
            } else {
                StrokeCap::Butt
            },
        });
        self.add_mark(mark)
    }
    fn add_mark(&mut self, mark: Mark) -> Result<()> {
        ensure(
            self.scene.marks.len() < self.budget.marks,
            "Funnel annotation Mark budget exceeded",
        )?;
        let commands = match &mark.geometry {
            Geometry::Path(p) => p.len(),
            _ => 0,
        };
        self.commands = self
            .commands
            .checked_add(commands)
            .ok_or_else(|| ChartError("Funnel annotation path budget overflow".into()))?;
        ensure(
            self.commands <= self.budget.path_commands,
            "Funnel annotation path command budget exceeded",
        )?;
        self.scene
            .draw_order
            .push(DrawItem::Mark(self.scene.marks.len()));
        self.scene.marks.push(mark);
        Ok(())
    }
    fn label(
        &mut self,
        label: Label,
        font: FontOptions,
        stroke: Option<LabelStroke>,
    ) -> Result<()> {
        ensure(
            self.scene.labels.len() < self.budget.labels,
            "Funnel annotation Label budget exceeded",
        )?;
        let index = self.scene.labels.len();
        self.scene.typography.labels.insert(index, font);
        if let Some(stroke) = stroke {
            self.scene.typography.label_strokes.insert(index, stroke);
        }
        self.scene.labels.push(label);
        self.scene.draw_order.push(DrawItem::Label(index));
        Ok(())
    }
    fn custom(&mut self, note: Scene, binding_index: usize) -> Result<()> {
        note.validate()?;
        ensure(
            note.width == self.scene.width
                && note.height == self.scene.height
                && note.background.3 == 0,
            "custom Funnel annotation note needs a transparent full-size Scene",
        )?;
        ensure(
            note.legend.is_empty()
                && note.legend_roles.is_empty()
                && note.legend_targets.is_empty()
                && note.label_series.is_empty()
                && note.marks.iter().all(|m| m.series.is_empty()),
            "custom Funnel annotation note must not introduce business series or legend targets",
        )?;
        let mark_base = self.scene.marks.len();
        let label_base = self.scene.labels.len();
        let mut ids = BTreeMap::new();
        for (index, mark) in note.marks.iter().enumerate() {
            ids.insert(
                mark.id.clone(),
                format!("{}note:{binding_index}:{index}", self.prefix),
            );
        }
        // Append pools first, then restore the caller's actual mixed paint order.
        let order_base = self.scene.draw_order.len();
        for mut mark in note.marks.iter().cloned() {
            mark.id = ids[&mark.id].clone();
            self.add_mark(mark)?;
        }
        for (index, label) in note.labels.iter().cloned().enumerate() {
            let font = note.resolved_label_font(index);
            self.label(
                label,
                FontOptions {
                    family: Some(font.family),
                    weight: Some(font.weight),
                },
                note.typography.label_strokes.get(&index).copied(),
            )?;
            if let Some(mode) = note.typography.label_whitespace.get(&index) {
                self.scene
                    .typography
                    .label_whitespace
                    .insert(label_base + index, *mode);
            }
            if let Some(clip) = note.label_clips.get(&index) {
                self.scene
                    .label_clips
                    .insert(label_base + index, clip.clone());
            }
        }
        self.scene.draw_order.truncate(order_base);
        self.scene
            .draw_order
            .extend(note.draw_items().into_iter().map(|item| match item {
                DrawItem::Mark(i) => DrawItem::Mark(mark_base + i),
                DrawItem::Label(i) => DrawItem::Label(label_base + i),
            }));
        self.scene
            .opacity_groups
            .extend(
                note.opacity_groups
                    .iter()
                    .enumerate()
                    .map(|(index, group)| MarkOpacityGroup {
                        id: format!("{}note:{binding_index}:group:{index}", self.prefix),
                        members: group.members.iter().map(|id| ids[id].clone()).collect(),
                        opacity: group.opacity,
                    }),
            );
        Ok(())
    }
}
fn shape(binding: &BoundFunnelAnnotation) -> Geometry {
    let degenerate = match binding.kind {
        FunnelAnnotationKind::Circle | FunnelAnnotationKind::Dot => binding.size == 0.,
        FunnelAnnotationKind::Rect => binding.width == 0. || binding.height == 0.,
    };
    // Native SVG primitives with a zero radius/dimension have no shape ink.
    // Exporting their commands as a generic stroked path would instead create
    // a line or a square-cap point footprint. Keep the paint slots, not that ink.
    if degenerate {
        return Geometry::Path(Vec::new());
    }
    match binding.kind {
        FunnelAnnotationKind::Circle | FunnelAnnotationKind::Dot => Geometry::Circle {
            x: binding.center[0],
            y: binding.center[1],
            radius: binding.size / 2.,
        },
        FunnelAnnotationKind::Rect => crate::cartesian::round_bar(
            Geometry::Rect {
                x: binding.center[0] - binding.width / 2.,
                y: binding.center[1] - binding.height / 2.,
                width: binding.width,
                height: binding.height,
            },
            binding.border_radius,
        ),
    }
}
fn validate_core(
    bindings: &[BoundFunnelAnnotation],
    theme: &FunnelAnnotationTheme,
    budget: FunnelAnnotationBudget,
) -> Result<()> {
    let mut marks = 0usize;
    let mut labels = 0usize;
    let mut commands = 0usize;
    for binding in bindings {
        let extent = if binding.kind == FunnelAnnotationKind::Rect {
            [binding.width / 2., binding.height / 2.]
        } else {
            [binding.size / 2.; 2]
        };
        ensure(
            (0..2).all(|axis| {
                [
                    binding.center[axis] - extent[axis],
                    binding.center[axis] + extent[axis],
                ]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
            }),
            "Funnel annotation shape exceeds Scene limits",
        )?;
        let link_copies = 1 + usize::from(theme.link.outline_width > 0.);
        let shape_copies = 1 + usize::from(theme.outline.outline_width > 0.);
        marks += link_copies + shape_copies;
        commands += 3 * link_copies
            + match shape(binding) {
                Geometry::Path(ref p) => p.len() * shape_copies,
                _ => 0,
            };
        if matches!(binding.note, FunnelAnnotationNote::Text(_)) {
            labels += 1 + usize::from(theme.text.outline_width > 0.);
        }
        ensure(
            marks <= budget.marks && labels <= budget.labels && commands <= budget.path_commands,
            "Funnel annotation builtin paint budget exceeded",
        )?;
    }
    Ok(())
}
pub(super) fn paint(
    frame: &FunnelSvgFrame,
    bindings: &[BoundFunnelAnnotation],
    factories: &mut FunnelAnnotationFactories<'_>,
    theme: &FunnelAnnotationTheme,
    budget: FunnelAnnotationBudget,
) -> Result<Scene> {
    validate_core(bindings, theme, budget)?;
    let mut scene = frame.config().scene()?;
    scene.background = Color(0, 0, 0, 0);
    let mut nonce = 0usize;
    let prefix = loop {
        let candidate = format!("funnel:annotation:{nonce}:");
        if !frame
            .scene()
            .marks
            .iter()
            .any(|m| m.id.starts_with(&candidate))
            && !frame
                .scene()
                .opacity_groups
                .iter()
                .any(|group| group.id.starts_with(&candidate))
        {
            break candidate;
        }
        nonce += 1;
    };
    let mut painter = Painter {
        scene,
        budget,
        commands: 0,
        prefix,
    };
    for (index, binding) in bindings.iter().enumerate() {
        let link = Geometry::Path(
            binding
                .computed
                .points
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    if i == 0 {
                        C::Move(P::new(p[0], p[1]))
                    } else {
                        C::Line(P::new(p[0], p[1]))
                    }
                })
                .collect(),
        );
        if theme.link.outline_width > 0. {
            painter.mark(link.clone(), theme.link, true, false)?;
        }
        let shape = shape(binding);
        // SVG DotAnnotationOutline uses the outline theme for its halo (the
        // upstream Canvas helper instead uses symbol; this API follows SVG).
        if theme.outline.outline_width > 0. {
            painter.mark(
                shape.clone(),
                theme.outline,
                true,
                binding.kind == FunnelAnnotationKind::Dot,
            )?;
        }
        painter.mark(
            shape,
            if binding.kind == FunnelAnnotationKind::Dot {
                theme.symbol
            } else {
                theme.outline
            },
            false,
            false,
        )?;
        painter.mark(link, theme.link, false, false)?;
        match &binding.note {
            FunnelAnnotationNote::Text(text) => {
                let label = Label {
                    rotation: 0.,
                    x: binding.computed.text[0],
                    y: binding.computed.text[1],
                    text: text.clone(),
                    color: theme.text.color,
                    size: theme.text.font_size,
                    anchor: TextAnchor::Start,
                };
                if theme.text.outline_width > 0. {
                    let halo = label.clone();
                    painter.label(
                        halo,
                        theme.text.font.clone(),
                        Some(LabelStroke {
                            color: theme.text.outline_color,
                            width: 2. * theme.text.outline_width,
                        }),
                    )?;
                }
                painter.label(label, theme.text.font.clone(), None)?;
            }
            FunnelAnnotationNote::Custom(key) => {
                let part = &frame.parts()[binding.part_index];
                let note = factories.note.as_mut().ok_or_else(|| {
                    ChartError("Funnel annotation note renderer is missing".into())
                })?(FunnelAnnotationNoteContext {
                    key,
                    frame,
                    binding,
                    part,
                    datum: &frame.source()[binding.source_index],
                    x: binding.computed.text[0],
                    y: binding.computed.text[1],
                })?;
                painter.custom(note, index)?;
            }
        }
    }
    painter.scene.validate()?;
    Ok(painter.scene)
}
