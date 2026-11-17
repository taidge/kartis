//! Measured, owned cell metadata for live Glory Widgets.
use std::sync::Arc;

use super::*;
use crate::{
    MAX_PLOT_WIDGET_SLOTS, PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsData, PlotWidgetsLayout,
};

/// An owned source cell and its original numeric group data. Empty cells have
/// `group: None`; their default paint value zero is not a source datum.
/// Cell bounds describe the full allocation slot, while the host frame carries
/// the actual current Mark (including custom cell geometry and paint).
#[derive(Clone, Debug, PartialEq)]
pub struct WaffleCellWidgetData {
    pub cell: WaffleCell,
    /// Cells in the same group share one owned snapshot; equality is by value.
    pub group: Option<Arc<WaffleGroup>>,
    pub total: f64,
    pub unit: f64,
    pub grid: ClipRect,
    pub cell_size: f64,
}

/// Borrowed source metadata and actual current paint for a placement decision.
#[derive(Clone, Copy, Debug)]
pub struct WaffleCellWidgetContext<'a> {
    pub layout: &'a WaffleLayout,
    pub cell: &'a WaffleCell,
    pub group: Option<&'a WaffleGroup>,
    pub mark: &'a Mark,
}

/// Build typed measured slots without changing allocation, Scene, or picking.
/// Only actual cell marks invoke the callback, including empty and zero-opacity
/// cells. In Areas mode occupied slots belong to union marks and are skipped;
/// empty slots still have real cell marks. This is not an area Widget adapter.
///
/// Use the packet with `Chart::plot_widgets_with_data`, strict composition, or
/// the corresponding Browser methods. Public layout metadata is checked against
/// its grid, group identities, actual marks and typed sites before callbacks.
/// Custom cell geometry/paint is supported: the actual Mark is authoritative for
/// ink; full-slot bounds and raw numeric group values remain source metadata.
/// Returning `None` omits a slot. Callbacks are borrowed, never retained. Errors
/// return no partial packet; callback-owned side effects are not rolled back.
///
/// At most 128 selected slots are allowed. Existing hosts validate placements,
/// measured clipping and supported capabilities before admitting a frame. This
/// API supplies Glory frames; it does not supply React spring animated props.
pub fn waffle_cell_widgets(
    layout: &WaffleLayout,
    revision: u64,
    mut placement: impl FnMut(&WaffleCellWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<WaffleCellWidgetData>> {
    validate_layout(layout)?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), mark))
        .collect();
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut groups = BTreeMap::new();
    for cell in &layout.cells {
        let Some(id) = cell.mark_id.as_deref() else {
            continue;
        };
        let group = cell.owner.map(|owner| &layout.groups[owner]);
        let context = WaffleCellWidgetContext {
            layout,
            cell,
            group,
            mark: marks[id],
        };
        if let Some(placement) = placement(&context)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: id.into(),
                revision,
                placement,
            });
            packet.data.insert(
                id.into(),
                WaffleCellWidgetData {
                    cell: cell.clone(),
                    group: cell.owner.map(|owner| {
                        groups
                            .entry(owner)
                            .or_insert_with(|| Arc::new(layout.groups[owner].clone()))
                            .clone()
                    }),
                    total: layout.total,
                    unit: layout.unit,
                    grid: layout.grid,
                    cell_size: layout.cell_size,
                },
            );
        }
    }
    Ok(packet)
}

