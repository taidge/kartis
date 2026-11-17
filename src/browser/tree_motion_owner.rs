//! Automatic Tree motion is an owner capability, not generic Scene animation.
use super::*;
use crate::browser::tree_animated_widgets::AnimatedFactory;

/// One actually adopted source and numeric presentation certificate.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTreeMotionFrame {
    pub data: TreeMotionFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub configuration: u64,
}

/// Automatic RAF is the default; Manual uses exact caller-supplied seconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTreeMotionClock {
    #[default]
    Automatic,
    Manual,
}
pub(super) struct MotionAttempt {
    options: Option<TreeMotionOptions>,
    configuration: u64,
    step: Option<f64>,
}
pub(super) type Renderer = dyn FnMut(&TreeMotionFrame) -> Result<TreeMotionPaintFrame>;
type MotionSourceBuilder = dyn FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout>;
pub(super) struct MotionMount {
    pub options: TreeMotionOptions,
    pub ordinary: bool,
    pub renderer: Box<Renderer>,
    pub widgets: Option<(BrowserTreeWidgetOptions, AnimatedFactory)>,
    pub has_widgets: bool,
}
#[derive(Clone)]
pub(super) struct MotionCandidate {
    pub model: TreeMotion,
    pub paint: TreeMotionPaintFrame,
    pub options: Option<TreeMotionOptions>,
    pub configuration: u64,
    pub time: Option<f64>,
}
pub(super) struct MotionController {
    pub accepted: RefCell<MotionCandidate>,
    desired: RefCell<Option<TreeMotionOptions>>,
    configuration: Cell<u64>,
    pending: Cell<bool>,
    pub ordinary: bool,
    pub widgets: bool,
    clock: Cell<BrowserTreeMotionClock>,
    step: Cell<Option<f64>>,
    pub rebase: Cell<bool>,
    renderer: RefCell<Option<Box<Renderer>>>,
}
impl MotionController {
    pub fn mounted(mount: MotionMount, initial: MotionCandidate) -> Self {
        Self {
            desired: RefCell::new(Some(mount.options)),
            configuration: Cell::new(0),
            pending: Cell::new(false),
            ordinary: mount.ordinary,
            widgets: mount.has_widgets,
            clock: Cell::new(BrowserTreeMotionClock::Automatic),
            step: Cell::new(None),
            rebase: Cell::new(false),
            renderer: RefCell::new(Some(mount.renderer)),
            accepted: RefCell::new(initial),
        }
    }
    pub fn pending(&self) -> bool {
        self.pending.get()
    }
    pub fn stop(&self) {
        self.pending.set(false);
        self.step.set(None);
        self.renderer.borrow_mut().take();
    }
}

