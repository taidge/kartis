//! Funnel property motion uses one candidate model and the owner's ordinary
//! journaled Stage. Target data is never replaced with a sampled fake Frame.
use super::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserFunnelMotionClock {
    #[default]
    Automatic,
    Manual,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserFunnelMotionFrame {
    pub data: FunnelMotionFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub configuration: u64,
}
#[derive(Clone, Debug)]
pub struct BrowserFunnelMotionSnapshot {
    pub active: bool,
    pub enabled: bool,
    pub clock: BrowserFunnelMotionClock,
    pub preference: MotionPreference,
    pub reduced: bool,
    pub configuration: u64,
    pub frame: Option<BrowserFunnelMotionFrame>,
    pub raf_pending: bool,
}
#[derive(Clone, Default)]
pub(super) struct Policy {
    pub options: Option<FunnelMotionOptions>,
    pub clock: BrowserFunnelMotionClock,
    pub preference: MotionPreference,
    pub configuration: u64,
}
impl Policy {
    pub fn changed(&mut self) -> Result<()> {
        self.configuration = self
            .configuration
            .checked_add(1)
            .ok_or_else(|| ChartError("Funnel motion configuration exhausted".into()))?;
        Ok(())
    }
}
#[derive(Clone)]
pub(super) struct State {
    pub model: FunnelMotion,
    pub reduced: bool,
}
#[derive(Default)]
pub(super) struct Timing {
    pub seconds: Option<f64>,
    pub timestamp: Option<f64>,
    pub rebase: bool,
}

pub(super) fn prepare(
    owner: &Driver,
    request: &Request,
    source: &SourceEnvelope,
    rebuild: bool,
    timing: &Timing,
) -> Result<Option<State>> {
    let Some(options) = &request.motion.options else {
        return Ok(None);
    };
    let old = owner.accepted.borrow();
    let mut model = match old.as_ref().and_then(|a| a.motion.as_ref()) {
        Some(state) => state.model.clone(),
        None => FunnelMotion::new(source.projection.clone(), options.clone())?,
    };
    if model.options() != options {
        model.set_options(options.clone())?;
    }
    let system = owner
        .window
        .match_media("(prefers-reduced-motion: reduce)")
        .map_err(host)?
        .is_some_and(|query| query.matches());
    let reduced = request.motion.preference == MotionPreference::Reduced
        || request.motion.preference == MotionPreference::System && system;
    if model.reduced_motion() != reduced {
        model.set_reduced_motion(reduced)?;
    }
    if rebuild || model.frame().target() != &source.projection {
        model.retarget(source.projection.clone())?;
    }
    if let Some(seconds) = timing.seconds {
        scene::ensure(
            request.motion.clock == BrowserFunnelMotionClock::Manual,
            "explicit Funnel motion intervals require Manual clock",
        )?;
        model.tick(seconds)?;
    } else if request.motion.clock == BrowserFunnelMotionClock::Automatic
        && owner.motion_visible()
        && let Some(timestamp) = timing.timestamp
    {
        let same_clock = old.as_ref().is_some_and(|a| {
            a.request.motion.clock == request.motion.clock
                && a.request.motion.preference == request.motion.preference
                && a.request.motion.configuration == request.motion.configuration
        });
        let was_settled = old
            .as_ref()
            .and_then(|a| a.motion.as_ref())
            .is_none_or(|m| m.model.frame().is_settled());
        let dt = if timing.rebase || !same_clock || rebuild && was_settled {
            0.
        } else {
            owner.motion_time.get().map_or(0., |previous| {
                ((timestamp - previous) / 1000.).clamp(0., 0.25)
            })
        };
        model.tick(dt)?;
    }
    Ok(Some(State { model, reduced }))
}

impl Driver {
    pub(super) fn motion_visible(&self) -> bool {
        if !self.content.is_connected() || self.content.owner_document().is_none_or(|d| d.hidden())
        {
            return false;
        }
        let b = self.content.get_bounding_client_rect();
        b.width() > 0. && b.height() > 0.
    }
    pub(super) fn motion_running(&self) -> bool {
        self.alive.get()
            && self.motion_visible()
            && self.accepted.borrow().as_ref().is_some_and(|a| {
                a.request.motion.clock == BrowserFunnelMotionClock::Automatic
                    && a.motion
                        .as_ref()
                        .is_some_and(|m| !m.model.frame().is_settled())
            })
    }
    pub(super) fn motion_snapshot(&self) -> BrowserFunnelMotionSnapshot {
        let accepted = self.accepted.borrow();
        let policy = accepted
            .as_ref()
            .map(|a| a.request.motion.clone())
            .unwrap_or_default();
        BrowserFunnelMotionSnapshot {
            active: self.alive.get(),
            enabled: policy.options.is_some(),
            clock: policy.clock,
            preference: policy.preference,
            reduced: accepted
                .as_ref()
                .and_then(|a| a.motion.as_ref())
                .is_some_and(|s| s.reduced),
            configuration: policy.configuration,
            frame: if self.alive.get() {
                accepted.as_ref().and_then(|a| a.source.motion.clone())
            } else {
                None
            },
            raf_pending: self.raf_id.get().is_some(),
        }
    }
    pub(super) fn bind_motion(self: &Rc<Self>) -> Result<()> {
        let document = self
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Funnel motion document missing".into()))?;
        let query = self
            .window
            .match_media("(prefers-reduced-motion: reduce)")
            .map_err(host)?;
        for (target, name) in std::iter::once((EventTarget::from(document), "visibilitychange"))
            .chain(query.map(|query| (EventTarget::from(query), "change")))
        {
            let weak = Rc::downgrade(self);
            let callback = Closure::wrap(Box::new(move |_: Event| {
                if let Some(owner) = weak.upgrade().filter(|o| o.alive.get())
                    && owner
                        .accepted
                        .borrow()
                        .as_ref()
                        .is_some_and(|a| a.request.motion.options.is_some())
                    && owner.motion_visible()
                    && let Err(error) = owner.queue(Command::MotionWake)
                {
                    *owner.error.borrow_mut() = Some(error);
                }
            }) as Box<dyn Fn(Event)>);
            target
                .add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())
                .map_err(host)?;
            self.listeners.borrow_mut().push(Listener {
                target,
                name,
                callback,
                capture: false,
            });
        }
        Ok(())
    }
}

