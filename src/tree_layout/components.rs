//! Real Tree label components alongside existing node/link roles. Component
//! keys retain identity independently of a label's current Scene ordinal.
use super::*;

/// Original source, complete computed pools and the actual current owned Label.
/// Text, font, color and clipping describe the label, independently of its node's
/// raw label and paint. Empty/transparent labels retain their component role.
#[derive(Clone, Debug)]
pub struct TreeLabelWidgetData {
    pub source: Arc<Vec<HierarchyDatum>>,
    pub nodes: Arc<Vec<TreeNode>>,
    pub links: Arc<Vec<TreeLink>>,
    pub labels: Arc<TreeWidgetLabels>,
    pub source_index: usize,
    pub node: TreeNode,
    pub label_index: usize,
    pub label: Label,
    pub font: ResolvedFont,
    pub whitespace: LabelWhitespace,
    pub clip: Option<ClipPath>,
    pub plot: ClipRect,
}
impl PartialEq for TreeLabelWidgetData {
    fn eq(&self, other: &Self) -> bool {
        shared_sources(&self.source, &other.source)
            && shared_nodes(&self.nodes, &other.nodes)
            && self.links == other.links
            && self.labels == other.labels
            && self.source_index == other.source_index
            && node_equal(&self.node, &other.node)
            && self.label_index == other.label_index
            && self.label == other.label
            && self.font == other.font
            && self.whitespace == other.whitespace
            && self.clip == other.clip
            && self.plot == other.plot
    }
}

/// An additive mixed component packet; the older TreeWidgetData enum is unchanged.
#[derive(Clone, Debug, PartialEq)]
pub enum TreeComponentData {
    Node(TreeNodeWidgetData),
    Link(TreeLinkWidgetData),
    Label(TreeLabelWidgetData),
}

#[derive(Clone, Copy, Debug)]
pub struct TreeLabelWidgetContext<'a> {
    pub layout: &'a TreeLayout,
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub node: &'a TreeNode,
    pub label_index: usize,
    pub label: &'a Label,
    pub font: &'a ResolvedFont,
    pub whitespace: LabelWhitespace,
    pub clip: Option<&'a ClipPath>,
    pub plot: ClipRect,
}

