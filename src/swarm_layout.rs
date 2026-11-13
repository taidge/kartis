//! Deterministic typed beeswarm. Nivo 0.99.0's `swarmplot/src/compute.ts`
//! uses a pair of positional forces plus forceCollide, so both coordinates can
//! move. This layout fixes the scale coordinate and packs conservative ink
//! discs across disjoint group bands. It rejects an infeasible greedy packing.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};

use crate::scene::ensure;
use crate::*;

mod group_axis;
mod packing;

const MAX_POINTS: usize = 2000;
const MAX_GROUPS: usize = 256;

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmPoint {
    pub id: String,
    pub group: String,
    /// Numeric value or integral UTC epoch milliseconds, according to the scale.
    pub value: f64,
    /// Override the default node size in the selected shape's unit.
    pub size: Option<f64>,
    pub color: Option<Color>,
    pub label: Option<String>,
}

impl SwarmPoint {
    pub fn new(id: impl Into<String>, group: impl Into<String>, value: f64) -> Self {
        Self {
            id: id.into(),
            group: group.into(),
            value,
            size: None,
            color: None,
            label: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmGroup {
    pub id: String,
    pub label: String,
    pub color: Option<Color>,
}

impl SwarmGroup {
    pub fn new(id: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: id.clone(),
            id,
            color: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwarmOrientation {
    #[default]
    Vertical,
    Horizontal,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SwarmValueFit {
    /// Inset the value range by the largest visible ink envelope. Every
    /// displayed value fits within the plot, including at the domain endpoints.
    #[default]
    Inset,
    /// Map values across the full plot and clip marks to it. Source rows whose
    /// entire conservative envelope is off plot have metadata but no mark.
    Clip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwarmNodeShape {
    Circle {
        diameter: f64,
    },
    FilledGlyph {
        shape: FilledGlyph,
        area: f64,
        rotation: f64,
    },
    StrokedGlyph {
        shape: StrokedGlyph,
        size: f64,
        rotation: f64,
    },
}

impl Default for SwarmNodeShape {
    fn default() -> Self {
        Self::Circle { diameter: 6. }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmNodeStyle {
    pub shape: SwarmNodeShape,
    pub fill: Option<Color>,
    pub stroke: Option<GraphicStroke>,
    pub opacity: f64,
}

impl Default for SwarmNodeStyle {
    fn default() -> Self {
        Self {
            shape: Default::default(),
            fill: None,
            stroke: None,
            opacity: 1.,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmLayoutOptions {
    pub orientation: SwarmOrientation,
    /// Explicit visual order, including empty groups. Inferred order is by ID.
    pub groups: Option<Vec<SwarmGroup>>,
    /// Hidden groups retain their band, color slot and legend item.
    pub hidden: BTreeSet<String>,
    pub value_scale: ContinuousScaleOptions,
    /// If omitted, infer both endpoints from the complete source, including
    /// hidden groups. This keeps the value scale stable when toggling groups.
    pub value_domain: Option<[f64; 2]>,
    pub value_fit: SwarmValueFit,
    /// Logical pixels between adjacent group bands.
    pub group_gap: f64,
    /// Minimum distance between ink envelopes, in logical pixels.
    pub spacing: f64,
    pub node: SwarmNodeStyle,
    /// Bottom for horizontal swarms, left for vertical swarms.
    pub value_axis: Option<AxisOptions>,
    /// Left for horizontal swarms, bottom for vertical swarms.
    pub group_axis: Option<CategoryAxisOptions>,
    pub value_format: ValueFormat,
}

impl Default for SwarmLayoutOptions {
    fn default() -> Self {
        Self {
            orientation: Default::default(),
            groups: None,
            hidden: BTreeSet::new(),
            value_scale: ContinuousScaleOptions {
                nice: ScaleNice::Default,
                ..Default::default()
            },
            value_domain: None,
            value_fit: Default::default(),
            group_gap: 0.,
            spacing: 2.,
            node: Default::default(),
            value_axis: Some(AxisOptions {
                grid: true,
                ..Default::default()
            }),
            group_axis: Some(CategoryAxisOptions::default()),
            value_format: ValueFormat::Auto,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmGroupLayout {
    pub id: String,
    pub label: String,
    pub color: Color,
    /// Cross-axis bounds before collision clearance between adjacent groups.
    pub band: [f64; 2],
    pub center: f64,
    pub hidden: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmSite {
    pub mark_id: String,
    pub node_id: String,
    pub group_id: String,
    pub group_index: usize,
    pub source_index: usize,
    pub raw_value: f64,
    pub formatted_value: String,
    pub label: String,
    pub color: Color,
    pub position: [f64; 2],
    pub size: f64,
    /// Radius about the actual glyph origin containing its fill and stroke.
    pub ink_radius: f64,
    pub mark_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SwarmPointMetadata {
    pub mark_id: String,
    pub node_id: String,
    pub group_index: usize,
    pub source_index: usize,
    pub raw_value: f64,
    pub size: f64,
    pub hidden: bool,
    /// Absent for hidden or entirely off-plot points.
    pub site_index: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct SwarmLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    pub value_scale: ResolvedContinuousScale,
    pub groups: Vec<SwarmGroupLayout>,
    pub sites: Vec<SwarmSite>,
    pub anchors: BTreeMap<String, InteractionAnchor>,
    /// Original source order, including hidden and culled points.
    pub metadata: Vec<SwarmPointMetadata>,
}

impl SwarmLayout {
    pub fn interaction_binding(&self) -> InteractionBinding {
        InteractionBinding {
            sites: self
                .sites
                .iter()
                .map(|site| InteractionSite {
                    mark_id: site.mark_id.clone(),
                    series: site.group_id.clone(),
                    label: site.label.clone(),
                    value: Some(site.raw_value),
                    formatted_value: Some(site.formatted_value.clone()),
                    color: site.color,
                    position: site.position,
                })
                .collect(),
            anchors: self.anchors.clone(),
            ..Default::default()
        }
    }
}

impl From<SwarmLayout> for SceneInteraction {
    fn from(layout: SwarmLayout) -> Self {
        let interaction = Some(layout.interaction_binding());
        Self {
            scene: layout.scene,
            plot: Some(layout.plot),
            interaction,
        }
    }
}

#[derive(Clone, Copy)]
struct Coordinates {
    horizontal: bool,
    primary: [f64; 2],
    cross: [f64; 2],
}

impl Coordinates {
    fn new(orientation: SwarmOrientation, plot: ClipRect) -> Self {
        let horizontal = orientation == SwarmOrientation::Horizontal;
        let x = [plot.x, plot.x + plot.width];
        let y = [plot.y, plot.y + plot.height];
        let (primary, cross) = if horizontal { (x, y) } else { (y, x) };
        Self {
            horizontal,
            primary,
            cross,
        }
    }

    fn position(self, primary: f64, cross: f64) -> [f64; 2] {
        if self.horizontal {
            [primary, cross]
        } else {
            [cross, primary]
        }
    }
}

#[derive(Clone)]
enum NodeGeometry {
    Circle { radius: f64 },
    Glyph(GlyphOptions),
}

impl SwarmNodeShape {
    fn geometry(self, size: Option<f64>) -> Result<(NodeGeometry, f64)> {
        let (node, value) = match self {
            Self::Circle { diameter } => {
                let value = size.unwrap_or(diameter);
                let radius = value / 2.;
                ensure(
                    value == 0. || radius > 0.,
                    "swarm diameter cannot be represented",
                )?;
                (NodeGeometry::Circle { radius }, value)
            }
            Self::FilledGlyph {
                shape,
                area,
                rotation,
            } => {
                let value = size.unwrap_or(area);
                (
                    NodeGeometry::Glyph(GlyphOptions {
                        shape: GlyphShape::Filled(shape),
                        size: value,
                        rotation,
                    }),
                    value,
                )
            }
            Self::StrokedGlyph {
                shape,
                size: nominal,
                rotation,
            } => {
                let value = size.unwrap_or(nominal);
                (
                    NodeGeometry::Glyph(GlyphOptions {
                        shape: GlyphShape::Stroked(shape),
                        size: value,
                        rotation,
                    }),
                    value,
                )
            }
        };
        ensure(value.is_finite() && value >= 0., "invalid swarm node size")?;
        Ok((node, value))
    }

    fn stroke_only(self) -> bool {
        matches!(self, Self::StrokedGlyph { .. })
    }
}

impl NodeGeometry {
    fn at(&self, [x, y]: [f64; 2]) -> Result<(Geometry, Vec<usize>)> {
        match self {
            Self::Circle { radius } => {
                ensure(
                    *radius <= 1e6
                        && (*radius == 0.
                            || (x + radius > x
                                && x - radius < x
                                && y + radius > y
                                && y - radius < y)),
                    "swarm circle extent cannot be represented",
                )?;
                Ok((
                    Geometry::Circle {
                        x,
                        y,
                        radius: *radius,
                    },
                    Vec::new(),
                ))
            }
            Self::Glyph(options) => {
                let glyph = glyph_geometry([x, y], options)?;
                Ok((glyph.geometry, glyph.anchor_indices))
            }
        }
    }
}

fn radial_extent(geometry: &Geometry, [x, y]: [f64; 2], stroke: Option<GraphicStroke>) -> f64 {
    let point_extent = |p: &GraphicPoint| (p.x - x).hypot(p.y - y);
    let (extent, path) = match geometry {
        Geometry::Circle { radius, .. } => (*radius, false),
        Geometry::Path(commands) => {
            let mut radius = 0_f64;
            for c in commands {
                match c {
                    PathCommand::Move(p) | PathCommand::Line(p) => {
                        radius = radius.max(point_extent(p))
                    }
                    PathCommand::Cubic { first, second, end } => {
                        radius = radius
                            .max(point_extent(first))
                            .max(point_extent(second))
                            .max(point_extent(end));
                    }
                    PathCommand::Close => {}
                }
            }
            (radius, true)
        }
        Geometry::Rect { .. } => unreachable!("swarm node factory emits circles or paths"),
    };
    let Some(stroke) = stroke.filter(|stroke| stroke.color.3 > 0) else {
        return extent;
    };
    let half = stroke.width / 2.;
    if extent == 0. {
        return match stroke.cap {
            StrokeCap::Butt => 0.,
            StrokeCap::Round => half,
            StrokeCap::Square => std::f64::consts::SQRT_2 * half,
        };
    }
    // Shared SVG/Canvas/CPU paths have miter limit 4. The radius of the
    // convex hull of all supports plus 4 half-widths includes every allowed
    // miter, bevel and cap, even when the glyph is rotated or hollow.
    extent + if path { 4. * half } else { half }
}

fn key(id: &str) -> Result<()> {
    ensure(
        !id.trim().is_empty() && id.len() <= 256,
        "swarm IDs must be nonempty and at most 256 bytes",
    )
}

fn mark_id(group: &str, node: &str) -> String {
    format!("swarm-node:{}:{group}:{}:{node}", group.len(), node.len())
}

fn packing_bands(groups: &[SwarmGroupLayout], spacing: f64) -> Vec<[f64; 2]> {
    let mut bands: Vec<_> = groups.iter().map(|group| group.band).collect();
    for i in 1..bands.len() {
        let gap = bands[i][0] - bands[i - 1][1];
        if gap < spacing {
            let inset = (spacing - gap) / 2.;
            bands[i - 1][1] -= inset;
            bands[i][0] += inset;
            if bands[i][0] - bands[i - 1][1] < spacing {
                bands[i - 1][1] = bands[i - 1][1].next_down();
                bands[i][0] = bands[i][0].next_up();
            }
        }
    }
    bands
}

fn group_layout(
    data: &[SwarmPoint],
    options: &SwarmLayoutOptions,
    coordinates: Coordinates,
    c: &ChartConfig,
) -> Result<Vec<SwarmGroupLayout>> {
    let explicit = options.groups.as_deref();
    ensure(
        explicit.is_none_or(|groups| groups.len() <= MAX_GROUPS),
        "swarm group budget exceeded",
    )?;
    let mut inferred = Vec::new();
    if explicit.is_none() {
        let mut ids = BTreeSet::new();
        for point in data {
            key(&point.group)?;
            ids.insert(point.group.as_str());
            ensure(ids.len() <= MAX_GROUPS, "swarm group budget exceeded")?;
        }
        inferred.extend(ids.into_iter().map(SwarmGroup::new));
    }
    let groups = explicit.unwrap_or(&inferred);
    let mut keys = BTreeSet::new();
    for group in groups {
        key(&group.id)?;
        ensure(
            group.label.len() <= MAX_LABEL_BYTES,
            "swarm group label budget exceeded",
        )?;
        ensure(keys.insert(group.id.as_str()), "duplicate swarm group ID")?;
    }
    ensure(
        options.hidden.iter().all(|key| keys.contains(key.as_str())),
        "unknown hidden swarm group",
    )?;
    if groups.is_empty() {
        return Ok(vec![]);
    }
    let [start, end] = coordinates.cross;
    let size = (end - start - options.group_gap * (groups.len() - 1) as f64) / groups.len() as f64;
    ensure(size > 0., "swarm group gaps exhaust the plot")?;
    let mut layout = Vec::with_capacity(groups.len());
    for (i, group) in groups.iter().enumerate() {
        let lo = start + (size + options.group_gap) * i as f64;
        let hi = if i + 1 == groups.len() {
            end
        } else {
            lo + size
        };
        ensure(
            lo < hi && hi <= end,
            "swarm group band cannot be represented",
        )?;
        let fallback = if options.node.shape.stroke_only() {
            options.node.stroke.map(|stroke| stroke.color)
        } else {
            options.node.fill
        };
        layout.push(SwarmGroupLayout {
            id: group.id.clone(),
            label: group.label.clone(),
            color: group.color.or(fallback).unwrap_or_else(|| c.theme.color(i)),
            band: [lo, hi],
            center: lo + (hi - lo) / 2.,
            hidden: options.hidden.contains(&group.id),
        });
    }
    Ok(layout)
}

struct Prepared {
    group: usize,
    geometry: NodeGeometry,
    fill: Option<Color>,
    stroke: Option<GraphicStroke>,
    radius: f64,
    size: f64,
}

fn prepare(
    data: &[SwarmPoint],
    groups: &[SwarmGroupLayout],
    style: &SwarmNodeStyle,
    kind: ScaleKind,
    coordinates: Coordinates,
) -> Result<Vec<Prepared>> {
    let groups_by_id: BTreeMap<_, _> = groups
        .iter()
        .enumerate()
        .map(|(i, g)| (g.id.as_str(), i))
        .collect();
    let mut identities = BTreeSet::new();
    let mut prepared = Vec::with_capacity(data.len());
    let mut text_bytes = 0usize;
    let stroke_only = style.shape.stroke_only();
    for p in data {
        key(&p.id)?;
        let index = *groups_by_id
            .get(p.group.as_str())
            .ok_or_else(|| ChartError("unknown swarm datum group".into()))?;
        ensure(
            identities.insert((index, p.id.as_str())),
            "duplicate swarm node ID within group",
        )?;
        ensure(p.value.is_finite(), "swarm value must be finite")?;
        if kind == ScaleKind::Utc {
            ensure(
                p.value.fract() == 0. && p.value.abs() <= MAX_TIMESTAMP_MS as f64,
                "swarm UTC value must be integral and bounded",
            )?;
        }
        ensure(
            p.label.as_ref().is_none_or(|l| l.len() <= MAX_LABEL_BYTES),
            "swarm datum label budget exceeded",
        )?;
        text_bytes += p.group.len() + p.id.len() + p.label.as_ref().map_or(0, String::len);
        ensure(text_bytes <= 4_000_000, "swarm source text budget exceeded")?;
        let (geometry, size) = style.shape.geometry(p.size)?;
        let (local, _) = geometry.at([0., 0.])?;
        let color = p.color.unwrap_or(groups[index].color);
        let fill = (!stroke_only).then_some(color);
        let mut stroke = style.stroke;
        if stroke_only && let Some(stroke) = &mut stroke {
            stroke.color = color;
            ensure(
                color.3 > 0,
                "swarm stroke-only node must have visible stroke color",
            )?;
        }
        let has_ink = style.opacity > 0.
            && (fill.is_some_and(|f| f.3 > 0) || stroke.is_some_and(|s| s.color.3 > 0));
        let mut radius = if has_ink {
            radial_extent(&local, [0., 0.], stroke)
        } else {
            0.
        };
        if radius > 0. && matches!(local, Geometry::Path(_)) {
            // When a local path is translated, addition and subtraction can
            // expand its recovered norm by a few world-coordinate ULPs. Any
            // accepted ink disc has its center within one radius of the plot.
            let coordinate_bound = coordinates.primary[1].max(coordinates.cross[1]);
            radius += 16. * f64::EPSILON * (coordinate_bound + radius + 1.);
        }
        ensure(radius.is_finite(), "swarm ink envelope overflow")?;
        prepared.push(Prepared {
            group: index,
            geometry,
            fill,
            stroke,
            radius,
            size,
        });
    }
    Ok(prepared)
}

fn resolve_scale(
    data: &[SwarmPoint],
    prepared: &[Prepared],
    options: &SwarmLayoutOptions,
    coordinates: Coordinates,
) -> Result<ResolvedContinuousScale> {
    let fallback = match options.value_scale.kind {
        kind if kind.requires_nonzero_domain() => [1., 10.],
        ScaleKind::Utc => [0., 86_400_000.],
        _ => [0., 1.],
    };
    let domain = options.value_domain.unwrap_or_else(|| {
        let Some(first) = data.first() else {
            return fallback;
        };
        data.iter().fold([first.value; 2], |[lo, hi], p| {
            [lo.min(p.value), hi.max(p.value)]
        })
    });
    let mut range = coordinates.primary;
    if options.value_fit == SwarmValueFit::Inset {
        let max = prepared.iter().map(|node| node.radius).fold(0., f64::max);
        range = [range[0] + max, range[1] - max];
        if options.value_scale.round {
            // The rounded scale must still fit its promised endpoint ink.
            range = [range[0].ceil(), range[1].floor()];
        }
        ensure(range[0] <= range[1], "swarm nodes exceed value-axis space")?;
    }
    if !coordinates.horizontal {
        range.reverse();
    }
    ResolvedContinuousScale::new(domain, range, &options.value_scale)
}

fn validate_options(o: &SwarmLayoutOptions) -> Result<()> {
    ensure(
        o.hidden.len() <= MAX_GROUPS,
        "swarm hidden group budget exceeded",
    )?;
    ensure(
        o.spacing.is_finite()
            && (0. ..=16384.).contains(&o.spacing)
            && o.group_gap.is_finite()
            && (0. ..=16384.).contains(&o.group_gap),
        "invalid swarm spacing or group gap",
    )?;
    ensure(
        o.node.opacity.is_finite() && (0. ..=1.).contains(&o.node.opacity),
        "invalid swarm node opacity",
    )?;
    ensure(
        o.node
            .stroke
            .is_none_or(|s| s.width.is_finite() && (0.000001..=10000.).contains(&s.width)),
        "invalid swarm stroke width",
    )?;
    if o.node.shape.stroke_only() {
        ensure(
            o.node.fill.is_none() && o.node.stroke.is_some_and(|s| s.color.3 > 0),
            "swarm stroke-only glyph requires a visible stroke and no fill",
        )?;
    }
    o.node.shape.geometry(None)?.0.at([0., 0.])?;
    o.value_scale.validate()?;
    o.value_format.validate()?;
    if let Some(axis) = &o.value_axis {
        axis.validate()?;
    }
    if let Some(axis) = &o.group_axis {
        axis.validate()?;
    }
    Ok(())
}

/// Compute a candidate Scene and typed business metadata in one call. The
/// value coordinate is exactly `layout.value_scale.map(raw_value)`. Only the
/// orthogonal coordinate changes during deterministic greedy packing.
pub fn swarm_layout(
    c: &ChartConfig,
    data: &[SwarmPoint],
    o: &SwarmLayoutOptions,
) -> Result<SwarmLayout> {
    ensure(
        data.len() <= MAX_POINTS,
        "swarm point packing budget exceeded",
    )?;
    validate_options(o)?;
    let mut scene = c.scene()?;
    let plot = ClipRect::plot(c);
    let coordinates = Coordinates::new(o.orientation, plot);
    let groups = group_layout(data, o, coordinates, c)?;
    let prepared = prepare(data, &groups, &o.node, o.value_scale.kind, coordinates)?;
    let value_scale = resolve_scale(data, &prepared, o, coordinates)?;
    let mut lanes = vec![Vec::<(usize, f64)>::new(); groups.len()];
    let mut metadata = Vec::with_capacity(data.len());
    for (i, (point, node)) in data.iter().zip(&prepared).enumerate() {
        let group = &groups[node.group];
        ensure(
            value_scale.chart_scale().accepts(point.value),
            "swarm datum is invalid for selected value scale",
        )?;
        let primary = value_scale.map(point.value);
        ensure(primary.is_finite(), "swarm value projection overflow")?;
        if !group.hidden {
            let [lo, hi] = coordinates.primary;
            match o.value_fit {
                SwarmValueFit::Inset => {
                    // Inset range contains the entire disc for in-domain data.
                    // Compare the same bound arithmetic used for range setup.
                    ensure(
                        primary >= lo + node.radius && primary <= hi - node.radius,
                        "swarm value or ink extends beyond inset plot",
                    )?;
                    lanes[node.group].push((i, primary));
                }
                SwarmValueFit::Clip => {
                    if primary >= lo - node.radius && primary <= hi + node.radius {
                        lanes[node.group].push((i, primary));
                    }
                }
            }
        }
        metadata.push(SwarmPointMetadata {
            mark_id: mark_id(&group.id, &point.id),
            node_id: point.id.clone(),
            group_index: node.group,
            source_index: i,
            raw_value: point.value,
            size: node.size,
            hidden: group.hidden,
            site_index: None,
        });
    }
    if let Some(axis_options) = &o.value_axis {
        axis(
            &mut scene,
            c,
            value_scale.chart_scale(),
            if coordinates.horizontal {
                AxisSide::Bottom
            } else {
                AxisSide::Left
            },
            axis_options,
            "swarm-value-axis",
        )?;
    }
    if let Some(axis_options) = &o.group_axis {
        group_axis::draw(&mut scene, c, coordinates, &groups, axis_options)?;
    }
    scene.legend.extend(groups.iter().map(|group| LegendItem {
        id: group.id.clone(),
        label: group.label.clone(),
        color: group.color,
    }));
    let mut layout = SwarmLayout {
        scene,
        plot,
        value_scale,
        groups,
        sites: Vec::with_capacity(data.len()),
        anchors: BTreeMap::new(),
        metadata,
    };
    let mut intervals = Vec::new();
    let mut placed = Vec::<packing::Placed>::new();
    let bands = packing_bands(&layout.groups, o.spacing);
    for (gi, (mut nodes, lane)) in lanes.into_iter().zip(bands).enumerate() {
        nodes.sort_unstable_by(|(a, x), (b, y)| {
            x.total_cmp(y).then_with(|| data[*a].id.cmp(&data[*b].id))
        });
        let group = &layout.groups[gi];
        placed.clear();
        for (i, primary) in nodes {
            let p = &data[i];
            let n = &prepared[i];
            let cross = packing::place(
                primary,
                n.radius,
                o.spacing,
                lane,
                group.center,
                &placed,
                &mut intervals,
            )?;
            let position = coordinates.position(primary, cross);
            let (geometry, selectors) = n.geometry.at(position)?;
            ensure(
                n.radius == 0. || radial_extent(&geometry, position, n.stroke) <= n.radius,
                "swarm translated glyph exceeds its ink envelope",
            )?;
            let format =
                if o.value_format == ValueFormat::Auto && o.value_scale.kind == ScaleKind::Utc {
                    &ValueFormat::Utc
                } else {
                    &o.value_format
                };
            let formatted_value = format.format(p.value)?;
            let label = format!(
                "{} · {}: {formatted_value}",
                group.label,
                p.label.as_ref().unwrap_or(&p.id)
            );
            ensure(
                label.len() <= MAX_LABEL_BYTES,
                "swarm formatted datum label budget exceeded",
            )?;
            let color = n
                .fill
                .or(n.stroke.map(|s| s.color))
                .expect("node has a representative color");
            let id = layout.metadata[i].mark_id.clone();
            if !selectors.is_empty() {
                layout.anchors.insert(
                    id.clone(),
                    InteractionAnchor::path_points(&geometry, position, &selectors)?,
                );
            }
            let mut mark = Mark::new(id.clone(), geometry, color);
            mark.series = group.id.clone();
            mark.label = label.clone();
            mark.value = p.value;
            mark.fill = n.fill;
            mark.stroke = n.stroke;
            mark.opacity = o.node.opacity;
            mark.clip = Some(plot);
            let mark_index = layout.scene.marks.len();
            layout.scene.marks.push(mark);
            layout.metadata[i].site_index = Some(layout.sites.len());
            layout.sites.push(SwarmSite {
                mark_id: id,
                node_id: p.id.clone(),
                group_id: group.id.clone(),
                group_index: gi,
                source_index: i,
                raw_value: p.value,
                formatted_value,
                label,
                color,
                position,
                size: n.size,
                ink_radius: n.radius,
                mark_index,
            });
            placed.push(packing::Placed {
                primary,
                cross,
                radius: n.radius,
            });
        }
    }
    layout.scene.validate()?;
    layout.interaction_binding().validate(&layout.scene)?;
    Ok(layout)
}
