//! Application-owned force/drag RAF binding for the existing BrowserChart.
//! Native listeners only reserve/coalesce input; simulation, publication and
//! application callbacks run in RAF. No global simulation timer is installed.
use super::*;
use crate::network::{
    ForceNodeState, ForceSimulation, NetworkDragController, NetworkDragMapping, NetworkDragOptions,
};
use crate::network_glyphs::{
    NetworkGlyphOptions, NetworkLinkContext, NetworkLinkGeometry,
    network_glyph_layout_with_link_factory, pick_network_glyph_node,
};
#[path = "network_widgets.rs"]
mod widgets;
pub use widgets::{BrowserNetworkNodeWidgetOptions, BrowserNetworkWidgetOptions};
use widgets::{NodeWidgetMount, NodeWidgets, PublishingData, RenderedFrame};

type LinkFactory = dyn FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry>;
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserNetworkOptions {
    pub chart: BrowserOptions,
    pub drag: NetworkDragOptions,
    pub ticks_per_frame: usize,
    /// Begin cooling from the supplied simulation's heat. False starts idle;
    /// a node drag or explicit reheat still starts its own bounded tick loop.
    pub auto_start: bool,
}
impl Default for BrowserNetworkOptions {
    fn default() -> Self {
        Self {
            chart: BrowserOptions {
                legend: false,
                ..Default::default()
            },
            drag: NetworkDragOptions::default(),
            ticks_per_frame: 1,
            auto_start: true,
        }
    }
}
#[derive(Clone, Debug)]
pub struct BrowserNetworkSnapshot {
    pub active: bool,
    pub chart: Option<BrowserSnapshot>,
    pub nodes: Vec<ForceNodeState>,
    pub dragging: Option<crate::network::NetworkDragSession>,
    pub captured: Option<i32>,
    pub alpha: f64,
    pub alpha_target: f64,
    pub ticks: u64,
    pub pending: bool,
    pub raf_pending: bool,
    pub suspended: bool,
    pub error: Option<ChartError>,
}
#[derive(Clone)]
struct Data {
    controller: NetworkDragController,
    config: ChartConfig,
    glyphs: Option<Rc<NetworkGlyphOptions>>,
    mapping: Option<NetworkDragMapping>,
    start: Option<[f64; 2]>,
    widget_revision: u64,
}
struct Replacement {
    simulation: ForceSimulation,
    glyphs: Option<Rc<NetworkGlyphOptions>>,
}

fn layout_for(
    simulation: &ForceSimulation,
    config: &ChartConfig,
    glyphs: Option<&NetworkGlyphOptions>,
    factory: Option<&mut LinkFactory>,
) -> Result<SceneInteraction> {
    if let Some(factory) = factory {
        let glyphs = glyphs
            .ok_or_else(|| ChartError("custom network links require a typed node frame".into()))?;
        return network_glyph_layout_with_link_factory(config, simulation, glyphs, factory)
            .map(Into::into);
    }
    match glyphs {
        Some(glyphs) => simulation.scene_with_glyphs(config, glyphs).map(Into::into),
        None => simulation.scene(config).map(Into::into),
    }
}
struct CaptureListener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for CaptureListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.callback.as_ref().unchecked_ref(),
            true,
        );
    }
}
struct Down {
    id: i32,
    node: String,
    point: [f64; 2],
}
enum Gesture {
    Down(Down),
    Move(i32, [f64; 2]),
    Up(i32, [f64; 2]),
    Cancel(i32),
}
type StepCallback = Box<dyn FnMut(BrowserNetworkSnapshot)>;
struct Driver {
    core: Weak<Engine>,
    window: Window,
    surface: HtmlElement,
    data: RefCell<Data>,
    link_factory: RefCell<Option<Box<LinkFactory>>>,
    rendering_links: Cell<bool>,
    node_widgets: Option<NodeWidgets>,
    staging_widgets: Cell<bool>,
    publishing_data: RefCell<Option<PublishingData>>,
    widget_revision_pending: Cell<Option<u64>>,
    reheat_pending: Cell<Option<f64>>,
    ticks_per_frame: usize,
    alive: Cell<bool>,
    processing: Cell<bool>,
    running: Cell<bool>,
    suspended: Cell<bool>,
    ticket: Cell<Option<i32>>,
    raf: RefCell<Option<RafCallback>>,
    listeners: RefCell<Vec<CaptureListener>>,
    captured: Cell<Option<i32>>,
    inputs: RefCell<VecDeque<Gesture>>,
    cancel_pending: Cell<bool>,
    replacement: RefCell<Option<Replacement>>,
    observer: RefCell<Option<ResizeBinding>>,
    events: RefCell<VecDeque<BrowserEvent>>,
    event_callback: RefCell<Option<EventCallback>>,
    event_generation: Cell<u64>,
    step_callback: RefCell<Option<StepCallback>>,
    step_generation: Cell<u64>,
    error: RefCell<Option<ChartError>>,
}
#[must_use = "retain BrowserNetwork; dropping it stops physics, captures and the BrowserChart"]
pub struct BrowserNetwork {
    chart: BrowserChart,
    driver: Rc<Driver>,
}
#[derive(Clone)]
pub struct BrowserNetworkHandle {
    driver: Weak<Driver>,
}
impl BrowserNetwork {
    pub fn mount(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
    ) -> Result<Self> {
        Self::mount_inner(root, simulation, config, options, None, None, None)
    }

