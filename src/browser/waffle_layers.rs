//! Retained Waffle whole-layer subtrees. Each runtime owns one complete Cage;
//! no Bond subscribes across independent Glory application runtimes.
use glory::Widget;

use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserWaffleSvgLayerFrame {
    pub data: WaffleSvgFrame,
    pub presentation: WaffleSvgPlan,
    pub source_generation: u64,
    pub publication: u64,
    pub key: String,
    pub index: usize,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub interactive: bool,
}
#[derive(Clone)]
pub struct BrowserWaffleSvgLayerContext {
    pub key: String,
    pub revision: u64,
    pub initial: BrowserWaffleSvgLayerFrame,
    pub frame: Bond<Option<BrowserWaffleSvgLayerFrame>>,
}
#[derive(Clone, PartialEq)]
pub(super) struct LayerSnapshot {
    pub frame: BrowserWaffleSvgLayerFrame,
    pub source: SourceEnvelope,
}
pub(super) struct InternalContext {
    pub key: String,
    pub revision: u64,
    pub initial: LayerSnapshot,
    pub frame: Bond<Option<LayerSnapshot>>,
    pub scope: glory::Scope,
}
type Factory = dyn FnMut(InternalContext) -> Result<Box<dyn Widget>>;
pub(super) struct Component {
    pub revision: u64,
    pub kind: Option<TypeId>,
    factory: RefCell<Option<Box<Factory>>>,
}
#[derive(Clone)]
pub struct BrowserWaffleSvgProgram {
    pub(super) layers: Vec<WaffleSvgLayer>,
    pub(super) custom: BTreeMap<String, Rc<Component>>,
    pub(super) budget: WaffleSvgBudget,
    pub(super) legends: Vec<WaffleSvgLegendSpec>,
    pub(super) interactive: bool,
}
impl BrowserWaffleSvgProgram {
    pub fn new(layers: Vec<WaffleSvgLayer>) -> Result<Self> {
        let value = Self {
            layers,
            custom: BTreeMap::new(),
            budget: WaffleSvgBudget::default(),
            legends: Vec::new(),
            interactive: true,
        };
        WaffleSvgPlan::validate_layers(&value.layers, value.budget)?;
        Ok(value)
    }
    pub fn defaults() -> Self {
        Self {
            layers: WaffleSvgPlan::default_layers(),
            custom: BTreeMap::new(),
            budget: WaffleSvgBudget::default(),
            legends: Vec::new(),
            interactive: true,
        }
    }
    pub fn custom<W: Widget + 'static>(
        self,
        registry: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(BrowserWaffleSvgLayerContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        self.custom_internal(registry, revision, None, move |c| {
            let frame = c.scope.bond(move || c.frame.get().map(|value| value.frame));
            Ok(Box::new(factory(BrowserWaffleSvgLayerContext {
                key: c.key,
                revision: c.revision,
                initial: c.initial.frame,
                frame,
            })?))
        })
    }
    pub(super) fn custom_internal(
        mut self,
        registry: impl Into<String>,
        revision: u64,
        kind: Option<TypeId>,
        factory: impl FnMut(InternalContext) -> Result<Box<dyn Widget>> + 'static,
    ) -> Result<Self> {
        let registry = registry.into();
        WaffleSvgLayer::new(
            "registry-check",
            WaffleSvgLayerKind::Custom(registry.clone()),
        )?;
        scene::ensure(
            !self.custom.contains_key(&registry),
            "duplicate Waffle SVG registry",
        )?;
        self.custom.insert(
            registry,
            Rc::new(Component {
                revision,
                kind,
                factory: RefCell::new(Some(Box::new(factory))),
            }),
        );
        Ok(self)
    }
    pub fn with_budget(mut self, budget: WaffleSvgBudget) -> Result<Self> {
        WaffleSvgPlan::validate_layers(&self.layers, budget)?;
        self.budget = budget;
        Ok(self)
    }
    pub fn with_legends(mut self, legends: Vec<WaffleSvgLegendSpec>) -> Result<Self> {
        for legend in &legends {
            legend.validate()?;
        }
        self.legends = legends;
        Ok(self)
    }
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }
    pub fn is_interactive(&self) -> bool {
        self.interactive
    }
    pub fn layers(&self) -> &[WaffleSvgLayer] {
        &self.layers
    }
    pub fn budget(&self) -> WaffleSvgBudget {
        self.budget
    }
    pub(super) fn validate(&self) -> Result<()> {
        WaffleSvgPlan::validate_layers(&self.layers, self.budget)?;
        for legend in &self.legends {
            legend.validate()?;
        }
        for layer in &self.layers {
            if let WaffleSvgLayerKind::Custom(key) = &layer.kind {
                scene::ensure(
                    self.custom.contains_key(key),
                    "Waffle SVG custom registry is missing",
                )?;
            }
        }
        Ok(())
    }
}
impl Default for BrowserWaffleSvgProgram {
    fn default() -> Self {
        Self::defaults()
    }
}

