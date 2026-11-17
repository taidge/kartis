//! Independent typed native mouse observation. No callback is reconstructed
//! from canonical activation, coalesced activity updates, or a fabricated Mark.
use super::*;

/// Seven independent mouse-compatible native phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTreeMouseKind {
    Enter,
    Move,
    Leave,
    Down,
    Up,
    Click,
    DoubleClick,
}
/// The actual target route, separate from Core canonical hover/selection.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrowserTreeMouseOrigin {
    Geometry,
    Mesh,
    SourceModelNearest,
    CapturedSource,
    BoundComponent,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserTreeMousePointer {
    pub id: i32,
    pub kind: String,
    pub primary: bool,
    pub captured: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserTreeMouseOccurrence {
    pub key: String,
    pub index: usize,
    pub publication: u64,
}
/// The first five phases carry actual PointerEvents (MouseEvent subclasses).
/// Mesh transitions can deliver kind Leave with event.type == pointermove.
#[derive(Clone)]
pub struct BrowserTreeMouseInput {
    pub kind: BrowserTreeMouseKind,
    pub event: web_sys::MouseEvent,
    pub client: [f64; 2],
    pub viewport: [f64; 2],
    pub scene: [f64; 2],
    pub pointer: Option<BrowserTreeMousePointer>,
    pub origin: BrowserTreeMouseOrigin,
    pub occurrence: Option<BrowserTreeMouseOccurrence>,
}
#[derive(Clone)]
pub struct BrowserTreeNodeMouseEvent {
    pub data: TreeNodeEventData,
    pub source_generation: u64,
    pub input: BrowserTreeMouseInput,
}
#[derive(Clone)]
pub struct BrowserTreeLinkMouseEvent {
    pub data: TreeLinkEventData,
    pub source_generation: u64,
    pub input: BrowserTreeMouseInput,
}
pub type TreeNodeMouseHandler = Box<dyn FnMut(BrowserTreeNodeMouseEvent) -> Result<()> + 'static>;
pub type TreeLinkMouseHandler = Box<dyn FnMut(BrowserTreeLinkMouseEvent) -> Result<()> + 'static>;

macro_rules! handlers {
    ($name:ident, $handler:ident) => {
        #[derive(Default)]
        pub struct $name {
            pub on_mouse_enter: Option<$handler>,
            pub on_mouse_move: Option<$handler>,
            pub on_mouse_leave: Option<$handler>,
            pub on_mouse_down: Option<$handler>,
            pub on_mouse_up: Option<$handler>,
            pub on_click: Option<$handler>,
            pub on_double_click: Option<$handler>,
        }
        impl $name {
            fn slot(&mut self, kind: BrowserTreeMouseKind) -> &mut Option<$handler> {
                match kind {
                    BrowserTreeMouseKind::Enter => &mut self.on_mouse_enter,
                    BrowserTreeMouseKind::Move => &mut self.on_mouse_move,
                    BrowserTreeMouseKind::Leave => &mut self.on_mouse_leave,
                    BrowserTreeMouseKind::Down => &mut self.on_mouse_down,
                    BrowserTreeMouseKind::Up => &mut self.on_mouse_up,
                    BrowserTreeMouseKind::Click => &mut self.on_click,
                    BrowserTreeMouseKind::DoubleClick => &mut self.on_double_click,
                }
            }
            fn any(&self) -> bool {
                self.on_mouse_enter.is_some()
                    || self.on_mouse_move.is_some()
                    || self.on_mouse_leave.is_some()
                    || self.on_mouse_down.is_some()
                    || self.on_mouse_up.is_some()
                    || self.on_click.is_some()
                    || self.on_double_click.is_some()
            }
        }
    };
}
handlers!(BrowserTreeNodeMouseHandlers, TreeNodeMouseHandler);
handlers!(BrowserTreeLinkMouseHandlers, TreeLinkMouseHandler);

/// Old chart double-click reset remains the default, even with hooks enabled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTreeDoubleClickBehavior {
    #[default]
    KeepChartReset,
    CallbacksOnlyOnHandledTarget,
}
/// Owner keeps the existing automatic native-move activity profile. The
/// explicit component profile changes current on Enter/Leave, not Node Move.
/// It applies to actual native SVG node paths and explicitly bound components;
/// Canvas surface source-model queries keep the Owner profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTreeNodeMouseActivity {
    #[default]
    Owner,
    NativeComponent,
}
#[derive(Default)]
pub struct BrowserTreeMouseHandlers {
    pub nodes: BrowserTreeNodeMouseHandlers,
    pub links: BrowserTreeLinkMouseHandlers,
    pub double_click: BrowserTreeDoubleClickBehavior,
    pub node_activity: BrowserTreeNodeMouseActivity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Target {
    Node(String),
    Link(String),
}
#[derive(Clone)]
struct Hit {
    target: Target,
    origin: BrowserTreeMouseOrigin,
    occurrence: Option<BrowserTreeMouseOccurrence>,
    binding: Option<u64>,
    mesh: bool,
}
impl Hit {
    fn same(&self, other: &Self) -> bool {
        self.target == other.target
            && self.binding == other.binding
            && self.occurrence.as_ref().map(|o| &o.key) == other.occurrence.as_ref().map(|o| &o.key)
    }
}
#[derive(Clone)]
struct Contact {
    pointer: i32,
    hit: Hit,
    native_component: bool,
    press: Option<PressToken>,
}
struct Binding {
    element: Element,
    target: Target,
    occurrence: Option<String>,
}
/// A real, explicitly owned DOM target. Dropping this guard unbinds it; it
/// neither owns the Tree nor changes the target's native event propagation.
pub struct BrowserTreeMouseBinding {
    owner: Weak<Driver>,
    id: u64,
}
impl BrowserTreeMouseBinding {
    pub fn unbind(&self) {
        if let Some(owner) = self.owner.upgrade() {
            owner.mouse.bindings.borrow_mut().remove(&self.id);
        }
    }
}
impl Drop for BrowserTreeMouseBinding {
    fn drop(&mut self) {
        self.unbind();
    }
}
struct MouseListener {
    target: EventTarget,
    name: &'static str,
    capture: bool,
    callback: Closure<dyn FnMut(Event)>,
}
impl Drop for MouseListener {
    fn drop(&mut self) {
        let _ = self.target.remove_event_listener_with_callback_and_bool(
            self.name,
            self.callback.as_ref().unchecked_ref(),
            self.capture,
        );
    }
}
struct MouseCleanup {
    window: Window,
    handle: i32,
    _callback: Closure<dyn FnMut()>,
}
impl Drop for MouseCleanup {
    fn drop(&mut self) {
        self.window.clear_timeout_with_handle(self.handle);
    }
}
struct GeometryCache {
    source: crate::surface::SurfaceFrameToken,
    model: SurfaceModel,
    ink_ids: BTreeSet<String>,
}
struct Envelope {
    event: Event,
    registration: u64,
    generation: u64,
    frame: TreeEventFrame,
    model_token: crate::surface::SurfaceFrameToken,
    svg_stage: Option<Rc<super::super::tree_svg_frame::Stage>>,
    widget_revision: u64,
    input: BrowserTreeMouseInput,
    deliveries: Vec<(BrowserTreeMouseKind, Hit)>,
    tooltip_hit: Option<Hit>,
    tooltip_kind: Option<BrowserTreeMouseKind>,
    press: Option<PressToken>,
    activity: Option<(Option<usize>, bool)>,
}
#[derive(Default)]
pub(super) struct MouseController {
    handlers: RefCell<BrowserTreeMouseHandlers>,
    registration: Cell<u64>,
    bindings: RefCell<BTreeMap<u64, Binding>>,
    next_binding: Cell<u64>,
    frame: RefCell<Option<(u64, TreeEventFrame)>>,
    geometry: RefCell<Option<GeometryCache>>,
    hover: RefCell<Option<Hit>>,
    contact: RefCell<Option<Contact>>,
    envelopes: RefCell<Vec<Envelope>>,
    cleanup: RefCell<Option<MouseCleanup>>,
    listeners: RefCell<Vec<MouseListener>>,
}

impl BrowserTree {
    /// The currently adopted source generation, including activity updates.
    /// It is independent of chart revision and SVG publication. Compare this
    /// with an event's generation to discard asynchronous work from old data.
    /// Reading does not rebuild or replay factories; stopped owners return Err.
    pub fn source_generation(&self) -> Result<u64> {
        self.handle().source_generation()
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserTreeMouseHandlers) -> Result<()> {
        self.handle().set_mouse_handlers(handlers)
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.handle().clear_mouse_handlers()
    }
    pub fn bind_node_mouse_element(
        &self,
        id: &str,
        element: Element,
    ) -> Result<BrowserTreeMouseBinding> {
        self.handle().bind_node_mouse_element(id, element)
    }
    /// `target_id` identifies the child endpoint of the Tree's unique link.
    pub fn bind_link_mouse_element(
        &self,
        target_id: &str,
        element: Element,
    ) -> Result<BrowserTreeMouseBinding> {
        self.handle().bind_link_mouse_element(target_id, element)
    }
}
impl BrowserTreeHandle {
    /// Read the adopted source epoch without entering the mutation busy guard.
    /// Post-adoption effects see the publishing candidate's epoch; an owner
    /// that has stopped or been dropped returns Err.
    pub fn source_generation(&self) -> Result<u64> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        scene::ensure(driver.alive.get(), "Tree owner is stopped")?;
        driver.core()?;
        let accepted = driver
            .accepted
            .try_borrow()
            .map_err(|_| ChartError("Tree accepted data is already in use".into()))?;
        let publishing = driver
            .publishing
            .try_borrow()
            .map_err(|_| ChartError("Tree publish data is already in use".into()))?;
        Ok(publishing
            .as_ref()
            .filter(|p| p.was_adopted())
            .map_or(accepted.generation, |p| p.candidate.generation))
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserTreeMouseHandlers) -> Result<()> {
        let driver = self.driver()?;
        if handlers.links.any() {
            scene::ensure(
                driver.chart()?.snapshot()?.renderer == BrowserRenderer::Svg,
                "Tree link mouse handlers require SVG",
            )?;
        }
        driver.mouse_frame()?;
        if handlers.nodes.any() || handlers.links.any() {
            driver.listen_mouse()?;
        }
        driver.mouse.registration.set(
            driver
                .mouse
                .registration
                .get()
                .checked_add(1)
                .ok_or_else(|| ChartError("Tree mouse handler generation exhausted".into()))?,
        );
        let old = driver.mouse.handlers.replace(handlers);
        driver.clear_mouse_cleanup();
        drop(old);
        if !driver.mouse.handlers.borrow().nodes.any()
            && !driver.mouse.handlers.borrow().links.any()
            && !driver.link_tooltip_interested()
        {
            let listeners = std::mem::take(&mut *driver.mouse.listeners.borrow_mut());
            driver.mouse.hover.borrow_mut().take();
            drop(listeners);
        }
        Ok(())
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.set_mouse_handlers(BrowserTreeMouseHandlers::default())
    }
    pub fn bind_node_mouse_element(
        &self,
        id: &str,
        element: Element,
    ) -> Result<BrowserTreeMouseBinding> {
        self.driver()?.bind_mouse(Target::Node(id.into()), element)
    }
    pub fn bind_link_mouse_element(
        &self,
        target_id: &str,
        element: Element,
    ) -> Result<BrowserTreeMouseBinding> {
        let driver = self.driver()?;
        scene::ensure(
            driver.chart()?.snapshot()?.renderer == BrowserRenderer::Svg,
            "Tree link mouse bindings require SVG",
        )?;
        driver.bind_mouse(Target::Link(target_id.into()), element)
    }
}

