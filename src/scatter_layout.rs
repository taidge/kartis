//! Independent typed scatter layouts. Nivo 0.99.0 references:
//! `scatterplot/src/{props.tsx,compute.ts,hooks.ts,Node.tsx}`. Default size is
//! a nine-pixel circle diameter. Glyph area/nominal size are explicit units.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::PI;

use glory::render::GraphicStroke;

use crate::scales::LinearScale;
use crate::scene::ensure;
use crate::*;
mod axes;
mod custom;
pub(crate) use custom::PreparedNode;
pub use custom::{ScatterNodeContext, ScatterNodeGeometry};

#[derive(Clone, Debug, PartialEq)]
pub struct ScatterDatum {
    pub id: String,
    pub x: CategoryKey,
    pub y: CategoryKey,
    pub size_value: Option<f64>,
    pub color: Option<Color>,
    pub label: Option<String>,
}
impl ScatterDatum {
    pub fn new(
        id: impl Into<String>,
        x: impl Into<CategoryKey>,
        y: impl Into<CategoryKey>,
    ) -> Self {
        Self {
            id: id.into(),
            x: x.into(),
            y: y.into(),
            size_value: None,
            color: None,
            label: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterSeries {
    pub id: String,
    pub label: String,
    pub data: Vec<ScatterDatum>,
    pub color: Option<Color>,
}
impl ScatterSeries {
    pub fn new(id: impl Into<String>, data: Vec<ScatterDatum>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            data,
            color: None,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScatterBound {
    #[default]
    Auto,
    Fixed(f64),
}
#[derive(Clone, Debug, PartialEq)]
pub enum ScatterAxisScale {
    /// Full numeric domain stops with plot-fraction range stops. The first
    /// fraction is 0 and the last is 1; ordinary Y orientation is bottom-up.
    Piecewise { options: PiecewiseScaleOptions },
    Continuous {
        options: ContinuousScaleOptions,
        min: ScatterBound,
        max: ScatterBound,
    },
    /// First occurrence wins; string and numeric keys remain distinct. Point Y
    /// runs from top to bottom, as Nivo's actual pointScale.ts does.
    Point {
        options: PointScaleOptions,
        domain: Option<Vec<CategoryKey>>,
        window: Option<[f64; 2]>,
    },
}
impl Default for ScatterAxisScale {
    fn default() -> Self {
        Self::Continuous {
            options: ContinuousScaleOptions {
                nice: ScaleNice::Default,
                ..Default::default()
            },
            min: ScatterBound::Fixed(0.),
            max: ScatterBound::Auto,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScatterScaleOptions {
    pub x: ScatterAxisScale,
    pub y: ScatterAxisScale,
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum ScatterMissingSize {
    #[default]
    Error,
    Skip,
    Fixed(f64),
}
#[derive(Clone, Debug, PartialEq)]
pub enum ScatterSize {
    Fixed(f64),
    /// Actual d3 scaleLinear values -> sizes; extrapolates unless clamped.
    /// Constant value domains use the midpoint size. Negative/nonfinite output
    /// is rejected, rather than taking an absolute value or silently clamping.
    Linear {
        values: [f64; 2],
        sizes: [f64; 2],
        clamp: bool,
        missing: ScatterMissingSize,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub enum ScatterNodeShape {
    Circle {
        diameter: ScatterSize,
    },
    FilledGlyph {
        shape: FilledGlyph,
        area: ScatterSize,
        rotation: f64,
    },
    StrokedGlyph {
        shape: StrokedGlyph,
        size: ScatterSize,
        rotation: f64,
    },
}
impl Default for ScatterNodeShape {
    fn default() -> Self {
        Self::Circle {
            diameter: ScatterSize::Fixed(9.),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScatterSizeUnit {
    CircleDiameter,
    FilledArea,
    StrokedNominal,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterNodeStyle {
    pub shape: ScatterNodeShape,
    pub fill: Option<Color>,
    pub paint: Option<FillPaint>,
    pub stroke: Option<GraphicStroke>,
    pub opacity: f64,
}
impl Default for ScatterNodeStyle {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            fill: None,
            paint: None,
            stroke: None,
            opacity: 1.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ScatterLabelText {
    #[default]
    Id,
    Coordinates,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterLabels {
    pub enabled: bool,
    pub text: ScatterLabelText,
    pub offset: [f64; 2],
    pub color: Option<Color>,
    pub size: f64,
    pub rotation: f64,
    pub anchor: TextAnchor,
    pub clip_to_plot: bool,
}
impl Default for ScatterLabels {
    fn default() -> Self {
        Self {
            enabled: false,
            text: Default::default(),
            offset: [0., -9.],
            color: None,
            size: 12.,
            rotation: 0.,
            anchor: TextAnchor::Middle,
            clip_to_plot: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterAxes {
    pub top: Option<AxisOptions>,
    pub right: Option<AxisOptions>,
    pub bottom: Option<AxisOptions>,
    pub left: Option<AxisOptions>,
}
impl Default for ScatterAxes {
    fn default() -> Self {
        Self {
            top: None,
            right: None,
            bottom: Some(AxisOptions {
                grid: true,
                ..Default::default()
            }),
            left: Some(AxisOptions {
                grid: true,
                ..Default::default()
            }),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterLayoutOptions {
    pub scales: ScatterScaleOptions,
    pub axes: ScatterAxes,
    pub node: ScatterNodeStyle,
    pub series_styles: BTreeMap<String, ScatterNodeStyle>,
    pub hidden: BTreeSet<String>,
    pub labels: ScatterLabels,
    pub x_format: ValueFormat,
    pub y_format: ValueFormat,
    pub clip_to_plot: bool,
}
impl Default for ScatterLayoutOptions {
    fn default() -> Self {
        Self {
            scales: Default::default(),
            axes: Default::default(),
            node: Default::default(),
            series_styles: Default::default(),
            hidden: Default::default(),
            labels: Default::default(),
            x_format: Default::default(),
            y_format: Default::default(),
            clip_to_plot: true,
        }
    }
}
#[derive(Clone, Debug)]
pub enum ResolvedScatterAxis {
    Continuous(ResolvedContinuousScale),
    Point(ScatterPointScale),
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterPointScale {
    base: PointScale,
    window: Option<[f64; 2]>,
    round: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterSite {
    pub mark_id: String,
    pub node_id: String,
    pub series_id: String,
    pub series_label: String,
    /// Authored, formatted datum label; independent of Scene vector indices.
    pub label: String,
    pub series_index: usize,
    pub source_index: usize,
    pub global_index: usize,
    pub raw_x: CategoryKey,
    pub raw_y: CategoryKey,
    pub formatted_x: String,
    pub formatted_y: String,
    pub size_value: Option<f64>,
    pub node_size: f64,
    pub size_unit: ScatterSizeUnit,
    pub position: [f64; 2],
    pub bounds: ClipRect,
    pub color: Color,
    pub mark_index: usize,
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ScatterNodeMetadata {
    pub mark_id: String,
    pub node_id: String,
    pub series_id: String,
    pub series_index: usize,
    pub source_index: usize,
    pub global_index: usize,
    pub raw_x: CategoryKey,
    pub raw_y: CategoryKey,
    pub size_value: Option<f64>,
    pub hidden: bool,
    pub position: Option<[f64; 2]>,
    pub mark_index: Option<usize>,
    pub node_size: Option<f64>,
}
#[derive(Clone, Debug)]
pub struct ScatterLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub sites: Vec<ScatterSite>,
    pub anchors: BTreeMap<String, InteractionAnchor>,
    pub x_scale: ResolvedScatterAxis,
    pub y_scale: ResolvedScatterAxis,
    pub metadata: Vec<ScatterNodeMetadata>,
    pub scale_options: ScatterScaleOptions,
}
impl ScatterSize {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Fixed(v) => size_valid(*v),
            Self::Linear {
                values,
                sizes,
                missing,
                ..
            } => {
                LinearScale::new(*values, *sizes)?;
                for s in sizes {
                    size_valid(*s)?;
                }
                if let ScatterMissingSize::Fixed(v) = missing {
                    size_valid(*v)?;
                }
                Ok(())
            }
        }
    }
    fn resolve(&self, raw: Option<f64>) -> Result<Option<f64>> {
        let value = match self {
            Self::Fixed(v) => *v,
            Self::Linear {
                values,
                sizes,
                clamp,
                missing,
            } => match raw {
                Some(v) => crate::scale_options::interpolate(*values, *sizes, v, *clamp),
                None => match missing {
                    ScatterMissingSize::Error => {
                        return Err(ChartError("scatter size value is missing".into()));
                    }
                    ScatterMissingSize::Skip => return Ok(None),
                    ScatterMissingSize::Fixed(v) => *v,
                },
            },
        };
        size_valid(value)?;
        Ok(Some(value))
    }
}
fn size_valid(v: f64) -> Result<()> {
    ensure(
        v.is_finite() && v >= 0.,
        "scatter size must be finite and nonnegative",
    )
}
impl ScatterNodeStyle {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.opacity.is_finite() && (0. ..=1.).contains(&self.opacity),
            "invalid scatter node opacity",
        )?;
        if let Some(stroke) = self.stroke {
            ensure(
                stroke.width.is_finite() && (0.000001..=256.).contains(&stroke.width),
                "invalid scatter node stroke",
            )?;
        }
        if let Some(paint) = &self.paint {
            paint.validate()?;
        }
        match &self.shape {
            ScatterNodeShape::Circle { diameter } => diameter.validate()?,
            ScatterNodeShape::FilledGlyph { area, rotation, .. } => {
                area.validate()?;
                rotation_valid(*rotation)?;
            }
            ScatterNodeShape::StrokedGlyph { size, rotation, .. } => {
                size.validate()?;
                rotation_valid(*rotation)?;
                ensure(
                    self.fill.is_none()
                        && self.paint.is_none()
                        && self.stroke.is_some_and(|s| s.color.3 > 0),
                    "stroke-only scatter glyph requires explicit visible stroke and no fill paint",
                )?;
            }
        }
        Ok(())
    }
}
fn rotation_valid(v: f64) -> Result<()> {
    ensure(
        v.is_finite() && v.abs() <= 1e6,
        "invalid scatter glyph rotation",
    )
}
impl ScatterLayoutOptions {
    pub fn validate(&self) -> Result<()> {
        self.node.validate()?;
        self.x_format.validate()?;
        self.y_format.validate()?;
        ensure(
            self.hidden.len() <= 256 && self.series_styles.len() <= 256,
            "scatter style/hidden count budget exceeded",
        )?;
        for id in &self.hidden {
            key_valid(id)?;
        }
        for (id, style) in &self.series_styles {
            key_valid(id)?;
            style.validate()?;
        }
        self.scales.x.validate()?;
        self.scales.y.validate()?;
        for axis in [
            &self.axes.top,
            &self.axes.right,
            &self.axes.bottom,
            &self.axes.left,
        ]
        .into_iter()
        .flatten()
        {
            axis.validate()?;
        }
        let l = &self.labels;
        ensure(
            l.offset.iter().all(|v| v.is_finite() && v.abs() <= 16384.)
                && l.size.is_finite()
                && (1. ..=256.).contains(&l.size)
                && l.rotation.is_finite()
                && (-360. ..=360.).contains(&l.rotation),
            "invalid scatter label style",
        )
    }
}
fn key_valid(s: &str) -> Result<()> {
    ensure(
        !s.is_empty() && s.len() <= 256,
        "scatter IDs must be nonempty and at most 256 bytes",
    )
}

fn source_valid(data: &[ScatterSeries], o: &ScatterLayoutOptions) -> Result<usize> {
    ensure(data.len() <= 256, "scatter series budget exceeded")?;
    let mut ids = BTreeSet::new();
    let mut count = 0usize;
    let mut bytes = 0usize;
    for row in data {
        key_valid(&row.id)?;
        ensure(ids.insert(&row.id), "duplicate scatter series ID")?;
        ensure(
            row.label.len() <= MAX_LABEL_BYTES,
            "scatter series label budget exceeded",
        )?;
        count = count
            .checked_add(row.data.len())
            .ok_or_else(|| ChartError("scatter source budget overflow".into()))?;
        ensure(count <= MAX_MARKS, "scatter source budget exceeded")?;
        bytes += row.id.len() + row.label.len();
        let mut local = BTreeSet::new();
        for p in &row.data {
            key_valid(&p.id)?;
            ensure(
                local.insert(&p.id),
                "duplicate scatter node ID within series",
            )?;
            o.scales.x.validate_raw(&p.x)?;
            o.scales.y.validate_raw(&p.y)?;
            ensure(
                p.size_value.is_none_or(f64::is_finite)
                    && p.label.as_ref().is_none_or(|s| s.len() <= MAX_LABEL_BYTES),
                "invalid scatter size value or node label",
            )?;
            bytes += p.id.len()
                + p.label.as_ref().map_or(0, String::len)
                + match &p.x {
                    CategoryKey::String(s) => s.len(),
                    _ => 0,
                }
                + match &p.y {
                    CategoryKey::String(s) => s.len(),
                    _ => 0,
                };
            ensure(bytes <= 4_000_000, "scatter source text budget exceeded")?;
        }
    }
    Ok(count)
}
fn shape(
    o: &ScatterNodeShape,
    raw: Option<f64>,
) -> Result<Option<(GlyphOptions, f64, ScatterSizeUnit)>> {
    let (spec, kind, rotation, unit) = match o {
        ScatterNodeShape::Circle { diameter } => (
            diameter,
            GlyphShape::Filled(FilledGlyph::Circle),
            0.,
            ScatterSizeUnit::CircleDiameter,
        ),
        ScatterNodeShape::FilledGlyph {
            shape,
            area,
            rotation,
        } => (
            area,
            GlyphShape::Filled(*shape),
            *rotation,
            ScatterSizeUnit::FilledArea,
        ),
        ScatterNodeShape::StrokedGlyph {
            shape,
            size,
            rotation,
        } => (
            size,
            GlyphShape::Stroked(*shape),
            *rotation,
            ScatterSizeUnit::StrokedNominal,
        ),
    };
    let Some(size) = spec.resolve(raw)? else {
        return Ok(None);
    };
    let area = if unit == ScatterSizeUnit::CircleDiameter {
        PI * (size / 2.).powi(2)
    } else {
        size
    };
    ensure(
        area.is_finite() && (size == 0. || area > 0.),
        "scatter size has no representable geometry",
    )?;
    Ok(Some((
        GlyphOptions {
            shape: kind,
            size: area,
            rotation,
        },
        size,
        unit,
    )))
}
fn business_id(series: &str, node: &str) -> String {
    format!(
        "scatter-node:{}:{series}:{}:{node}",
        series.len(),
        node.len()
    )
}
/// Compute independent scatter marks, axes and typed logical sites. All mapped
/// non-hidden nodes are retained, including zero sizes and off-plot centers.
/// Paint is clipped by the plot; neither glyph size nor its bounding-box center
/// substitutes for the authored coordinate used by nearest-site interaction.
pub fn scatter_layout(
    c: &ChartConfig,
    data: &[ScatterSeries],
    o: &ScatterLayoutOptions,
) -> Result<ScatterLayout> {
    scatter_layout_impl(c, data, o, None)
}

/// Build typed scatter nodes with caller-authored Scene geometry and paint.
///
/// The factory runs once per mapped, visible datum with a resolved size.
/// Built-in shape options still determine the size unit and inherited paint.
/// Axes, clipping, identity, labels and interaction remain owned by the layout.
/// A factory error or invalid geometry rejects the entire candidate.
pub fn scatter_layout_with_node_factory(
    c: &ChartConfig,
    data: &[ScatterSeries],
    o: &ScatterLayoutOptions,
    mut factory: impl FnMut(&ScatterNodeContext<'_>) -> Result<ScatterNodeGeometry>,
) -> Result<ScatterLayout> {
    scatter_layout_impl(c, data, o, Some(&mut factory))
}

type NodeFactory<'a> = dyn FnMut(&ScatterNodeContext<'_>) -> Result<ScatterNodeGeometry> + 'a;

fn scatter_layout_impl(
    c: &ChartConfig,
    data: &[ScatterSeries],
    o: &ScatterLayoutOptions,
    mut factory: Option<&mut NodeFactory<'_>>,
) -> Result<ScatterLayout> {
    c.validate()?;
    o.validate()?;
    let count = source_valid(data, o)?;
    let plot = ClipRect::plot(c);
    let all_x = || {
        data.iter()
            .flat_map(|r| r.data.iter().map(|p| p.x.clone()))
            .collect()
    };
    let all_y = || {
        data.iter()
            .flat_map(|r| r.data.iter().map(|p| p.y.clone()))
            .collect()
    };
    let visible_x = data
        .iter()
        .filter(|r| !o.hidden.contains(&r.id))
        .flat_map(|r| r.data.iter().map(|p| p.x.clone()))
        .collect();
    let visible_y = data
        .iter()
        .filter(|r| !o.hidden.contains(&r.id))
        .flat_map(|r| r.data.iter().map(|p| p.y.clone()))
        .collect();
    let x_scale = o
        .scales
        .x
        .resolve(visible_x, all_x(), [plot.x, plot.x + plot.width])?;
    let y_range = if matches!(o.scales.y, ScatterAxisScale::Point { .. }) {
        [plot.y, plot.y + plot.height]
    } else {
        [plot.y + plot.height, plot.y]
    };
    let y_scale = o.scales.y.resolve(visible_y, all_y(), y_range)?;
    let mut scene = c.scene()?;
    for (side, axis, scale, id) in [
        (AxisSide::Top, &o.axes.top, &x_scale, "scatter-axis-top"),
        (
            AxisSide::Right,
            &o.axes.right,
            &y_scale,
            "scatter-axis-right",
        ),
        (
            AxisSide::Bottom,
            &o.axes.bottom,
            &x_scale,
            "scatter-axis-bottom",
        ),
        (AxisSide::Left, &o.axes.left, &y_scale, "scatter-axis-left"),
    ] {
        if let Some(axis) = axis {
            scale.append_axis(&mut scene, c, side, axis, id)?;
        }
    }
    let mut commands = scene
        .marks
        .iter()
        .map(|m| match &m.geometry {
            Geometry::Path(p) => p.len(),
            _ => 0,
        })
        .sum::<usize>();
    let mut paint_stops = 0usize;
    let mut sites = Vec::with_capacity(count);
    let mut metadata = Vec::with_capacity(count);
    let mut anchors = BTreeMap::new();
    let mut global_index = 0;
    for (series_index, row) in data.iter().enumerate() {
        let style = o.series_styles.get(&row.id).unwrap_or(&o.node);
        let inherited = row.color.unwrap_or(c.theme.color(series_index));
        scene.legend.push(LegendItem {
            id: row.id.clone(),
            label: row.label.clone(),
            color: if matches!(style.shape, ScatterNodeShape::StrokedGlyph { .. }) {
                style.stroke.unwrap().color
            } else {
                style.fill.unwrap_or(inherited)
            },
        });
        for (source_index, p) in row.data.iter().enumerate() {
            let id = business_id(&row.id, &p.id);
            let hidden = o.hidden.contains(&row.id);
            let mut meta = ScatterNodeMetadata {
                mark_id: id.clone(),
                node_id: p.id.clone(),
                series_id: row.id.clone(),
                series_index,
                source_index,
                global_index,
                raw_x: p.x.clone(),
                raw_y: p.y.clone(),
                size_value: p.size_value,
                hidden,
                position: None,
                mark_index: None,
                node_size: None,
            };
            global_index += 1;
            if hidden {
                metadata.push(meta);
                continue;
            }
            let x = x_scale.map(&p.x)?;
            let y = y_scale.map(&p.y)?;
            // Explicit point domains may omit data. Preserve its source record,
            // without replacing an absent typed key with a numeric surrogate.
            let (Some(x), Some(y)) = (x, y) else {
                metadata.push(meta);
                continue;
            };
            let position = [x, y];
            meta.position = Some(position);
            let Some((glyph_options, node_size, size_unit)) = shape(&style.shape, p.size_value)?
            else {
                metadata.push(meta);
                continue;
            };
            meta.node_size = Some(node_size);
            let formatted_x = x_scale.format(&p.x, &o.x_format)?;
            let formatted_y = y_scale.format(&p.y, &o.y_format)?;
            let label = format!(
                "{} · {}: x: {}, y: {}",
                row.label,
                p.label.as_ref().unwrap_or(&p.id),
                formatted_x,
                formatted_y
            );
            ensure(
                label.len() <= MAX_LABEL_BYTES,
                "scatter datum label budget exceeded",
            )?;
            ensure(
                scene.marks.len() < MAX_MARKS,
                "scatter scene geometry budget exceeded",
            )?;
            let color = p.color.or(style.fill).unwrap_or(inherited);
            let stroke_only = matches!(glyph_options.shape, GlyphShape::Stroked(_));
            let fill = (!stroke_only).then_some(color);
            let mut stroke = style.stroke;
            if stroke_only && let (Some(color), Some(stroke)) = (p.color, &mut stroke) {
                stroke.color = color;
            }
            let color = if stroke_only {
                stroke.unwrap().color
            } else {
                color
            };
            let prepared = if let Some(factory) = factory.as_deref_mut() {
                let context = ScatterNodeContext {
                    datum: p,
                    series_id: &row.id,
                    series_label: &row.label,
                    mark_id: &id,
                    series_index,
                    source_index,
                    global_index: meta.global_index,
                    formatted_x: &formatted_x,
                    formatted_y: &formatted_y,
                    position,
                    plot,
                    size: node_size,
                    size_unit,
                    color,
                    fill,
                    fill_paint: style.paint.as_ref(),
                    stroke,
                    opacity: style.opacity,
                };
                factory(&context)?.prepare_at(
                    &id,
                    position,
                    color,
                    200_000usize.saturating_sub(commands),
                )?
            } else {
                let glyph = glyph_geometry(position, &glyph_options)?;
                let mut mark = glyph_mark(&id, &glyph, fill, stroke)?;
                mark.fill_paint = style.paint.clone();
                mark.opacity = style.opacity;
                let anchor = if glyph.anchor_indices.is_empty() {
                    None
                } else {
                    Some(InteractionAnchor::path_points(
                        &glyph.geometry,
                        glyph.anchor,
                        &glyph.anchor_indices,
                    )?)
                };
                PreparedNode {
                    mark,
                    bounds: glyph.bounds,
                    anchor,
                    color,
                }
            };
            let PreparedNode {
                mut mark,
                bounds,
                anchor,
                color: actual_color,
            } = prepared;
            let extra = match &mark.geometry {
                Geometry::Path(path) => path.len(),
                _ => 0,
            };
            commands = commands
                .checked_add(extra)
                .ok_or_else(|| ChartError("scatter path budget overflow".into()))?;
            ensure(
                commands <= 200_000,
                "scatter scene geometry budget exceeded",
            )?;
            paint_stops += mark
                .fill_paint
                .as_ref()
                .map_or(0, |paint| paint.stops().len());
            ensure(
                paint_stops <= MAX_SCENE_GRADIENT_STOPS,
                "scatter gradient stop budget exceeded",
            )?;
            mark.series = row.id.clone();
            mark.label = label.clone();
            mark.value = match p.y {
                CategoryKey::Number(v) => v,
                _ => 0.,
            };
            mark.clip = o.clip_to_plot.then_some(plot);
            if let Some(anchor) = anchor {
                anchors.insert(id.clone(), anchor);
            }
            let mark_index = scene.marks.len();
            scene.marks.push(mark);
            meta.mark_index = Some(mark_index);
            let label_index = if o.labels.enabled {
                ensure(
                    scene.labels.len() < MAX_MARKS,
                    "scatter data label budget exceeded",
                )?;
                let label_index = scene.labels.len();
                let text = match o.labels.text {
                    ScatterLabelText::Id => p.label.clone().unwrap_or_else(|| p.id.clone()),
                    ScatterLabelText::Coordinates => format!("{}, {}", formatted_x, formatted_y),
                };
                ensure(
                    text.len() <= MAX_LABEL_BYTES,
                    "scatter point-label text budget exceeded",
                )?;
                scene.labels.push(Label {
                    x: x + o.labels.offset[0],
                    y: y + o.labels.offset[1],
                    text,
                    size: o.labels.size,
                    color: o.labels.color.unwrap_or(c.theme.text),
                    anchor: o.labels.anchor,
                    rotation: o.labels.rotation,
                });
                scene.label_series.insert(label_index, row.id.clone());
                if o.labels.clip_to_plot {
                    scene.label_clips.insert(
                        label_index,
                        ClipPath {
                            geometry: Geometry::Rect {
                                x: plot.x,
                                y: plot.y,
                                width: plot.width,
                                height: plot.height,
                            },
                            rule: ClipRule::NonZero,
                        },
                    );
                }
                Some(label_index)
            } else {
                None
            };
            sites.push(ScatterSite {
                mark_id: id,
                node_id: p.id.clone(),
                series_id: row.id.clone(),
                series_label: row.label.clone(),
                label,
                series_index,
                source_index,
                global_index: meta.global_index,
                raw_x: p.x.clone(),
                raw_y: p.y.clone(),
                formatted_x,
                formatted_y,
                size_value: p.size_value,
                node_size,
                size_unit,
                position,
                bounds,
                color: actual_color,
                mark_index,
                label_index,
            });
            metadata.push(meta);
        }
    }
    scene.validate()?;
    let result = ScatterLayout {
        scene,
        plot,
        sites,
        anchors,
        x_scale,
        y_scale,
        metadata,
        scale_options: o.scales.clone(),
    };
    result.interaction_binding().validate(&result.scene)?;
    Ok(result)
}
impl ScatterLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            sites: self
                .sites
                .iter()
                .map(|s| InteractionSite {
                    mark_id: s.mark_id.clone(),
                    series: s.series_id.clone(),
                    label: s.label.clone(),
                    value: match s.raw_y {
                        CategoryKey::Number(v) => Some(v),
                        _ => None,
                    },
                    formatted_value: match s.raw_y {
                        CategoryKey::Number(_) => Some(s.formatted_y.clone()),
                        _ => None,
                    },
                    color: s.color,
                    position: s.position,
                })
                .collect(),
            anchors: self.anchors.clone(),
            ..Default::default()
        }
    }
    /// Explicit all-logical-site nearest selection, restricted to the plot.
    /// Exact paint picking remains `Scene::hit_test` / Picking::Geometry.
    pub fn nearest(&self, x: f64, y: f64, distance: f64) -> Result<Option<&ScatterSite>> {
        ensure(
            distance.is_finite() && (0. ..=1e6).contains(&distance),
            "invalid scatter nearest distance",
        )?;
        if !self.plot.contains(x, y) {
            return Ok(None);
        }
        self.interaction_binding().validate(&self.scene)?;
        let positions = self
            .sites
            .iter()
            .map(|s| (s.mark_id.clone(), s.position))
            .collect();
        let mesh = crate::mesh::PointMesh::from_scene_at(&self.scene, &positions)?;
        Ok(mesh
            .nearest(x, y, distance)
            .and_then(|p| self.sites.iter().find(|s| s.mark_id == p.id)))
    }
    pub fn viewport_controller(&self, limits: ViewportLimits) -> Result<DomainViewport> {
        let x = self.x_scale.coordinate_domain();
        let y = self.y_scale.coordinate_domain();
        ensure(
            self.x_scale.category_domain().is_none_or(|d| !d.is_empty())
                && self.y_scale.category_domain().is_none_or(|d| !d.is_empty()),
            "empty point axis is not navigable",
        )?;
        let xr = self.x_scale.range();
        let yr = self.y_scale.range();
        let mut viewport = DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new(
                    [x[0].min(x[1]), x[0].max(x[1])],
                    self.x_scale
                        .chart_scale()
                        .map_or(ScaleKind::Linear, ChartScale::kind),
                ),
                left: Some(ViewportAxis::new(
                    [y[0].min(y[1]), y[0].max(y[1])],
                    self.y_scale
                        .chart_scale()
                        .map_or(ScaleKind::Linear, ChartScale::kind),
                )),
                right: None,
            },
            self.plot,
            limits,
        )?
        .with_directions(ViewportDirections {
            x: (x[1] < x[0]) ^ (xr[1] < xr[0]),
            left: (y[1] < y[0]) ^ (yr[1] > yr[0]),
            right: false,
        })?;
        if self.x_scale.point_scale().is_some() {
            viewport = viewport.with_x_max_zoom((x[1] - x[0]).abs().max(1.))?;
        }
        if self.y_scale.point_scale().is_some() {
            viewport = viewport.with_y_max_zoom((y[1] - y[0]).abs().max(1.))?;
        }
        viewport.with_piecewise_scales(
            self.x_scale.chart_scale().filter(|s| s.is_piecewise()),
            self.y_scale.chart_scale().filter(|s| s.is_piecewise()),
            None,
        )
    }
    /// Re-layout a canonical controller request without reapplying nice or
    /// losing the authored visual direction. Point domains stay typed; only
    /// their explicit ordinal windows change. Reuse the original baseline
    /// layout's helper for a controller that was constructed from that layout.
    pub fn viewport_scale_options(&self, domains: ResolvedDomains) -> Result<ScatterScaleOptions> {
        self.viewport_scale_options_with_piecewise(domains, &PiecewiseViewportDomains::default())
    }
    /// Relayout with complete piecewise domain stops from the accepted
    /// controller. The endpoint-only helper rejects these axes explicitly.
    pub fn viewport_scale_options_with_piecewise(
        &self,
        domains: ResolvedDomains,
        piecewise: &PiecewiseViewportDomains,
    ) -> Result<ScatterScaleOptions> {
        fn axis(
            authored: &ScatterAxisScale,
            resolved: &ResolvedScatterAxis,
            domain: [f64; 2],
            vertical: bool,
            stops: Option<&Vec<f64>>,
        ) -> Result<ScatterAxisScale> {
            ensure(
                matches!(authored, ScatterAxisScale::Piecewise { .. }) == stops.is_some(),
                "piecewise scatter viewport requires its complete domain stops",
            )?;
            ensure(
                domain.iter().all(|v| v.is_finite()) && domain[0] < domain[1],
                "invalid scatter viewport domain",
            )?;
            let current = resolved.coordinate_domain();
            let range = resolved.range();
            let reverse = (current[1] < current[0])
                ^ if vertical {
                    range[1] > range[0]
                } else {
                    range[1] < range[0]
                };
            let next = match authored {
                ScatterAxisScale::Continuous { options, .. } => ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions {
                        nice: ScaleNice::Off,
                        reverse,
                        ..*options
                    },
                    min: ScatterBound::Fixed(domain[0]),
                    max: ScatterBound::Fixed(domain[1]),
                },
                ScatterAxisScale::Piecewise { options } => {
                    let mut options = options.clone();
                    options.domain = stops.unwrap().clone();
                    options.continuous.nice = ScaleNice::Off;
                    let first = options
                        .domain
                        .first()
                        .copied()
                        .ok_or_else(|| ChartError("empty piecewise viewport domain".into()))?;
                    let last = options.domain.last().copied().unwrap();
                    ensure(
                        [first.min(last), first.max(last)] == domain,
                        "piecewise viewport stops differ from endpoint domain",
                    )?;
                    ScatterAxisScale::Piecewise { options }
                }
                ScatterAxisScale::Point { options, .. } => ScatterAxisScale::Point {
                    options: *options,
                    domain: resolved.category_domain().map(<[CategoryKey]>::to_vec),
                    window: Some(domain),
                },
            };
            next.validate()?;
            if let ScatterAxisScale::Continuous { options, .. } = &next {
                ResolvedContinuousScale::new(domain, resolved.range(), options)?;
            } else if let ScatterAxisScale::Piecewise { options } = &next {
                crate::line_scales::resolve_plot_piecewise(options, resolved.range())?;
            }
            Ok(next)
        }
        ensure(
            domains.right.is_none() && piecewise.right.is_none(),
            "scatter has only one Y value domain",
        )?;
        Ok(ScatterScaleOptions {
            x: axis(
                &self.scale_options.x,
                &self.x_scale,
                domains.x,
                false,
                piecewise.x.as_ref(),
            )?,
            y: axis(
                &self.scale_options.y,
                &self.y_scale,
                domains
                    .left
                    .ok_or_else(|| ChartError("scatter viewport has no Y domain".into()))?,
                true,
                piecewise.left.as_ref(),
            )?,
        })
    }
}
impl From<ScatterLayout> for SceneInteraction {
    fn from(layout: ScatterLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            plot: Some(layout.plot),
            interaction,
        }
    }
}
