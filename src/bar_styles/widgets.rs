//! Measured live Bar Widgets from checked factory-layout source provenance.
use std::sync::Arc;

use super::*;

/// Original series and complete categories shared by the selected bar roles.
/// Current geometry, paint and controlled opacity belong to the host frame.
#[derive(Clone, Debug)]
pub struct BarWidgetData {
    pub series: Arc<BarSeries>,
    pub categories: Arc<Vec<String>>,
    pub bar: StyledBarDatum,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub original_rect: Option<ClipRect>,
    pub endpoints: [Option<f64>; 2],
    pub horizontal: bool,
    /// Current owned Scene value-label role, including empty factory text.
    pub label: Option<Label>,
    pub label_clip: Option<ClipPath>,
}

// Source signed zero is observable metadata even when geometry is unchanged.
impl PartialEq for BarWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.series, &other.series)
            || crate::widget_source_eq::bar_series(&self.series, &other.series))
            && self.categories == other.categories
            && self.bar == other.bar
            && crate::widget_source_eq::number(self.bar.raw_value, other.bar.raw_value)
            && self.position == other.position
            && self.plot == other.plot
            && self.original_rect == other.original_rect
            && self.endpoints == other.endpoints
            && self.horizontal == other.horizontal
            && self.label == other.label
            && self.label_clip == other.label_clip
    }
}

/// Borrowed only during placement. Original layout rectangles and unclamped
/// value endpoints remain independent of a custom Mark's actual painted hull.
#[derive(Clone, Copy, Debug)]
pub struct BarWidgetContext<'a> {
    pub layout: &'a CustomBarLayout,
    pub series: &'a BarSeries,
    pub category: &'a str,
    pub bar: &'a StyledBarDatum,
    pub mark: &'a Mark,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub original_rect: Option<ClipRect>,
    pub endpoints: [Option<f64>; 2],
    pub horizontal: bool,
    pub label: Option<&'a Label>,
    pub label_clip: Option<&'a ClipPath>,
}

struct Checked<'a> {
    binding: &'a InteractionBinding,
    marks: BTreeMap<&'a str, &'a Mark>,
}

fn preflight(layout: &CustomBarLayout) -> Result<Checked<'_>> {
    let binding = layout.widget_binding()?;
    let source = &layout.widget_source;
    crate::surface::validate_plot(Some(source.plot), &layout.layout.scene)?;
    ensure(
        layout.layout.bars.len() == source.placements.len()
            && layout.layout.bars.len() == binding.sites.len(),
        "Bar Widget source placement/site count changed",
    )?;
    let marks: BTreeMap<_, _> = layout
        .layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let sites: BTreeMap<_, _> = binding
        .sites
        .iter()
        .map(|s| (s.mark_id.as_str(), s))
        .collect();
    let mut ids = BTreeSet::new();
    for bar in &layout.layout.bars {
        let series = source
            .series
            .get(bar.source_series)
            .ok_or_else(|| ChartError("Bar Widget original series is missing".into()))?;
        let category = source
            .categories
            .get(bar.source_category)
            .ok_or_else(|| ChartError("Bar Widget original category is missing".into()))?;
        let raw = series
            .values
            .get(bar.source_category)
            .copied()
            .flatten()
            .ok_or_else(|| ChartError("Bar Widget source value is missing".into()))?;
        ensure(
            ids.insert(bar.id.as_str())
                && bar.series == series.id
                && bar.category == *category
                && bar.id == format!("styled-bar:{}:{}", token(&series.id), token(category))
                && raw.to_bits() == bar.raw_value.to_bits(),
            "Bar Widget datum differs from its original source",
        )?;
        let placement = source.placements.get(&bar.id).ok_or_else(|| {
            ChartError("Bar Widget original geometry placement is missing".into())
        })?;
        let mark = marks
            .get(bar.id.as_str())
            .ok_or_else(|| ChartError("Bar Widget actual mark is missing".into()))?;
        let site = sites
            .get(bar.id.as_str())
            .ok_or_else(|| ChartError("Bar Widget original typed site is missing".into()))?;
        ensure(
            mark.series == series.id
                && mark.value == raw
                && mark.clip == Some(source.plot)
                && site.series == series.id
                && site.label == mark.label
                && site.value.map(f64::to_bits) == Some(raw.to_bits())
                && site.position == placement.position,
            "Bar Widget actual mark or typed site differs from its source",
        )?;
        if let Some(rect) = placement.original_rect {
            rect.validate()?;
        }
        ensure(
            placement.endpoints.iter().flatten().all(|v| v.is_finite()),
            "invalid Bar Widget original value endpoints",
        )?;
    }
    Ok(Checked { binding, marks })
}

