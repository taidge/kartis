//! Waffle allocation, grid traversal, and group areas. The legacy `waffle`
//! retains its largest-remainder policy and original mark IDs.

use crate::{
    CategoryKey, ChartConfig, ChartError, ClipRect, Color, ColorModifier, Geometry, InheritedColor,
    InheritedColorContext, InteractionAnchor, InteractionBinding, InteractionSite, LegendItem,
    MAX_MARKS, Mark, OrdinalColorOptions, RadialDatum, Result, Scene, SceneInteraction,
    ValueFormat, cartesian::round_bar, crosshair::mark_center, radial::validate_data,
    scene::ensure,
};
use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};
use std::{
    collections::{BTreeMap, BTreeSet},
    ops::Range,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaffleFillDirection {
    /// Fill each row right to left, starting at the bottom.
    #[default]
    Top,
    /// Fill each column top to bottom, starting at the left.
    Right,
    /// Fill each row left to right, starting at the top.
    Bottom,
    /// Fill each column bottom to top, starting at the right.
    Left,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaffleAllocation {
    /// Round each visible group's value independently, as in Nivo. Later
    /// groups can run out of grid capacity when rounding expands the total.
    #[default]
    IndependentRound,
    /// Apportion the rounded aggregate by largest remainders. Resolve ties
    /// in source order.
    LargestRemainder,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum WaffleMode {
    /// Draw individual padded cells. Exact picking uses each cell's ink.
    #[default]
    Cells,
    /// Draw one solid path for each occupied group. Shared internal cell
    /// edges have no stroke or padding. Empty cells retain their cell style.
    Areas,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleLayoutOptions {
    pub rows: usize,
    pub columns: usize,
    /// Defaults to the sum of all source values, including hidden groups, or
    /// one for an empty chart. An explicit total must include all values.
    pub total: Option<f64>,
    pub direction: WaffleFillDirection,
    pub allocation: WaffleAllocation,
    pub mode: WaffleMode,
    /// The gap between painted cells. Half the gap is inset on each side.
    pub padding: f64,
    /// Clamped to half the painted cell size. Group area contours stay square.
    pub corner_radius: f64,
    pub empty_color: Color,
    pub empty_opacity: f64,
    pub border_width: f64,
    pub border_color: InheritedColor,
    pub colors: OrdinalColorOptions,
    /// Hidden groups keep their original colors and legend items. Their
    /// visible successors advance into the vacated slots.
    pub hidden: BTreeSet<String>,
    pub value_format: ValueFormat,
}

impl Default for WaffleLayoutOptions {
    fn default() -> Self {
        Self {
            rows: 10,
            columns: 10,
            total: None,
            direction: Default::default(),
            allocation: Default::default(),
            mode: Default::default(),
            padding: 1.,
            corner_radius: 0.,
            empty_color: Color(204, 204, 204, 255),
            empty_opacity: 1.,
            border_width: 0.,
            border_color: InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
            colors: Default::default(),
            hidden: BTreeSet::new(),
            value_format: ValueFormat::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleCell {
    pub index: usize,
    pub row: usize,
    pub column: usize,
    pub bounds: ClipRect,
    /// Source index into `WaffleLayout::groups`.
    pub owner: Option<usize>,
    /// Stable physical slot ID in cell mode and for empty slots in area mode.
    pub mark_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleGroup {
    pub source: RadialDatum,
    pub source_index: usize,
    pub formatted_value: String,
    pub hidden: bool,
    pub color: Color,
    pub border_color: Color,
    /// Requested half-open traversal range before grid capacity is applied.
    pub requested: Range<usize>,
    /// Occupied half-open traversal range clipped to actual grid slots.
    pub occupied: Range<usize>,
    /// Clockwise whole-slot outlines with implicit closing edges. Components
    /// that touch at a single vertex remain separate.
    pub polygons: Vec<Vec<[f64; 2]>>,
    pub area_mark_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WaffleLayout {
    pub scene: Scene,
    pub total: f64,
    pub unit: f64,
    pub grid: ClipRect,
    pub cell_size: f64,
    pub groups: Vec<WaffleGroup>,
    /// Includes empty slots and is ordered in the configured fill direction.
    pub cells: Vec<WaffleCell>,
    pub interaction: InteractionBinding,
    rows: usize,
    columns: usize,
    direction: WaffleFillDirection,
    plot: ClipRect,
}

impl WaffleLayout {
    /// Lookup a full grid slot in logical coordinates, including the visual
    /// padding. The grid's right and bottom edges are excluded.
    pub fn cell_at(&self, x: f64, y: f64) -> Option<&WaffleCell> {
        let axis = |value: f64, start: f64, extent: f64, count: usize| {
            (value >= start && value < start + extent).then(|| {
                let approx = (((value - start) / self.cell_size).floor() as usize).min(count - 1);
                // Correct round-off at shared boundaries using the same
                // projected integer edges as the layout.
                if approx > 0 && value < start + approx as f64 * self.cell_size {
                    approx - 1
                } else if approx + 1 < count
                    && value >= start + (approx + 1) as f64 * self.cell_size
                {
                    approx + 1
                } else {
                    approx
                }
            })
        };
        let column = axis(x, self.grid.x, self.grid.width, self.columns)?;
        let row = axis(y, self.grid.y, self.grid.height, self.rows)?;
        self.cells.get(index_of(
            self.direction,
            self.rows,
            self.columns,
            row,
            column,
        ))
    }

    /// Lookup the owning group by full slot, including gaps between cells.
    pub fn area_at(&self, x: f64, y: f64) -> Option<&WaffleGroup> {
        self.groups.get(self.cell_at(x, y)?.owner?)
    }

    pub fn into_scene_interaction(self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene,
            interaction: Some(self.interaction),
            plot: Some(self.plot),
        }
    }
}

/// Layout a centered, square waffle grid with typed slot and group metadata.
pub fn waffle_layout(
    config: &ChartConfig,
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
) -> Result<WaffleLayout> {
    let mut scene = config.scene()?;
    validate_data(data)?;
    let WaffleLayoutOptions {
        rows,
        columns,
        direction,
        ..
    } = *options;
    let count = rows
        .checked_mul(columns)
        .filter(|n| (1..=MAX_MARKS).contains(n))
        .ok_or_else(|| ChartError("invalid Waffle grid budget".into()))?;
    let plot = ClipRect::plot(config);
    let cell_size = (plot.width / columns as f64).min(plot.height / rows as f64);
    ensure(
        options.padding.is_finite() && (0. ..cell_size).contains(&options.padding),
        "Waffle padding must be smaller than the cell size",
    )?;
    ensure(
        options.corner_radius.is_finite() && options.corner_radius >= 0.,
        "invalid Waffle corner radius",
    )?;
    ensure(
        (0. ..=1.).contains(&options.empty_opacity),
        "invalid Waffle empty opacity",
    )?;
    ensure(
        options.border_width == 0. || (0.000001..=10_000.).contains(&options.border_width),
        "invalid Waffle border width",
    )?;
    options.border_color.validate()?;
    options.value_format.validate()?;
    let grid = centered_grid(plot, cell_size, rows, columns);
    let [total, unit] = resolve_total(data, options.total, count)?;
    let allocations = allocate(data, options, count, total, unit);
    let mut colors = options.colors.resolve(
        data.iter()
            .map(|datum| CategoryKey::from(datum.id.as_str())),
    )?;
    let border_color = |fill| {
        options
            .border_color
            .resolve_validated(InheritedColorContext {
                inherited: fill,
                surface: fill,
                backdrop: scene.background,
                theme: config.theme.text,
            })
    };
    let empty_border = border_color(options.empty_color)?;
    let mut groups = Vec::with_capacity(data.len());
    let mut cursor = 0;
    let mut owners = vec![None; count];
    for (source_index, (source, allocation)) in data.iter().zip(allocations).enumerate() {
        let requested = cursor..cursor + allocation;
        cursor = requested.end;
        let occupied = requested.start.min(count)..requested.end.min(count);
        owners[occupied.clone()].fill(Some(source_index));
        let color = colors
            .map_or_insert(CategoryKey::from(source.id.as_str()))?
            .ok_or_else(|| ChartError("Waffle group is missing an ordinal color".into()))?;
        groups.push(WaffleGroup {
            source: source.clone(),
            source_index,
            formatted_value: options.value_format.format(source.value)?,
            hidden: options.hidden.contains(&source.id),
            color,
            border_color: border_color(color)?,
            requested,
            occupied,
            polygons: Vec::new(),
            area_mark_id: None,
        });
    }

    let mut interaction = InteractionBinding::default();
    let mut cells = Vec::with_capacity(count);
    let ink = cell_size - options.padding;
    for (index, owner) in owners.into_iter().enumerate() {
        let (row, column) = slot_at(direction, rows, columns, index);
        let [x, y] = edge(grid, cell_size, row, column);
        let bounds = ClipRect {
            x,
            y,
            width: cell_size,
            height: cell_size,
        };
        ensure(
            x + cell_size > x && y + cell_size > y,
            "Waffle slot projection collapsed",
        )?;
        let mut cell = WaffleCell {
            index,
            row,
            column,
            bounds,
            owner,
            mark_id: None,
        };
        if options.mode == WaffleMode::Cells || owner.is_none() {
            let x = x + options.padding / 2.;
            let y = y + options.padding / 2.;
            ensure(
                x + ink > x && y + ink > y,
                "Waffle cell ink projection collapsed",
            )?;
            let geometry = round_bar(
                Geometry::Rect {
                    x,
                    y,
                    width: ink,
                    height: ink,
                },
                options.corner_radius,
            );
            let id = format!("waffle-cell:{row}:{column}");
            let group = owner.map(|index| &groups[index]);
            let mut mark = Mark::new(
                &id,
                geometry,
                group.map_or(options.empty_color, |g| g.color),
            );
            mark.stroke = border(
                options.border_width,
                group.map_or(empty_border, |g| g.border_color),
            );
            if let Some(group) = group {
                add_site(
                    &mut mark,
                    group,
                    [x + ink / 2., y + ink / 2.],
                    &[2, 4, 6, 8],
                    &mut interaction,
                )?;
            } else {
                mark.label.clear();
                mark.opacity = options.empty_opacity;
            }
            scene.marks.push(mark);
            cell.mark_id = Some(id);
        }
        cells.push(cell);
    }

    for group in &mut groups {
        let occupied = &cells[group.occupied.clone()];
        group.polygons = group_polygons(occupied, grid, cell_size);
        if options.mode == WaffleMode::Areas
            && let Some(first) = occupied.first()
        {
            let id = format!("waffle-area:{}", group.source.id);
            let path = area_path(&group.polygons);
            let support = path
                .iter()
                .enumerate()
                .filter_map(|(i, command)| (!matches!(command, PathCommand::Close)).then_some(i))
                .collect::<Vec<_>>();
            let mut mark = Mark::new(&id, Geometry::Path(path), group.color);
            mark.stroke = border(options.border_width, group.border_color);
            let position = [
                first.bounds.x + cell_size / 2.,
                first.bounds.y + cell_size / 2.,
            ];
            add_site(&mut mark, group, position, &support, &mut interaction)?;
            group.area_mark_id = Some(id);
            scene.marks.push(mark);
        }
        scene.legend.push(LegendItem {
            id: group.source.id.clone(),
            label: group.source.label.clone(),
            color: group.color,
        });
    }
    if matches!(
        direction,
        WaffleFillDirection::Top | WaffleFillDirection::Left
    ) {
        scene.legend.reverse();
    }
    interaction.validate(&scene)?;
    crate::surface::validate_plot(Some(plot), &scene)?;
    Ok(WaffleLayout {
        scene,
        total,
        unit,
        grid,
        cell_size,
        groups,
        cells,
        interaction,
        rows,
        columns,
        direction,
        plot,
    })
}

fn resolve_total(data: &[RadialDatum], explicit: Option<f64>, count: usize) -> Result<[f64; 2]> {
    let sum: f64 = data.iter().map(|datum| datum.value).sum();
    ensure(sum.is_finite(), "Waffle value sum overflow")?;
    let total = explicit.unwrap_or(if sum == 0. { 1. } else { sum });
    ensure(
        total.is_finite() && total > 0. && (sum <= total || (sum - total) / total <= 1e-12),
        "Waffle total must be positive and include all values",
    )?;
    let unit = total / count as f64;
    ensure(unit > 0., "Waffle unit is too small to represent")?;
    Ok([total, unit])
}

fn allocate(
    data: &[RadialDatum],
    options: &WaffleLayoutOptions,
    count: usize,
    total: f64,
    unit: f64,
) -> Vec<usize> {
    let quotas = data.iter().map(|datum| {
        if options.hidden.contains(&datum.id) {
            0.
        } else if unit.is_normal() {
            datum.value / unit
        } else {
            // Avoid losing quota precision through a rounded subnormal unit.
            datum.value / total * count as f64
        }
    });
    if options.allocation == WaffleAllocation::IndependentRound {
        return quotas.map(|quota| quota.round() as usize).collect();
    }
    let quotas = quotas.collect::<Vec<_>>();
    let occupied = (quotas.iter().sum::<f64>().round() as usize).min(count);
    let mut allocations = quotas
        .iter()
        .map(|quota| quota.floor() as usize)
        .collect::<Vec<_>>();
    let remaining = occupied.saturating_sub(allocations.iter().sum());
    let mut order: Vec<_> = (0..data.len()).collect();
    order.sort_by(|&a, &b| {
        quotas[b]
            .fract()
            .total_cmp(&quotas[a].fract())
            .then(a.cmp(&b))
    });
    for index in order.into_iter().take(remaining) {
        allocations[index] += 1;
    }
    allocations
}

fn centered_grid(plot: ClipRect, pitch: f64, rows: usize, columns: usize) -> ClipRect {
    let width = pitch * columns as f64;
    let height = pitch * rows as f64;
    ClipRect {
        x: plot.x + (plot.width - width).max(0.) / 2.,
        y: plot.y + (plot.height - height).max(0.) / 2.,
        width,
        height,
    }
}

fn slot_at(
    direction: WaffleFillDirection,
    rows: usize,
    columns: usize,
    index: usize,
) -> (usize, usize) {
    match direction {
        WaffleFillDirection::Top => (rows - 1 - index / columns, columns - 1 - index % columns),
        WaffleFillDirection::Right => (index % rows, index / rows),
        WaffleFillDirection::Bottom => (index / columns, index % columns),
        WaffleFillDirection::Left => (rows - 1 - index % rows, columns - 1 - index / rows),
    }
}

fn index_of(
    direction: WaffleFillDirection,
    rows: usize,
    columns: usize,
    row: usize,
    column: usize,
) -> usize {
    match direction {
        WaffleFillDirection::Top => (rows - 1 - row) * columns + columns - 1 - column,
        WaffleFillDirection::Right => column * rows + row,
        WaffleFillDirection::Bottom => row * columns + column,
        WaffleFillDirection::Left => (columns - 1 - column) * rows + rows - 1 - row,
    }
}

fn edge(grid: ClipRect, pitch: f64, row: usize, column: usize) -> [f64; 2] {
    [grid.x + column as f64 * pitch, grid.y + row as f64 * pitch]
}

fn border(width: f64, color: Color) -> Option<GraphicStroke> {
    (width > 0.).then_some(GraphicStroke {
        width,
        color,
        cap: StrokeCap::Butt,
    })
}

fn add_site(
    mark: &mut Mark,
    group: &WaffleGroup,
    position: [f64; 2],
    support: &[usize],
    binding: &mut InteractionBinding,
) -> Result<()> {
    mark.series.clone_from(&group.source.id);
    mark.label = if group.source.label.is_empty() {
        group.source.id.clone()
    } else {
        group.source.label.clone()
    };
    mark.value = group.source.value;
    let position = if let Some(center) = mark_center(mark) {
        center
    } else {
        let anchor = InteractionAnchor::path_offset_following(&mark.geometry, position, support)?;
        binding.anchors.insert(mark.id.clone(), anchor);
        position
    };
    binding.sites.push(InteractionSite {
        mark_id: mark.id.clone(),
        series: mark.series.clone(),
        label: mark.label.clone(),
        value: Some(mark.value),
        formatted_value: Some(group.formatted_value.clone()),
        color: group.color,
        position,
    });
    Ok(())
}

fn group_polygons(cells: &[WaffleCell], grid: ClipRect, pitch: f64) -> Vec<Vec<[f64; 2]>> {
    // An interval of a row-major or column-major fill has contiguous cells
    // in each occupied row. Store exact integer endpoints before projection.
    let mut rows = BTreeMap::<usize, [usize; 2]>::new();
    for cell in cells {
        let columns = rows
            .entry(cell.row)
            .or_insert([cell.column, cell.column + 1]);
        columns[0] = columns[0].min(cell.column);
        columns[1] = columns[1].max(cell.column + 1);
    }
    let rows: Vec<_> = rows.into_iter().collect();
    rows.chunk_by(|(above, [left, right]), (below, [next_left, next_right])| {
        *above + 1 == *below && left < next_right && next_left < right
    })
    .map(|part| {
        let mut vertices = Vec::with_capacity(part.len() * 4);
        for &(row, [_, right]) in part {
            vertices.extend([(row, right), (row + 1, right)]);
        }
        for &(row, [left, _]) in part.iter().rev() {
            vertices.extend([(row + 1, left), (row, left)]);
        }
        vertices.dedup();
        vertices
            .iter()
            .enumerate()
            .filter_map(|(i, &(row, column))| {
                let previous = vertices[(i + vertices.len() - 1) % vertices.len()];
                let next = vertices[(i + 1) % vertices.len()];
                let straight = (previous.0 == row && next.0 == row)
                    || (previous.1 == column && next.1 == column);
                (!straight).then(|| edge(grid, pitch, row, column))
            })
            .collect()
    })
    .collect()
}

fn area_path(polygons: &[Vec<[f64; 2]>]) -> Vec<PathCommand> {
    let mut commands = Vec::new();
    for polygon in polygons {
        let [[x, y], tail @ ..] = polygon.as_slice() else {
            continue;
        };
        commands.push(PathCommand::Move(GraphicPoint::new(*x, *y)));
        commands.extend(
            tail.iter()
                .map(|[x, y]| PathCommand::Line(GraphicPoint::new(*x, *y))),
        );
        commands.push(PathCommand::Close);
    }
    commands
}
