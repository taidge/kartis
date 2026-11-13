//! A typed single-Scene-mark extension, not a React/Widget component protocol.
use super::*;
pub use crate::ScatterNodeGeometry as BarDatumGeometry;
use crate::{
    FillPaint, InteractionAnchor, InteractionBinding, InteractionSite, ScatterNodeGeometry,
};
/// Borrowed source plus the resolved ordinary bar layout, before custom ink.
/// None sources, hidden series and fully cropped bars do not invoke factories.
#[derive(Clone, Copy, Debug)]
pub struct BarDatumContext<'a> {
    pub series: &'a BarSeries,
    pub category: &'a str,
    pub mark_id: &'a str,
    pub source_series: usize,
    pub source_category: usize,
    pub raw_value: f64,
    pub displayed_value: f64,
    pub interval: [f64; 2],
    pub band: [f64; 2],
    pub visible_rect: ClipRect,
    pub original_rect: Option<ClipRect>,
    /// Unclamped value endpoint projections; None means outside finite f64.
    pub endpoints: [Option<f64>; 2],
    /// Fixed ordinary Rect center, or visible-rect center for an ordinary Path.
    /// Zero-area rounded Rect placeholders retain their authored center through
    /// category clipping. This site is fixed before the custom callback.
    /// original_rect and endpoints separately preserve the uncropped layout.
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub geometry: &'a Geometry,
    pub color: Color,
    pub stroke: Option<glory::render::GraphicStroke>,
    pub horizontal: bool,
}
impl BarDatumContext<'_> {
    pub fn inherit(&self, geometry: Geometry) -> BarDatumGeometry {
        ScatterNodeGeometry {
            geometry,
            fill: Some(self.color),
            fill_paint: None,
            stroke: self.stroke,
            stroke_dash: None,
            opacity: 1.,
            anchor_commands: None,
        }
    }
}
pub type BarGeometryFactory<'a> = dyn FnMut(&BarDatumContext<'_>) -> Result<BarDatumGeometry> + 'a;

/// Original source and pure ordinary placement for one eligible value label.
/// Geometry/text factories do not change raw values, tooltip formatting or this site.
#[derive(Clone, Copy, Debug)]
pub struct BarLabelContext<'a> {
    pub series: &'a BarSeries,
    pub category: &'a str,
    pub mark_id: &'a str,
    pub source_series: usize,
    pub source_category: usize,
    pub raw_value: f64,
    pub displayed_value: f64,
    pub interval: [f64; 2],
    pub default_text: &'a str,
    pub position: [f64; 2],
    pub label_position: [f64; 2],
    pub plot: ClipRect,
}
pub type BarLabelFactory<'a> = dyn FnMut(&BarLabelContext<'_>) -> Result<String> + 'a;

/// Borrowed callbacks used only during construction. With a label factory,
/// eligible label text runs before bar geometry, in the ordinary source order.
/// Empty text retains its owned slot; results have the Scene 1024-byte limit.
/// Failures reject the frame, but external callback side effects are not undone.
/// With label=None, the legacy geometry factory path and error order are used.
#[derive(Default)]
pub struct BarFactories<'a> {
    pub bar: Option<&'a mut BarGeometryFactory<'a>>,
    pub label: Option<&'a mut BarLabelFactory<'a>>,
}

