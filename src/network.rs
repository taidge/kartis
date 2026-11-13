//! Explicit, application-owned force simulation with Barnes–Hut repulsion.
//!
//! Design references: local Nivo `packages/network/src/hooks.ts` and D3's
//! `d3-force/src/{manyBody,simulation,collide}.js`. Algorithms are implemented in
//! Rust without a timer, global scheduler, browser state, or random dependencies.
//! Positions are world-space CSS pixels around the plot center. Callers own the
//! tick loop, drag events, and repaint scheduling.
//! Exact evaluation (`theta=0`) and fully coincident leaf buckets can require
//! quadratic pair work. Ordinary distributed data uses aggregate cell visits.
use std::collections::BTreeMap;
use std::f64::consts::TAU;

use glory::render::{GraphicPoint, GraphicStroke, PathCommand, StrokeCap};

use crate::scene::{ensure, *};
use crate::{NetworkLink, NetworkNode};
#[path = "network_drag.rs"]
mod drag;
pub use drag::*;

pub const MAX_FORCE_NODES: usize = 10_000;
pub const MAX_FORCE_TICKS_PER_STEP: usize = 1000;
pub const MAX_QUADTREE_DEPTH: usize = 32;
pub const MAX_FORCE_COORDINATE: f64 = 500_000.;
const MAX_QUADTREE_CELLS: usize = MAX_FORCE_NODES * 8;
const LEAF_CAPACITY: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct ForceOptions {
    /// Repulsion magnitude is strength / distance, matching D3 many-body force.
    pub repulsion: f64,
    /// Zero disables aggregation and evaluates every eligible pair exactly.
    pub theta: f64,
    pub distance_min: f64,
    pub distance_max: Option<f64>,
    pub link_distance: f64,
    pub link_strength: f64,
    pub center: [f64; 2],
    pub center_strength: f64,
    pub collision_padding: f64,
    pub collision_strength: f64,
    pub collision_iterations: usize,
    /// Initial heat. `set_options` preserves current runtime heat; use `reheat`
    /// and `set_alpha_target` to control the active simulation.
    pub alpha: f64,
    pub alpha_min: f64,
    pub alpha_target: f64,
    pub alpha_decay: f64,
    /// Fraction of velocity lost per tick.
    pub velocity_decay: f64,
    pub max_speed: f64,
    pub labels: bool,
}
impl Default for ForceOptions {
    fn default() -> Self {
        Self {
            repulsion: 80.,
            theta: 0.8,
            distance_min: 1.,
            distance_max: None,
            link_distance: 70.,
            link_strength: 0.08,
            center: [0., 0.],
            center_strength: 0.006,
            collision_padding: 2.,
            collision_strength: 0.8,
            collision_iterations: 2,
            alpha: 1.,
            alpha_min: 0.001,
            alpha_target: 0.,
            alpha_decay: 0.0228,
            velocity_decay: 0.4,
            max_speed: 100.,
            labels: true,
        }
    }
}
fn position_valid(p: [f64; 2]) -> bool {
    p.iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_FORCE_COORDINATE)
}
fn unit_interval(v: f64) -> bool {
    v.is_finite() && (0. ..=1.).contains(&v)
}
impl ForceOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            self.repulsion.is_finite()
                && (0. ..=1_000_000.).contains(&self.repulsion)
                && self.theta.is_finite()
                && (0. ..=2.).contains(&self.theta)
                && self.distance_min.is_finite()
                && (0.000001..=1_000_000.).contains(&self.distance_min)
                && self
                    .distance_max
                    .is_none_or(|v| v.is_finite() && v >= self.distance_min && v <= 2_000_000.),
            "invalid repulsion options",
        )?;
        ensure(
            self.link_distance.is_finite()
                && (0. ..=1_000_000.).contains(&self.link_distance)
                && unit_interval(self.link_strength)
                && unit_interval(self.center_strength)
                && position_valid(self.center)
                && self.collision_padding.is_finite()
                && (0. ..=10_000.).contains(&self.collision_padding)
                && unit_interval(self.collision_strength)
                && self.collision_iterations <= 16,
            "invalid link, center, or collision options",
        )?;
        ensure(
            [
                self.alpha,
                self.alpha_min,
                self.alpha_target,
                self.alpha_decay,
                self.velocity_decay,
            ]
            .into_iter()
            .all(unit_interval)
                && self.max_speed.is_finite()
                && (0.000001..=10_000.).contains(&self.max_speed),
            "invalid integration or cooling options",
        )
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ForceStats {
    pub ticks: usize,
    /// Maximum single-tree allocation across charge/collision passes.
    pub quadtree_cells: usize,
    pub max_depth: usize,
    pub direct_interactions: u64,
    pub approximated_cells: u64,
    pub collision_pairs: u64,
    pub alpha: f64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ManyBodyForces {
    pub forces: Vec<[f64; 2]>,
    pub stats: ForceStats,
}
#[derive(Clone, Copy)]
struct Bounds {
    x: f64,
    y: f64,
    size: f64,
}
impl Bounds {
    fn contains(self, p: [f64; 2]) -> bool {
        let epsilon = self.size * 1e-12 + 1e-12;
        p[0] >= self.x - epsilon
            && p[0] <= self.x + self.size + epsilon
            && p[1] >= self.y - epsilon
            && p[1] <= self.y + self.size + epsilon
    }
    fn min_distance(self, p: [f64; 2]) -> f64 {
        let dx = (self.x - p[0]).max(0.).max(p[0] - self.x - self.size);
        let dy = (self.y - p[1]).max(0.).max(p[1] - self.y - self.size);
        dx.hypot(dy)
    }
    fn max_distance(self, p: [f64; 2]) -> f64 {
        (self.x - p[0])
            .abs()
            .max((self.x + self.size - p[0]).abs())
            .hypot((self.y - p[1]).abs().max((self.y + self.size - p[1]).abs()))
    }
}
struct Quad {
    bounds: Bounds,
    children: [Option<usize>; 4],
    members: Vec<usize>,
    count: usize,
    center: [f64; 2],
    max_radius: f64,
}
struct QuadTree {
    cells: Vec<Quad>,
    max_depth: usize,
}
impl QuadTree {
    fn build(positions: &[[f64; 2]], radii: &[f64]) -> Result<Self> {
        let mut tree = Self {
            cells: vec![],
            max_depth: 0,
        };
        if positions.is_empty() {
            return Ok(tree);
        }
        let mut min = [f64::INFINITY; 2];
        let mut max = [f64::NEG_INFINITY; 2];
        for p in positions {
            for axis in 0..2 {
                min[axis] = min[axis].min(p[axis]);
                max[axis] = max[axis].max(p[axis]);
            }
        }
        let size = (max[0] - min[0]).max(max[1] - min[1]).max(0.000001);
        tree.partition(
            positions,
            radii,
            (0..positions.len()).collect(),
            Bounds {
                x: min[0],
                y: min[1],
                size,
            },
            0,
        )?;
        Ok(tree)
    }
    fn partition(
        &mut self,
        positions: &[[f64; 2]],
        radii: &[f64],
        indices: Vec<usize>,
        bounds: Bounds,
        depth: usize,
    ) -> Result<usize> {
        ensure(
            self.cells.len() < MAX_QUADTREE_CELLS,
            "force quadtree cell budget exceeded",
        )?;
        self.max_depth = self.max_depth.max(depth);
        let count = indices.len();
        let mut center = [0., 0.];
        let mut max_radius = 0f64;
        for &i in &indices {
            center[0] += positions[i][0] / count as f64;
            center[1] += positions[i][1] / count as f64;
            max_radius = max_radius.max(radii[i]);
        }
        let coincident = indices
            .iter()
            .all(|i| positions[*i] == positions[indices[0]]);
        let leaf = count <= LEAF_CAPACITY || depth >= MAX_QUADTREE_DEPTH || coincident;
        let index = self.cells.len();
        self.cells.push(Quad {
            bounds,
            children: [None; 4],
            members: vec![],
            count,
            center,
            max_radius,
        });
        if leaf {
            self.cells[index].members = indices;
        } else {
            let half = bounds.size / 2.;
            let mut parts: [Vec<usize>; 4] = std::array::from_fn(|_| vec![]);
            for i in indices {
                let q = usize::from(positions[i][0] >= bounds.x + half)
                    + 2 * usize::from(positions[i][1] >= bounds.y + half);
                parts[q].push(i);
            }
            for (q, part) in parts.into_iter().enumerate() {
                if !part.is_empty() {
                    let child = Bounds {
                        x: bounds.x + (q % 2) as f64 * half,
                        y: bounds.y + (q / 2) as f64 * half,
                        size: half,
                    };
                    self.cells[index].children[q] =
                        Some(self.partition(positions, radii, part, child, depth + 1)?);
                }
            }
        }
        Ok(index)
    }
}
fn pair_direction(a: usize, b: usize) -> [f64; 2] {
    let mut hash = (a.min(b) as u64).wrapping_mul(0x9e3779b97f4a7c15)
        ^ (a.max(b) as u64).wrapping_add(0x517cc1b727220a95);
    hash ^= hash >> 30;
    hash = hash.wrapping_mul(0xbf58476d1ce4e5b9);
    hash ^= hash >> 27;
    let angle = (hash as u32) as f64 / u32::MAX as f64 * TAU;
    let sign = if a < b { 1. } else { -1. };
    [angle.cos() * sign, angle.sin() * sign]
}
fn charge_delta(delta: [f64; 2], fallback: [f64; 2], count: usize, o: &ForceOptions) -> [f64; 2] {
    let distance = delta[0].hypot(delta[1]);
    if o.distance_max.is_some_and(|max| distance > max) {
        return [0., 0.];
    }
    let direction = if distance > 0. {
        [delta[0] / distance, delta[1] / distance]
    } else {
        fallback
    };
    let strength = o.repulsion * count as f64 / distance.max(o.distance_min);
    [direction[0] * strength, direction[1] * strength]
}
/// Checked Barnes–Hut evaluation, without integration or alpha multiplication.
/// `theta = 0` is an exact direct-pair reference. Coincident points occupy one
/// bounded leaf and receive deterministic opposite pair directions.
pub fn many_body_forces(positions: &[[f64; 2]], o: &ForceOptions) -> Result<ManyBodyForces> {
    o.validate()?;
    ensure(
        positions.len() <= MAX_FORCE_NODES && positions.iter().copied().all(position_valid),
        "invalid force positions or node budget",
    )?;
    let tree = QuadTree::build(positions, &vec![0.; positions.len()])?;
    let mut stats = ForceStats {
        quadtree_cells: tree.cells.len(),
        max_depth: tree.max_depth,
        ..Default::default()
    };
    let mut forces = vec![[0., 0.]; positions.len()];
    if o.repulsion == 0. {
        return Ok(ManyBodyForces { forces, stats });
    }
    for (i, &p) in positions.iter().enumerate() {
        let mut stack = if tree.cells.is_empty() {
            vec![]
        } else {
            vec![0]
        };
        while let Some(index) = stack.pop() {
            let quad = &tree.cells[index];
            if o.distance_max
                .is_some_and(|max| quad.bounds.min_distance(p) > max)
            {
                continue;
            }
            let delta = [p[0] - quad.center[0], p[1] - quad.center[1]];
            let distance = delta[0].hypot(delta[1]);
            let within_cutoff = o
                .distance_max
                .is_none_or(|max| quad.bounds.max_distance(p) <= max);
            if o.theta > 0.
                && !quad.bounds.contains(p)
                && distance > 0.
                && quad.bounds.size / distance < o.theta
                && within_cutoff
            {
                let f = charge_delta(delta, [1., 0.], quad.count, o);
                forces[i][0] += f[0];
                forces[i][1] += f[1];
                stats.approximated_cells += 1;
            } else if !quad.members.is_empty() {
                for &j in &quad.members {
                    if i != j {
                        let delta = [p[0] - positions[j][0], p[1] - positions[j][1]];
                        let fallback = if delta == [0., 0.] {
                            pair_direction(i, j)
                        } else {
                            [1., 0.]
                        };
                        let f = charge_delta(delta, fallback, 1, o);
                        forces[i][0] += f[0];
                        forces[i][1] += f[1];
                        stats.direct_interactions += 1;
                    }
                }
            } else {
                stack.extend(quad.children.iter().rev().flatten().copied());
            }
        }
    }
    Ok(ManyBodyForces { forces, stats })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForceAxis {
    X,
    Y,
}
/// Additional validated forces; these compose with the built-in charge, link,
/// centering and collision forces. Strength zero disables an individual force.
#[derive(Clone, Debug, PartialEq)]
pub enum Force {
    Axis {
        axis: ForceAxis,
        target: f64,
        strength: f64,
    },
    Radial {
        center: [f64; 2],
        radius: f64,
        strength: f64,
    },
}
impl Force {
    fn validate(&self) -> Result<()> {
        match *self {
            Self::Axis {
                target, strength, ..
            } => ensure(
                position_valid([target, 0.]) && unit_interval(strength),
                "invalid axis force",
            ),
            Self::Radial {
                center,
                radius,
                strength,
            } => ensure(
                position_valid(center)
                    && radius.is_finite()
                    && (0. ..=MAX_FORCE_COORDINATE).contains(&radius)
                    && unit_interval(strength),
                "invalid radial force",
            ),
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct ForceNodeState {
    pub id: String,
    pub label: String,
    pub radius: f64,
    pub position: [f64; 2],
    pub velocity: [f64; 2],
    pub fixed: Option<[f64; 2]>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ForceEdge {
    pub source: usize,
    pub target: usize,
    pub value: f64,
}
#[derive(Clone, Copy)]
struct Kinematic {
    position: [f64; 2],
    velocity: [f64; 2],
    fixed: Option<[f64; 2]>,
}

/// Reusable deterministic simulation. A rejected tick leaves all positions,
/// velocities and cooling state unchanged. `step` admits its iteration budget
/// first; successful earlier ticks remain committed if a later tick fails.
#[derive(Clone, Debug)]
pub struct ForceSimulation {
    nodes: Vec<ForceNodeState>,
    edges: Vec<ForceEdge>,
    index: BTreeMap<String, usize>,
    degree: Vec<usize>,
    options: ForceOptions,
    additional: Vec<Force>,
    alpha: f64,
    total_ticks: u64,
}
impl ForceSimulation {
    pub fn new(
        nodes: &[NetworkNode],
        links: &[NetworkLink],
        options: ForceOptions,
    ) -> Result<Self> {
        options.validate()?;
        ensure(
            nodes.len() <= MAX_FORCE_NODES
                && nodes
                    .len()
                    .checked_add(links.len())
                    .is_some_and(|n| n <= MAX_MARKS),
            "force network input budget exceeded",
        )?;
        let mut index = BTreeMap::new();
        let mut states = Vec::with_capacity(nodes.len());
        for (i, node) in nodes.iter().enumerate() {
            ensure(
                !node.id.trim().is_empty()
                    && node.id.len() <= 512
                    && node.label.len() <= MAX_LABEL_BYTES
                    && node.radius.is_finite()
                    && (0.000001..=10_000.).contains(&node.radius)
                    && index.insert(node.id.clone(), i).is_none(),
                "invalid or duplicate force node",
            )?;
            let angle = i as f64 * 2.399963229728653;
            let radius = 10. * (i as f64 + 0.5).sqrt();
            let position = [
                options.center[0] + radius * angle.cos(),
                options.center[1] + radius * angle.sin(),
            ];
            ensure(position_valid(position), "force seed coordinate overflow")?;
            states.push(ForceNodeState {
                id: node.id.clone(),
                label: node.label.clone(),
                radius: node.radius,
                position,
                velocity: [0., 0.],
                fixed: None,
            });
        }
        let mut edges = Vec::with_capacity(links.len());
        let mut degree = vec![0; nodes.len()];
        for link in links {
            ensure(
                link.value.is_finite()
                    && link.value >= 0.
                    && link.source.len() + link.target.len() + 5 <= MAX_LABEL_BYTES,
                "invalid force link",
            )?;
            let source = *index
                .get(&link.source)
                .ok_or_else(|| ChartError("unknown force link source".into()))?;
            let target = *index
                .get(&link.target)
                .ok_or_else(|| ChartError("unknown force link target".into()))?;
            degree[source] += 1;
            degree[target] += 1;
            edges.push(ForceEdge {
                source,
                target,
                value: link.value,
            });
        }
        let alpha = options.alpha;
        Ok(Self {
            nodes: states,
            edges,
            index,
            degree,
            options,
            additional: vec![],
            alpha,
            total_ticks: 0,
        })
    }
    pub fn nodes(&self) -> &[ForceNodeState] {
        &self.nodes
    }
    pub fn edges(&self) -> &[ForceEdge] {
        &self.edges
    }
    pub fn node(&self, id: &str) -> Option<&ForceNodeState> {
        self.index.get(id).map(|i| &self.nodes[*i])
    }
    pub fn options(&self) -> &ForceOptions {
        &self.options
    }
    pub fn alpha(&self) -> f64 {
        self.alpha
    }
    pub fn total_ticks(&self) -> u64 {
        self.total_ticks
    }
    pub fn is_active(&self) -> bool {
        self.alpha > self.options.alpha_min || self.options.alpha_target > self.options.alpha_min
    }
    fn node_index(&self, id: &str) -> Result<usize> {
        self.index
            .get(id)
            .copied()
            .ok_or_else(|| ChartError("unknown force node".into()))
    }
    pub fn set_options(&mut self, options: ForceOptions) -> Result<()> {
        options.validate()?;
        self.options = options;
        Ok(())
    }
    pub fn set_position(&mut self, id: &str, position: [f64; 2]) -> Result<()> {
        ensure(position_valid(position), "invalid force position")?;
        let i = self.node_index(id)?;
        self.nodes[i].position = position;
        if self.nodes[i].fixed.is_some() {
            self.nodes[i].fixed = Some(position);
        }
        Ok(())
    }
    pub fn set_velocity(&mut self, id: &str, velocity: [f64; 2]) -> Result<()> {
        ensure(
            velocity.iter().all(|v| v.is_finite() && v.abs() <= 10_000.),
            "invalid force velocity",
        )?;
        let i = self.node_index(id)?;
        self.nodes[i].velocity = if self.nodes[i].fixed.is_some() {
            [0., 0.]
        } else {
            velocity
        };
        Ok(())
    }
    /// Fix/release a node during drag. Fixing updates position immediately and
    /// clears velocity; subsequent ticks hold the exact supplied coordinates.
    pub fn set_fixed(&mut self, id: &str, position: Option<[f64; 2]>) -> Result<()> {
        ensure(
            position.is_none_or(position_valid),
            "invalid fixed force position",
        )?;
        let i = self.node_index(id)?;
        self.nodes[i].fixed = position;
        if let Some(position) = position {
            self.nodes[i].position = position;
            self.nodes[i].velocity = [0., 0.];
        }
        Ok(())
    }
    pub fn reheat(&mut self, alpha: f64) -> Result<()> {
        ensure(unit_interval(alpha), "invalid reheat alpha")?;
        self.alpha = self.alpha.max(alpha);
        Ok(())
    }
    pub fn set_alpha_target(&mut self, alpha: f64) -> Result<()> {
        ensure(unit_interval(alpha), "invalid target alpha")?;
        self.options.alpha_target = alpha;
        Ok(())
    }
    pub fn add_force(&mut self, force: Force) -> Result<usize> {
        force.validate()?;
        ensure(
            self.additional.len() < 32,
            "additional force budget exceeded",
        )?;
        let index = self.additional.len();
        self.additional.push(force);
        Ok(index)
    }
    pub fn additional_forces(&self) -> &[Force] {
        &self.additional
    }
    pub fn clear_additional_forces(&mut self) {
        self.additional.clear();
    }
    /// One explicitly requested tick. It remains valid to tick a cooled simulation;
    /// callers can use `is_active()` to stop their own animation loop.
    pub fn tick(&mut self) -> Result<ForceStats> {
        let alpha =
            self.alpha + (self.options.alpha_target - self.alpha) * self.options.alpha_decay;
        let next_total = self
            .total_ticks
            .checked_add(1)
            .ok_or_else(|| ChartError("force tick counter overflow".into()))?;
        let positions: Vec<_> = self.nodes.iter().map(|node| node.position).collect();
        let ManyBodyForces {
            mut forces,
            mut stats,
        } = many_body_forces(&positions, &self.options)?;
        for edge in &self.edges {
            let (a, b) = (edge.source, edge.target);
            if a == b {
                continue;
            }
            let delta = [
                positions[b][0] - positions[a][0],
                positions[b][1] - positions[a][1],
            ];
            let distance = delta[0].hypot(delta[1]);
            let direction = if distance > 0. {
                [delta[0] / distance, delta[1] / distance]
            } else {
                pair_direction(a, b)
            };
            let strength = (distance - self.options.link_distance) * self.options.link_strength;
            let bias = self.degree[a] as f64 / (self.degree[a] + self.degree[b]) as f64;
            for axis in 0..2 {
                forces[a][axis] += direction[axis] * strength * (1. - bias);
                forces[b][axis] -= direction[axis] * strength * bias;
            }
        }
        for (i, node) in self.nodes.iter().enumerate() {
            for (axis, component) in forces[i].iter_mut().enumerate() {
                *component += (self.options.center[axis] - node.position[axis])
                    * self.options.center_strength;
            }
            for force in &self.additional {
                match *force {
                    Force::Axis {
                        axis,
                        target,
                        strength,
                    } => {
                        let axis = if axis == ForceAxis::X { 0 } else { 1 };
                        forces[i][axis] += (target - node.position[axis]) * strength;
                    }
                    Force::Radial {
                        center,
                        radius,
                        strength,
                    } => {
                        let delta = [node.position[0] - center[0], node.position[1] - center[1]];
                        let distance = delta[0].hypot(delta[1]);
                        let direction = if distance > 0. {
                            [delta[0] / distance, delta[1] / distance]
                        } else {
                            pair_direction(i, i.wrapping_add(1))
                        };
                        for axis in 0..2 {
                            forces[i][axis] += (radius - distance) * strength * direction[axis];
                        }
                    }
                }
            }
        }
        let mut next = Vec::with_capacity(self.nodes.len());
        for (node, force) in self.nodes.iter().zip(forces) {
            ensure(
                force.iter().all(|v| v.is_finite()),
                "nonfinite force proposal",
            )?;
            let mut velocity = [
                (node.velocity[0] + force[0] * alpha) * (1. - self.options.velocity_decay),
                (node.velocity[1] + force[1] * alpha) * (1. - self.options.velocity_decay),
            ];
            let speed = velocity[0].hypot(velocity[1]);
            if speed > self.options.max_speed {
                for v in &mut velocity {
                    *v = *v / speed * self.options.max_speed;
                }
            }
            let (position, velocity) = if let Some(fixed) = node.fixed {
                (fixed, [0., 0.])
            } else {
                (
                    [
                        node.position[0] + velocity[0],
                        node.position[1] + velocity[1],
                    ],
                    velocity,
                )
            };
            ensure(
                position_valid(position),
                "force integration coordinate budget exceeded",
            )?;
            next.push(Kinematic {
                position,
                velocity,
                fixed: node.fixed,
            });
        }
        if self.options.collision_strength > 0. {
            for _ in 0..self.options.collision_iterations {
                self.collide(&mut next, &mut stats)?;
            }
        }
        ensure(
            next.iter()
                .all(|n| position_valid(n.position) && n.velocity.iter().all(|v| v.is_finite())),
            "invalid force collision proposal",
        )?;
        for (node, next) in self.nodes.iter_mut().zip(next) {
            node.position = next.position;
            node.velocity = next.velocity;
        }
        self.alpha = alpha;
        self.total_ticks = next_total;
        stats.ticks = 1;
        stats.alpha = alpha;
        Ok(stats)
    }
    pub fn step(&mut self, ticks: usize) -> Result<ForceStats> {
        ensure(
            ticks <= MAX_FORCE_TICKS_PER_STEP,
            "force step iteration budget exceeded",
        )?;
        let mut total = ForceStats {
            alpha: self.alpha,
            ..Default::default()
        };
        for _ in 0..ticks {
            let stats = self.tick()?;
            total.ticks += 1;
            total.quadtree_cells = total.quadtree_cells.max(stats.quadtree_cells);
            total.max_depth = total.max_depth.max(stats.max_depth);
            total.direct_interactions += stats.direct_interactions;
            total.approximated_cells += stats.approximated_cells;
            total.collision_pairs += stats.collision_pairs;
            total.alpha = stats.alpha;
        }
        Ok(total)
    }
    fn collide(&self, next: &mut [Kinematic], stats: &mut ForceStats) -> Result<()> {
        let positions: Vec<_> = next.iter().map(|n| n.position).collect();
        let radii: Vec<_> = self.nodes.iter().map(|n| n.radius).collect();
        let tree = QuadTree::build(&positions, &radii)?;
        stats.quadtree_cells = stats.quadtree_cells.max(tree.cells.len());
        stats.max_depth = stats.max_depth.max(tree.max_depth);
        for i in 0..next.len() {
            let mut stack = if tree.cells.is_empty() {
                vec![]
            } else {
                vec![0]
            };
            while let Some(index) = stack.pop() {
                let quad = &tree.cells[index];
                if quad.bounds.min_distance(positions[i])
                    > radii[i] + quad.max_radius + self.options.collision_padding
                {
                    continue;
                }
                if quad.members.is_empty() {
                    stack.extend(quad.children.iter().rev().flatten().copied());
                    continue;
                }
                for &j in &quad.members {
                    if j <= i {
                        continue;
                    }
                    let delta = [
                        next[j].position[0] - next[i].position[0],
                        next[j].position[1] - next[i].position[1],
                    ];
                    let distance = delta[0].hypot(delta[1]);
                    let required = radii[i] + radii[j] + self.options.collision_padding;
                    let movable =
                        usize::from(next[i].fixed.is_none()) + usize::from(next[j].fixed.is_none());
                    if distance >= required || movable == 0 {
                        continue;
                    }
                    let direction = if distance > 0. {
                        [delta[0] / distance, delta[1] / distance]
                    } else {
                        pair_direction(i, j)
                    };
                    let correction =
                        (required - distance) * self.options.collision_strength / movable as f64;
                    for (axis, component) in direction.into_iter().enumerate() {
                        if next[i].fixed.is_none() {
                            next[i].position[axis] -= component * correction;
                        }
                        if next[j].fixed.is_none() {
                            next[j].position[axis] += component * correction;
                        }
                    }
                    stats.collision_pairs += 1;
                }
            }
        }
        Ok(())
    }
    /// Project world-space positions into an existing chart plot. No fitting or
    /// rescaling changes physics coordinates; the renderer clips to its frame.
    pub fn scene(&self, c: &ChartConfig) -> Result<Scene> {
        let mut scene = c.scene()?;
        let offset = [
            c.margin.left + c.inner_width() / 2.,
            c.margin.top + c.inner_height() / 2.,
        ];
        for (i, edge) in self.edges.iter().enumerate() {
            let a = &self.nodes[edge.source];
            let b = &self.nodes[edge.target];
            let mut mark = Mark::new(
                format!("force-link:{i}"),
                Geometry::Path(vec![
                    PathCommand::Move(GraphicPoint::new(
                        a.position[0] + offset[0],
                        a.position[1] + offset[1],
                    )),
                    PathCommand::Line(GraphicPoint::new(
                        b.position[0] + offset[0],
                        b.position[1] + offset[1],
                    )),
                ]),
                c.theme.grid,
            );
            mark.fill = None;
            mark.stroke = Some(GraphicStroke {
                color: c.theme.grid,
                width: edge.value.sqrt().clamp(1., 12.),
                cap: StrokeCap::Round,
            });
            mark.label = format!("{} → {}", a.id, b.id);
            mark.value = edge.value;
            scene.marks.push(mark);
        }
        for (i, node) in self.nodes.iter().enumerate() {
            let (x, y) = (node.position[0] + offset[0], node.position[1] + offset[1]);
            let mut mark = Mark::new(
                format!("force-node:{}", node.id),
                Geometry::Circle {
                    x,
                    y,
                    radius: node.radius,
                },
                c.theme.color(i),
            );
            mark.series = node.id.clone();
            mark.label = node.label.clone();
            scene.marks.push(mark);
            if self.options.labels {
                scene
                    .label_series
                    .insert(scene.labels.len(), node.id.clone());
                scene.labels.push(Label {
                    rotation: 0.,
                    x,
                    y: y - node.radius - 5.,
                    text: node.label.clone(),
                    color: c.theme.text,
                    size: c.theme.font_size,
                    anchor: TextAnchor::Middle,
                });
            }
        }
        scene.validate()?;
        Ok(scene)
    }
}
