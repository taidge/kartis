//! Nivo-style radial bars: each input series owns a radial ring, categories
//! accumulate along its angular value scale. Local reference: radial-bar/src/
//! {types.ts,hooks.ts,props.ts}; the older radial_bar/polar_bar APIs are unchanged.
use crate::{pie_layout::pie_arc, scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::TAU,
    sync::Arc,
};
pub const MAX_RADIAL_BAR_CELLS: usize = 8192;
#[derive(Clone, Debug, PartialEq)]
pub enum RadialBarLabelText {
    Value(ValueFormat),
    Percent(ValueFormat),
    Category,
    Ring,
}
impl Default for RadialBarLabelText {
    fn default() -> Self {
        Self::Value(ValueFormat::Auto)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadialBarLabels {
    pub text: RadialBarLabelText,
    pub skip_angle: f64,
    /// Suppress labels whose selected radius measure is smaller than this.
    pub skip_radius: f64,
    pub radius_measure: ArcLabelRadiusMeasure,
    pub radius_offset: f64,
    pub color: Option<Color>,
    /// None keeps the fixed color/theme fallback and the original label alpha.
    /// Some resolves a shared color role from the original category palette;
    /// custom paints do not replace inherited/surface. Theme uses `color` or
    /// theme.text, and backdrop is theme.background. Labels remain independent
    /// of the segment's `StackedRadialBarOptions::opacity`.
    pub color_profile: Option<InheritedColor>,
    pub size: Option<f64>,
    pub rotation: f64,
}
impl Default for RadialBarLabels {
    fn default() -> Self {
        Self {
            text: Default::default(),
            skip_angle: 0.,
            skip_radius: 0.,
            radius_measure: Default::default(),
            radius_offset: 0.5,
            color: None,
            color_profile: None,
            size: None,
            rotation: 0.,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackedRadialBarOptions {
    pub inner_radius: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    /// D3 band padding, used as both inner and outer ring padding.
    pub padding: f64,
    pub pad_angle: f64,
    pub max_value: Option<f64>,
    pub tracks: bool,
    pub track_color: Option<Color>,
    pub border: Option<GraphicStroke>,
    /// None keeps the fixed segment stroke. Some recolors an existing stroke
    /// from the pre-paint category palette (inherited/surface), its fixed
    /// color (Theme) and chart background (backdrop). Without a border the
    /// unused Theme fallback is the palette; no stroke is enabled. Tracks are
    /// independent. Real Some sources preflight even when zero or hidden.
    pub border_color_profile: Option<InheritedColor>,
    pub opacity: f64,
    pub value_format: ValueFormat,
    pub colors: BTreeMap<String, Color>,
    pub paints: BTreeMap<String, FillPaint>,
    /// Categories lose ink/owned labels, retaining the full domain and angular
    /// stack slots. This agrees with BrowserChart's default legend filtering.
    pub hidden: BTreeSet<String>,
    pub labels: Option<RadialBarLabels>,
    pub legend: bool,
    pub axes: PolarAxesOptions,
    pub value_ticks: PolarTickOptions,
}
impl Default for StackedRadialBarOptions {
    fn default() -> Self {
        Self {
            inner_radius: 0.3,
            start_angle: 0.,
            end_angle: TAU * 0.75,
            padding: 0.2,
            pad_angle: 0.,
            max_value: None,
            tracks: true,
            track_color: None,
            border: None,
            border_color_profile: None,
            opacity: 1.,
            value_format: ValueFormat::Auto,
            colors: Default::default(),
            paints: Default::default(),
            hidden: Default::default(),
            labels: None,
            legend: true,
            axes: Default::default(),
            value_ticks: Default::default(),
        }
    }
}
/// D3 arc geometry for the segments and tracks of a stacked radial bar chart.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StackedRadialArcStyle {
    /// Requested corner radius in chart pixels, clamped by the ring and sector.
    pub corner_radius: f64,
    /// Radius used to turn the chart's pad angle into a gap between segments.
    pub pad_radius: PiePadRadius,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackedRadialRing {
    pub id: String,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub total: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackedRadialSegment {
    pub mark_id: String,
    pub ring_id: String,
    pub category: String,
    pub value: f64,
    pub cumulative: [f64; 2],
    pub angles: [f64; 2],
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub visible: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct StackedRadialBarLayout {
    pub scene: Scene,
    pub frame: PolarFrame,
    pub domain: [f64; 2],
    pub rings: Vec<StackedRadialRing>,
    pub segments: Vec<StackedRadialSegment>,
    interaction_source: Arc<RadialInteractionSource>,
}
#[derive(Clone, Debug, PartialEq)]
struct RadialInteractionSource {
    frame: PolarFrame,
    domain: [f64; 2],
    rings: Vec<StackedRadialRing>,
    segments: Vec<StackedRadialSegment>,
    pad_angle: f64,
    arc_style: Option<StackedRadialArcStyle>,
    value_format: ValueFormat,
}
impl StackedRadialBarLayout {
    /// Bind visible original segments to their actual arc Paths. Tracks, axes
    /// and labels stay decorative. Metadata and datum geometry edits are rejected.
    /// Compatible live motion can resolve the returned anchors from its Paths.
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        self.scene.validate()?;
        let source = &self.interaction_source;
        ensure(
            self.frame == source.frame
                && self.domain == source.domain
                && self.rings == source.rings
                && self.segments == source.segments,
            "radial interaction source metadata was changed",
        )?;
        let marks = self
            .scene
            .marks
            .iter()
            .map(|mark| (mark.id.as_str(), mark))
            .collect::<BTreeMap<_, _>>();
        let mut binding = InteractionBinding::default();
        for segment in &self.segments {
            if !segment.visible {
                ensure(
                    !marks.contains_key(segment.mark_id.as_str()),
                    "hidden radial source gained a datum mark",
                )?;
                continue;
            }
            let mark = marks
                .get(segment.mark_id.as_str())
                .ok_or_else(|| ChartError("missing radial interaction mark".into()))?;
            let (geometry, has_area) = if let Some(style) = source.arc_style {
                let arc = styled_annular(
                    &source.frame,
                    segment.inner_radius,
                    segment.outer_radius,
                    segment.angles,
                    source.pad_angle,
                    &style,
                )?;
                (arc.geometry, !arc.outer_collapsed)
            } else {
                let direction = source.frame.span().signum();
                (
                    annular(
                        &source.frame,
                        segment.inner_radius,
                        segment.outer_radius,
                        segment.angles[0] + direction * source.pad_angle / 2.,
                        segment.angles[1] - direction * source.pad_angle / 2.,
                    ),
                    true,
                )
            };
            let formatted = source.value_format.format(segment.value)?;
            let label = format!("{} · {}: {}", segment.ring_id, segment.category, formatted);
            ensure(
                mark.series == segment.category
                    && mark.value == segment.value
                    && mark.label == label
                    && mark.geometry == geometry,
                "radial interaction mark disagrees with its source",
            )?;
            let angle = segment.angles[0] + (segment.angles[1] - segment.angles[0]) / 2.;
            let position = self
                .frame
                .point((segment.inner_radius + segment.outer_radius) / 2., angle);
            if let Some((site, anchor)) =
                crate::pie_layout::arc_interaction(mark, position, has_area, Some(formatted))?
            {
                binding.anchors.insert(mark.id.clone(), anchor);
                binding.sites.push(site);
            }
        }
        binding.validate(&self.scene)?;
        Ok(binding)
    }

    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        Ok(SceneInteraction {
            interaction: Some(self.interaction_binding()?),
            scene: self.scene,
            plot: None,
        })
    }
}
fn valid(id: &str) -> bool {
    !id.trim().is_empty() && id.len() <= 256
}
fn annular(f: &PolarFrame, inner: f64, outer: f64, start: f64, end: f64) -> Geometry {
    let p = |r, a| {
        let p = f.point(r, a);
        P::new(p[0], p[1])
    };
    let mut out = vec![C::Move(p(outer, start))];
    crate::polar_axes::arc_commands(f.center, outer, start, end, &mut out);
    if ((end - start).abs() - TAU).abs() <= f.turn_tolerance() {
        out.push(C::Close);
        if inner > 0. {
            out.push(C::Move(p(inner, end)));
            crate::polar_axes::arc_commands(f.center, inner, end, start, &mut out);
        }
    } else {
        out.push(C::Line(p(inner, end)));
        if inner > 0. {
            crate::polar_axes::arc_commands(f.center, inner, end, start, &mut out);
        }
    }
    out.push(C::Close);
    Geometry::Path(out)
}
fn styled_annular(
    frame: &PolarFrame,
    inner: f64,
    outer: f64,
    [start, end]: [f64; 2],
    pad: f64,
    style: &StackedRadialArcStyle,
) -> Result<pie_arc::ArcOutput> {
    pie_arc::build(pie_arc::ArcInput {
        center: frame.center,
        inner,
        outer,
        start,
        end,
        pad,
        pad_radius: match style.pad_radius {
            PiePadRadius::Auto => None,
            PiePadRadius::Fixed(radius) => Some(radius),
        },
        corner: style.corner_radius,
    })
}
pub fn stacked_radial_bar(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &StackedRadialBarOptions,
) -> Result<Scene> {
    Ok(layout_stacked_radial_bar(c, categories, series, o)?.scene)
}
pub fn layout_stacked_radial_bar(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &StackedRadialBarOptions,
) -> Result<StackedRadialBarLayout> {
    layout(c, categories, series, o, None)
}
/// Build a scene with rounded D3 arcs and radius-aware padding.
pub fn stacked_radial_bar_with_arc_style(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &StackedRadialBarOptions,
    style: &StackedRadialArcStyle,
) -> Result<Scene> {
    Ok(layout_stacked_radial_bar_with_arc_style(c, categories, series, o, style)?.scene)
}
/// Lay out a stacked radial bar chart using D3 arc geometry.
///
/// Segment angles remain the untrimmed value-scale intervals. The padding and
/// corner radius are applied to the mark paths for both tracks and segments.
/// A segment collapsed by padding contributes ink only through a visible border.
pub fn layout_stacked_radial_bar_with_arc_style(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &StackedRadialBarOptions,
    style: &StackedRadialArcStyle,
) -> Result<StackedRadialBarLayout> {
    ensure(
        (0. ..=16384.).contains(&style.corner_radius)
            && match style.pad_radius {
                PiePadRadius::Auto => true,
                PiePadRadius::Fixed(radius) => (0. ..=1e6).contains(&radius),
            },
        "invalid radial arc style",
    )?;
    layout(c, categories, series, o, Some(style))
}
fn layout(
    c: &ChartConfig,
    categories: &[String],
    series: &[BarSeries],
    o: &StackedRadialBarOptions,
    style: Option<&StackedRadialArcStyle>,
) -> Result<StackedRadialBarLayout> {
    let mut scene = c.scene()?;
    let frame = PolarFrame::from_config(c, o.inner_radius, o.start_angle, o.end_angle)?;
    o.axes.validate()?;
    o.value_ticks.validate()?;
    o.value_format.validate()?;
    ensure(
        categories.len() <= 256
            && series.len() <= 256
            && categories
                .len()
                .checked_mul(series.len())
                .is_some_and(|n| n <= MAX_RADIAL_BAR_CELLS),
        "stacked radial matrix budget exceeded",
    )?;
    let mut seen = BTreeSet::new();
    ensure(
        categories.iter().all(|id| valid(id) && seen.insert(id)),
        "invalid or duplicate radial category",
    )?;
    seen.clear();
    ensure(
        series.iter().all(|s| {
            valid(&s.id)
                && seen.insert(&s.id)
                && s.values.len() == categories.len()
                && s.values.iter().flatten().all(|v| v.is_finite() && *v >= 0.)
        }),
        "invalid radial ring IDs or values",
    )?;
    let ids = categories.iter().collect::<BTreeSet<_>>();
    ensure(
        o.colors
            .keys()
            .chain(o.paints.keys())
            .chain(o.hidden.iter())
            .all(|id| ids.contains(id))
            && o.colors.len() <= 256
            && o.paints.len() <= 256
            && o.hidden.len() <= 256,
        "unknown or oversized radial category style",
    )?;
    ensure(
        o.padding.is_finite()
            && (0. ..1.).contains(&o.padding)
            && o.pad_angle.is_finite()
            && (0. ..=frame.span().abs()).contains(&o.pad_angle)
            && o.opacity.is_finite()
            && (0. ..=1.).contains(&o.opacity)
            && o.border
                .is_none_or(|b| b.width.is_finite() && (0.000001..=10000.).contains(&b.width)),
        "invalid stacked radial style",
    )?;
    if let Some(profile) = &o.border_color_profile {
        profile.validate()?;
    }
    for paint in o.paints.values() {
        paint.validate()?;
    }
    if let Some(l) = &o.labels {
        // Intrinsic validation applies even without emitted/visible labels.
        if let Some(profile) = &l.color_profile {
            profile.validate()?;
        }
        match &l.text {
            RadialBarLabelText::Value(f) | RadialBarLabelText::Percent(f) => f.validate()?,
            _ => {}
        }
        ensure(
            l.skip_angle.is_finite()
                && (0. ..=TAU).contains(&l.skip_angle)
                && l.skip_radius.is_finite()
                && (0. ..=16384.).contains(&l.skip_radius)
                && l.radius_offset.is_finite()
                && (0. ..=1.).contains(&l.radius_offset)
                && l.size
                    .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v))
                && l.rotation.is_finite()
                && (-360. ..=360.).contains(&l.rotation),
            "invalid radial labels",
        )?;
    }
    let totals = series
        .iter()
        .map(|s| {
            s.values.iter().flatten().try_fold(0., |sum, v| {
                let total = sum + v;
                ensure(total.is_finite(), "radial ring total overflow")?;
                Ok(total)
            })
        })
        .collect::<Result<Vec<f64>>>()?;
    let source_max = totals.iter().copied().fold(0., f64::max);
    let maximum = o
        .max_value
        .unwrap_or(if source_max > 0. { source_max } else { 1. });
    ensure(
        maximum.is_finite() && maximum > 0. && maximum >= source_max,
        "radial maximum must include every ring total",
    )?;
    let scale = ChartScale::new(
        ScaleKind::Linear,
        [0., maximum],
        [frame.start_angle, frame.end_angle],
    )?;
    let ring_ids = series.iter().map(|s| s.id.clone()).collect::<Vec<_>>();
    let bands = scales::BandScale::new(
        ring_ids,
        [frame.inner_radius, frame.outer_radius],
        o.padding,
    )?;
    let colors = categories
        .iter()
        .enumerate()
        .map(|(i, id)| {
            (
                id.clone(),
                o.colors.get(id).copied().unwrap_or(c.theme.color(i)),
            )
        })
        .collect::<BTreeMap<_, _>>();
    // Keep the None path free of new palette-context resolution. For Some,
    // preflight each actual Some source category, including hidden, zero and
    // threshold-skipped values. A sparse absent or truly empty source does not
    // fabricate a Contrast context. Mark opacity never changes this role.
    let label_colors = if let Some(l) = &o.labels
        && let Some(profile) = &l.color_profile
    {
        Some(
            categories
                .iter()
                .enumerate()
                .filter(|(index, _)| series.iter().any(|s| s.values[*index].is_some()))
                .map(|(_, category)| {
                    let inherited = colors[category];
                    let color = profile.resolve_validated(InheritedColorContext {
                        inherited,
                        theme: l.color.unwrap_or(c.theme.text),
                        surface: inherited,
                        backdrop: c.theme.background,
                    })?;
                    Ok((category.clone(), color))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        )
    } else {
        None
    };
    // ArcsLayer resolves numeric datum.color before a defs/paint replaces fill.
    // Mirror that source qualification for typed actual Some values, including
    // zero, hidden and pad-collapsed segments. Empty/all-None categories have
    // no invented Contrast context. Keep None free of new context resolution.
    let segment_borders = if let Some(profile) = &o.border_color_profile {
        Some(
            categories
                .iter()
                .enumerate()
                .filter(|(index, _)| series.iter().any(|s| s.values[*index].is_some()))
                .map(|(_, category)| {
                    let inherited = colors[category];
                    let color = profile.resolve_validated(InheritedColorContext {
                        inherited,
                        theme: o.border.map_or(inherited, |border| border.color),
                        surface: inherited,
                        backdrop: c.theme.background,
                    })?;
                    Ok((
                        category.clone(),
                        o.border.map(|border| GraphicStroke { color, ..border }),
                    ))
                })
                .collect::<Result<BTreeMap<_, _>>>()?,
        )
    } else {
        None
    };
    if o.legend {
        scene.legend = categories
            .iter()
            .map(|id| LegendItem {
                id: id.clone(),
                label: id.clone(),
                color: colors[id],
            })
            .collect();
    }
    let mut rings = vec![];
    let mut segments = vec![];
    let mut radial_ticks = vec![];
    for (si, s) in series.iter().enumerate() {
        let inner = bands.map(&s.id).unwrap();
        let outer = inner + bands.bandwidth;
        ensure(
            inner.is_finite() && outer.is_finite() && outer > inner,
            "radial ring bandwidth collapsed",
        )?;
        rings.push(StackedRadialRing {
            id: s.id.clone(),
            inner_radius: inner,
            outer_radius: outer,
            total: totals[si],
        });
        radial_ticks.push(PolarTick::new(&s.id, &s.id, (inner + outer) / 2.));
        if o.tracks {
            let geometry = if let Some(style) = style {
                let arc = styled_annular(
                    &frame,
                    inner,
                    outer,
                    [frame.start_angle, frame.end_angle],
                    o.pad_angle,
                    style,
                )?;
                (!arc.outer_collapsed).then_some(arc.geometry)
            } else {
                Some(annular(
                    &frame,
                    inner,
                    outer,
                    frame.start_angle,
                    frame.end_angle,
                ))
            };
            if let Some(geometry) = geometry {
                let mut m = Mark::new(
                    format!("radial-track:{}:{}", s.id.len(), s.id),
                    geometry,
                    o.track_color.unwrap_or(c.theme.grid),
                );
                m.label.clear();
                scene.marks.push(m);
            }
        }
        let mut sum = 0.;
        for (ci, value) in s.values.iter().enumerate() {
            let Some(value) = *value else {
                continue;
            };
            let border = segment_borders
                .as_ref()
                .map_or(o.border, |borders| borders[&categories[ci]]);
            let base = sum;
            sum += value;
            let start = scale.map(base);
            let end = scale.map(sum);
            let extent = (end - start).abs();
            ensure(
                value == 0. || sum > base && extent > 0. && extent.is_finite(),
                "positive radial segment projection is not representable",
            )?;
            let path = if value == 0. || o.hidden.contains(&categories[ci]) {
                None
            } else if let Some(style) = style {
                let arc = styled_annular(&frame, inner, outer, [start, end], o.pad_angle, style)?;
                let has_area = !arc.outer_collapsed;
                let has_stroke = o.opacity > 0. && border.is_some_and(|border| border.color.3 > 0);
                (has_area || has_stroke).then_some((arc.geometry, has_area))
            } else if extent > o.pad_angle {
                let direction = frame.span().signum();
                let a = start + direction * o.pad_angle / 2.;
                let b = end - direction * o.pad_angle / 2.;
                Some((annular(&frame, inner, outer, a, b), true))
            } else {
                None
            };
            let mark_id = format!(
                "radial-segment:{}:{}:{}:{}",
                s.id.len(),
                s.id,
                categories[ci].len(),
                categories[ci]
            );
            segments.push(StackedRadialSegment {
                mark_id: mark_id.clone(),
                ring_id: s.id.clone(),
                category: categories[ci].clone(),
                value,
                cumulative: [base, sum],
                angles: [start, end],
                inner_radius: inner,
                outer_radius: outer,
                visible: path.is_some(),
            });
            let Some((geometry, has_area)) = path else {
                continue;
            };
            let mut m = Mark::new(mark_id, geometry, colors[&categories[ci]]);
            m.series = categories[ci].clone();
            m.value = value;
            m.label = format!(
                "{} · {}: {}",
                s.id,
                categories[ci],
                o.value_format.format(value)?
            );
            m.stroke = border;
            if has_area {
                m.fill_paint = o.paints.get(&categories[ci]).cloned();
            } else {
                m.fill = None;
            }
            m.opacity = o.opacity;
            scene.marks.push(m);
            if let Some(l) = &o.labels
                && has_area
                && extent >= l.skip_angle
                && l.radius_measure.measure(inner, outer) >= l.skip_radius
            {
                let text = match &l.text {
                    RadialBarLabelText::Value(f) => f.format(value)?,
                    RadialBarLabelText::Percent(f) => f.format(value / totals[si])?,
                    RadialBarLabelText::Category => categories[ci].clone(),
                    RadialBarLabelText::Ring => s.id.clone(),
                };
                let p = frame.point(
                    inner + (outer - inner) * l.radius_offset,
                    (start + end) / 2.,
                );
                let index = scene.labels.len();
                scene.label_series.insert(index, categories[ci].clone());
                scene.labels.push(Label {
                    x: p[0],
                    y: p[1],
                    text,
                    size: l.size.unwrap_or(c.theme.font_size),
                    color: label_colors.as_ref().map_or_else(
                        || l.color.unwrap_or(c.theme.text),
                        |colors| colors[&categories[ci]],
                    ),
                    anchor: TextAnchor::Middle,
                    rotation: l.rotation,
                });
            }
        }
    }
    let circular = polar_numeric_ticks(&scale, &o.value_ticks)?;
    append_polar_axes(&mut scene, c, &frame, &radial_ticks, &circular, &o.axes)?;
    scene.validate()?;
    let interaction_source = Arc::new(RadialInteractionSource {
        frame: frame.clone(),
        domain: [0., maximum],
        rings: rings.clone(),
        segments: segments.clone(),
        pad_angle: o.pad_angle,
        arc_style: style.copied(),
        value_format: o.value_format.clone(),
    });
    Ok(StackedRadialBarLayout {
        scene,
        frame,
        domain: [0., maximum],
        rings,
        segments,
        interaction_source,
    })
}