fn layout(context: &BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> {
    tree_svg_layout_with_activity(
        context.config,
        context.source,
        context.tree,
        context.layers,
        &mut TreeFactories::default(),
        context.activity,
        &mut TreeActivityFactories::default(),
    )
}
fn ordinary_program() -> Result<BrowserTreeSvgProgram> {
    BrowserTreeSvgProgram::new(
        [
            ("links", TreeSvgLayerKind::Links),
            ("nodes", TreeSvgLayerKind::Nodes),
            ("labels", TreeSvgLayerKind::Labels),
            ("scene-extras", TreeSvgLayerKind::SceneExtras),
            ("mesh", TreeSvgLayerKind::Mesh),
        ]
        .into_iter()
        .map(|(key, kind)| TreeSvgLayer::new(key, kind))
        .collect::<Result<Vec<_>>>()?,
    )
}
pub(super) fn admit(options: &BrowserTreeOptions) -> Result<()> {
    scene::ensure(
        options.chart.renderer == BrowserRenderer::Svg,
        "automatic Tree motion requires SVG",
    )?;
    scene::ensure(
        options.tree.orientation != TreeOrientation::Radial,
        "automatic Tree motion does not support Radial orientation",
    )?;
    let mut policy = options.chart.clone();
    policy.motion = MotionPreference::System;
    admit_svg_policy(&policy, None)
}
pub(super) fn ordinary_order(plan: &TreeSvgPlan) -> Result<()> {
    let literal: Vec<_> = plan
        .layers()
        .iter()
        .flat_map(|occurrence| occurrence.items().iter().copied())
        .collect();
    scene::ensure(
        literal == plan.frame().layout().scene.draw_items(),
        "ordinary Tree motion requires links, nodes, labels, then Scene extras source order; use explicit SVG layers for another order",
    )
}
impl BrowserTree {
    pub fn mount_with_motion(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        motion: TreeMotionOptions,
    ) -> Result<Self> {
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            ordinary_program()?,
            Box::new(layout),
            motion,
            true,
            Box::new(tree_motion_paint),
            None,
        )
    }
    pub fn mount_with_factory_and_motion(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        mut build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        motion: TreeMotionOptions,
    ) -> Result<Self> {
        let build = move |context: &BrowserTreeSvgBuildContext<'_>| {
            build(&BrowserTreeBuildContext {
                config: context.config,
                source: context.source,
                tree: context.tree,
                activity: context.activity,
            })
        };
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            ordinary_program()?,
            Box::new(build),
            motion,
            true,
            Box::new(tree_motion_paint),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_svg_layers_and_motion(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        motion: TreeMotionOptions,
    ) -> Result<Self> {
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            program,
            Box::new(layout),
            motion,
            false,
            Box::new(tree_motion_paint),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_motion_and_animated_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        motion: TreeMotionOptions,
        widgets: BrowserTreeWidgetOptions,
        mut factory: impl FnMut(BrowserTreeAnimatedWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        let factory: AnimatedFactory = Box::new(move |context| Ok(Box::new(factory(context)?)));
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            ordinary_program()?,
            Box::new(layout),
            motion,
            true,
            Box::new(tree_motion_paint),
            Some((widgets, factory)),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_svg_layers_motion_and_animated_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        motion: TreeMotionOptions,
        widgets: BrowserTreeWidgetOptions,
        factory: impl FnMut(BrowserTreeAnimatedWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_with_factory_svg_layers_motion_and_animated_components(
            root, source, config, options, program, layout, motion, widgets, factory,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_motion_renderer_and_animated_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: impl FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        motion: TreeMotionOptions,
        renderer: impl FnMut(&TreeMotionFrame) -> Result<TreeMotionPaintFrame> + 'static,
        widgets: BrowserTreeWidgetOptions,
        mut factory: impl FnMut(BrowserTreeAnimatedWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        let factory: AnimatedFactory = Box::new(move |context| Ok(Box::new(factory(context)?)));
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            program,
            Box::new(build),
            motion,
            false,
            Box::new(renderer),
            Some((widgets, factory)),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_svg_layers_motion_and_animated_components<
        W: glory::Widget + 'static,
    >(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: impl FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        motion: TreeMotionOptions,
        widgets: BrowserTreeWidgetOptions,
        mut factory: impl FnMut(BrowserTreeAnimatedWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        let factory: AnimatedFactory = Box::new(move |context| Ok(Box::new(factory(context)?)));
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            program,
            Box::new(build),
            motion,
            false,
            Box::new(tree_motion_paint),
            Some((widgets, factory)),
        )
    }
    /// An explicit per-frame native presenter supports edited source geometry.
    /// Its result must retain this exact immutable numeric motion snapshot.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_motion_renderer(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: impl FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        motion: TreeMotionOptions,
        renderer: impl FnMut(&TreeMotionFrame) -> Result<TreeMotionPaintFrame> + 'static,
    ) -> Result<Self> {
        Self::mount_motion_inner(
            root,
            source,
            config,
            options,
            program,
            Box::new(build),
            motion,
            false,
            Box::new(renderer),
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn mount_motion_inner(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: Box<MotionSourceBuilder>,
        motion: TreeMotionOptions,
        ordinary: bool,
        renderer: Box<Renderer>,
        widgets: Option<(BrowserTreeWidgetOptions, AnimatedFactory)>,
    ) -> Result<Self> {
        admit(&options)?;
        motion.validate()?;
        scene::ensure(
            widgets
                .as_ref()
                .is_none_or(|(options, _)| options.source_ordered),
            "animated Tree components require source order",
        )?;
        let svg = SvgMount::new(program, build)?;
        Self::mount_program_motion(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            None,
            None,
            Some(svg),
            Some(MotionMount {
                options: motion,
                ordinary,
                renderer,
                has_widgets: widgets.is_some(),
                widgets,
            }),
        )
    }
    pub fn set_motion_options(&self, options: Option<TreeMotionOptions>) -> Result<()> {
        self.handle().set_motion_options(options)
    }
    pub fn set_motion_clock(&self, clock: BrowserTreeMotionClock) -> Result<()> {
        self.handle().set_motion_clock(clock)
    }
    pub fn motion_clock(&self) -> Result<BrowserTreeMotionClock> {
        self.handle().motion_clock()
    }
    pub fn advance_motion(&self, seconds: f64) -> Result<()> {
        self.handle().advance_motion(seconds)
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserTreeMotionFrame>> {
        self.handle().motion_frame()
    }
}
impl BrowserTreeHandle {
    pub fn set_motion_clock(&self, clock: BrowserTreeMotionClock) -> Result<()> {
        let driver = self.driver()?;
        let motion = driver.motion.as_ref().ok_or_else(|| {
            ChartError("Tree was mounted without automatic SVG motion capability".into())
        })?;
        scene::ensure(
            !driver.processing.get(),
            "cannot change Tree motion clock during its frame",
        )?;
        driver.request()?;
        motion.clock.set(clock);
        motion.rebase.set(true);
        Ok(())
    }
    pub fn motion_clock(&self) -> Result<BrowserTreeMotionClock> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        scene::ensure(driver.alive.get(), "Tree owner is stopped")?;
        Ok(driver
            .motion
            .as_ref()
            .ok_or_else(|| {
                ChartError("Tree was mounted without automatic SVG motion capability".into())
            })?
            .clock
            .get())
    }
    /// Commit an exact manual interval with any already pending target. While
    /// a native press holds automatic presentation, a source-free step pauses.
    pub fn advance_motion(&self, seconds: f64) -> Result<()> {
        scene::ensure(
            seconds.is_finite() && (0. ..=60.).contains(&seconds),
            "Tree motion dt must be finite and in 0..=60 seconds",
        )?;
        let driver = self.driver()?;
        let motion = driver.motion.as_ref().ok_or_else(|| {
            ChartError("Tree was mounted without automatic SVG motion capability".into())
        })?;
        scene::ensure(
            motion.clock.get() == BrowserTreeMotionClock::Manual,
            "explicit Tree motion intervals require Manual clock",
        )?;
        scene::ensure(
            !driver.processing.get() && motion.step.get().is_none(),
            "Tree motion step is already pending",
        )?;
        driver.request()?;
        motion.step.set(Some(seconds));
        self.flush()
    }
    /// Configure only a motion-capable SVG owner. `None` snaps to the current
    /// target and clears exits without replacing retained live component scopes.
    pub fn set_motion_options(&self, options: Option<TreeMotionOptions>) -> Result<()> {
        if let Some(options) = &options {
            options.validate()?;
        }
        let driver = self.driver()?;
        let motion = driver.motion.as_ref().ok_or_else(|| {
            ChartError("Tree was mounted without automatic SVG motion capability".into())
        })?;
        let serial = motion
            .configuration
            .get()
            .checked_add(1)
            .ok_or_else(|| ChartError("Tree motion configuration exhausted".into()))?;
        driver.request()?;
        *motion.desired.borrow_mut() = options;
        motion.configuration.set(serial);
        motion.pending.set(true);
        Ok(())
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserTreeMotionFrame>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        scene::ensure(driver.alive.get(), "Tree owner is stopped")?;
        Ok(driver
            .core()?
            .tree_svg_stage()
            .and_then(|stage| stage.motion_frame()))
    }
}
impl Driver {
    pub(super) fn motion_pending(&self) -> bool {
        self.motion.as_ref().is_some_and(MotionController::pending)
    }
    pub(super) fn motion_running(&self) -> bool {
        self.motion.as_ref().is_some_and(|motion| {
            let accepted = motion.accepted.borrow();
            motion.clock.get() == BrowserTreeMotionClock::Automatic
                && self.motion_visible()
                && accepted.options.is_some()
                && !accepted.model.is_settled()
        })
    }
    pub(super) fn motion_step_pending(&self) -> bool {
        self.motion.as_ref().is_some_and(|m| m.step.get().is_some())
    }
    pub(super) fn motion_visible(&self) -> bool {
        if !self.surface.is_connected()
            || self.surface.owner_document().is_none_or(|doc| doc.hidden())
        {
            return false;
        }
        let bounds = self.surface.get_bounding_client_rect();
        bounds.width() > 0. && bounds.height() > 0.
    }
    pub(super) fn take_motion_attempt(&self, explicit: bool) -> Option<MotionAttempt> {
        self.motion.as_ref().map(|motion| {
            let choose = motion.pending.replace(false) || explicit;
            let old = motion.accepted.borrow();
            MotionAttempt {
                options: if choose {
                    motion.desired.borrow().clone()
                } else {
                    old.options.clone()
                },
                configuration: if choose {
                    motion.configuration.get()
                } else {
                    old.configuration
                },
                step: motion.step.take(),
            }
        })
    }
    pub(super) fn listen_motion(self: &Rc<Self>) -> Result<()> {
        if self.motion.is_none() {
            return Ok(());
        }
        let document = self
            .surface
            .owner_document()
            .ok_or_else(|| ChartError("Tree document missing".into()))?;
        let weak = Rc::downgrade(self);
        self.add_listener_on(document.into(), "visibilitychange", move |_| {
            if let Some(driver) = weak.upgrade()
                && driver.alive.get()
            {
                if let Some(motion) = &driver.motion {
                    motion.rebase.set(true);
                }
                if driver.motion_visible()
                    && let Err(error) = driver.request()
                {
                    driver.record(error);
                }
            }
        })
    }
    pub(super) fn prepare_motion(
        &self,
        candidate: &Accepted,
        source_update: bool,
        attempt: &MotionAttempt,
        time: Option<f64>,
    ) -> Result<Option<MotionCandidate>> {
        let Some(motion) = &self.motion else {
            return Ok(None);
        };
        let old = motion.accepted.borrow().clone();
        let options = attempt.options.clone();
        let configuration = attempt.configuration;
        let was_settled = old.model.is_settled();
        let mut model = old.model;
        if let Some(options) = &options {
            model.set_options(options.clone())?;
        }
        let reduced = self
            .core()?
            .state
            .borrow()
            .as_ref()
            .is_some_and(|state| state.reduced);
        model.set_reduced_motion(options.is_none() || reduced)?;
        if source_update {
            scene::ensure(
                candidate.layout.layout.options().orientation != TreeOrientation::Radial,
                "automatic Tree motion does not support Radial orientation",
            )?;
            model.retarget(TreeEventFrame::with_activity(
                candidate.request.config.clone(),
                candidate.layout.clone(),
            )?)?;
        }
        let held = self.press.get().held() && !source_update;
        let advance = time.filter(|_| {
            !held
                && motion.clock.get() == BrowserTreeMotionClock::Automatic
                && self.motion_visible()
        });
        if !held && let Some(dt) = attempt.step {
            model.tick(dt)?;
        } else if let Some(time) = advance {
            let dt = if motion.rebase.get() || source_update && was_settled {
                0.
            } else {
                old.time
                    .map_or(0., |old| ((time - old) / 1000.).clamp(0., 0.25))
            };
            model.tick(dt)?;
        }
        self.building.set(true);
        let result = (|| {
            let mut renderer = motion
                .renderer
                .try_borrow_mut()
                .map_err(|_| ChartError("Tree motion renderer is already in use".into()))?
                .take()
                .ok_or_else(|| ChartError("Tree motion renderer is already in use".into()))?;
            let result = renderer(model.frame());
            if self.alive.get() {
                *motion.renderer.borrow_mut() = Some(renderer);
            }
            scene::ensure(self.alive.get(), "Tree stopped during motion rendering")?;
            let paint = result?;
            scene::ensure(
                paint.motion() == model.frame(),
                "Tree motion renderer returned another numeric frame",
            )?;
            Ok(MotionCandidate {
                model,
                paint,
                options,
                configuration,
                time: if held || motion.rebase.get() || source_update && was_settled {
                    advance
                } else {
                    advance.or(old.time)
                },
            })
        })();
        self.building.set(false);
        result.map(Some)
    }
}
