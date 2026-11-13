//! Portable single-pointer node dragging. Positions remain in the simulation's
//! world coordinates; camera/host resize changes are explicitly rebased.
use super::{ForceSimulation, ForceStats, MAX_FORCE_COORDINATE};
use crate::{ChartConfig, ClipRect, Result, ZoomTransform, scene::ensure};
use std::collections::BTreeSet;
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkDragMapping {
    pub scene_size: [f64; 2],
    /// Browser client-coordinate bounding box (CSS pixels), including origin.
    pub client_bounds: ClipRect,
    pub view: ZoomTransform,
    /// The exact world-origin translation used by ForceSimulation::scene.
    pub world_offset: [f64; 2],
}
impl NetworkDragMapping {
    pub fn from_config(
        config: &ChartConfig,
        client_bounds: ClipRect,
        view: ZoomTransform,
    ) -> Result<Self> {
        config.scene()?;
        let value = Self {
            scene_size: [config.width, config.height],
            client_bounds,
            view,
            world_offset: [
                config.margin.left + config.inner_width() / 2.,
                config.margin.top + config.inner_height() / 2.,
            ],
        };
        value.validate()?;
        Ok(value)
    }
    pub fn validate(&self) -> Result<()> {
        self.client_bounds.validate()?;
        self.view.validate()?;
        ensure(
            self.client_bounds.width > 0.
                && self.client_bounds.height > 0.
                && self
                    .scene_size
                    .iter()
                    .all(|v| v.is_finite() && *v > 0. && *v <= 1_000_000.)
                && self
                    .world_offset
                    .iter()
                    .all(|v| v.is_finite() && v.abs() <= 1_000_000.),
            "invalid network drag coordinate mapping",
        )
    }
    pub fn client_to_scene(&self, client: [f64; 2]) -> Result<[f64; 2]> {
        self.validate()?;
        ensure(
            client.iter().all(|v| v.is_finite()),
            "invalid network pointer coordinates",
        )?;
        let p = [
            (client[0] - self.client_bounds.x) * self.scene_size[0] / self.client_bounds.width,
            (client[1] - self.client_bounds.y) * self.scene_size[1] / self.client_bounds.height,
        ];
        ensure(
            p.iter().all(|v| v.is_finite()),
            "network pointer coordinate overflow",
        )?;
        Ok(p)
    }
    pub fn client_to_world(&self, client: [f64; 2]) -> Result<[f64; 2]> {
        let p = self.client_to_scene(client)?;
        let p = self.view.unproject((p[0], p[1]))?;
        let p = [p.0 - self.world_offset[0], p.1 - self.world_offset[1]];
        ensure(
            p.iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_FORCE_COORDINATE),
            "network pointer exceeds world coordinate bounds",
        )?;
        Ok(p)
    }
    pub fn world_to_client(&self, world: [f64; 2]) -> Result<[f64; 2]> {
        self.validate()?;
        ensure(
            world
                .iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_FORCE_COORDINATE),
            "invalid network world position",
        )?;
        let p = self.view.project((
            world[0] + self.world_offset[0],
            world[1] + self.world_offset[1],
        ))?;
        let p = [
            self.client_bounds.x + p.0 * self.client_bounds.width / self.scene_size[0],
            self.client_bounds.y + p.1 * self.client_bounds.height / self.scene_size[1],
        ];
        ensure(
            p.iter().all(|v| v.is_finite()),
            "network client coordinate overflow",
        )?;
        Ok(p)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum NetworkDragRelease {
    /// Preserve pin status. A previously pinned node stays at its dragged point.
    #[default]
    RestoreFixedness,
    Free,
    Pin,
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkDragOptions {
    pub release: NetworkDragRelease,
    pub reheat: f64,
    pub alpha_target: f64,
    /// Additional picking tolerance in world units; no effect on drag offset.
    pub pick_padding: f64,
}
impl Default for NetworkDragOptions {
    fn default() -> Self {
        Self {
            release: NetworkDragRelease::RestoreFixedness,
            reheat: 0.3,
            alpha_target: 0.3,
            pick_padding: 0.,
        }
    }
}
impl NetworkDragOptions {
    pub fn validate(&self) -> Result<()> {
        ensure(
            [self.reheat, self.alpha_target]
                .iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(v))
                && self.pick_padding.is_finite()
                && (0. ..=64.).contains(&self.pick_padding),
            "invalid network drag options",
        )
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct NetworkDragSession {
    pub pointer_id: i32,
    pub node_id: String,
    pub grab_offset: [f64; 2],
    pub last_client: [f64; 2],
    original_position: [f64; 2],
    original_velocity: [f64; 2],
    original_fixed: Option<[f64; 2]>,
    original_alpha_target: f64,
}
#[derive(Clone, Debug)]
pub struct NetworkDragController {
    simulation: ForceSimulation,
    options: NetworkDragOptions,
    active: Option<NetworkDragSession>,
}
impl NetworkDragController {
    pub fn new(simulation: ForceSimulation, options: NetworkDragOptions) -> Result<Self> {
        options.validate()?;
        Ok(Self {
            simulation,
            options,
            active: None,
        })
    }
    pub fn simulation(&self) -> &ForceSimulation {
        &self.simulation
    }
    pub fn options(&self) -> &NetworkDragOptions {
        &self.options
    }
    pub fn active(&self) -> Option<&NetworkDragSession> {
        self.active.as_ref()
    }
    pub fn reheat(&mut self, alpha: f64) -> Result<()> {
        self.simulation.reheat(alpha)
    }
    /// Replace a complete validated graph. The retired simulation is returned
    /// after restoring any dragged node and its previous heat target.
    pub fn replace_simulation(&mut self, simulation: ForceSimulation) -> Result<ForceSimulation> {
        simulation.options().validate()?;
        let mut next = self.clone();
        next.cancel()?;
        let previous = std::mem::replace(&mut next.simulation, simulation);
        *self = next;
        Ok(previous)
    }
    pub fn into_simulation(mut self) -> Result<ForceSimulation> {
        self.cancel()?;
        Ok(self.simulation)
    }
    pub fn pick<'a>(
        &'a self,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
        hidden: &BTreeSet<String>,
    ) -> Result<Option<&'a str>> {
        let scene = mapping.client_to_scene(client)?;
        if scene[0] < 0.
            || scene[1] < 0.
            || scene[0] > mapping.scene_size[0]
            || scene[1] > mapping.scene_size[1]
        {
            return Ok(None);
        }
        let p = mapping.client_to_world(client)?;
        Ok(self
            .simulation
            .nodes()
            .iter()
            .rev()
            .find(|n| {
                !hidden.contains(&n.id)
                    && (p[0] - n.position[0]).hypot(p[1] - n.position[1])
                        <= n.radius + self.options.pick_padding
            })
            .map(|n| n.id.as_str()))
    }
    pub fn begin(
        &mut self,
        pointer_id: i32,
        node_id: &str,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
    ) -> Result<()> {
        ensure(
            self.active.is_none(),
            "a network pointer already owns a node",
        )?;
        let world = mapping.client_to_world(client)?;
        let node = self
            .simulation
            .node(node_id)
            .ok_or_else(|| crate::ChartError("unknown drag node".into()))?;
        let session = NetworkDragSession {
            pointer_id,
            node_id: node_id.into(),
            grab_offset: [node.position[0] - world[0], node.position[1] - world[1]],
            last_client: client,
            original_position: node.position,
            original_velocity: node.velocity,
            original_fixed: node.fixed,
            original_alpha_target: self.simulation.options().alpha_target,
        };
        let mut simulation = self.simulation.clone();
        simulation.set_fixed(node_id, Some(node.position))?;
        simulation.reheat(self.options.reheat)?;
        simulation
            .set_alpha_target(self.options.alpha_target.max(session.original_alpha_target))?;
        self.simulation = simulation;
        self.active = Some(session);
        Ok(())
    }
    pub fn move_pointer(
        &mut self,
        pointer_id: i32,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
    ) -> Result<bool> {
        let Some(active) = &self.active else {
            return Ok(false);
        };
        if active.pointer_id != pointer_id {
            return Ok(false);
        };
        let p = mapping.client_to_world(client)?;
        let p = [p[0] + active.grab_offset[0], p[1] + active.grab_offset[1]];
        let mut simulation = self.simulation.clone();
        simulation.set_fixed(&active.node_id, Some(p))?;
        self.simulation = simulation;
        self.active.as_mut().unwrap().last_client = client;
        Ok(true)
    }
    /// A new camera/host size changes pointer interpretation, not physics. Keep
    /// the node still and re-anchor the grab offset at its last client position.
    pub fn rebase(&mut self, mapping: &NetworkDragMapping) -> Result<()> {
        mapping.validate()?;
        if let Some(active) = &self.active {
            let p = mapping.client_to_world(active.last_client)?;
            let position = self.simulation.node(&active.node_id).unwrap().position;
            self.active.as_mut().unwrap().grab_offset = [position[0] - p[0], position[1] - p[1]];
        }
        Ok(())
    }
    pub fn end(
        &mut self,
        pointer_id: i32,
        client: [f64; 2],
        mapping: &NetworkDragMapping,
    ) -> Result<bool> {
        if self
            .active
            .as_ref()
            .is_none_or(|s| s.pointer_id != pointer_id)
        {
            return Ok(false);
        };
        let mut next = self.clone();
        next.move_pointer(pointer_id, client, mapping)?;
        let active = next.active.take().unwrap();
        let position = next.simulation.node(&active.node_id).unwrap().position;
        let fixed = match next.options.release {
            NetworkDragRelease::Pin => Some(position),
            NetworkDragRelease::Free => None,
            NetworkDragRelease::RestoreFixedness => active.original_fixed.map(|_| position),
        };
        next.simulation.set_fixed(&active.node_id, fixed)?;
        next.simulation
            .set_alpha_target(active.original_alpha_target)?;
        *self = next;
        Ok(true)
    }
    /// Cancel only this node's gesture, preserving unrelated simulated nodes.
    /// Runtime heat keeps cooling; the previous alpha target is restored.
    pub fn cancel(&mut self) -> Result<bool> {
        let Some(active) = self.active.clone() else {
            return Ok(false);
        };
        let mut simulation = self.simulation.clone();
        simulation.set_position(&active.node_id, active.original_position)?;
        simulation.set_fixed(&active.node_id, active.original_fixed)?;
        simulation.set_velocity(&active.node_id, active.original_velocity)?;
        simulation.set_alpha_target(active.original_alpha_target)?;
        self.simulation = simulation;
        self.active = None;
        Ok(true)
    }
    /// Unlike ForceSimulation::step, this controller commits all requested
    /// ticks together. A failed batch leaves physics and dragging unchanged.
    pub fn tick(&mut self, ticks: usize) -> Result<ForceStats> {
        let mut simulation = self.simulation.clone();
        let stats = simulation.step(ticks)?;
        self.simulation = simulation;
        Ok(stats)
    }
}
