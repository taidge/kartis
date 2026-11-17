//! Tree activity and fixed-depth color inheritance. This sidecar keeps the
//! existing Tree node/link/layout and five-function bundle shapes unchanged.
use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TreeActivityOptions {
    /// Original flat source ID, not a prefixed Mark ID. None clears activity.
    pub focus_id: Option<String>,
    pub highlight: TreeRelationOptions,
    /// None is unbounded depth. This typed profile accepts nonnegative integer
    /// depths, rather than arbitrary JavaScript fractional/NaN/negative values.
    pub fix_node_color_at_depth: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeActivityState {
    pub focus_id: Option<String>,
    pub focus_index: Option<usize>,
    pub highlight: TreeRelationOptions,
    pub fix_node_color_at_depth: Option<usize>,
    /// Source-node order. None means there is no current focus; false means
    /// inactive while a current focus exists.
    pub nodes: Vec<Option<bool>>,
    /// Complete computed-link order, including links disabled for painting.
    pub links: Vec<Option<bool>>,
}

/// Computed node after base diameter/color and activity classification, before
/// its selected modifier. `size` retains the base diameter, including its exact
/// function result; `node.radius` is the corresponding Rust half-size.
#[derive(Clone, Copy, Debug)]
pub struct TreeNodeModifierContext<'a> {
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub node: &'a TreeNode,
    pub size: f64,
    pub is_active: bool,
    pub activity: &'a TreeActivityState,
    pub plot: ClipRect,
}

