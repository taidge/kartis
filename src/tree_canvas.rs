//! Native Tree Canvas domain frames and literal, repeatable layer plans.
//!
//! These types do not turn custom paint into source geometry or component
//! slots. Native callbacks are available only on wasm with the `canvas` feature.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;

mod generator;
pub use generator::*;
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
mod native;
#[cfg(all(feature = "canvas", target_arch = "wasm32"))]
pub use native::*;

pub const MAX_TREE_CANVAS_LAYERS: usize = 128;
pub const MAX_TREE_CANVAS_WORK: usize = 200_000;
pub const MAX_TREE_CANVAS_PIXELS: u64 = 67_108_864;

/// Existing layouts capture only their actual Scene labels. This value never
/// claims Nivo's independently computed, disabled label pool can be recovered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeCanvasLabelQualification {
    CurrentScene,
    /// Complete constructor-computed pool. Current Scene label edits remain a
    /// separate view, available through each context's optional scene_index.
    AllComputed,
}

#[derive(Clone, Debug, PartialEq)]
struct FrameData {
    config: ChartConfig,
    layout: TreeLayout,
    activity: Option<Arc<TreeActivityState>>,
    node_by_id: BTreeMap<String, usize>,
    mark_indices: BTreeMap<String, usize>,
    label_qualification: TreeCanvasLabelQualification,
    labels: Vec<Label>,
    label_scene_indices: Vec<Option<usize>>,
}

