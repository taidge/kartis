//! Renderer-independent custom cells reuse the validated Scatter/Line glyph contract.
use super::*;
/// The same checked single-mark Geometry/FillPaint representation used by
/// custom Scatter and Line points. Rects and circles stay centered at the
/// logical cell origin. Use a Path for intentionally offset visible ink.
pub use crate::ScatterNodeGeometry as HeatmapCellGeometry;
use crate::{
    FillPaint, InteractionAnchor, InteractionBinding, InteractionSite, ScatterNodeGeometry,
};

/// Original nullable source and already-resolved layout for one visible cell.
/// Factories run in configured row/column order, once per visible source pair.
/// Sparse absent pairs and hidden cells do not invoke the callback.
#[derive(Clone, Copy, Debug)]
pub struct HeatmapCellContext<'a> {
    pub datum: &'a HeatCell,
    pub key: &'a HeatmapCellKey,
    pub mark_id: &'a str,
    /// Position in the original input slice, before identity ordering.
    pub source_index: usize,
    pub formatted_value: Option<&'a str>,
    pub band: ClipRect,
    /// Logical data origin, independent of custom visible ink bounds.
    pub position: [f64; 2],
    pub plot: ClipRect,
    /// Includes configured shape, size variation and rectangle corners.
    pub geometry: &'a Geometry,
    /// Numeric color-scale result. Custom paints do not change scale domains.
    pub color: Color,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
}
impl HeatmapCellContext<'_> {
    /// Keep the original solid fill/border, replacing only cell geometry.
    /// Returned opacity is a per-cell factor multiplied by the ordinary
    /// base/active/inactive HeatMap opacity profile, including owned labels.
    pub fn inherit(&self, geometry: Geometry) -> HeatmapCellGeometry {
        ScatterNodeGeometry {
            geometry,
            fill: self.fill,
            fill_paint: None,
            stroke: self.stroke,
            stroke_dash: None,
            opacity: 1.,
            anchor_commands: None,
        }
    }
}

/// A temporary, fallible cell-geometry callback, borrowed only during layout.
pub type HeatmapCellFactory<'a> =
    dyn FnMut(&HeatmapCellContext<'_>) -> Result<HeatmapCellGeometry> + 'a;

/// Original typed source and default text for one visible owned label.
/// Neither custom text nor custom cell paint rewrites this source context.
#[derive(Clone, Copy, Debug)]
pub struct HeatmapLabelContext<'a> {
    /// The original input cell, retaining None separately from Some(0).
    pub datum: &'a HeatCell,
    pub key: &'a HeatmapCellKey,
    pub mark_id: &'a str,
    /// Position in the original input slice, before row/column ordering.
    pub source_index: usize,
    /// Numeric formatting result; None for an explicit null source.
    pub formatted_value: Option<&'a str>,
    /// Existing numeric text or configured empty_label, possibly empty.
    pub default_text: &'a str,
    pub band: ClipRect,
    /// Logical data origin, independent of custom cell geometry.
    pub position: [f64; 2],
    /// Default text position, including label offset and baseline adjustment.
    pub label_position: [f64; 2],
    pub plot: ClipRect,
}

/// A temporary text callback, borrowed only during layout construction.
/// Results retain the existing 1024-byte Scene text limit. Empty text keeps
/// an owned empty Label slot and draws no glyphs, matching default labels.
pub type HeatmapLabelFactory<'a> = dyn FnMut(&HeatmapLabelContext<'_>) -> Result<String> + 'a;

/// Optional typed factories, borrowed only for this layout build.
/// Hidden/sparse cells never invoke either callback; labels=false additionally
/// suppresses the label callback. For each visible cell, the label callback
/// runs before the cell callback. Errors stop construction; callback-owned
/// external side effects are not rolled back. No closure is retained.
///
/// ```
/// use kartis::*;
/// let cells = [HeatCell::new("row", "column", 0.)];
/// let mut text =
///     |ctx: &HeatmapLabelContext<'_>| Ok(format!("{}: {}", ctx.datum.row, ctx.default_text));
/// let layout = heatmap_layout_with_factories(
///     &ChartConfig::default(),
///     &cells,
///     &HeatmapLayoutOptions::default(),
///     HeatmapFactories {
///         cell: None,
///         label: Some(&mut text),
///     },
/// )?;
/// assert!(
///     layout
///         .scene
///         .labels
///         .iter()
///         .any(|label| label.text == "row: 0")
/// );
/// # Ok::<(), ChartError>(())
/// ```
#[derive(Default)]
pub struct HeatmapFactories<'a> {
    pub cell: Option<&'a mut HeatmapCellFactory<'a>>,
    pub label: Option<&'a mut HeatmapLabelFactory<'a>>,
}

