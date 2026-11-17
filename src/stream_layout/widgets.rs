//! Borrowed placement and owned source data for actual Stream dot components.
use super::*;
use crate::{
    MAX_PLOT_WIDGET_SLOTS, PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsData, PlotWidgetsLayout,
};

/// Original nullable series, shared once per source series across selected dots.
/// Read the accepted host frame's Mark for current geometry and paint.
#[derive(Clone, Debug)]
pub struct StreamDotWidgetData {
    pub series: Arc<BarSeries>,
    pub series_index: usize,
    pub layer_id: String,
    pub layer_label: String,
    /// Resolved source layer color, independent of a custom dot's current paint.
    pub layer_color: Color,
    pub stack_index: usize,
    pub sample: StreamSample,
    pub plot: ClipRect,
}

// Source signed zero is observable metadata even when geometry is unchanged.
impl PartialEq for StreamDotWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.series, &other.series)
            || crate::widget_source_eq::bar_series(&self.series, &other.series))
            && self.series_index == other.series_index
            && self.layer_id == other.layer_id
            && self.layer_label == other.layer_label
            && self.layer_color == other.layer_color
            && self.stack_index == other.stack_index
            && self.sample == other.sample
            && crate::widget_source_eq::optional(self.sample.value, other.sample.value)
            && crate::widget_source_eq::category(&self.sample.category, &other.sample.category)
            && self.plot == other.plot
    }
}

/// Synchronous, borrowed current-layout inputs. This context is not retained.
#[derive(Clone, Copy, Debug)]
pub struct StreamDotWidgetContext<'a> {
    pub layout: &'a StreamLayout,
    pub series: &'a BarSeries,
    pub series_index: usize,
    pub layer: &'a StreamLayer,
    pub sample: &'a StreamSample,
    pub mark: &'a Mark,
}

/// Unforgeable provenance captured by the layout builder, not by the adapter.
/// One bounded snapshot is shared when StreamLayout is cloned. In particular,
/// selected slots never clone every value in their original series separately.
#[derive(Clone, Debug)]
pub(super) struct StreamWidgetSource {
    series: Vec<Arc<BarSeries>>,
    dots_enabled: bool,
    plot: ClipRect,
    stack: StackResult,
    layers: Vec<StreamLayer>,
    slices: Vec<StreamSlice>,
    x_scale: ResolvedLinePointScale,
    colors: OrdinalColorScale,
    y_scale: ResolvedContinuousScale,
    marks: BTreeMap<String, Mark>,
}

impl StreamWidgetSource {
    pub(super) fn capture(layout: &StreamLayout, series: &[BarSeries], dots_enabled: bool) -> Self {
        Self {
            series: series.iter().cloned().map(Arc::new).collect(),
            dots_enabled,
            plot: layout.plot,
            stack: layout.stack.clone(),
            layers: layout.layers.clone(),
            slices: layout.slices.clone(),
            x_scale: layout.x_scale.clone(),
            colors: layout.colors.clone(),
            y_scale: layout.y_scale.clone(),
            marks: layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("stream:"))
                .map(|m| (m.id.clone(), m.clone()))
                .collect(),
        }
    }

    fn validate(&self, layout: &StreamLayout) -> Result<()> {
        layout.scene.validate()?;
        crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
        ensure(
            layout.plot == self.plot
                && layout.stack == self.stack
                && layout.layers == self.layers
                && layout.slices == self.slices
                && layout.x_scale == self.x_scale
                && layout.colors == self.colors,
            "Stream Widget computed source metadata was changed",
        )?;
        ensure(
            layout
                .layers
                .iter()
                .zip(&self.layers)
                .all(|(actual, original)| {
                    actual
                        .samples
                        .iter()
                        .zip(&original.samples)
                        .all(|(a, b)| crate::widget_source_eq::category(&a.category, &b.category))
                }),
            "Stream Widget original category bits changed",
        )?;
        ensure(
            layout.y_scale.chart_scale().kind() == self.y_scale.chart_scale().kind()
                && layout.y_scale.chart_scale().configured_options()
                    == self.y_scale.chart_scale().configured_options()
                && layout.y_scale.source_domain_stops() == self.y_scale.source_domain_stops()
                && layout.y_scale.display_domain_stops() == self.y_scale.display_domain_stops()
                && layout.y_scale.range_stops() == self.y_scale.range_stops(),
            "Stream Widget value scale was changed",
        )?;
        let current: BTreeMap<_, _> = layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:"))
            .map(|m| (m.id.as_str(), m))
            .collect();
        ensure(
            current.len() == self.marks.len(),
            "Stream Widget source mark count changed",
        )?;
        for (id, accepted) in &self.marks {
            ensure(
                current.get(id.as_str()).is_some_and(|m| *m == accepted),
                "Stream Widget source mark differs from the accepted layout",
            )?;
        }
        layout.interaction.validate(&layout.scene)?;
        layout.slice_interaction.validate(&layout.scene)?;
        for layer in &layout.layers {
            let series = self
                .series
                .get(layer.original_index)
                .ok_or_else(|| ChartError("Stream Widget original series is missing".into()))?;
            ensure(
                series.id == layer.id,
                "Stream Widget original series identity changed",
            )?;
            for sample in &layer.samples {
                let raw = series
                    .values
                    .get(sample.source_index)
                    .ok_or_else(|| ChartError("Stream Widget source sample is missing".into()))?;
                ensure(
                    raw.map(f64::to_bits) == sample.value.map(f64::to_bits)
                        && layout.x_scale.map(&sample.category) == Some(sample.position[0])
                        && [sample.y0, sample.y1].map(|v| layout.y_scale.map(v)) == sample.screen_y,
                    "Stream Widget raw sample or projection was changed",
                )?;
            }
        }
        Ok(())
    }
}

