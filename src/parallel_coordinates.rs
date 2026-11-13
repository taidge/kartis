//! Configurable parallel coordinates. Nivo references: parallel-coordinates/
//! src/{defaults,types,hooks}.ts. Each variable is scaled independently before
//! the existing curve engine receives the shared screen-coordinate geometry.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::scene::ensure;
use crate::*;

pub const MAX_PARALLEL_DIMENSIONS: usize = 64;
pub const MAX_PARALLEL_RECORDS: usize = 2048;
pub const MAX_PARALLEL_AXIS_TICKS: usize = 8192;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallelLayout {
    /// Vertical variable axes, ordered from left to right (Nivo horizontal).
    #[default]
    Horizontal,
    /// Horizontal variable axes, ordered from bottom to top (Nivo vertical).
    Vertical,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallelTicksPosition {
    Before,
    #[default]
    After,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ParallelMissing {
    #[default]
    Gap,
    Connect,
    SkipRecord,
    Error,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParallelTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
}
impl ParallelTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v)),
            "invalid parallel text size",
        )
    }
    fn color(&self, c: &ChartConfig) -> Color {
        self.color.unwrap_or(c.theme.text)
    }
    fn size(&self, c: &ChartConfig) -> f64 {
        self.size.unwrap_or(c.theme.font_size)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelAxisStyle {
    pub line_color: Option<Color>,
    pub line_width: f64,
    pub ticks: ParallelTextStyle,
    pub caption: ParallelTextStyle,
    pub caption_offset: f64,
    pub caption_rotation: f64,
}
impl Default for ParallelAxisStyle {
    fn default() -> Self {
        Self {
            line_color: None,
            line_width: 1.,
            ticks: Default::default(),
            caption: Default::default(),
            caption_offset: 24.,
            caption_rotation: 0.,
        }
    }
}
impl ParallelAxisStyle {
    fn validate(&self) -> Result<()> {
        self.ticks.validate()?;
        self.caption.validate()?;
        ensure(
            self.line_width.is_finite()
                && (0. ..=256.).contains(&self.line_width)
                && self.caption_offset.is_finite()
                && (0. ..=16384.).contains(&self.caption_offset)
                && self.caption_rotation.is_finite()
                && (-360. ..=360.).contains(&self.caption_rotation),
            "invalid parallel axis style",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelDimension {
    pub id: String,
    /// None uses id; an empty string suppresses the caption. AxisOptions.legend
    /// overrides this caption when supplied.
    pub label: Option<String>,
    pub domain: Option<[f64; 2]>,
    pub scale: ScaleKind,
    pub reverse: bool,
    pub axis: AxisOptions,
    pub ticks_position: ParallelTicksPosition,
    pub style: ParallelAxisStyle,
    /// None uses axis.format for tooltip values.
    pub value_format: Option<ValueFormat>,
}
impl ParallelDimension {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: None,
            domain: None,
            scale: ScaleKind::Linear,
            reverse: false,
            axis: Default::default(),
            ticks_position: Default::default(),
            style: Default::default(),
            value_format: None,
        }
    }
}

/// Per-dimension overrides for the typed parallel coordinates layout.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ParallelScaleOptions {
    /// Lower and upper source endpoints. `None` infers that endpoint from data.
    pub bounds: [Option<f64>; 2],
    pub scale: ContinuousScaleOptions,
}

#[derive(Clone, Debug)]
pub struct ParallelDimensionLayout {
    pub id: String,
    pub source_domain: [f64; 2],
    /// X for a vertical variable axis, Y for a horizontal variable axis.
    pub position: f64,
    /// The exact scale used for both ticks and projected data points.
    pub scale: ChartScale,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParallelRowLayout {
    pub datum: ParallelDatum,
    pub source_index: usize,
    pub mark_id: String,
    /// Screen-space input vertices, including hidden rows and missing slots.
    pub points: Vec<Option<[f64; 2]>>,
    /// Whether the scene has a mark for this row.
    pub drawn: bool,
}

#[derive(Clone, Debug)]
pub struct ParallelCoordinatesLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub dimensions: Vec<ParallelDimensionLayout>,
    pub rows: Vec<ParallelRowLayout>,
    pub interaction: InteractionBinding,
}
impl ParallelCoordinatesLayout {
    pub fn into_scene_interaction(self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene,
            interaction: Some(self.interaction),
            plot: Some(self.plot),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelDatum {
    pub id: String,
    pub label: String,
    /// A group's stable ID is its legend/visibility identity. Ungrouped rows use
    /// their row ID. Group IDs may not collide with ungrouped row IDs.
    pub group: Option<String>,
    pub values: Vec<Option<f64>>,
}
impl ParallelDatum {
    pub fn new(id: impl Into<String>, values: Vec<Option<f64>>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            group: None,
            values,
        }
    }
}
impl From<&ParallelRecord> for ParallelDatum {
    fn from(row: &ParallelRecord) -> Self {
        Self::new(&row.id, row.values.clone())
    }
}
impl From<ParallelRecord> for ParallelDatum {
    fn from(row: ParallelRecord) -> Self {
        Self::new(row.id, row.values)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelLineStyle {
    pub color: Option<Color>,
    pub width: f64,
    pub opacity: f64,
    pub cap: StrokeCap,
    pub dash: Option<StrokeDash>,
}
impl Default for ParallelLineStyle {
    fn default() -> Self {
        Self {
            color: None,
            width: 2.,
            opacity: 0.5,
            cap: StrokeCap::Round,
            dash: None,
        }
    }
}
impl ParallelLineStyle {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.width.is_finite()
                && (0. ..=256.).contains(&self.width)
                && self.opacity.is_finite()
                && (0. ..=1.).contains(&self.opacity),
            "invalid parallel line style",
        )?;
        if let Some(dash) = &self.dash {
            dash.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ParallelCoordinatesOptions {
    pub layout: ParallelLayout,
    pub curve: Curve,
    pub missing: ParallelMissing,
    /// Clamp each input value to its own domain before connecting axes. Without
    /// clamping, projected values must be finite and within ±900,000 pixels.
    /// All row paths retain the full plot rectangle clip.
    pub clamp: bool,
    pub line: ParallelLineStyle,
    /// Complete style overrides, keyed by group ID or ungrouped row ID.
    pub series_styles: BTreeMap<String, ParallelLineStyle>,
    pub hidden: BTreeSet<String>,
    /// False matches Nivo's default empty legend list. At most 256 legend items.
    pub legend: bool,
    pub legend_labels: BTreeMap<String, String>,
    pub row_labels: bool,
    /// Optional plot mask for endpoint labels; captions/ticks are unaffected.
    pub clip_row_labels: bool,
    pub row_label_style: ParallelTextStyle,
    pub row_label_offset: [f64; 2],
    /// Include the first N variables in safe plain-text tooltips; remaining
    /// variables are explicitly counted. Scene's 1024-byte text budget applies.
    pub tooltip_dimensions: usize,
    pub missing_text: String,
}
impl Default for ParallelCoordinatesOptions {
    fn default() -> Self {
        Self {
            layout: Default::default(),
            curve: Curve::Linear,
            missing: Default::default(),
            clamp: false,
            line: Default::default(),
            series_styles: BTreeMap::new(),
            hidden: BTreeSet::new(),
            legend: false,
            legend_labels: BTreeMap::new(),
            row_labels: false,
            clip_row_labels: false,
            row_label_style: Default::default(),
            row_label_offset: [8., 0.],
            tooltip_dimensions: 8,
            missing_text: "missing".into(),
        }
    }
}
impl ParallelCoordinatesOptions {
    pub fn validate(&self) -> Result<()> {
        self.curve.validate()?;
        self.line.validate()?;
        self.row_label_style.validate()?;
        ensure(
            self.series_styles.len() <= 256
                && self.hidden.len() <= 256
                && self.legend_labels.len() <= 256,
            "parallel series option budget exceeded",
        )?;
        ensure(
            (1..=MAX_PARALLEL_DIMENSIONS).contains(&self.tooltip_dimensions)
                && self.missing_text.len() <= 128
                && self
                    .row_label_offset
                    .into_iter()
                    .all(|v| v.is_finite() && v.abs() <= 16384.),
            "invalid parallel label or tooltip options",
        )?;
        for id in self
            .hidden
            .iter()
            .chain(self.series_styles.keys())
            .chain(self.legend_labels.keys())
        {
            valid_id(id, 512)?;
        }
        for style in self.series_styles.values() {
            style.validate()?;
        }
        for label in self.legend_labels.values() {
            ensure(label.len() <= 1024, "parallel legend text budget exceeded")?;
        }
        Ok(())
    }
}
fn valid_id(id: &str, budget: usize) -> Result<()> {
    ensure(
        !id.trim().is_empty() && id.len() <= budget,
        "empty or oversized parallel identity",
    )
}
fn visible(row: &ParallelDatum, o: &ParallelCoordinatesOptions) -> bool {
    !o.hidden.contains(row.group.as_ref().unwrap_or(&row.id))
        && !(o.missing == ParallelMissing::SkipRecord && row.values.iter().any(Option::is_none))
}
fn axis_position(c: &ChartConfig, index: usize, count: usize, layout: ParallelLayout) -> f64 {
    match layout {
        ParallelLayout::Horizontal => {
            c.margin.left + c.inner_width() * index as f64 / (count - 1) as f64
        }
        ParallelLayout::Vertical => {
            c.height - c.margin.bottom - c.inner_height() * index as f64 / (count - 1) as f64
        }
    }
}
fn inferred_domain(
    kind: ScaleKind,
    index: usize,
    rows: &[ParallelDatum],
    o: &ParallelCoordinatesOptions,
    include_hidden: bool,
) -> [f64; 2] {
    let (mut low, mut high) = (f64::INFINITY, f64::NEG_INFINITY);
    for row in rows.iter().filter(|r| include_hidden || visible(r, o)) {
        if let Some(value) = row.values[index] {
            low = low.min(value);
            high = high.max(value);
        }
    }
    if low == f64::INFINITY {
        if kind.requires_nonzero_domain() {
            [1., 10.]
        } else {
            [0., 1.]
        }
    } else {
        [low, high]
    }
}
fn move_geometry(geometry: &mut Geometry, dx: f64, dy: f64) {
    let shift = |p: &mut P| {
        p.x += dx;
        p.y += dy;
    };
    match geometry {
        Geometry::Path(commands) => {
            for command in commands {
                match command {
                    C::Move(p) | C::Line(p) => shift(p),
                    C::Cubic { first, second, end } => {
                        shift(first);
                        shift(second);
                        shift(end);
                    }
                    C::Close => {}
                }
            }
        }
        Geometry::Rect { x, y, .. } | Geometry::Circle { x, y, .. } => {
            *x += dx;
            *y += dy;
        }
    }
}
fn axis(
    scene: &mut Scene,
    c: &ChartConfig,
    d: &ParallelDimension,
    scale: &ChartScale,
    position: f64,
    o: &ParallelCoordinatesOptions,
) -> Result<Vec<usize>> {
    let mut local = c.clone();
    local.theme.font_size = d.style.ticks.size(c);
    local.theme.text = d.style.ticks.color(c);
    let side = match (o.layout, d.ticks_position) {
        (ParallelLayout::Horizontal, ParallelTicksPosition::Before) => AxisSide::Left,
        (ParallelLayout::Horizontal, ParallelTicksPosition::After) => AxisSide::Right,
        (ParallelLayout::Vertical, ParallelTicksPosition::Before) => AxisSide::Top,
        (ParallelLayout::Vertical, ParallelTicksPosition::After) => AxisSide::Bottom,
    };
    let (dx, dy) = match side {
        AxisSide::Left => (position - c.margin.left, 0.),
        AxisSide::Right => (position - (c.width - c.margin.right), 0.),
        AxisSide::Top => (0., position - c.margin.top),
        AxisSide::Bottom => (0., position - (c.height - c.margin.bottom)),
    };
    let mut options = d.axis.clone();
    options.legend = None;
    let mut layer = c.scene()?;
    let axis_id = format!("parallel-axis:{}:{}", d.id.len(), d.id);
    crate::axes::axis(&mut layer, &local, scale, side, &options, &axis_id)?;
    let grid_prefix = format!("{axis_id}-grid-");
    let mut grid_indices = Vec::new();
    for mut mark in layer.marks {
        // Only the suffix appended by axes::axis declares a grid role. A
        // business dimension ID may itself contain "-grid-" or "-tick-".
        if mark.id.starts_with(&grid_prefix) {
            grid_indices.push(scene.marks.len());
        } else {
            move_geometry(&mut mark.geometry, dx, dy);
            mark.stroke = (d.style.line_width > 0.).then_some(GraphicStroke {
                color: d.style.line_color.unwrap_or(c.theme.text),
                width: d.style.line_width,
                cap: StrokeCap::Butt,
            });
        }
        scene.marks.push(mark);
    }
    for mut label in layer.labels {
        label.x += dx;
        label.y += dy;
        scene.labels.push(label);
    }
    let caption = d
        .axis
        .legend
        .as_deref()
        .or(d.label.as_deref())
        .unwrap_or(&d.id);
    if d.axis.visible && !caption.is_empty() {
        let size = d.style.caption.size(c);
        let (x, y, anchor) = match o.layout {
            ParallelLayout::Horizontal => (
                position,
                c.height - c.margin.bottom + d.style.caption_offset,
                TextAnchor::Middle,
            ),
            ParallelLayout::Vertical => (
                c.margin.left - d.style.caption_offset,
                position + size / 3.,
                TextAnchor::End,
            ),
        };
        scene.labels.push(Label {
            x,
            y,
            text: caption.into(),
            color: d.style.caption.color(c),
            size,
            anchor,
            rotation: d.style.caption_rotation,
        });
    }
    Ok(grid_indices)
}
fn tooltip(
    row: &ParallelDatum,
    dimensions: &[ParallelDimension],
    scales: &[ParallelDimensionLayout],
    o: &ParallelCoordinatesOptions,
) -> Result<String> {
    let mut text = if row.label.is_empty() {
        row.id.clone()
    } else {
        row.label.clone()
    };
    for (index, d) in dimensions.iter().take(o.tooltip_dimensions).enumerate() {
        let value = if let Some(value) = row.values[index] {
            let format = d.value_format.as_ref().unwrap_or(&d.axis.format);
            if *format == ValueFormat::Auto && scales[index].scale.kind() == ScaleKind::Utc {
                ValueFormat::Utc.format(value)?
            } else {
                format.format(value)?
            }
        } else {
            o.missing_text.clone()
        };
        text.push_str(&format!(
            " · {}: {value}",
            d.label.as_deref().unwrap_or(&d.id)
        ));
        ensure(text.len() <= 1024, "parallel tooltip byte budget exceeded")?;
    }
    if dimensions.len() > o.tooltip_dimensions {
        text.push_str(&format!(
            " · +{} dimensions",
            dimensions.len() - o.tooltip_dimensions
        ));
    }
    ensure(text.len() <= 1024, "parallel tooltip byte budget exceeded")?;
    Ok(text)
}

/// Independent variable scales with shared screen-space curves and stable row
/// identities. Supports Linear/Log/SymLog, both orientations, reversed ranges,
/// nullable values, styles and explicit axis configuration. No I/O or callbacks.
/// Mark.value is the first defined source value; Mark.label carries formatted
/// multidimensional values. Legacy `parallel_coordinates` retains its profile.
pub fn parallel_coordinates_with_options(
    c: &ChartConfig,
    dimensions: &[ParallelDimension],
    rows: &[ParallelDatum],
    o: &ParallelCoordinatesOptions,
) -> Result<Scene> {
    Ok(build_parallel(c, dimensions, rows, o, None)?.scene)
}

/// Return variable domains, row projections and typed picking data together.
/// Named scale profiles replace the corresponding dimension's domain, scale
/// kind, reversal and clamp settings. Unspecified profiles use the dimension
/// and chart settings. Domain inference includes hidden and skipped rows, so
/// hiding a group or dropping a line with missing points preserves axis scales.
pub fn parallel_layout_with_scale_options(
    c: &ChartConfig,
    dimensions: &[ParallelDimension],
    rows: &[ParallelDatum],
    o: &ParallelCoordinatesOptions,
    profiles: &BTreeMap<String, ParallelScaleOptions>,
) -> Result<ParallelCoordinatesLayout> {
    build_parallel(c, dimensions, rows, o, Some(profiles))
}

fn build_parallel(
    c: &ChartConfig,
    dimensions: &[ParallelDimension],
    rows: &[ParallelDatum],
    o: &ParallelCoordinatesOptions,
    profiles: Option<&BTreeMap<String, ParallelScaleOptions>>,
) -> Result<ParallelCoordinatesLayout> {
    let mut scene = c.scene()?;
    let plot = ClipRect::plot(c);
    o.validate()?;
    ensure(
        (2..=MAX_PARALLEL_DIMENSIONS).contains(&dimensions.len())
            && rows.len() <= MAX_PARALLEL_RECORDS,
        "parallel coordinates budget exceeded",
    )?;
    let mut dimension_ids = BTreeSet::new();
    for d in dimensions {
        valid_id(&d.id, 128)?;
        ensure(
            dimension_ids.insert(&d.id),
            "duplicate parallel dimension ID",
        )?;
        ensure(
            d.label.as_ref().is_none_or(|v| v.len() <= 1024),
            "parallel dimension label budget exceeded",
        )?;
        ensure(
            profiles.is_some() || !matches!(d.scale, ScaleKind::Utc),
            "parallel dimensions support linear/log/symlog scales",
        )?;
        d.axis.validate()?;
        d.style.validate()?;
        if let Some(format) = &d.value_format {
            format.validate()?;
        }
    }
    if let Some(profiles) = profiles {
        ensure(
            profiles.keys().all(|id| dimension_ids.contains(id)),
            "parallel scale profile refers to an unknown dimension",
        )?;
    }
    let mut record_ids = BTreeSet::new();
    let mut series = BTreeMap::<String, (bool, String)>::new();
    for row in rows {
        valid_id(&row.id, 512)?;
        ensure(record_ids.insert(&row.id), "duplicate parallel row ID")?;
        ensure(
            row.label.len() <= 1024
                && row.values.len() == dimensions.len()
                && row.values.iter().flatten().all(|v| v.is_finite()),
            "invalid parallel record values or dimensions",
        )?;
        if o.missing == ParallelMissing::Error {
            ensure(
                row.values.iter().all(Option::is_some),
                "parallel missing value policy rejects nullable records",
            )?;
        }
        let grouped = row.group.is_some();
        let id = row.group.as_ref().unwrap_or(&row.id);
        valid_id(id, 512)?;
        if let Some((was_grouped, _)) = series.get(id) {
            ensure(
                *was_grouped == grouped,
                "parallel group ID collides with an ungrouped record",
            )?;
        }
        series.entry(id.clone()).or_insert((
            grouped,
            if grouped || row.label.is_empty() {
                id.clone()
            } else {
                row.label.clone()
            },
        ));
    }
    ensure(
        o.series_styles
            .keys()
            .chain(o.legend_labels.keys())
            .all(|id| series.contains_key(id)),
        "parallel style/legend references an unknown series",
    )?;
    ensure(
        !o.legend || series.len() <= 256,
        "parallel legend exceeds 256 item budget",
    )?;
    let mut scales = vec![];
    let mut requested_ticks = 0usize;
    for (index, d) in dimensions.iter().enumerate() {
        let mut range = match o.layout {
            ParallelLayout::Horizontal => [c.height - c.margin.bottom, c.margin.top],
            ParallelLayout::Vertical => [c.margin.left, c.width - c.margin.right],
        };
        let (source_domain, scale) = if let Some(profile) = profiles.and_then(|p| p.get(&d.id)) {
            let mut domain = inferred_domain(profile.scale.kind, index, rows, o, true);
            for (end, explicit) in domain.iter_mut().zip(profile.bounds) {
                if let Some(value) = explicit {
                    *end = value;
                }
            }
            (
                domain,
                ChartScale::with_options(domain, range, &profile.scale)?,
            )
        } else {
            if d.reverse {
                range.reverse();
            }
            let domain = d
                .domain
                .unwrap_or_else(|| inferred_domain(d.scale, index, rows, o, profiles.is_some()));
            (
                domain,
                ChartScale::new(d.scale, domain, range)?.clamped(o.clamp),
            )
        };
        if d.axis.visible || d.axis.grid {
            requested_ticks +=
                if let Some(ticks) = crate::axes::calendar_axis_ticks(&scale, &d.axis)? {
                    ticks.len()
                } else if let Some(ticks) = &d.axis.tick_values {
                    ticks.len()
                } else {
                    scale.ticks(d.axis.tick_count).len()
                };
            ensure(
                requested_ticks <= MAX_PARALLEL_AXIS_TICKS,
                "parallel axis tick budget exceeded",
            )?;
        }
        scales.push(ParallelDimensionLayout {
            id: d.id.clone(),
            source_domain,
            position: axis_position(c, index, dimensions.len(), o.layout),
            scale,
        });
    }
    let mut grid_indices = BTreeSet::new();
    for (index, d) in dimensions.iter().enumerate() {
        grid_indices.extend(axis(
            &mut scene,
            c,
            d,
            &scales[index].scale,
            scales[index].position,
            o,
        )?);
    }
    let data_start = scene.marks.len();
    let palette = series
        .keys()
        .enumerate()
        .map(|(index, id)| (id.clone(), c.theme.color(index)))
        .collect::<BTreeMap<_, _>>();
    if o.legend {
        for (id, (_, label)) in &series {
            let style = o.series_styles.get(id).unwrap_or(&o.line);
            scene.legend.push(LegendItem {
                id: id.clone(),
                label: o.legend_labels.get(id).unwrap_or(label).clone(),
                color: style.color.unwrap_or(palette[id]),
            });
        }
    }
    let mut ordered = rows.iter().enumerate().collect::<Vec<_>>();
    ordered.sort_by(|a, b| a.1.id.cmp(&b.1.id));
    let mut projected_rows = Vec::new();
    let mut interaction = InteractionBinding::default();
    let disabled = AxisOptions {
        visible: false,
        grid: false,
        tick_count: 0,
        ..Default::default()
    };
    for (source_index, row) in ordered {
        let visible = visible(row, o);
        if !visible && profiles.is_none() {
            continue;
        }
        let series = row.group.as_ref().unwrap_or(&row.id);
        let style = o.series_styles.get(series).unwrap_or(&o.line);
        let color = style.color.unwrap_or(palette[series]);
        let mut points = vec![];
        let mut projected = profiles.map(|_| vec![None; dimensions.len()]);
        let mut last = None;
        for (index, value) in row.values.iter().enumerate() {
            if let Some(value) = value {
                ensure(
                    scales[index].scale.accepts(*value),
                    "parallel value is invalid for its variable scale",
                )?;
                let scaled = scales[index].scale.map(*value);
                ensure(
                    scaled.is_finite() && scaled.abs() <= 900000.,
                    "parallel projection exceeds coordinate budget; clamp or narrow input",
                )?;
                let position = scales[index].position;
                let point = match o.layout {
                    ParallelLayout::Horizontal => [position, scaled],
                    ParallelLayout::Vertical => [scaled, position],
                };
                if let Some(projected) = &mut projected {
                    projected[index] = Some(point);
                }
                last = Some(point);
                points.push(Point::new(
                    point[0],
                    c.margin.top + c.height - c.margin.bottom - point[1],
                ));
            } else if o.missing == ParallelMissing::Gap {
                points.push(Point::gap(c.margin.left));
            }
        }
        let mark_id = format!("parallel:{}:{}", row.id.len(), row.id);
        if let Some(projected) = projected {
            projected_rows.push(ParallelRowLayout {
                datum: row.clone(),
                source_index,
                mark_id: mark_id.clone(),
                points: projected,
                drawn: false,
            });
        }
        if !visible {
            continue;
        }
        let geometry = crate::line(
            c,
            &[Series::new(&row.id, points)],
            &LineOptions {
                curve: o.curve,
                points: false,
                x_domain: Some([c.margin.left, c.width - c.margin.right]),
                y_domain: Some([c.margin.top, c.height - c.margin.bottom]),
                axes: CartesianAxes {
                    x: disabled.clone(),
                    y: disabled.clone(),
                    ..Default::default()
                },
                ..Default::default()
            },
        )?;
        let commands = geometry
            .marks
            .into_iter()
            .filter(|m| m.id.starts_with("line-"))
            .flat_map(|m| m.geometry.commands())
            .collect::<Vec<_>>();
        if commands.is_empty() {
            continue;
        }
        let mut mark = Mark::new(mark_id, Geometry::Path(commands), color);
        mark.fill = None;
        mark.stroke = (style.width > 0.).then_some(GraphicStroke {
            color,
            width: style.width,
            cap: style.cap,
        });
        mark.stroke_dash = style.dash.clone();
        mark.opacity = style.opacity;
        mark.series = series.clone();
        mark.label = tooltip(row, dimensions, &scales, o)?;
        mark.value = row.values.iter().flatten().next().copied().unwrap_or(0.);
        mark.clip = Some(plot);
        if profiles.is_some() {
            projected_rows.last_mut().unwrap().drawn = true;
            add_interaction(&mut interaction, &mark, plot, color)?;
        }
        scene.marks.push(mark);
        if o.row_labels
            && let Some([x, y]) = last
        {
            if o.clip_row_labels {
                let clip = ClipRect::plot(c);
                scene.label_clips.insert(
                    scene.labels.len(),
                    ClipPath {
                        geometry: Geometry::Rect {
                            x: clip.x,
                            y: clip.y,
                            width: clip.width,
                            height: clip.height,
                        },
                        rule: ClipRule::NonZero,
                    },
                );
            }
            scene
                .label_series
                .insert(scene.labels.len(), series.clone());
            scene.labels.push(Label {
                x: x + o.row_label_offset[0],
                y: y + o.row_label_offset[1],
                text: if row.label.is_empty() {
                    row.id.clone()
                } else {
                    row.label.clone()
                },
                color: o.row_label_style.color(c),
                size: o.row_label_style.size(c),
                anchor: TextAnchor::Start,
                rotation: 0.,
            });
        }
    }
    scene.draw_order = grid_indices
        .iter()
        .copied()
        .chain(data_start..scene.marks.len())
        .chain((0..data_start).filter(|i| !grid_indices.contains(i)))
        .map(DrawItem::Mark)
        .chain((0..scene.labels.len()).map(DrawItem::Label))
        .collect();
    projected_rows.sort_by_key(|row| row.source_index);
    if profiles.is_some() {
        interaction.validate(&scene)?;
    } else {
        scene.validate()?;
    }
    Ok(ParallelCoordinatesLayout {
        scene,
        plot,
        dimensions: scales,
        rows: projected_rows,
        interaction,
    })
}

fn add_interaction(
    binding: &mut InteractionBinding,
    mark: &Mark,
    plot: ClipRect,
    color: Color,
) -> Result<()> {
    let Geometry::Path(commands) = &mark.geometry else {
        unreachable!("parallel rows always use paths")
    };
    let endpoints = || {
        commands.iter().enumerate().filter_map(|(index, command)| {
            let point = match command {
                C::Move(point) | C::Line(point) | C::Cubic { end: point, .. } => point,
                C::Close => return None,
            };
            Some((index, [point.x, point.y]))
        })
    };
    let (index, position) = endpoints()
        .find(|(_, p)| plot.contains(p[0], p[1]))
        .or_else(|| endpoints().next())
        .ok_or_else(|| ChartError("parallel row has no path endpoint".into()))?;
    binding.anchors.insert(
        mark.id.clone(),
        InteractionAnchor::path_points(&mark.geometry, position, &[index])?,
    );
    binding.sites.push(InteractionSite {
        mark_id: mark.id.clone(),
        series: mark.series.clone(),
        label: mark.label.clone(),
        value: Some(mark.value),
        formatted_value: None,
        color,
        position,
    });
    Ok(())
}