/// Accepted custom source is private so controlled-hover and binding creation
/// can reject public data/geometry/paint corruption before publishing a frame.
#[derive(Clone, Debug)]
pub(super) struct CustomSource {
    cells: Vec<ComputedHeatmapCell>,
    marks: Vec<Option<Mark>>,
    labels: Vec<Option<Label>>,
    rows: Vec<String>,
    columns: Vec<String>,
    plot: ClipRect,
}

/// Business source for the newly exposed binding API, including the default
/// layout. Legacy Scene compatibility uses mark.value=0 for an explicit null;
/// that scalar must never be allowed to authenticate a forged Some(0).
/// Appearance fields are deliberately absent: default Scene decoration and
/// legacy controlled-hover behavior remain independent of this source guard.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct BindingSourceCell {
    key: HeatmapCellKey,
    mark_id: String,
    value: Option<f64>,
    formatted_value: Option<String>,
    label: String,
    band: Option<ClipRect>,
    center: Option<[f64; 2]>,
    visible: bool,
}
impl From<&ComputedHeatmapCell> for BindingSourceCell {
    fn from(cell: &ComputedHeatmapCell) -> Self {
        Self {
            key: cell.key.clone(),
            mark_id: cell.mark_id.clone(),
            value: cell.value,
            formatted_value: cell.formatted_value.clone(),
            label: cell.label.clone(),
            band: cell.band,
            center: cell.center,
            visible: cell.visible,
        }
    }
}
impl HeatmapLayout {
    pub(super) fn freeze_binding_source(&mut self) {
        self.binding_source = self.cells.iter().map(BindingSourceCell::from).collect();
    }

    fn validate_owned_label_row(&self, cell_index: usize, source_row: &str) -> Result<()> {
        let slot = self
            .label_slots
            .get(cell_index)
            .ok_or_else(|| ChartError("HeatMap owned-label source structure was changed".into()))?;
        if let Some(slot) = slot {
            ensure(
                self.scene
                    .label_series
                    .get(slot)
                    .is_some_and(|row| row == source_row),
                "HeatMap accepted label row ownership was changed",
            )?;
        }
        Ok(())
    }

    fn validate_binding_source(&self) -> Result<()> {
        ensure(
            self.cells.len() == self.binding_source.len()
                && self.label_slots.len() == self.binding_source.len(),
            "HeatMap binding source structure was changed",
        )?;
        for (i, (cell, source)) in self.cells.iter().zip(&self.binding_source).enumerate() {
            ensure(
                BindingSourceCell::from(cell) == *source
                    && crate::widget_source_eq::optional(cell.value, source.value),
                "HeatMap original nullable binding source was changed",
            )?;
            self.validate_owned_label_row(i, &source.key.row)?;
        }
        Ok(())
    }

