//! Viewport-pixel brush paint and native control bridge. Only the temporary
//! paint Scene receives inverse-projected overlay marks; business data and
//! interaction bindings remain untouched.
use std::collections::VecDeque;

use glory::render::{
    DragCapture, DragPhase, Event, EventKind, FocusMode, GraphicStroke, NodeId, Query, QueryValue,
    Semantics, StrokeCap,
};
use glory::style::{Display, FlexDirection, FlexWrap};

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct RasterEditableBrushSnapshot {
    pub options: EditableBrushOptions,
    pub selection: Option<Brush>,
    pub handles: Vec<BrushHandleRect>,
    pub mode: Option<BrushMode>,
    pub session: Option<BrushSession>,
    pub active: bool,
    /// Current visible business anchors, recomputed after data/visibility/view changes.
    pub ids: Vec<String>,
}

pub(super) fn snapshot(model: &SurfaceModel) -> Result<Option<RasterEditableBrushSnapshot>> {
    let Some(brush) = model.editable_brush() else {
        return Ok(None);
    };
    Ok(Some(RasterEditableBrushSnapshot {
        options: *brush.options(),
        selection: brush.selection(),
        handles: brush.handles(),
        mode: brush.mode(),
        session: brush.session(),
        active: brush.active(),
        ids: model.editable_brush_ids()?,
    }))
}

pub(super) fn append_visual(scene: &mut Scene, model: &SurfaceModel) -> Result<()> {
    let Some(brush) = model.editable_brush() else {
        return Ok(());
    };
    let Some(selection) = brush.selection() else {
        return Ok(());
    };
    let transform = model.paint_transform();
    // Preserve the authored order, including labels, and put viewport controls
    // above every business item. A nonempty explicit order must list the new
    // marks too; otherwise renderers correctly skip them.
    scene.draw_order = scene.draw_items();
    let mut ids = scene
        .marks
        .iter()
        .map(|m| m.id.clone())
        .collect::<BTreeSet<_>>();
    let mut add = |name: &str, bounds: Brush, fill: Color| -> Result<()> {
        let a = transform.unproject((bounds.x0, bounds.y0))?;
        let b = transform.unproject((bounds.x1, bounds.y1))?;
        let prefix = format!("__kartis-editable-brush:{name}");
        let mut id = prefix.clone();
        let mut serial = 0usize;
        while ids.contains(&id) {
            serial += 1;
            id = format!("{prefix}:{serial}");
        }
        ids.insert(id.clone());
        let mut mark = Mark::new(
            id,
            Geometry::Rect {
                x: a.0,
                y: a.1,
                width: b.0 - a.0,
                height: b.1 - a.1,
            },
            fill,
        );
        mark.label.clear();
        mark.stroke = Some(GraphicStroke {
            color: Color(35, 100, 190, 255),
            width: 1. / transform.scale,
            cap: StrokeCap::Butt,
        });
        scene.draw_order.push(DrawItem::Mark(scene.marks.len()));
        scene.marks.push(mark);
        Ok(())
    };
    add("selection", selection, Color(55, 120, 205, 48))?;
    for handle in brush.handles() {
        add(
            handle.handle.name(),
            handle.bounds,
            Color(255, 255, 255, 255),
        )?;
    }
    Ok(())
}