pub(super) fn validate_layout(layout: &WaffleLayout) -> Result<()> {
    layout.scene.validate()?;
    layout.interaction.validate(&layout.scene)?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    let count = layout
        .rows
        .checked_mul(layout.columns)
        .filter(|count| layout.rows > 0 && layout.columns > 0 && (1..=MAX_MARKS).contains(count))
        .ok_or_else(|| ChartError("invalid Waffle Widget grid budget".into()))?;
    ensure(
        layout.cells.len() == count,
        "Waffle Widget cell count differs from its grid",
    )?;
    let pitch =
        (layout.plot.width / layout.columns as f64).min(layout.plot.height / layout.rows as f64);
    ensure(
        layout.cell_size.is_finite()
            && layout.cell_size > 0.
            && layout.cell_size == pitch
            && layout.grid == centered_grid(layout.plot, pitch, layout.rows, layout.columns),
        "Waffle Widget grid differs from its source geometry",
    )?;
    ensure(
        layout.total.is_finite()
            && layout.total > 0.
            && layout.unit.is_finite()
            && layout.unit > 0.
            && layout.unit == layout.total / count as f64,
        "invalid Waffle Widget total or unit",
    )?;
    let source: Vec<_> = layout
        .groups
        .iter()
        .map(|group| group.source.clone())
        .collect();
    validate_data(&source)?;
    resolve_total(&source, Some(layout.total), count, layout.nivo_overflow)?;
    let allocation_options = WaffleLayoutOptions {
        hidden: layout
            .groups
            .iter()
            .filter(|group| group.hidden)
            .map(|group| group.source.id.clone())
            .collect(),
        ..Default::default()
    };
    let requested: Vec<_> = layout
        .groups
        .iter()
        .map(|group| group.requested.end.saturating_sub(group.requested.start))
        .collect();
    let independent = allocate(
        &source,
        &allocation_options,
        count,
        layout.total,
        layout.unit,
    )?;
    let apportioned = allocate(
        &source,
        &WaffleLayoutOptions {
            allocation: WaffleAllocation::LargestRemainder,
            ..allocation_options
        },
        count,
        layout.total,
        layout.unit,
    )?;
    ensure(
        requested == independent || requested == apportioned,
        "Waffle Widget allocation differs from source values",
    )?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|mark| (mark.id.as_str(), mark))
        .collect();
    let sites: BTreeMap<_, _> = layout
        .interaction
        .sites
        .iter()
        .map(|site| (site.mark_id.as_str(), site))
        .collect();
    let mut cursor = 0;
    let mut owners = vec![None; count];
    for (index, group) in layout.groups.iter().enumerate() {
        ensure(
            group.source_index == index
                && group.requested.start == cursor
                && group.requested.start <= group.requested.end
                && (!group.hidden || group.requested.is_empty())
                && (group.source.value != 0. || group.requested.is_empty()),
            "invalid Waffle Widget group allocation",
        )?;
        cursor = group.requested.end;
        let expected = group.requested.start.min(count)..group.requested.end.min(count);
        ensure(
            group.occupied == expected,
            "Waffle Widget occupied range differs from allocation",
        )?;
        owners[expected].fill(Some(index));
        ensure(
            group.formatted_value.len() <= crate::MAX_LABEL_BYTES,
            "invalid Waffle Widget formatted value",
        )?;
    }
    let mut cell_ids = BTreeSet::new();
    let mut owned_cell_marks = 0;
    let mut owned_union_cells = 0;
    for (index, (cell, owner)) in layout.cells.iter().zip(owners).enumerate() {
        let (row, column) = slot_at(layout.direction, layout.rows, layout.columns, index);
        let [x, y] = edge(layout.grid, pitch, row, column);
        ensure(
            cell.index == index
                && cell.row == row
                && cell.column == column
                && cell.owner == owner
                && cell.bounds
                    == ClipRect {
                        x,
                        y,
                        width: pitch,
                        height: pitch,
                    },
            "Waffle Widget cell differs from its grid or owner",
        )?;
        let Some(id) = cell.mark_id.as_deref() else {
            ensure(owner.is_some(), "empty Waffle Widget cell has no mark")?;
            owned_union_cells += 1;
            continue;
        };
        ensure(
            id == format!("waffle-cell:{row}:{column}") && cell_ids.insert(id),
            "invalid or duplicate Waffle Widget cell mark identity",
        )?;
        let mark = marks
            .get(id)
            .copied()
            .ok_or_else(|| ChartError("Waffle Widget cell mark is missing".into()))?;
        if let Some(owner) = owner {
            owned_cell_marks += 1;
            validate_group_mark(mark, &layout.groups[owner], &sites)?;
        } else {
            ensure(
                mark.series.is_empty()
                    && mark.label.is_empty()
                    && mark.value == 0.
                    && !sites.contains_key(id),
                "empty Waffle Widget cell contains fabricated source data",
            )?;
        }
    }
    ensure(
        owned_cell_marks == 0 || owned_union_cells == 0,
        "Waffle Widget layout mixes cell and area allocation",
    )?;
    let areas = owned_union_cells > 0;
    let mut area_ids = BTreeSet::new();
    for group in &layout.groups {
        ensure(
            group.polygons
                == group_polygons(
                    &layout.cells[group.occupied.clone()],
                    layout.grid,
                    pitch,
                    layout.nivo_polygons,
                ),
            "Waffle Widget group polygons differ from occupied cells",
        )?;
        if areas && !group.occupied.is_empty() {
            let expected = format!("waffle-area:{}", group.source.id);
            ensure(
                group.area_mark_id.as_deref() == Some(expected.as_str())
                    && area_ids.insert(expected.clone()),
                "Waffle Widget area identity differs from its group",
            )?;
            let mark = marks
                .get(expected.as_str())
                .copied()
                .ok_or_else(|| ChartError("Waffle Widget area mark is missing".into()))?;
            ensure(
                mark.geometry == Geometry::Path(area_path(&group.polygons)),
                "Waffle Widget area ink differs from its occupied union",
            )?;
            validate_group_mark(mark, group, &sites)?;
        } else {
            ensure(
                group.area_mark_id.is_none(),
                "Waffle Widget group has an orphan area mark",
            )?;
        }
    }
    for mark in &layout.scene.marks {
        ensure(
            !mark.id.starts_with("waffle-cell:") || cell_ids.contains(mark.id.as_str()),
            "orphan Waffle Widget cell mark",
        )?;
        ensure(
            !mark.id.starts_with("waffle-area:") || area_ids.contains(&mark.id),
            "orphan Waffle Widget area mark",
        )?;
    }
    Ok(())
}

fn validate_group_mark(
    mark: &Mark,
    group: &WaffleGroup,
    sites: &BTreeMap<&str, &InteractionSite>,
) -> Result<()> {
    let label = if group.source.label.is_empty() {
        &group.source.id
    } else {
        &group.source.label
    };
    ensure(
        mark.series == group.source.id && mark.label == *label && mark.value == group.source.value,
        "Waffle Widget cell source differs from its actual mark",
    )?;
    let site = sites
        .get(mark.id.as_str())
        .copied()
        .ok_or_else(|| ChartError("Waffle Widget data mark has no typed site".into()))?;
    ensure(
        site.series == group.source.id
            && site.label == *label
            && site.value == Some(group.source.value)
            && site.formatted_value.as_deref() == Some(group.formatted_value.as_str()),
        "Waffle Widget typed site differs from its source group",
    )
}
