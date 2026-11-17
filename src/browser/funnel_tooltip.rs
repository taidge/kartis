//! Funnel cursor presentation over the same immutable source and adoption epoch.
//! The shared panel owns only presentation; it does not select a business part.
use std::any::TypeId;

use glory::Bond;

use super::funnel::{BrowserFunnelMouseOccurrence, BrowserFunnelSvgHit, SourceEnvelope};
use super::tree_tooltip_panel::{PanelComponent, PanelContent, PanelHost, PanelStage, Payload};
use super::*;

/// The shared natural-size nine-anchor alignment vocabulary. Funnel positioning
/// is cursor-only; corner anchors and a configurable gap are Rust extensions.
pub type BrowserFunnelTooltipAnchor = BrowserTreeTooltipAnchor;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserFunnelTooltipOptions {
    pub enabled: bool,
    pub anchor: BrowserFunnelTooltipAnchor,
    pub gap: f64,
}
impl Default for BrowserFunnelTooltipOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            anchor: BrowserFunnelTooltipAnchor::Top,
            gap: 14.,
        }
    }
}
impl BrowserFunnelTooltipOptions {
    pub(super) fn validate(&self) -> Result<()> {
        scene::ensure(
            self.gap.is_finite() && (0. ..=4096.).contains(&self.gap),
            "invalid Funnel tooltip gap",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelTooltipFrame {
    pub data: FunnelSvgFrame,
    pub part_index: usize,
    pub source_index: usize,
    pub source_generation: u64,
    pub publication: u64,
    pub occurrence: BrowserFunnelMouseOccurrence,
    pub pointer_client: [f64; 2],
    pub viewport_anchor: [f64; 2],
    pub requested_anchor: BrowserFunnelTooltipAnchor,
    pub anchor: BrowserFunnelTooltipAnchor,
}
impl FunnelTooltipFrame {
    pub fn part(&self) -> Result<&FunnelPart> {
        self.data
            .parts()
            .get(self.part_index)
            .ok_or_else(|| ChartError("Funnel tooltip part is missing".into()))
    }
    pub fn datum(&self) -> Result<&FunnelDatum> {
        self.data
            .source()
            .get(self.source_index)
            .ok_or_else(|| ChartError("Funnel tooltip source is missing".into()))
    }
    pub fn style(&self) -> Result<&FunnelComputedPartStyle> {
        self.data
            .styles()
            .get(self.part_index)
            .ok_or_else(|| ChartError("Funnel tooltip style is missing".into()))
    }
    /// Preserve the author's label (including empty text), colon and formatted
    /// value. No display-ID fallback or fabricated scalar source is introduced.
    pub fn text(&self) -> Result<String> {
        let part = self.part()?;
        Ok(format!("{}: {}", part.datum.label, part.formatted_value))
    }
}
#[derive(Clone)]
pub struct BrowserFunnelTooltipComponentContext {
    pub revision: u64,
    pub frame: Bond<Option<FunnelTooltipFrame>>,
}

impl Payload for FunnelTooltipFrame {
    type Context = BrowserFunnelTooltipComponentContext;
    const ROOT_CLASS: &'static str = "kartis-funnel-tooltip";
    const TEXT_CLASS: &'static str = "kartis-funnel-tooltip-text";
    const COMPONENT_CLASS: &'static str = "kartis-funnel-tooltip-component";
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context {
        BrowserFunnelTooltipComponentContext { revision, frame }
    }
    fn source_generation(&self) -> u64 {
        self.source_generation
    }
    fn scene(&self) -> &Scene {
        self.data.scene()
    }
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool {
        self.part().is_ok_and(|p| hidden.contains(&p.series))
    }
    fn world_anchor(&self) -> [f64; 2] {
        self.part().map_or([0., 0.], |p| p.center)
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
    fn anchor(&self) -> BrowserFunnelTooltipAnchor {
        self.anchor
    }
    fn resolve_anchor(
        &mut self,
        requested: BrowserFunnelTooltipAnchor,
        x: f64,
        width: f64,
    ) -> BrowserFunnelTooltipAnchor {
        self.requested_anchor = requested;
        self.anchor = match requested {
            BrowserFunnelTooltipAnchor::Left | BrowserFunnelTooltipAnchor::Right => {
                if x < width / 2. {
                    BrowserFunnelTooltipAnchor::Right
                } else {
                    BrowserFunnelTooltipAnchor::Left
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
            let part = frame.part()?;
            let chip = document.create_element("span").map_err(host)?;
            chip.set_attribute("class", "kartis-funnel-tooltip-chip")
                .map_err(host)?;
            chip.set_attribute("aria-hidden", "true").map_err(host)?;
            chip.set_attribute("style",&format!("display:inline-block;width:12px;height:12px;margin-right:6px;background:{};pointer-events:none",render::css_color(part.color))).map_err(host)?;
            let label = document.create_element("span").map_err(host)?;
            label.set_text_content(Some(&format!("{}: ", part.datum.label)));
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
    pub frame: FunnelTooltipFrame,
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
            .ok_or_else(|| ChartError("Funnel tooltip component scope missing".into()))
    }
}
impl Payload for InternalFrame {
    type Context = InternalContext;
    const ROOT_CLASS: &'static str = FunnelTooltipFrame::ROOT_CLASS;
    const TEXT_CLASS: &'static str = FunnelTooltipFrame::TEXT_CLASS;
    const COMPONENT_CLASS: &'static str = FunnelTooltipFrame::COMPONENT_CLASS;
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
    fn anchor(&self) -> BrowserFunnelTooltipAnchor {
        self.frame.anchor()
    }
    fn resolve_anchor(
        &mut self,
        a: BrowserFunnelTooltipAnchor,
        x: f64,
        w: f64,
    ) -> BrowserFunnelTooltipAnchor {
        self.frame.resolve_anchor(a, x, w)
    }
    fn text_record(
        document: &web_sys::Document,
        text: &str,
        frame: Option<&Self>,
    ) -> Result<HtmlElement> {
        FunnelTooltipFrame::text_record(document, text, frame.map(|f| &f.frame))
    }
}
pub(super) type Component = PanelComponent<InternalFrame>;
pub(super) fn legacy_component<W: glory::Widget + 'static>(
    revision: u64,
    mut factory: impl FnMut(BrowserFunnelTooltipComponentContext) -> Result<W> + 'static,
) -> Rc<Component> {
    Component::new(revision, move |c: InternalContext| {
        let source = c.frame;
        let frame = c
            .scope()?
            .bond(move || source.get().map(|value| value.frame));
        factory(BrowserFunnelTooltipComponentContext {
            revision: c.revision,
            frame,
        })
    })
}
#[derive(Clone, Default)]
pub(super) struct Config {
    pub options: Option<BrowserFunnelTooltipOptions>,
    pub component: Option<Rc<Component>>,
    pub kind: Option<TypeId>,
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
    id: CategoryKey,
    key: String,
    mark_id: String,
    client: [f64; 2],
}
impl Selection {
    pub fn from_hit(hit: &BrowserFunnelSvgHit) -> Result<Self> {
        let part = hit
            .data
            .parts()
            .get(hit.part_index)
            .ok_or_else(|| ChartError("Funnel tooltip hit part is missing".into()))?;
        scene::ensure(
            part.source_index == hit.source_index
                && part.mark_id == hit.mark_id
                && hit.client.iter().all(|p| p.is_finite()),
            "Funnel tooltip hit differs from source",
        )?;
        Ok(Self {
            id: part.datum.id.clone(),
            key: hit.key.clone(),
            mark_id: hit.mark_id.clone(),
            client: hit.client,
        })
    }
    pub fn matches(&self, key: &str, mark_id: &str) -> bool {
        self.key == key && self.mark_id == mark_id
    }
}
pub(super) struct Prepare<'a> {
    pub config: &'a Config,
    pub selection: Option<&'a Selection>,
    pub plan: &'a FunnelSvgPlan,
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
    data: FunnelSvgFrame,
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
            .ok_or_else(|| ChartError("Funnel component budget overflow".into()))?;
        scene::ensure(
            needed <= limit,
            "Funnel layer and tooltip component budget exceeded",
        )
    }
    pub fn frame(&self) -> Option<FunnelTooltipFrame> {
        self.internal_frame().map(|value| value.frame)
    }
    pub fn internal_frame(&self) -> Option<InternalFrame> {
        self.accepted
            .borrow()
            .as_ref()
            .and_then(|s| s.panel.frame())
    }
    pub fn prepare(self: &Rc<Self>, args: Prepare<'_>) -> Result<Rc<Stage>> {
        scene::ensure(self.alive.get(), "Funnel tooltip host stopped")?;
        self.preflight(args.config, 0, args.component_budget)?;
        self.panel.set_runtime_limit(args.component_budget);
        let data = args.plan.frame();
        scene::ensure(
            data == &args.source.projection,
            "Funnel tooltip raw/projection source differs",
        )?;
        args.source
            .validate_type(args.config.kind.or(args.source.kind()))?;
        let options = args.config.options.unwrap_or_default();
        let frame = if args.config.enabled() && args.interactive {
            if let Some(selection) = args.selection {
                let part = data.part_by_id(&selection.id)?;
                let occurrence = args.plan.layers().iter().find(|o| {
                    o.layer.key == selection.key && o.layer.kind == FunnelSvgLayerKind::Parts
                });
                if let (Some(index), Some(occurrence)) = (part, occurrence) {
                    let body = data.body_mark(index)?;
                    let included = body.filter(|mark| {
                        !args.hidden.contains(&mark.series)
                            && occurrence.items().iter().any(|item| match item {
                                DrawItem::Mark(index) => data.scene().marks[*index].id == mark.id,
                                DrawItem::Label(_) => false,
                            })
                    });
                    if let Some(body) = included {
                        scene::ensure(
                            body.id == data.parts()[index].mark_id,
                            "Funnel tooltip body changed",
                        )?;
                        Some(FunnelTooltipFrame {
                            data: data.clone(),
                            part_index: index,
                            source_index: data.parts()[index].source_index,
                            source_generation: args.source_generation,
                            publication: args.publication,
                            occurrence: BrowserFunnelMouseOccurrence {
                                key: occurrence.layer.key.clone(),
                                index: occurrence.index,
                                publication: args.publication,
                            },
                            pointer_client: selection.client,
                            viewport_anchor: [0., 0.],
                            requested_anchor: options.anchor,
                            anchor: options.anchor,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
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
                    .map(FunnelTooltipFrame::text)
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
            .ok_or_else(|| ChartError("Funnel tooltip host dropped".into()))?;
        self.panel.place(&self.model, &host.surface, true)
    }
    pub fn validate(&self) -> Result<()> {
        self.panel.validate(&self.model)?;
        if let Some(frame) = self.panel.frame() {
            scene::ensure(
                frame.source == self.source,
                "Funnel tooltip raw cohort differs",
            )?;
            let frame = frame.frame;
            scene::ensure(
                frame.data == self.data
                    && frame.source_generation == self.generation
                    && frame.publication == self.publication
                    && frame.occurrence.publication == self.publication
                    && frame.part()?.source_index == frame.source_index,
                "Funnel tooltip frame differs from adopted source",
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