impl RasterSurfaceHandle {
    pub fn configure_editable_brush(
        &self,
        options: Option<EditableBrushOptions>,
        policy: BrushReconfigure,
    ) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, surface_options| {
            let events = model.configure_editable_brush(options, policy)?;
            surface_options.editable_brush = options;
            Ok(events)
        })
    }

    pub fn set_brush_selection(&self, selection: Option<Brush>) -> Result<Vec<SurfaceEvent>> {
        self.engine()?
            .update(|model, _| model.set_brush_selection(selection))
    }

    pub fn editable_brush(&self) -> Result<Option<RasterEditableBrushSnapshot>> {
        let engine = self.engine()?;
        let data = engine.data.borrow();
        snapshot(&data.as_ref().expect("checked live raster surface").model)
    }

    pub fn dispatch_editable_brush(&self, input: EditableBrushInput) -> Result<Vec<SurfaceEvent>> {
        self.engine()?
            .update(|model, _| model.apply_editable_brush(input))
    }

    pub fn editable_brush_viewport_request(&self) -> Result<Option<ViewportRequest>> {
        let engine = self.engine()?;
        let data = engine.data.borrow();
        data.as_ref()
            .expect("checked live raster surface")
            .model
            .editable_brush_viewport_request()
    }

    /// Convert physical host coordinates to viewport pixels once. The shared
    /// controller then applies its own viewport geometry without a second
    /// camera inversion.
    pub fn dispatch_editable_brush_host(
        &self,
        input: EditableBrushInput,
        bounds: RasterSurfaceBounds,
    ) -> Result<Vec<SurfaceEvent>> {
        let engine = self.engine()?;
        engine.update(|model, _| {
            let scene = model.scene();
            let input = match input {
                EditableBrushInput::Begin {
                    id,
                    point,
                    target,
                    modifiers,
                } => EditableBrushInput::Begin {
                    id,
                    point: bounds.point(point, scene)?,
                    target,
                    modifiers,
                },
                EditableBrushInput::Move { session, id, point } => EditableBrushInput::Move {
                    session,
                    id,
                    point: bounds.point(point, scene)?,
                },
                EditableBrushInput::End { session, id, point } => EditableBrushInput::End {
                    session,
                    id,
                    point: bounds.point(point, scene)?,
                },
                other => other,
            };
            model.apply_editable_brush(input)
        })
    }
}

pub(super) struct ImageAndControls {
    pub engine: Weak<Engine>,
}

impl Widget for ImageAndControls {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let engine = self
            .engine
            .upgrade()
            .ok_or_else(|| ui_error(ChartError("raster surface stopped".into())))?;
        let frame = engine.frame;
        let scene = engine.scene;
        let brush = engine.brush_frame;
        let image_node = scope.cage(None);
        let pointer_node = scope.cage(None);
        let selected_target = scope.cage(BrushTarget::Selection);
        let handle = RasterSurfaceHandle {
            engine: self.engine.clone(),
        };
        let native = Rc::new(RefCell::new(NativeState::default()));
        let cleanup = native.clone();
        let retire = handle.clone();
        scope.on_cleanup(move || {
            retire_native(&cleanup, &retire);
        })?;
        let image_view = glory::ui::view()
            .class("kartis-raster-brush-image-host")
            .node_ref(pointer_node)
            .style(scope.bond(move || {
                frame.with(|f| {
                    Style::new()
                        .position(Position::Relative)
                        .width(Length::Px(f.width))
                        .height(Length::Px(f.height))
                })
            }));
        image_view
            .fill((
                glory::ui::image()
                    .class("kartis-raster-surface-image")
                    .node_ref(image_node)
                    .source(scope.bond(move || frame.with(|f| f.source.clone())))
                    .alt(scope.bond(move || scene.with(|s| s.title.clone())))
                    .style(scope.bond(move || {
                        frame.with(|f| {
                            Style::new()
                                .width(Length::Px(f.width))
                                .height(Length::Px(f.height))
                        })
                    })),
                BrushPlane {
                    handle: handle.clone(),
                    brush,
                    image_node,
                    selected_target,
                    native,
                },
            ))
            .show_in(scope)?;
        super::plot_pointer::image_channel(scope, pointer_node, self.engine.clone())?;
        KeyboardControls {
            handle,
            brush,
            selected_target,
        }
        .show_in(scope)
    }
}

