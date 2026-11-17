//! A dedicated SVG Funnel owner. Literal views never create a second business
//! source, and native current selection comes from actual owned body paths.
use std::any::{Any, TypeId};

use glory::Bond;

use super::{funnel_tooltip as tooltip, *};

#[path = "funnel_data.rs"]
mod data;
#[path = "funnel_events.rs"]
mod events;
#[path = "funnel_layers.rs"]
mod layers;
#[path = "funnel_motion.rs"]
mod motion;
#[path = "funnel_stage.rs"]
mod stage;
pub use data::*;
pub use events::{
    BrowserFunnelMouseActivity, BrowserFunnelMouseEvent, BrowserFunnelMouseHandlers,
    BrowserFunnelMouseInput, BrowserFunnelMouseKind, BrowserFunnelMouseOccurrence,
    BrowserFunnelMousePointer, FunnelMouseHandler,
};
pub use layers::{
    BrowserFunnelSvgLayerContext, BrowserFunnelSvgLayerFrame, BrowserFunnelSvgProgram,
};
pub use motion::{
    BrowserFunnelDataMotionMouseHandlers, BrowserFunnelMotionClock, BrowserFunnelMotionFrame,
    BrowserFunnelMotionMouseHandlers, BrowserFunnelMotionSnapshot, FunnelDataMotionMouseHandler,
    FunnelMotionMouseHandler,
};

/// Inputs to a source rebuild. Program, camera and hidden-only changes do not
/// invoke this function; the complete source frame is independent of its views.
#[derive(Clone, Copy, Debug)]
pub struct BrowserFunnelBuildContext<'a> {
    pub config: &'a ChartConfig,
    pub source: &'a [FunnelDatum],
    pub options: &'a FunnelLayoutOptions,
}

/// The original native observation. Its frame and publication are historical;
/// subsequent source adoption never rewrites an earlier observation.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserFunnelSvgHit {
    pub data: FunnelSvgFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub key: String,
    pub index: usize,
    pub part_index: usize,
    pub source_index: usize,
    pub mark_id: String,
    pub client: [f64; 2],
    pub viewport: [f64; 2],
    pub scene: [f64; 2],
    pub trusted: bool,
    pub native_type: String,
}

#[derive(Clone, Debug)]
pub struct BrowserFunnelSnapshot {
    pub active: bool,
    pub data: Option<FunnelSvgFrame>,
    pub source_generation: u64,
    pub publication: u64,
    pub current: Option<CategoryKey>,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub pending: bool,
    pub raf_pending: bool,
    pub error: Option<ChartError>,
    pub ui_errors: Vec<ChartError>,
    pub event_errors: Vec<ChartError>,
    pub last_hit: Option<BrowserFunnelSvgHit>,
    pub tooltip: Option<tooltip::FunnelTooltipFrame>,
}

/// Weak and accepted-occurrence-bound. Calls enqueue work and never flush from
/// a component callback. Constructor/pending and retired capabilities reject.
#[derive(Clone)]
pub struct BrowserFunnelCurrentPart {
    action: Rc<CurrentAction>,
}
type CurrentAction = dyn Fn(Option<&CategoryKey>) -> Result<()>;
impl BrowserFunnelCurrentPart {
    pub fn set_current_part(&self, id: Option<&CategoryKey>) -> Result<()> {
        (self.action)(id)
    }
}