#[derive(Clone, Copy, Debug)]
pub enum TreeComponentContext<'a> {
    Node(TreeNodeWidgetContext<'a>),
    Link(TreeLinkWidgetContext<'a>),
    Label(TreeLabelWidgetContext<'a>),
}

struct LabelRole {
    source_index: usize,
    key: String,
    font: ResolvedFont,
}

fn label_roles(layout: &TreeLayout) -> Result<BTreeMap<usize, LabelRole>> {
    let sources: BTreeMap<_, _> = layout
        .source()
        .iter()
        .enumerate()
        .map(|(index, datum)| (datum.id.as_str(), index))
        .collect();
    let mut owners = BTreeSet::new();
    let mut roles = BTreeMap::new();
    for index in 0..layout.scene.labels.len() {
        let font = layout.scene.resolved_label_font(index);
        font.validate()?;
        let Some(owner) = layout.scene.label_series.get(&index) else {
            // An unowned annotation has no original Tree datum to borrow.
            continue;
        };
        let source_index = *sources
            .get(owner.as_str())
            .ok_or_else(|| ChartError("Tree label component owner is not a source node".into()))?;
        ensure(
            owners.insert(source_index),
            "Tree label components require one current label per source owner",
        )?;
        let key = format!("tree-label:{owner}");
        ensure(
            key.len() <= MAX_LABEL_BYTES,
            "Tree label component key exceeds the text budget",
        )?;
        roles.insert(
            index,
            LabelRole {
                source_index,
                key,
                font,
            },
        );
    }
    Ok(roles)
}

struct Pools {
    source: Arc<Vec<HierarchyDatum>>,
    nodes: Arc<Vec<TreeNode>>,
    links: Arc<Vec<TreeLink>>,
    labels: Arc<TreeWidgetLabels>,
}
impl Pools {
    fn new(layout: &TreeLayout) -> Self {
        Self {
            source: layout.captured.source.clone(),
            nodes: Arc::new(layout.nodes.clone()),
            links: Arc::new(layout.links.clone()),
            labels: Arc::new(TreeWidgetLabels {
                labels: layout.scene.labels.clone(),
                clips: layout.scene.label_clips.clone(),
                series: layout.scene.label_series.clone(),
            }),
        }
    }
    fn data(&self, context: TreeComponentContext<'_>) -> TreeComponentData {
        match context {
            TreeComponentContext::Node(c) => TreeComponentData::Node(TreeNodeWidgetData {
                source: self.source.clone(),
                nodes: self.nodes.clone(),
                links: self.links.clone(),
                labels: self.labels.clone(),
                source_index: c.source_index,
                node: c.node.clone(),
                plot: c.layout.plot,
            }),
            TreeComponentContext::Link(c) => TreeComponentData::Link(TreeLinkWidgetData {
                source: self.source.clone(),
                nodes: self.nodes.clone(),
                links: self.links.clone(),
                labels: self.labels.clone(),
                link_index: c.link_index,
                link: c.link.clone(),
                plot: c.layout.plot,
            }),
            TreeComponentContext::Label(c) => TreeComponentData::Label(TreeLabelWidgetData {
                source: self.source.clone(),
                nodes: self.nodes.clone(),
                links: self.links.clone(),
                labels: self.labels.clone(),
                source_index: c.source_index,
                node: c.node.clone(),
                label_index: c.label_index,
                label: c.label.clone(),
                font: c.font.clone(),
                whitespace: c.whitespace,
                clip: c.clip.cloned(),
                plot: c.plot,
            }),
        }
    }
}

/// Select current node/link/owned-label components in actual Scene paint order.
///
/// Complete captured Tree and current Scene/label ownership/fonts/clips are
/// admitted before any placement callback. Duplicate label owners are rejected;
/// unowned annotations are skipped. Actual empty text, zero-radius/no-fill nodes
/// and transparent label colors do not remove roles. Source and computed pools
/// are shared once across every selected role, with one 128-slot budget.
///
/// Node/link AtDatum keeps the constructor center/midpoint. Label AtDatum stays
/// a real Label target and inherits its current anchor/rotation in the host;
/// Rect explicitly chooses an axis-aligned component placement instead. No fake
/// Mark, business site or node opacity inheritance is introduced. Borrowed
/// callbacks are never retained; errors return no partial packet, while external
/// callback effects cannot be rolled back.
pub fn tree_components(
    layout: &TreeLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<SceneWidgetsData<TreeComponentData>> {
    preflight(layout)?;
    let labels = label_roles(layout)?;
    let nodes: BTreeMap<_, _> = layout
        .nodes
        .iter()
        .filter_map(|node| node.mark_id.as_deref().map(|id| (id, node)))
        .collect();
    let links: BTreeMap<_, _> = layout
        .links
        .iter()
        .filter_map(|link| link.mark_id.as_deref().map(|id| (id, link)))
        .collect();
    let mut packet = SceneWidgetsData {
        layout: SceneWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = None;
    for item in layout.scene.draw_items() {
        let (key, target, context, origin) = match item {
            DrawItem::Mark(index) => {
                let mark = &layout.scene.marks[index];
                if let Some(node) = nodes.get(mark.id.as_str()) {
                    (
                        mark.id.clone(),
                        SceneWidgetTarget::Mark(mark.id.clone()),
                        TreeComponentContext::Node(TreeNodeWidgetContext {
                            layout,
                            source: layout.source(),
                            datum: &layout.source()[node.source_index],
                            source_index: node.source_index,
                            node,
                            mark,
                        }),
                        Some(node.position),
                    )
                } else if let Some(link) = links.get(mark.id.as_str()) {
                    (
                        mark.id.clone(),
                        SceneWidgetTarget::Mark(mark.id.clone()),
                        TreeComponentContext::Link(TreeLinkWidgetContext {
                            layout,
                            source: layout.source(),
                            source_datum: &layout.source()[link.source_index],
                            target_datum: &layout.source()[link.target_index],
                            link_index: link.link_index,
                            link,
                            mark,
                        }),
                        Some(link.position),
                    )
                } else {
                    continue;
                }
            }
            DrawItem::Label(index) => {
                let Some(role) = labels.get(&index) else {
                    continue;
                };
                (
                    role.key.clone(),
                    SceneWidgetTarget::Label(index),
                    TreeComponentContext::Label(TreeLabelWidgetContext {
                        layout,
                        source: layout.source(),
                        datum: &layout.source()[role.source_index],
                        source_index: role.source_index,
                        node: &layout.nodes[role.source_index],
                        label_index: index,
                        label: &layout.scene.labels[index],
                        font: &role.font,
                        whitespace: layout.scene.typography.label_whitespace(index),
                        clip: layout.scene.label_clips.get(&index),
                        plot: layout.plot,
                    }),
                    None,
                )
            }
        };
        let Some(selected) = placement(&context)? else {
            continue;
        };
        ensure(
            packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
            "Tree component slot budget exceeded",
        )?;
        let placement = origin.map_or(selected, |position| at_origin(selected, position));
        let pools = owned.get_or_insert_with(|| Pools::new(layout));
        packet.data.insert(key.clone(), pools.data(context));
        packet.layout.slots.push(SceneWidgetSlot {
            key,
            target,
            revision,
            placement,
        });
    }
    Ok(packet)
}

pub fn tree_label_widgets(
    layout: &TreeLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeLabelWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<SceneWidgetsData<TreeLabelWidgetData>> {
    let packet = tree_components(layout, revision, |context| match context {
        TreeComponentContext::Label(context) => placement(context),
        _ => Ok(None),
    })?;
    Ok(SceneWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(key, data)| {
                let TreeComponentData::Label(data) = data else {
                    unreachable!("label selector")
                };
                (key, data)
            })
            .collect(),
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeActivityComponentData {
    pub component: TreeComponentData,
    pub activity: Arc<TreeActivityState>,
    pub is_active: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct TreeActivityComponentContext<'a> {
    pub component: TreeComponentContext<'a>,
    pub activity: &'a TreeActivityState,
    pub is_active: Option<bool>,
}

/// A single mixed packet with the existing opaque activity/capture guard.
/// Label activity belongs to its source node; it never creates another datum.
pub fn tree_activity_components(
    layout: &TreeActivityLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<SceneWidgetsData<TreeActivityComponentData>> {
    // Reuse the authoritative activity guard, including the capture identity
    // which prevents grafting a different otherwise-valid public TreeLayout.
    tree_activity_frame(layout)?;
    let active = |context: TreeComponentContext<'_>| match context {
        TreeComponentContext::Node(context) => layout.activity.nodes[context.source_index],
        TreeComponentContext::Link(context) => layout.activity.links[context.link_index],
        TreeComponentContext::Label(context) => layout.activity.nodes[context.source_index],
    };
    let packet = tree_components(&layout.layout, revision, |context| {
        placement(&TreeActivityComponentContext {
            component: *context,
            activity: &layout.activity,
            is_active: active(*context),
        })
    })?;
    Ok(SceneWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(key, component)| {
                let is_active = match &component {
                    TreeComponentData::Node(data) => layout.activity.nodes[data.source_index],
                    TreeComponentData::Link(data) => layout.activity.links[data.link_index],
                    TreeComponentData::Label(data) => layout.activity.nodes[data.source_index],
                };
                (
                    key,
                    TreeActivityComponentData {
                        component,
                        activity: layout.activity.clone(),
                        is_active,
                    },
                )
            })
            .collect(),
    })
}
