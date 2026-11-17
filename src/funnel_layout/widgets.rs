//! Live Glory roles for a Funnel custom layer, without inventing component props.
use std::collections::BTreeMap;
use std::sync::Arc;

use super::*;
use crate::{
    ClipPath, LayoutRect, MAX_PLOT_WIDGET_SLOTS, PickingProfile, PlotWidgetPlacement,
    PlotWidgetSlot, PlotWidgetsData, PlotWidgetsLayout,
};

/// Actual physical roles, including decorative borders and separators. Labels
/// remain independent Scene paint and are available through the shared pool.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FunnelWidgetRole {
    Body,
    Border,
    BeforeSeparator,
    AfterSeparator,
}

/// Current complete label pool, clips and stage-series ownership. Pool indices
/// are actual Scene label indices; an absent owner denotes ordinary decoration.
#[derive(Clone, Debug, PartialEq)]
pub struct FunnelWidgetLabels {
    pub labels: Vec<Label>,
    pub clips: BTreeMap<usize, ClipPath>,
    pub series: BTreeMap<usize, String>,
}

/// One actual role with complete owned custom-layer inputs. All selected roles
/// share each pool once. A closing separator has no source or part index; its
/// midpoint never masquerades as a source datum. Read actual paint from the
/// accepted host frame, independently of the original stage profile.
#[derive(Clone, Debug)]
pub struct FunnelWidgetData {
    pub source: Arc<Vec<FunnelDatum>>,
    pub parts: Arc<Vec<FunnelPart>>,
    pub separators: Arc<Vec<FunnelSeparator>>,
    pub role: FunnelWidgetRole,
    pub source_index: Option<usize>,
    pub part_index: Option<usize>,
    pub separator_index: Option<usize>,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub labels: Arc<FunnelWidgetLabels>,
}

fn datum_equal(a: &FunnelDatum, b: &FunnelDatum) -> bool {
    a == b
        && crate::widget_source_eq::category(&a.id, &b.id)
        && crate::widget_source_eq::number(a.value, b.value)
        && crate::widget_source_eq::optional(a.size, b.size)
        && crate::widget_source_eq::paint(a.paint.as_ref(), b.paint.as_ref())
}
fn part_equal(a: &FunnelPart, b: &FunnelPart) -> bool {
    a == b && datum_equal(&a.datum, &b.datum)
}
fn separator_equal(a: &FunnelSeparator, b: &FunnelSeparator) -> bool {
    a == b
        && match (&a.part_id, &b.part_id) {
            (Some(a), Some(b)) => crate::widget_source_eq::category(a, b),
            (None, None) => true,
            _ => false,
        }
}
fn shared_equal<T>(a: &Arc<Vec<T>>, b: &Arc<Vec<T>>, equal: fn(&T, &T) -> bool) -> bool {
    Arc::ptr_eq(a, b) || a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| equal(a, b))
}
impl PartialEq for FunnelWidgetData {
    fn eq(&self, other: &Self) -> bool {
        shared_equal(&self.source, &other.source, datum_equal)
            && shared_equal(&self.parts, &other.parts, part_equal)
            && shared_equal(&self.separators, &other.separators, separator_equal)
            && self.role == other.role
            && self.source_index == other.source_index
            && self.part_index == other.part_index
            && self.separator_index == other.separator_index
            && self.position == other.position
            && self.plot == other.plot
            && self.labels == other.labels
    }
}

/// Borrowed current full layer and actual Mark, valid only during the callback.
#[derive(Clone, Copy, Debug)]
pub struct FunnelWidgetContext<'a> {
    pub layout: &'a FunnelLayout,
    pub source: &'a [FunnelDatum],
    pub datum: Option<&'a FunnelDatum>,
    pub part: Option<&'a FunnelPart>,
    pub separator: Option<&'a FunnelSeparator>,
    pub mark: &'a Mark,
    pub role: FunnelWidgetRole,
    pub source_index: Option<usize>,
    pub part_index: Option<usize>,
    pub separator_index: Option<usize>,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub labels: &'a [Label],
    pub label_clips: &'a BTreeMap<usize, ClipPath>,
    pub label_series: &'a BTreeMap<usize, String>,
}

#[derive(Clone, Copy)]
struct Role {
    role: FunnelWidgetRole,
    part: Option<usize>,
    separator: Option<usize>,
    position: [f64; 2],
}