struct BrushPlane {
    handle: RasterSurfaceHandle,
    brush: Cage<Option<RasterEditableBrushSnapshot>>,
    image_node: Cage<Option<NodeId>>,
    selected_target: Cage<BrushTarget>,
    native: Rc<RefCell<NativeState>>,
}
fn targets() -> impl Iterator<Item = BrushTarget> {
    [BrushTarget::Overlay, BrushTarget::Selection]
        .into_iter()
        .chain(
            [
                BrushHandle::N,
                BrushHandle::W,
                BrushHandle::E,
                BrushHandle::S,
                BrushHandle::Nw,
                BrushHandle::Ne,
                BrushHandle::Sw,
                BrushHandle::Se,
            ]
            .into_iter()
            .map(BrushTarget::Handle),
        )
}
fn target_name(target: BrushTarget) -> &'static str {
    match target {
        BrushTarget::Overlay => "extent",
        BrushTarget::Selection => "selection",
        BrushTarget::Handle(h) => h.name(),
    }
}
fn friendly_target_name(target: BrushTarget) -> &'static str {
    match target {
        BrushTarget::Overlay => "drawing area",
        BrushTarget::Selection => "selection",
        BrushTarget::Handle(BrushHandle::N) => "top edge",
        BrushTarget::Handle(BrushHandle::W) => "left edge",
        BrushTarget::Handle(BrushHandle::E) => "right edge",
        BrushTarget::Handle(BrushHandle::S) => "bottom edge",
        BrushTarget::Handle(BrushHandle::Nw) => "top-left corner",
        BrushTarget::Handle(BrushHandle::Ne) => "top-right corner",
        BrushTarget::Handle(BrushHandle::Sw) => "bottom-left corner",
        BrushTarget::Handle(BrushHandle::Se) => "bottom-right corner",
    }
}
fn bounds_for(snapshot: &RasterEditableBrushSnapshot, target: BrushTarget) -> Option<Brush> {
    match target {
        BrushTarget::Overlay => Some(snapshot.options.extent),
        BrushTarget::Selection => snapshot.selection,
        BrushTarget::Handle(h) => snapshot
            .handles
            .iter()
            .find(|item| item.handle == h)
            .map(|item| item.bounds),
    }
}
fn box_style(bounds: Option<Brush>) -> Style {
    let Some(bounds) = bounds else {
        return Style::new().display(Display::None);
    };
    if bounds.x1 <= bounds.x0 || bounds.y1 <= bounds.y0 {
        return Style::new().display(Display::None);
    }
    Style::new()
        .position(Position::Absolute)
        .left(Length::Px(bounds.x0))
        .top(Length::Px(bounds.y0))
        .width(Length::Px(bounds.x1 - bounds.x0))
        .height(Length::Px(bounds.y1 - bounds.y0))
}
impl Widget for BrushPlane {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        for target in targets() {
            let class = match target {
                BrushTarget::Overlay => "kartis-raster-brush-overlay".into(),
                BrushTarget::Selection => "kartis-raster-brush-selection".into(),
                BrushTarget::Handle(h) => format!("kartis-raster-brush-handle-{}", h.name()),
            };
            let brush = self.brush;
            let active = self.selected_target;
            let events = self.handle.clone();
            let errors = self.handle.clone();
            let native = self.native.clone();
            let lost = self.native.clone();
            let image_node = self.image_node;
            let owner = scope.clone();
            glory::ui::view()
                .class(class)
                .name(format!("Brush {}", friendly_target_name(target)))
                .focus(FocusMode::TabStop)
                .semantics(Some(Box::new(Semantics {
                    drag_capture: Some(DragCapture::PressFocus),
                    ..Default::default()
                })))
                .style(scope.bond(move || {
                    brush.with(|s| box_style(s.as_ref().and_then(|s| bounds_for(s, target))))
                }))
                .on(EventKind::Drag, move |event| {
                    let result = native_event(&owner, &native, image_node, &events, target, event);
                    events.report(result.map(|()| Vec::new()));
                })
                .on(EventKind::FocusChanged, move |event| {
                    if let Event::FocusChanged(focused) = event {
                        if focused {
                            active.revise(|old| *old = target);
                        } else {
                            retire_native(&lost, &errors);
                        }
                    }
                })
                .show_in(scope)?;
        }
        Ok(())
    }
}

