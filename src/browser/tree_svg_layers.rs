//! Literal SVG occurrence views and retained, plot-local Glory function layers.
//!
//! Custom subtrees live in an XHTML host inside a real SVG foreignObject.
//! They are presentation, not fabricated source marks or interaction sites.
use glory::{Bond, Widget};
use web_sys::js_sys;

use super::*;

const SVG_NS: &str = "http://www.w3.org/2000/svg";
const XHTML_NS: &str = "http://www.w3.org/1999/xhtml";

/// Complete accepted input for any literal layer occurrence, including a
/// builtin. Custom subtrees also receive it through their retained Bond;
/// `initial` in their constructor is tentative and that Bond starts at None.
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserTreeSvgLayerFrame {
    pub data: TreeSvgFrame,
    pub source_generation: u64,
    pub publication: u64,
    pub key: String,
    pub index: usize,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub interactive: bool,
}

#[derive(Clone)]
pub struct BrowserTreeSvgLayerContext {
    pub key: String,
    pub revision: u64,
    pub initial: BrowserTreeSvgLayerFrame,
    pub frame: Bond<Option<BrowserTreeSvgLayerFrame>>,
    pub current: BrowserTreeSvgCurrentNode,
}

/// A retained whole-layer component may opt into the owner's sampled motion.
/// Both Bonds derive from one adopted snapshot; `initial` remains tentative.
#[derive(Clone)]
pub struct BrowserTreeSvgAnimatedLayerContext {
    pub key: String,
    pub revision: u64,
    pub initial: BrowserTreeSvgLayerFrame,
    pub frame: Bond<Option<BrowserTreeSvgLayerFrame>>,
    pub animated_props: Bond<Option<TreeMotionFrame>>,
    pub current: BrowserTreeSvgCurrentNode,
}

type StaticFactory = dyn FnMut(BrowserTreeSvgLayerContext) -> Result<Box<dyn Widget>>;
type AnimatedFactory = dyn FnMut(BrowserTreeSvgAnimatedLayerContext) -> Result<Box<dyn Widget>>;
enum Factory {
    Static(Box<StaticFactory>),
    Animated(Box<AnimatedFactory>),
}
struct Component {
    revision: u64,
    animated: bool,
    factory: RefCell<Option<Factory>>,
}

/// An additive SVG-only program. Repeated registry names use independent
/// occurrence keys; same key/registry/revision retains the existing subtree.
#[derive(Clone)]
pub struct BrowserTreeSvgProgram {
    layers: Vec<TreeSvgLayer>,
    custom: BTreeMap<String, Rc<Component>>,
    budget: TreeSvgBudget,
    extras: TreeSvgExtrasPolicy,
    interactive: bool,
    debug_mesh: bool,
}
impl BrowserTreeSvgProgram {
    pub fn new(layers: Vec<TreeSvgLayer>) -> Result<Self> {
        let program = Self {
            layers,
            custom: BTreeMap::new(),
            budget: TreeSvgBudget::default(),
            extras: TreeSvgExtrasPolicy::RequireLayer,
            interactive: true,
            debug_mesh: false,
        };
        program.validate_specs()?;
        Ok(program)
    }
    pub fn defaults() -> Self {
        Self {
            layers: TreeSvgPlan::default_layers(),
            custom: BTreeMap::new(),
            budget: TreeSvgBudget::default(),
            extras: TreeSvgExtrasPolicy::RequireLayer,
            interactive: true,
            debug_mesh: false,
        }
    }
    pub fn custom<W: Widget + 'static>(
        mut self,
        key: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(BrowserTreeSvgLayerContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        let key = key.into();
        valid_key(&key)?;
        scene::ensure(
            !self.custom.contains_key(&key),
            "duplicate Tree SVG custom registry",
        )?;
        self.custom.insert(
            key,
            Rc::new(Component {
                revision,
                animated: false,
                factory: RefCell::new(Some(Factory::Static(Box::new(move |context| {
                    Ok(Box::new(factory(context)?))
                })))),
            }),
        );
        Ok(self)
    }
    pub fn custom_animated<W: Widget + 'static>(
        mut self,
        key: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(BrowserTreeSvgAnimatedLayerContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        let key = key.into();
        valid_key(&key)?;
        scene::ensure(
            !self.custom.contains_key(&key),
            "duplicate Tree SVG custom registry",
        )?;
        self.custom.insert(
            key,
            Rc::new(Component {
                revision,
                animated: true,
                factory: RefCell::new(Some(Factory::Animated(Box::new(move |context| {
                    Ok(Box::new(factory(context)?))
                })))),
            }),
        );
        Ok(self)
    }
    pub(crate) fn requires_motion(&self) -> bool {
        self.custom.values().any(|component| component.animated)
    }
    pub fn with_budget(mut self, budget: TreeSvgBudget) -> Result<Self> {
        budget.validate()?;
        self.budget = budget;
        self.validate_specs()?;
        Ok(self)
    }
    pub fn with_extras(mut self, extras: TreeSvgExtrasPolicy) -> Self {
        self.extras = extras;
        self
    }
    pub fn interactive(mut self, enabled: bool) -> Self {
        self.interactive = enabled;
        self
    }
    pub fn debug_mesh(mut self, enabled: bool) -> Self {
        self.debug_mesh = enabled;
        self
    }
    pub fn layers(&self) -> &[TreeSvgLayer] {
        &self.layers
    }
    pub fn budget(&self) -> TreeSvgBudget {
        self.budget
    }
    pub fn extras(&self) -> TreeSvgExtrasPolicy {
        self.extras
    }
    pub fn is_interactive(&self) -> bool {
        self.interactive
    }
    pub fn is_debug_mesh(&self) -> bool {
        self.debug_mesh
    }
    pub(crate) fn same_program(&self, other: &Self) -> bool {
        self.layers == other.layers
            && self.budget == other.budget
            && self.extras == other.extras
            && self.interactive == other.interactive
            && self.debug_mesh == other.debug_mesh
            && self.custom.len() == other.custom.len()
            && self.custom.iter().all(|(key, component)| {
                other
                    .custom
                    .get(key)
                    .is_some_and(|other| Rc::ptr_eq(component, other))
            })
    }
    pub(super) fn plan(&self, frame: &TreeSvgFrame) -> Result<TreeSvgPlan> {
        self.validate_for_build()?;
        TreeSvgPlan::new(frame, self.layers.clone(), self.budget, self.extras)
    }
    /// Complete frame-independent admission, before any source function or
    /// retained component constructor is invoked.
    pub(super) fn validate_for_build(&self) -> Result<()> {
        self.validate_specs()?;
        for layer in &self.layers {
            if let TreeSvgLayerKind::Custom(name) = &layer.kind {
                scene::ensure(
                    self.custom.contains_key(name),
                    "Tree SVG custom registry is missing",
                )?;
            }
        }
        Ok(())
    }
    fn validate_specs(&self) -> Result<()> {
        TreeSvgPlan::validate_layers(&self.layers, self.budget)?;
        let custom = self
            .layers
            .iter()
            .filter(|layer| matches!(layer.kind, TreeSvgLayerKind::Custom(_)))
            .count();
        scene::ensure(
            custom <= self.budget.components,
            "Tree SVG component budget exceeded",
        )?;
        let minimum_work = self
            .layers
            .iter()
            .filter(|layer| {
                matches!(
                    layer.kind,
                    TreeSvgLayerKind::Custom(_) | TreeSvgLayerKind::Mesh
                )
            })
            .count();
        scene::ensure(
            minimum_work <= self.budget.work,
            "Tree SVG source work budget exceeded",
        )
    }
}
impl Default for BrowserTreeSvgProgram {
    fn default() -> Self {
        Self::defaults()
    }
}
fn valid_key(key: &str) -> Result<()> {
    TreeSvgLayer::new("registry-check", TreeSvgLayerKind::Custom(key.into())).map(|_| ())
}

