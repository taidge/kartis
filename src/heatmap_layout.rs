//! Nullable HeatMap cells with independent band padding and exact identities.
//! References: nivo/packages/heatmap/src/{compute,hooks,types}.ts. Existing
//! `heatmap` defaults are unchanged. Sparse, absent pairs are not synthesized.
use crate::{
    ChartConfig, ChartError, ClipRect, Color, ColorInterpolation, ColorScale, ColorScaleOptions,
    ColorScaleProfile, Geometry, HeatCell, Label, LegendItem, Mark, Result, Scene, TextAnchor,
    TooltipDatum,
    axes::{CategoryAxisOptions, ValueFormat},
    scene::ensure,
};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::collections::{BTreeMap, BTreeSet};

mod custom;
pub use custom::{
    HeatmapCellContext, HeatmapCellFactory, HeatmapCellGeometry, HeatmapFactories,
    HeatmapLabelContext, HeatmapLabelFactory,
};

pub const MAX_HEATMAP_CELLS: usize = 20_000;
pub const MAX_HEATMAP_AXIS_TICKS: usize = 4_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HeatmapCellKey {
    pub row: String,
    pub column: String,
}
impl HeatmapCellKey {
    pub fn new(row: impl Into<String>, column: impl Into<String>) -> Self {
        Self {
            row: row.into(),
            column: column.into(),
        }
    }
    pub fn mark_id(&self) -> String {
        format!(
            "heatmap-cell:{}:{}:{}:{}",
            self.row.len(),
            self.row,
            self.column.len(),
            self.column
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeatmapShape {
    #[default]
    Rect,
    Circle,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct HeatmapBandPadding {
    pub inner: f64,
    pub outer: f64,
}
impl HeatmapBandPadding {
    fn validate(self) -> Result<()> {
        ensure(
            self.inner.is_finite()
                && (0. ..=1.).contains(&self.inner)
                && self.outer.is_finite()
                && (0. ..=1.).contains(&self.outer),
            "HeatMap band padding must be in 0..=1",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapSizeVariation {
    /// None uses the visible non-null extent. Equal values use the midpoint.
    pub domain: Option<[f64; 2]>,
    /// Linear multipliers of both band dimensions, in 0..=1. Clamped.
    pub sizes: [f64; 2],
}
impl Default for HeatmapSizeVariation {
    fn default() -> Self {
        Self {
            domain: None,
            sizes: [0.1, 1.],
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HeatmapHoverTarget {
    Cell,
    Row,
    Column,
    #[default]
    RowColumn,
    All,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HeatmapTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
}
impl HeatmapTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|s| s.is_finite() && (1. ..=256.).contains(&s)),
            "invalid HeatMap text size",
        )
    }
    fn size(&self, c: &ChartConfig) -> f64 {
        self.size.unwrap_or(c.theme.font_size)
    }
    fn color(&self, c: &ChartConfig) -> Color {
        self.color.unwrap_or(c.theme.text)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapAxis {
    pub options: CategoryAxisOptions,
    pub line: Option<GraphicStroke>,
    pub grid_color: Option<Color>,
    pub tick_size: f64,
    pub text: HeatmapTextStyle,
    pub legend_text: HeatmapTextStyle,
    pub legend_offset: f64,
}
impl Default for HeatmapAxis {
    fn default() -> Self {
        Self {
            options: CategoryAxisOptions::default(),
            line: Some(GraphicStroke {
                color: Color(110, 120, 135, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
            grid_color: None,
            tick_size: 4.,
            text: Default::default(),
            legend_text: Default::default(),
            legend_offset: 28.,
        }
    }
}
impl HeatmapAxis {
    fn validate(&self) -> Result<()> {
        self.options.validate()?;
        self.text.validate()?;
        self.legend_text.validate()?;
        validate_stroke(self.line)?;
        ensure(
            self.tick_size.is_finite()
                && (0. ..=1000.).contains(&self.tick_size)
                && self.legend_offset.is_finite()
                && (0. ..=16384.).contains(&self.legend_offset),
            "invalid HeatMap axis dimensions",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapAxes {
    pub top: Option<HeatmapAxis>,
    pub right: Option<HeatmapAxis>,
    pub bottom: Option<HeatmapAxis>,
    pub left: Option<HeatmapAxis>,
}
impl Default for HeatmapAxes {
    fn default() -> Self {
        Self {
            top: Some(Default::default()),
            right: None,
            bottom: None,
            left: Some(Default::default()),
        }
    }
}
impl HeatmapAxes {
    pub fn none() -> Self {
        Self {
            top: None,
            right: None,
            bottom: None,
            left: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct HeatmapLayoutOptions {
    pub shape: HeatmapShape,
    pub x_padding: HeatmapBandPadding,
    pub y_padding: HeatmapBandPadding,
    pub force_square: bool,
    pub size_variation: Option<HeatmapSizeVariation>,
    pub colors: ColorScaleOptions,
    pub corner_radius: f64,
    pub border: Option<GraphicStroke>,
    /// None keeps the fixed border. Some recolors only an existing border;
    /// inherited/surface use the numeric palette (unknown for explicit null),
    /// Theme uses its fixed color, and backdrop is the chart background.
    /// Real source contexts preflight even when hidden, zero-sized or without
    /// a border. No stroke is invented and mark opacity is applied separately.
    pub border_color_profile: Option<crate::InheritedColor>,
    pub labels: bool,
    pub label_style: HeatmapTextStyle,
    /// None preserves the existing uniform style.color/theme label color.
    /// Some resolves before custom geometry: inherited and surface use the
    /// numeric color-scale result (unknown for null), theme uses the existing
    /// label role color and backdrop is ChartConfig.theme.background.
    pub label_color_profile: Option<crate::InheritedColor>,
    pub label_offset: [f64; 2],
    pub value_format: ValueFormat,
    pub empty_label: String,
    /// None uses lexical ID order. Some must be a complete unique permutation
    /// of the source identities, including the identities subsequently hidden.
    pub row_order: Option<Vec<String>>,
    pub column_order: Option<Vec<String>>,
    pub row_labels: BTreeMap<String, String>,
    pub column_labels: BTreeMap<String, String>,
    pub hidden_rows: BTreeSet<String>,
    pub hidden_columns: BTreeSet<String>,
    pub axes: HeatmapAxes,
    /// Optional row visibility controls. This is not a numeric color legend.
    pub row_legend: bool,
    pub hover_target: HeatmapHoverTarget,
    pub active: Option<HeatmapCellKey>,
    pub opacity: f64,
    pub active_opacity: f64,
    pub inactive_opacity: f64,
}
impl Default for HeatmapLayoutOptions {
    fn default() -> Self {
        Self {
            shape: HeatmapShape::Rect,
            x_padding: Default::default(),
            y_padding: Default::default(),
            force_square: false,
            size_variation: None,
            colors: Default::default(),
            corner_radius: 0.,
            border: None,
            border_color_profile: None,
            labels: true,
            label_style: Default::default(),
            label_color_profile: None,
            label_offset: [0., 0.],
            value_format: ValueFormat::Auto,
            empty_label: "missing".into(),
            row_order: None,
            column_order: None,
            row_labels: BTreeMap::new(),
            column_labels: BTreeMap::new(),
            hidden_rows: BTreeSet::new(),
            hidden_columns: BTreeSet::new(),
            axes: Default::default(),
            row_legend: false,
            hover_target: Default::default(),
            active: None,
            opacity: 1.,
            active_opacity: 1.,
            inactive_opacity: 0.15,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedHeatmapCell {
    pub key: HeatmapCellKey,
    pub mark_id: String,
    /// Null remains distinct from zero. Scene marks require a finite scalar;
    /// their null compatibility value is zero, with an explicit missing label.
    pub value: Option<f64>,
    pub formatted_value: Option<String>,
    pub label: String,
    /// Hidden rows/columns have no projected band or geometry.
    pub band: Option<ClipRect>,
    pub center: Option<[f64; 2]>,
    pub geometry: Option<Geometry>,
    pub color: Color,
    pub opacity: f64,
    pub visible: bool,
}
impl ComputedHeatmapCell {
    pub fn tooltip_datum(&self) -> TooltipDatum<'_> {
        TooltipDatum {
            id: &self.mark_id,
            label: &self.label,
            series: &self.key.row,
            value: self.value,
        }
    }
}
#[derive(Clone, Debug)]
pub struct HeatmapLayout {
    pub scene: Scene,
    pub cells: Vec<ComputedHeatmapCell>,
    pub rows: Vec<String>,
    pub columns: Vec<String>,
    pub plot: ClipRect,
    pub color_scale: ColorScale,
    pub hover_target: HeatmapHoverTarget,
    pub active: Option<HeatmapCellKey>,
    pub active_ids: Vec<String>,
    by_key: BTreeMap<HeatmapCellKey, usize>,
    by_mark: BTreeMap<String, usize>,
    mark_slots: Vec<Option<usize>>,
    label_slots: Vec<Option<usize>>,
    base_label_colors: Vec<Color>,
    empty_label: String,
    factory_label_texts: Option<Vec<Option<String>>>,
    opacities: [f64; 3],
    cell_factors: Vec<f64>,
    accepted_opacities: Vec<f64>,
    anchors: custom::CellAnchors,
    custom_source: Option<custom::CustomSource>,
    binding_source: Vec<custom::BindingSourceCell>,
}
impl HeatmapLayout {
    pub fn cell(&self, key: &HeatmapCellKey) -> Option<&ComputedHeatmapCell> {
        self.by_key
            .get(key)
            .and_then(|i| self.cells.get(*i))
            .filter(|c| &c.key == key)
    }
    pub fn hit_test(&self, x: f64, y: f64) -> Option<&ComputedHeatmapCell> {
        let m = self.scene.hit_test(x, y)?;
        self.by_mark
            .get(&m.id)
            .and_then(|i| self.cells.get(*i))
            .filter(|c| c.mark_id == m.id)
    }
    /// Exact associations on visible source cells; sparse absent pairs stay
    /// absent, while explicit null cells participate normally.
    pub fn related(
        &self,
        key: &HeatmapCellKey,
        target: HeatmapHoverTarget,
    ) -> Result<Vec<&ComputedHeatmapCell>> {
        let source = self
            .cell(key)
            .ok_or_else(|| ChartError("unknown active HeatMap cell".into()))?;
        if !source.visible {
            return Ok(vec![]);
        }
        Ok(self
            .cells
            .iter()
            .filter(|c| c.visible && related(&c.key, key, target))
            .collect())
    }
    pub fn with_active(&self, active: Option<HeatmapCellKey>) -> Result<Self> {
        let mut next = self.clone();
        next.set_active(active)?;
        Ok(next)
    }
    /// Transactional controlled hover update. No geometry, domains or identity
    /// is recomputed. Unknown IDs reject; an existing hidden key has no peers.
    pub fn set_active(&mut self, active: Option<HeatmapCellKey>) -> Result<()> {
        self.validate_custom_source()?;
        ensure(
            self.cells.len() == self.mark_slots.len()
                && self.cells.len() == self.label_slots.len()
                && self.cells.len() == self.base_label_colors.len(),
            "HeatMap public metadata was structurally changed",
        )?;
        ensure(
            self.factory_label_texts
                .as_ref()
                .is_none_or(|texts| texts.len() == self.cells.len()),
            "HeatMap accepted label text structure was changed",
        )?;
        let ids = match &active {
            Some(key) => self
                .related(key, self.hover_target)?
                .into_iter()
                .map(|c| c.mark_id.clone())
                .collect::<Vec<_>>(),
            None => vec![],
        };
        let active_ids: BTreeSet<_> = ids.iter().map(String::as_str).collect();
        let mut scene = self.scene.clone();
        let mut opacities = Vec::with_capacity(self.cells.len());
        for (i, c) in self.cells.iter().enumerate() {
            ensure(
                self.by_key.get(&c.key) == Some(&i) && self.by_mark.get(&c.mark_id) == Some(&i),
                "HeatMap public cell identity was changed",
            )?;
            let opacity = self.cell_factors[i]
                * if ids.is_empty() {
                    self.opacities[0]
                } else if active_ids.contains(c.mark_id.as_str()) {
                    self.opacities[1]
                } else {
                    self.opacities[2]
                };
            if let Some(slot) = self.mark_slots[i] {
                let mark = scene.marks.get_mut(slot).ok_or_else(|| {
                    ChartError("HeatMap public Scene marks were structurally changed".into())
                })?;
                ensure(
                    mark.id == c.mark_id,
                    "HeatMap public Scene mark identity was changed",
                )?;
                mark.opacity = opacity;
            }
            if let Some(slot) = self.label_slots[i] {
                ensure(
                    scene.label_series.get(&slot) == Some(&c.key.row),
                    "HeatMap public label ownership was changed",
                )?;
                let Color(r, g, b, a) = self.base_label_colors[i];
                let label = scene.labels.get_mut(slot).ok_or_else(|| {
                    ChartError("HeatMap public Scene labels were structurally changed".into())
                })?;
                let expected_text = if let Some(texts) = &self.factory_label_texts {
                    texts[i].as_deref().ok_or_else(|| {
                        ChartError("HeatMap accepted label text is missing".into())
                    })?
                } else {
                    c.formatted_value.as_deref().unwrap_or(&self.empty_label)
                };
                ensure(
                    label.text == expected_text,
                    "HeatMap public label text was changed",
                )?;
                label.color = Color(r, g, b, (f64::from(a) * opacity).round() as u8);
            }
            opacities.push(opacity);
        }
        scene.validate()?;
        self.scene = scene;
        self.active = active;
        self.active_ids = ids;
        self.accepted_opacities = opacities.clone();
        for (c, opacity) in self.cells.iter_mut().zip(opacities) {
            c.opacity = opacity;
        }
        Ok(())
    }
}
fn related(a: &HeatmapCellKey, b: &HeatmapCellKey, t: HeatmapHoverTarget) -> bool {
    match t {
        HeatmapHoverTarget::Cell => a == b,
        HeatmapHoverTarget::Row => a.row == b.row,
        HeatmapHoverTarget::Column => a.column == b.column,
        HeatmapHoverTarget::RowColumn => a.row == b.row || a.column == b.column,
        HeatmapHoverTarget::All => true,
    }
}
fn validate_stroke(stroke: Option<GraphicStroke>) -> Result<()> {
    ensure(
        stroke.is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width)),
        "invalid HeatMap stroke",
    )
}
fn validate_options(o: &HeatmapLayoutOptions) -> Result<()> {
    if let Some(profile) = &o.label_color_profile {
        // Validate even without data, visible cells or enabled labels.
        profile.validate()?;
    }
    if let Some(profile) = &o.border_color_profile {
        profile.validate()?;
    }
    o.x_padding.validate()?;
    o.y_padding.validate()?;
    validate_stroke(o.border)?;
    o.label_style.validate()?;
    o.value_format.validate()?;
    ensure(
        o.corner_radius.is_finite()
            && (0. ..=16384.).contains(&o.corner_radius)
            && o.label_offset
                .iter()
                .all(|x| x.is_finite() && x.abs() <= 16384.)
            && o.empty_label.len() <= 512,
        "invalid HeatMap label or corner style",
    )?;
    ensure(
        [o.opacity, o.active_opacity, o.inactive_opacity]
            .iter()
            .all(|x| x.is_finite() && (0. ..=1.).contains(x)),
        "invalid HeatMap opacity",
    )?;
    if let Some(size) = &o.size_variation {
        ensure(
            size.sizes
                .iter()
                .all(|x| x.is_finite() && (0. ..=1.).contains(x))
                && size.domain.is_none_or(|d| d.iter().all(|x| x.is_finite())),
            "invalid HeatMap size variation",
        )?;
    }
    let mut ticks = 0usize;
    for a in [&o.axes.top, &o.axes.right, &o.axes.bottom, &o.axes.left]
        .into_iter()
        .flatten()
    {
        a.validate()?;
        ticks += a
            .options
            .tick_values
            .as_ref()
            .map_or(a.options.tick_count, Vec::len);
    }
    ensure(
        ticks <= MAX_HEATMAP_AXIS_TICKS,
        "HeatMap axis tick budget exceeded",
    )
}
fn order(explicit: &Option<Vec<String>>, ids: &BTreeSet<String>) -> Result<Vec<String>> {
    if let Some(order) = explicit {
        ensure(
            order.len() == ids.len()
                && order.iter().collect::<BTreeSet<_>>().len() == ids.len()
                && order.iter().all(|id| ids.contains(id)),
            "HeatMap order must be a complete unique ID permutation",
        )?;
        Ok(order.clone())
    } else {
        Ok(ids.iter().cloned().collect())
    }
}
#[derive(Clone)]
struct Bands {
    ids: Vec<String>,
    positions: BTreeMap<String, f64>,
    breadth: f64,
}
fn bands(ids: Vec<String>, start: f64, length: f64, padding: HeatmapBandPadding) -> Bands {
    let n = ids.len() as f64;
    let step = length / (n - padding.inner + padding.outer * 2.).max(1.);
    let offset = (length - step * (n - padding.inner)) / 2.;
    let breadth = step * (1. - padding.inner);
    let positions = ids
        .iter()
        .enumerate()
        .map(|(i, id)| (id.clone(), start + offset + i as f64 * step))
        .collect();
    Bands {
        ids,
        positions,
        breadth,
    }
}
fn extent(values: impl Iterator<Item = f64>) -> [f64; 2] {
    let mut d = [f64::INFINITY, f64::NEG_INFINITY];
    for v in values {
        d[0] = d[0].min(v);
        d[1] = d[1].max(v);
    }
    if d[0].is_finite() { d } else { [0., 1.] }
}
fn fraction(v: f64, d: [f64; 2]) -> f64 {
    if d[0] == d[1] {
        return 0.5;
    }
    // Scale first so opposite large finite endpoints cannot overflow a span.
    let magnitude = d[0].abs().max(d[1].abs()).max(v.abs());
    ((v / magnitude - d[0] / magnitude) / (d[1] / magnitude - d[0] / magnitude)).clamp(0., 1.)
}

pub fn heatmap_layout(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
) -> Result<HeatmapLayout> {
    heatmap_layout_with_color_profile(c, cells, o, ColorInterpolation::Srgb)
}

/// Use the selected color space for cell ink, metadata and the resolved scale.
pub fn heatmap_layout_with_color_profile(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    profile: impl Into<ColorScaleProfile>,
) -> Result<HeatmapLayout> {
    heatmap_layout_with_color_profile_and_factories(
        c,
        cells,
        o,
        profile,
        HeatmapFactories::default(),
    )
}

/// Replace visible cells with one checked Scene mark each. Null cells invoke
/// the factory and retain value=None in typed metadata. Labels remain owned
/// by the original row and keep the existing value/empty-label configuration.
pub fn heatmap_layout_with_cell_factory(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    factory: impl FnMut(&HeatmapCellContext<'_>) -> Result<HeatmapCellGeometry>,
) -> Result<HeatmapLayout> {
    heatmap_layout_with_color_profile_and_cell_factory(
        c,
        cells,
        o,
        ColorInterpolation::Srgb,
        factory,
    )
}

/// Select the same shared color-scale profile as the default layout while
/// replacing actual geometry/paint. Custom paints do not rewrite scale data.
pub fn heatmap_layout_with_color_profile_and_cell_factory(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    profile: impl Into<ColorScaleProfile>,
    mut factory: impl FnMut(&HeatmapCellContext<'_>) -> Result<HeatmapCellGeometry>,
) -> Result<HeatmapLayout> {
    heatmap_layout_with_color_profile_and_factories(
        c,
        cells,
        o,
        profile,
        HeatmapFactories {
            cell: Some(&mut factory),
            label: None,
        },
    )
}

/// Combine fallible typed owned-label text and cell geometry factories.
/// Default factories preserve the existing Scene, SVG and Canvas output.
/// A supplied factory freezes accepted source/paint/owned labels for checked
/// interaction updates. Raw values, tooltip text, numeric formatting and
/// pre-factory inherited label-color roles remain unchanged.
pub fn heatmap_layout_with_factories(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    factories: HeatmapFactories<'_>,
) -> Result<HeatmapLayout> {
    heatmap_layout_with_color_profile_and_factories(
        c,
        cells,
        o,
        ColorInterpolation::Srgb,
        factories,
    )
}

/// Combine typed factories with the shared numeric color-scale profile.
/// Callbacks borrow original source only during construction and are never
/// stored in the returned layout. See HeatmapFactories for order/error rules.
pub fn heatmap_layout_with_color_profile_and_factories(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    profile: impl Into<ColorScaleProfile>,
    factories: HeatmapFactories<'_>,
) -> Result<HeatmapLayout> {
    heatmap_layout_impl(c, cells, o, profile.into(), factories)
}

fn heatmap_layout_impl(
    c: &ChartConfig,
    cells: &[HeatCell],
    o: &HeatmapLayoutOptions,
    profile: ColorScaleProfile,
    factories: HeatmapFactories<'_>,
) -> Result<HeatmapLayout> {
    let HeatmapFactories {
        cell: mut factory,
        label: mut label_factory,
    } = factories;
    c.validate()?;
    validate_options(o)?;
    ensure(
        cells.len() <= MAX_HEATMAP_CELLS,
        "HeatMap cell budget exceeded",
    )?;
    let mut rows = BTreeSet::new();
    let mut columns = BTreeSet::new();
    let mut source = BTreeMap::new();
    let mut source_indices = BTreeMap::new();
    for (source_index, cell) in cells.iter().enumerate() {
        ensure(
            [&cell.row, &cell.column]
                .iter()
                .all(|id| !id.trim().is_empty() && id.len() <= 256),
            "HeatMap IDs need 1..=256 bytes",
        )?;
        ensure(
            cell.value.is_none_or(f64::is_finite),
            "HeatMap values must be finite or null",
        )?;
        let key = HeatmapCellKey::new(&cell.row, &cell.column);
        ensure(
            source.insert(key, cell).is_none(),
            "duplicate HeatMap row/column cell",
        )?;
        source_indices.insert(HeatmapCellKey::new(&cell.row, &cell.column), source_index);
        rows.insert(cell.row.clone());
        columns.insert(cell.column.clone());
    }
    ensure(
        o.hidden_rows.len() <= rows.len()
            && o.hidden_columns.len() <= columns.len()
            && o.row_labels.len() <= rows.len()
            && o.column_labels.len() <= columns.len(),
        "HeatMap identity options exceed input budget",
    )?;
    ensure(
        o.hidden_rows
            .iter()
            .chain(o.row_labels.keys())
            .all(|id| rows.contains(id))
            && o.hidden_columns
                .iter()
                .chain(o.column_labels.keys())
                .all(|id| columns.contains(id)),
        "HeatMap identity options refer to unknown IDs",
    )?;
    ensure(
        o.row_labels
            .values()
            .chain(o.column_labels.values())
            .all(|text| text.len() <= 512),
        "HeatMap display label budget exceeded",
    )?;
    let all_rows = order(&o.row_order, &rows)?;
    let all_columns = order(&o.column_order, &columns)?;
    if let Some(key) = &o.active {
        ensure(source.contains_key(key), "unknown active HeatMap cell")?;
    }
    ensure(
        !o.row_legend || all_rows.len() <= 256,
        "HeatMap row legend exceeds 256 entries",
    )?;
    for (axis, ids) in [
        (&o.axes.top, &columns),
        (&o.axes.bottom, &columns),
        (&o.axes.left, &rows),
        (&o.axes.right, &rows),
    ] {
        if let Some(a) = axis {
            ensure(
                a.options
                    .tick_values
                    .as_ref()
                    .is_none_or(|values| values.iter().all(|id| ids.contains(id))),
                "HeatMap axis refers to an unknown category",
            )?;
        }
    }
    let visible_rows: Vec<_> = all_rows
        .iter()
        .filter(|id| !o.hidden_rows.contains(*id))
        .cloned()
        .collect();
    let visible_columns: Vec<_> = all_columns
        .iter()
        .filter(|id| !o.hidden_columns.contains(*id))
        .cloned()
        .collect();
    let visible = |cell: &&HeatCell| {
        !o.hidden_rows.contains(&cell.row) && !o.hidden_columns.contains(&cell.column)
    };
    let color_scale = o
        .colors
        .resolve_with_profile(cells.iter().filter(visible).map(|cell| cell.value), profile)?;
    // Match Nivo useCellsStyle: resolve source data color then labelTextColor
    // before invoking any custom renderer/factory. Hidden and labels=false
    // sources still preflight their actual contrast context. An empty source
    // has no fabricated palette context; intrinsic profile validation suffices.
    let label_color_by_key = source
        .iter()
        .map(|(key, cell)| {
            let inherited = color_scale.map(cell.value);
            let color = if let Some(profile) = &o.label_color_profile {
                profile.resolve_validated(crate::InheritedColorContext {
                    inherited,
                    theme: o.label_style.color(c),
                    surface: inherited,
                    backdrop: c.theme.background,
                })?
            } else {
                o.label_style.color(c)
            };
            Ok((key.clone(), color))
        })
        .collect::<Result<BTreeMap<_, _>>>()?;
    let border_by_key = if let Some(profile) = &o.border_color_profile {
        Some(
            source
                .iter()
                .map(|(key, cell)| {
                    let inherited = color_scale.map(cell.value);
                    let color = profile.resolve_validated(crate::InheritedColorContext {
                        inherited,
                        theme: o.border.map_or(inherited, |border| border.color),
                        surface: inherited,
                        backdrop: c.theme.background,
                    })?;
                    Ok((
                        key.clone(),
                        o.border.map(|mut border| {
                            border.color = color;
                            border
                        }),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        )
    } else {
        None
    };
    let size_domain = o
        .size_variation
        .as_ref()
        .and_then(|s| s.domain)
        .unwrap_or_else(|| extent(cells.iter().filter(visible).filter_map(|cell| cell.value)));
    let mut plot = ClipRect::plot(c);
    if o.force_square && !visible_rows.is_empty() && !visible_columns.is_empty() {
        let size = (plot.width / visible_columns.len() as f64)
            .min(plot.height / visible_rows.len() as f64);
        let w = size * visible_columns.len() as f64;
        let h = size * visible_rows.len() as f64;
        plot.x += (plot.width - w) / 2.;
        plot.y += (plot.height - h) / 2.;
        plot.width = w;
        plot.height = h;
    }
    let x = bands(visible_columns.clone(), plot.x, plot.width, o.x_padding);
    let y = bands(visible_rows.clone(), plot.y, plot.height, o.y_padding);
    let mut scene = c.scene()?;
    for (side, axis) in [
        (Side::Top, &o.axes.top),
        (Side::Right, &o.axes.right),
        (Side::Bottom, &o.axes.bottom),
        (Side::Left, &o.axes.left),
    ] {
        if let Some(axis) = axis {
            axis_paint(
                &mut scene,
                c,
                plot,
                if side.horizontal() { &x } else { &y },
                if side.horizontal() {
                    &o.column_labels
                } else {
                    &o.row_labels
                },
                side,
                axis,
            )?;
        }
    }
    let custom_enabled = factory.is_some() || label_factory.is_some();
    let custom_label_enabled = label_factory.is_some();
    if custom_enabled {
        let visible_count = source.values().filter(|cell| visible(cell)).count();
        ensure(
            scene
                .marks
                .len()
                .checked_add(visible_count)
                .is_some_and(|count| count <= crate::scene::MAX_MARKS)
                && (!o.labels
                    || scene
                        .labels
                        .len()
                        .checked_add(visible_count)
                        .is_some_and(|count| count <= crate::scene::MAX_MARKS)),
            "HeatMap Scene mark or label budget exceeded before factories",
        )?;
    }
    let mut label_bytes = scene
        .labels
        .iter()
        .map(|label| label.text.len())
        .sum::<usize>();
    let mut anchors = BTreeMap::new();
    let mut cell_factors = vec![];
    let mut base_label_colors = vec![];
    let mut commands = scene
        .marks
        .iter()
        .map(|m| custom::command_count(&m.geometry))
        .sum::<usize>();
    let mut stops = scene
        .marks
        .iter()
        .map(|m| custom::paint_stops(m.fill_paint.as_ref()))
        .sum::<usize>();
    let mut computed = vec![];
    let mut mark_slots = vec![];
    let mut label_slots = vec![];
    let mut by_key = BTreeMap::new();
    let mut by_mark = BTreeMap::new();
    let row_rank: BTreeMap<_, _> = all_rows
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let column_rank: BTreeMap<_, _> = all_columns
        .iter()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    let mut entries: Vec<_> = source.into_iter().collect();
    entries.sort_by_key(|(key, _)| (row_rank[key.row.as_str()], column_rank[key.column.as_str()]));
    for (key, cell) in entries {
        let row = &key.row;
        let column = &key.column;
        let mark_id = key.mark_id();
        let value = cell.value;
        let formatted_value = value.map(|v| o.value_format.format(v)).transpose()?;
        let row_label = o.row_labels.get(row).unwrap_or(row);
        let column_label = o.column_labels.get(column).unwrap_or(column);
        let label = format!(
            "{row_label} · {column_label}: {}",
            formatted_value.as_deref().unwrap_or(&o.empty_label)
        );
        ensure(label.len() <= 1024, "HeatMap tooltip label budget exceeded")?;
        let mut color = color_scale.map(value);
        let label_color = label_color_by_key[&key];
        let border = border_by_key
            .as_ref()
            .map_or(o.border, |borders| borders[&key]);
        let band = match (x.positions.get(column), y.positions.get(row)) {
            (Some(px), Some(py)) => Some(ClipRect {
                x: *px,
                y: *py,
                width: x.breadth,
                height: y.breadth,
            }),
            _ => None,
        };
        let center = band.map(|r| [r.x + r.width / 2., r.y + r.height / 2.]);
        let mut geometry = band.map(|r| {
            let factor = match (&o.size_variation, value) {
                (Some(size), Some(value)) => {
                    let t = fraction(value, size_domain);
                    size.sizes[0] * (1. - t) + size.sizes[1] * t
                }
                _ => 1.,
            };
            let width = r.width * factor;
            let height = r.height * factor;
            let cx = r.x + r.width / 2.;
            let cy = r.y + r.height / 2.;
            match o.shape {
                HeatmapShape::Circle => Geometry::Circle {
                    x: cx,
                    y: cy,
                    radius: width.min(height) / 2.,
                },
                HeatmapShape::Rect => crate::cartesian::round_bar(
                    Geometry::Rect {
                        x: cx - width / 2.,
                        y: cy - height / 2.,
                        width,
                        height,
                    },
                    o.corner_radius,
                ),
            }
        });
        let mut mark_slot = None;
        let mut label_slot = None;
        let mut cell_factor = 1.;
        if let Some(default_geometry) = &geometry {
            let label_position = [
                center.unwrap()[0] + o.label_offset[0],
                center.unwrap()[1] + o.label_style.size(c) / 3. + o.label_offset[1],
            ];
            let text = if o.labels {
                let default_text = formatted_value.as_deref().unwrap_or(&o.empty_label);
                let text = if let Some(factory) = label_factory.as_deref_mut() {
                    factory(&HeatmapLabelContext {
                        datum: cell,
                        key: &key,
                        mark_id: &mark_id,
                        source_index: source_indices[&key],
                        formatted_value: formatted_value.as_deref(),
                        default_text,
                        band: band.unwrap(),
                        position: center.unwrap(),
                        label_position,
                        plot,
                    })?
                } else {
                    default_text.to_owned()
                };
                ensure(
                    text.len() <= crate::scene::MAX_LABEL_BYTES,
                    "HeatMap owned label text budget exceeded",
                )?;
                label_bytes = label_bytes
                    .checked_add(text.len())
                    .ok_or_else(|| ChartError("HeatMap label byte budget overflow".into()))?;
                ensure(
                    label_bytes <= crate::scene::MAX_MARKS * crate::scene::MAX_LABEL_BYTES,
                    "HeatMap cumulative label byte budget exceeded",
                )?;
                Some(text)
            } else {
                None
            };
            let mut mark = if let Some(factory) = factory.as_deref_mut() {
                let zero = match default_geometry {
                    Geometry::Circle { radius, .. } => *radius == 0.,
                    Geometry::Rect { width, height, .. } => *width == 0. || *height == 0.,
                    _ => false,
                };
                let context = HeatmapCellContext {
                    datum: cell,
                    key: &key,
                    mark_id: &mark_id,
                    source_index: source_indices[&key],
                    formatted_value: formatted_value.as_deref(),
                    band: band.unwrap(),
                    position: center.unwrap(),
                    plot,
                    geometry: default_geometry,
                    color,
                    fill: (!zero).then_some(color),
                    stroke: if zero { None } else { border },
                };
                let prepared = factory(&context)?.prepare_at(
                    &mark_id,
                    context.position,
                    color,
                    200_000usize.saturating_sub(commands),
                )?;
                color = prepared.color;
                cell_factor = prepared.mark.opacity;
                ensure(
                    cell_factor.is_finite() && (0. ..=1.).contains(&cell_factor),
                    "custom HeatMap cell opacity must be in 0..=1",
                )?;
                if let Some(anchor) = prepared.anchor {
                    anchors.insert(mark_id.clone(), anchor);
                }
                let mut mark = prepared.mark;
                mark.clip = Some(plot);
                mark
            } else {
                let mut mark = Mark::new(mark_id.clone(), default_geometry.clone(), color);
                mark.stroke = border;
                let zero = match default_geometry {
                    Geometry::Circle { radius, .. } => *radius == 0.,
                    Geometry::Rect { width, height, .. } => *width == 0. || *height == 0.,
                    _ => false,
                };
                if zero {
                    mark.fill = None;
                    mark.stroke = None;
                }
                mark
            };
            commands = commands
                .checked_add(custom::command_count(&mark.geometry))
                .ok_or_else(|| ChartError("HeatMap path command budget overflow".into()))?;
            stops += custom::paint_stops(mark.fill_paint.as_ref());
            ensure(
                commands <= 200_000 && stops <= crate::MAX_SCENE_GRADIENT_STOPS,
                "HeatMap geometry or gradient stop budget exceeded",
            )?;
            geometry = Some(mark.geometry.clone());
            mark.series = row.clone();
            mark.value = value.unwrap_or(0.);
            mark.label = label.clone();
            mark.opacity = o.opacity * cell_factor;
            mark_slot = Some(scene.marks.len());
            scene.marks.push(mark);
            if o.labels {
                label_slot = Some(scene.labels.len());
                scene.label_series.insert(scene.labels.len(), row.clone());
                scene.labels.push(Label {
                    x: label_position[0],
                    y: label_position[1],
                    text: text.unwrap(),
                    color: label_color,
                    size: o.label_style.size(c),
                    anchor: TextAnchor::Middle,
                    rotation: 0.,
                });
            }
        }
        by_mark.insert(mark_id.clone(), computed.len());
        by_key.insert(key.clone(), computed.len());
        computed.push(ComputedHeatmapCell {
            key,
            mark_id,
            value,
            formatted_value,
            label,
            band,
            center,
            geometry,
            color,
            opacity: o.opacity,
            visible: band.is_some(),
        });
        cell_factors.push(cell_factor);
        base_label_colors.push(label_color);
        mark_slots.push(mark_slot);
        label_slots.push(label_slot);
    }
    if o.row_legend {
        for row in &all_rows {
            scene.legend.push(LegendItem {
                id: row.clone(),
                label: o.row_labels.get(row).unwrap_or(row).clone(),
                color: c.theme.text,
            });
        }
    }
    scene.validate()?;
    let factory_label_texts = custom_label_enabled.then(|| {
        label_slots
            .iter()
            .map(|slot| slot.map(|i| scene.labels[i].text.clone()))
            .collect()
    });
    let mut layout = HeatmapLayout {
        scene,
        cells: computed,
        rows: visible_rows,
        columns: visible_columns,
        plot,
        color_scale,
        hover_target: o.hover_target,
        active: None,
        active_ids: vec![],
        by_key,
        by_mark,
        mark_slots,
        label_slots,
        base_label_colors,
        empty_label: o.empty_label.clone(),
        factory_label_texts,
        opacities: [o.opacity, o.active_opacity, o.inactive_opacity],
        cell_factors,
        accepted_opacities: vec![],
        anchors,
        custom_source: None,
        binding_source: vec![],
    };
    layout.set_active(o.active.clone())?;
    layout.freeze_binding_source();
    if custom_enabled {
        layout.freeze_custom_source();
        layout.interaction_binding()?;
    }
    Ok(layout)
}

#[derive(Clone, Copy)]
enum Side {
    Top,
    Right,
    Bottom,
    Left,
}
impl Side {
    fn horizontal(self) -> bool {
        matches!(self, Self::Top | Self::Bottom)
    }
    fn name(self) -> &'static str {
        match self {
            Self::Top => "top",
            Self::Right => "right",
            Self::Bottom => "bottom",
            Self::Left => "left",
        }
    }
}
fn axis_paint(
    scene: &mut Scene,
    c: &ChartConfig,
    plot: ClipRect,
    bands: &Bands,
    labels: &BTreeMap<String, String>,
    side: Side,
    a: &HeatmapAxis,
) -> Result<()> {
    if !a.options.visible || bands.ids.is_empty() {
        return Ok(());
    }
    let ticks = if let Some(values) = &a.options.tick_values {
        values
            .iter()
            .filter(|id| bands.positions.contains_key(*id))
            .cloned()
            .collect::<Vec<_>>()
    } else {
        bands
            .ids
            .iter()
            .step_by(bands.ids.len().div_ceil(a.options.tick_count.max(1)).max(1))
            .take(if a.options.tick_count == 0 {
                0
            } else {
                usize::MAX
            })
            .cloned()
            .collect()
    };
    let horizontal = side.horizontal();
    let before = matches!(side, Side::Top | Side::Left);
    let sign = if before { -1. } else { 1. };
    let fixed = match side {
        Side::Top => plot.y,
        Side::Right => plot.x + plot.width,
        Side::Bottom => plot.y + plot.height,
        Side::Left => plot.x,
    };
    let (start, end) = if horizontal {
        (plot.x, plot.x + plot.width)
    } else {
        (plot.y, plot.y + plot.height)
    };
    let p = |along: f64, across: f64| {
        if horizontal {
            P::new(along, across)
        } else {
            P::new(across, along)
        }
    };
    let mut commands = vec![C::Move(p(start, fixed)), C::Line(p(end, fixed))];
    let mut grid = vec![];
    for id in ticks {
        let along = bands.positions[&id] + bands.breadth / 2.;
        commands.push(C::Move(p(along, fixed)));
        commands.push(C::Line(p(along, fixed + sign * a.tick_size)));
        if a.options.grid {
            grid.push(C::Move(p(along, if horizontal { plot.y } else { plot.x })));
            grid.push(C::Line(p(
                along,
                if horizontal {
                    plot.y + plot.height
                } else {
                    plot.x + plot.width
                },
            )));
        }
        let size = a.text.size(c);
        let offset = a.tick_size + a.options.tick_padding;
        let (x, y, anchor) = if horizontal {
            (
                along,
                fixed + sign * offset + if before { 0. } else { size },
                TextAnchor::Middle,
            )
        } else {
            (
                fixed + sign * offset,
                along + size / 3.,
                if before {
                    TextAnchor::End
                } else {
                    TextAnchor::Start
                },
            )
        };
        scene.labels.push(Label {
            x,
            y,
            text: labels.get(&id).unwrap_or(&id).clone(),
            color: a.text.color(c),
            size,
            anchor,
            rotation: a.options.tick_rotation,
        });
    }
    for (id, path, stroke) in [
        (
            format!("heatmap-grid:{}", side.name()),
            grid,
            Some(GraphicStroke {
                color: a.grid_color.unwrap_or(c.theme.grid),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
        ),
        (format!("heatmap-axis:{}", side.name()), commands, a.line),
    ] {
        if !path.is_empty()
            && let Some(stroke) = stroke
        {
            let mut mark = Mark::new(id, Geometry::Path(path), stroke.color);
            mark.fill = None;
            mark.stroke = Some(stroke);
            mark.label.clear();
            scene.marks.push(mark);
        }
    }
    if let Some(text) = &a.options.legend {
        let across = fixed
            + sign * (a.tick_size + a.options.tick_padding + a.text.size(c) + a.legend_offset);
        let point = p((start + end) / 2., across);
        scene.labels.push(Label {
            x: point.x,
            y: point.y,
            text: text.clone(),
            color: a.legend_text.color(c),
            size: a.legend_text.size(c),
            anchor: TextAnchor::Middle,
            rotation: if horizontal {
                0.
            } else if before {
                -90.
            } else {
                90.
            },
        });
    }
    Ok(())
}
