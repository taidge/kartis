//! Borrowed Tree function properties. Node size is a diameter; projection keeps
//! the existing static inset and is completed before any function is called.
use super::*;

/// Original datum, topology and projected position, before size/color resolve.
/// `value` is the aggregate weight; `datum.value` is the original source weight.
/// No resolved size/color or active-state fields are supplied at this stage.
#[derive(Clone, Copy, Debug)]
pub struct TreeNodeFunctionContext<'a> {
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub parent_index: Option<usize>,
    pub children: &'a [usize],
    pub depth: usize,
    pub height: usize,
    pub value: f64,
    pub position: [f64; 2],
    pub plot: ClipRect,
}

/// A computed edge with both fully resolved nodes, including zero-size nodes.
/// Disabled link painting does not suppress these computed-link functions.
#[derive(Clone, Copy, Debug)]
pub struct TreeLinkFunctionContext<'a> {
    pub source: &'a [HierarchyDatum],
    pub link: &'a TreeLink,
    pub source_node: &'a TreeNode,
    pub target_node: &'a TreeNode,
    pub plot: ClipRect,
    /// None for the original five-function entry; activity builds provide their
    /// complete resolved node/link classification without changing TreeNode.
    pub activity: Option<&'a TreeActivityState>,
}

/// Fully resolved node for an eligible Scene label. The function changes only
/// the painted label text, not the original datum or canonical Mark label.
#[derive(Clone, Copy, Debug)]
pub struct TreeLabelFunctionContext<'a> {
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub node: &'a TreeNode,
    pub plot: ClipRect,
    pub activity: Option<&'a TreeActivityState>,
}

pub type TreeNodeSizeFactory<'a> = dyn FnMut(&TreeNodeFunctionContext<'_>) -> Result<f64> + 'a;
pub type TreeNodeColorFactory<'a> = dyn FnMut(&TreeNodeFunctionContext<'_>) -> Result<Color> + 'a;
pub type TreeLinkThicknessFactory<'a> = dyn FnMut(&TreeLinkFunctionContext<'_>) -> Result<f64> + 'a;
pub type TreeLinkColorFactory<'a> = dyn FnMut(&TreeLinkFunctionContext<'_>) -> Result<Color> + 'a;
pub type TreeLabelFactory<'a> = dyn FnMut(&TreeLabelFunctionContext<'_>) -> Result<String> + 'a;

/// Five synchronous functions borrowed for one build and never retained.
/// Every source/options/static Scene check precedes the first invocation.
/// Breadth-first nodes resolve size then color, then all links resolve thickness
/// then color, then eligible labels resolve text. Active/inactive modifiers and
/// Nivo's fixed-depth color inheritance are not implemented by this bundle.
#[derive(Default)]
pub struct TreeFactories<'a> {
    pub node_size: Option<&'a mut TreeNodeSizeFactory<'a>>,
    pub node_color: Option<&'a mut TreeNodeColorFactory<'a>>,
    pub link_thickness: Option<&'a mut TreeLinkThicknessFactory<'a>>,
    pub link_color: Option<&'a mut TreeLinkColorFactory<'a>>,
    pub label: Option<&'a mut TreeLabelFactory<'a>>,
}
impl TreeFactories<'_> {
    pub(super) fn is_empty(&self) -> bool {
        self.node_size.is_none()
            && self.node_color.is_none()
            && self.link_thickness.is_none()
            && self.link_color.is_none()
            && self.label.is_none()
    }
}

/// Project using the existing static styles, then resolve the five functions
/// before emitting the accepted Scene/captured Widget metadata. Dynamic node
/// diameter does not recompute the static projection inset. Explicit node color
/// overrides Palette/Fixed/None fill. Empty label strings keep their Label slot.
/// Errors return no layout; external callback effects cannot be rolled back.
pub fn tree_layout_with_factories(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
    factories: &mut TreeFactories<'_>,
) -> Result<TreeLayout> {
    layout_with_factories(c, data, o, factories)
}

/// Opt into authoritative labels for every computed node, including labels
/// omitted by the requested Scene gates. The label function runs once per BFS
/// node, while Scene painting still respects its original label visibility.
/// Reading the resulting pool never repeats the function.
pub fn tree_canvas_layout_with_factories(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
    factories: &mut TreeFactories<'_>,
) -> Result<TreeLayout> {
    build_tree(c, data, o, factories, None, TreeLabelMode::AllComputed).map(|(layout, _)| layout)
}