type Builder =
    dyn FnMut(&BrowserFunnelBuildContext<'_>, Option<&Rc<dyn Any>>) -> Result<SourceEnvelope>;
#[derive(Default)]
struct RawMount {
    source: Option<Rc<dyn Any>>,
    kind: Option<TypeId>,
}
type ResizeListener = (ResizeObserver, Closure<dyn FnMut(web_sys::js_sys::Array)>);
type HoverRole = (String, String);

#[derive(Clone)]
struct Request {
    config: ChartConfig,
    source: Vec<FunnelDatum>,
    raw: Option<Rc<dyn Any>>,
    options: FunnelLayoutOptions,
    program: BrowserFunnelSvgProgram,
    view: ZoomTransform,
    hidden: BTreeSet<String>,
    tooltip: tooltip::Config,
    motion: motion::Policy,
}
struct Accepted {
    request: Request,
    data: FunnelSvgFrame,
    source: SourceEnvelope,
    stage: Rc<stage::Stage>,
    generation: u64,
    publication: u64,
    hover: Option<HoverRole>,
    selection: Option<tooltip::Selection>,
    motion: Option<motion::State>,
}
enum Command {
    Source(Vec<FunnelDatum>),
    TypedSource(Vec<FunnelDatum>, Rc<dyn Any>),
    Config(ChartConfig),
    Options(FunnelLayoutOptions),
    Program(BrowserFunnelSvgProgram),
    Current(Option<CategoryKey>),
    NativeCurrent {
        hover: Option<HoverRole>,
        current: Option<Option<CategoryKey>>,
        tooltip: Option<Option<tooltip::Selection>>,
    },
    TooltipConfig(tooltip::Config),
    TooltipReset,
    View(ZoomTransform),
    Hidden(BTreeSet<String>),
    Resize(f64, f64),
    Refresh,
    MotionOptions(Option<FunnelMotionOptions>),
    MotionClock(BrowserFunnelMotionClock),
    MotionPreference(MotionPreference),
    MotionStep(f64),
    MotionWake,
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
    content: HtmlElement,
    container: HtmlElement,
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
    resize: RefCell<Option<ResizeListener>>,
    error: RefCell<Option<ChartError>>,
    ui_errors: RefCell<Vec<ChartError>>,
    event_errors: RefCell<VecDeque<ChartError>>,
    last_hit: RefCell<Option<(BrowserFunnelSvgHit, SourceEnvelope)>>,
    motion_time: Cell<Option<f64>>,
}

pub struct BrowserFunnel {
    driver: Rc<Driver>,
}
#[derive(Clone)]
pub struct BrowserFunnelHandle {
    driver: Weak<Driver>,
}

impl BrowserFunnel {
    pub fn mount_with_svg_layers(
        root: Element,
        source: Vec<FunnelDatum>,
        config: ChartConfig,
        options: FunnelLayoutOptions,
        program: BrowserFunnelSvgProgram,
    ) -> Result<Self> {
        Self::mount_with_factory_and_svg_layers(root, source, config, options, program, |c| {
            FunnelSvgFrame::new(c.config, c.source, c.options)
        })
    }
    pub fn mount_with_factory_and_svg_layers(
        root: Element,
        source: Vec<FunnelDatum>,
        config: ChartConfig,
        options: FunnelLayoutOptions,
        program: BrowserFunnelSvgProgram,
        mut builder: impl FnMut(&BrowserFunnelBuildContext<'_>) -> Result<FunnelSvgFrame> + 'static,
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
        source: Vec<FunnelDatum>,
        mut config: ChartConfig,
        options: FunnelLayoutOptions,
        program: BrowserFunnelSvgProgram,
        raw: RawMount,
        builder: impl FnMut(
            &BrowserFunnelBuildContext<'_>,
            Option<&Rc<dyn Any>>,
        ) -> Result<SourceEnvelope>
        + 'static,
    ) -> Result<Self> {
        let document = root
            .owner_document()
            .ok_or_else(|| ChartError("Funnel root has no document".into()))?;
        let window = document
            .default_view()
            .ok_or_else(|| ChartError("Funnel window missing".into()))?;
        let (width, height) = content_size(&root);
        if width > 0. && height > 0. {
            config.width = width;
            config.height = height;
        }
        let container: HtmlElement = document
            .create_element("div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        container.set_class_name("kartis-funnel-owner");
        container
            .set_attribute(
                "style",
                "position:relative;width:100%;height:100%;overflow:visible",
            )
            .map_err(host)?;
        let content: HtmlElement = document
            .create_element("div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        content.set_class_name("kartis-funnel");
        content
            .set_attribute(
                "style",
                "position:relative;width:100%;height:100%;overflow:clip;isolation:isolate",
            )
            .map_err(host)?;
        let overlay: HtmlElement = document
            .create_element("div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        overlay.set_class_name("kartis-funnel-tooltip-overlay");
        overlay.set_attribute("style","position:absolute;left:0;top:0;width:100%;height:100%;pointer-events:none;overflow:visible").map_err(host)?;
        container.append_child(&content).map_err(host)?;
        container.append_child(&overlay).map_err(host)?;
        let records = Rc::new(Cell::new(0));
        let tooltip = tooltip::Host::mount(
            &overlay,
            &content,
            records.clone(),
            program.budget().components,
        )?;
        root.append_child(&container).map_err(host)?;
        let driver = Rc::new(Driver {
            window,
            root,
            content,
            container,
            tooltip,
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
            motion_time: Cell::new(None),
        });
        let request = Request {
            config,
            source,
            raw: raw.source,
            options,
            program,
            view: ZoomTransform::default(),
            hidden: BTreeSet::new(),
            tooltip: tooltip::Config::default(),
            motion: motion::Policy::default(),
        };
        driver.busy.set(true);
        let result = driver.commit(request, true, None, None, motion::Timing::default());
        driver.busy.set(false);
        if let Err(error) = result {
            driver.stop();
            return Err(error);
        }
        if let Err(error) = driver.bind() {
            driver.stop();
            return Err(error);
        }
        Ok(Self { driver })
    }
    pub fn handle(&self) -> BrowserFunnelHandle {
        BrowserFunnelHandle {
            driver: Rc::downgrade(&self.driver),
        }
    }
    pub fn snapshot(&self) -> Result<BrowserFunnelSnapshot> {
        Ok(self.driver.snapshot())
    }
    pub fn flush(&self) -> Result<()> {
        self.driver.flush()
    }
    pub fn stop(&self) {
        self.driver.stop();
    }
    pub fn source_generation(&self) -> Result<u64> {
        self.handle().source_generation()
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserFunnelSvgLayerFrame>> {
        self.handle().svg_layer_frame(key)
    }
    pub fn set_source(&self, source: Vec<FunnelDatum>) -> Result<()> {
        self.handle().set_source(source)
    }
    pub fn set_config(&self, config: ChartConfig) -> Result<()> {
        self.handle().set_config(config)
    }
    pub fn set_options(&self, options: FunnelLayoutOptions) -> Result<()> {
        self.handle().set_options(options)
    }
    pub fn set_svg_program(&self, program: BrowserFunnelSvgProgram) -> Result<()> {
        self.handle().set_svg_program(program)
    }
    pub fn set_current_part(&self, id: Option<&CategoryKey>) -> Result<()> {
        self.handle().set_current_part(id)
    }
    pub fn set_view(&self, view: ZoomTransform) -> Result<()> {
        self.handle().set_view(view)
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<()> {
        self.handle().set_hidden(hidden)
    }
    pub fn resize(&self, width: f64, height: f64) -> Result<()> {
        self.handle().resize(width, height)
    }
    pub fn refresh(&self) -> Result<()> {
        self.handle().refresh()
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserFunnelMouseHandlers) -> Result<()> {
        self.handle().set_mouse_handlers(handlers)
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.handle().clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, options: tooltip::BrowserFunnelTooltipOptions) -> Result<()> {
        self.handle().set_tooltip_options(options)
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(tooltip::BrowserFunnelTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component(revision, factory)
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        self.handle().clear_tooltip_component()
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.handle().reset_tooltip()
    }
    pub fn tooltip(&self) -> Result<Option<tooltip::FunnelTooltipFrame>> {
        self.handle().tooltip()
    }
}
impl Drop for BrowserFunnel {
    fn drop(&mut self) {
        self.stop();
    }
}
impl BrowserFunnelHandle {
    fn owner(&self) -> Result<Rc<Driver>> {
        self.driver
            .upgrade()
            .ok_or_else(|| ChartError("Funnel owner dropped".into()))
    }
    pub fn snapshot(&self) -> Result<BrowserFunnelSnapshot> {
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
        let accepted = owner.accepted.borrow();
        Ok(accepted
            .as_ref()
            .ok_or_else(|| ChartError("Funnel has no accepted frame".into()))?
            .generation)
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserFunnelSvgLayerFrame>> {
        let owner = self.owner()?;
        owner.require_alive()?;
        Ok(owner
            .accepted
            .borrow()
            .as_ref()
            .and_then(|a| a.stage.frames.get(key).cloned()))
    }
    pub fn set_source(&self, source: Vec<FunnelDatum>) -> Result<()> {
        let owner = self.owner()?;
        scene::ensure(
            owner.raw_type.is_none(),
            "typed Funnel requires typed source",
        )?;
        owner.queue(Command::Source(source))
    }
    pub fn set_config(&self, config: ChartConfig) -> Result<()> {
        self.owner()?.queue(Command::Config(config))
    }
    pub fn set_options(&self, options: FunnelLayoutOptions) -> Result<()> {
        self.owner()?.queue(Command::Options(options))
    }
    pub fn set_svg_program(&self, program: BrowserFunnelSvgProgram) -> Result<()> {
        self.owner()?.queue(Command::Program(program))
    }
    pub fn set_current_part(&self, id: Option<&CategoryKey>) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        if let Some(id) = id {
            let a = owner.accepted.borrow();
            let data = &a
                .as_ref()
                .ok_or_else(|| ChartError("Funnel has no accepted source".into()))?
                .data;
            scene::ensure(
                data.part_by_id(id)?.is_some(),
                "Funnel current part references unknown source",
            )?;
        }
        owner.queue(Command::Current(id.cloned()))
    }
    pub fn set_view(&self, view: ZoomTransform) -> Result<()> {
        view.validate()?;
        self.owner()?.queue(Command::View(view))
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<()> {
        scene::ensure(
            hidden
                .iter()
                .all(|id| !id.is_empty() && id.len() <= MAX_LABEL_BYTES),
            "invalid Funnel hidden series",
        )?;
        self.owner()?.queue(Command::Hidden(hidden))
    }
    pub fn resize(&self, width: f64, height: f64) -> Result<()> {
        self.owner()?.queue(Command::Resize(width, height))
    }
    pub fn refresh(&self) -> Result<()> {
        self.owner()?.queue(Command::Refresh)
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserFunnelMouseHandlers) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        owner.events.replace(Some(handlers));
        Ok(())
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        owner.events.replace(None);
        Ok(())
    }
    pub fn set_tooltip_options(&self, options: tooltip::BrowserFunnelTooltipOptions) -> Result<()> {
        let owner = self.owner()?;
        let mut config = owner.tooltip_config()?;
        config.options = Some(options);
        owner.queue(Command::TooltipConfig(config))
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(tooltip::BrowserFunnelTooltipComponentContext) -> Result<W> + 'static,
    ) -> Result<()> {
        let owner = self.owner()?;
        let mut config = owner.tooltip_config()?;
        config.options.get_or_insert_with(Default::default);
        config.component = Some(tooltip::legacy_component(revision, factory));
        config.kind = None;
        owner.queue(Command::TooltipConfig(config))
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        let owner = self.owner()?;
        let mut config = owner.tooltip_config()?;
        config.component = None;
        config.kind = None;
        owner.queue(Command::TooltipConfig(config))
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.owner()?.queue(Command::TooltipReset)
    }
    pub fn tooltip(&self) -> Result<Option<tooltip::FunnelTooltipFrame>> {
        let owner = self.owner()?;
        Ok(if owner.alive.get() {
            owner.tooltip.frame()
        } else {
            None
        })
    }
}

impl Driver {
    fn require_alive(&self) -> Result<()> {
        scene::ensure(self.alive.get(), "Funnel owner stopped")
    }
    fn require_mutable(&self) -> Result<()> {
        self.require_alive()?;
        scene::ensure(!self.busy.get(), "Funnel owner is busy")
    }
    fn snapshot(&self) -> BrowserFunnelSnapshot {
        let a = self.accepted.borrow();
        BrowserFunnelSnapshot {
            active: self.alive.get(),
            data: a.as_ref().map(|a| a.data.clone()),
            source_generation: a.as_ref().map_or(0, |a| a.generation),
            publication: a.as_ref().map_or(0, |a| a.publication),
            current: a.as_ref().and_then(|a| a.request.options.current.clone()),
            view: a
                .as_ref()
                .map_or_else(ZoomTransform::default, |a| a.request.view),
            hidden: a
                .as_ref()
                .map_or_else(BTreeSet::new, |a| a.request.hidden.clone()),
            pending: !self.pending.borrow().is_empty(),
            raf_pending: self.raf_id.get().is_some(),
            error: self.error.borrow().clone(),
            ui_errors: self.ui_errors.borrow().clone(),
            event_errors: self.event_errors.borrow().iter().cloned().collect(),
            last_hit: self.last_hit.borrow().as_ref().map(|(hit, _)| hit.clone()),
            tooltip: self.tooltip.frame(),
        }
    }
    fn queue(self: &Rc<Self>, command: Command) -> Result<()> {
        self.require_mutable()?;
        scene::ensure(
            self.pending.borrow().len() < MAX_PENDING,
            "Funnel pending input budget exceeded",
        )?;
        self.pending.borrow_mut().push(command);
        if let Err(error) = self.request_raf() {
            self.pending.borrow_mut().pop();
            return Err(error);
        }
        Ok(())
    }
    fn request_raf(self: &Rc<Self>) -> Result<()> {
        if self.raf_id.get().is_some() {
            return Ok(());
        }
        let callbacks = self.raf.borrow();
        let callback = callbacks
            .as_ref()
            .ok_or_else(|| ChartError("Funnel RAF callback unavailable".into()))?;
        self.raf_id.set(Some(
            self.window
                .request_animation_frame(callback.as_ref().unchecked_ref())
                .map_err(host)?,
        ));
        Ok(())
    }
    fn flush(self: &Rc<Self>) -> Result<()> {
        self.flush_at(None)
    }
    fn flush_at(self: &Rc<Self>, timestamp: Option<f64>) -> Result<()> {
        self.require_mutable()?;
        scene::ensure(
            !self.events.is_dispatching(),
            "Funnel synchronous flush inside mouse handler is not allowed",
        )?;
        if let Some(id) = self.raf_id.take() {
            self.window.cancel_animation_frame(id).map_err(host)?;
        }
        let commands = std::mem::take(&mut *self.pending.borrow_mut());
        if commands.is_empty() && !(timestamp.is_some() && self.motion_running()) {
            return Ok(());
        }
        self.busy.set(true);
        let result = (|| {
            let (mut request, mut hover, mut selection) = {
                let accepted = self.accepted.borrow();
                let accepted = accepted
                    .as_ref()
                    .ok_or_else(|| ChartError("Funnel has no accepted request".into()))?;
                (
                    accepted.request.clone(),
                    accepted.hover.clone(),
                    accepted.selection.clone(),
                )
            };
            let mut build = false;
            let mut source_stage = false;
            let mut step = None;
            let mut rebase = false;
            for command in commands {
                match command {
                    Command::Source(source) => {
                        request.source = source;
                        request.raw = None;
                        build = true;
                    }
                    Command::TypedSource(source, raw) => {
                        request.source = source;
                        request.raw = Some(raw);
                        build = true;
                    }
                    Command::Config(config) => {
                        request.config = config;
                        build = true;
                    }
                    Command::Options(options) => {
                        request.options = options;
                        build = true;
                    }
                    Command::Program(program) => {
                        request.program = program;
                        source_stage = true;
                    }
                    Command::Current(current) => {
                        if !same_current(&request.options.current, &current) {
                            request.options.current = current;
                            build = true;
                        }
                    }
                    Command::NativeCurrent {
                        hover: role,
                        current,
                        tooltip,
                    } => {
                        hover = role;
                        if let Some(value) = tooltip {
                            selection = value;
                        }
                        if let Some(current) = current
                            && !same_current(&request.options.current, &current)
                        {
                            request.options.current = current;
                            build = true;
                        }
                    }
                    Command::View(view) => {
                        request.view = view;
                        source_stage = true;
                    }
                    Command::Hidden(hidden) => {
                        request.hidden = hidden;
                        source_stage = true;
                    }
                    Command::TooltipConfig(config) => request.tooltip = config,
                    Command::TooltipReset => {
                        request.tooltip = tooltip::Config::default();
                        selection = None;
                    }
                    Command::Resize(width, height) => {
                        if request.config.width != width || request.config.height != height {
                            request.config.width = width;
                            request.config.height = height;
                            build = true;
                        }
                    }
                    Command::Refresh => build = true,
                    Command::MotionOptions(options) => {
                        request.motion.options = options;
                        request.motion.changed()?;
                        source_stage = true;
                    }
                    Command::MotionClock(clock) => {
                        request.motion.clock = clock;
                        request.motion.changed()?;
                        source_stage = true;
                    }
                    Command::MotionPreference(preference) => {
                        request.motion.preference = preference;
                        request.motion.changed()?;
                        source_stage = true;
                    }
                    Command::MotionWake => {
                        source_stage = true;
                        rebase = true;
                    }
                    Command::MotionStep(seconds) => {
                        scene::ensure(step.is_none(), "Funnel motion step already pending")?;
                        step = Some(seconds);
                        source_stage = true;
                    }
                }
            }
            if step.is_some() {
                scene::ensure(
                    request.motion.options.is_some(),
                    "Funnel motion is not enabled",
                )?;
            }
            if build || source_stage || timestamp.is_some() && self.motion_running() {
                self.commit(
                    request,
                    build,
                    hover,
                    selection,
                    motion::Timing {
                        seconds: step,
                        timestamp,
                        rebase,
                    },
                )
            } else {
                self.commit_tooltip(request, hover, selection)
            }
        })();
        self.busy.set(false);
        *self.error.borrow_mut() = result.as_ref().err().cloned();
        result
    }
    fn commit(
        self: &Rc<Self>,
        request: Request,
        build: bool,
        hover: Option<HoverRole>,
        selection: Option<tooltip::Selection>,
        timing: motion::Timing,
    ) -> Result<()> {
        self.require_alive()?;
        request.program.validate()?;
        if let Some(options) = &request.motion.options {
            options.validate()?;
        }
        let old_stage = self.accepted.borrow().as_ref().map(|a| a.stage.clone());
        layers::preflight(self, &request.program, old_stage.as_deref())?;
        scene::ensure(
            request.tooltip.kind.is_none() || request.tooltip.kind == self.raw_type,
            "Funnel tooltip data type differs from owner",
        )?;
        self.tooltip.preflight(
            &request.tooltip,
            additional_layer_records(&request.program, old_stage.as_deref())?,
            request.program.budget().components,
        )?;
        funnel_svg_validate_inputs(&request.config, &request.source, &request.options)?;
        let mut data = if build {
            let mut builder = self
                .builder
                .borrow_mut()
                .take()
                .ok_or_else(|| ChartError("Funnel builder is already in use".into()))?;
            let built = builder(
                &BrowserFunnelBuildContext {
                    config: &request.config,
                    source: &request.source,
                    options: &request.options,
                },
                request.raw.as_ref(),
            );
            *self.builder.borrow_mut() = Some(builder);
            let frame = built?;
            frame.validate_type(self.raw_type)?;
            self.require_alive()?;
            scene::ensure(
                frame.projection.config() == &request.config
                    && frame.projection.options() == &request.options
                    && same_current(
                        &frame.projection.options().current,
                        &request.options.current,
                    )
                    && same_source(frame.projection.source(), &request.source),
                "Funnel builder returned a different source/config/options cohort",
            )?;
            frame
        } else {
            self.accepted
                .borrow()
                .as_ref()
                .ok_or_else(|| ChartError("Funnel has no accepted frame".into()))?
                .source
                .clone()
        };
        let (generation, publication, previous) = {
            let old = self.accepted.borrow();
            match old.as_ref() {
                Some(a) => (
                    if build {
                        a.generation
                            .checked_add(1)
                            .ok_or_else(|| ChartError("Funnel generation overflow".into()))?
                    } else {
                        a.generation
                    },
                    a.publication
                        .checked_add(1)
                        .ok_or_else(|| ChartError("Funnel publication overflow".into()))?,
                    Some(a.stage.clone()),
                ),
                None => (0, 0, None),
            }
        };
        let motion = motion::prepare(self, &request, &data, build, &timing)?;
        data.motion = motion.as_ref().map(|state| BrowserFunnelMotionFrame {
            data: state.model.frame().clone(),
            source_generation: generation,
            publication,
            configuration: request.motion.configuration,
        });
        let candidate = stage::Stage::prepare(
            self,
            &request,
            data.clone(),
            generation,
            publication,
            previous.clone(),
        )?;
        let tip = self.tooltip.prepare(tooltip::Prepare {
            config: &request.tooltip,
            selection: selection.as_ref(),
            plan: candidate.plan(),
            source_generation: generation,
            publication,
            hidden: &request.hidden,
            interactive: request.program.is_interactive(),
            component_budget: request.program.budget().components,
            source: &data,
        })?;
        candidate.place()?;
        let placed = tip
            .place()
            .and_then(|_| candidate.validate(self))
            .and_then(|_| tip.validate())
            .and_then(|_| self.require_alive());
        if let Err(error) = placed {
            let mut errors = vec![error.0];
            if let Err(error) = tip.rollback() {
                errors.push(error.0);
            }
            if let Err(error) = candidate.rollback() {
                errors.push(error.0);
            }
            return Err(ChartError(errors.join("; ")));
        }
        self.require_alive()?;
        // No user function, native write or runtime pump between these updates.
        candidate.adopt(previous.as_deref());
        tip.adopt();
        let hover = hover.filter(|role| candidate.contains_body(role));
        *self.accepted.borrow_mut() = Some(Accepted {
            request,
            data: data.projection.clone(),
            source: data,
            stage: candidate.clone(),
            generation,
            publication,
            hover,
            selection,
            motion,
        });
        self.motion_time
            .set(timing.timestamp.filter(|_| self.motion_visible()));
        let mut errors = candidate.publish();
        errors.extend(candidate.finish(previous.as_deref()));
        errors.extend(tip.publish());
        errors.extend(tip.finish());
        self.ui_errors.borrow_mut().extend(errors);
        if self.motion_running()
            && let Err(error) = self.request_raf()
        {
            // Data and every Cage are already accepted; scheduling failure is
            // a postcommit UI error, never a pretend source rollback.
            self.ui_errors.borrow_mut().push(error);
        }
        Ok(())
    }
    fn commit_tooltip(
        self: &Rc<Self>,
        request: Request,
        hover: Option<HoverRole>,
        selection: Option<tooltip::Selection>,
    ) -> Result<()> {
        self.require_alive()?;
        let (data, stage, generation, publication, motion) = {
            let accepted = self.accepted.borrow();
            let accepted = accepted
                .as_ref()
                .ok_or_else(|| ChartError("Funnel has no accepted source".into()))?;
            (
                accepted.source.clone(),
                accepted.stage.clone(),
                accepted.generation,
                accepted.publication,
                accepted.motion.clone(),
            )
        };
        self.tooltip
            .preflight(&request.tooltip, 0, request.program.budget().components)?;
        let tip = self.tooltip.prepare(tooltip::Prepare {
            config: &request.tooltip,
            selection: selection.as_ref(),
            plan: stage.plan(),
            source_generation: generation,
            publication,
            hidden: &request.hidden,
            interactive: request.program.is_interactive(),
            component_budget: request.program.budget().components,
            source: &data,
        })?;
        let placed = tip
            .place()
            .and_then(|_| tip.validate())
            .and_then(|_| self.require_alive());
        if let Err(error) = placed {
            return match tip.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => Err(ChartError(format!(
                    "{}; tooltip rollback: {}",
                    error.0, rollback.0
                ))),
            };
        }
        tip.adopt();
        let hover = hover.filter(|role| stage.contains_body(role));
        *self.accepted.borrow_mut() = Some(Accepted {
            request,
            data: data.projection.clone(),
            source: data,
            stage,
            generation,
            publication,
            hover,
            selection,
            motion,
        });
        let mut errors = tip.publish();
        errors.extend(tip.finish());
        self.ui_errors.borrow_mut().extend(errors);
        Ok(())
    }
    fn bind(self: &Rc<Self>) -> Result<()> {
        let owner = Rc::downgrade(self);
        *self.raf.borrow_mut() = Some(Closure::wrap(Box::new(move |time: f64| {
            if let Some(driver) = owner.upgrade().filter(|d| d.alive.get()) {
                driver.raf_id.set(None);
                let _ = driver.flush_at(Some(time));
            }
        }) as Box<dyn FnMut(f64)>));
        events::Events::bind(self)?;
        self.bind_motion()?;
        let owner = Rc::downgrade(self);
        let callback = Closure::wrap(Box::new(move |entries: web_sys::js_sys::Array| {
            let Some(owner) = owner.upgrade().filter(|d| d.alive.get() && !d.busy.get()) else {
                return;
            };
            for entry in entries.iter() {
                let entry: web_sys::ResizeObserverEntry = entry.unchecked_into();
                let rect = entry.content_rect();
                let (width, height) = (rect.width(), rect.height());
                if width > 0.
                    && height > 0.
                    && owner.accepted.borrow().as_ref().is_some_and(|a| {
                        a.request.config.width != width || a.request.config.height != height
                    })
                    && let Err(error) = owner.queue(Command::Resize(width, height))
                {
                    *owner.error.borrow_mut() = Some(error);
                }
                if width > 0.
                    && height > 0.
                    && owner
                        .accepted
                        .borrow()
                        .as_ref()
                        .is_some_and(|a| a.request.motion.options.is_some())
                    && let Err(error) = owner.queue(Command::MotionWake)
                {
                    *owner.error.borrow_mut() = Some(error);
                }
            }
        }) as Box<dyn FnMut(web_sys::js_sys::Array)>);
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).map_err(host)?;
        observer.observe(&self.root);
        *self.resize.borrow_mut() = Some((observer, callback));
        Ok(())
    }
    fn reserve_mouse(
        self: &Rc<Self>,
        event: &Event,
        mouse: &web_sys::MouseEvent,
        kind: BrowserFunnelMouseKind,
        activity: BrowserFunnelMouseActivity,
    ) -> Result<Option<(Rc<stage::Stage>, BrowserFunnelSvgHit)>> {
        let stage = {
            let a = self.accepted.borrow();
            let Some(a) = a.as_ref().filter(|a| a.request.program.is_interactive()) else {
                return Ok(None);
            };
            a.stage.clone()
        };
        if kind == BrowserFunnelMouseKind::Leave {
            // Ownership of an actual Out target is independent of its new
            // (possibly outside-window) client coordinates.
            let Some(role) = stage.leave_role(event) else {
                return Ok(None);
            };
            let related = mouse
                .related_target()
                .and_then(|t| t.dyn_into::<Element>().ok());
            let same = related
                .as_ref()
                .is_some_and(|element| stage.same_body(element, &role));
            if same {
                return Ok(None);
            }
            let hover = self.hover_intent();
            let current = if hover.as_ref() == Some(&role)
                || hover.is_none() && activity == BrowserFunnelMouseActivity::NativeComponent
            {
                Some(None)
            } else {
                None
            };
            let tip = self
                .tooltip_selection()
                .filter(|selection| selection.matches(&role.0, &role.1))
                .map(|_| None);
            if hover.as_ref() == Some(&role) || current.is_some() || tip.is_some() {
                self.queue_native(hover.filter(|hover| *hover != role), current, tip)?;
            }
            return Ok(stage.mouse_hit(event, mouse, true)?.map(|hit| (stage, hit)));
        }
        let hit = if kind == BrowserFunnelMouseKind::Click && mouse.detail() == 0 {
            stage.mouse_hit(event, mouse, true)?
        } else {
            stage.hit(event, mouse)?
        };
        if let Some(hit) = hit {
            let role = (hit.key.clone(), hit.mark_id.clone());
            let enter = self.hover_intent().as_ref() != Some(&role);
            if kind == BrowserFunnelMouseKind::Enter
                && (!enter
                    || mouse
                        .related_target()
                        .and_then(|target| target.dyn_into::<Element>().ok())
                        .is_some_and(|related| stage.same_body(&related, &role)))
            {
                // Source-driven native replacement can deliver a fresh Over
                // without leaving this logical occurrence/body. It is not a
                // second component Enter and must not overwrite manual current.
                // A subsequent actual Move remains an independent observation.
                return Ok(None);
            }
            *self.last_hit.borrow_mut() = Some((hit.clone(), stage.source.clone()));
            if kind != BrowserFunnelMouseKind::Click {
                let set_current = kind == BrowserFunnelMouseKind::Enter
                    || activity == BrowserFunnelMouseActivity::Owner && enter;
                let current =
                    set_current.then(|| Some(hit.data.parts()[hit.part_index].datum.id.clone()));
                let config = self.tooltip_config()?;
                let tip = if config.enabled() {
                    Some(Some(tooltip::Selection::from_hit(&hit)?))
                } else {
                    None
                };
                self.queue_native(Some(role), current, tip)?;
            }
            Ok(Some((stage, hit)))
        } else {
            if kind == BrowserFunnelMouseKind::Move
                && (self.hover_intent().is_some() || self.tooltip_selection().is_some())
            {
                let current = (activity == BrowserFunnelMouseActivity::Owner
                    && self.hover_intent().is_some())
                .then_some(None);
                let tip = self.tooltip_selection().is_some().then_some(None);
                self.queue_native(None, current, tip)?;
            }
            Ok(None)
        }
    }
    fn queue_native(
        self: &Rc<Self>,
        hover: Option<HoverRole>,
        current: Option<Option<CategoryKey>>,
        tooltip: Option<Option<tooltip::Selection>>,
    ) -> Result<()> {
        let changed_current = current
            .as_ref()
            .is_some_and(|value| !same_current(&self.current_intent(), value));
        if self.hover_intent() != hover || changed_current || tooltip.is_some() {
            self.queue(Command::NativeCurrent {
                hover,
                current,
                tooltip,
            })?;
        }
        Ok(())
    }
    fn current_intent(&self) -> Option<CategoryKey> {
        self.pending
            .borrow()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Current(current) => Some(current.clone()),
                Command::NativeCurrent {
                    current: Some(current),
                    ..
                } => Some(current.clone()),
                Command::Options(options) => Some(options.current.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                self.accepted
                    .borrow()
                    .as_ref()
                    .and_then(|accepted| accepted.request.options.current.clone())
            })
    }
    fn tooltip_config(&self) -> Result<tooltip::Config> {
        self.require_mutable()?;
        self.pending
            .borrow()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::TooltipConfig(config) => Some(config.clone()),
                Command::TooltipReset => Some(tooltip::Config::default()),
                _ => None,
            })
            .or_else(|| {
                self.accepted
                    .borrow()
                    .as_ref()
                    .map(|accepted| accepted.request.tooltip.clone())
            })
            .ok_or_else(|| ChartError("Funnel has no accepted tooltip configuration".into()))
    }
    fn tooltip_selection(&self) -> Option<tooltip::Selection> {
        self.pending
            .borrow()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::NativeCurrent {
                    tooltip: Some(selection),
                    ..
                } => Some(selection.clone()),
                Command::TooltipReset => Some(None),
                _ => None,
            })
            .unwrap_or_else(|| {
                self.accepted
                    .borrow()
                    .as_ref()
                    .and_then(|accepted| accepted.selection.clone())
            })
    }
    fn record_event_error(&self, error: ChartError) {
        let mut errors = self.event_errors.borrow_mut();
        if errors.len() == MAX_PENDING {
            errors.pop_front();
        }
        errors.push_back(error);
    }
    fn hover_intent(&self) -> Option<HoverRole> {
        self.pending
            .borrow()
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::NativeCurrent { hover, .. } => Some(hover.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                self.accepted
                    .borrow()
                    .as_ref()
                    .and_then(|accepted| accepted.hover.clone())
            })
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
        self.pending.borrow_mut().clear();
        self.tooltip.stop();
        for record in self.all_records.borrow().iter().filter_map(Weak::upgrade) {
            if let Some(error) = record.retire() {
                self.ui_errors.borrow_mut().push(error);
            }
        }
        self.container.remove();
    }
}
fn content_size(root: &Element) -> (f64, f64) {
    let Some(element) = root.dyn_ref::<HtmlElement>() else {
        return (0., 0.);
    };
    (
        f64::from(element.client_width()),
        f64::from(element.client_height()),
    )
}
fn additional_layer_records(
    program: &BrowserFunnelSvgProgram,
    previous: Option<&stage::Stage>,
) -> Result<usize> {
    let mut needed = 0usize;
    for layer in program.layers() {
        if let FunnelSvgLayerKind::Custom(registry) = &layer.kind {
            let component = &program.custom[registry];
            let retained = previous
                .and_then(|stage| stage.records.get(&layer.key))
                .is_some_and(|record| {
                    record.registry == *registry && record.component.revision == component.revision
                });
            needed = needed
                .checked_add(usize::from(!retained))
                .ok_or_else(|| ChartError("Funnel component count overflow".into()))?;
        }
    }
    Ok(needed)
}
fn same_current(a: &Option<CategoryKey>, b: &Option<CategoryKey>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => crate::widget_source_eq::category(a, b),
        (None, None) => true,
        _ => false,
    }
}
fn same_source(a: &[FunnelDatum], b: &[FunnelDatum]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a == b
                && crate::widget_source_eq::category(&a.id, &b.id)
                && crate::widget_source_eq::number(a.value, b.value)
                && crate::widget_source_eq::optional(a.size, b.size)
                && crate::widget_source_eq::paint(a.paint.as_ref(), b.paint.as_ref())
        })
}