impl BrowserFunnelHandle {
    pub fn enable_motion(&self, options: FunnelMotionOptions) -> Result<()> {
        self.set_motion_options(Some(options))
    }
    pub fn set_motion_options(&self, options: Option<FunnelMotionOptions>) -> Result<()> {
        self.owner()?.queue(Command::MotionOptions(options))
    }
    pub fn set_motion_clock(&self, clock: BrowserFunnelMotionClock) -> Result<()> {
        self.owner()?.queue(Command::MotionClock(clock))
    }
    pub fn set_motion_preference(&self, preference: MotionPreference) -> Result<()> {
        self.owner()?.queue(Command::MotionPreference(preference))
    }
    pub fn motion_clock(&self) -> Result<BrowserFunnelMotionClock> {
        Ok(self.motion_snapshot()?.clock)
    }
    pub fn step_motion(&self, seconds: f64) -> Result<()> {
        scene::ensure(
            seconds.is_finite() && (0. ..=60.).contains(&seconds),
            "Funnel motion dt must be finite and in 0..=60 seconds",
        )?;
        let owner = self.owner()?;
        owner.require_mutable()?;
        scene::ensure(
            !owner.events.is_dispatching(),
            "Funnel motion step inside mouse handler is not allowed",
        )?;
        owner.queue(Command::MotionStep(seconds))?;
        owner.flush()
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserFunnelMotionFrame>> {
        Ok(self.motion_snapshot()?.frame)
    }
    pub fn motion_snapshot(&self) -> Result<BrowserFunnelMotionSnapshot> {
        Ok(self.owner()?.motion_snapshot())
    }
}
impl BrowserFunnel {
    pub fn enable_motion(&self, o: FunnelMotionOptions) -> Result<()> {
        self.handle().enable_motion(o)
    }
    pub fn set_motion_options(&self, o: Option<FunnelMotionOptions>) -> Result<()> {
        self.handle().set_motion_options(o)
    }
    pub fn set_motion_clock(&self, c: BrowserFunnelMotionClock) -> Result<()> {
        self.handle().set_motion_clock(c)
    }
    pub fn motion_clock(&self) -> Result<BrowserFunnelMotionClock> {
        self.handle().motion_clock()
    }
    pub fn set_motion_preference(&self, p: MotionPreference) -> Result<()> {
        self.handle().set_motion_preference(p)
    }
    pub fn step_motion(&self, s: f64) -> Result<()> {
        self.handle().step_motion(s)
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserFunnelMotionFrame>> {
        self.handle().motion_frame()
    }
    pub fn motion_snapshot(&self) -> Result<BrowserFunnelMotionSnapshot> {
        self.handle().motion_snapshot()
    }
}

pub type FunnelMotionMouseHandler = Box<
    dyn FnMut(BrowserFunnelMouseEvent, Option<BrowserFunnelMotionFrame>) -> Result<()> + 'static,
>;
pub type FunnelDataMotionMouseHandler<D> = Box<
    dyn FnMut(BrowserFunnelDataMouseEvent<D>, Option<BrowserFunnelMotionFrame>) -> Result<()>
        + 'static,
>;
#[derive(Default)]
pub struct BrowserFunnelMotionMouseHandlers {
    pub on_mouse_enter: Option<FunnelMotionMouseHandler>,
    pub on_mouse_move: Option<FunnelMotionMouseHandler>,
    pub on_mouse_leave: Option<FunnelMotionMouseHandler>,
    pub on_click: Option<FunnelMotionMouseHandler>,
    pub activity: BrowserFunnelMouseActivity,
}
pub struct BrowserFunnelDataMotionMouseHandlers<D> {
    pub on_mouse_enter: Option<FunnelDataMotionMouseHandler<D>>,
    pub on_mouse_move: Option<FunnelDataMotionMouseHandler<D>>,
    pub on_mouse_leave: Option<FunnelDataMotionMouseHandler<D>>,
    pub on_click: Option<FunnelDataMotionMouseHandler<D>>,
    pub activity: BrowserFunnelMouseActivity,
}
impl<D> Default for BrowserFunnelDataMotionMouseHandlers<D> {
    fn default() -> Self {
        Self {
            on_mouse_enter: None,
            on_mouse_move: None,
            on_mouse_leave: None,
            on_click: None,
            activity: Default::default(),
        }
    }
}

impl BrowserFunnelHandle {
    pub fn set_mouse_handlers_with_motion(
        &self,
        handlers: BrowserFunnelMotionMouseHandlers,
    ) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        let mut slots: BTreeMap<BrowserFunnelMouseKind, data::ErasedMouseHandler> = BTreeMap::new();
        for (kind, slot) in [
            (BrowserFunnelMouseKind::Enter, handlers.on_mouse_enter),
            (BrowserFunnelMouseKind::Move, handlers.on_mouse_move),
            (BrowserFunnelMouseKind::Leave, handlers.on_mouse_leave),
            (BrowserFunnelMouseKind::Click, handlers.on_click),
        ] {
            if let Some(mut callback) = slot {
                slots.insert(
                    kind,
                    Box::new(move |event, source| callback(event, source.motion.clone())),
                );
            }
        }
        owner.events.replace_erased(handlers.activity, slots);
        Ok(())
    }
    pub fn set_tooltip_component_with_motion<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        mut factory: impl FnMut(
            tooltip::BrowserFunnelTooltipComponentContext,
            Bond<Option<BrowserFunnelMotionFrame>>,
        ) -> Result<W>
        + 'static,
    ) -> Result<()> {
        let owner = self.owner()?;
        let mut config = owner.tooltip_config()?;
        config.options.get_or_insert_with(Default::default);
        config.kind = None;
        config.component = Some(tooltip::Component::new(
            revision,
            move |c: tooltip::InternalContext| {
                let source = c.frame;
                let frame = c
                    .scope()?
                    .bond(move || source.get().map(|value| value.frame));
                let motion = c
                    .scope()?
                    .bond(move || source.get().and_then(|value| value.source.motion));
                factory(
                    tooltip::BrowserFunnelTooltipComponentContext {
                        revision: c.revision,
                        frame,
                    },
                    motion,
                )
            },
        ));
        owner.queue(Command::TooltipConfig(config))
    }
}
impl BrowserFunnel {
    pub fn set_mouse_handlers_with_motion(
        &self,
        h: BrowserFunnelMotionMouseHandlers,
    ) -> Result<()> {
        self.handle().set_mouse_handlers_with_motion(h)
    }
    pub fn set_tooltip_component_with_motion<W: glory::Widget + 'static>(
        &self,
        r: u64,
        f: impl FnMut(
            tooltip::BrowserFunnelTooltipComponentContext,
            Bond<Option<BrowserFunnelMotionFrame>>,
        ) -> Result<W>
        + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component_with_motion(r, f)
    }
}