/// Engine-prepared complete builtin occurrence, including any interleaved
/// source components. Its whole native carrier is movable; Glory children are
/// never reparented behind their platform's placement accounting.
pub(super) struct BuiltinOccurrence {
    pub key: String,
    pub element: Element,
}

pub(super) type CurrentFactory = Rc<dyn Fn(&str, Weak<Cell<bool>>) -> BrowserTreeSvgCurrentNode>;

struct Record {
    key: String,
    registry: String,
    component: Rc<Component>,
    carrier: Element,
    clip: HtmlElement,
    root: HtmlElement,
    web: Rc<WebApp>,
    frame: Cage<Option<LayerSnapshot>>,
    live: Rc<Cell<bool>>,
    budget: Rc<Cell<usize>>,
    retired: Cell<bool>,
    failed: Cell<bool>,
    updated: Cell<bool>,
}
#[derive(Clone, PartialEq)]
struct LayerSnapshot {
    source: BrowserTreeSvgLayerFrame,
    motion: Option<TreeMotionFrame>,
}
impl Record {
    fn retire(&self) -> Option<ChartError> {
        if self.retired.replace(true) {
            return None;
        }
        self.live.set(false);
        self.budget.set(self.budget.get().saturating_sub(1));
        let error = self.web.app().clear().err().map(host);
        self.carrier.remove();
        error
    }
}
impl Drop for Record {
    fn drop(&mut self) {
        let _ = self.retire();
    }
}
struct Shell(Element);
impl Drop for Shell {
    fn drop(&mut self) {
        self.0.remove();
    }
}

struct Move {
    node: Element,
    parent: Option<web_sys::Node>,
    next: Option<web_sys::Node>,
}
struct Placement {
    previous_style: Option<String>,
    shell_style: Option<String>,
    moves: Vec<Move>,
    attributes: Vec<(Element, String, Option<String>)>,
    focus: Option<HtmlElement>,
    shell_state: Option<String>,
}

struct Lease {
    host: Weak<Host>,
    adopted: Cell<bool>,
    pending: Cell<bool>,
}
impl Drop for Lease {
    fn drop(&mut self) {
        if self.pending.replace(false)
            && let Some(host) = self.host.upgrade()
        {
            host.pending.set(false);
        }
    }
}

/// All fallible DOM operations precede adoption. `finish` is the only normal
/// path that pumps accepted subtrees or drops retired scopes.
pub(super) struct Stage {
    host: Weak<Host>,
    plan: TreeSvgPlan,
    program: BrowserTreeSvgProgram,
    source_generation: u64,
    publication: u64,
    shell: Rc<Shell>,
    records: BTreeMap<String, Rc<Record>>,
    frames: BTreeMap<String, BrowserTreeSvgLayerFrame>,
    sampled_scene: Scene,
    motion: Option<TreeMotionFrame>,
    carriers: Vec<(Element, Element)>,
    view: ZoomTransform,
    paint_view: ZoomTransform,
    hidden: BTreeSet<String>,
    previous: RefCell<Option<Rc<Stage>>>,
    placed: RefCell<Option<Placement>>,
    lease: Rc<Lease>,
}

pub(super) struct Host {
    content: Element,
    alive: Cell<bool>,
    accepted: RefCell<Option<Rc<Stage>>>,
    retired: RefCell<Vec<Rc<Stage>>>,
    shells: RefCell<Vec<Weak<Shell>>>,
    records: Rc<Cell<usize>>,
    all_records: RefCell<Vec<Weak<Record>>>,
    pending: Cell<bool>,
}

impl Host {
    pub(super) fn new(content: Element) -> Result<Rc<Self>> {
        scene::ensure(
            content.owner_document().is_some(),
            "Tree SVG host has no document",
        )?;
        Ok(Rc::new(Self {
            content,
            alive: Cell::new(true),
            accepted: RefCell::new(None),
            retired: RefCell::new(Vec::new()),
            shells: RefCell::new(Vec::new()),
            records: Rc::new(Cell::new(0)),
            all_records: RefCell::new(Vec::new()),
            pending: Cell::new(false),
        }))
    }