/// Keep the exact authored Scene, private plot and nullable-safe original
/// business binding together with explicit PaintedGeometry picking. This also
/// retains exact factory Path anchors and an accurate plot when no bars exist.
/// It shares the complete accepted-source preflight with `bar_widgets`.
pub fn bar_widget_frame(layout: &CustomBarLayout) -> Result<SceneInteraction> {
    let checked = preflight(layout)?;
    let mut binding = checked.binding.clone();
    binding.picking_profile = PickingProfile::PaintedGeometry;
    let frame = SceneInteraction {
        scene: layout.layout.scene.clone(),
        interaction: Some(binding),
        plot: Some(layout.widget_source.plot),
    };
    frame.validate()?;
    Ok(frame)
}

/// Select actual Bar roles for measured Glory Widgets. Construct the source
/// with `bar_layout_with_factories(..., BarFactories::default())` to preserve
/// ordinary configured Bar rendering, or use the existing geometry/label and
/// scaled factory variants. StyledBarLayout alone cannot recover original
/// nullable series and unclamped geometry, so this adapter takes CustomBarLayout.
///
/// The complete private accepted frame, original source, typed binding and
/// geometry preflight finishes before any callback. Missing, hidden and fully
/// cropped bars have no role. Actual Some(0) and opacity-zero bars remain roles.
/// `set_active` is the supported controlled-opacity update: raw values and
/// placement remain fixed, while the frame Mark and owned label carry current
/// opacity/color. Labels and totals retain their original Scene ownership.
///
/// Returning None skips a role. Callbacks are borrowed and not retained; errors
/// return no packet, and external callback side effects cannot be rolled back.
/// At most 128 selected slots are admitted. Original series are Arc-shared once
/// per source series, and the complete category vector is shared by all roles.
/// Hosts check measured placement, clipping and backend support. Plain typed
/// Widgets overlay source ink; explicit PlotWidgetsComposition can replace it.
/// These are Glory frames, not React spring animatedProps or automatic labels.
pub fn bar_widgets(
    layout: &CustomBarLayout,
    revision: u64,
    mut placement: impl FnMut(&BarWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<BarWidgetData>> {
    let checked = preflight(layout)?;
    let source = &layout.widget_source;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(source.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    for bar in &layout.layout.bars {
        let series = &source.series[bar.source_series];
        let captured = source.placements[&bar.id];
        let context = BarWidgetContext {
            layout,
            series,
            category: &source.categories[bar.source_category],
            bar,
            mark: checked.marks[bar.id.as_str()],
            position: captured.position,
            plot: source.plot,
            original_rect: captured.original_rect,
            endpoints: captured.endpoints,
            horizontal: captured.horizontal,
            label: bar
                .label_index
                .and_then(|index| layout.layout.scene.labels.get(index)),
            label_clip: bar
                .label_index
                .and_then(|index| layout.layout.scene.label_clips.get(&index)),
        };
        if let Some(placement) = placement(&context)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: bar.id.clone(),
                revision,
                placement,
            });
            packet.data.insert(
                bar.id.clone(),
                BarWidgetData {
                    series: series.clone(),
                    categories: source.categories.clone(),
                    bar: bar.clone(),
                    position: captured.position,
                    plot: source.plot,
                    original_rect: captured.original_rect,
                    endpoints: captured.endpoints,
                    horizontal: captured.horizontal,
                    label: context.label.cloned(),
                    label_clip: context.label_clip.cloned(),
                },
            );
        }
    }
    Ok(packet)
}
