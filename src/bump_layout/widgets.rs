//! Measured live components for actual Bump point marks.
use std::sync::Arc;

use super::*;
use crate::{
    MAX_PLOT_WIDGET_SLOTS, PickingProfile, PlotWidgetPlacement, PlotWidgetSlot, PlotWidgetsData,
    PlotWidgetsLayout,
};

/// Complete original series shared across its selected points, and current
/// computed source data. Read geometry and paint from the accepted host Mark.
/// Equality preserves raw signed-zero bits without changing layout identity.
#[derive(Clone, Debug)]
pub struct BumpPointWidgetData {
    pub series: Arc<BumpSeries>,
    pub series_index: usize,
    pub point: BumpPoint,
    /// Resolved point-site representative color, independent of actual paint
    /// and the computed series' line color.
    pub color: Color,
    pub plot: ClipRect,
}

impl PartialEq for BumpPointWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.series, &other.series)
            || crate::widget_source_eq::bump_series(&self.series, &other.series))
            && self.series_index == other.series_index
            && self.point == other.point
            && crate::widget_source_eq::category(&self.point.raw_x, &other.point.raw_x)
            && crate::widget_source_eq::optional(self.point.rank, other.point.rank)
            && self.color == other.color
            && self.plot == other.plot
    }
}

/// Synchronous references into the current public layout, never retained.
#[derive(Clone, Copy, Debug)]
pub struct BumpPointWidgetContext<'a> {
    pub layout: &'a BumpLayout,
    pub computed: &'a BumpComputedSeries,
    pub series: &'a BumpSeries,
    pub datum: &'a BumpDatum,
    pub point: &'a BumpPoint,
    pub mark: &'a Mark,
}

fn same_raw(a: &CategoryKey, b: &CategoryKey) -> bool {
    match (a, b) {
        (CategoryKey::Number(a), CategoryKey::Number(b)) => a.to_bits() == b.to_bits(),
        (CategoryKey::String(a), CategoryKey::String(b)) => a == b,
        _ => false,
    }
}

struct Checked<'a> {
    marks: BTreeMap<&'a str, &'a Mark>,
    sites: BTreeMap<&'a str, &'a InteractionSite>,
}

fn scale_domain(scale: &PointScale, expected: &[CategoryKey]) -> Result<()> {
    let expected = expected
        .iter()
        .map(CategoryKey::identity)
        .collect::<Result<Vec<_>>>()?;
    let actual = scale
        .domain()
        .iter()
        .map(CategoryKey::identity)
        .collect::<Result<Vec<_>>>()?;
    ensure(
        actual == expected,
        "Bump Widget scale domain differs from source",
    )
}

fn between(value: f64, a: f64, b: f64) -> bool {
    value.is_finite() && value >= a.min(b) && value <= a.max(b)
}

fn check_line_points(layout: &BumpLayout, row: &BumpComputedSeries) -> Result<()> {
    let expected_len: usize = row
        .source
        .data
        .iter()
        .map(|d| if d.rank.is_some() { 3 } else { 1 })
        .sum();
    ensure(
        row.line_points.len() == expected_len,
        "Bump Widget shoulder point count changed",
    )?;
    let [start, end] = layout.x_scale.range();
    let half_step = layout.x_scale.step() * 0.5 * if end < start { -1. } else { 1. };
    let mut cursor = 0;
    for (index, point) in row.points.iter().enumerate() {
        if let Some([x, y]) = point.position {
            let [Some(a), Some(b), Some(c)] = row.line_points[cursor..cursor + 3] else {
                return Err(ChartError(
                    "Bump Widget shoulder contains a spurious gap".into(),
                ));
            };
            ensure(
                b == [x, y]
                    && a[1] == y
                    && c[1] == y
                    && if index == 0 {
                        a[0] == start
                    } else {
                        between(a[0], x, x - half_step)
                    }
                    && if index + 1 == row.points.len() && x != start {
                        c[0] == end
                    } else {
                        between(c[0], x, x + half_step)
                    },
                "Bump Widget shoulder differs from its source point",
            )?;
            cursor += 3;
        } else {
            ensure(
                row.line_points[cursor].is_none(),
                "Bump Widget source gap was removed",
            )?;
            cursor += 1;
        }
    }
    Ok(())
}

