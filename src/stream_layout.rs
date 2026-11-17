//! Stream layers, raw stacks, point scales and separate layer and slice bindings.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

mod factories;
mod widgets;
pub use factories::*;
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
pub use widgets::*;

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
    fn validate(&self, dynamic_dot_border: bool) -> Result<()> {
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
                    geometry.paint_mode != GlyphPaintMode::Stroke
                        || dots.border_width > 0.
                        || dynamic_dot_border,
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
    widget_source: Option<Arc<widgets::StreamWidgetSource>>,
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
    layout(
        c,
        categories,
        series,
        o,
        &mut StreamFactories::default(),
        None,
    )
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
    layout(
        c,
        categories,
        series,
        o,
        &mut StreamFactories::default(),
        Some(&mut factory),
    )
}

fn layout(
    c: &ChartConfig,
    categories: &[CategoryKey],
    series: &[BarSeries],
    o: &StreamLayoutOptions,
    factories: &mut StreamFactories<'_>,
    mut factory: Option<&mut DotFactory<'_>>,
) -> Result<StreamLayout> {
    c.validate()?;
    o.validate(factories.dot_border_width.is_some())?;
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
        widget_source: None,
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
    ensure(
        factories.dot_size.is_none() || o.dots.as_ref().is_none_or(|dots| dots.glyph.is_none()),
        "Stream dot size function cannot be combined with a configured glyph: diameter and glyph size have different units",
    )?;
    let category_labels = categories
        .iter()
        .map(|category| crate::line_scales::category_text(category, &o.axes.x.format))
        .collect::<Result<Vec<_>>>()?;
    struct PreparedLayer {
        layer: StreamLayer,
        area: Option<(Geometry, usize, usize)>,
    }
    let mut prepared_layers = Vec::with_capacity(result.stack.layers.len());
    let mut original_commands = path_commands;
    let mut original_stops = result
        .scene
        .marks
        .iter()
        .map(|mark| {
            mark.fill_paint
                .as_ref()
                .map_or(0, |paint| paint.stops().len())
        })
        .sum::<usize>();
    // Validate every original ID, projection and path before user callbacks.
    for layer in &result.stack.layers {
        let style = o.styles.get(&layer.id).unwrap_or(&o.style);
        let color = if factories.color.is_some() {
            // This internal placeholder is replaced before exposing a computed
            // layer to border/dot callbacks or emitting any paint. An explicit
            // function need not have a static ordinal fallback, and must not
            // extend that separate, caller-declared ordinal mapping.
            Color(0, 0, 0, 0)
        } else {
            style
                .color
                .or(result
                    .colors
                    .map_or_insert(CategoryKey::String(layer.id.clone()))?)
                .ok_or_else(|| ChartError("missing stream layer color".into()))?
        };
        let label = o
            .labels
            .get(&layer.id)
            .filter(|label| !label.is_empty())
            .cloned()
            .unwrap_or_else(|| layer.id.clone());
        let hidden = o.hidden.contains(&layer.id);
        let mut samples = Vec::with_capacity(categories.len());
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
                x.is_finite()
                    && x.abs() <= 900_000.
                    && y.into_iter().all(|v| v.is_finite() && v.abs() <= 900_000.),
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
            if let Some(dots) = o.dots.as_ref().filter(|_| !hidden && point.value.is_some())
                && let Some(glyph) = &dots.glyph
            {
                let glyph = glyph_geometry(position, glyph)?;
                if factory.is_none()
                    && let Geometry::Path(commands) = &glyph.geometry
                {
                    original_commands = original_commands
                        .checked_add(commands.len())
                        .ok_or_else(|| ChartError("Stream original path budget overflow".into()))?;
                    ensure(
                        original_commands <= 200_000,
                        "stream path command budget exceeded",
                    )?;
                }
            }
            ensure(
                hidden
                    || factories.label.is_some()
                    || label.len() + " · ".len() + category_labels[point.column].len()
                        <= MAX_LABEL_BYTES,
                "Stream sample label exceeds text budget",
            )?;
            samples.push(StreamSample {
                mark_id,
                source_index: point.column,
                category: category.clone(),
                value: point.value,
                formatted_value: None,
                y0: point.y0,
                y1: point.y1,
                screen_y: y,
                position,
            });
        }
        let area = if !hidden || factories.label.is_some() || factories.color.is_some() {
            area_geometry(&samples, o.curve)
        } else {
            // Legacy hidden layers never rendered a path. Only the new
            // label/color geometry contexts request an otherwise unused path;
            // formatter/dot/border functions do not change hidden eligibility.
            None
        };
        if !hidden && area.is_some() {
            original_stops = original_stops
                .checked_add(style.paint.as_ref().map_or(0, |paint| paint.stops().len()))
                .ok_or_else(|| ChartError("Stream original paint budget overflow".into()))?;
            ensure(
                original_stops <= crate::MAX_SCENE_GRADIENT_STOPS,
                "stream paint stop budget exceeded",
            )?;
        }
        if let Some((Geometry::Path(commands), ..)) = &area {
            // Finite path controls and original path budgets are independent of
            // function results. Hidden geometry is still supplied to label/color.
            for command in commands {
                let valid = match command {
                    C::Move(p) | C::Line(p) => [p.x, p.y]
                        .into_iter()
                        .all(|v| v.is_finite() && v.abs() <= 1e6),
                    C::Cubic { first, second, end } => [first, second, end].into_iter().all(|p| {
                        [p.x, p.y]
                            .into_iter()
                            .all(|v| v.is_finite() && v.abs() <= 1e6)
                    }),
                    C::Close => true,
                };
                ensure(valid, "Stream area control projection overflow")?;
            }
            if !hidden {
                original_commands = original_commands
                    .checked_add(commands.len())
                    .ok_or_else(|| ChartError("Stream original path budget overflow".into()))?;
                ensure(
                    original_commands <= 200_000,
                    "stream path command budget exceeded",
                )?;
            }
        }
        prepared_layers.push(PreparedLayer {
            layer: StreamLayer {
                id: layer.id.clone(),
                label,
                original_index: layer.original_index,
                stack_index: layer.stack_index,
                area_mark_id: (!hidden && area.is_some())
                    .then(|| format!("stream:area:{}:{}", layer.id.len(), layer.id)),
                color,
                hidden,
                samples,
            },
            area,
        });
    }
    let estimated_marks = result.scene.marks.len()
        + prepared_layers
            .iter()
            .filter(|p| p.layer.area_mark_id.is_some())
            .count()
        + prepared_layers
            .iter()
            .filter(|p| !p.layer.hidden)
            .map(|p| p.layer.samples.len())
            .sum::<usize>();
    ensure(
        estimated_marks <= MAX_MARKS,
        "stream rendered mark budget exceeded",
    )?;
    let mut dots = Vec::new();
    let mut sample_sites = BTreeMap::new();
    for mut prepared in prepared_layers {
        let computed = &mut prepared.layer;
        let series = &series[computed.original_index];
        let style = o.styles.get(&computed.id).unwrap_or(&o.style);
        for sample in &mut computed.samples {
            sample.formatted_value = sample
                .value
                .map(|value| {
                    let text = match &mut factories.formatted_value {
                        Some(format) => format(value)?,
                        None => o.value_format.format(value)?,
                    };
                    ensure(
                        text.len() <= MAX_LABEL_BYTES,
                        "Stream formatted value exceeds text budget",
                    )?;
                    Ok(text)
                })
                .transpose()?;
        }
        let path_context = StreamLayerPathContext {
            series,
            series_index: computed.original_index,
            id: &computed.id,
            path: prepared.area.as_ref().map(|(geometry, ..)| geometry),
            plot: result.plot,
        };
        if let Some(label) = &mut factories.label {
            computed.label = label(&path_context)?;
            ensure(
                computed.label.len() <= MAX_LABEL_BYTES,
                "Stream layer label exceeds text budget",
            )?;
        }
        if !computed.hidden {
            for text in &category_labels {
                ensure(
                    computed.label.len() + " · ".len() + text.len() <= MAX_LABEL_BYTES,
                    "Stream sample label exceeds text budget",
                )?;
            }
        }
        if let Some(color) = &mut factories.color {
            computed.color = color(&path_context)?;
        }
        let color = computed.color;
        // Resolve full-layer border before dots, even at width0 when explicitly
        // requested. Unused static inherited colors retain their old admission.
        let resolved_border = if computed.area_mark_id.is_some() {
            match &mut factories.border_color {
                Some(border) => Some(border(&StreamLayerStyleContext {
                    series,
                    series_index: computed.original_index,
                    layer: computed,
                    path: prepared.area.as_ref().map(|(g, ..)| g),
                    plot: result.plot,
                })?),
                None if style.border_width > 0. => {
                    Some(style.border_color.resolve(color_context(c, color))?)
                }
                None => None,
            }
        } else {
            None
        };
        if !computed.hidden {
            for sample in &computed.samples {
                let dot = sample_mark(
                    DotEnvironment {
                        c,
                        o,
                        series,
                        remaining_commands: 200_000usize.saturating_sub(path_commands),
                    },
                    computed,
                    sample,
                    factories,
                    factory.as_deref_mut(),
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
        }
        if let Some(id) = &computed.area_mark_id {
            let (geometry, a, b) = prepared.area.take().unwrap();
            let mut mark = Mark::new(id, geometry, color);
            mark.series = computed.id.clone();
            mark.label = computed.label.clone();
            let last = computed.samples.last();
            let value = last.and_then(|sample| sample.value);
            mark.value = value.unwrap_or(0.);
            mark.fill = Some(crate::area_bump::alpha(color, style.fill_opacity));
            mark.fill_paint = style
                .paint
                .as_ref()
                .map(|paint| crate::area_bump::paint_alpha(paint, style.fill_opacity));
            mark.stroke = if style.border_width > 0. {
                Some(GraphicStroke {
                    color: resolved_border.unwrap(),
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
            if !mark.label.is_empty() {
                let p = endpoint(&commands[a]).unwrap();
                let q = endpoint(&commands[b]).unwrap();
                let position = [p[0] * 0.5 + q[0] * 0.5, p[1] * 0.5 + q[1] * 0.5];
                result.interaction.anchors.insert(
                    mark.id.clone(),
                    InteractionAnchor::path_points(&mark.geometry, position, &[a, b])?,
                );
                result.interaction.sites.push(InteractionSite {
                    mark_id: mark.id.clone(),
                    series: computed.id.clone(),
                    label: computed.label.clone(),
                    value,
                    formatted_value: last.and_then(|sample| sample.formatted_value.clone()),
                    color,
                    position,
                });
            }
            result.scene.marks.push(mark);
        }
        if o.legend {
            result.scene.legend.push(LegendItem {
                id: computed.id.clone(),
                label: computed.label.clone(),
                color,
            });
        }
        result.layers.push(prepared.layer);
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
    result.widget_source = Some(Arc::new(widgets::StreamWidgetSource::capture(
        &result,
        series,
        o.dots.is_some(),
    )));
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

struct DotEnvironment<'a> {
    c: &'a ChartConfig,
    o: &'a StreamLayoutOptions,
    series: &'a BarSeries,
    remaining_commands: usize,
}
fn sample_mark(
    environment: DotEnvironment<'_>,
    layer: &StreamLayer,
    sample: &StreamSample,
    factories: &mut StreamFactories<'_>,
    factory: Option<&mut DotFactory<'_>>,
) -> Result<PreparedDot> {
    let DotEnvironment {
        c,
        o,
        series,
        remaining_commands,
    } = environment;
    let [x, y] = sample.position;
    let visible_dot = o.dots.as_ref().filter(|_| sample.value.is_some());
    let inherited = color_context(c, layer.color);
    let mut resolved_size = visible_dot.map_or(0., |dot| dot.size);
    let mut resolved_width = visible_dot.map_or(0., |dot| dot.border_width);
    let mut explicit_dot_color = false;
    let (color, stroke) = if let Some(style) = visible_dot {
        let context = StreamDotStyleContext {
            series,
            series_index: layer.original_index,
            layer,
            sample,
            position: sample.position,
            plot: ClipRect::plot(c),
            style,
        };
        if let Some(size) = &mut factories.dot_size {
            resolved_size = size(&context)?;
        }
        ensure(
            (0. ..=4096.).contains(&resolved_size),
            "invalid Stream function dot size",
        )?;
        let color = match &mut factories.dot_color {
            Some(color) => {
                explicit_dot_color = true;
                color(&context)?
            }
            None => style.color.resolve(inherited)?,
        };
        if let Some(width) = &mut factories.dot_border_width {
            resolved_width = width(&context)?;
        }
        ensure(
            border_width(resolved_width),
            "invalid Stream function dot border width",
        )?;
        let border_color = match &mut factories.dot_border_color {
            Some(color) => Some(color(&context)?),
            None if resolved_width > 0. => Some(style.border_color.resolve(inherited)?),
            None => None,
        };
        ensure(
            !style
                .glyph
                .as_ref()
                .is_some_and(|g| matches!(g.shape, GlyphShape::Stroked(_)))
                || resolved_width > 0.,
            "stroke-only Stream glyph requires a dot border",
        )?;
        let stroke = (resolved_width > 0.).then(|| GraphicStroke {
            color: border_color.unwrap(),
            width: resolved_width,
            cap: StrokeCap::Butt,
        });
        (color, stroke)
    } else {
        (layer.color, None)
    };
    let mut mark = Mark::new(
        &sample.mark_id,
        Geometry::Circle {
            x,
            y,
            radius: resolved_size / 2.,
        },
        color,
    );
    mark.stroke = stroke;
    let mut anchor = None;
    let mut point_color = if explicit_dot_color {
        color
    } else {
        layer.color
    };
    let mut ink = visible_dot.is_some() && (resolved_size > 0. || resolved_width > 0.);
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
