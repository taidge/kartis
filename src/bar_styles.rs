//! Configured bar layout. Primary references inspected locally:
//! E:/Repos/nivo/packages/bar/src/{types.ts,compute/common.ts,
//! compute/grouped.ts,compute/stacked.ts,compute/totals.ts} (0.99.0).
//! Bands, stacks, totals and labels are solved together in data coordinates;
//! clipping never replaces a true rounded endpoint with a new rounded cap.
use std::collections::{BTreeMap, BTreeSet};

use crate::scales::{BandScale, BandScaleOptions};
use crate::scene::ensure;
use crate::*;
mod custom;
mod widgets;
pub use custom::{
    BarDatumContext, BarDatumGeometry, BarFactories, BarGeometryFactory, BarLabelContext,
    BarLabelFactory, CustomBarLayout, ScaledCustomBarLayout, bar_layout_with_bar_factory,
    bar_layout_with_factories, bar_layout_with_scale_options_and_bar_factory,
    bar_layout_with_scale_options_and_factories,
};
pub use widgets::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarColorBy {
    #[default]
    Series,
    Category,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLabelPosition {
    Start,
    #[default]
    Middle,
    End,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLabelValue {
    #[default]
    Raw,
    Displayed,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarZeroExtentLabels {
    /// Preserve the default requirement that both visible dimensions are positive.
    #[default]
    Skip,
    /// Allow a zero value-axis extent with positive visible category breadth.
    /// The original unclamped endpoints must project to the same finite pixel;
    /// cropping alone cannot qualify a label. Existing skip thresholds still apply.
    ValueAxis,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BarTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
    pub rotation: f64,
}
impl BarTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && self.rotation.is_finite()
                && (-360. ..=360.).contains(&self.rotation),
            "invalid bar text style",
        )
    }
    fn label(
        &self,
        c: &ChartConfig,
        position: [f64; 2],
        text: String,
        anchor: TextAnchor,
    ) -> Label {
        Label {
            x: position[0],
            y: position[1],
            text,
            color: self.color.unwrap_or(c.theme.text),
            size: self.size.unwrap_or(c.theme.font_size),
            rotation: self.rotation,
            anchor,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarLabels {
    pub enabled: bool,
    pub zero_extent: BarZeroExtentLabels,
    /// Explicit owned value-label role; None keeps the existing fixed/theme
    /// style. Some overrides style.color, while Theme resolves that style.
    /// Inherited and surface colors are the numeric pre-factory palette,
    /// independent of custom paint or the typed site's representative color.
    pub color_profile: Option<InheritedColor>,
    /// Nivo's visual rectangle convention: horizontal start=left/end=right;
    /// vertical start=bottom/end=top. Offset is rightward/upward, respectively.
    pub position: BarLabelPosition,
    pub offset: f64,
    pub skip_width: f64,
    pub skip_height: f64,
    pub value: BarLabelValue,
    pub format: ValueFormat,
    pub style: BarTextStyle,
    pub clip_to_plot: bool,
}
impl Default for BarLabels {
    fn default() -> Self {
        Self {
            enabled: false,
            zero_extent: BarZeroExtentLabels::Skip,
            color_profile: None,
            position: Default::default(),
            offset: 0.,
            skip_width: 0.,
            skip_height: 0.,
            value: Default::default(),
            format: ValueFormat::Auto,
            style: Default::default(),
            clip_to_plot: true,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarTotalMode {
    #[default]
    Net,
    PositiveNegative,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BarTotalKind {
    Net,
    Positive,
    Negative,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarTotals {
    pub enabled: bool,
    pub mode: BarTotalMode,
    /// Percent bars retain raw totals unless Displayed is explicitly selected.
    pub value: BarLabelValue,
    pub format: ValueFormat,
    pub offset: f64,
    pub style: BarTextStyle,
    pub clip_to_plot: bool,
}
impl Default for BarTotals {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: Default::default(),
            value: Default::default(),
            format: ValueFormat::Auto,
            offset: 10.,
            style: Default::default(),
            clip_to_plot: false,
        }
    }
}
/// Select the identities shown by the styled Bar legend.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BarLegendSource {
    /// Preserve the original series-key legend and series controls.
    #[default]
    Keys,
    /// Source categories, with the first ordered visible key's palette color.
    /// These are category items, not automatic series visibility controls.
    Indexes,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BarStyleOptions {
    /// Explicit shared band profile; excludes the two legacy category padding overrides.
    /// Rounding resolves on the complete category range before window projection.
    pub category_scale: Option<BandScaleOptions>,
    /// Reverse the explicit category range without reordering source identities.
    /// Requires category_scale; horizontal true matches Nivo's y-band direction.
    pub category_reverse: bool,
    /// Ratios of category step. None inherits BarOptions.padding, preserving
    /// its original D3-band inner=outer padding convention.
    pub category_inner_padding: Option<f64>,
    pub category_outer_padding: Option<f64>,
    /// Logical pixel gap between grouped series. Unlike category padding this
    /// is a fixed pixel distance, as Nivo's innerPadding. Stacks ignore it.
    pub inner_padding: f64,
    pub color_by: BarColorBy,
    /// Keys are series/category IDs according to color_by. Palette fallbacks
    /// use lexical IDs unless an explicit series_order selects the series
    /// palette order. Hidden sources keep their palette slots.
    pub colors: BTreeMap<String, Color>,
    /// Raw datum text, independent of a percent value axis or label format.
    /// None inherits BarOptions.value_axis.format for compatibility.
    pub tooltip_format: Option<ValueFormat>,
    pub labels: BarLabels,
    pub totals: BarTotals,
    /// Master legend switch. Keys keep the original series controls; Indexes
    /// use category identities and never automatically toggle series.
    pub legend: bool,
    pub legend_source: BarLegendSource,
    /// Complete source series-ID permutation. None preserves lexical order.
    /// Some drives bars, grouped/stacked/percent order, palette slots and Keys
    /// legend order. Hidden keys keep their full palette slots. Original source
    /// indices remain unchanged.
    pub series_order: Option<Vec<String>>,
}
impl Default for BarStyleOptions {
    fn default() -> Self {
        Self {
            category_scale: None,
            category_reverse: false,
            category_inner_padding: None,
            category_outer_padding: None,
            inner_padding: 0.,
            color_by: Default::default(),
            colors: Default::default(),
            tooltip_format: None,
            labels: Default::default(),
            totals: Default::default(),
            legend: true,
            legend_source: BarLegendSource::Keys,
            series_order: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct StyledBarDatum {
    pub id: String,
    pub series: String,
    pub category: String,
    pub source_series: usize,
    pub source_category: usize,
    pub raw_value: f64,
    pub displayed_value: f64,
    pub interval: [f64; 2],
    /// Original, unclipped category position and breadth (including the group
    /// slot). This remains the rounded geometry's real category boundary.
    pub band: [f64; 2],
    pub visible_rect: ClipRect,
    pub color: Color,
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StyledBarTotal {
    pub category: String,
    pub kind: BarTotalKind,
    /// None means that mathematical sum cannot be represented as finite f64.
    /// Only the explicitly selected quantity must be representable.
    pub raw_value: Option<f64>,
    pub displayed_value: Option<f64>,
    pub value: f64,
    /// Display-domain position, not the quantity printed by a raw total.
    pub anchor_value: f64,
    /// None when the true anchor cannot be projected inside Scene's finite
    /// coordinate profile. An off-range total is never moved to the crop edge.
    pub position: Option<[f64; 2]>,
    pub label_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StyledBarLayout {
    /// Present only for an explicitly configured category profile. The same
    /// mapping supplies bars, axis/grid centers, total anchors and source bands.
    pub category_scale: Option<BandScale>,
    pub scene: Scene,
    pub bars: Vec<StyledBarDatum>,
    pub totals: Vec<StyledBarTotal>,
    pub value_domain: [f64; 2],
    pub category_step: f64,
    pub category_bandwidth: f64,
}
/// A styled bar layout and the scale used to place its value axis and bars.
/// The source domain can differ from the visible domain when nice is enabled.
#[derive(Clone, Debug)]
pub struct ScaledBarLayout {
    pub layout: StyledBarLayout,
    pub value_scale: ResolvedContinuousScale,
}
fn bounded(v: f64) -> bool {
    v.is_finite() && (0. ..=16384.).contains(&v)
}
fn token(id: &str) -> String {
    format!("{}:{id}", id.len())
}
/// Compensated normalized sum: intermediate overflow must not reject a finite
/// signed net sum, nor may disabled grouped totals reject independent bars.
fn sum(values: impl Iterator<Item = f64>) -> Option<f64> {
    let values = values.collect::<Vec<_>>();
    let maximum = values.iter().map(|v| v.abs()).fold(0., f64::max);
    if maximum == 0. {
        return Some(0.);
    }
    let (mut total, mut error) = (0., 0.);
    for value in values {
        let y = value / maximum - error;
        let t = total + y;
        error = (t - total) - y;
        total = t;
    }
    let value = total * maximum;
    value
        .is_finite()
        .then_some(if value == 0. { 0. } else { value })
}
fn text_clip(scene: &mut Scene, index: usize, c: &ChartConfig) {
    let r = ClipRect::plot(c);
    scene.label_clips.insert(
        index,
        ClipPath {
            geometry: Geometry::Rect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
            rule: ClipRule::NonZero,
        },
    );
}
fn full_rect(
    scale: &ChartScale,
    interval: [f64; 2],
    band: [f64; 2],
    horizontal: bool,
) -> Option<ClipRect> {
    let a = scale.map(interval[0]);
    let b = scale.map(interval[1]);
    let r = if horizontal {
        ClipRect {
            x: a.min(b),
            y: band[0],
            width: (b - a).abs(),
            height: band[1],
        }
    } else {
        ClipRect {
            x: band[0],
            y: a.min(b),
            width: band[1],
            height: (b - a).abs(),
        }
    };
    r.validate().is_ok().then_some(r)
}
fn bar_logical_position(geometry: &Geometry, rect: ClipRect) -> [f64; 2] {
    match geometry {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => [x + width / 2., y + height / 2.],
        _ => [rect.x + rect.width / 2., rect.y + rect.height / 2.],
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BarValidation {
    Full,
    /// Plan the same ordinary layout, but validate axes and all actual labels
    /// before callbacks. Source bar ink is replaced before final validation.
    LabelsBeforeFactories,
}
fn validate(
    style: &BarStyleOptions,
    options: &BarOptions,
    categories: &[String],
    data: &[BarSeries],
) -> Result<()> {
    ensure(
        !style.category_reverse || style.category_scale.is_some(),
        "category reversal requires an explicit band profile",
    )?;
    if let Some(profile) = &style.category_scale {
        profile.validate()?;
        ensure(
            style.category_inner_padding.is_none() && style.category_outer_padding.is_none(),
            "configured category scale excludes legacy padding overrides",
        )?;
    }
    ensure(
        style
            .category_inner_padding
            .is_none_or(|v| v.is_finite() && (0. ..1.).contains(&v))
            && style
                .category_outer_padding
                .is_none_or(|v| v.is_finite() && (0. ..=4.).contains(&v))
            && bounded(style.inner_padding),
        "invalid independent bar padding",
    )?;
    ensure(
        options.hidden.len() <= 256
            && options
                .hidden
                .iter()
                .all(|id| !id.is_empty() && id.len() <= 256),
        "invalid hidden bar series",
    )?;
    if let Some(order) = &style.series_order {
        ordered_series(data, Some(order))?;
    }
    validate_colors(style, categories, data)?;
    let l = &style.labels;
    if let Some(format) = &style.tooltip_format {
        format.validate()?;
    }
    ensure(
        l.offset.is_finite()
            && l.offset.abs() <= 16384.
            && bounded(l.skip_width)
            && bounded(l.skip_height),
        "invalid bar label geometry",
    )?;
    l.style.validate()?;
    if let Some(profile) = &l.color_profile {
        profile.validate()?;
    }
    l.format.validate()?;
    ensure(bounded(style.totals.offset), "invalid bar totals offset")?;
    style.totals.style.validate()?;
    style.totals.format.validate()
}

fn ordered_series<'a>(
    data: &'a [BarSeries],
    order: Option<&[String]>,
) -> Result<Vec<(usize, &'a BarSeries)>> {
    if let Some(order) = order {
        ensure(
            order.len() == data.len() && order.len() <= 256,
            "Bar series order must contain every source series exactly once",
        )?;
        let source = data
            .iter()
            .enumerate()
            .map(|(i, series)| (series.id.as_str(), (i, series)))
            .collect::<BTreeMap<_, _>>();
        let mut seen = BTreeSet::new();
        order
            .iter()
            .map(|id| {
                ensure(seen.insert(id), "duplicate Bar series order identity")?;
                source
                    .get(id.as_str())
                    .copied()
                    .ok_or_else(|| ChartError("unknown Bar series order identity".into()))
            })
            .collect()
    } else {
        let mut source = data.iter().enumerate().collect::<Vec<_>>();
        source.sort_by(|a, b| a.1.id.cmp(&b.1.id));
        Ok(source)
    }
}

fn validate_colors(
    style: &BarStyleOptions,
    categories: &[String],
    data: &[BarSeries],
) -> Result<()> {
    if style.colors.is_empty() {
        return Ok(());
    }
    let ids = match style.color_by {
        BarColorBy::Series => data.iter().map(|s| s.id.as_str()).collect::<BTreeSet<_>>(),
        BarColorBy::Category => categories
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
    };
    ensure(
        style.colors.len() <= crate::MAX_MARKS
            && style.colors.keys().all(|id| ids.contains(id.as_str())),
        "unknown bar color ID",
    )
}
pub fn bar_with_style(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    options: &BarOptions,
    style: &BarStyleOptions,
) -> Result<Scene> {
    Ok(bar_layout_with_style(c, categories, data, options, style)?.scene)
}
/// Shared bar geometry, metadata, labels and totals. Hidden IDs are an input to
/// layout: call again to redistribute grouped slots and recompute aggregates.
/// Generic Scene-only legend filtering cannot recompute aggregate totals.
pub fn bar_layout_with_style(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
) -> Result<StyledBarLayout> {
    validate_value_scale(o.value_scale)?;
    let (layout, ()) = build_bar_layout(c, categories, data, o, style, false, |domain, range| {
        Ok((ChartScale::new(o.value_scale, domain, range)?, ()))
    })?;
    Ok(layout)
}

/// Resolve ordinal colors against every source series or category, including
/// hidden series and categories outside the visible window. Explicit style
/// colors override the scale. Missing scale colors use the ordinary theme.
pub fn bar_layout_with_ordinal_colors(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    colors: &OrdinalColorScale,
) -> Result<StyledBarLayout> {
    let style = style.with_ordinal_colors(categories, data, colors)?;
    bar_layout_with_style(c, categories, data, o, &style)
}

impl BarStyleOptions {
    /// Apply a resolved color scale before passing this style to a bar layout.
    /// The style can also be used with independently configured value scales.
    pub fn with_ordinal_colors(
        &self,
        categories: &[String],
        data: &[BarSeries],
        colors: &OrdinalColorScale,
    ) -> Result<Self> {
        crate::cartesian::validate_bars(categories, data)?;
        validate_colors(self, categories, data)?;
        let mut styled = self.clone();
        let mut add = |id: &String| {
            if let Some(color) = colors.map(&CategoryKey::from(id.as_str())) {
                styled.colors.entry(id.clone()).or_insert(color);
            }
        };
        match self.color_by {
            BarColorBy::Series => data.iter().map(|series| &series.id).for_each(&mut add),
            BarColorBy::Category => categories.iter().for_each(add),
        }
        Ok(styled)
    }
}

/// Lay out styled bars with the shared continuous scale. The supplied scale
/// configuration selects the value scale for this entry point. Zero-based bars
/// accept Linear and SymLog, including for signed and percent stacks.
///
/// The returned scale preserves the configured clamp behavior for data queries.
/// Bar ink is always cropped to the plot. A rounded bar keeps its authored
/// endpoints through that crop, and total offsets follow the value direction.
pub fn bar_layout_with_scale_options(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    value_scale: &ContinuousScaleOptions,
) -> Result<ScaledBarLayout> {
    validate_value_scale(value_scale.kind)?;
    value_scale.validate()?;
    let (layout, value_scale) =
        build_bar_layout(c, categories, data, o, style, true, |domain, range| {
            let resolved = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((resolved.chart_scale().clone(), resolved))
        })?;
    Ok(ScaledBarLayout {
        layout,
        value_scale,
    })
}

fn validate_value_scale(kind: ScaleKind) -> Result<()> {
    ensure(
        kind.supports_zero_baseline(),
        "zero-baseline bars require linear, symlog, sqrt or positive power scale",
    )
}

fn build_bar_layout<S>(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    configured: bool,
    resolve: impl FnOnce([f64; 2], [f64; 2]) -> Result<(ChartScale, S)>,
) -> Result<(StyledBarLayout, S)> {
    build_bar_layout_impl(
        c,
        categories,
        data,
        o,
        style,
        (configured, None, BarValidation::Full),
        resolve,
    )
}

fn build_bar_layout_impl<S>(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    profile: (bool, Option<&mut custom::FactoryState<'_>>, BarValidation),
    resolve: impl FnOnce([f64; 2], [f64; 2]) -> Result<(ChartScale, S)>,
) -> Result<(StyledBarLayout, S)> {
    let (configured, mut custom, validation) = profile;
    c.validate()?;
    crate::cartesian::validate_bars(categories, data)?;
    crate::cartesian::validate_bar_style(o.corner_radius, o.border)?;
    if let Some(profile) = &o.border_color_profile {
        profile.validate()?;
    }
    validate(style, o, categories, data)?;
    ensure(
        o.padding.is_finite() && (0. ..1.).contains(&o.padding),
        "invalid inherited bar padding",
    )?;
    let source_order = ordered_series(data, style.series_order.as_deref())?;
    let visible = source_order
        .iter()
        .copied()
        .filter(|(_, s)| !o.hidden.contains(&s.id))
        .collect::<Vec<_>>();
    let n = categories.len();
    let mut intervals = vec![vec![None; n]; visible.len()];
    let mut positive = vec![0.; n];
    let mut negative = vec![0.; n];
    for i in 0..n {
        let pmax = visible
            .iter()
            .filter_map(|(_, s)| s.values[i])
            .filter(|v| *v > 0.)
            .fold(0., f64::max);
        let nmax = visible
            .iter()
            .filter_map(|(_, s)| s.values[i])
            .filter(|v| *v < 0.)
            .map(|v| -v)
            .fold(0., f64::max);
        let pn = if pmax > 0. {
            visible
                .iter()
                .filter_map(|(_, s)| s.values[i])
                .filter(|v| *v > 0.)
                .map(|v| v / pmax)
                .sum::<f64>()
        } else {
            0.
        };
        let nn = if nmax > 0. {
            visible
                .iter()
                .filter_map(|(_, s)| s.values[i])
                .filter(|v| *v < 0.)
                .map(|v| -v / nmax)
                .sum::<f64>()
        } else {
            0.
        };
        for (j, (_, s)) in visible.iter().enumerate() {
            let Some(raw) = s.values[i] else { continue };
            let value = if o.layout == BarLayout::Percent {
                if raw > 0. {
                    raw / pmax / pn
                } else if raw < 0. {
                    raw / nmax / nn
                } else {
                    0.
                }
            } else {
                raw
            };
            let start = if o.layout == BarLayout::Grouped {
                0.
            } else if raw >= 0. {
                positive[i]
            } else {
                negative[i]
            };
            let end = start + value;
            ensure(end.is_finite(), "bar stack total overflow")?;
            intervals[j][i] = Some([start, end]);
            if o.layout == BarLayout::Grouped {
                positive[i] = positive[i].max(end);
                negative[i] = negative[i].min(end);
            } else if value >= 0. {
                positive[i] = end;
            } else {
                negative[i] = end;
            }
        }
    }
    let mut domain = o.value_domain.unwrap_or_else(|| {
        if o.layout == BarLayout::Percent {
            return [
                if negative.iter().any(|v| *v < 0.) {
                    -1.
                } else {
                    0.
                },
                1.,
            ];
        }
        let lo = negative.iter().copied().fold(0., f64::min);
        let hi = positive.iter().copied().fold(0., f64::max);
        if lo == hi { [0., 1.] } else { [lo, hi] }
    });
    // The legacy profile accepts ascending domains. The configured profile
    // also accepts a descending domain and preserves that order in metadata.
    ensure(
        domain.iter().all(|v| v.is_finite()) && (configured || domain[0] <= domain[1]),
        "invalid styled bar domain",
    )?;
    if domain[0] == 0. {
        domain[0] = 0.;
    }
    if domain[1] == 0. {
        domain[1] = 0.;
    }
    let (scale, resolved) = resolve(
        domain,
        if o.horizontal {
            [c.margin.left, c.width - c.margin.right]
        } else {
            [c.height - c.margin.bottom, c.margin.top]
        },
    )?;
    let domain = scale.domain();
    let [domain_lo, domain_hi] = [domain[0].min(domain[1]), domain[0].max(domain[1])];
    let value_range = scale.range();
    let value_limits = [
        value_range[0].min(value_range[1]),
        value_range[0].max(value_range[1]),
    ];
    let increasing_direction = if (domain[0] <= domain[1]) == (value_range[0] <= value_range[1]) {
        1.
    } else {
        -1.
    };
    // A crop is separate from the configured data-clamp policy. Project the
    // real endpoints before clipping ink or deciding whether a label fits.
    let unclamped = scale.clone().clamped(false);
    let projected = scale.clone().clamped(true);
    let value_pixel = |value| {
        if configured {
            crate::cartesian::bounded_bar_pixel(&unclamped, value, value_limits)
        } else {
            projected.map(value)
        }
    };
    let clip = ClipRect::plot(c);
    let range = if o.horizontal {
        [clip.y, clip.y + clip.height]
    } else {
        [clip.x, clip.x + clip.width]
    };
    let span = range[1] - range[0];
    let window = o.category_window.unwrap_or([0., 1.]);
    ensure(
        window.iter().all(|v| v.is_finite())
            && window[0] >= 0.
            && window[1] <= 1.
            && window[0] < window[1],
        "invalid styled bar category window",
    )?;
    let (step, band, positions, category_scale) = if let Some(profile) = &style.category_scale {
        let scale = BandScale::with_options(
            categories.to_vec(),
            if style.category_reverse {
                [range[1], range[0]]
            } else {
                range
            },
            profile,
        )?
        .windowed(window)?;
        let positions = (0..n)
            .map(|i| scale.at(i).expect("validated category index"))
            .collect::<Vec<_>>();
        ensure(
            positions
                .iter()
                .all(|p| p.is_finite() && (p + scale.bandwidth).is_finite()),
            "configured bar band projection overflow",
        )?;
        (scale.step(), scale.bandwidth, positions, Some(scale))
    } else {
        let inner = style.category_inner_padding.unwrap_or(o.padding);
        let outer = style.category_outer_padding.unwrap_or(o.padding);
        let original_step = span / (n as f64 - inner + 2. * outer).max(1.);
        let window_span = window[1] - window[0];
        let step = original_step / window_span;
        let band = step * (1. - inner);
        let offset = (original_step * outer - span * window[0]) / window_span;
        let positions = (0..n)
            .map(|i| range[0] + offset + i as f64 * step)
            .collect::<Vec<_>>();
        ensure(
            [step, band, offset].iter().all(|v| v.is_finite())
                && positions
                    .iter()
                    .all(|p| p.is_finite() && (p + band).is_finite()),
            "styled bar band projection overflow",
        )?;
        (step, band, positions, None)
    };
    let groups = if o.layout == BarLayout::Grouped {
        visible.len().max(1)
    } else {
        1
    };
    let gap = if o.layout == BarLayout::Grouped {
        style.inner_padding
    } else {
        0.
    };
    let available = band - gap * groups.saturating_sub(1) as f64;
    ensure(
        available >= 0.,
        "grouped bar gaps exceed category bandwidth",
    )?;
    let breadth = available / groups as f64;
    let source_count = categories
        .len()
        .checked_mul(visible.len())
        .ok_or_else(|| ChartError("bar output count overflow".into()))?;
    let label_budget = if style.labels.enabled {
        source_count
    } else {
        0
    } + if style.totals.enabled {
        n * if style.totals.mode == BarTotalMode::Net {
            1
        } else {
            2
        }
    } else {
        0
    };
    // Source work is bounded independently of visible output. Admission below
    // counts actual emitted labels/marks: a narrow category window or skip
    // policy must not be rejected by an invisible worst-case label count.
    ensure(
        source_count <= MAX_MARKS,
        "styled bar output budget exceeded",
    )?;
    let series_palette = source_order
        .iter()
        .enumerate()
        .map(|(i, (_, series))| (series.id.clone(), c.theme.color(i)))
        .collect::<BTreeMap<_, _>>();
    let category_palette = categories
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .enumerate()
        .map(|(i, id)| (id, c.theme.color(i)))
        .collect::<BTreeMap<_, _>>();
    let color = |series: &str, category: &str| {
        let (id, palette) = match style.color_by {
            BarColorBy::Series => (series, &series_palette),
            BarColorBy::Category => (category, &category_palette),
        };
        style.colors.get(id).copied().unwrap_or(palette[id])
    };
    // Resolve every actual Some source role before any factory callback,
    // including hidden/cropped sources and labels disabled by eligibility.
    let label_colors = if let Some(profile) = &style.labels.color_profile {
        let mut colors = BTreeMap::new();
        for row in data {
            for (i, value) in row.values.iter().enumerate() {
                if value.is_none() {
                    continue;
                }
                let key = if style.color_by == BarColorBy::Series {
                    &row.id
                } else {
                    &categories[i]
                };
                if !colors.contains_key(key) {
                    let palette = color(&row.id, &categories[i]);
                    colors.insert(
                        key.clone(),
                        profile.resolve_validated(InheritedColorContext {
                            inherited: palette,
                            surface: palette,
                            theme: style.labels.style.color.unwrap_or(c.theme.text),
                            backdrop: c.theme.background,
                        })?,
                    );
                }
            }
        }
        Some(colors)
    } else {
        None
    };
    // The border is resolved from the same numeric palette as owned labels,
    // before factories can replace fill or representative color. Keep missing
    // sources distinct and preflight hidden/cropped actual Some values.
    let borders = if o.border_color_profile.is_some() {
        let mut borders = BTreeMap::new();
        for row in data {
            for (i, value) in row.values.iter().enumerate() {
                if value.is_none() {
                    continue;
                }
                let key = if style.color_by == BarColorBy::Series {
                    &row.id
                } else {
                    &categories[i]
                };
                if !borders.contains_key(key) {
                    borders.insert(
                        key.clone(),
                        crate::cartesian::resolve_bar_border(c, o, color(&row.id, &categories[i]))?,
                    );
                }
            }
        }
        Some(borders)
    } else {
        None
    };
    let mut scene = c.scene()?;
    crate::cartesian::bar_axes_positions(
        &mut scene,
        c,
        categories,
        &positions.iter().map(|p| p + band / 2.).collect::<Vec<_>>(),
        &projected,
        o,
    )?;
    scene.marks.reserve(source_count);
    scene
        .labels
        .reserve(label_budget.min(MAX_MARKS.saturating_sub(scene.labels.len())));
    let mut bars = Vec::with_capacity(source_count);
    let mut totals = vec![];
    if style.legend {
        match style.legend_source {
            BarLegendSource::Keys => {
                for (_, series) in &source_order {
                    let id = &series.id;
                    scene.legend.push(LegendItem {
                        id: id.clone(),
                        label: id.clone(),
                        color: if style.color_by == BarColorBy::Series {
                            style.colors.get(id).copied().unwrap_or(series_palette[id])
                        } else {
                            Color(110, 125, 140, 255)
                        },
                    });
                }
            }
            BarLegendSource::Indexes => {
                if breadth > 0.
                    && let Some((_, first)) = visible.first()
                {
                    ensure(
                        categories.len() <= 256,
                        "Bar index legend exceeds 256 categories",
                    )?;
                    // Nivo indexes consume source bars, including null/zero,
                    // before barsWithValue and plotting/crop filters. Custom
                    // paint does not change that source bar's numeric color.
                    for category in categories {
                        scene.legend.push(LegendItem {
                            id: category.clone(),
                            label: category.clone(),
                            color: color(&first.id, category),
                        });
                        scene
                            .legend_roles
                            .insert(category.clone(), LegendRole::Category);
                    }
                    if o.horizontal {
                        scene.legend.reverse();
                    }
                }
            }
        }
    }
    if let Some(state) = custom.as_deref_mut() {
        state.count_scene(&scene)?;
    }
    for (slot, (si, s)) in visible.iter().enumerate() {
        for (i, category) in categories.iter().enumerate() {
            let Some(interval) = intervals[slot][i] else {
                continue;
            };
            let raw = s.values[i].unwrap();
            let displayed = interval[1] - interval[0];
            let lower = interval[0].min(interval[1]);
            let upper = interval[0].max(interval[1]);
            if upper < domain_lo
                || lower > domain_hi
                || lower != upper && (upper == domain_lo || lower == domain_hi)
            {
                continue;
            }
            let original_band = [
                positions[i]
                    + slot as f64 * (breadth + gap) * f64::from(o.layout == BarLayout::Grouped),
                breadth,
            ];
            let a = original_band[0].max(range[0]);
            let b = (original_band[0] + breadth).min(range[1]);
            if b <= a {
                continue;
            }
            let x = value_pixel(interval[0]);
            let y = value_pixel(interval[1]);
            let rect = if o.horizontal {
                ClipRect {
                    x: x.min(y),
                    y: a,
                    width: (x - y).abs(),
                    height: b - a,
                }
            } else {
                ClipRect {
                    x: a,
                    y: x.min(y),
                    width: b - a,
                    height: (x - y).abs(),
                }
            };
            let id = format!("styled-bar:{}:{}", token(&s.id), token(category));
            let fill = color(&s.id, category);
            let border = borders.as_ref().map_or(o.border, |borders| {
                let key = if style.color_by == BarColorBy::Series {
                    &s.id
                } else {
                    category
                };
                borders[key]
            });
            let geometry = if o.corner_radius > 0. {
                crate::cartesian::rounded_bar_projection(
                    &unclamped,
                    interval,
                    original_band,
                    clip,
                    o.horizontal,
                    o.corner_radius,
                    border,
                )
            } else {
                Geometry::Rect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height,
                }
            };
            let position = bar_logical_position(&geometry, rect);
            let mut mark = if let Some(state) = custom.as_deref_mut() {
                let original_rect = full_rect(&unclamped, interval, original_band, o.horizontal);
                let context = BarDatumContext {
                    series: s,
                    category,
                    mark_id: &id,
                    source_series: *si,
                    source_category: i,
                    raw_value: raw,
                    displayed_value: displayed,
                    interval,
                    band: original_band,
                    visible_rect: rect,
                    original_rect,
                    endpoints: interval.map(|v| {
                        let p = unclamped.map(v);
                        p.is_finite().then_some(p)
                    }),
                    position,
                    plot: clip,
                    geometry: &geometry,
                    color: fill,
                    stroke: border,
                    horizontal: o.horizontal,
                };
                state.prepare(&context)?
            } else {
                let mut mark = Mark::new(id.clone(), geometry, fill);
                mark.stroke = border;
                mark
            };
            let mark_opacity = mark.opacity;

            mark.series = s.id.clone();
            mark.value = raw;
            mark.label = format!(
                "{} · {}: {}",
                category,
                s.id,
                style
                    .tooltip_format
                    .as_ref()
                    .unwrap_or(&o.value_axis.format)
                    .format(raw)?
            );
            mark.clip = Some(clip);
            ensure(
                scene.marks.len() < MAX_MARKS,
                "styled bar mark budget exceeded",
            )?;
            scene.marks.push(mark);
            let mut label_index = None;
            let l = &style.labels;
            let positive_area = rect.width > 0. && rect.height > 0.;
            let zero_value_extent = l.zero_extent == BarZeroExtentLabels::ValueAxis
                && if o.horizontal {
                    rect.height > 0. && rect.width == 0.
                } else {
                    rect.width > 0. && rect.height == 0.
                }
                && {
                    let start = unclamped.map(interval[0]);
                    let end = unclamped.map(interval[1]);
                    start.is_finite() && end.is_finite() && start == end
                };
            if l.enabled
                && (positive_area || zero_value_extent)
                && rect.width >= l.skip_width
                && rect.height >= l.skip_height
                && let Some(original) = full_rect(&unclamped, interval, original_band, o.horizontal)
            {
                let position = if o.horizontal {
                    [
                        original.x
                            + match l.position {
                                BarLabelPosition::Start => 0.,
                                BarLabelPosition::Middle => original.width / 2.,
                                BarLabelPosition::End => original.width,
                            }
                            + l.offset,
                        original.y + original.height / 2.,
                    ]
                } else {
                    [
                        original.x + original.width / 2.,
                        original.y
                            + match l.position {
                                BarLabelPosition::Start => original.height,
                                BarLabelPosition::Middle => original.height / 2.,
                                BarLabelPosition::End => 0.,
                            }
                            - l.offset,
                    ]
                };
                // Do not relocate a clipped original endpoint to the crop edge.
                if clip.contains(position[0], position[1]) {
                    ensure(
                        scene.labels.len() < MAX_MARKS,
                        "styled bar label budget exceeded",
                    )?;
                    let index = scene.labels.len();
                    let anchor = if o.horizontal && l.position != BarLabelPosition::Middle {
                        TextAnchor::Start
                    } else {
                        TextAnchor::Middle
                    };
                    scene.label_series.insert(index, s.id.clone());
                    if l.clip_to_plot {
                        text_clip(&mut scene, index, c);
                    }
                    scene.labels.push(l.style.label(
                        c,
                        position,
                        l.format.format(if l.value == BarLabelValue::Raw {
                            raw
                        } else {
                            displayed
                        })?,
                        anchor,
                    ));
                    if let Some(colors) = &label_colors {
                        let key = if style.color_by == BarColorBy::Series {
                            &s.id
                        } else {
                            category
                        };
                        scene.labels[index].color = colors[key];
                    }
                    label_index = Some(index);
                }
            }
            if custom.is_some()
                && let Some(index) = label_index
            {
                custom
                    .as_deref_mut()
                    .unwrap()
                    .label_colors
                    .insert(index, scene.labels[index].color);
                let Color(r, g, b, a) = scene.labels[index].color;
                scene.labels[index].color =
                    Color(r, g, b, (f64::from(a) * mark_opacity).round() as u8);
            }
            bars.push(StyledBarDatum {
                id,
                series: s.id.clone(),
                category: category.clone(),
                source_series: *si,
                source_category: i,
                raw_value: raw,
                displayed_value: displayed,
                interval,
                band: original_band,
                visible_rect: rect,
                color: fill,
                label_index,
            });
        }
    }
    if style.totals.enabled {
        for (i, category) in categories.iter().enumerate() {
            let center = positions[i] + band / 2.;
            if center < range[0] || center > range[1] {
                continue;
            }
            let values = visible
                .iter()
                .filter_map(|(_, s)| s.values[i])
                .collect::<Vec<_>>();
            if values.is_empty() {
                continue;
            }
            let t = &style.totals;
            let kinds = match t.mode {
                BarTotalMode::Net => vec![BarTotalKind::Net],
                BarTotalMode::PositiveNegative => {
                    vec![BarTotalKind::Positive, BarTotalKind::Negative]
                }
            };
            for kind in kinds {
                let matches = |v: f64| match kind {
                    BarTotalKind::Net => true,
                    BarTotalKind::Positive => v > 0.,
                    BarTotalKind::Negative => v < 0.,
                };
                if kind != BarTotalKind::Net && !values.iter().any(|v| matches(*v)) {
                    continue;
                }
                let raw_value = sum(values.iter().copied().filter(|v| matches(*v)));
                let displayed_value = sum(intervals
                    .iter()
                    .filter_map(|row| row[i])
                    .map(|v| v[1] - v[0])
                    .filter(|v| matches(*v)));
                let value = match t.value {
                    BarLabelValue::Raw => raw_value,
                    BarLabelValue::Displayed => displayed_value,
                }
                .ok_or_else(|| ChartError("selected bar total is not representable".into()))?;
                let negative_side = kind == BarTotalKind::Negative
                    || kind == BarTotalKind::Net && positive[i] == 0. && negative[i] < 0.;
                let anchor_value = if negative_side {
                    negative[i]
                } else {
                    positive[i]
                };
                let pixel = unclamped.map(anchor_value);
                let direction = if negative_side {
                    -increasing_direction
                } else {
                    increasing_direction
                };
                let candidate = if o.horizontal {
                    [pixel + direction * t.offset, center]
                } else {
                    [center, pixel + direction * t.offset]
                };
                let position = candidate
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                    .then_some(candidate);
                let mut label_index = None;
                if anchor_value >= domain_lo
                    && anchor_value <= domain_hi
                    && let Some(position) = position
                    && (!t.clip_to_plot || clip.contains(position[0], position[1]))
                {
                    ensure(
                        scene.labels.len() < MAX_MARKS,
                        "styled bar label budget exceeded",
                    )?;
                    let index = scene.labels.len();
                    if t.clip_to_plot {
                        text_clip(&mut scene, index, c);
                    }
                    let anchor = if o.horizontal {
                        if direction < 0. {
                            TextAnchor::End
                        } else {
                            TextAnchor::Start
                        }
                    } else {
                        TextAnchor::Middle
                    };
                    scene
                        .labels
                        .push(t.style.label(c, position, t.format.format(value)?, anchor));
                    label_index = Some(index);
                }
                totals.push(StyledBarTotal {
                    category: category.clone(),
                    kind,
                    raw_value,
                    displayed_value,
                    value,
                    anchor_value,
                    position,
                    label_index,
                });
            }
        }
    }
    if validation == BarValidation::Full {
        scene.validate()?;
    } else {
        // Label factories cannot change which slots exist. Count/validate every
        // actual axis/value/total label before any callback, without charging
        // ordinary bar ink that the geometry factory will replace.
        let source_ids = bars.iter().map(|b| b.id.as_str()).collect::<BTreeSet<_>>();
        for mark in scene
            .marks
            .iter()
            .filter(|m| source_ids.contains(m.id.as_str()))
        {
            ensure(
                mark.id.len() <= MAX_LABEL_BYTES
                    && mark.label.len() <= MAX_LABEL_BYTES
                    && mark.series.len() <= MAX_LABEL_BYTES,
                "Bar source metadata exceeds text budget before factories",
            )?;
        }
        let mut axes_and_labels = scene.clone();
        axes_and_labels
            .marks
            .retain(|m| !source_ids.contains(m.id.as_str()));
        axes_and_labels.validate()?;
    }
    Ok((
        StyledBarLayout {
            category_scale,
            scene,
            bars,
            totals,
            value_domain: domain,
            category_step: step,
            category_bandwidth: band,
        },
        resolved,
    ))
}
