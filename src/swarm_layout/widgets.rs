//! Borrowed placement and owned original data for actual packed Swarm nodes.
use std::sync::Arc;

use super::*;

/// Complete original input, shared once across selected nodes, and the current
/// packed site/group/scale. The datum is `source[source_index]`. The accepted
/// host frame supplies actual geometry and paint; `ink_radius` remains the
/// original packing disc even when a geometry factory offsets its Path ink.
#[derive(Clone, Debug)]
pub struct SwarmNodeWidgetData {
    pub source: Arc<Vec<SwarmPoint>>,
    pub source_index: usize,
    pub site_index: usize,
    pub site: SwarmSite,
    pub group: SwarmGroupLayout,
    pub plot: ClipRect,
    pub value_scale: ResolvedContinuousScale,
}

fn number(a: f64, b: f64) -> bool {
    a == b && a.to_bits() == b.to_bits()
}
fn source_equal(a: &[SwarmPoint], b: &[SwarmPoint]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a == b
                && number(a.value, b.value)
                && match (a.size, b.size) {
                    (Some(a), Some(b)) => number(a, b),
                    (None, None) => true,
                    _ => false,
                }
        })
}
fn scale_equal(a: &ResolvedContinuousScale, b: &ResolvedContinuousScale) -> bool {
    a.source_domain_stops() == b.source_domain_stops()
        && a.display_domain_stops() == b.display_domain_stops()
        && a.range_stops() == b.range_stops()
        && a.chart_scale().kind() == b.chart_scale().kind()
        && a.chart_scale().configured_options() == b.chart_scale().configured_options()
}
impl PartialEq for SwarmNodeWidgetData {
    fn eq(&self, other: &Self) -> bool {
        (Arc::ptr_eq(&self.source, &other.source) || source_equal(&self.source, &other.source))
            && self.source_index == other.source_index
            && self.site_index == other.site_index
            && self.site == other.site
            && number(self.site.raw_value, other.site.raw_value)
            && self.group == other.group
            && self.plot == other.plot
            && scale_equal(&self.value_scale, &other.value_scale)
    }
}

/// Synchronous references to the supplied original input and current real Mark.
#[derive(Clone, Copy, Debug)]
pub struct SwarmNodeWidgetContext<'a> {
    pub layout: &'a SwarmLayout,
    pub source: &'a [SwarmPoint],
    pub datum: &'a SwarmPoint,
    pub source_index: usize,
    /// Packed/drawing order, which can differ from original source order.
    pub site_index: usize,
    pub site: &'a SwarmSite,
    pub group: &'a SwarmGroupLayout,
    pub mark: &'a Mark,
}

// SwarmLayout does not retain its original orientation/configuration. Require
// one consistent current orientation rather than inventing those lost options.
fn orientation_valid(layout: &SwarmLayout, horizontal: bool) -> bool {
    let c = Coordinates::new(
        if horizontal {
            SwarmOrientation::Horizontal
        } else {
            SwarmOrientation::Vertical
        },
        layout.plot,
    );
    let [lo, hi] = c.primary;
    let range = layout.value_scale.range_stops();
    if range.iter().any(|v| !v.is_finite() || *v < lo || *v > hi)
        || layout
            .groups
            .iter()
            .any(|g| g.band[0] < c.cross[0] || g.band[1] > c.cross[1])
    {
        return false;
    }
    let mut previous: Option<(usize, f64, &str)> = None;
    for s in &layout.sites {
        let primary = layout.value_scale.map(s.raw_value);
        let cross = s.position[usize::from(horizontal)];
        let g = &layout.groups[s.group_index];
        if s.position[usize::from(!horizontal)] != primary
            || cross < g.band[0]
            || cross > g.band[1]
            || primary < lo - s.ink_radius
            || primary > hi + s.ink_radius
        {
            return false;
        }
        if let Some((group, value, id)) = previous
            && (group > s.group_index
                || (group == s.group_index
                    && (value.total_cmp(&primary).is_gt()
                        || (value.total_cmp(&primary).is_eq() && id >= s.node_id.as_str()))))
        {
            return false;
        }
        previous = Some((s.group_index, primary, &s.node_id));
    }
    // A visible datum whose center is in the plot cannot have been culled by
    // either Inset or Clip, irrespective of the unretained original shape.
    layout.metadata.iter().all(|m| {
        let p = layout.value_scale.map(m.raw_value);
        m.hidden || m.site_index.is_some() || p < lo || p > hi
    })
}

