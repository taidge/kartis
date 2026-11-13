//! Discrete rank lines with the shoulder construction used by Nivo Bump.
use crate::{
    AxisOptions, CategoryKey, ChartConfig, ChartError, ClipRect, Color, Curve, Geometry,
    GlyphOptions, GlyphPaintMode, InheritedColor, InheritedColorContext, InteractionAnchor,
    InteractionBinding, InteractionSite, Label, LegendItem, LinePointGeometry, Mark,
    OrdinalColorOptions, PointScale, PointScaleOptions, Result, Scene, SceneInteraction,
    StrokeDash, TextAnchor, ValueFormat, cartesian::path, glyph_geometry, glyph_mark,
    line_scales::category_text, scene::ensure,
};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BUMP_POINTS: usize = 8192;

#[derive(Clone, Debug, PartialEq)]
pub struct BumpDatum {
    pub x: CategoryKey,
    pub rank: Option<f64>,
}
impl BumpDatum {
    pub fn new(x: impl Into<CategoryKey>, rank: f64) -> Self {
        Self {
            x: x.into(),
            rank: Some(rank),
        }
    }
    pub fn gap(x: impl Into<CategoryKey>) -> Self {
        Self {
            x: x.into(),
            rank: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpSeries {
    pub id: String,
    pub label: String,
    pub data: Vec<BumpDatum>,
}
impl BumpSeries {
    pub fn new(id: impl Into<String>, data: Vec<BumpDatum>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            data,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BumpInterpolation {
    #[default]
    Smooth,
    Linear,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpPointStyle {
    /// Circle diameter in logical pixels. Zero retains an unpainted logical
    /// site for nearest and slice interaction.
    pub size: f64,
    /// Uses the shared glyph's area or nominal size when provided.
    pub glyph: Option<GlyphOptions>,
    pub color: InheritedColor,
    pub border: Option<GraphicStroke>,
}
impl Default for BumpPointStyle {
    fn default() -> Self {
        Self {
            size: 6.,
            glyph: None,
            color: InheritedColor::FromContext(vec![]),
            border: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpSeriesStyle {
    pub color: Option<Color>,
    pub width: f64,
    pub opacity: f64,
    pub dash: Option<StrokeDash>,
    pub point: BumpPointStyle,
}
impl Default for BumpSeriesStyle {
    fn default() -> Self {
        Self {
            color: None,
            width: 2.,
            opacity: 1.,
            dash: None,
            point: Default::default(),
        }
    }
}
impl BumpSeriesStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.width == 0. || (0.000001..=256.).contains(&self.width),
            "invalid Bump line width",
        )?;
        ensure(
            (0. ..=1.).contains(&self.opacity) && (0. ..=512.).contains(&self.point.size),
            "invalid Bump opacity or point size",
        )?;
        if let Some(stroke) = self.point.border {
            ensure(
                (0.000001..=256.).contains(&stroke.width),
                "invalid Bump point border",
            )?;
        }
        self.point.color.validate()?;
        if let Some(glyph) = &self.point.glyph {
            let geometry = glyph_geometry([0., 0.], glyph)?;
            ensure(
                geometry.paint_mode != GlyphPaintMode::Stroke || self.point.border.is_some(),
                "stroke-only Bump glyph requires a point border",
            )?;
        }
        if let Some(dash) = &self.dash {
            dash.validate()?;
        }
        Ok(())
    }
}
/// Optional overrides for one endpoint. Absent fields inherit the shared
/// [`BumpEndpointLabels`] padding and color. Size and enabled flags stay shared.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BumpEndpointLabelStyle {
    pub padding: Option<f64>,
    pub color: Option<InheritedColor>,
}
impl BumpEndpointLabelStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.padding
                .is_none_or(|p| p.is_finite() && (0. ..=16384.).contains(&p)),
            "invalid Bump endpoint label padding override",
        )?;
        if let Some(color) = &self.color {
            color.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpEndpointLabels {
    pub start: bool,
    pub end: bool,
    pub padding: f64,
    pub color: InheritedColor,
    pub size: Option<f64>,
    /// Start-only overrides; absent fields inherit `padding` and `color`.
    pub start_style: Option<BumpEndpointLabelStyle>,
    /// End-only overrides; absent fields inherit `padding` and `color`.
    pub end_style: Option<BumpEndpointLabelStyle>,
}
impl Default for BumpEndpointLabels {
    fn default() -> Self {
        Self {
            start: false,
            end: true,
            padding: 16.,
            color: InheritedColor::FromContext(vec![]),
            size: None,
            start_style: None,
            end_style: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpAxes {
    pub top: Option<AxisOptions>,
    pub right: Option<AxisOptions>,
    pub bottom: Option<AxisOptions>,
    pub left: Option<AxisOptions>,
}
impl Default for BumpAxes {
    fn default() -> Self {
        let grid = AxisOptions {
            grid: true,
            ..Default::default()
        };
        Self {
            top: Some(Default::default()),
            right: None,
            bottom: Some(grid.clone()),
            left: Some(grid),
        }
    }
}
impl BumpAxes {
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
pub struct BumpLayoutOptions {
    /// Fraction of one X step occupied by the flat portions around a point.
    /// Values above one are capped at a full step, as in Nivo.
    pub x_padding: f64,
    pub x_scale: PointScaleOptions,
    pub rank_scale: PointScaleOptions,
    pub interpolation: BumpInterpolation,
    pub style: BumpSeriesStyle,
    /// Complete overrides by series ID.
    pub series_styles: BTreeMap<String, BumpSeriesStyle>,
    pub colors: OrdinalColorOptions,
    /// Hide ink while preserving full source category/rank lattices and colors.
    pub hidden: BTreeSet<String>,
    pub axes: BumpAxes,
    /// Explicit typed X ticks, shared by the top and bottom axes.
    pub x_ticks: Option<Vec<CategoryKey>>,
    pub rank_format: ValueFormat,
    pub labels: BumpEndpointLabels,
    pub legend: bool,
}
impl Default for BumpLayoutOptions {
    fn default() -> Self {
        let scale = PointScaleOptions {
            padding: 0.5,
            ..Default::default()
        };
        Self {
            x_padding: 0.6,
            x_scale: scale,
            rank_scale: scale,
            interpolation: Default::default(),
            style: Default::default(),
            series_styles: BTreeMap::new(),
            colors: Default::default(),
            hidden: BTreeSet::new(),
            axes: Default::default(),
            x_ticks: None,
            rank_format: ValueFormat::Auto,
            labels: Default::default(),
            legend: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BumpPoint {
    pub source_index: usize,
    pub raw_x: CategoryKey,
    pub rank: Option<f64>,
    pub position: Option<[f64; 2]>,
    pub mark_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpComputedSeries {
    pub source: BumpSeries,
    pub source_index: usize,
    pub color: Color,
    pub hidden: bool,
    pub points: Vec<BumpPoint>,
    /// Includes a None separator for each source gap and the pre/post
    /// transition points in full Scene coordinates.
    pub line_points: Vec<Option<[f64; 2]>>,
    pub line_mark_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BumpLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub series: Vec<BumpComputedSeries>,
    pub x_scale: PointScale,
    pub rank_scale: PointScale,
    pub interaction: InteractionBinding,
}
impl BumpLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        self.interaction.clone()
    }
    pub fn into_scene_interaction(self) -> SceneInteraction {
        SceneInteraction {
            scene: self.scene,
            interaction: Some(self.interaction),
            plot: Some(self.plot),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BumpPointContext<'a> {
    pub series: &'a BumpSeries,
    pub datum: &'a BumpDatum,
    pub series_index: usize,
    pub source_index: usize,
    pub mark_id: &'a str,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub color: Color,
    pub style: &'a BumpPointStyle,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
}
impl BumpPointContext<'_> {
    pub fn inherit(&self, geometry: Geometry) -> LinePointGeometry {
        LinePointGeometry {
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
type PointFactory<'a> = dyn FnMut(&BumpPointContext<'_>) -> Result<LinePointGeometry> + 'a;

pub fn bump_layout(
    c: &ChartConfig,
    data: &[BumpSeries],
    o: &BumpLayoutOptions,
) -> Result<BumpLayout> {
    layout(c, data, o, None)
}
pub fn bump_layout_with_point_factory(
    c: &ChartConfig,
    data: &[BumpSeries],
    o: &BumpLayoutOptions,
    mut factory: impl FnMut(&BumpPointContext<'_>) -> Result<LinePointGeometry>,
) -> Result<BumpLayout> {
    layout(c, data, o, Some(&mut factory))
}

fn layout(
    c: &ChartConfig,
    data: &[BumpSeries],
    o: &BumpLayoutOptions,
    mut factory: Option<&mut PointFactory<'_>>,
) -> Result<BumpLayout> {
    let mut scene = c.scene()?;
    validate(data, o)?;
    let plot = ClipRect::plot(c);
    let x_scale = PointScale::new(
        data.iter()
            .flat_map(|s| s.data.iter().map(|p| p.x.clone()))
            .collect(),
        [plot.x, plot.x + plot.width],
        &o.x_scale,
    )?;
    let mut ranks = data
        .iter()
        .flat_map(|s| s.data.iter().filter_map(|p| p.rank))
        .collect::<Vec<_>>();
    ranks.sort_by(f64::total_cmp);
    ranks.dedup_by(|a, b| *a == *b);
    let rank_scale = PointScale::new(
        ranks.into_iter().map(CategoryKey::Number).collect(),
        [plot.y, plot.y + plot.height],
        &o.rank_scale,
    )?;
    let mut colors = o
        .colors
        .resolve(data.iter().map(|s| CategoryKey::from(s.id.as_str())))?;
    axes(&mut scene, c, &x_scale, &rank_scale, o)?;
    let [x_start, x_end] = x_scale.range();
    let shoulder =
        x_scale.step() * (o.x_padding * 0.5).min(0.5) * if x_end < x_start { -1. } else { 1. };
    let mut computed = Vec::with_capacity(data.len());
    let mut binding = InteractionBinding::default();
    let mut commands = 0;
    for (series_index, series) in data.iter().enumerate() {
        let style = o.series_styles.get(&series.id).unwrap_or(&o.style);
        let color = style
            .color
            .or(colors.map_or_insert(CategoryKey::from(series.id.as_str()))?)
            .ok_or_else(|| ChartError("Bump series is missing an ordinal color".into()))?;
        let hidden = o.hidden.contains(&series.id);
        let mut row = BumpComputedSeries {
            source: series.clone(),
            source_index: series_index,
            color,
            hidden,
            points: Vec::with_capacity(series.data.len()),
            line_points: vec![],
            line_mark_ids: vec![],
        };
        let mut points = Vec::new();
        for (source_index, datum) in series.data.iter().enumerate() {
            let position = datum.rank.map(|rank| {
                [
                    x_scale.map(&datum.x).unwrap(),
                    rank_scale.map(&CategoryKey::Number(rank)).unwrap(),
                ]
            });
            if let Some([x, y]) = position {
                row.line_points.extend([
                    Some([
                        if source_index == 0 {
                            x_start
                        } else {
                            x - shoulder
                        },
                        y,
                    ]),
                    Some([x, y]),
                    Some([
                        if source_index + 1 == series.data.len() && x != x_start {
                            x_end
                        } else {
                            x + shoulder
                        },
                        y,
                    ]),
                ]);
            } else {
                row.line_points.push(None);
            }
            let mut point = BumpPoint {
                source_index,
                raw_x: datum.x.clone(),
                rank: datum.rank,
                position,
                mark_id: None,
            };
            if !hidden && let Some(position) = position {
                let id = point_id(&series.id, &datum.x)?;
                let fill = style.point.color.resolve(InheritedColorContext {
                    inherited: color,
                    theme: c.theme.text,
                    surface: color,
                    backdrop: c.theme.background,
                })?;
                let is_stroked = style
                    .point
                    .glyph
                    .as_ref()
                    .is_some_and(|g| matches!(g.shape, crate::GlyphShape::Stroked(_)));
                let context = BumpPointContext {
                    series,
                    datum,
                    series_index,
                    source_index,
                    mark_id: &id,
                    position,
                    plot,
                    color,
                    style: &style.point,
                    fill: (!is_stroked).then_some(fill),
                    stroke: style.point.border,
                };
                let (mut mark, anchor, point_color) = if let Some(factory) = factory.as_deref_mut()
                {
                    let prepared = factory(&context)?.prepare_at(
                        &id,
                        position,
                        color,
                        200_000usize.saturating_sub(commands),
                    )?;
                    (prepared.mark, prepared.anchor, prepared.color)
                } else if let Some(glyph) = &style.point.glyph {
                    let geometry = glyph_geometry(position, glyph)?;
                    let mark = glyph_mark(&id, &geometry, context.fill, style.point.border)?;
                    let anchor = matches!(mark.geometry, Geometry::Path(_))
                        .then(|| {
                            InteractionAnchor::path_points(
                                &mark.geometry,
                                position,
                                &geometry.anchor_indices,
                            )
                        })
                        .transpose()?;
                    (mark, anchor, fill)
                } else {
                    let mut mark = Mark::new(
                        &id,
                        Geometry::Circle {
                            x: position[0],
                            y: position[1],
                            radius: style.point.size / 2.,
                        },
                        fill,
                    );
                    mark.stroke = style.point.border;
                    (mark, None, fill)
                };
                if let Geometry::Path(path) = &mark.geometry {
                    commands += path.len();
                }
                mark.series.clone_from(&series.id);
                mark.label = format!(
                    "{} · {}",
                    series.label,
                    category_text(&datum.x, &ValueFormat::Auto)?
                );
                mark.value = datum.rank.unwrap();
                mark.clip = Some(plot);
                mark.opacity *= style.opacity;
                if let Some(anchor) = anchor {
                    binding.anchors.insert(id.clone(), anchor);
                }
                binding.sites.push(InteractionSite {
                    mark_id: id.clone(),
                    series: series.id.clone(),
                    label: mark.label.clone(),
                    value: datum.rank,
                    formatted_value: Some(o.rank_format.format(mark.value)?),
                    color: point_color,
                    position,
                });
                point.mark_id = Some(id);
                points.push(mark);
            }
            row.points.push(point);
        }
        if !hidden {
            for (run_index, run) in row
                .line_points
                .split(Option::is_none)
                .filter(|run| !run.is_empty())
                .enumerate()
            {
                let run = run
                    .iter()
                    .map(|p| {
                        let [x, y] = p.unwrap();
                        P::new(x, y)
                    })
                    .collect::<Vec<_>>();
                if style.width > 0. {
                    let id = format!("bump-line:{}:{}:{run_index}", series.id.len(), series.id);
                    let path = path(
                        &run,
                        match o.interpolation {
                            BumpInterpolation::Smooth => Curve::Basis,
                            BumpInterpolation::Linear => Curve::Linear,
                        },
                    );
                    commands += path.len();
                    ensure(commands <= 200_000, "Bump path command budget exceeded")?;
                    let mut mark = Mark::new(&id, Geometry::Path(path), color);
                    mark.fill = None;
                    mark.label.clear();
                    mark.series.clone_from(&series.id);
                    mark.stroke = Some(GraphicStroke {
                        color,
                        width: style.width,
                        cap: StrokeCap::Butt,
                    });
                    mark.stroke_dash = style.dash.clone();
                    mark.opacity = style.opacity;
                    mark.clip = Some(plot);
                    scene.marks.push(mark);
                    row.line_mark_ids.push(id);
                }
            }
            scene.marks.extend(points);
            endpoint_labels(&mut scene, c, &row, style, &o.labels)?;
        }
        if o.legend {
            scene.legend.push(LegendItem {
                id: series.id.clone(),
                label: series.label.clone(),
                color,
            });
        }
        computed.push(row);
    }
    binding.validate(&scene)?;
    Ok(BumpLayout {
        scene,
        plot,
        series: computed,
        x_scale,
        rank_scale,
        interaction: binding,
    })
}

fn point_id(series: &str, x: &CategoryKey) -> Result<String> {
    let key = x.identity()?;
    Ok(format!(
        "bump-point:{}:{}:{}:{}",
        series.len(),
        series,
        key.len(),
        key
    ))
}

fn validate(data: &[BumpSeries], o: &BumpLayoutOptions) -> Result<()> {
    ensure(data.len() <= 256, "Bump series budget exceeded")?;
    ensure(
        o.x_padding.is_finite() && (0. ..=1e6).contains(&o.x_padding),
        "invalid Bump shoulder padding",
    )?;
    o.x_scale.validate()?;
    o.rank_scale.validate()?;
    o.rank_format.validate()?;
    o.style.validate()?;
    o.labels.color.validate()?;
    ensure(
        o.labels.padding.is_finite()
            && (0. ..=16384.).contains(&o.labels.padding)
            && o.labels.size.is_none_or(|s| (1. ..=256.).contains(&s)),
        "invalid Bump endpoint labels",
    )?;
    for style in [&o.labels.start_style, &o.labels.end_style]
        .into_iter()
        .flatten()
    {
        style.validate()?;
    }
    ensure(
        o.series_styles.len() <= 256 && o.hidden.len() <= 256,
        "Bump style/hidden budget exceeded",
    )?;
    for style in o.series_styles.values() {
        style.validate()?;
    }
    let mut ids = BTreeSet::new();
    let mut count = 0;
    for row in data {
        ensure(
            !row.id.trim().is_empty()
                && row.id.len() <= 256
                && row.label.len() <= 512
                && ids.insert(&row.id),
            "invalid Bump series identity",
        )?;
        count += row.data.len();
        ensure(
            count <= MAX_BUMP_POINTS,
            "Bump source point budget exceeded",
        )?;
        let mut categories = BTreeSet::new();
        for point in &row.data {
            let key = point.x.identity()?;
            ensure(
                key.len() <= 512 && categories.insert(key),
                "Bump X keys must be unique per series and at most 512 bytes",
            )?;
            ensure(
                point.rank.is_none_or(f64::is_finite),
                "Bump ranks must be finite or missing",
            )?;
        }
    }
    ensure(
        o.hidden
            .iter()
            .chain(o.series_styles.keys())
            .all(|id| ids.contains(id)),
        "Bump option refers to unknown series",
    )?;
    if let Some(ticks) = &o.x_ticks {
        ensure(ticks.len() <= 1000, "Bump X tick budget exceeded")?;
        for tick in ticks {
            tick.validate()?;
        }
    }
    for (horizontal, axis) in [
        (true, &o.axes.top),
        (true, &o.axes.bottom),
        (false, &o.axes.left),
        (false, &o.axes.right),
    ] {
        if let Some(axis) = axis {
            axis.validate()?;
            ensure(
                axis.tick_calendar.is_none() && (!horizontal || axis.tick_values.is_none()),
                "Bump axes use discrete ranks and typed X ticks",
            )?;
        }
    }
    Ok(())
}

fn endpoint_labels(
    scene: &mut Scene,
    c: &ChartConfig,
    row: &BumpComputedSeries,
    style: &BumpSeriesStyle,
    options: &BumpEndpointLabels,
) -> Result<()> {
    for (enabled, start) in [(options.start, true), (options.end, false)] {
        if !enabled {
            continue;
        }
        let point = if start {
            row.line_points.first()
        } else {
            row.line_points.last()
        };
        let Some(Some([x, y])) = point else {
            continue;
        };
        let endpoint_style = if start {
            options.start_style.as_ref()
        } else {
            options.end_style.as_ref()
        };
        let padding = endpoint_style
            .and_then(|style| style.padding)
            .unwrap_or(options.padding);
        let color = endpoint_style
            .and_then(|style| style.color.as_ref())
            .unwrap_or(&options.color);
        let Color(r, g, b, a) = color.resolve(InheritedColorContext {
            inherited: row.color,
            theme: c.theme.text,
            surface: row.color,
            backdrop: scene.background,
        })?;
        let label = Label {
            x: x + if start { -padding } else { padding },
            y: *y,
            rotation: 0.,
            text: row.source.label.clone(),
            color: Color(r, g, b, (f64::from(a) * style.opacity).round() as u8),
            size: options.size.unwrap_or(c.theme.font_size),
            anchor: if start {
                TextAnchor::End
            } else {
                TextAnchor::Start
            },
        };
        scene
            .label_series
            .insert(scene.labels.len(), row.source.id.clone());
        scene.labels.push(label);
    }
    Ok(())
}

fn axes(
    scene: &mut Scene,
    c: &ChartConfig,
    x: &PointScale,
    y: &PointScale,
    options: &BumpLayoutOptions,
) -> Result<()> {
    for (side, axis) in [
        (crate::AxisSide::Top, &options.axes.top),
        (crate::AxisSide::Right, &options.axes.right),
        (crate::AxisSide::Bottom, &options.axes.bottom),
        (crate::AxisSide::Left, &options.axes.left),
    ] {
        let Some(axis) = axis else {
            continue;
        };
        let horizontal = matches!(side, crate::AxisSide::Top | crate::AxisSide::Bottom);
        let scale = if horizontal { x } else { y };
        let ticks = if horizontal {
            options
                .x_ticks
                .clone()
                .unwrap_or_else(|| auto_ticks(scale, axis.tick_count))
        } else if let Some(ticks) = &axis.tick_values {
            ticks.iter().copied().map(CategoryKey::Number).collect()
        } else {
            auto_ticks(scale, axis.tick_count)
        };
        let negative = matches!(side, crate::AxisSide::Top | crate::AxisSide::Left);
        let direction = if negative { -1. } else { 1. };
        let baseline = match side {
            crate::AxisSide::Top => c.margin.top,
            crate::AxisSide::Bottom => c.height - c.margin.bottom,
            crate::AxisSide::Left => c.margin.left,
            crate::AxisSide::Right => c.width - c.margin.right,
        };
        let project = |a, b| if horizontal { [a, b] } else { [b, a] };
        let name = match side {
            crate::AxisSide::Top => "top",
            crate::AxisSide::Bottom => "bottom",
            crate::AxisSide::Left => "left",
            crate::AxisSide::Right => "right",
        };
        let mut seen = BTreeSet::new();
        for tick in ticks {
            let identity = tick.identity()?;
            if !seen.insert(identity.clone()) {
                continue;
            }
            let value = scale
                .map(&tick)
                .ok_or_else(|| ChartError("Bump axis tick is outside its point domain".into()))?;
            if axis.grid {
                let span = if horizontal {
                    [c.margin.top, c.height - c.margin.bottom]
                } else {
                    [c.margin.left, c.width - c.margin.right]
                };
                segment(
                    scene,
                    format!("bump-axis-{name}-grid:{identity}"),
                    project(value, span[0]),
                    project(value, span[1]),
                    c.theme.grid,
                );
            }
            if axis.visible {
                segment(
                    scene,
                    format!("bump-axis-{name}-tick:{identity}"),
                    project(value, baseline),
                    project(value, baseline + direction * axis.tick_size),
                    c.theme.text,
                );
                let [lx, ly] = project(
                    value,
                    baseline + direction * (axis.tick_size + axis.tick_padding),
                );
                scene.labels.push(Label {
                    x: lx,
                    y: ly
                        + if horizontal && !negative {
                            c.theme.font_size
                        } else if !horizontal {
                            c.theme.font_size / 3.
                        } else {
                            0.
                        },
                    text: category_text(&tick, &axis.format)?,
                    color: c.theme.text,
                    size: c.theme.font_size,
                    rotation: axis.tick_rotation,
                    anchor: if horizontal {
                        TextAnchor::Middle
                    } else if negative {
                        TextAnchor::End
                    } else {
                        TextAnchor::Start
                    },
                });
            }
        }
        if axis.visible {
            let [start, end] = if horizontal {
                [c.margin.left, c.width - c.margin.right]
            } else {
                [c.margin.top, c.height - c.margin.bottom]
            };
            segment(
                scene,
                format!("bump-axis-{name}"),
                project(start, baseline),
                project(end, baseline),
                c.theme.text,
            );
            if let Some(text) = &axis.legend {
                let [lx, ly] = project(
                    (start + end) / 2.,
                    baseline
                        + direction
                            * (axis.tick_size + axis.tick_padding + c.theme.font_size * 2.5),
                );
                scene.labels.push(Label {
                    x: lx,
                    y: ly,
                    text: text.clone(),
                    color: c.theme.text,
                    size: c.theme.font_size,
                    anchor: TextAnchor::Middle,
                    rotation: if horizontal { 0. } else { -90. },
                });
            }
        }
    }
    Ok(())
}
fn auto_ticks(scale: &PointScale, count: usize) -> Vec<CategoryKey> {
    if count == 0 {
        return vec![];
    }
    scale
        .domain()
        .iter()
        .step_by(scale.domain().len().div_ceil(count).max(1))
        .cloned()
        .collect()
}
fn segment(scene: &mut Scene, id: String, a: [f64; 2], b: [f64; 2], color: Color) {
    let mut mark = Mark::new(
        id,
        Geometry::Path(vec![
            C::Move(P::new(a[0], a[1])),
            C::Line(P::new(b[0], b[1])),
        ]),
        color,
    );
    mark.fill = None;
    mark.label.clear();
    mark.stroke = Some(GraphicStroke {
        color,
        width: 1.,
        cap: StrokeCap::Butt,
    });
    scene.marks.push(mark);
}