    pub(super) fn freeze_custom_source(&mut self) {
        self.custom_source = Some(CustomSource {
            cells: self.cells.clone(),
            marks: self
                .mark_slots
                .iter()
                .map(|s| s.map(|i| self.scene.marks[i].clone()))
                .collect(),
            labels: self
                .label_slots
                .iter()
                .map(|s| s.map(|i| self.scene.labels[i].clone()))
                .collect(),
            rows: self.rows.clone(),
            columns: self.columns.clone(),
            plot: self.plot,
        });
    }
    pub(super) fn validate_custom_source(&self) -> Result<()> {
        let Some(source) = &self.custom_source else {
            return Ok(());
        };
        ensure(
            self.cells.len() == source.cells.len()
                && self.mark_slots.len() == source.marks.len()
                && self.label_slots.len() == source.labels.len()
                && self.base_label_colors.len() == source.cells.len()
                && self.rows == source.rows
                && self.columns == source.columns
                && self.plot == source.plot,
            "custom HeatMap source structure was changed",
        )?;
        for (i, cell) in self.cells.iter().enumerate() {
            self.validate_owned_label_row(i, &source.cells[i].key.row)?;
            ensure(
                self.accepted_opacities.get(i) == Some(&cell.opacity),
                "custom HeatMap accepted opacity was changed",
            )?;
            let mut expected = source.cells[i].clone();
            expected.opacity = cell.opacity;
            ensure(
                *cell == expected && crate::widget_source_eq::optional(cell.value, expected.value),
                "custom HeatMap source metadata was changed",
            )?;
            if let Some(mark) = &source.marks[i] {
                let mut expected = mark.clone();
                expected.opacity = cell.opacity;
                ensure(
                    self.mark_slots[i].and_then(|s| self.scene.marks.get(s)) == Some(&expected),
                    "custom HeatMap accepted geometry or paint was changed",
                )?;
            }
            if let Some(label) = &source.labels[i] {
                let mut expected = label.clone();
                let Color(r, g, b, a) = self.base_label_colors[i];
                expected.color = Color(r, g, b, (f64::from(a) * cell.opacity).round() as u8);
                ensure(
                    self.label_slots[i].and_then(|s| self.scene.labels.get(s)) == Some(&expected),
                    "custom HeatMap accepted owned label was changed",
                )?;
            }
        }
        Ok(())
    }

    /// Nullable typed business metadata and original logical cell positions,
    /// with the same Path-following anchor descriptors as Scatter/Line.
    /// Exact geometry picking still uses the actual Scene ink.
    /// Both default and custom layouts validate private source identity,
    /// nullability, formatted values, band/origin and owned-label rows. Default
    /// mark/label appearance and ordinary Scene decorations remain editable;
    /// custom layouts additionally retain their accepted factory frame guard.
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        self.validate_binding_source()?;
        self.validate_custom_source()?;
        let mut anchors = self.anchors.clone();
        let marks = self
            .scene
            .marks
            .iter()
            .map(|m| (m.id.as_str(), m))
            .collect::<BTreeMap<_, _>>();
        for cell in self.cells.iter().filter(|c| c.visible) {
            let mark = marks
                .get(cell.mark_id.as_str())
                .copied()
                .ok_or_else(|| ChartError("HeatMap data mark is missing".into()))?;
            let position = cell
                .center
                .ok_or_else(|| ChartError("visible HeatMap cell has no logical position".into()))?;
            if !anchors.contains_key(&cell.mark_id)
                && crate::crosshair::mark_center(mark) != Some(position)
            {
                let node = ScatterNodeGeometry {
                    geometry: mark.geometry.clone(),
                    fill: mark.fill,
                    fill_paint: mark.fill_paint.clone(),
                    stroke: mark.stroke,
                    stroke_dash: mark.stroke_dash.clone(),
                    opacity: mark.opacity,
                    anchor_commands: None,
                };
                if let Some(anchor) = node
                    .prepare_at(&mark.id, position, cell.color, 200_000)?
                    .anchor
                {
                    anchors.insert(cell.mark_id.clone(), anchor);
                }
            }
        }
        let result = InteractionBinding {
            sites: self
                .cells
                .iter()
                .filter(|c| c.visible)
                .map(|c| InteractionSite {
                    mark_id: c.mark_id.clone(),
                    series: c.key.row.clone(),
                    label: c.label.clone(),
                    value: c.value,
                    formatted_value: c.formatted_value.clone(),
                    color: c.color,
                    position: c.center.unwrap(),
                })
                .collect(),
            anchors,
            ..Default::default()
        };
        result.validate(&self.scene)?;
        Ok(result)
    }
}

pub(super) fn command_count(geometry: &Geometry) -> usize {
    if let Geometry::Path(path) = geometry {
        path.len()
    } else {
        0
    }
}
pub(super) fn paint_stops(paint: Option<&FillPaint>) -> usize {
    paint.map_or(0, |p| p.stops().len())
}

// Keep the anchor type in the private field declarations explicit.
pub(super) type CellAnchors = BTreeMap<String, InteractionAnchor>;