fn preflight(layout: &BumpLayout) -> Result<Checked<'_>> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    ensure(
        layout.series.len() <= 256,
        "Bump Widget series budget exceeded",
    )?;
    let count = layout.series.iter().try_fold(0usize, |n, row| {
        n.checked_add(row.source.data.len())
            .ok_or_else(|| ChartError("Bump Widget source count overflow".into()))
    })?;
    ensure(
        count <= MAX_BUMP_POINTS,
        "Bump Widget source budget exceeded",
    )?;
    let binding = &layout.interaction;
    binding.validate(&layout.scene)?;
    ensure(
        binding.ink_owners.is_empty()
            && binding.part_data.is_empty()
            && binding.regions.is_empty()
            && binding.polar.is_none()
            && binding.polar_spoke.is_none(),
        "Bump Widget point roles require the original point-only binding",
    )?;
    let marks = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect::<BTreeMap<_, _>>();
    let sites = binding
        .sites
        .iter()
        .map(|s| (s.mark_id.as_str(), s))
        .collect::<BTreeMap<_, _>>();
    let mut series_ids = BTreeSet::new();
    let mut x_ids = BTreeSet::new();
    let mut xs = vec![];
    let mut ranks = vec![];
    for row in &layout.series {
        ensure(
            !row.source.id.trim().is_empty()
                && row.source.id.len() <= 256
                && row.source.label.len() <= 512
                && series_ids.insert(row.source.id.as_str()),
            "invalid Bump Widget source series identity",
        )?;
        let mut keys = BTreeSet::new();
        for datum in &row.source.data {
            let id = datum.x.identity()?;
            ensure(
                id.len() <= 512 && keys.insert(id.clone()) && datum.rank.is_none_or(f64::is_finite),
                "invalid Bump Widget raw datum",
            )?;
            if x_ids.insert(id) {
                xs.push(datum.x.clone());
            }
            if let Some(rank) = datum.rank {
                ranks.push(rank);
            }
        }
    }
    scale_domain(&layout.x_scale, &xs)?;
    ranks.sort_by(f64::total_cmp);
    ranks.dedup_by(|a, b| *a == *b);
    scale_domain(
        &layout.rank_scale,
        &ranks
            .into_iter()
            .map(CategoryKey::Number)
            .collect::<Vec<_>>(),
    )?;
    for (scale, expected) in [
        (
            &layout.x_scale,
            [layout.plot.x, layout.plot.x + layout.plot.width],
        ),
        (
            &layout.rank_scale,
            [layout.plot.y, layout.plot.y + layout.plot.height],
        ),
    ] {
        let mut reverse = expected;
        reverse.reverse();
        ensure(
            (scale.range() == expected || scale.range() == reverse)
                && scale.step().is_finite()
                && scale.step() >= 0.,
            "Bump Widget scale range differs from plot",
        )?;
    }
    let mut point_ids = BTreeSet::new();
    let mut line_ids = BTreeSet::new();
    for (series_index, row) in layout.series.iter().enumerate() {
        ensure(
            row.source_index == series_index && row.points.len() == row.source.data.len(),
            "Bump Widget source index/count changed",
        )?;
        for (source_index, (datum, point)) in row.source.data.iter().zip(&row.points).enumerate() {
            let projected = if let Some(rank) = datum.rank {
                Some([
                    layout
                        .x_scale
                        .map(&datum.x)
                        .ok_or_else(|| ChartError("Bump Widget X projection is missing".into()))?,
                    layout
                        .rank_scale
                        .map(&CategoryKey::Number(rank))
                        .ok_or_else(|| {
                            ChartError("Bump Widget rank projection is missing".into())
                        })?,
                ])
            } else {
                None
            };
            ensure(
                point.source_index == source_index
                    && same_raw(&point.raw_x, &datum.x)
                    && point.rank.map(f64::to_bits) == datum.rank.map(f64::to_bits)
                    && point.position == projected,
                "Bump Widget computed point differs from source",
            )?;
            let expected_id = (!row.hidden && datum.rank.is_some())
                .then(|| point_id(&row.source.id, &datum.x))
                .transpose()?;
            ensure(
                point.mark_id == expected_id,
                "Bump Widget source eligibility or point identity changed",
            )?;
            if let Some(id) = &expected_id {
                let mark = marks
                    .get(id.as_str())
                    .ok_or_else(|| ChartError("Bump Widget actual point mark is missing".into()))?;
                let site = sites.get(id.as_str()).ok_or_else(|| {
                    ChartError("Bump Widget original point site is missing".into())
                })?;
                let label = format!(
                    "{} · {}",
                    row.source.label,
                    category_text(&datum.x, &ValueFormat::Auto)?
                );
                ensure(
                    point_ids.insert(id.clone())
                        && mark.series == row.source.id
                        && mark.label == label
                        && mark.value == datum.rank.unwrap()
                        && mark.clip == Some(layout.plot)
                        && site.series == row.source.id
                        && site.label == label
                        && site.value.map(f64::to_bits) == datum.rank.map(f64::to_bits)
                        && Some(site.position) == projected
                        && binding.anchor_for(mark)? == projected,
                    "Bump Widget mark or anchor disagrees with its source point",
                )?;
            }
        }
        check_line_points(layout, row)?;
        let runs = row
            .line_points
            .split(Option::is_none)
            .filter(|r| !r.is_empty())
            .collect::<Vec<_>>();
        ensure(
            !row.hidden || row.line_mark_ids.is_empty(),
            "hidden Bump Widget series owns line marks",
        )?;
        ensure(
            row.line_mark_ids.is_empty() || row.line_mark_ids.len() == runs.len(),
            "Bump Widget line run count changed",
        )?;
        for (i, id) in row.line_mark_ids.iter().enumerate() {
            let expected = format!("bump-line:{}:{}:{i}", row.source.id.len(), row.source.id);
            let mark = marks
                .get(id.as_str())
                .ok_or_else(|| ChartError("Bump Widget line mark is missing".into()))?;
            let run = runs[i]
                .iter()
                .map(|p| {
                    let [x, y] = p.unwrap();
                    P::new(x, y)
                })
                .collect::<Vec<_>>();
            let matches_path = match &mark.geometry {
                Geometry::Path(actual) => {
                    *actual == path(&run, Curve::Linear) || *actual == path(&run, Curve::Basis)
                }
                _ => false,
            };
            ensure(
                *id == expected
                    && line_ids.insert(id.clone())
                    && mark.series == row.source.id
                    && mark.label.is_empty()
                    && mark.fill.is_none()
                    && mark.stroke.is_some()
                    && mark.clip == Some(layout.plot)
                    && matches_path,
                "Bump Widget line identity/geometry changed",
            )?;
        }
    }
    ensure(
        sites.len() == point_ids.len()
            && layout.scene.marks.iter().all(|m| {
                (!m.id.starts_with("bump-point:") || point_ids.contains(&m.id))
                    && (!m.id.starts_with("bump-line:") || line_ids.contains(&m.id))
            }),
        "orphan Bump Widget source mark or typed site",
    )?;
    Ok(Checked { marks, sites })
}

