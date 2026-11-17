//! An application-owned Tree driven by reserved native pointer intent.
//! Native listeners reserve input; a separate owner RAF builds and adopts it.
use std::sync::Arc;

use super::*;
use crate::widget::plot_widgets::composition::CompositionStage;

#[path = "tree_input.rs"]
mod input;
#[path = "tree_widgets.rs"]
mod widgets;
use input::{PressState, PressToken};
pub use widgets::BrowserTreeWidgetOptions;
use widgets::{ComponentMount, Components};
#[path = "tree_canvas_owner.rs"]
mod canvas_owner;
pub use canvas_owner::BrowserTreeCanvasProgram;
use canvas_owner::admit_canvas_policy;
#[path = "tree_tooltip.rs"]
mod tooltip;
use tooltip::{TooltipConfig, TooltipController};
#[path = "tree_events.rs"]
mod mouse_events;
use mouse_events::MouseController;
pub use mouse_events::*;
#[path = "tree_svg_owner.rs"]
mod svg_owner;
pub use svg_owner::{BrowserTreeSvgBuildContext, BrowserTreeSvgCurrentNode};
use svg_owner::{SvgController, SvgMount, admit_svg_policy};
#[path = "tree_motion_owner.rs"]
mod motion_owner;
pub use motion_owner::{BrowserTreeMotionClock, BrowserTreeMotionFrame};
use motion_owner::{MotionCandidate, MotionController, MotionMount};

use super::tree_tooltip_host::Stage as TooltipStage;