/// Select measured Widgets for actual non-null dots, without editing the layout.
/// The complete source, geometry and typed-binding preflight runs before any
/// callback. A borrowed callback can return None to omit a dot from the packet.
///
/// Like Nivo's numeric StreamDots datum mapping, enabled zero-sized and
/// zero-opacity dots still invoke placement. Hidden layers do not. The Rust
/// nullable extension differs from Nivo's number-only datum: None samples and
/// dots=None are slice placeholders, not dot roles, and never invoke placement.
/// Full original None/Some(0) values remain in each shared series snapshot and
/// slice binding. Start/Center/End preserve their configured boundary order.
///
/// At most 128 selected slots are returned. Geometry, actual paint and Path
/// anchors come from the host's accepted frame; this data does not copy paint.
/// The host checks measured placement and clip support. Callback errors return
/// no packet; external side effects cannot be rolled back. Ordinary typed
/// Widgets overlay original ink; use PlotWidgetsComposition for replacement.
pub fn stream_dot_widgets(
    layout: &StreamLayout,
    revision: u64,
    mut placement: impl FnMut(&StreamDotWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<StreamDotWidgetData>> {
    let source = layout
        .widget_source
        .as_ref()
        .ok_or_else(|| ChartError("Stream Widget source provenance is missing".into()))?;
    source.validate(layout)?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    if !source.dots_enabled {
        return Ok(packet);
    }
    for layer in &layout.layers {
        if layer.hidden {
            continue;
        }
        for sample in &layer.samples {
            if sample.value.is_none() {
                continue;
            }
            // All eligible marks were validated in the complete preflight above.
            let mark = marks[sample.mark_id.as_str()];
            let series = &source.series[layer.original_index];
            let context = StreamDotWidgetContext {
                layout,
                series,
                series_index: layer.original_index,
                layer,
                sample,
                mark,
            };
            if let Some(placement) = placement(&context)? {
                ensure(
                    packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                    "plot Widget slot budget exceeded",
                )?;
                packet.layout.slots.push(PlotWidgetSlot {
                    mark_id: mark.id.clone(),
                    revision,
                    placement,
                });
                packet.data.insert(
                    mark.id.clone(),
                    StreamDotWidgetData {
                        series: series.clone(),
                        series_index: layer.original_index,
                        layer_id: layer.id.clone(),
                        layer_label: layer.label.clone(),
                        layer_color: layer.color,
                        stack_index: layer.stack_index,
                        sample: sample.clone(),
                        plot: layout.plot,
                    },
                );
            }
        }
    }
    Ok(packet)
}
