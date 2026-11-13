//! Owned HTML brush controls shared by the SVG and Canvas browser hosts.
use super::*;
use crate::brush::{
    BrushHandle, BrushMode, BrushModifiers, BrushReconfigure, BrushSession, BrushTarget,
    EditableBrush, EditableBrushInput, EditableBrushOptions,
};

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserBrushSnapshot {
    pub options: EditableBrushOptions,
    pub selection: Option<Brush>,
    /// Current visible data IDs, recomputed from the accepted scene and binding.
    pub ids: Vec<String>,
    pub active: bool,
    pub mode: Option<BrushMode>,
    pub session: Option<BrushSession>,
    pub captured_pointer: Option<i32>,
    pub touch: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Capture {
    pub(super) id: i32,
    token: u64,
    pub(super) touch: bool,
}

pub(super) enum Command {
    Configure(Option<EditableBrushOptions>, BrushReconfigure),
    Selection(Option<Brush>),
    Pointer {
        kind: PointerKind,
        id: i32,
        token: u64,
        client: (f64, f64),
        modifiers: BrushModifiers,
    },
    Modifiers {
        token: u64,
        modifiers: BrushModifiers,
    },
    Nudge {
        target: BrushTarget,
        delta: (f64, f64),
    },
    Clear,
    Cancel {
        token: u64,
        id: i32,
    },
}

pub(super) struct Host {
    root: HtmlElement,
    overlay: HtmlElement,
    selection: HtmlElement,
    handles: Vec<(BrushHandle, HtmlElement)>,
}

const HANDLES: [(BrushHandle, &str, &str); 8] = [
    (BrushHandle::N, "n", "top edge"),
    (BrushHandle::W, "w", "left edge"),
    (BrushHandle::E, "e", "right edge"),
    (BrushHandle::S, "s", "bottom edge"),
    (BrushHandle::Nw, "nw", "top-left corner"),
    (BrushHandle::Ne, "ne", "top-right corner"),
    (BrushHandle::Sw, "sw", "bottom-left corner"),
    (BrushHandle::Se, "se", "bottom-right corner"),
];

fn control(document: &web_sys::Document, class: &str, label: &str) -> Result<HtmlElement> {
    let control = document
        .create_element("button")
        .map_err(host)?
        .dyn_into::<HtmlElement>()
        .map_err(host)?;
    control.set_class_name(class);
    control.set_attribute("type", "button").map_err(host)?;
    control.set_attribute("aria-label", label).map_err(host)?;
    for (name, value) in [
        ("position", "absolute"),
        ("box-sizing", "border-box"),
        ("padding", "0"),
        ("margin", "0"),
        ("min-width", "0"),
        ("min-height", "0"),
        ("pointer-events", "auto"),
        ("touch-action", "none"),
        ("border", "1px solid #275b98"),
    ] {
        css(&control, name, value)?;
    }
    Ok(control)
}

impl Host {
    pub(super) fn new(document: &web_sys::Document) -> Result<Self> {
        let root = element(document, "kartis-editable-brush")?;
        root.set_attribute("role", "group").map_err(host)?;
        root.set_attribute("aria-label", "Editable chart selection")
            .map_err(host)?;
        for (name, value) in [
            ("position", "absolute"),
            ("inset", "0"),
            ("pointer-events", "none"),
            ("z-index", "2"),
        ] {
            css(&root, name, value)?;
        }
        let overlay = element(document, "kartis-editable-brush-overlay")?;
        for (name, value) in [
            ("position", "absolute"),
            ("pointer-events", "auto"),
            ("touch-action", "none"),
            ("cursor", "crosshair"),
            ("background", "transparent"),
        ] {
            css(&overlay, name, value)?;
        }
        root.append_child(&overlay).map_err(host)?;
        let selection = control(
            document,
            "kartis-editable-brush-control kartis-editable-brush-selection",
            "Move chart selection; use arrow keys, Shift for ten pixels, Delete to clear",
        )?;
        selection
            .set_attribute("data-brush-target", "selection")
            .map_err(host)?;
        css(&selection, "background", "rgba(60,130,210,.18)")?;
        css(&selection, "cursor", "move")?;
        root.append_child(&selection).map_err(host)?;
        let mut handles = Vec::with_capacity(8);
        for (handle, short, label) in HANDLES {
            let node = control(
                document,
                &format!(
                    "kartis-editable-brush-control kartis-editable-brush-handle kartis-editable-brush-handle--{short}"
                ),
                &format!("Resize selection {label}; use arrow keys, Shift for ten pixels"),
            )?;
            node.set_attribute("data-brush-target", short)
                .map_err(host)?;
            node.set_attribute("role", "slider").map_err(host)?;
            css(&node, "background", "rgba(255,255,255,.9)")?;
            css(
                &node,
                "cursor",
                match handle {
                    BrushHandle::N | BrushHandle::S => "ns-resize",
                    BrushHandle::E | BrushHandle::W => "ew-resize",
                    BrushHandle::Nw | BrushHandle::Se => "nwse-resize",
                    BrushHandle::Ne | BrushHandle::Sw => "nesw-resize",
                },
            )?;
            root.append_child(&node).map_err(host)?;
            handles.push((handle, node));
        }
        Ok(Self {
            root,
            overlay,
            selection,
            handles,
        })
    }

    pub(super) fn publish(
        &self,
        surface: &HtmlElement,
        brush: Option<&EditableBrush>,
        scene: &Scene,
    ) -> Result<()> {
        let Some(brush) = brush else {
            self.root.remove();
            return Ok(());
        };
        brush.options().validate()?;
        let extent = brush.options().extent;
        let bounds = |node: &HtmlElement, b: Brush| -> Result<()> {
            b.validate()?;
            for (name, value) in [
                ("left", b.x0 / scene.width * 100.),
                ("top", b.y0 / scene.height * 100.),
                ("width", (b.x1 - b.x0) / scene.width * 100.),
                ("height", (b.y1 - b.y0) / scene.height * 100.),
            ] {
                css(node, name, &format!("{value}%"))?;
            }
            Ok(())
        };
        bounds(&self.overlay, extent)?;
        let selection = brush.selection();
        css(
            &self.selection,
            "display",
            if selection.is_some() { "block" } else { "none" },
        )?;
        if let Some(b) = selection {
            bounds(&self.selection, b)?;
            self.selection
                .set_attribute(
                    "aria-description",
                    &format!("x {} to {}, y {} to {}", b.x0, b.x1, b.y0, b.y1),
                )
                .map_err(host)?;
        }
        let geometry = brush.handles();
        for (handle, node) in &self.handles {
            if let Some(rect) = geometry.iter().find(|r| r.handle == *handle) {
                bounds(node, rect.bounds)?;
                css(node, "display", "block")?;
                let b = selection.expect("handles require selection");
                let vertical = matches!(handle, BrushHandle::N | BrushHandle::S);
                let value = match handle {
                    BrushHandle::W | BrushHandle::Nw | BrushHandle::Sw => b.x0,
                    BrushHandle::E | BrushHandle::Ne | BrushHandle::Se => b.x1,
                    BrushHandle::N => b.y0,
                    BrushHandle::S => b.y1,
                };
                node.set_attribute(
                    "aria-orientation",
                    if vertical { "vertical" } else { "horizontal" },
                )
                .map_err(host)?;
                node.set_attribute(
                    "aria-valuemin",
                    &if vertical { extent.y0 } else { extent.x0 }.to_string(),
                )
                .map_err(host)?;
                node.set_attribute(
                    "aria-valuemax",
                    &if vertical { extent.y1 } else { extent.x1 }.to_string(),
                )
                .map_err(host)?;
                node.set_attribute("aria-valuenow", &value.to_string())
                    .map_err(host)?;
                node.set_attribute(
                    "aria-valuetext",
                    &format!("x {} to {}, y {} to {}", b.x0, b.x1, b.y0, b.y1),
                )
                .map_err(host)?;
            } else {
                css(node, "display", "none")?;
            }
        }
        self.root
            .set_attribute("data-active", if brush.active() { "true" } else { "false" })
            .map_err(host)?;
        if self.root.parent_node().is_none() {
            surface.append_child(&self.root).map_err(host)?;
        }
        Ok(())
    }
}

fn modifiers(event: &web_sys::MouseEvent, space: bool) -> BrushModifiers {
    BrushModifiers {
        alt: event.alt_key(),
        shift: event.shift_key(),
        meta: event.meta_key(),
        ctrl: event.ctrl_key(),
        space,
    }
}

fn keyboard_modifiers(event: &KeyboardEvent, space: bool) -> BrushModifiers {
    BrushModifiers {
        alt: event.alt_key(),
        shift: event.shift_key(),
        meta: event.meta_key(),
        ctrl: event.ctrl_key(),
        space,
    }
}

fn focused_target(event: &Event, surface: &HtmlElement) -> Option<BrushTarget> {
    let node = event
        .target()?
        .dyn_into::<Element>()
        .ok()?
        .closest(".kartis-editable-brush-control")
        .ok()??;
    if !surface.contains(Some(&node)) {
        return None;
    }
    if node
        .dyn_ref::<HtmlElement>()?
        .style()
        .get_property_value("display")
        .ok()?
        == "none"
    {
        return None;
    }
    match node.get_attribute("data-brush-target")?.as_str() {
        "selection" => Some(BrushTarget::Selection),
        value => HANDLES
            .iter()
            .find(|(_, short, _)| *short == value)
            .map(|(handle, ..)| BrushTarget::Handle(*handle)),
    }
}

impl Engine {
    pub(super) fn editable_pointer(
        self: &Rc<Self>,
        kind: PointerKind,
        pointer: &PointerEvent,
        event: &Event,
    ) -> bool {
        if !self.alive.get() {
            return false;
        }
        let current = self.editable_capture.get();
        if matches!(kind, PointerKind::Down) {
            if current.is_some() {
                return true;
            }
            if pointer.button() != 0
                || !pointer.is_primary()
                || self.captured.get().is_some()
                || !self.touches.borrow().is_empty()
            {
                return false;
            }
            let guard = self.state.borrow();
            let Some(state) = guard.as_ref() else {
                return false;
            };
            let Some(brush) = state.model.editable_brush() else {
                return false;
            };
            let Ok(point) = state.coordinates(client_coordinates(pointer)) else {
                return false;
            };
            if brush.hit_test(point).ok().flatten().is_none() {
                return false;
            }
            if pointer.ctrl_key() {
                event.prevent_default();
                return true;
            }
            let Some(token) = self.editable_generation.get().checked_add(1) else {
                if self.fault.borrow().is_none() {
                    *self.fault.borrow_mut() =
                        Some(ChartError("browser brush generation exhausted".into()));
                }
                let _ = self.request();
                return true;
            };
            self.editable_generation.set(token);
            let capture = Capture {
                id: pointer.pointer_id(),
                token,
                touch: pointer.pointer_type() == "touch",
            };
            self.editable_capture.set(Some(capture));
            let surface = state.surface.clone();
            drop(guard);
            let _ = surface.set_pointer_capture(capture.id);
            if let Some(target) = event
                .target()
                .and_then(|v| v.dyn_into::<HtmlElement>().ok())
                .filter(|v| v.has_attribute("data-brush-target"))
            {
                let _ = target.focus();
            } else {
                let _ = surface.focus();
            }
            event.prevent_default();
            self.enqueue(Input::EditableBrush(Command::Pointer {
                kind,
                id: capture.id,
                token,
                client: client_coordinates(pointer),
                modifiers: modifiers(pointer, self.editable_space.get()),
            }));
            return true;
        }
        if let Some(capture) = current {
            if capture.id != pointer.pointer_id() {
                return true;
            }
            event.prevent_default();
            self.enqueue(Input::EditableBrush(Command::Pointer {
                kind,
                id: capture.id,
                token: capture.token,
                client: client_coordinates(pointer),
                modifiers: modifiers(pointer, self.editable_space.get()),
            }));
            if matches!(kind, PointerKind::Up | PointerKind::Cancel) {
                self.release_editable_capture();
            }
            return true;
        }
        if matches!(kind, PointerKind::Move) {
            let state = self.state.borrow();
            if let Some(state) = state.as_ref()
                && let Some(brush) = state.model.editable_brush()
                && let Ok(point) = state.coordinates(client_coordinates(pointer))
                && brush.hit_test(point).ok().flatten().is_some()
            {
                self.enqueue(Input::Action(SurfaceInput::Leave));
                return true;
            }
        }
        false
    }

    pub(super) fn release_editable_capture(&self) {
        if let Some(state) = self.state.borrow().as_ref() {
            self.retire_editable_capture(&state.surface);
        } else {
            self.editable_capture.set(None);
        }
    }

    pub(super) fn retire_editable_capture(&self, surface: &HtmlElement) {
        if let Some(capture) = self.editable_capture.take()
            && surface.has_pointer_capture(capture.id)
        {
            let _ = surface.release_pointer_capture(capture.id);
        }
    }

    pub(super) fn editable_key_owned(&self, event: &Event) -> bool {
        self.editable_capture.get().is_some()
            || self
                .state
                .borrow()
                .as_ref()
                .is_some_and(|state| focused_target(event, &state.surface).is_some())
    }

    pub(super) fn sync_editable_capture(&self) {
        let retire = self.editable_capture.get().is_some()
            && self
                .state
                .borrow()
                .as_ref()
                .is_none_or(|s| s.model.editable_brush().is_none_or(|b| !b.active()));
        if retire {
            self.release_editable_capture();
        }
    }

    pub(super) fn editable_contact_live(&self, state: &State) -> bool {
        self.editable_capture.get().is_some_and(|capture| {
            state.editable_transport == Some((capture.token, capture.id))
                && state
                    .model
                    .editable_brush()
                    .is_some_and(|brush| brush.active())
        })
    }

    pub(super) fn install_editable_brush(self: &Rc<Self>) -> Result<()> {
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "pointermove", move |event| {
            let Some(engine) = weak.upgrade() else {
                return;
            };
            let Some(pointer) = event.dyn_ref::<PointerEvent>() else {
                return;
            };
            if engine.editable_capture.get().is_none() {
                return;
            }
            let inside = engine.state.borrow().as_ref().is_some_and(|state| {
                event
                    .target()
                    .and_then(|v| v.dyn_into::<web_sys::Node>().ok())
                    .is_some_and(|n| state.surface.contains(Some(&n)))
            });
            if !inside {
                engine.editable_pointer(PointerKind::Move, pointer, &event);
            }
        })?;
        for (name, kind) in [
            ("pointerup", PointerKind::Up),
            ("pointercancel", PointerKind::Cancel),
        ] {
            let weak = Rc::downgrade(self);
            self.add(self.window.clone().into(), name, move |event| {
                if let Some(engine) = weak.upgrade()
                    && let Some(pointer) = event.dyn_ref::<PointerEvent>()
                {
                    engine.editable_pointer(kind, pointer, &event);
                }
            })?;
        }
        for (name, pressed) in [("keydown", true), ("keyup", false)] {
            let weak = Rc::downgrade(self);
            self.add(self.window.clone().into(), name, move |event| {
                let Some(engine) = weak.upgrade() else {
                    return;
                };
                let Some(key) = event.dyn_ref::<KeyboardEvent>() else {
                    return;
                };
                // A creator ends with focus on the surface, not a control.
                // Its later Space keyup must clear the latch even though the
                // completed gesture no longer owns keyboard input.
                if key.key() == " " && !pressed {
                    engine.editable_space.set(false);
                }
                if !engine.editable_key_owned(&event) {
                    return;
                }
                if key.key() == " " {
                    engine.editable_space.set(pressed);
                    event.prevent_default();
                }
                if let Some(capture) = engine.editable_capture.get() {
                    if key.key() == "Escape" && pressed {
                        engine.enqueue(Input::EditableBrush(Command::Cancel {
                            token: capture.token,
                            id: capture.id,
                        }));
                        engine.release_editable_capture();
                        event.prevent_default();
                    } else {
                        engine.enqueue(Input::EditableBrush(Command::Modifiers {
                            token: capture.token,
                            modifiers: keyboard_modifiers(key, engine.editable_space.get()),
                        }));
                    }
                    return;
                }
                if !pressed || key.ctrl_key() || key.meta_key() || key.alt_key() {
                    return;
                }
                let target = engine
                    .state
                    .borrow()
                    .as_ref()
                    .and_then(|s| focused_target(&event, &s.surface));
                let Some(target) = target else {
                    return;
                };
                let step = if key.shift_key() { 10. } else { 1. };
                let delta = match key.key().as_str() {
                    "ArrowLeft" => Some((-step, 0.)),
                    "ArrowRight" => Some((step, 0.)),
                    "ArrowUp" => Some((0., -step)),
                    "ArrowDown" => Some((0., step)),
                    _ => None,
                };
                if let Some(delta) = delta {
                    event.prevent_default();
                    engine.enqueue(Input::EditableBrush(Command::Nudge { target, delta }));
                } else if matches!(key.key().as_str(), "Escape" | "Delete" | "Backspace") {
                    event.prevent_default();
                    engine.enqueue(Input::EditableBrush(Command::Clear));
                }
            })?;
        }
        let weak = Rc::downgrade(self);
        self.add(self.window.clone().into(), "blur", move |_| {
            if let Some(engine) = weak.upgrade() {
                engine.cancel_editable_input();
            }
        })?;
        if let Some(document) = self.window.document() {
            let weak = Rc::downgrade(self);
            let owner = document.clone();
            self.add(document.into(), "visibilitychange", move |_| {
                if owner.hidden()
                    && let Some(engine) = weak.upgrade()
                {
                    engine.cancel_editable_input();
                }
            })?;
        }
        let surface = self.state.borrow().as_ref().unwrap().surface.clone();
        let weak = Rc::downgrade(self);
        let owner = surface.clone();
        self.add(surface.into(), "focusout", move |event| {
            let Some(engine) = weak.upgrade() else {
                return;
            };
            let related = web_sys::js_sys::Reflect::get(&event, &"relatedTarget".into())
                .ok()
                .and_then(|v| v.dyn_into::<web_sys::Node>().ok());
            if related.is_none_or(|target| !owner.contains(Some(&target))) {
                engine.cancel_editable_input();
            }
        })?;
        Ok(())
    }

    fn cancel_editable_input(&self) {
        // Owned DOM publication can temporarily detach the focused control.
        // A rollback restores that same node and its focus before commit.
        if self.processing.get() {
            return;
        }
        self.editable_space.set(false);
        if let Some(capture) = self.editable_capture.get() {
            self.enqueue(Input::EditableBrush(Command::Cancel {
                token: capture.token,
                id: capture.id,
            }));
            self.release_editable_capture();
        }
    }
}

