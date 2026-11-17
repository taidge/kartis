//! Immutable Tree event data, independent of native event transport or picking.
//!
//! Capturing a frame validates its original source and constructor caches. It
//! neither picks nor activates anything. Browser adapters attach an accepted
//! generation and actual native-event provenance separately.
use std::collections::BTreeMap;

use crate::*;

/// A shared, immutable source cohort for typed Tree node and link events.
///
/// Cloning this frame or selecting a target shares the captured pool; it does
/// not clone the complete layout or replay user layout/style functions.
/// Node positions remain in outer logical Scene coordinates. This is source
/// data, not a claim that every source role currently has visible paint.
#[derive(Clone, Debug)]
pub struct TreeEventFrame(TreeCanvasFrame);

impl TreeEventFrame {
    pub fn new(config: ChartConfig, layout: TreeLayout) -> Result<Self> {
        TreeCanvasFrame::new(config, layout).map(Self)
    }

    pub fn with_activity(config: ChartConfig, layout: TreeActivityLayout) -> Result<Self> {
        TreeCanvasFrame::with_activity(config, layout).map(Self)
    }

    pub fn config(&self) -> &ChartConfig {
        self.0.config()
    }

    pub fn theme(&self) -> &Theme {
        self.0.theme()
    }

    pub fn layout(&self) -> &TreeLayout {
        self.0.layout()
    }

    /// The original source order, including raw signed-zero values.
    pub fn source(&self) -> &[HierarchyDatum] {
        self.0.source()
    }

    pub fn nodes(&self) -> &[TreeNode] {
        &self.layout().nodes
    }

    pub fn links(&self) -> &[TreeLink] {
        &self.layout().links
    }

    pub fn computed_link_styles(&self) -> &[TreeComputedLinkStyle] {
        self.0.computed_link_styles()
    }

    pub fn activity(&self) -> Option<&TreeActivityState> {
        self.0.activity()
    }

    pub fn scene(&self) -> &Scene {
        &self.layout().scene
    }

    pub fn plot(&self) -> ClipRect {
        self.layout().plot
    }

    pub fn node_by_id(&self) -> &BTreeMap<String, usize> {
        self.0.node_by_id()
    }

    /// Resolve an original source Mark, if it actually exists in this cohort.
    pub fn source_mark(&self, id: &str) -> Option<&Mark> {
        self.0.source_mark(id)
    }

    /// Capture one source node without allocating a second data pool.
    pub fn node(&self, source_index: usize) -> Result<TreeNodeEventData> {
        self.0.node_context(source_index)?;
        Ok(TreeNodeEventData {
            frame: self.clone(),
            source_index,
        })
    }

    pub fn find_node(&self, source_id: &str) -> Option<TreeNodeEventData> {
        self.node_by_id()
            .get(source_id)
            .map(|&source_index| TreeNodeEventData {
                frame: self.clone(),
                source_index,
            })
    }

    /// Capture a computed link and both original endpoints, including links
    /// whose disabled rendering never created a Mark or canonical control.
    pub fn link(&self, link_index: usize) -> Result<TreeLinkEventData> {
        if link_index >= self.links().len() {
            return Err(ChartError("Tree event link index is missing".into()));
        }
        Ok(TreeLinkEventData {
            frame: self.clone(),
            link_index,
        })
    }
}

/// One node and its shared original source/configuration/activity cohort.
#[derive(Clone, Debug)]
pub struct TreeNodeEventData {
    frame: TreeEventFrame,
    source_index: usize,
}

impl TreeNodeEventData {
    pub fn frame(&self) -> &TreeEventFrame {
        &self.frame
    }

    pub fn source_index(&self) -> usize {
        self.source_index
    }

    pub fn datum(&self) -> &HierarchyDatum {
        &self.frame.source()[self.source_index]
    }

    pub fn node(&self) -> &TreeNode {
        &self.frame.nodes()[self.source_index]
    }

    pub fn mark(&self) -> Option<&Mark> {
        self.node()
            .mark_id
            .as_deref()
            .and_then(|id| self.frame.source_mark(id))
    }

    pub fn is_active(&self) -> Option<bool> {
        self.frame
            .activity()
            .and_then(|activity| activity.nodes[self.source_index])
    }
}

/// One computed link, its original raw endpoints and resolved constructor
/// appearance. This does not fabricate a canonical link interaction site.
#[derive(Clone, Debug)]
pub struct TreeLinkEventData {
    frame: TreeEventFrame,
    link_index: usize,
}

impl TreeLinkEventData {
    pub fn frame(&self) -> &TreeEventFrame {
        &self.frame
    }

    pub fn link_index(&self) -> usize {
        self.link_index
    }

    pub fn link(&self) -> &TreeLink {
        &self.frame.links()[self.link_index]
    }

    pub fn source_index(&self) -> usize {
        self.link().source_index
    }

    pub fn target_index(&self) -> usize {
        self.link().target_index
    }

    pub fn source_datum(&self) -> &HierarchyDatum {
        &self.frame.source()[self.source_index()]
    }

    pub fn target_datum(&self) -> &HierarchyDatum {
        &self.frame.source()[self.target_index()]
    }

    pub fn source_node(&self) -> &TreeNode {
        &self.frame.nodes()[self.source_index()]
    }

    pub fn target_node(&self) -> &TreeNode {
        &self.frame.nodes()[self.target_index()]
    }

    pub fn style(&self) -> &TreeComputedLinkStyle {
        &self.frame.computed_link_styles()[self.link_index]
    }

    pub fn mark(&self) -> Option<&Mark> {
        self.link()
            .mark_id
            .as_deref()
            .and_then(|id| self.frame.source_mark(id))
    }

    pub fn is_active(&self) -> Option<bool> {
        self.frame
            .activity()
            .and_then(|activity| activity.links[self.link_index])
    }
}
