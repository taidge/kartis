//! Visual force-network nodes. Nivo's `packages/network/src/hooks.ts` applies
//! node style after simulation; SVG and Canvas use half the visual node size
//! as their circle radius. These options likewise leave force radii, link
//! endpoints, and the simulation's coordinates independent of node paint.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::GraphicStroke;

use crate::network::{ForceSimulation, NetworkDragController, NetworkDragMapping};
use crate::scene::ensure;
use crate::{
    ChartConfig, ClipRect, Color, Geometry, GlyphOptions, GlyphPaintMode, InteractionAnchor,
    InteractionBinding, InteractionSite, RelationLayout, RelationOptions, Result, Scene,
    SceneInteraction, glyph_geometry, glyph_mark, network_relation_layout,
};

/// Visual geometry and paint for one stable raw simulation node ID.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkNodeGlyph {
    pub glyph: GlyphOptions,
    /// None inherits the node palette for a filled glyph. Stroke-only glyphs
    /// require an explicit visible stroke and do not accept a fill color.
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NetworkGlyphOptions {
    /// Unlisted nodes keep their original circle and label placement.
    pub nodes: BTreeMap<String, NetworkNodeGlyph>,
}

/// Node identity and positions in both the simulation and authored Scene.
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkGlyphSite {
    pub node_id: String,
    pub mark_id: String,
    pub world_position: [f64; 2],
    /// Original glyph origin in Scene coordinates, before the camera.
    pub position: [f64; 2],
    pub physics_radius: f64,
    /// Geometry bounds, excluding stroke ink.
    pub bounds: ClipRect,
    pub label_index: Option<usize>,
}

/// A glyph frame whose Scene and interaction metadata describe the same nodes.
#[derive(Clone, Debug)]
pub struct NetworkGlyphLayout {
    scene: Scene,
    sites: Vec<NetworkGlyphSite>,
    binding: InteractionBinding,
    world_offset: [f64; 2],
}

impl NetworkGlyphLayout {
    pub fn scene(&self) -> &Scene {
        &self.scene
    }

    pub fn sites(&self) -> &[NetworkGlyphSite] {
        &self.sites
    }

    /// Nodes with paint and a nonempty label receive typed datum bindings.
    /// An unlabeled node can still be dragged by its visible shape.
    pub fn interaction_binding(&self) -> &InteractionBinding {
        &self.binding
    }

    /// Pick the topmost visible node through the same CSS and camera mapping
    /// as network dragging. The force radius and legacy circle pick padding
    /// do not replace the glyph contour. Undashed strokes include the shared
    /// Scene hit tolerance. Links never participate in drag pick.
    pub fn pick_node(
        &self,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
        hidden: &BTreeSet<String>,
    ) -> Result<Option<&NetworkGlyphSite>> {
        ensure(
            mapping.world_offset == self.world_offset,
            "network glyph pick mapping does not match the layout",
        )?;
        Ok(
            pick_network_glyph_node(&self.scene, client, mapping, hidden)?
                .and_then(|id| self.sites.iter().find(|site| site.node_id == id)),
        )
    }
}

fn visible_color(mark: &crate::Mark) -> Option<Color> {
    mark.fill.filter(|color| color.3 > 0).or(mark
        .stroke
        .map(|stroke| stroke.color)
        .filter(|color| color.3 > 0))
}

pub(crate) fn pick_network_glyph_node<'a>(
    scene: &'a Scene,
    client: [f64; 2],
    mapping: &NetworkDragMapping,
    hidden: &BTreeSet<String>,
) -> Result<Option<&'a str>> {
    ensure(
        mapping.scene_size == [scene.width, scene.height],
        "network glyph pick mapping does not match the layout",
    )?;
    let pixel = mapping.client_to_scene(client)?;
    if pixel[0] < 0. || pixel[1] < 0. || pixel[0] > scene.width || pixel[1] > scene.height {
        return Ok(None);
    }
    let (x, y) = mapping.view.unproject((pixel[0], pixel[1]))?;
    for mark in scene.paint_marks().rev() {
        let Some(id) = mark.id.strip_prefix("force-node:") else {
            continue;
        };
        if id != mark.series
            || hidden.contains(id)
            || mark.opacity == 0.
            || visible_color(mark).is_none()
        {
            continue;
        }
        // Reuse the actual zero-size cap footprint used by the renderers.
        let ink = crate::render::paint_mark(mark)?;
        if crate::scene::contains(&ink, x, y) {
            return Ok(Some(id));
        }
    }
    Ok(None)
}

impl From<NetworkGlyphLayout> for SceneInteraction {
    fn from(layout: NetworkGlyphLayout) -> Self {
        Self {
            scene: layout.scene,
            interaction: Some(layout.binding),
            plot: None,
        }
    }
}

