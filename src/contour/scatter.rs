use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use super::{
    ContourLayout, ContourLayoutOptions, ContourLevelMetadata, ContourLimits, DensityField,
    DensityOptions, DensityPoint, DensityThresholds, checked, density_layout, estimate_density,
};
use crate::*;

/// Explicit weights by original (series ID, datum ID). Missing weights are one;
/// unknown IDs and nonfinite values reject rather than silently losing inputs.
/// Scatter's `size_value` never participates in density weighting.
pub type DensityWeights = BTreeMap<(String, String), f64>;

/// Scatter owns the estimator extent through its accepted plot. There is no
/// independently authored `size` field that can silently disagree with it.
#[derive(Clone, Copy, Debug)]
pub struct ScatterDensityOptions {
    pub bandwidth: f64,
    pub cell_size: f64,
    pub limits: ContourLimits,
}
impl Default for ScatterDensityOptions {
    fn default() -> Self {
        let options = DensityOptions::default();
        Self {
            bandwidth: options.bandwidth,
            cell_size: options.cell_size,
            limits: options.limits,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DensityObservation {
    pub mark_id: String,
    pub series_id: String,
    pub node_id: String,
    pub source_index: usize,
    pub raw_x: CategoryKey,
    pub raw_y: CategoryKey,
    pub position: [f64; 2],
    pub weight: f64,
}
#[derive(Clone, Debug)]
pub struct ScatterDensityLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub binding: InteractionBinding,
    pub levels: Vec<ContourLevelMetadata>,
    pub field: DensityField,
    pub observations: Vec<DensityObservation>,
    retained: Arc<RetainedDensity>,
}
#[derive(Debug)]
struct RetainedDensity {
    config: ChartConfig,
    scatter: ScatterLayout,
    field: DensityField,
    observations: Vec<DensityObservation>,
}
impl ScatterDensityLayout {
    pub fn scene_interaction(&self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene.clone(),
            interaction: Some(self.binding.clone()),
            plot: Some(self.plot),
        }
    }

    /// Extract new thresholds or paint from the same accepted weighted field.
    /// The original config, axes, source Scene and observations are retained
    /// privately. Public output fields are snapshots, not replacement inputs.
    /// Data, source visibility, weights, bandwidth, axes and extent changes
    /// require a new [`scatter_density_layout`] call.
    pub fn recontour(
        &self,
        thresholds: &DensityThresholds,
        limits: ContourLimits,
        options: &ContourLayoutOptions,
    ) -> Result<Self> {
        render_retained(self.retained.clone(), thresholds, limits, options)
    }
}

/// Estimate from all mapped, non-hidden scatter observations, including points
/// whose glyph is size-zero. Values outside the plot retain D3's padded-field
/// contribution. The caller's numeric/category projection happens only in the
/// original ScatterLayout; bandwidth and KDE positions use plot-local pixels.
/// Source geometry/fonts/order and typed raw scatter identity remain intact.
pub fn scatter_density_layout(
    config: &ChartConfig,
    scatter: &ScatterLayout,
    weights: &DensityWeights,
    compute: ScatterDensityOptions,
    thresholds: &DensityThresholds,
    options: &ContourLayoutOptions,
) -> Result<ScatterDensityLayout> {
    config.validate()?;
    options.limits.validate()?;
    scatter.scene.validate()?;
    let base_binding = scatter.interaction_binding();
    base_binding.validate(&scatter.scene)?;
    checked(
        scatter.scene.width == config.width
            && scatter.scene.height == config.height
            && scatter.plot == ClipRect::plot(config),
        "scatter density frame/config mismatch",
    )?;
    checked(
        scatter.scene.typography.default == config.theme.font,
        "scatter density default font does not match its frame",
    )?;
    checked(
        scatter.metadata.len() <= 20_000 && weights.len() <= 20_000,
        "density source/weight budget exceeded",
    )?;
    checked(
        weights.values().all(|v| v.is_finite()),
        "density weight must be finite",
    )?;
    let mut ids = BTreeSet::new();
    let mut series = BTreeMap::<&str, (usize, usize)>::new();
    let mut previous_series: Option<(&str, usize)> = None;
    for (global, item) in scatter.metadata.iter().enumerate() {
        if previous_series.is_none_or(|previous| previous.0 != item.series_id) {
            checked(
                !series.contains_key(item.series_id.as_str())
                    && previous_series.is_none_or(|previous| previous.1 < item.series_index),
                "scatter density source series order is inconsistent",
            )?;
        }
        previous_series = Some((&item.series_id, item.series_index));
        let next = series
            .entry(&item.series_id)
            .or_insert((item.series_index, 0));
        checked(
            item.global_index == global
                && item.series_index == next.0
                && item.source_index == next.1,
            "scatter density source order disagrees with authored indices",
        )?;
        next.1 += 1;
        checked(
            !item.series_id.is_empty()
                && item.series_id.len() <= 1024
                && !item.node_id.is_empty()
                && item.node_id.len() <= 1024
                && !item.mark_id.is_empty()
                && item.mark_id.len() <= 1024,
            "invalid density source identity",
        )?;
        checked(
            ids.insert((item.series_id.clone(), item.node_id.clone())),
            "duplicate scatter density identity",
        )?;
    }
    checked(
        weights.keys().all(|id| ids.contains(id)),
        "density weight refers to an absent source datum",
    )?;
    let mut observations = Vec::new();
    let mut points = Vec::new();
    let sites = scatter
        .sites
        .iter()
        .map(|s| (s.mark_id.as_str(), s))
        .collect::<BTreeMap<_, _>>();
    checked(
        sites.len() == scatter.sites.len(),
        "duplicate painted scatter site",
    )?;
    let metadata = scatter
        .metadata
        .iter()
        .map(|m| (m.mark_id.as_str(), m))
        .collect::<BTreeMap<_, _>>();
    checked(
        metadata.len() == scatter.metadata.len()
            && sites.keys().all(|id| metadata.contains_key(id)),
        "painted scatter datum has no unique source metadata",
    )?;
    let painted_ids = scatter
        .scene
        .marks
        .iter()
        .map(|mark| mark.id.as_str())
        .collect::<BTreeSet<_>>();
    for item in &scatter.metadata {
        if !item.hidden {
            let expected = scatter
                .x_scale
                .map(&item.raw_x)?
                .zip(scatter.y_scale.map(&item.raw_y)?)
                .map(|(x, y)| [x, y]);
            checked(
                item.position == expected,
                "scatter density source position disagrees with resolved axes",
            )?;
        }
        let site = sites.get(item.mark_id.as_str());
        checked(
            !painted_ids.contains(item.mark_id.as_str()) || site.is_some(),
            "painted scatter source has no typed site",
        )?;
        if let Some(site) = site {
            checked(
                !item.hidden
                    && item.position == Some(site.position)
                    && item.mark_index == Some(site.mark_index)
                    && item.node_size == Some(site.node_size)
                    && item.source_index == site.source_index
                    && item.series_index == site.series_index
                    && item.global_index == site.global_index
                    && item.size_value == site.size_value
                    && item.raw_x == site.raw_x
                    && item.raw_y == site.raw_y
                    && item.series_id == site.series_id
                    && item.node_id == site.node_id
                    && scatter
                        .scene
                        .marks
                        .get(site.mark_index)
                        .is_some_and(|mark| mark.id == item.mark_id),
                "scatter density metadata disagrees with painted datum",
            )?;
        } else {
            checked(
                item.mark_index.is_none() && item.node_size.is_none(),
                "scatter metadata refers to an absent painted datum",
            )?;
        }
        if item.hidden {
            checked(
                item.position.is_none() && site.is_none(),
                "hidden source retains painted coordinates",
            )?;
            continue;
        }
        let Some(position) = item.position else {
            continue;
        };
        // Check externally modified layouts before mixing their typed source
        // descriptors with an unrelated painted frame.
        let expected = [
            scatter.x_scale.map(&item.raw_x)?,
            scatter.y_scale.map(&item.raw_y)?,
        ];
        checked(
            expected == [Some(position[0]), Some(position[1])],
            "scatter density source position disagrees with resolved axes",
        )?;
        if let Some(site) = sites.get(item.mark_id.as_str()) {
            checked(
                site.position == position
                    && site.raw_x == item.raw_x
                    && site.raw_y == item.raw_y
                    && site.series_id == item.series_id
                    && site.node_id == item.node_id,
                "scatter density metadata disagrees with painted datum",
            )?;
        }
        let weight = weights
            .get(&(item.series_id.clone(), item.node_id.clone()))
            .copied()
            .unwrap_or(1.);
        points.push(DensityPoint {
            position: [position[0] - scatter.plot.x, position[1] - scatter.plot.y],
            weight,
        });
        observations.push(DensityObservation {
            mark_id: item.mark_id.clone(),
            series_id: item.series_id.clone(),
            node_id: item.node_id.clone(),
            source_index: item.source_index,
            raw_x: item.raw_x.clone(),
            raw_y: item.raw_y.clone(),
            position,
            weight,
        });
    }
    // The field covers the current plot, not the default 960x500 estimator
    // extent. This prevents applying a second fit/camera to the observation.
    let field = estimate_density(
        &points,
        DensityOptions {
            size: [scatter.plot.width, scatter.plot.height],
            bandwidth: compute.bandwidth,
            cell_size: compute.cell_size,
            limits: compute.limits,
        },
    )?;
    render_retained(
        Arc::new(RetainedDensity {
            config: config.clone(),
            scatter: scatter.clone(),
            field,
            observations,
        }),
        thresholds,
        compute.limits,
        options,
    )
}

fn render_retained(
    retained: Arc<RetainedDensity>,
    thresholds: &DensityThresholds,
    limits: ContourLimits,
    options: &ContourLayoutOptions,
) -> Result<ScatterDensityLayout> {
    options.limits.validate()?;
    let existing_commands =
        retained
            .scatter
            .scene
            .marks
            .iter()
            .try_fold(0usize, |total, mark| {
                let count = match &mark.geometry {
                    Geometry::Path(p) => p.len(),
                    Geometry::Rect { .. } => 5,
                    Geometry::Circle { .. } => 6,
                };
                total
                    .checked_add(count)
                    .ok_or_else(|| ChartError("scatter path count overflow".into()))
            })?;
    let mut style = options.clone();
    style.limits.commands = style.limits.commands.min(
        200_000usize
            .checked_sub(existing_commands)
            .ok_or_else(|| ChartError("scatter leaves no contour command budget".into()))?,
    );
    let contours = density_layout(
        &retained.config,
        &retained.field,
        thresholds,
        limits,
        &style,
    )?;
    compose(retained, contours)
}

fn compose(
    retained: Arc<RetainedDensity>,
    contours: ContourLayout,
) -> Result<ScatterDensityLayout> {
    let scatter = &retained.scatter;
    let source_series = scatter
        .metadata
        .iter()
        .map(|s| s.series_id.as_str())
        .collect::<BTreeSet<_>>();
    checked(
        contours
            .levels
            .iter()
            .all(|level| !source_series.contains(level.id.as_str())),
        "contour namespace collides with a source series ID",
    )?;
    let mut content = Vec::new();
    let mut label_slots = BTreeMap::new();
    let mut next_label = scatter.scene.labels.len();
    for item in contours.scene.draw_items() {
        content.push(match item {
            DrawItem::Mark(index) => LayerContent::Mark(contours.scene.marks[index].clone()),
            DrawItem::Label(index) => {
                label_slots.insert(index, next_label);
                next_label += 1;
                LayerContent::Label {
                    label: contours.scene.labels[index].clone(),
                    series: contours.scene.label_series.get(&index).cloned(),
                }
            }
        });
    }
    let mut scene = compose_layers(
        &scatter.scene,
        &[
            ChartLayer::Custom(CustomLayer::new("density", content)?),
            ChartLayer::References(NamedLayer::all("scatter", &scatter.scene)?),
        ],
    )?;
    for (old, font) in &contours.scene.typography.labels {
        let new = *label_slots
            .get(old)
            .ok_or_else(|| ChartError("density label font refers to a missing slot".into()))?;
        scene.typography.labels.insert(new, font.clone());
    }
    for (old, stroke) in &contours.scene.typography.label_strokes {
        let new = *label_slots
            .get(old)
            .ok_or_else(|| ChartError("density label stroke refers to missing slot".into()))?;
        scene.typography.label_strokes.insert(new, *stroke);
    }
    for (old, mode) in &contours.scene.typography.label_whitespace {
        let new = *label_slots
            .get(old)
            .ok_or_else(|| ChartError("density whitespace refers to missing slot".into()))?;
        scene.typography.label_whitespace.insert(new, *mode);
    }
    for (old, clip) in &contours.scene.label_clips {
        let new = *label_slots
            .get(old)
            .ok_or_else(|| ChartError("density label clip refers to a missing slot".into()))?;
        scene.label_clips.insert(new, clip.clone());
    }
    scene.legend.extend(contours.scene.legend);
    let mut binding = scatter.interaction_binding();
    binding.sites.extend(contours.binding.sites);
    for (id, anchor) in contours.binding.anchors {
        checked(
            binding.anchors.insert(id, anchor).is_none(),
            "density/scatter anchor ID collision",
        )?;
    }
    binding.validate(&scene)?;
    let output = ScatterDensityLayout {
        scene,
        plot: scatter.plot,
        binding,
        levels: contours.levels,
        field: retained.field.clone(),
        observations: retained.observations.clone(),
        retained,
    };
    output.scene_interaction().validate()?;
    Ok(output)
}
