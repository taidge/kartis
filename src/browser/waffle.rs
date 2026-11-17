//! Dedicated literal SVG Waffle owner. Source captures and native presentations
//! are adopted together; transparent area targets are not source paint Marks.
use std::any::{Any, TypeId};

use glory::Bond;

use super::*;

#[path = "waffle_data.rs"]
mod data;
#[path = "waffle_events.rs"]
mod events;
#[path = "waffle_layers.rs"]
mod layers;
#[path = "waffle_stage.rs"]
mod stage;
#[path = "waffle_tooltip.rs"]
mod tooltip;
use data::SourceEnvelope;
pub use data::*;
pub use events::{
    BrowserWaffleMouseEvent, BrowserWaffleMouseHandlers, BrowserWaffleMouseInput,
    BrowserWaffleMouseKind, BrowserWaffleMouseOccurrence, BrowserWaffleMousePointer,
    WaffleMouseHandler,
};
pub use layers::{
    BrowserWaffleSvgLayerContext, BrowserWaffleSvgLayerFrame, BrowserWaffleSvgProgram,
};
pub use tooltip::{
    BrowserWaffleTooltipAnchor, BrowserWaffleTooltipComponentContext, BrowserWaffleTooltipOptions,
    WaffleTooltipFrame,
};

#[derive(Clone, Copy, Debug)]
pub struct BrowserWaffleBuildContext<'a> {
    pub config: &'a ChartConfig,
    pub source: &'a [RadialDatum],
    pub options: &'a WaffleLayoutOptions,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserWaffleSvgHit {
    pub data: WaffleSvgFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub key: String,
    pub index: usize,
    pub source_index: usize,
    pub polygon_index: usize,
    pub area_id: String,
    pub client: [f64; 2],
    pub viewport: [f64; 2],
    pub scene: [f64; 2],
    pub trusted: bool,
    pub native_type: String,
}
#[derive(Clone, Debug)]
pub struct BrowserWaffleSnapshot {
    pub active: bool,
    pub data: Option<WaffleSvgFrame>,
    pub presentation: Option<WaffleSvgPlan>,
    pub source_generation: u64,
    pub publication: u64,
    pub hovered: Option<String>,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub pending: bool,
    pub raf_pending: bool,
    pub error: Option<ChartError>,
    pub ui_errors: Vec<ChartError>,
    pub event_errors: Vec<ChartError>,
    pub last_hit: Option<BrowserWaffleSvgHit>,
    pub tooltip: Option<WaffleTooltipFrame>,
}
type Builder =
    dyn FnMut(&BrowserWaffleBuildContext<'_>, Option<&Rc<dyn Any>>) -> Result<SourceEnvelope>;
#[derive(Default)]
struct RawMount {
    source: Option<Rc<dyn Any>>,
    kind: Option<TypeId>,
}
type HoverRole = (String, String, usize);
type ResizeRegistration = (ResizeObserver, Closure<dyn FnMut(web_sys::js_sys::Array)>);
#[derive(Clone)]
struct Request {
    config: ChartConfig,
    source: Vec<RadialDatum>,
    raw: Option<Rc<dyn Any>>,
    options: WaffleLayoutOptions,
    program: BrowserWaffleSvgProgram,
    view: ZoomTransform,
    tooltip: tooltip::Config,
}
struct Accepted {
    request: Request,
    source: SourceEnvelope,
    stage: Rc<stage::Stage>,
    generation: u64,
    publication: u64,
    hover: Option<HoverRole>,
    selection: Option<tooltip::Selection>,
}
enum Command {
    Source(Vec<RadialDatum>),
    TypedSource(Vec<RadialDatum>, Rc<dyn Any>),
    Config(ChartConfig),
    Options(WaffleLayoutOptions),
    Hidden(BTreeSet<String>),
    Program(BrowserWaffleSvgProgram),
    View(ZoomTransform),
    Resize(f64, f64),
    Refresh,
    Native {
        hover: Option<HoverRole>,
        selection: Option<tooltip::Selection>,
    },
    Tooltip(tooltip::Config),
    TooltipReset,
}
struct Listener {
    target: EventTarget,
    name: &'static str,
    callback: Closure<dyn Fn(Event)>,
    capture: bool,
}
impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.callback.as_ref().unchecked_ref(),
            self.capture,
        );
    }
}
pub(super) struct Driver {
    window: Window,
    root: Element,
    container: HtmlElement,
    content: HtmlElement,
    tooltip: Rc<tooltip::Host>,
    events: events::Events,
    alive: Cell<bool>,
    busy: Cell<bool>,
    accepted: RefCell<Option<Accepted>>,
    pending: RefCell<Vec<Command>>,
    builder: RefCell<Option<Box<Builder>>>,
    raw_type: Option<TypeId>,
    records: Rc<Cell<usize>>,
    all_records: RefCell<Vec<Weak<layers::Record>>>,
    raf: RefCell<Option<RafCallback>>,
    raf_id: Cell<Option<i32>>,
    listeners: RefCell<Vec<Listener>>,
    resize: RefCell<Option<ResizeRegistration>>,
    error: RefCell<Option<ChartError>>,
    ui_errors: RefCell<Vec<ChartError>>,
    event_errors: RefCell<VecDeque<ChartError>>,
    last_hit: RefCell<Option<(BrowserWaffleSvgHit, SourceEnvelope)>>,
}
pub struct BrowserWaffle {
    driver: Rc<Driver>,
}
#[derive(Clone)]
pub struct BrowserWaffleHandle {
    driver: Weak<Driver>,
}

