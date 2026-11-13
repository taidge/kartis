//! Configured radar layout. References: E:/Repos/nivo/packages/radar/src/
//! {hooks.ts,defaults.ts,RadarGridLevels.tsx,RadarDots.tsx,RadarLayer.tsx}.
//! Closed interpolation reuses the independently tested D3-shape implementation
//! in cartesian.rs; it does not close an open spline with a straight segment.
use std::collections::{BTreeMap, BTreeSet};
use std::f64::consts::{PI, TAU};

use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};

use super::{RadarSeries, center, point, polygon, valid_key};
use crate::scatter_layout::PreparedNode;
use crate::scene::{ensure, *};
use crate::{
    Color, Curve, GlyphOptions, GlyphPaintMode, InheritedColor, InheritedColorContext,
    InteractionAnchor, MAX_SCENE_GRADIENT_STOPS, StrokeDash, ValueFormat, glyph_geometry,
    glyph_mark,
};

#[path = "radar_value_scale.rs"]
mod radar_value_scale;
pub use radar_value_scale::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadarGridShape {
    Polygon,
    #[default]
    Circle,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarLineStyle {
    pub enabled: bool,
    /// None inherits the series color (grid/spokes inherit theme.grid).
    pub color: Option<Color>,
    pub width: f64,
    pub opacity: f64,
    pub dash: Option<StrokeDash>,
}
impl Default for RadarLineStyle {
    fn default() -> Self {
        Self {
            enabled: true,
            color: None,
            width: 2.,
            opacity: 1.,
            dash: None,
        }
    }
}
impl RadarLineStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.width.is_finite()
                && (0. ..=256.).contains(&self.width)
                && self.opacity.is_finite()
                && (0. ..=1.).contains(&self.opacity),
            "invalid radar line style",
        )?;
        if let Some(dash) = &self.dash {
            dash.validate()?;
        }
        Ok(())
    }
    fn stroke(&self, inherited: Color) -> Option<GraphicStroke> {
        (self.enabled && self.width > 0. && self.opacity > 0.).then(|| GraphicStroke {
            color: alpha(self.color.unwrap_or(inherited), self.opacity),
            width: self.width,
            cap: StrokeCap::Butt,
        })
    }
    fn apply(&self, mark: &mut Mark, inherited: Color) {
        if let Some(stroke) = self.stroke(inherited) {
            mark.stroke = Some(stroke);
            mark.stroke_dash = self.dash.clone();
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct RadarTextStyle {
    pub color: Option<Color>,
    pub size: Option<f64>,
    /// Degrees about each label's own anchor.
    pub rotation: f64,
}
impl RadarTextStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.size
                .is_none_or(|s| s.is_finite() && (1. ..=256.).contains(&s))
                && self.rotation.is_finite()
                && (-360. ..=360.).contains(&self.rotation),
            "invalid radar text style",
        )
    }
    fn label(&self, c: &ChartConfig, p: P, text: String, anchor: TextAnchor) -> Label {
        Label {
            x: p.x,
            y: p.y,
            text,
            color: self.color.unwrap_or(c.theme.text),
            size: self.size.unwrap_or(c.theme.font_size),
            anchor,
            rotation: self.rotation,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarGridOptions {
    pub shape: RadarGridShape,
    pub levels: usize,
    /// Explicit increasing domain values, including neither extrapolation nor
    /// implicit nice rounding. None uses evenly spaced positive radii.
    pub values: Option<Vec<f64>>,
    pub line: RadarLineStyle,
    pub spokes: RadarLineStyle,
    pub tick_labels: bool,
    pub tick_label_axis: usize,
    pub tick_label_offset: [f64; 2],
    pub tick_format: ValueFormat,
    pub tick_style: RadarTextStyle,
}
impl Default for RadarGridOptions {
    fn default() -> Self {
        Self {
            shape: RadarGridShape::Circle,
            levels: 5,
            values: None,
            line: RadarLineStyle {
                width: 1.,
                ..Default::default()
            },
            spokes: RadarLineStyle {
                width: 1.,
                ..Default::default()
            },
            tick_labels: false,
            tick_label_axis: 0,
            tick_label_offset: [4., -2.],
            tick_format: ValueFormat::Auto,
            tick_style: Default::default(),
        }
    }
}
/// Size is the equivalent circle diameter; other glyphs have the same area.
/// The polygon formulas follow D3-shape's square/diamond/triangle/cross symbols.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadarPointShape {
    #[default]
    Circle,
    Square,
    Diamond,
    Triangle,
    Cross,
}
impl RadarPointShape {
    fn geometry(self, p: P, size: f64) -> Geometry {
        let r = size / 2.;
        if self == Self::Circle {
            return Geometry::Circle {
                x: p.x,
                y: p.y,
                radius: r,
            };
        }
        let area = PI * r * r;
        let vertices = match self {
            Self::Square => {
                let h = area.sqrt() / 2.;
                vec![[-h, -h], [h, -h], [h, h], [-h, h]]
            }
            Self::Diamond => {
                let x = (area / (2. * 3_f64.sqrt())).sqrt();
                let y = x * 3_f64.sqrt();
                vec![[0., -y], [x, 0.], [0., y], [-x, 0.]]
            }
            Self::Triangle => {
                let x = (area / 3_f64.sqrt()).sqrt();
                let h = x * 3_f64.sqrt();
                vec![[0., -2. * h / 3.], [x, h / 3.], [-x, h / 3.]]
            }
            Self::Cross => {
                let a = (area / 20.).sqrt();
                [
                    [-3., -1.],
                    [-1., -1.],
                    [-1., -3.],
                    [1., -3.],
                    [1., -1.],
                    [3., -1.],
                    [3., 1.],
                    [1., 1.],
                    [1., 3.],
                    [-1., 3.],
                    [-1., 1.],
                    [-3., 1.],
                ]
                .map(|[x, y]| [x * a, y * a])
                .to_vec()
            }
            Self::Circle => unreachable!(),
        };
        polygon(vertices.into_iter().map(|[x, y]| P::new(p.x + x, p.y + y)))
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarPointOptions {
    pub enabled: bool,
    /// Explicit D3 filled-area/stroked-size glyph. Some overrides the legacy
    /// equivalent-circle diameter and shape. Stroke-only glyphs require an
    /// enabled, positive point border; no stroke is silently invented.
    pub glyph: Option<GlyphOptions>,
    pub shape: RadarPointShape,
    pub size: f64,
    pub fill: Option<Color>,
    /// None preserves fixed `fill` or the original series palette. Some
    /// overrides it using that palette as inherited/surface; Theme uses the
    /// same fixed/palette fallback and backdrop is the chart background.
    /// Ordinary stroke-only glyphs and their inherited factory context keep
    /// no fill. A factory may still explicitly choose its own geometry/paint.
    pub fill_color_profile: Option<InheritedColor>,
    pub border: RadarLineStyle,
    /// None preserves `border.color` or the original series palette. Some
    /// resolves before the existing border.opacity multiplier, applied once.
    /// The role does not enable a border or invent a stroke for glyphs.
    pub border_color_profile: Option<InheritedColor>,
    pub labels: bool,
    pub label_offset: [f64; 2],
    pub label_style: RadarTextStyle,
    pub value_format: ValueFormat,
}
impl Default for RadarPointOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            glyph: None,
            shape: Default::default(),
            size: 6.,
            fill: None,
            fill_color_profile: None,
            border: RadarLineStyle {
                enabled: false,
                ..Default::default()
            },
            border_color_profile: None,
            labels: false,
            label_offset: [0., -12.],
            label_style: Default::default(),
            value_format: ValueFormat::Auto,
        }
    }
}
impl RadarPointOptions {
    fn validate(&self) -> Result<()> {
        for profile in [&self.fill_color_profile, &self.border_color_profile]
            .into_iter()
            .flatten()
        {
            profile.validate()?;
        }
        ensure(
            (self.glyph.is_some()
                || self.size.is_finite() && (0.000001..=256.).contains(&self.size))
                && bounded_offset(self.label_offset),
            "invalid radar point options",
        )?;
        self.border.validate()?;
        self.label_style.validate()?;
        self.value_format.validate().and_then(|_| {
            if let Some(options) = &self.glyph {
                let glyph = glyph_geometry([0., 0.], options)?;
                ensure(
                    glyph.paint_mode != GlyphPaintMode::Stroke
                        || self.border.enabled
                            && self.border.width > 0.
                            && self.border.opacity > 0.,
                    "stroke-only radar glyph requires an explicit visible point border",
                )?;
            }
            Ok(())
        })
    }

