//! Borrowed link rendering after force layout, as in Nivo's linkComponent and
//! renderLink hooks. Custom paint never feeds positions back into simulation.
use super::*;
use crate::network::{ForceEdge, ForceNodeState};
use crate::{FillPaint, ScatterNodeGeometry, StrokeDash};

/// One bounded mark, with a logical origin at the original link midpoint.
/// Use a Path for offset or curved ink. Native links remain decorative; use
/// the explicit relation entry point to expose edges as business data.
pub type NetworkLinkGeometry = ScatterNodeGeometry;

/// Original source and target states and the resolved original link mark.
/// Scene coordinates include the world origin translation exactly once.
#[derive(Clone, Copy, Debug)]
pub struct NetworkLinkContext<'a> {
    pub edge: &'a ForceEdge,
    pub link_index: usize,
    pub source: &'a ForceNodeState,
    pub target: &'a ForceNodeState,
    pub mark_id: &'a str,
    pub source_position: [f64; 2],
    pub target_position: [f64; 2],
    pub position: [f64; 2],
    pub world_offset: [f64; 2],
    pub plot: ClipRect,
    pub geometry: &'a Geometry,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<&'a StrokeDash>,
    pub opacity: f64,
}
impl NetworkLinkContext<'_> {
    pub fn inherit(&self, geometry: Geometry) -> NetworkLinkGeometry {
        NetworkLinkGeometry {
            geometry,
            fill: self.fill,
            fill_paint: self.fill_paint.cloned(),
            stroke: self.stroke,
            stroke_dash: self.stroke_dash.cloned(),
            opacity: self.opacity,
            anchor_commands: None,
        }
    }
}

/// Immutable source identity and original endpoints, independent of custom ink.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkLinkMetadata {
    pub link_index: usize,
    pub mark_id: String,
    pub source_id: String,
    pub target_id: String,
    pub value: f64,
    pub source_position: [f64; 2],
    pub target_position: [f64; 2],
    pub position: [f64; 2],
    /// Custom geometry bounds, excluding stroke extension.
    pub bounds: ClipRect,
}

/// A coherent glyph/node frame and caller-rendered links. Read-only access
/// keeps node dragging, typed node sites and source link metadata consistent.
#[derive(Clone, Debug)]
pub struct NetworkLinkLayout {
    nodes: NetworkGlyphLayout,
    links: Vec<NetworkLinkMetadata>,
}
impl NetworkLinkLayout {
    pub fn scene(&self) -> &Scene {
        self.nodes.scene()
    }
    pub fn node_layout(&self) -> &NetworkGlyphLayout {
        &self.nodes
    }
    pub fn links(&self) -> &[NetworkLinkMetadata] {
        &self.links
    }
    pub fn interaction_binding(&self) -> &InteractionBinding {
        self.nodes.interaction_binding()
    }
    /// Opt into painted node ownership for semantic/raster hosts. Custom links
    /// remain visible decoration, not node controls or drag targets. Host paint
    /// support (for example semantic Graphic gradients) is a separate check.
    pub fn into_painted_scene_interaction(self) -> Result<SceneInteraction> {
        let mut frame: SceneInteraction = self.into();
        frame
            .interaction
            .as_mut()
            .expect("network node binding")
            .picking_profile = crate::PickingProfile::PaintedGeometry;
        frame.validate()?;
        crate::surface::SurfaceModel::new_with_interaction(
            frame.scene.clone(),
            crate::surface::SurfaceOptions {
                plot: frame.plot,
                ..Default::default()
            },
            frame.interaction.clone(),
        )?;
        Ok(frame)
    }
}
impl From<NetworkLinkLayout> for SceneInteraction {
    fn from(layout: NetworkLinkLayout) -> Self {
        layout.nodes.into()
    }
}

