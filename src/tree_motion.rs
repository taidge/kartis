//! Explicit-clock Tree property springs, independent of painting and input.
//!
//! This foundation does not schedule browser frames, paint a Scene, grant
//! interaction eligibility, or mount retained components. Business IDs are the
//! Rust transition keys; unlike Nivo path UIDs, reparenting alone does not change
//! a node or label key. Each outgoing row retains its original immutable data.
//! `frame().target().scene()` remains the original target Scene, including
//! arbitrary user-authored geometry. It is not an animated paint Scene, and
//! these numeric properties never replace that geometry or authorize picking.
use std::sync::Arc;

use crate::motion::{PropertyState, SpringConfig};
use crate::scene::ensure;
use crate::*;

mod tracks;
use tracks::{Family, Input};

/// The complete retained union is bounded separately for each role family.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeMotionOptions {
    pub spring: SpringConfig,
    pub max_nodes: usize,
    pub max_links: usize,
    pub max_labels: usize,
}
impl Default for TreeMotionOptions {
    fn default() -> Self {
        Self {
            spring: SpringConfig {
                mass: 1.,
                stiffness: 120.,
                damping: 14.,
                ..SpringConfig::default()
            },
            max_nodes: MAX_MARKS,
            max_links: MAX_MARKS,
            max_labels: MAX_MARKS,
        }
    }
}
impl TreeMotionOptions {
    pub fn validate(&self) -> Result<()> {
        self.spring.validate()?;
        ensure(
            [self.max_nodes, self.max_links, self.max_labels]
                .into_iter()
                .all(|limit| limit <= MAX_MARKS),
            "Tree motion budget exceeds Scene limits",
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeMotionPhase {
    Live,
    Leaving,
}

/// A structural pair; delimiter-containing source IDs cannot alias an edge.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TreeMotionLinkKey {
    pub source_id: String,
    pub target_id: String,
}

/// Independently bounded numeric channels only; these values do not create
/// Mark ink. A renderer must validate their composed geometry separately.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeMotionNodeProps {
    pub x: f64,
    pub y: f64,
    /// Diameter, not the computed node's radius.
    pub diameter: f64,
    /// A computed `None` color has transparent numeric channels here. The
    /// original `data().node().color` remains `None`, with no fill permission.
    pub color: Color,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeMotionLinkProps {
    pub source: [f64; 2],
    pub target: [f64; 2],
    pub thickness: f64,
    pub color: Color,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeMotionLabelProps {
    pub x: f64,
    pub y: f64,
    /// Direct numeric rotation, bounded for Scene paint; no shortest-angle
    /// reinterpretation is applied to the physical spring.
    pub rotation: f64,
}

/// One actual owned CurrentScene label and its immutable original source.
/// Unowned annotations never become Tree motion label roles.
#[derive(Clone, Debug)]
pub struct TreeMotionLabelData {
    frame: TreeEventFrame,
    source_index: usize,
    label_index: usize,
}
impl TreeMotionLabelData {
    pub fn frame(&self) -> &TreeEventFrame {
        &self.frame
    }
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn label_index(&self) -> usize {
        self.label_index
    }
    pub fn datum(&self) -> &HierarchyDatum {
        &self.frame.source()[self.source_index]
    }
    pub fn node(&self) -> &TreeNode {
        &self.frame.nodes()[self.source_index]
    }
    pub fn label(&self) -> &Label {
        &self.frame.scene().labels[self.label_index]
    }
    pub fn font(&self) -> ResolvedFont {
        self.frame.scene().resolved_label_font(self.label_index)
    }
    pub fn whitespace(&self) -> LabelWhitespace {
        self.frame
            .scene()
            .typography
            .label_whitespace(self.label_index)
    }
    pub fn clip(&self) -> Option<&ClipPath> {
        self.frame.scene().label_clips.get(&self.label_index)
    }
    pub fn is_active(&self) -> Option<bool> {
        self.frame
            .activity()
            .and_then(|activity| activity.nodes[self.source_index])
    }
}

macro_rules! row {
    ($name:ident, $key:ty, $borrow:ty, $data:ty, $props:ty, $count:literal, $order:literal) => {
        #[derive(Clone, Debug)]
        pub struct $name {
            key: $key,
            data: $data,
            phase: TreeMotionPhase,
            instance: u64,
            props: $props,
            states: [PropertyState; $count],
        }
        impl $name {
            pub fn key(&self) -> &$borrow {
                &self.key
            }
            pub fn data(&self) -> &$data {
                &self.data
            }
            pub fn phase(&self) -> TreeMotionPhase {
                self.phase
            }
            /// Stable while retained or revived, new after actual retirement.
            pub fn instance(&self) -> u64 {
                self.instance
            }
            pub fn props(&self) -> &$props {
                &self.props
            }
            #[doc = $order]
            pub fn states(&self) -> &[PropertyState; $count] {
                &self.states
            }
        }
    };
}
row!(
    TreeMotionNode,
    String,
    str,
    TreeNodeEventData,
    TreeMotionNodeProps,
    7,
    "Raw physical [x,y,diameter,r,g,b,a], before presentation clamping."
);
row!(
    TreeMotionLink,
    TreeMotionLinkKey,
    TreeMotionLinkKey,
    TreeLinkEventData,
    TreeMotionLinkProps,
    9,
    "Raw physical [source_x,source_y,target_x,target_y,thickness,r,g,b,a]."
);
row!(
    TreeMotionLabel,
    String,
    str,
    TreeMotionLabelData,
    TreeMotionLabelProps,
    3,
    "Raw physical [x,y,rotation], with direct numeric rotation."
);

#[derive(Clone, Debug)]
struct FrameData {
    target: TreeEventFrame,
    target_serial: u64,
    sample_serial: u64,
    nodes: Vec<TreeMotionNode>,
    links: Vec<TreeMotionLink>,
    labels: Vec<TreeMotionLabel>,
    unowned_labels: Vec<usize>,
}

/// A cheap shared numeric snapshot, not a sampled Scene. Complete live node
/// and link roles include disabled/zero/no-Mark data without paint permission.
/// Live rows follow the new source/computed/label
/// pool order. Exits stay after their nearest retained old left neighbor; an
/// old leading prefix stays before its first retained neighbor. Without any
/// neighbor, exits append in their previous order. Live order never changes.
/// Equality compares shared snapshot identity: clones are equal, separately
/// captured frames are not, even when their geometry happens to match.
#[derive(Clone, Debug)]
pub struct TreeMotionFrame(Arc<FrameData>);
impl PartialEq for TreeMotionFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for TreeMotionFrame {}
impl TreeMotionFrame {
    pub fn target(&self) -> &TreeEventFrame {
        &self.0.target
    }
    /// Successful retarget attempts, including equal geometry or raw-only edits.
    pub fn target_serial(&self) -> u64 {
        self.0.target_serial
    }
    /// Successful nonzero unsettled ticks or explicit convergence operations.
    pub fn sample_serial(&self) -> u64 {
        self.0.sample_serial
    }
    pub fn nodes(&self) -> &[TreeMotionNode] {
        &self.0.nodes
    }
    pub fn links(&self) -> &[TreeMotionLink] {
        &self.0.links
    }
    pub fn labels(&self) -> &[TreeMotionLabel] {
        &self.0.labels
    }
    /// Indices into the current target Scene; no synthetic owner is assigned.
    pub fn unowned_label_indices(&self) -> &[usize] {
        &self.0.unowned_labels
    }
}

/// Pure independent Tree springs. All fallible operations clone, validate and
/// replace atomically. Browser users must additionally adopt the resulting
/// controller together with their actual paint/Widget transaction.
#[derive(Clone, Debug)]
pub struct TreeMotion {
    options: TreeMotionOptions,
    target: TreeEventFrame,
    nodes: Family<String, TreeNodeEventData, 7>,
    links: Family<TreeMotionLinkKey, TreeLinkEventData, 9>,
    labels: Family<String, TreeMotionLabelData, 3>,
    unowned_labels: Vec<usize>,
    next_instance: u64,
    target_serial: u64,
    sample_serial: u64,
    reduced: bool,
    frame: TreeMotionFrame,
}
impl TreeMotion {
    pub fn new(target: TreeEventFrame, options: TreeMotionOptions) -> Result<Self> {
        options.validate()?;
        let frame = TreeMotionFrame(Arc::new(FrameData {
            target: target.clone(),
            target_serial: 0,
            sample_serial: 0,
            nodes: vec![],
            links: vec![],
            labels: vec![],
            unowned_labels: vec![],
        }));
        let mut motion = Self {
            options,
            target: target.clone(),
            nodes: Family::default(),
            links: Family::default(),
            labels: Family::default(),
            unowned_labels: vec![],
            next_instance: 0,
            target_serial: 0,
            sample_serial: 0,
            reduced: false,
            frame,
        };
        motion.install(target)?;
        Ok(motion)
    }
    pub fn options(&self) -> &TreeMotionOptions {
        &self.options
    }
    /// Atomically update future spring behavior and retained-union limits.
    /// Existing samples, immutable data, instances and serials stay unchanged.
    /// New rest thresholds take effect only on subsequent nonzero ticks.
    pub fn set_options(&mut self, options: TreeMotionOptions) -> Result<()> {
        options.validate()?;
        let mut next = self.clone();
        next.nodes.validate(options.max_nodes)?;
        next.links.validate(options.max_links)?;
        next.labels.validate(options.max_labels)?;
        next.nodes.set_config(options.spring)?;
        next.links.set_config(options.spring)?;
        next.labels.set_config(options.spring)?;
        next.options = options;
        *self = next;
        Ok(())
    }
    pub fn target(&self) -> &TreeEventFrame {
        &self.target
    }
    pub fn frame(&self) -> &TreeMotionFrame {
        &self.frame
    }
    pub fn reduced_motion(&self) -> bool {
        self.reduced
    }
    pub fn is_settled(&self) -> bool {
        self.nodes.is_settled() && self.links.is_settled() && self.labels.is_settled()
    }
    /// Even identical numeric targets refresh live immutable source data.
    pub fn retarget(&mut self, target: TreeEventFrame) -> Result<()> {
        let mut next = self.clone();
        next.target_serial = advance(next.target_serial, "target")?;
        next.install(target)?;
        *self = next;
        Ok(())
    }
    pub fn tick(&mut self, dt: f64) -> Result<&TreeMotionFrame> {
        ensure(
            dt.is_finite() && (0. ..=60.).contains(&dt),
            "Tree motion dt must be finite and in 0..=60 seconds",
        )?;
        if dt == 0. || self.is_settled() {
            return Ok(&self.frame);
        }
        let mut next = self.clone();
        next.sample_serial = advance(next.sample_serial, "sample")?;
        next.nodes.tick(dt)?;
        next.links.tick(dt)?;
        next.labels.tick(dt)?;
        next.capture()?;
        *self = next;
        Ok(&self.frame)
    }
    pub fn finish(&mut self) -> Result<()> {
        if self.is_settled() {
            return Ok(());
        }
        let mut next = self.clone();
        next.sample_serial = advance(next.sample_serial, "sample")?;
        next.nodes.finish();
        next.links.finish();
        next.labels.finish();
        next.capture()?;
        *self = next;
        Ok(())
    }
    pub fn set_reduced_motion(&mut self, reduced: bool) -> Result<()> {
        let mut next = self.clone();
        if reduced && !next.is_settled() {
            next.sample_serial = advance(next.sample_serial, "sample")?;
        }
        next.reduced = reduced;
        next.nodes.set_reduced(reduced);
        next.links.set_reduced(reduced);
        next.labels.set_reduced(reduced);
        next.capture()?;
        *self = next;
        Ok(())
    }
    fn install(&mut self, target: TreeEventFrame) -> Result<()> {
        let input = Inputs::new(&target)?;
        self.nodes.retarget(
            input.nodes,
            Some(2),
            &self.options,
            self.options.max_nodes,
            self.reduced,
            &mut self.next_instance,
        )?;
        self.links.retarget(
            input.links,
            None,
            &self.options,
            self.options.max_links,
            self.reduced,
            &mut self.next_instance,
        )?;
        self.labels.retarget(
            input.labels,
            None,
            &self.options,
            self.options.max_labels,
            self.reduced,
            &mut self.next_instance,
        )?;
        self.unowned_labels = input.unowned_labels;
        self.target = target;
        self.capture()
    }
    fn capture(&mut self) -> Result<()> {
        self.nodes.validate(self.options.max_nodes)?;
        self.links.validate(self.options.max_links)?;
        self.labels.validate(self.options.max_labels)?;
        let nodes = self
            .nodes
            .ordered()
            .map(|(key, track)| {
                let states = track.states();
                let x = coordinate(states[0].value);
                let y = coordinate(states[1].value);
                let diameter = states[2].value.clamp(0., 2_000_000.);
                TreeMotionNode {
                    key: key.clone(),
                    data: track.data.clone(),
                    phase: track.phase,
                    instance: track.instance,
                    props: TreeMotionNodeProps {
                        x,
                        y,
                        diameter,
                        color: color(&states[3..]),
                    },
                    states,
                }
            })
            .collect();
        let links = self
            .links
            .ordered()
            .map(|(key, track)| {
                let states = track.states();
                TreeMotionLink {
                    key: key.clone(),
                    data: track.data.clone(),
                    phase: track.phase,
                    instance: track.instance,
                    props: TreeMotionLinkProps {
                        source: [coordinate(states[0].value), coordinate(states[1].value)],
                        target: [coordinate(states[2].value), coordinate(states[3].value)],
                        thickness: states[4].value.clamp(0., 10_000.),
                        color: color(&states[5..]),
                    },
                    states,
                }
            })
            .collect();
        let labels = self
            .labels
            .ordered()
            .map(|(key, track)| {
                let states = track.states();
                TreeMotionLabel {
                    key: key.clone(),
                    data: track.data.clone(),
                    phase: track.phase,
                    instance: track.instance,
                    props: TreeMotionLabelProps {
                        x: coordinate(states[0].value),
                        y: coordinate(states[1].value),
                        rotation: states[2].value.clamp(-360., 360.),
                    },
                    states,
                }
            })
            .collect();
        self.frame = TreeMotionFrame(Arc::new(FrameData {
            target: self.target.clone(),
            target_serial: self.target_serial,
            sample_serial: self.sample_serial,
            nodes,
            links,
            labels,
            unowned_labels: self.unowned_labels.clone(),
        }));
        Ok(())
    }
}

fn advance(serial: u64, kind: &str) -> Result<u64> {
    serial
        .checked_add(1)
        .ok_or_else(|| ChartError(format!("Tree motion {kind} serial exhausted")))
}
fn coordinate(value: f64) -> f64 {
    value.clamp(-1_000_000., 1_000_000.)
}
fn color(states: &[PropertyState]) -> Color {
    let channel = |index: usize| states[index].value.clamp(0., 255.).round() as u8;
    Color(channel(0), channel(1), channel(2), channel(3))
}

struct Inputs {
    nodes: Vec<Input<String, TreeNodeEventData, 7>>,
    links: Vec<Input<TreeMotionLinkKey, TreeLinkEventData, 9>>,
    labels: Vec<Input<String, TreeMotionLabelData, 3>>,
    unowned_labels: Vec<usize>,
}
impl Inputs {
    fn new(target: &TreeEventFrame) -> Result<Self> {
        target.scene().validate()?;
        let nodes = (0..target.nodes().len())
            .map(|index| {
                let data = target.node(index)?;
                let node = data.node();
                let Color(r, g, b, a) = node.color.unwrap_or(Color(0, 0, 0, 0));
                Ok(Input {
                    key: data.datum().id.clone(),
                    values: [
                        node.position[0],
                        node.position[1],
                        node.radius * 2.,
                        r.into(),
                        g.into(),
                        b.into(),
                        a.into(),
                    ],
                    data,
                })
            })
            .collect::<Result<_>>()?;
        let links = (0..target.links().len())
            .map(|index| {
                let data = target.link(index)?;
                let link = data.link();
                let style = data.style();
                let Color(r, g, b, a) = style.color;
                Ok(Input {
                    key: TreeMotionLinkKey {
                        source_id: data.source_datum().id.clone(),
                        target_id: data.target_datum().id.clone(),
                    },
                    values: [
                        link.source_position[0],
                        link.source_position[1],
                        link.target_position[0],
                        link.target_position[1],
                        style.thickness,
                        r.into(),
                        g.into(),
                        b.into(),
                        a.into(),
                    ],
                    data,
                })
            })
            .collect::<Result<_>>()?;
        let mut labels = Vec::new();
        let mut unowned_labels = Vec::new();
        for (index, label) in target.scene().labels.iter().enumerate() {
            let Some(owner) = target.scene().label_series.get(&index) else {
                unowned_labels.push(index);
                continue;
            };
            let source_index = *target
                .node_by_id()
                .get(owner)
                .ok_or_else(|| ChartError("Tree motion label owner is missing".into()))?;
            labels.push(Input {
                key: owner.clone(),
                data: TreeMotionLabelData {
                    frame: target.clone(),
                    source_index,
                    label_index: index,
                },
                values: [label.x, label.y, label.rotation],
            });
        }
        Ok(Self {
            nodes,
            links,
            labels,
            unowned_labels,
        })
    }
}
