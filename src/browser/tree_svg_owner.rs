//! Tree owner control for literal SVG layers. Native render stages live in the
//! sibling SVG host, sharing the Engine's source/Widget/tooltip boundary.
use super::*;
use crate::browser::tree_svg_layers::{
    BrowserTreeSvgLayerFrame, BrowserTreeSvgProgram, CurrentFactory,
};

/// Inputs to one authoritative SVG Tree build. Label eligibility belongs to
/// the accepted/attempted literal program, rather than an unrelated Scene edit.
#[derive(Clone, Copy, Debug)]
pub struct BrowserTreeSvgBuildContext<'a> {
    pub config: &'a ChartConfig,
    pub source: &'a [HierarchyDatum],
    pub tree: &'a TreeOptions,
    pub activity: &'a TreeActivityOptions,
    pub layers: &'a [TreeSvgLayer],
    pub labels_required: bool,
}

type CurrentCallback = dyn Fn(Option<&str>) -> Result<()>;

/// A weak, accepted-occurrence action. The callback queues current-node work
/// for a later owner RAF; it never flushes a chart from a component callback.
#[derive(Clone)]
pub struct BrowserTreeSvgCurrentNode {
    callback: Rc<CurrentCallback>,
}
impl BrowserTreeSvgCurrentNode {
    pub(super) fn from_callback(callback: impl Fn(Option<&str>) -> Result<()> + 'static) -> Self {
        Self {
            callback: Rc::new(callback),
        }
    }
    pub fn set_current_node(&self, id: Option<&str>) -> Result<()> {
        (self.callback)(id)
    }
}

type SvgBuilder = dyn FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout>;

struct Bootstrap {
    ids: BTreeSet<String>,
    focus: Option<String>,
}
struct CurrentCommand {
    key: String,
    id: Option<String>,
    source_generation: u64,
    live: Weak<Cell<bool>>,
}

pub(super) struct SvgMount {
    pub program: BrowserTreeSvgProgram,
    builder: Box<SvgBuilder>,
    binding: Rc<RefCell<Weak<Driver>>>,
    bootstrap: Rc<RefCell<Option<Bootstrap>>>,
    commands: Rc<RefCell<Option<CurrentCommand>>>,
    pub current_factory: CurrentFactory,
}
impl SvgMount {
    pub(super) fn new(program: BrowserTreeSvgProgram, builder: Box<SvgBuilder>) -> Result<Self> {
        program.validate_for_build()?;
        let binding = Rc::new(RefCell::new(Weak::<Driver>::new()));
        let bootstrap = Rc::new(RefCell::new(None::<Bootstrap>));
        let commands = Rc::new(RefCell::new(None::<CurrentCommand>));
        let target = binding.clone();
        let initial = bootstrap.clone();
        let queue = commands.clone();
        let current_factory: CurrentFactory = Rc::new(move |key, live| {
            let key = key.to_owned();
            let target = target.clone();
            let initial = initial.clone();
            let queue = queue.clone();
            BrowserTreeSvgCurrentNode::from_callback(move |id| {
                scene::ensure(
                    live.upgrade().is_some_and(|flag| flag.get()),
                    "Tree SVG current-node occurrence is not accepted or was retired",
                )?;
                let owner = target.borrow().upgrade();
                let generation = if let Some(driver) = &owner {
                    scene::ensure(driver.alive.get(), "Tree SVG owner stopped")?;
                    let snapshot = driver.snapshot()?;
                    if let Some(id) = id {
                        scene::ensure(
                            snapshot
                                .layout
                                .layout
                                .source()
                                .iter()
                                .any(|datum| datum.id == id),
                            "Tree SVG current-node action references an unknown source",
                        )?;
                    }
                    if snapshot.layout.activity.focus_id.as_deref() == id
                        && driver.authored.borrow().activity.focus_id.as_deref() == id
                    {
                        return Ok(());
                    }
                    driver.svg_generation()
                } else {
                    let initial = initial.borrow();
                    let state = initial
                        .as_ref()
                        .ok_or_else(|| ChartError("Tree SVG owner is unavailable".into()))?;
                    if let Some(id) = id {
                        scene::ensure(
                            state.ids.contains(id),
                            "Tree SVG initial current-node action references an unknown source",
                        )?;
                    }
                    if state.focus.as_deref() == id {
                        return Ok(());
                    }
                    0
                };
                if let Some(driver) = &owner {
                    driver.request()?;
                }
                *queue.borrow_mut() = Some(CurrentCommand {
                    key: key.clone(),
                    id: id.map(str::to_owned),
                    source_generation: generation,
                    live: live.clone(),
                });
                Ok(())
            })
        });
        Ok(Self {
            program,
            builder,
            binding,
            bootstrap,
            commands,
            current_factory,
        })
    }
    pub(super) fn build(&mut self, request: &Request) -> Result<TreeActivityLayout> {
        let context = svg_context(request, &self.program);
        let layout = (self.builder)(&context)?;
        *self.bootstrap.borrow_mut() = Some(Bootstrap {
            ids: layout
                .layout
                .source()
                .iter()
                .map(|datum| datum.id.clone())
                .collect(),
            focus: layout.activity.focus_id.clone(),
        });
        Ok(layout)
    }
}