impl Driver {
    pub(super) fn mouse_generation(&self) -> u64 {
        let accepted = self.accepted.borrow();
        let publishing = self.publishing.borrow();
        publishing
            .as_ref()
            .filter(|p| p.was_adopted())
            .map_or(accepted.generation, |p| p.candidate.generation)
    }
    fn mouse_widget_revision(&self) -> u64 {
        let accepted = self.accepted.borrow();
        let publishing = self.publishing.borrow();
        publishing
            .as_ref()
            .filter(|p| p.was_adopted())
            .map_or(accepted.widget_revision, |p| p.candidate.widget_revision)
    }
    fn mouse_envelope_is_current(&self, envelope: &Envelope) -> bool {
        if !self.alive.get()
            || envelope.registration != self.mouse.registration.get()
            || envelope.generation != self.mouse_generation()
            || envelope.widget_revision != self.mouse_widget_revision()
        {
            return false;
        }
        let current = self.core.upgrade().and_then(|core| core.tree_svg_stage());
        match (&envelope.svg_stage, current) {
            (Some(captured), Some(current)) => {
                current.was_adopted()
                    && captured.same_admission(&current)
                    && captured.publication() == current.publication()
            }
            (None, None) => true,
            _ => false,
        }
    }
    pub(super) fn mouse_frame(&self) -> Result<(u64, TreeEventFrame)> {
        let generation = self.mouse_generation();
        if let Some((old, frame)) = self.mouse.frame.borrow().as_ref()
            && *old == generation
        {
            return Ok((generation, frame.clone()));
        }
        let (config, layout) = {
            let accepted = self.accepted.borrow();
            let publishing = self.publishing.borrow();
            let value = publishing
                .as_ref()
                .filter(|p| p.was_adopted())
                .map_or(&*accepted, |p| &p.candidate);
            (value.request.config.clone(), value.layout.clone())
        };
        let frame = TreeEventFrame::with_activity(config, layout)?;
        self.mouse.frame.replace(Some((generation, frame.clone())));
        Ok((generation, frame))
    }
    fn mouse_occurrence(&self, key: &str) -> Option<BrowserTreeMouseOccurrence> {
        let stage = self.core.upgrade()?.tree_svg_stage()?;
        if !stage.was_adopted() {
            return None;
        }
        stage
            .plan()
            .layers()
            .iter()
            .find(|o| o.layer.key == key)
            .map(|o| BrowserTreeMouseOccurrence {
                key: key.into(),
                index: o.index,
                publication: stage.publication(),
            })
    }
    fn mouse_owned(&self, element: &Element) -> bool {
        let Some(core) = self.core.upgrade() else {
            return false;
        };
        element.is_connected()
            && element
                .closest(".kartis-browser")
                .ok()
                .flatten()
                .is_some_and(|outer| outer == *core.outer)
            && (self.surface.contains(Some(element.as_ref()))
                || self
                    .overlay
                    .as_ref()
                    .is_some_and(|o| o.contains(Some(element.as_ref()))))
            && element.closest("[inert]").ok().flatten().is_none()
            && self.svg.as_ref().is_none_or(|_| {
                core.tree_svg_stage()
                    .is_some_and(|s| s.was_adopted() && s.root().contains(Some(element.as_ref())))
            })
    }
    fn bind_mouse(
        self: &Rc<Self>,
        target: Target,
        element: Element,
    ) -> Result<BrowserTreeMouseBinding> {
        scene::ensure(
            self.mouse_owned(&element),
            "Tree mouse target is not an active owned element",
        )?;
        let (_, frame) = self.mouse_frame()?;
        scene::ensure(
            target_exists(&frame, &target),
            "Tree mouse target source is missing",
        )?;
        scene::ensure(
            !self
                .mouse
                .bindings
                .borrow()
                .values()
                .any(|b| b.element == element),
            "Tree mouse element is already bound",
        )?;
        scene::ensure(
            self.mouse.bindings.borrow().len() < crate::MAX_MARKS,
            "Tree mouse binding budget exceeded",
        )?;
        let occurrence = element
            .closest(".kartis-tree-svg-occurrence")
            .map_err(host)?
            .and_then(|g| g.get_attribute("data-tree-svg-key"));
        let id = self
            .mouse
            .next_binding
            .get()
            .checked_add(1)
            .ok_or_else(|| ChartError("Tree mouse binding identity exhausted".into()))?;
        self.mouse.next_binding.set(id);
        self.mouse.bindings.borrow_mut().insert(
            id,
            Binding {
                element,
                target,
                occurrence,
            },
        );
        Ok(BrowserTreeMouseBinding {
            owner: Rc::downgrade(self),
            id,
        })
    }
    fn mouse_binding_hit(&self, element: &Element, frame: &TreeEventFrame) -> Option<Hit> {
        // Innermost explicit binding wins; registration order does not make a
        // broad container steal its descendant's independently bound role.
        let bindings = self.mouse.bindings.borrow();
        let mut current = Some(element.clone());
        while let Some(node) = current {
            if let Some((&id, binding)) = bindings.iter().find(|(_, b)| b.element == node)
                && self.mouse_owned(&binding.element)
                && target_exists(frame, &binding.target)
            {
                return Some(Hit {
                    target: binding.target.clone(),
                    origin: BrowserTreeMouseOrigin::BoundComponent,
                    occurrence: binding
                        .occurrence
                        .as_deref()
                        .and_then(|k| self.mouse_occurrence(k)),
                    binding: Some(id),
                    mesh: false,
                });
            }
            current = node.parent_element();
        }
        None
    }
    fn prepare_mouse_geometry(&self, model: &SurfaceModel) -> Result<()> {
        let token = model.frame_token();
        let rebuild = self.mouse.geometry.borrow().as_ref().is_none_or(|c| {
            c.source != token
                || c.model.view() != model.view()
                || c.model.hidden() != model.hidden()
        });
        if rebuild {
            let mut projected = model.clone();
            if self.svg.is_none() {
                let order = model
                    .scene()
                    .draw_items()
                    .into_iter()
                    .filter_map(|item| match item {
                        DrawItem::Mark(i) => Some(i),
                        DrawItem::Label(_) => None,
                    })
                    .collect();
                projected.set_source_paint_order(order)?;
            }
            let ink_ids = projected.source_ink_ids();
            self.mouse.geometry.replace(Some(GeometryCache {
                source: token,
                model: projected,
                ink_ids,
            }));
        }
        Ok(())
    }
    fn mouse_geometry_target(
        &self,
        model: &SurfaceModel,
        point: (f64, f64),
    ) -> Result<Option<usize>> {
        self.prepare_mouse_geometry(model)?;
        let geometry = self.mouse.geometry.borrow();
        let cached = &geometry.as_ref().expect("prepared event geometry").model;
        // Source projection, view, and hidden owners are cached together.
        // No layout or geometry projection is rebuilt for an unchanged Move.
        if !model.plot().contains(point.0, point.1) {
            return Ok(None);
        }
        let world = model.paint_transform().unproject(point)?;
        let query = cached.paint_transform().project(world)?;
        cached.source_paint_target(query)
    }
    fn mouse_source_contact(&self, event: &Event) -> Option<Contact> {
        let contact = self.mouse.contact.borrow().clone()?;
        let token = contact.press?;
        if token.widget || self.press.get().cancelled() || self.press.get().token() != Some(token) {
            return None;
        }
        let matches = if let Some(pointer) = event.dyn_ref::<PointerEvent>() {
            pointer.pointer_id() == contact.pointer
        } else {
            event.dyn_ref::<web_sys::MouseEvent>().is_some_and(|mouse| {
                matches!(event.type_().as_str(), "click" | "dblclick")
                    && mouse.detail() > 0
                    && mouse.button() == 0
            })
        };
        matches.then_some(contact)
    }
    fn mouse_hit(
        &self,
        event: &Event,
        frame: &TreeEventFrame,
        model: &SurfaceModel,
        client: [f64; 2],
    ) -> Result<Option<Hit>> {
        if !self.event_within_plot(event) || !self.svg_native_enabled() {
            return Ok(None);
        }
        let elements: Vec<Element> = event
            .composed_path()
            .iter()
            .filter_map(|n| n.dyn_into().ok())
            .collect();
        if let Some(target) = elements.first()
            && let Some(hit) = self.mouse_binding_hit(target, frame)
        {
            return Ok(Some(hit));
        }
        if elements.iter().any(|e| {
            has_class(e, "kartis-plot-widget-slot") || has_class(e, "kartis-tree-svg-custom-host")
        }) {
            return Ok(None);
        }
        let bounds = self.surface.get_bounding_client_rect();
        if bounds.width() <= 0. || bounds.height() <= 0. {
            return Ok(None);
        }
        let point = (
            (client[0] - bounds.x()) / bounds.width() * model.scene().width,
            (client[1] - bounds.y()) / bounds.height() * model.scene().height,
        );
        let mut occurrence = elements
            .iter()
            .find(|e| has_class(e, "kartis-tree-svg-occurrence"))
            .and_then(|e| e.get_attribute("data-tree-svg-key"))
            .and_then(|k| self.mouse_occurrence(&k));
        let source_contact = self.mouse_source_contact(event);
        let captured = source_contact.is_some()
            || event.dyn_ref::<PointerEvent>().is_some_and(|p| {
                self.core
                    .upgrade()
                    .is_some_and(|c| c.captured.get() == Some(p.pointer_id()))
            });
        if captured && occurrence.is_none() {
            occurrence = source_contact
                .as_ref()
                .and_then(|contact| contact.hit.occurrence.as_ref())
                .and_then(|old| self.mouse_occurrence(&old.key));
        }
        let actual_mesh = source_contact
            .as_ref()
            .is_some_and(|contact| contact.hit.origin == BrowserTreeMouseOrigin::Mesh)
            || elements
                .iter()
                .any(|e| e.has_attribute("data-tree-svg-mesh"))
            || event.dyn_ref::<PointerEvent>().is_some_and(|p| {
                self.core
                    .upgrade()
                    .is_some_and(|c| c.tree_svg_mesh_contacts.borrow().contains(&p.pointer_id()))
            });
        if self.svg.is_some() {
            let Some(stage) = self
                .core
                .upgrade()
                .and_then(|c| c.tree_svg_stage())
                .filter(|s| s.was_adopted())
            else {
                return Ok(None);
            };
            if !elements.iter().any(|e| e == &stage.root()) && !captured {
                return Ok(None);
            }
        }
        if actual_mesh {
            return match self.picking {
                BrowserTreePicking::Nearest { distance } => {
                    Ok(self.nearest(client, distance)?.map(|id| Hit {
                        target: Target::Node(id),
                        origin: BrowserTreeMouseOrigin::Mesh,
                        occurrence,
                        binding: None,
                        mesh: true,
                    }))
                }
                BrowserTreePicking::Geometry => Ok(None),
            };
        }
        self.prepare_mouse_geometry(model)?;
        // SVG paths are the original actual role, including decorative links
        // that intentionally have no canonical business interaction Site.
        if let Some(id) = elements.iter().find_map(|e| e.get_attribute("data-id"))
            && self
                .mouse
                .geometry
                .borrow()
                .as_ref()
                .is_some_and(|c| c.ink_ids.contains(&id))
            && let Some(target) = target_for_mark(frame, &id)
        {
            return Ok(Some(Hit {
                target,
                origin: BrowserTreeMouseOrigin::Geometry,
                occurrence,
                binding: None,
                mesh: false,
            }));
        }
        let captured_geometry = source_contact.as_ref().is_some_and(|contact| {
            matches!(
                contact.hit.origin,
                BrowserTreeMouseOrigin::Geometry | BrowserTreeMouseOrigin::CapturedSource
            )
        });
        if self.svg.is_none()
            && !captured_geometry
            && let BrowserTreePicking::Nearest { distance } = self.picking
        {
            return Ok(self.nearest(client, distance)?.map(|id| Hit {
                target: Target::Node(id),
                origin: BrowserTreeMouseOrigin::SourceModelNearest,
                occurrence: None,
                binding: None,
                mesh: true,
            }));
        }
        if !captured && self.chart()?.snapshot()?.renderer == BrowserRenderer::Svg {
            // A native SVG role requires its actual eligible data-id path.
            // Text/annotations never masquerade as a native node underneath.
            return Ok(None);
        }
        let hit = self
            .mouse_geometry_target(model, point)?
            .and_then(|i| target_for_mark(frame, &model.scene().marks[i].id));
        Ok(hit.map(|target| Hit {
            target,
            origin: if captured {
                BrowserTreeMouseOrigin::CapturedSource
            } else {
                BrowserTreeMouseOrigin::Geometry
            },
            occurrence,
            binding: None,
            mesh: false,
        }))
    }
    fn refresh_mouse_hit(&self, mut hit: Hit, frame: &TreeEventFrame) -> Option<Hit> {
        if !target_exists(frame, &hit.target) {
            return None;
        }
        if let Some(id) = hit.binding {
            let bindings = self.mouse.bindings.borrow();
            let binding = bindings.get(&id)?;
            if !self.mouse_owned(&binding.element) {
                return None;
            }
        }
        if let Some(old) = &hit.occurrence {
            hit.occurrence = Some(self.mouse_occurrence(&old.key)?);
        }
        Some(hit)
    }
    fn capture_mouse(self: &Rc<Self>, event: Event, kind: BrowserTreeMouseKind) -> Result<()> {
        let public_active =
            self.mouse.handlers.borrow().nodes.any() || self.mouse.handlers.borrow().links.any();
        if !self.alive.get() || (!public_active && !self.link_tooltip_interested()) {
            return Ok(());
        }
        let Some(mouse) = event.dyn_ref::<web_sys::MouseEvent>() else {
            return Ok(());
        };
        if kind != BrowserTreeMouseKind::Move && !self.event_within_plot(&event) {
            return Ok(());
        }
        let keyboard_click = mouse.detail() == 0
            && matches!(
                kind,
                BrowserTreeMouseKind::Click | BrowserTreeMouseKind::DoubleClick
            );
        if let Some(p) = event.dyn_ref::<PointerEvent>() {
            if p.pointer_type() == "touch" {
                return Ok(());
            }
            if !p.is_primary()
                && !matches!(
                    kind,
                    BrowserTreeMouseKind::Click | BrowserTreeMouseKind::DoubleClick
                )
            {
                return Ok(());
            }
        }
        let client = client_coordinates(mouse);
        let client = [client.0, client.1];
        if !client.iter().all(|v| v.is_finite()) {
            return Ok(());
        }
        let (generation, frame) = self.mouse_frame()?;
        let model = self.model()?;
        if self.svg.is_some() && kind != BrowserTreeMouseKind::Move {
            let path = event.composed_path();
            let stage = self.core()?.tree_svg_stage();
            let accepted_path = stage.as_ref().is_some_and(|stage| {
                stage.was_adopted()
                    && path
                        .iter()
                        .any(|n| n == wasm_bindgen::JsValue::from(stage.root()))
            });
            let captured = self.mouse_source_contact(&event).is_some()
                || event.dyn_ref::<PointerEvent>().is_some_and(|p| {
                    self.core
                        .upgrade()
                        .is_some_and(|c| c.captured.get() == Some(p.pointer_id()))
                });
            if !accepted_path && !captured {
                return Ok(());
            }
        }
        if kind == BrowserTreeMouseKind::Down {
            self.mouse.contact.borrow_mut().take();
        }
        let renderer = self.chart()?.snapshot()?.renderer;
        let canvas_geometry =
            renderer == BrowserRenderer::Canvas && self.picking == BrowserTreePicking::Geometry;
        let captured_native_component = self
            .mouse_source_contact(&event)
            .is_some_and(|contact| contact.native_component);
        let preserve_native_current = public_active
            && renderer == BrowserRenderer::Svg
            && (self.event_within_plot(&event) || self.mouse_source_contact(&event).is_some())
            && (self.picking == BrowserTreePicking::Geometry || captured_native_component)
            && self.mouse.handlers.borrow().node_activity
                == BrowserTreeNodeMouseActivity::NativeComponent;
        let hit = self.mouse_hit(&event, &frame, &model, client)?;
        let tooltip_hit = hit.clone();
        let previous = self
            .mouse
            .hover
            .borrow()
            .clone()
            .and_then(|h| self.refresh_mouse_hit(h, &frame));
        let mut deliveries = Vec::new();
        match kind {
            BrowserTreeMouseKind::Enter => {
                if let Some(hit) = hit {
                    if !previous.as_ref().is_some_and(|old| old.same(&hit)) {
                        deliveries.push((kind, hit.clone()));
                    }
                    self.mouse.hover.replace(Some(hit));
                }
            }
            BrowserTreeMouseKind::Leave => {
                let related = event
                    .dyn_ref::<PointerEvent>()
                    .and_then(|p| p.related_target())
                    .and_then(|n| n.dyn_into::<Element>().ok());
                let same = related.as_ref().is_some_and(|node| {
                    let source_contact = self.mouse_source_contact(&event);
                    let related_surface = *node == Element::from(self.surface.clone());
                    // The exact Surface is an ancestor of the literal layer
                    // root, not a bindable child of that root. Only a matching
                    // live source lease may normalize this capture boundary.
                    if !(self.mouse_owned(node) || related_surface && source_contact.is_some()) {
                        return false;
                    }
                    let related_occurrence = node
                        .closest(".kartis-tree-svg-occurrence")
                        .ok()
                        .flatten()
                        .and_then(|e| e.get_attribute("data-tree-svg-key"))
                        .or_else(|| {
                            related_surface
                                .then(|| {
                                    source_contact
                                        .as_ref()
                                        .and_then(|contact| contact.hit.occurrence.as_ref())
                                        .map(|o| o.key.clone())
                                })
                                .flatten()
                        });
                    if related_occurrence.as_deref()
                        != hit
                            .as_ref()
                            .and_then(|h| h.occurrence.as_ref().map(|o| o.key.as_str()))
                    {
                        return false;
                    }
                    if let Some(hit) = &hit {
                        if related_surface
                            && source_contact.is_some()
                            && previous.as_ref().is_some_and(|old| old.same(hit))
                        {
                            let bounds = self.surface.get_bounding_client_rect();
                            let point = (
                                (client[0] - bounds.x()) / bounds.width() * model.scene().width,
                                (client[1] - bounds.y()) / bounds.height() * model.scene().height,
                            );
                            let target = if hit.mesh {
                                match self.picking {
                                    BrowserTreePicking::Nearest { distance } => self
                                        .nearest(client, distance)
                                        .ok()
                                        .flatten()
                                        .map(Target::Node),
                                    BrowserTreePicking::Geometry => None,
                                }
                            } else {
                                self.mouse_geometry_target(&model, point)
                                    .ok()
                                    .flatten()
                                    .and_then(|index| {
                                        target_for_mark(&frame, &model.scene().marks[index].id)
                                    })
                            };
                            if target.as_ref() == Some(&hit.target) {
                                return true;
                            }
                        }
                        if let Some(id) = hit.binding {
                            return self
                                .mouse
                                .bindings
                                .borrow()
                                .get(&id)
                                .is_some_and(|b| b.element.contains(Some(node.as_ref())));
                        }
                        if hit.mesh {
                            return node
                                .closest("[data-tree-svg-mesh]")
                                .ok()
                                .flatten()
                                .is_some()
                                || (self.svg.is_none()
                                    && self.surface.contains(Some(node.as_ref())));
                        }
                        return node
                            .closest("[data-id]")
                            .ok()
                            .flatten()
                            .and_then(|n| n.get_attribute("data-id"))
                            .and_then(|id| target_for_mark(&frame, &id))
                            .is_some_and(|t| t == hit.target);
                    }
                    false
                });
                if !same {
                    if let Some(hit) = hit.or(previous) {
                        deliveries.push((kind, hit));
                    }
                    self.mouse.hover.borrow_mut().take();
                }
            }
            BrowserTreeMouseKind::Move => {
                if hit.as_ref().is_some_and(|h| h.mesh) || previous.as_ref().is_some_and(|h| h.mesh)
                {
                    match (&previous, &hit) {
                        (Some(old), Some(next)) if old.same(next) => {
                            deliveries.push((kind, next.clone()))
                        }
                        (Some(old), _) => {
                            deliveries.push((BrowserTreeMouseKind::Leave, old.clone()))
                        }
                        (None, Some(next)) => {
                            deliveries.push((BrowserTreeMouseKind::Enter, next.clone()))
                        }
                        _ => {}
                    }
                } else if canvas_geometry
                    && hit.as_ref().is_none_or(|h| h.binding.is_none())
                    && previous.as_ref().is_none_or(|h| h.binding.is_none())
                {
                    match (&previous, &hit) {
                        (Some(old), Some(next)) if old.same(next) => {
                            deliveries.push((kind, next.clone()))
                        }
                        (old, next) => {
                            if let Some(old) = old {
                                deliveries.push((BrowserTreeMouseKind::Leave, old.clone()));
                            }
                            if let Some(next) = next {
                                deliveries.push((BrowserTreeMouseKind::Enter, next.clone()));
                            }
                        }
                    }
                } else if let Some(hit) = &hit {
                    deliveries.push((kind, hit.clone()));
                }
                self.mouse.hover.replace(hit);
            }
            _ => {
                // Nivo's discrete Mesh queries update its internal match but
                // do not enqueue external current/tooltip setters.
                if hit.as_ref().is_some_and(|h| h.mesh) || previous.as_ref().is_some_and(|h| h.mesh)
                {
                    self.mouse.hover.replace(hit.clone());
                }
                if let Some(hit) = hit {
                    if kind == BrowserTreeMouseKind::Down
                        && let Some(p) = event.dyn_ref::<PointerEvent>()
                    {
                        let native_component = renderer == BrowserRenderer::Svg
                            && matches!(hit.target, Target::Node(_))
                            && hit.binding.is_none()
                            && event
                                .composed_path()
                                .iter()
                                .filter_map(|n| n.dyn_into::<Element>().ok())
                                .any(|e| e.has_attribute("data-id"));
                        self.mouse.contact.replace(Some(Contact {
                            pointer: p.pointer_id(),
                            hit: hit.clone(),
                            native_component,
                            press: self.press.get().token(),
                        }));
                    }
                    deliveries.push((kind, hit));
                }
            }
        }
        let tooltip_kind = if kind == BrowserTreeMouseKind::Leave && deliveries.is_empty() {
            None
        } else {
            Some(kind)
        };
        let tooltip_hit = if kind == BrowserTreeMouseKind::Leave {
            deliveries.first().map(|(_, hit)| hit.clone())
        } else {
            tooltip_hit
        };
        if deliveries.is_empty()
            && !(preserve_native_current && kind == BrowserTreeMouseKind::Move)
            && !(self.link_tooltip_interested() && kind == BrowserTreeMouseKind::Move)
        {
            return Ok(());
        }
        if !keyboard_click
            && matches!(
                kind,
                BrowserTreeMouseKind::Up
                    | BrowserTreeMouseKind::Click
                    | BrowserTreeMouseKind::DoubleClick
            )
            && self.press.get().cancelled()
        {
            return Ok(());
        }
        let bounds = self.surface.get_bounding_client_rect();
        if bounds.width() <= 0. || bounds.height() <= 0. {
            return Ok(());
        }
        let viewport = [
            (client[0] - bounds.x()) / bounds.width() * model.scene().width,
            (client[1] - bounds.y()) / bounds.height() * model.scene().height,
        ];
        let world = model
            .paint_transform()
            .unproject((viewport[0], viewport[1]))?;
        let pointer = event
            .dyn_ref::<PointerEvent>()
            .map(|p| BrowserTreeMousePointer {
                id: p.pointer_id(),
                kind: p.pointer_type(),
                primary: p.is_primary(),
                captured: self
                    .core
                    .upgrade()
                    .is_some_and(|c| c.captured.get() == Some(p.pointer_id())),
            });
        let input = BrowserTreeMouseInput {
            kind,
            event: mouse.clone(),
            client,
            viewport,
            scene: [world.0, world.1],
            pointer,
            origin: deliveries
                .first()
                .map_or(BrowserTreeMouseOrigin::Geometry, |(_, h)| h.origin),
            occurrence: deliveries.first().and_then(|(_, h)| h.occurrence.clone()),
        };
        let activity = if preserve_native_current && kind == BrowserTreeMouseKind::Move {
            let index = deliveries.first().and_then(|(_, hit)| match &hit.target {
                Target::Node(id) => frame.node_by_id().get(id).copied(),
                Target::Link(_) => None,
            });
            Some((index, false))
        } else if public_active
            && let Some((_, hit)) = deliveries.first()
            && !hit.mesh
            && (hit.binding.is_some()
                || (renderer == BrowserRenderer::Svg
                    && event
                        .composed_path()
                        .iter()
                        .filter_map(|n| n.dyn_into::<Element>().ok())
                        .any(|e| e.has_attribute("data-id"))))
            && let Target::Node(id) = &hit.target
        {
            match kind {
                BrowserTreeMouseKind::Enter if event.is_trusted() => {
                    Some((frame.node_by_id().get(id).copied(), true))
                }
                BrowserTreeMouseKind::Leave if event.is_trusted() => Some((None, true)),
                BrowserTreeMouseKind::Move
                    if self.mouse.handlers.borrow().node_activity
                        == BrowserTreeNodeMouseActivity::NativeComponent =>
                {
                    Some((frame.node_by_id().get(id).copied(), false))
                }
                _ => None,
            }
        } else {
            None
        };
        if kind == BrowserTreeMouseKind::DoubleClick {
            let handlers = self.mouse.handlers.borrow();
            let handled = deliveries.iter().any(|(_, hit)| match &hit.target {
                Target::Node(_) => handlers.nodes.on_double_click.is_some(),
                Target::Link(_) => handlers.links.on_double_click.is_some(),
            });
            if handled
                && handlers.double_click
                    == BrowserTreeDoubleClickBehavior::CallbacksOnlyOnHandledTarget
            {
                self.core()?
                    .tree_mouse_double_click
                    .replace(Some(event.clone()));
            }
        }
        scene::ensure(
            self.mouse.envelopes.borrow().len() < 256,
            "Tree native mouse envelope budget exceeded",
        )?;
        self.mouse.envelopes.borrow_mut().push(Envelope {
            event,
            registration: self.mouse.registration.get(),
            generation,
            frame,
            model_token: model.frame_token(),
            svg_stage: self
                .core()?
                .tree_svg_stage()
                .filter(|stage| stage.was_adopted()),
            widget_revision: self.mouse_widget_revision(),
            input,
            deliveries,
            tooltip_hit,
            tooltip_kind,
            press: if keyboard_click {
                None
            } else {
                self.press.get().token()
            },
            activity,
        });
        self.schedule_mouse_cleanup()?;
        Ok(())
    }
    fn deliver_mouse(&self, event: &Event) -> Result<()> {
        let envelope = {
            let mut queue = self.mouse.envelopes.borrow_mut();
            queue
                .iter()
                .position(|e| e.event == *event)
                .map(|i| queue.remove(i))
        };
        let Some(envelope) = envelope else {
            return Ok(());
        };
        if !self.mouse_envelope_is_current(&envelope)
            || self.model()?.frame_token() != envelope.model_token
        {
            return Ok(());
        }
        if envelope.press.is_some_and(|old| {
            self.press.get().token() != Some(old) || self.press.get().cancelled()
        }) {
            return Ok(());
        }
        if let Some(kind) = envelope.tooltip_kind {
            let (id, origin, occurrence, binding) = envelope.tooltip_hit.as_ref().map_or(
                (None, BrowserTreeMouseOrigin::Geometry, None, None),
                |hit| {
                    (
                        match &hit.target {
                            Target::Link(id) => Some(id.as_str()),
                            Target::Node(_) => None,
                        },
                        hit.origin,
                        hit.occurrence.clone(),
                        hit.binding,
                    )
                },
            );
            self.select_link_tooltip(
                kind,
                id,
                envelope.input.client,
                origin,
                occurrence,
                binding,
                envelope.input.event.is_trusted(),
            );
            if self.tooltip_dirty.get() || self.link_tooltip_dirty.get() {
                self.request()?;
            }
        }
        if let Some((index, set_current)) = envelope.activity {
            self.intent.set(Some(PointerIntent::MouseNode {
                client: envelope.input.client,
                trusted: envelope.input.event.is_trusted(),
                index,
                generation: envelope.generation,
                set_current,
            }));
            self.request()?;
        }
        for (kind, hit) in &envelope.deliveries {
            if !self.mouse_envelope_is_current(&envelope)
                || self.model()?.frame_token() != envelope.model_token
            {
                break;
            }
            let Some(hit) = self.refresh_mouse_hit(hit.clone(), &envelope.frame) else {
                continue;
            };
            let kind = *kind;
            let mut input = envelope.input.clone();
            input.kind = kind;
            input.origin = hit.origin;
            input.occurrence = hit.occurrence;
            match hit.target {
                Target::Node(id) => {
                    let Some(data) = envelope.frame.find_node(&id) else {
                        continue;
                    };
                    let callback = self.mouse.handlers.borrow_mut().nodes.slot(kind).take();
                    if let Some(mut callback) = callback {
                        let result = callback(BrowserTreeNodeMouseEvent {
                            data,
                            source_generation: envelope.generation,
                            input,
                        });
                        if self.alive.get()
                            && envelope.registration == self.mouse.registration.get()
                        {
                            *self.mouse.handlers.borrow_mut().nodes.slot(kind) = Some(callback);
                        } else {
                            drop(callback);
                        }
                        result?;
                    }
                }
                Target::Link(id) => {
                    let Some(index) = link_index(&envelope.frame, &id) else {
                        continue;
                    };
                    let data = envelope.frame.link(index)?;
                    let callback = self.mouse.handlers.borrow_mut().links.slot(kind).take();
                    if let Some(mut callback) = callback {
                        let result = callback(BrowserTreeLinkMouseEvent {
                            data,
                            source_generation: envelope.generation,
                            input,
                        });
                        if self.alive.get()
                            && envelope.registration == self.mouse.registration.get()
                        {
                            *self.mouse.handlers.borrow_mut().links.slot(kind) = Some(callback);
                        } else {
                            drop(callback);
                        }
                        result?;
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn mouse_link_binding_valid(&self, binding: u64, id: &str) -> bool {
        self.mouse
            .bindings
            .borrow()
            .get(&binding)
            .is_some_and(|bound| {
                bound.target == Target::Link(id.into()) && self.mouse_owned(&bound.element)
            })
    }
    pub(super) fn prune_mouse_interest(&self) {
        if self.mouse.handlers.borrow().nodes.any()
            || self.mouse.handlers.borrow().links.any()
            || self.link_tooltip_interested()
        {
            return;
        }
        self.clear_mouse_cleanup();
        let listeners = std::mem::take(&mut *self.mouse.listeners.borrow_mut());
        self.mouse.hover.borrow_mut().take();
        self.mouse.contact.borrow_mut().take();
        drop(listeners);
    }
    pub(super) fn clear_mouse_cleanup(&self) {
        let ticket = self.mouse.cleanup.borrow_mut().take();
        self.mouse.envelopes.borrow_mut().clear();
        if let Some(core) = self.core.upgrade() {
            core.tree_mouse_double_click.borrow_mut().take();
        }
        drop(ticket);
    }
    fn schedule_mouse_cleanup(self: &Rc<Self>) -> Result<()> {
        if self.mouse.cleanup.borrow().is_some() {
            return Ok(());
        }
        let weak = Rc::downgrade(self);
        let callback = Closure::<dyn FnMut()>::new(move || {
            if let Some(owner) = weak.upgrade() {
                owner.clear_mouse_cleanup();
            }
        });
        // Native UA dispatch can run microtasks between capture and bubble.
        // A subsequent task retires only undelivered/stopped envelopes after
        // synchronous native transport and typed callbacks have completed.
        let handle = match self
            .window
            .set_timeout_with_callback_and_timeout_and_arguments_0(
                callback.as_ref().unchecked_ref(),
                0,
            ) {
            Ok(handle) => handle,
            Err(error) => {
                self.clear_mouse_cleanup();
                return Err(host(error));
            }
        };
        self.mouse.cleanup.replace(Some(MouseCleanup {
            window: self.window.clone(),
            handle,
            _callback: callback,
        }));
        Ok(())
    }
    pub(super) fn listen_mouse(self: &Rc<Self>) -> Result<()> {
        if !self.mouse.listeners.borrow().is_empty() {
            return Ok(());
        }
        let mut listeners = Vec::new();
        for (name, kind) in [
            ("pointerover", BrowserTreeMouseKind::Enter),
            ("pointermove", BrowserTreeMouseKind::Move),
            ("pointerout", BrowserTreeMouseKind::Leave),
            ("pointerdown", BrowserTreeMouseKind::Down),
            ("pointerup", BrowserTreeMouseKind::Up),
            ("click", BrowserTreeMouseKind::Click),
            ("dblclick", BrowserTreeMouseKind::DoubleClick),
        ] {
            for capture in [true, false] {
                let weak = Rc::downgrade(self);
                let callback = Closure::new(move |event: Event| {
                    if let Some(owner) = weak.upgrade().filter(|d| d.alive.get()) {
                        let result = if capture {
                            owner.capture_mouse(event, kind)
                        } else {
                            owner.deliver_mouse(&event)
                        };
                        if let Err(error) = result
                            && owner.alive.get()
                        {
                            owner.record(error);
                            let _ = owner.request();
                        }
                    }
                });
                let target: EventTarget = self.window.clone().into();
                target
                    .add_event_listener_with_callback_and_bool(
                        name,
                        callback.as_ref().unchecked_ref(),
                        capture,
                    )
                    .map_err(host)?;
                listeners.push(MouseListener {
                    target,
                    name,
                    capture,
                    callback,
                });
            }
        }
        *self.mouse.listeners.borrow_mut() = listeners;
        Ok(())
    }
}

impl MouseController {
    pub(super) fn stop(&self) {
        let listeners = std::mem::take(&mut *self.listeners.borrow_mut());
        let handlers = self.handlers.replace(BrowserTreeMouseHandlers::default());
        let bindings = std::mem::take(&mut *self.bindings.borrow_mut());
        let cleanup = self.cleanup.borrow_mut().take();
        self.envelopes.borrow_mut().clear();
        self.hover.borrow_mut().take();
        self.contact.borrow_mut().take();
        self.frame.borrow_mut().take();
        self.geometry.borrow_mut().take();
        drop(cleanup);
        drop(listeners);
        drop(handlers);
        drop(bindings);
    }
}
fn link_index(frame: &TreeEventFrame, target_id: &str) -> Option<usize> {
    frame
        .links()
        .iter()
        .position(|link| frame.source()[link.target_index].id == target_id)
}
fn target_exists(frame: &TreeEventFrame, target: &Target) -> bool {
    match target {
        Target::Node(id) => frame.node_by_id().contains_key(id),
        Target::Link(id) => link_index(frame, id).is_some(),
    }
}
fn target_for_mark(frame: &TreeEventFrame, id: &str) -> Option<Target> {
    frame
        .nodes()
        .iter()
        .find(|n| n.mark_id.as_deref() == Some(id))
        .map(|n| Target::Node(frame.source()[n.source_index].id.clone()))
        .or_else(|| {
            frame
                .links()
                .iter()
                .find(|l| l.mark_id.as_deref() == Some(id))
                .map(|l| Target::Link(frame.source()[l.target_index].id.clone()))
        })
}
