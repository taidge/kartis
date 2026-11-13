//! Explicit D3/Nivo pie geometry and measured link-label placement.
//!
//! References: d3-shape 3.2.0 src/{pie,arc}.js; local Nivo 0.99.0
//! packages/pie/src/hooks.ts:105..159,349..395 and arcs/src/boundingBox.ts.
//! Angles are radians clockwise from twelve o'clock. Legacy `pie` is unchanged.
pub(crate) mod pie_arc;
use crate::{scene::ensure, *};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as C, StrokeCap};
use std::{
    collections::{BTreeMap, BTreeSet},
    f64::consts::{FRAC_PI_2, TAU},
    sync::Arc,
};
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum PiePadRadius {
    #[default]
    Auto,
    Fixed(f64),
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PiePadding {
    /// Existing equal inner/outer angular gaps. Zero values do not reserve padding.
    LegacyAngularGap,
    /// Arc generator uses the D3 pie's clamped per-datum `padAngle`.
    D3Applied { pad_radius: PiePadRadius },
    /// Nivo passes the requested pad as a constant to the arc generator, even
    /// when the pie allocation has clamped its per-datum pad.
    NivoRequested { pad_radius: PiePadRadius },
}
impl Default for PiePadding {
    fn default() -> Self {
        Self::NivoRequested {
            pad_radius: PiePadRadius::Auto,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PieFit {
    Centered,
    /// Fits the complete base sector including the center, independently of
    /// data/hidden/active/labels/borders. Exact cardinals, without Nivo's rounded
    /// integer-degree boundary overshoot. This is not tight annulus fitting.
    #[default]
    NivoSector,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieLayoutOptions {
    pub pie: PieOptions,
    pub padding: PiePadding,
    pub fit: PieFit,
}
impl Default for PieLayoutOptions {
    fn default() -> Self {
        Self {
            pie: Default::default(),
            padding: PiePadding::NivoRequested {
                pad_radius: PiePadRadius::Auto,
            },
            fit: Default::default(),
        }
    }
}

/// Optional color sources for existing pie borders and label roles.
/// Theme resolves to the theme text for labels and grid ink for strokes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PieColorStyles {
    pub border: Option<InheritedColor>,
    pub arc_label: Option<InheritedColor>,
    pub link_label: Option<InheritedColor>,
    pub link_line: Option<InheritedColor>,
}
impl PieColorStyles {
    fn validate(&self) -> Result<()> {
        for style in [
            &self.border,
            &self.arc_label,
            &self.link_label,
            &self.link_line,
        ]
        .into_iter()
        .flatten()
        {
            style.validate()?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieSliceLayout {
    pub id: String,
    pub source_index: usize,
    pub rank: usize,
    pub value: f64,
    pub fraction: f64,
    /// Untrimmed allocated angles for D3 profiles, trimmed angles for Legacy.
    pub start_angle: f64,
    pub end_angle: f64,
    pub requested_pad: f64,
    pub applied_pad: f64,
    pub geometry_pad: f64,
    pub inner_radius: f64,
    pub outer_radius: f64,
    pub inner_angles: [f64; 2],
    pub outer_angles: [f64; 2],
    pub inner_collapsed: bool,
    pub outer_collapsed: bool,
    /// Fill-only collapsed arcs have no mark. An explicitly stroked collapsed
    /// arc retains D3's real radial border and its current raw datum.
    pub mark_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieLinkLabelCandidate {
    pub id: String,
    pub series: String,
    pub signature: String,
    pub label: Label,
    pub font: crate::ResolvedFont,
    pub points: [P; 3],
    leader: Option<Mark>,
    pub text_offset: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieMeasuredLinkLabel {
    pub id: String,
    pub signature: String,
    /// Actual bounds relative to the Label anchor, after its rotation. Include
    /// bearings/ascent/descent from the chosen font; no width heuristic is used.
    pub bounds: LayoutRect,
}
impl PieLinkLabelCandidate {
    pub fn measured(&self, bounds: LayoutRect) -> PieMeasuredLinkLabel {
        PieMeasuredLinkLabel {
            id: self.id.clone(),
            signature: self.signature.clone(),
            bounds,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PieLinkPlacementOptions {
    pub collision: LabelCollisionOptions,
    pub y_offsets: Vec<f64>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PiePlacedLinkLabel {
    pub id: String,
    pub bounds: Option<LayoutRect>,
    pub label_index: Option<usize>,
    pub leader_index: Option<usize>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct PieLayout {
    pub scene: Scene,
    pub center: [f64; 2],
    pub radius: f64,
    pub inner_radius: f64,
    pub bounds: ClipRect,
    pub slices: Vec<PieSliceLayout>,
    pub link_candidates: Vec<PieLinkLabelCandidate>,
    pub link_placements: Vec<PiePlacedLinkLabel>,
    base_scene: Scene,
    placement_scene: Scene,
    interaction_source: Arc<PieInteractionSource>,
}
#[derive(Clone, Debug, PartialEq)]
struct PieInteractionSource {
    center: [f64; 2],
    radius: f64,
    inner_radius: f64,
    bounds: ClipRect,
    slices: Vec<PieSliceLayout>,
}
impl PieLayout {
    /// Select the shared font for native arc and link labels before measuring.
    /// Customized Scenes must be authored after measured placement.
    pub fn with_label_font(&self, font: crate::FontOptions) -> Result<Self> {
        font.validate()?;
        self.scene.validate()?;
        ensure(
            self.link_placements.is_empty() && self.scene == self.placement_scene,
            "select pie label fonts before measured placement and Scene customization",
        )?;
        let mut result = self.clone();
        let font = font.inherit(&self.base_scene.typography.default);
        result.scene.typography.default = font.clone();
        result.base_scene.typography.default = font.clone();
        result.placement_scene.typography.default = font.clone();
        let resolved = font.resolve(&crate::ResolvedFont::default());
        for candidate in &mut result.link_candidates {
            ensure(
                signature(
                    &candidate.label,
                    &candidate.points,
                    &candidate.series,
                    &candidate.font,
                ) == candidate.signature,
                "mutated pie measurement candidate",
            )?;
            candidate.font = resolved.clone();
            candidate.signature = signature(
                &candidate.label,
                &candidate.points,
                &candidate.series,
                &candidate.font,
            );
        }
        result.scene.validate()?;
        Ok(result)
    }

    /// Bind original slice values to the actual arc Paths. Logical positions
    /// use each active slice's real radial midpoint and allocated angle midpoint.
    /// Source metadata, path topology and business identity edits are rejected.
    /// Paint-only visibility changes can omit ink without manufacturing sites.
    pub fn interaction_binding(&self) -> Result<InteractionBinding> {
        self.scene.validate()?;
        let source = &self.interaction_source;
        ensure(
            self.center == source.center
                && self.radius == source.radius
                && self.inner_radius == source.inner_radius
                && self.bounds == source.bounds
                && self.slices == source.slices,
            "pie interaction source metadata was changed",
        )?;
        let mut binding = InteractionBinding::default();
        let mark_ids = self
            .scene
            .marks
            .iter()
            .map(|mark| mark.id.as_str())
            .collect::<BTreeSet<_>>();
        for slice in &self.slices {
            let Some(index) = slice.mark_index else {
                ensure(
                    !mark_ids.contains(slice.id.as_str()),
                    "collapsed pie source gained a datum mark",
                )?;
                continue;
            };
            let mark = self
                .scene
                .marks
                .get(index)
                .ok_or_else(|| ChartError("missing pie interaction mark".into()))?;
            let original = self
                .base_scene
                .marks
                .get(index)
                .ok_or_else(|| ChartError("missing pie source mark".into()))?;
            ensure(
                mark.id == slice.id
                    && mark.series == slice.id
                    && mark.value == slice.value
                    && mark.label == original.label
                    && mark.geometry == original.geometry,
                "pie interaction mark disagrees with its source",
            )?;
            let angle = slice.start_angle + (slice.end_angle - slice.start_angle) / 2.;
            let radius = (slice.inner_radius + slice.outer_radius) / 2.;
            let position = [
                self.center[0] + radius * angle.sin(),
                self.center[1] - radius * angle.cos(),
            ];
            if let Some((site, anchor)) =
                arc_interaction(mark, position, !slice.outer_collapsed, None)?
            {
                binding.anchors.insert(mark.id.clone(), anchor);
                binding.sites.push(site);
            }
        }
        binding.validate(&self.scene)?;
        Ok(binding)
    }

    /// The base sector's fitting bounds exclude active radius extensions.
    /// Hosts receive the complete Scene viewport without an extra plot crop.
    pub fn into_scene_interaction(self) -> Result<SceneInteraction> {
        Ok(SceneInteraction {
            interaction: Some(self.interaction_binding()?),
            scene: self.scene,
            plot: None,
        })
    }

    /// Pure two-stage placement: bad/stale/incomplete measurements or exhausted
    /// work return Err without changing this layout. Hidden labels lose leaders
    /// too. Greedy placement is deterministic, not a global crossing optimizer.
    /// Place before editing/appending custom Scene layers. Any edit to the public
    /// `scene` since the last successful layout/placement makes this method return
    /// Err; custom data is never silently discarded. Repeated placement on an
    /// unchanged returned layout is supported.
    pub fn place_link_labels(
        &self,
        measurements: &[PieMeasuredLinkLabel],
        options: &PieLinkPlacementOptions,
    ) -> Result<Self> {
        ensure(
            self.link_candidates.len() <= 256
                && self.slices.len() <= 256
                && self.link_placements.len() <= 256,
            "pie layout metadata budget exceeded",
        )?;
        self.scene.validate()?;
        ensure(
            self.scene == self.placement_scene,
            "place pie labels before customizing the Scene; authored edits cannot be discarded",
        )?;
        ensure(
            self.slices.iter().all(|s| s.id.len() <= 1000)
                && self
                    .link_placements
                    .iter()
                    .all(|p| p.id.len() <= MAX_LABEL_BYTES),
            "pie metadata identity budget exceeded",
        )?;
        ensure(
            self.link_candidates.iter().all(|c| {
                c.id.len() <= MAX_LABEL_BYTES
                    && c.series.len() <= 1000
                    && c.signature.len() <= 8192
                    && c.label.text.len() <= MAX_LABEL_BYTES
            }),
            "pie measurement candidate byte budget exceeded",
        )?;
        ensure(
            measurements.len() == self.link_candidates.len(),
            "pie measurements must cover the current candidates exactly",
        )?;
        ensure(
            options.y_offsets.len() <= 16
                && options
                    .y_offsets
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1e6),
            "invalid pie link displacement budget",
        )?;
        let mut supplied = BTreeMap::new();
        for m in measurements {
            ensure(
                supplied.insert(m.id.as_str(), m).is_none(),
                "duplicate pie label measurement",
            )?;
            m.bounds.validate()?;
        }
        let mut labels = Vec::with_capacity(measurements.len());
        for c in &self.link_candidates {
            c.font.validate()?;
            ensure(
                signature(&c.label, &c.points, &c.series, &c.font) == c.signature
                    && c.font
                        == self
                            .base_scene
                            .typography
                            .default
                            .resolve(&crate::ResolvedFont::default()),
                "mutated pie measurement candidate",
            )?;
            let m = supplied
                .get(c.id.as_str())
                .ok_or_else(|| ChartError("missing current pie label measurement".into()))?;
            ensure(
                m.signature == c.signature,
                "stale pie label measurement signature",
            )?;
            let rect = LayoutRect {
                x: c.label.x + m.bounds.x,
                y: c.label.y + m.bounds.y,
                ..m.bounds
            };
            rect.validate()?;
            labels.push(LabelCandidate {
                id: c.id.clone(),
                rect,
                priority: 0,
                offsets: options.y_offsets.iter().map(|y| (0., *y)).collect(),
            });
        }
        let placements = layout_labels(&labels, &options.collision)?;
        let mut result = self.clone();
        result.scene = self.base_scene.clone();
        result.link_placements.clear();
        for (i, (c, p)) in self.link_candidates.iter().zip(placements).enumerate() {
            let mut placed = PiePlacedLinkLabel {
                id: c.id.clone(),
                bounds: p.rect,
                label_index: None,
                leader_index: None,
            };
            if let Some(rect) = p.rect {
                let dx = rect.x - labels[i].rect.x;
                let dy = rect.y - labels[i].rect.y;
                let mut label = c.label.clone();
                label.x += dx;
                label.y += dy;
                if let Some(mut leader) = c.leader.clone() {
                    let mut points = c.points;
                    points[1].y += dy;
                    points[2].x += dx;
                    points[2].y += dy;
                    leader.geometry = Geometry::Path(vec![
                        C::Move(points[0]),
                        C::Line(points[1]),
                        C::Line(points[2]),
                    ]);
                    placed.leader_index = Some(result.scene.marks.len());
                    result.scene.marks.push(leader);
                }
                placed.label_index = Some(result.scene.labels.len());
                push_label(&mut result.scene, label, &c.series);
            }
            result.link_placements.push(placed);
        }
        result.scene.validate()?;
        result.placement_scene = result.scene.clone();
        Ok(result)
    }
}

pub(crate) fn arc_interaction(
    mark: &Mark,
    position: [f64; 2],
    has_area: bool,
    formatted_value: Option<String>,
) -> Result<Option<(InteractionSite, InteractionAnchor)>> {
    let fill = if has_area {
        match &mark.fill_paint {
            Some(FillPaint::Pattern(pattern)) => {
                let size = match pattern.shape {
                    PatternShape::Dots { diameter, .. } => diameter,
                    PatternShape::Squares { size, .. } => size,
                    PatternShape::Lines { width } => width,
                };
                if pattern.background.3 > 0 {
                    Some(pattern.background)
                } else {
                    (size > 0. && pattern.foreground.3 > 0).then_some(pattern.foreground)
                }
            }
            Some(paint) => paint
                .stops()
                .iter()
                .find(|stop| stop.color.3 > 0)
                .map(|stop| stop.color),
            None => mark.fill.filter(|color| color.3 > 0),
        }
    } else {
        None
    };
    let stroke = mark
        .stroke
        .filter(|stroke| stroke.width > 0. && stroke.color.3 > 0)
        .map(|stroke| stroke.color);
    if mark.opacity == 0. || mark.label.is_empty() {
        return Ok(None);
    }
    let Some(color) = fill.or(stroke) else {
        return Ok(None);
    };
    let Geometry::Path(path) = &mark.geometry else {
        return Err(ChartError("radial interaction requires an arc Path".into()));
    };
    let points = path
        .iter()
        .enumerate()
        .filter_map(|(index, command)| (!matches!(command, C::Close)).then_some(index))
        .collect::<Vec<_>>();
    let anchor = InteractionAnchor::path_offset_following(&mark.geometry, position, &points)?;
    Ok(Some((
        InteractionSite {
            mark_id: mark.id.clone(),
            series: mark.series.clone(),
            label: mark.label.clone(),
            value: Some(mark.value),
            formatted_value,
            color,
            position,
        },
        anchor,
    )))
}

fn point(center: [f64; 2], r: f64, a: f64) -> P {
    P::new(center[0] + r * a.sin(), center[1] - r * a.cos())
}
fn push_label(s: &mut Scene, label: Label, series: &str) {
    let slot = s.labels.len();
    s.labels.push(label);
    s.label_series.insert(slot, series.into());
}
fn signature(label: &Label, points: &[P; 3], series: &str, font: &crate::ResolvedFont) -> String {
    let mut s = format!(
        "{}:{}:{}:{}:{}:{:?}:{}",
        series.len(),
        series,
        label.text.len(),
        label.text,
        label.size.to_bits(),
        label.anchor,
        label.rotation.to_bits()
    );
    for v in [
        label.x,
        label.y,
        points[0].x,
        points[0].y,
        points[1].x,
        points[1].y,
        points[2].x,
        points[2].y,
    ] {
        s.push_str(&format!(":{:016x}", if v == 0. { 0 } else { v.to_bits() }));
    }
    s.push(':');
    s.push_str(&font.signature());
    s
}
fn frame(c: &ChartConfig, o: &PieLayoutOptions) -> Result<([f64; 2], f64, ClipRect)> {
    let mut center = [
        c.margin.left + c.inner_width() / 2.,
        c.margin.top + c.inner_height() / 2.,
    ];
    let mut r = c.inner_width().min(c.inner_height()) / 2.;
    let start = o.pie.start_angle;
    let end = o.pie.end_angle;
    let mut points = vec![
        P::new(0., 0.),
        point([0., 0.], 1., start),
        point([0., 0.], 1., end),
    ];
    let lo = start.min(end);
    let hi = start.max(end);
    let k0 = (lo / FRAC_PI_2).ceil() as i64;
    let k1 = (hi / FRAC_PI_2).floor() as i64;
    for k in k0..=k1 {
        points.push(point([0., 0.], 1., k as f64 * FRAC_PI_2));
    }
    let xmin = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
    let xmax = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
    let ymin = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
    let ymax = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
    if o.fit == PieFit::NivoSector {
        let w = xmax - xmin;
        let h = ymax - ymin;
        ensure(
            w > 0. && h > 0.,
            "pie fit sector collapses at floating-point precision",
        )?;
        r = (c.inner_width() / w).min(c.inner_height() / h);
        center = [
            c.margin.left + (c.inner_width() - (xmax - xmin) * r) / 2. - xmin * r,
            c.margin.top + (c.inner_height() - (ymax - ymin) * r) / 2. - ymin * r,
        ];
    }
    let bounds = ClipRect {
        x: center[0] + xmin * r,
        y: center[1] + ymin * r,
        width: (xmax - xmin) * r,
        height: (ymax - ymin) * r,
    };
    bounds.validate()?;
    ensure(
        r.is_finite() && r <= 1e6,
        "pie fit radius exceeds coordinate profile",
    )?;
    Ok((center, r, bounds))
}
struct LinkGeometry {
    center: [f64; 2],
    outer: f64,
    middle: f64,
    fraction: f64,
    color: Color,
}
fn candidate(
    c: &ChartConfig,
    d: &RadialDatum,
    style: &PieStyleOptions,
    g: LinkGeometry,
    colors: &PieColorStyles,
) -> Result<PieLinkLabelCandidate> {
    let LinkGeometry {
        center,
        outer,
        middle,
        fraction,
        color,
    } = g;
    let l = &style.link_labels;
    let side = if middle.sin() >= 0. { 1. } else { -1. };
    let first = point(center, outer + l.offset, middle);
    let elbow = point(center, outer + l.offset + l.diagonal_length, middle);
    let end = P::new(elbow.x + side * l.straight_length, elbow.y);
    let outside = InheritedColorContext {
        inherited: color,
        theme: c.theme.text,
        surface: Color(0, 0, 0, 0),
        backdrop: c.theme.background,
    };
    let label = Label {
        x: end.x + side * l.text_offset,
        y: end.y,
        text: l.text.text(d, fraction)?,
        size: l.style.size.unwrap_or(c.theme.font_size),
        color: resolve_color(
            colors.link_label.as_ref(),
            l.style.color.unwrap_or(c.theme.text),
            outside,
        )?,
        anchor: if side > 0. {
            TextAnchor::Start
        } else {
            TextAnchor::End
        },
        rotation: l.style.rotation,
    };
    let id = format!("pie-layout-link:{}:{}", d.id.len(), d.id);
    let leader = if l.thickness > 0. && (l.diagonal_length > 0. || l.straight_length > 0.) {
        let fallback = match l.color {
            PieLinkColor::Theme => c.theme.grid,
            PieLinkColor::Slice => color,
            PieLinkColor::Fixed(v) => v,
        };
        let ink = resolve_color(
            colors.link_line.as_ref(),
            fallback,
            InheritedColorContext {
                theme: c.theme.grid,
                ..outside
            },
        )?;
        let mut m = Mark::new(
            &id,
            Geometry::Path(vec![C::Move(first), C::Line(elbow), C::Line(end)]),
            ink,
        );
        m.fill = None;
        m.stroke = Some(GraphicStroke {
            color: ink,
            width: l.thickness,
            cap: StrokeCap::Butt,
        });
        m.series = d.id.clone();
        m.label.clear();
        Some(m)
    } else {
        None
    };
    let points = [first, elbow, end];
    let font = c.theme.font.resolve(&crate::ResolvedFont::default());
    let signature = signature(&label, &points, &d.id, &font);
    Ok(PieLinkLabelCandidate {
        id,
        series: d.id.clone(),
        signature,
        label,
        font,
        points,
        leader,
        text_offset: l.text_offset,
    })
}

fn resolve_color(
    profile: Option<&InheritedColor>,
    fallback: Color,
    context: InheritedColorContext,
) -> Result<Color> {
    profile.map_or(Ok(fallback), |color| color.resolve_validated(context))
}

/// Independent pie model. All supplied values are finite/nonnegative and <=256
/// identities; hidden values are removed before allocation, while visible zero
/// values reserve D3 padding. All-zero sources use padding-only allocation.
pub fn pie_layout(
    c: &ChartConfig,
    data: &[RadialDatum],
    o: &PieLayoutOptions,
) -> Result<PieLayout> {
    pie_layout_with_color_factory(c, data, o, &PieColorStyles::default(), |_, fallback| {
        Ok(fallback)
    })
}

/// Resolve slice colors from stable datum IDs. Missing colors use the theme.
pub fn pie_layout_with_ordinal_colors(
    c: &ChartConfig,
    data: &[RadialDatum],
    o: &PieLayoutOptions,
    palette: &OrdinalColorScale,
    styles: &PieColorStyles,
) -> Result<PieLayout> {
    pie_layout_with_color_factory(c, data, o, styles, |datum, fallback| {
        Ok(palette
            .map(&CategoryKey::from(datum.id.as_str()))
            .unwrap_or(fallback))
    })
}

/// Resolve color once per supplied datum in source order, including hidden
/// slices. A returned color feeds the legend, fill and inherited role colors.
pub fn pie_layout_with_color_factory(
    c: &ChartConfig,
    data: &[RadialDatum],
    o: &PieLayoutOptions,
    colors: &PieColorStyles,
    mut color: impl FnMut(&RadialDatum, Color) -> Result<Color>,
) -> Result<PieLayout> {
    let mut scene = c.scene()?;
    crate::radial::validate_data(data)?;
    o.pie.style.validate(data)?;
    colors.validate()?;
    let p = &o.pie;
    let span = p.end_angle - p.start_angle;
    ensure(
        p.inner_radius.is_finite() && (0. ..1.).contains(&p.inner_radius),
        "pie inner radius must be in [0,1)",
    )?;
    ensure(
        p.start_angle.is_finite()
            && p.end_angle.is_finite()
            && p.start_angle.abs() <= 1e6
            && p.end_angle.abs() <= 1e6
            && span.abs() > 0.
            && span.abs() <= TAU + 1e-12,
        "pie angular extent must be in (0,2pi]",
    )?;
    ensure(
        p.pad_angle.is_finite() && (0. ..=TAU).contains(&p.pad_angle),
        "invalid pie pad angle",
    )?;
    ensure(
        p.hidden.len() <= MAX_MARKS
            && p.hidden
                .iter()
                .all(|id| !id.trim().is_empty() && id.len() <= MAX_LABEL_BYTES),
        "invalid pie hidden IDs",
    )?;
    let pad_radius = match o.padding {
        PiePadding::LegacyAngularGap => None,
        PiePadding::D3Applied { pad_radius } | PiePadding::NivoRequested { pad_radius } => {
            match pad_radius {
                PiePadRadius::Auto => None,
                PiePadRadius::Fixed(r) => {
                    ensure(
                        r.is_finite() && (0. ..=1e6).contains(&r),
                        "invalid pie pad radius",
                    )?;
                    Some(r)
                }
            }
        }
    };
    let (center, radius, bounds) = frame(c, o)?;
    let inner_radius = radius * p.inner_radius;
    scene.legend = data
        .iter()
        .enumerate()
        .map(|(i, d)| {
            Ok(LegendItem {
                id: d.id.clone(),
                label: d.label.clone(),
                color: color(d, c.theme.color(i))?,
            })
        })
        .collect::<Result<_>>()?;
    let mut order = (0..data.len())
        .filter(|i| {
            !p.hidden.contains(&data[*i].id)
                && (o.padding != PiePadding::LegacyAngularGap || data[*i].value > 0.)
        })
        .collect::<Vec<_>>();
    if p.sort_by_value {
        order.sort_by(|a, b| data[*b].value.total_cmp(&data[*a].value));
    }
    let max = order.iter().map(|i| data[*i].value).fold(0., f64::max);
    let sum = if max > 0. {
        order.iter().map(|i| data[*i].value / max).sum::<f64>()
    } else {
        0.
    };
    let applied_pad = if order.is_empty() {
        0.
    } else {
        p.pad_angle.min(span.abs() / order.len() as f64)
    };
    let available = (span.abs() - applied_pad * order.len() as f64).max(0.);
    let direction = span.signum();
    let mut angle = p.start_angle;
    let mut slices = Vec::with_capacity(order.len());
    let mut links = Vec::with_capacity(order.len());
    let mut occupied = data.iter().map(|d| d.id.clone()).collect::<BTreeSet<_>>();
    for (rank, index) in order.into_iter().enumerate() {
        let d = &data[index];
        let fraction = if sum > 0. { d.value / max / sum } else { 0. };
        ensure(
            d.value == 0. || fraction > 0.,
            "positive pie fraction collapses at floating-point precision",
        )?;
        let extent = available * fraction;
        let (start, end) = if o.padding == PiePadding::LegacyAngularGap {
            let start = angle + direction * applied_pad / 2.;
            let end = start + direction * extent;
            angle = end + direction * applied_pad / 2.;
            (start, end)
        } else {
            let start = angle;
            let end = start + direction * (extent + applied_pad);
            angle = end;
            (start, end)
        };
        let active = p.style.active_ids.contains(&d.id);
        let inner = (inner_radius
            - if active {
                p.style.active_inner_radius_offset
            } else {
                0.
            })
        .max(0.);
        let outer = radius
            + if active {
                p.style.active_outer_radius_offset
            } else {
                0.
            };
        ensure(
            outer > 0. && inner < outer,
            "active pie radii must form a positive annulus",
        )?;
        let geometry_pad = match o.padding {
            PiePadding::LegacyAngularGap => 0.,
            PiePadding::D3Applied { .. } => applied_pad,
            PiePadding::NivoRequested { .. } => p.pad_angle,
        };
        let arc = if o.padding == PiePadding::LegacyAngularGap {
            pie_arc::ArcOutput {
                geometry: crate::radial::legacy_pie_sector(
                    center[0],
                    center[1],
                    inner,
                    outer,
                    start,
                    end,
                    p.style.corner_radius,
                ),
                inner_angles: [start, end],
                outer_angles: [start, end],
                inner_collapsed: inner == 0. || extent == 0.,
                outer_collapsed: extent == 0.,
            }
        } else {
            pie_arc::build(pie_arc::ArcInput {
                center,
                inner,
                outer,
                start,
                end,
                pad: geometry_pad,
                pad_radius,
                corner: p.style.corner_radius,
            })?
        };
        let mut slice = PieSliceLayout {
            id: d.id.clone(),
            source_index: index,
            rank,
            value: d.value,
            fraction,
            start_angle: start,
            end_angle: end,
            requested_pad: p.pad_angle,
            applied_pad,
            geometry_pad,
            inner_radius: inner,
            outer_radius: outer,
            inner_angles: arc.inner_angles,
            outer_angles: arc.outer_angles,
            inner_collapsed: arc.inner_collapsed,
            outer_collapsed: arc.outer_collapsed,
            mark_index: None,
        };
        if !arc.outer_collapsed
            || (p.style.border.is_some() && o.padding != PiePadding::LegacyAngularGap)
        {
            let color = scene.legend[index].color;
            let Color(red, green, blue, alpha) = color;
            let fill = Color(
                red,
                green,
                blue,
                (f64::from(alpha) * p.style.fill_opacity).round() as u8,
            );
            let context = InheritedColorContext {
                inherited: color,
                theme: c.theme.text,
                surface: if arc.outer_collapsed {
                    Color(0, 0, 0, 0)
                } else {
                    fill
                },
                backdrop: c.theme.background,
            };
            let mut mark = Mark::new(&d.id, arc.geometry, fill);
            mark.series = d.id.clone();
            mark.label = d.label.clone();
            mark.value = d.value;
            mark.stroke = p.style.border;
            if let Some(stroke) = &mut mark.stroke {
                stroke.color = resolve_color(
                    colors.border.as_ref(),
                    stroke.color,
                    InheritedColorContext {
                        theme: c.theme.grid,
                        ..context
                    },
                )?;
            }
            slice.mark_index = Some(scene.marks.len());
            scene.marks.push(mark);
            let middle = start + (end - start) / 2.;
            let label_extent = (end - start).abs();
            let a = &p.style.arc_labels;
            if p.labels
                && a.enabled
                && label_extent >= a.skip_angle
                && a.radius_measure.measure(inner, outer) >= a.skip_radius
                && (!a.legacy_fit
                    || label_extent * outer > a.style.size.unwrap_or(c.theme.font_size) * 1.5)
            {
                let at = point(center, inner + (outer - inner) * a.radius_offset, middle);
                push_label(
                    &mut scene,
                    Label {
                        x: at.x,
                        y: at.y,
                        text: a.text.text(d, fraction)?,
                        size: a.style.size.unwrap_or(c.theme.font_size),
                        color: resolve_color(
                            colors.arc_label.as_ref(),
                            a.style.color.unwrap_or(c.theme.text),
                            context,
                        )?,
                        anchor: TextAnchor::Middle,
                        rotation: a.style.rotation,
                    },
                    &d.id,
                );
            }
            if p.style.link_labels.enabled && label_extent >= p.style.link_labels.skip_angle {
                let mut link = candidate(
                    c,
                    d,
                    &p.style,
                    LinkGeometry {
                        center,
                        outer,
                        middle,
                        fraction,
                        color,
                    },
                    colors,
                )?;
                let base = link.id.clone();
                let mut salt = 0;
                while occupied.contains(&link.id) {
                    salt += 1;
                    link.id = format!("{base}:{salt}");
                }
                occupied.insert(link.id.clone());
                if let Some(mark) = &mut link.leader {
                    mark.id = link.id.clone();
                }
                links.push(link);
            }
        }
        slices.push(slice);
    }
    scene.validate()?;
    let base_scene = scene.clone();
    for link in &links {
        if let Some(mark) = &link.leader {
            scene.marks.push(mark.clone());
        }
        push_label(&mut scene, link.label.clone(), &link.series);
    }
    scene.validate()?;
    let placement_scene = scene.clone();
    let interaction_source = Arc::new(PieInteractionSource {
        center,
        radius,
        inner_radius,
        bounds,
        slices: slices.clone(),
    });
    Ok(PieLayout {
        scene,
        center,
        radius,
        inner_radius,
        bounds,
        slices,
        link_candidates: links,
        link_placements: vec![],
        base_scene,
        placement_scene,
        interaction_source,
    })
}
pub fn pie_with_layout_options(
    c: &ChartConfig,
    data: &[RadialDatum],
    o: &PieLayoutOptions,
) -> Result<Scene> {
    Ok(pie_layout(c, data, o)?.scene)
}
