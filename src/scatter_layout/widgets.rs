//! Actual Scatter node roles with borrowed placement and owned original data.
use std::sync::Arc;

use super::*;

/// Original complete series shared across selected nodes, current computed
/// site and full plot. The raw datum is `series.data[site.source_index]`.
/// Actual geometry and paint are supplied by the accepted host frame's Mark.
/// Equality preserves raw signed-zero bits without changing category identity.
#[derive(Clone, Debug)]
pub struct ScatterNodeWidgetData {
    pub series: Arc<ScatterSeries>,
    pub site: ScatterSite,
    pub plot: ClipRect,
}

impl PartialEq for ScatterNodeWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.series, &other.series)
            || crate::widget_source_eq::scatter_series(&self.series, &other.series))
            && self.site == other.site
            && crate::widget_source_eq::category(&self.site.raw_x, &other.site.raw_x)
            && crate::widget_source_eq::category(&self.site.raw_y, &other.site.raw_y)
            && crate::widget_source_eq::optional(self.site.size_value, other.site.size_value)
            && self.plot == other.plot
    }
}

/// Synchronous references into the supplied source and actual layout.
#[derive(Clone, Copy, Debug)]
pub struct ScatterNodeWidgetContext<'a> {
    pub layout: &'a ScatterLayout,
    pub series: &'a ScatterSeries,
    pub datum: &'a ScatterDatum,
    pub site: &'a ScatterSite,
    pub mark: &'a Mark,
}

fn same_raw(actual: &CategoryKey, source: &CategoryKey) -> bool {
    match (actual, source) {
        (CategoryKey::String(a), CategoryKey::String(b)) => a == b,
        (CategoryKey::Number(a), CategoryKey::Number(b)) => a.to_bits() == b.to_bits(),
        _ => false,
    }
}

fn same_axis(actual: &ResolvedScatterAxis, expected: &ResolvedScatterAxis) -> bool {
    match (actual, expected) {
        (ResolvedScatterAxis::Point(a), ResolvedScatterAxis::Point(b)) => a == b,
        (ResolvedScatterAxis::Continuous(a), ResolvedScatterAxis::Continuous(b)) => {
            a.source_domain_stops() == b.source_domain_stops()
                && a.display_domain_stops() == b.display_domain_stops()
                && a.range_stops() == b.range_stops()
                && a.chart_scale().kind() == b.chart_scale().kind()
                && a.chart_scale().configured_options() == b.chart_scale().configured_options()
        }
        _ => false,
    }
}

