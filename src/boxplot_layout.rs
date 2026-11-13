//! Sample-based box plots. References: nivo 0.99.0
//! `packages/boxplot/src/{types.ts,props.ts,compute/stratification.ts,
//! compute/generation.ts}`. R-7 quantiles default to 10/25/50/75/90 percent.
//! Nullable observations are omitted, never coerced to zero. Empty cells retain
//! their category slot and produce no fabricated statistics.
use crate::{scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::collections::{BTreeMap, BTreeSet};

mod factories;
mod legend;
pub use factories::{
    BoxPlotColorContext, BoxPlotColorFactory, BoxPlotFactories, boxplot_layout_with_factories,
    boxplot_layout_with_scale_options_and_factories,
};
pub use legend::{BoxPlotLegendContext, BoxPlotLegendLabelFactory, BoxPlotLegendMode};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StatisticsOrientation {
    #[default]
    Vertical,
    Horizontal,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoxObservation {
    pub id: String,
    pub group: String,
    /// Empty identifies the single ungrouped series.
    pub subgroup: String,
    pub value: Option<f64>,
}
impl BoxObservation {
    pub fn new(
        id: impl Into<String>,
        group: impl Into<String>,
        subgroup: impl Into<String>,
        value: Option<f64>,
    ) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            subgroup: subgroup.into(),
            value,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum BoxWhiskers {
    /// Outer two configured quantiles, matching Nivo's default summary.
    #[default]
    Quantiles,
    /// Observations inside Q1-factor*IQR .. Q3+factor*IQR. Interpolated
    /// quartiles bound the whiskers when no observation reaches a box edge.
    Tukey {
        factor: f64,
    },
    Extrema,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxOutlierShape {
    #[default]
    Circle,
    Square,
    Diamond,
}
/// Select which original computed values supply automatic value-domain bounds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxPlotAutoDomainSource {
    /// Preserve the original visible sample-extrema domain.
    #[default]
    SampleExtent,
    /// Use all five configured quantile values from visible boxes, as Nivo's
    /// summary-domain inference does. Whisker policy and raw observations stay
    /// unchanged. Interpolated summaries use Nivo's x + (y - x) * t arithmetic
    /// when the intermediate difference and result are finite. Only overflow
    /// falls back to the existing stable weighted interpolation. This also
    /// preserves Nivo's constant-value and interpolated signed-zero behavior.
    /// Samples are sorted numerically with stable input-order ties in this
    /// mode, matching Nivo, including the source order of positive/negative zero.
    /// SampleExtent retains the original total-order/ID tie-break sorting.
    /// Empty input retains the existing finite fallback. For UTC,
    /// automatic endpoints use Date TimeClip (truncate toward zero, normalize
    /// zero). Fixed endpoints remain strictly integral, and statistics/query
    /// values retain their original fractional milliseconds. Unlike Nivo's
    /// falsy-zero factory branch, a constant zero summary keeps domain [0, 0].
    SummaryQuantiles,
}
/// Independently automatic or fixed value-domain endpoints. None derives that
/// endpoint using `BoxPlotLayoutOptions::auto_domain_source`. Both fixed endpoints
/// bypass automatic empty-domain inference, like the legacy complete domain.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BoxPlotDomainBounds {
    pub min: Option<f64>,
    pub max: Option<f64>,
}
/// Numeric palette identity for box fills and inherited stroke roles.
/// This does not change subgroup ownership or series visibility.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BoxPlotColorBy {
    /// Preserve the original subgroup palette and Series legend.
    #[default]
    SubGroup,
    /// Use the complete group palette. The legend lists only groups with
    /// actual visible, nonempty boxes and carries Category identities.
    Group,
}
#[derive(Clone, PartialEq)]
pub struct BoxPlotLayoutOptions {
    pub orientation: StatisticsOrientation,
    /// None uses lexical order. Explicit lists must contain every source key;
    /// additional keys reserve empty groups/subgroups.
    pub groups: Option<Vec<String>>,
    pub subgroups: Option<Vec<String>>,
    /// Canonical subgroup series IDs returned by `boxplot_series_id`.
    /// Group category legend IDs do not change series visibility.
    pub hidden: BTreeSet<String>,
    pub quantiles: [f64; 5],
    pub whiskers: BoxWhiskers,
    /// Select automatic endpoint input and summary interpolation arithmetic.
    /// Explicit domains bypass inference; summary interpolation still applies.
    pub auto_domain_source: BoxPlotAutoDomainSource,
    pub domain: Option<[f64; 2]>,
    /// Opt-in independent endpoints; conflicts with `domain`, even if both
    /// bounds are automatic. Fixed values may crop the source. With one fixed
    /// nonzero endpoint and no source/tick values, its sign seeds the existing
    /// empty Log/inverse-Pow fallback. Actual invalid source/ticks still reject.
    /// Scale validation, nice, reverse, rounding and clipping remain shared.
    pub domain_bounds: Option<BoxPlotDomainBounds>,
    pub scale: ScaleKind,
    pub padding: f64,
    /// Explicit D3 group band profile. Some replaces `padding`; None preserves
    /// the legacy group positions. Subgroup `inner_padding` remains pixels.
    /// Round applies to the complete group band before subgroup division.
    pub category_scale: Option<crate::scales::BandScaleOptions>,
    /// Reverse the complete group range; requires category_scale. Subgroup
    /// order inside each band is unchanged. This is a D3/Rust extension, not
    /// Nivo's indexScale prop. Horizontal order stays legacy unless explicit.
    pub category_reverse: bool,
    /// Logical pixel gap between visible subgroup boxes.
    /// Visible boxes must retain strictly positive breadth. A zero-width
    /// configured band or a gap consuming the band rejects, even though that
    /// band profile is valid D3. Empty/all-hidden sources need no box breadth.
    pub inner_padding: f64,
    /// Select the complete group or subgroup palette domain. SubGroup keeps
    /// the original output. Group legends are categories, not series toggles.
    pub color_by: BoxPlotColorBy,
    /// Overrides keyed by the selected color_by domain, including reserved and
    /// hidden identities. Missing entries use the full domain's theme index.
    pub colors: BTreeMap<String, Color>,
    pub fill_opacity: f64,
    pub border_radius: f64,
    pub border: Option<GraphicStroke>,
    pub median: GraphicStroke,
    pub whisker: GraphicStroke,
    /// Optional box-outline color role. None preserves the fixed border;
    /// Some only recolors an existing stroke, without enabling a missing one.
    /// Inherited/surface use this box's selected numeric palette, Theme its fixed
    /// stroke color, and backdrop the chart background. Fill opacity is separate.
    pub border_color_profile: Option<InheritedColor>,
    /// Optional median-stroke role with the same palette/backdrop contract;
    /// Theme uses median.color. Width and cap retain their configured meaning.
    pub median_color_profile: Option<InheritedColor>,
    /// Optional role shared by both whisker stems and end caps. Theme uses
    /// whisker.color; cap_size and a zero stroke width still disable their ink.
    /// Profiles validate even when no box exists. Actual contexts resolve only
    /// for visible, nonempty summary boxes, including constant-value boxes.
    pub whisker_color_profile: Option<InheritedColor>,
    /// Fraction of box breadth, 0 disables end caps.
    pub cap_size: f64,
    pub outlier_radius: f64,
    pub outlier_shape: BoxOutlierShape,
    pub outlier_fill: Option<Color>,
    pub outlier_border: Option<GraphicStroke>,
    pub value_axis: AxisOptions,
    pub group_axis: CategoryAxisOptions,
    pub value_format: ValueFormat,
    /// Display captions keyed by the full raw color_by domain (group or
    /// subgroup, not canonical subgroup series IDs). None and empty maps keep
    /// the existing captions; missing keys keep their original display label.
    /// Keys and UTF-8 byte lengths are checked even with legend=false or no
    /// visible boxes. In Keys mode captions do not change row identities,
    /// roles, colors or order; repeated/empty captions keep separate rows.
    /// In ByLabel mode this map supplies each summary's default_text before
    /// the optional factory and exact-text deduplication. This map itself is
    /// not an arbitrary callback or source-property accessor.
    pub legend_labels: Option<BTreeMap<String, String>>,
    /// Keys preserves the original complete-key rows and caption map behavior.
    /// ByLabel visits actual summaries, keeps the first representative of each
    /// exact text, and stably orders unique rows by full color-domain index.
    pub legend_mode: BoxPlotLegendMode,
    pub legend: bool,
}
impl std::fmt::Debug for BoxPlotLayoutOptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut output = f.debug_struct("BoxPlotLayoutOptions");
        output.field("orientation", &self.orientation);
        output.field("groups", &self.groups);
        output.field("subgroups", &self.subgroups);
        output.field("hidden", &self.hidden);
        output.field("quantiles", &self.quantiles);
        output.field("whiskers", &self.whiskers);
        output.field("auto_domain_source", &self.auto_domain_source);
        output.field("domain", &self.domain);
        output.field("domain_bounds", &self.domain_bounds);
        output.field("scale", &self.scale);
        output.field("padding", &self.padding);
        output.field("category_scale", &self.category_scale);
        output.field("category_reverse", &self.category_reverse);
        output.field("inner_padding", &self.inner_padding);
        output.field("color_by", &self.color_by);
        output.field("colors", &self.colors);
        output.field("fill_opacity", &self.fill_opacity);
        output.field("border_radius", &self.border_radius);
        output.field("border", &self.border);
        output.field("median", &self.median);
        output.field("whisker", &self.whisker);
        output.field("border_color_profile", &self.border_color_profile);
        output.field("median_color_profile", &self.median_color_profile);
        output.field("whisker_color_profile", &self.whisker_color_profile);
        output.field("cap_size", &self.cap_size);
        output.field("outlier_radius", &self.outlier_radius);
        output.field("outlier_shape", &self.outlier_shape);
        output.field("outlier_fill", &self.outlier_fill);
        output.field("outlier_border", &self.outlier_border);
        output.field("value_axis", &self.value_axis);
        output.field("group_axis", &self.group_axis);
        output.field("value_format", &self.value_format);
        output.field("legend_labels", &self.legend_labels);
        if self.legend_mode != BoxPlotLegendMode::Keys {
            output.field("legend_mode", &self.legend_mode);
        }
        output.field("legend", &self.legend);
        output.finish()
    }
}
impl Default for BoxPlotLayoutOptions {
    fn default() -> Self {
        Self {
            orientation: Default::default(),
            groups: None,
            subgroups: None,
            hidden: Default::default(),
            quantiles: [0.1, 0.25, 0.5, 0.75, 0.9],
            whiskers: Default::default(),
            auto_domain_source: Default::default(),
            domain: None,
            domain_bounds: None,
            scale: ScaleKind::Linear,
            padding: 0.1,
            category_scale: None,
            category_reverse: false,
            inner_padding: 6.,
            color_by: Default::default(),
            colors: Default::default(),
            fill_opacity: 1.,
            border_radius: 0.,
            border: None,
            median: stats_stroke(Color(45, 52, 64, 255), 2.),
            whisker: stats_stroke(Color(78, 121, 167, 255), 2.),
            border_color_profile: None,
            median_color_profile: None,
            whisker_color_profile: None,
            cap_size: 0.6,
            outlier_radius: 3.,
            outlier_shape: Default::default(),
            outlier_fill: None,
            outlier_border: None,
            value_axis: AxisOptions {
                grid: true,
                ..Default::default()
            },
            group_axis: Default::default(),
            value_format: ValueFormat::Auto,
            legend_labels: None,
            legend_mode: BoxPlotLegendMode::Keys,
            legend: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoxOutlier {
    pub source_id: String,
    pub value: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ComputedBox {
    pub id: String,
    pub group: String,
    pub subgroup: String,
    pub count: usize,
    pub extrema: [f64; 2],
    pub quantiles: [f64; 5],
    pub mean: f64,
    pub whiskers: [f64; 2],
    pub outliers: Vec<BoxOutlier>,
    pub source_ids: Vec<String>,
    pub center: f64,
    pub breadth: f64,
    pub rect: ClipRect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct BoxPlotLayout {
    pub scene: Scene,
    pub boxes: Vec<ComputedBox>,
    pub groups: Vec<String>,
    pub subgroups: Vec<String>,
    pub domain: [f64; 2],
    /// Actual full group scale when explicitly configured, shared by axes,
    /// statistics and outliers; None preserves the legacy output profile.
    pub category_scale: Option<crate::scales::BandScale>,
}

/// A BoxPlot with the shared scale used for its axis and statistical geometry.
/// `layout.domain` is the display domain, including any nice extension.
#[derive(Clone, Debug)]
pub struct ScaledBoxPlotLayout {
    pub layout: BoxPlotLayout,
    pub value_scale: ResolvedContinuousScale,
}

impl ScaledBoxPlotLayout {
    /// Project a summary value with the configured query clamp and rounding.
    /// For UTC, interpolated statistics can be fractional epoch milliseconds.
    /// Domain endpoints, axis ticks, and original UTC samples are integral.
    pub fn map_statistic(&self, value: f64) -> Result<f64> {
        let pixel = statistic_pixel(self.value_scale.chart_scale(), value);
        ensure(pixel.is_finite(), "invalid boxplot statistic projection")?;
        Ok(pixel)
    }
}

fn statistic_pixel(scale: &ChartScale, value: f64) -> f64 {
    if scale.kind() == ScaleKind::Utc && value.abs() <= MAX_TIMESTAMP_MS as f64 {
        // D3's UTC scale projects numeric queries continuously, even though
        // domain dates and the inverse use integral TimeClip milliseconds.
        scale.map_transformed(value)
    } else {
        scale.map(value)
    }
}

/// Collision-free ownership key used by box legends and every submark.
pub fn boxplot_series_id(subgroup: &str) -> String {
    format!("boxplot-series:{}:{subgroup}", subgroup.len())
}
pub(crate) fn stats_stroke(color: Color, width: f64) -> GraphicStroke {
    GraphicStroke {
        color,
        width,
        cap: StrokeCap::Butt,
    }
}
pub(crate) fn stats_id(s: &str, empty: bool) -> Result<()> {
    ensure(
        (empty || !s.is_empty()) && s.len() <= 256,
        "statistics IDs must be nonempty and at most 256 bytes",
    )
}
pub(crate) fn stats_stroke_valid(s: Option<GraphicStroke>) -> Result<()> {
    ensure(
        s.is_none_or(|s| s.width.is_finite() && (0. ..=256.).contains(&s.width)),
        "invalid statistics stroke",
    )
}
fn ordered_keys(
    explicit: &Option<Vec<String>>,
    known: BTreeSet<String>,
    allow_empty: bool,
) -> Result<Vec<String>> {
    if let Some(keys) = explicit {
        ensure(keys.len() <= 256, "statistics category budget exceeded")?;
        let mut seen = BTreeSet::new();
        for key in keys {
            stats_id(key, allow_empty)?;
            ensure(seen.insert(key.clone()), "duplicate statistics category")?;
        }
        ensure(
            known.is_subset(&seen),
            "explicit statistics order omits a source category",
        )?;
        Ok(keys.clone())
    } else {
        ensure(known.len() <= 256, "statistics category budget exceeded")?;
        Ok(known.into_iter().collect())
    }
}
fn quantile(v: &[&BoxObservation], p: f64, summary_native: bool) -> f64 {
    let z = (v.len() - 1) as f64 * p;
    let a = z.floor() as usize;
    let b = z.ceil() as usize;
    let x = v[a].value.unwrap();
    let y = v[b].value.unwrap();
    if a == b {
        x
    } else {
        if summary_native {
            let delta = y - x;
            let native = x + delta * z.fract();
            if delta.is_finite() && native.is_finite() {
                return native;
            }
        }
        x * (1. - z.fract()) + y * z.fract()
    }
}
pub(crate) fn stats_path(points: &[[f64; 2]]) -> Geometry {
    Geometry::Path(
        points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == 0 {
                    C::Move(P::new(p[0], p[1]))
                } else {
                    C::Line(P::new(p[0], p[1]))
                }
            })
            .collect(),
    )
}
pub(crate) fn stats_extent(values: impl Iterator<Item = f64>) -> [f64; 2] {
    let mut lo = f64::INFINITY;
    let mut hi = f64::NEG_INFINITY;
    for v in values {
        lo = lo.min(v);
        hi = hi.max(v);
    }
    if lo == f64::INFINITY {
        [0., 1.]
    } else {
        [lo, hi]
    }
}
fn empty_nonzero_domain(
    values: impl Iterator<Item = f64>,
    kind: ScaleKind,
    empty_seed: Option<f64>,
) -> Result<[f64; 2]> {
    let (invalid_value, mixed_sign) = if kind == ScaleKind::Log {
        (
            "log fallback values must be finite and nonzero",
            "log fallback values must have one sign",
        )
    } else {
        (
            "power fallback values must be finite and nonzero",
            "power fallback values must have one sign",
        )
    };
    let mut negative = None;
    for value in values {
        ensure(value.is_finite() && value != 0., invalid_value)?;
        let sign = value < 0.;
        ensure(negative.is_none_or(|old| old == sign), mixed_sign)?;
        negative = Some(sign);
    }
    if negative.is_none()
        && let Some(value) = empty_seed
    {
        ensure(value.is_finite() && value != 0., invalid_value)?;
        negative = Some(value < 0.);
    }
    Ok(if negative == Some(true) {
        [-10., -1.]
    } else {
        [1., 10.]
    })
}
fn box_shape(rect: ClipRect, radius: f64) -> Geometry {
    let ClipRect {
        x,
        y,
        width: w,
        height: h,
    } = rect;
    let r = radius.min(w / 2.).min(h / 2.);
    if r == 0. {
        return Geometry::Rect {
            x,
            y,
            width: w,
            height: h,
        };
    }
    let k = r * 0.5522847498307936;
    let cubic = |first, second, end| C::Cubic { first, second, end };
    Geometry::Path(vec![
        C::Move(P::new(x + r, y)),
        C::Line(P::new(x + w - r, y)),
        cubic(
            P::new(x + w - r + k, y),
            P::new(x + w, y + r - k),
            P::new(x + w, y + r),
        ),
        C::Line(P::new(x + w, y + h - r)),
        cubic(
            P::new(x + w, y + h - r + k),
            P::new(x + w - r + k, y + h),
            P::new(x + w - r, y + h),
        ),
        C::Line(P::new(x + r, y + h)),
        cubic(
            P::new(x + r - k, y + h),
            P::new(x, y + h - r + k),
            P::new(x, y + h - r),
        ),
        C::Line(P::new(x, y + r)),
        cubic(P::new(x, y + r - k), P::new(x + r - k, y), P::new(x + r, y)),
        C::Close,
    ])
}
fn outlier_shape(p: [f64; 2], r: f64, shape: BoxOutlierShape) -> Geometry {
    match shape {
        BoxOutlierShape::Circle => Geometry::Circle {
            x: p[0],
            y: p[1],
            radius: r,
        },
        BoxOutlierShape::Square => Geometry::Rect {
            x: p[0] - r,
            y: p[1] - r,
            width: 2. * r,
            height: 2. * r,
        },
        BoxOutlierShape::Diamond => Geometry::Path(vec![
            C::Move(P::new(p[0], p[1] - r)),
            C::Line(P::new(p[0] + r, p[1])),
            C::Line(P::new(p[0], p[1] + r)),
            C::Line(P::new(p[0] - r, p[1])),
            C::Close,
        ]),
    }
}
/// Transactional pure layout: source and any previously accepted Scene are
/// untouched on failure. Subgroup series IDs own every statistical submark;
/// Group color legends carry independent Category identities.
pub fn boxplot_with_options(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
) -> Result<Scene> {
    Ok(boxplot_layout(c, data, o)?.scene)
}
pub fn boxplot_layout(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
) -> Result<BoxPlotLayout> {
    let (layout, ()) =
        build_boxplot_layout(c, data, o, o.scale, false, None, None, |domain, range| {
            Ok((ChartScale::new(o.scale, domain, range)?, ()))
        })?;
    Ok(layout)
}

/// Use a shared continuous scale for the value axis and BoxPlot geometry.
///
/// This entry point takes its scale kind from `value_scale`. Source domains
/// default to the visible sample extrema, including outliers. Descending
/// domains and reversed ranges are independent. Log requires same-sign
/// nonzero visible samples, including outliers. UTC uses bounded integral
/// epoch-millisecond samples with fractional numeric quantiles.
///
/// Box and whisker endpoints use the actual projection before plot clipping,
/// regardless of the query clamp policy. Outliers retain their source value
/// and their true glyph can intersect the plot even when its center is outside.
pub fn boxplot_layout_with_scale_options(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    value_scale: &ContinuousScaleOptions,
) -> Result<ScaledBoxPlotLayout> {
    value_scale.validate()?;
    let (layout, value_scale) = build_boxplot_layout(
        c,
        data,
        o,
        value_scale.kind,
        true,
        None,
        None,
        |domain, range| {
            let resolved = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((resolved.chart_scale().clone(), resolved))
        },
    )?;
    Ok(ScaledBoxPlotLayout {
        layout,
        value_scale,
    })
}

/// Compute a ByLabel legend from one borrowed call per actual visible summary.
/// The factory is never retained. Keys + factory rejects before any callback.
/// Source, scales, geometry and paints validate before the first callback.
/// Empty text is a real row; text deduplication retains its first full source.
/// Factory side effects that happened before a later Err cannot be rolled back.
pub fn boxplot_layout_with_legend_label_factory(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    factory: &mut BoxPlotLegendLabelFactory<'_>,
) -> Result<BoxPlotLayout> {
    let (layout, ()) = build_boxplot_layout(
        c,
        data,
        o,
        o.scale,
        false,
        Some(factory),
        None,
        |domain, range| Ok((ChartScale::new(o.scale, domain, range)?, ())),
    )?;
    Ok(layout)
}

/// The same borrowed legend factory with the shared configured value scale.
/// quantile_probabilities in the context are the configured probabilities;
/// computed_box.quantiles remain the original (possibly fractional UTC) values.
pub fn boxplot_layout_with_scale_options_and_legend_label_factory(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    value_scale: &ContinuousScaleOptions,
    factory: &mut BoxPlotLegendLabelFactory<'_>,
) -> Result<ScaledBoxPlotLayout> {
    value_scale.validate()?;
    let (layout, value_scale) = build_boxplot_layout(
        c,
        data,
        o,
        value_scale.kind,
        true,
        Some(factory),
        None,
        |domain, range| {
            let resolved = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((resolved.chart_scale().clone(), resolved))
        },
    )?;
    Ok(ScaledBoxPlotLayout {
        layout,
        value_scale,
    })
}

#[allow(clippy::too_many_arguments)]
fn build_boxplot_layout<S>(
    c: &ChartConfig,
    data: &[BoxObservation],
    o: &BoxPlotLayoutOptions,
    kind: ScaleKind,
    configured: bool,
    legend_factory: Option<&mut BoxPlotLegendLabelFactory<'_>>,
    source_colors: Option<&BTreeMap<String, Color>>,
    resolve: impl FnOnce([f64; 2], [f64; 2]) -> Result<(ChartScale, S)>,
) -> Result<(BoxPlotLayout, S)> {
    ensure(
        legend_factory.is_none() || o.legend_mode == BoxPlotLegendMode::ByLabel,
        "BoxPlot legend factory requires ByLabel mode",
    )?;
    let mut scene = c.scene()?;
    ensure(
        o.domain.is_none() || o.domain_bounds.is_none(),
        "boxplot domain and independent bounds conflict",
    )?;
    if let Some(bounds) = o.domain_bounds {
        ensure(
            bounds.min.is_none_or(f64::is_finite) && bounds.max.is_none_or(f64::is_finite),
            "boxplot domain bounds must be finite",
        )?;
    }
    ensure(
        !o.category_reverse || o.category_scale.is_some(),
        "boxplot category reversal requires an explicit band profile",
    )?;
    if let Some(options) = &o.category_scale {
        options.validate()?;
    }
    ensure(data.len() <= 10000, "boxplot sample budget exceeded")?;
    ensure(
        o.quantiles
            .iter()
            .all(|v| v.is_finite() && (0. ..=1.).contains(v))
            && o.quantiles.windows(2).all(|p| p[0] <= p[1]),
        "invalid boxplot quantiles",
    )?;
    ensure(
        o.padding.is_finite()
            && (0. ..1.).contains(&o.padding)
            && o.inner_padding.is_finite()
            && (0. ..=16384.).contains(&o.inner_padding)
            && o.fill_opacity.is_finite()
            && (0. ..=1.).contains(&o.fill_opacity)
            && o.border_radius.is_finite()
            && o.border_radius >= 0.
            && o.cap_size.is_finite()
            && (0. ..=1.).contains(&o.cap_size)
            && o.outlier_radius.is_finite()
            && (0. ..=256.).contains(&o.outlier_radius),
        "invalid boxplot style",
    )?;
    if let BoxWhiskers::Tukey { factor } = o.whiskers {
        ensure(
            factor.is_finite() && (0. ..=100.).contains(&factor),
            "invalid Tukey factor",
        )?;
    }
    ensure(
        configured || kind != ScaleKind::Utc,
        "interpolated boxplot quantiles do not support integer UTC scale",
    )?;
    for stroke in [o.border, Some(o.median), Some(o.whisker), o.outlier_border] {
        stats_stroke_valid(stroke)?;
    }
    for profile in [
        &o.border_color_profile,
        &o.median_color_profile,
        &o.whisker_color_profile,
    ]
    .into_iter()
    .flatten()
    {
        profile.validate()?;
    }
    o.value_axis.validate()?;
    o.group_axis.validate()?;
    o.value_format.validate()?;
    let mut ids = BTreeSet::new();
    let mut gs = BTreeSet::new();
    let mut ss = BTreeSet::new();
    let mut cells: BTreeMap<(&str, &str), Vec<&BoxObservation>> = BTreeMap::new();
    for d in data {
        stats_id(&d.id, false)?;
        stats_id(&d.group, false)?;
        stats_id(&d.subgroup, true)?;
        ensure(ids.insert(&d.id), "duplicate boxplot observation ID")?;
        ensure(
            d.value.is_none_or(f64::is_finite),
            "boxplot samples must be finite",
        )?;
        if kind == ScaleKind::Utc {
            ensure(
                d.value
                    .is_none_or(|v| v.fract() == 0. && v.abs() <= MAX_TIMESTAMP_MS as f64),
                "boxplot UTC samples must be bounded integral epoch milliseconds",
            )?;
        }
        gs.insert(d.group.clone());
        ss.insert(d.subgroup.clone());
        if d.value.is_some() {
            cells.entry((&d.group, &d.subgroup)).or_default().push(d);
        }
    }
    let groups = ordered_keys(&o.groups, gs, false)?;
    let subgroups = ordered_keys(&o.subgroups, ss, true)?;
    ensure(
        groups.len().saturating_mul(subgroups.len()) <= 2048,
        "boxplot cell budget exceeded",
    )?;
    let color_keys = match o.color_by {
        BoxPlotColorBy::SubGroup => &subgroups,
        BoxPlotColorBy::Group => &groups,
    };
    ensure(
        o.colors.keys().all(|s| color_keys.contains(s)),
        "unknown boxplot color key",
    )?;
    if let Some(labels) = &o.legend_labels {
        ensure(
            labels.keys().all(|key| color_keys.contains(key)),
            "unknown boxplot legend label key",
        )?;
        ensure(
            labels.values().all(|label| label.len() <= MAX_LABEL_BYTES),
            "boxplot legend label exceeds byte budget",
        )?;
    }
    let visible = subgroups
        .iter()
        .filter(|s| !o.hidden.contains(&boxplot_series_id(s)))
        .collect::<Vec<_>>();
    let horizontal = o.orientation == StatisticsOrientation::Horizontal;
    let mut category_range = if horizontal {
        [c.margin.top, c.height - c.margin.bottom]
    } else {
        [c.margin.left, c.width - c.margin.right]
    };
    let band = if let Some(options) = &o.category_scale {
        if o.category_reverse {
            category_range.reverse();
        }
        crate::scales::BandScale::with_options(groups.clone(), category_range, options)?
    } else {
        crate::scales::BandScale::new(groups.clone(), category_range, o.padding)?
    };
    let breadth = if visible.is_empty() {
        0.
    } else {
        (band.bandwidth - o.inner_padding * (visible.len() - 1) as f64) / visible.len() as f64
    };
    ensure(
        visible.is_empty() || breadth > 0.,
        "boxplot subgroup gap exceeds group band",
    )?;
    let mut boxes = Vec::new();
    for group in &groups {
        for (si, sub) in visible.iter().enumerate() {
            let Some(v) = cells.get_mut(&(group.as_str(), sub.as_str())) else {
                continue;
            };
            if o.auto_domain_source == BoxPlotAutoDomainSource::SummaryQuantiles {
                v.sort_by(|a, b| {
                    // Every source value was validated finite above. Stable
                    // numeric ties preserve Nivo's original signed-zero order.
                    a.value.unwrap().partial_cmp(&b.value.unwrap()).unwrap()
                });
            } else {
                v.sort_by(|a, b| {
                    a.value
                        .unwrap()
                        .total_cmp(&b.value.unwrap())
                        .then(a.id.cmp(&b.id))
                });
            }
            let qs = o.quantiles.map(|p| {
                quantile(
                    v,
                    p,
                    o.auto_domain_source == BoxPlotAutoDomainSource::SummaryQuantiles,
                )
            });
            let extrema = [v[0].value.unwrap(), v[v.len() - 1].value.unwrap()];
            let magnitude = extrema[0].abs().max(extrema[1].abs());
            let mean = if magnitude == 0. {
                0.
            } else {
                (v.iter().map(|d| d.value.unwrap() / magnitude).sum::<f64>() / v.len() as f64)
                    .clamp(-1., 1.)
                    * magnitude
            };
            ensure(
                qs.into_iter().chain([mean]).all(f64::is_finite),
                "boxplot summary overflow",
            )?;
            let whiskers = match o.whiskers {
                BoxWhiskers::Quantiles => [qs[0], qs[4]],
                BoxWhiskers::Extrema => extrema,
                BoxWhiskers::Tukey { factor } => {
                    let delta = if factor == 0. {
                        0.
                    } else {
                        factor * (qs[3] - qs[1])
                    };
                    let lo = qs[1] - delta;
                    let hi = qs[3] + delta;
                    [
                        v.iter()
                            .find(|d| d.value.unwrap() >= lo)
                            .unwrap()
                            .value
                            .unwrap()
                            .min(qs[1]),
                        v.iter()
                            .rev()
                            .find(|d| d.value.unwrap() <= hi)
                            .unwrap()
                            .value
                            .unwrap()
                            .max(qs[3]),
                    ]
                }
            };
            let outliers = v
                .iter()
                .filter(|d| d.value.unwrap() < whiskers[0] || d.value.unwrap() > whiskers[1])
                .map(|d| BoxOutlier {
                    source_id: d.id.clone(),
                    value: d.value.unwrap(),
                })
                .collect();
            boxes.push(ComputedBox {
                id: format!("box:{}:{group}:{}:{sub}", group.len(), sub.len()),
                group: group.clone(),
                subgroup: (*sub).clone(),
                count: v.len(),
                extrema,
                quantiles: qs,
                mean,
                whiskers,
                outliers,
                source_ids: v.iter().map(|d| d.id.clone()).collect(),
                center: band.map(group).unwrap()
                    + si as f64 * (breadth + o.inner_padding)
                    + breadth / 2.,
                breadth,
                rect: ClipRect {
                    x: 0.,
                    y: 0.,
                    width: 0.,
                    height: 0.,
                },
            });
        }
    }
    let domain = if let Some(domain) = o.domain {
        domain
    } else if let Some(BoxPlotDomainBounds {
        min: Some(min),
        max: Some(max),
    }) = o.domain_bounds
    {
        [min, max]
    } else {
        let bounds = o.domain_bounds.unwrap_or_default();
        let mut inferred = if boxes.is_empty() && kind.requires_nonzero_domain() {
            let seed = bounds.min.or(bounds.max);
            if let Some(ticks) = o.value_axis.tick_values.as_ref().filter(|v| !v.is_empty()) {
                empty_nonzero_domain(ticks.iter().copied(), kind, seed)?
            } else {
                empty_nonzero_domain(data.iter().filter_map(|d| d.value), kind, seed)?
            }
        } else {
            match o.auto_domain_source {
                BoxPlotAutoDomainSource::SampleExtent => {
                    stats_extent(boxes.iter().flat_map(|b| b.extrema))
                }
                BoxPlotAutoDomainSource::SummaryQuantiles => {
                    stats_extent(boxes.iter().flat_map(|b| b.quantiles))
                }
            }
        };
        if o.auto_domain_source == BoxPlotAutoDomainSource::SummaryQuantiles
            && kind == ScaleKind::Utc
        {
            for (value, fixed) in inferred.iter_mut().zip([bounds.min, bounds.max]) {
                if fixed.is_none() {
                    ensure(
                        value.is_finite() && value.abs() <= MAX_TIMESTAMP_MS as f64,
                        "boxplot UTC automatic summary endpoint exceeds TimeClip range",
                    )?;
                    let clipped = value.trunc();
                    *value = if clipped == 0. { 0. } else { clipped };
                }
            }
        }
        if let Some(min) = bounds.min {
            inferred[0] = min;
        }
        if let Some(max) = bounds.max {
            inferred[1] = max;
        }
        inferred
    };
    let (scale, resolved) = resolve(
        domain,
        if horizontal {
            [c.margin.left, c.width - c.margin.right]
        } else {
            [c.height - c.margin.bottom, c.margin.top]
        },
    )?;
    let clip = ClipRect::plot(c);
    let centers = groups
        .iter()
        .map(|g| band.map(g).unwrap() + band.bandwidth / 2.)
        .collect::<Vec<_>>();
    crate::cartesian::bar_axes_positions(
        &mut scene,
        c,
        &groups,
        &centers,
        &scale,
        &BarOptions {
            horizontal,
            value_axis: o.value_axis.clone(),
            category_axis: o.group_axis.clone(),
            ..Default::default()
        },
    )?;
    let domain = scale.domain();
    let scale = scale.clamped(false);
    let range = scale.range();
    let outlier_format = if kind == ScaleKind::Utc && o.value_format == ValueFormat::Auto {
        &ValueFormat::Utc
    } else {
        &o.value_format
    };
    for b in &mut boxes {
        ensure(
            b.extrema.into_iter().all(|v| scale.accepts(v)),
            "boxplot values invalid for scale",
        )?;
        // Retain off-plot endpoints where their corners, strokes or outlier
        // radii can reach the clip. More distant endpoints can safely saturate
        // beyond that guard even if numeric extrapolation would overflow.
        let guard = breadth / 2. + 512.;
        let limits = [
            range[0].min(range[1]) - guard,
            range[0].max(range[1]) + guard,
        ];
        let coord = |v: f64| -> Result<f64> {
            let p = if !configured {
                scale.map(v)
            } else if kind == ScaleKind::Utc {
                statistic_pixel(&scale, v).clamp(limits[0], limits[1])
            } else {
                crate::cartesian::bounded_bar_pixel(&scale, v, limits)
            };
            ensure(
                p.is_finite() && p.abs() <= 16_777_216.,
                "boxplot projection exceeds coordinate profile",
            )?;
            Ok(p)
        };
        let a = coord(b.quantiles[1])?;
        let z = coord(b.quantiles[3])?;
        b.rect = if horizontal {
            ClipRect {
                x: a.min(z),
                y: b.center - breadth / 2.,
                width: (a - z).abs(),
                height: breadth,
            }
        } else {
            ClipRect {
                x: b.center - breadth / 2.,
                y: a.min(z),
                width: breadth,
                height: (a - z).abs(),
            }
        };
        let color_key = match o.color_by {
            BoxPlotColorBy::SubGroup => &b.subgroup,
            BoxPlotColorBy::Group => &b.group,
        };
        let palette = color_keys.iter().position(|s| s == color_key).unwrap();
        let color = source_colors
            .and_then(|colors| colors.get(&b.id))
            .copied()
            .unwrap_or_else(|| {
                o.colors
                    .get(color_key)
                    .copied()
                    .unwrap_or(c.theme.color(palette))
            });
        // Only actual visible summary boxes have paint contexts. Empty/hidden
        // slots never fabricate one, while collapsed statistics and disabled
        // strokes still validate this real box's chosen role contexts.
        let role_color = |profile: &Option<InheritedColor>, theme| {
            profile.as_ref().map_or(Ok(theme), |profile| {
                profile.resolve_validated(InheritedColorContext {
                    inherited: color,
                    theme,
                    surface: color,
                    backdrop: c.theme.background,
                })
            })
        };
        let border_color = role_color(
            &o.border_color_profile,
            o.border.map_or(color, |stroke| stroke.color),
        )?;
        let border = o.border.map(|stroke| GraphicStroke {
            color: border_color,
            ..stroke
        });
        let median = GraphicStroke {
            color: role_color(&o.median_color_profile, o.median.color)?,
            ..o.median
        };
        let whisker = GraphicStroke {
            color: role_color(&o.whisker_color_profile, o.whisker.color)?,
            ..o.whisker
        };
        let label = if b.subgroup.is_empty() {
            b.group.clone()
        } else {
            format!("{} · {}", b.group, b.subgroup)
        };
        let mut mark = Mark::new(b.id.clone(), box_shape(b.rect, o.border_radius), color);
        mark.series = boxplot_series_id(&b.subgroup);
        mark.value = b.quantiles[2];
        mark.label = label.clone();
        mark.opacity = o.fill_opacity;
        mark.clip = Some(clip);
        // Separate border keeps fill opacity from fading the outline.
        scene.marks.push(mark);
        if let Some(border) = border.filter(|stroke| stroke.width > 0.) {
            let mut m = Mark::new(
                format!("{}:border", b.id),
                scene.marks.last().unwrap().geometry.clone(),
                color,
            );
            m.fill = None;
            m.stroke = Some(border);
            m.series = boxplot_series_id(&b.subgroup);
            m.label.clear();
            m.clip = Some(clip);
            scene.marks.push(m);
        }
        let point = |value: f64, offset: f64| -> Result<[f64; 2]> {
            let v = coord(value)?;
            Ok(if horizontal {
                [v, b.center + offset]
            } else {
                [b.center + offset, v]
            })
        };
        for (part, pairs, stroke) in [
            (
                "median",
                vec![(
                    point(b.quantiles[2], -breadth / 2.)?,
                    point(b.quantiles[2], breadth / 2.)?,
                )],
                median,
            ),
            (
                "whiskers",
                vec![
                    (point(b.whiskers[0], 0.)?, point(b.quantiles[1], 0.)?),
                    (point(b.quantiles[3], 0.)?, point(b.whiskers[1], 0.)?),
                ],
                whisker,
            ),
            (
                "caps",
                vec![
                    (
                        point(b.whiskers[0], -breadth * o.cap_size / 2.)?,
                        point(b.whiskers[0], breadth * o.cap_size / 2.)?,
                    ),
                    (
                        point(b.whiskers[1], -breadth * o.cap_size / 2.)?,
                        point(b.whiskers[1], breadth * o.cap_size / 2.)?,
                    ),
                ],
                whisker,
            ),
        ] {
            if stroke.width == 0. || (part == "caps" && o.cap_size == 0.) {
                continue;
            }
            let mut commands = Vec::new();
            for (p, q) in pairs {
                commands.extend([C::Move(P::new(p[0], p[1])), C::Line(P::new(q[0], q[1]))]);
            }
            let mut m = Mark::new(format!("{}:{part}", b.id), Geometry::Path(commands), color);
            m.fill = None;
            m.stroke = Some(stroke);
            m.series = boxplot_series_id(&b.subgroup);
            if part == "median" {
                m.value = b.quantiles[2];
                m.label = format!("{label} · median");
            } else {
                m.label.clear();
            }
            m.clip = Some(clip);
            scene.marks.push(m);
        }
        if o.outlier_radius > 0. {
            for d in &b.outliers {
                let p = point(d.value, 0.)?;
                let mut m = Mark::new(
                    format!("{}:outlier:{}:{}", b.id, d.source_id.len(), d.source_id),
                    outlier_shape(p, o.outlier_radius, o.outlier_shape),
                    o.outlier_fill.unwrap_or(color),
                );
                m.stroke = o.outlier_border.filter(|stroke| stroke.width > 0.);
                m.series = boxplot_series_id(&b.subgroup);
                m.value = d.value;
                m.label = format!(
                    "{} · {}: outlier {}",
                    b.group,
                    b.subgroup,
                    outlier_format.format(d.value)?
                );
                m.clip = Some(clip);
                scene.marks.push(m);
            }
        }
        ensure(
            scene.marks.len() <= MAX_MARKS,
            "boxplot mark budget exceeded",
        )?;
    }
    if o.legend_mode == BoxPlotLegendMode::ByLabel {
        // The callback cannot observe an otherwise invalid source, scale,
        // projection, paint, axis, or pre-existing resource-budget frame.
        scene.validate()?;
        legend::build(
            &mut scene,
            c,
            data,
            o,
            &boxes,
            &groups,
            &subgroups,
            legend_factory,
            source_colors,
        )?;
    } else if o.legend {
        // Only the opt-in color path allocates this first-summary index.
        // Keys with no actual summary keep the legacy reserved palette row.
        let mut first_colors = BTreeMap::new();
        if let Some(colors) = source_colors {
            for summary in &boxes {
                let key = match o.color_by {
                    BoxPlotColorBy::SubGroup => &summary.subgroup,
                    BoxPlotColorBy::Group => &summary.group,
                };
                first_colors
                    .entry(key.as_str())
                    .or_insert(colors[&summary.id]);
            }
        }
        match o.color_by {
            BoxPlotColorBy::SubGroup => {
                for (i, s) in subgroups.iter().enumerate() {
                    scene.legend.push(LegendItem {
                        id: boxplot_series_id(s),
                        label: o
                            .legend_labels
                            .as_ref()
                            .and_then(|labels| labels.get(s))
                            .cloned()
                            .unwrap_or_else(|| {
                                if s.is_empty() {
                                    "Samples".into()
                                } else {
                                    s.clone()
                                }
                            }),
                        color: first_colors.get(s.as_str()).copied().unwrap_or_else(|| {
                            o.colors.get(s).copied().unwrap_or(c.theme.color(i))
                        }),
                    });
                }
            }
            BoxPlotColorBy::Group => {
                for (i, group) in groups.iter().enumerate() {
                    if !boxes.iter().any(|b| &b.group == group) {
                        continue;
                    }
                    scene.legend.push(LegendItem {
                        id: group.clone(),
                        label: o
                            .legend_labels
                            .as_ref()
                            .and_then(|labels| labels.get(group))
                            .cloned()
                            .unwrap_or_else(|| group.clone()),
                        color: first_colors
                            .get(group.as_str())
                            .copied()
                            .unwrap_or_else(|| {
                                o.colors.get(group).copied().unwrap_or(c.theme.color(i))
                            }),
                    });
                    scene
                        .legend_roles
                        .insert(group.clone(), LegendRole::Category);
                }
            }
        }
    }
    scene.validate()?;
    Ok((
        BoxPlotLayout {
            scene,
            boxes,
            groups,
            subgroups,
            domain,
            category_scale: o.category_scale.as_ref().map(|_| band),
        },
        resolved,
    ))
}