fn preflight(layout: &SwarmLayout, source: Option<&[SwarmPoint]>) -> Result<()> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    ensure(
        layout.metadata.len() <= MAX_POINTS && layout.groups.len() <= MAX_GROUPS,
        "Swarm Widget source/group budget exceeded",
    )?;
    let mut group_ids = BTreeSet::new();
    let mut end = None;
    for g in &layout.groups {
        key(&g.id)?;
        ensure(
            group_ids.insert(g.id.as_str())
                && g.label.len() <= MAX_LABEL_BYTES
                && g.band.iter().all(|x| x.is_finite())
                && g.band[0] < g.band[1]
                && g.center == g.band[0] + (g.band[1] - g.band[0]) / 2.
                && end.is_none_or(|v| v <= g.band[0]),
            "invalid Swarm Widget group",
        )?;
        end = Some(g.band[1]);
    }
    let mut identities = BTreeSet::new();
    let mut indices = BTreeSet::new();
    for (i, meta) in layout.metadata.iter().enumerate() {
        key(&meta.node_id)?;
        let group = layout
            .groups
            .get(meta.group_index)
            .ok_or_else(|| ChartError("Swarm Widget metadata group is missing".into()))?;
        ensure(
            meta.source_index == i
                && meta.mark_id == mark_id(&group.id, &meta.node_id)
                && identities.insert(meta.mark_id.as_str())
                && meta.raw_value.is_finite()
                && layout.value_scale.chart_scale().accepts(meta.raw_value)
                && layout.value_scale.map(meta.raw_value).is_finite()
                && meta.size.is_finite()
                && meta.size >= 0.
                && meta.hidden == group.hidden,
            "Swarm Widget metadata differs from its source identity/group",
        )?;
        if let Some(si) = meta.site_index {
            ensure(
                !meta.hidden && indices.insert(si),
                "duplicate or hidden Swarm Widget site",
            )?;
            let s = layout
                .sites
                .get(si)
                .ok_or_else(|| ChartError("Swarm Widget source site is missing".into()))?;
            ensure(
                s.source_index == i
                    && s.mark_id == meta.mark_id
                    && s.node_id == meta.node_id
                    && s.group_index == meta.group_index
                    && s.group_id == group.id
                    && number(s.raw_value, meta.raw_value)
                    && number(s.size, meta.size),
                "Swarm Widget site differs from its source metadata",
            )?;
        }
    }
    ensure(
        indices.len() == layout.sites.len(),
        "orphan Swarm Widget site",
    )?;
    if let Some(source) = source {
        ensure(
            source.len() == layout.metadata.len(),
            "Swarm Widget original source count changed",
        )?;
        let mut text_bytes = 0usize;
        for (p, meta) in source.iter().zip(&layout.metadata) {
            key(&p.id)?;
            key(&p.group)?;
            ensure(
                p.id == meta.node_id
                    && p.group == layout.groups[meta.group_index].id
                    && number(p.value, meta.raw_value)
                    && p.size
                        .is_none_or(|s| s.is_finite() && s >= 0. && number(s, meta.size))
                    && p.label.as_ref().is_none_or(|s| s.len() <= MAX_LABEL_BYTES),
                "Swarm Widget original source differs from metadata",
            )?;
            text_bytes += p.id.len() + p.group.len() + p.label.as_ref().map_or(0, String::len);
            ensure(
                text_bytes <= 4_000_000,
                "Swarm Widget source text budget exceeded",
            )?;
            if layout.value_scale.chart_scale().kind() == ScaleKind::Utc {
                ensure(
                    p.value.fract() == 0. && p.value.abs() <= MAX_TIMESTAMP_MS as f64,
                    "Swarm Widget UTC source is not integral and bounded",
                )?;
            }
        }
    }
    let mut marks = BTreeSet::new();
    let mut last_mark = None;
    for (i, s) in layout.sites.iter().enumerate() {
        ensure(
            layout.metadata[s.source_index].site_index == Some(i)
                && s.position.iter().all(|x| x.is_finite())
                && s.ink_radius.is_finite()
                && s.ink_radius >= 0.
                && s.formatted_value.len() <= MAX_LABEL_BYTES
                && s.label.len() <= MAX_LABEL_BYTES,
            "invalid Swarm Widget packed site",
        )?;
        if let Some(source) = source {
            let datum = &source[s.source_index];
            ensure(
                s.label
                    == format!(
                        "{} · {}: {}",
                        layout.groups[s.group_index].label,
                        datum.label.as_ref().unwrap_or(&datum.id),
                        s.formatted_value
                    ),
                "Swarm Widget formatted source label changed",
            )?;
        }
        let mark = layout
            .scene
            .marks
            .get(s.mark_index)
            .ok_or_else(|| ChartError("Swarm Widget actual mark is missing".into()))?;
        ensure(
            marks.insert(mark.id.as_str())
                && mark.id == s.mark_id
                && mark.series == s.group_id
                && mark.label == s.label
                && mark.value == s.raw_value
                && mark.clip == Some(layout.plot)
                && last_mark.is_none_or(|index| index < s.mark_index),
            "Swarm Widget actual mark differs from its packed site",
        )?;
        last_mark = Some(s.mark_index);
    }
    ensure(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("swarm-node:"))
            .all(|m| marks.contains(m.id.as_str())),
        "orphan Swarm Widget source mark",
    )?;
    ensure(
        orientation_valid(layout, true) || orientation_valid(layout, false),
        "Swarm Widget packing order, band or value projection changed",
    )?;
    layout.interaction_binding().validate(&layout.scene)
}

