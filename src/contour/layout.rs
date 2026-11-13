use super::{
    ContourGrid, ContourLevel, ContourLimits, ContourOptions, DensityField, DensityThresholds,
    checked, contours,
};
use crate::*;
use glory::render::{GraphicPoint, GraphicStroke, PathCommand};
use std::collections::{BTreeMap, BTreeSet};

/// Scene admission is deliberately stricter than the raw algorithm budget.
/// Limits may only decrease; complete rings are never simplified or split to
/// bypass a renderer's smaller per-Graphic limit.
#[derive(Clone, Copy, Debug)]
pub struct ContourSceneLimits {
    pub commands: usize,
    pub levels: usize,
    pub polygons: usize,
    pub rings: usize,
}
impl Default for ContourSceneLimits {
    fn default() -> Self {
        Self {
            commands: 200_000,
            levels: 256,
            polygons: 20_000,
            rings: 20_000,
        }
    }
}
impl ContourSceneLimits {
    pub(crate) fn validate(self) -> Result<()> {
        let max = Self::default();
        checked(
            self.commands <= max.commands
                && self.levels <= max.levels
                && self.polygons <= max.polygons
                && self.rings <= max.rings,
            "contour Scene limits exceed the fixed profile",
        )
    }
}

#[derive(Clone, Debug)]
pub struct ContourLayoutOptions {
    pub id: String,
    pub colors: ColorScaleOptions,
    pub color_profile: ColorScaleProfile,
    pub opacity: f64,
    pub stroke: Option<GraphicStroke>,
    pub hidden: BTreeSet<String>,
    pub value_format: ValueFormat,
    pub color_legend: Option<ColorLegendOptions>,
    pub level_legend: bool,
    pub limits: ContourSceneLimits,
}
impl Default for ContourLayoutOptions {
    fn default() -> Self {
        Self {
            id: "contour".into(),
            colors: Default::default(),
            color_profile: ColorInterpolation::default().into(),
            opacity: 1.,
            stroke: None,
            hidden: Default::default(),
            value_format: Default::default(),
            color_legend: None,
            level_legend: false,
            limits: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContourLevelMetadata {
    pub id: String,
    pub threshold: f64,
    pub source_index: usize,
    pub duplicate_rank: usize,
    pub polygons: usize,
    pub holes: usize,
    pub points: usize,
    pub hidden: bool,
    pub mark_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct ContourLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub levels: Vec<ContourLevelMetadata>,
    pub colors: ColorScale,
    pub binding: InteractionBinding,
    pub path_commands: usize,
}
impl ContourLayout {
    pub fn scene_interaction(&self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene.clone(),
            interaction: Some(self.binding.clone()),
            plot: Some(self.plot),
        }
    }
}

pub fn contour_layout(
    config: &ChartConfig,
    grid: ContourGrid<'_>,
    compute: &ContourOptions,
    options: &ContourLayoutOptions,
) -> Result<ContourLayout> {
    let levels = contours(grid, compute)?;
    contour_levels_layout(
        config,
        &levels,
        ClipRect {
            x: 0.,
            y: 0.,
            width: grid.width as f64,
            height: grid.height as f64,
        },
        options,
    )
}

/// Reuses an accepted immutable aggregate when only thresholds or paint change.
pub fn density_layout(
    config: &ChartConfig,
    field: &DensityField,
    thresholds: &DensityThresholds,
    compute_limits: ContourLimits,
    options: &ContourLayoutOptions,
) -> Result<ContourLayout> {
    config.validate()?;
    let plot = ClipRect::plot(config);
    checked(
        field.size() == [plot.width, plot.height],
        "density field must be recomputed for the current plot size",
    )?;
    let levels = field.contours(thresholds, compute_limits)?;
    let size = field.size();
    contour_levels_layout(
        config,
        &levels,
        ClipRect {
            x: 0.,
            y: 0.,
            width: size[0],
            height: size[1],
        },
        options,
    )
}

/// Checked planar adapter for already-computed levels. Raw ring topology and
/// winding are retained; caller-authored rings must follow the contour contract.
/// This is not a spherical GeoJSON/projector entry point.
pub fn contour_levels_layout(
    config: &ChartConfig,
    levels: &[ContourLevel],
    source_extent: ClipRect,
    options: &ContourLayoutOptions,
) -> Result<ContourLayout> {
    config.validate()?;
    source_extent.validate()?;
    options.limits.validate()?;
    options.value_format.validate()?;
    checked(
        source_extent.width > 0. && source_extent.height > 0.,
        "contour source extent must be positive",
    )?;
    checked(
        !options.id.is_empty() && options.id.len() <= 512,
        "invalid contour namespace",
    )?;
    checked(
        options.opacity.is_finite() && (0. ..=1.).contains(&options.opacity),
        "invalid contour opacity",
    )?;
    checked(
        options.hidden.len() <= 256 && options.hidden.iter().all(|s| s.len() <= 1024),
        "contour hidden ID budget exceeded",
    )?;
    checked(
        levels.len() <= options.limits.levels && levels.iter().all(|l| l.value.is_finite()),
        "contour level budget or threshold invalid",
    )?;
    if let Some(stroke) = options.stroke {
        checked(
            stroke.width.is_finite() && stroke.width > 0. && stroke.width <= 10_000.,
            "invalid contour stroke width",
        )?;
    }
    let plot = ClipRect::plot(config);
    let project = |point: [f64; 2]| {
        GraphicPoint::new(
            plot.x + (point[0] - source_extent.x) / source_extent.width * plot.width,
            plot.y + (point[1] - source_extent.y) / source_extent.height * plot.height,
        )
    };
    // Charge hidden and empty input geometry as well: hiding is not an escape
    // hatch around the caller's accepted resource profile.
    let (mut commands, mut polygons, mut rings) = (0usize, 0usize, 0usize);
    for level in levels {
        polygons = polygons
            .checked_add(level.polygons.len())
            .ok_or_else(|| ChartError("contour polygon count overflow".into()))?;
        checked(
            polygons <= options.limits.polygons,
            "contour polygon budget exceeded",
        )?;
        for polygon in &level.polygons {
            for ring in std::iter::once(&polygon.outer).chain(&polygon.holes) {
                rings = rings
                    .checked_add(1)
                    .ok_or_else(|| ChartError("contour ring count overflow".into()))?;
                commands = commands
                    .checked_add(ring.len())
                    .ok_or_else(|| ChartError("contour command count overflow".into()))?;
                checked(
                    rings <= options.limits.rings && commands <= options.limits.commands,
                    "contour Scene ring or path budget exceeded",
                )?;
                checked(
                    ring.len() >= 4 && ring.first() == ring.last(),
                    "contour rings must explicitly close",
                )?;
                for &point in ring {
                    let p = project(point);
                    checked(
                        point.iter().all(|v| v.is_finite())
                            && [p.x, p.y]
                                .into_iter()
                                .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
                        "contour coordinate overflow",
                    )?;
                }
            }
        }
    }
    let colors = options
        .colors
        .resolve_with_profile(levels.iter().map(|l| Some(l.value)), options.color_profile)?;
    let mut scene = config.scene()?;
    let mut binding = InteractionBinding::default();
    let mut metadata = Vec::with_capacity(levels.len());
    let mut order = (0..levels.len()).collect::<Vec<_>>();
    order.sort_by(|&a, &b| levels[a].value.partial_cmp(&levels[b].value).unwrap());
    let mut ranks = BTreeMap::<u64, usize>::new();
    for index in order {
        let level = &levels[index];
        let key = if level.value == 0. {
            0
        } else {
            level.value.to_bits()
        };
        let rank = ranks.entry(key).or_default();
        let id = format!(
            "contour:{}:{}:{key:016x}:{}",
            options.id.len(),
            options.id,
            rank
        );
        let hidden = options.hidden.contains(&id);
        let mut item = ContourLevelMetadata {
            id: id.clone(),
            threshold: level.value,
            source_index: index,
            duplicate_rank: *rank,
            polygons: level.polygons.len(),
            holes: level.polygons.iter().map(|p| p.holes.len()).sum(),
            points: level
                .polygons
                .iter()
                .flat_map(|p| std::iter::once(&p.outer).chain(&p.holes))
                .map(Vec::len)
                .sum(),
            hidden,
            mark_id: None,
        };
        *rank += 1;
        if !hidden && !level.polygons.is_empty() {
            let mut path = Vec::with_capacity(item.points);
            for polygon in &level.polygons {
                for ring in std::iter::once(&polygon.outer).chain(&polygon.holes) {
                    path.push(PathCommand::Move(project(ring[0])));
                    for &point in &ring[1..ring.len() - 1] {
                        path.push(PathCommand::Line(project(point)));
                    }
                    path.push(PathCommand::Close);
                }
            }
            let PathCommand::Move(first) = path[0] else {
                unreachable!()
            };
            let position = [first.x, first.y];
            let geometry = Geometry::Path(path);
            let anchor = InteractionAnchor::path_points(&geometry, position, &[0])?;
            let color = colors.map(Some(level.value));
            let formatted = options.value_format.format(level.value)?;
            let mut mark = Mark::new(id.clone(), geometry, color);
            mark.value = level.value;
            mark.series = id.clone();
            mark.label = format!("Threshold {formatted}");
            mark.opacity = options.opacity;
            mark.stroke = options.stroke;
            mark.clip = Some(plot);
            binding.sites.push(InteractionSite {
                mark_id: id.clone(),
                series: id.clone(),
                label: mark.label.clone(),
                value: Some(level.value),
                formatted_value: Some(formatted.clone()),
                color,
                position,
            });
            binding.anchors.insert(id.clone(), anchor);
            if options.level_legend {
                scene.legend.push(LegendItem {
                    id: id.clone(),
                    label: format!("Threshold {formatted}"),
                    color,
                });
            }
            scene.marks.push(mark);
            item.mark_id = Some(id);
        }
        metadata.push(item);
    }
    if let Some(legend) = &options.color_legend {
        append_color_legend(&mut scene, &colors, legend)?;
    }
    binding.validate(&scene)?;
    let output = ContourLayout {
        scene,
        plot,
        levels: metadata,
        colors,
        binding,
        path_commands: commands,
    };
    output.scene_interaction().validate()?;
    Ok(output)
}