fn decorate(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    options: &NetworkGlyphOptions,
    mut scene: Scene,
) -> Result<NetworkGlyphLayout> {
    let nodes = simulation.nodes();
    ensure(
        options.nodes.len() <= nodes.len()
            && options.nodes.keys().all(|id| simulation.node(id).is_some()),
        "network glyph styles refer to unknown nodes or exceed the node budget",
    )?;
    let mut sites = Vec::with_capacity(nodes.len());
    let mut binding = InteractionBinding::default();
    let first_node = simulation.edges().len();
    for (index, node) in nodes.iter().enumerate() {
        let mark = &mut scene.marks[first_node + index];
        // Both simulation and relation adapters retain node order and only
        // rename edges. Reuse the actual center authored by ForceSimulation.
        let Geometry::Circle { x, y, radius } = mark.geometry else {
            return Err(crate::ChartError(
                "network base node is not a circle".into(),
            ));
        };
        let position = [x, y];
        let label_index = simulation.options().labels.then_some(index);
        let mut bounds = ClipRect {
            x: x - radius,
            y: y - radius,
            width: radius * 2.,
            height: radius * 2.,
        };
        if let Some(style) = options.nodes.get(&node.id) {
            let glyph = glyph_geometry(position, &style.glyph)?;
            let fill = match glyph.paint_mode {
                GlyphPaintMode::Fill => style.fill.or(mark.fill),
                GlyphPaintMode::Stroke => {
                    ensure(
                        style.fill.is_none() && style.stroke.is_some_and(|s| s.color.3 > 0),
                        "stroke-only network glyph requires visible stroke and no fill",
                    )?;
                    None
                }
            };
            let painted = glyph_mark(&mark.id, &glyph, fill, style.stroke)?;
            mark.geometry = painted.geometry;
            mark.fill = painted.fill.filter(|color| color.3 > 0);
            mark.stroke = painted.stroke.filter(|stroke| stroke.color.3 > 0);
            bounds = glyph.bounds;
            if let Some(index) = label_index {
                // A circle has no miter. For polygonal glyphs the shared
                // renderer's miter limit of four bounds the stroke extension.
                let stroke_extent = mark.stroke.map_or(0., |s| {
                    if matches!(mark.geometry, Geometry::Circle { .. }) || glyph.size == 0. {
                        s.width / 2.
                    } else {
                        2. * s.width
                    }
                });
                scene.labels[index].y = bounds.y - stroke_extent - 5.;
            }
            if !mark.label.is_empty()
                && visible_color(mark).is_some()
                && !glyph.anchor_indices.is_empty()
            {
                binding.anchors.insert(
                    mark.id.clone(),
                    InteractionAnchor::path_points(
                        &mark.geometry,
                        position,
                        &glyph.anchor_indices,
                    )?,
                );
            }
        }
        if !mark.label.is_empty()
            && let Some(color) = visible_color(mark)
        {
            binding.sites.push(InteractionSite {
                mark_id: mark.id.clone(),
                series: node.id.clone(),
                label: mark.label.clone(),
                value: None,
                formatted_value: None,
                color,
                position,
            });
        }
        sites.push(NetworkGlyphSite {
            node_id: node.id.clone(),
            mark_id: mark.id.clone(),
            world_position: node.position,
            position,
            physics_radius: node.radius,
            bounds,
            label_index,
        });
    }
    binding.validate(&scene)?;
    Ok(NetworkGlyphLayout {
        scene,
        sites,
        binding,
        world_offset: [
            c.margin.left + c.inner_width() / 2.,
            c.margin.top + c.inner_height() / 2.,
        ],
    })
}

pub fn network_glyph_layout(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    options: &NetworkGlyphOptions,
) -> Result<NetworkGlyphLayout> {
    decorate(c, simulation, options, simulation.scene(c)?)
}

/// Compose the same actual glyphs with network relation selection. Raw node
/// IDs, explicit edge IDs, label ownership, and link endpoints are retained.
pub fn network_glyph_relation_layout(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    glyphs: &NetworkGlyphOptions,
    edge_ids: &[String],
    options: &RelationOptions,
) -> Result<RelationLayout> {
    let relation = network_relation_layout(c, simulation, edge_ids, options)?;
    let frame = decorate(c, simulation, glyphs, relation.authored_scene().clone())?;
    RelationLayout::new(frame.scene, relation.graph().clone(), options.clone())
}

impl ForceSimulation {
    pub fn scene_with_glyphs(
        &self,
        c: &ChartConfig,
        options: &NetworkGlyphOptions,
    ) -> Result<NetworkGlyphLayout> {
        network_glyph_layout(c, self, options)
    }
}

impl NetworkDragController {
    /// Begin from the actual current glyph hit, then use the existing force
    /// drag session for fixedness, heat, grab offset, movement and rebase.
    pub fn begin_with_glyphs(
        &mut self,
        c: &ChartConfig,
        glyphs: &NetworkGlyphOptions,
        pointer_id: i32,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
        hidden: &BTreeSet<String>,
    ) -> Result<bool> {
        let frame = self.simulation().scene_with_glyphs(c, glyphs)?;
        let Some(site) = frame.pick_node(client, mapping, hidden)? else {
            return Ok(false);
        };
        self.begin(pointer_id, &site.node_id, client, mapping)?;
        Ok(true)
    }
}
