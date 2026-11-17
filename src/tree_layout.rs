//! Captured Tree source/topology and live node/link roles. The original Tree
//! calculation remains authoritative; original logical centers are not inferred
//! from current shape bounds. Rust flat IDs are globally unique source IDs,
//! unlike Nivo's nested hierarchy/path UIDs; radial orientation is an extension.
use glory::render::GraphicPoint as Point;

use super::*;
use crate::*;
#[path = "tree_layout/factories.rs"]
mod factories;
pub use factories::*;
#[path = "tree_layout/activity.rs"]
mod activity;
pub use activity::*;
#[path = "tree_layout/components.rs"]
mod components;
pub use components::*;

#[derive(Clone, Debug, PartialEq)]
pub struct TreeNode {
    pub source_index: usize,
    pub parent_index: Option<usize>,
    pub children: Vec<usize>,
    pub depth: usize,
    pub height: usize,
    /// Original source weight; internal nodes currently require zero.
    pub raw_value: f64,
    /// Aggregate descendant weight, also used by the compatible Mark scalar.
    pub value: f64,
    pub position: [f64; 2],
    pub radius: f64,
    /// Constructor style, independent of any later legal current Mark paint.
    pub style: TreeNodeStyle,
    pub color: Option<Color>,
    pub mark_id: Option<String>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct TreeLink {
    pub link_index: usize,
    /// Indices into both the complete flat source and computed nodes.
    pub source_index: usize,
    pub target_index: usize,
    pub source_position: [f64; 2],
    pub target_position: [f64; 2],
    pub position: [f64; 2],
    /// Disabled links retain computed metadata without inventing a Mark.
    pub mark_id: Option<String>,
}

/// Final constructor-resolved link appearance, independent of whether the
/// corresponding link paints a Mark. Thickness retains its original zero sign;
/// color retains its alpha independently of the link's separate opacity.
#[derive(Clone, Copy, Debug)]
pub struct TreeComputedLinkStyle {
    pub thickness: f64,
    pub color: Color,
}
impl PartialEq for TreeComputedLinkStyle {
    fn eq(&self, other: &Self) -> bool {
        crate::widget_source_eq::number(self.thickness, other.thickness)
            && self.color == other.color
    }
}

/// A constructor-computed Canvas label, including labels omitted from the
/// current Scene. Entries use breadth-first order and original source indices.
/// Later Scene edits do not rewrite this pool. `eligible` records the requested
/// Scene-label gates, independently of zero size, empty text or transparency.
#[derive(Clone, Debug, PartialEq)]
pub struct TreeComputedCanvasLabel {
    pub source_index: usize,
    pub label: Label,
    pub font: ResolvedFont,
    pub whitespace: LabelWhitespace,
    pub stroke: Option<crate::LabelStroke>,
    pub clip: Option<ClipPath>,
    pub eligible: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TreeLabelMode {
    CurrentScene,
    AllComputed,
    SvgOmitted,
}

#[derive(Clone)]
struct CapturedTree {
    source: Arc<Vec<HierarchyDatum>>,
    options: TreeOptions,
    nodes: Vec<TreeNode>,
    links: Vec<TreeLink>,
    plot: ClipRect,
    size: [f64; 2],
    root: Option<usize>,
    link_styles: Vec<TreeComputedLinkStyle>,
    canvas_labels: Option<Arc<Vec<TreeComputedCanvasLabel>>>,
}
impl std::fmt::Debug for CapturedTree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Preserve the existing public TreeLayout Debug text. This additional
        // cache is separately observable through its accessor, not old Debug/Eq.
        f.debug_struct("CapturedTree")
            .field("source", &self.source)
            .field("options", &self.options)
            .field("nodes", &self.nodes)
            .field("links", &self.links)
            .field("plot", &self.plot)
            .field("size", &self.size)
            .field("root", &self.root)
            .finish()
    }
}
/// Public current Scene with private constructor source/topology/placement.
/// Computed fields must retain their captured meaning. Current Scene paint and
/// legal geometry can be edited; Paths keep the original logical node anchor.
/// Label text, placement, clips and ownership may be updated consistently.
/// Tree includes its root, preserving `tree_with_options` behavior:
/// `TreeOptions.hierarchy.include_root` has no effect for Tree.
#[derive(Clone, Debug)]
pub struct TreeLayout {
    pub scene: Scene,
    pub plot: ClipRect,
    /// Original source order; each source index addresses the same node.
    pub nodes: Vec<TreeNode>,
    /// Breadth-first source edge order, including disabled links.
    pub links: Vec<TreeLink>,
    captured: Arc<CapturedTree>,
}
fn source_equal(a: &[HierarchyDatum], b: &[HierarchyDatum]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.id == b.id
                && a.label == b.label
                && a.parent == b.parent
                && crate::widget_source_eq::number(a.value, b.value)
        })
}
fn node_equal(a: &TreeNode, b: &TreeNode) -> bool {
    a == b && crate::widget_source_eq::number(a.raw_value, b.raw_value)
}
fn nodes_equal(a: &[TreeNode], b: &[TreeNode]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| node_equal(a, b))
}
impl PartialEq for TreeLayout {
    fn eq(&self, other: &Self) -> bool {
        self.scene == other.scene
            && self.plot == other.plot
            && nodes_equal(&self.nodes, &other.nodes)
            && self.links == other.links
            && self.captured.options == other.captured.options
            && source_equal(self.source(), other.source())
            && self.captured.root == other.captured.root
            && nodes_equal(&self.captured.nodes, &other.captured.nodes)
            && self.captured.links == other.captured.links
            && self.captured.plot == other.captured.plot
            && self.captured.size == other.captured.size
    }
}
impl TreeLayout {
    fn capture(
        c: &ChartConfig,
        data: &[HierarchyDatum],
        o: &TreeOptions,
        scene: Scene,
        root: Option<usize>,
        computed: (Vec<TreeNode>, Vec<TreeLink>, Vec<ResolvedLinkStyle>),
        canvas_labels: Option<Arc<Vec<TreeComputedCanvasLabel>>>,
    ) -> Result<Self> {
        let (nodes, links, link_styles) = computed;
        let plot = ClipRect::plot(c);
        let captured = Arc::new(CapturedTree {
            source: Arc::new(data.to_vec()),
            options: o.clone(),
            nodes: nodes.clone(),
            links: links.clone(),
            plot,
            size: [scene.width, scene.height],
            root,
            link_styles: link_styles
                .into_iter()
                .map(|style| TreeComputedLinkStyle {
                    thickness: style.width,
                    color: style.color,
                })
                .collect(),
            canvas_labels,
        });
        Ok(Self {
            scene,
            plot,
            nodes,
            links,
            captured,
        })
    }
    pub fn source(&self) -> &[HierarchyDatum] {
        &self.captured.source
    }
    pub fn options(&self) -> &TreeOptions {
        &self.captured.options
    }
    pub fn root_index(&self) -> Option<usize> {
        self.captured.root
    }
    /// Final constructor styles in complete `links` order, including disabled,
    /// zero-thickness and transparent links. Reading does not replay functions;
    /// later legal edits to the public Scene do not rewrite this computed cache.
    /// The legacy TreeLayout equality deliberately does not compare this cache.
    pub fn computed_link_styles(&self) -> &[TreeComputedLinkStyle] {
        &self.captured.link_styles
    }
    /// Complete constructor-computed labels from an explicit `tree_canvas_*`
    /// build. Old Tree constructors return None; even an empty Canvas build
    /// returns Some(empty). Reading never replays any application function.
    /// Legacy TreeLayout equality and Debug deliberately omit this cache.
    pub fn computed_canvas_labels(&self) -> Option<&[TreeComputedCanvasLabel]> {
        self.captured.canvas_labels.as_deref().map(Vec::as_slice)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TreeWidgetLabels {
    pub labels: Vec<Label>,
    pub clips: BTreeMap<usize, ClipPath>,
    pub series: BTreeMap<usize, String>,
}
#[derive(Clone, Debug)]
pub struct TreeNodeWidgetData {
    pub source: Arc<Vec<HierarchyDatum>>,
    pub nodes: Arc<Vec<TreeNode>>,
    pub links: Arc<Vec<TreeLink>>,
    pub source_index: usize,
    pub node: TreeNode,
    pub plot: ClipRect,
    pub labels: Arc<TreeWidgetLabels>,
}
#[derive(Clone, Debug)]
pub struct TreeLinkWidgetData {
    pub source: Arc<Vec<HierarchyDatum>>,
    pub nodes: Arc<Vec<TreeNode>>,
    pub links: Arc<Vec<TreeLink>>,
    pub link_index: usize,
    pub link: TreeLink,
    pub plot: ClipRect,
    pub labels: Arc<TreeWidgetLabels>,
}
fn shared_sources(a: &Arc<Vec<HierarchyDatum>>, b: &Arc<Vec<HierarchyDatum>>) -> bool {
    Arc::ptr_eq(a, b) || source_equal(a, b)
}
fn shared_nodes(a: &Arc<Vec<TreeNode>>, b: &Arc<Vec<TreeNode>>) -> bool {
    Arc::ptr_eq(a, b) || nodes_equal(a, b)
}
impl PartialEq for TreeNodeWidgetData {
    fn eq(&self, other: &Self) -> bool {
        shared_sources(&self.source, &other.source)
            && shared_nodes(&self.nodes, &other.nodes)
            && self.links == other.links
            && self.source_index == other.source_index
            && node_equal(&self.node, &other.node)
            && self.plot == other.plot
            && self.labels == other.labels
    }
}
impl PartialEq for TreeLinkWidgetData {
    fn eq(&self, other: &Self) -> bool {
        shared_sources(&self.source, &other.source)
            && shared_nodes(&self.nodes, &other.nodes)
            && self.links == other.links
            && self.link_index == other.link_index
            && self.link == other.link
            && self.plot == other.plot
            && self.labels == other.labels
    }
}
#[derive(Clone, Debug, PartialEq)]
pub enum TreeWidgetData {
    Node(TreeNodeWidgetData),
    Link(TreeLinkWidgetData),
}
#[derive(Clone, Copy, Debug)]
pub struct TreeNodeWidgetContext<'a> {
    pub layout: &'a TreeLayout,
    pub source: &'a [HierarchyDatum],
    pub datum: &'a HierarchyDatum,
    pub source_index: usize,
    pub node: &'a TreeNode,
    pub mark: &'a Mark,
}
#[derive(Clone, Copy, Debug)]
pub struct TreeLinkWidgetContext<'a> {
    pub layout: &'a TreeLayout,
    pub source: &'a [HierarchyDatum],
    pub source_datum: &'a HierarchyDatum,
    pub target_datum: &'a HierarchyDatum,
    pub link_index: usize,
    pub link: &'a TreeLink,
    pub mark: &'a Mark,
}
#[derive(Clone, Copy, Debug)]
pub enum TreeWidgetContext<'a> {
    Node(TreeNodeWidgetContext<'a>),
    Link(TreeLinkWidgetContext<'a>),
}

fn preflight(layout: &TreeLayout) -> Result<InteractionBinding> {
    layout.scene.validate()?;
    crate::surface::validate_plot(Some(layout.plot), &layout.scene)?;
    let capture = &layout.captured;
    ensure(
        layout.plot == capture.plot
            && [layout.scene.width, layout.scene.height] == capture.size
            && nodes_equal(&layout.nodes, &capture.nodes)
            && layout.links == capture.links,
        "Tree Widget computed metadata differs from its constructor",
    )?;
    let marks: BTreeMap<_, _> = layout
        .scene
        .marks
        .iter()
        .map(|m| (m.id.as_str(), m))
        .collect();
    let mut expected = BTreeSet::new();
    let mut binding = InteractionBinding {
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    for node in &layout.nodes {
        let datum = &capture.source[node.source_index];
        let Some(id) = &node.mark_id else {
            continue;
        };
        let mark = marks
            .get(id.as_str())
            .ok_or_else(|| ChartError("Tree Widget node Mark is missing".into()))?;
        ensure(
            expected.insert(id.as_str())
                && mark.series == datum.id
                && mark.label == datum.label
                && mark.value == node.value,
            "Tree Widget node Mark identity or source value changed",
        )?;
        // This validates supported current Circle/Rect centers and derives a
        // Path anchor at the captured logical origin, including offset Paths.
        let current = ScatterNodeGeometry {
            geometry: mark.geometry.clone(),
            fill: mark.fill,
            fill_paint: mark.fill_paint.clone(),
            stroke: mark.stroke,
            stroke_dash: mark.stroke_dash.clone(),
            opacity: mark.opacity,
            anchor_commands: None,
        }
        .prepare_at(
            id,
            node.position,
            node.color
                .or(node.style.border.map(|s| s.color))
                .unwrap_or(Color(0, 0, 0, 0)),
            200_000,
        )?;
        if !datum.label.is_empty() {
            if let Some(anchor) = current.anchor {
                binding.anchors.insert(id.clone(), anchor);
            }
            binding.sites.push(InteractionSite {
                mark_id: id.clone(),
                series: datum.id.clone(),
                label: datum.label.clone(),
                value: Some(node.value),
                formatted_value: None,
                color: current.color,
                position: node.position,
            });
        }
    }
    for link in &layout.links {
        let Some(id) = &link.mark_id else {
            continue;
        };
        let mark = marks
            .get(id.as_str())
            .ok_or_else(|| ChartError("Tree Widget link Mark is missing".into()))?;
        ensure(
            expected.insert(id.as_str())
                && mark.series.is_empty()
                && mark.label.is_empty()
                && mark.value == 0.,
            "Tree Widget link Mark identity or decorative role changed",
        )?;
    }
    for mark in &layout.scene.marks {
        ensure(
            expected.contains(mark.id.as_str())
                || !(mark.id.starts_with("node:") || mark.id.starts_with("link:")),
            "Tree Widget Scene has an orphan managed Mark",
        )?;
    }
    let ids: BTreeSet<_> = capture.source.iter().map(|d| d.id.as_str()).collect();
    ensure(
        layout
            .scene
            .label_series
            .values()
            .all(|id| ids.contains(id.as_str())),
        "Tree Widget label owner is not a source node",
    )?;
    binding.validate(&layout.scene)?;
    Ok(binding)
}
/// Current Scene, accurate plot and explicit painted picking with canonical
/// labeled nodes only. Links and empty-label nodes do not invent business
/// controls. No-ink labeled nodes remain logical sites without a paint footprint.
/// Constructor topology and positions are private; current legal paint/Path
/// geometry and labels may differ. A moved Path retains its original anchor.
pub fn tree_widget_frame(layout: &TreeLayout) -> Result<SceneInteraction> {
    let binding = preflight(layout)?;
    let frame = SceneInteraction {
        scene: layout.scene.clone(),
        plot: Some(layout.plot),
        interaction: Some(binding),
    };
    frame.validate()?;
    Ok(frame)
}
fn at_origin(placement: PlotWidgetPlacement, position: [f64; 2]) -> PlotWidgetPlacement {
    match placement {
        PlotWidgetPlacement::AtDatum { offset, measured } => {
            PlotWidgetPlacement::Rect(LayoutRect {
                x: position[0] + offset[0],
                y: position[1] + offset[1],
                width: measured.width,
                height: measured.height,
            })
        }
        other => other,
    }
}
/// One source-ordered link/node packet with a single 128-selected-slot budget.
/// Complete captured metadata, current Marks and anchors are validated before
/// any placement callback. Actual zero-radius, transparent and no-ink nodes
/// remain roles; disabled links do not. Root emission retains the legacy Tree
/// contract. The source/nodes/links/label pools are shared once, never per role.
/// AtDatum uses the original node center or true endpoint midpoint, not custom
/// Path bounds. Errors return no packet; external callback effects do not roll
/// back. Factories are borrowed and never retained. No React props are supplied.
pub fn tree_widgets(
    layout: &TreeLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<TreeWidgetData>> {
    preflight(layout)?;
    let nodes: BTreeMap<_, _> = layout
        .nodes
        .iter()
        .filter_map(|n| n.mark_id.as_deref().map(|id| (id, n)))
        .collect();
    let links: BTreeMap<_, _> = layout
        .links
        .iter()
        .filter_map(|l| l.mark_id.as_deref().map(|id| (id, l)))
        .collect();
    let mut packet = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(layout.plot),
            slots: vec![],
        },
        data: BTreeMap::new(),
    };
    let mut owned = None;
    for mark in layout.scene.paint_marks() {
        let (context, position) = if let Some(node) = nodes.get(mark.id.as_str()) {
            (
                TreeWidgetContext::Node(TreeNodeWidgetContext {
                    layout,
                    source: layout.source(),
                    datum: &layout.source()[node.source_index],
                    source_index: node.source_index,
                    node,
                    mark,
                }),
                node.position,
            )
        } else if let Some(link) = links.get(mark.id.as_str()) {
            (
                TreeWidgetContext::Link(TreeLinkWidgetContext {
                    layout,
                    source: layout.source(),
                    source_datum: &layout.source()[link.source_index],
                    target_datum: &layout.source()[link.target_index],
                    link_index: link.link_index,
                    link,
                    mark,
                }),
                link.position,
            )
        } else {
            continue;
        };
        let Some(placement) = placement(&context)? else {
            continue;
        };
        ensure(
            packet.layout.slots.len() < MAX_PLOT_WIDGET_SLOTS,
            "plot Widget slot budget exceeded",
        )?;
        let (source, nodes, links, labels) = owned.get_or_insert_with(|| {
            (
                layout.captured.source.clone(),
                Arc::new(layout.nodes.clone()),
                Arc::new(layout.links.clone()),
                Arc::new(TreeWidgetLabels {
                    labels: layout.scene.labels.clone(),
                    clips: layout.scene.label_clips.clone(),
                    series: layout.scene.label_series.clone(),
                }),
            )
        });
        let data = match context {
            TreeWidgetContext::Node(c) => TreeWidgetData::Node(TreeNodeWidgetData {
                source: source.clone(),
                nodes: nodes.clone(),
                links: links.clone(),
                labels: labels.clone(),
                source_index: c.source_index,
                node: c.node.clone(),
                plot: layout.plot,
            }),
            TreeWidgetContext::Link(c) => TreeWidgetData::Link(TreeLinkWidgetData {
                source: source.clone(),
                nodes: nodes.clone(),
                links: links.clone(),
                labels: labels.clone(),
                link_index: c.link_index,
                link: c.link.clone(),
                plot: layout.plot,
            }),
        };
        packet.layout.slots.push(PlotWidgetSlot {
            mark_id: mark.id.clone(),
            revision,
            placement: at_origin(placement, position),
        });
        packet.data.insert(mark.id.clone(), data);
    }
    Ok(packet)
}
pub fn tree_node_widgets(
    layout: &TreeLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<TreeNodeWidgetData>> {
    let packet = tree_widgets(layout, revision, |c| match c {
        TreeWidgetContext::Node(c) => placement(c),
        TreeWidgetContext::Link(_) => Ok(None),
    })?;
    Ok(PlotWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(id, data)| {
                let TreeWidgetData::Node(data) = data else {
                    unreachable!("node selector")
                };
                (id, data)
            })
            .collect(),
    })
}
pub fn tree_link_widgets(
    layout: &TreeLayout,
    revision: u64,
    mut placement: impl FnMut(&TreeLinkWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>>,
) -> Result<PlotWidgetsData<TreeLinkWidgetData>> {
    let packet = tree_widgets(layout, revision, |c| match c {
        TreeWidgetContext::Link(c) => placement(c),
        TreeWidgetContext::Node(_) => Ok(None),
    })?;
    Ok(PlotWidgetsData {
        layout: packet.layout,
        data: packet
            .data
            .into_iter()
            .map(|(id, data)| {
                let TreeWidgetData::Link(data) = data else {
                    unreachable!("link selector")
                };
                (id, data)
            })
            .collect(),
    })
}

pub fn tree_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
) -> Result<TreeLayout> {
    layout_with_factories(c, data, o, &mut TreeFactories::default())
}

