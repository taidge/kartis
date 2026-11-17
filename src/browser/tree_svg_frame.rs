//! One source/physical-paint/Glory-layer admission, outside Engine borrows.
use super::plot_widgets::svg_occurrences::{Stage as SourceStage, occurrence_scene};
use super::tree_svg_layers::{BuiltinOccurrence, CurrentFactory, Host, Stage as LayerStage};
use super::*;
use crate::widget::plot_widgets::PlotWidgetsPacket;

pub(super) struct Mount {
    pub layout: SceneInteraction,
    pub plan: TreeSvgPlan,
    pub program: BrowserTreeSvgProgram,
    pub picking: BrowserTreePicking,
    pub components: Option<plot_widgets::Mount>,
    pub generation: u64,
    pub publication: u64,
    pub current: CurrentFactory,
    pub motion: Option<MotionMount>,
}
pub(super) struct MotionMount {
    pub paint: TreeMotionPaintFrame,
    pub frame: BrowserTreeMotionFrame,
    pub widgets: Option<(
        BrowserTreeWidgetOptions,
        super::tree_animated_widgets::AnimatedFactory,
    )>,
}
pub(super) struct Stage {
    layout: SceneInteraction,
    target_layout: SceneInteraction,
    motion: Option<MotionPresentation>,
    previous_motion: Option<BrowserTreeMotionFrame>,
    animated: Option<Rc<super::tree_animated_widgets::Stage>>,
    plan: TreeSvgPlan,
    program: BrowserTreeSvgProgram,
    picking: BrowserTreePicking,
    packet: Option<PlotWidgetsPacket>,
    source: Option<Rc<SourceStage>>,
    layers: Rc<LayerStage>,
    current: CurrentFactory,
    generation: u64,
    ratio: f64,
    projection: Vec<usize>,
    admission: Rc<Cell<bool>>,
}
#[derive(Clone)]
pub(super) struct MotionPresentation {
    pub paint: TreeMotionPaintFrame,
    pub frame: BrowserTreeMotionFrame,
    pub revision: u64,
    pub target_changed: bool,
}
impl Stage {
    pub(super) fn motion_frame(&self) -> Option<BrowserTreeMotionFrame> {
        if self.was_adopted() {
            self.motion.as_ref().map(|m| m.frame.clone())
        } else {
            self.previous_motion.clone()
        }
    }
    pub(super) fn was_adopted(&self) -> bool {
        self.admission.get()
    }
    pub(super) fn same_admission(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.admission, &other.admission)
    }
    pub(super) fn plan(&self) -> &TreeSvgPlan {
        &self.plan
    }
    pub(super) fn publication(&self) -> u64 {
        self.layers.publication()
    }
    pub(super) fn root(&self) -> Element {
        self.layers.root()
    }
    /// Automatic tooltip eligibility follows admitted live sampled ink. The
    /// target Mark remains the immutable payload, including zero target width.
    pub(super) fn tooltip_mark<'a>(&'a self, target: &'a Mark) -> Option<&'a Mark> {
        if self.motion.is_none() {
            return Some(target);
        }
        self.projection.iter().find_map(|index| {
            let mark = &self.layout.scene.marks[*index];
            (mark.id == target.id).then_some(mark)
        })
    }

    fn validate(&self, model: &SurfaceModel) -> Result<()> {
        scene::ensure(
            *model.scene() == self.layout.scene,
            "Tree SVG presentation Scene differs",
        )?;
        self.layers.validate_source(model)
    }
    fn matches(&self, model: &SurfaceModel, ratio: f64) -> bool {
        self.layers.view() == model.view()
            && self.layers.hidden() == model.hidden()
            && self.ratio == ratio
    }
    fn place(&self, model: &SurfaceModel) -> Result<()> {
        let result = (|| {
            // Label masks need the occurrence's resolved CSS dimensions. Attach
            // its foreignObject before placing children, while every accepted
            // data Cage still holds the previous publication.
            self.layers.place()?;
            if let Some(source) = &self.source {
                source.place()?;
            }
            if let Some(animated) = &self.animated {
                animated.place()?;
            }
            if let Some(source) = &self.source {
                source.validate()?;
            }
            if let Some(animated) = &self.animated {
                animated.validate()?;
            }
            self.layers.validate_source(model)?;
            self.layers.validate()
        })();
        if result.is_err() {
            self.rollback()?;
        }
        result
    }
    pub(super) fn rollback(&self) -> Result<()> {
        if self.was_adopted() {
            return Ok(());
        }
        if let Some(animated) = &self.animated {
            animated.rollback()?;
        }
        if let Some(source) = &self.source {
            source.rollback()?;
        }
        self.layers.rollback()?;
        Ok(())
    }
    pub(super) fn adopt(&self, tooltip: Option<&Rc<tree_tooltip_host::Stage>>) {
        if self.was_adopted() {
            return;
        }
        if let Some(source) = &self.source {
            source.adopt();
        }
        if let Some(animated) = &self.animated {
            animated.adopt();
        }
        self.layers.adopt();
        if let Some(tooltip) = tooltip {
            tooltip.adopt();
        }
        self.admission.set(true);
    }
    pub(super) fn finish(&self) -> Vec<ChartError> {
        let mut errors = self.layers.publish();
        if let Some(source) = &self.source {
            errors.extend(source.finish());
        }
        if let Some(animated) = &self.animated {
            errors.extend(animated.finish());
        }
        errors.extend(self.layers.finish());
        errors
    }
}