/// Computed link after base thickness/color with final endpoint nodes.
#[derive(Clone, Copy, Debug)]
pub struct TreeLinkModifierContext<'a> {
    pub source: &'a [HierarchyDatum],
    pub link: &'a TreeLink,
    pub source_node: &'a TreeNode,
    pub target_node: &'a TreeNode,
    pub thickness: f64,
    pub color: Color,
    pub is_active: bool,
    pub activity: &'a TreeActivityState,
    pub plot: ClipRect,
}
pub type TreeNodeSizeModifierFactory<'a> =
    dyn FnMut(&TreeNodeModifierContext<'_>) -> Result<f64> + 'a;
pub type TreeLinkThicknessModifierFactory<'a> =
    dyn FnMut(&TreeLinkModifierContext<'_>) -> Result<f64> + 'a;

/// Only the selected active/inactive function runs for each computed role.
/// With no focus none of these functions run. Missing functions keep the base
/// diameter/thickness. The functions are borrowed for one build, never retained.
#[derive(Default)]
pub struct TreeActivityFactories<'a> {
    pub active_node_size: Option<&'a mut TreeNodeSizeModifierFactory<'a>>,
    pub inactive_node_size: Option<&'a mut TreeNodeSizeModifierFactory<'a>>,
    pub active_link_thickness: Option<&'a mut TreeLinkThicknessModifierFactory<'a>>,
    pub inactive_link_thickness: Option<&'a mut TreeLinkThicknessModifierFactory<'a>>,
}

#[derive(Clone, Debug)]
pub struct TreeActivityLayout {
    pub layout: TreeLayout,
    pub activity: Arc<TreeActivityState>,
    accepted_activity: Arc<TreeActivityState>,
    accepted_capture: Arc<CapturedTree>,
}
impl PartialEq for TreeActivityLayout {
    fn eq(&self, other: &Self) -> bool {
        self.layout == other.layout
            && self.activity == other.activity
            && self.accepted_activity == other.accepted_activity
    }
}
impl TreeActivityLayout {
    pub fn node_id_from_mark(&self, mark_id: &str) -> Option<&str> {
        self.layout.captured.nodes.iter().find_map(|node| {
            (node.mark_id.as_deref() == Some(mark_id))
                .then(|| self.layout.source()[node.source_index].id.as_str())
        })
    }
    pub fn mark_id_for_node(&self, source_id: &str) -> Option<&str> {
        self.layout.captured.nodes.iter().find_map(|node| {
            (self.layout.source()[node.source_index].id == source_id)
                .then_some(node.mark_id.as_deref())
                .flatten()
        })
    }
    fn validate_activity(&self) -> Result<()> {
        ensure(
            self.activity == self.accepted_activity
                && Arc::ptr_eq(&self.layout.captured, &self.accepted_capture),
            "Tree activity state or layout differs from its accepted constructor",
        )
    }
}

pub(super) struct ActivityBuild<'a, 'b> {
    pub options: &'a TreeActivityOptions,
    pub factories: &'a mut TreeActivityFactories<'b>,
}
pub(super) fn resolve_activity(
    data: &[HierarchyDatum],
    hierarchy: Option<&Hierarchy>,
    links: &[TreeLink],
    options: &TreeActivityOptions,
) -> Result<Arc<TreeActivityState>> {
    let focus_index = options
        .focus_id
        .as_ref()
        .map(|id| {
            data.iter()
                .position(|d| d.id == *id)
                .ok_or_else(|| ChartError("unknown Tree activity focus source ID".into()))
        })
        .transpose()?;
    let mut nodes = vec![None; data.len()];
    let mut edges = vec![None; links.len()];
    if let Some(focus) = focus_index {
        let hierarchy = hierarchy.unwrap();
        let mut ancestors = vec![false; data.len()];
        let mut parent = hierarchy.parent[focus];
        while let Some(index) = parent {
            ancestors[index] = true;
            parent = hierarchy.parent[index];
        }
        let mut descendants = vec![false; data.len()];
        let mut queue = hierarchy.children[focus].clone();
        let mut cursor = 0;
        while cursor < queue.len() {
            let index = queue[cursor];
            descendants[index] = true;
            queue.extend_from_slice(&hierarchy.children[index]);
            cursor += 1;
        }
        for (i, node) in nodes.iter_mut().enumerate() {
            *node = Some(
                i == focus
                    || options.highlight.ancestor_nodes && ancestors[i]
                    || options.highlight.descendant_nodes && descendants[i],
            );
        }
        for link in links {
            edges[link.link_index] = Some(
                options.highlight.ancestor_links
                    && (link.target_index == focus || ancestors[link.target_index])
                    || options.highlight.descendant_links
                        && (link.source_index == focus || descendants[link.source_index]),
            );
        }
    }
    Ok(Arc::new(TreeActivityState {
        focus_id: options.focus_id.clone(),
        focus_index,
        highlight: options.highlight.clone(),
        fix_node_color_at_depth: options.fix_node_color_at_depth,
        nodes,
        links: edges,
    }))
}

/// Full base-function plus activity build. Projection keeps the existing static
/// inset. Unknown focus fails before callbacks. Fixed-depth inheritance skips
/// deeper base color functions and uses the parent's final color, including
/// None/no-fill. Active modifiers run before children, links and labels consume
/// final nodes. Actual host interaction is provided by the separate live owner.
pub fn tree_layout_with_activity(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    options: &TreeOptions,
    base: &mut TreeFactories<'_>,
    activity: &TreeActivityOptions,
    modifiers: &mut TreeActivityFactories<'_>,
) -> Result<TreeActivityLayout> {
    layout_with_activity_labels(
        c,
        data,
        options,
        base,
        activity,
        modifiers,
        TreeLabelMode::CurrentScene,
    )
}

/// Activity-aware Canvas metadata build. Every computed node receives one
/// label-function call after final size/color/activity resolution, including
/// nodes whose labels are omitted from the Scene. Actual Scene gates and
/// original canonical datum labels remain unchanged.
pub fn tree_canvas_layout_with_activity(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    options: &TreeOptions,
    base: &mut TreeFactories<'_>,
    activity: &TreeActivityOptions,
    modifiers: &mut TreeActivityFactories<'_>,
) -> Result<TreeActivityLayout> {
    layout_with_activity_labels(
        c,
        data,
        options,
        base,
        activity,
        modifiers,
        TreeLabelMode::AllComputed,
    )
}

pub(super) fn layout_with_activity_labels(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    options: &TreeOptions,
    base: &mut TreeFactories<'_>,
    activity: &TreeActivityOptions,
    modifiers: &mut TreeActivityFactories<'_>,
    label_mode: TreeLabelMode,
) -> Result<TreeActivityLayout> {
    let (layout, state) = build_tree(
        c,
        data,
        options,
        base,
        Some(ActivityBuild {
            options: activity,
            factories: modifiers,
        }),
        label_mode,
    )?;
    let activity = state.expect("activity build always resolves a state");
    Ok(TreeActivityLayout {
        accepted_capture: layout.captured.clone(),
        layout,
        accepted_activity: activity.clone(),
        activity,
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeActivityWidgetData {
    pub tree: TreeWidgetData,
    pub activity: Arc<TreeActivityState>,
    pub is_active: Option<bool>,
}
#[derive(Clone, Copy, Debug)]
pub struct TreeActivityWidgetContext<'a> {
    pub tree: TreeWidgetContext<'a>,
    pub activity: &'a TreeActivityState,
    pub is_active: Option<bool>,
}
pub fn tree_activity_frame(layout: &TreeActivityLayout) -> Result<SceneInteraction> {
    layout.validate_activity()?;
    tree_widget_frame(&layout.layout)
}
pub fn tree_activity_widgets(
    layout: &TreeActivityLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeActivityWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<TreeActivityWidgetData>> {
    layout.validate_activity()?;
    let active = |role: TreeWidgetContext<'_>| match role {
        TreeWidgetContext::Node(context) => layout.activity.nodes[context.source_index],
        TreeWidgetContext::Link(context) => layout.activity.links[context.link_index],
    };
    let packet = tree_widgets(&layout.layout, revision, |context| {
        placement(&TreeActivityWidgetContext {
            tree: *context,
            activity: &layout.activity,
            is_active: active(*context),
        })
    })?;
    Ok(PlotWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(id, tree)| {
                let is_active = match &tree {
                    TreeWidgetData::Node(node) => layout.activity.nodes[node.source_index],
                    TreeWidgetData::Link(link) => layout.activity.links[link.link_index],
                };
                (
                    id,
                    TreeActivityWidgetData {
                        tree,
                        activity: layout.activity.clone(),
                        is_active,
                    },
                )
            })
            .collect(),
    })
}
