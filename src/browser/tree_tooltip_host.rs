//! A real retained presentation component, independent of Scene picking.
use glory::Bond;

use super::*;

/// Position a Tree tooltip at its accepted node or the last native cursor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTreeTooltipPosition {
    #[default]
    Fixed,
    Cursor,
}
/// Natural-size alignment. This is separate from the existing tooltip enum.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BrowserTreeTooltipAnchor {
    #[default]
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    TopLeft,
    Center,
}
impl BrowserTreeTooltipAnchor {
    pub(super) fn translation(self) -> &'static str {
        match self {
            Self::Top => "translate(-50%,-100%)",
            Self::TopRight => "translate(0%,-100%)",
            Self::Right => "translate(0%,-50%)",
            Self::BottomRight => "translate(0%,0%)",
            Self::Bottom => "translate(-50%,0%)",
            Self::BottomLeft => "translate(-100%,0%)",
            Self::Left => "translate(-100%,-50%)",
            Self::TopLeft => "translate(-100%,-100%)",
            Self::Center => "translate(-50%,-50%)",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserTreeTooltipOptions {
    pub position: BrowserTreeTooltipPosition,
    pub anchor: BrowserTreeTooltipAnchor,
    pub gap: f64,
}
impl Default for BrowserTreeTooltipOptions {
    fn default() -> Self {
        Self {
            position: BrowserTreeTooltipPosition::Fixed,
            anchor: BrowserTreeTooltipAnchor::Top,
            gap: 14.,
        }
    }
}
impl BrowserTreeTooltipOptions {
    pub(super) fn validate(&self) -> Result<()> {
        scene::ensure(
            self.gap.is_finite() && (0. ..=4096.).contains(&self.gap),
            "invalid Tree tooltip gap",
        )
    }
}
/// An accepted logical tooltip. Its box has natural DOM/Glory dimensions.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeTooltipFrame {
    pub data: TreeTooltipData,
    pub source_generation: u64,
    pub pointer_client: Option<[f64; 2]>,
    pub viewport_anchor: [f64; 2],
    pub anchor: BrowserTreeTooltipAnchor,
}
/// The component is retained across nodes. None means no native tooltip target.
#[derive(Clone)]
pub struct BrowserTreeTooltipComponentContext {
    pub revision: u64,
    pub frame: Bond<Option<TreeTooltipFrame>>,
}

/// SVG link tooltips follow the native cursor; corner anchors are a Rust extension.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrowserTreeLinkTooltipOptions {
    pub anchor: BrowserTreeTooltipAnchor,
    pub gap: f64,
}
impl Default for BrowserTreeLinkTooltipOptions {
    fn default() -> Self {
        Self {
            anchor: BrowserTreeTooltipAnchor::Top,
            gap: 14.,
        }
    }
}
impl BrowserTreeLinkTooltipOptions {
    pub(super) fn validate(&self) -> Result<()> {
        self.panel().validate()
    }
    pub(super) fn panel(self) -> BrowserTreeTooltipOptions {
        BrowserTreeTooltipOptions {
            position: BrowserTreeTooltipPosition::Cursor,
            anchor: self.anchor,
            gap: self.gap,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLinkTooltipFrame {
    pub data: TreeLinkTooltipData,
    pub source_generation: u64,
    pub pointer_client: [f64; 2],
    pub viewport_anchor: [f64; 2],
    pub requested_anchor: BrowserTreeTooltipAnchor,
    pub anchor: BrowserTreeTooltipAnchor,
    pub occurrence: Option<BrowserTreeMouseOccurrence>,
    pub origin: BrowserTreeMouseOrigin,
}
#[derive(Clone)]
pub struct BrowserTreeLinkTooltipComponentContext {
    pub revision: u64,
    pub frame: Bond<Option<TreeLinkTooltipFrame>>,
}

use super::tree_tooltip_panel::{PanelComponent, PanelContent, PanelHost, PanelStage, Payload};
pub(super) type Component = PanelComponent<TreeTooltipFrame>;
pub(super) type Content = PanelContent<TreeTooltipFrame>;
pub(super) type LinkComponent = PanelComponent<TreeLinkTooltipFrame>;
pub(super) type LinkContent = PanelContent<TreeLinkTooltipFrame>;

impl Payload for TreeTooltipFrame {
    type Context = BrowserTreeTooltipComponentContext;
    const ROOT_CLASS: &'static str = "kartis-tree-tooltip";
    const TEXT_CLASS: &'static str = "kartis-tree-tooltip-text";
    const COMPONENT_CLASS: &'static str = "kartis-tree-tooltip-component";
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context {
        BrowserTreeTooltipComponentContext { revision, frame }
    }
    fn source_generation(&self) -> u64 {
        self.source_generation
    }
    fn scene(&self) -> &Scene {
        &self.data.layout().scene
    }
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool {
        hidden.contains(&self.data.datum().id)
    }
    fn world_anchor(&self) -> [f64; 2] {
        self.data.anchor()
    }
    fn pointer_client(&self) -> Option<[f64; 2]> {
        self.pointer_client
    }
    fn viewport_anchor(&self) -> [f64; 2] {
        self.viewport_anchor
    }
    fn set_viewport_anchor(&mut self, point: [f64; 2]) {
        self.viewport_anchor = point;
    }
    fn anchor(&self) -> BrowserTreeTooltipAnchor {
        self.anchor
    }
    fn resolve_anchor(
        &mut self,
        requested: BrowserTreeTooltipAnchor,
        _: f64,
        _: f64,
    ) -> BrowserTreeTooltipAnchor {
        self.anchor = requested;
        requested
    }
}
impl Payload for TreeLinkTooltipFrame {
    type Context = BrowserTreeLinkTooltipComponentContext;
    const ROOT_CLASS: &'static str = "kartis-tree-link-tooltip";
    const TEXT_CLASS: &'static str = "kartis-tree-link-tooltip-text";
    const COMPONENT_CLASS: &'static str = "kartis-tree-link-tooltip-component";
    fn context(revision: u64, frame: Bond<Option<Self>>) -> Self::Context {
        BrowserTreeLinkTooltipComponentContext { revision, frame }
    }
    fn source_generation(&self) -> u64 {
        self.source_generation
    }
    fn scene(&self) -> &Scene {
        self.data.frame().scene()
    }
    fn is_hidden(&self, hidden: &BTreeSet<String>) -> bool {
        self.data
            .mark()
            .is_some_and(|mark| hidden.contains(&mark.series))
    }
    fn world_anchor(&self) -> [f64; 2] {
        self.data.link().position
    }
    fn pointer_client(&self) -> Option<[f64; 2]> {
        Some(self.pointer_client)
    }
    fn viewport_anchor(&self) -> [f64; 2] {
        self.viewport_anchor
    }
    fn set_viewport_anchor(&mut self, point: [f64; 2]) {
        self.viewport_anchor = point;
    }
    fn anchor(&self) -> BrowserTreeTooltipAnchor {
        self.anchor
    }
    fn resolve_anchor(
        &mut self,
        requested: BrowserTreeTooltipAnchor,
        x: f64,
        width: f64,
    ) -> BrowserTreeTooltipAnchor {
        self.requested_anchor = requested;
        self.anchor = match requested {
            BrowserTreeTooltipAnchor::Left | BrowserTreeTooltipAnchor::Right => {
                if x < width / 2. {
                    BrowserTreeTooltipAnchor::Right
                } else {
                    BrowserTreeTooltipAnchor::Left
                }
            }
            other => other,
        };
        self.anchor
    }
}

#[derive(Clone)]
pub(super) struct PanelRequest<F: Payload> {
    pub frame: Option<F>,
    pub options: BrowserTreeTooltipOptions,
    pub content: PanelContent<F>,
}
impl<F: Payload> Default for PanelRequest<F> {
    fn default() -> Self {
        Self {
            frame: None,
            options: Default::default(),
            content: PanelContent::Unmanaged,
        }
    }
}
#[derive(Clone)]
pub(super) struct Request {
    pub source_generation: u64,
    pub node: PanelRequest<TreeTooltipFrame>,
    pub link: PanelRequest<TreeLinkTooltipFrame>,
}
pub(super) struct Host {
    node: Rc<PanelHost<TreeTooltipFrame>>,
    link: Rc<PanelHost<TreeLinkTooltipFrame>>,
    accepted: RefCell<Option<Rc<Stage>>>,
}
pub(super) struct Stage {
    host: Weak<Host>,
    pub(super) source_generation: u64,
    request: Request,
    node: Rc<PanelStage<TreeTooltipFrame>>,
    link: Rc<PanelStage<TreeLinkTooltipFrame>>,
    adopted: Cell<bool>,
}
impl Host {
    fn mount(parent: &HtmlElement) -> Result<Rc<Self>> {
        Ok(Rc::new(Self {
            node: PanelHost::mount(parent)?,
            link: PanelHost::mount(parent)?,
            accepted: RefCell::new(None),
        }))
    }
    fn prepare(self: &Rc<Self>, request: Request, appearance: TooltipOptions) -> Result<Rc<Stage>> {
        scene::ensure(
            request.node.frame.is_none() || request.link.frame.is_none(),
            "Tree tooltip families cannot both be selected",
        )?;
        let node = self.node.prepare(
            request.source_generation,
            request.node.frame.clone(),
            request.node.options,
            request.node.content.clone(),
            appearance.clone(),
        )?;
        let link = self.link.prepare(
            request.source_generation,
            request.link.frame.clone(),
            request.link.options,
            request.link.content.clone(),
            appearance,
        )?;
        Ok(Rc::new(Stage {
            host: Rc::downgrade(self),
            source_generation: request.source_generation,
            request,
            node,
            link,
            adopted: Cell::new(false),
        }))
    }
    fn frame(&self) -> Option<TreeTooltipFrame> {
        self.node.frame()
    }
    fn link_frame(&self) -> Option<TreeLinkTooltipFrame> {
        self.link.frame()
    }
    fn pump(&self) -> Vec<ChartError> {
        let mut errors = self.node.pump();
        errors.extend(self.link.pump());
        errors
    }
    pub(super) fn stop(&self) {
        self.accepted.borrow_mut().take();
        self.node.stop();
        self.link.stop();
    }
}
impl Stage {
    pub(super) fn was_adopted(&self) -> bool {
        self.adopted.get()
    }
    pub(super) fn managed(&self) -> bool {
        self.node.managed() || self.link.frame().is_some()
    }
    fn request(&self) -> Request {
        let mut request = self.request.clone();
        request.node.frame = self.node.frame();
        request.link.frame = self.link.frame();
        request
    }
    fn sync_occurrence(&self, svg: Option<&super::tree_svg_frame::Stage>) {
        self.link.update_frame(|frame| {
            let Some(value) = frame.as_mut() else { return; };
            let Some(old) = value.occurrence.as_ref() else { return; };
            let current = svg.and_then(|stage| stage.plan().layers().iter().find(|occurrence|
                occurrence.layer.key == old.key && (value.origin == BrowserTreeMouseOrigin::BoundComponent
                    || (occurrence.layer.kind == TreeSvgLayerKind::Links && value.data.mark().is_some_and(|mark|
                        occurrence.items().iter().any(|item| matches!(item, DrawItem::Mark(index) if stage.plan().frame().layout().scene.marks[*index].id == mark.id))))))
                .map(|occurrence| BrowserTreeMouseOccurrence { key: old.key.clone(), index: occurrence.index, publication: stage.publication() }));
            if let Some(current) = current { value.occurrence = Some(current); } else { *frame = None; }
        });
    }
    pub(super) fn validate(&self, model: &SurfaceModel) -> Result<()> {
        self.node.validate(model)?;
        self.link.validate(model)
    }
    pub(super) fn place(
        &self,
        model: &SurfaceModel,
        surface: &HtmlElement,
        enabled: bool,
    ) -> Result<()> {
        self.node.place(model, surface, enabled)?;
        if let Err(error) = self.link.place(model, surface, enabled) {
            self.node.rollback()?;
            return Err(error);
        }
        Ok(())
    }
    pub(super) fn rollback(&self) -> Result<()> {
        self.link.rollback()?;
        self.node.rollback()
    }
    pub(super) fn adopt(self: &Rc<Self>) {
        if self.adopted.get() {
            return;
        }
        self.node.adopt();
        self.link.adopt();
        if self.node.was_adopted() && self.link.was_adopted() {
            self.adopted.set(true);
            if let Some(host) = self.host.upgrade() {
                host.accepted.replace(Some(self.clone()));
            }
        }
    }
}
pub(super) struct Candidate {
    pub(super) input: Option<Box<Input>>,
    pub(super) stage: Rc<Stage>,
    pub(super) source_stage: Option<Rc<crate::widget::plot_widgets::composition::CompositionStage>>,
}
impl Engine {
    fn tooltip_host(&self) -> Result<Rc<Host>> {
        scene::ensure(self.alive.get(), "Tree tooltip chart stopped")?;
        if let Some(host) = self.tree_tooltips.borrow().as_ref() {
            return Ok(host.clone());
        }
        let host = Host::mount(&self.outer)?;
        *self.tree_tooltips.borrow_mut() = Some(host.clone());
        Ok(host)
    }
    pub(super) fn prepare_tree_tooltips(&self, request: Request) -> Result<Rc<Stage>> {
        let appearance = self
            .state
            .try_borrow()
            .map_err(|_| ChartError("Tree tooltip frame is already in use".into()))?
            .as_ref()
            .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?
            .options
            .tooltip_options
            .clone();
        self.tooltip_host()?.prepare(request, appearance)
    }
    pub(super) fn tree_tooltip_frame(&self) -> Option<TreeTooltipFrame> {
        if !self.alive.get() {
            return None;
        }
        self.tree_tooltips
            .borrow()
            .as_ref()
            .and_then(|host| host.frame())
    }
    pub(super) fn tree_link_tooltip_frame(&self) -> Option<TreeLinkTooltipFrame> {
        if !self.alive.get() {
            return None;
        }
        self.tree_tooltips
            .borrow()
            .as_ref()
            .and_then(|host| host.link_frame())
    }
    pub(super) fn enqueue_tree_tooltip(
        &self,
        layout: Option<SceneInteraction>,
        bitmap: Option<PreparedTreeCanvas>,
        packet: Option<crate::widget::plot_widgets::PlotWidgetsPacket>,
        stage: Rc<Stage>,
    ) -> Result<Option<Rc<crate::widget::plot_widgets::composition::CompositionStage>>> {
        let mut source_stage = None;
        let input = if let Some(layout) = layout {
            let prepared = {
                let state = self
                    .state
                    .try_borrow()
                    .map_err(|_| ChartError("Tree tooltip frame is already in use".into()))?;
                let s = state
                    .as_ref()
                    .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
                if let Some(bitmap) = &bitmap {
                    tree_canvas_frame::policy(&s.options, packet.as_ref())?;
                    tree_canvas_frame::validate_bitmap(&layout.scene, bitmap)?;
                    scene::ensure(
                        s.motion.is_none() && s.current_blends().is_empty() && s.blends.is_empty(),
                        "raw Tree Canvas does not support animation or blends",
                    )?;
                }
                let (model, prepared) = if let Some(packet) = &packet {
                    let (model, _) = s.plot_widget_layout_candidate(layout.clone(), packet)?;
                    let prepared = plot_widgets::prepare(&model, packet, true)?;
                    (model, Some(prepared))
                } else {
                    s.admit_interaction_scene(&layout.scene, layout.interaction.as_ref())?;
                    let mut model = s.model.clone();
                    model.set_layout(layout.clone())?;
                    (model, None)
                };
                stage.validate(&model)?;
                prepared
            };
            if let Some(prepared) = prepared {
                let host = self
                    .plot_widgets
                    .borrow()
                    .clone()
                    .ok_or_else(|| ChartError("Tree component host retired".into()))?;
                source_stage = Some(host.stage(prepared)?);
            }
            scene::ensure(
                self.alive.get(),
                "Tree tooltip stopped during source component construction",
            )?;
            if let Some(bitmap) = bitmap {
                let composition = packet.map(|packet| {
                    (
                        Rc::new(packet),
                        source_stage.as_ref().expect("source stage").clone(),
                    )
                });
                Some(Box::new(Input::TreeCanvas(Box::new(
                    tree_canvas_frame::Candidate {
                        layout,
                        bitmap,
                        composition,
                    },
                ))))
            } else if let Some(packet) = packet {
                Some(Box::new(Input::PlotWidgetComposition(Box::new((
                    Some(layout),
                    packet,
                    source_stage.as_ref().expect("source stage").clone(),
                )))))
            } else {
                Some(Box::new(Input::Layout(Box::new((
                    layout,
                    MarkBlends::default(),
                )))))
            }
        } else {
            scene::ensure(
                bitmap.is_none() && packet.is_none(),
                "Tree tooltip-only update carries source paint",
            )?;
            let state = self
                .state
                .try_borrow()
                .map_err(|_| ChartError("Tree tooltip frame is already in use".into()))?;
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
            scene::ensure(
                s.tree_tooltip
                    .as_ref()
                    .is_none_or(|old| old.source_generation == stage.source_generation),
                "Tree tooltip-only source generation is stale",
            )?;
            stage.validate(&s.tree_tooltip_model()?)?;
            None
        };
        self.try_enqueue(Input::TreeTooltip(Box::new(Candidate {
            input,
            stage,
            source_stage: source_stage.clone(),
        })))?;
        Ok(source_stage)
    }
    pub(super) fn tooltip_stage_for_paint(&self) -> Result<Option<Rc<Stage>>> {
        let (stage, appearance) = {
            let state = self.state.borrow();
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
            (s.tree_tooltip.clone(), s.options.tooltip_options.clone())
        };
        match stage {
            Some(stage) if !stage.was_adopted() => Ok(Some(stage)),
            Some(stage) => {
                let host = self.tooltip_host()?;
                Ok(Some(host.prepare(stage.request(), appearance)?))
            }
            None => {
                let host = self.tree_tooltips.borrow().clone();
                if let Some(host) = host.filter(|h| h.accepted.borrow().is_some()) {
                    Ok(Some(host.prepare(
                        Request {
                            source_generation: 0,
                            node: Default::default(),
                            link: Default::default(),
                        },
                        appearance,
                    )?))
                } else {
                    Ok(None)
                }
            }
        }
    }
    pub(super) fn place_tree_tooltip(&self, stage: &Rc<Stage>) -> Result<()> {
        let (model, surface, enabled, svg) = {
            let state = self.state.borrow();
            let s = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree tooltip chart stopped".into()))?;
            (
                s.tree_tooltip_model()?,
                s.surface.clone(),
                s.options.tooltip,
                s.tree_svg.clone(),
            )
        };
        stage.sync_occurrence(svg.as_deref());
        stage.place(&model, &surface, enabled)
    }
    pub(super) fn pump_tree_tooltip(&self) -> Vec<ChartError> {
        let host = self.tree_tooltips.borrow().clone();
        host.map_or_else(Vec::new, |host| host.pump())
    }
}