    fn paint(&self, c: &ChartConfig, palette: Color) -> Result<RadarPointPaint> {
        let resolve = |profile: &InheritedColor, theme| {
            profile.resolve_validated(InheritedColorContext {
                inherited: palette,
                theme,
                surface: palette,
                backdrop: c.theme.background,
            })
        };
        let legacy_fill = self.fill.unwrap_or(palette);
        let fill = if let Some(profile) = &self.fill_color_profile {
            resolve(profile, legacy_fill)?
        } else {
            legacy_fill
        };
        let stroke = if let Some(profile) = &self.border_color_profile {
            let mut border = self.border.clone();
            border.color = Some(resolve(profile, border.color.unwrap_or(palette))?);
            border.stroke(palette)
        } else {
            self.border.stroke(palette)
        };
        Ok(RadarPointPaint { fill, stroke })
    }
}

#[derive(Clone, Copy)]
struct RadarPointPaint {
    fill: Color,
    stroke: Option<GraphicStroke>,
}

/// A caller-authored point mark in full Scene coordinates. Use
/// [`RadarPointContext::inherit`] to start with the resolved point paint.
pub use crate::scatter_layout::ScatterNodeGeometry as RadarPointGeometry;

/// Source data, logical position and inherited paint for a visible Radar dot.
#[derive(Clone, Copy, Debug)]
pub struct RadarPointContext<'a> {
    pub series: &'a RadarSeries,
    pub series_index: usize,
    pub axis_id: &'a str,
    pub axis_label: &'a str,
    pub axis_index: usize,
    pub mark_id: &'a str,
    pub value: f64,
    pub formatted_value: &'a str,
    pub position: [f64; 2],
    pub plot: ClipRect,
    pub point_options: &'a RadarPointOptions,
    /// Legacy series swatch: polygon fill_color, then polygon border.color,
    /// then the theme palette. Point-role inherited/surface use the original
    /// theme palette independently; resolved point paint is in fill/stroke.
    pub color: Color,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
    pub stroke_dash: Option<&'a StrokeDash>,
}