pub(super) struct FactoryState<'a> {
    factory: Option<&'a mut BarGeometryFactory<'a>>,
    anchors: BTreeMap<String, InteractionAnchor>,
    positions: BTreeMap<String, [f64; 2]>,
    colors: BTreeMap<String, Color>,
    commands: usize,
    stops: usize,
    dash_commands: usize,
    dash_segments: usize,
    dash_work: usize,
    pub label_colors: BTreeMap<usize, Color>,
}
impl<'a> FactoryState<'a> {
    fn new(factory: &'a mut BarGeometryFactory<'a>) -> Self {
        Self::optional(Some(factory))
    }
    fn optional(factory: Option<&'a mut BarGeometryFactory<'a>>) -> Self {
        Self {
            factory,
            anchors: Default::default(),
            positions: Default::default(),
            colors: Default::default(),
            commands: 0,
            stops: 0,
            dash_commands: 0,
            dash_segments: 0,
            dash_work: 0,
            label_colors: Default::default(),
        }
    }
    pub fn count_scene(&mut self, scene: &Scene) -> Result<()> {
        self.count_base_scene(scene, &BTreeSet::new())
    }
    fn count_base_scene(&mut self, scene: &Scene, source_ids: &BTreeSet<&str>) -> Result<()> {
        for m in &scene.marks {
            if source_ids.contains(m.id.as_str()) {
                continue;
            }
            self.commands += command_count(&m.geometry);
            self.stops += paint_stops(m.fill_paint.as_ref());
            self.admit_dash(m)?;
        }
        self.budget()
    }
    fn budget(&self) -> Result<()> {
        ensure(
            self.commands <= 200_000 && self.stops <= crate::MAX_SCENE_GRADIENT_STOPS,
            "custom Bar scene geometry/paint budget exceeded",
        )
    }
    fn admit_dash(&mut self, mark: &Mark) -> Result<()> {
        if let Some(dash) = &mark.stroke_dash {
            let limits = crate::StrokeDashLimits {
                max_output_commands: crate::MAX_DASH_COMMANDS - self.dash_commands,
                max_flattened_segments: crate::MAX_DASH_COMMANDS - self.dash_segments,
                max_work: crate::MAX_DASH_WORK.saturating_sub(self.dash_work),
                ..Default::default()
            };
            let d = crate::dash_geometry_detailed(&mark.geometry, dash, &limits)?;
            self.dash_commands += d.geometry.commands().len();
            self.dash_segments += d.flattened_segments;
            self.dash_work += d.work;
        }
        Ok(())
    }
    pub fn prepare(&mut self, c: &BarDatumContext<'_>) -> Result<Mark> {
        let node = if let Some(factory) = self.factory.as_deref_mut() {
            factory(c)?
        } else {
            c.inherit(c.geometry.clone())
        };
        ensure(
            node.opacity.is_finite() && (0. ..=1.).contains(&node.opacity),
            "custom Bar opacity must be in 0..=1",
        )?;
        self.commands = self
            .commands
            .checked_add(command_count(&node.geometry))
            .ok_or_else(|| ChartError("custom Bar commands overflow".into()))?;
        self.stops = self
            .stops
            .checked_add(paint_stops(node.fill_paint.as_ref()))
            .ok_or_else(|| ChartError("custom Bar paint stops overflow".into()))?;
        self.budget()?;
        if let Some(paint) = &node.fill_paint {
            paint.validate()?;
        }
        if let Some(dash) = &node.stroke_dash {
            dash.validate()?;
        }
        let prepared = node.prepare_at(c.mark_id, c.position, c.color, 200_000)?;
        self.admit_dash(&prepared.mark)?;
        if let Some(anchor) = prepared.anchor {
            self.anchors.insert(c.mark_id.into(), anchor);
        }
        self.colors.insert(c.mark_id.into(), prepared.color);
        self.positions.insert(c.mark_id.into(), c.position);
        Ok(prepared.mark)
    }
}
fn command_count(g: &Geometry) -> usize {
    if let Geometry::Path(p) = g {
        p.len()
    } else {
        0
    }
}
fn paint_stops(p: Option<&FillPaint>) -> usize {
    p.map_or(0, |p| p.stops().len())
}
/// Custom frames retain private accepted provenance. Public scene/metadata edits
/// must be rejected before bindings or controlled-hover publish another frame.
#[derive(Clone, Debug)]
struct PaintedBarSource {
    mark_slot: usize,
    opacity: f64,
    label: Option<(usize, Color)>,
}
#[derive(Clone, Debug)]
pub struct CustomBarLayout {
    pub layout: StyledBarLayout,
    accepted: StyledBarLayout,
    base_marks: Vec<PaintedBarSource>,
    binding: InteractionBinding,
}
impl CustomBarLayout {
    fn freeze(
        layout: StyledBarLayout,
        state: FactoryState<'_>,
        formatter: &ValueFormat,
    ) -> Result<Self> {
        let mut base_marks = Vec::new();
        let mut sites = Vec::new();
        let marks = layout
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.as_str(), (i, m)))
            .collect::<BTreeMap<_, _>>();
        for b in &layout.bars {
            let (index, m) = marks
                .get(b.id.as_str())
                .copied()
                .ok_or_else(|| ChartError("custom Bar source mark missing".into()))?;
            base_marks.push(PaintedBarSource {
                mark_slot: index,
                opacity: m.opacity,
                label: b.label_index.map(|i| (i, state.label_colors[&i])),
            });
            sites.push(InteractionSite {
                mark_id: b.id.clone(),
                series: b.series.clone(),
                label: m.label.clone(),
                value: Some(b.raw_value),
                formatted_value: Some(formatter.format(b.raw_value)?),
                color: state.colors[&b.id],
                position: state.positions[&b.id],
            });
        }
        let binding = InteractionBinding {
            sites,
            anchors: state.anchors,
            ..Default::default()
        };
        binding.validate(&layout.scene)?;
        Ok(Self {
            accepted: layout.clone(),
            layout,
            base_marks,
            binding,
        })
    }
    fn validate(&self) -> Result<()> {
        ensure(
            self.layout == self.accepted,
            "custom Bar accepted frame or source metadata was changed",
        )?;
        self.layout.scene.validate()
    }
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        self.validate()?;
        self.binding.validate(&self.layout.scene)?;
        Ok(self.binding.clone())
    }
    /// Dim inactive bars by a factor of their original opacity. Owned value-label
    /// alpha follows the same factor; totals/axes/legend remain independently owned.
    /// Resize, hide and data changes require a fresh factory layout.
    pub fn set_active(&mut self, mark_id: Option<&str>, inactive_factor: f64) -> Result<()> {
        self.validate()?;
        ensure(
            inactive_factor.is_finite() && (0. ..=1.).contains(&inactive_factor),
            "invalid custom Bar inactive opacity",
        )?;
        ensure(
            mark_id.is_none_or(|id| self.binding.sites.iter().any(|s| s.mark_id == id)),
            "unknown custom Bar active id",
        )?;
        let mut next = self.accepted.clone();
        for source in &self.base_marks {
            let m = &mut next.scene.marks[source.mark_slot];
            let factor = if mark_id.is_some_and(|id| id != m.id) {
                inactive_factor
            } else {
                1.
            };
            m.opacity = source.opacity * factor;
            if let Some((index, Color(r, g, b, a))) = source.label {
                next.scene.labels[index].color =
                    Color(r, g, b, (f64::from(a) * m.opacity).round() as u8);
            }
        }
        next.scene.validate()?;
        self.accepted = next.clone();
        self.layout = next;
        Ok(())
    }
}
#[derive(Clone, Debug)]
pub struct ScaledCustomBarLayout {
    pub layout: CustomBarLayout,
    pub value_scale: ResolvedContinuousScale,
}
/// Run a typed factory inside numeric layout, once per emitted source bar.
/// Actual ink is independently plot-clipped. Errors reject the entire frame.
pub fn bar_layout_with_bar_factory(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    mut factory: impl FnMut(&BarDatumContext<'_>) -> Result<BarDatumGeometry>,
) -> Result<CustomBarLayout> {
    validate_value_scale(o.value_scale)?;
    let mut state = FactoryState::new(&mut factory);
    let (layout, ()) = build_bar_layout_impl(
        c,
        categories,
        data,
        o,
        style,
        (false, Some(&mut state), BarValidation::Full),
        |domain, range| Ok((ChartScale::new(o.value_scale, domain, range)?, ())),
    )?;
    CustomBarLayout::freeze(
        layout,
        state,
        style
            .tooltip_format
            .as_ref()
            .unwrap_or(&o.value_axis.format),
    )
}
pub fn bar_layout_with_scale_options_and_bar_factory(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    value_scale: &ContinuousScaleOptions,
    mut factory: impl FnMut(&BarDatumContext<'_>) -> Result<BarDatumGeometry>,
) -> Result<ScaledCustomBarLayout> {
    validate_value_scale(value_scale.kind)?;
    value_scale.validate()?;
    let mut state = FactoryState::new(&mut factory);
    let (layout, value_scale) = build_bar_layout_impl(
        c,
        categories,
        data,
        o,
        style,
        (true, Some(&mut state), BarValidation::Full),
        |domain, range| {
            let resolved = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((resolved.chart_scale().clone(), resolved))
        },
    )?;
    Ok(ScaledCustomBarLayout {
        layout: CustomBarLayout::freeze(
            layout,
            state,
            style
                .tooltip_format
                .as_ref()
                .unwrap_or(&o.value_axis.format),
        )?,
        value_scale,
    })
}

fn paint_label_plan(
    mut layout: StyledBarLayout,
    scale: &ChartScale,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    factories: BarFactories<'_>,
) -> Result<CustomBarLayout> {
    let BarFactories { bar, label } = factories;
    let label = label.ok_or_else(|| ChartError("Bar label factory missing".into()))?;
    let mut state = FactoryState::optional(bar);
    let ids = layout
        .bars
        .iter()
        .map(|b| b.id.as_str())
        .collect::<BTreeSet<_>>();
    state.count_base_scene(&layout.scene, &ids)?;
    let slots = layout
        .scene
        .marks
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let scale = scale.clone().clamped(false);
    for b in &layout.bars {
        let slot = slots[&b.id];
        let original = &layout.scene.marks[slot];
        let position = bar_logical_position(&original.geometry, b.visible_rect);
        let plot = original
            .clip
            .ok_or_else(|| ChartError("Bar source plot clip missing".into()))?;
        if let Some(index) = b.label_index {
            let original_label = &layout.scene.labels[index];
            let text = label(&BarLabelContext {
                series: &data[b.source_series],
                category: &categories[b.source_category],
                mark_id: &b.id,
                source_series: b.source_series,
                source_category: b.source_category,
                raw_value: b.raw_value,
                displayed_value: b.displayed_value,
                interval: b.interval,
                default_text: &original_label.text,
                position,
                label_position: [original_label.x, original_label.y],
                plot,
            })?;
            ensure(
                text.len() <= 1024,
                "Bar label factory text exceeds 1024 bytes",
            )?;
            layout.scene.labels[index].text = text;
        }
        let mut mark = state.prepare(&BarDatumContext {
            series: &data[b.source_series],
            category: &categories[b.source_category],
            mark_id: &b.id,
            source_series: b.source_series,
            source_category: b.source_category,
            raw_value: b.raw_value,
            displayed_value: b.displayed_value,
            interval: b.interval,
            band: b.band,
            visible_rect: b.visible_rect,
            original_rect: full_rect(&scale, b.interval, b.band, o.horizontal),
            endpoints: b.interval.map(|value| {
                let pixel = scale.map(value);
                pixel.is_finite().then_some(pixel)
            }),
            position,
            plot,
            geometry: &original.geometry,
            color: b.color,
            stroke: original.stroke,
            horizontal: o.horizontal,
        })?;
        mark.series = original.series.clone();
        mark.value = original.value;
        mark.label = original.label.clone();
        mark.clip = original.clip;
        if let Some(index) = b.label_index {
            let color = layout.scene.labels[index].color;
            state.label_colors.insert(index, color);
            let Color(r, g, b, a) = color;
            layout.scene.labels[index].color =
                Color(r, g, b, (f64::from(a) * mark.opacity).round() as u8);
        }
        layout.scene.marks[slot] = mark;
    }
    layout.scene.validate()?;
    CustomBarLayout::freeze(
        layout,
        state,
        style
            .tooltip_format
            .as_ref()
            .unwrap_or(&o.value_axis.format),
    )
}

/// Configure owned label text and/or one checked geometry mark per datum.
/// Only eligible labels invoke label; hidden/missing/cropped/threshold-skipped
/// and default zero-extent labels do not. No callback is retained for hover.
pub fn bar_layout_with_factories(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    factories: BarFactories<'_>,
) -> Result<CustomBarLayout> {
    if factories.label.is_none() {
        return if let Some(factory) = factories.bar {
            bar_layout_with_bar_factory(c, categories, data, o, style, factory)
        } else {
            bar_layout_with_bar_factory(c, categories, data, o, style, |ctx| {
                Ok(ctx.inherit(ctx.geometry.clone()))
            })
        };
    }
    validate_value_scale(o.value_scale)?;
    let (layout, scale) = build_bar_layout_impl(
        c,
        categories,
        data,
        o,
        style,
        (false, None, BarValidation::LabelsBeforeFactories),
        |domain, range| {
            let scale = ChartScale::new(o.value_scale, domain, range)?;
            Ok((scale.clone(), scale))
        },
    )?;
    paint_label_plan(layout, &scale, categories, data, o, style, factories)
}

pub fn bar_layout_with_scale_options_and_factories(
    c: &ChartConfig,
    categories: &[String],
    data: &[BarSeries],
    o: &BarOptions,
    style: &BarStyleOptions,
    value_scale: &ContinuousScaleOptions,
    factories: BarFactories<'_>,
) -> Result<ScaledCustomBarLayout> {
    if factories.label.is_none() {
        return if let Some(factory) = factories.bar {
            bar_layout_with_scale_options_and_bar_factory(
                c,
                categories,
                data,
                o,
                style,
                value_scale,
                factory,
            )
        } else {
            bar_layout_with_scale_options_and_bar_factory(
                c,
                categories,
                data,
                o,
                style,
                value_scale,
                |ctx| Ok(ctx.inherit(ctx.geometry.clone())),
            )
        };
    }
    validate_value_scale(value_scale.kind)?;
    value_scale.validate()?;
    let (layout, value_scale) = build_bar_layout_impl(
        c,
        categories,
        data,
        o,
        style,
        (true, None, BarValidation::LabelsBeforeFactories),
        |domain, range| {
            let scale = ResolvedContinuousScale::new(domain, range, value_scale)?;
            Ok((scale.chart_scale().clone(), scale))
        },
    )?;
    let layout = paint_label_plan(
        layout,
        value_scale.chart_scale(),
        categories,
        data,
        o,
        style,
        factories,
    )?;
    Ok(ScaledCustomBarLayout {
        layout,
        value_scale,
    })
}