// Only arithmetic reconstruction gets a bounded rounding allowance. Raw data,
// identities, actual paths, and authored source fields remain exact.
fn near(a: f64, b: f64) -> bool {
    a.is_finite()
        && b.is_finite()
        && (a == b || (a - b).abs() <= 32. * f64::EPSILON * a.abs().max(b.abs()).max(1.))
}
fn coordinate(p: [f64; 2]) -> bool {
    p.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
}
fn axis_bounds(r: ClipRect, axis: usize) -> (f64, f64) {
    if axis == 0 {
        (r.x, r.width)
    } else {
        (r.y, r.height)
    }
}

/// Check one of the two supported flow orientations without claiming to recover
/// unretained options such as spacing, size callbacks or curve selection.
fn profiles_match(layout: &FunnelLayout, flow: usize) -> bool {
    if layout.parts.is_empty() {
        return layout.bandwidth == 0.;
    }
    let cross = 1 - flow;
    let (origin, length) = axis_bounds(layout.plot, flow);
    let (cross_origin, cross_length) = axis_bounds(layout.plot, cross);
    let center = layout.parts[0].center[cross];
    let n = layout.parts.len();
    let spacing = if n > 1 {
        (length - layout.bandwidth * n as f64) / (n - 1) as f64
    } else {
        0.
    };
    if layout.bandwidth <= 0.
        || spacing < -32. * f64::EPSILON * length
        || layout.size_range[1] > cross_length
        || (center < cross_origin + layout.size_range[1] / 2.
            && !near(center, cross_origin + layout.size_range[1] / 2.))
        || (center > cross_origin + cross_length - layout.size_range[1] / 2.
            && !near(
                center,
                cross_origin + cross_length - layout.size_range[1] / 2.,
            ))
    {
        return false;
    }
    layout.parts.iter().enumerate().all(|(index, part)| {
        let (start, span) = axis_bounds(part.bounds, flow);
        let (low, width) = axis_bounds(part.bounds, cross);
        let profile = part.profile;
        let extension = low - profile[0].before[cross];
        let next_width = axis_bounds(layout.parts[(index + 1).min(n - 1)].bounds, cross).1;
        near(start, origin + index as f64 * (layout.bandwidth + spacing))
            && near(start + span, start + layout.bandwidth)
            && near(part.center[flow], start + span / 2.)
            && near(part.center[cross], center)
            && near(low + width / 2., center)
            && extension >= -32. * f64::EPSILON * low.abs().max(1.)
            && (part.is_current || near(extension, 0.))
            && near(profile[0].before[flow], start)
            && near(profile[3].before[flow], start + span)
            && near(
                profile[1].before[flow] + profile[2].before[flow],
                start * 2. + span,
            )
            && profile
                .windows(2)
                .all(|p| p[0].before[flow] <= p[1].before[flow])
            && profile.iter().enumerate().all(|(i, p)| {
                near(p.before[flow], p.after[flow])
                    && near((p.before[cross] + p.after[cross]) / 2., center)
                    && near(
                        p.after[cross],
                        p.before[cross] + if i < 2 { width } else { next_width } + 2. * extension,
                    )
                    && if i == 1 || i == 3 {
                        near(p.before[cross], profile[i - 1].before[cross])
                            && near(p.after[cross], profile[i - 1].after[cross])
                    } else {
                        true
                    }
            })
    })
}

fn separator_id(separator: &FunnelSeparator) -> Result<String> {
    let side = match separator.side {
        FunnelSeparatorSide::Before => "before",
        FunnelSeparatorSide::After => "after",
    };
    Ok(if let Some(id) = &separator.part_id {
        format!("funnel:separator:{side}:stage:{}", id.identity()?)
    } else {
        format!("funnel:separator:{side}:tail")
    })
}

