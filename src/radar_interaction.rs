//! Typed radar sectors. References: Nivo 0.99 RadarSlices.tsx/RadarSlice.tsx:
//! rotation-halfStep, one sector per input axis, and stable ascending sort
//! followed by reverse (including reversed source order for equal values).
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::TAU;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};

use crate::radial::{
    RadarPointFactory, RadarPointLabelFactory, radar_paint_angle, radar_scene_with_factories,
};
use crate::scene::ensure;
use crate::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PolarSliceDirection {
    #[default]
    Clockwise,
    CounterClockwise,
}
impl PolarSliceDirection {
    fn sign(self) -> f64 {
        if self == Self::Clockwise { 1. } else { -1. }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarSlice {
    pub id: String,
    pub label: String,
    pub axis_id: String,
    pub axis_index: usize,
    pub start_angle: f64,
    pub end_angle: f64,
    pub angle: f64,
    pub anchor: [f64; 2],
    /// Current visible datum IDs in tooltip order, not lexical order.
    pub member_ids: Vec<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PolarSliceIndex {
    center: [f64; 2],
    radius: f64,
    rotation: f64,
    direction: PolarSliceDirection,
    slices: Vec<PolarSlice>,
    by_id: BTreeMap<String, usize>,
    by_mark: BTreeMap<String, usize>,
}
impl PolarSliceIndex {
    pub fn new(
        center: [f64; 2],
        radius: f64,
        rotation: f64,
        direction: PolarSliceDirection,
        slices: Vec<PolarSlice>,
    ) -> Result<Self> {
        let mut index = Self {
            center,
            radius,
            rotation,
            direction,
            slices,
            by_id: BTreeMap::new(),
            by_mark: BTreeMap::new(),
        };
        index.validate()?;
        for (i, s) in index.slices.iter().enumerate() {
            index.by_id.insert(s.id.clone(), i);
            for m in &s.member_ids {
                index.by_mark.insert(m.clone(), i);
            }
        }
        Ok(index)
    }
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.center.iter().all(|v| v.is_finite() && v.abs() <= 1e6)
                && self.radius.is_finite()
                && (0. ..=1e6).contains(&self.radius)
                && self.radius > 0.
                && self.rotation.is_finite()
                && self.rotation.abs() <= 1e6
                && (3..=128).contains(&self.slices.len()),
            "invalid polar slice frame or axis budget",
        )?;
        let n = self.slices.len();
        let step = TAU / n as f64;
        let mut ids = BTreeSet::new();
        let mut axes = BTreeSet::new();
        let mut members = BTreeSet::new();
        let epsilon = 64. * f64::EPSILON * self.rotation.abs().max(TAU);
        for (i, s) in self.slices.iter().enumerate() {
            let angle = self.rotation + self.direction.sign() * step * i as f64;
            let start = angle - self.direction.sign() * step / 2.;
            let end = angle + self.direction.sign() * step / 2.;
            let p = tip(self.center, self.radius, angle);
            ensure(
                valid_id(&s.id, 1024)
                    && valid_id(&s.axis_id, 256)
                    && s.label.len() <= 1024
                    && s.axis_index == i
                    && ids.insert(&s.id)
                    && axes.insert(&s.axis_id)
                    && s.angle.is_finite()
                    && s.start_angle.is_finite()
                    && s.end_angle.is_finite()
                    && (s.angle - angle).abs() <= epsilon
                    && (s.start_angle - start).abs() <= epsilon
                    && (s.end_angle - end).abs() <= epsilon
                    && s.anchor.iter().zip(p).all(|(a, b)| {
                        a.is_finite() && (*a - b).abs() <= 1e-8 * self.radius.max(1.)
                    })
                    && s.member_ids.len() <= 128,
                "invalid polar slice identity, partition or tip",
            )?;
            for id in &s.member_ids {
                ensure(
                    valid_id(id, 1024) && members.insert(id) && members.len() <= 16384,
                    "invalid polar slice datum membership",
                )?;
            }
        }
        Ok(())
    }
    pub fn center(&self) -> [f64; 2] {
        self.center
    }
    pub fn radius(&self) -> f64 {
        self.radius
    }
    pub fn rotation(&self) -> f64 {
        self.rotation
    }
    pub fn direction(&self) -> PolarSliceDirection {
        self.direction
    }
    pub fn slices(&self) -> &[PolarSlice] {
        &self.slices
    }
    pub fn slice(&self, id: &str) -> Option<&PolarSlice> {
        self.by_id.get(id).map(|i| &self.slices[*i])
    }
    pub fn slice_for_mark(&self, id: &str) -> Option<&PolarSlice> {
        self.by_mark.get(id).map(|i| &self.slices[*i])
    }
    /// Half-open sectors: directed start inclusive, end belongs to the next
    /// axis. The center deterministically chooses the first axis. Finite
    /// roundoff within a few ulps of a boundary snaps to that boundary.
    pub fn pick(&self, x: f64, y: f64) -> Option<&PolarSlice> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        let dx = x - self.center[0];
        let dy = y - self.center[1];
        // Coordinate ulps must not turn an extremely small circle into a
        // large absolute hit disk when its center is far from the origin.
        let radial_epsilon = (16.
            * f64::EPSILON
            * self
                .radius
                .max(self.center[0].abs())
                .max(self.center[1].abs())
                .max(1.))
        .min(self.radius * 1e-10);
        if dx.hypot(dy) > self.radius + radial_epsilon {
            return None;
        }
        if dx == 0. && dy == 0. {
            return self.slices.first();
        }
        let step = TAU / self.slices.len() as f64;
        let angle = dx.atan2(-dy);
        let mut unit = (self.direction.sign() * (angle - self.rotation.rem_euclid(TAU))
            + step / 2.)
            .rem_euclid(TAU)
            / step;
        let nearest = unit.round();
        let tolerance = 16. * f64::EPSILON * self.rotation.abs().max(TAU) / step;
        if (unit - nearest).abs() <= tolerance {
            unit = nearest;
        }
        self.slices.get((unit.floor() as usize) % self.slices.len())
    }
    pub fn spoke(&self, id: &str) -> Result<Geometry> {
        let s = self
            .slice(id)
            .ok_or_else(|| ChartError("unknown polar slice ID".into()))?;
        Ok(Geometry::Path(vec![
            PathCommand::Move(GraphicPoint::new(self.center[0], self.center[1])),
            PathCommand::Line(GraphicPoint::new(s.anchor[0], s.anchor[1])),
        ]))
    }
}
fn valid_id(id: &str, budget: usize) -> bool {
    !id.trim().is_empty() && id.len() <= budget
}
fn tip(c: [f64; 2], r: f64, a: f64) -> [f64; 2] {
    [c[0] + r * a.sin(), c[1] - r * a.cos()]
}
fn point_id(series: &str, axis: &str) -> String {
    format!(
        "radar-point:{}:{series}:{}:{axis}",
        series.len(),
        axis.len()
    )
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadarSliceOrder {
    #[default]
    ValueDescending,
    ValueAscending,
    SeriesOrder,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarInteractionOptions {
    /// True selects angular slices. False binds actual point glyphs for the
    /// Surface's geometry/nearest/slice picker without a polar override.
    pub slices: bool,
    pub direction: PolarSliceDirection,
    pub order: RadarSliceOrder,
    pub spoke: Option<GraphicStroke>,
}
impl Default for RadarInteractionOptions {
    fn default() -> Self {
        Self {
            slices: true,
            direction: Default::default(),
            order: Default::default(),
            spoke: Some(GraphicStroke {
                color: Color(65, 75, 90, 255),
                width: 1.,
                cap: StrokeCap::Butt,
            }),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarSliceRow {
    pub series_id: String,
    pub series_label: String,
    pub series_index: usize,
    pub raw_value: f64,
    pub formatted_value: String,
    pub color: Color,
    pub point_mark_id: String,
    pub glyph_mark_id: Option<String>,
    pub position: [f64; 2],
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarSliceMetadata {
    pub id: String,
    pub axis_id: String,
    pub axis_label: String,
    pub axis_index: usize,
    pub angle: f64,
    pub start_angle: f64,
    pub end_angle: f64,
    pub tip: [f64; 2],
    pub rows: Vec<RadarSliceRow>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarLayout {
    pub scene: Scene,
    pub center: [f64; 2],
    pub radius: f64,
    pub domain: [f64; 2],
    pub slices: Vec<RadarSliceMetadata>,
    pub polar: PolarSliceIndex,
    pub spoke: Option<GraphicStroke>,
    pub anchors: BTreeMap<String, InteractionAnchor>,
    pub slice_interaction: bool,
}
impl RadarLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            sites: self
                .slices
                .iter()
                .flat_map(|s| s.rows.iter())
                .map(|r| InteractionSite {
                    mark_id: r.point_mark_id.clone(),
                    series: r.series_id.clone(),
                    label: r.series_label.clone(),
                    value: Some(r.raw_value),
                    formatted_value: Some(r.formatted_value.clone()),
                    color: r.color,
                    position: r.position,
                })
                .collect(),
            anchors: self.anchors.clone(),
            polar: self.slice_interaction.then(|| self.polar.clone()),
            polar_spoke: self.slice_interaction.then_some(self.spoke).flatten(),
            ..Default::default()
        }
    }
}
impl From<RadarLayout> for SceneInteraction {
    fn from(layout: RadarLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            interaction,
            plot: None,
        }
    }
}

/// Radar geometry, raw interaction values and the exact shared radius scale.
#[derive(Clone, Debug)]
pub struct RadarScaledLayout {
    pub layout: RadarLayout,
    pub value_scale: RadarValueScale,
    pub ticks: Vec<RadarValueTick>,
}
impl From<RadarScaledLayout> for SceneInteraction {
    fn from(layout: RadarScaledLayout) -> Self {
        layout.layout.into()
    }
}

/// Build a typed Radar layout from a checked numeric radius scale. Row
/// values and ordering remain in source units even when the display domain
/// is expanded by nice rounding.
pub fn radar_layout_with_value_scale(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    interaction: &RadarInteractionOptions,
) -> Result<RadarScaledLayout> {
    radar_scaled_layout(c, axes, data, layout, scale, interaction, None)
}

/// Use a shared radius scale and replace each enabled, visible dot with
/// caller-authored geometry. See [`radar_layout_with_point_factory`].
pub fn radar_layout_with_value_scale_and_point_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    interaction: &RadarInteractionOptions,
    mut factory: impl FnMut(&RadarPointContext<'_>) -> Result<RadarPointGeometry>,
) -> Result<RadarScaledLayout> {
    radar_scaled_layout(
        c,
        axes,
        data,
        layout,
        scale,
        interaction,
        Some(&mut factory),
    )
}

fn radar_scaled_layout(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    interaction: &RadarInteractionOptions,
    factory: Option<&mut RadarPointFactory<'_>>,
) -> Result<RadarScaledLayout> {
    let (layout, value_scale, ticks) =
        radar_layout_inner(c, axes, data, layout, Some(scale), interaction, factory)?;
    Ok(RadarScaledLayout {
        layout,
        value_scale: value_scale.expect("explicit Radar scale requested"),
        ticks,
    })
}

/// Additive typed slice layout; original radar_with_options remains unchanged.
/// Enabled point glyphs bind their actual shape. Disabled points retain
/// explicit zero-ink sites for nearest/slice interaction, never fake Exact ink.
pub fn radar_layout(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
    interaction: &RadarInteractionOptions,
) -> Result<RadarLayout> {
    Ok(radar_layout_inner(c, axes, data, o, None, interaction, None)?.0)
}

/// Draw a custom mark for each visible, enabled point in paint order. Paths
/// can offset the ink from the mapped datum position. Circles and rectangles
/// must be centered at that position. The point's label follows the datum,
/// and its authored ink is clipped to the plot.
///
/// Exact picking uses the custom shape. Nearest and angular slice picking
/// retain the logical data position and series/axis identity. Hidden series
/// and disabled points skip the factory. An error rejects the entire layout.
pub fn radar_layout_with_point_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    interaction: &RadarInteractionOptions,
    mut factory: impl FnMut(&RadarPointContext<'_>) -> Result<RadarPointGeometry>,
) -> Result<RadarLayout> {
    Ok(radar_layout_inner(c, axes, data, layout, None, interaction, Some(&mut factory))?.0)
}

fn radar_layout_inner(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
    scale: Option<&RadarValueScaleOptions>,
    interaction: &RadarInteractionOptions,
    factory: Option<&mut RadarPointFactory<'_>>,
) -> Result<(RadarLayout, Option<RadarValueScale>, Vec<RadarValueTick>)> {
    radar_layout_inner_with_labels(c, axes, data, o, scale, interaction, factory, None)
}

/// Replace only owned label text, retaining point identity, tooltip rows,
/// logical anchors and the existing interaction binding.
pub fn radar_layout_with_point_label_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    interaction: &RadarInteractionOptions,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<RadarLayout> {
    Ok(radar_layout_inner_with_labels(
        c,
        axes,
        data,
        layout,
        None,
        interaction,
        None,
        Some(&mut labels),
    )?
    .0)
}

/// Geometry then label text, once for each eligible point in paint order.
/// Geometry retains its existing eligibility even when owned labels are off.
/// A geometry error skips that point's label callback; either error rejects
/// the build. Neither borrowed callback is stored in the resulting layout.
pub fn radar_layout_with_point_and_label_factories(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    interaction: &RadarInteractionOptions,
    mut geometry: impl FnMut(&RadarPointContext<'_>) -> Result<RadarPointGeometry>,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<RadarLayout> {
    Ok(radar_layout_inner_with_labels(
        c,
        axes,
        data,
        layout,
        None,
        interaction,
        Some(&mut geometry),
        Some(&mut labels),
    )?
    .0)
}

/// Owned point-label text using the shared checked radius scale.
pub fn radar_layout_with_value_scale_and_point_label_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    interaction: &RadarInteractionOptions,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<RadarScaledLayout> {
    let (layout, value_scale, ticks) = radar_layout_inner_with_labels(
        c,
        axes,
        data,
        layout,
        Some(scale),
        interaction,
        None,
        Some(&mut labels),
    )?;
    Ok(RadarScaledLayout {
        layout,
        value_scale: value_scale.expect("explicit Radar scale requested"),
        ticks,
    })
}

/// Combined borrowed geometry and owned-text factories with the same shared
/// radius scale, domain, tooltip source units and interaction contract.
#[allow(clippy::too_many_arguments)]
pub fn radar_layout_with_value_scale_and_point_and_label_factories(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    interaction: &RadarInteractionOptions,
    mut geometry: impl FnMut(&RadarPointContext<'_>) -> Result<RadarPointGeometry>,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<RadarScaledLayout> {
    let (layout, value_scale, ticks) = radar_layout_inner_with_labels(
        c,
        axes,
        data,
        layout,
        Some(scale),
        interaction,
        Some(&mut geometry),
        Some(&mut labels),
    )?;
    Ok(RadarScaledLayout {
        layout,
        value_scale: value_scale.expect("explicit Radar scale requested"),
        ticks,
    })
}

#[allow(clippy::too_many_arguments)]
fn radar_layout_inner_with_labels(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
    scale: Option<&RadarValueScaleOptions>,
    interaction: &RadarInteractionOptions,
    factory: Option<&mut RadarPointFactory<'_>>,
    labels: Option<&mut RadarPointLabelFactory<'_>>,
) -> Result<(RadarLayout, Option<RadarValueScale>, Vec<RadarValueTick>)> {
    let reversed = interaction.direction == PolarSliceDirection::CounterClockwise;
    ensure(
        data.iter().all(|s| s.label.len() <= 1024)
            && axes.iter().all(|id| {
                o.axis_labels_by_id
                    .get(id)
                    .is_none_or(|label| label.len() <= 1024)
            })
            && interaction
                .spoke
                .is_none_or(|s| s.width.is_finite() && (0.0001..=256.).contains(&s.width)),
        "invalid radar interaction label or spoke",
    )?;
    let mut rendered =
        radar_scene_with_factories(c, axes, data, o, scale, reversed, factory, labels, true)?;
    let center = rendered.center;
    let radius = rendered.radius;
    let step = TAU / axes.len() as f64;
    let mut marks = rendered
        .scene
        .marks
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id.clone(), i))
        .collect::<BTreeMap<_, _>>();
    let mut slices = vec![];
    let mut anchors = BTreeMap::new();
    for (j, axis) in axes.iter().enumerate() {
        let angle = o.rotation + interaction.direction.sign() * step * j as f64;
        let axis_label = o.axis_labels_by_id.get(axis).unwrap_or(axis).clone();
        let mut rows = vec![];
        for (i, d) in data
            .iter()
            .enumerate()
            .filter(|(_, d)| !o.hidden.contains(&d.id))
        {
            let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
            let points = style.points.as_ref().unwrap_or(&o.points);
            let value = d.values[j];
            let formatted = points.value_format.format(value)?;
            let point_mark_id = point_id(&d.id, axis);
            let mut position = tip(
                center,
                rendered.radius_at(value)?,
                radar_paint_angle(o.rotation, j, axes.len(), reversed),
            );
            let mut glyph = None;
            let mut needs_site = true;
            let mut color = style
                .fill_color
                .or(style.border.color)
                .unwrap_or(c.theme.color(i));
            if let Some(custom) = rendered.custom_points.remove(&point_mark_id) {
                if let Some(anchor) = custom.anchor {
                    anchors.insert(point_mark_id.clone(), anchor);
                }
                color = custom.color;
                glyph = Some(point_mark_id.clone());
                needs_site = false;
            } else if let Some(&index) = marks.get(&point_mark_id) {
                let m = &rendered.scene.marks[index];
                match m.geometry {
                    Geometry::Circle { x, y, .. } => {
                        position = [x, y];
                        needs_site = false;
                    }
                    Geometry::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => {
                        position = [x + width / 2., y + height / 2.];
                        needs_site = false;
                    }
                    Geometry::Path(ref path) => {
                        let indices = if let Some(glyph) = &points.glyph {
                            glyph_geometry(position, glyph)?.anchor_indices
                        } else {
                            path.iter()
                                .enumerate()
                                .filter_map(|(i, command)| {
                                    (!matches!(command, PathCommand::Close)).then_some(i)
                                })
                                .collect::<Vec<_>>()
                        };
                        anchors.insert(
                            point_mark_id.clone(),
                            InteractionAnchor::path_points(&m.geometry, position, &indices)?,
                        );
                        needs_site = false;
                    }
                }
                if !needs_site {
                    glyph = Some(point_mark_id.clone());
                }
            }
            if needs_site {
                ensure(
                    rendered.scene.marks.len() < MAX_MARKS,
                    "radar interaction site Scene budget exceeded",
                )?;
                let mut m = Mark::new(
                    &point_mark_id,
                    Geometry::Circle {
                        x: position[0],
                        y: position[1],
                        radius: 0.,
                    },
                    Color(0, 0, 0, 0),
                );
                m.series = d.id.clone();
                m.label = format!("{} · {}: {}", d.label, axis_label, formatted);
                m.value = value;
                m.clip = Some(ClipRect::plot(c));
                marks.insert(point_mark_id.clone(), rendered.scene.marks.len());
                rendered.scene.marks.push(m);
            }
            rows.push(RadarSliceRow {
                series_id: d.id.clone(),
                series_label: d.label.clone(),
                series_index: i,
                raw_value: value,
                formatted_value: formatted,
                color,
                point_mark_id,
                glyph_mark_id: glyph,
                position,
            });
        }
        match interaction.order {
            RadarSliceOrder::ValueDescending => rows.sort_by(|a, b| {
                b.raw_value
                    .partial_cmp(&a.raw_value)
                    .unwrap()
                    .then(b.series_index.cmp(&a.series_index))
            }),
            RadarSliceOrder::ValueAscending => rows.sort_by(|a, b| {
                a.raw_value
                    .partial_cmp(&b.raw_value)
                    .unwrap()
                    .then(a.series_index.cmp(&b.series_index))
            }),
            RadarSliceOrder::SeriesOrder => {}
        }
        slices.push(RadarSliceMetadata {
            id: format!("radar-slice:{}:{axis}", axis.len()),
            axis_id: axis.clone(),
            axis_label,
            axis_index: j,
            angle,
            start_angle: angle - interaction.direction.sign() * step / 2.,
            end_angle: angle + interaction.direction.sign() * step / 2.,
            tip: tip(center, radius, angle),
            rows,
        });
    }
    let polar = PolarSliceIndex::new(
        center,
        radius,
        o.rotation,
        interaction.direction,
        slices
            .iter()
            .map(|s| PolarSlice {
                id: s.id.clone(),
                label: s.axis_label.clone(),
                axis_id: s.axis_id.clone(),
                axis_index: s.axis_index,
                start_angle: s.start_angle,
                end_angle: s.end_angle,
                angle: s.angle,
                anchor: s.tip,
                member_ids: s.rows.iter().map(|r| r.point_mark_id.clone()).collect(),
            })
            .collect(),
    )?;
    rendered.scene.validate()?;
    let layout = RadarLayout {
        scene: rendered.scene,
        center,
        radius,
        domain: rendered.domain,
        slices,
        polar,
        spoke: interaction.spoke,
        anchors,
        slice_interaction: interaction.slices,
    };
    layout.interaction_binding().validate(&layout.scene)?;
    Ok((layout, rendered.scale, rendered.ticks))
}