pub(super) fn policy(options: &BrowserOptions) -> Result<()> {
    scene::ensure(
        options.renderer == BrowserRenderer::Svg,
        "Tree SVG layers require SVG renderer",
    )?;
    scene::ensure(
        options.highlight == BrowserHighlight::Controlled
            && options.crosshair.is_none()
            && options.editable_brush.is_none(),
        "Tree SVG layers require controlled highlighting without crosshair or editable brush",
    )?;
    scene::ensure(
        options.interaction.picking == crate::surface::Picking::Geometry
            && options.interaction.view_mode == crate::surface::SurfaceViewMode::Transform,
        "Tree SVG layers require transformed source Geometry picking",
    )?;
    scene::ensure(
        options.motion != MotionPreference::Animate,
        "Tree SVG layers do not support forced animation",
    )
}
impl BrowserChart {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn mount_tree_svg(
        root: Element,
        layout: SceneInteraction,
        mut options: BrowserOptions,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        picking: BrowserTreePicking,
        components: Option<plot_widgets::Mount>,
        generation: u64,
        publication: u64,
        current: CurrentFactory,
    ) -> Result<Self> {
        policy(&options)?;
        options.interaction.plot = layout.plot;
        options.interaction_data = layout.interaction.clone();
        Self::mount_base(
            root,
            layout.scene.clone(),
            options,
            MarkBlends::default(),
            None,
            None,
            Some(Mount {
                layout,
                plan,
                program,
                picking,
                components,
                generation,
                publication,
                current,
                motion: None,
            }),
            true,
        )
    }
}
impl BrowserChart {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn mount_tree_motion_svg(
        root: Element,
        layout: SceneInteraction,
        mut options: BrowserOptions,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        picking: BrowserTreePicking,
        generation: u64,
        publication: u64,
        current: CurrentFactory,
        paint: TreeMotionPaintFrame,
        frame: BrowserTreeMotionFrame,
        widgets: Option<(
            BrowserTreeWidgetOptions,
            super::tree_animated_widgets::AnimatedFactory,
        )>,
    ) -> Result<Self> {
        let mut checked = options.clone();
        checked.motion = MotionPreference::System;
        policy(&checked)?;
        options.interaction.plot = layout.plot;
        options.interaction_data = layout.interaction.clone();
        Self::mount_base(
            root,
            layout.scene.clone(),
            options,
            MarkBlends::default(),
            None,
            None,
            Some(Mount {
                layout,
                plan,
                program,
                picking,
                components: None,
                generation,
                publication,
                current,
                motion: Some(MotionMount {
                    paint,
                    frame,
                    widgets,
                }),
            }),
            true,
        )
    }
}
impl Engine {
    pub(super) fn tree_svg_mesh_target(&self, event: &Event) -> bool {
        let Some(stage) = self.tree_svg_stage() else {
            return false;
        };
        event
            .target()
            .and_then(|t| t.dyn_into::<Element>().ok())
            .and_then(|e| e.closest("[data-tree-svg-mesh]").ok().flatten())
            .is_some_and(|e| stage.root().contains(Some(&e)))
    }
    pub(super) fn reserve_tree_svg_publication(&self) -> Result<u64> {
        let next = self
            .tree_svg_publication
            .get()
            .checked_add(1)
            .ok_or_else(|| ChartError("Tree SVG publication exhausted".into()))?;
        self.tree_svg_publication.set(next);
        Ok(next)
    }
    pub(super) fn tree_svg_stage(&self) -> Option<Rc<Stage>> {
        self.state
            .try_borrow()
            .ok()
            .and_then(|s| s.as_ref().and_then(|s| s.tree_svg.clone()))
    }
    pub(super) fn tree_svg_packet(&self) -> Option<PlotWidgetsPacket> {
        self.tree_svg_stage().and_then(|s| s.packet.clone())
    }
    pub(super) fn tree_svg_layer_frame(&self, key: &str) -> Option<BrowserTreeSvgLayerFrame> {
        self.tree_svg_host
            .borrow()
            .as_ref()
            .and_then(|h| h.current_frame(key))
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_tree_svg(
        &self,
        layout: SceneInteraction,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        picking: BrowserTreePicking,
        packet: Option<PlotWidgetsPacket>,
        generation: u64,
        publication: u64,
        current: CurrentFactory,
    ) -> Result<Rc<Stage>> {
        self.prepare_svg_inner(
            layout,
            plan,
            program,
            picking,
            packet,
            generation,
            publication,
            current,
            None,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_tree_motion_svg(
        &self,
        layout: SceneInteraction,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        picking: BrowserTreePicking,
        generation: u64,
        publication: u64,
        current: CurrentFactory,
        paint: TreeMotionPaintFrame,
        frame: BrowserTreeMotionFrame,
        revision: u64,
        target_changed: bool,
    ) -> Result<Rc<Stage>> {
        let prior = self.tree_svg_stage();
        self.prepare_svg_inner(
            layout,
            plan,
            program,
            picking,
            None,
            generation,
            publication,
            current,
            prior.as_ref(),
            Some(MotionPresentation {
                paint,
                frame,
                revision,
                target_changed,
            }),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_svg_inner(
        &self,
        layout: SceneInteraction,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        picking: BrowserTreePicking,
        packet: Option<PlotWidgetsPacket>,
        generation: u64,
        publication: u64,
        current: CurrentFactory,
        prior: Option<&Rc<Stage>>,
        motion: Option<MotionPresentation>,
    ) -> Result<Rc<Stage>> {
        scene::ensure(self.alive.get(), "Tree SVG chart stopped")?;
        program.validate_for_build()?;
        scene::ensure(
            !self.queue.borrow().iter().any(|input| match input {
                Input::Scene(_, animate) => *animate,
                Input::InteractionScene(c) => c.2,
                Input::ContourScene(..) | Input::BlendedAnimation(_) => true,
                Input::Layout(c) => !c.1.is_empty(),
                _ => false,
            }),
            "Tree SVG cannot prepare while animation or blends are queued",
        )?;
        let (mut model, surface) = {
            let state = self
                .state
                .try_borrow()
                .map_err(|_| ChartError("Tree SVG frame is already in use".into()))?;
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree SVG chart stopped".into()))?;
            let mut policy_options = s.options.clone();
            if motion.is_some() {
                policy_options.motion = MotionPreference::System;
            }
            policy(&policy_options)?;
            scene::ensure(
                s.tree_canvas.is_none()
                    && s.motion.is_none()
                    && s.current_blends().is_empty()
                    && s.blends.is_empty(),
                "Tree SVG layers do not support raw Canvas, active animation or blends",
            )?;
            (s.model.clone(), s.surface.clone())
        };
        if let Some(candidate) = self.tree_svg_retarget_model.borrow().as_ref() {
            model = candidate.clone();
        }
        scene::ensure(
            plan.frame().layout().scene == layout.scene,
            "Tree SVG layout differs from plan",
        )?;
        let target_layout = layout;
        let layout = match &motion {
            Some(motion) => motion_layout(&target_layout, &motion.paint)?,
            None => target_layout.clone(),
        };
        let previous_motion = prior.and_then(|p| p.motion_frame());

        let excluded = packet
            .as_ref()
            .map(|p| p.replace_ink().clone())
            .unwrap_or_default();
        let mut projection = if program.is_interactive() {
            match &motion {
                Some(motion) => {
                    let live: BTreeSet<_> =
                        motion.paint.live_mark_indices().iter().copied().collect();
                    plan.layers()
                        .iter()
                        .flat_map(|o| motion_items(&motion.paint, &o.layer.kind))
                        .filter_map(|i| match i {
                            DrawItem::Mark(index) if live.contains(&index) => Some(index),
                            _ => None,
                        })
                        .filter(|i| !excluded.contains(&layout.scene.marks[*i].id))
                        .collect()
                }
                None => plan
                    .mark_indices()
                    .into_iter()
                    .filter(|i| !excluded.contains(&layout.scene.marks[*i].id))
                    .collect(),
            }
        } else {
            Vec::new()
        };
        if prior.is_none() || motion.is_some() && self.tree_svg_retarget_model.borrow().is_none() {
            model.set_projected_layout(layout.clone(), projection.clone())?;
        } else {
            model.set_widget_replacements(BTreeSet::new())?;
            model.set_source_paint_order(projection.clone())?;
        }
        let ratio = blends::ratio(self.window.device_pixel_ratio(), model.scene());
        let source = self.tree_svg_sources.borrow().clone();
        let animated_host = self.tree_animated_widgets.borrow().clone();
        let estimate = if let (Some(host), Some(motion)) = (&animated_host, &motion) {
            host.estimate(
                &motion.paint,
                &plan,
                motion.revision,
                prior.and_then(|stage| stage.animated.as_ref()),
            )?
        } else {
            match (&source, &packet) {
                (Some(source), Some(packet)) => {
                    if prior.is_some() {
                        source.live()
                    } else {
                        source.estimate(&plan, packet)?
                    }
                }
                (None, None) => 0,
                _ => {
                    return Err(ChartError(
                        "Tree SVG source component mount differs from packet".into(),
                    ));
                }
            }
        };
        let existing = self.tree_svg_host.borrow().clone();
        let host = if let Some(host) = existing {
            host
        } else {
            let host = Host::new(surface.clone().into())?;
            *self.tree_svg_host.borrow_mut() = Some(host.clone());
            host
        };
        let layer_prior =
            prior.filter(|p| p.generation == generation && p.program.same_program(&program));
        if layer_prior.is_none() {
            host.preflight(&plan, &program, estimate)?;
        }
        let language = ordered_widgets::inherited(&surface.clone().into(), "lang");
        let (source_stage, mut composed) = if let (Some(source), Some(packet)) = (&source, &packet)
        {
            let (stage, elements) = source.prepare(
                &plan,
                packet,
                &model,
                ratio,
                language,
                prior.and_then(|p| p.source.as_ref()),
            )?;
            (Some(stage), elements)
        } else {
            (None, BTreeMap::new())
        };
        let (animated, animated_elements) =
            if let (Some(host), Some(motion)) = (&animated_host, &motion) {
                let prepared = if motion.target_changed {
                    host.prepare_target(
                        &motion.paint,
                        &model,
                        &plan,
                        motion.revision,
                        generation,
                        publication,
                        prior.and_then(|p| p.animated.as_ref()),
                    )
                } else {
                    host.prepare_sample(
                        &motion.paint,
                        &model,
                        &plan,
                        motion.revision,
                        generation,
                        publication,
                        prior.and_then(|p| p.animated.as_ref()),
                    )
                };
                let (stage, elements) = prepared?;
                projection.retain(|index| {
                    !stage
                        .replacement_mark_ids()
                        .contains(&layout.scene.marks[*index].id)
                });
                model.set_source_paint_order(projection.clone())?;
                (Some(stage), elements)
            } else {
                (None, BTreeMap::new())
            };
        composed.extend(animated_elements);
        let document = surface
            .owner_document()
            .ok_or_else(|| ChartError("Tree SVG document missing".into()))?;
        let hit_ids = model.source_ink_ids();
        let mut builtins = Vec::new();
        for occurrence in plan.layers() {
            if matches!(occurrence.layer.kind, TreeSvgLayerKind::Custom(_)) {
                continue;
            }
            let element = if let Some(element) = composed.remove(&occurrence.layer.key) {
                element
            } else {
                let native = if occurrence.layer.kind == TreeSvgLayerKind::Mesh {
                    mesh(
                        &document,
                        &plan,
                        &model,
                        picking,
                        program.is_interactive(),
                        program.is_debug_mesh(),
                    )?
                } else {
                    let items = motion.as_ref().map(|m| {
                        motion_items(&m.paint, &occurrence.layer.kind).collect::<Vec<_>>()
                    });
                    let (scene, _) = occurrence_scene(
                        model.scene(),
                        items.as_deref().unwrap_or(occurrence.items()),
                    );
                    svg::native_view(
                        &document,
                        &scene.without_series(model.hidden()),
                        model.paint_transform(),
                        &hit_ids,
                    )?
                };
                native_carrier(&document, model.scene(), &occurrence.layer.key, native)?
            };
            builtins.push(BuiltinOccurrence {
                key: occurrence.layer.key.clone(),
                element,
            });
        }
        let source_count = source.as_ref().map_or(0, |s| s.live())
            + animated_host.as_ref().map_or(0, |s| s.live());
        let layers = if let Some(prior) = layer_prior {
            if let Some(motion) = &motion {
                prior.layers.retarget_motion(
                    &model,
                    builtins,
                    source_count,
                    publication,
                    motion.frame.data.clone(),
                )?
            } else {
                prior
                    .layers
                    .retarget(&model, builtins, source_count, publication)?
            }
        } else {
            if let Some(motion) = &motion {
                host.prepare_motion(
                    plan.clone(),
                    program.clone(),
                    &model,
                    generation,
                    publication,
                    current.clone(),
                    builtins,
                    source_count,
                    motion.frame.data.clone(),
                )?
            } else {
                host.prepare(
                    plan.clone(),
                    program.clone(),
                    &model,
                    generation,
                    publication,
                    current.clone(),
                    builtins,
                    source_count,
                )?
            }
        };
        scene::ensure(self.alive.get(), "Tree SVG stopped during preparation")?;
        Ok(Rc::new(Stage {
            layout,
            target_layout,
            motion,
            previous_motion,
            animated,
            plan,
            program,
            picking,
            packet,
            source: source_stage,
            layers,
            current,
            generation,
            ratio,
            projection,
            admission: prior
                .filter(|p| !p.was_adopted())
                .map_or_else(|| Rc::new(Cell::new(false)), |p| p.admission.clone()),
        }))
    }
    pub(super) fn enqueue_tree_svg(
        &self,
        stage: Rc<Stage>,
        tooltip: Rc<tree_tooltip_host::Stage>,
    ) -> Result<()> {
        {
            let state = self
                .state
                .try_borrow()
                .map_err(|_| ChartError("Tree SVG frame is already in use".into()))?;
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree SVG chart stopped".into()))?;
            let mut model = s.model.clone();
            model.set_layout(stage.layout.clone())?;
            model.set_source_paint_order(stage.projection.clone())?;
            stage.validate(&model)?;
            let mut tooltip_model = model.clone();
            if stage.motion.is_some() {
                tooltip_model.set_layout(stage.target_layout.clone())?;
            }
            tooltip.validate(&tooltip_model)?;
        }
        self.try_enqueue(Input::TreeTooltip(Box::new(tree_tooltip_host::Candidate {
            input: Some(Box::new(Input::TreeSvg(stage))),
            stage: tooltip,
            source_stage: None,
        })))
    }
    pub(super) fn repaint_tree_svg_if_needed(
        &self,
        baseline: Option<&(GestureState, Option<String>)>,
    ) -> Result<Option<Rc<Stage>>> {
        let (stage, candidate, focus) = {
            let state = self.state.borrow();
            let Some(s) = state.as_ref() else {
                return Ok(None);
            };
            let Some(stage) = &s.tree_svg else {
                return Ok(None);
            };
            let ratio = blends::ratio(self.window.device_pixel_ratio(), s.model.scene());
            if stage.matches(&s.model, ratio) {
                return Ok(Some(stage.clone()));
            }
            (stage.clone(), s.gesture(), s.focused.clone())
        };
        let (accepted, accepted_focus) =
            baseline.ok_or_else(|| ChartError("Tree SVG repaint checkpoint missing".into()))?;
        {
            let mut state = self.state.borrow_mut();
            let s = state.as_mut().expect("SVG chart");
            s.restore_gesture(accepted.clone());
            s.focused = accepted_focus.clone();
        }
        // Retarget against the candidate model while snapshots still expose the
        // accepted one. No factory/layout/placement callback is repeated.
        let result = self.retarget_svg(&stage, &candidate.model);
        scene::ensure(self.alive.get(), "Tree SVG stopped during retarget")?;
        let next = result?;
        {
            let mut state = self.state.borrow_mut();
            let s = state
                .as_mut()
                .ok_or_else(|| ChartError("Tree SVG chart stopped".into()))?;
            s.restore_gesture(candidate);
            s.focused = focus;
            s.tree_svg = Some(next.clone());
        }
        Ok(Some(next))
    }
    fn retarget_svg(&self, stage: &Rc<Stage>, model: &SurfaceModel) -> Result<Rc<Stage>> {
        // prepare_svg_inner gets its authoritative candidate model through a
        // temporary override, not by publishing it to State observers.
        let publication = self.reserve_tree_svg_publication()?;
        self.tree_svg_retarget_model.replace(Some(model.clone()));
        let result = self.prepare_svg_inner(
            stage.target_layout.clone(),
            stage.plan.clone(),
            stage.program.clone(),
            stage.picking,
            stage.packet.clone(),
            stage.generation,
            publication,
            stage.current.clone(),
            Some(stage),
            stage.motion.clone().map(|mut m| {
                m.frame.publication = publication;
                m.target_changed = false;
                m
            }),
        );
        self.tree_svg_retarget_model.borrow_mut().take();
        result
    }
    pub(super) fn place_tree_svg(&self) -> Result<Option<Rc<Stage>>> {
        let (stage, model) = {
            let state = self.state.borrow();
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree SVG chart stopped".into()))?;
            (s.tree_svg.clone(), s.model.clone())
        };
        if let Some(stage) = &stage
            && !stage.was_adopted()
        {
            stage.place(&model)?;
        }
        Ok(stage)
    }
}
impl State {
    pub(super) fn tree_tooltip_model(&self) -> Result<SurfaceModel> {
        let mut model = self.model.clone();
        if let Some(stage) = &self.tree_svg
            && stage.motion.is_some()
        {
            model.set_layout(stage.target_layout.clone())?;
        }
        Ok(model)
    }
    pub(super) fn process_tree_svg(&mut self, stage: Rc<Stage>) -> Result<Vec<SurfaceEvent>> {
        let mut policy_options = self.options.clone();
        if stage.motion.is_some() {
            policy_options.motion = MotionPreference::System;
        }
        policy(&policy_options)?;
        scene::ensure(
            self.motion.is_none() && self.blends.is_empty(),
            "Tree SVG cannot accept active animation or blends",
        )?;
        let mut model = self.model.clone();
        let changes = model.set_projected_layout(stage.layout.clone(), stage.projection.clone())?;
        stage.validate(&model)?;
        self.model = model;
        self.tree_svg = Some(stage);
        self.tree_canvas = None;
        self.motion = None;
        self.last_time = None;
        self.blends = Rc::default();
        Ok(changes)
    }
}

/// Keep builtin and Glory occurrences in the same native compositing family.
/// In particular WebKit can promote positioned foreignObject content above a
/// later sibling native SVG; full-Scene foreignObjects preserve literal order.
fn native_carrier(
    document: &web_sys::Document,
    scene: &Scene,
    key: &str,
    native: Element,
) -> Result<Element> {
    const SVG: &str = "http://www.w3.org/2000/svg";
    let carrier = document
        .create_element_ns(Some(SVG), "foreignObject")
        .map_err(host)?;
    carrier
        .set_attribute("class", "kartis-tree-svg-native")
        .map_err(host)?;
    carrier
        .set_attribute("data-tree-svg-key", key)
        .map_err(host)?;
    for (name, value) in [
        ("x", 0.),
        ("y", 0.),
        ("width", scene.width),
        ("height", scene.height),
    ] {
        carrier
            .set_attribute(name, &value.to_string())
            .map_err(host)?;
    }
    carrier
        .set_attribute("style", "overflow:hidden;pointer-events:none")
        .map_err(host)?;
    let content = document
        .create_element_ns(Some("http://www.w3.org/1999/xhtml"), "div")
        .map_err(host)?;
    content
        .set_attribute("class", "kartis-tree-svg-native-content")
        .map_err(host)?;
    content.set_attribute("style", "position:relative;width:100%;height:100%;pointer-events:none;isolation:isolate;overflow:hidden").map_err(host)?;
    let viewport = document.create_element_ns(Some(SVG), "svg").map_err(host)?;
    for (name, value) in [
        ("class", "kartis-tree-svg-native-view"),
        ("width", "100%"),
        ("height", "100%"),
        ("preserveAspectRatio", "none"),
        ("pointer-events", "none"),
        ("role", "presentation"),
        ("aria-hidden", "true"),
    ] {
        viewport.set_attribute(name, value).map_err(host)?;
    }
    viewport
        .set_attribute("viewBox", &format!("0 0 {} {}", scene.width, scene.height))
        .map_err(host)?;
    viewport.append_child(&native).map_err(host)?;
    content.append_child(&viewport).map_err(host)?;
    carrier.append_child(&content).map_err(host)?;
    Ok(carrier)
}

fn mesh(
    document: &web_sys::Document,
    plan: &TreeSvgPlan,
    model: &SurfaceModel,
    picking: BrowserTreePicking,
    interactive: bool,
    debug: bool,
) -> Result<Element> {
    let node = |name: &str| {
        document
            .create_element_ns(Some("http://www.w3.org/2000/svg"), name)
            .map_err(host)
    };
    let group = node("g")?;
    let BrowserTreePicking::Nearest { distance } = picking else {
        return Ok(group);
    };
    if !interactive {
        return Ok(group);
    };
    let plot = plan.frame().plot();
    let view = model.paint_transform();
    let viewport = node("svg")?;
    for (name, value) in [
        ("x", plot.x),
        ("y", plot.y),
        ("width", plot.width),
        ("height", plot.height),
    ] {
        viewport
            .set_attribute(name, &value.to_string())
            .map_err(host)?;
    }
    viewport
        .set_attribute(
            "viewBox",
            &format!("{} {} {} {}", plot.x, plot.y, plot.width, plot.height),
        )
        .map_err(host)?;
    viewport
        .set_attribute("preserveAspectRatio", "none")
        .map_err(host)?;
    viewport.set_attribute("overflow", "hidden").map_err(host)?;
    viewport
        .set_attribute("pointer-events", "none")
        .map_err(host)?;
    viewport.append_child(&group).map_err(host)?;
    let rect = node("rect")?;
    for (n, v) in [
        ("x", plot.x),
        ("y", plot.y),
        ("width", plot.width),
        ("height", plot.height),
    ] {
        rect.set_attribute(n, &v.to_string()).map_err(host)?;
    }
    rect.set_attribute("data-tree-svg-mesh", "true")
        .map_err(host)?;
    rect.set_attribute("fill", "transparent").map_err(host)?;
    rect.set_attribute("pointer-events", "all").map_err(host)?;
    group.append_child(&rect).map_err(host)?;
    if debug {
        let points = plan
            .frame()
            .nodes()
            .iter()
            .filter(|n| {
                !model
                    .hidden()
                    .contains(&plan.frame().source()[n.source_index].id)
            })
            .map(|n| {
                let p = view.project((n.position[0], n.position[1]))?;
                Ok(crate::mesh::MeshPoint {
                    id: plan.frame().source()[n.source_index].id.clone(),
                    x: p.0,
                    y: p.1,
                    clip: None,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let cells = crate::mesh::PointMesh::new(points)?.voronoi(plot)?;
        for cell in cells {
            let path = node("path")?;
            let mut d = String::new();
            use std::fmt::Write;
            for (i, p) in cell.polygon.iter().enumerate() {
                write!(d, "{}{} {}", if i == 0 { "M" } else { "L" }, p.0, p.1)
                    .expect("String write");
            }
            d.push('Z');
            path.set_attribute("d", &d).map_err(host)?;
            path.set_attribute("fill", "none").map_err(host)?;
            path.set_attribute("stroke", "rgba(0,0,0,0.35)")
                .map_err(host)?;
            path.set_attribute("pointer-events", "none").map_err(host)?;
            group.append_child(&path).map_err(host)?;
        }
        if distance > 0. {
            for n in plan.frame().nodes().iter().filter(|n| {
                !model
                    .hidden()
                    .contains(&plan.frame().source()[n.source_index].id)
            }) {
                let circle = node("circle")?;
                let p = view.project((n.position[0], n.position[1]))?;
                for (k, v) in [("cx", p.0), ("cy", p.1), ("r", distance)] {
                    circle.set_attribute(k, &v.to_string()).map_err(host)?;
                }
                circle.set_attribute("fill", "none").map_err(host)?;
                circle
                    .set_attribute("stroke", "rgba(0,0,0,0.2)")
                    .map_err(host)?;
                circle
                    .set_attribute("pointer-events", "none")
                    .map_err(host)?;
                group.append_child(&circle).map_err(host)?;
            }
        }
    }
    Ok(viewport)
}

fn motion_items<'a>(
    paint: &'a TreeMotionPaintFrame,
    kind: &TreeSvgLayerKind,
) -> impl Iterator<Item = DrawItem> + 'a {
    let items = match kind {
        TreeSvgLayerKind::Nodes => paint.node_items(),
        TreeSvgLayerKind::Links => paint.link_items(),
        TreeSvgLayerKind::Labels => paint.label_items(),
        TreeSvgLayerKind::SceneExtras => paint.extras(),
        _ => &[],
    };
    items.iter().copied()
}
fn motion_layout(
    target: &SceneInteraction,
    paint: &TreeMotionPaintFrame,
) -> Result<SceneInteraction> {
    let mut interaction = target.interaction.clone();
    if let Some(binding) = &mut interaction {
        let marks: BTreeMap<_, _> = paint
            .scene()
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect();
        for site in &mut binding.sites {
            let mark = marks
                .get(site.mark_id.as_str())
                .ok_or_else(|| ChartError("Tree motion active site Mark is missing".into()))?;
            let anchor = if let Some(anchor) = binding.anchors.get(&site.mark_id) {
                Some(anchor.resolve(&mark.geometry)?)
            } else {
                crate::crosshair::mark_center(mark)
            };
            site.position = anchor.ok_or_else(|| {
                ChartError("Tree motion geometry needs an explicit source anchor".into())
            })?;
        }
        binding.validate(paint.scene())?;
    }
    Ok(SceneInteraction {
        scene: paint.scene().clone(),
        plot: target.plot,
        interaction,
    })
}
