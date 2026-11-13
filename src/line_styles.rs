//! Series-local line/area/point presentation using the existing Cartesian
//! layout. Nivo references: line/src/{defaults,Points,Areas,LinesItem}.ts(x).
//! Colors may carry independent alpha; areas and stroke opacity are separate.
use std::collections::BTreeMap;

use glory::render::{GraphicStroke, StrokeCap};

use crate::scene::ensure;
use crate::*;

#[derive(Clone, Debug, Default, PartialEq)]
pub enum LinePointLabelText {
    /// The same y formatter used by the layout's point tooltip.
    #[default]
    FormattedY,
    Value(ValueFormat),
    Coordinates {
        x: ValueFormat,
        y: ValueFormat,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct LinePointLabels {
    pub text: LinePointLabelText,
    pub offset: [f64; 2],
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
    pub anchor: TextAnchor,
    pub clip_to_plot: bool,
}
impl Default for LinePointLabels {
    fn default() -> Self {
        Self {
            text: Default::default(),
            offset: [0., -9.],
            color: None,
            size: None,
            rotation: 0.,
            anchor: TextAnchor::Middle,
            clip_to_plot: false,
        }
    }
}
impl LinePointLabels {
    fn validate(&self) -> Result<()> {
        match &self.text {
            LinePointLabelText::FormattedY => {}
            LinePointLabelText::Value(f) => f.validate()?,
            LinePointLabelText::Coordinates { x, y } => {
                x.validate()?;
                y.validate()?;
            }
        }
        ensure(
            self.offset
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 16384.)
                && self
                    .size
                    .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && self.rotation.is_finite()
                && (-360. ..=360.).contains(&self.rotation),
            "invalid line point-label style",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineSeriesStyle {
    pub color: Option<Color>,
    pub width: f64,
    pub line_opacity: f64,
    pub dash: Option<StrokeDash>,
    /// None inherits LineOptions.points. False can still keep zero-radius
    /// interaction sites when LineStyleOptions.interactive_sites is true.
    pub points: Option<bool>,
    pub point_radius: f64,
    pub point_color: Option<Color>,
    pub point_border: Option<GraphicStroke>,
    /// Some uses the shared D3 glyph's explicit size/rotation; None retains
    /// the historical circle radius. Stroke-only glyphs require point_border.
    pub point_glyph: Option<GlyphOptions>,
    pub point_labels: Option<LinePointLabels>,
    pub area_opacity: f64,
    pub area_paint: Option<FillPaint>,
}
impl Default for LineSeriesStyle {
    fn default() -> Self {
        Self {
            color: None,
            width: 2.,
            line_opacity: 1.,
            dash: None,
            points: None,
            point_radius: 3.5,
            point_color: None,
            point_border: None,
            point_glyph: None,
            point_labels: None,
            area_opacity: 0.35,
            area_paint: None,
        }
    }
}
impl LineSeriesStyle {
    pub fn validate(&self) -> Result<()> {
        if let Some(glyph) = &self.point_glyph {
            let geometry = glyph_geometry([0., 0.], glyph)?;
            ensure(
                geometry.paint_mode != GlyphPaintMode::Stroke || self.point_border.is_some(),
                "stroke-only line glyph requires an explicit point border",
            )?;
        }
        ensure(
            self.width.is_finite()
                && (0. ..=256.).contains(&self.width)
                && self.point_radius.is_finite()
                && (0. ..=256.).contains(&self.point_radius)
                && [self.line_opacity, self.area_opacity]
                    .iter()
                    .all(|v| v.is_finite() && (0. ..=1.).contains(v)),
            "invalid line series paint style",
        )?;
        if let Some(stroke) = self.point_border {
            ensure(
                stroke.width.is_finite() && (0.000001..=256.).contains(&stroke.width),
                "invalid line point border",
            )?;
        }
        if let Some(dash) = &self.dash {
            dash.validate()?;
        }
        if let Some(labels) = &self.point_labels {
            labels.validate()?;
        }
        if let Some(paint) = &self.area_paint {
            paint.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LineStyleOptions {
    pub series: LineSeriesStyle,
    /// Complete overrides. An omitted color uses the ordinal palette when
    /// provided and falls back to the theme. Other fields use this entry as is.
    pub series_styles: BTreeMap<String, LineSeriesStyle>,
    pub connect_missing: bool,
    /// Zero-radius, unpainted circles preserve KD-tree/slice sites when points
    /// are disabled. Select Picking::Nearest/SliceX/SliceY to use those sites.
    pub interactive_sites: bool,
}
impl LineStyleOptions {
    pub fn validate(&self) -> Result<()> {
        self.series.validate()?;
        ensure(
            self.series_styles.len() <= 256,
            "line series style count budget exceeded",
        )?;
        for (id, style) in &self.series_styles {
            ensure(
                !id.is_empty() && id.len() <= 256,
                "invalid styled line series ID",
            )?;
            style.validate()?;
        }
        Ok(())
    }
    fn style(&self, id: &str) -> &LineSeriesStyle {
        self.series_styles.get(id).unwrap_or(&self.series)
    }
}

/// Caller-authored geometry and paint for one Line point.
pub use crate::scatter_layout::ScatterNodeGeometry as LinePointGeometry;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LinePointX<'a> {
    Continuous(f64),
    Category(&'a CategoryKey),
}
impl LinePointX<'_> {
    fn format(self, format: &ValueFormat, scale: ScaleKind) -> Result<String> {
        match self {
            Self::Continuous(value) => line_format(format, value, scale),
            Self::Category(key) => crate::line_scales::category_text(key, format),
        }
    }
}

/// Original values and resolved presentation for an enabled Line point.
#[derive(Clone, Copy, Debug)]
pub struct LinePointContext<'a> {
    pub series_id: &'a str,
    pub series_index: usize,
    pub source_index: usize,
    pub mark_id: &'a str,
    pub raw_x: LinePointX<'a>,
    pub raw_y: f64,
    pub formatted_x: &'a str,
    pub formatted_y: &'a str,
    /// The exact data anchor in full Scene coordinates.
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub series_color: Color,
    pub style: &'a LineSeriesStyle,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
}
impl LinePointContext<'_> {
    /// Replace geometry while preserving the configured point fill and border.
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

pub(crate) type LinePointFactory<'a> =
    dyn FnMut(&LinePointContext<'_>) -> Result<LinePointGeometry> + 'a;

#[derive(Default)]
pub(crate) struct LinePointRender<'a> {
    pub colors: Option<&'a OrdinalColorScale>,
    pub factory: Option<&'a mut LinePointFactory<'a>>,
    pub categories: Option<(&'a [PointLineSeries], &'a ValueFormat)>,
}

fn point_label_text(
    text: &LinePointLabelText,
    raw_x: LinePointX<'_>,
    raw_y: f64,
    layout: &LineOptions,
) -> Result<String> {
    match text {
        LinePointLabelText::FormattedY => line_format(&layout.axes.y.format, raw_y, layout.y_scale),
        LinePointLabelText::Value(f) => line_format(f, raw_y, layout.y_scale),
        LinePointLabelText::Coordinates { x, y } => Ok(format!(
            "{}, {}",
            raw_x.format(x, layout.x_scale)?,
            line_format(y, raw_y, layout.y_scale)?
        )),
    }
}
/// Reuse the complete line scale/domain/clipping/curve pipeline, then apply
/// paint by explicit generated roles. Point IDs contain the series ID and its
/// original source index, so missing-value connection never renumbers sites.
pub fn line_layout_with_style(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
) -> Result<LineLayout> {
    line_layout_impl(config, data, layout, style, None, Default::default())
}

/// Use a resolved ordinal series palette for the line, area, points, data
/// colors and legend. Explicit series paint overrides the palette. A missing
/// palette key uses the theme fallback.
pub fn line_layout_with_ordinal_colors(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    colors: &OrdinalColorScale,
) -> Result<LineLayout> {
    line_layout_impl(
        config,
        data,
        layout,
        style,
        None,
        LinePointRender {
            colors: Some(colors),
            ..Default::default()
        },
    )
}

/// Replace enabled points within the displayed domains. Hidden series, gaps
/// and disabled points do not call the factory. The geometry uses the plot
/// clip; labels, nearest and slice picking use the original data anchor.
/// Series swatches and interaction colors retain their resolved series color.
pub fn line_layout_with_point_factory(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    colors: Option<&OrdinalColorScale>,
    mut factory: impl FnMut(&LinePointContext<'_>) -> Result<LinePointGeometry>,
) -> Result<LineLayout> {
    line_layout_impl(
        config,
        data,
        layout,
        style,
        None,
        LinePointRender {
            colors,
            factory: Some(&mut factory),
            categories: None,
        },
    )
}
pub(crate) fn line_layout_with_resolved_scales(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: (&ChartScale, &ChartScale),
    render: LinePointRender<'_>,
) -> Result<LineLayout> {
    line_layout_impl(config, data, layout, style, Some(scales), render)
}
fn line_layout_impl(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
    scales: Option<(&ChartScale, &ChartScale)>,
    mut render: LinePointRender<'_>,
) -> Result<LineLayout> {
    style.validate()?;
    crate::cartesian::validate_series(data)?;
    ensure(
        layout.hidden.len() <= MAX_MARKS
            && layout
                .hidden
                .iter()
                .all(|id| !id.is_empty() && id.len() <= 256),
        "invalid hidden line IDs",
    )?;
    let mut input = data.to_vec();
    let mut indices = Vec::with_capacity(data.len());
    for row in &mut input {
        let original: Vec<_> = row
            .data
            .iter()
            .enumerate()
            .filter(|(_, p)| !style.connect_missing || p.y.is_some())
            .map(|(i, _)| i)
            .collect();
        row.data = original.iter().map(|i| row.data[*i].clone()).collect();
        indices.push(original);
    }
    let mut options = layout.clone();
    if style.connect_missing && options.x_domain.is_none() {
        options.x_domain = Some(crate::cartesian::scale_domain(
            data.iter()
                .filter(|row| !layout.hidden.contains(&row.id))
                .flat_map(|row| row.data.iter().map(|p| p.x)),
            layout.x_scale,
        )?);
    }
    options.points = style.interactive_sites
        || data.iter().any(|row| {
            let s = style.style(&row.id);
            s.points.unwrap_or(layout.points) || s.point_labels.is_some()
        });
    let mut scene = if let Some((x, y)) = scales {
        crate::cartesian::line_with_scales(config, &input, &options, x, y)?
    } else {
        crate::line(config, &input, &options)?
    };
    let series_indices: BTreeMap<_, _> = data
        .iter()
        .enumerate()
        .map(|(i, row)| (row.id.as_str(), i))
        .collect();
    let color = |i, id: &str| {
        style
            .style(id)
            .color
            .or_else(|| {
                render
                    .colors
                    .and_then(|scale| scale.map(&CategoryKey::from(id)))
            })
            .unwrap_or_else(|| config.theme.color(i))
    };
    let series_colors: Vec<_> = data
        .iter()
        .enumerate()
        .map(|(i, row)| color(i, &row.id))
        .collect();
    let mut marks = Vec::with_capacity(scene.marks.len());
    let mut sites = Vec::new();
    let mut anchors = BTreeMap::new();
    let mut point_labels = Vec::new();
    let mut point_commands = 0usize;
    let mut point_stops = 0usize;
    let clip = ClipRect::plot(config);
    for mut mark in std::mem::take(&mut scene.marks) {
        let Some(&si) = series_indices.get(mark.series.as_str()) else {
            marks.push(mark);
            continue;
        };
        let row = &data[si];
        let s = style.style(&row.id);
        let color = series_colors[si];
        let prefix = format!("{}:{}", row.id.len(), row.id);
        if let Some(suffix) = mark.id.strip_prefix(&format!("point-{si}-")) {
            let pi = suffix
                .parse::<usize>()
                .map_err(|_| ChartError("unexpected line point ID".into()))?;
            let original = *indices[si]
                .get(pi)
                .ok_or_else(|| ChartError("line source-index mapping is absent".into()))?;
            let Geometry::Circle { x, y, .. } = mark.geometry else {
                return Err(ChartError("unexpected line point geometry".into()));
            };
            mark.id = format!("line-point:{prefix}:{original}");
            let source = &row.data[original];
            let raw_y = source.y.unwrap();
            let (raw_x, x_format) = render.categories.map_or(
                (LinePointX::Continuous(source.x), &layout.axes.x.format),
                |(rows, format)| (LinePointX::Category(&rows[si].data[original].x), format),
            );
            let mut label_index = None;
            if let Some(labels) = &s.point_labels {
                let index = scene.labels.len();
                label_index = Some(index);
                point_labels.push(LinePointLabelSite {
                    series_id: row.id.clone(),
                    source_index: original,
                    label_index: index,
                });
                scene.label_series.insert(index, row.id.clone());
                if labels.clip_to_plot {
                    scene.label_clips.insert(
                        index,
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
                scene.labels.push(Label {
                    x: x + labels.offset[0],
                    y: y + labels.offset[1],
                    rotation: labels.rotation,
                    text: point_label_text(&labels.text, raw_x, raw_y, layout)?,
                    color: labels.color.unwrap_or(config.theme.text),
                    size: labels.size.unwrap_or(config.theme.font_size),
                    anchor: labels.anchor,
                });
            }
            let enabled = s.points.unwrap_or(layout.points);
            let custom = enabled && render.factory.is_some();
            let builtin = enabled && (s.point_radius > 0. || s.point_glyph.is_some());
            if !custom && !builtin && !style.interactive_sites {
                continue;
            }
            let formatted_x = raw_x.format(x_format, layout.x_scale)?;
            let formatted_y = line_format(&layout.axes.y.format, raw_y, layout.y_scale)?;
            if let Some(factory) = render.factory.as_deref_mut().filter(|_| enabled) {
                let context = LinePointContext {
                    series_id: &row.id,
                    series_index: si,
                    source_index: original,
                    mark_id: &mark.id,
                    raw_x,
                    raw_y,
                    formatted_x: &formatted_x,
                    formatted_y: &formatted_y,
                    position: [x, y],
                    plot: clip,
                    series_color: color,
                    style: s,
                    fill: (!s
                        .point_glyph
                        .as_ref()
                        .is_some_and(|glyph| matches!(glyph.shape, GlyphShape::Stroked(_))))
                    .then_some(s.point_color.unwrap_or(color)),
                    stroke: s.point_border,
                };
                let crate::scatter_layout::PreparedNode {
                    mark: mut point,
                    anchor,
                    ..
                } = factory(&context)?.prepare_at(
                    &mark.id,
                    context.position,
                    color,
                    200_000usize.saturating_sub(point_commands),
                )?;
                if let Geometry::Path(path) = &point.geometry {
                    point_commands += path.len();
                }
                point_stops += point
                    .fill_paint
                    .as_ref()
                    .map_or(0, |paint| paint.stops().len());
                ensure(
                    point_stops <= MAX_SCENE_GRADIENT_STOPS,
                    "line point gradient stop budget exceeded",
                )?;
                if let Some(anchor) = anchor {
                    anchors.insert(mark.id.clone(), anchor);
                }
                point.series = mark.series;
                point.label = mark.label;
                point.value = mark.value;
                point.clip = mark.clip;
                mark = point;
            } else if builtin {
                if let Some(glyph) = &s.point_glyph {
                    let geometry = glyph_geometry([x, y], glyph)?;
                    let fill = (geometry.paint_mode == GlyphPaintMode::Fill)
                        .then_some(s.point_color.unwrap_or(color));
                    let painted = glyph_mark(&mark.id, &geometry, fill, s.point_border)?;
                    mark.geometry = painted.geometry;
                    mark.fill = painted.fill;
                    mark.stroke = painted.stroke;
                    if matches!(mark.geometry, Geometry::Path(_)) {
                        anchors.insert(
                            mark.id.clone(),
                            InteractionAnchor::path_points(
                                &mark.geometry,
                                [x, y],
                                &geometry.anchor_indices,
                            )?,
                        );
                    }
                } else {
                    mark.geometry = Geometry::Circle {
                        x,
                        y,
                        radius: s.point_radius,
                    };
                    mark.fill = Some(s.point_color.unwrap_or(color));
                    mark.stroke = s.point_border;
                }
            } else {
                mark.geometry = Geometry::Circle { x, y, radius: 0. };
                mark.fill = Some(Color(0, 0, 0, 0));
                mark.stroke = None;
            }
            sites.push(LineSite {
                mark_id: mark.id.clone(),
                series_id: row.id.clone(),
                source_index: original,
                raw_x: source.x,
                raw_y,
                formatted_x,
                formatted_y,
                series_color: color,
                position: [x, y],
                label_index,
            });
            marks.push(mark);
        } else if let Some(suffix) = mark.id.strip_prefix(&format!("line-{si}-")) {
            if s.width == 0. {
                continue;
            }
            mark.id = format!("line-stroke:{prefix}:{suffix}");
            mark.stroke = Some(GraphicStroke {
                color,
                width: s.width,
                cap: StrokeCap::Round,
            });
            mark.stroke_dash = s.dash.clone();
            mark.opacity = s.line_opacity;
            marks.push(mark);
        } else if let Some(suffix) = mark.id.strip_prefix(&format!("area-{si}-")) {
            mark.id = format!("line-area:{prefix}:{suffix}");
            mark.fill = Some(color);
            mark.fill_paint = s.area_paint.clone();
            mark.opacity = s.area_opacity;
            marks.push(mark);
        } else {
            return Err(ChartError("unknown generated line data role".into()));
        }
    }
    scene.marks = marks;
    // Keep hidden identities available for application-controlled rebuilding,
    // as pie does; hidden data is still omitted from domains and geometry.
    scene.legend = data
        .iter()
        .enumerate()
        .map(|(i, row)| LegendItem {
            id: row.id.clone(),
            label: row.id.clone(),
            color: series_colors[i],
        })
        .collect();
    scene.validate()?;
    let positions = sites
        .iter()
        .map(|site| (site.mark_id.clone(), site.position))
        .collect();
    let mesh = crate::mesh::PointMesh::from_scene_at(&scene, &positions)?;
    let source_order = sites
        .iter()
        .enumerate()
        .map(|(i, s)| (s.mark_id.as_str(), i))
        .collect::<BTreeMap<_, _>>();
    let make_slices = |axis: crate::mesh::SliceAxis| {
        let index = crate::mesh::SliceIndex::new(&mesh, axis);
        index
            .slices()
            .iter()
            .enumerate()
            .map(|(i, slice)| {
                let before = if i == 0 {
                    if axis == crate::mesh::SliceAxis::X {
                        clip.x
                    } else {
                        clip.y
                    }
                } else {
                    (index.slices()[i - 1].coordinate + slice.coordinate) / 2.
                };
                let after = if i + 1 == index.slices().len() {
                    if axis == crate::mesh::SliceAxis::X {
                        clip.x + clip.width
                    } else {
                        clip.y + clip.height
                    }
                } else {
                    (index.slices()[i + 1].coordinate + slice.coordinate) / 2.
                };
                let bounds = if axis == crate::mesh::SliceAxis::X {
                    ClipRect {
                        x: before,
                        y: clip.y,
                        width: (after - before).max(0.),
                        height: clip.height,
                    }
                } else {
                    ClipRect {
                        x: clip.x,
                        y: before,
                        width: clip.width,
                        height: (after - before).max(0.),
                    }
                };
                LineSlice {
                    id: format!(
                        "line-slice:{}:{:016x}",
                        if axis == crate::mesh::SliceAxis::X {
                            "x"
                        } else {
                            "y"
                        },
                        slice.coordinate.to_bits()
                    ),
                    axis,
                    coordinate: slice.coordinate,
                    bounds,
                    member_ids: {
                        let mut ids = slice.ids.clone();
                        ids.sort_by_key(|id| {
                            std::cmp::Reverse(source_order.get(id.as_str()).copied().unwrap_or(0))
                        });
                        ids
                    },
                }
            })
            .collect::<Vec<_>>()
    };
    let slices_x = make_slices(crate::mesh::SliceAxis::X);
    let slices_y = make_slices(crate::mesh::SliceAxis::Y);
    let result = LineLayout {
        scene,
        sites,
        slices_x,
        slices_y,
        plot: clip,
        anchors,
        point_labels,
    };
    result.interaction_binding().validate(&result.scene)?;
    Ok(result)
}

fn line_format(format: &ValueFormat, value: f64, scale: ScaleKind) -> Result<String> {
    if scale == ScaleKind::Utc && *format == ValueFormat::Auto {
        ValueFormat::Utc.format(value)
    } else {
        format.format(value)
    }
}
/// Original Scene-only entry point; its drawing and IDs remain compatible.
pub fn line_with_style(
    config: &ChartConfig,
    data: &[Series],
    layout: &LineOptions,
    style: &LineStyleOptions,
) -> Result<Scene> {
    line_layout_with_style(config, data, layout, style).map(|layout| layout.scene)
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineSite {
    pub mark_id: String,
    pub series_id: String,
    pub source_index: usize,
    pub raw_x: f64,
    pub raw_y: f64,
    pub formatted_x: String,
    pub formatted_y: String,
    pub series_color: Color,
    pub position: [f64; 2],
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinePointLabelSite {
    pub series_id: String,
    pub source_index: usize,
    pub label_index: usize,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineSlice {
    pub id: String,
    pub axis: crate::mesh::SliceAxis,
    pub coordinate: f64,
    pub bounds: ClipRect,
    pub member_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LineLayout {
    pub plot: ClipRect,
    pub scene: Scene,
    pub sites: Vec<LineSite>,
    pub slices_x: Vec<LineSlice>,
    pub slices_y: Vec<LineSlice>,
    pub anchors: BTreeMap<String, InteractionAnchor>,
    /// Source ownership also covers labels whose visible glyph/site is disabled.
    pub point_labels: Vec<LinePointLabelSite>,
}
impl LineLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            anchors: self.anchors.clone(),
            sites: self
                .sites
                .iter()
                .map(|site| InteractionSite {
                    mark_id: site.mark_id.clone(),
                    series: site.series_id.clone(),
                    label: site.series_id.clone(),
                    value: Some(site.raw_y),
                    formatted_value: Some(site.formatted_y.clone()),
                    color: site.series_color,
                    position: site.position,
                })
                .collect(),
            ..Default::default()
        }
    }
}
impl From<LineLayout> for SceneInteraction {
    fn from(layout: LineLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            interaction,
            plot: Some(layout.plot),
        }
    }
}