/// Mount-time chart policy and the authored Tree/activity configuration.
/// Default nearest-node picking has the finite logical radius 1,000,000.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTreeOptions {
    pub chart: BrowserOptions,
    pub tree: TreeOptions,
    pub activity: TreeActivityOptions,
    pub picking: BrowserTreePicking,
}
/// Activity picking is independent of the chart's strict painted picking.
/// Nearest distance is measured in logical viewport pixels before external
/// CSS scale; zoom divides this distance before querying world coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BrowserTreePicking {
    Geometry,
    Nearest { distance: f64 },
}
impl Default for BrowserTreePicking {
    fn default() -> Self {
        Self::Nearest {
            distance: 1_000_000.,
        }
    }
}
impl BrowserTreePicking {
    fn validate(self) -> Result<()> {
        scene::ensure(
            match self {
                Self::Geometry => true,
                Self::Nearest { distance } => {
                    distance.is_finite() && (0. ..=1_000_000.).contains(&distance)
                }
            },
            "invalid Tree nearest-node distance",
        )
    }
}
impl Default for BrowserTreeOptions {
    fn default() -> Self {
        Self {
            chart: BrowserOptions {
                legend: false,
                highlight: BrowserHighlight::Controlled,
                ..Default::default()
            },
            tree: TreeOptions::default(),
            activity: TreeActivityOptions::default(),
            picking: BrowserTreePicking::default(),
        }
    }
}
/// Inputs for one synchronous build. Factories created inside this callback are
/// borrowed by the pure layout builder and need not outlive this invocation.
#[derive(Clone, Copy, Debug)]
pub struct BrowserTreeBuildContext<'a> {
    pub config: &'a ChartConfig,
    pub source: &'a [HierarchyDatum],
    pub tree: &'a TreeOptions,
    pub activity: &'a TreeActivityOptions,
}
type Builder = dyn FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout>;
#[derive(Clone)]
struct Request {
    source: Arc<Vec<HierarchyDatum>>,
    config: ChartConfig,
    tree: TreeOptions,
    activity: TreeActivityOptions,
}
impl Request {
    fn context(&self) -> BrowserTreeBuildContext<'_> {
        BrowserTreeBuildContext {
            config: &self.config,
            source: &self.source,
            tree: &self.tree,
            activity: &self.activity,
        }
    }
}
fn default_layout(context: &BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> {
    tree_layout_with_activity(
        context.config,
        context.source,
        context.tree,
        &mut TreeFactories::default(),
        context.activity,
        &mut TreeActivityFactories::default(),
    )
}
fn validate_request(request: &Request) -> Result<()> {
    default_layout(&request.context()).map(|_| ())
}
fn validate_candidate(request: &Request, candidate: &TreeActivityLayout) -> Result<()> {
    tree_activity_frame(candidate)?;
    let original = candidate.layout.source();
    scene::ensure(
        original.len() == request.source.len()
            && original.iter().zip(request.source.iter()).all(|(a, b)| {
                a.id == b.id
                    && a.label == b.label
                    && a.parent == b.parent
                    && a.value.to_bits() == b.value.to_bits()
            }),
        "Tree builder changed its requested source",
    )?;
    scene::ensure(
        candidate.layout.options() == &request.tree
            && candidate.layout.plot == ClipRect::plot(&request.config)
            && candidate.layout.scene.width == request.config.width
            && candidate.layout.scene.height == request.config.height,
        "Tree builder changed its requested configuration",
    )?;
    scene::ensure(
        candidate.activity.focus_id == request.activity.focus_id
            && candidate.activity.highlight == request.activity.highlight
            && candidate.activity.fix_node_color_at_depth
                == request.activity.fix_node_color_at_depth,
        "Tree builder changed its requested activity",
    )
}
#[derive(Clone)]
struct Accepted {
    request: Request,
    layout: TreeActivityLayout,
    widget_revision: u64,
    generation: u64,
}
struct Publishing {
    candidate: Accepted,
    stage: Option<Rc<CompositionStage>>,
    tooltip: Rc<TooltipStage>,
    tooltip_config: TooltipConfig,
    svg: Option<Rc<super::tree_svg_frame::Stage>>,
    svg_program: Option<super::tree_svg_layers::BrowserTreeSvgProgram>,
    motion: Option<MotionCandidate>,
}
impl Publishing {
    fn was_adopted(&self) -> bool {
        self.stage.as_ref().is_none_or(|s| s.was_adopted())
            && self.svg.as_ref().is_none_or(|s| s.was_adopted())
            && self.tooltip.was_adopted()
    }
}
/// The accepted activity/layout is preserved on a failed build or paint.
/// `desired_focus` can differ after a rejected candidate; `refresh` retries it.
#[derive(Clone, Debug)]
pub struct BrowserTreeSnapshot {
    pub active: bool,
    pub chart: Option<BrowserSnapshot>,
    pub layout: TreeActivityLayout,
    pub config: ChartConfig,
    pub widget_revision: u64,
    pub desired_focus: Option<String>,
    pub pending: bool,
    pub raf_pending: bool,
    pub error: Option<ChartError>,
    pub picking: BrowserTreePicking,
}
type UpdateCallback = Box<dyn FnMut(BrowserTreeSnapshot)>;
#[derive(Clone, Copy)]
enum PointerIntent {
    Move([f64; 2], bool),
    // The last bit records an actual owned Mesh rectangle target, rather than
    // inferring nearest-node coverage from mount-time picking configuration.
    SvgMove([f64; 2], bool, bool),
    MouseNode {
        client: [f64; 2],
        trusted: bool,
        index: Option<usize>,
        generation: u64,
        set_current: bool,
    },
    Leave,
}
struct NativeListener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for NativeListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.callback.as_ref().unchecked_ref(),
            true,
        );
    }
}
struct CachedMesh {
    hidden: BTreeSet<String>,
    mesh: crate::mesh::PointMesh,
}
struct ReleaseTicket {
    handle: i32,
    token: PressToken,
    _callback: Closure<dyn FnMut()>,
}
struct Driver {
    core: Weak<Engine>,
    window: Window,
    surface: HtmlElement,
    input_root: Element,
    overlay: Option<Element>,
    picking: BrowserTreePicking,
    intent: Cell<Option<PointerIntent>>,
    press: Cell<PressState>,
    release: RefCell<Option<ReleaseTicket>>,
    mesh: RefCell<Option<CachedMesh>>,
    mouse: MouseController,
    listeners: RefCell<Vec<NativeListener>>,
    accepted: RefCell<Accepted>,
    authored: RefCell<Request>,
    desired_request_pending: Cell<bool>,
    builder: RefCell<Option<Box<Builder>>>,
    components: Option<Components>,
    canvas: Option<RefCell<Option<BrowserTreeCanvasProgram>>>,
    svg: Option<SvgController>,
    motion: Option<MotionController>,
    motion_revision_dirty: Cell<bool>,
    publishing: RefCell<Option<Publishing>>,
    alive: Cell<bool>,
    processing: Cell<bool>,
    building: Cell<bool>,
    painting: Cell<bool>,
    dirty: Cell<bool>,
    canvas_dirty: Cell<bool>,
    tooltip: RefCell<TooltipController>,
    tooltip_dirty: Cell<bool>,
    link_tooltip_dirty: Cell<bool>,
    next_generation: Cell<u64>,
    desired_revision: Cell<u64>,
    ticket: Cell<Option<i32>>,
    raf: RefCell<Option<RafCallback>>,
    events: RefCell<VecDeque<BrowserEvent>>,
    event_callback: RefCell<Option<EventCallback>>,
    event_generation: Cell<u64>,
    update_callback: RefCell<Option<UpdateCallback>>,
    update_generation: Cell<u64>,
    error: RefCell<Option<ChartError>>,
}
#[must_use = "retain BrowserTree; dropping it stops its RAF and chart"]
pub struct BrowserTree {
    _chart: BrowserChart,
    driver: Rc<Driver>,
}
#[derive(Clone)]
pub struct BrowserTreeHandle {
    driver: Weak<Driver>,
}
impl BrowserTree {
    pub fn mount(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            None,
        )
    }
    pub fn mount_with_factory(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(root, source, config, options, Box::new(build), None)
    }
    /// Retain typed components in the same strict candidate as activity/paint.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_widgets<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<TreeActivityWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            Some(ComponentMount::new(widgets, placement, factory)),
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_and_widgets<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<TreeActivityWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            source,
            config,
            options,
            Box::new(build),
            Some(ComponentMount::new(widgets, placement, factory)),
        )
    }
    /// Retain node, link and real label components in one accepted owner
    /// packet. Label AtDatum placements rotate their measured local subtree;
    /// selected source ink is replaced only after strict child preparation.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            source,
            config,
            options,
            Box::new(default_layout),
            Some(ComponentMount::components(widgets, placement, factory)),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_and_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            source,
            config,
            options,
            Box::new(build),
            Some(ComponentMount::components(widgets, placement, factory)),
        )
    }

    fn mount_inner(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: Box<Builder>,
        widgets: Option<ComponentMount>,
    ) -> Result<Self> {
        Self::mount_program(root, source, config, options, build, widgets, None, None)
    }
    #[allow(clippy::too_many_arguments)]
    fn mount_program(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: Box<Builder>,
        widgets: Option<ComponentMount>,
        canvas: Option<BrowserTreeCanvasProgram>,
        svg: Option<SvgMount>,
    ) -> Result<Self> {
        Self::mount_program_motion(
            root, source, config, options, build, widgets, canvas, svg, None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn mount_program_motion(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        mut build: Box<Builder>,
        mut widgets: Option<ComponentMount>,
        mut canvas: Option<BrowserTreeCanvasProgram>,
        mut svg: Option<SvgMount>,
        mut motion: Option<MotionMount>,
    ) -> Result<Self> {
        scene::ensure(
            canvas.is_none() || svg.is_none(),
            "Tree Canvas and SVG programs are exclusive",
        )?;
        if canvas.is_some() || svg.is_some() {
            if canvas.is_some() {
                admit_canvas_policy(&options.chart, widgets.as_ref().map(|w| &w.runtime.options))?;
            } else {
                if motion.is_some() {
                    motion_owner::admit(&options)?;
                } else {
                    admit_svg_policy(&options.chart, widgets.as_ref().map(|w| &w.runtime.options))?;
                }
            }
            scene::ensure(
                !root.has_child_nodes()
                    && !root.has_attribute("data-gly-app")
                    && !root.has_attribute("data-kartis-browser"),
                "browser chart needs an empty unclaimed root",
            )?;
        }
        let BrowserTreeOptions {
            chart: mut chart_options,
            tree,
            activity,
            picking,
        } = options;
        picking.validate()?;
        scene::ensure(
            chart_options.interaction_data.is_none(),
            "Tree owner supplies its own interaction binding",
        )?;
        scene::ensure(
            chart_options.interaction.view_mode == surface::SurfaceViewMode::Transform,
            "Tree owner requires Scene-transform interaction",
        )?;
        scene::ensure(
            chart_options.interaction.picking == surface::Picking::Geometry,
            "Tree chart requires strict Geometry picking; configure activity through BrowserTreeOptions.picking",
        )?;
        let request = Request {
            source: Arc::new(source),
            config,
            tree,
            activity,
        };
        validate_request(&request)?;
        scene::ensure(
            motion.is_some() || svg.as_ref().is_none_or(|s| !s.program.requires_motion()),
            "animated SVG layers require Tree motion capability",
        )?;
        let initial = match svg.as_mut() {
            Some(svg) => svg.build(&request)?,
            None => build(&request.context())?,
        };
        validate_candidate(&request, &initial)?;
        let frame = tree_activity_frame(&initial)?;
        chart_options.interaction_data = frame.interaction.clone();
        chart_options.interaction.plot = frame.plot;
        let mut initial_packet = if let Some(svg) = &svg {
            widgets
                .as_ref()
                .map(|widgets| {
                    widgets.runtime.packet_for_svg(
                        &initial,
                        widgets.runtime.options.revision,
                        svg.program.layers(),
                    )
                })
                .transpose()?
        } else if canvas.is_some() {
            widgets
                .as_ref()
                .map(|w| w.runtime.packet(&initial, w.runtime.options.revision))
                .transpose()?
        } else {
            None
        };
        let raw = canvas
            .as_mut()
            .map(|program| {
                let domain = TreeCanvasFrame::from_all_computed_activity(
                    request.config.clone(),
                    initial.clone(),
                )?;
                let window = web_sys::window()
                    .ok_or_else(|| ChartError("browser window unavailable".into()))?;
                let document = root
                    .owner_document()
                    .ok_or_else(|| ChartError("Tree root has no document".into()))?;
                let mut model = SurfaceModel::new_with_interaction(
                    frame.scene.clone(),
                    chart_options.interaction.clone(),
                    chart_options.interaction_data.clone(),
                )?;
                if let Some(brush) = chart_options.editable_brush {
                    model.configure_editable_brush(Some(brush), BrushReconfigure::Clamp)?;
                }
                if let Some(packet) = &initial_packet {
                    super::plot_widgets::prepare(&model, packet, true)?;
                }
                let options = TreeCanvasOptions {
                    pixel_ratio: blends::ratio(window.device_pixel_ratio(), &frame.scene),
                    view: model.paint_transform(),
                    ..Default::default()
                };
                program.prepare(&document, &domain, options, &mut || Ok(()))
            })
            .transpose()?;
        let revision = motion
            .as_ref()
            .and_then(|m| m.widgets.as_ref())
            .map_or_else(
                || widgets.as_ref().map_or(0, |w| w.runtime.options.revision),
                |(options, _)| options.revision,
            );
        let initial_motion = if let Some(motion) = motion.as_mut() {
            let model = TreeMotion::new(
                TreeEventFrame::with_activity(request.config.clone(), initial.clone())?,
                motion.options.clone(),
            )?;
            let paint = (motion.renderer)(model.frame())?;
            scene::ensure(
                paint.motion() == model.frame(),
                "Tree motion renderer returned another numeric frame",
            )?;
            Some(MotionCandidate {
                model,
                paint,
                options: Some(motion.options.clone()),
                configuration: 0,
                time: None,
            })
        } else {
            None
        };
        let svg_data = svg
            .as_ref()
            .map(|_| TreeSvgFrame::with_activity(request.config.clone(), initial.clone()))
            .transpose()?;
        let chart = if let Some(svg) = &svg {
            let plan = svg
                .program
                .plan(svg_data.as_ref().expect("SVG initial data"))?;
            let components = widgets
                .as_mut()
                .map(|widgets| {
                    widgets.take_svg_mount(initial_packet.take().expect("SVG source packet"))
                })
                .transpose()?;
            if motion.as_ref().is_some_and(|m| m.ordinary) {
                motion_owner::ordinary_order(&plan)?;
            }
            if let Some(initial_motion) = &initial_motion {
                let mount = motion.as_mut().expect("motion mount");
                BrowserChart::mount_tree_motion_svg(
                    root,
                    frame,
                    chart_options,
                    plan,
                    svg.program.clone(),
                    picking,
                    0,
                    0,
                    svg.current_factory.clone(),
                    initial_motion.paint.clone(),
                    BrowserTreeMotionFrame {
                        data: initial_motion.model.frame().clone(),
                        source_generation: 0,
                        publication: 0,
                        configuration: 0,
                    },
                    mount.widgets.take(),
                )?
            } else {
                BrowserChart::mount_tree_svg(
                    root,
                    frame,
                    chart_options,
                    plan,
                    svg.program.clone(),
                    picking,
                    components,
                    0,
                    0,
                    svg.current_factory.clone(),
                )?
            }
        } else {
            match (widgets.as_mut(), raw) {
                (Some(widgets), Some(raw)) => widgets.mount_canvas(
                    root,
                    &initial,
                    chart_options,
                    raw,
                    initial_packet.expect("preflight component packet"),
                )?,
                (None, Some(raw)) => {
                    BrowserChart::mount_tree_canvas(root, frame.scene, chart_options, raw, None)?
                }
                (Some(widgets), None) => widgets.mount(root, &initial, chart_options)?,
                (None, None) => {
                    BrowserChart::mount_tree_scene(root, frame.scene, chart_options, None)?
                }
            }
        };
        let surface = chart
            .engine
            .state
            .borrow()
            .as_ref()
            .expect("mounted Tree state")
            .surface
            .clone();
        // Overlay Widgets are siblings of the Surface. Capture the actual
        // mounted chart's plot and its direct Widget layer, never a global
        // selector that could claim another chart's controls or nested chart.
        let parent = surface
            .parent_element()
            .ok_or_else(|| ChartError("Tree Surface has no input parent".into()))?;
        let input_root = if has_class(&parent, "kartis-browser-plot") {
            parent
        } else {
            surface.clone().into()
        };
        let mut overlay = None;
        let mut child = input_root.first_element_child();
        while let Some(element) = child {
            child = element.next_element_sibling();
            if has_class(&element, "kartis-browser-plot-widgets") {
                overlay = Some(element);
                break;
            }
        }
        let driver = Rc::new(Driver {
            core: Rc::downgrade(&chart.engine),
            window: chart.engine.window.clone(),
            surface,
            input_root,
            overlay,
            picking,
            intent: Cell::new(None),
            press: Cell::new(PressState::default()),
            release: RefCell::new(None),
            mesh: RefCell::new(None),
            mouse: MouseController::default(),
            listeners: RefCell::new(vec![]),
            accepted: RefCell::new(Accepted {
                request: request.clone(),
                layout: initial,
                widget_revision: revision,
                generation: 0,
            }),
            authored: RefCell::new(request),
            desired_request_pending: Cell::new(false),
            builder: RefCell::new(Some(build)),
            components: widgets.map(|w| w.runtime),
            canvas: canvas.map(|program| RefCell::new(Some(program))),
            svg: svg
                .map(|mount| SvgController::mounted(mount, svg_data.expect("SVG initial data"))),
            motion: motion.map(|mount| {
                MotionController::mounted(mount, initial_motion.expect("motion initial"))
            }),
            motion_revision_dirty: Cell::new(false),
            publishing: RefCell::new(None),
            alive: Cell::new(true),
            processing: Cell::new(false),
            building: Cell::new(false),
            painting: Cell::new(false),
            dirty: Cell::new(false),
            canvas_dirty: Cell::new(false),
            tooltip: RefCell::new(TooltipController::default()),
            tooltip_dirty: Cell::new(false),
            link_tooltip_dirty: Cell::new(false),
            next_generation: Cell::new(0),
            desired_revision: Cell::new(revision),
            ticket: Cell::new(None),
            raf: RefCell::new(None),
            events: RefCell::new(VecDeque::new()),
            event_callback: RefCell::new(None),
            event_generation: Cell::new(0),
            update_callback: RefCell::new(None),
            update_generation: Cell::new(0),
            error: RefCell::new(None),
        });
        if let Some(svg) = &driver.svg {
            svg.attach(&driver);
        }
        if driver.canvas.is_some() {
            let weak = Rc::downgrade(&driver);
            chart
                .engine
                .set_tree_canvas_repaint(Rc::new(move |frame, options| {
                    weak.upgrade()
                        .ok_or_else(|| ChartError("Tree Canvas owner was dropped".into()))?
                        .repaint_canvas(frame, options)
                }));
        }
        let weak = Rc::downgrade(&driver);
        *driver.raf.borrow_mut() = Some(Closure::new(move |time: f64| {
            if let Some(driver) = weak.upgrade() {
                driver.ticket.set(None);
                if let Err(error) = driver.frame_at(Some(time)) {
                    driver.record(error);
                    driver.emit();
                }
            }
        }));
        let weak = Rc::downgrade(&driver);
        chart.on_event(move |event| {
            if let Some(driver) = weak.upgrade()
                && driver.alive.get()
            {
                driver.event(event);
            }
        });
        driver.listen()?;
        driver.listen_motion()?;
        let [width, height] = driver.logical_size()?;
        if width > 0. && height > 0. {
            let mut authored = driver.authored.borrow_mut();
            if authored.config.width != width || authored.config.height != height {
                authored.config.width = width;
                authored.config.height = height;
                driver.dirty.set(true);
                driver.request()?;
            }
        }
        if driver.svg_pending() {
            driver.request()?;
        }
        Ok(Self {
            _chart: chart,
            driver,
        })
    }
    pub fn handle(&self) -> BrowserTreeHandle {
        BrowserTreeHandle {
            driver: Rc::downgrade(&self.driver),
        }
    }
    pub fn snapshot(&self) -> Result<BrowserTreeSnapshot> {
        self.driver.snapshot()
    }
    /// The actual mounted chart highlighting policy.
    pub fn highlight(&self) -> Result<BrowserHighlight> {
        self.handle().highlight()
    }
    pub fn flush(&self) -> Result<()> {
        self.handle().flush()
    }
    pub fn refresh(&self) -> Result<()> {
        self.handle().refresh()
    }
    pub fn clear_current(&self) -> Result<()> {
        self.handle().clear_current()
    }
    pub fn stop(&self) {
        self.driver.stop();
    }
    pub fn on_event(&self, callback: impl FnMut(BrowserEvent) + 'static) {
        self.handle().on_event(callback);
    }
    pub fn on_update(&self, callback: impl FnMut(BrowserTreeSnapshot) + 'static) {
        self.handle().on_update(callback);
    }
}
impl Drop for BrowserTree {
    fn drop(&mut self) {
        self.driver.stop();
    }
}

impl BrowserTreeHandle {
    fn driver(&self) -> Result<Rc<Driver>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        scene::ensure(
            driver.alive.get() && driver.core.upgrade().is_some_and(|e| e.alive.get()),
            "Tree owner is stopped",
        )?;
        scene::ensure(
            !driver.building.get() && !driver.painting.get(),
            "Tree candidate callbacks cannot mutate their owner",
        )?;
        Ok(driver)
    }
    pub fn is_active(&self) -> bool {
        self.driver
            .upgrade()
            .is_some_and(|d| d.alive.get() && d.core.upgrade().is_some_and(|e| e.alive.get()))
    }
    pub fn snapshot(&self) -> Result<BrowserTreeSnapshot> {
        self.driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?
            .snapshot()
    }
    /// Read the actual chart policy, rather than the caller's requested value.
    pub fn highlight(&self) -> Result<BrowserHighlight> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        let core = driver.core()?;
        let state = core
            .state
            .try_borrow()
            .map_err(|_| ChartError("Tree browser frame is already in use".into()))?;
        state
            .as_ref()
            .map(|s| s.options.highlight)
            .ok_or_else(|| ChartError("Tree chart is stopped".into()))
    }
    pub fn stop(&self) {
        if let Some(driver) = self.driver.upgrade() {
            driver.stop();
        }
    }
    pub fn on_event(&self, callback: impl FnMut(BrowserEvent) + 'static) {
        if let Ok(driver) = self.driver() {
            driver
                .event_generation
                .set(driver.event_generation.get().wrapping_add(1));
            *driver.event_callback.borrow_mut() = Some(Box::new(callback));
        }
    }
    pub fn on_update(&self, callback: impl FnMut(BrowserTreeSnapshot) + 'static) {
        if let Ok(driver) = self.driver() {
            driver
                .update_generation
                .set(driver.update_generation.get().wrapping_add(1));
            *driver.update_callback.borrow_mut() = Some(Box::new(callback));
        }
    }
    /// Flush pending native input and one coalesced Tree candidate. An idle
    /// flush does not invoke layout factories or repaint the accepted chart.
    pub fn flush(&self) -> Result<()> {
        let driver = self.driver()?;
        if driver.processing.get() {
            return Ok(());
        }
        if let Some(ticket) = driver.ticket.take() {
            driver.window.cancel_animation_frame(ticket).map_err(host)?;
        }
        match driver.frame() {
            Ok(()) => Ok(()),
            Err(error) => {
                driver.record(error.clone());
                driver.emit();
                Err(error)
            }
        }
    }
    /// Reevaluate retained functions explicitly, including after a rejection.
    pub fn refresh(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        driver.clear_svg_control_only();
        driver.dirty.set(true);
        driver.desired_request_pending.set(true);
        Ok(())
    }
    pub fn clear_current(&self) -> Result<()> {
        let driver = self.driver()?;
        let mut next = driver.authored.borrow().clone();
        next.activity.focus_id = None;
        driver.replace(next)
    }
    /// Unknown current source IDs reject before any retained function runs.
    /// Clear current explicitly before removing its source node.
    pub fn set_source(&self, source: Vec<HierarchyDatum>) -> Result<()> {
        let driver = self.driver()?;
        let mut next = driver.authored.borrow().clone();
        next.source = Arc::new(source);
        driver.replace(next)
    }
    pub fn set_config(&self, config: ChartConfig) -> Result<()> {
        let driver = self.driver()?;
        let mut next = driver.authored.borrow().clone();
        next.config = config;
        driver.replace(next)
    }
    pub fn set_tree_options(&self, options: TreeOptions) -> Result<()> {
        let driver = self.driver()?;
        let mut next = driver.authored.borrow().clone();
        next.tree = options;
        driver.replace(next)
    }
    pub fn set_activity_options(&self, options: TreeActivityOptions) -> Result<()> {
        let driver = self.driver()?;
        let mut next = driver.authored.borrow().clone();
        next.activity = options;
        driver.replace(next)
    }
    pub fn set_widget_revision(&self, revision: u64) -> Result<()> {
        let driver = self.driver()?;
        scene::ensure(
            driver.components.is_some() || driver.motion.as_ref().is_some_and(|m| m.widgets),
            "Tree was mounted without components",
        )?;
        driver.request()?;
        driver.desired_revision.set(revision);
        if driver.motion.is_some() {
            driver.motion_revision_dirty.set(true);
        } else {
            driver.dirty.set(true);
        }
        Ok(())
    }
    /// Dispatch to the chart's strict painted interaction. Nearest activity is
    /// reserved by actual native pointer movement, not by this chart-only API.
    pub fn dispatch(&self, input: SurfaceInput) -> Result<()> {
        let driver = self.driver()?;
        driver.chart()?.dispatch(input)?;
        driver.request()
    }
    pub fn reset_view(&self) -> Result<()> {
        self.dispatch(SurfaceInput::Reset)
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<()> {
        let driver = self.driver()?;
        driver.chart()?.set_hidden(hidden)?;
        driver.request()
    }
}

