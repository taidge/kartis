//! Stream layers, raw stacks, point scales and separate layer and slice bindings.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use crate::cartesian::path;
use crate::scene::{MAX_LABEL_BYTES, MAX_MARKS, ensure};
use crate::stack::{StackOffset, StackOptions, StackResult, stack};
use crate::{
    BarSeries, CartesianAxes, CategoryKey, ChartConfig, ChartError, ClipRect, Color, ColorModifier,
    ContinuousScaleOptions, Curve, FillPaint, Geometry, GlyphOptions, GlyphPaintMode, GlyphShape,
    InheritedColor, InheritedColorContext, InteractionAnchor, InteractionBinding, InteractionSite,
    LegendItem, LineOptions, LineStyleOptions, Mark, OrdinalColorOptions, OrdinalColorScale,
    PointLineOptions, PointScaleOptions, ResolvedContinuousScale, ResolvedLinePointScale, Result,
    ScaleKind, ScaleNice, Scene, SceneInteraction, ValueFormat, glyph_geometry, glyph_mark,
    point_line_layout,
};

pub const MAX_STREAM_CELLS: usize = 16_384;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StreamDotPosition {
    Start,
    #[default]
    Center,
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamDotOptions {
    pub position: StreamDotPosition,
    /// Circle diameter in chart pixels.
    pub size: f64,
    /// Shared glyph area or nominal size. `None` uses the circle diameter.
    pub glyph: Option<GlyphOptions>,
    pub color: InheritedColor,
    pub border_width: f64,
    pub border_color: InheritedColor,
}

impl Default for StreamDotOptions {
    fn default() -> Self {
        Self {
            position: Default::default(),
            size: 6.,
            glyph: None,
            color: InheritedColor::FromContext(vec![]),
            border_width: 0.,
            border_color: InheritedColor::FromContext(vec![]),
        }
    }
}

/// A caller-authored point uses the shared Line/Scatter geometry contract.
pub use crate::LinePointGeometry as StreamDotGeometry;

#[derive(Clone, Copy, Debug)]
pub struct StreamDotContext<'a> {
    pub series: &'a BarSeries,
    pub series_index: usize,
    pub layer_label: &'a str,
    pub sample: &'a StreamSample,
    /// The exact Start, Center or End coordinate in full Scene pixels.
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub color: Color,
    pub style: &'a StreamDotOptions,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
}

