//! Typed live Widgets for actual visual force-network nodes.
use std::sync::Arc;

use super::*;
use crate::network::{ForceNodeState, MAX_FORCE_NODES};
use crate::{
    LayoutRect, MAX_PLOT_WIDGET_SLOTS, PickingProfile, PlotWidgetPlacement, PlotWidgetSlot,
    PlotWidgetsData, PlotWidgetsLayout,
};

/// A snapshot of the accepted simulation, shared once by selected nodes.
/// This is current force state, not arbitrary caller RawDatum fields. Visual
/// geometry/paint comes from the accepted host frame's Mark; physics radius
/// remains independent of glyph size. Original state floats retain exact bits.
#[derive(Clone, Debug)]
pub struct NetworkNodeWidgetData {
    pub source: Arc<Vec<ForceNodeState>>,
    pub source_index: usize,
    pub site: NetworkGlyphSite,
    pub world_offset: [f64; 2],
    pub ticks: u64,
    pub alpha: f64,
}
fn number(a: f64, b: f64) -> bool {
    a == b && a.to_bits() == b.to_bits()
}
fn pair(a: [f64; 2], b: [f64; 2]) -> bool {
    a.into_iter().zip(b).all(|(a, b)| number(a, b))
}
pub(super) fn node_equal(a: &ForceNodeState, b: &ForceNodeState) -> bool {
    let ForceNodeState {
        id,
        label,
        radius,
        position,
        velocity,
        fixed,
    } = a;
    let ForceNodeState {
        id: other_id,
        label: other_label,
        radius: other_radius,
        position: other_position,
        velocity: other_velocity,
        fixed: other_fixed,
    } = b;
    id == other_id
        && label == other_label
        && number(*radius, *other_radius)
        && pair(*position, *other_position)
        && pair(*velocity, *other_velocity)
        && match (fixed, other_fixed) {
            (None, None) => true,
            (Some(a), Some(b)) => pair(*a, *b),
            _ => false,
        }
}
impl PartialEq for NetworkNodeWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.source, &other.source)
            || self.source.len() == other.source.len()
                && self
                    .source
                    .iter()
                    .zip(other.source.iter())
                    .all(|(a, b)| node_equal(a, b)))
            && self.source_index == other.source_index
            && self.site == other.site
            && self.world_offset == other.world_offset
            && self.ticks == other.ticks
            && number(self.alpha, other.alpha)
    }
}
/// Borrowed placement context, invoked synchronously in simulation node order.
#[derive(Clone, Copy, Debug)]
pub struct NetworkNodeWidgetContext<'a> {
    pub layout: &'a NetworkGlyphLayout,
    pub simulation: &'a ForceSimulation,
    pub node: &'a ForceNodeState,
    pub source_index: usize,
    pub site: &'a NetworkGlyphSite,
    pub mark: &'a crate::Mark,
    pub world_offset: [f64; 2],
}
pub(super) fn preflight(
    layout: &NetworkGlyphLayout,
    simulation: Option<&ForceSimulation>,
) -> Result<()> {
    layout.scene.validate()?;
    ensure(
        layout.sites.len() <= MAX_FORCE_NODES,
        "Network Widget node budget exceeded",
    )?;
    ensure(
        layout
            .world_offset
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 1e6),
        "invalid Network Widget world offset",
    )?;
    layout.binding.validate(&layout.scene)?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut ids = BTreeSet::new();
    if let Some(simulation) = simulation {
        ensure(
            simulation.nodes().len() == layout.sites.len(),
            "Network Widget simulation node count changed",
        )?;
        simulation.options().validate()?;
    }
    for (index, site) in layout.sites.iter().enumerate() {
        ensure(
            ids.insert(site.node_id.as_str())
                && !site.node_id.trim().is_empty()
                && site.node_id.len() <= 512
                && site.mark_id == format!("force-node:{}", site.node_id),
            "invalid or duplicate Network Widget node identity",
        )?;
        ensure(
            site.world_position
                .iter()
                .chain(&site.position)
                .all(|v| v.is_finite() && v.abs() <= 1e6)
                && site.physics_radius.is_finite()
                && (0.000001..=10000.).contains(&site.physics_radius)
                && site.position
                    == [
                        site.world_position[0] + layout.world_offset[0],
                        site.world_position[1] + layout.world_offset[1],
                    ],
            "Network Widget world/Scene position or physics radius differs",
        )?;
        site.bounds.validate()?;
        let mark = marks
            .get(site.mark_id.as_str())
            .ok_or_else(|| crate::ChartError("Network Widget node mark is missing".into()))?;
        ensure(
            mark.series == site.node_id && mark.value == 0.,
            "Network Widget node mark identity or compatibility scalar changed",
        )?;
        if let Some(label) = site.label_index {
            ensure(
                layout
                    .scene
                    .labels
                    .get(label)
                    .is_some_and(|label| label.x == site.position[0])
                    && layout.scene.label_series.get(&label) == Some(&site.node_id),
                "Network Widget owned label differs from its node",
            )?;
        }
        if let Some(simulation) = simulation {
            let node = &simulation.nodes()[index];
            ensure(
                node.id == site.node_id
                    && node.label == mark.label
                    && node.radius == site.physics_radius
                    && pair(node.position, site.world_position),
                "Network Widget simulation differs from current glyph frame",
            )?;
            ensure(
                node.velocity
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 10000.)
                    && node.fixed.is_none_or(|p| {
                        p.iter().all(|v| {
                            v.is_finite() && v.abs() <= crate::network::MAX_FORCE_COORDINATE
                        })
                    }),
                "invalid Network Widget force-state snapshot",
            )?;
        }
    }
    ensure(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("force-node:"))
            .count()
            == layout.sites.len(),
        "orphan Network Widget source node",
    )?;
    Ok(())
}
/// Clone the exact authored Scene with its existing canonical node binding and
/// explicit PaintedGeometry. Empty-label/unpainted nodes do not acquire fake
/// business labels or canonical datum controls; typed packet attachment is
/// independent of that eligibility. Network coordinates span the full Scene.
pub fn network_widget_frame(layout: &NetworkGlyphLayout) -> Result<SceneInteraction> {
    preflight(layout, None)?;
    let mut binding = layout.binding.clone();
    binding.picking_profile = PickingProfile::PaintedGeometry;
    let frame = SceneInteraction {
        scene: layout.scene.clone(),
        interaction: Some(binding),
        plot: None,
    };
    frame.validate()?;
    crate::surface::SurfaceModel::new_with_interaction(
        frame.scene.clone(),
        crate::surface::SurfaceOptions::default(),
        frame.interaction.clone(),
    )?;
    Ok(frame)
}
/// Build typed roles for every actual node, including empty-label/no-ink and
/// zero visual-size nodes. All layout/source checks precede the first callback.
/// Returning None skips a role; failure returns no packet. External callback
/// side effects do not roll back. One complete current force-state snapshot is
/// shared by selected nodes. Tick count/heat and all source float bits are data.
///
/// AtDatum placements are resolved to explicit rectangles using the saved exact
/// glyph origin. This also supports decorative Paths without inventing a datum
/// label/site or guessing the Path bounds center. Rebuild the packet for each
/// accepted force frame; a stable revision retains component state. Scene export
/// contains authored source ink, not live child Widgets. Hosts check placement
/// and clip support. This adapter neither ticks nor mutates the simulation.
pub fn network_node_widgets(
    layout: &NetworkGlyphLayout,
    simulation: &ForceSimulation,
    revision: u64,
    mut placement: impl FnMut(&NetworkNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<NetworkNodeWidgetData>> {
    preflight(layout, Some(simulation))?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = None;
    for (source_index, site) in layout.sites.iter().enumerate() {
        let ctx = NetworkNodeWidgetContext {
            layout,
            simulation,
            node: &simulation.nodes()[source_index],
            source_index,
            site,
            mark: marks[site.mark_id.as_str()],
            world_offset: layout.world_offset,
        };
        if let Some(placement) = placement(&ctx)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            let placement = match placement {
                PlotWidgetPlacement::AtDatum { offset, measured } => {
                    PlotWidgetPlacement::Rect(LayoutRect {
                        x: site.position[0] + offset[0],
                        y: site.position[1] + offset[1],
                        width: measured.width,
                        height: measured.height,
                    })
                }
                rect => rect,
            };
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: site.mark_id.clone(),
                revision,
                placement,
            });
            packet.data.insert(
                site.mark_id.clone(),
                NetworkNodeWidgetData {
                    source: owned
                        .get_or_insert_with(|| Arc::new(simulation.nodes().to_vec()))
                        .clone(),
                    source_index,
                    site: site.clone(),
                    world_offset: layout.world_offset,
                    ticks: simulation.total_ticks(),
                    alpha: simulation.alpha(),
                },
            );
        }
    }
    Ok(packet)
}
