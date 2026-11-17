//! Typed render customization, after the ordinary packing pass.
//!
//! Nivo 0.99's swarmplot `types.ts` exposes SVG `circleComponent` and Canvas
//! `renderCircle`. `hooks.ts` computes nodes before `Circles.tsx` or
//! `SwarmPlotCanvas.tsx` invokes those renderers. Likewise custom ink here does
//! not feed back into the collision solver.
use super::*;
/// The checked single-mark geometry and paint contract shared with Scatter.
pub use crate::ScatterNodeGeometry as SwarmNodeGeometry;

/// Original source and immutable, already-packed node supplied to a factory.
#[derive(Clone, Copy, Debug)]
pub struct SwarmNodeContext<'a> {
    pub datum: &'a SwarmPoint,
    pub group: &'a SwarmGroupLayout,
    pub site: &'a SwarmSite,
    pub source_index: usize,
    pub group_index: usize,
    /// Index in the ordinary packed/drawing order, not source input order.
    pub site_index: usize,
    pub position: [f64; 2],
    /// Original shape size, in the configured shape's unit.
    pub size: f64,
    /// Conservative collision disc used by the completed ordinary packing.
    /// This is not recomputed from custom ink.
    pub ink_radius: f64,
    pub plot: ClipRect,
    pub value_scale: &'a ResolvedContinuousScale,
    /// The inherited shape before replacement.
    pub geometry: &'a Geometry,
    pub color: Color,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<&'a StrokeDash>,
    pub opacity: f64,
}
impl SwarmNodeContext<'_> {
    /// Replace geometry while retaining the complete inherited paint.
    pub fn inherit(&self, geometry: Geometry) -> SwarmNodeGeometry {
        SwarmNodeGeometry {
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

impl SwarmLayout {
    /// Opt into the painted-ink Geometry profile required by Glory
    /// `chart_layout`, validating its actual ink before returning a frame.
    /// This profile does not admit Nearest or slice picking. The original
    /// `From<SwarmLayout>` conversion retains legacy Geometry/Nearest behavior.
    /// Semantic Graphic paint and per-mark command limits still apply.
    pub fn into_painted_scene_interaction(self) -> Result<SceneInteraction> {
        let mut frame = SceneInteraction::from(self);
        frame.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
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

/// Build custom nodes after validating and completing the ordinary packing.
///
/// Callbacks run once per original site in packed drawing order. Hidden groups
/// and nodes culled by `SwarmValueFit::Clip` never call the factory. Zero-sized,
/// transparent and zero-opacity nodes retain their ordinary logical sites and
/// do call it. The callback is borrowed only for this build and is not retained.
///
/// Custom geometry does not change collision discs, source values, scale,
/// source indices, group bands or legend colors. Consequently custom ink may
/// overlap its neighbors or be clipped at the original plot boundary. Exact
/// picking uses the new geometry; nearest picking retains the original packed
/// position, subject to the host's ordinary opacity/paint eligibility rules.
/// Paths can intentionally offset their ink from that position.
///
/// Inheriting the exact original geometry preserves the complete default frame,
/// including its original animation anchor. A callback error or invalid geometry
/// rejects the entire candidate; callback-owned external effects are not undone.
pub fn swarm_layout_with_node_factory(
    c: &ChartConfig,
    data: &[SwarmPoint],
    o: &SwarmLayoutOptions,
    mut factory: impl FnMut(&SwarmNodeContext<'_>) -> Result<SwarmNodeGeometry>,
) -> Result<SwarmLayout> {
    let mut layout = swarm_layout(c, data, o)?;
    let owned: BTreeSet<_> = layout.sites.iter().map(|s| s.mark_index).collect();
    let commands = |geometry: &Geometry| match geometry {
        Geometry::Path(path) => path.len(),
        _ => 0,
    };
    let stops = |paint: Option<&FillPaint>| paint.map_or(0, |p| p.stops().len());
    let mut count: usize = layout
        .scene
        .marks
        .iter()
        .enumerate()
        .filter(|(i, _)| !owned.contains(i))
        .map(|(_, m)| commands(&m.geometry))
        .sum();
    let mut paint_stops: usize = layout
        .scene
        .marks
        .iter()
        .enumerate()
        .filter(|(i, _)| !owned.contains(i))
        .map(|(_, m)| stops(m.fill_paint.as_ref()))
        .sum();
    for site_index in 0..layout.sites.len() {
        let site = &layout.sites[site_index];
        let original = &layout.scene.marks[site.mark_index];
        let context = SwarmNodeContext {
            datum: &data[site.source_index],
            group: &layout.groups[site.group_index],
            site,
            source_index: site.source_index,
            group_index: site.group_index,
            site_index,
            position: site.position,
            size: site.size,
            ink_radius: site.ink_radius,
            plot: layout.plot,
            value_scale: &layout.value_scale,
            geometry: &original.geometry,
            color: site.color,
            fill: original.fill,
            fill_paint: original.fill_paint.as_ref(),
            stroke: original.stroke,
            stroke_dash: original.stroke_dash.as_ref(),
            opacity: original.opacity,
        };
        let custom = factory(&context)?;
        let inherited = custom == context.inherit(context.geometry.clone());
        count = count
            .checked_add(commands(&custom.geometry))
            .ok_or_else(|| ChartError("custom swarm command count overflow".into()))?;
        ensure(count <= 200_000, "custom swarm geometry budget exceeded")?;
        if let Some(paint) = &custom.fill_paint {
            paint.validate()?;
        }
        paint_stops = paint_stops
            .checked_add(stops(custom.fill_paint.as_ref()))
            .ok_or_else(|| ChartError("custom swarm paint budget overflow".into()))?;
        ensure(
            paint_stops <= MAX_SCENE_GRADIENT_STOPS,
            "custom swarm paint budget exceeded",
        )?;
        if inherited {
            continue;
        }
        let prepared = custom.prepare_at(&site.mark_id, site.position, site.color, 200_000)?;
        let mut mark = prepared.mark;
        mark.series = original.series.clone();
        mark.label = original.label.clone();
        mark.value = original.value;
        mark.clip = original.clip;
        mark.clip_path = original.clip_path.clone();
        layout.anchors.remove(&site.mark_id);
        if let Some(anchor) = prepared.anchor {
            layout.anchors.insert(site.mark_id.clone(), anchor);
        }
        layout.scene.marks[site.mark_index] = mark;
        layout.sites[site_index].color = prepared.color;
    }
    layout.scene.validate()?;
    layout.interaction_binding().validate(&layout.scene)?;
    Ok(layout)
}