/// Immutable owned source/configuration for one native paint candidate.
#[derive(Clone, Debug)]
pub struct TreeCanvasFrame(Arc<FrameData>);
impl PartialEq for TreeCanvasFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.0 == other.0
                && self.computed_link_styles() == other.computed_link_styles()
                && self.layout().computed_canvas_labels()
                    == other.layout().computed_canvas_labels())
    }
}
impl TreeCanvasFrame {
    pub fn new(config: ChartConfig, layout: TreeLayout) -> Result<Self> {
        tree_widget_frame(&layout)?;
        Self::capture(
            config,
            layout,
            None,
            TreeCanvasLabelQualification::CurrentScene,
        )
    }
    pub fn with_activity(config: ChartConfig, layout: TreeActivityLayout) -> Result<Self> {
        tree_activity_frame(&layout)?;
        Self::capture(
            config,
            layout.layout,
            Some(layout.activity),
            TreeCanvasLabelQualification::CurrentScene,
        )
    }
    pub fn from_all_computed(config: ChartConfig, layout: TreeLayout) -> Result<Self> {
        tree_widget_frame(&layout)?;
        Self::capture(
            config,
            layout,
            None,
            TreeCanvasLabelQualification::AllComputed,
        )
    }
    pub fn from_all_computed_activity(
        config: ChartConfig,
        layout: TreeActivityLayout,
    ) -> Result<Self> {
        tree_activity_frame(&layout)?;
        Self::capture(
            config,
            layout.layout,
            Some(layout.activity),
            TreeCanvasLabelQualification::AllComputed,
        )
    }
    fn capture(
        config: ChartConfig,
        layout: TreeLayout,
        activity: Option<Arc<TreeActivityState>>,
        label_qualification: TreeCanvasLabelQualification,
    ) -> Result<Self> {
        config.validate()?;
        ensure(
            config.width == layout.scene.width
                && config.height == layout.scene.height
                && ClipRect::plot(&config) == layout.plot,
            "Tree Canvas config differs from layout dimensions or plot",
        )?;
        ensure(
            layout.computed_link_styles().len() == layout.links.len(),
            "Tree Canvas computed link style count differs",
        )?;
        let node_by_id = layout
            .source()
            .iter()
            .enumerate()
            .map(|(i, datum)| (datum.id.clone(), i))
            .collect();
        let mark_indices = layout
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.clone(), i))
            .collect();
        let labels = if label_qualification == TreeCanvasLabelQualification::AllComputed {
            let computed = layout.computed_canvas_labels().ok_or_else(|| {
                ChartError("Tree Canvas requires an authoritative AllComputed constructor".into())
            })?;
            let mut owners = BTreeSet::new();
            for owner in layout.scene.label_series.values() {
                ensure(
                    owners.insert(owner),
                    "Tree Canvas current label owner is ambiguous",
                )?;
            }
            computed.iter().map(|c| c.label.clone()).collect()
        } else {
            layout.scene.labels.clone()
        };
        let label_scene_indices =
            if label_qualification == TreeCanvasLabelQualification::AllComputed {
                let indices = layout
                    .scene
                    .label_series
                    .iter()
                    .map(|(&i, id)| (id.as_str(), i))
                    .collect::<BTreeMap<_, _>>();
                layout
                    .computed_canvas_labels()
                    .expect("validated computed pool")
                    .iter()
                    .map(|c| {
                        indices
                            .get(layout.source()[c.source_index].id.as_str())
                            .copied()
                    })
                    .collect()
            } else {
                (0..labels.len()).map(Some).collect()
            };
        Ok(Self(Arc::new(FrameData {
            config,
            layout,
            activity,
            node_by_id,
            mark_indices,
            label_qualification,
            labels,
            label_scene_indices,
        })))
    }
    pub fn config(&self) -> &ChartConfig {
        &self.0.config
    }
    pub fn theme(&self) -> &Theme {
        &self.0.config.theme
    }
    pub fn layout(&self) -> &TreeLayout {
        &self.0.layout
    }
    pub fn source(&self) -> &[HierarchyDatum] {
        self.0.layout.source()
    }
    pub fn activity(&self) -> Option<&TreeActivityState> {
        self.0.activity.as_deref()
    }
    pub fn node_by_id(&self) -> &BTreeMap<String, usize> {
        &self.0.node_by_id
    }
    pub fn computed_link_styles(&self) -> &[TreeComputedLinkStyle] {
        self.0.layout.computed_link_styles()
    }
    pub fn labels(&self) -> &[Label] {
        &self.0.labels
    }
    pub fn source_mark(&self, id: &str) -> Option<&Mark> {
        self.0
            .mark_indices
            .get(id)
            .map(|&i| &self.layout().scene.marks[i])
    }
    pub fn label_qualification(&self) -> TreeCanvasLabelQualification {
        self.0.label_qualification
    }
    /// Raw links belong to both source nodes even when their decorative Mark
    /// series is empty or disabled painting omitted that Mark altogether.
    pub fn role_is_hidden(&self, role: TreeCanvasRole, hidden: &BTreeSet<String>) -> Result<bool> {
        Ok(match role {
            TreeCanvasRole::Node(i) => hidden.contains(&self.node_context(i)?.datum.id),
            TreeCanvasRole::Link(i) => {
                let l = self
                    .layout()
                    .links
                    .get(i)
                    .ok_or_else(|| ChartError("Tree Canvas link index is missing".into()))?;
                hidden.contains(&self.source()[l.source_index].id)
                    || hidden.contains(&self.source()[l.target_index].id)
            }
            TreeCanvasRole::Label(i) => self
                .label_context(i)?
                .owner
                .is_some_and(|id| hidden.contains(id)),
            TreeCanvasRole::Mesh | TreeCanvasRole::Custom => false,
        })
    }
    pub fn node_context(&self, index: usize) -> Result<TreeCanvasNodeContext<'_>> {
        let node = self
            .layout()
            .nodes
            .get(index)
            .ok_or_else(|| ChartError("Tree Canvas node index is missing".into()))?;
        Ok(TreeCanvasNodeContext {
            frame: self,
            datum: &self.source()[index],
            source_index: index,
            node,
            mark: node.mark_id.as_deref().and_then(|id| self.source_mark(id)),
            is_active: self.activity().and_then(|a| a.nodes[index]),
        })
    }
    pub fn label_context(&self, index: usize) -> Result<TreeCanvasLabelContext<'_>> {
        let scene = &self.layout().scene;
        let label = self
            .labels()
            .get(index)
            .ok_or_else(|| ChartError("Tree Canvas label index is missing".into()))?;
        if self.label_qualification() == TreeCanvasLabelQualification::AllComputed {
            let computed = &self
                .layout()
                .computed_canvas_labels()
                .expect("captured AllComputed pool")[index];
            let source_index = computed.source_index;
            let owner = self.source()[source_index].id.as_str();
            let scene_index = self.0.label_scene_indices[index];
            return Ok(TreeCanvasLabelContext {
                frame: self,
                label_index: index,
                label,
                owner: Some(owner),
                source_index: Some(source_index),
                datum: Some(&self.source()[source_index]),
                node: Some(&self.layout().nodes[source_index]),
                is_active: self.activity().and_then(|a| a.nodes[source_index]),
                font: computed.font.clone(),
                whitespace: computed.whitespace,
                stroke: computed.stroke,
                clip: computed.clip.as_ref(),
                scene_index,
                scene_label: scene_index.map(|i| &scene.labels[i]),
                eligible: computed.eligible,
            });
        }
        let owner = scene.label_series.get(&index).map(String::as_str);
        let source_index = owner.and_then(|id| self.node_by_id().get(id).copied());
        Ok(TreeCanvasLabelContext {
            frame: self,
            label_index: index,
            label,
            owner,
            source_index,
            datum: source_index.map(|i| &self.source()[i]),
            node: source_index.map(|i| &self.layout().nodes[i]),
            is_active: source_index.and_then(|i| self.activity().and_then(|a| a.nodes[i])),
            font: scene.resolved_label_font(index),
            whitespace: scene.typography.label_whitespace(index),
            stroke: scene.typography.label_strokes.get(&index).copied(),
            clip: scene.label_clips.get(&index),
            scene_index: Some(index),
            scene_label: Some(&scene.labels[index]),
            eligible: true,
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct TreeCanvasNodeContext<'a> {
    pub frame: &'a TreeCanvasFrame,
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub node: &'a TreeNode,
    pub mark: Option<&'a Mark>,
    pub is_active: Option<bool>,
}

#[derive(Debug)]
pub struct TreeCanvasLinkContext<'a> {
    pub frame: &'a TreeCanvasFrame,
    pub link_index: usize,
    pub link: &'a TreeLink,
    pub source_node: &'a TreeNode,
    pub target_node: &'a TreeNode,
    pub style: &'a TreeComputedLinkStyle,
    pub mark: Option<&'a Mark>,
    pub is_active: Option<bool>,
    pub generator: &'a mut TreeCanvasLinkGenerator,
}

