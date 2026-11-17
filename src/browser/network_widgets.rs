//! Retained node components rebuilt with each accepted force/drag/resize frame.
use glory::Widget;

use super::*;
use crate::network_glyphs::{
    NetworkNodeWidgetContext, NetworkNodeWidgetData, NetworkWidgetContext, NetworkWidgetData,
    network_node_widgets, network_widget_frame, network_widgets,
};
use crate::widget::plot_widgets::composition::CompositionStage;
use crate::widget::plot_widgets::{PlotWidgetFactory, PlotWidgetsPacket, composition_factory};

/// Strict node-component composition, fixed for the mounted lifetime. A stable
/// revision retains child state; request a new revision to rebuild it. Widgets
/// own their native input; unreplaced source nodes keep ordinary force dragging.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserNetworkNodeWidgetOptions {
    pub revision: u64,
    pub replace_ink: bool,
    pub source_ordered: bool,
}
impl Default for BrowserNetworkNodeWidgetOptions {
    fn default() -> Self {
        Self {
            revision: 0,
            replace_ink: true,
            source_ordered: true,
        }
    }
}
/// One strict stage for links and nodes. Revision is shared across both roles;
/// stable revisions retain children by the authored mark identity. Link identity
/// is `force-link:index`. Request a new revision on edge reorder to rebuild
/// children rather than reuse index-local state for different business edges.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserNetworkWidgetOptions {
    pub revision: u64,
    pub replace_ink: bool,
    pub source_ordered: bool,
}
impl Default for BrowserNetworkWidgetOptions {
    fn default() -> Self {
        Self {
            revision: 0,
            replace_ink: true,
            source_ordered: true,
        }
    }
}
type NodePlacement =
    dyn FnMut(&NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>;
type ComponentPlacement =
    dyn FnMut(&NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>;
enum Placement {
    Nodes(Box<NodePlacement>),
    Components(Box<ComponentPlacement>),
}
pub(super) struct NodeWidgets {
    pub options: BrowserNetworkNodeWidgetOptions,
    placement: RefCell<Option<Placement>>,
    stopped: Cell<bool>,
}
pub(super) struct NodeWidgetMount {
    pub runtime: NodeWidgets,
    factory: Option<PlotWidgetFactory>,
}
pub(super) struct RenderedFrame {
    pub frame: SceneInteraction,
    packet: Option<PlotWidgetsPacket>,
}
impl RenderedFrame {
    pub fn plain(frame: SceneInteraction) -> Self {
        Self {
            frame,
            packet: None,
        }
    }
}
impl NodeWidgetMount {
    fn new<W: Widget + 'static>(
        options: BrowserNetworkNodeWidgetOptions,
        placement: impl FnMut(&NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkNodeWidgetData>) -> Result<W> + 'static,
    ) -> Self {
        Self {
            runtime: NodeWidgets {
                options,
                placement: RefCell::new(Some(Placement::Nodes(Box::new(placement)))),
                stopped: Cell::new(false),
            },
            factory: Some(composition_factory(factory)),
        }
    }
    fn components<W: Widget + 'static>(
        options: BrowserNetworkWidgetOptions,
        placement: impl FnMut(&NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkWidgetData>) -> Result<W> + 'static,
    ) -> Self {
        Self {
            runtime: NodeWidgets {
                options: BrowserNetworkNodeWidgetOptions {
                    revision: options.revision,
                    replace_ink: options.replace_ink,
                    source_ordered: options.source_ordered,
                },
                placement: RefCell::new(Some(Placement::Components(Box::new(placement)))),
                stopped: Cell::new(false),
            },
            factory: Some(composition_factory(factory)),
        }
    }
    pub fn mount(
        &mut self,
        root: Element,
        initial: RenderedFrame,
        options: BrowserOptions,
    ) -> Result<BrowserChart> {
        let packet = initial.packet.expect("network component candidate");
        BrowserChart::mount_inner(
            root,
            initial.frame.scene,
            options,
            MarkBlends::default(),
            Some(super::super::plot_widgets::Mount {
                layout: packet,
                factory: self.factory.take().expect("node factory mounts once"),
            }),
        )
    }
}
impl NodeWidgets {
    fn packet<T: Clone + PartialEq + 'static>(
        &self,
        widgets: PlotWidgetsData<T>,
    ) -> PlotWidgetsPacket {
        let replace_ink = if self.options.replace_ink {
            widgets
                .layout
                .slots
                .iter()
                .map(|s| s.mark_id.clone())
                .collect()
        } else {
            BTreeSet::new()
        };
        let composition = PlotWidgetsComposition {
            widgets,
            replace_ink,
        };
        if self.options.source_ordered {
            PlotWidgetsPacket::in_scene(composition)
        } else {
            PlotWidgetsPacket::composition(composition)
        }
    }
    pub fn clear(&self) {
        self.stopped.set(true);
        self.placement.borrow_mut().take();
    }
    pub fn layout(
        &self,
        simulation: &ForceSimulation,
        config: &ChartConfig,
        glyphs: Option<&NetworkGlyphOptions>,
        link: Option<&mut LinkFactory>,
        revision: u64,
    ) -> Result<RenderedFrame> {
        scene::ensure(!self.stopped.get(), "network node components stopped")?;
        let glyphs =
            glyphs.ok_or_else(|| ChartError("network node components require glyphs".into()))?;
        let mut placement = self
            .placement
            .try_borrow_mut()
            .map_err(|_| ChartError("network node placement is already in use".into()))?
            .take()
            .ok_or_else(|| ChartError("network node placement is already in use".into()))?;
        let result = (|| match &mut placement {
            Placement::Nodes(placement) => {
                let nodes = match link {
                    Some(link) => {
                        network_glyph_layout_with_link_factory(config, simulation, glyphs, link)?
                            .node_layout()
                            .clone()
                    }
                    None => simulation.scene_with_glyphs(config, glyphs)?,
                };
                let widgets = network_node_widgets(&nodes, simulation, revision, |context| {
                    scene::ensure(!self.stopped.get(), "network node components stopped")?;
                    let result = placement(context);
                    scene::ensure(!self.stopped.get(), "network node components stopped")?;
                    result
                })?;
                Ok(RenderedFrame {
                    frame: network_widget_frame(&nodes)?,
                    packet: Some(self.packet(widgets)),
                })
            }
            Placement::Components(placement) => {
                let mut inherit =
                    |ctx: &NetworkLinkContext<'_>| Ok(ctx.inherit(ctx.geometry.clone()));
                let links = network_glyph_layout_with_link_factory(
                    config,
                    simulation,
                    glyphs,
                    link.unwrap_or(&mut inherit),
                )?;
                let widgets = network_widgets(&links, simulation, revision, |context| {
                    scene::ensure(!self.stopped.get(), "network components stopped")?;
                    let result = placement(context);
                    scene::ensure(!self.stopped.get(), "network components stopped")?;
                    result
                })?;
                Ok(RenderedFrame {
                    frame: network_widget_frame(links.node_layout())?,
                    packet: Some(self.packet(widgets)),
                })
            }
        })();
        if !self.stopped.get() {
            *self.placement.borrow_mut() = Some(placement);
        }
        result
    }
    pub fn admit(&self, state: &State, frame: &RenderedFrame) -> Result<()> {
        state.plot_widget_layout_candidate(
            frame.frame.clone(),
            frame.packet.as_ref().expect("network composition"),
        )?;
        Ok(())
    }
    pub fn publish(
        &self,
        chart: &BrowserHandle,
        frame: RenderedFrame,
    ) -> Result<Rc<CompositionStage>> {
        chart.enqueue_composition_packet(
            Some(frame.frame),
            frame.packet.expect("network composition"),
        )
    }
}
pub(super) struct PublishingData {
    pub data: Data,
    pub stage: Rc<CompositionStage>,
}
impl Driver {
    pub(super) fn clear_publishing_data(&self) {
        // Disposal can call user cleanup. Never hold the RefCell while dropping
        // the last unadopted stage and its candidate child scopes.
        let pending = self.publishing_data.borrow_mut().take();
        drop(pending);
    }
    pub(super) fn widget_logical_size(&self) -> Result<[f64; 2]> {
        let style = self
            .window
            .get_computed_style(&self.surface)
            .map_err(host)?
            .ok_or_else(|| ChartError("network surface computed style is missing".into()))?;
        let dimension = |name| -> Result<f64> {
            let text = style.get_property_value(name).map_err(host)?;
            let value = text
                .trim()
                .strip_suffix("px")
                .and_then(|text| text.trim().parse::<f64>().ok())
                .ok_or_else(|| {
                    ChartError("network surface requires resolved CSS pixel dimensions".into())
                })?;
            scene::ensure(
                value.is_finite() && value > 0. && value <= 1_000_000.,
                "invalid network logical surface dimensions",
            )?;
            Ok(value)
        };
        Ok([dimension("width")?, dimension("height")?])
    }
    pub(super) fn require_widget_mutation_ready(&self) -> Result<()> {
        scene::ensure(
            self.node_widgets.is_none()
                || (!self.rendering_links.get() && !self.staging_widgets.get()),
            "network node candidate callbacks cannot mutate their owner",
        )
    }
}
impl BrowserNetwork {
    /// Mount links and nodes in a single typed, strict composition. Placement
    /// runs in authored Scene order (links, then nodes); return None to leave a
    /// role unmounted. Both roles share the slot budget, revision and atomic
    /// force/Scene/Widget acceptance. Widgets own native input; link Widgets do
    /// not create canonical source controls or force-drag targets.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_widgets<W: Widget + 'static>(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
        widgets: BrowserNetworkWidgetOptions,
        placement: impl FnMut(&NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network component driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            None,
            Some(NodeWidgetMount::components(widgets, placement, factory)),
        )
    }
    /// Combine exportable link geometry with live link/node components. The
    /// original endpoint midpoint remains the link's AtDatum anchor even when
    /// its custom Path is offset. Geometry and both component roles form one
    /// candidate; a failure in any role preserves the accepted frame.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_link_factory_and_widgets<W: Widget + 'static>(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
        widgets: BrowserNetworkWidgetOptions,
        link_factory: impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + 'static,
        placement: impl FnMut(&NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network component driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            Some(Box::new(link_factory)),
            Some(NodeWidgetMount::components(widgets, placement, factory)),
        )
    }
    pub fn set_widget_revision(&self, revision: u64) -> Result<()> {
        self.handle().set_widget_revision(revision)
    }
    /// Mount actual Glory node Widgets. Placement is retained and borrowed on
    /// each complete force candidate, after glyph/link layout. Strict factory,
    /// build, layout and paint failures preserve accepted physics and children.
    /// Stable IDs/revision retain child state through ticks, drag and resize.
    /// CSS viewport changes themselves are external and cannot be rolled back.
    /// Widgets handle their own native input; source dragging excludes replaced
    /// ink. AtDatum resolves the exact saved glyph origin, including unlabeled
    /// nodes. External callback side effects do not roll back.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_node_widgets<W: Widget + 'static>(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
        widgets: BrowserNetworkNodeWidgetOptions,
        placement: impl FnMut(&NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkNodeWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network node driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            None,
            Some(NodeWidgetMount::new(widgets, placement, factory)),
        )
    }
    /// Combine dynamic link ink with live node components. Neither link ink nor
    /// Widget placement changes force radii, endpoints or the collision solver.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_link_factory_and_node_widgets<W: Widget + 'static>(
        root: Element,
        simulation: ForceSimulation,
        config: ChartConfig,
        options: BrowserNetworkOptions,
        glyphs: NetworkGlyphOptions,
        widgets: BrowserNetworkNodeWidgetOptions,
        link_factory: impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + 'static,
        placement: impl FnMut(&NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<NetworkNodeWidgetData>) -> Result<W> + 'static,
    ) -> Result<Self> {
        scene::ensure(
            options.chart.interaction_data.is_none(),
            "network node driver supplies its own interaction binding",
        )?;
        Self::mount_inner(
            root,
            simulation,
            config,
            options,
            Some(Rc::new(glyphs)),
            Some(Box::new(link_factory)),
            Some(NodeWidgetMount::new(widgets, placement, factory)),
        )
    }
    pub fn set_node_widget_revision(&self, revision: u64) -> Result<()> {
        self.handle().set_node_widget_revision(revision)
    }
}
impl BrowserNetworkHandle {
    /// Request a shared rebuild of the mounted node/link composition. Ordinary
    /// force frames retain both roles; edge reordering retains index identities
    /// unless the application requests a new revision.
    pub fn set_widget_revision(&self, revision: u64) -> Result<()> {
        self.set_node_widget_revision(revision)
    }
    /// Queue a strict component rebuild. The accepted revision changes only
    /// after its matching force/Scene/Widget candidate successfully paints.
    pub fn set_node_widget_revision(&self, revision: u64) -> Result<()> {
        let driver = self.driver()?;
        scene::ensure(
            driver.node_widgets.is_some(),
            "network has no node components",
        )?;
        driver.request()?;
        driver.widget_revision_pending.set(Some(revision));
        Ok(())
    }
}
