//! Mixed Cartesian charts with one resolved x model and independent value axes.
//!
//! Normalization, domains, stack intervals, bar slots, identities and axes are
//! resolved once. Existing line geometry is reused only after those shared
//! decisions; its axes/legends are discarded. Nullable categorical gaps remain
//! gaps. Numeric series retain irregular sampling. Source row order does not
//! change identities, colors, legend order or bar placement.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd, StrokeCap};

use crate::scales::BandScale;
use crate::scene::ensure;
use crate::{
    AxisOptions, AxisSide, BarLayout, CartesianAxes, CategoryAxisOptions, ChartConfig, ChartError,
    ChartScale, ClipRect, Color, ContinuousScaleOptions, Curve, Geometry, Label, LegendItem,
    LineOptions, Mark, Point, ResolvedContinuousScale, Result, ScaleKind, Scene, Series,
    TextAnchor, ValueFormat, YAxisOptions, YAxisSide,
};

mod configured;
mod intervals;
pub use configured::{
    ComboLayout, ComboScaleOptions, ComboSite, ResolvedComboXScale, combo_layout_with_scale_options,
};

pub const MAX_COMBO_POINTS: usize = 5000;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComboKind {
    Bar,
    Line,
    Area,
    Scatter,
    /// A vertical interval centered on the datum's y value.
    ErrorBars,
    /// A precomputed five-number summary, sharing the grouped bar band slots.
    BoxPlot,
    /// A piecewise-linear low/high envelope. Missing rows/categories split it.
    RangeBand,
}
/// Interval endpoints in the assigned value axis's data units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ComboSpread {
    Range {
        low: f64,
        high: f64,
    },
    Box {
        low: f64,
        q1: f64,
        median: f64,
        q3: f64,
        high: f64,
    },
}
impl ComboSpread {
    fn values(self) -> Vec<f64> {
        match self {
            Self::Range { low, high } => vec![low, high],
            Self::Box {
                low,
                q1,
                median,
                q3,
                high,
            } => vec![low, q1, median, q3, high],
        }
    }
    fn bounds(self) -> [f64; 2] {
        match self {
            Self::Range { low, high } | Self::Box { low, high, .. } => [low, high],
        }
    }
}

