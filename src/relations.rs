//! Explicit graph identities and transactional, topology-based highlighting.
//! Tree/Chord/Sankey predicates follow local Nivo 0.99 hooks.ts/Sankey.tsx.
//! Network neighbor queries are an explicit extension; no series-name inference.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::scene::ensure;
use crate::*;

mod charts;
pub use charts::*;
pub const MAX_RELATION_NODES: usize = 8_000;
pub const MAX_RELATION_EDGES: usize = 16_000;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RelationFocus {
    Node(String),
    Edge(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationNode {
    pub id: String,
    pub mark_ids: Vec<String>,
    pub label_indices: Vec<usize>,
    /// Source visibility, independent of the runtime hidden set. A node can
    /// exist without ink (e.g. a tree's omitted root) and still be visible here.
    pub visible: bool,
}
impl RelationNode {
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            mark_ids: vec![],
            label_indices: vec![],
            visible: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub mark_ids: Vec<String>,
    pub label_indices: Vec<usize>,
    pub visible: bool,
}
impl RelationEdge {
    pub fn new(
        id: impl Into<String>,
        source: impl Into<String>,
        target: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            source: source.into(),
            target: target.into(),
            mark_ids: vec![],
            label_indices: vec![],
            visible: true,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RelationGraph {
    nodes: Vec<RelationNode>,
    edges: Vec<RelationEdge>,
    by_node: BTreeMap<String, usize>,
    by_edge: BTreeMap<String, usize>,
    incoming: Vec<Vec<usize>>,
    outgoing: Vec<Vec<usize>>,
    mark_owners: BTreeMap<String, RelationFocus>,
    label_owners: BTreeMap<usize, RelationFocus>,
    forest: bool,
}
fn key(id: &str, maximum: usize) -> bool {
    !id.trim().is_empty() && id.len() <= maximum
}
impl RelationGraph {
    pub fn new(mut nodes: Vec<RelationNode>, mut edges: Vec<RelationEdge>) -> Result<Self> {
        ensure(
            nodes.len() <= MAX_RELATION_NODES && edges.len() <= MAX_RELATION_EDGES,
            "relation graph budget exceeded",
        )?;
        ensure(
            nodes.iter().all(|n| key(&n.id, 512))
                && edges
                    .iter()
                    .all(|e| key(&e.id, 1024) && key(&e.source, 512) && key(&e.target, 512)),
            "invalid relation identity",
        )?;
        let mut mark_count = 0usize;
        let mut label_count = 0usize;
        for (marks, labels) in nodes
            .iter()
            .map(|n| (&n.mark_ids, &n.label_indices))
            .chain(edges.iter().map(|e| (&e.mark_ids, &e.label_indices)))
        {
            mark_count = mark_count
                .checked_add(marks.len())
                .ok_or_else(|| ChartError("relation binding overflow".into()))?;
            label_count = label_count
                .checked_add(labels.len())
                .ok_or_else(|| ChartError("relation binding overflow".into()))?;
            ensure(
                mark_count <= MAX_MARKS
                    && label_count <= MAX_MARKS
                    && marks.iter().all(|m| key(m, 1024)),
                "relation binding budget or identity invalid",
            )?;
        }
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        let by_node = nodes
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.clone(), i))
            .collect::<BTreeMap<_, _>>();
        let by_edge = edges
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect::<BTreeMap<_, _>>();
        ensure(
            by_node.len() == nodes.len() && by_edge.len() == edges.len(),
            "duplicate relation node or edge ID",
        )?;
        let mut incoming = vec![vec![]; nodes.len()];
        let mut outgoing = incoming.clone();
        for (i, e) in edges.iter().enumerate() {
            let a = *by_node
                .get(&e.source)
                .ok_or_else(|| ChartError("unknown relation source".into()))?;
            let b = *by_node
                .get(&e.target)
                .ok_or_else(|| ChartError("unknown relation target".into()))?;
            outgoing[a].push(i);
            incoming[b].push(i);
        }
        let mut mark_owners = BTreeMap::new();
        let mut label_owners = BTreeMap::new();
        for (focus, marks, labels) in nodes
            .iter()
            .map(|n| {
                (
                    RelationFocus::Node(n.id.clone()),
                    &n.mark_ids,
                    &n.label_indices,
                )
            })
            .chain(edges.iter().map(|e| {
                (
                    RelationFocus::Edge(e.id.clone()),
                    &e.mark_ids,
                    &e.label_indices,
                )
            }))
        {
            for mark in marks {
                ensure(
                    mark_owners.insert(mark.clone(), focus.clone()).is_none(),
                    "relation mark has multiple owners",
                )?;
            }
            for &slot in labels {
                ensure(
                    label_owners.insert(slot, focus.clone()).is_none(),
                    "relation label has multiple owners",
                )?;
            }
        }
        let mut degree = incoming.iter().map(Vec::len).collect::<Vec<_>>();
        let mut queue = degree
            .iter()
            .enumerate()
            .filter(|(_, d)| **d == 0)
            .map(|(i, _)| i)
            .collect::<VecDeque<_>>();
        let mut visited = 0usize;
        while let Some(i) = queue.pop_front() {
            visited += 1;
            for &e in &outgoing[i] {
                let j = by_node[&edges[e].target];
                degree[j] -= 1;
                if degree[j] == 0 {
                    queue.push_back(j);
                }
            }
        }
        let forest = visited == nodes.len() && incoming.iter().all(|v| v.len() <= 1);
        Ok(Self {
            nodes,
            edges,
            by_node,
            by_edge,
            incoming,
            outgoing,
            mark_owners,
            label_owners,
            forest,
        })
    }
    pub fn nodes(&self) -> &[RelationNode] {
        &self.nodes
    }
    pub fn edges(&self) -> &[RelationEdge] {
        &self.edges
    }
    pub fn node(&self, id: &str) -> Option<&RelationNode> {
        self.by_node.get(id).map(|&i| &self.nodes[i])
    }
    pub fn edge(&self, id: &str) -> Option<&RelationEdge> {
        self.by_edge.get(id).map(|&i| &self.edges[i])
    }
    pub fn is_forest(&self) -> bool {
        self.forest
    }
    pub fn focus_for_mark(&self, id: &str) -> Option<&RelationFocus> {
        self.mark_owners.get(id)
    }
    pub fn incoming(&self, id: &str) -> Option<Vec<&RelationEdge>> {
        self.by_node
            .get(id)
            .map(|&i| self.incoming[i].iter().map(|&e| &self.edges[e]).collect())
    }
    pub fn outgoing(&self, id: &str) -> Option<Vec<&RelationEdge>> {
        self.by_node
            .get(id)
            .map(|&i| self.outgoing[i].iter().map(|&e| &self.edges[e]).collect())
    }
    fn contains(&self, focus: &RelationFocus) -> bool {
        match focus {
            RelationFocus::Node(id) => self.by_node.contains_key(id),
            RelationFocus::Edge(id) => self.by_edge.contains_key(id),
        }
    }
    fn node_visible(&self, id: &str, hidden: &BTreeSet<String>) -> bool {
        self.nodes[self.by_node[id]].visible && !hidden.contains(id)
    }
    fn edge_visible(&self, e: &RelationEdge, hidden: &BTreeSet<String>) -> bool {
        e.visible && self.node_visible(&e.source, hidden) && self.node_visible(&e.target, hidden)
    }
    fn focus_visible(&self, focus: &RelationFocus, hidden: &BTreeSet<String>) -> bool {
        match focus {
            RelationFocus::Node(id) => self.node_visible(id, hidden),
            RelationFocus::Edge(id) => self.edge_visible(&self.edges[self.by_edge[id]], hidden),
        }
    }
    fn validate_hidden(&self, hidden: &BTreeSet<String>) -> Result<()> {
        ensure(
            hidden.len() <= self.nodes.len()
                && hidden.iter().all(|id| self.by_node.contains_key(id)),
            "unknown hidden relation node",
        )
    }
    fn traverse(
        &self,
        start: usize,
        ancestors: bool,
        hidden: &BTreeSet<String>,
        through_hidden: bool,
    ) -> (BTreeSet<usize>, BTreeSet<usize>) {
        let mut found = BTreeSet::from([start]);
        let mut links = BTreeSet::new();
        let mut queue = VecDeque::from([start]);
        while let Some(i) = queue.pop_front() {
            for &edge in if ancestors {
                &self.incoming[i]
            } else {
                &self.outgoing[i]
            } {
                let e = &self.edges[edge];
                if !through_hidden && !self.edge_visible(e, hidden) {
                    continue;
                }
                links.insert(edge);
                let j = self.by_node[if ancestors { &e.source } else { &e.target }];
                if found.insert(j) {
                    queue.push_back(j);
                }
            }
        }
        found.remove(&start);
        (found, links)
    }
    /// Node/edge IDs in the result are ordered sets, never parsed series names.
    /// Hidden endpoints remove incident edges. ThroughHidden traverses the full
    /// topology but still excludes hidden elements from the returned ink sets.
    pub fn related(
        &self,
        focus: Option<&RelationFocus>,
        mode: &RelationMode,
        hidden: &BTreeSet<String>,
        traversal: RelationTraversal,
    ) -> Result<RelationSet> {
        self.validate_hidden(hidden)?;
        if matches!(mode, RelationMode::Tree(_)) {
            ensure(
                self.forest,
                "tree relations require an acyclic single-parent graph",
            )?;
        }
        let Some(focus) = focus else {
            return Ok(RelationSet::default());
        };
        ensure(self.contains(focus), "unknown relation focus")?;
        if !self.focus_visible(focus, hidden) {
            return Ok(RelationSet::default());
        }
        let mut result = RelationSet {
            focus: Some(focus.clone()),
            highlight: true,
            ..Default::default()
        };
        match (mode, focus) {
            (RelationMode::Tree(flags), RelationFocus::Node(id)) => {
                result.nodes.insert(id.clone());
                let i = self.by_node[id];
                for (ancestors, want_nodes, want_edges) in [
                    (true, flags.ancestor_nodes, flags.ancestor_links),
                    (false, flags.descendant_nodes, flags.descendant_links),
                ] {
                    if !want_nodes && !want_edges {
                        continue;
                    }
                    let (nodes, edges) = self.traverse(
                        i,
                        ancestors,
                        hidden,
                        traversal == RelationTraversal::ThroughHidden,
                    );
                    if want_nodes {
                        result.nodes.extend(
                            nodes
                                .into_iter()
                                .map(|i| self.nodes[i].id.clone())
                                .filter(|id| self.node_visible(id, hidden)),
                        );
                    }
                    if want_edges {
                        result.edges.extend(
                            edges
                                .into_iter()
                                .map(|i| &self.edges[i])
                                .filter(|e| self.edge_visible(e, hidden))
                                .map(|e| e.id.clone()),
                        );
                    }
                }
            }
            (RelationMode::Tree(_), RelationFocus::Edge(_)) => {
                return Err(ChartError("tree relation focus must be a node".into()));
            }
            (_, RelationFocus::Node(id)) => {
                result.nodes.insert(id.clone());
                let i = self.by_node[id];
                for &e in self.incoming[i].iter().chain(&self.outgoing[i]) {
                    let e = &self.edges[e];
                    if !self.edge_visible(e, hidden) {
                        continue;
                    }
                    result.edges.insert(e.id.clone());
                    if !matches!(mode, RelationMode::Chord) {
                        result.nodes.insert(e.source.clone());
                        result.nodes.insert(e.target.clone());
                    }
                }
                if matches!(mode, RelationMode::Chord) && result.edges.is_empty() {
                    result.highlight = false;
                }
            }
            (_, RelationFocus::Edge(id)) => {
                let edge = &self.edges[self.by_edge[id]];
                result
                    .nodes
                    .extend([edge.source.clone(), edge.target.clone()]);
                if matches!(mode, RelationMode::Sankey) {
                    result.edges.extend(
                        self.edges
                            .iter()
                            .filter(|e| {
                                e.source == edge.source
                                    && e.target == edge.target
                                    && self.edge_visible(e, hidden)
                            })
                            .map(|e| e.id.clone()),
                    );
                } else {
                    result.edges.insert(id.clone());
                }
            }
        }
        Ok(result)
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RelationTraversal {
    #[default]
    Visible,
    ThroughHidden,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRelationOptions {
    pub ancestor_nodes: bool,
    pub descendant_nodes: bool,
    pub ancestor_links: bool,
    pub descendant_links: bool,
}
impl Default for TreeRelationOptions {
    fn default() -> Self {
        Self {
            ancestor_nodes: true,
            descendant_nodes: false,
            ancestor_links: true,
            descendant_links: false,
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum RelationMode {
    #[default]
    Neighbors,
    Tree(TreeRelationOptions),
    Chord,
    Sankey,
}
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelationSet {
    pub focus: Option<RelationFocus>,
    pub nodes: BTreeSet<String>,
    pub edges: BTreeSet<String>,
    pub highlight: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct RelationPaint {
    /// Multiplies authored opacity unless absolute_opacity is supplied.
    pub opacity: f64,
    pub absolute_opacity: Option<f64>,
    pub label_opacity: f64,
    /// Overrides existing filled ink, including gradients. No fill is created
    /// on stroke-only marks; stroke overrides likewise require a source stroke.
    pub fill: Option<Color>,
    pub stroke: Option<Color>,
    pub stroke_width: Option<f64>,
}
impl Default for RelationPaint {
    fn default() -> Self {
        Self {
            opacity: 1.,
            absolute_opacity: None,
            label_opacity: 1.,
            fill: None,
            stroke: None,
            stroke_width: None,
        }
    }
}
impl RelationPaint {
    fn validate(&self) -> Result<()> {
        ensure(
            [self.opacity, self.label_opacity]
                .into_iter()
                .all(|v| v.is_finite() && (0. ..=1.).contains(&v))
                && self
                    .absolute_opacity
                    .is_none_or(|v| v.is_finite() && (0. ..=1.).contains(&v))
                && self
                    .stroke_width
                    .is_none_or(|v| v.is_finite() && (0. ..=10_000.).contains(&v)),
            "invalid relation highlight paint",
        )
    }
    fn apply(&self, mark: &mut Mark) {
        mark.opacity = self.absolute_opacity.unwrap_or(mark.opacity * self.opacity);
        if let Some(color) = self.fill.filter(|_| mark.has_fill()) {
            mark.fill = Some(color);
            mark.fill_paint = None;
        }
        if let Some(stroke) = &mut mark.stroke {
            if let Some(color) = self.stroke {
                stroke.color = color
            }
            if let Some(width) = self.stroke_width {
                stroke.width = width
            }
        }
        if self.stroke_width == Some(0.) {
            mark.stroke = None;
            mark.stroke_dash = None;
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RelationOptions {
    pub mode: RelationMode,
    pub traversal: RelationTraversal,
    pub active_node: RelationPaint,
    pub inactive_node: RelationPaint,
    pub active_edge: RelationPaint,
    pub inactive_edge: RelationPaint,
}
impl Default for RelationOptions {
    fn default() -> Self {
        Self {
            mode: Default::default(),
            traversal: Default::default(),
            active_node: Default::default(),
            active_edge: Default::default(),
            inactive_node: RelationPaint {
                opacity: 0.2,
                label_opacity: 0.2,
                ..Default::default()
            },
            inactive_edge: RelationPaint {
                opacity: 0.1,
                label_opacity: 0.1,
                ..Default::default()
            },
        }
    }
}
impl RelationOptions {
    /// Nivo tree defaults: ancestors are active; descendants are opt-in.
    pub fn tree() -> Self {
        Self {
            mode: RelationMode::Tree(TreeRelationOptions::default()),
            ..Default::default()
        }
    }
    pub fn chord() -> Self {
        Self {
            mode: RelationMode::Chord,
            ..Default::default()
        }
    }
    pub fn sankey() -> Self {
        Self {
            mode: RelationMode::Sankey,
            ..Default::default()
        }
    }
    fn validate(&self) -> Result<()> {
        for p in [
            &self.active_node,
            &self.inactive_node,
            &self.active_edge,
            &self.inactive_edge,
        ] {
            p.validate()?
        }
        Ok(())
    }
}

/// Owns an immutable authored snapshot. Read-only scene access prevents style
/// accumulation or silently discarding caller edits on subsequent focus changes.
/// Construct a new snapshot with custom layers before binding, or use replace.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationLayout {
    source: Scene,
    scene: Scene,
    graph: RelationGraph,
    options: RelationOptions,
    hidden: BTreeSet<String>,
    focus: Option<RelationFocus>,
    selection: RelationSet,
}
impl RelationLayout {
    pub fn new(scene: Scene, graph: RelationGraph, options: RelationOptions) -> Result<Self> {
        scene.validate()?;
        options.validate()?;
        let ids = scene
            .marks
            .iter()
            .map(|m| m.id.as_str())
            .collect::<BTreeSet<_>>();
        ensure(
            graph.mark_owners.keys().all(|id| ids.contains(id.as_str()))
                && graph.label_owners.keys().all(|&i| i < scene.labels.len()),
            "relation binding refers to missing Scene ink",
        )?;
        let mut result = Self {
            source: scene.clone(),
            scene,
            graph,
            options,
            hidden: Default::default(),
            focus: None,
            selection: Default::default(),
        };
        let (scene, selection) = result.compose(None, &result.hidden, &result.options)?;
        result.scene = scene;
        result.selection = selection;
        Ok(result)
    }
    pub fn scene(&self) -> &Scene {
        &self.scene
    }
    pub fn authored_scene(&self) -> &Scene {
        &self.source
    }
    pub fn into_scene(self) -> Scene {
        self.scene
    }
    pub fn graph(&self) -> &RelationGraph {
        &self.graph
    }
    pub fn options(&self) -> &RelationOptions {
        &self.options
    }
    pub fn hidden(&self) -> &BTreeSet<String> {
        &self.hidden
    }
    pub fn focus(&self) -> Option<&RelationFocus> {
        self.focus.as_ref()
    }
    pub fn selection(&self) -> &RelationSet {
        &self.selection
    }
    pub fn set_focus(&mut self, focus: Option<RelationFocus>) -> Result<()> {
        let (scene, selection) = self.compose(focus.as_ref(), &self.hidden, &self.options)?;
        self.scene = scene;
        self.focus = selection.focus.clone();
        self.selection = selection;
        Ok(())
    }
    pub fn with_focus(mut self, focus: Option<RelationFocus>) -> Result<Self> {
        self.set_focus(focus)?;
        Ok(self)
    }
    pub fn set_hidden(&mut self, hidden: BTreeSet<String>) -> Result<()> {
        let (scene, selection) = self.compose(self.focus.as_ref(), &hidden, &self.options)?;
        self.scene = scene;
        self.hidden = hidden;
        self.focus = selection.focus.clone();
        self.selection = selection;
        Ok(())
    }
    pub fn set_options(&mut self, options: RelationOptions) -> Result<()> {
        options.validate()?;
        let (scene, selection) = self.compose(self.focus.as_ref(), &self.hidden, &options)?;
        self.scene = scene;
        self.options = options;
        self.focus = selection.focus.clone();
        self.selection = selection;
        Ok(())
    }
    /// Data replacement retains only still-existing hidden IDs and focus, then
    /// recomputes topology/paint atomically. A removed focus clears selection.
    pub fn replace(&mut self, mut next: Self) -> Result<()> {
        let hidden = self
            .hidden
            .iter()
            .filter(|id| next.graph.by_node.contains_key(*id))
            .cloned()
            .collect();
        let focus = self
            .focus
            .as_ref()
            .filter(|f| next.graph.contains(f))
            .cloned();
        let (scene, selection) = next.compose(focus.as_ref(), &hidden, &self.options)?;
        next.scene = scene;
        next.options = self.options.clone();
        next.hidden = hidden;
        next.focus = selection.focus.clone();
        next.selection = selection;
        *self = next;
        Ok(())
    }
    fn paint_for<'a>(
        focus: &RelationFocus,
        selection: &RelationSet,
        options: &'a RelationOptions,
    ) -> &'a RelationPaint {
        match focus {
            RelationFocus::Node(id) => {
                if selection.nodes.contains(id) {
                    &options.active_node
                } else {
                    &options.inactive_node
                }
            }
            RelationFocus::Edge(id) => {
                if selection.edges.contains(id) {
                    &options.active_edge
                } else {
                    &options.inactive_edge
                }
            }
        }
    }
    fn compose(
        &self,
        focus: Option<&RelationFocus>,
        hidden: &BTreeSet<String>,
        options: &RelationOptions,
    ) -> Result<(Scene, RelationSet)> {
        options.validate()?;
        let selection = self
            .graph
            .related(focus, &options.mode, hidden, options.traversal)?;
        let mut scene = self.source.clone();
        let mut mark_slots = vec![None; scene.marks.len()];
        let mut label_slots = vec![None; scene.labels.len()];
        scene.marks = scene
            .marks
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut m)| {
                if let Some(owner) = self.graph.mark_owners.get(&m.id) {
                    if !self.graph.focus_visible(owner, hidden) {
                        return None;
                    }
                    if selection.highlight {
                        Self::paint_for(owner, &selection, options).apply(&mut m);
                    }
                }
                Some((i, m))
            })
            .enumerate()
            .map(|(new, (old, m))| {
                mark_slots[old] = Some(new);
                m
            })
            .collect();
        scene.labels = scene
            .labels
            .into_iter()
            .enumerate()
            .filter_map(|(i, mut label)| {
                if let Some(owner) = self.graph.label_owners.get(&i) {
                    if !self.graph.focus_visible(owner, hidden) {
                        return None;
                    }
                    if selection.highlight {
                        let p = Self::paint_for(owner, &selection, options);
                        label.color.3 = (f64::from(label.color.3) * p.label_opacity).round() as u8;
                    }
                }
                Some((i, label))
            })
            .enumerate()
            .map(|(new, (old, label))| {
                label_slots[old] = Some(new);
                label
            })
            .collect();
        scene.typography.remap_labels(
            &label_slots
                .iter()
                .enumerate()
                .filter_map(|(old, new)| new.map(|new| (old, new)))
                .collect(),
        );
        scene.label_series = scene
            .label_series
            .into_iter()
            .filter_map(|(old, owner)| label_slots[old].map(|new| (new, owner)))
            .collect();
        scene.label_clips = scene
            .label_clips
            .into_iter()
            .filter_map(|(old, clip)| label_slots[old].map(|new| (new, clip)))
            .collect();
        scene.draw_order = scene
            .draw_order
            .into_iter()
            .filter_map(|item| match item {
                DrawItem::Mark(i) => mark_slots[i].map(DrawItem::Mark),
                DrawItem::Label(i) => label_slots[i].map(DrawItem::Label),
            })
            .collect();
        scene.validate()?;
        Ok((scene, selection))
    }
}