impl RadarPointContext<'_> {
    /// Replace the geometry while keeping the configured Radar point paint.
    pub fn inherit(&self, geometry: Geometry) -> RadarPointGeometry {
        RadarPointGeometry {
            geometry,
            fill: self.fill,
            fill_paint: None,
            stroke: self.stroke,
            stroke_dash: self.stroke_dash.cloned(),
            opacity: 1.,
            anchor_commands: None,
        }
    }
}

pub(crate) type RadarPointFactory<'a> =
    dyn FnMut(&RadarPointContext<'_>) -> Result<RadarPointGeometry> + 'a;

/// Input for an owned Radar point-label string. Nivo's `index` is this
/// point's axis ID and `key` is its source series ID. These are the checked
/// typed Radar inputs, not an accessor into an arbitrary raw object.
#[derive(Clone, Copy, Debug)]
pub struct RadarPointLabelContext<'a> {
    pub index: &'a str,
    pub key: &'a str,
    pub value: f64,
    pub formatted_value: &'a str,
    /// Logical datum position, source series, identities and resolved paint.
    /// A custom geometry factory does not replace this logical position.
    pub point: RadarPointContext<'a>,
}

pub(crate) type RadarPointLabelFactory<'a> =
    dyn FnMut(&RadarPointLabelContext<'_>) -> Result<String> + 'a;

pub(crate) struct RadarCustomPoint {
    pub(crate) anchor: Option<InteractionAnchor>,
    pub(crate) color: Color,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RadarSeriesStyle {
    pub fill_enabled: bool,
    pub fill_color: Option<Color>,
    pub fill_opacity: f64,
    pub border: RadarLineStyle,
    /// None uses RadarLayoutOptions.points.
    pub points: Option<RadarPointOptions>,
}
impl Default for RadarSeriesStyle {
    fn default() -> Self {
        Self {
            fill_enabled: true,
            fill_color: None,
            fill_opacity: 0.25,
            border: Default::default(),
            points: None,
        }
    }
}
impl RadarSeriesStyle {
    fn validate(&self) -> Result<()> {
        ensure(
            self.fill_opacity.is_finite() && (0. ..=1.).contains(&self.fill_opacity),
            "invalid radar fill opacity",
        )?;
        self.border.validate()?;
        if let Some(p) = &self.points {
            p.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RadarLayoutOptions {
    /// Explicit finite ascending domain. All values must lie inside it.
    /// None uses [0,max(all source values)], including hidden series so a
    /// controlled hidden update agrees with BrowserChart legend filtering.
    pub domain: Option<[f64; 2]>,
    pub curve: Curve,
    /// Clockwise radians from twelve o'clock.
    pub rotation: f64,
    pub radius_ratio: f64,
    pub grid: RadarGridOptions,
    pub axis_labels: bool,
    pub axis_label_offset: f64,
    pub axis_label_style: RadarTextStyle,
    /// Axis strings remain IDs; this map changes only their displayed labels.
    pub axis_labels_by_id: BTreeMap<String, String>,
    pub axis_styles: BTreeMap<String, RadarTextStyle>,
    pub series: RadarSeriesStyle,
    pub series_styles: BTreeMap<String, RadarSeriesStyle>,
    pub points: RadarPointOptions,
    pub hidden: BTreeSet<String>,
    pub legend: bool,
}
impl Default for RadarLayoutOptions {
    fn default() -> Self {
        Self {
            domain: None,
            curve: Curve::LinearClosed,
            rotation: 0.,
            radius_ratio: 0.85,
            grid: Default::default(),
            axis_labels: true,
            axis_label_offset: 16.,
            axis_label_style: Default::default(),
            axis_labels_by_id: Default::default(),
            axis_styles: Default::default(),
            series: Default::default(),
            series_styles: Default::default(),
            points: Default::default(),
            hidden: Default::default(),
            legend: true,
        }
    }
}
fn alpha(mut color: Color, opacity: f64) -> Color {
    color.3 = (f64::from(color.3) * opacity).round() as u8;
    color
}
fn bounded_offset(offset: [f64; 2]) -> bool {
    offset
        .into_iter()
        .all(|v| v.is_finite() && v.abs() <= 16384.)
}
fn valid_id(id: &str) -> bool {
    valid_key(id) && id.len() <= 256
}
fn ratio(v: f64, domain: [f64; 2]) -> f64 {
    let span = domain[1] - domain[0];
    if span.is_finite() {
        (v - domain[0]) / span
    } else {
        (v / 2. - domain[0] / 2.) / (domain[1] / 2. - domain[0] / 2.)
    }
}
fn lerp(domain: [f64; 2], t: f64) -> f64 {
    domain[0] * (1. - t) + domain[1] * t
}
fn token(id: &str) -> String {
    format!("{}:{id}", id.len())
}
/// Configured shared Scene. IDs encode source series and axis strings by byte
/// length, so reordering data never changes datum identity. Values outside the
/// explicit domain are errors; this API never silently clamps/extrapolates.
/// Fill and border alpha multiply their own Color alpha independently (RGBA8);
/// Mark.opacity stays one, preventing fill opacity from dimming the border.
pub fn radar_with_options(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
) -> Result<Scene> {
    Ok(radar_scene(c, axes, data, o, None, false, None)?.scene)
}

/// Configured Radar Scene with an explicit numeric radius profile. Domains
/// are ascending. With no domain, linear and symlog include zero and log
/// requires a distinct, one-sign source extent. An explicit source domain is
/// checked before nice expansion; grid values use the final display domain.
pub fn radar_with_value_scale(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
) -> Result<Scene> {
    Ok(radar_scene(c, axes, data, layout, Some(scale), false, None)?.scene)
}

/// Replace only owned point-label text. The borrowed callback is called once
/// per unhidden, enabled point whose resolved options have `labels = true`.
/// This is owned-label eligibility, independent of transparent or zero ink.
/// It is never stored or called again by hover. Empty text is allowed; text
/// is plain text and is limited to the existing 1024-byte Scene label budget.
pub fn radar_with_point_label_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<Scene> {
    Ok(radar_scene_with_factories(
        c,
        axes,
        data,
        layout,
        None,
        false,
        None,
        Some(&mut labels),
        false,
    )?
    .scene)
}

/// Point-label text with the same checked shared numeric radius scale used
/// by the existing Radar scaled builders.
pub fn radar_with_value_scale_and_point_label_factory(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    layout: &RadarLayoutOptions,
    scale: &RadarValueScaleOptions,
    mut labels: impl FnMut(&RadarPointLabelContext<'_>) -> Result<String>,
) -> Result<Scene> {
    Ok(radar_scene_with_factories(
        c,
        axes,
        data,
        layout,
        Some(scale),
        false,
        None,
        Some(&mut labels),
        false,
    )?
    .scene)
}

pub(crate) struct RadarSceneLayout {
    pub(crate) scene: Scene,
    pub(crate) domain: [f64; 2],
    pub(crate) center: [f64; 2],
    pub(crate) radius: f64,
    pub(crate) scale: Option<RadarValueScale>,
    pub(crate) ticks: Vec<RadarValueTick>,
    pub(crate) custom_points: BTreeMap<String, RadarCustomPoint>,
}
impl RadarSceneLayout {
    pub(crate) fn radius_at(&self, value: f64) -> Result<f64> {
        project_radius(value, self.domain, self.radius, self.scale.as_ref())
    }
}

fn project_radius(
    value: f64,
    domain: [f64; 2],
    radius: f64,
    scale: Option<&RadarValueScale>,
) -> Result<f64> {
    if let Some(scale) = scale {
        scale.radius_at(value)
    } else {
        Ok(radius * ratio(value, domain))
    }
}

fn axis_in_paint_order(index: usize, count: usize, reversed: bool) -> usize {
    if reversed && index != 0 {
        count - index
    } else {
        index
    }
}

pub(crate) fn radar_paint_angle(rotation: f64, axis: usize, count: usize, reversed: bool) -> f64 {
    rotation + TAU * axis_in_paint_order(axis, count, reversed) as f64 / count as f64
}

fn source_domain(data: &[RadarSeries], kind: Option<RadarValueScaleKind>) -> [f64; 2] {
    let values = data.iter().flat_map(|series| &series.values).copied();
    if matches!(kind, Some(RadarValueScaleKind::Log { .. }))
        || matches!(kind, Some(RadarValueScaleKind::Pow { exponent }) if exponent < 0.)
    {
        return values.fold([f64::INFINITY, f64::NEG_INFINITY], |[lo, hi], value| {
            [lo.min(value), hi.max(value)]
        });
    }
    let [lo, hi] = values.fold([0_f64, 0_f64], |[lo, hi], value| {
        [lo.min(value), hi.max(value)]
    });
    let lo = if kind.is_some() { lo } else { 0. };
    [lo, if lo == hi { 1. } else { hi }]
}

pub(crate) fn radar_scene(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
    scale_options: Option<&RadarValueScaleOptions>,
    reversed: bool,
    factory: Option<&mut RadarPointFactory<'_>>,
) -> Result<RadarSceneLayout> {
    radar_scene_with_factories(
        c,
        axes,
        data,
        o,
        scale_options,
        reversed,
        factory,
        None,
        false,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn radar_scene_with_factories(
    c: &ChartConfig,
    axes: &[String],
    data: &[RadarSeries],
    o: &RadarLayoutOptions,
    scale_options: Option<&RadarValueScaleOptions>,
    reversed: bool,
    mut factory: Option<&mut RadarPointFactory<'_>>,
    mut label_factory: Option<&mut RadarPointLabelFactory<'_>>,
    interaction_sites: bool,
) -> Result<RadarSceneLayout> {
    c.validate()?;
    ensure(
        (3..=128).contains(&axes.len())
            && axes.iter().all(|id| valid_id(id))
            && axes.iter().collect::<BTreeSet<_>>().len() == axes.len(),
        "radar requires 3..128 distinct bounded axes",
    )?;
    ensure(
        data.len() <= 128
            && data.iter().all(|s| {
                valid_id(&s.id)
                    && s.label.len() <= MAX_LABEL_BYTES
                    && s.values.len() == axes.len()
                    && s.values.iter().all(|v| v.is_finite())
            })
            && data.iter().map(|s| &s.id).collect::<BTreeSet<_>>().len() == data.len(),
        "invalid configured radar data",
    )?;
    o.curve.validate()?;
    ensure(
        matches!(
            o.curve,
            Curve::LinearClosed
                | Curve::BasisClosed
                | Curve::CardinalClosed
                | Curve::CardinalClosedTension { .. }
                | Curve::CatmullRomClosed { .. }
        ),
        "radar requires an actual closed curve",
    )?;
    ensure(
        o.rotation.is_finite()
            && o.rotation.abs() <= 1e6
            && o.radius_ratio.is_finite()
            && (0. ..=1.).contains(&o.radius_ratio)
            && o.radius_ratio > 0.
            && o.axis_label_offset.is_finite()
            && o.axis_label_offset.abs() <= 16384.,
        "invalid radar geometry options",
    )?;
    ensure(
        (1..=64).contains(&o.grid.levels)
            && o.grid.tick_label_axis < axes.len()
            && bounded_offset(o.grid.tick_label_offset),
        "invalid radar grid options",
    )?;
    ensure(
        o.axis_labels_by_id.len() <= 128
            && o.axis_styles.len() <= 128
            && o.series_styles.len() <= 128
            && o.hidden.len() <= 128
            && o.hidden.iter().all(|id| valid_id(id)),
        "radar style/hidden budget exceeded",
    )?;
    for (id, label) in &o.axis_labels_by_id {
        ensure(
            axes.contains(id) && label.len() <= MAX_LABEL_BYTES,
            "invalid radar axis label override",
        )?;
    }
    for (id, style) in &o.axis_styles {
        ensure(axes.contains(id), "unknown radar axis style")?;
        style.validate()?;
    }
    for (id, style) in &o.series_styles {
        ensure(
            data.iter().any(|s| s.id == *id),
            "unknown radar series style",
        )?;
        style.validate()?;
    }
    o.series.validate()?;
    o.points.validate()?;
    o.axis_label_style.validate()?;
    o.grid.line.validate()?;
    o.grid.spokes.validate()?;
    o.grid.tick_style.validate()?;
    o.grid.tick_format.validate()?;
    let domain = o
        .domain
        .unwrap_or_else(|| source_domain(data, scale_options.map(|s| s.kind)));
    ensure(
        domain.iter().all(|v| v.is_finite())
            && domain[0] < domain[1]
            && data
                .iter()
                .flat_map(|s| &s.values)
                .all(|v| *v >= domain[0] && *v <= domain[1]),
        "radar values must lie inside an ascending finite domain",
    )?;
    let (cx, cy, r) = center(c);
    let r = r * o.radius_ratio;
    let scale = scale_options
        .map(|options| RadarValueScale::new(domain, r, options))
        .transpose()?;
    let domain = scale
        .as_ref()
        .map_or(domain, RadarValueScale::display_domain);
    let values = if let Some(values) = &o.grid.values {
        ensure(
            values.len() <= 64
                && values
                    .iter()
                    .all(|v| v.is_finite() && *v >= domain[0] && *v <= domain[1])
                && values.windows(2).all(|w| w[0] < w[1]),
            "invalid radar grid values",
        )?;
        values.clone()
    } else if let (Some(scale), Some(options)) = (&scale, scale_options) {
        scale.ticks(options.grid_ticks, o.grid.levels)?
    } else {
        (1..=o.grid.levels)
            .map(|i| lerp(domain, i as f64 / o.grid.levels as f64))
            .collect()
    };
    ensure(values.len() <= 64, "radar grid tick budget exceeded")?;
    let ticks = values
        .into_iter()
        .map(|value| {
            Ok(RadarValueTick {
                value,
                radius: project_radius(value, domain, r, scale.as_ref())?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if scale.is_some() {
        ensure(
            ticks.windows(2).all(|pair| pair[0].radius < pair[1].radius),
            "radar grid values must project to distinct increasing radii",
        )?;
    }
    // Admit marks, labels and path commands before output allocation. A dense
    // cross-symbol chart can exceed 200k commands even within source limits;
    // reject it before allocating the complete output geometry.
    let grid_count = if o.grid.line.enabled && o.grid.line.width > 0. {
        ticks.iter().filter(|tick| tick.value > domain[0]).count()
    } else {
        0
    };
    let spoke_count = if o.grid.spokes.enabled && o.grid.spokes.width > 0. {
        axes.len()
    } else {
        0
    };
    let mut marks = grid_count + spoke_count;
    let mut commands = if o.grid.shape == RadarGridShape::Polygon && o.grid.line.enabled {
        grid_count * (axes.len() + 1)
    } else {
        0
    };
    commands += spoke_count * 2;
    let mut labels = if o.axis_labels { axes.len() } else { 0 };
    labels += if o.grid.tick_labels { ticks.len() } else { 0 };
    for d in data.iter().filter(|d| !o.hidden.contains(&d.id)) {
        let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
        let p = style.points.as_ref().unwrap_or(&o.points);
        marks += 1 + if p.enabled { axes.len() } else { 0 };
        commands += axes.len() + if o.curve == Curve::LinearClosed { 1 } else { 2 };
        if p.enabled && factory.is_none() {
            commands += axes.len()
                * if let Some(glyph) = &p.glyph {
                    glyph_geometry([0., 0.], glyph)?.geometry.commands().len()
                } else {
                    match p.shape {
                        RadarPointShape::Circle => 0,
                        RadarPointShape::Triangle => 4,
                        RadarPointShape::Square | RadarPointShape::Diamond => 5,
                        RadarPointShape::Cross => 13,
                    }
                };
        }
        labels += if p.enabled && p.labels { axes.len() } else { 0 };
    }
    ensure(
        marks <= MAX_MARKS && labels <= MAX_MARKS && commands <= 200_000,
        "configured radar output budget exceeded",
    )?;
    // Resolve only real source-series contexts, including hidden/disabled
    // points, before invoking any custom point factory. An empty source has
    // intrinsic profile validation above but no fabricated palette context.
    // With both roles None, paint() keeps the legacy fixed/palette path and
    // never calls the shared color-role resolver.
    let point_paints = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
            style
                .points
                .as_ref()
                .unwrap_or(&o.points)
                .paint(c, c.theme.color(i))
        })
        .collect::<Result<Vec<_>>>()?;
    let mut scene = c.scene()?;
    scene.marks.reserve(marks);
    scene.labels.reserve(labels);
    let angle = |j| radar_paint_angle(o.rotation, j, axes.len(), reversed);
    let ordered_axes = || (0..axes.len()).map(|i| axis_in_paint_order(i, axes.len(), reversed));
    let point_at = |j, v| -> Result<P> {
        Ok(point(
            cx,
            cy,
            project_radius(v, domain, r, scale.as_ref())?,
            angle(j),
        ))
    };
    for (i, tick) in ticks.iter().enumerate() {
        let RadarValueTick { value, radius } = *tick;
        if radius > 0. && o.grid.line.enabled && o.grid.line.width > 0. {
            let geometry = match o.grid.shape {
                RadarGridShape::Circle => Geometry::Circle {
                    x: cx,
                    y: cy,
                    radius,
                },
                RadarGridShape::Polygon => {
                    polygon(ordered_axes().map(|j| point(cx, cy, radius, angle(j))))
                }
            };
            let mut mark = Mark::new(format!("radar-grid:{i}"), geometry, c.theme.grid);
            mark.fill = None;
            mark.label.clear();
            o.grid.line.apply(&mut mark, c.theme.grid);
            scene.marks.push(mark);
        }
        if o.grid.tick_labels {
            let p = point(cx, cy, radius, angle(o.grid.tick_label_axis));
            scene.labels.push(o.grid.tick_style.label(
                c,
                P::new(
                    p.x + o.grid.tick_label_offset[0],
                    p.y + o.grid.tick_label_offset[1],
                ),
                o.grid.tick_format.format(value)?,
                TextAnchor::Start,
            ));
        }
    }
    for j in ordered_axes() {
        let id = &axes[j];
        if o.grid.spokes.enabled && o.grid.spokes.width > 0. {
            let mut mark = Mark::new(
                format!("radar-spoke:{}", token(id)),
                Geometry::Path(vec![
                    C::Move(P::new(cx, cy)),
                    C::Line(point(cx, cy, r, angle(j))),
                ]),
                c.theme.grid,
            );
            mark.fill = None;
            mark.label.clear();
            o.grid.spokes.apply(&mut mark, c.theme.grid);
            scene.marks.push(mark);
        }
        if o.axis_labels {
            let p = point(cx, cy, r + o.axis_label_offset, angle(j));
            let anchor = if (p.x - cx).abs() < 1e-8 {
                TextAnchor::Middle
            } else if p.x > cx {
                TextAnchor::Start
            } else {
                TextAnchor::End
            };
            scene
                .labels
                .push(o.axis_styles.get(id).unwrap_or(&o.axis_label_style).label(
                    c,
                    p,
                    o.axis_labels_by_id.get(id).unwrap_or(id).clone(),
                    anchor,
                ));
        }
    }
    // All polygon fills precede all point glyphs, so a later series never
    // covers an earlier series' interactive datum with its translucent area.
    for (i, d) in data.iter().enumerate() {
        let color = c.theme.color(i);
        let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
        if o.legend {
            scene.legend.push(LegendItem {
                id: d.id.clone(),
                label: d.label.clone(),
                color: style.fill_color.or(style.border.color).unwrap_or(color),
            });
        }
        if o.hidden.contains(&d.id) {
            continue;
        }
        let points = ordered_axes()
            .map(|j| point_at(j, d.values[j]))
            .collect::<Result<Vec<_>>>()?;
        let mut path = crate::cartesian::path(&points, o.curve);
        // D3 Basis/Cardinal/CatmullRomClosed return to their first spline
        // point geometrically but do not emit Z. Z supplies the closed stroke
        // join, without adding a fake closing chord to any open interpolation.
        if !matches!(path.last(), Some(C::Close)) {
            path.push(C::Close);
        }
        let mut mark = Mark::new(
            format!("radar-series:{}", token(&d.id)),
            Geometry::Path(path),
            color,
        );
        mark.series = d.id.clone();
        mark.label = d.label.clone();
        mark.fill = if style.fill_enabled && style.fill_opacity > 0. {
            Some(alpha(style.fill_color.unwrap_or(color), style.fill_opacity))
        } else {
            None
        };
        style.border.apply(&mut mark, color);
        scene.marks.push(mark);
    }
    let mut custom_points = BTreeMap::new();
    let mut paint_stops = 0;
    if factory.is_some() {
        // A factory replaces all enabled glyphs. Use the actual base Scene
        // command count so those glyphs only pay for their authored geometry.
        commands = scene
            .marks
            .iter()
            .map(|mark| match &mark.geometry {
                Geometry::Path(path) => path.len(),
                _ => 0,
            })
            .sum();
    }
    let plot = ClipRect::plot(c);
    if label_factory.is_some() {
        // New text-factory builders preflight the already-known shared Scene,
        // every generated tooltip string and source projection before either
        // callback. Old entry points retain their established error timing.
        // Disabled dots still generate typed interaction sites in layouts.
        scene.validate()?;
        for d in data.iter().filter(|d| !o.hidden.contains(&d.id)) {
            let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
            let opts = style.points.as_ref().unwrap_or(&o.points);
            if !opts.enabled && !interaction_sites {
                continue;
            }
            for j in ordered_axes() {
                let formatted = opts.value_format.format(d.values[j])?;
                let axis_label = o.axis_labels_by_id.get(&axes[j]).unwrap_or(&axes[j]);
                ensure(
                    format!("{} · {}: {}", d.label, axis_label, formatted).len() <= MAX_LABEL_BYTES,
                    "radar tooltip text budget exceeded",
                )?;
                let _ = point_at(j, d.values[j])?;
            }
        }
    }
    for (i, d) in data
        .iter()
        .enumerate()
        .filter(|(_, d)| !o.hidden.contains(&d.id))
    {
        let style = o.series_styles.get(&d.id).unwrap_or(&o.series);
        let opts = style.points.as_ref().unwrap_or(&o.points);
        if !opts.enabled {
            continue;
        }
        let color = c.theme.color(i);
        let paint = point_paints[i];
        for j in ordered_axes() {
            let value = d.values[j];
            let p = point_at(j, value)?;
            let formatted = opts.value_format.format(value)?;
            let axis_label = o.axis_labels_by_id.get(&axes[j]).unwrap_or(&axes[j]);
            let text = format!("{} · {}: {}", d.label, axis_label, formatted);
            ensure(
                text.len() <= MAX_LABEL_BYTES,
                "radar tooltip text budget exceeded",
            )?;
            let id = format!("radar-point:{}:{}", token(&d.id), token(&axes[j]));
            let stroke = paint.stroke;
            let context = RadarPointContext {
                series: d,
                series_index: i,
                axis_id: &axes[j],
                axis_label,
                axis_index: j,
                mark_id: &id,
                value,
                formatted_value: &formatted,
                position: [p.x, p.y],
                plot,
                point_options: opts,
                color: style.fill_color.or(style.border.color).unwrap_or(color),
                fill: (!opts
                    .glyph
                    .as_ref()
                    .is_some_and(|glyph| matches!(glyph.shape, crate::GlyphShape::Stroked(_))))
                .then_some(paint.fill),
                stroke,
                stroke_dash: stroke.and(opts.border.dash.as_ref()),
            };
            let mut mark = if let Some(factory) = factory.as_deref_mut() {
                let PreparedNode {
                    mut mark,
                    anchor,
                    color,
                    ..
                } = factory(&context)?.prepare_at(
                    &id,
                    context.position,
                    context.color,
                    200_000usize.saturating_sub(commands),
                )?;
                if let Geometry::Path(path) = &mark.geometry {
                    commands += path.len();
                }
                paint_stops += mark
                    .fill_paint
                    .as_ref()
                    .map_or(0, |paint| paint.stops().len());
                ensure(
                    paint_stops <= MAX_SCENE_GRADIENT_STOPS,
                    "radar point gradient stop budget exceeded",
                )?;
                mark.clip = Some(plot);
                custom_points.insert(id.clone(), RadarCustomPoint { anchor, color });
                mark
            } else if let Some(options) = &opts.glyph {
                let glyph = glyph_geometry([p.x, p.y], options)?;
                glyph_mark(
                    id.clone(),
                    &glyph,
                    (glyph.paint_mode == GlyphPaintMode::Fill).then_some(paint.fill),
                    paint.stroke,
                )?
            } else {
                Mark::new(id.clone(), opts.shape.geometry(p, opts.size), paint.fill)
            };
            mark.series = d.id.clone();
            mark.label = text;
            mark.value = value;
            if factory.is_none()
                && let Some(stroke) = paint.stroke
            {
                mark.stroke = Some(stroke);
                mark.stroke_dash = opts.border.dash.clone();
            }
            scene.marks.push(mark);
            if opts.labels {
                // Geometry runs first, retaining its legacy order. Text is
                // requested only after that point's geometry is accepted.
                let owned_text = if let Some(labels) = label_factory.as_deref_mut() {
                    labels(&RadarPointLabelContext {
                        index: context.axis_id,
                        key: &context.series.id,
                        value: context.value,
                        formatted_value: context.formatted_value,
                        point: context,
                    })?
                } else {
                    formatted
                };
                ensure(
                    owned_text.len() <= MAX_LABEL_BYTES,
                    "radar point label text budget exceeded",
                )?;
                scene.label_series.insert(scene.labels.len(), d.id.clone());
                scene.labels.push(opts.label_style.label(
                    c,
                    P::new(p.x + opts.label_offset[0], p.y + opts.label_offset[1]),
                    owned_text,
                    TextAnchor::Middle,
                ));
            }
        }
    }
    scene.validate()?;
    Ok(RadarSceneLayout {
        scene,
        domain,
        center: [cx, cy],
        radius: r,
        scale,
        ticks,
        custom_points,
    })
}