fn preflight(layout: &ScatterLayout, source: &[ScatterSeries]) -> Result<()> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    let options = ScatterLayoutOptions {
        scales: layout.scale_options.clone(),
        ..Default::default()
    };
    options.scales.x.validate()?;
    options.scales.y.validate()?;
    let count = source_valid(source, &options)?;
    ensure(
        layout.metadata.len() == count,
        "Scatter Widget source metadata count changed",
    )?;
    let mut hidden = vec![None; source.len()];
    let mut global = 0;
    for (series_index, series) in source.iter().enumerate() {
        for (source_index, datum) in series.data.iter().enumerate() {
            let meta = &layout.metadata[global];
            ensure(
                meta.mark_id == business_id(&series.id, &datum.id)
                    && meta.node_id == datum.id
                    && meta.series_id == series.id
                    && meta.series_index == series_index
                    && meta.source_index == source_index
                    && meta.global_index == global
                    && same_raw(&meta.raw_x, &datum.x)
                    && same_raw(&meta.raw_y, &datum.y)
                    && meta.size_value.map(f64::to_bits) == datum.size_value.map(f64::to_bits),
                "Scatter Widget metadata differs from its original source",
            )?;
            ensure(
                hidden[series_index].is_none_or(|flag| flag == meta.hidden),
                "Scatter Widget series has inconsistent hidden metadata",
            )?;
            hidden[series_index] = Some(meta.hidden);
            global += 1;
        }
    }
    let resolve = |spec: &ScatterAxisScale, horizontal: bool| {
        let key = |datum: &ScatterDatum| {
            if horizontal {
                datum.x.clone()
            } else {
                datum.y.clone()
            }
        };
        let all = source.iter().flat_map(|s| s.data.iter().map(key)).collect();
        let visible = source
            .iter()
            .enumerate()
            .filter(|(i, _)| hidden[*i] != Some(true))
            .flat_map(|(_, s)| s.data.iter().map(key))
            .collect();
        let range = if horizontal {
            [layout.plot.x, layout.plot.x + layout.plot.width]
        } else if matches!(spec, ScatterAxisScale::Point { .. }) {
            [layout.plot.y, layout.plot.y + layout.plot.height]
        } else {
            [layout.plot.y + layout.plot.height, layout.plot.y]
        };
        spec.resolve(visible, all, range)
    };
    ensure(
        same_axis(&layout.x_scale, &resolve(&layout.scale_options.x, true)?)
            && same_axis(&layout.y_scale, &resolve(&layout.scale_options.y, false)?),
        "Scatter Widget resolved axes differ from the declared source scales",
    )?;
    let sites: BTreeMap<_, _> = layout
        .sites
        .iter()
        .map(|site| (site.global_index, site))
        .collect();
    ensure(
        sites.len() == layout.sites.len(),
        "duplicate Scatter Widget source site",
    )?;
    let mut marks = BTreeSet::new();
    for meta in &layout.metadata {
        let projected = if meta.hidden {
            None
        } else {
            match (
                layout.x_scale.map(&meta.raw_x)?,
                layout.y_scale.map(&meta.raw_y)?,
            ) {
                (Some(x), Some(y)) => Some([x, y]),
                _ => None,
            }
        };
        ensure(
            meta.position == projected,
            "Scatter Widget metadata projection changed",
        )?;
        let Some(position) = projected else {
            ensure(
                meta.node_size.is_none()
                    && meta.mark_index.is_none()
                    && !sites.contains_key(&meta.global_index),
                "hidden or omitted Scatter Widget source has an invented role",
            )?;
            continue;
        };
        let Some(size) = meta.node_size else {
            ensure(
                meta.size_value.is_none()
                    && meta.mark_index.is_none()
                    && !sites.contains_key(&meta.global_index),
                "missing-size Scatter Widget source has an invented role",
            )?;
            continue;
        };
        size_valid(size)?;
        let site = sites
            .get(&meta.global_index)
            .ok_or_else(|| ChartError("Scatter Widget mapped source site is missing".into()))?;
        let series = &source[meta.series_index];
        let datum = &series.data[meta.source_index];
        ensure(
            site.mark_id == meta.mark_id
                && site.node_id == meta.node_id
                && site.series_id == meta.series_id
                && site.series_label == series.label
                && site.series_index == meta.series_index
                && site.source_index == meta.source_index
                && site.global_index == meta.global_index
                && same_raw(&site.raw_x, &datum.x)
                && same_raw(&site.raw_y, &datum.y)
                && site.size_value.map(f64::to_bits) == datum.size_value.map(f64::to_bits)
                && site.position == position
                && site.node_size.to_bits() == size.to_bits()
                && meta.mark_index == Some(site.mark_index),
            "Scatter Widget computed site differs from its source metadata",
        )?;
        let expected_label = format!(
            "{} · {}: x: {}, y: {}",
            series.label,
            datum.label.as_ref().unwrap_or(&datum.id),
            site.formatted_x,
            site.formatted_y
        );
        ensure(
            site.formatted_x.len() <= MAX_LABEL_BYTES
                && site.formatted_y.len() <= MAX_LABEL_BYTES
                && site.label == expected_label,
            "Scatter Widget formatted source label changed",
        )?;
        let mark = layout
            .scene
            .marks
            .get(site.mark_index)
            .ok_or_else(|| ChartError("Scatter Widget actual mark index is missing".into()))?;
        ensure(
            mark.id == site.mark_id
                && marks.insert(mark.id.as_str())
                && mark.series == site.series_id
                && mark.label == site.label
                && mark.value
                    == if let CategoryKey::Number(value) = datum.y {
                        value
                    } else {
                        0.
                    }
                && mark.clip.is_none_or(|clip| clip == layout.plot),
            "Scatter Widget actual mark differs from its source site",
        )?;
        // Reuse the exact existing primitive/control-hull bounds contract. The
        // temporary preparation has no paint and does not alter the real Mark.
        let geometry = ScatterNodeGeometry {
            geometry: mark.geometry.clone(),
            fill: None,
            fill_paint: None,
            stroke: None,
            stroke_dash: None,
            opacity: 1.,
            anchor_commands: None,
        };
        let prepared = geometry.prepare_at(&mark.id, position, site.color, 200_000)?;
        ensure(
            site.bounds == prepared.bounds,
            "Scatter Widget bounds differ from actual geometry",
        )?;
        if let Some(index) = site.label_index {
            ensure(
                layout.scene.labels.get(index).is_some()
                    && layout.scene.label_series.get(&index) == Some(&site.series_id),
                "Scatter Widget source label index is invalid",
            )?;
        }
    }
    ensure(
        marks.len() == layout.sites.len()
            && layout
                .scene
                .marks
                .iter()
                .filter(|m| m.id.starts_with("scatter-node:"))
                .all(|m| marks.contains(m.id.as_str())),
        "orphan Scatter Widget site or source mark",
    )?;
    layout.interaction_binding().validate(&layout.scene)
}