/// Build the ordinary Tree Scene plus authoritative labels for every computed
/// node. Scene-label visibility remains unchanged. This opt-in constructor is
/// separate from legacy gated label computation and does not execute a raw
/// Canvas renderer.
pub fn tree_canvas_layout(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
) -> Result<TreeLayout> {
    build_tree(
        c,
        data,
        o,
        &mut TreeFactories::default(),
        None,
        TreeLabelMode::AllComputed,
    )
    .map(|(layout, _)| layout)
}

fn layout_with_factories(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
    factories: &mut TreeFactories<'_>,
) -> Result<TreeLayout> {
    build_tree(c, data, o, factories, None, TreeLabelMode::CurrentScene).map(|(layout, _)| layout)
}

pub(crate) fn layout_for_svg(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    options: &TreeOptions,
    factories: &mut TreeFactories<'_>,
    labels: bool,
) -> Result<TreeLayout> {
    build_tree(
        c,
        data,
        options,
        factories,
        None,
        if labels {
            TreeLabelMode::CurrentScene
        } else {
            TreeLabelMode::SvgOmitted
        },
    )
    .map(|(layout, _)| layout)
}

pub(crate) fn activity_for_svg(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    options: &TreeOptions,
    layers: &[crate::TreeSvgLayer],
    base: &mut TreeFactories<'_>,
    state: &TreeActivityOptions,
    modifiers: &mut TreeActivityFactories<'_>,
) -> Result<TreeActivityLayout> {
    activity::layout_with_activity_labels(
        c,
        data,
        options,
        base,
        state,
        modifiers,
        if crate::TreeSvgPlan::labels_required(layers) {
            TreeLabelMode::CurrentScene
        } else {
            TreeLabelMode::SvgOmitted
        },
    )
}