struct KeyboardControls {
    handle: RasterSurfaceHandle,
    brush: Cage<Option<RasterEditableBrushSnapshot>>,
    selected_target: Cage<BrushTarget>,
}
impl Widget for KeyboardControls {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let brush = self.brush;
        let handle = self.handle.clone();
        let active = self.selected_target;
        glory::ui::view()
            .class("kartis-raster-brush-keyboard")
            .name("Brush editing controls")
            .style(scope.bond(move || {
                brush.with(|s| {
                    if s.is_some() {
                        Style::new()
                            .display(Display::Flex)
                            .flex_direction(FlexDirection::Row)
                            .flex_wrap(FlexWrap::Wrap)
                            .gap(4.)
                    } else {
                        Style::new().display(Display::None)
                    }
                })
            }))
            .fill(glory::ui::widget(move |scope| {
                for target in targets().filter(|target| *target != BrushTarget::Overlay) {
                    let selected = active;
                    glory::ui::button()
                        .class(format!(
                            "kartis-raster-brush-target-{}",
                            target_name(target)
                        ))
                        .label(format!("Edit brush {}", friendly_target_name(target)))
                        .enabled(scope.bond(move || {
                            brush.with(|s| {
                                s.as_ref().is_some_and(|s| bounds_for(s, target).is_some())
                            })
                        }))
                        .on(EventKind::Activate, move |_| {
                            selected.revise(|old| *old = target);
                        })
                        .show_in(scope)?;
                }
                for (name, delta) in [
                    ("left", (-1., 0.)),
                    ("right", (1., 0.)),
                    ("up", (0., -1.)),
                    ("down", (0., 1.)),
                ] {
                    let input = handle.clone();
                    glory::ui::button()
                        .class(format!("kartis-raster-brush-nudge-{name}"))
                        .label(format!("Move selected brush control {name}"))
                        .enabled(scope.bond(move || {
                            brush.with(|s| s.as_ref().is_some_and(|s| s.selection.is_some()))
                        }))
                        .on(EventKind::Activate, move |_| {
                            let mut target = active.get_untracked();
                            if brush
                                .get_untracked()
                                .as_ref()
                                .and_then(|s| bounds_for(s, target))
                                .is_none()
                                || target == BrushTarget::Overlay
                            {
                                target = BrushTarget::Selection;
                            }
                            input.report(input.dispatch_editable_brush(
                                EditableBrushInput::Nudge { target, delta },
                            ));
                        })
                        .show_in(scope)?;
                }
                let reset = handle.clone();
                glory::ui::button()
                    .class("kartis-raster-brush-reset")
                    .label("Reset brush selection")
                    .on(EventKind::Activate, move |_| {
                        if let Ok(Some(state)) = reset.editable_brush() {
                            let b = state.options.extent;
                            let dx = (b.x1 - b.x0) / 4.;
                            let dy = (b.y1 - b.y0) / 4.;
                            reset.report(reset.set_brush_selection(Some(Brush {
                                x0: b.x0 + dx,
                                x1: b.x1 - dx,
                                y0: b.y0 + dy,
                                y1: b.y1 - dy,
                            })));
                        }
                    })
                    .show_in(scope)?;
                let clear = handle.clone();
                glory::ui::button()
                    .class("kartis-raster-brush-clear")
                    .label("Clear brush selection")
                    .on(EventKind::Activate, move |_| {
                        clear.report(clear.dispatch_editable_brush(EditableBrushInput::Clear));
                    })
                    .show_in(scope)
            }))
            .show_in(scope)
    }
}

