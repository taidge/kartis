//! Private retained tooltip panel. Both typed roles are adopted by one host Stage.
use glory::{Bond, Widget};

use super::tree_tooltip_host::{
    BrowserTreeTooltipAnchor, BrowserTreeTooltipOptions, BrowserTreeTooltipPosition,
};
use super::*;

pub(super) trait Payload: Clone + PartialEq + 'static {
    type Context;
    const ROOT_CLASS: &'static str;
    const TEXT_CLASS: &'static str;
    const COMPONENT_CLASS: &'static str;
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context;
    fn context_with_scope(
        revision: u64,
        frame: Bond<Option<Self>>,
        _scope: &glory::Scope,
    ) -> Self::Context {
        Self::context(revision, frame)
    }
    fn source_generation(&self) -> u64;
    fn scene(&self) -> &Scene;
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool;
    fn world_anchor(&self) -> [f64; 2];
    fn pointer_client(&self) -> Option<[f64; 2]>;
    fn viewport_anchor(&self) -> [f64; 2];
    fn set_viewport_anchor(&mut self, point: [f64; 2]);
    fn anchor(&self) -> BrowserTreeTooltipAnchor;
    fn resolve_anchor(
        &mut self,
        requested: BrowserTreeTooltipAnchor,
        local_x: f64,
        width: f64,
    ) -> BrowserTreeTooltipAnchor;
    fn text_record(
        document: &web_sys::Document,
        text: &str,
        _: Option<&Self>,
    ) -> Result<HtmlElement> {
        let root = element(document, Self::TEXT_CLASS)?;
        root.set_text_content(Some(text));
        Ok(root)
    }
}

