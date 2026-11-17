//! Borrowed replacement ink for actual Bullet range, measure and marker marks.
use std::collections::{HashMap, HashSet};

use glory::render::PathCommand;

use super::*;
use crate::{FillPaint, ScatterNodeGeometry, StrokeDash};

/// Original source, layout and paint for one actually drawn Bullet part.
/// Coordinates are full Scene pixels. Rows and parts remain the original
/// snapshots throughout callback traversal, before any metadata replacements.
#[derive(Clone, Copy, Debug)]
pub struct BulletPartContext<'a> {
    pub datum: &'a BulletDatum,
    pub source_row_index: usize,
    pub row: &'a BulletRowLayout,
    pub row_index: usize,
    pub part: &'a BulletPart,
    /// Zero-based rank within this kind, including omitted zero/clipped parts.
    /// Range/measure rank is sorted endpoint order; marker rank is source order.
    pub rank: usize,
    pub geometry: &'a Geometry,
    /// Center of the original visible part, retained for nearest picking.
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<&'a StrokeDash>,
    pub opacity: f64,
}
impl BulletPartContext<'_> {
    /// Replace ink while retaining the original configured paint.
    pub fn inherit(&self, geometry: Geometry) -> ScatterNodeGeometry {
        ScatterNodeGeometry {
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

/// Custom ink plus explicit data sites; the existing BulletLayout is unchanged.
#[derive(Clone, Debug, PartialEq)]
pub struct CustomBulletLayout {
    pub layout: BulletLayout,
    pub interaction: InteractionBinding,
    pub plot: ClipRect,
}
/// The resolved scales used by the customized value-scale entry.
#[derive(Clone, Debug)]
pub struct CustomScaledBulletLayout {
    pub layout: CustomBulletLayout,
    pub shared_value_scale: Option<ResolvedContinuousScale>,
    pub row_value_scales: BTreeMap<String, ResolvedContinuousScale>,
}

impl CustomBulletLayout {
    /// Retain legacy geometry/nearest interaction for Surface or browser hosts.
    pub fn into_scene_interaction(self) -> SceneInteraction {
        SceneInteraction {
            scene: self.layout.scene,
            interaction: Some(self.interaction),
            plot: Some(self.plot),
        }
    }

    /// Opt into solid painted-ink picking, including the profile required by
    /// Glory `chart_layout`. Reject unsupported painted geometry before returning
    /// a candidate. This profile admits Geometry picking, not Nearest or slices.
    /// Semantic Graphic/text limits still apply when mounting a Glory chart.
    pub fn into_painted_scene_interaction(self) -> Result<SceneInteraction> {
        let mut frame = self.into_scene_interaction();
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

impl CustomScaledBulletLayout {
    /// Consume the layout for a host; keep a clone when resolved scales are needed.
    pub fn into_scene_interaction(self) -> SceneInteraction {
        self.layout.into_scene_interaction()
    }

    /// Opt into the same preflighted solid painted-ink profile as the plain entry.
    pub fn into_painted_scene_interaction(self) -> Result<SceneInteraction> {
        self.layout.into_painted_scene_interaction()
    }
}

/// Replace each actual data mark once, in visible-row / kind / rank order.
/// Axes, omitted zero/clipped parts and hidden rows do not invoke the callback.
/// Synthetic domain-ending range marks do. Borrowed callbacks are not retained.
///
/// Identity inheritance preserves the entire inner layout. Replacement ink
/// updates only paint and part.geometry; identity, raw intervals, track clips,
/// labels and legends retain the original layout. Rect/Circle must remain
/// centered on position; use Path for offset ink. Exact picking follows ink,
/// nearest picking follows the original part center. Failure returns no partial
/// layout; application-owned callback side effects cannot be rolled back.
pub fn bullet_layout_with_part_factory(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
    factory: &mut impl FnMut(&BulletPartContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<CustomBulletLayout> {
    customize(c, data, o, bullet_layout(c, data, o)?, factory)
}

/// Compose custom part ink with the existing resolved value-scale profile.
pub fn bullet_layout_with_scale_options_and_part_factory(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
    scale: &ContinuousScaleOptions,
    factory: &mut impl FnMut(&BulletPartContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<CustomScaledBulletLayout> {
    let scaled = bullet_layout_with_scale_options(c, data, o, scale)?;
    Ok(CustomScaledBulletLayout {
        layout: customize(c, data, o, scaled.layout, factory)?,
        shared_value_scale: scaled.shared_value_scale,
        row_value_scales: scaled.row_value_scales,
    })
}

fn original_position(mark: &Mark) -> Result<[f64; 2]> {
    if let Some(position) = crate::crosshair::mark_center(mark) {
        return Ok(position);
    }
    if let Geometry::Path(path) = &mark.geometry
        && let [PathCommand::Move(a), PathCommand::Line(b)] = path.as_slice()
    {
        return Ok([(a.x + b.x) / 2., (a.y + b.y) / 2.]);
    }
    Err(ChartError("invalid original Bullet part geometry".into()))
}

fn customize(
    c: &ChartConfig,
    data: &[BulletDatum],
    o: &BulletLayoutOptions,
    mut layout: BulletLayout,
    factory: &mut impl FnMut(&BulletPartContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<CustomBulletLayout> {
    let plot = ClipRect::plot(c);
    let mut interaction = InteractionBinding::default();
    let marks: HashMap<_, _> = layout
        .scene
        .marks
        .iter()
        .enumerate()
        .map(|(index, mark)| (mark.id.clone(), index))
        .collect();
    let sources: HashMap<_, _> = data
        .iter()
        .enumerate()
        .map(|(index, datum)| (datum.id.as_str(), index))
        .collect();
    let replaced: HashSet<_> = layout
        .rows
        .iter()
        .flat_map(|row| row.parts.iter().map(|part| part.id.as_str()))
        .collect();
    let commands = |geometry: &Geometry| match geometry {
        Geometry::Path(path) => path.len(),
        _ => 0,
    };
    let stops = |paint: Option<&FillPaint>| paint.map_or(0, |p| p.stops().len());
    // Include unchanged axes/decorations; exclude every old replaced body.
    let mut command_count: usize = layout
        .scene
        .marks
        .iter()
        .filter(|mark| !replaced.contains(mark.id.as_str()))
        .map(|mark| commands(&mark.geometry))
        .sum();
    let mut stop_count: usize = layout
        .scene
        .marks
        .iter()
        .filter(|mark| !replaced.contains(mark.id.as_str()))
        .map(|mark| stops(mark.fill_paint.as_ref()))
        .sum();
    let mut geometries = Vec::new();
    for (row_index, row) in layout.rows.iter().enumerate() {
        let source_row_index = sources[row.id.as_str()];
        let mut ranks = [0; 3];
        for (part_index, part) in row.parts.iter().enumerate() {
            let kind_index = match part.kind {
                BulletPartKind::Range => 0,
                BulletPartKind::Measure => 1,
                BulletPartKind::Marker => 2,
            };
            let rank = ranks[kind_index];
            ranks[kind_index] += 1;
            let Some(&slot) = marks.get(&part.id) else {
                continue;
            };
            let original = &layout.scene.marks[slot];
            let position = original_position(original)?;
            let context = BulletPartContext {
                datum: &data[source_row_index],
                source_row_index,
                row,
                row_index,
                part,
                rank,
                geometry: &original.geometry,
                position,
                plot,
                fill: original.fill,
                fill_paint: original.fill_paint.as_ref(),
                stroke: original.stroke,
                stroke_dash: original.stroke_dash.as_ref(),
                opacity: original.opacity,
            };
            let custom = factory(&context)?;
            let inherited = custom == context.inherit(context.geometry.clone());
            if let Some(paint) = &custom.fill_paint {
                paint.validate()?;
            }
            command_count = command_count
                .checked_add(commands(&custom.geometry))
                .ok_or_else(|| ChartError("custom Bullet command budget overflow".into()))?;
            stop_count = stop_count
                .checked_add(stops(custom.fill_paint.as_ref()))
                .ok_or_else(|| ChartError("custom Bullet paint budget overflow".into()))?;
            ensure(
                command_count <= 200_000,
                "custom Bullet geometry budget exceeded",
            )?;
            ensure(
                stop_count <= MAX_SCENE_GRADIENT_STOPS,
                "custom Bullet paint budget exceeded",
            )?;
            // Use the same visible-fill / visible-stroke representative rule
            // for identity and replacement ink, without rewriting identity ink.
            let fallback_color = original
                .fill
                .or(original.stroke.map(|s| s.color))
                .unwrap_or(Color(0, 0, 0, 0));
            let prepared = custom.prepare_at(&part.id, position, fallback_color, 200_000)?;
            let (color, anchor) = if inherited {
                (prepared.color, prepared.anchor)
            } else {
                let mut mark = prepared.mark;
                mark.series = original.series.clone();
                mark.label = original.label.clone();
                mark.value = original.value;
                mark.clip = original.clip;
                mark.clip_path = original.clip_path.clone();
                geometries.push((row_index, part_index, mark.geometry.clone()));
                layout.scene.marks[slot] = mark;
                (prepared.color, prepared.anchor)
            };
            let accepted = &layout.scene.marks[slot];
            if let Some(anchor) = anchor {
                interaction.anchors.insert(part.id.clone(), anchor);
            }
            interaction.sites.push(InteractionSite {
                mark_id: part.id.clone(),
                series: row.id.clone(),
                label: accepted.label.clone(),
                value: Some(part.raw_value),
                formatted_value: Some(o.value_format.format(part.raw_value)?),
                color,
                position,
            });
        }
    }
    for (row, part, geometry) in geometries {
        layout.rows[row].parts[part].geometry = geometry;
    }
    interaction.validate(&layout.scene)?;
    Ok(CustomBulletLayout {
        layout,
        interaction,
        plot,
    })
}
