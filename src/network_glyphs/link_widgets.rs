//! Live force-link roles and one-budget link/node compositions.
use std::sync::Arc;

use super::*;
use crate::network::{ForceEdge, ForceNodeState};
use crate::{
    ChartError, LayoutRect, MAX_MARKS, MAX_PLOT_WIDGET_SLOTS, Mark, NetworkNodeWidgetContext,
    NetworkNodeWidgetData, PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsData, PlotWidgetsLayout,
    ScatterNodeGeometry,
};

/// Current complete force nodes and edges shared across selected link roles.
/// `source_index` indexes `edges`, not nodes; its endpoints index `source`.
/// Geometry and paint come from the accepted host Mark. Values retain exact
/// original force-state bits, including unselected siblings.
#[derive(Clone, Debug)]
pub struct NetworkLinkWidgetData {
    pub source: Arc<Vec<ForceNodeState>>,
    pub edges: Arc<Vec<ForceEdge>>,
    pub source_index: usize,
    pub site: NetworkLinkMetadata,
    pub world_offset: [f64; 2],
    pub ticks: u64,
    pub alpha: f64,
}
fn number(a: f64, b: f64) -> bool {
    a == b && a.to_bits() == b.to_bits()
}
fn edge_equal(a: &ForceEdge, b: &ForceEdge) -> bool {
    a.source == b.source && a.target == b.target && number(a.value, b.value)
}
impl PartialEq for NetworkLinkWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.source, &other.source)
            || self.source.len() == other.source.len()
                && self
                    .source
                    .iter()
                    .zip(other.source.iter())
                    .all(|(a, b)| widgets::node_equal(a, b)))
            && (Arc::ptr_eq(&self.edges, &other.edges)
                || self.edges.len() == other.edges.len()
                    && self
                        .edges
                        .iter()
                        .zip(other.edges.iter())
                        .all(|(a, b)| edge_equal(a, b)))
            && self.source_index == other.source_index
            && self.site == other.site
            && number(self.site.value, other.site.value)
            && self.world_offset == other.world_offset
            && self.ticks == other.ticks
            && number(self.alpha, other.alpha)
    }
}
/// Borrowed current edge, its actual source/target states and original midpoint.
/// Placement references are synchronous and are never retained by this adapter.
#[derive(Clone, Copy, Debug)]
pub struct NetworkLinkWidgetContext<'a> {
    pub layout: &'a NetworkLinkLayout,
    pub simulation: &'a ForceSimulation,
    pub edge: &'a ForceEdge,
    pub link_index: usize,
    pub source: &'a ForceNodeState,
    pub target: &'a ForceNodeState,
    pub site: &'a NetworkLinkMetadata,
    pub mark: &'a Mark,
    pub world_offset: [f64; 2],
    pub source_position: [f64; 2],
    pub target_position: [f64; 2],
    pub position: [f64; 2],
}
/// One typed packet for source-ordered actual link and node roles. Each inner
/// payload keeps its own equality contract; no phase or presentation surrogate
/// stands in for the current force-state snapshot.
#[derive(Clone, Debug, PartialEq)]
pub enum NetworkWidgetData {
    Node(NetworkNodeWidgetData),
    Link(NetworkLinkWidgetData),
}
#[derive(Clone, Copy, Debug)]
pub enum NetworkWidgetContext<'a> {
    Node(NetworkNodeWidgetContext<'a>),
    Link(NetworkLinkWidgetContext<'a>),
}

fn preflight(layout: &NetworkLinkLayout, simulation: Option<&ForceSimulation>) -> Result<()> {
    let nodes = layout.node_layout();
    widgets::preflight(nodes, simulation)?;
    let links = layout.links();
    let scene = layout.scene();
    ensure(
        links
            .len()
            .checked_add(nodes.sites.len())
            .is_some_and(|n| n <= MAX_MARKS && n == scene.marks.len()),
        "Network link Widget role/mark count changed",
    )?;
    if let Some(simulation) = simulation {
        ensure(
            simulation.edges().len() == links.len(),
            "Network link Widget edge count changed",
        )?;
    }
    let by_id: BTreeMap<_, _> = nodes
        .sites
        .iter()
        .map(|n| (n.node_id.as_str(), n))
        .collect();
    for (index, site) in links.iter().enumerate() {
        let source = by_id
            .get(site.source_id.as_str())
            .ok_or_else(|| ChartError("Network link Widget source node is missing".into()))?;
        let target = by_id
            .get(site.target_id.as_str())
            .ok_or_else(|| ChartError("Network link Widget target node is missing".into()))?;
        ensure(
            site.link_index == index
                && site.mark_id == format!("force-link:{index}")
                && site.value.is_finite()
                && site.value >= 0.
                && site.source_position == source.position
                && site.target_position == target.position
                && site.position
                    == [
                        source.position[0] * 0.5 + target.position[0] * 0.5,
                        source.position[1] * 0.5 + target.position[1] * 0.5,
                    ],
            "Network link Widget endpoints, midpoint or identity changed",
        )?;
        if let Some(simulation) = simulation {
            let edge = &simulation.edges()[index];
            let a = simulation
                .nodes()
                .get(edge.source)
                .ok_or_else(|| ChartError("Network link Widget source index is missing".into()))?;
            let b = simulation
                .nodes()
                .get(edge.target)
                .ok_or_else(|| ChartError("Network link Widget target index is missing".into()))?;
            ensure(
                edge.value.is_finite()
                    && edge.value >= 0.
                    && number(edge.value, site.value)
                    && a.id == site.source_id
                    && b.id == site.target_id,
                "Network link Widget edge differs from its current simulation",
            )?;
        }
        let mark = &scene.marks[index];
        ensure(
            mark.id == site.mark_id
                && mark.series.is_empty()
                && mark.label == format!("{} → {}", site.source_id, site.target_id)
                && mark.value == site.value
                && mark.clip.is_none()
                && mark.clip_path.is_none(),
            "Network link Widget actual mark differs from its edge",
        )?;
        let checked = ScatterNodeGeometry {
            geometry: mark.geometry.clone(),
            fill: None,
            fill_paint: None,
            stroke: None,
            stroke_dash: None,
            opacity: 1.,
            anchor_commands: None,
        }
        .prepare_at(&site.mark_id, site.position, Color(0, 0, 0, 0), 200_000)?;
        ensure(
            site.bounds == checked.bounds,
            "Network link Widget bounds differ from actual geometry",
        )?;
    }
    for (index, site) in nodes.sites.iter().enumerate() {
        ensure(
            scene.marks[links.len() + index].id == site.mark_id,
            "Network Widget links must precede nodes in source drawing order",
        )?;
    }
    ensure(
        nodes
            .binding
            .sites
            .iter()
            .all(|site| site.mark_id.starts_with("force-node:")),
        "decorative Network links cannot invent canonical datum controls",
    )?;
    Ok(())
}

/// Clone the exact authored link/node Scene and retain only the existing
/// canonical node binding, with explicit PaintedGeometry. Links gain no fake
/// business label, datum control, typed hit site or node drag contour.
pub fn network_link_widget_frame(layout: &NetworkLinkLayout) -> Result<SceneInteraction> {
    preflight(layout, None)?;
    network_widget_frame(layout.node_layout())
}
fn resolve(placement: PlotWidgetPlacement, [x, y]: [f64; 2]) -> PlotWidgetPlacement {
    match placement {
        PlotWidgetPlacement::AtDatum { offset, measured } => {
            PlotWidgetPlacement::Rect(LayoutRect {
                x: x + offset[0],
                y: y + offset[1],
                width: measured.width,
                height: measured.height,
            })
        }
        placement => placement,
    }
}
fn slot(
    packet: &mut PlotWidgetsData<NetworkWidgetData>,
    id: &str,
    revision: u64,
    placement: PlotWidgetPlacement,
) -> Result<()> {
    ensure(
        packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
        "plot Widget slot budget exceeded",
    )?;
    packet.layout.slots.push(PlotWidgetSlot {
        mark_id: id.into(),
        revision,
        placement,
    });
    Ok(())
}
/// Borrow actual links first, then actual nodes, with one complete preflight
/// before any callback and a single 128-slot budget. None skips a role;
/// errors return no packet. Each selected role shares one complete current-node
/// snapshot, and selected links additionally share one complete edge snapshot.
/// Zero-valued/self/parallel/transparent/no-ink actual links remain eligible.
/// External callback effects cannot roll back. Hosts check placement and clip
/// support; source ink is retained until explicit composition replaces it.
///
/// Link identity is the existing `force-link:i` index. Reordering edges reuses
/// that index identity; this is not an invented stable business edge ID. Both
/// AtDatum variants resolve saved real origins to Rects, so custom offset Path
/// bounds never substitute for a link midpoint or a node's force origin. These
/// adapters neither tick forces nor implement React spring/animated props.
pub fn network_widgets(
    layout: &NetworkLinkLayout,
    simulation: &ForceSimulation,
    revision: u64,
    mut placement: impl FnMut(&NetworkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<NetworkWidgetData>> {
    preflight(layout, Some(simulation))?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut source = None;
    let mut edges = None;
    let nodes = layout.node_layout();
    for (index, site) in layout.links().iter().enumerate() {
        let edge = &simulation.edges()[index];
        let context = NetworkLinkWidgetContext {
            layout,
            simulation,
            edge,
            link_index: index,
            source: &simulation.nodes()[edge.source],
            target: &simulation.nodes()[edge.target],
            site,
            mark: &layout.scene().marks[index],
            world_offset: nodes.world_offset,
            source_position: site.source_position,
            target_position: site.target_position,
            position: site.position,
        };
        if let Some(placement) = placement(&NetworkWidgetContext::Link(context))? {
            slot(
                &mut packet,
                &site.mark_id,
                revision,
                resolve(placement, site.position),
            )?;
            packet.data.insert(
                site.mark_id.clone(),
                NetworkWidgetData::Link(NetworkLinkWidgetData {
                    source: source
                        .get_or_insert_with(|| Arc::new(simulation.nodes().to_vec()))
                        .clone(),
                    edges: edges
                        .get_or_insert_with(|| Arc::new(simulation.edges().to_vec()))
                        .clone(),
                    source_index: index,
                    site: site.clone(),
                    world_offset: nodes.world_offset,
                    ticks: simulation.total_ticks(),
                    alpha: simulation.alpha(),
                }),
            );
        }
    }
    for (index, site) in nodes.sites.iter().enumerate() {
        let context = NetworkNodeWidgetContext {
            layout: nodes,
            simulation,
            node: &simulation.nodes()[index],
            source_index: index,
            site,
            mark: &layout.scene().marks[layout.links().len() + index],
            world_offset: nodes.world_offset,
        };
        if let Some(placement) = placement(&NetworkWidgetContext::Node(context))? {
            slot(
                &mut packet,
                &site.mark_id,
                revision,
                resolve(placement, site.position),
            )?;
            packet.data.insert(
                site.mark_id.clone(),
                NetworkWidgetData::Node(NetworkNodeWidgetData {
                    source: source
                        .get_or_insert_with(|| Arc::new(simulation.nodes().to_vec()))
                        .clone(),
                    source_index: index,
                    site: site.clone(),
                    world_offset: nodes.world_offset,
                    ticks: simulation.total_ticks(),
                    alpha: simulation.alpha(),
                }),
            );
        }
    }
    Ok(packet)
}
/// Attach only link roles. The complete node/edge frame is validated first.
/// AtDatum uses the original real midpoint even when custom Path ink is offset.
/// Identity is edge-index based; source contains current force states rather
/// than arbitrary original caller RawDatum fields. Canonical controls remain
/// exactly those of `network_link_widget_frame`, which does not bind links.
pub fn network_link_widgets(
    layout: &NetworkLinkLayout,
    simulation: &ForceSimulation,
    revision: u64,
    mut placement: impl FnMut(&NetworkLinkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<NetworkLinkWidgetData>> {
    let packet = network_widgets(layout, simulation, revision, |context| match context {
        NetworkWidgetContext::Link(context) => placement(context),
        NetworkWidgetContext::Node(_) => Ok(None),
    })?;
    Ok(PlotWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(id, data)| {
                let NetworkWidgetData::Link(data) = data else {
                    unreachable!("only link placements selected")
                };
                (id, data)
            })
            .collect(),
    })
}
