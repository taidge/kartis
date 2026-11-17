//! Literal Tree SVG render occurrences over one immutable business source.
//!
//! This domain does not clone logical Marks to represent repeated painting.
//! Retained Glory layer factories are provided by the browser SVG owner.
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;

pub const MAX_TREE_SVG_LAYERS: usize = 128;
pub const MAX_TREE_SVG_WORK: usize = 200_000;
pub const MAX_TREE_SVG_SOURCE_COMMANDS: usize = 2_000_000;
pub const MAX_TREE_SVG_COMPONENTS: usize = 128;
pub const MAX_TREE_SVG_KEY_BYTES: usize = 128;

fn valid_key(key: &str) -> bool {
    !key.trim().is_empty() && key.len() <= MAX_TREE_SVG_KEY_BYTES
}

fn geometry_cost(geometry: &Geometry) -> usize {
    match geometry {
        Geometry::Circle { .. } => 6,
        Geometry::Rect { .. } => 5,
        Geometry::Path(path) => path.len(),
    }
}

/// An occurrence kind, distinct from the stable key of its retained instance.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeSvgLayerKind {
    Links,
    Nodes,
    Labels,
    Mesh,
    /// Actual unowned Scene annotations/title items, in their source order.
    SceneExtras,
    /// A registered whole-tree Glory component factory.
    Custom(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSvgLayer {
    pub key: String,
    pub kind: TreeSvgLayerKind,
}
impl TreeSvgLayer {
    pub fn new(key: impl Into<String>, kind: TreeSvgLayerKind) -> Result<Self> {
        let layer = Self {
            key: key.into(),
            kind,
        };
        layer.validate()?;
        Ok(layer)
    }
    fn validate(&self) -> Result<()> {
        ensure(valid_key(&self.key), "invalid Tree SVG occurrence key")?;
        if let TreeSvgLayerKind::Custom(registry) = &self.kind {
            ensure(valid_key(registry), "invalid Tree SVG custom registry key")?;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TreeSvgExtrasPolicy {
    #[default]
    RequireLayer,
    Omit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TreeSvgBudget {
    pub layers: usize,
    pub work: usize,
    pub source_commands: usize,
    pub components: usize,
}
impl Default for TreeSvgBudget {
    fn default() -> Self {
        Self {
            layers: MAX_TREE_SVG_LAYERS,
            work: MAX_TREE_SVG_WORK,
            source_commands: MAX_TREE_SVG_SOURCE_COMMANDS,
            components: MAX_TREE_SVG_COMPONENTS,
        }
    }
}
impl TreeSvgBudget {
    pub fn validate(self) -> Result<()> {
        ensure(
            self.layers <= MAX_TREE_SVG_LAYERS
                && self.work <= MAX_TREE_SVG_WORK
                && self.source_commands <= MAX_TREE_SVG_SOURCE_COMMANDS
                && self.components <= MAX_TREE_SVG_COMPONENTS,
            "invalid Tree SVG budget",
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct FrameData {
    source: TreeCanvasFrame,
    node_items: Vec<DrawItem>,
    link_items: Vec<DrawItem>,
    label_items: Vec<DrawItem>,
    extras: Vec<DrawItem>,
}

/// Complete validated data for custom layers, including unpainted nodes and
/// computed link styles. Positions retain the existing outer Scene coordinates;
/// `inner_position` converts them to a plot-local custom carrier coordinate.
#[derive(Clone, Debug)]
pub struct TreeSvgFrame(Arc<FrameData>);
impl PartialEq for TreeSvgFrame {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}
impl TreeSvgFrame {
    pub fn new(config: ChartConfig, layout: TreeLayout) -> Result<Self> {
        Self::capture(TreeCanvasFrame::new(config, layout)?)
    }
    pub fn with_activity(config: ChartConfig, layout: TreeActivityLayout) -> Result<Self> {
        Self::capture(TreeCanvasFrame::with_activity(config, layout)?)
    }
    fn capture(source: TreeCanvasFrame) -> Result<Self> {
        let layout = source.layout();
        let mark_by_id: BTreeMap<_, _> = layout
            .scene
            .marks
            .iter()
            .enumerate()
            .map(|(i, mark)| (mark.id.as_str(), i))
            .collect();
        let mut bfs = layout.root_index().into_iter().collect::<Vec<_>>();
        let mut cursor = 0;
        while cursor < bfs.len() {
            bfs.extend(layout.nodes[bfs[cursor]].children.iter().copied());
            cursor += 1;
        }
        ensure(
            bfs.len() == layout.nodes.len(),
            "Tree SVG node traversal differs from source",
        )?;
        let mut owned = BTreeSet::new();
        let mut resolve = |id: Option<&str>| -> Result<Option<DrawItem>> {
            id.map(|id| {
                let index = *mark_by_id
                    .get(id)
                    .ok_or_else(|| ChartError("Tree SVG source Mark is missing".into()))?;
                ensure(owned.insert(index), "Tree SVG source role is ambiguous")?;
                Ok(DrawItem::Mark(index))
            })
            .transpose()
        };
        let mut node_items = Vec::new();
        for index in bfs {
            if let Some(item) = resolve(layout.nodes[index].mark_id.as_deref())? {
                node_items.push(item);
            }
        }
        let mut link_items = Vec::new();
        for link in &layout.links {
            if let Some(item) = resolve(link.mark_id.as_deref())? {
                link_items.push(item);
            }
        }
        let label_items: Vec<_> = (0..layout.scene.labels.len())
            .filter(|index| {
                layout
                    .scene
                    .label_series
                    .get(index)
                    .is_some_and(|id| source.node_by_id().contains_key(id))
            })
            .map(DrawItem::Label)
            .collect();
        let labels: BTreeSet<_> = label_items
            .iter()
            .filter_map(|item| match item {
                DrawItem::Label(index) => Some(*index),
                _ => None,
            })
            .collect();
        let extras = layout
            .scene
            .draw_items()
            .into_iter()
            .filter(|item| match item {
                DrawItem::Mark(index) => !owned.contains(index),
                DrawItem::Label(index) => !labels.contains(index),
            })
            .collect();
        Ok(Self(Arc::new(FrameData {
            source,
            node_items,
            link_items,
            label_items,
            extras,
        })))
    }
    pub fn config(&self) -> &ChartConfig {
        self.0.source.config()
    }
    pub fn theme(&self) -> &Theme {
        self.0.source.theme()
    }
    pub fn layout(&self) -> &TreeLayout {
        self.0.source.layout()
    }
    pub fn source(&self) -> &[HierarchyDatum] {
        self.0.source.source()
    }
    pub fn nodes(&self) -> &[TreeNode] {
        &self.layout().nodes
    }
    pub fn links(&self) -> &[TreeLink] {
        &self.layout().links
    }
    pub fn activity(&self) -> Option<&TreeActivityState> {
        self.0.source.activity()
    }
    pub fn node_by_id(&self) -> &BTreeMap<String, usize> {
        self.0.source.node_by_id()
    }
    pub fn computed_link_styles(&self) -> &[TreeComputedLinkStyle] {
        self.0.source.computed_link_styles()
    }
    pub fn plot(&self) -> ClipRect {
        self.layout().plot
    }
    pub fn inner_size(&self) -> [f64; 2] {
        [self.plot().width, self.plot().height]
    }
    pub fn inner_position(&self, position: [f64; 2]) -> Result<[f64; 2]> {
        ensure(
            position.iter().all(|v| v.is_finite()),
            "invalid Tree SVG position",
        )?;
        let point = [position[0] - self.plot().x, position[1] - self.plot().y];
        ensure(
            point.iter().all(|v| v.is_finite()),
            "Tree SVG inner position overflow",
        )?;
        Ok(point)
    }
    pub fn source_mark(&self, id: &str) -> Option<&Mark> {
        self.0.source.source_mark(id)
    }
    pub fn extras(&self) -> &[DrawItem] {
        &self.0.extras
    }
    pub fn link_generator(&self) -> TreeSvgLinkGenerator {
        TreeSvgLinkGenerator::new(
            self.layout().options().style.links.curve,
            self.layout().options().orientation,
        )
    }
    fn items(&self, kind: &TreeSvgLayerKind) -> &[DrawItem] {
        match kind {
            TreeSvgLayerKind::Nodes => &self.0.node_items,
            TreeSvgLayerKind::Links => &self.0.link_items,
            TreeSvgLayerKind::Labels => &self.0.label_items,
            TreeSvgLayerKind::SceneExtras => &self.0.extras,
            TreeSvgLayerKind::Mesh | TreeSvgLayerKind::Custom(_) => &[],
        }
    }
}

/// SVG-only endpoint generator. Native Canvas bindings are not exposed.
#[derive(Clone, Debug)]
pub struct TreeSvgLinkGenerator(TreeCanvasLinkGenerator);
impl TreeSvgLinkGenerator {
    pub fn new(curve: TreeLinkCurve, orientation: TreeOrientation) -> Self {
        Self(TreeCanvasLinkGenerator::new(curve, orientation))
    }
    pub fn curve(&self) -> TreeLinkCurve {
        self.0.curve()
    }
    pub fn set_curve(&mut self, curve: TreeLinkCurve) {
        self.0.set_curve(curve);
    }
    pub fn orientation(&self) -> TreeOrientation {
        self.0.orientation()
    }
    pub fn set_orientation(&mut self, orientation: TreeOrientation) {
        self.0.set_orientation(orientation);
    }
    pub fn geometry(&self, source: [f64; 2], target: [f64; 2]) -> Result<Geometry> {
        self.0.geometry(source, target)
    }
    pub fn svg_path(&self, source: [f64; 2], target: [f64; 2]) -> Result<String> {
        self.0.svg_path(source, target)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeSvgOccurrence {
    pub index: usize,
    pub layer: TreeSvgLayer,
    items: Vec<DrawItem>,
}
impl TreeSvgOccurrence {
    pub fn items(&self) -> &[DrawItem] {
        &self.items
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeSvgPlan {
    frame: TreeSvgFrame,
    layers: Vec<TreeSvgOccurrence>,
    budget: TreeSvgBudget,
    work: usize,
}
impl TreeSvgPlan {
    pub fn default_layers() -> Vec<TreeSvgLayer> {
        [
            ("extras", TreeSvgLayerKind::SceneExtras),
            ("links", TreeSvgLayerKind::Links),
            ("nodes", TreeSvgLayerKind::Nodes),
            ("labels", TreeSvgLayerKind::Labels),
            ("mesh", TreeSvgLayerKind::Mesh),
        ]
        .into_iter()
        .map(|(key, kind)| TreeSvgLayer {
            key: key.into(),
            kind,
        })
        .collect()
    }
    pub fn defaults(frame: &TreeSvgFrame) -> Result<Self> {
        Self::new(
            frame,
            Self::default_layers(),
            TreeSvgBudget::default(),
            TreeSvgExtrasPolicy::RequireLayer,
        )
    }
    /// Check frame-independent metadata before any source function or Widget factory.
    pub fn validate_layers(layers: &[TreeSvgLayer], budget: TreeSvgBudget) -> Result<()> {
        budget.validate()?;
        ensure(
            layers.len() <= budget.layers,
            "Tree SVG layer budget exceeded",
        )?;
        let mut keys = BTreeSet::new();
        for layer in layers {
            layer.validate()?;
            ensure(keys.insert(&layer.key), "duplicate Tree SVG occurrence key")?;
        }
        Ok(())
    }
    pub fn labels_required(layers: &[TreeSvgLayer]) -> bool {
        layers
            .iter()
            .any(|layer| layer.kind == TreeSvgLayerKind::Labels)
    }
    pub fn new(
        frame: &TreeSvgFrame,
        layers: Vec<TreeSvgLayer>,
        budget: TreeSvgBudget,
        extras: TreeSvgExtrasPolicy,
    ) -> Result<Self> {
        Self::validate_layers(&layers, budget)?;
        ensure(
            frame.extras().is_empty()
                || extras == TreeSvgExtrasPolicy::Omit
                || layers
                    .iter()
                    .any(|layer| layer.kind == TreeSvgLayerKind::SceneExtras),
            "Tree SVG plan must explicitly include or omit Scene extras",
        )?;
        let mut work = 0usize;
        let mut commands = 0usize;
        let mut custom = 0usize;
        let mut occurrences = Vec::with_capacity(layers.len());
        for (index, layer) in layers.into_iter().enumerate() {
            let items = frame.items(&layer.kind).to_vec();
            let item_positions: BTreeMap<_, _> = items
                .iter()
                .enumerate()
                .filter_map(|(position, item)| match item {
                    DrawItem::Mark(index) => {
                        Some((frame.layout().scene.marks[*index].id.as_str(), position))
                    }
                    _ => None,
                })
                .collect();
            for group in &frame.layout().scene.opacity_groups {
                let positions: Vec<_> = group
                    .members
                    .iter()
                    .filter_map(|id| item_positions.get(id.as_str()).copied())
                    .collect();
                ensure(
                    positions.is_empty()
                        || (positions.len() == group.members.len()
                            && positions.windows(2).all(|p| p[1] == p[0] + 1)),
                    "Tree SVG occurrence splits or reorders an opacity group",
                )?;
            }
            let cost = if matches!(
                layer.kind,
                TreeSvgLayerKind::Custom(_) | TreeSvgLayerKind::Mesh
            ) {
                1
            } else {
                items.len()
            };
            work = work
                .checked_add(cost)
                .ok_or_else(|| ChartError("Tree SVG work overflow".into()))?;
            ensure(work <= budget.work, "Tree SVG source work budget exceeded")?;
            if matches!(layer.kind, TreeSvgLayerKind::Custom(_)) {
                custom += 1;
                ensure(
                    custom <= budget.components,
                    "Tree SVG component budget exceeded",
                )?;
            }
            for item in &items {
                let count = match item {
                    DrawItem::Mark(i) => {
                        let mark = &frame.layout().scene.marks[*i];
                        let mut cost = geometry_cost(&mark.geometry);
                        for additional in [
                            mark.clip_path
                                .as_ref()
                                .map_or(0, |clip| geometry_cost(&clip.geometry)),
                            mark.stroke_dash
                                .as_ref()
                                .map_or(0, |dash| dash.segments.len()),
                            mark.fill_paint
                                .as_ref()
                                .map_or(0, |paint| paint.stops().len().max(1)),
                        ] {
                            cost = cost.checked_add(additional).ok_or_else(|| {
                                ChartError("Tree SVG resource cost overflow".into())
                            })?;
                        }
                        cost
                    }
                    DrawItem::Label(index) => frame
                        .layout()
                        .scene
                        .label_clips
                        .get(index)
                        .map_or(1, |clip| 1 + geometry_cost(&clip.geometry)),
                };
                commands = commands
                    .checked_add(count)
                    .ok_or_else(|| ChartError("Tree SVG command overflow".into()))?;
                ensure(
                    commands <= budget.source_commands,
                    "Tree SVG source command budget exceeded",
                )?;
            }
            occurrences.push(TreeSvgOccurrence {
                index,
                layer,
                items,
            });
        }
        Ok(Self {
            frame: frame.clone(),
            layers: occurrences,
            budget,
            work,
        })
    }
    pub fn frame(&self) -> &TreeSvgFrame {
        &self.frame
    }
    pub fn layers(&self) -> &[TreeSvgOccurrence] {
        &self.layers
    }
    pub fn budget(&self) -> TreeSvgBudget {
        self.budget
    }
    pub fn work(&self) -> usize {
        self.work
    }
    /// Original source Mark indices in actual literal occurrence order.
    /// This is a projection descriptor, not a claim that every Mark has ink or
    /// a canonical owner. The Surface applies hidden/replacement/paint checks.
    pub fn mark_indices(&self) -> Vec<usize> {
        self.layers
            .iter()
            .flat_map(|layer| &layer.items)
            .filter_map(|item| match item {
                DrawItem::Mark(index) => Some(*index),
                _ => None,
            })
            .collect()
    }
}

/// Build labels only when the literal SVG list contains a Labels occurrence.
/// Repeated Labels occurrences do not replay the borrowed getter. Original
/// source options remain captured; old constructors retain their own schedule.
pub fn tree_svg_layout(
    config: &ChartConfig,
    source: &[HierarchyDatum],
    options: &TreeOptions,
    layers: &[TreeSvgLayer],
) -> Result<TreeLayout> {
    tree_svg_layout_with_factories(
        config,
        source,
        options,
        layers,
        &mut TreeFactories::default(),
    )
}

pub fn tree_svg_layout_with_factories(
    config: &ChartConfig,
    source: &[HierarchyDatum],
    options: &TreeOptions,
    layers: &[TreeSvgLayer],
    factories: &mut TreeFactories<'_>,
) -> Result<TreeLayout> {
    TreeSvgPlan::validate_layers(layers, TreeSvgBudget::default())?;
    crate::radial::layout_for_svg(
        config,
        source,
        options,
        factories,
        TreeSvgPlan::labels_required(layers),
    )
}

pub fn tree_svg_layout_with_activity(
    config: &ChartConfig,
    source: &[HierarchyDatum],
    options: &TreeOptions,
    layers: &[TreeSvgLayer],
    base: &mut TreeFactories<'_>,
    activity: &TreeActivityOptions,
    modifiers: &mut TreeActivityFactories<'_>,
) -> Result<TreeActivityLayout> {
    TreeSvgPlan::validate_layers(layers, TreeSvgBudget::default())?;
    crate::radial::activity_for_svg(config, source, options, layers, base, activity, modifiers)
}
