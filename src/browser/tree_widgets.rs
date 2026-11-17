//! Strict retained Tree components sharing their owner's activity acceptance.
use glory::Widget;

use super::*;
use crate::widget::plot_widgets::{
    PlotWidgetFactory, PlotWidgetsPacket, composition_factory, scene_composition_factory,
};

/// Component policy fixed for the mounted owner. A new revision reconstructs
/// children; stable identities and revision retain their local state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrowserTreeWidgetOptions {
    pub revision: u64,
    pub replace_ink: bool,
    pub source_ordered: bool,
}
impl Default for BrowserTreeWidgetOptions {
    fn default() -> Self {
        Self {
            revision: 0,
            replace_ink: true,
            source_ordered: true,
        }
    }
}
type LegacyPlacement =
    dyn FnMut(&TreeActivityWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>;
type ScenePlacement =
    dyn FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>;
enum Placement {
    Legacy(Box<LegacyPlacement>),
    Scene(Box<ScenePlacement>),
}
pub(super) struct Components {
    pub options: BrowserTreeWidgetOptions,
    placement: RefCell<Option<Placement>>,
    stopped: Cell<bool>,
}
pub(super) struct ComponentMount {
    pub runtime: Components,
    factory: Option<PlotWidgetFactory>,
}
impl ComponentMount {
    pub(super) fn take_svg_mount(
        &mut self,
        packet: PlotWidgetsPacket,
    ) -> Result<super::super::plot_widgets::Mount> {
        scene::ensure(
            self.runtime.options.source_ordered,
            "Tree SVG components require source-ordered composition",
        )?;
        Ok(super::super::plot_widgets::Mount {
            layout: packet,
            factory: self
                .factory
                .take()
                .ok_or_else(|| ChartError("Tree component factory already mounted".into()))?,
        })
    }
    pub fn mount_canvas(
        &mut self,
        root: Element,
        initial: &TreeActivityLayout,
        options: BrowserOptions,
        bitmap: PreparedTreeCanvas,
        packet: PlotWidgetsPacket,
    ) -> Result<BrowserChart> {
        let frame = tree_activity_frame(initial)?;
        BrowserChart::mount_tree_canvas(
            root,
            frame.scene,
            options,
            bitmap,
            Some(super::super::plot_widgets::Mount {
                layout: packet,
                factory: self
                    .factory
                    .take()
                    .expect("Tree component factory mounts once"),
            }),
        )
    }
    pub fn new<W: Widget + 'static>(
        options: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(PlotWidgetCompositionContext<TreeActivityWidgetData>) -> Result<W> + 'static,
    ) -> Self {
        Self {
            runtime: Components {
                options,
                placement: RefCell::new(Some(Placement::Legacy(Box::new(placement)))),
                stopped: Cell::new(false),
            },
            factory: Some(composition_factory(factory)),
        }
    }
    pub fn components<W: Widget + 'static>(
        options: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Self {
        Self {
            runtime: Components {
                options,
                placement: RefCell::new(Some(Placement::Scene(Box::new(placement)))),
                stopped: Cell::new(false),
            },
            factory: Some(scene_composition_factory(factory)),
        }
    }
    pub fn mount(
        &mut self,
        root: Element,
        initial: &TreeActivityLayout,
        options: BrowserOptions,
    ) -> Result<BrowserChart> {
        let frame = tree_activity_frame(initial)?;
        let packet = self
            .runtime
            .packet(initial, self.runtime.options.revision)?;
        BrowserChart::mount_tree_scene(
            root,
            frame.scene,
            options,
            Some(super::super::plot_widgets::Mount {
                layout: packet,
                factory: self
                    .factory
                    .take()
                    .expect("Tree component factory mounts once"),
            }),
        )
    }
}
impl Components {
    pub fn packet(&self, layout: &TreeActivityLayout, revision: u64) -> Result<PlotWidgetsPacket> {
        self.packet_filtered(layout, revision, None)
    }
    pub(super) fn packet_for_svg(
        &self,
        layout: &TreeActivityLayout,
        revision: u64,
        layers: &[TreeSvgLayer],
    ) -> Result<PlotWidgetsPacket> {
        scene::ensure(
            self.options.source_ordered,
            "Tree SVG components require source-ordered composition",
        )?;
        self.packet_filtered(layout, revision, Some(layers))
    }
    fn packet_filtered(
        &self,
        layout: &TreeActivityLayout,
        revision: u64,
        layers: Option<&[TreeSvgLayer]>,
    ) -> Result<PlotWidgetsPacket> {
        let included = |kind: TreeSvgLayerKind| {
            layers.is_none_or(|layers| layers.iter().any(|l| l.kind == kind))
        };
        scene::ensure(!self.stopped.get(), "Tree components are stopped")?;
        let mut placement = self
            .placement
            .try_borrow_mut()
            .map_err(|_| ChartError("Tree placement is already in use".into()))?
            .take()
            .ok_or_else(|| ChartError("Tree placement is already in use".into()))?;
        let result = match &mut placement {
            Placement::Legacy(placement) => tree_activity_widgets(layout, revision, |context| {
                let kind = match context.tree {
                    TreeWidgetContext::Node(_) => TreeSvgLayerKind::Nodes,
                    TreeWidgetContext::Link(_) => TreeSvgLayerKind::Links,
                };
                if !included(kind) {
                    return Ok(None);
                }
                scene::ensure(!self.stopped.get(), "Tree components are stopped")?;
                let result = placement(context);
                scene::ensure(!self.stopped.get(), "Tree components are stopped")?;
                result
            })
            .map(|widgets| {
                let replace_ink = if self.options.replace_ink {
                    widgets
                        .layout
                        .slots
                        .iter()
                        .map(|slot| slot.mark_id.clone())
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
            }),
            Placement::Scene(placement) => tree_activity_components(layout, revision, |context| {
                let kind = match context.component {
                    TreeComponentContext::Node(_) => TreeSvgLayerKind::Nodes,
                    TreeComponentContext::Link(_) => TreeSvgLayerKind::Links,
                    TreeComponentContext::Label(_) => TreeSvgLayerKind::Labels,
                };
                if !included(kind) {
                    return Ok(None);
                }
                scene::ensure(!self.stopped.get(), "Tree components are stopped")?;
                let result = placement(context);
                scene::ensure(!self.stopped.get(), "Tree components are stopped")?;
                result
            })
            .map(|widgets| {
                let replace_ink = if self.options.replace_ink {
                    widgets
                        .layout
                        .slots
                        .iter()
                        .map(|slot| slot.key.clone())
                        .collect()
                } else {
                    BTreeSet::new()
                };
                let composition = SceneWidgetsComposition {
                    widgets,
                    replace_ink,
                };
                if self.options.source_ordered {
                    PlotWidgetsPacket::scene_in_scene(composition)
                } else {
                    PlotWidgetsPacket::scene_composition(composition)
                }
            }),
        };
        if !self.stopped.get() {
            *self.placement.borrow_mut() = Some(placement);
        }
        result
    }
    pub fn clear(&self) {
        self.stopped.set(true);
        self.placement.borrow_mut().take();
    }
}