    /// Mount a force chart with per-node visual glyphs. The driver owns the
    /// typed datum binding and preserves authored fill/stroke on hover and
    /// selection. Dragging follows the visible contours and physics origins.
    pub fn mount_with_glyphs(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network glyph driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            None,
            None,
        )
    }

    /// Retain an explicit link factory and rebuild its ink from the current
    /// simulation on tick, drag, resize and replacement candidates. Node glyphs
    /// still determine drag picking. Link ink remains decorative and does not
    /// alter forces or link endpoints. Errors preserve the accepted frame and
    /// force state; external factory side effects cannot be rolled back.
    /// Replacement admission may invoke the factory before its later RAF call.
    pub fn mount_with_link_factory(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
        factory: impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + 'static,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network link driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            Some(Box::new(factory)),
            None,
        )
    }

    fn mount_inner(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: Option<Rc<NetworkGlyphOptions>>,
        mut link_factory: Option<Box<LinkFactory>>,
        mut widget_mount: Option<NodeWidgetMount>,
    ) -> Result<Self> {
        let BrowserNetworkOptions {
            chart: mut chart_options,
            drag,
            ticks_per_frame,
            auto_start,
        } = options;
        scene::ensure(
            (1..=16).contains(&ticks_per_frame),
            "network ticks_per_frame must be 1..=16",
        )?;
        scene::ensure(
            chart_options.interaction.view_mode == crate::surface::SurfaceViewMode::Transform,
            "force dragging requires a Scene-transform BrowserChart",
        )?;
        let controller = NetworkDragController::new(simulation, drag)?;
        let widget_revision = widget_mount
            .as_ref()
            .map_or(0, |w| w.runtime.options.revision);
        let initial = match widget_mount.as_mut() {
            Some(widgets) => widgets.runtime.layout(
                controller.simulation(),
                &config,
                glyphs.as_deref(),
                link_factory.as_deref_mut(),
                widget_revision,
            )?,
            None => RenderedFrame::plain(layout_for(
                controller.simulation(),
                &config,
                glyphs.as_deref(),
                link_factory.as_deref_mut(),
            )?),
        };
        if glyphs.is_some() {
            chart_options.interaction_data = initial.frame.interaction.clone();
            chart_options.highlight = BrowserHighlight::Controlled;
        }
        let (chart, node_widgets) = match widget_mount {
            Some(mut mount) => {
                let chart = mount.mount(root, initial, chart_options)?;
                (chart, Some(mount.runtime))
            }
            None => (
                BrowserChart::mount(root, initial.frame.scene, chart_options)?,
                None,
            ),
        };
        let surface = chart
            .engine
            .state
            .borrow()
            .as_ref()
            .unwrap()
            .surface
            .clone();
        let driver = Rc::new(Driver {
            core: Rc::downgrade(&chart.engine),
            window: chart.engine.window.clone(),
            surface,
            data: RefCell::new(Data {
                controller,
                config,
                glyphs,
                mapping: None,
                start: None,
                widget_revision,
            }),
            link_factory: RefCell::new(link_factory),
            rendering_links: Cell::new(false),
            node_widgets,
            staging_widgets: Cell::new(false),
            publishing_data: RefCell::new(None),
            widget_revision_pending: Cell::new(None),
            reheat_pending: Cell::new(None),
            running: Cell::new(auto_start),
            suspended: Cell::new(false),
            ticks_per_frame,
            alive: Cell::new(true),
            processing: Cell::new(false),
            ticket: Cell::new(None),
            raf: RefCell::new(None),
            listeners: RefCell::new(vec![]),
            captured: Cell::new(None),
            inputs: RefCell::new(VecDeque::new()),
            cancel_pending: Cell::new(false),
            replacement: RefCell::new(None),
            observer: RefCell::new(None),
            events: RefCell::new(VecDeque::new()),
            event_callback: RefCell::new(None),
            event_generation: Cell::new(0),
            step_callback: RefCell::new(None),
            step_generation: Cell::new(0),
            error: RefCell::new(None),
        });
        let owner = Self {
            chart,
            driver: driver.clone(),
        };
        let weak = Rc::downgrade(&driver);
        *driver.raf.borrow_mut() = Some(Closure::new(move |_: f64| {
            if let Some(driver) = weak.upgrade() {
                driver.ticket.set(None);
                if let Err(error) = driver.frame(false) {
                    driver.recover(error);
                }
            }
        }));
        if driver.node_widgets.is_none() {
            let weak = Rc::downgrade(&driver);
            owner
                .chart
                .on_resize_with_interaction(move |width, height| {
                    let driver = weak
                        .upgrade()
                        .ok_or_else(|| ChartError("network binding was dropped".into()))?;
                    scene::ensure(driver.alive.get(), "network binding is stopped")?;
                    let data = driver
                        .data
                        .try_borrow()
                        .map_err(|_| ChartError("network layout is already in use".into()))?
                        .clone();
                    let mut config = data.config.clone();
                    config.width = width;
                    config.height = height;
                    driver
                        .layout(
                            data.controller.simulation(),
                            &config,
                            data.glyphs.as_deref(),
                            data.widget_revision,
                        )
                        .map(|candidate| candidate.frame)
                });
        }
        let weak = Rc::downgrade(&driver);
        owner.chart.on_event(move |event| {
            if let Some(driver) = weak.upgrade()
                && driver.alive.get()
            {
                if driver.events.borrow().len() < 512 {
                    driver.events.borrow_mut().push_back(event);
                } else {
                    *driver.error.borrow_mut() =
                        Some(ChartError("network event queue exceeded 512 events".into()));
                }
                if let Err(error) = driver.request() {
                    driver.fatal(error);
                }
            }
        });
        driver.listen()?;
        if auto_start {
            driver.request()?;
        }
        Ok(owner)
    }
    pub fn handle(&self) -> BrowserNetworkHandle {
        BrowserNetworkHandle {
            driver: Rc::downgrade(&self.driver),
        }
    }
    pub fn snapshot(&self) -> Result<BrowserNetworkSnapshot> {
        self.driver.snapshot()
    }
    pub fn flush(&self) -> Result<()> {
        self.handle().flush()
    }
    pub fn stop(&self) {
        self.driver.stop();
    }
    pub fn on_event(&self, callback: impl FnMut(BrowserEvent) + 'static) {
        self.handle().on_event(callback);
    }
    pub fn on_tick(&self, callback: impl FnMut(BrowserNetworkSnapshot) + 'static) {
        self.handle().on_tick(callback);
    }
}
impl Drop for BrowserNetwork {
    fn drop(&mut self) {
        self.driver.stop();
    }
}
impl BrowserNetworkHandle {
    fn driver(&self) -> Result<Rc<Driver>> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("network owner was dropped".into()))?;
        scene::ensure(driver.alive.get(), "network owner is stopped")?;
        driver.require_widget_mutation_ready()?;
        Ok(driver)
    }
    pub fn is_active(&self) -> bool {
        self.driver
            .upgrade()
            .is_some_and(|d| d.alive.get() && d.core.upgrade().is_some_and(|e| e.alive.get()))
    }
    pub fn snapshot(&self) -> Result<BrowserNetworkSnapshot> {
        self.driver
            .upgrade()
            .ok_or_else(|| ChartError("network owner was dropped".into()))?
            .snapshot()
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
    pub fn on_tick(&self, callback: impl FnMut(BrowserNetworkSnapshot) + 'static) {
        if let Ok(driver) = self.driver() {
            driver
                .step_generation
                .set(driver.step_generation.get().wrapping_add(1));
            *driver.step_callback.borrow_mut() = Some(Box::new(callback));
        }
    }
    pub fn flush(&self) -> Result<()> {
        let driver = self.driver()?;
        if driver.processing.get() {
            return Ok(());
        }
        if let Some(ticket) = driver.ticket.take() {
            driver.window.cancel_animation_frame(ticket).map_err(host)?;
        }
        match driver.frame(true) {
            Ok(()) => Ok(()),
            Err(error) => {
                driver.recover(error.clone());
                Err(error)
            }
        }
    }
    pub fn dispatch(&self, input: SurfaceInput) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        driver.chart()?.dispatch(input)
    }
    pub fn reset_view(&self) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        driver.chart()?.reset_view()
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<()> {
        let driver = self.driver()?;
        driver.request()?;
        driver.chart()?.set_hidden(hidden)
    }
    pub fn set_selected(&self, node: Option<&str>) -> Result<()> {
        let driver = self.driver()?;
        if let Some(id) = node {
            scene::ensure(
                driver
                    .data
                    .borrow()
                    .controller
                    .simulation()
                    .node(id)
                    .is_some(),
                "unknown selected network node",
            )?;
        }
        driver.request()?;
        driver
            .core()?
            .try_enqueue(Input::Selected(node.map(|id| format!("force-node:{id}"))))
    }
    pub fn reheat(&self, alpha: f64) -> Result<()> {
        let driver = self.driver()?;
        let mut data = driver
            .data
            .try_borrow()
            .map_err(|_| ChartError("network data is already in use".into()))?
            .clone();
        data.controller.reheat(alpha)?;
        driver.request()?;
        driver.running.set(true);
        if driver.node_widgets.is_some() {
            driver.reheat_pending.set(Some(
                driver
                    .reheat_pending
                    .get()
                    .map_or(alpha, |pending| pending.max(alpha)),
            ));
        } else {
            *driver.data.borrow_mut() = data;
        }
        Ok(())
    }
    /// Queue the latest complete graph without remounting. Scene admission and
    /// RAF admission happen before replacing an earlier pending candidate.
    /// A glyph driver retains the styles of node IDs present in the new graph.
    pub fn replace_simulation(&self, simulation: ForceSimulation) -> Result<()> {
        let driver = self.driver()?;
        let glyphs = driver.data.borrow().glyphs.as_ref().map(|glyphs| {
            if glyphs.nodes.keys().all(|id| simulation.node(id).is_some()) {
                glyphs.clone()
            } else {
                Rc::new(NetworkGlyphOptions {
                    nodes: glyphs
                        .nodes
                        .iter()
                        .filter(|(id, _)| simulation.node(id).is_some())
                        .map(|(id, style)| (id.clone(), style.clone()))
                        .collect(),
                })
            }
        });
        driver.queue_replacement(Replacement { simulation, glyphs })
    }

    /// Queue new physics and visual styles together for a glyph driver.
    /// Unknown style IDs or invalid glyph geometry reject the whole update.
    pub fn replace_simulation_with_glyphs(
        &self,
        simulation: ForceSimulation,
        glyphs: NetworkGlyphOptions,
    ) -> Result<()> {
        let driver = self.driver()?;
        scene::ensure(
            driver.data.borrow().glyphs.is_some(),
            "network was not mounted with glyphs",
        )?;
        driver.queue_replacement(Replacement {
            simulation,
            glyphs: Some(Rc::new(glyphs)),
        })
    }
    /// Camera configuration follows the chart's pan/zoom enablement and limits.
    /// A batch of ordinary SurfaceInput actions is validated before enqueueing.
    pub fn set_view(&self, view: ZoomTransform) -> Result<()> {
        view.validate()?;
        let driver = self.driver()?;
        let core = driver.core()?;
        let actions = {
            let state = core
                .state
                .try_borrow()
                .map_err(|_| ChartError("browser frame is in use".into()))?;
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            if state.model.view() == view
                && core.queue.borrow().is_empty()
                && !core.resize_pending.get()
            {
                return Ok(());
            }
            let mut model = state.model.clone();
            let plot = model.plot();
            let anchor = (plot.x + plot.width / 2., plot.y + plot.height / 2.);
            scene::ensure(
                model.options().zoom || view.scale == model.view().scale,
                "network zoom is disabled",
            )?;
            scene::ensure(
                model.options().pan
                    || (view.translate_x == model.view().translate_x
                        && view.translate_y == model.view().translate_y),
                "network pan is disabled",
            )?;
            let mut actions = vec![SurfaceInput::Reset];
            model.apply(SurfaceInput::Reset)?;
            let factor = view.scale / model.view().scale;
            let wheel = SurfaceInput::Wheel {
                point: anchor,
                delta: -factor.ln() / 0.002,
            };
            model.apply(wheel)?;
            actions.push(wheel);
            let current = model.view();
            let dx = view.translate_x - current.translate_x;
            let dy = view.translate_y - current.translate_y;
            if dx != 0. || dy != 0. {
                let id = i32::MIN;
                for action in [
                    SurfaceInput::Down {
                        id,
                        point: anchor,
                        brush: false,
                    },
                    SurfaceInput::Move {
                        id,
                        point: (anchor.0 + dx, anchor.1 + dy),
                    },
                    SurfaceInput::Cancel { id },
                ] {
                    model.apply(action)?;
                    actions.push(action);
                }
            }
            let actual = model.view();
            scene::ensure(
                (actual.scale - view.scale).abs() < 1e-8
                    && (actual.translate_x - view.translate_x).abs() < 1e-8
                    && (actual.translate_y - view.translate_y).abs() < 1e-8,
                "requested network view is outside enabled pan/zoom limits",
            )?;
            actions
        };
        scene::ensure(
            core.queue.borrow().len() + actions.len() <= MAX_PENDING,
            "browser input queue exceeded its 256-event budget",
        )?;
        driver.request()?;
        core.request()?;
        core.queue
            .borrow_mut()
            .extend(actions.into_iter().map(Input::Action));
        Ok(())
    }
}
impl Driver {
    fn layout(
        &self,
        simulation: &ForceSimulation,
        config: &ChartConfig,
        glyphs: Option<&NetworkGlyphOptions>,
        widget_revision: u64,
    ) -> Result<RenderedFrame> {
        scene::ensure(
            !self.rendering_links.replace(true),
            "network link factory is already in use",
        )?;
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.rendering_links);
        // Take the callback out so a callback can read accepted state or stop
        // its owner without borrowing the factory cell recursively.
        let mut factory = self.link_factory.borrow_mut().take();
        let result = match &self.node_widgets {
            Some(widgets) => widgets.layout(
                simulation,
                config,
                glyphs,
                factory.as_deref_mut(),
                widget_revision,
            ),
            None => layout_for(simulation, config, glyphs, factory.as_deref_mut())
                .map(RenderedFrame::plain),
        };
        if self.alive.get() {
            *self.link_factory.borrow_mut() = factory;
        }
        result
    }
    fn core(&self) -> Result<Rc<Engine>> {
        let core = self
            .core
            .upgrade()
            .ok_or_else(|| ChartError("browser chart was dropped".into()))?;
        scene::ensure(core.alive.get(), "browser chart is stopped")?;
        Ok(core)
    }
    fn chart(&self) -> Result<BrowserHandle> {
        self.core()?;
        Ok(BrowserHandle {
            engine: self.core.clone(),
        })
    }
    fn accepted_model(&self) -> Result<SurfaceModel> {
        let core = self.core()?;
        let state = core
            .state
            .try_borrow()
            .map_err(|_| ChartError("browser frame is in use".into()))?;
        state
            .as_ref()
            .map(|state| state.model.clone())
            .ok_or_else(|| ChartError("browser chart is stopped".into()))
    }
    fn queue_replacement(&self, replacement: Replacement) -> Result<()> {
        let mut config = self.data.borrow().config.clone();
        let core = self.core()?;
        {
            let state = core
                .state
                .try_borrow()
                .map_err(|_| ChartError("browser frame is in use".into()))?;
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            config.width = state.model.scene().width;
            config.height = state.model.scene().height;
        }
        // The retained user factory may read state, stop its owner or reject a
        // nested replacement. Never execute it while holding the core state.
        let widget_revision = self
            .widget_revision_pending
            .get()
            .unwrap_or_else(|| self.data.borrow().widget_revision);
        let candidate = self.layout(
            &replacement.simulation,
            &config,
            replacement.glyphs.as_deref(),
            widget_revision,
        )?;
        scene::ensure(
            self.alive.get() && core.alive.get(),
            "network stopped during link rendering",
        )?;
        {
            let state = core
                .state
                .try_borrow()
                .map_err(|_| ChartError("browser frame is in use".into()))?;
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            if let Some(widgets) = &self.node_widgets {
                widgets.admit(state, &candidate)?;
            } else if candidate.frame.interaction.is_some() {
                state.admit_interaction_scene(
                    &candidate.frame.scene,
                    candidate.frame.interaction.as_ref(),
                )?;
                surface::validate_plot(state.model.options().plot, &candidate.frame.scene)?;
            } else {
                state.admit_scene(&candidate.frame.scene)?;
            }
        }
        self.request()?;
        *self.replacement.borrow_mut() = Some(replacement);
        Ok(())
    }
    fn request(&self) -> Result<()> {
        scene::ensure(self.alive.get(), "network binding is stopped")?;
        if self.ticket.get().is_none() {
            let raf = self.raf.borrow();
            let raf = raf
                .as_ref()
                .ok_or_else(|| ChartError("network RAF unavailable".into()))?;
            self.ticket.set(Some(
                self.window
                    .request_animation_frame(raf.as_ref().unchecked_ref())
                    .map_err(host)?,
            ));
        }
        Ok(())
    }
    fn snapshot(&self) -> Result<BrowserNetworkSnapshot> {
        let data = self
            .data
            .try_borrow()
            .map_err(|_| ChartError("network data is already in use".into()))?;
        let publishing = self
            .publishing_data
            .try_borrow()
            .map_err(|_| ChartError("network publish snapshot is already in use".into()))?;
        let data = publishing
            .as_ref()
            .filter(|pending| {
                self.alive.get()
                    && self.core.upgrade().is_some_and(|core| core.alive.get())
                    && pending.stage.was_adopted()
            })
            .map_or(&*data, |pending| &pending.data);
        let simulation = data.controller.simulation();
        Ok(BrowserNetworkSnapshot {
            active: self.alive.get() && self.core.upgrade().is_some_and(|e| e.alive.get()),
            chart: self.chart().and_then(|h| h.snapshot()).ok(),
            nodes: simulation.nodes().to_vec(),
            dragging: data.controller.active().cloned(),
            captured: self.captured.get(),
            alpha: simulation.alpha(),
            alpha_target: simulation.options().alpha_target,
            ticks: simulation.total_ticks(),
            pending: !self.inputs.borrow().is_empty()
                || self.cancel_pending.get()
                || self.replacement.borrow().is_some()
                || self.widget_revision_pending.get().is_some()
                || self.reheat_pending.get().is_some(),
            raf_pending: self.ticket.get().is_some(),
            suspended: self.suspended.get(),
            error: self.error.borrow().clone(),
        })
    }
    fn release_capture(&self) {
        if let Some(id) = self.captured.take()
            && self.surface.has_pointer_capture(id)
        {
            let _ = self.surface.release_pointer_capture(id);
        }
    }
    fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        if let Some(ticket) = self.ticket.take() {
            let _ = self.window.cancel_animation_frame(ticket);
        }
        self.release_capture();
        self.clear_publishing_data();
        self.listeners.borrow_mut().clear();
        if let Some((observer, _)) = self.observer.borrow_mut().take() {
            observer.disconnect();
        }
        self.inputs.borrow_mut().clear();
        self.cancel_pending.set(false);
        self.replacement.borrow_mut().take();
        self.widget_revision_pending.set(None);
        self.reheat_pending.set(None);
        if let Some(widgets) = &self.node_widgets {
            widgets.clear();
        }
        self.events.borrow_mut().clear();
        let _ = self.data.borrow_mut().controller.cancel();
        self.event_callback.borrow_mut().take();
        self.step_callback.borrow_mut().take();
        self.link_factory.borrow_mut().take();
        if let Ok(chart) = self.chart() {
            chart.stop();
        }
    }
    fn fatal(&self, error: ChartError) {
        *self.error.borrow_mut() = Some(error);
        self.stop();
    }
    fn recover(&self, error: ChartError) {
        if self.processing.replace(true) {
            self.record(error);
            return;
        }
        struct Guard<'a>(&'a Cell<bool>);
        impl Drop for Guard<'_> {
            fn drop(&mut self) {
                self.0.set(false);
            }
        }
        let _guard = Guard(&self.processing);
        self.clear_publishing_data();
        if !self.core.upgrade().is_some_and(|e| e.alive.get()) {
            self.fatal(error);
            return;
        }
        if let Some(ticket) = self.ticket.take() {
            let _ = self.window.cancel_animation_frame(ticket);
        }
        self.running.set(false);
        self.release_capture();
        self.inputs.borrow_mut().clear();
        self.cancel_pending.set(false);
        self.replacement.borrow_mut().take();
        self.widget_revision_pending.set(None);
        self.reheat_pending.set(None);
        if self.node_widgets.is_some() {
            // Preserve the complete accepted physics snapshot. Native capture
            // is released now; cancellation is a future candidate, accepted
            // only when its matching Widget/paint frame also succeeds.
            self.cancel_pending
                .set(self.data.borrow().controller.active().is_some());
            self.record(error);
            self.emit();
            return;
        }
        // Retain the last painted positions. A failed candidate must not
        // destroy the accepted chart or leave a released pointer heating a pin.
        {
            let mut data = self.data.borrow_mut();
            if let (Some(active), Some(mapping)) =
                (data.controller.active().cloned(), data.mapping.clone())
            {
                let _ = data
                    .controller
                    .end(active.pointer_id, active.last_client, &mapping);
            }
            data.start = None;
        }
        self.record(error);
        self.emit();
    }
    fn mapping(&self, config: &ChartConfig) -> Result<NetworkDragMapping> {
        let core = self.core()?;
        let state = core
            .state
            .try_borrow()
            .map_err(|_| ChartError("browser frame is in use".into()))?;
        let state = state
            .as_ref()
            .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
        let mut config = config.clone();
        config.width = state.model.scene().width;
        config.height = state.model.scene().height;
        let b = self.surface.get_bounding_client_rect();
        NetworkDragMapping::from_config(
            &config,
            ClipRect {
                x: b.x(),
                y: b.y(),
                width: b.width(),
                height: b.height(),
            },
            state.model.view(),
        )
    }
    fn enqueue(&self, input: Gesture) -> Result<()> {
        self.request()?;
        let mut queue = self.inputs.borrow_mut();
        let coalesce = matches!((&input,queue.back()),(Gesture::Move(id,_),Some(Gesture::Move(previous,_))) if id==previous);
        if coalesce {
            queue.pop_back();
        }
        if queue.len() >= 64 {
            queue.clear();
            self.cancel_pending.set(true);
            drop(queue);
            self.release_capture();
            self.record(ChartError(
                "network input queue exceeded 64 events; gesture cancelled".into(),
            ));
        } else {
            queue.push_back(input);
        }
        Ok(())
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
                if self.alive.get() && generation == self.event_generation.get() {
                    *self.event_callback.borrow_mut() = Some(callback);
                }
            }
        }
    }
    fn frame(&self, force_rebuild: bool) -> Result<()> {
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
        let bounds = self.surface.get_bounding_client_rect();
        if bounds.width() <= 0. || bounds.height() <= 0. {
            // A synchronous flush may see a hidden box before ResizeObserver
            // has delivered it. One wake lets a same-frame restoration resume;
            // a box that remains hidden goes idle until its observer wakes it.
            if !self.suspended.replace(true) {
                self.request()?;
            }
            self.release_capture();
            self.inputs.borrow_mut().clear();
            self.cancel_pending.set(false);
            if self.node_widgets.is_some() {
                // A hidden host cannot publish the matching Widget frame.
                // Preserve accepted physics until restoration can commit the
                // cancellation and its complete typed payload together.
                self.cancel_pending
                    .set(self.data.borrow().controller.active().is_some());
            } else {
                let mut data = self.data.borrow_mut();
                data.controller.cancel()?;
                data.mapping = None;
                data.start = None;
            }
            self.emit();
            return Ok(());
        }
        self.suspended.set(false);
        let chart = self.chart()?;
        let snapshot = chart.snapshot()?;
        if snapshot.pending != 0 || snapshot.animating {
            chart.flush()?;
        }
        if !self.alive.get() {
            return Ok(());
        }
        let mut data = self.data.borrow().clone();
        let previous_size = [data.config.width, data.config.height];
        let replacement = self.replacement.borrow_mut().take();
        let replaced = replacement.is_some();
        if let Some(Replacement { simulation, glyphs }) = replacement {
            data.controller.replace_simulation(simulation)?;
            data.glyphs = glyphs;
            data.mapping = None;
            data.start = None;
            self.release_capture();
            self.inputs.borrow_mut().clear();
            self.cancel_pending.set(false);
            self.running.set(true);
        }
        // Reheat is monotonic: coalesced requests retain their greatest heat
        // and apply to the latest complete graph. Both remain tentative until
        // this candidate's component/paint commit.
        let reheated = self.reheat_pending.get().is_some();
        if let Some(alpha) = self.reheat_pending.take() {
            data.controller.reheat(alpha)?;
        }
        let mapping = if self.node_widgets.is_some() {
            // Layout dimensions are untransformed CSS pixels. The screen
            // rectangle below remains transformed for native pointer mapping.
            // Treating its dimensions as layout would cancel an external CSS
            // scale and change the physical size of every force world unit.
            let [width, height] = self.widget_logical_size()?;
            data.config.width = width;
            data.config.height = height;
            let model = self.accepted_model()?;
            NetworkDragMapping::from_config(
                &data.config,
                ClipRect {
                    x: bounds.x(),
                    y: bounds.y(),
                    width: bounds.width(),
                    height: bounds.height(),
                },
                model.view(),
            )?
        } else {
            self.mapping(&data.config)?
        };
        let revised = self.widget_revision_pending.get().is_some();
        if let Some(revision) = self.widget_revision_pending.take() {
            data.widget_revision = revision;
        }
        data.config.width = mapping.scene_size[0];
        data.config.height = mapping.scene_size[1];
        if data.mapping.as_ref() != Some(&mapping) {
            data.controller.rebase(&mapping)?;
            data.mapping = Some(mapping.clone());
        }
        let hidden = self.chart()?.snapshot()?.hidden;
        let hidden_cancelled = data
            .controller
            .active()
            .is_some_and(|s| hidden.contains(&s.node_id));
        if hidden_cancelled {
            data.controller.cancel()?;
            self.release_capture();
            self.inputs.borrow_mut().clear();
            self.cancel_pending.set(false);
        }
        let inputs = std::mem::take(&mut *self.inputs.borrow_mut());
        let gesture_work = !inputs.is_empty();
        let mut clicks = vec![];
        let mut hover = None;
        let cancelled = self.cancel_pending.replace(false);
        if cancelled {
            data.controller.cancel()?;
            data.start = None;
        }
        for input in inputs {
            match input {
                Gesture::Down(down) => {
                    if data.glyphs.is_some() && hidden.contains(&down.node) {
                        continue;
                    }
                    match data
                        .controller
                        .begin(down.id, &down.node, down.point, &mapping)
                    {
                        Ok(()) => {
                            data.start = Some(down.point);
                            self.running.set(true);
                        }
                        Err(error) => self.record(error),
                    }
                }
                Gesture::Move(id, point) => {
                    if let Err(error) = data.controller.move_pointer(id, point, &mapping) {
                        self.record(error);
                    }
                }
                Gesture::Cancel(id) => {
                    if data.controller.active().is_some_and(|s| s.pointer_id == id) {
                        data.controller.cancel()?;
                        data.start = None;
                    }
                }
                Gesture::Up(id, point) => {
                    let node = data.controller.active().map(|s| s.node_id.clone());
                    match data.controller.end(id, point, &mapping) {
                        Ok(true) => {
                            if data
                                .start
                                .is_some_and(|p| (p[0] - point[0]).hypot(p[1] - point[1]) < 3.)
                                && let Some(node) = node
                            {
                                clicks.push(node);
                            }
                            hover = Some((id, point));
                            data.start = None;
                        }
                        Ok(false) => {}
                        Err(error) => {
                            self.record(error);
                            data.controller.cancel()?;
                            data.start = None;
                        }
                    }
                }
            }
        }
        if data.controller.active().is_none() {
            self.release_capture();
        }
        if let Some(active) = data.controller.active() {
            hover = Some((active.pointer_id, active.last_client));
        }
        let tick = (self.running.get() && data.controller.simulation().is_active())
            || data.controller.active().is_some();
        if tick {
            data.controller.tick(self.ticks_per_frame)?;
        }
        let resized = [data.config.width, data.config.height] != previous_size;
        if self.node_widgets.is_some()
            && !force_rebuild
            && !tick
            && !replaced
            && !reheated
            && !revised
            && !gesture_work
            && !cancelled
            && !hidden_cancelled
            && !resized
        {
            // An accepted layout rebuilds the painted hit index and its frame
            // token. A stationary PhysicalHover therefore reports that new
            // frame, which schedules this driver. Do not answer a pure event
            // with another identical layout/token and create an idle RAF loop.
            // BrowserChart already flushed any actual view/hover input above.
            // Explicit flush still rebuilds, so applications can reevaluate a
            // retained placement callback without changing simulation state.
            self.emit();
            if self.alive.get()
                && (!self.inputs.borrow().is_empty()
                    || self.cancel_pending.get()
                    || self.replacement.borrow().is_some()
                    || self.widget_revision_pending.get().is_some()
                    || self.reheat_pending.get().is_some()
                    || !self.events.borrow().is_empty())
            {
                self.request()?;
            }
            return Ok(());
        }
        let simulation = data.controller.simulation();
        let candidate = self.layout(
            simulation,
            &data.config,
            data.glyphs.as_deref(),
            data.widget_revision,
        )?;
        scene::ensure(self.alive.get(), "network stopped during node rendering")?;
        let replacement_hidden = replaced.then(|| {
            hidden
                .into_iter()
                .filter(|id| simulation.node(id).is_some())
                .collect()
        });
        // SurfaceModel clones retain the accepted Scene by Rc. Compare its
        // identity after painting so a rejected typed layout cannot advance
        // the force positions or become the next drag target. SurfaceModel
        // allocates a fresh Scene Rc on every accepted set_layout, even for
        // identical paint (fixed nodes with tick/heat-only Widget data).
        let before = if data.glyphs.is_some() {
            let accepted = self.accepted_model()?;
            if let Some(widgets) = &self.node_widgets {
                self.staging_widgets.set(true);
                let result = widgets.publish(&chart, candidate);
                self.staging_widgets.set(false);
                let stage = result?;
                *self.publishing_data.borrow_mut() = Some(PublishingData {
                    data: data.clone(),
                    stage,
                });
            } else {
                chart.set_layout(candidate.frame)?;
            }
            if let Some(hidden) = replacement_hidden {
                chart.set_hidden(hidden)?;
            }
            Some(accepted)
        } else {
            if let Some(hidden) = replacement_hidden {
                chart.set_hidden(hidden)?;
            }
            chart.set_scene(candidate.frame.scene)?;
            None
        };
        if let Some((id, client)) = hover {
            let p = mapping.client_to_scene(client)?;
            self.chart()?.dispatch(SurfaceInput::Move {
                id,
                point: (p[0], p[1]),
            })?;
        }
        for node in clicks {
            self.core()?
                .try_enqueue(Input::Selected(Some(format!("force-node:{node}"))))?;
            self.core()?
                .try_enqueue(Input::Activate(format!("force-node:{node}")))?;
        }
        self.staging_widgets.set(self.node_widgets.is_some());
        let painted = self.chart().and_then(|chart| chart.flush());
        self.staging_widgets.set(false);
        painted?;
        if !self.alive.get() {
            return Ok(());
        }
        if let Some(before) = before {
            let after = self.accepted_model()?;
            scene::ensure(
                !std::ptr::eq(before.scene(), after.scene()),
                "network glyph frame was not accepted",
            )?;
        }
        *self.data.borrow_mut() = data;
        self.clear_publishing_data();
        self.emit();
        if !self.alive.get() {
            return Ok(());
        }
        if tick {
            let generation = self.step_generation.get();
            let callback = self.step_callback.borrow_mut().take();
            if let Some(mut callback) = callback {
                callback(self.snapshot()?);
                if self.alive.get() && generation == self.step_generation.get() {
                    *self.step_callback.borrow_mut() = Some(callback);
                }
            }
        }
        if self.alive.get()
            && ((self.running.get() && self.data.borrow().controller.simulation().is_active())
                || self.data.borrow().controller.active().is_some()
                || !self.inputs.borrow().is_empty()
                || self.cancel_pending.get()
                || self.replacement.borrow().is_some()
                || self.widget_revision_pending.get().is_some()
                || self.reheat_pending.get().is_some()
                || !self.events.borrow().is_empty())
        {
            self.request()?;
        }
        Ok(())
    }
    fn listen(self: &Rc<Self>) -> Result<()> {
        let weak = Rc::downgrade(self);
        self.add(self.surface.clone().into(), "pointerdown", move |event| {
            let Some(driver) = weak.upgrade() else { return };
            if !driver.alive.get() {
                return;
            }
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if driver.captured.get().is_some() || driver.cancel_pending.get() {
                event.prevent_default();
                event.stop_immediate_propagation();
                return;
            }
            if driver.node_widgets.is_some()
                && event
                    .target()
                    .and_then(|target| target.dyn_into::<Element>().ok())
                    .is_some_and(|element| {
                        element
                            .closest(".kartis-plot-widget-slot")
                            .ok()
                            .flatten()
                            .is_some()
                    })
            {
                return;
            }
            if p.button() != 0 || !p.is_primary() || p.shift_key() {
                return;
            }
            let result = (|| -> Result<Option<String>> {
                let core = driver.core()?;
                if core.captured.get().is_some() || !core.touches.borrow().is_empty() {
                    return Ok(None);
                }
                let data = driver
                    .data
                    .try_borrow()
                    .map_err(|_| ChartError("network data is in use".into()))?;
                let mapping = driver.mapping(&data.config)?;
                let point = client_coordinates(p);
                let state = core
                    .state
                    .try_borrow()
                    .map_err(|_| ChartError("browser frame is in use".into()))?;
                let state = state
                    .as_ref()
                    .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
                let hidden = state.model.hidden();
                let painted = driver.node_widgets.as_ref().map(|_| {
                    state
                        .model
                        .scene()
                        .without_mark_ink(state.model.widget_replacements())
                });
                let picked = if data.glyphs.is_some() {
                    if driver.suspended.get() {
                        return Ok(None);
                    }
                    // Filter before picking, rather than discarding only the
                    // topmost answer: an unreplaced contour underneath a
                    // replaced node must remain a valid native drag target.
                    pick_network_glyph_node(
                        painted.as_ref().unwrap_or(state.model.scene()),
                        [point.0, point.1],
                        &mapping,
                        hidden,
                    )?
                    .filter(|id| data.controller.simulation().node(id).is_some())
                } else {
                    data.controller.pick([point.0, point.1], &mapping, hidden)?
                };
                Ok(picked.map(str::to_owned))
            })();
            let node = match result {
                Ok(Some(id)) => id,
                Ok(None) => return,
                Err(error) => {
                    driver.record(error);
                    return;
                }
            };
            let point = client_coordinates(p);
            if let Err(error) = driver.enqueue(Gesture::Down(Down {
                id: p.pointer_id(),
                node,
                point: [point.0, point.1],
            })) {
                driver.fatal(error);
                return;
            }
            if driver.cancel_pending.get() {
                event.prevent_default();
                event.stop_immediate_propagation();
                return;
            }
            if let Err(error) = driver
                .surface
                .set_pointer_capture(p.pointer_id())
                .map_err(host)
            {
                driver.fatal(error);
                return;
            }
            driver.captured.set(Some(p.pointer_id()));
            let _ = driver.surface.focus();
            event.prevent_default();
            event.stop_immediate_propagation();
        })?;
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "pointermove", move |event| {
            let Some(driver) = weak.upgrade() else { return };
            let Some(p) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if driver.captured.get() != Some(p.pointer_id()) {
                return;
            }
            event.prevent_default();
            event.stop_immediate_propagation();
            let point = client_coordinates(p);
            if ![point.0, point.1].iter().all(|v| v.is_finite()) {
                return;
            }
            if let Err(error) = driver.enqueue(Gesture::Move(p.pointer_id(), [point.0, point.1])) {
                driver.fatal(error);
            }
        })?;
        for name in ["pointerup", "pointercancel", "lostpointercapture"] {
            let weak = Rc::downgrade(self);
            self.add(self.window.clone().into(), name, move |event| {
                let Some(driver) = weak.upgrade() else { return };
                let Some(p) = event.dyn_ref::<PointerEvent>() else {
                    return;
                };
                if driver.captured.get() != Some(p.pointer_id()) {
                    return;
                }
                event.prevent_default();
                event.stop_immediate_propagation();
                let point = client_coordinates(p);
                let terminal =
                    if name == "pointerup" && [point.0, point.1].iter().all(|v| v.is_finite()) {
                        Gesture::Up(p.pointer_id(), [point.0, point.1])
                    } else {
                        Gesture::Cancel(p.pointer_id())
                    };
                if name == "lostpointercapture"
                    && driver.surface.has_pointer_capture(p.pointer_id())
                {
                    return;
                }
                if let Err(error) = driver.enqueue(terminal) {
                    driver.fatal(error);
                    return;
                }
                driver.release_capture();
            })?;
        }
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "blur", move |event| {
            if let Some(driver) = weak.upgrade()
                && event
                    .target()
                    .is_some_and(|target| target == driver.window.clone().into())
                && driver.captured.get().is_some()
            {
                if let Some(id) = driver.captured.get()
                    && let Err(error) = driver.enqueue(Gesture::Cancel(id))
                {
                    driver.fatal(error);
                    return;
                }
                driver.release_capture();
            }
        })?;
        if let Some(document) = self.window.document() {
            let weak = Rc::downgrade(self);
            let observed = document.clone();
            self.add(document.into(), "visibilitychange", move |_| {
                if observed.hidden()
                    && let Some(driver) = weak.upgrade()
                    && driver.captured.get().is_some()
                {
                    if let Some(id) = driver.captured.get()
                        && let Err(error) = driver.enqueue(Gesture::Cancel(id))
                    {
                        driver.fatal(error);
                        return;
                    }
                    driver.release_capture();
                }
            })?;
        }
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "pagehide", move |_| {
            if let Some(driver) = weak.upgrade() {
                driver.stop();
            }
        })?;
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(driver) = weak.upgrade()
                && driver.alive.get()
                && let Err(error) = driver.request()
            {
                driver.fatal(error);
            }
        });
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).map_err(host)?;
        observer.observe(&self.surface);
        *self.observer.borrow_mut() = Some((observer, callback));
        Ok(())
    }
    fn add(
        &self,
        target: EventTarget,
        name: &'static str,
        callback: impl FnMut(Event) + 'static,
    ) -> Result<()> {
        let callback = Closure::<dyn FnMut(Event)>::new(callback);
        let options = web_sys::AddEventListenerOptions::new();
        options.set_capture(true);
        options.set_passive(false);
        target
            .add_event_listener_with_callback_and_add_event_listener_options(
                name,
                callback.as_ref().unchecked_ref(),
                &options,
            )
            .map_err(host)?;
        self.listeners.borrow_mut().push(CaptureListener {
            target,
            name,
            callback,
        });
        Ok(())
    }
}