fn separators_match(layout: &FunnelLayout, flow: usize) -> bool {
    if layout.parts.is_empty() {
        return layout.separators.is_empty();
    }
    let cross = 1 - flow;
    let (origin, length) = axis_bounds(layout.plot, flow);
    let (cross_origin, cross_length) = axis_bounds(layout.plot, cross);
    let n = layout.parts.len();
    let spacing = if n > 1 {
        (length - layout.bandwidth * n as f64) / (n - 1) as f64
    } else {
        0.
    };
    [FunnelSeparatorSide::Before, FunnelSeparatorSide::After]
        .into_iter()
        .all(|side| {
            let rows: Vec<_> = layout
                .separators
                .iter()
                .filter(|s| s.side == side)
                .collect();
            let center = layout.parts[0].center[cross];
            let reserve = if side == FunnelSeparatorSide::Before {
                center - cross_origin - layout.size_range[1] / 2.
            } else {
                cross_origin + cross_length - center - layout.size_range[1] / 2.
            };
            if rows.is_empty() {
                return near(
                    center,
                    if side == FunnelSeparatorSide::Before {
                        cross_origin + layout.size_range[1] / 2.
                    } else {
                        cross_origin + cross_length - layout.size_range[1] / 2.
                    },
                );
            }
            let mut offset = None;
            rows.iter().enumerate().all(|(index, separator)| {
                let (start, _) = axis_bounds(layout.parts[index.min(n - 1)].bounds, flow);
                let (low, width) = axis_bounds(layout.parts[index.min(n - 1)].bounds, cross);
                let at = if index == n {
                    origin + length
                } else {
                    start - spacing / 2.
                };
                let (edge, gap, inner, boundary, sign) = if side == FunnelSeparatorSide::Before {
                    (
                        separator.from[cross],
                        low - separator.to[cross],
                        separator.to[cross],
                        low,
                        -1.,
                    )
                } else {
                    (
                        separator.to[cross],
                        separator.from[cross] - low - width,
                        separator.from[cross],
                        low + width,
                        1.,
                    )
                };
                // Compare reconstructed positions, not small differences of
                // large coordinates: subtraction can lose several gap ULPs.
                let same_gap =
                    offset.is_none_or(|expected| near(inner, boundary + sign * expected));
                if offset.is_none() {
                    offset = Some(gap);
                }
                near(separator.from[flow], at)
                    && near(separator.to[flow], at)
                    && near(
                        edge,
                        if side == FunnelSeparatorSide::Before {
                            cross_origin
                        } else {
                            cross_origin + cross_length
                        },
                    )
                    && (gap >= 0. || near(inner, boundary))
                    && (gap <= reserve || near(inner, boundary + sign * reserve))
                    && same_gap
            })
        })
}