pub(super) fn additional(
    owner: &Driver,
    program: &BrowserWaffleSvgProgram,
    previous: Option<&stage::Stage>,
) -> Result<usize> {
    program.validate()?;
    let mut needed = 0usize;
    for layer in &program.layers {
        if let WaffleSvgLayerKind::Custom(registry) = &layer.kind {
            let component = &program.custom[registry];
            scene::ensure(
                component.kind.is_none() || component.kind == owner.raw_type,
                "Waffle layer data type differs from owner",
            )?;
            let retained = previous
                .and_then(|old| old.records.get(&layer.key))
                .is_some_and(|old| {
                    old.registry == *registry && old.component.revision == component.revision
                });
            needed = needed
                .checked_add(usize::from(!retained))
                .ok_or_else(|| ChartError("Waffle component count overflow".into()))?;
        }
    }
    Ok(needed)
}

pub(super) struct Record {
    pub key: String,
    pub registry: String,
    pub component: Rc<Component>,
    pub carrier: Element,
    pub clip: HtmlElement,
    pub root: HtmlElement,
    web: Rc<WebApp>,
    pub frame: Cage<Option<LayerSnapshot>>,
    pub live: Rc<Cell<bool>>,
    count: Rc<Cell<usize>>,
    retired: Cell<bool>,
    failed: Cell<bool>,
}
impl Record {
    pub fn construct(
        owner: &Rc<Driver>,
        registry: &str,
        component: Rc<Component>,
        initial: BrowserWaffleSvgLayerFrame,
        source: SourceEnvelope,
    ) -> Result<Rc<Self>> {
        let document = owner
            .content
            .owner_document()
            .ok_or_else(|| ChartError("Waffle document missing".into()))?;
        let carrier = stage::svg_node(&document, "foreignObject")?;
        carrier
            .set_attribute("class", "kartis-waffle-svg-custom")
            .map_err(host)?;
        carrier
            .set_attribute("data-waffle-svg-key", &initial.key)
            .map_err(host)?;
        carrier
            .set_attribute("data-waffle-svg-registry", registry)
            .map_err(host)?;
        carrier
            .set_attribute("style", "overflow:hidden;pointer-events:none")
            .map_err(host)?;
        let viewport: HtmlElement = document
            .create_element_ns(Some(stage::XHTML_NS), "div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        viewport.set_class_name("kartis-waffle-svg-custom-viewport");
        viewport.set_attribute("style", "position:relative;width:100%;height:100%;overflow:clip;pointer-events:none;isolation:isolate").map_err(host)?;
        let clip: HtmlElement = document
            .create_element_ns(Some(stage::XHTML_NS), "div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        clip.set_class_name("kartis-waffle-svg-custom-clip");
        clip.set_attribute(
            "style",
            "position:absolute;overflow:clip;pointer-events:none",
        )
        .map_err(host)?;
        let root: HtmlElement = document
            .create_element_ns(Some(stage::XHTML_NS), "div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        root.set_class_name("kartis-waffle-svg-custom-host kartis-plot-widget-slot");
        root.set_attribute("xmlns", stage::XHTML_NS).map_err(host)?;
        root.set_attribute("data-waffle-svg-key", &initial.key)
            .map_err(host)?;
        root.set_attribute("inert", "").map_err(host)?;
        root.set_attribute("style", "position:absolute;left:0;top:0;display:block;overflow:clip;pointer-events:none;box-sizing:border-box;transform-origin:0 0").map_err(host)?;
        stage::pose(&carrier, &clip, &root, &initial)?;
        clip.append_child(&root).map_err(host)?;
        viewport.append_child(&clip).map_err(host)?;
        carrier.append_child(&viewport).map_err(host)?;
        let live = Rc::new(Cell::new(false));
        let initialized = Rc::new(RefCell::new(None));
        let result = initialized.clone();
        let weak_owner = Rc::downgrade(owner);
        let ctor = component.clone();
        let key = initial.key.clone();
        let revision = component.revision;
        let web = glory::web::launch_with(root.clone().into(), move |scope| {
            let frame = scope.cage(None::<LayerSnapshot>);
            *result.borrow_mut() = Some(frame);
            glory::ui::widget(move |scope| {
                let bond = scope.bond(move || frame.get());
                let mut factory = ctor
                    .factory
                    .try_borrow_mut()
                    .map_err(|_| {
                        glory::render::Error::Backend("Waffle factory already in use".into())
                    })?
                    .take()
                    .ok_or_else(|| {
                        glory::render::Error::Backend("Waffle factory already in use".into())
                    })?;
                let widget = factory(InternalContext {
                    key: key.clone(),
                    revision,
                    initial: LayerSnapshot {
                        frame: initial.clone(),
                        source: source.clone(),
                    },
                    frame: bond,
                    scope: scope.clone(),
                });
                *ctor.factory.borrow_mut() = Some(factory);
                let widget = widget.map_err(|error| glory::render::Error::Backend(error.0))?;
                if !weak_owner.upgrade().is_some_and(|owner| owner.alive.get()) {
                    return Err(glory::render::Error::Backend(
                        "Waffle stopped during factory".into(),
                    )
                    .into());
                }
                widget.show_in(scope)
            })
        })
        .map_err(host)?;
        // No fallible operation follows the counter increment. In particular
        // a component's constructor can legitimately expose a missing host key;
        // its launched App still needs cleanup on that error path.
        let prepared: Result<_> = (|| {
            owner.require_alive()?;
            let frame = initialized
                .borrow_mut()
                .take()
                .ok_or_else(|| ChartError("Waffle component did not initialize".into()))?;
            let key = initial_key(&root)?;
            let count = owner
                .records
                .get()
                .checked_add(1)
                .ok_or_else(|| ChartError("Waffle component count overflow".into()))?;
            Ok((frame, key, count))
        })();
        let (frame, key, count) = match prepared {
            Ok(prepared) => prepared,
            Err(error) => {
                carrier.remove();
                return match web.app().clear() {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(ChartError(format!(
                        "{}; Waffle construction cleanup: {}",
                        error.0,
                        host(cleanup).0
                    ))),
                };
            }
        };
        owner.records.set(count);
        let record = Rc::new(Self {
            key,
            registry: registry.into(),
            component,
            carrier,
            clip,
            root,
            web: Rc::new(web),
            frame,
            live,
            count: owner.records.clone(),
            retired: Cell::new(false),
            failed: Cell::new(false),
        });
        owner
            .all_records
            .borrow_mut()
            .retain(|record| record.strong_count() > 0);
        owner.all_records.borrow_mut().push(Rc::downgrade(&record));
        Ok(record)
    }
    pub fn retire(&self) -> Option<ChartError> {
        if self.retired.replace(true) {
            return None;
        }
        self.live.set(false);
        self.count.set(self.count.get().saturating_sub(1));
        let error = self.web.app().clear().err().map(host);
        self.carrier.remove();
        error
    }
    pub fn pump(&self) -> Option<ChartError> {
        if self.retired.get() {
            return None;
        }
        let result = self.web.app().pump().map_err(host);
        if self.retired.get() {
            return result.err();
        }
        match result {
            Ok(_) => {
                // Failure is latched for this Record. In particular a deferred
                // flush inside Scope::batch can return Ok before effects run;
                // neither that nor a new source snapshot proves recovery. A
                // new custom revision constructs a fresh Record instead.
                if !self.failed.get() {
                    let shown = (|| {
                        carrier_css(&self.carrier, "visibility", "visible")?;
                        carrier_css(&self.carrier, "opacity", "1")?;
                        self.root.remove_attribute("inert").map_err(host)?;
                        carrier_style(&self.carrier)?
                            .remove_property("display")
                            .map(|_| ())
                            .map_err(host)
                    })();
                    if let Err(error) = shown {
                        return Some(self.hide_failed(error));
                    }
                }
                None
            }
            Err(error) => Some(self.hide_failed(error)),
        }
    }
    pub fn is_failed(&self) -> bool {
        self.failed.get()
    }
    fn hide_failed(&self, error: ChartError) -> ChartError {
        self.failed.set(true);
        let mut errors = vec![error.0];
        // WebKit can retain painted descendants across a foreignObject's
        // visibility/opacity change. display:none removes the entire owned
        // carrier from presentation; inert independently excludes input.
        for (name, value) in [
            ("display", "none"),
            ("visibility", "hidden"),
            ("opacity", "0"),
        ] {
            if let Err(error) = carrier_css(&self.carrier, name, value) {
                errors.push(error.0);
            }
        }
        if let Err(error) = self.root.set_attribute("inert", "").map_err(host) {
            errors.push(error.0);
        }
        if errors.len() > 1 {
            // A native isolation write may itself fail after data adoption.
            // Detaching only this owned carrier is a presentation fallback;
            // its data/Scope stays accepted and latched until replacement.
            self.carrier.remove();
        }
        ChartError(errors.join("; "))
    }
}
fn carrier_css(element: &Element, name: &str, value: &str) -> Result<()> {
    carrier_style(element)?
        .set_property(name, value)
        .map_err(host)
}
fn carrier_style(element: &Element) -> Result<web_sys::CssStyleDeclaration> {
    web_sys::js_sys::Reflect::get(element, &"style".into())
        .map_err(host)?
        .dyn_into::<web_sys::CssStyleDeclaration>()
        .map_err(host)
}
fn initial_key(root: &HtmlElement) -> Result<String> {
    root.get_attribute("data-waffle-svg-key")
        .ok_or_else(|| ChartError("Waffle component key missing".into()))
}
impl Drop for Record {
    fn drop(&mut self) {
        let _ = self.retire();
    }
}
