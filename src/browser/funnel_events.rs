//! Four independent native body observations. Callback payloads belong to the
//! captured admission, never a later frame or a reconstructed activation.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BrowserFunnelMouseKind {
    Enter,
    Move,
    Leave,
    Click,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserFunnelMouseActivity {
    Owner,
    #[default]
    NativeComponent,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFunnelMousePointer {
    pub id: i32,
    pub kind: String,
    pub primary: bool,
    pub captured: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserFunnelMouseOccurrence {
    pub key: String,
    pub index: usize,
    pub publication: u64,
}
#[derive(Clone, PartialEq)]
pub struct BrowserFunnelMouseInput {
    pub kind: BrowserFunnelMouseKind,
    pub event: web_sys::MouseEvent,
    pub client: [f64; 2],
    pub viewport: [f64; 2],
    pub scene: [f64; 2],
    pub pointer: Option<BrowserFunnelMousePointer>,
    pub occurrence: BrowserFunnelMouseOccurrence,
}
#[derive(Clone)]
pub struct BrowserFunnelMouseEvent {
    pub data: FunnelSvgFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub part_index: usize,
    pub source_index: usize,
    pub input: BrowserFunnelMouseInput,
}
pub type FunnelMouseHandler = Box<dyn FnMut(BrowserFunnelMouseEvent) -> Result<()> + 'static>;
#[derive(Default)]
pub struct BrowserFunnelMouseHandlers {
    pub on_mouse_enter: Option<FunnelMouseHandler>,
    pub on_mouse_move: Option<FunnelMouseHandler>,
    pub on_mouse_leave: Option<FunnelMouseHandler>,
    pub on_click: Option<FunnelMouseHandler>,
    pub activity: BrowserFunnelMouseActivity,
}
impl BrowserFunnelMouseHandlers {
    fn slot(&mut self, kind: BrowserFunnelMouseKind) -> &mut Option<FunnelMouseHandler> {
        match kind {
            BrowserFunnelMouseKind::Enter => &mut self.on_mouse_enter,
            BrowserFunnelMouseKind::Move => &mut self.on_mouse_move,
            BrowserFunnelMouseKind::Leave => &mut self.on_mouse_leave,
            BrowserFunnelMouseKind::Click => &mut self.on_click,
        }
    }
    fn has(&self, kind: BrowserFunnelMouseKind) -> bool {
        match kind {
            BrowserFunnelMouseKind::Enter => self.on_mouse_enter.is_some(),
            BrowserFunnelMouseKind::Move => self.on_mouse_move.is_some(),
            BrowserFunnelMouseKind::Leave => self.on_mouse_leave.is_some(),
            BrowserFunnelMouseKind::Click => self.on_click.is_some(),
        }
    }
}
struct Registration {
    handlers: RefCell<BrowserFunnelMouseHandlers>,
    typed: RefCell<BTreeMap<BrowserFunnelMouseKind, data::ErasedMouseHandler>>,
}
struct Envelope {
    event: Event,
    stage: Rc<stage::Stage>,
    registration: Rc<Registration>,
    payload: BrowserFunnelMouseEvent,
    source: SourceEnvelope,
}
struct Cleanup {
    window: Window,
    id: i32,
    _callback: Closure<dyn FnMut()>,
}
impl Drop for Cleanup {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.id);
    }
}
#[derive(Default)]
pub(super) struct Events {
    registration: RefCell<Option<Rc<Registration>>>,
    envelopes: RefCell<Vec<Envelope>>,
    cleanup: RefCell<Option<Cleanup>>,
    dispatching: Cell<bool>,
}
struct Dispatch<'a>(&'a Cell<bool>);
impl Drop for Dispatch<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}
impl Events {
    pub fn replace_typed<D: 'static>(&self, handlers: BrowserFunnelDataMouseHandlers<D>) {
        let (activity, typed) = handlers.erase();
        self.replace_erased(activity, typed);
    }
    pub fn replace_erased(
        &self,
        activity: BrowserFunnelMouseActivity,
        typed: BTreeMap<BrowserFunnelMouseKind, data::ErasedMouseHandler>,
    ) {
        *self.registration.borrow_mut() = Some(Rc::new(Registration {
            handlers: RefCell::new(BrowserFunnelMouseHandlers {
                activity,
                ..Default::default()
            }),
            typed: RefCell::new(typed),
        }));
        self.envelopes.borrow_mut().clear();
    }
    pub fn replace(&self, handlers: Option<BrowserFunnelMouseHandlers>) {
        *self.registration.borrow_mut() = handlers.map(|handlers| {
            Rc::new(Registration {
                handlers: RefCell::new(handlers),
                typed: RefCell::new(BTreeMap::new()),
            })
        });
        self.envelopes.borrow_mut().clear();
    }
    pub fn is_dispatching(&self) -> bool {
        self.dispatching.get()
    }
    pub fn activity(&self, tooltip_configured: bool) -> BrowserFunnelMouseActivity {
        self.registration.borrow().as_ref().map_or(
            if tooltip_configured {
                BrowserFunnelMouseActivity::NativeComponent
            } else {
                BrowserFunnelMouseActivity::Owner
            },
            |registration| registration.handlers.borrow().activity,
        )
    }
    pub fn stop(&self) {
        self.registration.borrow_mut().take();
        self.envelopes.borrow_mut().clear();
        self.cleanup.borrow_mut().take();
    }
    pub fn bind(owner: &Rc<Driver>) -> Result<()> {
        for (name, kind) in [
            ("pointerover", BrowserFunnelMouseKind::Enter),
            ("pointermove", BrowserFunnelMouseKind::Move),
            ("pointerout", BrowserFunnelMouseKind::Leave),
            ("click", BrowserFunnelMouseKind::Click),
        ] {
            for capture in [true, false] {
                let weak = Rc::downgrade(owner);
                let callback = Closure::wrap(Box::new(move |event: Event| {
                    let Some(owner) = weak
                        .upgrade()
                        .filter(|owner| owner.alive.get() && !owner.busy.get())
                    else {
                        return;
                    };
                    let result = if capture {
                        owner.events.capture(&owner, &event, kind)
                    } else {
                        owner.events.deliver(&owner, &event)
                    };
                    if let Err(error) = result {
                        owner.record_event_error(error);
                    }
                }) as Box<dyn Fn(Event)>);
                let target: EventTarget = owner.window.clone().into();
                target
                    .add_event_listener_with_callback_and_bool(
                        name,
                        callback.as_ref().unchecked_ref(),
                        capture,
                    )
                    .map_err(host)?;
                owner.listeners.borrow_mut().push(Listener {
                    target,
                    name,
                    callback,
                    capture,
                });
            }
        }
        Ok(())
    }
    fn capture(
        &self,
        owner: &Rc<Driver>,
        event: &Event,
        kind: BrowserFunnelMouseKind,
    ) -> Result<()> {
        if self.dispatching.get() {
            return if owner
                .accepted
                .borrow()
                .as_ref()
                .is_some_and(|accepted| accepted.stage.leave_role(event).is_some())
            {
                Err(ChartError(
                    "recursive Funnel mouse dispatch rejected".into(),
                ))
            } else {
                Ok(())
            };
        }
        self.dispatching.set(true);
        let _dispatch = Dispatch(&self.dispatching);
        let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return Ok(());
        };
        if let Some(pointer) = event.dyn_ref::<PointerEvent>()
            && (pointer.pointer_type() == "touch"
                || kind != BrowserFunnelMouseKind::Click && !pointer.is_primary())
        {
            return Ok(());
        }
        let tooltip_configured = owner
            .accepted
            .borrow()
            .as_ref()
            .is_some_and(|accepted| accepted.request.tooltip.is_configured());
        let activity = self.activity(tooltip_configured);
        let Some((stage, hit)) = owner.reserve_mouse(event, mouse, kind, activity)? else {
            return Ok(());
        };
        let registration = self.registration.borrow().clone();
        let Some(registration) = registration.filter(|registration| {
            registration.handlers.borrow().has(kind)
                || registration.typed.borrow().contains_key(&kind)
        }) else {
            return Ok(());
        };
        scene::ensure(
            self.envelopes.borrow().len() < MAX_PENDING,
            "Funnel mouse envelope budget exceeded",
        )?;
        let pointer = event
            .dyn_ref::<PointerEvent>()
            .map(|pointer| BrowserFunnelMousePointer {
                id: pointer.pointer_id(),
                kind: pointer.pointer_type(),
                primary: pointer.is_primary(),
                captured: event
                    .target()
                    .and_then(|target| target.dyn_into::<Element>().ok())
                    .is_some_and(|element| element.has_pointer_capture(pointer.pointer_id())),
            });
        let payload = BrowserFunnelMouseEvent {
            data: hit.data,
            source_generation: hit.source_generation,
            publication: hit.publication,
            part_index: hit.part_index,
            source_index: hit.source_index,
            input: BrowserFunnelMouseInput {
                kind,
                event: mouse.clone(),
                client: hit.client,
                viewport: hit.viewport,
                scene: hit.scene,
                pointer,
                occurrence: BrowserFunnelMouseOccurrence {
                    key: hit.key,
                    index: hit.index,
                    publication: hit.publication,
                },
            },
        };
        self.envelopes.borrow_mut().push(Envelope {
            event: event.clone(),
            source: stage.source.clone(),
            stage,
            registration,
            payload,
        });
        self.schedule_cleanup(owner)
    }
    fn deliver(&self, owner: &Rc<Driver>, event: &Event) -> Result<()> {
        let envelope = {
            let mut pending = self.envelopes.borrow_mut();
            pending
                .iter()
                .position(|envelope| envelope.event == *event)
                .map(|index| pending.remove(index))
        };
        let Some(envelope) = envelope else {
            return Ok(());
        };
        if self.dispatching.replace(true) {
            return Err(ChartError(
                "recursive Funnel mouse dispatch rejected".into(),
            ));
        }
        let _dispatch = Dispatch(&self.dispatching);
        let same_registration = self
            .registration
            .borrow()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, &envelope.registration));
        let same_admission = owner.accepted.borrow().as_ref().is_some_and(|current| {
            Rc::ptr_eq(&current.stage, &envelope.stage)
                && current.generation == envelope.payload.source_generation
                && current.publication == envelope.payload.publication
        });
        if !same_registration || !same_admission || !owner.alive.get() {
            return Ok(());
        }
        let kind = envelope.payload.input.kind;
        let typed = envelope.registration.typed.borrow_mut().remove(&kind);
        if let Some(mut callback) = typed {
            let result = callback(envelope.payload, &envelope.source);
            if owner.alive.get()
                && self
                    .registration
                    .borrow()
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &envelope.registration))
            {
                envelope
                    .registration
                    .typed
                    .borrow_mut()
                    .insert(kind, callback);
            }
            return result;
        }
        let callback = envelope
            .registration
            .handlers
            .borrow_mut()
            .slot(kind)
            .take();
        let Some(mut callback) = callback else {
            return Ok(());
        };
        let result = callback(envelope.payload);
        if owner.alive.get()
            && self
                .registration
                .borrow()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, &envelope.registration))
        {
            *envelope.registration.handlers.borrow_mut().slot(kind) = Some(callback);
        }
        result
    }
    fn schedule_cleanup(&self, owner: &Rc<Driver>) -> Result<()> {
        if self.cleanup.borrow().is_some() {
            return Ok(());
        }
        let weak = Rc::downgrade(owner);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(owner) = weak.upgrade() {
                owner.events.envelopes.borrow_mut().clear();
                owner.events.cleanup.borrow_mut().take();
            }
        });
        let id = owner
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            )
            .map_err(host)?;
        *self.cleanup.borrow_mut() = Some(Cleanup {
            window: owner.window.clone(),
            id,
            _callback: callback,
        });
        Ok(())
    }
}
