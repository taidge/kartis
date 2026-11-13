//! Typed funnel stages using Nivo's band and separator geometry and the shared D3 curves.
use std::collections::BTreeSet;

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};

use crate::cartesian::path;
use crate::scene::{MAX_LABEL_BYTES, ensure};
use crate::{
    CategoryKey, ChartConfig, ChartError, ClipRect, Color, Curve, FillPaint, Geometry,
    InheritedColor, InheritedColorContext, InteractionAnchor, InteractionBinding, InteractionSite,
    Label, LegendItem, Mark, OrdinalColorOptions, OrdinalColorScale, Result, Scene,
    SceneInteraction, TextAnchor, ValueFormat,
};

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelDatum {
    pub id: CategoryKey,
    pub label: String,
    pub value: f64,
    /// Width across the flow, in chart pixels, for `FunnelSize::DatumPixels`.
    pub size: Option<f64>,
    pub color: Option<Color>,
    pub paint: Option<FillPaint>,
}

impl FunnelDatum {
    pub fn new(id: impl Into<CategoryKey>, value: f64) -> Self {
        let id = id.into();
        Self {
            label: key_label(&id),
            id,
            value,
            size: None,
            color: None,
            paint: None,
        }
    }
}

fn key_label(key: &CategoryKey) -> String {
    match key {
        CategoryKey::String(s) => s.clone(),
        CategoryKey::Number(n) => (if *n == 0. { 0. } else { *n }).to_string(),
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FunnelDirection {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FunnelInterpolation {
    #[default]
    Smooth,
    Linear,
}

impl From<FunnelInterpolation> for Curve {
    fn from(value: FunnelInterpolation) -> Self {
        match value {
            FunnelInterpolation::Smooth => Self::Basis,
            FunnelInterpolation::Linear => Self::Linear,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum FunnelSize {
    #[default]
    Value,
    /// Missing datum sizes have zero width.
    DatumPixels,
    /// Cycle the specified sizes in source order.
    OrdinalPixels(Vec<f64>),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FunnelSeparatorOptions {
    /// Reserve a band of this width outside the largest stage.
    pub length: f64,
    /// Gap between the end of a separator and the stage's starting edge.
    pub offset: f64,
    /// Defaults to a 1 px line in the chart theme's grid color.
    pub stroke: Option<GraphicStroke>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FunnelLabelText {
    #[default]
    Value,
    Datum,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FunnelLabelOptions {
    pub text: FunnelLabelText,
    /// The theme color for this role is the chart background.
    pub color: InheritedColor,
    pub size: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelLayoutOptions {
    pub direction: FunnelDirection,
    pub interpolation: FunnelInterpolation,
    pub spacing: f64,
    /// Fraction of each band reserved for the two constant-width sections.
    pub shape_blending: f64,
    pub size: FunnelSize,
    pub colors: OrdinalColorOptions,
    pub fill_opacity: f64,
    /// Borders run along the two sides of a stage and leave its ends open.
    pub border_width: f64,
    pub border_color: InheritedColor,
    pub border_opacity: f64,
    pub labels: Option<FunnelLabelOptions>,
    pub before: Option<FunnelSeparatorOptions>,
    pub after: Option<FunnelSeparatorOptions>,
    pub value_format: ValueFormat,
    /// Controlled current stage. The expansion applies to both side profiles.
    pub current: Option<CategoryKey>,
    pub current_size_extension: f64,
    pub current_border_width: Option<f64>,
    pub legend: bool,
}

impl Default for FunnelLayoutOptions {
    fn default() -> Self {
        Self {
            direction: Default::default(),
            interpolation: Default::default(),
            spacing: 0.,
            shape_blending: 0.66,
            size: Default::default(),
            colors: Default::default(),
            fill_opacity: 1.,
            border_width: 6.,
            border_color: InheritedColor::FromContext(vec![]),
            border_opacity: 0.66,
            labels: Some(Default::default()),
            before: Some(Default::default()),
            after: Some(Default::default()),
            value_format: Default::default(),
            current: None,
            current_size_extension: 0.,
            current_border_width: None,
            legend: true,
        }
    }
}

fn finite_size(value: f64) -> bool {
    (0. ..=16384.).contains(&value)
}

fn valid_border_width(width: f64) -> bool {
    width == 0. || (0.000001..=10000.).contains(&width)
}

impl FunnelLayoutOptions {
    fn validate(&self) -> Result<()> {
        ensure(
            finite_size(self.spacing)
                && finite_size(self.current_size_extension)
                && (0. ..=1.).contains(&self.shape_blending)
                && (0. ..=1.).contains(&self.fill_opacity)
                && (0. ..=1.).contains(&self.border_opacity)
                && valid_border_width(self.border_width)
                && self.current_border_width.is_none_or(valid_border_width),
            "invalid funnel spacing, blending, extension or opacity",
        )?;
        for separator in [self.before, self.after].into_iter().flatten() {
            ensure(
                finite_size(separator.length)
                    && finite_size(separator.offset)
                    && separator
                        .stroke
                        .is_none_or(|stroke| stroke.width > 0. && valid_border_width(stroke.width)),
                "invalid funnel separator length, offset or stroke",
            )?;
        }
        if let FunnelSize::OrdinalPixels(sizes) = &self.size {
            ensure(
                (1..=256).contains(&sizes.len()) && sizes.iter().copied().all(finite_size),
                "funnel ordinal size needs 1 to 256 finite widths",
            )?;
        }
        if let Some(labels) = &self.labels {
            ensure(
                labels.size.is_none_or(|size| (1. ..=256.).contains(&size)),
                "invalid funnel label size",
            )?;
            labels.color.validate()?;
        }
        if let Some(current) = &self.current {
            current.validate()?;
        }
        self.value_format.validate()?;
        self.border_color.validate()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunnelSeparatorSide {
    Before,
    After,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelSeparator {
    /// `None` is the closing separator at the last stage's end.
    pub part_id: Option<CategoryKey>,
    pub side: FunnelSeparatorSide,
    pub from: [f64; 2],
    pub to: [f64; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FunnelProfilePoint {
    pub before: [f64; 2],
    pub after: [f64; 2],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelPart {
    pub datum: FunnelDatum,
    pub source_index: usize,
    pub mark_id: String,
    pub series: String,
    /// Bounds of the start width over the entire stage band, before extension.
    pub bounds: ClipRect,
    pub center: [f64; 2],
    pub color: Color,
    pub formatted_value: String,
    pub is_current: bool,
    /// The four input samples for the two linear or cubic basis curves.
    pub profile: [FunnelProfilePoint; 4],
}

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelLayout {
    pub scene: Scene,
    pub parts: Vec<FunnelPart>,
    pub separators: Vec<FunnelSeparator>,
    pub value_domain: [f64; 2],
    pub size_range: [f64; 2],
    pub bandwidth: f64,
    pub colors: OrdinalColorScale,
    plot: ClipRect,
    interaction: InteractionBinding,
}

impl FunnelLayout {
    pub fn plot(&self) -> ClipRect {
        self.plot
    }

    pub fn part_for_mark(&self, id: &str) -> Option<&FunnelPart> {
        self.parts.iter().find(|part| part.mark_id == id)
    }

    pub fn interaction_binding(&self) -> InteractionBinding {
        self.interaction.clone()
    }

    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        let candidate = SceneInteraction {
            scene: self.scene,
            plot: Some(self.plot),
            interaction: Some(self.interaction),
        };
        candidate.validate()?;
        Ok(candidate)
    }
}

pub struct FunnelSizeContext<'a> {
    pub datum: &'a FunnelDatum,
    pub source_index: usize,
    pub scaled_size: f64,
    pub available_size: f64,
}

pub fn funnel_layout(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
) -> Result<FunnelLayout> {
    funnel_layout_with_size(c, data, o, |context| {
        Ok(match &o.size {
            FunnelSize::Value => context.scaled_size,
            FunnelSize::DatumPixels => context.datum.size.unwrap_or(0.),
            FunnelSize::OrdinalPixels(sizes) => sizes[context.source_index % sizes.len()],
        })
    })
}

struct Frame {
    plot: ClipRect,
    direction: FunnelDirection,
    length: f64,
    cross: f64,
    before: f64,
    available: f64,
}

impl Frame {
    fn new(plot: ClipRect, o: &FunnelLayoutOptions) -> Result<Self> {
        let (length, cross) = match o.direction {
            FunnelDirection::Vertical => (plot.height, plot.width),
            FunnelDirection::Horizontal => (plot.width, plot.height),
        };
        let reserve =
            |side: Option<FunnelSeparatorOptions>| side.map_or(0., |s| s.length + s.offset);
        let before = reserve(o.before);
        let available = cross - before - reserve(o.after);
        ensure(available > 0., "funnel separators consume the cross axis")?;
        Ok(Self {
            plot,
            direction: o.direction,
            length,
            cross,
            before,
            available,
        })
    }

    fn point(&self, flow: f64, cross: f64) -> [f64; 2] {
        let (x, y) = match self.direction {
            FunnelDirection::Vertical => (cross, flow),
            FunnelDirection::Horizontal => (flow, cross),
        };
        [self.plot.x + x, self.plot.y + y]
    }

    fn bounds(&self, start: f64, end: f64, edges: [f64; 2]) -> ClipRect {
        let [x, y] = self.point(start, edges[0]);
        let [x1, y1] = self.point(end, edges[1]);
        ClipRect {
            x,
            y,
            width: x1 - x,
            height: y1 - y,
        }
    }

    fn edges(&self, size: f64) -> [f64; 2] {
        let low = self.before + (self.available - size) / 2.;
        [low, low + size]
    }
}

fn part_keys(data: &[FunnelDatum]) -> Result<Vec<String>> {
    ensure(data.len() <= 256, "funnel exceeds 256 stages")?;
    let mut seen = BTreeSet::new();
    data.iter()
        .map(|datum| {
            let key = datum.id.identity()?;
            ensure(
                key.len() <= MAX_LABEL_BYTES - 32
                    && !matches!(&datum.id, CategoryKey::String(id) if id.trim().is_empty())
                    && datum.label.len() <= MAX_LABEL_BYTES
                    && seen.insert(key.clone()),
                "invalid or duplicate funnel stage identity",
            )?;
            ensure(
                datum.value.is_finite() && datum.value >= 0. && datum.size.is_none_or(finite_size),
                "invalid funnel value or pixel size",
            )?;
            if let Some(paint) = &datum.paint {
                paint.validate()?;
            }
            Ok(key)
        })
        .collect()
}

fn graphic_point([x, y]: [f64; 2]) -> P {
    P::new(x, y)
}

fn append_separators(
    c: &ChartConfig,
    o: &FunnelLayoutOptions,
    frame: &Frame,
    data: &[FunnelDatum],
    keys: &[String],
    sizes: &[f64],
    layout: &mut FunnelLayout,
) {
    if data.is_empty() {
        return;
    }
    for (side, name, options) in [
        (FunnelSeparatorSide::Before, "before", o.before),
        (FunnelSeparatorSide::After, "after", o.after),
    ] {
        let Some(options) = options else {
            continue;
        };
        let stroke = options.stroke.unwrap_or(GraphicStroke {
            color: c.theme.grid,
            width: 1.,
            cap: StrokeCap::Butt,
        });
        for index in 0..=data.len() {
            let tail = index == data.len();
            let flow = if tail {
                frame.length
            } else {
                index as f64 * (layout.bandwidth + o.spacing) - o.spacing / 2.
            };
            let [low, high] = frame.edges(sizes[index.min(data.len() - 1)]);
            let (from, to) = match side {
                FunnelSeparatorSide::Before => (0., low - options.offset),
                FunnelSeparatorSide::After => (high + options.offset, frame.cross),
            };
            let from = frame.point(flow, from);
            let to = frame.point(flow, to);
            let id = keys.get(index).map_or_else(
                || format!("funnel:separator:{name}:tail"),
                |key| format!("funnel:separator:{name}:stage:{key}"),
            );
            let mut mark = Mark::new(
                id,
                Geometry::Path(vec![
                    Cmd::Move(graphic_point(from)),
                    Cmd::Line(graphic_point(to)),
                ]),
                stroke.color,
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(stroke);
            layout.scene.marks.push(mark);
            layout.separators.push(FunnelSeparator {
                part_id: data.get(index).map(|d| d.id.clone()),
                side,
                from,
                to,
            });
        }
    }
}

/// Use an application-supplied cross-axis pixel width. The default scaled
/// value and the space remaining between the separator reserves are supplied.
pub fn funnel_layout_with_size(
    c: &ChartConfig,
    data: &[FunnelDatum],
    o: &FunnelLayoutOptions,
    mut size: impl FnMut(FunnelSizeContext<'_>) -> Result<f64>,
) -> Result<FunnelLayout> {
    let scene = c.scene()?;
    o.validate()?;
    let keys = part_keys(data)?;
    let frame = Frame::new(ClipRect::plot(c), o)?;
    let bandwidth = if data.is_empty() {
        0.
    } else {
        (frame.length - o.spacing * (data.len() - 1) as f64) / data.len() as f64
    };
    ensure(
        data.is_empty() || bandwidth > 0.,
        "funnel spacing consumes the flow axis",
    )?;
    let max = data.iter().map(|datum| datum.value).fold(0., f64::max);
    let sizes = data
        .iter()
        .enumerate()
        .map(|(source_index, datum)| {
            let t = if max == 0. { 0.5 } else { datum.value / max };
            let width = size(FunnelSizeContext {
                datum,
                source_index,
                scaled_size: frame.available * t,
                available_size: frame.available,
            })?;
            ensure(finite_size(width), "invalid funnel stage pixel width")?;
            Ok(width)
        })
        .collect::<Result<Vec<_>>>()?;
    let colors = o
        .colors
        .resolve(data.iter().map(|datum| datum.id.clone()))?;
    let mut layout = FunnelLayout {
        scene,
        colors,
        bandwidth,
        plot: frame.plot,
        parts: Vec::with_capacity(data.len()),
        separators: vec![],
        value_domain: [0., max],
        size_range: [0., frame.available],
        interaction: InteractionBinding::default(),
    };
    append_separators(c, o, &frame, data, &keys, &sizes, &mut layout);

    let current = o.current.as_ref().map(CategoryKey::identity).transpose()?;
    let curve = o.interpolation.into();
    for (i, datum) in data.iter().enumerate() {
        let key = &keys[i];
        let series = format!("funnel:{key}");
        let mark_id = format!("funnel:stage:{key}");
        let mapped_color = layout.colors.map_or_insert(datum.id.clone())?;
        let color = datum
            .color
            .or(mapped_color)
            .ok_or_else(|| ChartError("missing funnel stage color".into()))?;
        let formatted_value = o.value_format.format(datum.value)?;
        let label = if datum.label.is_empty() {
            key_label(&datum.id)
        } else {
            datum.label.clone()
        };
        let is_current = current.as_ref() == Some(key);
        let extension = if is_current {
            o.current_size_extension
        } else {
            0.
        };
        let start = i as f64 * (bandwidth + o.spacing);
        let end = if i + 1 == data.len() {
            frame.length
        } else {
            start + bandwidth
        };
        let near = frame.edges(sizes[i]);
        let far = frame.edges(sizes.get(i + 1).copied().unwrap_or(sizes[i]));
        let center = frame.point(
            start + (end - start) / 2.,
            frame.before + frame.available / 2.,
        );
        let blend = (end - start) * o.shape_blending / 2.;
        let profile = [
            (start, near),
            (start + blend, near),
            (end - blend, far),
            (end, far),
        ]
        .map(|(flow, [low, high])| FunnelProfilePoint {
            before: frame.point(flow, low - extension),
            after: frame.point(flow, high + extension),
        });
        let part = FunnelPart {
            datum: datum.clone(),
            source_index: i,
            series,
            mark_id,
            color,
            bounds: frame.bounds(start, end, near),
            center,
            formatted_value,
            is_current,
            profile,
        };
        append_part(c, o, curve, part, label, key, &mut layout)?;
    }
    layout.interaction.validate(&layout.scene)?;
    Ok(layout)
}

fn append_part(
    c: &ChartConfig,
    o: &FunnelLayoutOptions,
    curve: Curve,
    part: FunnelPart,
    label: String,
    key: &str,
    layout: &mut FunnelLayout,
) -> Result<()> {
    let border_width = if part.is_current {
        o.current_border_width.unwrap_or(o.border_width)
    } else {
        o.border_width
    };
    let low = part.profile.map(|point| graphic_point(point.before));
    let high = part.profile.map(|point| graphic_point(point.after));
    let context = InheritedColorContext {
        inherited: part.color,
        theme: c.theme.text,
        surface: part.color,
        backdrop: c.theme.background,
    };
    let mut low_reverse = low;
    low_reverse.reverse();
    if border_width > 0. && o.border_opacity > 0. {
        let color = o.border_color.resolve_validated(context)?;
        let mut high_reverse = high;
        high_reverse.reverse();
        let border = path(&low, curve)
            .into_iter()
            .chain(path(&high_reverse, curve))
            .collect();
        let mut mark = Mark::new(
            format!("funnel:border:{key}"),
            Geometry::Path(border),
            color,
        );
        mark.fill = None;
        mark.label.clear();
        mark.series = part.series.clone();
        mark.opacity = o.border_opacity;
        mark.stroke = Some(GraphicStroke {
            color,
            width: border_width,
            cap: StrokeCap::Butt,
        });
        layout.scene.marks.push(mark);
    }

    if low != high {
        let mut area = path(&high, curve);
        let side_len = area.len();
        area.extend(
            path(&low_reverse, curve)
                .into_iter()
                .map(|command| match command {
                    Cmd::Move(point) => Cmd::Line(point),
                    command => command,
                }),
        );
        area.push(Cmd::Close);
        let mut mark = Mark::new(&part.mark_id, Geometry::Path(area), part.color);
        mark.series = part.series.clone();
        mark.label = label.clone();
        mark.value = part.datum.value;
        mark.opacity = o.fill_opacity;
        mark.fill_paint = part.datum.paint.clone();
        let anchor = InteractionAnchor::path_points(
            &mark.geometry,
            part.center,
            &[0, side_len - 1, side_len, side_len * 2 - 1],
        )?;
        layout.interaction.anchors.insert(mark.id.clone(), anchor);
        layout.interaction.sites.push(InteractionSite {
            mark_id: mark.id.clone(),
            series: mark.series.clone(),
            label: label.clone(),
            value: Some(mark.value),
            formatted_value: Some(part.formatted_value.clone()),
            color: part.color,
            position: part.center,
        });
        layout.scene.marks.push(mark);
    }
    if let Some(labels) = &o.labels {
        let size = labels.size.unwrap_or(c.theme.font_size);
        let text = match labels.text {
            FunnelLabelText::Value => part.formatted_value.clone(),
            FunnelLabelText::Datum => label.clone(),
        };
        layout
            .scene
            .label_series
            .insert(layout.scene.labels.len(), part.series.clone());
        layout.scene.labels.push(Label {
            x: part.center[0],
            y: part.center[1] + size * 0.35,
            text,
            size,
            color: labels.color.resolve_validated(InheritedColorContext {
                theme: c.theme.background,
                ..context
            })?,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
    }
    if o.legend {
        layout.scene.legend.push(LegendItem {
            id: part.series.clone(),
            label,
            color: part.color,
        });
    }
    layout.parts.push(part);
    Ok(())
}
