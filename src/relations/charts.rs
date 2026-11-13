use super::*;

struct Bindings {
    marks: BTreeMap<String, usize>,
    labels: BTreeMap<String, Vec<usize>>,
}
impl Bindings {
    fn new(scene: &Scene) -> Self {
        let mut labels = BTreeMap::<String, Vec<usize>>::new();
        for (&i, owner) in &scene.label_series {
            labels.entry(owner.clone()).or_default().push(i);
        }
        Self {
            marks: scene
                .marks
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id.clone(), i))
                .collect(),
            labels,
        }
    }
    fn labels(&self, id: &str) -> Vec<usize> {
        self.labels.get(id).cloned().unwrap_or_default()
    }
    fn attached(&self, id: &str) -> Vec<String> {
        self.marks
            .contains_key(id)
            .then(|| id.to_owned())
            .into_iter()
            .collect()
    }
}

/// Reuses the ordinary styled tree solver. Tree edges use the child's raw ID,
/// consistent with one parent per global node identity. Select `Tree(flags)`
/// for Nivo's independent ancestor/descendant node/link predicates.
pub fn tree_relation_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    tree: &TreeOptions,
    options: &RelationOptions,
) -> Result<RelationLayout> {
    options.validate()?;
    ensure(
        data.len() <= MAX_RELATION_NODES,
        "tree relation source budget exceeded",
    )?;
    let scene = crate::tree_with_options(c, data, tree)?;
    let binding = Bindings::new(&scene);
    let mut nodes = Vec::with_capacity(data.len());
    let mut edges = vec![];
    for datum in data {
        let mut n = RelationNode::new(&datum.id);
        n.mark_ids = binding.attached(&format!("node:{}", datum.id));
        n.label_indices = binding.labels(&datum.id);
        nodes.push(n);
        if let Some(parent) = &datum.parent {
            let mut e = RelationEdge::new(&datum.id, parent, &datum.id);
            e.mark_ids = binding.attached(&format!("link:{}", datum.id));
            edges.push(e);
        }
    }
    RelationLayout::new(scene, RelationGraph::new(nodes, edges)?, options.clone())
}

/// Stable explicit edge IDs are supplied in `simulation.edges()` order. The
/// legacy simulation has positional link IDs, so this new adapter deliberately
/// replaces only those IDs; raw force-node IDs still work with node dragging.
pub fn network_relation_layout(
    c: &ChartConfig,
    simulation: &ForceSimulation,
    edge_ids: &[String],
    options: &RelationOptions,
) -> Result<RelationLayout> {
    options.validate()?;
    ensure(
        simulation.nodes().len() <= MAX_RELATION_NODES
            && edge_ids.len() == simulation.edges().len()
            && edge_ids.len() <= MAX_RELATION_EDGES
            && edge_ids.iter().all(|s| key(s, 512)),
        "network relation edge IDs must align",
    )?;
    let mut scene = simulation.scene(c)?;
    let binding = Bindings::new(&scene);
    let mut nodes = vec![];
    let mut edges = vec![];
    for node in simulation.nodes() {
        let mut n = RelationNode::new(&node.id);
        n.mark_ids = binding.attached(&format!("force-node:{}", node.id));
        n.label_indices = binding.labels(&node.id);
        nodes.push(n);
    }
    for (i, (edge, id)) in simulation.edges().iter().zip(edge_ids).enumerate() {
        let index = *binding
            .marks
            .get(&format!("force-link:{i}"))
            .ok_or_else(|| ChartError("network relation source mark missing".into()))?;
        let mark = &mut scene.marks[index];
        mark.id = format!("relation-network-link:{}:{}", id.len(), id);
        // Force's legacy links are decorative. The relation adapter exposes
        // edges as real pointer/keyboard data; ownership stays in the graph,
        // while the source node supplies a stable visible series identity.
        mark.series = simulation.nodes()[edge.source].id.clone();
        let mut e = RelationEdge::new(
            id,
            &simulation.nodes()[edge.source].id,
            &simulation.nodes()[edge.target].id,
        );
        e.mark_ids = vec![mark.id.clone()];
        edges.push(e);
    }
    RelationLayout::new(scene, RelationGraph::new(nodes, edges)?, options.clone())
}
impl ForceSimulation {
    pub fn scene_with_relations(
        &self,
        c: &ChartConfig,
        edge_ids: &[String],
        options: &RelationOptions,
    ) -> Result<RelationLayout> {
        network_relation_layout(c, self, edge_ids, options)
    }
}