/// Preserve the exact current Scene and plot, with explicit painted picking
/// and the original point/Path anchors, after the same complete source preflight.
pub fn bump_widget_frame(layout: &BumpLayout) -> Result<SceneInteraction> {
    preflight(layout)?;
    let mut binding = layout.interaction.clone();
    binding.picking_profile = PickingProfile::PaintedGeometry;
    let frame = SceneInteraction {
        scene: layout.scene.clone(),
        interaction: Some(binding),
        plot: Some(layout.plot),
    };
    frame.validate()?;
    Ok(frame)
}

/// Measured live components for actual Bump point Marks, in series/source order.
/// Every source, scale, computed point, line run, real Mark and typed anchor is
/// checked before any placement callback. Public coherent edits remain supported;
/// original style, formatting, shoulder padding and factory options are not
/// retained or reconstructed. Current representative color and actual paint can
/// differ. The source Scene, labels, lines and binding are never edited.
///
/// Hidden/gap data have no actual point role. Like Nivo's computed-series point
/// filter, missing ranks retain line gaps without creating point components.
/// Size-zero and opacity-zero actual points remain eligible. None skips a role;
/// callback errors return no packet, but external side effects cannot roll back.
/// Each selected series is cloned once and shared by Arc, with at most 128 slots.
/// Hosts check placement/clip support and retain source ink unless explicit
/// composition replaces it. No React springs or automatic animated props apply.
pub fn bump_point_widgets(
    layout: &BumpLayout,
    revision: u64,
    mut placement: impl FnMut(&BumpPointWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<BumpPointWidgetData>> {
    let checked = preflight(layout)?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = BTreeMap::new();
    for computed in &layout.series {
        for point in &computed.points {
            let Some(id) = &point.mark_id else {
                continue;
            };
            let context = BumpPointWidgetContext {
                layout,
                computed,
                series: &computed.source,
                datum: &computed.source.data[point.source_index],
                point,
                mark: checked.marks[id.as_str()],
            };
            if let Some(placement) = placement(&context)? {
                ensure(
                    packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
                    "plot Widget slot budget exceeded",
                )?;
                packet.layout.slots.push(PlotWidgetSlot {
                    mark_id: id.clone(),
                    revision,
                    placement,
                });
                packet.data.insert(
                    id.clone(),
                    BumpPointWidgetData {
                        series: owned
                            .entry(computed.source_index)
                            .or_insert_with(|| Arc::new(computed.source.clone()))
                            .clone(),
                        series_index: computed.source_index,
                        point: point.clone(),
                        color: checked.sites[id.as_str()].color,
                        plot: layout.plot,
                    },
                );
            }
        }
    }
    Ok(packet)
}
