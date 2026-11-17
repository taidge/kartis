//! Actual HeatMap cell components share accepted source, placement and paint.
use super::*;
use crate::{
    MAX_PLOT_WIDGET_SLOTS, PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsData, PlotWidgetsLayout,
    SceneInteraction,
};

/// Owned original input and the current computed cell. Explicit null values
/// remain None; absent row/column pairs never acquire synthetic Widget data.
/// Inspect the live frame's Mark for current paint, including user decoration.
#[derive(Clone, Debug)]
pub struct HeatmapCellWidgetData {
    pub datum: HeatCell,
    /// Index in the original input slice, before configured identity ordering.
    pub source_index: usize,
    pub cell: ComputedHeatmapCell,
    pub plot: ClipRect,
}

// Source signed zero is observable metadata even when geometry is unchanged.
impl PartialEq for HeatmapCellWidgetData {
    fn eq(&self, other: &Self) -> bool {
        self.datum == other.datum
            && crate::widget_source_eq::optional(self.datum.value, other.datum.value)
            && self.source_index == other.source_index
            && self.cell == other.cell
            && crate::widget_source_eq::optional(self.cell.value, other.cell.value)
            && self.plot == other.plot
    }
}

/// Borrowed only while choosing a measured placement. Actual geometry can be
/// a custom Path; the band and logical center retain their original meanings.
#[derive(Clone, Copy, Debug)]
pub struct HeatmapCellWidgetContext<'a> {
    pub layout: &'a HeatmapLayout,
    pub datum: &'a HeatCell,
    pub source_index: usize,
    pub cell: &'a ComputedHeatmapCell,
    pub mark: &'a Mark,
}

impl HeatmapLayout {
    /// Move the accepted nullable binding, plot and Scene together. In
    /// particular, an explicit null cell is not rebound as the compatibility
    /// Mark scalar zero. This also preserves custom Path anchor descriptors.
    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        self.scene.validate()?;
        let interaction = self.interaction_binding()?;
        Ok(SceneInteraction {
            scene: self.scene,
            interaction: Some(interaction),
            plot: Some(self.plot),
        })
    }

    /// Opt into solid painted-geometry input, as required by Glory
    /// `chart_layout`. Reject unsupported paint/geometry before returning the
    /// candidate. The ordinary conversion retains legacy browser picking.
    pub fn into_painted_scene_interaction(self) -> Result<SceneInteraction> {
        let mut frame = self.into_scene_interaction()?;
        frame.interaction.as_mut().unwrap().picking_profile =
            crate::PickingProfile::PaintedGeometry;
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

/// Build measured live Widget slots with owned cell data, without editing the
/// layout. Visible actual cell marks invoke the callback in computed order,
/// including explicit null, zero-sized and zero-opacity cells. Hidden cells
/// and absent pairs do not invoke it. Returning None skips that cell.
///
/// The complete source/metadata preflight finishes before any callback. Source
/// identity, nullability, original input index, accepted geometry and opacity
/// must agree. Default-layout paint decoration remains available: use the
/// frame Mark for actual fill/stroke rather than assuming cell.color is a fill.
/// Custom factory layouts retain their stricter existing source guard.
///
/// The callback is borrowed, not retained. At most 128 returned slots are
/// admitted. Measured placement and host clip support are checked by the
/// receiving Chart/BrowserChart. Callback side effects outside candidate
/// scopes cannot be rolled back. For source replacement, pass this packet to
/// PlotWidgetsComposition and explicitly choose replace_ink IDs. Ordinary
/// typed slot APIs keep original ink. Live Widgets are not in Scene exports;
/// this API does not implement Nivo animatedProps or automatic springs.
pub fn heatmap_cell_widgets(
    layout: &HeatmapLayout,
    revision: u64,
    mut placement: impl FnMut(&HeatmapCellWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<HeatmapCellWidgetData>> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    // Reuse private business-source and custom factory provenance checks.
    layout.interaction_binding()?;
    ensure(
        layout.cells.len() == layout.input_cells.len()
            && layout.cells.len() == layout.accepted_opacities.len(),
        "HeatMap Widget source structure was changed",
    )?;
    let source: BTreeMap<_, _> = layout
        .input_cells
        .iter()
        .enumerate()
        .map(|(i, datum)| ((datum.row.as_str(), datum.column.as_str()), (i, datum)))
        .collect();
    ensure(
        source.len() == layout.input_cells.len(),
        "duplicate HeatMap Widget source",
    )?;
    let rows: BTreeSet<_> = layout.rows.iter().map(String::as_str).collect();
    let columns: BTreeSet<_> = layout.columns.iter().map(String::as_str).collect();
    let source_rows: BTreeSet<_> = source.keys().map(|key| key.0).collect();
    let source_columns: BTreeSet<_> = source.keys().map(|key| key.1).collect();
    ensure(
        rows.len() == layout.rows.len()
            && columns.len() == layout.columns.len()
            && rows.is_subset(&source_rows)
            && columns.is_subset(&source_columns),
        "HeatMap Widget identity order was changed",
    )?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut visible_ids = BTreeSet::new();
    let mut selected = Vec::new();
    for (i, cell) in layout.cells.iter().enumerate() {
        let (source_index, datum) = source
            .get(&(cell.key.row.as_str(), cell.key.column.as_str()))
            .copied()
            .ok_or_else(|| ChartError("HeatMap Widget source is missing".into()))?;
        ensure(
            cell.mark_id == cell.key.mark_id()
                && crate::widget_source_eq::optional(cell.value, datum.value)
                && layout.by_key.get(&cell.key) == Some(&i)
                && layout.by_mark.get(&cell.mark_id) == Some(&i),
            "HeatMap Widget source identity was changed",
        )?;
        ensure(
            layout.accepted_opacities[i] == cell.opacity,
            "HeatMap Widget accepted opacity was changed",
        )?;
        if !cell.visible {
            ensure(
                !marks.contains_key(cell.mark_id.as_str())
                    && cell.geometry.is_none()
                    && cell.band.is_none()
                    && cell.center.is_none(),
                "hidden HeatMap Widget cell has ink or geometry",
            )?;
            continue;
        }
        ensure(
            rows.contains(cell.key.row.as_str()) && columns.contains(cell.key.column.as_str()),
            "HeatMap Widget visible identity order is missing",
        )?;
        let mark = marks
            .get(cell.mark_id.as_str())
            .copied()
            .ok_or_else(|| ChartError("HeatMap Widget mark is missing".into()))?;
        ensure(
            cell.geometry.as_ref() == Some(&mark.geometry)
                && mark.opacity == cell.opacity
                && mark.series == cell.key.row
                && mark.value == cell.value.unwrap_or(0.)
                && mark.label == cell.label,
            "HeatMap Widget mark differs from its current cell",
        )?;
        visible_ids.insert(mark.id.as_str());
        selected.push((cell, mark, source_index, datum));
    }
    ensure(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("heatmap-cell:"))
            .all(|m| visible_ids.contains(m.id.as_str())),
        "orphan HeatMap Widget cell mark",
    )?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    for (cell, mark, source_index, datum) in selected {
        let context = HeatmapCellWidgetContext {
            layout,
            datum,
            source_index,
            cell,
            mark,
        };
        if let Some(placement) = placement(&context)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: mark.id.clone(),
                revision,
                placement,
            });
            packet.data.insert(
                mark.id.clone(),
                HeatmapCellWidgetData {
                    datum: datum.clone(),
                    source_index,
                    cell: cell.clone(),
                    plot: layout.plot,
                },
            );
        }
    }
    Ok(packet)
}