fn preflight(layout: &FunnelLayout, source: &[FunnelDatum]) -> Result<BTreeMap<String, Role>> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    layout.interaction.validate(&layout.scene)?;
    let keys = part_keys(source)?;
    ensure(
        source.len() == layout.parts.len()
            && layout.value_domain == [0., source.iter().map(|d| d.value).fold(0., f64::max)]
            && layout.size_range[0] == 0.
            && layout.size_range[1].is_finite()
            && layout.size_range[1] > 0.
            && layout.size_range[1] <= layout.plot.width.max(layout.plot.height)
            && layout.bandwidth.is_finite()
            && layout.parts.iter().filter(|p| p.is_current).count() <= 1,
        "Funnel Widget layer totals or source count changed",
    )?;
    ensure(
        layout.interaction.ink_owners.is_empty()
            && layout.interaction.part_data.is_empty()
            && layout.interaction.regions.is_empty()
            && layout.interaction.polar.is_none()
            && layout.interaction.polar_spoke.is_none(),
        "Funnel Widget layer has unsupported extra interaction metadata",
    )?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let sites: BTreeMap<_, _> = layout
        .interaction
        .sites
        .iter()
        .map(|s| (s.mark_id.as_str(), s))
        .collect();
    let mut roles = BTreeMap::new();
    let mut series = BTreeSet::new();
    let mut bodies = 0;
    for (index, (part, datum)) in layout.parts.iter().zip(source).enumerate() {
        ensure(
            part.source_index == index
                && datum_equal(&part.datum, datum)
                && part.mark_id == format!("funnel:stage:{}", keys[index])
                && part.series == format!("funnel:{}", keys[index])
                && coordinate(part.center)
                && part.formatted_value.len() <= MAX_LABEL_BYTES
                && part
                    .profile
                    .iter()
                    .all(|p| coordinate(p.before) && coordinate(p.after)),
            "Funnel Widget part differs from its original source",
        )?;
        part.bounds.validate()?;
        series.insert(part.series.as_str());
        let drawable = part.profile.iter().any(|p| p.before != p.after);
        ensure(
            marks.contains_key(part.mark_id.as_str()) == drawable,
            "Funnel Widget body eligibility changed",
        )?;
        if let Some(mark) = marks.get(part.mark_id.as_str()) {
            let site = sites
                .get(mark.id.as_str())
                .ok_or_else(|| ChartError("Funnel Widget body site is missing".into()))?;
            let label = if datum.label.is_empty() {
                key_label(&datum.id)
            } else {
                datum.label.clone()
            };
            ensure(
                mark.series == part.series
                    && mark.label == label
                    && mark.value == datum.value
                    && mark.clip.is_none()
                    && mark.clip_path.is_none()
                    && site.series == part.series
                    && site.label == label
                    && site.value == Some(datum.value)
                    && site.formatted_value.as_deref() == Some(part.formatted_value.as_str())
                    && site.color == part.color
                    && site.position == part.center
                    && layout.interaction.anchor_for(mark)? == Some(part.center),
                "Funnel Widget body, binding or original anchor changed",
            )?;
            roles.insert(
                mark.id.clone(),
                Role {
                    role: FunnelWidgetRole::Body,
                    part: Some(index),
                    separator: None,
                    position: part.center,
                },
            );
            bodies += 1;
        }
        let border_id = format!("funnel:border:{}", keys[index]);
        if let Some(mark) = marks.get(border_id.as_str()) {
            let border = |curve| {
                let low = part.profile.map(|p| graphic_point(p.before));
                let mut high = part.profile.map(|p| graphic_point(p.after));
                high.reverse();
                Geometry::Path(
                    path(&low, curve)
                        .into_iter()
                        .chain(path(&high, curve))
                        .collect(),
                )
            };
            ensure(
                mark.series == part.series
                    && mark.label.is_empty()
                    && mark.value == 0.
                    && mark.fill.is_none()
                    && mark.fill_paint.is_none()
                    && mark.stroke.is_some()
                    && mark.clip.is_none()
                    && mark.clip_path.is_none()
                    && (mark.geometry == border(Curve::Linear)
                        || mark.geometry == border(Curve::Basis)),
                "Funnel Widget border differs from its profile",
            )?;
            roles.insert(
                border_id,
                Role {
                    role: FunnelWidgetRole::Border,
                    part: Some(index),
                    separator: None,
                    position: part.center,
                },
            );
        }
    }
    ensure(
        bodies == sites.len(),
        "Funnel Widget has an orphan body site",
    )?;
    ensure(
        (profiles_match(layout, 0) && separators_match(layout, 0))
            || (profiles_match(layout, 1) && separators_match(layout, 1)),
        "Funnel Widget bounds, centers or profiles disagree",
    )?;
    let mut before = vec![];
    let mut after = vec![];
    for (index, separator) in layout.separators.iter().enumerate() {
        ensure(
            !source.is_empty() && coordinate(separator.from) && coordinate(separator.to),
            "invalid Funnel Widget separator coordinates",
        )?;
        let side = match separator.side {
            FunnelSeparatorSide::Before => &mut before,
            FunnelSeparatorSide::After => &mut after,
        };
        let part_index = if let Some(id) = &separator.part_id {
            let i = side.len();
            ensure(
                i < source.len() && crate::widget_source_eq::category(id, &source[i].id),
                "Funnel Widget separator source or order changed",
            )?;
            Some(i)
        } else {
            ensure(
                side.len() == source.len(),
                "Funnel Widget closing separator changed",
            )?;
            None
        };
        side.push(index);
        let id = separator_id(separator)?;
        let mark = marks
            .get(id.as_str())
            .ok_or_else(|| ChartError("Funnel Widget separator Mark is missing".into()))?;
        ensure(
            mark.geometry
                == Geometry::Path(vec![
                    Cmd::Move(graphic_point(separator.from)),
                    Cmd::Line(graphic_point(separator.to)),
                ])
                && mark.series.is_empty()
                && mark.label.is_empty()
                && mark.value == 0.
                && mark.fill.is_none()
                && mark.fill_paint.is_none()
                && mark.stroke.is_some()
                && mark.clip.is_none()
                && mark.clip_path.is_none(),
            "Funnel Widget actual separator disagrees with metadata",
        )?;
        let role = match separator.side {
            FunnelSeparatorSide::Before => FunnelWidgetRole::BeforeSeparator,
            FunnelSeparatorSide::After => FunnelWidgetRole::AfterSeparator,
        };
        ensure(
            roles
                .insert(
                    id,
                    Role {
                        role,
                        part: part_index,
                        separator: Some(index),
                        position: [
                            separator.from[0] * 0.5 + separator.to[0] * 0.5,
                            separator.from[1] * 0.5 + separator.to[1] * 0.5,
                        ],
                    },
                )
                .is_none(),
            "duplicate Funnel Widget role",
        )?;
    }
    ensure(
        [before.len(), after.len()]
            .into_iter()
            .all(|n| n == 0 || n == source.len() + 1),
        "Funnel Widget separator side is incomplete",
    )?;
    ensure(
        roles.len() == marks.len(),
        "Funnel Widget Scene contains an orphan physical role",
    )?;
    for owner in layout.scene.label_series.values() {
        ensure(
            series.contains(owner.as_str()),
            "Funnel Widget label owner is missing",
        )?;
    }
    Ok(roles)
}