/// Labels attached to real datum positions, independent of point visibility.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboPointLabels {
    pub format: ValueFormat,
    pub offset: [f64; 2],
    pub color: Option<Color>,
    pub size: Option<f64>,
}
impl Default for ComboPointLabels {
    fn default() -> Self {
        Self {
            format: ValueFormat::Auto,
            offset: [0., -9.],
            color: None,
            size: None,
        }
    }
}
/// Series-local paint and point-label properties. Defaults preserve the
/// original combo line/bar/area appearance; sizes are logical pixels.
#[derive(Clone, Debug, PartialEq)]
pub struct ComboSeriesStyle {
    pub line_width: f64,
    /// None inherits line_points, except scatter defaults to visible points.
    pub points: Option<bool>,
    /// None inherits ComboOptions::point_radius (a radius, not a diameter).
    pub point_radius: Option<f64>,
    pub point_color: Option<Color>,
    pub point_border: Option<GraphicStroke>,
    pub area_opacity: f64,
    pub opacity: f64,
    pub point_labels: Option<ComboPointLabels>,
    pub median_width: f64,
    /// None uses the theme's text color so the median remains visible inside
    /// the series-colored box fill.
    pub median_color: Option<Color>,
    pub whisker_width: f64,
    /// Box whisker cap width as a fraction of the grouped box width.
    pub whisker_end_size: f64,
    /// Error-bar cap width (full width, in logical pixels).
    pub error_cap_width: f64,
}
impl Default for ComboSeriesStyle {
    fn default() -> Self {
        Self {
            line_width: 2.,
            points: None,
            point_radius: None,
            point_color: None,
            point_border: None,
            area_opacity: 0.35,
            opacity: 1.,
            point_labels: None,
            median_width: 2.,
            median_color: None,
            whisker_width: 2.,
            whisker_end_size: 0.6,
            error_cap_width: 8.,
        }
    }
}
impl ComboSeriesStyle {
    pub fn validate(&self) -> Result<()> {
        ensure(
            [
                self.line_width,
                self.median_width,
                self.whisker_width,
                self.error_cap_width,
            ]
            .into_iter()
            .all(|v| v.is_finite() && (0. ..=256.).contains(&v))
                && self
                    .point_radius
                    .is_none_or(|v| v.is_finite() && (0. ..=256.).contains(&v)),
            "invalid combo series size",
        )?;
        ensure(
            [self.opacity, self.area_opacity, self.whisker_end_size]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(&v)),
            "invalid combo series opacity or whisker fraction",
        )?;
        if let Some(border) = self.point_border {
            ensure(
                border.width.is_finite() && (0.000001..=256.).contains(&border.width),
                "invalid combo point border width",
            )?;
        }
        if let Some(labels) = &self.point_labels {
            labels.format.validate()?;
            ensure(
                labels
                    .offset
                    .into_iter()
                    .all(|v| v.is_finite() && v.abs() <= 16384.)
                    && labels
                        .size
                        .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v)),
                "invalid combo point label style",
            )?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ComboXValue {
    Number(f64),
    Category(String),
}
impl From<f64> for ComboXValue {
    fn from(v: f64) -> Self {
        Self::Number(v)
    }
}
impl From<String> for ComboXValue {
    fn from(v: String) -> Self {
        Self::Category(v)
    }
}
impl From<&str> for ComboXValue {
    fn from(v: &str) -> Self {
        Self::Category(v.into())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComboPoint {
    pub id: Option<String>,
    pub x: ComboXValue,
    pub y: Option<f64>,
    pub spread: Option<ComboSpread>,
}
impl ComboPoint {
    pub fn new(x: impl Into<ComboXValue>, y: f64) -> Self {
        Self {
            id: None,
            x: x.into(),
            y: Some(y),
            spread: None,
        }
    }
    pub fn gap(x: impl Into<ComboXValue>) -> Self {
        Self {
            id: None,
            x: x.into(),
            y: None,
            spread: None,
        }
    }
    pub fn identified(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }
    pub fn with_bounds(mut self, low: f64, high: f64) -> Self {
        self.spread = Some(ComboSpread::Range { low, high });
        self
    }
    pub fn range(x: impl Into<ComboXValue>, low: f64, high: f64) -> Self {
        let middle = if low == high {
            low
        } else {
            low * 0.5 + high * 0.5
        };
        Self::new(x, middle).with_bounds(low, high)
    }
    /// Precomputed [low whisker, lower quartile, median, upper quartile, high
    /// whisker]. No implicit sample quantile or outlier policy is applied.
    pub fn box_plot(x: impl Into<ComboXValue>, values: [f64; 5]) -> Self {
        let [low, q1, median, q3, high] = values;
        let mut point = Self::new(x, median);
        point.spread = Some(ComboSpread::Box {
            low,
            q1,
            median,
            q3,
            high,
        });
        point
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComboSeries {
    pub id: String,
    pub kind: ComboKind,
    pub axis: YAxisSide,
    pub data: Vec<ComboPoint>,
    pub color: Option<Color>,
    pub style: ComboSeriesStyle,
}
impl ComboSeries {
    pub fn new(id: impl Into<String>, kind: ComboKind, data: Vec<ComboPoint>) -> Self {
        Self {
            id: id.into(),
            kind,
            axis: YAxisSide::Left,
            data,
            color: None,
            style: Default::default(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum ComboXAxis {
    Numeric {
        domain: Option<[f64; 2]>,
        scale: ScaleKind,
    },
    Categories {
        values: Vec<String>,
        padding: f64,
    },
}
impl Default for ComboXAxis {
    fn default() -> Self {
        Self::Numeric {
            domain: None,
            scale: ScaleKind::Linear,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComboOptions {
    pub bar_corner_radius: f64,
    pub bar_border: Option<GraphicStroke>,
    pub category_window: Option<[f64; 2]>,
    pub x: ComboXAxis,
    pub x_axis: AxisOptions,
    pub category_axis: CategoryAxisOptions,
    pub x_side: AxisSide,
    pub left: YAxisOptions,
    pub right: YAxisOptions,
    pub curve: Curve,
    pub bar_layout: BarLayout,
    pub bar_width_ratio: f64,
    /// Optional logical-pixel width for one numeric x bar band before grouping.
    /// Auto width uses bar sites only, so dense scatter points cannot shrink it.
    pub numeric_bar_width: Option<f64>,
    pub point_radius: f64,
    pub line_points: bool,
    pub baseline: f64,
    pub hidden: BTreeSet<String>,
    /// Empty selects lexical series-ID order. Otherwise a complete permutation,
    /// including hidden series, determines palette/legend/stack/group ordering.
    pub series_order: Vec<String>,
}
impl Default for ComboOptions {
    fn default() -> Self {
        Self {
            bar_corner_radius: 0.,
            bar_border: None,
            category_window: None,
            x: Default::default(),
            x_axis: Default::default(),
            category_axis: Default::default(),
            x_side: AxisSide::Bottom,
            left: YAxisOptions {
                axis: AxisOptions {
                    grid: true,
                    ..Default::default()
                },
                ..Default::default()
            },
            right: Default::default(),
            curve: Curve::Linear,
            bar_layout: BarLayout::Grouped,
            bar_width_ratio: 0.8,
            numeric_bar_width: None,
            point_radius: 4.,
            line_points: true,
            baseline: 0.,
            hidden: BTreeSet::new(),
            series_order: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Key {
    Number(u64),
    Category(String),
}
impl Key {
    fn id(&self) -> String {
        match self {
            Self::Number(v) => format!("n{v:016x}"),
            Self::Category(v) => format!("c{}:{v}", v.len()),
        }
    }
}
fn key(value: &ComboXValue) -> Result<Key> {
    Ok(match value {
        ComboXValue::Number(v) => {
            ensure(v.is_finite(), "combo x coordinates must be finite")?;
            Key::Number(if *v == 0. {
                0.0_f64.to_bits()
            } else {
                v.to_bits()
            })
        }
        ComboXValue::Category(v) => {
            valid_id(v)?;
            Key::Category(v.clone())
        }
    })
}
fn valid_id(id: &str) -> Result<()> {
    ensure(
        !id.trim().is_empty() && id.len() <= 256,
        "combo IDs must be nonempty and at most 256 bytes",
    )
}
fn axis_index(axis: YAxisSide) -> usize {
    usize::from(axis == YAxisSide::Right)
}
fn extent(values: impl Iterator<Item = f64>, kind: ScaleKind) -> Result<[f64; 2]> {
    let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
    for v in values {
        ensure(v.is_finite(), "combo domain input must be finite")?;
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo == f64::INFINITY {
        return Ok(if kind.requires_nonzero_domain() {
            [1., 10.]
        } else {
            [0., 1.]
        });
    }
    if lo == hi {
        match kind {
            kind if kind.requires_nonzero_domain() => {
                ensure(lo != 0., "log combo domain cannot contain zero")?;
                let a = lo / 10.;
                let b = lo * 10.;
                let a = if a == 0. { lo } else { a };
                let b = if b.is_finite() { b } else { lo };
                lo = a.min(b);
                hi = a.max(b);
            }
            ScaleKind::Utc => {
                lo = (lo - 86_400_000.).max(-crate::MAX_TIMESTAMP_MS as f64);
                hi = (hi + 86_400_000.).min(crate::MAX_TIMESTAMP_MS as f64);
            }
            _ => {
                let pad = (lo.abs() * 0.05).max(1.);
                let a = lo - pad;
                let b = hi + pad;
                if a.is_finite() {
                    lo = a;
                }
                if b.is_finite() {
                    hi = b;
                }
            }
        }
    }
    ensure(
        lo.is_finite() && hi.is_finite() && lo < hi,
        "combo domain cannot be resolved; provide an explicit finite interval",
    )?;
    Ok([lo, hi])
}
enum XEngine {
    Numeric(ChartScale),
    Categories {
        scale: BandScale,
        indices: BTreeMap<String, usize>,
    },
}
impl XEngine {
    fn domain(&self) -> [f64; 2] {
        match self {
            Self::Numeric(s) => s.domain(),
            Self::Categories { scale, .. } => scale.range(),
        }
    }
    fn kind(&self) -> ScaleKind {
        match self {
            Self::Numeric(s) => s.kind(),
            _ => ScaleKind::Linear,
        }
    }
    fn input(&self, p: &ComboPoint) -> Result<(f64, Option<usize>)> {
        match (self, &p.x) {
            (Self::Numeric(s), ComboXValue::Number(x)) => {
                ensure(
                    s.accepts(*x),
                    "combo x is invalid for selected numeric scale",
                )?;
                Ok((*x, None))
            }
            (Self::Categories { scale, indices }, ComboXValue::Category(x)) => {
                let i = *indices.get(x).ok_or_else(|| {
                    ChartError("combo point references an unknown category".into())
                })?;
                Ok((scale.at(i).unwrap() + scale.bandwidth / 2., Some(i)))
            }
            _ => Err(ChartError(
                "combo x values do not match the selected x model".into(),
            )),
        }
    }
    fn pixel(&self, x: f64, configured: bool) -> Option<f64> {
        let p = match self {
            Self::Numeric(s) if scale_contains(s, x, configured) => s.map(x),
            Self::Categories { scale, .. }
                if (scale.range()[0]..=scale.range()[1]).contains(&x) =>
            {
                x
            }
            _ => return None,
        };
        p.is_finite().then_some(p)
    }
    fn bar_pixel(&self, x: f64, configured: bool) -> Option<f64> {
        match self {
            Self::Numeric(scale) => {
                let domain = scale.domain();
                if !configured && domain[0] == domain[1] && x != domain[0] {
                    return None;
                }
                let pixel = scale.map(x);
                pixel.is_finite().then_some(pixel)
            }
            Self::Categories { .. } => Some(x),
        }
    }
}
fn ordered_domain(scale: &ChartScale) -> [f64; 2] {
    let [a, b] = scale.domain();
    [a.min(b), a.max(b)]
}
fn scale_contains(scale: &ChartScale, value: f64, configured: bool) -> bool {
    let [lo, hi] = ordered_domain(scale);
    (lo..=hi).contains(&value)
        || configured && (scale.is_clamped() || lo == hi) && scale.accepts(value)
}
fn resolve_scale(
    configured: bool,
    kind: ScaleKind,
    domain: [f64; 2],
    range: [f64; 2],
    options: Option<ContinuousScaleOptions>,
) -> Result<(ChartScale, Option<ResolvedContinuousScale>)> {
    if configured {
        let options = options.unwrap_or_else(|| ContinuousScaleOptions {
            kind,
            ..Default::default()
        });
        let resolved = ResolvedContinuousScale::new(domain, range, &options)?;
        Ok((resolved.chart_scale().clone(), Some(resolved)))
    } else {
        Ok((ChartScale::new(kind, domain, range)?, None))
    }
}
struct Row {
    source_index: usize,
    key: Key,
    datum_id: String,
    x: f64,
    ordinal: Option<usize>,
    y: Option<f64>,
    spread: Option<ComboSpread>,
    x_label: String,
}
fn formatted(format: &ValueFormat, kind: ScaleKind, value: f64) -> Result<String> {
    if kind == ScaleKind::Utc && *format == ValueFormat::Auto {
        ValueFormat::Utc.format(value)
    } else {
        format.format(value)
    }
}
fn label(c: &ChartConfig, x: f64, y: f64, text: String, anchor: TextAnchor) -> Label {
    Label {
        x,
        y,
        text,
        color: c.theme.text,
        size: c.theme.font_size,
        anchor,
        rotation: 0.,
    }
}
fn stroke(c: Color) -> GraphicStroke {
    GraphicStroke {
        color: c,
        width: 1.,
        cap: StrokeCap::Butt,
    }
}
fn category_axes(
    scene: &mut Scene,
    c: &ChartConfig,
    scale: &BandScale,
    o: &ComboOptions,
) -> Result<()> {
    let opts = &o.category_axis;
    opts.validate()?;
    let requested = opts
        .tick_values
        .as_ref()
        .map(|ticks| ticks.iter().collect::<BTreeSet<_>>());
    if let Some(ticks) = &requested {
        ensure(
            ticks.iter().all(|v| scale.map(v).is_some()),
            "combo category ticks reference unknown categories",
        )?;
    }
    let direction = if o.x_side == AxisSide::Top { -1. } else { 1. };
    let y = if direction < 0. {
        c.margin.top
    } else {
        c.height - c.margin.bottom
    };
    let visible = (0..scale.domain().len())
        .filter(|i| {
            scale.at(*i).is_some_and(|x| {
                let center = x + scale.bandwidth / 2.;
                center >= c.margin.left && center <= c.width - c.margin.right
            })
        })
        .collect::<Vec<_>>();
    let first = visible.first().copied().unwrap_or(0);
    let stride = visible.len().div_ceil(opts.tick_count.max(1)).max(1);
    for (index, value) in scale.domain().iter().enumerate() {
        if requested.as_ref().map_or(
            opts.tick_count == 0 || index.saturating_sub(first) % stride != 0,
            |ticks| !ticks.contains(value),
        ) {
            continue;
        }
        let x = scale.at(index).unwrap() + scale.bandwidth / 2.;
        if x < c.margin.left || x > c.width - c.margin.right {
            continue;
        }
        if opts.grid {
            let mut mark = Mark::new(
                format!("combo-category-grid-{index}"),
                Geometry::Path(vec![
                    Cmd::Move(P::new(x, c.margin.top)),
                    Cmd::Line(P::new(x, c.height - c.margin.bottom)),
                ]),
                c.theme.grid,
            );
            mark.label.clear();
            mark.fill = None;
            mark.stroke = Some(stroke(c.theme.grid));
            scene.marks.push(mark);
        }
        if opts.visible {
            let mut text = label(
                c,
                x,
                y + direction
                    * (opts.tick_padding
                        + if direction > 0. {
                            c.theme.font_size
                        } else {
                            0.
                        }),
                value.clone(),
                TextAnchor::Middle,
            );
            text.rotation = opts.tick_rotation;
            scene.labels.push(text);
        }
    }
    if opts.visible {
        let mut mark = Mark::new(
            "combo-category-axis",
            Geometry::Path(vec![
                Cmd::Move(P::new(c.margin.left, y)),
                Cmd::Line(P::new(c.width - c.margin.right, y)),
            ]),
            c.theme.text,
        );
        mark.label.clear();
        mark.fill = None;
        mark.stroke = Some(stroke(c.theme.text));
        scene.marks.push(mark);
        if let Some(text) = &opts.legend {
            scene.labels.push(label(
                c,
                c.margin.left + c.inner_width() / 2.,
                y + direction * (opts.tick_padding + c.theme.font_size * 2.5),
                text.clone(),
                TextAnchor::Middle,
            ));
        }
    }
    Ok(())
}
/// Normalize and render a mixed chart. Bars on distinct y axes never share a
/// stack: grouped bars use distinct series slots, stacked/percent bars use one
/// slot per value axis. Percent stacking normalizes positive/negative totals
/// independently; tooltips retain raw source values as well as plotted percent.
/// Numeric bars admit Linear/UTC x, and all bars require Linear/SymLog y. Other
/// Numeric box sites share the bar bands (Linear/UTC x), while their summaries
/// may use any supported value scale. Range bands have piecewise-linear edges;
/// their projected coordinates must stay within the Scene coordinate budget.
/// Other numeric series reuse ChartScale's supported scale kinds. Box summaries
/// are precomputed; this function does not infer sample quantiles or outliers.
pub fn combo(c: &ChartConfig, data: &[ComboSeries], o: &ComboOptions) -> Result<Scene> {
    combo_impl(c, data, o, None).map(|out| out.scene)
}

struct ComboOutput {
    scene: Scene,
    layout: Option<configured::LayoutParts>,
}

fn combo_impl(
    c: &ChartConfig,
    data: &[ComboSeries],
    o: &ComboOptions,
    profiles: Option<&ComboScaleOptions>,
) -> Result<ComboOutput> {
    let configured = profiles.is_some();
    if let Some(profiles) = profiles {
        profiles.validate(o)?;
    }
    let x_profile = profiles.and_then(|p| p.x);
    let y_profiles = [
        profiles.and_then(|p| p.left),
        profiles.and_then(|p| p.right),
    ];
    let y_kinds = [
        y_profiles[0].map_or(o.left.scale, |p| p.kind),
        y_profiles[1].map_or(o.right.scale, |p| p.kind),
    ];
    let mut scene = c.scene()?;
    crate::cartesian::validate_bar_style(o.bar_corner_radius, o.bar_border)?;
    o.curve.validate()?;
    o.x_axis.validate()?;
    o.category_axis.validate()?;
    o.left.axis.validate()?;
    o.right.axis.validate()?;
    ensure(
        matches!(o.x_side, AxisSide::Top | AxisSide::Bottom),
        "combo x axis must be top/bottom",
    )?;
    ensure(
        o.bar_width_ratio.is_finite()
            && (0. ..=1.).contains(&o.bar_width_ratio)
            && o.point_radius.is_finite()
            && (0. ..=256.).contains(&o.point_radius)
            && o.baseline.is_finite(),
        "invalid combo geometry options",
    )?;
    ensure(
        o.numeric_bar_width
            .is_none_or(|v| v.is_finite() && (0. ..=16384.).contains(&v)),
        "invalid numeric combo bar width",
    )?;
    ensure(
        data.len() <= 256 && o.hidden.len() <= 256 && o.series_order.len() <= 256,
        "combo series/options budget exceeded",
    )?;
    for id in &o.hidden {
        valid_id(id)?;
    }
    let mut id_map = BTreeMap::new();
    let mut total = 0usize;
    let mut numeric = vec![];
    let mut numeric_bars = vec![];
    let mut has_bars = false;
    for (index, s) in data.iter().enumerate() {
        valid_id(&s.id)?;
        s.style.validate()?;
        ensure(
            id_map.insert(s.id.clone(), index).is_none(),
            "duplicate combo series id",
        )?;
        total = total
            .checked_add(s.data.len())
            .ok_or_else(|| ChartError("combo input budget overflow".into()))?;
        ensure(
            total <= MAX_COMBO_POINTS,
            "combo exceeds 5000 source point budget",
        )?;
        let mut keys = BTreeSet::new();
        let mut duplicate_x = false;
        let mut point_ids = BTreeSet::new();
        for p in &s.data {
            ensure(
                p.y.is_none_or(f64::is_finite),
                "combo y values must be finite",
            )?;
            intervals::validate_point(p, s.kind)?;
            if configured {
                if let ComboXAxis::Numeric { scale, .. } = o.x
                    && x_profile.map_or(scale, |p| p.kind) == ScaleKind::Utc
                    && let ComboXValue::Number(v) = p.x
                {
                    configured::utc_value(v)?;
                }
                if y_kinds[axis_index(s.axis)] == ScaleKind::Utc {
                    if let Some(v) = p.y {
                        configured::utc_value(v)?;
                    }
                    if let Some(spread) = p.spread {
                        for v in spread.values() {
                            configured::utc_value(v)?;
                        }
                    }
                }
            }
            duplicate_x |= !keys.insert(key(&p.x)?);
            if let Some(id) = &p.id {
                valid_id(id)?;
                ensure(point_ids.insert(id), "duplicate explicit combo point ID")?;
            }
            if !o.hidden.contains(&s.id)
                && let ComboXValue::Number(x) = p.x
            {
                numeric.push(x);
                if matches!(s.kind, ComboKind::Bar | ComboKind::BoxPlot) {
                    numeric_bars.push(x);
                }
            }
        }
        ensure(
            !duplicate_x || s.kind == ComboKind::Scatter && s.data.iter().all(|p| p.id.is_some()),
            "duplicate x needs scatter points with explicit unique IDs",
        )?;
        if !o.hidden.contains(&s.id) && matches!(s.kind, ComboKind::Bar | ComboKind::BoxPlot) {
            has_bars = true;
        }
    }
    let order = if o.series_order.is_empty() {
        id_map.values().copied().collect::<Vec<_>>()
    } else {
        let unique = o.series_order.iter().collect::<BTreeSet<_>>();
        ensure(
            unique.len() == data.len()
                && unique.len() == o.series_order.len()
                && unique.iter().all(|id| id_map.contains_key(*id)),
            "combo series_order must be a complete unique permutation",
        )?;
        o.series_order.iter().map(|id| id_map[id]).collect()
    };
    numeric.sort_by(f64::total_cmp);
    numeric.dedup_by(|a, b| *a == *b);
    numeric_bars.sort_by(f64::total_cmp);
    numeric_bars.dedup_by(|a, b| *a == *b);
    let mut resolved_x = None;
    let x = match &o.x {
        ComboXAxis::Numeric { domain, scale } => {
            let scale = x_profile.map_or(*scale, |p| p.kind);
            ensure(
                o.category_window.is_none(),
                "category window requires a categorical x axis",
            )?;
            if has_bars {
                ensure(
                    matches!(
                        scale,
                        ScaleKind::Linear
                            | ScaleKind::Utc
                            | ScaleKind::Pow { .. }
                            | ScaleKind::Sqrt
                    ),
                    "numeric combo bars/boxes require linear, UTC or power x",
                )?;
            }
            let mut d = match domain {
                Some(d) => *d,
                None => extent(numeric.iter().copied(), scale)?,
            };
            if domain.is_none() && numeric_bars.len() > 1 {
                let gap = numeric_bars
                    .windows(2)
                    .map(|w| w[1] - w[0])
                    .filter(|v| *v > 0.)
                    .fold(f64::INFINITY, f64::min);
                ensure(gap.is_finite(), "numeric combo bar spacing overflow")?;
                let half = if gap / 2. > 0. { gap / 2. } else { gap };
                d = [numeric[0] - half, numeric[numeric.len() - 1] + half];
                if scale == ScaleKind::Utc {
                    d = [
                        d[0].floor().max(-crate::MAX_TIMESTAMP_MS as f64),
                        d[1].ceil().min(crate::MAX_TIMESTAMP_MS as f64),
                    ];
                }
            }
            let (chart, resolved) = resolve_scale(
                configured,
                scale,
                d,
                [c.margin.left, c.width - c.margin.right],
                x_profile,
            )?;
            resolved_x = resolved;
            XEngine::Numeric(chart)
        }
        ComboXAxis::Categories { values, padding } => {
            ensure(values.len() <= 10_000, "combo category budget exceeded")?;
            for value in values {
                valid_id(value)?;
            }
            let indices = values
                .iter()
                .enumerate()
                .map(|(i, v)| (v.clone(), i))
                .collect();
            XEngine::Categories {
                scale: BandScale::new(
                    values.clone(),
                    [c.margin.left, c.width - c.margin.right],
                    *padding,
                )?
                .windowed(o.category_window.unwrap_or([0., 1.]))?,
                indices,
            }
        }
    };
    let mut rows = Vec::with_capacity(data.len());
    for s in data {
        if o.hidden.contains(&s.id) {
            // Finite/identity budgets were checked above. Retain structural x
            // validation without comparing hidden data to an empty auto domain.
            for p in &s.data {
                match (&x, &p.x) {
                    (XEngine::Numeric(_), ComboXValue::Number(_)) => {}
                    (XEngine::Categories { indices, .. }, ComboXValue::Category(value)) => ensure(
                        indices.contains_key(value),
                        "combo point references an unknown category",
                    )?,
                    _ => {
                        return Err(ChartError(
                            "combo x values do not match the selected x model".into(),
                        ));
                    }
                }
            }
            rows.push(vec![]);
            continue;
        }
        let mut series_rows = Vec::with_capacity(s.data.len());
        for (source_index, p) in s.data.iter().enumerate() {
            let (input, ordinal) = x.input(p)?;
            let mut x_label = match &p.x {
                ComboXValue::Number(v) => formatted(&o.x_axis.format, x.kind(), *v)?,
                ComboXValue::Category(v) => v.clone(),
            };
            if let Some(id) = &p.id {
                x_label = format!("{id} @ {x_label}");
            }
            let point_key = key(&p.x)?;
            let datum_id =
                p.id.as_ref()
                    .map_or_else(|| point_key.id(), |id| format!("p{}:{id}", id.len()));
            series_rows.push(Row {
                source_index,
                key: point_key,
                datum_id,
                x: input,
                ordinal,
                y: p.y,
                spread: p.spread,
                x_label,
            });
        }
        series_rows.sort_by(|a, b| {
            a.x.total_cmp(&b.x)
                .then_with(|| a.datum_id.cmp(&b.datum_id))
        });
        rows.push(series_rows);
    }
    let mut totals = BTreeMap::<(usize, Key), [f64; 2]>::new();
    let mut bar_axes = BTreeSet::new();
    for &index in &order {
        let s = &data[index];
        if o.hidden.contains(&s.id) || s.kind != ComboKind::Bar {
            continue;
        }
        let side = axis_index(s.axis);
        bar_axes.insert(side);
        ensure(
            y_kinds[side].supports_zero_baseline(),
            "combo bars require a value scale with a defined zero baseline",
        )?;
        for row in &rows[index] {
            if o.bar_layout == BarLayout::Percent
                && let Some(value) = row.y
            {
                let sum = totals.entry((side, row.key.clone())).or_insert([0., 0.]);
                sum[usize::from(value < 0.)] += value;
                ensure(
                    sum.iter().all(|v| v.is_finite()),
                    "combo stack total overflow",
                )?;
            }
        }
    }
    let mut intervals = BTreeMap::<(usize, Key), [f64; 2]>::new();
    let mut cumulative = BTreeMap::<(usize, Key), [f64; 2]>::new();
    let mut extents: [Vec<f64>; 2] = [vec![], vec![]];
    for &index in &order {
        let s = &data[index];
        if o.hidden.contains(&s.id) {
            continue;
        }
        let side = axis_index(s.axis);
        if s.kind == ComboKind::Area {
            ensure(
                !configured || y_kinds[side].supports_zero_baseline(),
                "configured combo area baselines require a value scale with a defined zero baseline",
            )?;
            extents[side].push(o.baseline);
        }
        for row in &rows[index] {
            let Some(raw) = row.y else {
                continue;
            };
            if s.kind == ComboKind::Bar {
                let sign = usize::from(raw < 0.);
                let value = if o.bar_layout == BarLayout::Percent {
                    let total = totals[&(side, row.key.clone())][sign].abs();
                    if total == 0. { 0. } else { raw / total }
                } else {
                    raw
                };
                let values = cumulative
                    .entry((side, row.key.clone()))
                    .or_insert([0., 0.]);
                let base = if o.bar_layout == BarLayout::Grouped {
                    0.
                } else {
                    values[sign]
                };
                let end = base + value;
                ensure(end.is_finite(), "combo stack endpoint overflow")?;
                values[sign] = end;
                intervals.insert((index, row.key.clone()), [base, end]);
                extents[side].extend([base, end]);
            } else {
                extents[side].push(raw);
                if let Some(spread) = row.spread {
                    extents[side].extend(spread.values());
                }
            }
        }
    }
    let mut y_scales = vec![];
    let mut resolved_y = [None, None];
    for (side, settings) in [&o.left, &o.right].into_iter().enumerate() {
        let d = match settings.domain {
            Some(d) => d,
            None => extent(extents[side].iter().copied(), y_kinds[side])?,
        };
        if !configured && bar_axes.contains(&side) {
            ensure(
                d[0] <= 0. && d[1] >= 0.,
                "combo bar value domains must contain zero",
            )?;
        }
        let (chart, resolved) = resolve_scale(
            configured,
            y_kinds[side],
            d,
            [c.height - c.margin.bottom, c.margin.top],
            y_profiles[side],
        )?;
        resolved_y[side] = resolved;
        y_scales.push(chart);
    }
    for (index, s) in data.iter().enumerate() {
        if o.hidden.contains(&s.id) {
            continue;
        }
        if s.kind == ComboKind::Area {
            ensure(
                y_scales[axis_index(s.axis)].accepts(o.baseline),
                "combo area baseline is invalid for its assigned scale",
            )?;
        }
        for row in &rows[index] {
            if let Some(y) = row.y {
                ensure(
                    y_scales[axis_index(s.axis)].accepts(y),
                    "combo value is invalid for its assigned axis",
                )?;
                if let Some(spread) = row.spread {
                    let scale = &y_scales[axis_index(s.axis)];
                    let log_sign = scale.domain()[0].signum();
                    ensure(
                        spread.values().into_iter().all(|v| {
                            scale.accepts(v)
                                && (!configured
                                    || scale.kind() != ScaleKind::Log
                                    || v != 0. && v.signum() == log_sign)
                        }),
                        "combo interval is invalid for its assigned axis",
                    )?;
                }
            }
        }
    }
    match &x {
        XEngine::Numeric(scale) => {
            crate::axes::axis(&mut scene, c, scale, o.x_side, &o.x_axis, "combo-axis-x")?
        }
        XEngine::Categories { scale, .. } => category_axes(&mut scene, c, scale, o)?,
    }
    for (side, settings) in [&o.left, &o.right].into_iter().enumerate() {
        if data
            .iter()
            .any(|s| axis_index(s.axis) == side && !o.hidden.contains(&s.id))
        {
            crate::axes::axis(
                &mut scene,
                c,
                &y_scales[side],
                if side == 0 {
                    AxisSide::Left
                } else {
                    AxisSide::Right
                },
                &settings.axis,
                if side == 0 {
                    "combo-axis-left"
                } else {
                    "combo-axis-right"
                },
            )?;
        }
    }
    let mut grouped_slots = if o.bar_layout == BarLayout::Grouped {
        vec![]
    } else {
        bar_axes
            .iter()
            .map(|side| (false, *side))
            .collect::<Vec<_>>()
    };
    for &index in &order {
        if !o.hidden.contains(&data[index].id)
            && (data[index].kind == ComboKind::BoxPlot
                || data[index].kind == ComboKind::Bar && o.bar_layout == BarLayout::Grouped)
        {
            grouped_slots.push((true, index));
        }
    }
    let band = match &x {
        XEngine::Categories { scale, .. } => scale.bandwidth * o.bar_width_ratio,
        XEngine::Numeric(_) => {
            let mut positions = numeric_bars
                .iter()
                .filter_map(|v| x.pixel(*v, configured))
                .collect::<Vec<_>>();
            positions.sort_by(f64::total_cmp);
            let gap = positions
                .windows(2)
                .map(|w| w[1] - w[0])
                .filter(|v| *v > 0.)
                .fold(f64::INFINITY, f64::min);
            if let Some(width) = o.numeric_bar_width {
                width
            } else if gap.is_finite() {
                gap * o.bar_width_ratio
            } else {
                c.inner_width() * o.bar_width_ratio
            }
        }
    };
    let clip = ClipRect::plot(c);
    let mut paint = Vec::<(usize, usize, Mark)>::new();
    for (palette, &index) in order.iter().enumerate() {
        let s = &data[index];
        let color = s.color.unwrap_or(c.theme.color(palette));
        scene.legend.push(LegendItem {
            id: s.id.clone(),
            label: s.id.clone(),
            color,
        });
        if o.hidden.contains(&s.id) {
            continue;
        }
        let side = axis_index(s.axis);
        let ys = &y_scales[side];
        let y_options = if side == 0 { &o.left } else { &o.right };
        let datum_radius = s.style.point_radius.unwrap_or(o.point_radius);
        let show_points = s
            .style
            .points
            .unwrap_or(s.kind == ComboKind::Scatter || o.line_points);
        let group_width = band / grouped_slots.len().max(1) as f64;
        if matches!(
            s.kind,
            ComboKind::ErrorBars | ComboKind::BoxPlot | ComboKind::RangeBand
        ) {
            let box_slot = grouped_slots.iter().position(|key| *key == (true, index));
            let marks = intervals::render(
                c,
                s,
                &rows[index],
                &x,
                ys,
                y_options,
                clip,
                band,
                group_width,
                box_slot,
                if show_points { datum_radius } else { 0. },
                color,
                configured,
            )?;
            for (kind, mark) in marks {
                paint.push((kind, palette, mark));
            }
            intervals::point_labels(
                &mut scene,
                c,
                s,
                &rows[index],
                &x,
                ys,
                box_slot.map(|slot| -band / 2. + (slot as f64 + 0.5) * group_width),
                configured,
            )?;
            continue;
        }
        if s.kind != ComboKind::Bar {
            intervals::point_labels(&mut scene, c, s, &rows[index], &x, ys, None, configured)?;
        }
        if s.kind == ComboKind::Bar {
            let unclamped_y = ys.clone().clamped(false);
            let slot_key = if o.bar_layout == BarLayout::Grouped {
                (true, index)
            } else {
                (false, side)
            };
            let slot = grouped_slots
                .iter()
                .position(|key| *key == slot_key)
                .unwrap();
            let width = group_width;
            for row in &rows[index] {
                let Some(raw) = row.y else {
                    continue;
                };
                let Some(center) = x.bar_pixel(row.x, configured) else {
                    continue;
                };
                let left = center - band / 2. + slot as f64 * width;
                if left + width <= clip.x || left >= clip.x + clip.width {
                    continue;
                }
                let [base, end] = intervals[&(index, row.key.clone())];
                let d = ordered_domain(ys);
                if base.max(end) < d[0]
                    || base.min(end) > d[1]
                    || base != end && (base.max(end) == d[0] || base.min(end) == d[1])
                {
                    continue;
                }
                let a = ys.map(base.clamp(d[0], d[1]));
                let b = ys.map(end.clamp(d[0], d[1]));
                let mut mark = Mark::new(
                    intervals::identity(s, row),
                    Geometry::Rect {
                        x: left,
                        y: a.min(b),
                        width,
                        height: (a - b).abs(),
                    },
                    color,
                );
                mark.series = s.id.clone();
                mark.opacity = s.style.opacity;
                if o.bar_corner_radius > 0. || configured && o.bar_border.is_some() {
                    mark.geometry = crate::cartesian::rounded_bar_projection(
                        &unclamped_y,
                        [base, end],
                        [left, width],
                        clip,
                        false,
                        o.bar_corner_radius,
                        o.bar_border,
                    );
                }
                mark.stroke = o.bar_border;
                mark.value = raw;
                mark.clip = Some(clip);
                let value = if o.bar_layout == BarLayout::Percent {
                    let percent = ValueFormat::Percent(1).format(end - base)?;
                    format!("{percent} (raw {})", ValueFormat::Auto.format(raw)?)
                } else {
                    formatted(&y_options.axis.format, ys.kind(), raw)?
                };
                mark.label = format!("{} · {}: {value}", s.id, row.x_label);
                ensure(
                    mark.label.len() <= 1024,
                    "combo tooltip byte budget exceeded",
                )?;
                if (d[0]..=d[1]).contains(&end) {
                    intervals::label_at(
                        &mut scene,
                        c,
                        s,
                        row,
                        left + width / 2.,
                        ys.map(end),
                        configured.then_some(ys.kind()),
                    )?;
                }
                paint.push((1, palette, mark));
            }
            continue;
        }
        let mut points = vec![];
        let mut mapping = vec![];
        let mut previous = None;
        for (ri, row) in rows[index].iter().enumerate() {
            if let (Some(last), Some(current)) = (previous, row.ordinal)
                && current > last + 1
            {
                points.push(Point::gap(
                    (points.last().map_or(row.x, |p: &Point| p.x) + row.x) / 2.,
                ));
                mapping.push(None);
            }
            points.push(Point { x: row.x, y: row.y });
            mapping.push(Some(ri));
            previous = row.ordinal;
        }
        let mut local = c.clone();
        local.theme.colors = vec![color];
        let disabled = AxisOptions {
            visible: false,
            grid: false,
            tick_count: 0,
            ..Default::default()
        };
        let line_options = LineOptions {
            curve: if s.kind == ComboKind::Scatter {
                Curve::Linear
            } else {
                o.curve
            },
            area: s.kind == ComboKind::Area,
            points: show_points,
            baseline: o.baseline,
            x_domain: Some(x.domain()),
            y_domain: Some(ys.domain()),
            x_scale: x.kind(),
            y_scale: ys.kind(),
            axes: CartesianAxes {
                x: disabled.clone(),
                y: disabled,
                x_side: o.x_side,
                y_side: if side == 0 {
                    AxisSide::Left
                } else {
                    AxisSide::Right
                },
            },
            ..Default::default()
        };
        let series = [Series::new(&s.id, points)];
        let layer = if configured {
            let category_scale;
            let xs = match &x {
                XEngine::Numeric(xs) => xs,
                XEngine::Categories { scale, .. } => {
                    // Category rows already use band-center pixels. An
                    // identity x projection clips those pixels to the plot.
                    category_scale =
                        ChartScale::new(ScaleKind::Linear, scale.range(), scale.range())?;
                    &category_scale
                }
            };
            crate::cartesian::line_with_scales(&local, &series, &line_options, xs, ys)?
        } else {
            crate::line(&local, &series, &line_options)?
        };
        for mut mark in layer.marks {
            if mark.series.is_empty() {
                continue;
            }
            if mark.id.starts_with("point-") {
                let pi = mark
                    .id
                    .rsplit('-')
                    .next()
                    .unwrap()
                    .parse::<usize>()
                    .unwrap();
                let row = &rows[index][mapping[pi].unwrap()];
                mark.id = intervals::identity(s, row);
                mark.label = format!(
                    "{} · {}: {}",
                    s.id,
                    row.x_label,
                    formatted(&y_options.axis.format, ys.kind(), row.y.unwrap())?
                );
                ensure(
                    mark.label.len() <= 1024,
                    "combo tooltip byte budget exceeded",
                )?;
                if let Geometry::Circle { radius, .. } = &mut mark.geometry {
                    *radius = datum_radius;
                }
                mark.fill = Some(s.style.point_color.unwrap_or(color));
                mark.stroke = s.style.point_border;
                mark.opacity = s.style.opacity;
                paint.push((3, palette, mark));
            } else if s.kind != ComboKind::Scatter {
                let kind = if mark.id.starts_with("area-") { 0 } else { 2 };
                let suffix = mark.id.splitn(3, '-').nth(2).unwrap();
                mark.id = format!("combo:{}:{}:path:{kind}:{suffix}", s.id.len(), s.id);
                if kind == 0 {
                    mark.opacity = s.style.area_opacity * s.style.opacity;
                } else {
                    mark.opacity = s.style.opacity;
                    mark.stroke = (s.style.line_width > 0.).then(|| GraphicStroke {
                        width: s.style.line_width,
                        ..mark.stroke.unwrap_or_else(|| stroke(color))
                    });
                }
                paint.push((kind, palette, mark));
            }
        }
    }
    paint.sort_by_key(|(kind, series, _)| (*kind, *series));
    let mut foreground = vec![];
    let mut grids = vec![];
    for mark in scene.marks.drain(..) {
        if mark.id.contains("-grid-") {
            grids.push(mark);
        } else {
            foreground.push(mark);
        }
    }
    scene.marks = grids;
    scene.marks.extend(paint.into_iter().map(|(_, _, m)| m));
    scene.marks.extend(foreground);
    scene.validate()?;
    let layout = if configured {
        let (sites, anchors) = configured::sites(
            &scene,
            configured::SiteSource {
                data,
                rows: &rows,
                y_scales: &y_scales,
                x: &x,
                intervals: &intervals,
                grouped_slots: &grouped_slots,
                band,
                options: o,
            },
        )?;
        let x_scale = match x {
            XEngine::Numeric(_) => {
                ResolvedComboXScale::Numeric(resolved_x.expect("configured numeric x"))
            }
            XEngine::Categories { scale, .. } => ResolvedComboXScale::Categories(scale),
        };
        let [left, right] = resolved_y;
        Some(configured::LayoutParts {
            plot: clip,
            x_scale,
            left_scale: left.expect("configured left scale"),
            right_scale: right.expect("configured right scale"),
            sites,
            anchors,
        })
    } else {
        None
    };
    Ok(ComboOutput { scene, layout })
}