fn build_tree(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
    factories: &mut TreeFactories<'_>,
    mut activity: Option<activity::ActivityBuild<'_, '_>>,
    label_mode: TreeLabelMode,
) -> Result<(TreeLayout, Option<Arc<TreeActivityState>>)> {
    let s = c.scene()?;
    hierarchy_options(&o.hierarchy)?;
    o.style.validate()?;
    ensure(
        o.node_radius.is_finite()
            && o.node_radius >= 0.
            && o.node_radius * 2. < c.inner_width().min(c.inner_height())
            && [o.sibling_separation, o.subtree_separation]
                .iter()
                .all(|v| v.is_finite() && (0.000001..=1024.).contains(v))
            && o.rotation.is_finite()
            && o.rotation.abs() <= 1e6,
        "invalid tree layout options",
    )?;
    let Some(h) = hierarchy(data)? else {
        let state = activity
            .as_ref()
            .map(|a| activity::resolve_activity(data, None, &[], a.options))
            .transpose()?;
        return Ok((
            TreeLayout::capture(
                c,
                data,
                o,
                finish(s)?,
                None,
                (vec![], vec![], vec![]),
                (label_mode == TreeLabelMode::AllComputed).then(|| Arc::new(vec![])),
            )?,
            state,
        ));
    };
    let inset = (0..data.len())
        .map(|i| o.style.node(h.children[i].is_empty()).extent(o.node_radius))
        .fold(0., f64::max);
    ensure(
        inset * 2. < c.inner_width().min(c.inner_height()),
        "tree styled node extent exceeds plot",
    )?;
    let breadth = match o.mode {
        TreeMode::Tidy => Tidy::new(&h, o).layout(h.root)?,
        TreeMode::Cluster => cluster_breadth(&h),
    };
    let min = breadth.iter().copied().fold(f64::INFINITY, f64::min);
    let max = breadth.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pad = o.sibling_separation / 2.;
    let span = max - min + 2. * pad;
    let normalized: Vec<_> = breadth.iter().map(|v| (v - min + pad) / span).collect();
    let depth = |i: usize| {
        if h.max_depth == 0 {
            0.
        } else {
            h.depth[i] as f64 / h.max_depth as f64
        }
    };
    let (cx, cy, radius) = center(c);
    let radius = radius - inset;
    let position = |i: usize| {
        let d = depth(i);
        let b = normalized[i];
        let r = inset;
        let x = |v: f64| c.margin.left + r + v * (c.inner_width() - 2. * r);
        let y = |v: f64| c.margin.top + r + v * (c.inner_height() - 2. * r);
        match o.orientation {
            TreeOrientation::LeftToRight => Point::new(x(d), y(b)),
            TreeOrientation::RightToLeft => Point::new(x(1. - d), y(b)),
            TreeOrientation::TopToBottom => Point::new(x(b), y(d)),
            TreeOrientation::BottomToTop => Point::new(x(b), y(1. - d)),
            TreeOrientation::Radial => point(cx, cy, radius * d, o.rotation + TAU * b),
        }
    };
    let positions: Vec<_> = (0..data.len()).map(position).collect();
    let position = |i: usize| positions[i];
    let mut height = vec![0; data.len()];
    for &i in h.order.iter().rev() {
        height[i] = h.children[i]
            .iter()
            .map(|&j| height[j] + 1)
            .max()
            .unwrap_or(0);
    }
    let mut nodes: Vec<TreeNode> = (0..data.len())
        .map(|i| {
            let style = o.style.node(h.children[i].is_empty());
            let p = position(i);
            TreeNode {
                source_index: i,
                parent_index: h.parent[i],
                children: h.children[i].clone(),
                depth: h.depth[i],
                height: height[i],
                raw_value: data[i].value,
                value: h.totals[i],
                position: [p.x, p.y],
                radius: style.radius(o.node_radius),
                style: style.clone(),
                color: style.color(c, i),
                mark_id: Some(format!("node:{}", data[i].id)),
            }
        })
        .collect();
    let links: Vec<TreeLink> = h
        .order
        .iter()
        .filter_map(|&i| h.parent[i].map(|parent| (parent, i)))
        .enumerate()
        .map(|(link_index, (parent, i))| {
            let p = position(parent);
            let q = position(i);
            TreeLink {
                link_index,
                source_index: parent,
                target_index: i,
                source_position: [p.x, p.y],
                target_position: [q.x, q.y],
                position: [p.x * 0.5 + q.x * 0.5, p.y * 0.5 + q.y * 0.5],
                mark_id: o
                    .style
                    .links
                    .enabled
                    .then(|| format!("link:{}", data[i].id)),
            }
        })
        .collect();
    let activity_state = activity
        .as_ref()
        .map(|a| activity::resolve_activity(data, Some(&h), &links, a.options))
        .transpose()?;
    let link_geometries: Vec<_> = links
        .iter()
        .map(|link| {
            link.mark_id.as_ref().map(|_| {
                tree_styles::link_geometry(
                    o.style.links.curve,
                    tree_styles::TreeLinkGeometry {
                        source: position(link.source_index),
                        target: position(link.target_index),
                        source_polar: [
                            radius * depth(link.source_index),
                            o.rotation + TAU * normalized[link.source_index],
                        ],
                        target_polar: [
                            radius * depth(link.target_index),
                            o.rotation + TAU * normalized[link.target_index],
                        ],
                        center: [cx, cy],
                        orientation: o.orientation,
                    },
                )
            })
        })
        .collect();
    let mut labels = vec![None; data.len()];
    let mut link_styles: Vec<_> = links
        .iter()
        .map(|link| default_link_style(c, o, &nodes, link))
        .collect();
    let mut paint_options = o.clone();
    if label_mode == TreeLabelMode::SvgOmitted {
        paint_options.hierarchy.labels = false;
    }
    let render =
        |nodes: &[TreeNode], link_styles: &[ResolvedLinkStyle], labels: &[Option<String>]| {
            paint_tree(TreePaintPlan {
                c,
                data,
                o: &paint_options,
                hierarchy: &h,
                normalized: &normalized,
                nodes,
                links: &links,
                link_geometries: &link_geometries,
                link_styles,
                labels,
            })
        };
    // Validate the entire original source/options/projection/path/Scene before
    // invoking user code. This is a preparation pass, not an accepted Layout;
    // the function path emits its Scene from the resolved computed plan below.
    let original = render(&nodes, &link_styles, &labels)?;
    if factories.is_empty()
        && activity.as_ref().is_none_or(|a| {
            a.options.focus_id.is_none() && a.options.fix_node_color_at_depth.is_none()
        })
    {
        let canvas_labels =
            capture_canvas_labels(c, data, o, &h, &normalized, (&nodes, &labels), label_mode)?;
        return Ok((
            TreeLayout::capture(
                c,
                data,
                o,
                original,
                Some(h.root),
                (nodes, links, link_styles),
                canvas_labels,
            )?,
            activity_state,
        ));
    }
    let plot = ClipRect::plot(c);
    for &i in &h.order {
        let node = &nodes[i];
        let context = TreeNodeFunctionContext {
            source: data,
            datum: &data[i],
            source_index: i,
            parent_index: node.parent_index,
            children: &node.children,
            depth: node.depth,
            height: node.height,
            value: node.value,
            position: node.position,
            plot,
        };
        let size = match &mut factories.node_size {
            Some(size) => {
                let value = size(&context)?;
                ensure(
                    value.is_finite() && (0. ..=32768.).contains(&value),
                    "invalid Tree function node size",
                )?;
                Some(value)
            }
            None => None,
        };
        let inherited = activity
            .as_ref()
            .and_then(|a| a.options.fix_node_color_at_depth)
            .filter(|depth| node.depth > 0 && node.depth > *depth)
            .and(node.parent_index)
            .map(|parent| nodes[parent].color);
        let color = if let Some(inherited) = inherited {
            Some(inherited)
        } else {
            match &mut factories.node_color {
                Some(color) => Some(Some(color(&context)?)),
                None => None,
            }
        };
        let diameter = size.unwrap_or(nodes[i].radius * 2.);
        if let Some(diameter) = size {
            nodes[i].radius = diameter / 2.;
            nodes[i].style.radius = Some(diameter / 2.);
        }
        if let Some(color) = color {
            nodes[i].color = color;
            nodes[i].style.fill = color.map_or(TreeNodeFill::None, TreeNodeFill::Fixed);
        }
        if let Some(is_active) = activity_state.as_ref().and_then(|s| s.nodes[i]) {
            let build = activity.as_mut().unwrap();
            let modifier = if is_active {
                &mut build.factories.active_node_size
            } else {
                &mut build.factories.inactive_node_size
            };
            if let Some(modifier) = modifier {
                let diameter = modifier(&TreeNodeModifierContext {
                    source: data,
                    datum: &data[i],
                    node: &nodes[i],
                    size: diameter,
                    is_active,
                    activity: activity_state.as_deref().unwrap(),
                    plot,
                })?;
                ensure(
                    diameter.is_finite() && (0. ..=32768.).contains(&diameter),
                    "invalid Tree activity node size",
                )?;
                nodes[i].radius = diameter / 2.;
                nodes[i].style.radius = Some(diameter / 2.);
            }
        }
    }
    for link in &links {
        let mut style = default_link_style(c, o, &nodes, link);
        let context = TreeLinkFunctionContext {
            source: data,
            link,
            source_node: &nodes[link.source_index],
            target_node: &nodes[link.target_index],
            plot,
            activity: activity_state.as_deref(),
        };
        if let Some(thickness) = &mut factories.link_thickness {
            style.width = thickness(&context)?;
            ensure(
                style.width.is_finite()
                    && (style.width == 0. || (0.000001..=10000.).contains(&style.width)),
                "invalid Tree function link thickness",
            )?;
        }
        if let Some(color) = &mut factories.link_color {
            style.color = color(&context)?;
        }
        if let Some(is_active) = activity_state
            .as_ref()
            .and_then(|s| s.links[link.link_index])
        {
            let build = activity.as_mut().unwrap();
            let modifier = if is_active {
                &mut build.factories.active_link_thickness
            } else {
                &mut build.factories.inactive_link_thickness
            };
            if let Some(modifier) = modifier {
                style.width = modifier(&TreeLinkModifierContext {
                    source: data,
                    link,
                    source_node: &nodes[link.source_index],
                    target_node: &nodes[link.target_index],
                    thickness: style.width,
                    color: style.color,
                    is_active,
                    activity: activity_state.as_deref().unwrap(),
                    plot,
                })?;
                ensure(
                    style.width.is_finite()
                        && (style.width == 0. || (0.000001..=10000.).contains(&style.width)),
                    "invalid Tree activity link thickness",
                )?;
            }
        }
        link_styles[link.link_index] = style;
    }
    if label_mode != TreeLabelMode::SvgOmitted
        && let Some(label) = &mut factories.label
    {
        for &i in &h.order {
            if label_mode == TreeLabelMode::AllComputed
                || (o.hierarchy.labels
                    && tree_styles::label_eligible(&o.style.labels, nodes[i].children.is_empty()))
            {
                let text = label(&TreeLabelFunctionContext {
                    source: data,
                    datum: &data[i],
                    source_index: i,
                    node: &nodes[i],
                    plot,
                    activity: activity_state.as_deref(),
                })?;
                ensure(
                    text.len() <= crate::scene::MAX_LABEL_BYTES,
                    "Tree function label exceeds text budget",
                )?;
                labels[i] = Some(text);
            }
        }
    }
    let scene = render(&nodes, &link_styles, &labels)?;
    let canvas_labels =
        capture_canvas_labels(c, data, o, &h, &normalized, (&nodes, &labels), label_mode)?;
    Ok((
        TreeLayout::capture(
            c,
            data,
            o,
            scene,
            Some(h.root),
            (nodes, links, link_styles),
            canvas_labels,
        )?,
        activity_state,
    ))
}