#[derive(Default)]
struct NativeState {
    generation: u64,
    gesture: Option<NativeGesture>,
    failed_session: Option<BrushSession>,
}
struct NativeGesture {
    generation: u64,
    from: NodeId,
    target: BrushTarget,
    start: (f64, f64),
    revision: u64,
    bounds: Option<RasterSurfaceBounds>,
    session: Option<BrushSession>,
    pending: VecDeque<(DragPhase, (f64, f64))>,
    terminal: bool,
    checking_bounds: bool,
}
fn retire_native(state: &Rc<RefCell<NativeState>>, handle: &RasterSurfaceHandle) {
    let gesture = state.borrow_mut().gesture.take();
    if let Some(session) = gesture.and_then(|g| g.session)
        && handle
            .editable_brush()
            .ok()
            .flatten()
            .is_some_and(|s| s.session == Some(session))
    {
        let result = handle.dispatch_editable_brush(EditableBrushInput::Cancel { session, id: 1 });
        if result.is_err() {
            state.borrow_mut().failed_session = Some(session);
        }
        handle.report(result);
    }
}
fn native_apply(
    state: &Rc<RefCell<NativeState>>,
    handle: &RasterSurfaceHandle,
    generation: u64,
    phase: DragPhase,
    point: (f64, f64),
) -> Result<()> {
    let (bounds, target, session) = {
        let state = state.borrow();
        let Some(g) = state
            .gesture
            .as_ref()
            .filter(|g| g.generation == generation)
        else {
            return Ok(());
        };
        (g.bounds.expect("query completed"), g.target, g.session)
    };
    let input = if phase == DragPhase::Start {
        EditableBrushInput::Begin {
            id: 1,
            point,
            target,
            modifiers: Default::default(),
        }
    } else {
        let Some(session) = session else {
            return Ok(());
        };
        if handle
            .editable_brush()?
            .is_none_or(|brush| brush.session != Some(session))
        {
            state.borrow_mut().gesture = None;
            return Ok(());
        }
        match phase {
            DragPhase::Move => EditableBrushInput::Move {
                session,
                id: 1,
                point,
            },
            DragPhase::Drop => EditableBrushInput::End {
                session,
                id: 1,
                point,
            },
            DragPhase::Cancel => EditableBrushInput::Cancel { session, id: 1 },
            DragPhase::Start => unreachable!(),
        }
    };
    let events = match handle.dispatch_editable_brush_host(input, bounds) {
        Ok(events) => events,
        Err(error) => {
            if matches!(phase, DragPhase::Drop | DragPhase::Cancel) {
                retire_native(state, handle);
            }
            return Err(error);
        }
    };
    // An observer may retire this Start and immediately create a different
    // session. Bind the transport to the returned event, never to that new one.
    let started = events.iter().find_map(|event| match event {
        SurfaceEvent::EditableBrush { event, .. }
            if event.phase == BrushPhase::Start && event.source == BrushSource::User =>
        {
            event.session
        }
        _ => None,
    });
    let current = handle.editable_brush()?.and_then(|s| s.session);
    let mut state = state.borrow_mut();
    if let Some(g) = state
        .gesture
        .as_mut()
        .filter(|g| g.generation == generation)
    {
        if phase == DragPhase::Start {
            g.session = started;
        }
        if matches!(phase, DragPhase::Drop | DragPhase::Cancel)
            || current != g.session
            || current.is_none()
        {
            state.gesture = None;
        }
    }
    Ok(())
}
fn native_event(
    scope: &Scope,
    state: &Rc<RefCell<NativeState>>,
    image_node: Cage<Option<NodeId>>,
    handle: &RasterSurfaceHandle,
    target: BrushTarget,
    event: Event,
) -> Result<()> {
    let Event::Drag { from, phase, x, y } = event else {
        return Ok(());
    };
    if phase == DragPhase::Start {
        retire_native(state, handle);
        let Some(editor) = handle.editable_brush()? else {
            return Ok(());
        };
        if editor.active {
            if state.borrow().failed_session == editor.session {
                handle.set_brush_selection(editor.selection)?;
                state.borrow_mut().failed_session = None;
            } else {
                return Ok(());
            }
        }
        let Some(image) = image_node.get_untracked() else {
            return Ok(());
        };
        let revision = handle.snapshot()?.revision;
        let generation = state
            .borrow()
            .generation
            .checked_add(1)
            .ok_or_else(|| ChartError("native brush generation exhausted".into()))?;
        *state.borrow_mut() = NativeState {
            generation,
            gesture: Some(NativeGesture {
                generation,
                from,
                target,
                start: (x, y),
                revision,
                bounds: None,
                session: None,
                pending: VecDeque::new(),
                terminal: false,
                checking_bounds: true,
            }),
            failed_session: None,
        };
        let pending = state.clone();
        let input = handle.clone();
        scope
            .request(Query::Bounds(image), move |result| {
                let apply = || -> Result<()> {
                    let (start, revision) = {
                        let state = pending.borrow();
                        let Some(g) = state
                            .gesture
                            .as_ref()
                            .filter(|g| g.generation == generation)
                        else {
                            return Ok(());
                        };
                        (g.start, g.revision)
                    };
                    if !input.is_active()
                        || image_node.get_untracked() != Some(image)
                        || input.snapshot()?.revision != revision
                    {
                        pending.borrow_mut().gesture = None;
                        return Ok(());
                    }
                    let QueryValue::Bounds(bounds) = result.map_err(error)? else {
                        return Err(ChartError("native brush requires image bounds".into()));
                    };
                    let queued = {
                        let mut state = pending.borrow_mut();
                        let g = state.gesture.as_mut().expect("generation checked");
                        g.bounds = Some(RasterSurfaceBounds {
                            x: bounds.x,
                            y: bounds.y,
                            width: bounds.width,
                            height: bounds.height,
                        });
                        g.checking_bounds = false;
                        std::mem::take(&mut g.pending)
                    };
                    native_apply(&pending, &input, generation, DragPhase::Start, start)?;
                    for (phase, point) in queued {
                        native_apply(&pending, &input, generation, phase, point)?;
                    }
                    Ok(())
                };
                let result = apply();
                if result.is_err() {
                    retire_native(&pending, &input);
                }
                input.report(result.map(|()| Vec::new()));
            })
            .map_err(error)?;
        return Ok(());
    }
    if phase == DragPhase::Cancel {
        retire_native(state, handle);
        return Ok(());
    }
    let overflow = state
        .borrow()
        .gesture
        .as_ref()
        .is_some_and(|g| g.from == from && !g.terminal && g.pending.len() >= 256);
    if overflow {
        retire_native(state, handle);
        return Err(ChartError(
            "native brush pending input budget exceeded".into(),
        ));
    }
    let ready = {
        let mut state = state.borrow_mut();
        let Some(g) = state
            .gesture
            .as_mut()
            .filter(|g| g.from == from && !g.terminal)
        else {
            return Ok(());
        };
        g.pending.push_back((phase, (x, y)));
        g.terminal = phase == DragPhase::Drop;
        match (g.bounds, g.checking_bounds) {
            (Some(bounds), false) => {
                g.checking_bounds = true;
                Some((g.generation, bounds))
            }
            _ => None,
        }
    };
    if let Some((generation, accepted_bounds)) = ready {
        let Some(image) = image_node.get_untracked() else {
            retire_native(state, handle);
            return Ok(());
        };
        let pending = state.clone();
        let input = handle.clone();
        let request = scope.request(Query::Bounds(image), move |result| {
            let apply = || -> Result<()> {
                if pending
                    .borrow()
                    .gesture
                    .as_ref()
                    .is_none_or(|g| g.generation != generation)
                {
                    return Ok(());
                }
                if !input.is_active() || image_node.get_untracked() != Some(image) {
                    retire_native(&pending, &input);
                    return Ok(());
                }
                let QueryValue::Bounds(bounds) = result.map_err(error)? else {
                    return Err(ChartError("native brush requires image bounds".into()));
                };
                let bounds = RasterSurfaceBounds {
                    x: bounds.x,
                    y: bounds.y,
                    width: bounds.width,
                    height: bounds.height,
                };
                // Host layout changes explicitly cancel the old pixel baseline.
                // They never reinterpret an old press using a new image origin.
                if bounds != accepted_bounds {
                    retire_native(&pending, &input);
                    return Ok(());
                }
                let queued = {
                    let mut state = pending.borrow_mut();
                    let g = state.gesture.as_mut().expect("generation checked");
                    g.checking_bounds = false;
                    std::mem::take(&mut g.pending)
                };
                for (phase, point) in queued {
                    native_apply(&pending, &input, generation, phase, point)?;
                }
                Ok(())
            };
            let result = apply();
            if result.is_err() {
                retire_native(&pending, &input);
            }
            input.report(result.map(|()| Vec::new()));
        });
        if let Err(error) = request {
            retire_native(state, handle);
            return Err(super::error(error));
        }
    }
    Ok(())
}
