//! Borrowed cell geometry; area unions and source allocation remain unchanged.
use std::collections::{HashMap, HashSet};

use super::*;
use crate::{FillPaint, ScatterNodeGeometry, StrokeDash};

#[derive(Clone, Copy, Debug)]
pub struct WaffleCellContext<'a> {
    pub cell: &'a WaffleCell,
    pub group: Option<&'a WaffleGroup>,
    pub geometry: &'a Geometry,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub grid: ClipRect,
    pub padding: f64,
    pub corner_radius: f64,
    pub fill: Option<Color>,
    pub fill_paint: Option<&'a FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<&'a StrokeDash>,
    pub opacity: f64,
}
impl WaffleCellContext<'_> {
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

/// Replace each actually drawn cell, in traversal order, with one bounded mark.
/// Empty cells also invoke the callback but never become typed data sites.
/// In Areas mode only empty cells have cell marks; occupied area unions remain.
/// Rect/Circle use the original slot center; offset ink requires Path geometry.
/// Group palette/legend colors and full-slot lookup do not follow custom ink.
/// Callback-owned side effects cannot be rolled back; callbacks are not stored.
pub fn waffle_layout_with_cell_factory(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    factory: &mut impl FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<WaffleLayout> {
    waffle_layout_with_factories_and_cell_factory(
        config,
        data,
        options,
        &mut WaffleFactories::default(),
        factory,
    )
}

/// Resolve every group function and the single empty-style template before
/// invoking the existing cell geometry function. Functions are never replayed.
pub fn waffle_layout_with_factories_and_cell_factory(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    factories: &mut WaffleFactories<'_>,
    factory: &mut impl FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<WaffleLayout> {
    let candidate = waffle_layout_with_factories(config, data, options, factories)?;
    customize_cells(options, candidate, factory)
}
pub(crate) fn customize_cells(
    options: &WaffleLayoutOptions,
    mut candidate: WaffleLayout,
    factory: &mut impl FnMut(&WaffleCellContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<WaffleLayout> {
    let marks: HashMap<_, _> = candidate
        .scene
        .marks
        .iter()
        .enumerate()
        .map(|(index, mark)| (mark.id.clone(), index))
        .collect();
    let sites: HashMap<_, _> = candidate
        .interaction
        .sites
        .iter()
        .enumerate()
        .map(|(index, site)| (site.mark_id.clone(), index))
        .collect();
    let replaced: HashSet<_> = candidate
        .cells
        .iter()
        .filter_map(|cell| cell.mark_id.as_deref())
        .collect();
    let commands = |g: &Geometry| {
        if let Geometry::Path(p) = g {
            p.len()
        } else {
            0
        }
    };
    let stops = |p: Option<&FillPaint>| p.map_or(0, |p| p.stops().len());
    let mut command_count: usize = candidate
        .scene
        .marks
        .iter()
        .filter(|m| !replaced.contains(m.id.as_str()))
        .map(|m| commands(&m.geometry))
        .sum();
    let mut stop_count: usize = candidate
        .scene
        .marks
        .iter()
        .filter(|m| !replaced.contains(m.id.as_str()))
        .map(|m| stops(m.fill_paint.as_ref()))
        .sum();
    for cell in &candidate.cells {
        let Some(id) = &cell.mark_id else {
            continue;
        };
        let slot = marks[id];
        let original = &candidate.scene.marks[slot];
        let group = cell.owner.map(|index| &candidate.groups[index]);
        let site = sites.get(id).copied();
        let position = site.map_or(
            [
                cell.bounds.x + cell.bounds.width / 2.,
                cell.bounds.y + cell.bounds.height / 2.,
            ],
            |index| candidate.interaction.sites[index].position,
        );
        let context = WaffleCellContext {
            cell,
            group,
            geometry: &original.geometry,
            position,
            plot: candidate.plot,
            grid: candidate.grid,
            padding: options.padding,
            corner_radius: options.corner_radius,
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
            .ok_or_else(|| ChartError("custom Waffle command budget overflow".into()))?;
        stop_count = stop_count
            .checked_add(stops(custom.fill_paint.as_ref()))
            .ok_or_else(|| ChartError("custom Waffle paint budget overflow".into()))?;
        ensure(
            command_count <= 200_000,
            "custom Waffle geometry budget exceeded",
        )?;
        ensure(
            stop_count <= crate::MAX_SCENE_GRADIENT_STOPS,
            "custom Waffle paint budget exceeded",
        )?;
        // Identity inheritance preserves original rounded-path anchors and all metadata.
        if inherited {
            continue;
        }
        let prepared = custom.prepare_at(
            id,
            position,
            group.map_or(options.empty_color, |g| g.color),
            200_000,
        )?;
        let mut mark = prepared.mark;
        mark.series = original.series.clone();
        mark.label = original.label.clone();
        mark.value = original.value;
        mark.clip = original.clip;
        mark.clip_path = original.clip_path.clone();
        if let Some(index) = site {
            candidate.interaction.anchors.remove(id);
            if let Some(anchor) = prepared.anchor {
                candidate.interaction.anchors.insert(id.clone(), anchor);
            }
            candidate.interaction.sites[index].color = prepared.color;
        }
        candidate.scene.marks[slot] = mark;
    }
    candidate.interaction.validate(&candidate.scene)?;
    Ok(candidate)
}