/// Clone the original Scene and canonical body binding with explicit painted
/// picking. Decorative roles create no canonical controls or interaction sites.
pub fn funnel_widget_frame(layout: &FunnelLayout) -> Result<SceneInteraction> {
    let source: Vec<_> = layout.parts.iter().map(|p| p.datum.clone()).collect();
    preflight(layout, &source)?;
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

/// Attach live Glory Widgets to actual Funnel custom-layer roles in Scene paint
/// order. Nivo exposes custom layers, not part/label/separator component props.
/// Zero-width absent bodies create no role; zero-opacity and no-ink actual Marks
/// remain eligible. `None` skips one role. AtDatum uses the original part center
/// or separator midpoint even for offset custom body ink. Closing separators
/// retain `None` source ownership. Labels remain independent original Scene ink.
///
/// Complete source/structure/binding validation precedes callbacks. Coherent
/// public edits remain possible; original style/factory options are not frozen
/// or reconstructed. Source numeric bits remain exact in owned equality while
/// geometry retains ordinary numeric equality. Each owned pool is cloned once
/// on first selection. The shared limit is 128 selected roles. Host placement
/// validation is unchanged. Errors return no packet, but user side effects are
/// external to the transaction. No React generators or spring props are supplied.
pub fn funnel_widgets(
    layout: &FunnelLayout,
    source: &[FunnelDatum],
    revision: u64,
    mut placement: impl FnMut(&FunnelWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<FunnelWidgetData>> {
    let roles = preflight(layout, source)?;
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = None;
    for mark in layout.scene.paint_marks() {
        let r = roles[&mark.id];
        let context = FunnelWidgetContext {
            layout,
            source,
            datum: r.part.map(|i| &source[i]),
            part: r.part.map(|i| &layout.parts[i]),
            separator: r.separator.map(|i| &layout.separators[i]),
            mark,
            role: r.role,
            source_index: r.part,
            part_index: r.part,
            separator_index: r.separator,
            position: r.position,
            plot: layout.plot,
            labels: &layout.scene.labels,
            label_clips: &layout.scene.label_clips,
            label_series: &layout.scene.label_series,
        };
        let Some(placement) = placement(&context)? else {
            continue;
        };
        ensure(
            packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
            "plot Widget slot budget exceeded",
        )?;
        let placement = match placement {
            PlotWidgetPlacement::AtDatum { offset, measured } => {
                PlotWidgetPlacement::Rect(LayoutRect {
                    x: r.position[0] + offset[0],
                    y: r.position[1] + offset[1],
                    width: measured.width,
                    height: measured.height,
                })
            }
            placement => placement,
        };
        let (source, parts, separators, labels) = owned.get_or_insert_with(|| {
            (
                Arc::new(source.to_vec()),
                Arc::new(layout.parts.clone()),
                Arc::new(layout.separators.clone()),
                Arc::new(FunnelWidgetLabels {
                    labels: layout.scene.labels.clone(),
                    clips: layout.scene.label_clips.clone(),
                    series: layout.scene.label_series.clone(),
                }),
            )
        });
        packet.layout.slots.push(PlotWidgetSlot {
            mark_id: mark.id.clone(),
            revision,
            placement,
        });
        packet.data.insert(
            mark.id.clone(),
            FunnelWidgetData {
                source: source.clone(),
                parts: parts.clone(),
                separators: separators.clone(),
                labels: labels.clone(),
                role: r.role,
                source_index: r.part,
                part_index: r.part,
                separator_index: r.separator,
                position: r.position,
                plot: layout.plot,
            },
        );
    }
    Ok(packet)
}