pub(super) struct SvgController {
    accepted: RefCell<BrowserTreeSvgProgram>,
    desired: RefCell<BrowserTreeSvgProgram>,
    pending: Cell<bool>,
    control_only: Cell<bool>,
    attempt: RefCell<Option<BrowserTreeSvgProgram>>,
    builder: RefCell<Option<Box<SvgBuilder>>>,
    binding: Rc<RefCell<Weak<Driver>>>,
    bootstrap: Rc<RefCell<Option<Bootstrap>>>,
    commands: Rc<RefCell<Option<CurrentCommand>>>,
    pub current_factory: CurrentFactory,
    data: RefCell<Option<(u64, TreeSvgFrame)>>,
}
impl SvgController {
    pub(super) fn mounted(mount: SvgMount, data: TreeSvgFrame) -> Self {
        Self {
            accepted: RefCell::new(mount.program.clone()),
            desired: RefCell::new(mount.program),
            pending: Cell::new(false),
            control_only: Cell::new(false),
            attempt: RefCell::new(None),
            builder: RefCell::new(Some(mount.builder)),
            binding: mount.binding,
            bootstrap: mount.bootstrap,
            commands: mount.commands,
            current_factory: mount.current_factory,
            data: RefCell::new(Some((0, data))),
        }
    }
    pub(super) fn attach(&self, driver: &Rc<Driver>) {
        *self.binding.borrow_mut() = Rc::downgrade(driver);
        self.bootstrap.borrow_mut().take();
    }
    pub(super) fn has_pending(&self) -> bool {
        self.pending.get() || self.commands.borrow().is_some()
    }
    pub(super) fn stop(&self) {
        self.pending.set(false);
        self.control_only.set(false);
        self.commands.borrow_mut().take();
        let attempt = self.attempt.borrow_mut().take();
        let builder = self.builder.borrow_mut().take();
        let data = self.data.borrow_mut().take();
        let empty = BrowserTreeSvgProgram::new(vec![]).expect("empty SVG program is valid");
        let accepted = self.accepted.replace(empty.clone());
        let desired = self.desired.replace(empty);
        *self.binding.borrow_mut() = Weak::new();
        self.bootstrap.borrow_mut().take();
        drop((attempt, builder, data, accepted, desired));
    }
}

fn svg_context<'a>(
    request: &'a Request,
    program: &'a BrowserTreeSvgProgram,
) -> BrowserTreeSvgBuildContext<'a> {
    BrowserTreeSvgBuildContext {
        config: &request.config,
        source: &request.source,
        tree: &request.tree,
        activity: &request.activity,
        layers: program.layers(),
        labels_required: TreeSvgPlan::labels_required(program.layers()),
    }
}

