use super::*;
use crate::{SemanticTextMeasurement, SemanticTextProfile, SemanticTextRequest};

/// Admission remains in the public Chart candidate validation, after the
/// unchanged Scene bounds and command-budget checks. A single Rect has the
/// same coverage under either authored fill rule.
pub(super) fn rectangular_clip(scene: &Scene, index: usize) -> Option<ClipRect> {
    let Geometry::Rect {
        x,
        y,
        width,
        height,
    } = &scene.label_clips.get(&index)?.geometry
    else {
        return None;
    };
    Some(ClipRect {
        x: *x,
        y: *y,
        width: *width,
        height: *height,
    })
}

/// The existing measured/collision/plain/outlined placement is in Scene
/// coordinates. Only its containing origin changes; no source geometry,
/// text, font, glyph commands or alpha is rewritten.
pub(super) fn relative_to_clip(
    index: usize,
    mut style: Style,
    state: &State,
    clipped: bool,
) -> Style {
    if clipped && let Some(clip) = state.scene.with(|scene| rectangular_clip(scene, index)) {
        if let Some(Length::Px(left)) = style.left {
            style.left = Some(Length::Px(left - clip.x));
        }
        if let Some(Length::Px(top)) = style.top {
            style.top = Some(Length::Px(top - clip.y));
        }
    }
    style
}

pub(super) struct SemanticLabelsInput {
    pub profile: Vary<SemanticTextProfile>,
    pub measure: RenderFn<SemanticTextRequest, SemanticTextMeasurement>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct SemanticLabelsFrame {
    pub profile: SemanticTextProfile,
    pub labels: BTreeMap<usize, SemanticTextMeasurement>,
}

#[derive(Clone)]
pub(super) struct SemanticLabelsState {
    pub frame: Cage<SemanticLabelsFrame>,
    measure: Renderer<SemanticTextRequest, SemanticTextMeasurement>,
}

impl SemanticLabelsState {
    pub fn new(
        scope: &Scope,
        measure: RenderFn<SemanticTextRequest, SemanticTextMeasurement>,
    ) -> Self {
        Self {
            frame: scope.cage(SemanticLabelsFrame::default()),
            measure: Renderer::new(Some(measure)),
        }
    }

    pub fn clear(&self) {
        self.measure.clear();
    }

    pub fn prepare(
        &self,
        scene: &Scene,
        hidden: &BTreeSet<String>,
        profile: SemanticTextProfile,
        mut outlined: impl FnMut(&Label) -> bool,
    ) -> Result<SemanticLabelsFrame> {
        profile.validate()?;
        let mut requests = Vec::new();
        let mut input_bytes = 0usize;
        for (index, label) in scene.labels.iter().enumerate() {
            if label.color.3 == 0
                || outlined(label)
                || scene
                    .label_series
                    .get(&index)
                    .is_some_and(|id| hidden.contains(id))
            {
                continue;
            }
            ensure(label.rotation == 0., "ordinary semantic Text cannot rotate")?;
            let request = profile.request(label, &scene.resolved_label_font(index))?;
            input_bytes += request.text.len()
                + request
                    .font
                    .family
                    .iter()
                    .map(|family| match family {
                        crate::FontFamily::Named(name) => name.len(),
                        _ => 1,
                    })
                    .sum::<usize>();
            ensure(
                input_bytes <= 64 * 1024 * 1024,
                "semantic text input budget exceeded",
            )?;
            requests.push((index, request));
        }
        // Admit the entire bounded request set before invoking application code.
        let previous = self.frame.get_untracked();
        let mut frame = SemanticLabelsFrame {
            profile,
            labels: BTreeMap::new(),
        };
        for (index, request) in requests {
            let measurement = if let Some(previous) = previous.labels.get(&index)
                && previous.request() == &request
            {
                previous.clone()
            } else {
                self.measure.call(request.clone())?
            };
            measurement.validate_for(&request)?;
            measurement.place_label(&scene.labels[index])?;
            frame.labels.insert(index, measurement);
        }
        Ok(frame)
    }
}

impl State {
    pub(super) fn prepare_semantic_labels(
        &self,
        scene: &Scene,
        hidden: &BTreeSet<String>,
        profile: SemanticTextProfile,
    ) -> Result<Option<SemanticLabelsFrame>> {
        self.semantic_labels
            .as_ref()
            .map(|state| {
                state.prepare(scene, hidden, profile, |label| {
                    #[cfg(feature = "raster")]
                    {
                        self.outlined
                            .as_ref()
                            .is_some_and(|config| !config.1.rotated_only || label.rotation != 0.)
                            || self.painted.as_ref().is_some_and(|config| {
                                !config.1.rotated_only || label.rotation != 0.
                            })
                    }
                    #[cfg(not(feature = "raster"))]
                    {
                        let _ = label;
                        false
                    }
                })
            })
            .transpose()
    }
}

/// Both the plain and un-outlined fallback path use the same line-box contract.
pub(super) fn ordinary_container(index: usize, label: &Label, state: &State) -> Style {
    let measurement = state
        .semantic_labels
        .as_ref()
        .and_then(|state| state.frame.with(|frame| frame.labels.get(&index).cloned()));
    if let Some(measurement) = measurement {
        let mut placement = measurement
            .place_label(label)
            .expect("accepted semantic label placement");
        let mut visible = true;
        if let Some(ink) = state
            .layout
            .with(|layout| layout.labels.get(&index).copied())
        {
            if let Some(ink) = ink {
                placement = placement
                    .with_placed_ink_bounds(ink)
                    .expect("accepted semantic label collision delta");
            } else {
                visible = false;
            }
        }
        let rect = placement.line_box;
        Style {
            display: Some(if visible {
                Display::Flex
            } else {
                Display::None
            }),
            ..Style::new()
                .position(Position::Absolute)
                .left(Length::Px(rect.x))
                .top(Length::Px(rect.y))
                .width(Length::Px(rect.width))
                .height(Length::Px(rect.height))
                .pointer_events(PointerEvents::None)
        }
    } else {
        let mut style = state.scene.with(|scene| label_container(label, scene));
        if let Some(placement) = state
            .layout
            .with(|layout| layout.labels.get(&index).copied())
        {
            match placement {
                None => style.display = Some(Display::None),
                Some(rect) => {
                    style.left = Some(Length::Px(rect.x));
                    style.top = Some(Length::Px(rect.y));
                    style.width = Some(Length::Px(rect.width));
                    style.height = Some(Length::Px(rect.height));
                    style.justify = Some(Justify::Start);
                }
            }
        }
        style
    }
}

pub(super) fn ordinary_text_style(index: usize, label: &Label, state: &State) -> Style {
    let line_height = state.semantic_labels.as_ref().and_then(|state| {
        state.frame.with(|frame| {
            frame
                .labels
                .get(&index)
                .map(|measurement| measurement.request().line_height)
        })
    });
    let font = state.scene.with(|scene| scene.resolved_label_font(index));
    Style {
        color: Some(label.color),
        font_size: Some(label.size),
        font_family: Some(font.family),
        font_weight: Some(font.weight),
        line_height: Some(line_height.unwrap_or(1.)),
        display: line_height.map(|_| Display::Block),
        letter_spacing: line_height.map(|_| 0.),
        white_space: Some(WhiteSpace::NoWrap),
        shrink: Some(0.),
        ..Style::new()
    }
}
