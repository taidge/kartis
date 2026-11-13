//! Cyclic Sankey with explicit feedback ribbons, including self/parallel links.
//!
//! Nivo packages/sankey/src/{hooks,links}.ts uses D3's common node/port scale.
//! The top/bottom feedback routing concept is documented by the primary
//! implementation <https://github.com/tomshanley/d3-sankey-circular>. This is an
//! independent bounded layout: SCCs, weighted feedback ordering, forward-DAG
//! alignment and crossing-reducing barycentric sweeps; no link is removed.
//! Return horizontal bands and column-gap vertical strips are disjoint.
//! Ribbons can cross other ribbons at junction approaches; no global minimum
//! crossing or planar embedding is claimed. All paths retain true flow width.
use std::collections::{BTreeMap, BTreeSet};

use glory::render::{GraphicPoint as P, PathCommand as Cmd};

use crate::scene::ensure;
use crate::{
    ChartConfig, ChartError, ClipRect, FlowNode, Geometry, Label, LegendItem, Mark, Result,
    SankeyAlignment, SankeyOptions, Scene, TextAnchor,
};

pub const MAX_CIRCULAR_NODES: usize = 256;
pub const MAX_CIRCULAR_LINKS: usize = 1024;
#[derive(Clone, Debug, PartialEq)]
pub struct CircularFlowLink {
    pub id: String,
    pub source: String,
    pub target: String,
    pub value: f64,
}
impl CircularFlowLink {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
        value: f64,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            value,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CircularReturnSide {
    #[default]
    Balanced,
    Top,
    Bottom,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CircularSankeyOptions {
    pub appearance: SankeyOptions,
    pub alignment: SankeyAlignment,
    /// Stable barycentric sweeps; candidates are accepted only when the
    /// lexicographic (crossings, vertical distance) quality does not worsen.
    pub iterations: usize,
    /// Minimum clearance between return bands, vertical strips and node bodies.
    pub route_padding: f64,
    pub return_side: CircularReturnSide,
}
impl Default for CircularSankeyOptions {
    fn default() -> Self {
        Self {
            appearance: SankeyOptions::default(),
            alignment: SankeyAlignment::Left,
            iterations: 8,
            route_padding: 6.,
            return_side: CircularReturnSide::Balanced,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReturnSide {
    Top,
    Bottom,
}
#[derive(Clone, Debug, PartialEq)]
pub enum CircularLinkRoute {
    Forward,
    Return {
        side: ReturnSide,
        lane: usize,
        center_y: f64,
    },
}
#[derive(Clone, Debug, PartialEq)]
pub struct CircularNodeLayout {
    pub id: String,
    pub component: usize,
    pub column: usize,
    pub rect: ClipRect,
    pub inflow: f64,
    pub outflow: f64,
    pub value: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CircularLinkLayout {
    pub id: String,
    pub source: String,
    pub target: String,
    pub value: f64,
    pub width: f64,
    /// Centers on the source's right and target's left edge, respectively.
    pub source_port: [f64; 2],
    pub target_port: [f64; 2],
    pub route: CircularLinkRoute,
    /// Return paths use orthogonal mitered ribbons; forward paths use cubic
    /// ribbons. The sampled centerline is provided for routing inspection.
    pub centerline: Vec<[f64; 2]>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CircularSankeyMetrics {
    pub components: usize,
    pub return_links: usize,
    /// Rank crossings for forward link pairs sharing both endpoint columns;
    /// this is an optimization objective, not a global curve intersection count.
    pub forward_crossings_before: usize,
    pub forward_crossings_after: usize,
    /// Unique lane bands never overlap, even if their X intervals overlap.
    pub return_lane_collisions: usize,
    /// Logical pixels per raw unit when representable. Subnormal raw flows can
    /// have finite ribbons even when this quotient would overflow.
    pub pixels_per_unit: Option<f64>,
    pub flow_normalization: f64,
    pub pixels_per_normalized_unit: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CircularSankeyLayout {
    pub scene: Scene,
    pub nodes: Vec<CircularNodeLayout>,
    pub links: Vec<CircularLinkLayout>,
    pub metrics: CircularSankeyMetrics,
}
#[derive(Clone)]
struct Edge {
    id: String,
    s: usize,
    t: usize,
    value: f64,
    mass: f64,
    back: bool,
    side: ReturnSide,
    lane: usize,
}
fn checked_sum(a: f64, b: f64) -> Result<f64> {
    let v = a + b;
    ensure(v.is_finite(), "circular Sankey node flow total overflow")?;
    Ok(v)
}
fn valid_id(id: &str) -> Result<()> {
    ensure(
        !id.is_empty() && id.len() <= 256,
        "circular Sankey IDs must contain 1..=256 bytes",
    )
}
fn components(n: usize, edges: &[Edge]) -> Vec<usize> {
    let mut graph = vec![vec![]; n];
    let mut reverse = vec![vec![]; n];
    for e in edges {
        graph[e.s].push(e.t);
        reverse[e.t].push(e.s);
    }
    let mut seen = vec![false; n];
    let mut order = vec![];
    for root in 0..n {
        if seen[root] {
            continue;
        }
        seen[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, index)) = stack.last_mut() {
            if *index < graph[*node].len() {
                let child = graph[*node][*index];
                *index += 1;
                if !seen[child] {
                    seen[child] = true;
                    stack.push((child, 0));
                }
            } else {
                order.push(*node);
                stack.pop();
            }
        }
    }
    let mut ids = vec![usize::MAX; n];
    let mut c = 0;
    for &root in order.iter().rev() {
        if ids[root] != usize::MAX {
            continue;
        }
        ids[root] = c;
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            for &child in &reverse[node] {
                if ids[child] == usize::MAX {
                    ids[child] = c;
                    stack.push(child);
                }
            }
        }
        c += 1;
    }
    // Number SCCs by lexical first node, independent of DFS incidental order.
    let mut minima = vec![usize::MAX; c];
    for (i, id) in ids.iter().enumerate() {
        minima[*id] = minima[*id].min(i);
    }
    let mut ranked = (0..c).collect::<Vec<_>>();
    ranked.sort_by_key(|i| minima[*i]);
    let mut remap = vec![0; c];
    for (i, old) in ranked.into_iter().enumerate() {
        remap[old] = i;
    }
    ids.into_iter().map(|i| remap[i]).collect()
}
fn feedback_order(n: usize, edges: &[Edge], component: &[usize]) -> Vec<usize> {
    let count = component.iter().max().map_or(0, |v| v + 1);
    let mut groups = vec![BTreeSet::new(); count];
    for (i, c) in component.iter().enumerate() {
        groups[*c].insert(i);
    }
    let mut incoming = vec![BTreeSet::new(); count];
    let mut outgoing = vec![BTreeSet::new(); count];
    for e in edges {
        let (a, b) = (component[e.s], component[e.t]);
        if a != b {
            outgoing[a].insert(b);
            incoming[b].insert(a);
        }
    }
    let mut ready = (0..count)
        .filter(|c| incoming[*c].is_empty())
        .collect::<BTreeSet<_>>();
    let mut order = vec![];
    while let Some(c) = ready.pop_first() {
        let members = &mut groups[c];
        while !members.is_empty() {
            let node = *members
                .iter()
                .max_by(|a, b| {
                    let score = |i: usize| {
                        edges
                            .iter()
                            .filter(|e| {
                                e.s != e.t && members.contains(&e.s) && members.contains(&e.t)
                            })
                            .map(|e| {
                                if e.s == i {
                                    e.mass
                                } else if e.t == i {
                                    -e.mass
                                } else {
                                    0.
                                }
                            })
                            .sum::<f64>()
                    };
                    score(**a).total_cmp(&score(**b)).then_with(|| b.cmp(a))
                })
                .unwrap();
            members.remove(&node);
            order.push(node);
        }
        for &target in &outgoing[c] {
            incoming[target].remove(&c);
            if incoming[target].is_empty() {
                ready.insert(target);
            }
        }
    }
    debug_assert_eq!(order.len(), n);
    order
}
fn columns(order: &[usize], edges: &[Edge], alignment: SankeyAlignment) -> Vec<usize> {
    let n = order.len();
    let mut depth = vec![0usize; n];
    let mut height = vec![0usize; n];
    for &i in order {
        for e in edges.iter().filter(|e| !e.back && e.s == i) {
            depth[e.t] = depth[e.t].max(depth[i] + 1);
        }
    }
    for &i in order.iter().rev() {
        for e in edges.iter().filter(|e| !e.back && e.s == i) {
            height[i] = height[i].max(height[e.t] + 1);
        }
    }
    let last = depth.iter().copied().max().unwrap_or(0);
    (0..n)
        .map(|i| match alignment {
            SankeyAlignment::Left => depth[i],
            SankeyAlignment::Right => last - height[i],
            SankeyAlignment::Justify => {
                if edges.iter().any(|e| !e.back && e.s == i) {
                    depth[i]
                } else {
                    last
                }
            }
            SankeyAlignment::Center => {
                if edges.iter().any(|e| !e.back && e.t == i) {
                    depth[i]
                } else {
                    edges
                        .iter()
                        .filter(|e| !e.back && e.s == i)
                        .map(|e| depth[e.t].saturating_sub(1))
                        .min()
                        .unwrap_or(0)
                }
            }
        })
        .collect()
}
fn ranks(groups: &[Vec<usize>], n: usize) -> Vec<usize> {
    let mut r = vec![0; n];
    for g in groups {
        for (i, node) in g.iter().enumerate() {
            r[*node] = i;
        }
    }
    r
}
fn quality(groups: &[Vec<usize>], col: &[usize], edges: &[Edge], n: usize) -> (usize, usize) {
    let r = ranks(groups, n);
    let mut crossings = 0;
    let mut distance = 0;
    for (i, a) in edges.iter().enumerate() {
        if a.back {
            continue;
        }
        distance += r[a.s].abs_diff(r[a.t]);
        for b in &edges[i + 1..] {
            if !b.back
                && col[a.s] == col[b.s]
                && col[a.t] == col[b.t]
                && a.s != b.s
                && a.t != b.t
                && ((r[a.s] < r[b.s]) != (r[a.t] < r[b.t]))
            {
                crossings += 1;
            }
        }
    }
    (crossings, distance)
}
fn optimize(
    groups: &mut Vec<Vec<usize>>,
    col: &[usize],
    edges: &[Edge],
    iterations: usize,
    n: usize,
) -> (usize, usize) {
    let before = quality(groups, col, edges, n);
    let mut best = before;
    for pass in 0..iterations {
        let r = ranks(groups, n);
        let mut candidate = groups.clone();
        for g in &mut candidate {
            g.sort_by(|a, b| {
                let bary = |i: usize| {
                    let mut weight = 0.;
                    let mut value = 0.;
                    for e in edges
                        .iter()
                        .filter(|e| !e.back && if pass % 2 == 0 { e.t == i } else { e.s == i })
                    {
                        let other = if pass % 2 == 0 { e.s } else { e.t };
                        let w = if e.mass > 0. { e.mass } else { f64::EPSILON };
                        value += r[other] as f64 * w;
                        weight += w;
                    }
                    if weight > 0. {
                        value / weight
                    } else {
                        r[i] as f64
                    }
                };
                bary(*a).total_cmp(&bary(*b)).then(a.cmp(b))
            });
        }
        let score = quality(&candidate, col, edges, n);
        if score < best {
            *groups = candidate;
            best = score;
        }
    }
    (before.0, best.0)
}
fn ribbon(points: &[[f64; 2]], width: f64) -> Vec<Cmd> {
    let half = width / 2.;
    let mut normals = vec![];
    for w in points.windows(2) {
        let dx = w[1][0] - w[0][0];
        let dy = w[1][1] - w[0][1];
        let len = dx.hypot(dy);
        normals.push([-dy / len, dx / len]);
    }
    let side = |sign: f64| {
        points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                let normal = if i == 0 {
                    normals[0]
                } else if i == points.len() - 1 {
                    *normals.last().unwrap()
                } else {
                    [
                        normals[i - 1][0] + normals[i][0],
                        normals[i - 1][1] + normals[i][1],
                    ]
                };
                P::new(
                    p[0] + sign * half * normal[0],
                    p[1] + sign * half * normal[1],
                )
            })
            .collect::<Vec<_>>()
    };
    let mut upper = side(1.);
    let lower = side(-1.);
    upper.extend(lower.into_iter().rev());
    let mut cmds = vec![Cmd::Move(upper[0])];
    cmds.extend(upper.into_iter().skip(1).map(Cmd::Line));
    cmds.push(Cmd::Close);
    cmds
}
/// Compute complete graph metadata and a common SVG/Canvas/Glory Scene.
/// Inputs are never mutated; all layout/identity/budget validation is atomic.
pub fn layout_sankey_circular(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[CircularFlowLink],
    o: &CircularSankeyOptions,
) -> Result<CircularSankeyLayout> {
    let mut scene = c.scene()?;
    let a = &o.appearance;
    ensure(
        nodes.len() <= MAX_CIRCULAR_NODES && links.len() <= MAX_CIRCULAR_LINKS,
        "circular Sankey graph budget exceeded",
    )?;
    ensure(
        a.node_width.is_finite()
            && a.node_width > 0.
            && a.node_width <= c.inner_width()
            && a.node_padding.is_finite()
            && a.node_padding >= 0.
            && a.link_opacity.is_finite()
            && (0. ..=1.).contains(&a.link_opacity)
            && o.route_padding.is_finite()
            && (0.000001..=1000.).contains(&o.route_padding)
            && o.iterations <= 32,
        "invalid circular Sankey options",
    )?;
    let mut sorted = nodes.iter().collect::<Vec<_>>();
    sorted.sort_by(|a, b| a.id.cmp(&b.id));
    let n = sorted.len();
    let mut index = BTreeMap::new();
    for (i, node) in sorted.iter().enumerate() {
        valid_id(&node.id)?;
        ensure(
            node.label.len() <= 512,
            "circular Sankey node label budget exceeded",
        )?;
        ensure(
            index.insert(node.id.as_str(), i).is_none(),
            "duplicate circular Sankey node ID",
        )?;
    }
    let mut link_ids = BTreeSet::new();
    let mut edges = vec![];
    let mut inflow = vec![0.; n];
    let mut outflow = vec![0.; n];
    for link in links {
        valid_id(&link.id)?;
        ensure(
            link_ids.insert(link.id.as_str()),
            "duplicate circular Sankey link ID",
        )?;
        ensure(
            link.value.is_finite() && link.value >= 0.,
            "circular Sankey flows must be finite and nonnegative",
        )?;
        let s = *index
            .get(link.source.as_str())
            .ok_or_else(|| ChartError("unknown circular Sankey source".into()))?;
        let t = *index
            .get(link.target.as_str())
            .ok_or_else(|| ChartError("unknown circular Sankey target".into()))?;
        inflow[t] = checked_sum(inflow[t], link.value)?;
        outflow[s] = checked_sum(outflow[s], link.value)?;
        edges.push(Edge {
            id: link.id.clone(),
            s,
            t,
            value: link.value,
            mass: 0.,
            back: false,
            side: ReturnSide::Top,
            lane: 0,
        });
    }
    edges.sort_by(|a, b| a.id.cmp(&b.id));
    if n == 0 {
        return Ok(CircularSankeyLayout {
            scene,
            nodes: vec![],
            links: vec![],
            metrics: Default::default(),
        });
    }
    let values = (0..n)
        .map(|i| inflow[i].max(outflow[i]))
        .collect::<Vec<_>>();
    let maximum = values
        .iter()
        .copied()
        .fold(0., f64::max)
        .max(f64::from_bits(1));
    for e in &mut edges {
        e.mass = e.value / maximum;
        ensure(
            e.value == 0. || e.mass > 0.,
            "circular Sankey dynamic range underflows flow precision",
        )?;
    }
    let component = components(n, &edges);
    let order = feedback_order(n, &edges, &component);
    let mut rank = vec![0; n];
    for (i, node) in order.iter().enumerate() {
        rank[*node] = i;
    }
    for e in &mut edges {
        e.back = rank[e.s] >= rank[e.t];
    }
    let col = columns(&order, &edges, o.alignment);
    let count = col.iter().copied().max().unwrap_or(0) + 1;
    let mut groups = vec![vec![]; count];
    for (i, column) in col.iter().enumerate() {
        groups[*column].push(i);
    }
    let (before, after) = optimize(&mut groups, &col, &edges, o.iterations, n);
    let mut top_mass = 0.;
    let mut bottom_mass = 0.;
    let mut top_count = 0;
    let mut bottom_count = 0;
    // Longest returns first; balance mass, then lane count. Lexical link IDs
    // break ties, preserving paint/route identity under source permutations.
    let mut returns = edges
        .iter()
        .enumerate()
        .filter(|(_, e)| e.back && e.value > 0.)
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    returns.sort_by(|a, b| {
        col[edges[*b].s]
            .abs_diff(col[edges[*b].t])
            .cmp(&col[edges[*a].s].abs_diff(col[edges[*a].t]))
            .then(edges[*a].id.cmp(&edges[*b].id))
    });
    for &i in &returns {
        let e = &mut edges[i];
        let top = match o.return_side {
            CircularReturnSide::Top => true,
            CircularReturnSide::Bottom => false,
            CircularReturnSide::Balanced => {
                top_mass < bottom_mass || top_mass == bottom_mass && top_count <= bottom_count
            }
        };
        if top {
            e.side = ReturnSide::Top;
            e.lane = top_count;
            top_count += 1;
            top_mass += e.mass;
        } else {
            e.side = ReturnSide::Bottom;
            e.lane = bottom_count;
            bottom_count += 1;
            bottom_mass += e.mass;
        }
    }
    let padding = o.route_padding;
    let vertical_fixed =
        (top_count + bottom_count + usize::from(top_count > 0) + usize::from(bottom_count > 0))
            as f64
            * padding;
    let total_returns = top_mass + bottom_mass;
    let mut unit = f64::INFINITY;
    for group in &groups {
        let fixed = vertical_fixed + group.len().saturating_sub(1) as f64 * a.node_padding;
        ensure(
            fixed < c.inner_height(),
            "circular Sankey vertical lane/node spacing exceeds plot",
        )?;
        let mass = group.iter().map(|i| values[*i] / maximum).sum::<f64>() + total_returns;
        if mass > 0. {
            unit = unit.min((c.inner_height() - fixed) / mass);
        }
    }
    let mut strips_mass = vec![0.; count + 1];
    let mut strips_count = vec![0; count + 1];
    for &i in &returns {
        let e = &edges[i];
        for gap in [col[e.s] + 1, col[e.t]] {
            strips_mass[gap] += e.mass;
            strips_count[gap] += 1;
        }
    }
    let horizontal_fixed = count as f64 * a.node_width
        + strips_count
            .iter()
            .map(|n| (*n + 1) as f64 * padding)
            .sum::<f64>();
    ensure(
        horizontal_fixed < c.inner_width(),
        "circular Sankey horizontal return spacing exceeds plot",
    )?;
    let horizontal_mass = strips_mass.iter().sum::<f64>();
    if horizontal_mass > 0. {
        unit = unit.min((c.inner_width() - horizontal_fixed) / horizontal_mass);
    }
    if !unit.is_finite() {
        unit = 1.;
    }
    ensure(
        unit.is_finite() && unit > 0.,
        "circular Sankey has no representable flow scale",
    )?;
    let top_height = if top_count > 0 {
        top_mass * unit + (top_count + 1) as f64 * padding
    } else {
        0.
    };
    let bottom_height = if bottom_count > 0 {
        bottom_mass * unit + (bottom_count + 1) as f64 * padding
    } else {
        0.
    };
    let band_top = c.margin.top + top_height;
    let band_height = c.inner_height() - top_height - bottom_height;
    let mut gaps = (0..=count)
        .map(|i| strips_mass[i] * unit + (strips_count[i] + 1) as f64 * padding)
        .collect::<Vec<_>>();
    let spare = (c.inner_width() - count as f64 * a.node_width - gaps.iter().sum::<f64>()).max(0.);
    if count > 1 {
        for g in &mut gaps[1..count] {
            *g += spare / (count - 1) as f64;
        }
    } else {
        gaps[0] += spare / 2.;
        gaps[1] += spare / 2.;
    }
    let mut xs = vec![0.; count];
    let mut cursor = c.margin.left + gaps[0];
    for i in 0..count {
        xs[i] = cursor;
        cursor += a.node_width + gaps[i + 1];
    }
    let mut rects = vec![
        ClipRect {
            x: 0.,
            y: 0.,
            width: 0.,
            height: 0.
        };
        n
    ];
    for (column, group) in groups.iter().enumerate() {
        let used = group
            .iter()
            .map(|i| values[*i] / maximum * unit)
            .sum::<f64>()
            + group.len().saturating_sub(1) as f64 * a.node_padding;
        let mut y = band_top + (band_height - used).max(0.) / 2.;
        for &i in group {
            let h = values[i] / maximum * unit;
            rects[i] = ClipRect {
                x: xs[column],
                y,
                width: a.node_width,
                height: h,
            };
            y += h + a.node_padding;
        }
    }
    let node_rank = ranks(&groups, n);
    let mut incoming = vec![vec![]; n];
    let mut outgoing = vec![vec![]; n];
    for (i, e) in edges.iter().enumerate() {
        incoming[e.t].push(i);
        outgoing[e.s].push(i);
    }
    let port_order = |i: usize, source: bool| {
        let e = &edges[i];
        let other = if source { e.t } else { e.s };
        (
            if !e.back {
                1
            } else if e.side == ReturnSide::Top {
                0
            } else {
                2
            },
            e.lane,
            col[other],
            node_rank[other],
            i,
        )
    };
    let mut source_y = vec![0.; edges.len()];
    let mut target_y = source_y.clone();
    for i in 0..n {
        outgoing[i].sort_by_key(|e| port_order(*e, true));
        incoming[i].sort_by_key(|e| port_order(*e, false));
        let mut y = rects[i].y;
        for &e in &outgoing[i] {
            source_y[e] = y + edges[e].mass * unit / 2.;
            y += edges[e].mass * unit;
        }
        let mut y = rects[i].y;
        for &e in &incoming[i] {
            target_y[e] = y + edges[e].mass * unit / 2.;
            y += edges[e].mass * unit;
        }
    }
    let mut right_cursor = vec![padding; count];
    let mut left_cursor = vec![padding; count];
    let mut legs = vec![(0., 0.); edges.len()];
    for &i in &returns {
        let e = &edges[i];
        let width = e.mass * unit;
        let s = col[e.s];
        let t = col[e.t];
        let right = xs[s] + a.node_width + right_cursor[s] + width / 2.;
        let left = xs[t] - left_cursor[t] - width / 2.;
        legs[i] = (right, left);
        right_cursor[s] += width + padding;
        left_cursor[t] += width + padding;
    }
    let mut top_cursor = c.margin.top + padding;
    let mut bottom_cursor = c.height - c.margin.bottom - padding;
    let mut lanes = vec![0.; edges.len()];
    for &i in &returns {
        let e = &edges[i];
        let width = e.mass * unit;
        if e.side == ReturnSide::Top {
            lanes[i] = top_cursor + width / 2.;
            top_cursor += width + padding;
        } else {
            lanes[i] = bottom_cursor - width / 2.;
            bottom_cursor -= width + padding;
        }
    }
    let mut laid_links = vec![];
    for (i, e) in edges.iter().enumerate() {
        let width = e.mass * unit;
        ensure(
            e.value == 0. || width > 0.,
            "circular Sankey projected flow underflow",
        )?;
        let source = [rects[e.s].x + a.node_width, source_y[i]];
        let target = [rects[e.t].x, target_y[i]];
        let (route, centerline, path) = if e.back && e.value > 0. {
            let (right, left) = legs[i];
            let points = vec![
                source,
                [right, source[1]],
                [right, lanes[i]],
                [left, lanes[i]],
                [left, target[1]],
                target,
            ];
            (
                CircularLinkRoute::Return {
                    side: e.side,
                    lane: e.lane,
                    center_y: lanes[i],
                },
                points.clone(),
                ribbon(&points, width),
            )
        } else {
            let mid = (source[0] + target[0]) / 2.;
            let h = width / 2.;
            (
                if e.back {
                    CircularLinkRoute::Return {
                        side: e.side,
                        lane: e.lane,
                        center_y: source[1],
                    }
                } else {
                    CircularLinkRoute::Forward
                },
                vec![source, target],
                vec![
                    Cmd::Move(P::new(source[0], source[1] - h)),
                    Cmd::Cubic {
                        first: P::new(mid, source[1] - h),
                        second: P::new(mid, target[1] - h),
                        end: P::new(target[0], target[1] - h),
                    },
                    Cmd::Line(P::new(target[0], target[1] + h)),
                    Cmd::Cubic {
                        first: P::new(mid, target[1] + h),
                        second: P::new(mid, source[1] + h),
                        end: P::new(source[0], source[1] + h),
                    },
                    Cmd::Close,
                ],
            )
        };
        let mut mark = Mark::new(
            format!("circular-link:{}:{}", e.id.len(), e.id),
            Geometry::Path(path),
            c.theme.color(e.s),
        );
        mark.series = sorted[e.s].id.clone();
        mark.label = format!("{} → {}: {}", sorted[e.s].label, sorted[e.t].label, e.value);
        mark.value = e.value;
        mark.opacity = a.link_opacity;
        if width == 0. {
            mark.opacity = 0.;
        }
        scene.marks.push(mark);
        laid_links.push(CircularLinkLayout {
            id: e.id.clone(),
            source: sorted[e.s].id.clone(),
            target: sorted[e.t].id.clone(),
            value: e.value,
            width,
            source_port: source,
            target_port: target,
            route,
            centerline,
        });
    }
    let mut laid_nodes = vec![];
    for (i, node) in sorted.iter().enumerate() {
        let r = rects[i];
        let color = c.theme.color(i);
        let mut mark = Mark::new(
            format!("circular-node:{}:{}", node.id.len(), node.id),
            Geometry::Rect {
                x: r.x,
                y: r.y,
                width: r.width,
                height: r.height,
            },
            color,
        );
        mark.series = node.id.clone();
        mark.label = format!(
            "{}: {} (in {}, out {})",
            node.label, values[i], inflow[i], outflow[i]
        );
        mark.value = values[i];
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: node.id.clone(),
            label: node.label.clone(),
            color,
        });
        if a.labels {
            scene
                .label_series
                .insert(scene.labels.len(), node.id.clone());
            scene.labels.push(Label {
                x: r.x + r.width / 2.,
                y: r.y + r.height / 2. + c.theme.font_size / 3.,
                text: node.label.clone(),
                color: c.theme.text,
                size: c.theme.font_size,
                anchor: TextAnchor::Middle,
                rotation: 0.,
            });
        }
        laid_nodes.push(CircularNodeLayout {
            id: node.id.clone(),
            component: component[i],
            column: col[i],
            rect: r,
            inflow: inflow[i],
            outflow: outflow[i],
            value: values[i],
        });
    }
    scene.validate()?;
    let mut lane_collisions = 0;
    for (i, a) in laid_links.iter().enumerate() {
        if let CircularLinkRoute::Return { side, center_y, .. } = a.route
            && a.width > 0.
        {
            for b in &laid_links[i + 1..] {
                if let CircularLinkRoute::Return {
                    side: other,
                    center_y: y,
                    ..
                } = b.route
                    && b.width > 0.
                    && side == other
                    && (y - center_y).abs() < (a.width + b.width) / 2. - 1e-9
                {
                    lane_collisions += 1;
                }
            }
        }
    }
    ensure(
        lane_collisions == 0,
        "circular Sankey return lane collision",
    )?;
    let ratio = unit / maximum;
    let metrics = CircularSankeyMetrics {
        components: component.iter().max().unwrap() + 1,
        return_links: returns.len(),
        forward_crossings_before: before,
        forward_crossings_after: after,
        return_lane_collisions: lane_collisions,
        pixels_per_unit: ratio.is_finite().then_some(ratio),
        flow_normalization: maximum,
        pixels_per_normalized_unit: unit,
    };
    Ok(CircularSankeyLayout {
        scene,
        nodes: laid_nodes,
        links: laid_links,
        metrics,
    })
}
pub fn sankey_circular(
    c: &ChartConfig,
    nodes: &[FlowNode],
    links: &[CircularFlowLink],
    o: &CircularSankeyOptions,
) -> Result<Scene> {
    Ok(layout_sankey_circular(c, nodes, links, o)?.scene)
}