impl State {
    pub(super) fn process_editable_brush(
        &mut self,
        command: Command,
        window: &Window,
    ) -> Result<Vec<SurfaceEvent>> {
        match command {
            Command::Configure(options, policy) => {
                let host = if options.is_some() && self.editable_host.is_none() {
                    Some(Host::new(&window.document().ok_or_else(|| {
                        ChartError("browser document unavailable".into())
                    })?)?)
                } else {
                    None
                };
                let events = self.model.configure_editable_brush(options, policy)?;
                self.options.editable_brush = options;
                if let Some(host) = host {
                    self.editable_host = Some(host);
                }
                self.editable_transport = None;
                Ok(events)
            }
            Command::Selection(selection) => {
                let events = self.model.set_brush_selection(selection)?;
                self.editable_transport = None;
                Ok(events)
            }
            Command::Pointer {
                kind,
                id,
                token,
                client,
                modifiers,
            } => {
                if matches!(kind, PointerKind::Down) {
                    let point = self.coordinates(client)?;
                    let Some(brush) = self.model.editable_brush() else {
                        return Ok(vec![]);
                    };
                    let Some(target) = brush.hit_test(point)? else {
                        return Ok(vec![]);
                    };
                    let events = self.model.apply_editable_brush(EditableBrushInput::Begin {
                        id,
                        point,
                        target,
                        modifiers,
                    })?;
                    self.editable_transport = self
                        .model
                        .editable_brush()
                        .and_then(|b| b.session())
                        .map(|_| (token, id));
                    self.freeze_motion();
                    return Ok(events);
                }
                if self.editable_transport != Some((token, id)) {
                    return Ok(vec![]);
                }
                let Some(session) = self.model.editable_brush().and_then(|b| b.session()) else {
                    return Ok(vec![]);
                };
                if matches!(kind, PointerKind::Cancel) {
                    let events = self
                        .model
                        .apply_editable_brush(EditableBrushInput::Cancel { session, id })?;
                    self.editable_transport = None;
                    return Ok(events);
                }
                let point = match self.coordinates(client) {
                    Ok(point) => point,
                    Err(error) if matches!(kind, PointerKind::Up) => {
                        self.model
                            .apply_editable_brush(EditableBrushInput::Cancel { session, id })?;
                        self.editable_transport = None;
                        return Err(error);
                    }
                    Err(error) => return Err(error),
                };
                let mut events = self
                    .model
                    .apply_editable_brush(EditableBrushInput::Modifiers { session, modifiers })?;
                events.extend(self.model.apply_editable_brush(
                    if matches!(kind, PointerKind::Up) {
                        EditableBrushInput::End { session, id, point }
                    } else {
                        EditableBrushInput::Move { session, id, point }
                    },
                )?);
                if matches!(kind, PointerKind::Up) {
                    self.editable_transport = None;
                }
                Ok(events)
            }
            Command::Modifiers { token, modifiers } => {
                if self
                    .editable_transport
                    .is_none_or(|(current, _)| current != token)
                {
                    return Ok(vec![]);
                }
                let Some(session) = self.model.editable_brush().and_then(|b| b.session()) else {
                    return Ok(vec![]);
                };
                self.model
                    .apply_editable_brush(EditableBrushInput::Modifiers { session, modifiers })
            }
            Command::Cancel { token, id } => {
                if self.editable_transport != Some((token, id)) {
                    return Ok(vec![]);
                }
                let Some(session) = self.model.editable_brush().and_then(|b| b.session()) else {
                    return Ok(vec![]);
                };
                let events = self
                    .model
                    .apply_editable_brush(EditableBrushInput::Cancel { session, id })?;
                self.editable_transport = None;
                Ok(events)
            }
            Command::Nudge { target, delta } => {
                let events = self
                    .model
                    .apply_editable_brush(EditableBrushInput::Nudge { target, delta })?;
                self.editable_transport = None;
                Ok(events)
            }
            Command::Clear => {
                let events = self.model.apply_editable_brush(EditableBrushInput::Clear)?;
                self.editable_transport = None;
                Ok(events)
            }
        }
    }
}