fn capture_canvas_labels(
    c: &ChartConfig,
    data: &[HierarchyDatum],
    o: &TreeOptions,
    hierarchy: &Hierarchy,
    normalized: &[f64],
    computed: (&[TreeNode], &[Option<String>]),
    mode: TreeLabelMode,
) -> Result<Option<Arc<Vec<TreeComputedCanvasLabel>>>> {
    if mode != TreeLabelMode::AllComputed {
        return Ok(None);
    }
    let (nodes, texts) = computed;
    let mut options = o.style.labels.clone();
    options.visibility = TreeLabels::All;
    let mut validation = c.scene()?;
    for &index in &hierarchy.order {
        let node = &nodes[index];
        let label = tree_styles::tree_label(
            c,
            &options,
            tree_styles::TreeLabelGeometry {
                position: Point::new(node.position[0], node.position[1]),
                orientation: o.orientation,
                angle: o.rotation + TAU * normalized[index],
                leaf: node.children.is_empty(),
                style: &node.style,
                fallback_radius: o.node_radius,
                text: texts[index].as_deref().unwrap_or(&data[index].label),
            },
        )
        .expect("All visibility always computes a Tree label");
        let label_index = validation.labels.len();
        validation.labels.push(label);
        validation
            .label_series
            .insert(label_index, data[index].id.clone());
        if o.style.clip_plot {
            validation.label_clips.insert(
                label_index,
                ClipPath {
                    geometry: Geometry::Rect {
                        x: c.margin.left,
                        y: c.margin.top,
                        width: c.inner_width(),
                        height: c.inner_height(),
                    },
                    rule: ClipRule::NonZero,
                },
            );
        }
    }
    validation.validate()?;
    let labels = hierarchy
        .order
        .iter()
        .enumerate()
        .map(|(label_index, &source_index)| TreeComputedCanvasLabel {
            source_index,
            label: validation.labels[label_index].clone(),
            font: validation.resolved_label_font(label_index),
            whitespace: validation.typography.label_whitespace(label_index),
            stroke: validation
                .typography
                .label_strokes
                .get(&label_index)
                .copied(),
            clip: validation.label_clips.get(&label_index).cloned(),
            eligible: o.hierarchy.labels
                && tree_styles::label_eligible(
                    &o.style.labels,
                    nodes[source_index].children.is_empty(),
                ),
        })
        .collect();
    Ok(Some(Arc::new(labels)))
}