impl StreamDotContext<'_> {
    /// Replace geometry while keeping the configured dot fill and border.
    pub fn inherit(&self, geometry: Geometry) -> StreamDotGeometry {
        StreamDotGeometry {
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

type DotFactory<'a> = dyn FnMut(&StreamDotContext<'_>) -> Result<StreamDotGeometry> + 'a;

#[derive(Clone, Debug, PartialEq)]
pub struct StreamLayerStyle {
    pub color: Option<Color>,
    pub paint: Option<FillPaint>,
    pub fill_opacity: f64,
    pub border_width: f64,
    pub border_color: InheritedColor,
}

impl Default for StreamLayerStyle {
    fn default() -> Self {
        Self {
            color: None,
            paint: None,
            fill_opacity: 1.,
            border_width: 0.,
            border_color: InheritedColor::FromContext(vec![ColorModifier::Darker(1.)]),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamLayoutOptions {
    pub stack: StackOptions,
    pub curve: Curve,
    pub x_scale: PointScaleOptions,
    pub y_scale: ContinuousScaleOptions,
    /// Override either raw stack-domain endpoint before resolving the scale.
    pub y_bounds: [Option<f64>; 2],
    pub axes: CartesianAxes,
    pub x_ticks: Option<Vec<CategoryKey>>,
    pub colors: OrdinalColorOptions,
    pub style: StreamLayerStyle,
    /// Complete style overrides keyed by series ID.
    pub styles: BTreeMap<String, StreamLayerStyle>,
    pub labels: BTreeMap<String, String>,
    /// Hide ink while retaining stack slots, scales, colors and raw metadata.
    pub hidden: BTreeSet<String>,
    pub dots: Option<StreamDotOptions>,
    pub value_format: ValueFormat,
    pub legend: bool,
}

impl Default for StreamLayoutOptions {
    fn default() -> Self {
        Self {
            stack: StackOptions {
                offset: StackOffset::Wiggle,
                ..Default::default()
            },
            curve: Curve::CatmullRom { alpha: 0.5 },
            x_scale: Default::default(),
            y_scale: ContinuousScaleOptions {
                nice: ScaleNice::Default,
                ..Default::default()
            },
            y_bounds: [None, None],
            axes: Default::default(),
            x_ticks: None,
            colors: Default::default(),
            style: Default::default(),
            styles: Default::default(),
            labels: Default::default(),
            hidden: Default::default(),
            dots: None,
            value_format: Default::default(),
            legend: false,
        }
    }
}

fn border_width(width: f64) -> bool {
    width == 0. || (0.000001..=256.).contains(&width)
}

fn style_valid(style: &StreamLayerStyle) -> Result<()> {
    ensure(
        (0. ..=1.).contains(&style.fill_opacity) && border_width(style.border_width),
        "invalid stream layer opacity or border width",
    )?;
    style.border_color.validate()?;
    if let Some(paint) = &style.paint {
        paint.validate()?;
    }
    Ok(())
}

impl StreamLayoutOptions {
    fn validate(&self) -> Result<()> {
        self.curve.validate()?;
        ensure(
            matches!(
                self.curve,
                Curve::Linear
                    | Curve::Basis
                    | Curve::Cardinal
                    | Curve::CardinalTension { .. }
                    | Curve::CatmullRom { .. }
                    | Curve::Monotone
                    | Curve::MonotoneX
                    | Curve::MonotoneY
                    | Curve::Natural
                    | Curve::Step
                    | Curve::StepBefore
                    | Curve::StepAfter
            ),
            "stream requires an area curve",
        )?;
        self.x_scale.validate()?;
        self.y_scale.validate()?;
        ensure(
            self.y_scale.kind != ScaleKind::Utc,
            "stream stack boundaries require a numeric value scale",
        )?;
        self.axes.validate()?;
        self.value_format.validate()?;
        ensure(
            self.styles.len() <= 256
                && self.labels.len() <= 256
                && self.hidden.len() <= 256
                && self.y_bounds.iter().flatten().all(|v| v.is_finite()),
            "invalid stream options or label budgets",
        )?;
        style_valid(&self.style)?;
        for style in self.styles.values() {
            style_valid(style)?;
        }
        for label in self.labels.values() {
            ensure(
                label.len() <= MAX_LABEL_BYTES,
                "stream layer label exceeds text budget",
            )?;
        }
        if let Some(dots) = &self.dots {
            ensure(
                (0. ..=4096.).contains(&dots.size) && border_width(dots.border_width),
                "invalid stream dot size or border",
            )?;
            dots.color.validate()?;
            dots.border_color.validate()?;
            if let Some(glyph) = &dots.glyph {
                let geometry = glyph_geometry([0., 0.], glyph)?;
                ensure(
                    geometry.paint_mode != GlyphPaintMode::Stroke || dots.border_width > 0.,
                    "stroke-only Stream glyph requires a dot border",
                )?;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamSample {
    pub mark_id: String,
    pub source_index: usize,
    pub category: CategoryKey,
    pub value: Option<f64>,
    pub formatted_value: Option<String>,
    /// Raw stack boundaries, before scale nicening, rounding or clamping.
    pub y0: f64,
    pub y1: f64,
    pub screen_y: [f64; 2],
    /// Dot position, or the band midpoint when dots are disabled.
    pub position: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamLayer {
    pub id: String,
    pub label: String,
    pub original_index: usize,
    pub stack_index: usize,
    pub area_mark_id: Option<String>,
    pub color: Color,
    pub hidden: bool,
    pub samples: Vec<StreamSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StreamSlice {
    pub source_index: usize,
    pub category: CategoryKey,
    pub x: f64,
    /// Input-layer indices sorted by the displayed end boundary, top first.
    pub layer_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
pub struct StreamLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub stack: StackResult,
    pub x_scale: ResolvedLinePointScale,
    pub y_scale: ResolvedContinuousScale,
    pub colors: OrdinalColorScale,
    pub layers: Vec<StreamLayer>,
    pub slices: Vec<StreamSlice>,
    interaction: InteractionBinding,
    slice_interaction: InteractionBinding,
}

impl StreamLayout {
    /// Filled layers and enabled dot ink participate in exact picking.
    /// Whole-layer values use the final source column; samples retain all data.
    pub fn interaction_binding(&self) -> InteractionBinding {
        self.interaction.clone()
    }

    /// One logical site per visible series and source category. A null source
    /// retains `None`. Disabled dots use zero-radius, unpainted sample geometry.
    pub fn slice_interaction_binding(&self) -> InteractionBinding {
        self.slice_interaction.clone()
    }

    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        self.into_interaction(false)
    }

    pub fn into_slice_interaction(self) -> Result<SceneInteraction> {
        self.into_interaction(true)
    }

    fn into_interaction(self, slices: bool) -> Result<SceneInteraction> {
        let candidate = SceneInteraction {
            scene: self.scene,
            plot: Some(self.plot),
            interaction: Some(if slices {
                self.slice_interaction
            } else {
                self.interaction
            }),
        };
        candidate.validate()?;
        Ok(candidate)
    }
}

fn keys(categories: &[CategoryKey]) -> Result<Vec<String>> {
    let mut seen = BTreeSet::new();
    categories
        .iter()
        .map(|key| {
            let id = key.identity()?;
            ensure(
                seen.insert(id.clone()),
                "duplicate stream category identity",
            )?;
            Ok(id)
        })
        .collect()
}

fn boundary_path(points: &[P], curve: Curve) -> Vec<C> {
    let commands = path(points, curve);
    if curve != Curve::Step {
        return commands;
    }
    // The shared line path retains the source vertices between step segments.
    // D3's area boundary omits those collinear intermediate vertices.
    let last = commands.len().saturating_sub(1);
    commands
        .into_iter()
        .enumerate()
        .filter(|(index, _)| *index == 0 || *index == last || index % 3 != 0)
        .map(|(_, command)| command)
        .collect()
}

fn area_geometry(samples: &[StreamSample], curve: Curve) -> Option<(Geometry, usize, usize)> {
    let upper = samples
        .iter()
        .map(|p| P::new(p.position[0], p.screen_y[1]))
        .collect::<Vec<_>>();
    let lower = samples
        .iter()
        .rev()
        .map(|p| P::new(p.position[0], p.screen_y[0]))
        .collect::<Vec<_>>();
    let mut high = boundary_path(&upper, curve);
    let reverse_curve = match curve {
        Curve::StepBefore => Curve::StepAfter,
        Curve::StepAfter => Curve::StepBefore,
        curve => curve,
    };
    let mut low = boundary_path(&lower, reverse_curve);
    if high.is_empty() || low.is_empty() {
        return None;
    }
    if matches!(high.last(), Some(C::Close)) {
        high.pop();
    }
    if matches!(low.last(), Some(C::Close)) {
        low.pop();
    }
    let end = high.len() - 1;
    let reverse_start = high.len();
    if let Some(C::Move(point)) = low.first() {
        low[0] = C::Line(*point);
    }
    high.extend(low);
    high.push(C::Close);
    Some((Geometry::Path(high), end, reverse_start))
}

fn endpoint(command: &C) -> Option<[f64; 2]> {
    match command {
        C::Move(p) | C::Line(p) | C::Cubic { end: p, .. } => Some([p.x, p.y]),
        C::Close => None,
    }
}

pub fn stream_layout(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
) -> Result<StreamLayout> {
    layout(c, categories, series, o, None)
}

/// Draw each enabled, visible and non-null dot from its original stack sample.
/// Offset paths preserve the sample's logical position for nearest/slice input.
pub fn stream_layout_with_dot_factory(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
    mut factory: impl FnMut(&StreamDotContext<'_>) -> Result<StreamDotGeometry>,
) -> Result<StreamLayout> {
    layout(c, categories, series, o, Some(&mut factory))
}

fn layout(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
    mut factory: Option<&mut DotFactory<'_>>,
) -> Result<StreamLayout> {
    c.validate()?;
    o.validate()?;
    ensure(
        categories.len() <= 4096
            && series
                .len()
                .checked_mul(categories.len())
                .is_some_and(|n| n <= MAX_STREAM_CELLS),
        "stream source cell budget exceeded",
    )?;
    let category_keys = keys(categories)?;
    ensure(
        series.iter().all(|s| s.values.len() == categories.len()),
        "stream category and series lengths must align",
    )?;
    let known = series
        .iter()
        .map(|s| s.id.as_str())
        .collect::<BTreeSet<_>>();
    ensure(
        o.styles
            .keys()
            .chain(o.labels.keys())
            .chain(o.hidden.iter())
            .all(|key| known.contains(key.as_str())),
        "stream options refer to an unknown series",
    )?;
    let stack = stack(series, &o.stack)?;
    let mut bounds = stack.domain;
    for (bound, override_value) in bounds.iter_mut().zip(o.y_bounds) {
        if let Some(value) = override_value {
            *bound = value;
        }
    }
    let scales = point_line_layout(
        c,
        &[],
        &PointLineOptions {
            line: LineOptions {
                y_domain: Some(bounds),
                axes: o.axes.clone(),
                ..Default::default()
            },
            point: o.x_scale,
            y_scale: o.y_scale,
            domain: Some(categories.to_vec()),
            tick_values: o.x_ticks.clone(),
            window: None,
        },
        &LineStyleOptions::default(),
    )?;
    let colors = o
        .colors
        .resolve(series.iter().map(|s| CategoryKey::String(s.id.clone())))?;
    let mut result = StreamLayout {
        scene: scales.scene,
        plot: scales.plot,
        x_scale: scales.x_scale,
        y_scale: scales.y_scale,
        stack,
        colors,
        layers: vec![],
        slices: vec![],
        interaction: Default::default(),
        slice_interaction: Default::default(),
    };
    result.scene.draw_order.clear();
    let mut path_commands = result
        .scene
        .marks
        .iter()
        .map(|mark| match &mark.geometry {
            Geometry::Path(path) => path.len(),
            _ => 0,
        })
        .sum::<usize>();
    let mut dots = Vec::new();
    let mut sample_sites = BTreeMap::new();
    for layer in &result.stack.layers {
        let style = o.styles.get(&layer.id).unwrap_or(&o.style);
        let color = style
            .color
            .or(result
                .colors
                .map_or_insert(CategoryKey::String(layer.id.clone()))?)
            .ok_or_else(|| ChartError("missing stream layer color".into()))?;
        let label = o
            .labels
            .get(&layer.id)
            .filter(|label| !label.is_empty())
            .cloned()
            .unwrap_or_else(|| layer.id.clone());
        let hidden = o.hidden.contains(&layer.id);
        let mut computed = StreamLayer {
            id: layer.id.clone(),
            label,
            original_index: layer.original_index,
            stack_index: layer.stack_index,
            area_mark_id: None,
            color,
            hidden,
            samples: Vec::with_capacity(categories.len()),
        };
        for point in &layer.points {
            let category = &categories[point.column];
            let mark_id = format!(
                "stream:sample:{}:{}:{}:{}",
                layer.id.len(),
                layer.id,
                category_keys[point.column].len(),
                category_keys[point.column]
            );
            ensure(
                mark_id.len() <= MAX_LABEL_BYTES,
                "stream series/category identity exceeds mark budget",
            )?;
            let x = result.x_scale.map(category).unwrap();
            let y = [point.y0, point.y1].map(|v| result.y_scale.map(v));
            ensure(
                y.into_iter().all(|v| v.is_finite() && v.abs() <= 900_000.),
                "stream boundary projection overflow",
            )?;
            let position = [
                x,
                match o
                    .dots
                    .as_ref()
                    .map_or(StreamDotPosition::Center, |dots| dots.position)
                {
                    StreamDotPosition::Start => y[0],
                    StreamDotPosition::Center => y[0] * 0.5 + y[1] * 0.5,
                    StreamDotPosition::End => y[1],
                },
            ];
            let sample = StreamSample {
                mark_id,
                source_index: point.column,
                category: category.clone(),
                value: point.value,
                formatted_value: point.value.map(|v| o.value_format.format(v)).transpose()?,
                y0: point.y0,
                y1: point.y1,
                screen_y: y,
                position,
            };
            if !hidden {
                let dot = sample_mark(
                    c,
                    &computed,
                    &sample,
                    o,
                    &series[layer.original_index],
                    factory.as_deref_mut(),
                    200_000usize.saturating_sub(path_commands),
                )?;
                if let Geometry::Path(path) = &dot.mark.geometry {
                    path_commands += path.len();
                }
                ensure(
                    path_commands <= 200_000,
                    "stream path command budget exceeded",
                )?;
                sample_sites.insert(dot.site.mark_id.clone(), dot.site.clone());
                if let Some(anchor) = dot.anchor {
                    result
                        .slice_interaction
                        .anchors
                        .insert(dot.mark.id.clone(), anchor.clone());
                    if dot.ink {
                        result
                            .interaction
                            .anchors
                            .insert(dot.mark.id.clone(), anchor);
                    }
                }
                if dot.ink {
                    result.interaction.sites.push(dot.site);
                }
                dots.push(dot.mark);
            }
            computed.samples.push(sample);
        }
        if !hidden && let Some((geometry, a, b)) = area_geometry(&computed.samples, o.curve) {
            let mut mark = Mark::new(
                format!("stream:area:{}:{}", layer.id.len(), layer.id),
                geometry,
                color,
            );
            mark.series = layer.id.clone();
            mark.label = computed.label.clone();
            let value = computed.samples.last().and_then(|sample| sample.value);
            mark.value = value.unwrap_or(0.);
            mark.fill = Some(crate::area_bump::alpha(color, style.fill_opacity));
            mark.fill_paint = style
                .paint
                .as_ref()
                .map(|paint| crate::area_bump::paint_alpha(paint, style.fill_opacity));
            mark.stroke = if style.border_width > 0. {
                Some(GraphicStroke {
                    color: style.border_color.resolve(color_context(c, color))?,
                    width: style.border_width,
                    cap: StrokeCap::Butt,
                })
            } else {
                None
            };
            mark.clip = Some(result.plot);
            let commands = mark.geometry.commands();
            path_commands += commands.len();
            ensure(
                path_commands <= 200_000,
                "stream path command budget exceeded",
            )?;
            let p = endpoint(&commands[a]).unwrap();
            let q = endpoint(&commands[b]).unwrap();
            let position = [p[0] * 0.5 + q[0] * 0.5, p[1] * 0.5 + q[1] * 0.5];
            result.interaction.anchors.insert(
                mark.id.clone(),
                InteractionAnchor::path_points(&mark.geometry, position, &[a, b])?,
            );
            result.interaction.sites.push(InteractionSite {
                mark_id: mark.id.clone(),
                series: layer.id.clone(),
                label: computed.label.clone(),
                value,
                formatted_value: value.map(|v| o.value_format.format(v)).transpose()?,
                color,
                position,
            });
            computed.area_mark_id = Some(mark.id.clone());
            result.scene.marks.push(mark);
        }
        if o.legend {
            result.scene.legend.push(LegendItem {
                id: layer.id.clone(),
                label: computed.label.clone(),
                color,
            });
        }
        result.layers.push(computed);
    }
    result.scene.marks.extend(dots);
    for (source_index, category) in categories.iter().enumerate() {
        let mut layer_indices = (0..result.layers.len())
            .filter(|i| !result.layers[*i].hidden)
            .collect::<Vec<_>>();
        layer_indices.sort_by(|a, b| {
            result.layers[*a].samples[source_index].screen_y[1]
                .total_cmp(&result.layers[*b].samples[source_index].screen_y[1])
                .then_with(|| a.cmp(b))
        });
        // SurfaceModel presents Cartesian slices in reverse binding order.
        // Supply each column bottom first so its rows match StreamSlice.
        for layer in layer_indices.iter().rev() {
            let id = &result.layers[*layer].samples[source_index].mark_id;
            result
                .slice_interaction
                .sites
                .push(sample_sites.remove(id).unwrap());
        }
        result.slices.push(StreamSlice {
            source_index,
            category: category.clone(),
            x: result.x_scale.map(category).unwrap(),
            layer_indices,
        });
    }
    ensure(
        result.scene.marks.len() <= MAX_MARKS,
        "stream rendered mark budget exceeded",
    )?;
    result.interaction.validate(&result.scene)?;
    result.slice_interaction.validate(&result.scene)?;
    Ok(result)
}

fn color_context(c: &ChartConfig, color: Color) -> InheritedColorContext {
    InheritedColorContext {
        inherited: color,
        theme: c.theme.text,
        surface: color,
        backdrop: c.theme.background,
    }
}

struct PreparedDot {
    mark: Mark,
    site: InteractionSite,
    anchor: Option<InteractionAnchor>,
    ink: bool,
}

fn sample_mark(
    c: &ChartConfig,
    layer: &StreamLayer,
    sample: &StreamSample,
    o: &StreamLayoutOptions,
    series: &BarSeries,
    factory: Option<&mut DotFactory<'_>>,
    remaining_commands: usize,
) -> Result<PreparedDot> {
    let [x, y] = sample.position;
    let visible_dot = o.dots.as_ref().filter(|_| sample.value.is_some());
    let inherited = color_context(c, layer.color);
    let color = visible_dot.map_or(Ok(layer.color), |dot| dot.color.resolve(inherited))?;
    let stroke = visible_dot
        .filter(|dot| dot.border_width > 0.)
        .map(|dot| {
            Ok(GraphicStroke {
                color: dot.border_color.resolve(inherited)?,
                width: dot.border_width,
                cap: StrokeCap::Butt,
            })
        })
        .transpose()?;
    let mut mark = Mark::new(
        &sample.mark_id,
        Geometry::Circle {
            x,
            y,
            radius: visible_dot.map_or(0., |dot| dot.size / 2.),
        },
        color,
    );
    mark.stroke = stroke;
    let mut anchor = None;
    let mut point_color = layer.color;
    let mut ink = visible_dot.is_some_and(|dot| dot.size > 0. || dot.border_width > 0.);
    if let Some(style) = visible_dot {
        let fill = (!style
            .glyph
            .as_ref()
            .is_some_and(|g| matches!(g.shape, GlyphShape::Stroked(_))))
        .then_some(color);
        if let Some(factory) = factory {
            let prepared = factory(&StreamDotContext {
                series,
                series_index: layer.original_index,
                layer_label: &layer.label,
                sample,
                position: sample.position,
                plot: ClipRect::plot(c),
                color: layer.color,
                style,
                fill,
                stroke,
            })?
            .prepare_at(
                &sample.mark_id,
                sample.position,
                layer.color,
                remaining_commands,
            )?;
            mark = prepared.mark;
            anchor = prepared.anchor;
            point_color = prepared.color;
            ink = mark.opacity > 0.
                && (mark.stroke.is_some()
                    || mark.has_fill()
                        && prepared.bounds.width > 0.
                        && prepared.bounds.height > 0.);
        } else if let Some(glyph) = &style.glyph {
            let geometry = glyph_geometry(sample.position, glyph)?;
            mark = glyph_mark(&sample.mark_id, &geometry, fill, stroke)?;
            anchor = matches!(mark.geometry, Geometry::Path(_))
                .then(|| {
                    InteractionAnchor::path_points(
                        &mark.geometry,
                        sample.position,
                        &geometry.anchor_indices,
                    )
                })
                .transpose()?;
            point_color = fill
                .or(stroke.map(|stroke| stroke.color))
                .unwrap_or(layer.color);
            ink = glyph.size > 0. || stroke.is_some();
        }
    }
    mark.series = layer.id.clone();
    mark.label = format!(
        "{} · {}",
        layer.label,
        crate::line_scales::category_text(&sample.category, &o.axes.x.format)?
    );
    mark.value = sample.value.unwrap_or(0.);
    mark.clip = Some(ClipRect::plot(c));
    let site = InteractionSite {
        mark_id: mark.id.clone(),
        series: layer.id.clone(),
        label: mark.label.clone(),
        value: sample.value,
        formatted_value: sample.formatted_value.clone(),
        color: point_color,
        position: sample.position,
    };
    Ok(PreparedDot {
        mark,
        site,
        anchor,
        ink,
    })
}