/// Build measured Widgets for actual Scatter node marks, keeping all original
/// datum fields and complete series data. Preflight validates every source,
/// metadata record, resolved axis, site, mark and typed anchor before invoking
/// any callback. Hidden series, missing-size skips and point-domain omissions
/// have no node role. Zero size/opacity and off-plot actual nodes remain eligible.
///
/// Source and layout must form a coherent current frame. Public layout edits
/// remain supported; no immutable original-input snapshot is introduced. Size
/// accessor and numeric format options are not retained by ScatterLayout: this
/// adapter checks resolved sizes and constructed labels, not an invented copy
/// of those original options. Geometry/paint comes from the accepted Mark.
///
/// The placement callback is borrowed and not stored. Returning None skips a
/// node; errors return no packet. At most 128 selected slots are returned. Each
/// selected series is cloned once and shared by Arc. Hosts validate placement
/// and clipping; ordinary typed Widgets retain source ink, while explicit
/// PlotWidgetsComposition can replace it. External callback side effects are
/// not rolled back. This API does not implement React springs or animatedProps.
pub fn scatter_node_widgets(
    layout: &ScatterLayout,
    source: &[ScatterSeries],
    revision: u64,
    mut placement: impl FnMut(&ScatterNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<ScatterNodeWidgetData>> {
    preflight(layout, source)?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = BTreeMap::new();
    for site in &layout.sites {
        let series = &source[site.series_index];
        let context = ScatterNodeWidgetContext {
            layout,
            series,
            datum: &series.data[site.source_index],
            site,
            mark: &layout.scene.marks[site.mark_index],
        };
        if let Some(placement) = placement(&context)? {
            ensure(
                packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                "plot Widget slot budget exceeded",
            )?;
            packet.layout.slots.push(PlotWidgetSlot {
                mark_id: site.mark_id.clone(),
                revision,
                placement,
            });
            packet.data.insert(
                site.mark_id.clone(),
                ScatterNodeWidgetData {
                    series: owned
                        .entry(site.series_index)
                        .or_insert_with(|| Arc::new(series.clone()))
                        .clone(),
                    site: site.clone(),
                    plot: layout.plot,
                },
            );
        }
    }
    Ok(packet)
}