#[derive(Clone, Copy)]
struct ResolvedLinkStyle {
    width: f64,
    color: Color,
}
fn default_link_style(
    c: &ChartConfig,
    o: &TreeOptions,
    nodes: &[TreeNode],
    link: &TreeLink,
) -> ResolvedLinkStyle {
    let color = |index: usize| {
        nodes[index]
            .color
            .or(nodes[index].style.border.map(|s| s.color))
            .unwrap_or(c.theme.grid)
    };
    ResolvedLinkStyle {
        width: o.style.links.width,
        color: match o.style.links.color {
            TreeLinkColor::Theme => c.theme.grid,
            TreeLinkColor::Fixed(color) => color,
            TreeLinkColor::Source => color(link.source_index),
            TreeLinkColor::Target => color(link.target_index),
        },
    }
}
struct TreePaintPlan<'a> {
    c: &'a ChartConfig,
    data: &'a [HierarchyDatum],
    o: &'a TreeOptions,
    hierarchy: &'a Hierarchy,
    normalized: &'a [f64],
    nodes: &'a [TreeNode],
    links: &'a [TreeLink],
    link_geometries: &'a [Option<Geometry>],
    link_styles: &'a [ResolvedLinkStyle],
    labels: &'a [Option<String>],
}
fn paint_tree(plan: TreePaintPlan<'_>) -> Result<Scene> {
    let TreePaintPlan {
        c,
        data,
        o,
        hierarchy: h,
        normalized,
        nodes,
        links,
        link_geometries,
        link_styles,
        labels,
    } = plan;
    let mut scene = c.scene()?;
    for link in links {
        let Some(id) = &link.mark_id else {
            continue;
        };
        let style = link_styles[link.link_index];
        let mut mark = Mark::new(
            id,
            link_geometries[link.link_index].as_ref().unwrap().clone(),
            style.color,
        );
        mark.fill = None;
        mark.stroke = (style.width > 0.).then_some(GraphicStroke {
            color: style.color,
            width: style.width,
            cap: o.style.links.cap,
        });
        // A zero-width function result keeps the actual decorative Mark without
        // inventing a stroke or an otherwise unsupported dash-without-stroke.
        mark.stroke_dash = if mark.stroke.is_some() {
            o.style.links.dash.clone()
        } else {
            None
        };
        mark.opacity = o.style.links.opacity;
        if o.style.clip_plot {
            mark.clip = Some(ClipRect::plot(c));
        }
        mark.label.clear();
        scene.marks.push(mark);
    }
    for &i in &h.order {
        let node = &nodes[i];
        let p = Point::new(node.position[0], node.position[1]);
        let leaf = node.children.is_empty();
        let style = &node.style;
        let mut mark = node_mark(data, h, i, style.geometry(p, o.node_radius), c);
        mark.fill = node.color;
        mark.stroke = style.border;
        mark.opacity = style.opacity;
        if o.style.clip_plot {
            mark.clip = Some(ClipRect::plot(c));
        }
        scene.marks.push(mark);
        if o.hierarchy.labels
            && let Some(label) = tree_styles::tree_label(
                c,
                &o.style.labels,
                tree_styles::TreeLabelGeometry {
                    position: p,
                    orientation: o.orientation,
                    angle: o.rotation + TAU * normalized[i],
                    leaf,
                    style,
                    fallback_radius: o.node_radius,
                    text: labels[i].as_deref().unwrap_or(&data[i].label),
                },
            )
        {
            let index = scene.labels.len();
            scene.labels.push(label);
            scene.label_series.insert(index, data[i].id.clone());
            if o.style.clip_plot {
                scene.label_clips.insert(
                    index,
                    crate::ClipPath {
                        geometry: Geometry::Rect {
                            x: c.margin.left,
                            y: c.margin.top,
                            width: c.inner_width(),
                            height: c.inner_height(),
                        },
                        rule: crate::ClipRule::NonZero,
                    },
                );
            }
        }
    }
    finish(scene)
}