type PanelFactory<F> = dyn FnMut(<F as Payload>::Context) -> Result<Box<dyn Widget>>;
pub(super) struct PanelComponent<F: Payload> {
    revision: u64,
    factory: RefCell<Option<Box<PanelFactory<F>>>>,
}
impl<F: Payload> PanelComponent<F> {
    pub(super) fn revision(&self) -> u64 {
        self.revision
    }
    pub(super) fn new<W: Widget>(
        revision: u64,
        mut factory: impl FnMut(<F as Payload>::Context) -> Result<W> + 'static,
    ) -> Rc<Self> {
        Rc::new(Self {
            revision,
            factory: RefCell::new(Some(Box::new(move |c| Ok(Box::new(factory(c)?))))),
        })
    }
}
#[derive(Clone)]
pub(super) enum PanelContent<F: Payload> {
    Unmanaged,
    Text(String),
    Component(Rc<PanelComponent<F>>),
}
struct PanelRecord<F: Payload> {
    root: HtmlElement,
    web: Option<WebApp>,
    frame: Option<Cage<Option<F>>>,
    component: Option<Rc<PanelComponent<F>>>,
    failed: Cell<bool>,
    _runtime: Option<RuntimeLease>,
}
struct RuntimeBudget {
    records: Rc<Cell<usize>>,
    limit: Cell<usize>,
}
struct RuntimeLease(Rc<RuntimeBudget>);
impl RuntimeLease {
    fn acquire(budget: &Rc<RuntimeBudget>) -> Result<Self> {
        scene::ensure(
            budget.records.get() < budget.limit.get(),
            "tooltip component budget exceeded",
        )?;
        budget.records.set(budget.records.get() + 1);
        Ok(Self(budget.clone()))
    }
}
impl Drop for RuntimeLease {
    fn drop(&mut self) {
        self.0.records.set(self.0.records.get().saturating_sub(1));
    }
}
impl<F: Payload> Drop for PanelRecord<F> {
    fn drop(&mut self) {
        if let Some(web) = &self.web {
            let _ = web.app().clear();
        }
        self.root.remove();
    }
}
struct Placement {
    host_style: Option<String>,
    root_style: Option<String>,
    previous_style: Option<String>,
    parent: Option<web_sys::Node>,
    next: Option<web_sys::Node>,
    focus: Option<HtmlElement>,
}
pub(super) struct PanelStage<F: Payload> {
    host: Weak<PanelHost<F>>,
    pub(super) source_generation: u64,
    frame: RefCell<Option<F>>,
    options: BrowserTreeTooltipOptions,
    content: PanelContent<F>,
    appearance: TooltipOptions,
    record: Option<Rc<PanelRecord<F>>>,
    previous: RefCell<Option<Rc<PanelRecord<F>>>>,
    placed: RefCell<Option<Placement>>,
    adopted: Cell<bool>,
    reserved: Cell<bool>,
    created_shell: Cell<bool>,
}
impl<F: Payload> PanelStage<F> {
    pub(super) fn was_adopted(&self) -> bool {
        self.adopted.get()
    }
    pub(super) fn managed(&self) -> bool {
        !matches!(self.content, PanelContent::Unmanaged)
    }
    pub(super) fn frame(&self) -> Option<F> {
        self.frame.borrow().clone()
    }
    pub(super) fn update_frame(&self, update: impl FnOnce(&mut Option<F>)) {
        update(&mut self.frame.borrow_mut());
    }
    pub(super) fn validate(&self, model: &SurfaceModel) -> Result<()> {
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree tooltip host retired".into()))?;
        scene::ensure(
            host.alive.get() && !self.adopted.get(),
            "Tree tooltip stage retired or adopted",
        )?;
        if let Some(frame) = self.frame.borrow().as_ref() {
            scene::ensure(
                frame.source_generation() == self.source_generation
                    && frame.scene() == model.scene()
                    && frame.viewport_anchor().iter().all(|x| x.is_finite())
                    && frame
                        .pointer_client()
                        .is_none_or(|p| p.iter().all(|x| x.is_finite())),
                "Tree tooltip frame differs from source generation or Scene",
            )?;
        }
        Ok(())
    }
    /// Fallible DOM movement happens before either accepted data stage changes.
    pub(super) fn place(
        &self,
        model: &SurfaceModel,
        surface: &HtmlElement,
        enabled: bool,
    ) -> Result<()> {
        self.validate(model)?;
        if self.placed.borrow().is_some() {
            return Ok(());
        }
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree tooltip host retired".into()))?;
        if !self.managed() && host.root.borrow().is_none() {
            return Ok(());
        }
        let existed = host.root.borrow().is_some();
        let root = host.root()?;
        if !existed {
            self.created_shell.set(true);
        }
        let mut frame = self.frame.borrow().clone();
        if frame.as_ref().is_some_and(|f| f.is_hidden(model.hidden())) {
            frame = None;
        }
        let bounds = surface.get_bounding_client_rect();
        if bounds.width() <= 0. || bounds.height() <= 0. || !enabled {
            frame = None;
        }
        let (point, local_size) = if let Some(frame) = frame.as_mut() {
            let viewport = match self.options.position {
                BrowserTreeTooltipPosition::Fixed => {
                    let p = frame.world_anchor();
                    model.paint_transform().project((p[0], p[1]))?
                }
                BrowserTreeTooltipPosition::Cursor => {
                    let p = frame.pointer_client().ok_or_else(|| {
                        ChartError("Tree cursor tooltip has no native pointer".into())
                    })?;
                    (
                        (p[0] - bounds.x()) / bounds.width() * model.scene().width,
                        (p[1] - bounds.y()) / bounds.height() * model.scene().height,
                    )
                }
            };
            frame.set_viewport_anchor([viewport.0, viewport.1]);
            let parent_size = resolved_size(&host.parent)?;
            let parent_bounds = host.parent.get_bounding_client_rect();
            scene::ensure(
                parent_bounds.width() > 0. && parent_bounds.height() > 0.,
                "Tree tooltip parent has no CSS box",
            )?;
            let client = match self.options.position {
                BrowserTreeTooltipPosition::Fixed => [
                    bounds.x() + viewport.0 / model.scene().width * bounds.width(),
                    bounds.y() + viewport.1 / model.scene().height * bounds.height(),
                ],
                BrowserTreeTooltipPosition::Cursor => {
                    frame.pointer_client().expect("checked native cursor")
                }
            };
            let local = [
                (client[0] - parent_bounds.x()) / parent_bounds.width() * parent_size[0],
                (client[1] - parent_bounds.y()) / parent_bounds.height() * parent_size[1],
            ];
            let local_size = [
                bounds.width() / parent_bounds.width() * parent_size[0],
                bounds.height() / parent_bounds.height() * parent_size[1],
            ];
            let anchor = frame.resolve_anchor(self.options.anchor, local[0], parent_bounds.width());
            let gap = self.options.gap;
            let shift = match anchor {
                BrowserTreeTooltipAnchor::Top => [0., -gap],
                BrowserTreeTooltipAnchor::TopRight => [gap, -gap],
                BrowserTreeTooltipAnchor::Right => [gap, 0.],
                BrowserTreeTooltipAnchor::BottomRight => [gap, gap],
                BrowserTreeTooltipAnchor::Bottom => [0., gap],
                BrowserTreeTooltipAnchor::BottomLeft => [-gap, gap],
                BrowserTreeTooltipAnchor::Left => [-gap, 0.],
                BrowserTreeTooltipAnchor::TopLeft => [-gap, -gap],
                BrowserTreeTooltipAnchor::Center => [0., 0.],
            };
            // Round untransformed local CSS coordinates as the public tooltip
            // primitive does; ancestor CSS scaling then applies exactly once.
            (
                Some([
                    (local[0] + shift[0] + 0.5).floor(),
                    (local[1] + shift[1] + 0.5).floor(),
                ]),
                Some(local_size),
            )
        } else {
            (None, None)
        };
        // Budgeted Funnel panels own a separate HTML overlay, outside the SVG
        // source carrier journal. Moving even the same retained record can
        // blur its focused descendant before a later style write is rejected.
        let focus = if host.runtime_budget.is_some() {
            root.owner_document()
                .and_then(|document| document.active_element())
                .filter(|active| {
                    self.record
                        .as_ref()
                        .is_some_and(|record| record.root.contains(Some(active.as_ref())))
                        || self
                            .previous
                            .borrow()
                            .as_ref()
                            .is_some_and(|record| record.root.contains(Some(active.as_ref())))
                })
                .and_then(|active| active.dyn_into::<HtmlElement>().ok())
        } else {
            None
        };
        let placement = Placement {
            host_style: root.get_attribute("style"),
            root_style: self
                .record
                .as_ref()
                .and_then(|r| r.root.get_attribute("style")),
            previous_style: self
                .previous
                .borrow()
                .as_ref()
                .and_then(|r| r.root.get_attribute("style")),
            parent: self.record.as_ref().and_then(|r| r.root.parent_node()),
            next: self.record.as_ref().and_then(|r| r.root.next_sibling()),
            focus: focus.clone(),
        };
        *self.placed.borrow_mut() = Some(placement);
        let result: Result<()> = (|| {
            if let Some(record) = &self.record {
                root.append_child(&record.root).map_err(host_error)?;
                let failed = host.runtime_budget.is_some() && record.failed.get();
                css(
                    &record.root,
                    "visibility",
                    if failed { "hidden" } else { "visible" },
                )?;
                css(&record.root, "position", "relative")?;
                css(
                    &record.root,
                    "display",
                    if failed { "none" } else { "block" },
                )?;
            }
            if let Some(old) = self.previous.borrow().as_ref()
                && self.record.as_ref().is_none_or(|r| !Rc::ptr_eq(old, r))
            {
                css(&old.root, "display", "none")?;
            }
            if let Some(local_size) = local_size {
                css(
                    &root,
                    "max-width",
                    &format!("{}px", local_size[0].min(1_000_000.)),
                )?;
                css(
                    &root,
                    "max-height",
                    &format!("{}px", local_size[1].min(1_000_000.)),
                )?;
            }
            let anchor = frame
                .as_ref()
                .map_or(self.options.anchor, |frame| frame.anchor());
            css(&root, "transform", anchor.translation())?;
            css(
                &root,
                "font-size",
                &format!("{}px", self.appearance.text.size),
            )?;
            css(
                &root,
                "font-weight",
                &self.appearance.text.weight.to_string(),
            )?;
            css(
                &root,
                "line-height",
                &self.appearance.text.line_height.to_string(),
            )?;
            css(&root, "padding", &format!("{}px", self.appearance.padding))?;
            if let Some(color) = self.appearance.background {
                css(&root, "background", &render::css_color(color))?;
            }
            if let Some(color) = self.appearance.text.color {
                css(&root, "color", &render::css_color(color))?;
            }
            if let Some(frame) = frame.as_ref() {
                let font = frame
                    .scene()
                    .typography
                    .tooltip_font(&self.appearance.text.font, self.appearance.text.weight);
                css(&root, "font-family", &font.css_family())?;
            }
            if let Some([x, y]) = point {
                css(&root, "left", &format!("{x}px"))?;
                css(&root, "top", &format!("{y}px"))?;
            }
            let empty_text = matches!(&self.content, PanelContent::Text(text) if text.is_empty());
            let failed = host.runtime_budget.is_some()
                && self
                    .record
                    .as_ref()
                    .is_some_and(|record| record.failed.get());
            css(
                &root,
                "display",
                if point.is_some() && self.managed() && !empty_text && !failed {
                    "block"
                } else {
                    "none"
                },
            )?;
            // A successful replacement/hide must not refocus the retiring
            // record. Only a still-visible identical retained record qualifies.
            if point.is_some()
                && self.managed()
                && !empty_text
                && !failed
                && let Some(focus) = &focus
                && let Some(record) = &self.record
                && self
                    .previous
                    .borrow()
                    .as_ref()
                    .is_some_and(|previous| Rc::ptr_eq(previous, record))
                && record.root.contains(Some(focus.as_ref()))
            {
                restore_dom_focus(focus)?;
            }
            Ok(())
        })();
        if result.is_err() {
            self.rollback()?;
        }
        result?;
        *self.frame.borrow_mut() = frame;
        Ok(())
    }
    /// Invoked at the source composition adoption boundary, never user code.
    pub(super) fn adopt(self: &Rc<Self>) {
        let Some(host) = self.host.upgrade().filter(|h| h.alive.get()) else {
            return;
        };
        if self.adopted.replace(true) {
            return;
        }
        if self.reserved.replace(false) {
            host.pending.set(host.pending.get().saturating_sub(1));
        }
        if let Some(record) = &self.record
            && let Some(frame) = record.frame
        {
            let value = self.frame();
            if frame.get_untracked() != value {
                frame.revise(|old| *old = value);
            }
        }
        self.created_shell.set(false);
        if let Some(previous) = host.accepted.replace(Some(self.clone())) {
            host.retired.borrow_mut().push(previous);
        }
        self.previous.borrow_mut().take();
        self.placed.borrow_mut().take();
    }
    pub(super) fn rollback(&self) -> Result<()> {
        if self.adopted.get() {
            return Ok(());
        }
        let Some(p) = self.placed.borrow_mut().take() else {
            self.discard_new_shell();
            return Ok(());
        };
        let Some(host) = self.host.upgrade() else {
            return Ok(());
        };
        if let Some(root) = host.root.borrow().as_ref() {
            restore_style(root, p.host_style.as_deref())?;
        }
        if let Some(record) = &self.record {
            if let Some(parent) = p.parent {
                parent
                    .insert_before(&record.root, p.next.as_ref())
                    .map_err(host_error)?;
            } else {
                record.root.remove();
            }
            restore_style(&record.root, p.root_style.as_deref())?;
        }
        if let Some(old) = self.previous.borrow().as_ref() {
            restore_style(&old.root, p.previous_style.as_deref())?;
        }
        self.discard_new_shell();
        if let Some(focus) = p.focus {
            restore_dom_focus(&focus)?;
        }
        Ok(())
    }
    fn discard_new_shell(&self) {
        if self.created_shell.replace(false)
            && let Some(host) = self.host.upgrade()
            && let Some(root) = host.root.borrow_mut().take()
        {
            root.remove();
        }
    }
}
impl<F: Payload> Drop for PanelStage<F> {
    fn drop(&mut self) {
        let _ = self.rollback();
        if self.reserved.replace(false)
            && let Some(host) = self.host.upgrade()
        {
            host.pending.set(host.pending.get().saturating_sub(1));
        }
    }
}
fn host_error(e: impl std::fmt::Debug) -> ChartError {
    super::host(e)
}
fn resolved_size(node: &HtmlElement) -> Result<[f64; 2]> {
    let window = node
        .owner_document()
        .and_then(|d| d.default_view())
        .ok_or_else(|| ChartError("Tree tooltip window missing".into()))?;
    let style = window
        .get_computed_style(node)
        .map_err(host_error)?
        .ok_or_else(|| ChartError("Tree tooltip computed style missing".into()))?;
    let read = |name| -> Result<f64> {
        let value = style.get_property_value(name).map_err(host_error)?;
        value
            .trim()
            .strip_suffix("px")
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|v| v.is_finite() && *v >= 0.)
            .ok_or_else(|| ChartError("Tree tooltip requires resolved CSS dimensions".into()))
    };
    Ok([read("width")?, read("height")?])
}
fn restore_style(node: &HtmlElement, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => node.set_attribute("style", value),
        None => node.remove_attribute("style"),
    }
    .map_err(host_error)
}
pub(super) struct PanelHost<F: Payload> {
    root: RefCell<Option<HtmlElement>>,
    parent: HtmlElement,
    accepted: RefCell<Option<Rc<PanelStage<F>>>>,
    retired: RefCell<Vec<Rc<PanelStage<F>>>>,
    pending: Cell<usize>,
    alive: Cell<bool>,
    runtime_budget: Option<Rc<RuntimeBudget>>,
}
impl<F: Payload> PanelHost<F> {
    pub(super) fn mount(parent: &HtmlElement) -> Result<Rc<Self>> {
        Ok(Rc::new(Self {
            root: RefCell::new(None),
            parent: parent.clone(),
            accepted: RefCell::new(None),
            retired: RefCell::new(vec![]),
            pending: Cell::new(0),
            alive: Cell::new(true),
            runtime_budget: None,
        }))
    }
    /// Optional shared runtime accounting and latched failure isolation. The
    /// existing Tree mounts keep None and their preceding presentation policy.
    pub(super) fn mount_budgeted(
        parent: &HtmlElement,
        records: Rc<Cell<usize>>,
        limit: usize,
    ) -> Result<Rc<Self>> {
        let mut host = Self::mount(parent)?;
        Rc::get_mut(&mut host)
            .expect("new tooltip host is unique")
            .runtime_budget = Some(Rc::new(RuntimeBudget {
            records,
            limit: Cell::new(limit),
        }));
        Ok(host)
    }
    pub(super) fn set_runtime_limit(&self, limit: usize) {
        if let Some(budget) = &self.runtime_budget {
            budget.limit.set(limit);
        }
    }
    fn root(&self) -> Result<HtmlElement> {
        if let Some(root) = self.root.borrow().as_ref() {
            return Ok(root.clone());
        }
        let document = self
            .parent
            .owner_document()
            .ok_or_else(|| ChartError("Tree tooltip document missing".into()))?;
        let root = element(&document, F::ROOT_CLASS)?;
        root.set_attribute("role", "tooltip").map_err(host_error)?;
        root.set_attribute("aria-live", "polite")
            .map_err(host_error)?;
        root.set_attribute("style","display:none;position:absolute;z-index:1000;pointer-events:none;overflow:auto;white-space:pre-wrap;box-sizing:border-box;background:#20334e;color:white;border-radius:5px").map_err(host_error)?;
        self.parent.append_child(&root).map_err(host_error)?;
        *self.root.borrow_mut() = Some(root.clone());
        Ok(root)
    }