impl<D: 'static> BrowserFunnelDataSvgProgram<D> {
    pub fn custom_animated<W: glory::Widget + 'static>(
        mut self,
        registry: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(
            BrowserFunnelDataSvgLayerContext<D>,
            Bond<Option<BrowserFunnelMotionFrame>>,
        ) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        self.inner =
            self.inner
                .custom_internal(registry, revision, Some(TypeId::of::<D>()), move |c| {
                    let initial = BrowserFunnelDataSvgLayerFrame::from_legacy(
                        c.initial.frame.clone(),
                        &c.initial.source,
                    )?;
                    let source = c.frame;
                    let frame = c.scope.bond(move || {
                        source.get().map(|value| {
                            BrowserFunnelDataSvgLayerFrame::from_legacy(value.frame, &value.source)
                                .expect("validated typed motion layer")
                        })
                    });
                    let motion = c
                        .scope
                        .bond(move || source.get().and_then(|value| value.source.motion));
                    Ok(Box::new(factory(
                        BrowserFunnelDataSvgLayerContext {
                            key: c.key,
                            revision: c.revision,
                            initial,
                            frame,
                            current: c.current,
                        },
                        motion,
                    )?))
                })?;
        Ok(self)
    }
}
impl<D: 'static> BrowserFunnelDataHandle<D> {
    pub fn enable_motion(&self, o: FunnelMotionOptions) -> Result<()> {
        self.inner.enable_motion(o)
    }
    pub fn set_motion_options(&self, o: Option<FunnelMotionOptions>) -> Result<()> {
        self.inner.set_motion_options(o)
    }
    pub fn set_motion_clock(&self, c: BrowserFunnelMotionClock) -> Result<()> {
        self.inner.set_motion_clock(c)
    }
    pub fn motion_clock(&self) -> Result<BrowserFunnelMotionClock> {
        self.inner.motion_clock()
    }
    pub fn set_motion_preference(&self, p: MotionPreference) -> Result<()> {
        self.inner.set_motion_preference(p)
    }
    pub fn step_motion(&self, s: f64) -> Result<()> {
        self.inner.step_motion(s)
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserFunnelMotionFrame>> {
        self.inner.motion_frame()
    }
    pub fn motion_snapshot(&self) -> Result<BrowserFunnelMotionSnapshot> {
        self.inner.motion_snapshot()
    }
    pub fn set_mouse_handlers_with_motion(
        &self,
        handlers: BrowserFunnelDataMotionMouseHandlers<D>,
    ) -> Result<()> {
        let owner = self.inner.owner()?;
        owner.require_mutable()?;
        let mut slots: BTreeMap<BrowserFunnelMouseKind, data::ErasedMouseHandler> = BTreeMap::new();
        for (kind, slot) in [
            (BrowserFunnelMouseKind::Enter, handlers.on_mouse_enter),
            (BrowserFunnelMouseKind::Move, handlers.on_mouse_move),
            (BrowserFunnelMouseKind::Leave, handlers.on_mouse_leave),
            (BrowserFunnelMouseKind::Click, handlers.on_click),
        ] {
            if let Some(mut callback) = slot {
                slots.insert(
                    kind,
                    Box::new(move |event, source| {
                        callback(
                            BrowserFunnelDataMouseEvent::from_legacy(event, source)?,
                            source.motion.clone(),
                        )
                    }),
                );
            }
        }
        owner.events.replace_erased(handlers.activity, slots);
        Ok(())
    }
    pub fn set_tooltip_component_with_motion<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        mut factory: impl FnMut(
            BrowserFunnelDataTooltipComponentContext<D>,
            Bond<Option<BrowserFunnelMotionFrame>>,
        ) -> Result<W>
        + 'static,
    ) -> Result<()> {
        let owner = self.inner.owner()?;
        let mut config = owner.tooltip_config()?;
        config.options.get_or_insert_with(Default::default);
        config.kind = Some(TypeId::of::<D>());
        config.component = Some(tooltip::Component::new(
            revision,
            move |c: tooltip::InternalContext| {
                let source = c.frame;
                let frame = c.scope()?.bond(move || {
                    source.get().map(|value| {
                        FunnelDataTooltipFrame::from_legacy(value.frame, &value.source)
                            .expect("validated typed motion tooltip")
                    })
                });
                let motion = c
                    .scope()?
                    .bond(move || source.get().and_then(|value| value.source.motion));
                factory(
                    BrowserFunnelDataTooltipComponentContext {
                        revision: c.revision,
                        frame,
                    },
                    motion,
                )
            },
        ));
        owner.queue(Command::TooltipConfig(config))
    }
}
impl<D: 'static> BrowserFunnelData<D> {
    pub fn enable_motion(&self, o: FunnelMotionOptions) -> Result<()> {
        self.handle().enable_motion(o)
    }
    pub fn set_motion_options(&self, o: Option<FunnelMotionOptions>) -> Result<()> {
        self.handle().set_motion_options(o)
    }
    pub fn set_motion_clock(&self, c: BrowserFunnelMotionClock) -> Result<()> {
        self.handle().set_motion_clock(c)
    }
    pub fn motion_clock(&self) -> Result<BrowserFunnelMotionClock> {
        self.handle().motion_clock()
    }
    pub fn set_motion_preference(&self, p: MotionPreference) -> Result<()> {
        self.handle().set_motion_preference(p)
    }
    pub fn step_motion(&self, s: f64) -> Result<()> {
        self.handle().step_motion(s)
    }
    pub fn motion_frame(&self) -> Result<Option<BrowserFunnelMotionFrame>> {
        self.handle().motion_frame()
    }
    pub fn motion_snapshot(&self) -> Result<BrowserFunnelMotionSnapshot> {
        self.handle().motion_snapshot()
    }
    pub fn set_mouse_handlers_with_motion(
        &self,
        h: BrowserFunnelDataMotionMouseHandlers<D>,
    ) -> Result<()> {
        self.handle().set_mouse_handlers_with_motion(h)
    }
    pub fn set_tooltip_component_with_motion<W: glory::Widget + 'static>(
        &self,
        r: u64,
        f: impl FnMut(
            BrowserFunnelDataTooltipComponentContext<D>,
            Bond<Option<BrowserFunnelMotionFrame>>,
        ) -> Result<W>
        + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component_with_motion(r, f)
    }
}