/// Validate the current layout's structure and typed anchors, then clone its
/// exact Scene with an explicit PaintedGeometry binding and full plot. The
/// original conversion's picking policy is unchanged. Original input agreement
/// is additionally checked by `swarm_node_widgets`, which receives that input.
pub fn swarm_widget_frame(layout: &SwarmLayout) -> Result<SceneInteraction> {
    preflight(layout, None)?;
    layout.clone().into_painted_scene_interaction()
}

/// Create actual packed node roles, including zero-size/opacity/no-ink Marks.
/// Hidden/culled input has no role. Every source record, group, current scale
/// projection, packed site, Mark and typed anchor is checked before the first
/// borrowed callback. None skips a role; errors return no packet. External
/// callback effects do not roll back. At most 128 selected slots are returned.
///
/// The complete original input is cloned once and shared by Arc across slots;
/// equality preserves original value/size signed-zero bits, including unselected
/// siblings. Source and public layout must be coherent, but original packing,
/// shape, orientation and format options are not retained and cannot be restored
/// or frozen by this adapter. In particular, coherent public edits remain legal.
/// Custom Path ink does not change the original packed anchor/collision disc.
/// Hosts validate placement/clip capabilities. Source ink is retained unless an
/// explicit composition replaces it; this is not automatic spring animation.
pub fn swarm_node_widgets(
    layout: &SwarmLayout,
    source: &[SwarmPoint],
    revision: u64,
    mut placement: impl FnMut(&SwarmNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<SwarmNodeWidgetData>> {
    preflight(layout, Some(source))?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = None;
    for (site_index, site) in layout.sites.iter().enumerate() {
        let context = SwarmNodeWidgetContext {
            layout,
            source,
            datum: &source[site.source_index],
            source_index: site.source_index,
            site_index,
            site,
            group: &layout.groups[site.group_index],
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
                SwarmNodeWidgetData {
                    source: owned
                        .get_or_insert_with(|| Arc::new(source.to_vec()))
                        .clone(),
                    source_index: site.source_index,
                    site_index,
                    site: site.clone(),
                    group: context.group.clone(),
                    plot: layout.plot,
                    value_scale: layout.value_scale.clone(),
                },
            );
        }
    }
    Ok(packet)
}