impl SankeyStyledLayout {
    /// Both ordinary and circular styled layouts expose the same explicit IDs.
    /// Zero-width or source-hidden links remain in topology without visible ink.
    pub fn relation_layout(&self, options: &RelationOptions) -> Result<RelationLayout> {
        options.validate()?;
        ensure(
            self.nodes.len() <= MAX_RELATION_NODES
                && self.links.len() <= MAX_RELATION_EDGES
                && self.nodes.iter().all(|n| key(&n.id, 512))
                && self
                    .links
                    .iter()
                    .all(|e| key(&e.id, 1024) && key(&e.source, 512) && key(&e.target, 512)),
            "Sankey relation source budget or identity invalid",
        )?;
        self.scene.validate()?;
        let binding = Bindings::new(&self.scene);
        let nodes = self
            .nodes
            .iter()
            .map(|node| {
                let mut n = RelationNode::new(&node.id);
                n.visible = node.visible;
                n.mark_ids = binding.attached(&node.mark_id);
                n.label_indices = binding.labels(&node.id);
                n
            })
            .collect();
        let edges = self
            .links
            .iter()
            .map(|link| {
                let mut e = RelationEdge::new(&link.id, &link.source, &link.target);
                e.visible = link.visible;
                e.mark_ids = binding.attached(&link.mark_id);
                e
            })
            .collect();
        RelationLayout::new(
            self.scene.clone(),
            RelationGraph::new(nodes, edges)?,
            options.clone(),
        )
    }
}

/// Identity of an undirected raw chord pair. Length prefixes prevent delimiter
/// collisions; endpoint sorting preserves identity under input permutation.
pub fn chord_relation_edge_id(a: &str, b: &str) -> String {
    let (a, b) = if a <= b { (a, b) } else { (b, a) };
    format!("{}:{}:{}:{}", a.len(), a, b.len(), b)
}
/// Uses the existing undirected chord geometry. Group and ribbon Scene IDs are
/// upgraded to raw identities by the producer, never inferred from series.
pub fn chord_relation_layout(
    c: &ChartConfig,
    ids: &[String],
    matrix: &[Vec<f64>],
    pad_angle: f64,
    options: &RelationOptions,
) -> Result<RelationLayout> {
    options.validate()?;
    let mut scene = crate::chord(c, ids, matrix, pad_angle)?;
    let binding = Bindings::new(&scene);
    let mut nodes = vec![];
    let mut edges = vec![];
    for (i, id) in ids.iter().enumerate() {
        let mut node = RelationNode::new(id);
        node.label_indices = binding.labels(id);
        if let Some(&index) = binding.marks.get(&format!("group:{i}")) {
            let mark = &mut scene.marks[index];
            mark.id = format!("relation-chord-node:{}:{}", id.len(), id);
            node.mark_ids.push(mark.id.clone());
        }
        nodes.push(node);
    }
    for i in 0..ids.len() {
        for j in i..ids.len() {
            if matrix[i][j] == 0. && matrix[j][i] == 0. {
                continue;
            }
            let id = chord_relation_edge_id(&ids[i], &ids[j]);
            let mut edge = RelationEdge::new(&id, &ids[i], &ids[j]);
            if let Some(&index) = binding.marks.get(&format!("ribbon:{i}:{j}")) {
                let mark = &mut scene.marks[index];
                mark.id = format!("relation-chord-link:{id}");
                edge.mark_ids.push(mark.id.clone());
            }
            edges.push(edge);
        }
    }
    RelationLayout::new(scene, RelationGraph::new(nodes, edges)?, options.clone())
}