impl Driver {
    fn core(&self) -> Result<Rc<Engine>> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| ChartError("Tree chart was dropped".into()))?;
        scene::ensure(core.alive.get(), "Tree chart is stopped")?;
        Ok(core)
    }
    fn chart(&self) -> Result<BrowserHandle> {
        self.core()?;
        Ok(BrowserHandle {
            engine: self.core.clone(),
        })
    }
    fn model(&self) -> Result<SurfaceModel> {
        let core = self.core()?;
        let state = core
            .state
            .try_borrow()
            .map_err(|_| ChartError("Tree browser frame is already in use".into()))?;
        state
            .as_ref()
            .map(|s| s.model.clone())
            .ok_or_else(|| ChartError("Tree chart is stopped".into()))
    }
    fn logical_size(&self) -> Result<[f64; 2]> {
        let core = self.core()?;
        let surface = core
            .state
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("Tree chart is stopped".into()))?
            .surface
            .clone();
        let style = self
            .window
            .get_computed_style(&surface)
            .map_err(host)?
            .ok_or_else(|| ChartError("Tree surface computed style is missing".into()))?;
        let dimension = |name| -> Result<f64> {
            let text = style.get_property_value(name).map_err(host)?;
            let n = text
                .trim()
                .strip_suffix("px")
                .and_then(|s| s.trim().parse::<f64>().ok())
                .ok_or_else(|| {
                    ChartError("Tree surface requires resolved CSS pixel dimensions".into())
                })?;
            scene::ensure(
                n.is_finite() && (0. ..=1_000_000.).contains(&n),
                "invalid Tree logical surface dimensions",
            )?;
            Ok(n)
        };
        Ok([dimension("width")?, dimension("height")?])
    }
    fn request(&self) -> Result<()> {
        scene::ensure(self.alive.get(), "Tree owner is stopped")?;
        if self.ticket.get().is_none() {
            let raf = self.raf.borrow();
            let raf = raf
                .as_ref()
                .ok_or_else(|| ChartError("Tree RAF is unavailable".into()))?;
            self.ticket.set(Some(
                self.window
                    .request_animation_frame(raf.as_ref().unchecked_ref())
                    .map_err(host)?,
            ));
        }
        Ok(())
    }
    fn replace(&self, next: Request) -> Result<()> {
        validate_request(&next)?;
        self.request()?;
        self.clear_svg_control_only();
        *self.authored.borrow_mut() = next;
        self.desired_request_pending.set(true);
        self.dirty.set(true);
        Ok(())
    }
    fn event(&self, event: BrowserEvent) {
        if !self.painting.get()
            && let BrowserEvent::Resized { width, height } = &event
            && *width > 0.
            && *height > 0.
        {
            match self.logical_size() {
                Ok([width, height]) if width > 0. && height > 0. => {
                    let mut request = self.authored.borrow_mut();
                    if request.config.width != width || request.config.height != height {
                        request.config.width = width;
                        request.config.height = height;
                        self.dirty.set(true);
                    }
                }
                Ok(_) => {}
                Err(error) => self.record(error),
            }
        }
        if let BrowserEvent::Error(error) = &event {
            *self.error.borrow_mut() = Some(error.clone());
        }
        if self.events.borrow().len() < 512 {
            self.events.borrow_mut().push_back(event);
        } else {
            *self.error.borrow_mut() =
                Some(ChartError("Tree event queue exceeded 512 events".into()));
        }
        if let Err(error) = self.request() {
            self.record(error);
        }
    }
    fn record(&self, error: ChartError) {
        *self.error.borrow_mut() = Some(error.clone());
        if self.events.borrow().len() < 512 {
            self.events
                .borrow_mut()
                .push_back(BrowserEvent::Error(error));
        }
    }
    fn emit(&self) {
        while self.alive.get() {
            let Some(event) = self.events.borrow_mut().pop_front() else {
                break;
            };
            let generation = self.event_generation.get();
            let callback = self.event_callback.borrow_mut().take();
            if let Some(mut callback) = callback {
                callback(event);
                if self.alive.get() && self.event_generation.get() == generation {
                    *self.event_callback.borrow_mut() = Some(callback);
                }
            }
        }
    }
    fn clear_publishing(&self) {
        let pending = self.publishing.borrow_mut().take();
        drop(pending);
    }
    fn clear_release_ticket(&self) {
        let ticket = self.release.borrow_mut().take();
        if let Some(ticket) = ticket {
            self.window.clear_timeout_with_handle(ticket.handle);
            drop(ticket);
        }
    }
    fn finish_press(&self, token: PressToken) {
        let mut press = self.press.get();
        let finished = press.finish(token);
        self.press.set(press);
        if let Some(finished) = finished {
            if finished.was_active
                && let Ok(core) = self.core()
                && let Err(error) = core.tree_press_end(finished.token.id)
            {
                self.record(error);
            }
            if self.alive.get()
                && let Err(error) = self.request()
            {
                self.record(error);
            }
        }
    }
    fn queue_press_release(self: &Rc<Self>, id: i32) -> Result<()> {
        let mut press = self.press.get();
        let Some(token) = press.up(id) else {
            return Ok(());
        };
        self.press.set(press);
        if self
            .release
            .borrow()
            .as_ref()
            .is_some_and(|ticket| ticket.token == token)
        {
            return Ok(());
        }
        self.clear_release_ticket();
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(driver) = weak.upgrade()
                && driver.alive.get()
            {
                driver.finish_press(token);
            }
        });
        // The native Up and its generated Click must finish before an automatic
        // layout can replace their target. An explicit flush in an Up listener
        // remains protected too; this is not a repeated RAF polling loop.
        let handle = match self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            ) {
            Ok(handle) => handle,
            Err(error) => {
                self.finish_press(token);
                return Err(host(error));
            }
        };
        *self.release.borrow_mut() = Some(ReleaseTicket {
            handle,
            token,
            _callback: callback,
        });
        Ok(())
    }
    fn cancel_accepted_press(&self) {
        let mut press = self.press.get();
        let cancelled = press.cancel_accepted();
        self.press.set(press);
        self.intent.set(None);
        if cancelled.is_some()
            && let Ok(core) = self.core()
            && let Err(error) = core.tree_press_cancel()
        {
            self.record(error);
        }
    }
    fn cancel_native_press(&self) {
        self.cancel_accepted_press();
        self.intent.set(Some(PointerIntent::Leave));
        if self.alive.get()
            && let Err(error) = self.request()
        {
            self.record(error);
        }
    }
    fn abort_native_press(&self) {
        self.clear_release_ticket();
        let mut press = self.press.get();
        let aborted = press.abort();
        self.press.set(press);
        self.intent.set(None);
        if aborted.is_some()
            && let Ok(core) = self.core()
            && let Err(error) = core.tree_press_cancel()
        {
            self.record(error);
        }
    }
    fn event_within_plot(&self, event: &Event) -> bool {
        let Some(core) = self.core.upgrade() else {
            return false;
        };
        let path = event.composed_path();
        // A chart nested inside a custom Widget owns its own native intent.
        // The first actual browser-chart ancestor must be this exact owner.
        if path
            .iter()
            .filter_map(|node| node.dyn_into::<Element>().ok())
            .find(|element| has_class(element, "kartis-browser"))
            .is_none_or(|element| element != *core.outer)
        {
            return false;
        }
        let surface: wasm_bindgen::JsValue = self.surface.clone().into();
        path.iter().any(|node| {
            node == surface
                || self
                    .overlay
                    .as_ref()
                    .is_some_and(|overlay| node == wasm_bindgen::JsValue::from(overlay.clone()))
        })
    }
    fn native_down(&self, pointer: &PointerEvent, event: &Event) -> Result<()> {
        if !event.is_trusted()
            || !pointer.is_primary()
            || pointer.pointer_type() == "touch"
            || pointer.button() != 0
        {
            return Ok(());
        }
        let mut press = self.press.get();
        if !self.event_within_plot(event) {
            if let Some(retired) = press.clear_retired() {
                self.clear_release_ticket();
                self.press.set(press);
                if retired.was_active {
                    if let Err(error) = self.core()?.tree_press_end(retired.token.id) {
                        self.record(error);
                    }
                    self.request()?;
                }
            }
            return Ok(());
        }
        let widget = event.composed_path().iter().any(|node| {
            node.dyn_ref::<Element>().is_some_and(|element| {
                element.get_attribute("class").is_some_and(|classes| {
                    classes.split_ascii_whitespace().any(|class| {
                        matches!(
                            class,
                            "kartis-plot-widget-slot" | "kartis-tree-svg-custom-host"
                        )
                    })
                })
            })
        });
        let Some(started) = press.begin(pointer.pointer_id(), widget) else {
            return Ok(());
        };
        self.clear_release_ticket();
        if let Some(previous) = started.previous_active
            && let Err(error) = self.core()?.tree_press_end(previous.id)
        {
            self.record(error);
        }
        self.core()?
            .tree_press_begin(started.token.id, started.token.widget)?;
        self.press.set(press);
        Ok(())
    }
    fn snapshot(&self) -> Result<BrowserTreeSnapshot> {
        let accepted = self
            .accepted
            .try_borrow()
            .map_err(|_| ChartError("Tree accepted data is already in use".into()))?;
        let publishing = self
            .publishing
            .try_borrow()
            .map_err(|_| ChartError("Tree publish data is already in use".into()))?;
        let alive = self.alive.get() && self.core.upgrade().is_some_and(|e| e.alive.get());
        let data = publishing
            .as_ref()
            .filter(|p| alive && p.was_adopted())
            .map_or(&*accepted, |p| &p.candidate);
        let chart = self.chart().and_then(|c| c.snapshot()).ok();
        Ok(BrowserTreeSnapshot {
            active: alive,
            layout: data.layout.clone(),
            config: data.request.config.clone(),
            widget_revision: data.widget_revision,
            desired_focus: self.authored.borrow().activity.focus_id.clone(),
            pending: self.dirty.get()
                || self.canvas_dirty.get()
                || self.tooltip_dirty.get()
                || self.link_tooltip_dirty.get()
                || self.svg_pending()
                || self.motion_pending()
                || self.motion_revision_dirty.get()
                || self.motion_step_pending()
                || self.motion_running()
                || self.intent.get().is_some()
                || chart.as_ref().is_some_and(|s| s.pending > 0),
            raf_pending: self.ticket.get().is_some(),
            chart,
            error: self.error.borrow().clone(),
            picking: self.picking,
        })
    }
    fn build(&self, request: &Request) -> Result<TreeActivityLayout> {
        validate_request(request)?;
        if self.svg.is_some() {
            return self.build_svg(request);
        }
        let mut build = self
            .builder
            .try_borrow_mut()
            .map_err(|_| ChartError("Tree builder is already in use".into()))?
            .take()
            .ok_or_else(|| ChartError("Tree builder is already in use".into()))?;
        self.building.set(true);
        let result = build(&request.context());
        self.building.set(false);
        if self.alive.get() {
            *self.builder.borrow_mut() = Some(build);
        }
        scene::ensure(self.alive.get(), "Tree stopped during its build")?;
        let candidate = result?;
        validate_candidate(request, &candidate)?;
        Ok(candidate)
    }
    fn frame(&self) -> Result<()> {
        self.frame_at(None)
    }
    fn frame_at(&self, time: Option<f64>) -> Result<()> {
        if !self.alive.get() {
            return Ok(());
        }
        if self.processing.replace(true) {
            return Ok(());
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.processing);
        if !self.core.upgrade().is_some_and(|e| e.alive.get()) {
            self.stop();
            return Ok(());
        }
        let chart = self.chart()?;
        let snapshot = chart.snapshot()?;
        if snapshot.pending > 0 || snapshot.animating {
            chart.flush()?;
        }
        if !self.alive.get() {
            return Ok(());
        }
        let controlled_request = self.consume_svg_current()?;
        self.exclude_tooltip()?;
        if let Some(token) = self.press.get().token()
            && self.press.get().held()
            && !self.core()?.tree_press_active(token.id)
        {
            // A successful chart-only mutation (for example hidden/reset) or
            // a transport cancellation can retire the shared lease first.
            self.cancel_accepted_press();
        }
        // Existing dirty work is an explicit request (or measured resize).
        // A held native hover never overwrites it. Rejections keep both the
        // accepted gesture and latest deferred intent; only adoption retires
        // them. Chart input above continues to process Down/Move/Up meanwhile.
        let svg_program_dirty = self.svg_program_pending();
        if (self.press.get().held() || !self.motion_visible())
            && let Some(motion) = &self.motion
        {
            motion.rebase.set(true);
        }
        let motion_revision_dirty = self.motion_revision_dirty.replace(false);
        let explicit = motion_revision_dirty
            || self.dirty.get()
            || self.canvas_dirty.get()
            || svg_program_dirty
            || self.motion_pending();
        let intent = if self.press.get().held() || explicit {
            None
        } else {
            self.intent.take().filter(|intent| match intent {
                PointerIntent::MouseNode { generation, .. } => {
                    *generation == self.mouse_generation()
                }
                _ => true,
            })
        };
        let mut native_request = controlled_request;
        if let Some(intent) = intent {
            let next = match intent {
                PointerIntent::MouseNode { index, .. } => index.and_then(|index| {
                    self.mouse_frame()
                        .ok()
                        .and_then(|(_, frame)| frame.source().get(index).map(|d| d.id.clone()))
                }),
                PointerIntent::Leave => None,
                PointerIntent::SvgMove(client, _, mesh) => self.svg_native_node(client, mesh)?,
                PointerIntent::Move(client, _) => match self.picking {
                    BrowserTreePicking::Geometry => {
                        let model = self.model()?;
                        let accepted = self.accepted.borrow();
                        model
                            .hovered()
                            .iter()
                            .find_map(|id| accepted.layout.node_id_from_mark(id).map(str::to_owned))
                    }
                    BrowserTreePicking::Nearest { distance } => self.nearest(client, distance)?,
                },
            };
            self.select_tooltip(intent, next.as_deref());
            if self.motion.is_some() {
                let mut request = self.accepted.borrow().request.clone();
                if !matches!(
                    intent,
                    PointerIntent::MouseNode {
                        set_current: false,
                        ..
                    }
                ) && request.activity.focus_id != next
                {
                    request.activity.focus_id = next;
                    native_request = Some(request);
                    self.dirty.set(true);
                }
            } else {
                let mut request = self.authored.borrow_mut();
                if !matches!(
                    intent,
                    PointerIntent::MouseNode {
                        set_current: false,
                        ..
                    }
                ) && request.activity.focus_id != next
                {
                    request.activity.focus_id = next;
                    self.dirty.set(true);
                }
            }
        }
        let mut layout_dirty = self.dirty.replace(false);
        let canvas_dirty = self.canvas_dirty.replace(false);
        let tooltip_dirty = ((!self.press.get().held() || explicit)
            && self.tooltip_dirty.replace(false))
            | self.link_tooltip_dirty.replace(false);
        let motion_due = time.is_some() && !self.press.get().held() && self.motion_running();
        let motion_dirty = self.motion_pending()
            || motion_due
            || self.motion_step_pending()
            || motion_revision_dirty;
        if layout_dirty || canvas_dirty || tooltip_dirty || svg_program_dirty || motion_dirty {
            let retry_desired = self.svg_retry_explicit(explicit);
            let svg_program = self.take_svg_program(retry_desired);
            let svg_program_changed = svg_program
                .as_ref()
                .is_some_and(|program| self.svg_program_changed(program));
            if svg_program
                .as_ref()
                .is_some_and(|program| self.svg_label_gate_changed(program))
            {
                layout_dirty = true;
            }
            scene::ensure(
                self.motion.is_some() || svg_program.as_ref().is_none_or(|p| !p.requires_motion()),
                "animated SVG layers require Tree motion capability",
            )?;
            let tooltip_config = self.take_tooltip_config(retry_desired);
            let motion_attempt = self.take_motion_attempt(retry_desired);
            let source_update = layout_dirty || canvas_dirty;
            let svg_update =
                self.svg.is_some() && (source_update || svg_program_dirty || motion_dirty);
            // Failed attempts consume publication numbers too. Native view,
            // hidden-series and DPR retargets reserve from this same Engine.
            let svg_publication = svg_update
                .then(|| self.core()?.reserve_tree_svg_publication())
                .transpose()?;
            let mut candidate = if layout_dirty {
                let generation = self
                    .next_generation
                    .get()
                    .checked_add(1)
                    .ok_or_else(|| ChartError("Tree source generation exhausted".into()))?;
                self.next_generation.set(generation);
                let request = native_request
                    .clone()
                    .unwrap_or_else(|| self.authored.borrow().clone());
                if self.motion.is_some() {
                    scene::ensure(
                        request.tree.orientation != TreeOrientation::Radial,
                        "automatic Tree motion does not support Radial orientation",
                    )?;
                }
                let layout = self.build(&request)?;
                Accepted {
                    request,
                    layout,
                    widget_revision: if native_request.is_some() {
                        self.accepted.borrow().widget_revision
                    } else {
                        self.desired_revision.get()
                    },
                    generation,
                }
            } else {
                self.accepted.borrow().clone()
            };
            if motion_revision_dirty {
                candidate.widget_revision = self.desired_revision.get();
            }
            let frame = (source_update || svg_update)
                .then(|| tree_activity_frame(&candidate.layout))
                .transpose()?;
            self.building.set(true);
            let packet = if !source_update && !svg_update {
                Ok(None)
            } else if self.svg.is_some() {
                let core = self.core()?;
                let program = svg_program.as_ref().expect("SVG attempt program");
                let accepted_roles = core.tree_svg_stage().map(|stage| {
                    svg_owner::source_roles(
                        stage
                            .plan()
                            .layers()
                            .iter()
                            .map(|occurrence| &occurrence.layer),
                    )
                });
                let next_roles = svg_owner::source_roles(program.layers().iter());
                if layout_dirty || accepted_roles.as_ref() != Some(&next_roles) {
                    self.components
                        .as_ref()
                        .map(|widgets| {
                            widgets.packet_for_svg(
                                &candidate.layout,
                                candidate.widget_revision,
                                program.layers(),
                            )
                        })
                        .transpose()
                } else {
                    Ok(core.tree_svg_packet())
                }
            } else if layout_dirty {
                self.components
                    .as_ref()
                    .map(|w| w.packet(&candidate.layout, candidate.widget_revision))
                    .transpose()
            } else {
                let core = self.core()?;
                let state = core.state.borrow();
                Ok(state
                    .as_ref()
                    .and_then(|s| s.plot_widgets.as_ref())
                    .map(|p| p.as_ref().clone()))
            };
            self.building.set(false);
            let packet = packet?;
            scene::ensure(self.alive.get(), "Tree stopped during component placement")?;
            if self.canvas.is_some()
                && let Some(packet) = &packet
            {
                let core = self.core()?;
                let state = core.state.borrow();
                let state = state
                    .as_ref()
                    .ok_or_else(|| ChartError("Tree chart is stopped".into()))?;
                let (model, _) = state.plot_widget_layout_candidate(
                    frame.as_ref().expect("source frame").clone(),
                    packet,
                )?;
                super::plot_widgets::prepare(&model, packet, true)?;
            }
            let motion_candidate = if svg_update && self.motion.is_some() {
                self.prepare_motion(
                    &candidate,
                    layout_dirty,
                    motion_attempt.as_ref().expect("motion attempt"),
                    time,
                )?
            } else {
                None
            };
            let svg_stage = if svg_update {
                self.building.set(true);
                let prepared = (|| {
                    let data = self.svg_data(&candidate)?;
                    let program = svg_program.as_ref().expect("SVG program");
                    let plan = program.plan(&data)?;
                    if self.motion.as_ref().is_some_and(|m| m.ordinary) {
                        motion_owner::ordinary_order(&plan)?;
                    }
                    if let Some(motion) = &motion_candidate {
                        self.core()?.prepare_tree_motion_svg(
                            frame.clone().expect("motion target frame"),
                            plan,
                            program.clone(),
                            self.picking,
                            candidate.generation,
                            svg_publication.expect("motion publication"),
                            self.svg
                                .as_ref()
                                .expect("motion SVG")
                                .current_factory
                                .clone(),
                            motion.paint.clone(),
                            BrowserTreeMotionFrame {
                                data: motion.model.frame().clone(),
                                source_generation: candidate.generation,
                                publication: svg_publication.expect("motion publication"),
                                configuration: motion.configuration,
                            },
                            candidate.widget_revision,
                            layout_dirty || svg_program_dirty || motion_revision_dirty,
                        )
                    } else {
                        self.core()?.prepare_tree_svg(
                            frame.clone().expect("SVG source frame"),
                            plan,
                            program.clone(),
                            self.picking,
                            packet.clone(),
                            candidate.generation,
                            svg_publication.expect("SVG publication"),
                            self.svg
                                .as_ref()
                                .expect("SVG controller")
                                .current_factory
                                .clone(),
                        )
                    }
                })();
                self.building.set(false);
                scene::ensure(
                    self.alive.get(),
                    "Tree stopped during SVG layer preparation",
                )?;
                Some(prepared?)
            } else {
                None
            };
            let tooltip = self.prepare_tooltip(
                &candidate,
                source_update,
                &tooltip_config,
                svg_program_changed,
                svg_stage.as_deref(),
            )?;
            let bitmap = if source_update {
                self.prepare_canvas_candidate(&candidate)?
            } else {
                None
            };
            let raw_receipt = bitmap.clone();
            self.error.borrow_mut().take();
            self.painting.set(true);
            let result = (|| {
                let stage = if let Some(svg) = &svg_stage {
                    self.core()?
                        .enqueue_tree_svg(svg.clone(), tooltip.clone())?;
                    None
                } else {
                    self.core()?
                        .enqueue_tree_tooltip(frame, bitmap, packet, tooltip.clone())?
                };
                *self.publishing.borrow_mut() = Some(Publishing {
                    candidate: candidate.clone(),
                    stage,
                    tooltip: tooltip.clone(),
                    tooltip_config,
                    svg: svg_stage.clone(),
                    svg_program: svg_program.clone(),
                    motion: motion_candidate.clone(),
                });
                chart.flush()
            })();
            self.painting.set(false);
            if let Err(error) = result {
                self.clear_publishing();
                return Err(error);
            }
            if !self.alive.get() {
                self.clear_publishing();
                return Ok(());
            }
            let bitmap_adopted = match &raw_receipt {
                Some(bitmap) => self.core()?.raw_bitmap_matches(bitmap),
                None => true,
            };
            let adopted = bitmap_adopted
                && self.publishing.borrow().as_ref().is_some_and(|p| {
                    p.was_adopted()
                        && p.svg.as_ref().is_none_or(|candidate| {
                            self.core
                                .upgrade()
                                .and_then(|core| core.tree_svg_stage())
                                .is_some_and(|accepted| candidate.same_admission(&accepted))
                        })
                });
            if !adopted {
                self.clear_publishing();
                return Err(self
                    .error
                    .borrow()
                    .clone()
                    .unwrap_or_else(|| ChartError("Tree activity frame was not accepted".into())));
            }
            if source_update || motion_revision_dirty {
                if source_update && self.motion.is_some() {
                    if native_request.is_some() {
                        if !self.desired_request_pending.get() {
                            *self.authored.borrow_mut() = candidate.request.clone();
                        }
                    } else {
                        self.desired_request_pending.set(false);
                    }
                }
                *self.accepted.borrow_mut() = candidate;
            }
            if layout_dirty {
                self.mesh.borrow_mut().take();
            }
            let tooltip_config = self
                .publishing
                .borrow()
                .as_ref()
                .expect("adopted Tree publication")
                .tooltip_config
                .clone();
            if let Some(motion) = self
                .publishing
                .borrow()
                .as_ref()
                .and_then(|p| p.motion.clone())
            {
                *self
                    .motion
                    .as_ref()
                    .expect("motion controller")
                    .accepted
                    .borrow_mut() = motion;
            }
            if motion_candidate.is_some()
                && let Some(motion) = &self.motion
            {
                motion.rebase.set(false);
            }
            self.accept_tooltip_config(tooltip_config);
            let program = self
                .publishing
                .borrow()
                .as_ref()
                .and_then(|publication| publication.svg_program.clone());
            if let Some(program) = program {
                self.accept_svg_program(program);
            }
            if svg_program_changed {
                // Reordered native coverage needs fresh native intent. Do this
                // only after adoption so a rejected program keeps its tooltip.
                self.intent.set(None);
                self.retire_tooltip_selection();
            }
            self.clear_publishing();
            self.exclude_tooltip()?;
            if explicit {
                self.cancel_accepted_press();
            }
            let generation = self.update_generation.get();
            let callback = self.update_callback.borrow_mut().take();
            if let Some(mut callback) = callback {
                callback(self.snapshot()?);
                if self.alive.get() && self.update_generation.get() == generation {
                    *self.update_callback.borrow_mut() = Some(callback);
                }
            }
        }
        self.emit();
        if self.alive.get()
            && (self.dirty.get()
                || self.canvas_dirty.get()
                || self.svg_pending()
                || !self.press.get().held() && self.tooltip_dirty.get()
                || self.link_tooltip_dirty.get()
                || self.motion_pending()
                || self.motion_revision_dirty.get()
                || self.motion_step_pending()
                || !self.press.get().held() && self.motion_running()
                || !self.press.get().held() && self.intent.get().is_some()
                || !self.events.borrow().is_empty())
        {
            self.request()?;
        } else if let Some(ticket) = self.ticket.take() {
            let _ = self.window.cancel_animation_frame(ticket);
        }
        Ok(())
    }
    fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        if let Some(ticket) = self.ticket.take() {
            let _ = self.window.cancel_animation_frame(ticket);
        }
        self.dirty.set(false);
        self.canvas_dirty.set(false);
        self.motion_revision_dirty.set(false);
        if let Some(svg) = &self.svg {
            svg.stop();
        }
        self.clear_tooltip_controller();
        if let Some(motion) = &self.motion {
            motion.stop();
        }
        self.abort_native_press();
        self.listeners.borrow_mut().clear();
        self.clear_mouse_cleanup();
        self.mouse.stop();
        self.mesh.borrow_mut().take();
        self.clear_publishing();
        if let Some(components) = &self.components {
            components.clear();
        }
        self.builder.borrow_mut().take();
        if let Some(canvas) = &self.canvas {
            canvas.borrow_mut().take();
        }
        self.events.borrow_mut().clear();
        self.event_callback.borrow_mut().take();
        self.update_callback.borrow_mut().take();
        if let Ok(chart) = self.chart() {
            chart.stop();
        }
    }
    fn nearest(&self, client: [f64; 2], distance: f64) -> Result<Option<String>> {
        let rect = self.surface.get_bounding_client_rect();
        if rect.width() <= 0. || rect.height() <= 0. {
            return Ok(None);
        }
        let model = self.model()?;
        let point = (
            (client[0] - rect.x()) / rect.width() * model.scene().width,
            (client[1] - rect.y()) / rect.height() * model.scene().height,
        );
        scene::ensure(
            [point.0, point.1].iter().all(|n| n.is_finite()),
            "invalid Tree pointer coordinates",
        )?;
        if !model.plot().contains(point.0, point.1) {
            return Ok(None);
        }
        let transform = model.paint_transform();
        let world = transform.unproject(point)?;
        let hidden = model.hidden();
        let mut cache = self.mesh.borrow_mut();
        if cache.as_ref().is_none_or(|cached| &cached.hidden != hidden) {
            let accepted = self.accepted.borrow();
            let layout = &accepted.layout.layout;
            let points = layout
                .nodes
                .iter()
                .filter_map(|node| {
                    let id = &layout.source()[node.source_index].id;
                    (!hidden.contains(id)).then(|| crate::mesh::MeshPoint {
                        id: id.clone(),
                        x: node.position[0],
                        y: node.position[1],
                        clip: None,
                    })
                })
                .collect();
            *cache = Some(CachedMesh {
                hidden: hidden.clone(),
                mesh: crate::mesh::PointMesh::new(points)?,
            });
        }
        Ok(cache
            .as_ref()
            .unwrap()
            .mesh
            .nearest(world.0, world.1, distance / transform.scale)
            .map(|p| p.id.clone()))
    }
    fn listen(self: &Rc<Self>) -> Result<()> {
        let weak = Rc::downgrade(self);
        self.add_window_listener("pointerdown", move |event| {
            let Some(driver) = weak.upgrade().filter(|d| d.alive.get()) else {
                return;
            };
            let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if let Err(error) = driver.native_down(pointer, &event) {
                driver.record(error);
            }
        })?;
        for (name, cancel) in [("pointerup", false), ("pointercancel", true)] {
            let weak = Rc::downgrade(self);
            self.add_window_listener(name, move |event| {
                let Some(driver) = weak.upgrade().filter(|d| d.alive.get()) else {
                    return;
                };
                let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if !pointer.is_primary()
                    || pointer.pointer_type() == "touch"
                    || driver
                        .press
                        .get()
                        .token()
                        .is_none_or(|p| p.id != pointer.pointer_id())
                {
                    return;
                }
                if cancel {
                    driver.cancel_native_press();
                    // Native cancellation ends paint protection now. Retain
                    // only its old click identity until an Up/new Down/blur:
                    // a protocol cancel need not synthesize a browser release.
                } else if let Err(error) = driver.queue_press_release(pointer.pointer_id()) {
                    driver.record(error);
                }
            })?;
        }
        let weak = Rc::downgrade(self);
        self.add_window_listener("click", move |event| {
            let Some(driver) = weak.upgrade().filter(|d| d.alive.get()) else {
                return;
            };
            if !event.is_trusted() || !driver.event_within_plot(&event) {
                return;
            }
            let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() else {
                return;
            };
            // Keyboard activation remains a valid independent input even when
            // an older physical press has been explicitly cancelled.
            if mouse.button() != 0 || mouse.detail() <= 0 {
                return;
            }
            let Some(token) = driver.press.get().token() else {
                return;
            };
            if event.dyn_ref::<PointerEvent>().is_some_and(|pointer| {
                // Primary identity comes from the recorded Down; some UAs do
                // not preserve isPrimary on the derived Click event.
                pointer.pointer_type() == "touch" || pointer.pointer_id() != token.id
            }) {
                return;
            }
            // WebKit can dispatch a MouseEvent without a pointerId. The one
            // retained sequence plus this Surface lineage and nonzero detail
            // identify its old primary click; a new native Down clears it.
            if driver.press.get().cancelled() {
                event.prevent_default();
                event.stop_immediate_propagation();
            }
            if let Err(error) = driver.queue_press_release(token.id) {
                driver.record(error);
            }
        })?;
        let weak = Rc::downgrade(self);
        self.add_window_listener("blur", move |event| {
            let Some(driver) = weak.upgrade().filter(|d| d.alive.get()) else {
                return;
            };
            if event.target().is_none_or(|target| {
                target != driver.window.clone().unchecked_into::<EventTarget>()
            }) {
                return;
            }
            driver.clear_release_ticket();
            driver.cancel_native_press();
        })?;
        let weak = Rc::downgrade(self);
        self.add_listener_on(
            self.input_root.clone().into(),
            "lostpointercapture",
            move |event| {
                let Some(driver) = weak.upgrade().filter(|d| d.alive.get()) else {
                    return;
                };
                if !driver.event_within_plot(&event) {
                    return;
                }
                let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                // A normal Up releases capture too; its pending release task owns
                // the tail. Only unexpected loss of a still-held Down cancels it.
                if let Some(token) = driver.press.get().token()
                    && token.id == pointer.pointer_id()
                    && driver.press.get().held()
                    && driver.release.borrow().is_none()
                {
                    if driver
                        .core()
                        .is_ok_and(|core| core.tree_press_active(token.id))
                    {
                        driver.cancel_native_press();
                    } else {
                        // Explicit accepted replacement already retired transport.
                        // Do not turn its capture release into a new hover intent.
                        driver.cancel_accepted_press();
                    }
                }
            },
        )?;
        let weak = Rc::downgrade(self);
        // Replacing the native SVG under a stationary pointer can retire the
        // browser's old enter/leave lineage. Observe the next real Move even
        // outside the owner so automatic current/tooltip intent still clears.
        // SVG routing only queries owned geometry/Mesh or its captured source
        // gesture; outside and custom input produce Leave, never global nearest.
        let move_target = if self.svg.is_some() {
            self.window.clone().into()
        } else {
            self.input_root.clone().into()
        };
        self.add_listener_on(move_target, "pointermove", move |event| {
            let Some(driver) = weak.upgrade() else {
                return;
            };
            if !driver.alive.get()
                || !driver.svg_native_enabled()
                || (driver.svg.is_none() && !driver.event_within_plot(&event))
            {
                return;
            }
            let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if !pointer.is_primary() || pointer.pointer_type() == "touch" {
                return;
            }
            let (x, y) = client_coordinates(pointer);
            if ![x, y].iter().all(|n| n.is_finite()) {
                return;
            }
            let intent = if driver.svg.is_some() {
                driver.svg_pointer_intent(&event, [x, y])
            } else {
                PointerIntent::Move([x, y], event.is_trusted())
            };
            driver.intent.set(Some(intent));
            if let Err(error) = driver.request() {
                driver.record(error);
            }
        })?;
        let weak = Rc::downgrade(self);
        self.add_listener_on(
            self.input_root.clone().into(),
            "pointerleave",
            move |event| {
                let Some(driver) = weak.upgrade() else {
                    return;
                };
                if !driver.alive.get() || !driver.svg_native_enabled() {
                    return;
                }
                let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if !pointer.is_primary() || pointer.pointer_type() == "touch" {
                    return;
                }
                let target = event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok());
                if !driver
                    .input_root
                    .unchecked_ref::<web_sys::Node>()
                    .is_same_node(target.as_ref())
                {
                    return;
                }
                if driver.svg.is_some()
                    && driver.press.get().held()
                    && driver
                        .press
                        .get()
                        .token()
                        .is_some_and(|token| !token.widget && token.id == pointer.pointer_id())
                {
                    let bounds = driver.surface.get_bounding_client_rect();
                    let (x, y) = client_coordinates(pointer);
                    if x >= bounds.left()
                        && x < bounds.right()
                        && y >= bounds.top()
                        && y < bounds.bottom()
                    {
                        // Capture release can emit a native Leave chain even
                        // while the pointer remains over this Surface. Keep
                        // the source gesture's last coalesced Move through its
                        // Up tail. Real outside Moves use the window listener.
                        return;
                    }
                }
                // Moving Surface -> this owner's sibling overlay is still inside
                // the single plot input domain, not a Tree Leave.
                if pointer
                    .related_target()
                    .and_then(|target| target.dyn_into::<web_sys::Node>().ok())
                    .is_some_and(|node| driver.input_root.contains(Some(&node)))
                {
                    return;
                }
                driver.intent.set(Some(PointerIntent::Leave));
                if let Err(error) = driver.request() {
                    driver.record(error);
                }
            },
        )
    }
    fn add_window_listener(
        &self,
        name: &'static str,
        callback: impl FnMut(Event) + 'static,
    ) -> Result<()> {
        self.add_listener_on(self.window.clone().into(), name, callback)
    }
    fn add_listener_on(
        &self,
        target: EventTarget,
        name: &'static str,
        callback: impl FnMut(Event) + 'static,
    ) -> Result<()> {
        let callback = Closure::<dyn FnMut(Event)>::new(callback);
        target
            .add_event_listener_with_callback_and_bool(
                name,
                callback.as_ref().unchecked_ref(),
                true,
            )
            .map_err(host)?;
        self.listeners.borrow_mut().push(NativeListener {
            target,
            name,
            callback,
        });
        Ok(())
    }
}

fn has_class(element: &Element, class: &str) -> bool {
    element
        .get_attribute("class")
        .is_some_and(|classes| classes.split_ascii_whitespace().any(|value| value == class))
}
