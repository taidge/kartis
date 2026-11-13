//! Explicit-ID Sankey presentation, using the ordinary and circular solvers.
//!
//! Vertical layout swaps the flow and breadth dimensions *before* solving.
//! Visibility never changes the solved topology, ports, or common flow scale.
//! Links belong to their source series; `hidden` additionally hides links
//! incident to a hidden target. Re-layout with that set for two-ended legends.
use crate::scene::ensure;
use crate::{
    ChartConfig, ChartError, CircularFlowLink, CircularLinkRoute, CircularSankeyMetrics,
    CircularSankeyOptions, ClipRect, Color, FillPaint, FlowLink, FlowNode, Geometry, GradientStop,
    Label, LegendItem, Mark, Result, ReturnSide, SankeyLayoutOptions, Scene, TextAnchor,
    layout_sankey_circular, sankey_with_layout,
};
use glory::render::{GraphicPoint as P, GraphicStroke, PathCommand as Cmd};
use std::collections::{BTreeMap, BTreeSet};

/// Both solvers use the same explicit-ID link input. Parallel links need
/// distinct IDs; input reordering does not change their identity or palette.
pub type SankeyStyleLink = CircularFlowLink;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyNodeStyle {
    pub color: Option<Color>,
    pub opacity: f64,
    pub border: Option<GraphicStroke>,
    /// Clamped to half the solved node breadth and thickness.
    pub corner_radius: f64,
}
impl Default for SankeyNodeStyle {
    fn default() -> Self {
        Self {
            color: None,
            opacity: 1.,
            border: None,
            corner_radius: 0.,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyLinkColor {
    #[default]
    Source,
    Target,
    Fixed(Color),
    /// Actual logical-coordinate source-to-target gradient, including returns.
    Gradient,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SankeyLinkStyle {
    pub color: SankeyLinkColor,
    /// None preserves the original solver's `appearance.link_opacity`.
    pub opacity: Option<f64>,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyLabelPosition {
    /// Ordinary solver: above nodes; circular solver: node centers.
    #[default]
    Legacy,
    /// Facing the interior of the plot, beside the node (Nivo semantics).
    Inside,
    /// Facing the nearest outer flow-axis boundary, beside the node.
    Outside,
    Center,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SankeyLabelOrientation {
    #[default]
    Horizontal,
    Vertical,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyLabelStyle {
    pub position: SankeyLabelPosition,
    pub orientation: SankeyLabelOrientation,
    pub padding: f64,
    pub offset: [f64; 2],
    pub color: Option<Color>,
    pub size: Option<f64>,
}
impl Default for SankeyLabelStyle {
    fn default() -> Self {
        Self {
            position: SankeyLabelPosition::Legacy,
            orientation: SankeyLabelOrientation::Horizontal,
            padding: 9.,
            offset: [0., 0.],
            color: None,
            size: None,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyStyleOptions {
    pub orientation: SankeyOrientation,
    /// Scene legends have a 256-entry budget. Disable for larger DAGs.
    pub legend: bool,
    pub node: SankeyNodeStyle,
    /// Complete per-ID styles, rather than an ambiguous cascade of partials.
    pub nodes: BTreeMap<String, SankeyNodeStyle>,
    pub link: SankeyLinkStyle,
    pub links: BTreeMap<String, SankeyLinkStyle>,
    pub label: SankeyLabelStyle,
    pub labels: BTreeMap<String, SankeyLabelStyle>,
    /// Cubic control distance / flow-axis distance, in 0..=1. Return ribbons
    /// retain their collision-checked orthogonal routing.
    pub forward_curvature: f64,
    /// Node IDs. A hidden node and every incident link lose ink and labels;
    /// metadata and legend stay complete, and no ports or widths are moved.
    pub hidden: BTreeSet<String>,
}
impl Default for SankeyStyleOptions {
    fn default() -> Self {
        Self {
            orientation: SankeyOrientation::Horizontal,
            legend: true,
            node: SankeyNodeStyle::default(),
            nodes: BTreeMap::new(),
            link: SankeyLinkStyle::default(),
            links: BTreeMap::new(),
            label: SankeyLabelStyle::default(),
            labels: BTreeMap::new(),
            forward_curvature: 0.5,
            hidden: BTreeSet::new(),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyStyledNode {
    pub id: String,
    pub mark_id: String,
    pub rect: ClipRect,
    pub inflow: f64,
    pub outflow: f64,
    pub value: f64,
    pub visible: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SankeyReturnSide {
    Top,
    Bottom,
    Left,
    Right,
}
#[derive(Clone, Debug, PartialEq)]
pub enum SankeyStyledRoute {
    Forward,
    Return {
        side: SankeyReturnSide,
        lane: usize,
        coordinate: f64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyStyledLink {
    pub id: String,
    pub mark_id: String,
    pub source: String,
    pub target: String,
    pub value: f64,
    pub width: f64,
    pub source_port: [f64; 2],
    pub target_port: [f64; 2],
    pub route: SankeyStyledRoute,
    /// Exact cubic centerline for forward ribbons, polyline for returns.
    pub centerline: Geometry,
    pub visible: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SankeyStyledLayout {
    pub scene: Scene,
    pub nodes: Vec<SankeyStyledNode>,
    pub links: Vec<SankeyStyledLink>,
    pub circular_metrics: Option<CircularSankeyMetrics>,
}

fn validate_style(
    o: &SankeyStyleOptions,
    nodes: &[FlowNode],
    links: &[SankeyStyleLink],
) -> Result<()> {
    ensure(
        nodes.len() <= 2_000 && links.len() <= 8_000,
        "styled Sankey input exceeds budget",
    )?;
    ensure(
        !o.legend || nodes.len() <= 256,
        "styled Sankey legend exceeds 256 entries; disable its legend for larger DAGs",
    )?;
    let node_ids: BTreeSet<_> = nodes.iter().map(|n| n.id.as_str()).collect();
    let mut link_ids = BTreeSet::new();
    for l in links {
        ensure(
            !l.id.trim().is_empty() && l.id.len() <= 256 && link_ids.insert(l.id.as_str()),
            "styled Sankey links need unique IDs of 1..=256 bytes",
        )?;
    }
    ensure(
        o.nodes.len() <= nodes.len()
            && o.labels.len() <= nodes.len()
            && o.hidden.len() <= nodes.len()
            && o.links.len() <= links.len(),
        "Sankey style map exceeds input budget",
    )?;
    ensure(
        o.nodes
            .keys()
            .chain(o.labels.keys())
            .chain(o.hidden.iter())
            .all(|id| node_ids.contains(id.as_str()))
            && o.links.keys().all(|id| link_ids.contains(id.as_str())),
        "Sankey style refers to an unknown ID",
    )?;
    ensure(
        o.forward_curvature.is_finite() && (0. ..=1.).contains(&o.forward_curvature),
        "invalid Sankey curvature",
    )?;
    for s in std::iter::once(&o.node).chain(o.nodes.values()) {
        ensure(
            s.opacity.is_finite()
                && (0. ..=1.).contains(&s.opacity)
                && s.corner_radius.is_finite()
                && (0. ..=1_000_000.).contains(&s.corner_radius),
            "invalid Sankey node style",
        )?;
        if let Some(b) = &s.border {
            ensure(
                b.width.is_finite() && (0.000001..=10000.).contains(&b.width),
                "invalid Sankey node border",
            )?;
        }
    }
    for s in std::iter::once(&o.link).chain(o.links.values()) {
        ensure(
            s.opacity
                .is_none_or(|v| v.is_finite() && (0. ..=1.).contains(&v)),
            "invalid Sankey link opacity",
        )?;
    }
    for s in std::iter::once(&o.label).chain(o.labels.values()) {
        ensure(
            s.padding.is_finite()
                && (0. ..=1_000_000.).contains(&s.padding)
                && s.offset
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.)
                && s.size
                    .is_none_or(|v| v.is_finite() && (1. ..=256.).contains(&v)),
            "invalid Sankey label style",
        )?;
    }
    Ok(())
}
fn canonical_config(c: &ChartConfig, orientation: SankeyOrientation) -> Result<ChartConfig> {
    c.validate()?;
    let mut canonical = c.clone();
    if orientation == SankeyOrientation::Vertical {
        std::mem::swap(&mut canonical.width, &mut canonical.height);
        canonical.margin = crate::Margin {
            left: c.margin.top,
            right: c.margin.bottom,
            top: c.margin.left,
            bottom: c.margin.right,
        };
    }
    Ok(canonical)
}
fn point(p: [f64; 2], vertical: bool) -> [f64; 2] {
    if vertical { [p[1], p[0]] } else { p }
}
fn geometry(g: Geometry, vertical: bool) -> Geometry {
    if !vertical {
        return g;
    }
    let swap = |p: P| P::new(p.y, p.x);
    match g {
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Geometry::Rect {
            x: y,
            y: x,
            width: height,
            height: width,
        },
        Geometry::Circle { x, y, radius } => Geometry::Circle { x: y, y: x, radius },
        Geometry::Path(p) => Geometry::Path(
            p.into_iter()
                .map(|cmd| match cmd {
                    Cmd::Move(p) => Cmd::Move(swap(p)),
                    Cmd::Line(p) => Cmd::Line(swap(p)),
                    Cmd::Cubic { first, second, end } => Cmd::Cubic {
                        first: swap(first),
                        second: swap(second),
                        end: swap(end),
                    },
                    Cmd::Close => Cmd::Close,
                })
                .collect(),
        ),
    }
}
fn rect(r: ClipRect, vertical: bool) -> ClipRect {
    if vertical {
        ClipRect {
            x: r.y,
            y: r.x,
            width: r.height,
            height: r.width,
        }
    } else {
        r
    }
}
fn identity(role: &str, id: &str) -> String {
    format!("sankey-{role}:{}:{id}", id.len())
}
fn forward(source: [f64; 2], target: [f64; 2], width: f64, curvature: f64) -> (Geometry, Geometry) {
    let [x0, y0] = source;
    let [x1, y1] = target;
    let a = x0 + (x1 - x0) * curvature;
    let b = x1 - (x1 - x0) * curvature;
    let h = width / 2.;
    (
        Geometry::Path(vec![
            Cmd::Move(P::new(x0, y0 - h)),
            Cmd::Cubic {
                first: P::new(a, y0 - h),
                second: P::new(b, y1 - h),
                end: P::new(x1, y1 - h),
            },
            Cmd::Line(P::new(x1, y1 + h)),
            Cmd::Cubic {
                first: P::new(b, y1 + h),
                second: P::new(a, y0 + h),
                end: P::new(x0, y0 + h),
            },
            Cmd::Close,
        ]),
        Geometry::Path(vec![
            Cmd::Move(P::new(x0, y0)),
            Cmd::Cubic {
                first: P::new(a, y0),
                second: P::new(b, y1),
                end: P::new(x1, y1),
            },
        ]),
    )
}

/// Styled DAG layout. Input is sorted by stable ID before relaxation and port
/// ordering. Existing `sankey*` functions retain their input-order defaults.
pub fn sankey_with_style(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[SankeyStyleLink],
    layout: &SankeyLayoutOptions,
    style: &SankeyStyleOptions,
) -> Result<SankeyStyledLayout> {
    validate_style(style, nodes, links)?;
    let canonical = canonical_config(c, style.orientation)?;
    let mut nodes = nodes.to_vec();
    nodes.sort_by(|a, b| a.id.cmp(&b.id));
    let mut links = links.to_vec();
    links.sort_by(|a, b| a.id.cmp(&b.id));
    let legacy: Vec<_> = links
        .iter()
        .map(|l| FlowLink {
            source: l.source.clone(),
            target: l.target.clone(),
            value: l.value,
        })
        .collect();
    let source = sankey_with_layout(&canonical, &nodes, &legacy, layout)?;
    let mut marks: BTreeMap<_, _> = source
        .marks
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    let mut laid_nodes = vec![];
    for n in &nodes {
        let m = marks
            .remove(&format!("node:{}", n.id))
            .ok_or_else(|| ChartError("missing solved Sankey node".into()))?;
        let Geometry::Rect {
            x,
            y,
            width,
            height,
        } = m.geometry
        else {
            return Err(ChartError("invalid solved Sankey node".into()));
        };
        let inflow: f64 = links
            .iter()
            .filter(|l| l.target == n.id)
            .map(|l| l.value)
            .sum();
        let outflow: f64 = links
            .iter()
            .filter(|l| l.source == n.id)
            .map(|l| l.value)
            .sum();
        laid_nodes.push(SankeyStyledNode {
            id: n.id.clone(),
            mark_id: identity("node", &n.id),
            rect: ClipRect {
                x,
                y,
                width,
                height,
            },
            inflow,
            outflow,
            value: m.value,
            visible: true,
        });
    }
    let by_id: BTreeMap<_, _> = laid_nodes.iter().map(|n| (n.id.as_str(), n)).collect();
    let mut laid_links = vec![];
    let mut link_marks = vec![];
    for (i, l) in links.iter().enumerate() {
        let s = by_id[l.source.as_str()].rect;
        let t = by_id[l.target.as_str()].rect;
        let (source_port, target_port, width) = if let Some(m) = marks.remove(&format!("flow:{i}"))
        {
            let Geometry::Path(p) = m.geometry else {
                return Err(ChartError("invalid solved Sankey ribbon".into()));
            };
            let (Cmd::Move(a), Cmd::Cubic { end: b, .. }, Cmd::Line(d)) = (&p[0], &p[1], &p[2])
            else {
                return Err(ChartError("invalid solved Sankey ports".into()));
            };
            let width = d.y - b.y;
            ([a.x, a.y + width / 2.], [b.x, b.y + width / 2.], width)
        } else {
            ([s.x + s.width, s.y], [t.x, t.y], 0.)
        };
        ensure(
            l.value == 0. || width > 0.,
            "styled Sankey projected flow underflow",
        )?;
        let (shape, centerline) = forward(source_port, target_port, width, style.forward_curvature);
        let m = Mark::new(identity("link", &l.id), shape, c.theme.color(0));
        laid_links.push(SankeyStyledLink {
            id: l.id.clone(),
            mark_id: m.id.clone(),
            source: l.source.clone(),
            target: l.target.clone(),
            value: l.value,
            width,
            source_port,
            target_port,
            route: SankeyStyledRoute::Forward,
            centerline,
            visible: true,
        });
        link_marks.push(m);
    }
    // The ordinary serializer omits zero-width ribbons. Recover their exact
    // structural port positions using the solver's adjacent-center/ID order.
    for incoming in [false, true] {
        let mut groups: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (i, l) in laid_links.iter().enumerate() {
            groups
                .entry(if incoming {
                    l.target.clone()
                } else {
                    l.source.clone()
                })
                .or_default()
                .push(i);
        }
        for (id, mut indices) in groups {
            indices.sort_by(|a, b| {
                let opposite = |i: usize| {
                    let l = &laid_links[i];
                    let n = by_id[if incoming {
                        l.source.as_str()
                    } else {
                        l.target.as_str()
                    }];
                    n.rect.y + n.rect.height / 2.
                };
                opposite(*a).total_cmp(&opposite(*b)).then(a.cmp(b))
            });
            let r = by_id[id.as_str()].rect;
            let mut cursor = r.y;
            for i in indices {
                let l = &mut laid_links[i];
                let p = if incoming {
                    &mut l.target_port
                } else {
                    &mut l.source_port
                };
                if l.value == 0. {
                    *p = [r.x + if incoming { 0. } else { r.width }, cursor];
                } else {
                    cursor = p[1] + l.width / 2.;
                }
            }
        }
    }
    for (l, m) in laid_links.iter_mut().zip(&mut link_marks) {
        if l.value == 0. {
            let (shape, centerline) =
                forward(l.source_port, l.target_port, 0., style.forward_curvature);
            m.geometry = shape;
            l.centerline = centerline;
        }
    }
    present(
        c,
        &nodes,
        laid_nodes,
        laid_links,
        link_marks,
        None,
        style,
        layout.appearance.labels,
        layout.appearance.link_opacity,
        false,
    )
}

/// Styled cyclic layout. Return paths keep the original solver's true widths,
/// clearance, lane allocation, and ports; vertical return sides are Left/Right.
pub fn circular_sankey_with_style(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[SankeyStyleLink],
    layout: &CircularSankeyOptions,
    style: &SankeyStyleOptions,
) -> Result<SankeyStyledLayout> {
    validate_style(style, nodes, links)?;
    let canonical = canonical_config(c, style.orientation)?;
    let solved = layout_sankey_circular(&canonical, nodes, links, layout)?;
    let mut marks: BTreeMap<_, _> = solved
        .scene
        .marks
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    let laid_nodes = solved
        .nodes
        .into_iter()
        .map(|n| SankeyStyledNode {
            id: n.id.clone(),
            mark_id: identity("node", &n.id),
            rect: n.rect,
            inflow: n.inflow,
            outflow: n.outflow,
            value: n.value,
            visible: true,
        })
        .collect();
    let mut laid_links = vec![];
    let mut link_marks = vec![];
    for l in solved.links {
        let old = format!("circular-link:{}:{}", l.id.len(), l.id);
        let mut m = marks
            .remove(&old)
            .ok_or_else(|| ChartError("missing circular Sankey ribbon".into()))?;
        m.id = identity("link", &l.id);
        let (route, centerline) = match l.route {
            CircularLinkRoute::Forward => {
                let (shape, center) = forward(
                    l.source_port,
                    l.target_port,
                    l.width,
                    style.forward_curvature,
                );
                m.geometry = shape;
                (SankeyStyledRoute::Forward, center)
            }
            CircularLinkRoute::Return {
                side,
                lane,
                center_y,
            } => {
                let side = match (side, style.orientation) {
                    (ReturnSide::Top, SankeyOrientation::Horizontal) => SankeyReturnSide::Top,
                    (ReturnSide::Bottom, SankeyOrientation::Horizontal) => SankeyReturnSide::Bottom,
                    (ReturnSide::Top, SankeyOrientation::Vertical) => SankeyReturnSide::Left,
                    (ReturnSide::Bottom, SankeyOrientation::Vertical) => SankeyReturnSide::Right,
                };
                let center = Geometry::Path(
                    l.centerline
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            if i == 0 {
                                Cmd::Move(P::new(p[0], p[1]))
                            } else {
                                Cmd::Line(P::new(p[0], p[1]))
                            }
                        })
                        .collect(),
                );
                (
                    SankeyStyledRoute::Return {
                        side,
                        lane,
                        coordinate: center_y,
                    },
                    center,
                )
            }
        };
        laid_links.push(SankeyStyledLink {
            id: l.id,
            mark_id: m.id.clone(),
            source: l.source,
            target: l.target,
            value: l.value,
            width: l.width,
            source_port: l.source_port,
            target_port: l.target_port,
            route,
            centerline,
            visible: true,
        });
        link_marks.push(m);
    }
    present(
        c,
        nodes,
        laid_nodes,
        laid_links,
        link_marks,
        Some(solved.metrics),
        style,
        layout.appearance.labels,
        layout.appearance.link_opacity,
        true,
    )
}

#[allow(clippy::too_many_arguments)]
fn present(
    c: &ChartConfig,
    input: &[FlowNode],
    mut nodes: Vec<SankeyStyledNode>,
    mut links: Vec<SankeyStyledLink>,
    marks: Vec<Mark>,
    metrics: Option<CircularSankeyMetrics>,
    style: &SankeyStyleOptions,
    labels: bool,
    opacity: f64,
    circular: bool,
) -> Result<SankeyStyledLayout> {
    let vertical = style.orientation == SankeyOrientation::Vertical;
    let mut scene = c.scene()?;
    let text: BTreeMap<_, _> = input
        .iter()
        .map(|n| (n.id.as_str(), n.label.as_str()))
        .collect();
    let colors: BTreeMap<_, _> = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| {
            (
                n.id.clone(),
                style
                    .nodes
                    .get(&n.id)
                    .unwrap_or(&style.node)
                    .color
                    .unwrap_or(c.theme.color(i)),
            )
        })
        .collect();
    for (mut m, l) in marks.into_iter().zip(&mut links) {
        let s = style.links.get(&l.id).unwrap_or(&style.link);
        m.id = l.mark_id.clone();
        m.series = l.source.clone();
        m.value = l.value;
        m.label = format!(
            "{} → {}: {}",
            text[l.source.as_str()],
            text[l.target.as_str()],
            l.value
        );
        m.opacity = if l.width == 0. {
            0.
        } else {
            s.opacity.unwrap_or(opacity)
        };
        m.fill = Some(match s.color {
            SankeyLinkColor::Target => colors[&l.target],
            SankeyLinkColor::Fixed(color) => color,
            _ => colors[&l.source],
        });
        l.source_port = point(l.source_port, vertical);
        l.target_port = point(l.target_port, vertical);
        l.centerline = geometry(l.centerline.clone(), vertical);
        m.geometry = geometry(m.geometry, vertical);
        if s.color == SankeyLinkColor::Gradient && l.source_port != l.target_port {
            m.fill_paint = Some(FillPaint::Linear {
                start: l.source_port,
                end: l.target_port,
                stops: vec![
                    GradientStop::new(0., colors[&l.source]),
                    GradientStop::new(1., colors[&l.target]),
                ],
            });
        }
        l.visible =
            !style.hidden.contains(&l.source) && !style.hidden.contains(&l.target) && l.width > 0.;
        if l.visible {
            scene.marks.push(m);
        }
    }
    for n in &mut nodes {
        let s = style.nodes.get(&n.id).unwrap_or(&style.node);
        n.rect = rect(n.rect, vertical);
        let r = n.rect;
        let mut m = Mark::new(
            n.mark_id.clone(),
            crate::cartesian::round_bar(
                Geometry::Rect {
                    x: r.x,
                    y: r.y,
                    width: r.width,
                    height: r.height,
                },
                s.corner_radius,
            ),
            colors[&n.id],
        );
        m.stroke = s.border;
        m.opacity = s.opacity;
        m.value = n.value;
        m.series = n.id.clone();
        m.label = format!(
            "{}: {} (in {}, out {})",
            text[n.id.as_str()],
            n.value,
            n.inflow,
            n.outflow
        );
        n.visible = !style.hidden.contains(&n.id);
        if style.legend {
            scene.legend.push(LegendItem {
                id: n.id.clone(),
                label: text[n.id.as_str()].to_owned(),
                color: colors[&n.id],
            });
        }
        if !n.visible {
            continue;
        }
        scene.marks.push(m);
        if labels {
            let s = style.labels.get(&n.id).unwrap_or(&style.label);
            let mut label = node_label(c, r, text[n.id.as_str()], s, vertical, circular);
            label.x += s.offset[0];
            label.y += s.offset[1];
            scene.label_series.insert(scene.labels.len(), n.id.clone());
            scene.labels.push(label);
        }
    }
    scene.validate()?;
    Ok(SankeyStyledLayout {
        scene,
        nodes,
        links,
        circular_metrics: metrics,
    })
}
fn node_label(
    c: &ChartConfig,
    r: ClipRect,
    text: &str,
    s: &SankeyLabelStyle,
    vertical: bool,
    circular: bool,
) -> Label {
    let size = s.size.unwrap_or(c.theme.font_size);
    let rotation = if s.orientation == SankeyLabelOrientation::Vertical {
        -90.
    } else {
        0.
    };
    let mut label = Label {
        x: r.x + r.width / 2.,
        y: r.y + r.height / 2.,
        text: text.into(),
        color: s.color.unwrap_or(c.theme.text),
        size,
        anchor: TextAnchor::Middle,
        rotation,
    };
    match s.position {
        SankeyLabelPosition::Center => {}
        SankeyLabelPosition::Legacy => {
            if !circular {
                if vertical {
                    label.x = r.x - 5.;
                } else {
                    label.y = r.y - 5.;
                }
                // Legacy ordinary labels use an explicit baseline above nodes.
                if !vertical && rotation == 0. {
                    return label;
                }
            }
        }
        SankeyLabelPosition::Inside | SankeyLabelPosition::Outside => {
            let near = if vertical {
                r.y + r.height / 2. < c.margin.top + c.inner_height() / 2.
            } else {
                r.x + r.width / 2. < c.margin.left + c.inner_width() / 2.
            };
            let after = near == (s.position == SankeyLabelPosition::Inside);
            if vertical {
                label.y = if after {
                    r.y + r.height + s.padding
                } else {
                    r.y - s.padding
                };
                if rotation != 0. {
                    label.anchor = if after {
                        TextAnchor::End
                    } else {
                        TextAnchor::Start
                    };
                }
            } else {
                label.x = if after {
                    r.x + r.width + s.padding
                } else {
                    r.x - s.padding
                };
                if rotation == 0. {
                    label.anchor = if after {
                        TextAnchor::Start
                    } else {
                        TextAnchor::End
                    };
                }
            }
        }
    }
    // Explicit visual-center to alphabetic-baseline offset, rotated with text.
    if rotation == 0. {
        label.y += size / 3.;
    } else {
        label.x += size / 3.;
    }
    label
}