#[derive(Debug)]
pub struct TreeCanvasLayerContext<'a> {
    pub frame: &'a TreeCanvasFrame,
    pub layer_index: usize,
    pub layer: &'a TreeCanvasLayer,
    pub generator: &'a mut TreeCanvasLinkGenerator,
}

#[derive(Clone, Debug)]
pub struct TreeCanvasLabelContext<'a> {
    pub frame: &'a TreeCanvasFrame,
    pub label_index: usize,
    pub label: &'a Label,
    pub owner: Option<&'a str>,
    pub source_index: Option<usize>,
    pub datum: Option<&'a HierarchyDatum>,
    pub node: Option<&'a TreeNode>,
    pub is_active: Option<bool>,
    pub font: ResolvedFont,
    pub whitespace: LabelWhitespace,
    pub stroke: Option<crate::LabelStroke>,
    pub clip: Option<&'a ClipPath>,
    /// Ordinal in the current Scene, distinct from the active computed pool.
    pub scene_index: Option<usize>,
    pub scene_label: Option<&'a Label>,
    pub eligible: bool,
}

/// Layer identity is separate from an occurrence: repeating any variant is
/// intentional and preserved, including a Links layer with no edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeCanvasLayer {
    Links,
    Nodes,
    Labels,
    Mesh,
    Custom(String),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TreeCanvasRole {
    Node(usize),
    Link(usize),
    Label(usize),
    Mesh,
    Custom,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeCanvasLayerPlan {
    pub layer_index: usize,
    pub layer: TreeCanvasLayer,
    roles: Vec<TreeCanvasRole>,
}
impl TreeCanvasLayerPlan {
    pub fn roles(&self) -> &[TreeCanvasRole] {
        &self.roles
    }
}

/// Bounds host work and allocations, not arbitrary native operations performed
/// by trusted callbacks. Raw context access is not an untrusted-code sandbox.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeCanvasBudget {
    pub layers: usize,
    pub work: usize,
    pub source_commands: usize,
    pub live_pixels: u64,
}
impl Default for TreeCanvasBudget {
    fn default() -> Self {
        Self {
            layers: MAX_TREE_CANVAS_LAYERS,
            work: MAX_TREE_CANVAS_WORK,
            source_commands: MAX_TREE_CANVAS_WORK,
            live_pixels: MAX_TREE_CANVAS_PIXELS,
        }
    }
}
impl TreeCanvasBudget {
    pub fn validate(self) -> Result<()> {
        ensure(
            self.layers <= MAX_TREE_CANVAS_LAYERS
                && self.work <= MAX_TREE_CANVAS_WORK
                && self.source_commands <= MAX_TREE_CANVAS_WORK
                && self.live_pixels <= MAX_TREE_CANVAS_PIXELS,
            "Tree Canvas budget exceeds supported bounds",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeCanvasPlan {
    frame: TreeCanvasFrame,
    layers: Vec<TreeCanvasLayerPlan>,
    budget: TreeCanvasBudget,
    work: usize,
}
impl TreeCanvasPlan {
    pub fn defaults(frame: &TreeCanvasFrame) -> Result<Self> {
        Self::new(
            frame,
            vec![
                TreeCanvasLayer::Links,
                TreeCanvasLayer::Nodes,
                TreeCanvasLayer::Labels,
                TreeCanvasLayer::Mesh,
            ],
            TreeCanvasBudget::default(),
        )
    }
    pub fn new(
        frame: &TreeCanvasFrame,
        layers: Vec<TreeCanvasLayer>,
        budget: TreeCanvasBudget,
    ) -> Result<Self> {
        budget.validate()?;
        ensure(
            layers.len() <= budget.layers,
            "Tree Canvas layer budget exceeded",
        )?;
        let mut bfs = frame.layout().root_index().into_iter().collect::<Vec<_>>();
        let mut cursor = 0;
        while cursor < bfs.len() {
            let i = bfs[cursor];
            bfs.extend(frame.layout().nodes[i].children.iter().copied());
            cursor += 1;
        }
        let mut commands = 0usize;
        let mut work = 0usize;
        let mut planned = Vec::with_capacity(layers.len());
        for (layer_index, layer) in layers.into_iter().enumerate() {
            let roles: Vec<_> = match &layer {
                TreeCanvasLayer::Nodes => bfs.iter().copied().map(TreeCanvasRole::Node).collect(),
                TreeCanvasLayer::Links => (0..frame.layout().links.len())
                    .map(TreeCanvasRole::Link)
                    .collect(),
                TreeCanvasLayer::Labels => (0..frame.labels().len())
                    .filter(|&i| {
                        frame.label_qualification() == TreeCanvasLabelQualification::CurrentScene
                            || frame
                                .layout()
                                .computed_canvas_labels()
                                .expect("captured pool")[i]
                                .eligible
                    })
                    .map(TreeCanvasRole::Label)
                    .collect(),
                TreeCanvasLayer::Mesh => vec![TreeCanvasRole::Mesh],
                TreeCanvasLayer::Custom(key) => {
                    ensure(
                        !key.is_empty() && key.len() <= MAX_LABEL_BYTES,
                        "invalid Tree Canvas custom layer key",
                    )?;
                    vec![TreeCanvasRole::Custom]
                }
            };
            work = work
                .checked_add(roles.len())
                .ok_or_else(|| ChartError("Tree Canvas work overflow".into()))?;
            ensure(
                work <= budget.work,
                "Tree Canvas renderer work budget exceeded",
            )?;
            for role in &roles {
                let mark = match *role {
                    TreeCanvasRole::Node(i) => frame.layout().nodes[i]
                        .mark_id
                        .as_deref()
                        .and_then(|id| frame.source_mark(id)),
                    TreeCanvasRole::Link(i) => frame.layout().links[i]
                        .mark_id
                        .as_deref()
                        .and_then(|id| frame.source_mark(id)),
                    _ => None,
                };
                if let Some(mark) = mark {
                    let cost = match &mark.geometry {
                        Geometry::Rect { .. } => 5,
                        Geometry::Circle { .. } => 6,
                        Geometry::Path(p) => p.len(),
                    };
                    commands = commands
                        .checked_add(cost)
                        .ok_or_else(|| ChartError("Tree Canvas source command overflow".into()))?;
                    ensure(
                        commands <= budget.source_commands,
                        "Tree Canvas source command budget exceeded",
                    )?;
                }
            }
            planned.push(TreeCanvasLayerPlan {
                layer_index,
                layer,
                roles,
            });
        }
        Ok(Self {
            frame: frame.clone(),
            layers: planned,
            budget,
            work,
        })
    }
    pub fn frame(&self) -> &TreeCanvasFrame {
        &self.frame
    }
    pub fn layers(&self) -> &[TreeCanvasLayerPlan] {
        &self.layers
    }
    pub fn budget(&self) -> TreeCanvasBudget {
        self.budget
    }
    pub fn work(&self) -> usize {
        self.work
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TreeCanvasMeshOptions {
    pub debug: bool,
    pub use_mesh: bool,
    pub interactive: bool,
    pub detection_radius: f64,
}
impl Default for TreeCanvasMeshOptions {
    fn default() -> Self {
        Self {
            debug: false,
            use_mesh: true,
            interactive: true,
            detection_radius: f64::INFINITY,
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeCanvasOptions {
    pub pixel_ratio: f64,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub mesh: TreeCanvasMeshOptions,
    /// Accepted/sibling allocations already charged to this owner's budget.
    pub reserved_pixels: u64,
}
impl Default for TreeCanvasOptions {
    fn default() -> Self {
        Self {
            pixel_ratio: 1.,
            view: ZoomTransform::default(),
            hidden: BTreeSet::new(),
            mesh: TreeCanvasMeshOptions::default(),
            reserved_pixels: 0,
        }
    }
}
impl TreeCanvasOptions {
    pub fn backing_size(
        &self,
        frame: &TreeCanvasFrame,
        budget: TreeCanvasBudget,
        copies: u64,
    ) -> Result<[u32; 2]> {
        budget.validate()?;
        ensure(
            (1..=2).contains(&copies),
            "invalid Tree Canvas backing copy count",
        )?;
        self.view.validate()?;
        ensure(
            self.pixel_ratio.is_finite() && (0.25..=8.).contains(&self.pixel_ratio),
            "invalid Tree Canvas pixel ratio",
        )?;
        ensure(
            self.mesh.detection_radius == f64::INFINITY
                || (self.mesh.detection_radius.is_finite()
                    && (0. ..=1e6).contains(&self.mesh.detection_radius)),
            "invalid Tree Canvas mesh radius",
        )?;
        let width = (frame.config().width * self.pixel_ratio).ceil() as u32;
        let height = (frame.config().height * self.pixel_ratio).ceil() as u32;
        let pixels = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|v| v.checked_mul(copies))
            .and_then(|v| v.checked_add(self.reserved_pixels));
        ensure(
            pixels.is_some_and(|p| p <= budget.live_pixels),
            "Tree Canvas live backing pixel budget exceeded",
        )?;
        Ok([width, height])
    }
}