pub(super) fn source_roles<'a>(layers: impl Iterator<Item = &'a TreeSvgLayer>) -> [bool; 3] {
    let mut roles = [false; 3];
    for layer in layers {
        match layer.kind {
            TreeSvgLayerKind::Nodes => roles[0] = true,
            TreeSvgLayerKind::Links => roles[1] = true,
            TreeSvgLayerKind::Labels => roles[2] = true,
            _ => {}
        }
    }
    roles
}
fn default_svg_layout(context: &BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> {
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
pub(super) fn admit_svg_policy(
    options: &BrowserOptions,
    widgets: Option<&BrowserTreeWidgetOptions>,
) -> Result<()> {
    scene::ensure(
        options.renderer == BrowserRenderer::Svg,
        "Tree SVG layers require the SVG renderer",
    )?;
    scene::ensure(
        options.highlight == BrowserHighlight::Controlled
            && options.crosshair.is_none()
            && options.editable_brush.is_none(),
        "Tree SVG layers require Controlled paint without crosshair or editable brush",
    )?;
    scene::ensure(
        options.motion != MotionPreference::Animate,
        "Tree SVG layers do not provide automatic scene animation",
    )?;
    super::super::tree_svg_frame::policy(options)?;
    scene::ensure(
        widgets.is_none_or(|widgets| widgets.source_ordered),
        "Tree SVG layer components require source-ordered composition",
    )
}

impl BrowserTree {
    pub fn mount_with_svg_layers(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            None,
            None,
            Some(SvgMount::new(program, Box::new(default_svg_layout))?),
        )
    }
    pub fn mount_with_factory_and_svg_layers(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: impl FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            None,
            None,
            Some(SvgMount::new(program, Box::new(build))?),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_and_svg_layers_and_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeSvgProgram,
        build: impl FnMut(&BrowserTreeSvgBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            Some(ComponentMount::components(widgets, placement, factory)),
            None,
            Some(SvgMount::new(program, Box::new(build))?),
        )
    }
}

impl BrowserTreeHandle {
    pub fn set_svg_program(&self, program: BrowserTreeSvgProgram) -> Result<()> {
        program.validate_for_build()?;
        let driver = self.driver()?;
        let svg = driver
            .svg
            .as_ref()
            .ok_or_else(|| ChartError("Tree was mounted without SVG function layers".into()))?;
        let data = driver.svg_data(&driver.accepted.borrow())?;
        program.plan(&data)?;
        driver.request()?;
        let retired = svg.desired.replace(program);
        svg.pending.set(true);
        drop(retired);
        Ok(())
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserTreeSvgLayerFrame>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG owner dropped".into()))?;
        if !driver.alive.get() {
            return Ok(None);
        }
        Ok(driver.core()?.tree_svg_layer_frame(key))
    }
}

impl Driver {
    pub(super) fn svg_native_enabled(&self) -> bool {
        self.svg
            .as_ref()
            .is_none_or(|svg| svg.accepted.borrow().is_interactive())
    }
    pub(super) fn svg_pointer_intent(&self, event: &Event, client: [f64; 2]) -> PointerIntent {
        let Some(stage) = self
            .core
            .upgrade()
            .and_then(|core| core.tree_svg_stage())
            .filter(|stage| stage.was_adopted())
        else {
            return PointerIntent::Leave;
        };
        if !self.event_within_plot(event) {
            return PointerIntent::Leave;
        }
        // Native capture retargets events to the Surface, so their path no
        // longer contains the SVG occurrence that actually began the gesture.
        // Inherit only this owner's held primary source gesture. Widget
        // gestures and uncaptured input still follow their real DOM lineage.
        if let Some(pointer) = event.dyn_ref::<PointerEvent>()
            && let Some(token) = self.press.get().token()
            && self.press.get().held()
            && !token.widget
            && token.id == pointer.pointer_id()
            && let Some(core) = self.core.upgrade()
            && core.captured.get() == Some(token.id)
            && event.target().is_some_and(|target| {
                target == self.surface.clone().unchecked_into::<EventTarget>()
            })
        {
            let mesh = core.tree_svg_mesh_contacts.borrow().contains(&token.id);
            return PointerIntent::SvgMove(client, event.is_trusted(), mesh);
        }
        let root = stage.root();
        let elements = event
            .composed_path()
            .iter()
            .filter_map(|node| node.dyn_into::<Element>().ok())
            .collect::<Vec<_>>();
        if !elements.iter().any(|node| node == &root)
            || elements.iter().any(|node| {
                has_class(node, "kartis-tree-svg-custom-host")
                    || has_class(node, "kartis-plot-widget-slot")
            })
        {
            return PointerIntent::Leave;
        }
        let interactive = self
            .svg
            .as_ref()
            .is_some_and(|svg| svg.accepted.borrow().is_interactive());
        if !interactive {
            return PointerIntent::Leave;
        }
        let mesh = elements
            .iter()
            .any(|node| node.has_attribute("data-tree-svg-mesh"));
        PointerIntent::SvgMove(client, event.is_trusted(), mesh)
    }

    pub(super) fn svg_native_node(&self, client: [f64; 2], mesh: bool) -> Result<Option<String>> {
        if mesh {
            return match self.picking {
                BrowserTreePicking::Nearest { distance } => self.nearest(client, distance),
                BrowserTreePicking::Geometry => Ok(None),
            };
        }
        let rect = self.surface.get_bounding_client_rect();
        if rect.width() <= 0. || rect.height() <= 0. {
            return Ok(None);
        }
        let model = self.model()?;
        let point = (
            (client[0] - rect.x()) / rect.width() * model.scene().width,
            (client[1] - rect.y()) / rect.height() * model.scene().height,
        );
        let Some(index) = model.source_paint_target(point)? else {
            return Ok(None);
        };
        let accepted = self.accepted.borrow();
        Ok(accepted
            .layout
            .node_id_from_mark(&model.scene().marks[index].id)
            .map(str::to_owned))
    }

    fn svg_generation(&self) -> u64 {
        self.publishing
            .borrow()
            .as_ref()
            .filter(|publication| publication.was_adopted())
            .map_or_else(
                || self.accepted.borrow().generation,
                |publication| publication.candidate.generation,
            )
    }
    pub(super) fn svg_data(&self, candidate: &Accepted) -> Result<TreeSvgFrame> {
        let svg = self
            .svg
            .as_ref()
            .ok_or_else(|| ChartError("Tree SVG controller missing".into()))?;
        if let Some((generation, data)) = svg.data.borrow().as_ref()
            && *generation == candidate.generation
        {
            return Ok(data.clone());
        }
        let data = TreeSvgFrame::with_activity(
            candidate.request.config.clone(),
            candidate.layout.clone(),
        )?;
        *svg.data.borrow_mut() = Some((candidate.generation, data.clone()));
        Ok(data)
    }
    pub(super) fn svg_pending(&self) -> bool {
        self.svg.as_ref().is_some_and(SvgController::has_pending)
    }
    pub(super) fn svg_program_pending(&self) -> bool {
        self.svg.as_ref().is_some_and(|svg| svg.pending.get())
    }
    pub(super) fn svg_retry_explicit(&self, explicit: bool) -> bool {
        self.svg
            .as_ref()
            .map_or(explicit, |svg| explicit && !svg.control_only.replace(false))
    }
    pub(super) fn clear_svg_control_only(&self) {
        if let Some(svg) = &self.svg {
            svg.control_only.set(false);
        }
    }
    pub(super) fn svg_label_gate_changed(&self, program: &BrowserTreeSvgProgram) -> bool {
        self.svg.as_ref().is_some_and(|svg| {
            TreeSvgPlan::labels_required(program.layers())
                != TreeSvgPlan::labels_required(svg.accepted.borrow().layers())
        })
    }
    pub(super) fn svg_program_changed(&self, program: &BrowserTreeSvgProgram) -> bool {
        self.svg
            .as_ref()
            .is_some_and(|svg| !svg.accepted.borrow().same_program(program))
    }
    pub(super) fn take_svg_program(&self, explicit: bool) -> Option<BrowserTreeSvgProgram> {
        let svg = self.svg.as_ref()?;
        let pending = svg.pending.replace(false);
        let program = if pending || explicit {
            svg.desired.borrow().clone()
        } else {
            svg.accepted.borrow().clone()
        };
        *svg.attempt.borrow_mut() = Some(program.clone());
        Some(program)
    }
    pub(super) fn build_svg(&self, request: &Request) -> Result<TreeActivityLayout> {
        let svg = self
            .svg
            .as_ref()
            .ok_or_else(|| ChartError("Tree SVG builder missing".into()))?;
        let program = svg
            .attempt
            .borrow()
            .clone()
            .unwrap_or_else(|| svg.accepted.borrow().clone());
        program.validate_for_build()?;
        let mut build = svg
            .builder
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("Tree SVG builder is already in use".into()))?;
        self.building.set(true);
        let result = build(&svg_context(request, &program));
        self.building.set(false);
        if self.alive.get() {
            *svg.builder.borrow_mut() = Some(build);
        }
        scene::ensure(self.alive.get(), "Tree stopped during SVG source build")?;
        let layout = result?;
        validate_candidate(request, &layout)?;
        Ok(layout)
    }
    pub(super) fn consume_svg_current(&self) -> Result<Option<Request>> {
        let Some(svg) = &self.svg else {
            return Ok(None);
        };
        let Some(command) = svg.commands.borrow_mut().take() else {
            return Ok(None);
        };
        if !command.live.upgrade().is_some_and(|live| live.get())
            || command.source_generation != self.svg_generation()
        {
            self.record(ChartError(format!(
                "Tree SVG current-node action for {} was retired",
                command.key
            )));
            return Ok(None);
        }
        let accepted_current = self.motion.is_some() && !self.dirty.get();
        let mut request = if accepted_current {
            self.accepted.borrow().request.clone()
        } else {
            self.authored.borrow().clone()
        };
        if command
            .id
            .as_ref()
            .is_some_and(|id| !request.source.iter().any(|datum| &datum.id == id))
        {
            self.record(ChartError(
                "Tree SVG current-node action conflicts with pending source".into(),
            ));
            return Ok(None);
        }
        if request.activity.focus_id != command.id {
            request.activity.focus_id = command.id;
            validate_request(&request)?;
            svg.control_only
                .set(!self.dirty.get() && !self.canvas_dirty.get() && !svg.pending.get());
            self.dirty.set(true);
            if accepted_current {
                return Ok(Some(request));
            }
            *self.authored.borrow_mut() = request;
        }
        Ok(None)
    }
    pub(super) fn accept_svg_program(&self, program: BrowserTreeSvgProgram) {
        if let Some(svg) = &self.svg {
            let retired = svg.accepted.replace(program);
            svg.attempt.borrow_mut().take();
            drop(retired);
        }
    }
}