    /// Must be called before either source-component or custom factories. The
    /// external count includes all actually live/pending source records.
    pub(super) fn preflight(
        &self,
        plan: &TreeSvgPlan,
        program: &BrowserTreeSvgProgram,
        source_live_pending: usize,
    ) -> Result<()> {
        scene::ensure(self.alive.get(), "Tree SVG host retired")?;
        scene::ensure(!self.pending.get(), "Tree SVG candidate already pending")?;
        scene::ensure(
            program.plan(plan.frame())? == *plan,
            "Tree SVG program differs from plan",
        )?;
        let previous = self.accepted.borrow();
        let mut needed = 0usize;
        for occurrence in plan.layers() {
            if let TreeSvgLayerKind::Custom(name) = &occurrence.layer.kind {
                let component = &program.custom[name];
                let retained = previous
                    .as_ref()
                    .and_then(|s| s.records.get(&occurrence.layer.key))
                    .is_some_and(|record| {
                        record.registry == *name && record.component.revision == component.revision
                    });
                needed += usize::from(!retained);
            }
        }
        let total = self
            .records
            .get()
            .checked_add(needed)
            .and_then(|n| n.checked_add(source_live_pending))
            .ok_or_else(|| ChartError("Tree SVG component count overflow".into()))?;
        scene::ensure(
            total <= plan.budget().components,
            "Tree SVG live/pending component budget exceeded",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare(
        self: &Rc<Self>,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        model: &SurfaceModel,
        source_generation: u64,
        publication: u64,
        current_factory: CurrentFactory,
        source: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
    ) -> Result<Rc<Stage>> {
        self.prepare_inner(
            plan,
            program,
            model,
            source_generation,
            publication,
            current_factory,
            source,
            source_live_pending,
            None,
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_motion(
        self: &Rc<Self>,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        model: &SurfaceModel,
        source_generation: u64,
        publication: u64,
        current_factory: CurrentFactory,
        source: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
        motion: TreeMotionFrame,
    ) -> Result<Rc<Stage>> {
        self.prepare_inner(
            plan,
            program,
            model,
            source_generation,
            publication,
            current_factory,
            source,
            source_live_pending,
            Some(motion),
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn prepare_inner(
        self: &Rc<Self>,
        plan: TreeSvgPlan,
        program: BrowserTreeSvgProgram,
        model: &SurfaceModel,
        source_generation: u64,
        publication: u64,
        current_factory: CurrentFactory,
        source: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
        motion: Option<TreeMotionFrame>,
    ) -> Result<Rc<Stage>> {
        self.preflight(&plan, &program, source_live_pending)?;
        scene::ensure(
            plan.frame().layout().scene
                == *motion
                    .as_ref()
                    .map_or(model.scene(), |frame| frame.target().scene()),
            "Tree SVG source Scene differs from model",
        )?;
        scene::ensure(
            !program.requires_motion() || motion.is_some(),
            "animated SVG layer requires an animated Tree mount",
        )?;
        let builtin_keys: BTreeSet<_> = plan
            .layers()
            .iter()
            .filter(|o| !matches!(o.layer.kind, TreeSvgLayerKind::Custom(_)))
            .map(|o| o.layer.key.as_str())
            .collect();
        let mut source_map = BTreeMap::new();
        for occurrence in source {
            scene::ensure(
                builtin_keys.contains(occurrence.key.as_str()),
                "Tree SVG builtin carrier is orphaned",
            )?;
            scene::ensure(
                !source_map
                    .values()
                    .any(|node: &Element| node.is_same_node(Some(occurrence.element.as_ref()))),
                "Tree SVG builtin carriers alias",
            )?;
            scene::ensure(
                source_map
                    .insert(occurrence.key, occurrence.element)
                    .is_none(),
                "duplicate Tree SVG builtin carrier",
            )?;
        }
        scene::ensure(
            source_map.len() == builtin_keys.len(),
            "Tree SVG builtin carrier is missing",
        )?;
        let document = self
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Tree SVG document missing".into()))?;
        let root = svg_node(&document, "svg")?;
        root.set_attribute("id", &svg::owner_namespace(&document)?)
            .map_err(host)?;
        root.set_attribute("class", "kartis-tree-svg-layers")
            .map_err(host)?;
        root.set_attribute("data-tree-svg-active", "false")
            .map_err(host)?;
        root.set_attribute("data-tree-svg-state", "prepared")
            .map_err(host)?;
        root.set_attribute("width", "100%").map_err(host)?;
        root.set_attribute("height", "100%").map_err(host)?;
        root.set_attribute("preserveAspectRatio", "none")
            .map_err(host)?;
        root.set_attribute(
            "viewBox",
            &format!("0 0 {} {}", model.scene().width, model.scene().height),
        )
        .map_err(host)?;
        root.set_attribute("role", "presentation").map_err(host)?;
        root.set_attribute("style", "position:absolute;left:0;top:0;width:100%;height:100%;display:block;visibility:hidden;opacity:0;pointer-events:none;overflow:hidden").map_err(host)?;
        let background = svg_node(&document, "rect")?;
        for (name, value) in [
            ("width", "100%"),
            ("height", "100%"),
            ("pointer-events", "none"),
        ] {
            background.set_attribute(name, value).map_err(host)?;
        }
        background
            .set_attribute("fill", &render::css_color(model.scene().background))
            .map_err(host)?;
        root.append_child(&background).map_err(host)?;
        let shell = Rc::new(Shell(root));
        self.shells.borrow_mut().retain(|s| s.strong_count() > 0);
        self.shells.borrow_mut().push(Rc::downgrade(&shell));
        self.content.append_child(&shell.0).map_err(host)?;
        self.pending.set(true);
        let lease = Rc::new(Lease {
            host: Rc::downgrade(self),
            adopted: Cell::new(false),
            pending: Cell::new(true),
        });
        let previous = self.accepted.borrow().clone();
        let view = model.view();
        let paint_view = model.paint_transform();
        let hidden = model.hidden().clone();
        let result = (|| {
            let mut records = BTreeMap::new();
            let mut frames = BTreeMap::new();
            let mut carriers = Vec::new();
            for occurrence in plan.layers() {
                let group = svg_node(&document, "g")?;
                group
                    .set_attribute("class", "kartis-tree-svg-occurrence")
                    .map_err(host)?;
                group
                    .set_attribute("data-tree-svg-key", &occurrence.layer.key)
                    .map_err(host)?;
                group
                    .set_attribute("data-tree-svg-index", &occurrence.index.to_string())
                    .map_err(host)?;
                group
                    .set_attribute("data-tree-svg-kind", kind_name(&occurrence.layer.kind))
                    .map_err(host)?;
                shell.0.append_child(&group).map_err(host)?;
                let frame = BrowserTreeSvgLayerFrame {
                    data: plan.frame().clone(),
                    source_generation,
                    publication,
                    key: occurrence.layer.key.clone(),
                    index: occurrence.index,
                    view,
                    hidden: hidden.clone(),
                    interactive: program.interactive,
                };
                if let TreeSvgLayerKind::Custom(name) = &occurrence.layer.kind {
                    let component = program.custom[name].clone();
                    let retained = previous
                        .as_ref()
                        .and_then(|s| s.records.get(&occurrence.layer.key))
                        .filter(|r| {
                            r.registry == *name && r.component.revision == component.revision
                        })
                        .cloned();
                    let record = match retained {
                        Some(record) => record,
                        None => self.construct(
                            &group,
                            name,
                            component,
                            frame.clone(),
                            &current_factory,
                        )?,
                    };
                    scene::ensure(
                        self.alive.get(),
                        "Tree SVG stopped during layer construction",
                    )?;
                    carriers.push((group, record.carrier.clone()));
                    records.insert(occurrence.layer.key.clone(), record);
                } else {
                    carriers.push((
                        group,
                        source_map
                            .remove(&occurrence.layer.key)
                            .expect("preflight builtin"),
                    ));
                }
                frames.insert(occurrence.layer.key.clone(), frame);
            }
            Ok(Rc::new(Stage {
                host: Rc::downgrade(self),
                plan,
                program,
                source_generation,
                publication,
                shell,
                records,
                frames,
                sampled_scene: model.scene().clone(),
                motion,
                carriers,
                view,
                paint_view,
                hidden,
                previous: RefCell::new(previous),
                placed: RefCell::new(None),
                lease,
            }))
        })();
        if result.is_err() {
            self.pending.set(false);
        }
        result
    }

    fn construct(
        self: &Rc<Self>,
        group: &Element,
        registry: &str,
        component: Rc<Component>,
        initial: BrowserTreeSvgLayerFrame,
        current_factory: &CurrentFactory,
    ) -> Result<Rc<Record>> {
        let document = self
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Tree SVG document missing".into()))?;
        let carrier = svg_node(&document, "foreignObject")?;
        carrier
            .set_attribute("class", "kartis-tree-svg-custom")
            .map_err(host)?;
        carrier
            .set_attribute("data-tree-svg-key", &initial.key)
            .map_err(host)?;
        carrier
            .set_attribute("data-tree-svg-registry", registry)
            .map_err(host)?;
        carrier
            .set_attribute("style", "overflow:hidden;pointer-events:none")
            .map_err(host)?;
        let viewport = document
            .create_element_ns(Some(XHTML_NS), "div")
            .map_err(host)?;
        viewport
            .set_attribute("class", "kartis-tree-svg-custom-viewport")
            .map_err(host)?;
        viewport.set_attribute("style", "position:relative;width:100%;height:100%;pointer-events:none;isolation:isolate;overflow:hidden").map_err(host)?;
        let clip: HtmlElement = document
            .create_element_ns(Some(XHTML_NS), "div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        clip.set_attribute("class", "kartis-tree-svg-custom-clip")
            .map_err(host)?;
        clip.set_attribute(
            "style",
            "position:absolute;overflow:hidden;pointer-events:none",
        )
        .map_err(host)?;
        let root: HtmlElement = document
            .create_element_ns(Some(XHTML_NS), "div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        root.set_attribute("class", "kartis-tree-svg-custom-host")
            .map_err(host)?;
        root.set_attribute("xmlns", XHTML_NS).map_err(host)?;
        root.set_attribute("inert", "").map_err(host)?;
        root.set_attribute("data-tree-svg-key", &initial.key)
            .map_err(host)?;
        root.set_attribute("style","position:absolute;left:0;top:0;display:block;pointer-events:none;overflow:hidden;box-sizing:border-box;transform-origin:0 0").map_err(host)?;
        place_custom(
            &carrier,
            &clip,
            &root,
            &initial.data.layout().scene,
            initial.data.plot(),
            initial.view,
        )?;
        let font = initial
            .data
            .layout()
            .scene
            .typography
            .default
            .resolve(&ResolvedFont::default());
        root.style()
            .set_property("font-family", &font.css_family())
            .map_err(host)?;
        root.style()
            .set_property(
                "font-size",
                &format!("{}px", initial.data.theme().font_size),
            )
            .map_err(host)?;
        root.style()
            .set_property("font-weight", &font.weight.to_string())
            .map_err(host)?;
        root.style()
            .set_property("color", &render::css_color(initial.data.theme().text))
            .map_err(host)?;
        clip.append_child(&root).map_err(host)?;
        viewport.append_child(&clip).map_err(host)?;
        carrier.append_child(&viewport).map_err(host)?;
        group.append_child(&carrier).map_err(host)?;
        let live = Rc::new(Cell::new(false));
        let current = current_factory(&initial.key, Rc::downgrade(&live));
        let result = Rc::new(RefCell::new(None));
        let captured = result.clone();
        let owner = Rc::downgrade(self);
        let constructed = component.clone();
        let key = initial.key.clone();
        let record_key = key.clone();
        let revision = component.revision;
        let web = glory::web::launch_with(root.clone().into(), move |scope| {
            let frame = scope.cage(None::<LayerSnapshot>);
            *captured.borrow_mut() = Some(frame);
            glory::ui::widget(move |scope| {
                let bond = scope.bond(move || {
                    frame.with(|value| value.as_ref().map(|value| value.source.clone()))
                });
                let animated_props = scope.bond(move || {
                    frame.with(|value| value.as_ref().and_then(|value| value.motion.clone()))
                });
                let mut factory = constructed
                    .factory
                    .try_borrow_mut()
                    .map_err(|_| {
                        glory::render::Error::Backend("Tree SVG factory is already in use".into())
                    })?
                    .take()
                    .ok_or_else(|| {
                        glory::render::Error::Backend("Tree SVG factory is already in use".into())
                    })?;
                let built = match &mut factory {
                    Factory::Static(factory) => factory(BrowserTreeSvgLayerContext {
                        key: key.clone(),
                        revision,
                        initial: initial.clone(),
                        frame: bond,
                        current: current.clone(),
                    }),
                    Factory::Animated(factory) => factory(BrowserTreeSvgAnimatedLayerContext {
                        key: key.clone(),
                        revision,
                        initial: initial.clone(),
                        frame: bond,
                        animated_props,
                        current: current.clone(),
                    }),
                };
                *constructed.factory.borrow_mut() = Some(factory);
                let widget = built.map_err(|error| glory::render::Error::Backend(error.0))?;
                if !owner.upgrade().is_some_and(|h| h.alive.get()) {
                    return Err(glory::render::Error::Backend(
                        "Tree SVG stopped during its factory".into(),
                    )
                    .into());
                }
                widget.show_in(scope)
            })
        });
        let web = match web {
            Ok(web) => web,
            Err(error) => {
                carrier.remove();
                return Err(host(error));
            }
        };
        let frame = result
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("Tree SVG component did not initialize".into()))?;
        self.records.set(self.records.get() + 1);
        let record = Rc::new(Record {
            key: record_key,
            registry: registry.into(),
            component,
            carrier,
            clip,
            root,
            web: Rc::new(web),
            frame,
            live,
            budget: self.records.clone(),
            retired: Cell::new(false),
            failed: Cell::new(false),
            updated: Cell::new(true),
        });
        self.all_records
            .borrow_mut()
            .retain(|record| record.upgrade().is_some_and(|r| !r.retired.get()));
        self.all_records.borrow_mut().push(Rc::downgrade(&record));
        Ok(record)
    }

    pub(super) fn current_frame(&self, key: &str) -> Option<BrowserTreeSvgLayerFrame> {
        self.accepted
            .borrow()
            .as_ref()
            .and_then(|stage| stage.frames.get(key).cloned())
    }
    pub(super) fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        self.pending.set(false);
        let accepted = self.accepted.borrow_mut().take();
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        let records = std::mem::take(&mut *self.all_records.borrow_mut());
        for record in records.into_iter().filter_map(|r| r.upgrade()) {
            let _ = record.retire();
        }
        if let Some(stage) = &accepted {
            for record in stage.records.values() {
                let _ = record.retire();
            }
        }
        for stage in &retired {
            for record in stage.records.values() {
                let _ = record.retire();
            }
        }
        let shells = std::mem::take(&mut *self.shells.borrow_mut());
        for shell in shells.into_iter().filter_map(|s| s.upgrade()) {
            shell.0.remove();
        }
        drop(accepted);
        drop(retired);
    }
}

fn kind_name(kind: &TreeSvgLayerKind) -> &'static str {
    match kind {
        TreeSvgLayerKind::Links => "links",
        TreeSvgLayerKind::Nodes => "nodes",
        TreeSvgLayerKind::Labels => "labels",
        TreeSvgLayerKind::Mesh => "mesh",
        TreeSvgLayerKind::SceneExtras => "scene-extras",
        TreeSvgLayerKind::Custom(_) => "custom",
    }
}
fn svg_node(document: &web_sys::Document, name: &str) -> Result<Element> {
    document.create_element_ns(Some(SVG_NS), name).map_err(host)
}
fn place_custom(
    carrier: &Element,
    clip: &HtmlElement,
    root: &HtmlElement,
    scene: &Scene,
    plot: ClipRect,
    view: ZoomTransform,
) -> Result<()> {
    for (name, value) in [
        ("x", 0.),
        ("y", 0.),
        ("width", scene.width),
        ("height", scene.height),
    ] {
        carrier
            .set_attribute(name, &value.to_string())
            .map_err(host)?;
    }
    for (name, value) in [
        ("left", plot.x),
        ("top", plot.y),
        ("width", plot.width),
        ("height", plot.height),
    ] {
        clip.style()
            .set_property(name, &format!("{value}px"))
            .map_err(host)?;
    }
    root.style()
        .set_property("width", &format!("{}px", plot.width))
        .map_err(host)?;
    root.style()
        .set_property("height", &format!("{}px", plot.height))
        .map_err(host)?;
    // The fixed clip contributes the plot origin. Raw local view translation
    // then maps plot-local application coordinates exactly once; paint_view
    // already contains a margin correction and must not be used here.
    root.style()
        .set_property(
            "transform",
            &format!(
                "matrix({},0,0,{},{},{})",
                view.scale, view.scale, view.translate_x, view.translate_y
            ),
        )
        .map_err(host)?;
    Ok(())
}

impl Stage {
    pub(super) fn publication(&self) -> u64 {
        self.publication
    }
    pub(super) fn view(&self) -> ZoomTransform {
        self.view
    }
    pub(super) fn hidden(&self) -> &BTreeSet<String> {
        &self.hidden
    }
    pub(super) fn was_adopted(&self) -> bool {
        self.lease.adopted.get()
    }
    pub(super) fn root(&self) -> Element {
        self.shell.0.clone()
    }
    pub(super) fn validate_source(&self, model: &SurfaceModel) -> Result<()> {
        let owner = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG host retired".into()))?;
        scene::ensure(owner.alive.get(), "Tree SVG host retired")?;
        scene::ensure(
            self.sampled_scene == *model.scene()
                && self.hidden == *model.hidden()
                && self.view == model.view()
                && self.paint_view == model.paint_transform(),
            "Tree SVG stage differs from source/view/hidden",
        )
    }
    pub(super) fn validate(&self) -> Result<()> {
        let owner = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG host retired".into()))?;
        scene::ensure(
            owner.alive.get() && !self.was_adopted(),
            "Tree SVG stage retired or adopted",
        )?;
        scene::ensure(
            self.placed.borrow().is_some(),
            "Tree SVG placement is incomplete",
        )?;
        scene::ensure(
            self.carriers.iter().all(|(parent, node)| {
                node.parent_node()
                    .is_some_and(|actual| actual.is_same_node(Some(parent.as_ref())))
            }),
            "Tree SVG carrier placement differs",
        )
    }
    /// Reproject a queued or accepted candidate without replaying factories.
    /// A queued candidate shares its adoption lease with every retarget.
    pub(super) fn retarget(
        self: &Rc<Self>,
        model: &SurfaceModel,
        builtins: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
        publication: u64,
    ) -> Result<Rc<Self>> {
        self.retarget_inner(
            model,
            builtins,
            source_live_pending,
            publication,
            self.motion.clone(),
        )
    }
    pub(super) fn retarget_motion(
        self: &Rc<Self>,
        model: &SurfaceModel,
        builtins: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
        publication: u64,
        motion: TreeMotionFrame,
    ) -> Result<Rc<Self>> {
        self.retarget_inner(
            model,
            builtins,
            source_live_pending,
            publication,
            Some(motion),
        )
    }
    fn retarget_inner(
        self: &Rc<Self>,
        model: &SurfaceModel,
        builtins: Vec<BuiltinOccurrence>,
        source_live_pending: usize,
        publication: u64,
        motion: Option<TreeMotionFrame>,
    ) -> Result<Rc<Self>> {
        let owner = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG host retired".into()))?;
        scene::ensure(owner.alive.get(), "Tree SVG host retired")?;
        scene::ensure(
            self.plan.frame().layout().scene
                == *motion
                    .as_ref()
                    .map_or(model.scene(), |frame| frame.target().scene()),
            "Tree SVG retarget changed source",
        )?;
        let total = owner
            .records
            .get()
            .checked_add(source_live_pending)
            .ok_or_else(|| ChartError("Tree SVG component count overflow".into()))?;
        scene::ensure(
            total <= self.plan.budget().components,
            "Tree SVG live/pending component budget exceeded",
        )?;
        let mut source = BTreeMap::new();
        for builtin in builtins {
            scene::ensure(
                !source
                    .values()
                    .any(|node: &Element| node.is_same_node(Some(builtin.element.as_ref()))),
                "Tree SVG builtin carriers alias",
            )?;
            scene::ensure(
                source.insert(builtin.key, builtin.element).is_none(),
                "duplicate Tree SVG builtin carrier",
            )?;
        }
        let wanted: BTreeSet<_> = self
            .plan
            .layers()
            .iter()
            .filter(|o| !matches!(o.layer.kind, TreeSvgLayerKind::Custom(_)))
            .map(|o| o.layer.key.as_str())
            .collect();
        scene::ensure(
            source.keys().map(String::as_str).collect::<BTreeSet<_>>() == wanted,
            "Tree SVG retarget builtin carriers differ",
        )?;
        let document = owner
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Tree SVG document missing".into()))?;
        let root = svg_node(&document, "svg")?;
        root.set_attribute("id", &svg::owner_namespace(&document)?)
            .map_err(host)?;
        root.set_attribute("class", "kartis-tree-svg-layers")
            .map_err(host)?;
        root.set_attribute("data-tree-svg-active", "false")
            .map_err(host)?;
        root.set_attribute("data-tree-svg-state", "prepared")
            .map_err(host)?;
        root.set_attribute("width", "100%").map_err(host)?;
        root.set_attribute("height", "100%").map_err(host)?;
        root.set_attribute("preserveAspectRatio", "none")
            .map_err(host)?;
        root.set_attribute(
            "viewBox",
            &format!("0 0 {} {}", model.scene().width, model.scene().height),
        )
        .map_err(host)?;
        root.set_attribute("role", "presentation").map_err(host)?;
        root.set_attribute("style","position:absolute;left:0;top:0;width:100%;height:100%;display:block;visibility:hidden;opacity:0;pointer-events:none;overflow:hidden").map_err(host)?;
        let background = svg_node(&document, "rect")?;
        for (name, value) in [
            ("width", "100%"),
            ("height", "100%"),
            ("pointer-events", "none"),
        ] {
            background.set_attribute(name, value).map_err(host)?;
        }
        background
            .set_attribute("fill", &render::css_color(model.scene().background))
            .map_err(host)?;
        root.append_child(&background).map_err(host)?;
        let shell = Rc::new(Shell(root));
        owner.shells.borrow_mut().retain(|s| s.strong_count() > 0);
        owner.shells.borrow_mut().push(Rc::downgrade(&shell));
        owner.content.append_child(&shell.0).map_err(host)?;
        let lease = if self.was_adopted() {
            scene::ensure(
                owner
                    .accepted
                    .borrow()
                    .as_ref()
                    .is_some_and(|s| Rc::ptr_eq(s, self)),
                "Tree SVG retarget is stale",
            )?;
            scene::ensure(!owner.pending.get(), "Tree SVG candidate already pending")?;
            owner.pending.set(true);
            Rc::new(Lease {
                host: Rc::downgrade(&owner),
                adopted: Cell::new(false),
                pending: Cell::new(true),
            })
        } else {
            self.lease.clone()
        };
        let view = model.view();
        let paint_view = model.paint_transform();
        let hidden = model.hidden().clone();
        let mut carriers = Vec::new();
        for occurrence in self.plan.layers() {
            let group = svg_node(&document, "g")?;
            group
                .set_attribute("class", "kartis-tree-svg-occurrence")
                .map_err(host)?;
            group
                .set_attribute("data-tree-svg-key", &occurrence.layer.key)
                .map_err(host)?;
            group
                .set_attribute("data-tree-svg-index", &occurrence.index.to_string())
                .map_err(host)?;
            group
                .set_attribute("data-tree-svg-kind", kind_name(&occurrence.layer.kind))
                .map_err(host)?;
            shell.0.append_child(&group).map_err(host)?;
            let carrier = if matches!(occurrence.layer.kind, TreeSvgLayerKind::Custom(_)) {
                self.records[&occurrence.layer.key].carrier.clone()
            } else {
                source
                    .remove(&occurrence.layer.key)
                    .expect("validated retarget carrier")
            };
            carriers.push((group, carrier));
        }
        let frames = self
            .frames
            .iter()
            .map(|(key, value)| {
                let mut value = value.clone();
                value.view = view;
                value.hidden = hidden.clone();
                value.publication = publication;
                (key.clone(), value)
            })
            .collect();
        Ok(Rc::new(Self {
            host: Rc::downgrade(&owner),
            plan: self.plan.clone(),
            program: self.program.clone(),
            source_generation: self.source_generation,
            publication,
            shell,
            records: self.records.clone(),
            frames,
            sampled_scene: model.scene().clone(),
            motion,
            carriers,
            view,
            paint_view,
            hidden,
            previous: RefCell::new(owner.accepted.borrow().clone()),
            placed: RefCell::new(None),
            lease,
        }))
    }
    pub(super) fn place(&self) -> Result<()> {
        let owner = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Tree SVG host retired".into()))?;
        scene::ensure(
            owner.alive.get() && !self.was_adopted(),
            "Tree SVG stage retired or adopted",
        )?;
        if self.placed.borrow().is_some() {
            return Ok(());
        }
        let mut attributes = Vec::new();
        for record in self.records.values() {
            for name in ["x", "y", "width", "height"] {
                attributes.push((
                    record.carrier.clone(),
                    name.into(),
                    record.carrier.get_attribute(name),
                ));
            }
            attributes.push((
                record.clip.clone().into(),
                "style".into(),
                record.clip.get_attribute("style"),
            ));
            attributes.push((
                record.root.clone().into(),
                "style".into(),
                record.root.get_attribute("style"),
            ));
            attributes.push((
                record.root.clone().into(),
                "inert".into(),
                record.root.get_attribute("inert"),
            ));
        }
        let moves = self
            .carriers
            .iter()
            .map(|(_, node)| Move {
                node: node.clone(),
                parent: node.parent_node(),
                next: node.next_sibling(),
            })
            .collect();
        let previous_style = self
            .previous
            .borrow()
            .as_ref()
            .and_then(|s| s.shell.0.get_attribute("style"));
        let focus = owner
            .content
            .owner_document()
            .and_then(|d| d.active_element())
            .filter(|active| {
                self.carriers
                    .iter()
                    .any(|(_, carrier)| carrier.contains(Some(active.as_ref())))
            })
            .and_then(|element| element.dyn_into::<HtmlElement>().ok());
        *self.placed.borrow_mut() = Some(Placement {
            previous_style,
            shell_style: self.shell.0.get_attribute("style"),
            moves,
            attributes,
            focus: focus.clone(),
            shell_state: self.shell.0.get_attribute("data-tree-svg-state"),
        });
        let result = (|| {
            for (parent, carrier) in &self.carriers {
                parent.append_child(carrier).map_err(host)?;
            }
            for record in self.records.values() {
                let frame = &self.frames[&record.key];
                place_custom(
                    &record.carrier,
                    &record.clip,
                    &record.root,
                    &frame.data.layout().scene,
                    frame.data.plot(),
                    frame.view,
                )?;
                record.root.remove_attribute("inert").map_err(host)?;
                let font = frame
                    .data
                    .layout()
                    .scene
                    .typography
                    .default
                    .resolve(&ResolvedFont::default());
                record
                    .root
                    .style()
                    .set_property("font-family", &font.css_family())
                    .map_err(host)?;
                record
                    .root
                    .style()
                    .set_property("font-size", &format!("{}px", frame.data.theme().font_size))
                    .map_err(host)?;
                record
                    .root
                    .style()
                    .set_property("font-weight", &font.weight.to_string())
                    .map_err(host)?;
                record
                    .root
                    .style()
                    .set_property("color", &render::css_color(frame.data.theme().text))
                    .map_err(host)?;
            }
            if let Some(previous) = self.previous.borrow().as_ref() {
                set_css(&previous.shell.0, "visibility", "hidden")?;
                set_css(&previous.shell.0, "opacity", "0")?;
            }
            set_css(&self.shell.0, "visibility", "visible")?;
            set_css(&self.shell.0, "opacity", "1")?;
            self.shell
                .0
                .set_attribute("data-tree-svg-state", "placed")
                .map_err(host)?;
            if let Some(focus) = focus {
                restore_dom_focus(&focus)?;
            }
            scene::ensure(owner.alive.get(), "Tree SVG stopped during placement")
        })();
        if let Err(error) = result {
            return match self.rollback() {
                Ok(()) => Err(error),
                Err(rollback) => Err(ChartError(format!(
                    "{}; Tree SVG rollback: {}",
                    error.0, rollback.0
                ))),
            };
        }
        Ok(())
    }
    pub(super) fn rollback(&self) -> Result<()> {
        if self.was_adopted() {
            return Ok(());
        }
        let placement = self.placed.borrow_mut().take();
        let Some(placement) = placement else {
            return Ok(());
        };
        let mut errors = Vec::new();
        for movement in placement.moves.into_iter().rev() {
            let result = match movement.parent {
                Some(parent) => parent
                    .insert_before(&movement.node, movement.next.as_ref())
                    .map(|_| ())
                    .map_err(host),
                None => {
                    movement.node.remove();
                    Ok(())
                }
            };
            if let Err(error) = result {
                errors.push(error.0);
            }
        }
        for (node, name, value) in placement.attributes {
            if let Err(error) = restore_attribute(&node, &name, value.as_deref()) {
                errors.push(error.0);
            }
        }
        if let Err(error) =
            restore_attribute(&self.shell.0, "style", placement.shell_style.as_deref())
        {
            errors.push(error.0);
        }
        if let Err(error) = restore_attribute(
            &self.shell.0,
            "data-tree-svg-state",
            placement.shell_state.as_deref(),
        ) {
            errors.push(error.0);
        }
        if let Some(previous) = self.previous.borrow().as_ref()
            && let Err(error) = restore_attribute(
                &previous.shell.0,
                "style",
                placement.previous_style.as_deref(),
            )
        {
            errors.push(error.0);
        }
        if let Some(focus) = placement.focus
            && let Err(error) = restore_dom_focus(&focus)
        {
            errors.push(error.0);
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ChartError(errors.join("; ")))
        }
    }
    /// No native writes, user construction, cleanup or runtime pump here.
    pub(super) fn adopt(self: &Rc<Self>) {
        let Some(owner) = self.host.upgrade().filter(|h| h.alive.get()) else {
            return;
        };
        if self.lease.adopted.replace(true) {
            return;
        }
        if self.lease.pending.replace(false) {
            owner.pending.set(false);
        }
        if let Some(previous) = self.previous.borrow().as_ref() {
            for (key, record) in &previous.records {
                if self.records.get(key).is_none_or(|r| !Rc::ptr_eq(r, record)) {
                    record.live.set(false);
                    if record.frame.get_untracked().is_some() {
                        record.frame.revise(|frame| *frame = None);
                    }
                }
            }
        }
        for record in self
            .plan
            .layers()
            .iter()
            .filter_map(|occurrence| self.records.get(&occurrence.layer.key))
        {
            let value = Some(LayerSnapshot {
                source: self.frames[&record.key].clone(),
                motion: if record.component.animated {
                    self.motion.clone()
                } else {
                    None
                },
            });
            if record.frame.get_untracked() != value {
                record.updated.set(true);
                record.frame.revise(|frame| *frame = value);
            }
            record.live.set(true);
        }
        let previous = owner.accepted.replace(Some(self.clone()));
        if let Some(previous) = previous {
            owner.retired.borrow_mut().push(previous);
        }
        let old = self.previous.borrow_mut().take();
        drop(old);
        self.placed.borrow_mut().take();
    }
    pub(super) fn finish(&self) -> Vec<ChartError> {
        self.host
            .upgrade()
            .map_or_else(Vec::new, |host| host.pump())
    }
    /// Post-adoption DOM diagnostics. The Engine invokes this after every
    /// metadata stage adopts and before any source or custom runtime pumps.
    pub(super) fn publish(&self) -> Vec<ChartError> {
        if !self.was_adopted() {
            return vec![ChartError("Tree SVG publication is not adopted".into())];
        }
        self.host
            .upgrade()
            .map_or_else(Vec::new, |host| host.publish())
    }
}
impl Drop for Stage {
    fn drop(&mut self) {
        let _ = self.rollback();
    }
}

impl Host {
    fn publish(&self) -> Vec<ChartError> {
        let accepted = self.accepted.borrow().clone();
        let retired = self.retired.borrow().clone();
        let mut errors = Vec::new();
        if let Some(accepted) = accepted {
            for (name, value) in [
                ("data-tree-svg-active", "true"),
                ("data-tree-svg-state", "accepted"),
            ] {
                if let Err(error) = accepted.shell.0.set_attribute(name, value) {
                    errors.push(host(error));
                }
            }
        }
        for stage in retired {
            if let Err(error) = stage.shell.0.set_attribute("data-tree-svg-active", "false") {
                errors.push(host(error));
            }
        }
        errors
    }
    pub(super) fn pump(&self) -> Vec<ChartError> {
        if !self.alive.get() {
            return Vec::new();
        }
        let mut errors = self.publish();
        let accepted = self.accepted.borrow().clone();
        let retired = std::mem::take(&mut *self.retired.borrow_mut());
        if let Some(accepted) = accepted {
            for record in accepted
                .plan
                .layers()
                .iter()
                .filter_map(|occurrence| accepted.records.get(&occurrence.layer.key))
            {
                if !self.alive.get() {
                    break;
                }
                let updated = record.updated.replace(false);
                let pumped = record.web.app().pump();
                if !self.alive.get() || record.retired.get() {
                    break;
                }
                if let Err(error) = pumped {
                    errors.push(host(error));
                    hide_failed(record, &mut errors);
                } else if !record.failed.get() || updated {
                    let shown = (|| {
                        set_css(&record.carrier, "visibility", "visible")?;
                        set_css(&record.carrier, "opacity", "1")?;
                        record.root.remove_attribute("inert").map_err(host)
                    })();
                    match shown {
                        Ok(()) => record.failed.set(false),
                        Err(error) => {
                            errors.push(error);
                            hide_failed(record, &mut errors);
                        }
                    }
                }
            }
        }
        // Every source/layer/tooltip metadata stage has already been adopted.
        // Retired callbacks therefore observe either the complete new cohort or
        // a None layer frame, before their final cleanup is run.
        for stage in &retired {
            for record in stage
                .plan
                .layers()
                .iter()
                .filter_map(|occurrence| stage.records.get(&occurrence.layer.key))
                .filter(|r| !r.live.get() && !r.retired.get())
            {
                if self.alive.get()
                    && let Err(error) = record.web.app().pump()
                {
                    errors.push(host(error));
                }
                if let Some(error) = record.retire() {
                    errors.push(error);
                }
            }
        }
        drop(retired);
        errors
    }
}
fn hide_failed(record: &Record, errors: &mut Vec<ChartError>) {
    record.failed.set(true);
    record.updated.set(false);
    for (name, value) in [("opacity", "0"), ("visibility", "hidden")] {
        if let Err(error) = set_css(&record.carrier, name, value) {
            errors.push(error);
        }
    }
    if let Err(error) = record.root.set_attribute("inert", "") {
        errors.push(host(error));
    }
}
fn set_css(element: &Element, name: &str, value: &str) -> Result<()> {
    let style = js_sys::Reflect::get(element, &wasm_bindgen::JsValue::from_str("style"))
        .map_err(host)?
        .dyn_into::<web_sys::CssStyleDeclaration>()
        .map_err(host)?;
    style.set_property(name, value).map_err(host)
}
fn restore_attribute(node: &Element, name: &str, value: Option<&str>) -> Result<()> {
    match value {
        Some(value) => node.set_attribute(name, value),
        None => node.remove_attribute(name),
    }
    .map_err(host)
}