fn customize_links(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    scene: &mut Scene,
    factory: &mut (impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + ?Sized),
) -> Result<Vec<NetworkLinkMetadata>> {
    let offset = [
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ];
    let command_count = |g: &Geometry| match g {
        Geometry::Path(path) => path.len(),
        _ => 0,
    };
    let mut commands = scene.marks[simulation.edges().len()..]
        .iter()
        .map(|m| command_count(&m.geometry))
        .sum::<usize>();
    let mut stops = scene.marks[simulation.edges().len()..]
        .iter()
        .map(|m| m.fill_paint.as_ref().map_or(0, |paint| paint.stops().len()))
        .sum::<usize>();
    let mut links = Vec::with_capacity(simulation.edges().len());
    for (index, edge) in simulation.edges().iter().enumerate() {
        let source = &simulation.nodes()[edge.source];
        let target = &simulation.nodes()[edge.target];
        let source_position = [
            source.position[0] + offset[0],
            source.position[1] + offset[1],
        ];
        let target_position = [
            target.position[0] + offset[0],
            target.position[1] + offset[1],
        ];
        let position = [
            source_position[0] * 0.5 + target_position[0] * 0.5,
            source_position[1] * 0.5 + target_position[1] * 0.5,
        ];
        let original = &scene.marks[index];
        let replacement = factory(&NetworkLinkContext {
            edge,
            link_index: index,
            source,
            target,
            mark_id: &original.id,
            source_position,
            target_position,
            position,
            world_offset: offset,
            plot: ClipRect::plot(c),
            geometry: &original.geometry,
            fill: original.fill,
            fill_paint: original.fill_paint.as_ref(),
            stroke: original.stroke,
            stroke_dash: original.stroke_dash.as_ref(),
            opacity: original.opacity,
        })?;
        if let Some(paint) = &replacement.fill_paint {
            paint.validate()?;
        }
        let prepared = replacement.prepare_at(
            &original.id,
            position,
            c.theme.grid,
            200_000usize.saturating_sub(commands),
        )?;
        commands = commands
            .checked_add(command_count(&prepared.mark.geometry))
            .ok_or_else(|| {
                crate::ChartError("custom network link command count overflow".into())
            })?;
        stops = stops
            .checked_add(
                prepared
                    .mark
                    .fill_paint
                    .as_ref()
                    .map_or(0, |paint| paint.stops().len()),
            )
            .ok_or_else(|| crate::ChartError("custom network link paint count overflow".into()))?;
        ensure(
            commands <= 200_000,
            "custom network link path budget exceeded",
        )?;
        ensure(
            stops <= crate::MAX_SCENE_GRADIENT_STOPS,
            "custom network link paint budget exceeded",
        )?;
        let mark = &mut scene.marks[index];
        mark.geometry = prepared.mark.geometry;
        mark.fill = prepared.mark.fill;
        mark.fill_paint = prepared.mark.fill_paint;
        mark.stroke = prepared.mark.stroke;
        mark.stroke_dash = prepared.mark.stroke_dash;
        mark.opacity = prepared.mark.opacity;
        links.push(NetworkLinkMetadata {
            link_index: index,
            mark_id: mark.id.clone(),
            source_id: source.id.clone(),
            target_id: target.id.clone(),
            value: edge.value,
            source_position,
            target_position,
            position,
            bounds: prepared.bounds,
        });
    }
    scene.validate()?;
    Ok(links)
}

/// Invoke a borrowed factory once per link, in simulation edge order, including
/// self, parallel and zero-valued links. Existing nodes, physics radii, labels,
/// identities and draw order remain unchanged. The closure is not retained.
/// Rejected geometry or callback errors discard the complete candidate; caller
/// callback side effects are not reversible. Inheriting all geometry and paint
/// preserves the original glyph Scene and node interaction binding.
pub fn network_glyph_layout_with_link_factory(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    glyphs: &NetworkGlyphOptions,
    factory: &mut (impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + ?Sized),
) -> Result<NetworkLinkLayout> {
    let mut nodes = network_glyph_layout(c, simulation, glyphs)?;
    let links = customize_links(c, simulation, &mut nodes.scene, factory)?;
    nodes.binding.validate(&nodes.scene)?;
    Ok(NetworkLinkLayout { nodes, links })
}

/// Compose custom link ink with explicit stable edge IDs and relation ownership.
/// Relation highlighting starts from the customized authored Scene each time.
pub fn network_glyph_relation_layout_with_link_factory(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    glyphs: &NetworkGlyphOptions,
    edge_ids: &[String],
    options: &RelationOptions,
    factory: &mut (impl FnMut(&NetworkLinkContext<'_>) -> Result<NetworkLinkGeometry> + ?Sized),
) -> Result<RelationLayout> {
    let base = network_glyph_relation_layout(c, simulation, glyphs, edge_ids, options)?;
    let mut scene = base.authored_scene().clone();
    customize_links(c, simulation, &mut scene, factory)?;
    RelationLayout::new(scene, base.graph().clone(), options.clone())
}