    pub(super) fn prepare(
        self: &Rc<Self>,
        source_generation: u64,
        frame: Option<F>,
        options: BrowserTreeTooltipOptions,
        content: PanelContent<F>,
        appearance: TooltipOptions,
    ) -> Result<Rc<PanelStage<F>>> {
        scene::ensure(self.alive.get(), "Tree tooltip host retired")?;
        options.validate()?;
        scene::ensure(
            self.pending.get() == 0,
            "Tree tooltip pending component budget exceeded",
        )?;
        if let Some(frame) = &frame {
            scene::ensure(
                frame.source_generation() == source_generation,
                "Tree tooltip source generation differs",
            )?;
        }
        if let PanelContent::Text(text) = &content {
            scene::ensure(text.len() <= 65_536, "Tree tooltip text budget exceeded")?;
        }
        let previous = self
            .accepted
            .borrow()
            .as_ref()
            .and_then(|s| s.record.clone());
        let record = match &content {
            PanelContent::Unmanaged => None,
            PanelContent::Text(text) => {
                let doc = self
                    .parent
                    .owner_document()
                    .ok_or_else(|| ChartError("Tree tooltip document missing".into()))?;
                let root = F::text_record(&doc, text, frame.as_ref())?;
                Some(Rc::new(PanelRecord {
                    root,
                    web: None,
                    frame: None,
                    component: None,
                    failed: Cell::new(false),
                    _runtime: None,
                }))
            }
            PanelContent::Component(component) => {
                if let Some(record) = previous.as_ref().filter(|r| {
                    r.component
                        .as_ref()
                        .is_some_and(|old| old.revision == component.revision)
                }) {
                    Some(record.clone())
                } else {
                    Some(self.construct(component.clone())?)
                }
            }
        };
        scene::ensure(
            self.alive.get(),
            "Tree tooltip stopped during component construction",
        )?;
        self.pending.set(self.pending.get() + 1);
        Ok(Rc::new(PanelStage {
            host: Rc::downgrade(self),
            source_generation,
            frame: RefCell::new(frame),
            options,
            content,
            appearance,
            record,
            previous: RefCell::new(previous),
            placed: RefCell::new(None),
            adopted: Cell::new(false),
            reserved: Cell::new(true),
            created_shell: Cell::new(false),
        }))
    }
    fn construct(self: &Rc<Self>, component: Rc<PanelComponent<F>>) -> Result<Rc<PanelRecord<F>>> {
        let runtime = self
            .runtime_budget
            .as_ref()
            .map(RuntimeLease::acquire)
            .transpose()?;
        let document = self
            .parent
            .owner_document()
            .ok_or_else(|| ChartError("Tree tooltip document missing".into()))?;
        let root = element(&document, F::COMPONENT_CLASS)?;
        root.set_attribute(
            "style",
            if self.runtime_budget.is_some() {
                "display:none;position:absolute;visibility:hidden;pointer-events:none"
            } else {
                "position:absolute;visibility:hidden;pointer-events:none"
            },
        )
        .map_err(host_error)?;
        self.parent.append_child(&root).map_err(host_error)?;
        let result = Rc::new(RefCell::new(None));
        let captured = result.clone();
        let revision = component.revision;
        let constructed = component.clone();
        let owner = Rc::downgrade(self);
        let web = glory::web::launch_with(root.clone().into(), move |scope| {
            let frame = scope.cage(None::<F>);
            *captured.borrow_mut() = Some(frame);
            glory::ui::widget(move |scope| {
                let bond = scope.bond(move || frame.get());
                let mut factory = constructed
                    .factory
                    .try_borrow_mut()
                    .map_err(|_| {
                        glory::render::Error::Backend(
                            "Tree tooltip factory is already in use".into(),
                        )
                    })?
                    .take()
                    .ok_or_else(|| {
                        glory::render::Error::Backend(
                            "Tree tooltip factory is already in use".into(),
                        )
                    })?;
                let built = factory(F::context_with_scope(revision, bond, scope));
                *constructed.factory.borrow_mut() = Some(factory);
                let widget = built.map_err(|error| glory::render::Error::Backend(error.0))?;
                if !owner.upgrade().is_some_and(|host| host.alive.get()) {
                    return Err(glory::render::Error::Backend(
                        "Tree tooltip stopped during its factory".into(),
                    )
                    .into());
                }
                widget.show_in(scope)
            })
        });
        let web = match web {
            Ok(web) => web,
            Err(error) => {
                root.remove();
                return Err(host_error(error));
            }
        };
        let frame = result
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("Tree tooltip component did not initialize".into()))?;
        Ok(Rc::new(PanelRecord {
            root,
            web: Some(web),
            frame: Some(frame),
            component: Some(component),
            failed: Cell::new(false),
            _runtime: runtime,
        }))
    }
    pub(super) fn frame(&self) -> Option<F> {
        self.accepted.borrow().as_ref().and_then(|s| s.frame())
    }
    pub(super) fn pump(&self) -> Vec<ChartError> {
        let record = self
            .accepted
            .borrow()
            .as_ref()
            .and_then(|s| s.record.clone());
        let mut errors = vec![];
        if let Some(record) = record
            && !(self.runtime_budget.is_some() && record.failed.get())
            && let Some(web) = &record.web
            && let Err(error) = web.app().pump()
        {
            errors.push(host_error(error));
            // The typed payload remains accepted. A failed dynamic view must
            // not leave an old-source popup visible as if it had updated.
            if let Some(root) = self.root.borrow().as_ref()
                && let Err(error) = css(root, "display", "none")
            {
                errors.push(error);
            }
            if self.runtime_budget.is_some() {
                record.failed.set(true);
                let isolated = (|| {
                    css(&record.root, "display", "none")?;
                    css(&record.root, "visibility", "hidden")?;
                    css(&record.root, "opacity", "0")?;
                    record.root.set_attribute("inert", "").map_err(host_error)
                })();
                if let Err(error) = isolated {
                    errors.push(error);
                    record.root.remove();
                }
            }
        }
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        drop(retired);
        errors
    }
    pub(super) fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        let accepted = self.accepted.borrow_mut().take();
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        drop(accepted);
        drop(retired);
        if let Some(root) = self.root.borrow_mut().take() {
            root.remove();
        }
    }
}