impl BrowserWaffle {
    pub fn mount_with_svg_layers(
        root: Element,
        source: Vec<RadialDatum>,
        config: ChartConfig,
        options: WaffleLayoutOptions,
        program: BrowserWaffleSvgProgram,
    ) -> Result<Self> {
        Self::mount_with_factory_and_svg_layers(root, source, config, options, program, |c| {
            WaffleSvgFrame::new(c.config, c.source, c.options)
        })
    }
    pub fn mount_with_factory_and_svg_layers(
        root: Element,
        source: Vec<RadialDatum>,
        config: ChartConfig,
        options: WaffleLayoutOptions,
        program: BrowserWaffleSvgProgram,
        mut builder: impl FnMut(&BrowserWaffleBuildContext<'_>) -> Result<WaffleSvgFrame> + 'static,
    ) -> Result<Self> {
        Self::mount_erased(
            root,
            source,
            config,
            options,
            program,
            RawMount::default(),
            move |c, _| builder(c).map(SourceEnvelope::legacy),
        )
    }
    fn mount_erased(
        root: Element,
        source: Vec<RadialDatum>,
        mut config: ChartConfig,
        options: WaffleLayoutOptions,
        program: BrowserWaffleSvgProgram,
        raw: RawMount,
        builder: impl FnMut(
            &BrowserWaffleBuildContext<'_>,
            Option<&Rc<dyn Any>>,
        ) -> Result<SourceEnvelope>
        + 'static,
    ) -> Result<Self> {
        let document = root
            .owner_document()
            .ok_or_else(|| ChartError("Waffle document missing".into()))?;
        let window = document
            .default_view()
            .ok_or_else(|| ChartError("Waffle window missing".into()))?;
        let (w, h) = content_size(&root);
        if w > 0. && h > 0. {
            config.width = w;
            config.height = h;
        }
        let make = |class: &str, style: &str| -> Result<HtmlElement> {
            let node: HtmlElement = document
                .create_element("div")
                .map_err(host)?
                .dyn_into()
                .map_err(host)?;
            node.set_class_name(class);
            node.set_attribute("style", style).map_err(host)?;
            Ok(node)
        };
        let container = make(
            "kartis-waffle-owner",
            "position:relative;width:100%;height:100%;overflow:visible",
        )?;
        let content = make(
            "kartis-waffle",
            "position:relative;width:100%;height:100%;overflow:clip;isolation:isolate",
        )?;
        let overlay = make(
            "kartis-waffle-tooltip-overlay",
            "position:absolute;left:0;top:0;width:100%;height:100%;overflow:visible;pointer-events:none",
        )?;
        container.append_child(&content).map_err(host)?;
        container.append_child(&overlay).map_err(host)?;
        let records = Rc::new(Cell::new(0));
        let tip = tooltip::Host::mount(
            &overlay,
            &content,
            records.clone(),
            program.budget.components,
        )?;
        root.append_child(&container).map_err(host)?;
        let owner = Rc::new(Driver {
            window,
            root,
            container,
            content,
            tooltip: tip,
            events: events::Events::default(),
            alive: Cell::new(true),
            busy: Cell::new(false),
            accepted: RefCell::new(None),
            pending: RefCell::new(Vec::new()),
            builder: RefCell::new(Some(Box::new(builder))),
            raw_type: raw.kind,
            records,
            all_records: RefCell::new(Vec::new()),
            raf: RefCell::new(None),
            raf_id: Cell::new(None),
            listeners: RefCell::new(Vec::new()),
            resize: RefCell::new(None),
            error: RefCell::new(None),
            ui_errors: RefCell::new(Vec::new()),
            event_errors: RefCell::new(VecDeque::new()),
            last_hit: RefCell::new(None),
        });
        let request = Request {
            config,
            source,
            options,
            program,
            raw: raw.source,
            view: ZoomTransform::default(),
            tooltip: tooltip::Config::default(),
        };
        owner.busy.set(true);
        let mounted = owner.commit(request, true, true, None, None);
        owner.busy.set(false);
        if let Err(error) = mounted.and_then(|_| owner.bind()) {
            owner.stop();
            return Err(error);
        }
        Ok(Self { driver: owner })
    }
    pub fn handle(&self) -> BrowserWaffleHandle {
        BrowserWaffleHandle {
            driver: Rc::downgrade(&self.driver),
        }
    }
    pub fn snapshot(&self) -> Result<BrowserWaffleSnapshot> {
        self.handle().snapshot()
    }
    pub fn flush(&self) -> Result<()> {
        self.handle().flush()
    }
    pub fn stop(&self) {
        self.driver.stop();
    }
    pub fn source_generation(&self) -> Result<u64> {
        self.handle().source_generation()
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserWaffleSvgLayerFrame>> {
        self.handle().svg_layer_frame(key)
    }
    pub fn set_source(&self, v: Vec<RadialDatum>) -> Result<()> {
        self.handle().set_source(v)
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.handle().set_config(v)
    }
    pub fn set_options(&self, v: WaffleLayoutOptions) -> Result<()> {
        self.handle().set_options(v)
    }
    pub fn set_hidden(&self, v: BTreeSet<String>) -> Result<()> {
        self.handle().set_hidden(v)
    }
    pub fn set_svg_program(&self, v: BrowserWaffleSvgProgram) -> Result<()> {
        self.handle().set_svg_program(v)
    }
    pub fn set_view(&self, v: ZoomTransform) -> Result<()> {
        self.handle().set_view(v)
    }
    pub fn resize(&self, w: f64, h: f64) -> Result<()> {
        self.handle().resize(w, h)
    }
    pub fn refresh(&self) -> Result<()> {
        self.handle().refresh()
    }
    pub fn set_mouse_handlers(&self, v: BrowserWaffleMouseHandlers) -> Result<()> {
        self.handle().set_mouse_handlers(v)
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.handle().clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, v: BrowserWaffleTooltipOptions) -> Result<()> {
        self.handle().set_tooltip_options(v)
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserWaffleTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component(revision, factory)
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        self.handle().clear_tooltip_component()
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.handle().reset_tooltip()
    }
    pub fn tooltip(&self) -> Result<Option<WaffleTooltipFrame>> {
        self.handle().tooltip()
    }
}
impl Drop for BrowserWaffle {
    fn drop(&mut self) {
        self.stop();
    }
}
impl BrowserWaffleHandle {
    fn owner(&self) -> Result<Rc<Driver>> {
        self.driver
            .upgrade()
            .ok_or_else(|| ChartError("Waffle owner dropped".into()))
    }
    pub fn snapshot(&self) -> Result<BrowserWaffleSnapshot> {
        Ok(self.owner()?.snapshot())
    }
    pub fn flush(&self) -> Result<()> {
        self.owner()?.flush()
    }
    pub fn stop(&self) {
        if let Some(owner) = self.driver.upgrade() {
            owner.stop();
        }
    }
    pub fn source_generation(&self) -> Result<u64> {
        let owner = self.owner()?;
        owner.require_alive()?;
        Ok(owner
            .accepted
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("Waffle has no source".into()))?
            .generation)
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserWaffleSvgLayerFrame>> {
        let owner = self.owner()?;
        owner.require_alive()?;
        Ok(owner
            .accepted
            .borrow()
            .as_ref()
            .and_then(|a| a.stage.frames.get(key).cloned()))
    }
    pub fn set_source(&self, v: Vec<RadialDatum>) -> Result<()> {
        let owner = self.owner()?;
        scene::ensure(
            owner.raw_type.is_none(),
            "typed Waffle requires typed source",
        )?;
        owner.queue(Command::Source(v))
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.owner()?.queue(Command::Config(v))
    }
    pub fn set_options(&self, v: WaffleLayoutOptions) -> Result<()> {
        self.owner()?.queue(Command::Options(v))
    }
    pub fn set_hidden(&self, v: BTreeSet<String>) -> Result<()> {
        self.owner()?.queue(Command::Hidden(v))
    }
    pub fn set_svg_program(&self, v: BrowserWaffleSvgProgram) -> Result<()> {
        self.owner()?.queue(Command::Program(v))
    }
    pub fn set_view(&self, v: ZoomTransform) -> Result<()> {
        v.validate()?;
        self.owner()?.queue(Command::View(v))
    }
    pub fn resize(&self, w: f64, h: f64) -> Result<()> {
        self.owner()?.queue(Command::Resize(w, h))
    }
    pub fn refresh(&self) -> Result<()> {
        self.owner()?.queue(Command::Refresh)
    }
    pub fn set_mouse_handlers(&self, v: BrowserWaffleMouseHandlers) -> Result<()> {
        let o = self.owner()?;
        o.require_mutable()?;
        o.events.replace(Some(v));
        Ok(())
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        let o = self.owner()?;
        o.require_mutable()?;
        o.events.replace(None);
        Ok(())
    }
    pub fn set_tooltip_options(&self, v: BrowserWaffleTooltipOptions) -> Result<()> {
        v.validate()?;
        let o = self.owner()?;
        let mut c = o.tooltip_config()?;
        c.options = Some(v);
        o.queue(Command::Tooltip(c))
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserWaffleTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        let o = self.owner()?;
        let mut c = o.tooltip_config()?;
        c.options.get_or_insert_with(Default::default);
        c.component = Some(tooltip::legacy_component(revision, factory));
        c.kind = None;
        o.queue(Command::Tooltip(c))
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        let o = self.owner()?;
        let mut c = o.tooltip_config()?;
        c.component = None;
        c.kind = None;
        o.queue(Command::Tooltip(c))
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.owner()?.queue(Command::TooltipReset)
    }
    pub fn tooltip(&self) -> Result<Option<WaffleTooltipFrame>> {
        let o = self.owner()?;
        Ok(if o.alive.get() {
            o.tooltip.frame()
        } else {
            None
        })
    }
}

impl Driver {
    fn require_alive(&self) -> Result<()> {
        scene::ensure(self.alive.get(), "Waffle owner stopped")
    }
    fn require_mutable(&self) -> Result<()> {
        self.require_alive()?;
        scene::ensure(!self.busy.get(), "Waffle owner busy")
    }
    fn snapshot(&self) -> BrowserWaffleSnapshot {
        let a = self.accepted.borrow();
        let a = a.as_ref();
        BrowserWaffleSnapshot {
            active: self.alive.get(),
            data: a.map(|a| a.source.projection.clone()),
            presentation: a.map(|a| a.stage.plan().clone()),
            source_generation: a.map_or(0, |a| a.generation),
            publication: a.map_or(0, |a| a.publication),
            hovered: a.and_then(|a| a.hover.as_ref().map(|r| r.1.clone())),
            view: a.map_or_else(ZoomTransform::default, |a| a.request.view),
            hidden: a.map_or_else(BTreeSet::new, |a| a.request.options.hidden.clone()),
            pending: !self.pending.borrow().is_empty(),
            raf_pending: self.raf_id.get().is_some(),
            error: self.error.borrow().clone(),
            ui_errors: self.ui_errors.borrow().clone(),
            event_errors: self.event_errors.borrow().iter().cloned().collect(),
            last_hit: self.last_hit.borrow().as_ref().map(|v| v.0.clone()),
            tooltip: self.tooltip.frame(),
        }
    }
    fn queue(self: &Rc<Self>, command: Command) -> Result<()> {
        self.require_mutable()?;
        scene::ensure(
            self.pending.borrow().len() < MAX_PENDING,
            "Waffle pending budget exceeded",
        )?;
        self.pending.borrow_mut().push(command);
        if let Err(error) = self.request_raf() {
            self.pending.borrow_mut().pop();
            return Err(error);
        }
        Ok(())
    }
    fn request_raf(&self) -> Result<()> {
        if self.raf_id.get().is_none() {
            let raf = self.raf.borrow();
            let cb = raf
                .as_ref()
                .ok_or_else(|| ChartError("Waffle RAF unavailable".into()))?;
            self.raf_id.set(Some(
                self.window
                    .request_animation_frame(cb.as_ref().unchecked_ref())
                    .map_err(host)?,
            ));
        }
        Ok(())
    }
    fn flush(self: &Rc<Self>) -> Result<()> {
        self.require_mutable()?;
        scene::ensure(
            !self.events.is_dispatching(),
            "Waffle synchronous flush inside mouse handler is not allowed",
        )?;
        if let Some(id) = self.raf_id.take() {
            self.window.cancel_animation_frame(id).map_err(host)?;
        }
        let commands = std::mem::take(&mut *self.pending.borrow_mut());
        if commands.is_empty() {
            return Ok(());
        }
        self.busy.set(true);
        let result = (|| {
            let (mut request, mut hover, mut selection) = {
                let a = self.accepted.borrow();
                let a = a
                    .as_ref()
                    .ok_or_else(|| ChartError("Waffle has no source".into()))?;
                (a.request.clone(), a.hover.clone(), a.selection.clone())
            };
            let (mut build, mut present) = (false, false);
            for c in commands {
                match c {
                    Command::Source(v) => {
                        request.source = v;
                        build = true;
                    }
                    Command::TypedSource(v, raw) => {
                        request.source = v;
                        request.raw = Some(raw);
                        build = true;
                    }
                    Command::Config(v) => {
                        request.config = v;
                        build = true;
                    }
                    Command::Options(v) => {
                        request.options = v;
                        build = true;
                    }
                    Command::Hidden(v) => {
                        request.options.hidden = v;
                        build = true;
                    }
                    Command::Resize(w, h) => {
                        request.config.width = w;
                        request.config.height = h;
                        build = true;
                    }
                    Command::Refresh => build = true,
                    Command::Program(v) => {
                        request.program = v;
                        present = true;
                    }
                    Command::View(v) => {
                        request.view = v;
                        present = true;
                    }
                    Command::Native {
                        hover: h,
                        selection: s,
                    } => {
                        hover = h;
                        selection = s;
                    }
                    Command::Tooltip(v) => request.tooltip = v,
                    Command::TooltipReset => {
                        request.tooltip = tooltip::Config::default();
                        selection = None;
                    }
                }
            }
            self.commit(request, build, present || build, hover, selection)
        })();
        self.busy.set(false);
        *self.error.borrow_mut() = result.as_ref().err().cloned();
        result
    }
    fn commit(
        self: &Rc<Self>,
        request: Request,
        build: bool,
        present: bool,
        hover: Option<HoverRole>,
        selection: Option<tooltip::Selection>,
    ) -> Result<()> {
        request.program.validate()?;
        request.view.validate()?;
        waffle_svg_validate_inputs(&request.config, &request.source, &request.options)?;
        scene::ensure(
            request.options.mode == WaffleMode::Cells,
            "literal Waffle SVG requires Cells mode",
        )?;
        let previous = self.accepted.borrow().as_ref().map(|a| a.stage.clone());
        let needed = layers::additional(self, &request.program, previous.as_deref())?;
        self.tooltip
            .preflight(&request.tooltip, needed, request.program.budget.components)?;
        let source = if build {
            let mut builder = self
                .builder
                .borrow_mut()
                .take()
                .ok_or_else(|| ChartError("Waffle builder already in use".into()))?;
            let result = builder(
                &BrowserWaffleBuildContext {
                    config: &request.config,
                    source: &request.source,
                    options: &request.options,
                },
                request.raw.as_ref(),
            );
            if self.alive.get() {
                *self.builder.borrow_mut() = Some(builder);
            }
            let source = result?;
            source.validate_type(self.raw_type)?;
            scene::ensure(
                source.projection.config() == &request.config
                    && source.projection.options() == &request.options
                    && same_source(source.projection.source(), &request.source),
                "Waffle builder changed request source/config/options",
            )?;
            source
        } else {
            self.accepted
                .borrow()
                .as_ref()
                .ok_or_else(|| ChartError("Waffle has no source".into()))?
                .source
                .clone()
        };
        self.require_alive()?;
        let (generation, publication) =
            self.accepted
                .borrow()
                .as_ref()
                .map_or(Ok((0, 0)), |a| -> Result<_> {
                    Ok((
                        a.generation
                            .checked_add(u64::from(build))
                            .ok_or_else(|| ChartError("Waffle generation overflow".into()))?,
                        a.publication
                            .checked_add(u64::from(present))
                            .ok_or_else(|| ChartError("Waffle publication overflow".into()))?,
                    ))
                })?;
        let candidate = if present {
            stage::Stage::prepare(
                self,
                &request,
                source.clone(),
                generation,
                publication,
                previous.clone(),
            )?
        } else {
            previous
                .clone()
                .ok_or_else(|| ChartError("Waffle has no presentation".into()))?
        };
        // Selection is an occurrence lease, not persistent source intent. An
        // omitted/disabled area must not resurrect a tooltip when restored.
        // This changes only the candidate; refusal retains the accepted lease.
        let selection = selection
            .filter(|selection| selection.is_live(candidate.plan(), request.program.interactive));
        let tip = self.tooltip.prepare(tooltip::Prepare {
            config: &request.tooltip,
            selection: selection.as_ref(),
            plan: candidate.plan(),
            source_generation: generation,
            publication,
            hidden: &request.options.hidden,
            interactive: request.program.interactive,
            component_budget: request.program.budget.components,
            source: &source,
        })?;
        let placed = (|| {
            if present {
                candidate.place()?;
            }
            tip.place()?;
            candidate.validate(self)?;
            tip.validate()?;
            self.require_alive()
        })();
        if let Err(error) = placed {
            let mut errors = vec![error.0];
            if let Err(e) = tip.rollback() {
                errors.push(e.0);
            }
            if present && let Err(e) = candidate.rollback() {
                errors.push(e.0);
            }
            return Err(ChartError(errors.join("; ")));
        }
        if present {
            candidate.adopt(previous.as_deref());
        }
        tip.adopt();
        let hover = hover.filter(|r| request.program.interactive && candidate.contains_area(r));
        *self.accepted.borrow_mut() = Some(Accepted {
            request,
            source,
            stage: candidate.clone(),
            generation,
            publication,
            hover,
            selection,
        });
        let mut errors = if present {
            candidate.publish()
        } else {
            Vec::new()
        };
        if present {
            errors.extend(candidate.finish(previous.as_deref()));
        }
        errors.extend(tip.publish());
        errors.extend(tip.finish());
        self.ui_errors.borrow_mut().extend(errors);
        Ok(())
    }
    fn tooltip_config(&self) -> Result<tooltip::Config> {
        self.require_alive()?;
        for c in self.pending.borrow().iter().rev() {
            match c {
                Command::Tooltip(c) => return Ok(c.clone()),
                Command::TooltipReset => return Ok(tooltip::Config::default()),
                _ => {}
            }
        }
        self.accepted
            .borrow()
            .as_ref()
            .map(|a| a.request.tooltip.clone())
            .ok_or_else(|| ChartError("Waffle has no source".into()))
    }
    fn hover_intent(&self) -> Option<HoverRole> {
        for c in self.pending.borrow().iter().rev() {
            if let Command::Native { hover, .. } = c {
                return hover.clone();
            }
        }
        self.accepted
            .borrow()
            .as_ref()
            .and_then(|a| a.hover.clone())
    }
    fn native(
        self: &Rc<Self>,
        event: &Event,
        mouse: &web_sys::MouseEvent,
        kind: BrowserWaffleMouseKind,
    ) -> Result<Option<(Rc<stage::Stage>, BrowserWaffleSvgHit)>> {
        let stage = {
            let a = self.accepted.borrow();
            let Some(a) = a.as_ref().filter(|a| a.request.program.interactive) else {
                return Ok(None);
            };
            a.stage.clone()
        };
        let hit = stage.hit(event, mouse, kind == BrowserWaffleMouseKind::Leave)?;
        let Some(hit) = hit else {
            return Ok(None);
        };
        let role = (
            hit.key.clone(),
            hit.data.source()[hit.source_index].id.clone(),
            hit.polygon_index,
        );
        if kind == BrowserWaffleMouseKind::Enter && self.hover_intent().as_ref() == Some(&role) {
            return Ok(None);
        }
        if kind == BrowserWaffleMouseKind::Leave {
            if let Some(related) = mouse
                .related_target()
                .and_then(|v| v.dyn_into::<Element>().ok())
                && stage.same_area(&related, &role)
            {
                return Ok(None);
            }
            if self.hover_intent().as_ref() != Some(&role) {
                return Ok(None);
            }
            self.queue(Command::Native {
                hover: None,
                selection: None,
            })?;
        } else {
            let selection = if kind == BrowserWaffleMouseKind::Click {
                None
            } else {
                Some(tooltip::Selection::from_hit(&hit)?)
            };
            self.queue(Command::Native {
                hover: if kind == BrowserWaffleMouseKind::Click {
                    self.hover_intent()
                } else {
                    Some(role)
                },
                selection,
            })?;
        }
        *self.last_hit.borrow_mut() = Some((hit.clone(), stage.source.clone()));
        Ok(Some((stage, hit)))
    }
    fn push_event_error(&self, e: ChartError) {
        let mut errors = self.event_errors.borrow_mut();
        if errors.len() == 64 {
            errors.pop_front();
        }
        errors.push_back(e);
    }
    fn bind(self: &Rc<Self>) -> Result<()> {
        let owner = Rc::downgrade(self);
        *self.raf.borrow_mut() = Some(Closure::wrap(Box::new(move |_: f64| {
            if let Some(o) = owner.upgrade().filter(|o| o.alive.get()) {
                o.raf_id.set(None);
                let _ = o.flush();
            }
        }) as Box<dyn FnMut(f64)>));
        events::Events::bind(self)?;
        let weak = Rc::downgrade(self);
        let callback = Closure::wrap(Box::new(move |entries: web_sys::js_sys::Array| {
            let Some(o) = weak.upgrade().filter(|o| o.alive.get() && !o.busy.get()) else {
                return;
            };
            for entry in entries.iter() {
                let e: web_sys::ResizeObserverEntry = entry.unchecked_into();
                let r = e.content_rect();
                if r.width() > 0.
                    && r.height() > 0.
                    && o.accepted.borrow().as_ref().is_some_and(|a| {
                        a.request.config.width != r.width() || a.request.config.height != r.height()
                    })
                    && let Err(e) = o.queue(Command::Resize(r.width(), r.height()))
                {
                    *o.error.borrow_mut() = Some(e);
                }
            }
        }) as Box<dyn FnMut(web_sys::js_sys::Array)>);
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).map_err(host)?;
        observer.observe(&self.root);
        *self.resize.borrow_mut() = Some((observer, callback));
        Ok(())
    }
    fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        if let Some(id) = self.raf_id.take() {
            let _ = self.window.cancel_animation_frame(id);
        }
        self.listeners.borrow_mut().clear();
        self.events.stop();
        if let Some((observer, _)) = self.resize.borrow_mut().take() {
            observer.disconnect();
        }
        self.tooltip.stop();
        for record in self.all_records.borrow().iter().filter_map(Weak::upgrade) {
            record.frame.revise(|frame| *frame = None);
            if let Some(e) = record.retire() {
                self.ui_errors.borrow_mut().push(e);
            }
        }
        self.pending.borrow_mut().clear();
        self.accepted.borrow_mut().take();
        self.builder.borrow_mut().take();
        self.container.remove();
    }
}
fn content_size(root: &Element) -> (f64, f64) {
    (
        f64::from(root.client_width()),
        f64::from(root.client_height()),
    )
}
fn same_source(a: &[RadialDatum], b: &[RadialDatum]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.id == b.id && a.label == b.label && a.value.to_bits() == b.value.to_bits()
        })
}
