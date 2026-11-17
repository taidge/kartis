//! Waffle cursor presentation over the same immutable source and adoption epoch.
//! The shared panel owns only presentation; it does not select a business part.
use std::any::TypeId;

use glory::Bond;

use super::super::tree_tooltip_panel::{
    PanelComponent, PanelContent, PanelHost, PanelStage, Payload,
};
use super::*;

/// The shared natural-size nine-anchor alignment vocabulary. Waffle positioning
/// is cursor-only; corner anchors and a configurable gap are Rust extensions.
pub type BrowserWaffleTooltipAnchor = BrowserTreeTooltipAnchor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserWaffleTooltipOptions {
    pub enabled: bool,
    pub anchor: BrowserWaffleTooltipAnchor,
    pub gap: f64,
}
impl Default for BrowserWaffleTooltipOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: BrowserWaffleTooltipAnchor::Top,
            gap: 14.,
        }
    }
}
impl BrowserWaffleTooltipOptions {
    pub(super) fn validate(&self) -> Result<()> {
        scene::ensure(
            self.gap.is_finite() && (0. ..=4096.).contains(&self.gap),
            "invalid Waffle tooltip gap",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleTooltipFrame {
    pub data: WaffleSvgFrame,
    pub polygon_index: usize,
    pub source_index: usize,
    pub source_generation: u64,
    pub publication: u64,
    pub occurrence: BrowserWaffleMouseOccurrence,
    pub pointer_client: [f64; 2],
    pub viewport_anchor: [f64; 2],
    pub requested_anchor: BrowserWaffleTooltipAnchor,
    pub anchor: BrowserWaffleTooltipAnchor,
}
impl WaffleTooltipFrame {
    pub fn group(&self) -> Result<&WaffleGroup> {
        self.data
            .layout()
            .groups
            .get(self.source_index)
            .ok_or_else(|| ChartError("Waffle tooltip part is missing".into()))
    }
    pub fn datum(&self) -> Result<&RadialDatum> {
        self.data
            .source()
            .get(self.source_index)
            .ok_or_else(|| ChartError("Waffle tooltip source is missing".into()))
    }
    /// Preserve the author's label (including empty text), colon and formatted
    /// value. No display-ID fallback or fabricated scalar source is introduced.
    pub fn text(&self) -> Result<String> {
        let part = self.group()?;
        Ok(format!("{}: {}", part.source.label, part.formatted_value))
    }
}
#[derive(Clone)]
pub struct BrowserWaffleTooltipComponentContext {
    pub revision: u64,
    pub frame: Bond<Option<WaffleTooltipFrame>>,
}

impl Payload for WaffleTooltipFrame {
    type Context = BrowserWaffleTooltipComponentContext;
    const ROOT_CLASS: &'static str = "kartis-waffle-tooltip";
    const TEXT_CLASS: &'static str = "kartis-waffle-tooltip-text";
    const COMPONENT_CLASS: &'static str = "kartis-waffle-tooltip-component";
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context {
        BrowserWaffleTooltipComponentContext { revision, frame }
    }
    fn source_generation(&self) -> u64 {
        self.source_generation
    }
    fn scene(&self) -> &Scene {
        self.data.scene()
    }
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool {
        self.group().is_ok_and(|p| hidden.contains(&p.source.id))
    }
    fn world_anchor(&self) -> [f64; 2] {
        self.group()
            .map_or([0., 0.], |_| [self.data.plot().x, self.data.plot().y])
    }
    fn pointer_client(&self) -> Option<[f64; 2]> {
        Some(self.pointer_client)
    }
    fn viewport_anchor(&self) -> [f64; 2] {
        self.viewport_anchor
    }
    fn set_viewport_anchor(&mut self, p: [f64; 2]) {
        self.viewport_anchor = p;
    }
    fn anchor(&self) -> BrowserWaffleTooltipAnchor {
        self.anchor
    }
    fn resolve_anchor(
        &mut self,
        requested: BrowserWaffleTooltipAnchor,
        x: f64,
        width: f64,
    ) -> BrowserWaffleTooltipAnchor {
        self.requested_anchor = requested;
        self.anchor = match requested {
            BrowserWaffleTooltipAnchor::Left | BrowserWaffleTooltipAnchor::Right => {
                if x < width / 2. {
                    BrowserWaffleTooltipAnchor::Right
                } else {
                    BrowserWaffleTooltipAnchor::Left
                }
            }
            other => other,
        };
        self.anchor
    }
    fn text_record(
        document: &web_sys::Document,
        text: &str,
        frame: Option<&Self>,
    ) -> Result<HtmlElement> {
        let root: HtmlElement = document
            .create_element("div")
            .map_err(host)?
            .dyn_into()
            .map_err(host)?;
        root.set_class_name(Self::TEXT_CLASS);
        if let Some(frame) = frame {
            let part = frame.group()?;
            let chip = document.create_element("span").map_err(host)?;
            chip.set_attribute("class", "kartis-waffle-tooltip-chip")
                .map_err(host)?;
            chip.set_attribute("aria-hidden", "true").map_err(host)?;
            chip.set_attribute("style",&format!("display:inline-block;width:12px;height:12px;margin-right:6px;background:{};pointer-events:none",render::css_color(part.color))).map_err(host)?;
            let label = document.create_element("span").map_err(host)?;
            label.set_text_content(Some(&format!("{}: ", part.source.label)));
            let value = document.create_element("strong").map_err(host)?;
            value.set_text_content(Some(&part.formatted_value));
            root.append_child(&chip).map_err(host)?;
            root.append_child(&label).map_err(host)?;
            root.append_child(&value).map_err(host)?;
        } else {
            root.set_text_content(Some(text));
        }
        Ok(root)
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct InternalFrame {
    pub frame: WaffleTooltipFrame,
    pub source: SourceEnvelope,
}
pub(super) struct InternalContext {
    pub revision: u64,
    pub frame: Bond<Option<InternalFrame>>,
    scope: Option<glory::Scope>,
}
impl InternalContext {
    pub fn scope(&self) -> Result<&glory::Scope> {
        self.scope
            .as_ref()
            .ok_or_else(|| ChartError("Waffle tooltip component scope missing".into()))
    }
}
impl Payload for InternalFrame {
    type Context = InternalContext;
    const ROOT_CLASS: &'static str = WaffleTooltipFrame::ROOT_CLASS;
    const TEXT_CLASS: &'static str = WaffleTooltipFrame::TEXT_CLASS;
    const COMPONENT_CLASS: &'static str = WaffleTooltipFrame::COMPONENT_CLASS;
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context {
        InternalContext {
            revision,
            frame,
            scope: None,
        }
    }
    fn context_with_scope(
        revision: u64,
        frame: Bond<Option<Self>>,
        scope: &glory::Scope,
    ) -> Self::Context {
        InternalContext {
            revision,
            frame,
            scope: Some(scope.clone()),
        }
    }
    fn source_generation(&self) -> u64 {
        self.frame.source_generation()
    }
    fn scene(&self) -> &Scene {
        self.source.paint_scene()
    }
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool {
        self.frame.is_hidden(hidden)
    }
    fn world_anchor(&self) -> [f64; 2] {
        self.frame.world_anchor()
    }
    fn pointer_client(&self) -> Option<[f64; 2]> {
        self.frame.pointer_client()
    }
    fn viewport_anchor(&self) -> [f64; 2] {
        self.frame.viewport_anchor()
    }
    fn set_viewport_anchor(&mut self, p: [f64; 2]) {
        self.frame.set_viewport_anchor(p);
    }
    fn anchor(&self) -> BrowserWaffleTooltipAnchor {
        self.frame.anchor()
    }
    fn resolve_anchor(
        &mut self,
        a: BrowserWaffleTooltipAnchor,
        x: f64,
        w: f64,
    ) -> BrowserWaffleTooltipAnchor {
        self.frame.resolve_anchor(a, x, w)
    }
    fn text_record(
        document: &web_sys::Document,
        text: &str,
        frame: Option<&Self>,
    ) -> Result<HtmlElement> {
        WaffleTooltipFrame::text_record(document, text, frame.map(|f| &f.frame))
    }
}
pub(super) type Component = PanelComponent<InternalFrame>;
pub(super) fn legacy_component<W: glory::Widget + 'static>(
    revision: u64,
    mut factory: impl FnMut(BrowserWaffleTooltipComponentContext) -> Result<W> + 'static,
) -> Rc<Component> {
    Component::new(revision, move |c: InternalContext| {
        let source = c.frame;
        let frame = c
            .scope()?
            .bond(move || source.get().map(|value| value.frame));
        factory(BrowserWaffleTooltipComponentContext {
            revision: c.revision,
            frame,
        })
    })
}
#[derive(Clone)]
pub(super) struct Config {
    pub options: Option<BrowserWaffleTooltipOptions>,
    pub component: Option<Rc<Component>>,
    pub kind: Option<TypeId>,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            options: Some(BrowserWaffleTooltipOptions::default()),
            component: None,
            kind: None,
        }
    }
}
impl Config {
    pub fn is_configured(&self) -> bool {
        self.options.is_some()
    }
    pub fn enabled(&self) -> bool {
        self.options.is_some_and(|o| o.enabled)
    }
}

#[derive(Clone, Debug)]
pub(super) struct Selection {
    id: String,
    key: String,
    polygon_index: usize,
    client: [f64; 2],
}
impl Selection {
    pub fn is_live(&self, plan: &WaffleSvgPlan, interactive: bool) -> bool {
        if !interactive {
            return false;
        }
        let Some(source_index) = plan.frame().group_by_id(&self.id) else {
            return false;
        };
        plan.frame()
            .area(source_index, self.polygon_index)
            .is_ok()
            && plan.layers().iter().any(|occurrence| {
                occurrence.layer.key == self.key
                    && occurrence.layer.kind == WaffleSvgLayerKind::Areas
                    && occurrence.items().iter().any(|item| {
                        matches!(item, WaffleSvgItem::Area { source_index: source, polygon_index: polygon }
                            if *source == source_index && *polygon == self.polygon_index)
                    })
            })
    }

    pub fn from_hit(hit: &BrowserWaffleSvgHit) -> Result<Self> {
        let group = hit
            .data
            .layout()
            .groups
            .get(hit.source_index)
            .ok_or_else(|| ChartError("Waffle tooltip group missing".into()))?;
        scene::ensure(
            hit.client.iter().all(|v| v.is_finite()),
            "Waffle tooltip client invalid",
        )?;
        Ok(Self {
            id: group.source.id.clone(),
            key: hit.key.clone(),
            polygon_index: hit.polygon_index,
            client: hit.client,
        })
    }
}
pub(super) struct Prepare<'a> {
    pub config: &'a Config,
    pub selection: Option<&'a Selection>,
    pub plan: &'a WaffleSvgPlan,
    pub source_generation: u64,
    pub publication: u64,
    pub hidden: &'a BTreeSet<String>,
    pub interactive: bool,
    pub component_budget: usize,
    pub source: &'a SourceEnvelope,
}
struct Model {
    data: SourceEnvelope,
    hidden: BTreeSet<String>,
    model: Rc<SurfaceModel>,
}
pub(super) struct Host {
    panel: Rc<PanelHost<InternalFrame>>,
    surface: HtmlElement,
    records: Rc<Cell<usize>>,
    cache: RefCell<Option<Model>>,
    accepted: RefCell<Option<Rc<Stage>>>,
    alive: Cell<bool>,
}
pub(super) struct Stage {
    host: Weak<Host>,
    panel: Rc<PanelStage<InternalFrame>>,
    model: Rc<SurfaceModel>,
    config: Config,
    data: WaffleSvgFrame,
    generation: u64,
    publication: u64,
    source: SourceEnvelope,
}
impl Host {
    pub fn mount(
        parent: &HtmlElement,
        surface: &HtmlElement,
        records: Rc<Cell<usize>>,
        limit: usize,
    ) -> Result<Rc<Self>> {
        Ok(Rc::new(Self {
            panel: PanelHost::mount_budgeted(parent, records.clone(), limit)?,
            surface: surface.clone(),
            records,
            cache: RefCell::new(None),
            accepted: RefCell::new(None),
            alive: Cell::new(true),
        }))
    }
    pub fn additional_records(&self, config: &Config) -> usize {
        if !config.is_configured() {
            return 0;
        }
        let Some(component) = &config.component else {
            return 0;
        };
        let same = self.accepted.borrow().as_ref().is_some_and(|s| {
            s.config.is_configured()
                && s.config
                    .component
                    .as_ref()
                    .is_some_and(|old| old.revision() == component.revision())
        });
        usize::from(!same)
    }
    pub fn preflight(&self, config: &Config, additional_layers: usize, limit: usize) -> Result<()> {
        if let Some(options) = config.options {
            options.validate()?;
        }
        let needed = self
            .records
            .get()
            .checked_add(additional_layers)
            .and_then(|n| n.checked_add(self.additional_records(config)))
            .ok_or_else(|| ChartError("Waffle component budget overflow".into()))?;
        scene::ensure(
            needed <= limit,
            "Waffle layer and tooltip component budget exceeded",
        )
    }
    pub fn frame(&self) -> Option<WaffleTooltipFrame> {
        self.internal_frame().map(|value| value.frame)
    }
    pub fn internal_frame(&self) -> Option<InternalFrame> {
        self.accepted
            .borrow()
            .as_ref()
            .and_then(|s| s.panel.frame())
    }
    pub fn prepare(self: &Rc<Self>, args: Prepare<'_>) -> Result<Rc<Stage>> {
        scene::ensure(self.alive.get(), "Waffle tooltip host stopped")?;
        self.preflight(args.config, 0, args.component_budget)?;
        self.panel.set_runtime_limit(args.component_budget);
        let data = args.plan.frame();
        scene::ensure(
            data == &args.source.projection,
            "Waffle tooltip raw/projection source differs",
        )?;
        args.source
            .validate_type(args.config.kind.or(args.source.kind()))?;
        let options = args.config.options.unwrap_or_default();
        let frame = if args.config.enabled() && args.interactive {
            args.selection.and_then(|selection| {
                let source_index = data.group_by_id(&selection.id)?;
                let occurrence = args.plan.layers().iter().find(|o| {
                    o.layer.key == selection.key && o.layer.kind == WaffleSvgLayerKind::Areas
                })?;
                data.area(source_index, selection.polygon_index).ok()?;
                Some(WaffleTooltipFrame {
                    data: data.clone(),
                    source_index,
                    polygon_index: selection.polygon_index,
                    source_generation: args.source_generation,
                    publication: args.publication,
                    occurrence: BrowserWaffleMouseOccurrence {
                        key: occurrence.layer.key.clone(),
                        index: occurrence.index,
                        publication: args.publication,
                    },
                    pointer_client: selection.client,
                    viewport_anchor: [0., 0.],
                    requested_anchor: options.anchor,
                    anchor: options.anchor,
                })
            })
        } else {
            None
        };
        let content = if !args.config.is_configured() {
            PanelContent::Unmanaged
        } else if let Some(component) = &args.config.component {
            PanelContent::Component(component.clone())
        } else {
            PanelContent::Text(
                frame
                    .as_ref()
                    .map(WaffleTooltipFrame::text)
                    .transpose()?
                    .unwrap_or_default(),
            )
        };
        let cached = {
            self.cache
                .borrow()
                .as_ref()
                .filter(|m| m.data == *args.source && m.hidden == *args.hidden)
                .map(|m| m.model.clone())
        };
        let model = if let Some(model) = cached {
            model
        } else {
            let mut model =
                SurfaceModel::new(args.source.paint_scene().clone(), SurfaceOptions::default())?;
            model.set_hidden(args.hidden.clone())?;
            let model = Rc::new(model);
            *self.cache.borrow_mut() = Some(Model {
                data: args.source.clone(),
                hidden: args.hidden.clone(),
                model: model.clone(),
            });
            model
        };
        let appearance = TooltipOptions::default();
        let frame = frame.map(|frame| InternalFrame {
            frame,
            source: args.source.clone(),
        });
        let panel = self.panel.prepare(
            args.source_generation,
            frame,
            BrowserTreeTooltipOptions {
                position: BrowserTreeTooltipPosition::Cursor,
                anchor: options.anchor,
                gap: options.gap,
            },
            content,
            appearance,
        )?;
        Ok(Rc::new(Stage {
            host: Rc::downgrade(self),
            panel,
            model,
            config: args.config.clone(),
            data: data.clone(),
            generation: args.source_generation,
            publication: args.publication,
            source: args.source.clone(),
        }))
    }
    pub fn stop(&self) {
        if self.alive.replace(false) {
            self.accepted.borrow_mut().take();
            self.cache.borrow_mut().take();
            self.panel.stop();
        }
    }
}
impl Stage {
    pub fn place(&self) -> Result<()> {
        let host = self
            .host
            .upgrade()
            .ok_or_else(|| ChartError("Waffle tooltip host dropped".into()))?;
        self.panel.place(&self.model, &host.surface, true)
    }
    pub fn validate(&self) -> Result<()> {
        self.panel.validate(&self.model)?;
        if let Some(frame) = self.panel.frame() {
            scene::ensure(
                frame.source == self.source,
                "Waffle tooltip raw cohort differs",
            )?;
            let frame = frame.frame;
            scene::ensure(
                frame.data == self.data
                    && frame.source_generation == self.generation
                    && frame.publication == self.publication
                    && frame.occurrence.publication == self.publication
                    && frame.group()?.source_index == frame.source_index,
                "Waffle tooltip frame differs from adopted source",
            )?;
        }
        Ok(())
    }
    pub fn rollback(&self) -> Result<()> {
        self.panel.rollback()
    }
    pub fn adopt(self: &Rc<Self>) {
        if let Some(host) = self.host.upgrade().filter(|h| h.alive.get()) {
            self.panel.adopt();
            *host.accepted.borrow_mut() = Some(self.clone());
        }
    }
    pub fn publish(&self) -> Vec<ChartError> {
        Vec::new()
    }
    pub fn finish(&self) -> Vec<ChartError> {
        self.host
            .upgrade()
            .map_or_else(Vec::new, |h| h.panel.pump())
    }
}
