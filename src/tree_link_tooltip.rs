//! Immutable Tree link tooltip data without a fabricated scalar link value.
use crate::*;

/// Both original endpoints, computed nodes and resolved style of one Tree link.
///
/// Cloning shares the captured event pool. Data alone does not create native
/// paint, an interaction site or a visible tooltip. The browser presentation
/// supplies accepted generation, pointer coordinates and occurrence provenance.
#[derive(Clone, Debug)]
pub struct TreeLinkTooltipData(TreeLinkEventData);

impl PartialEq for TreeLinkTooltipData {
    fn eq(&self, other: &Self) -> bool {
        // A pool is immutable. Distinct captures are conservatively different:
        // structural layout equality can omit constructor caches or zero signs.
        self.link_index() == other.link_index()
            && std::ptr::eq(self.frame().scene(), other.frame().scene())
    }
}

impl TreeLinkTooltipData {
    /// Resolve a link by its child's original business ID, not a Mark ID or
    /// link-array index. Validation precedes lookup, even for disabled links.
    pub fn new(
        config: ChartConfig,
        layout: TreeActivityLayout,
        target_source_id: &str,
    ) -> Result<Self> {
        let frame = TreeEventFrame::with_activity(config, layout)?;
        let link_index = frame
            .links()
            .iter()
            .position(|link| frame.source()[link.target_index].id == target_source_id)
            .ok_or_else(|| ChartError("Tree link tooltip references an unknown child ID".into()))?;
        Ok(Self::from_link(frame.link(link_index)?))
    }

    /// Reuse a validated link and its existing immutable pool without cloning
    /// the layout or replaying source/style functions.
    pub fn from_link(data: TreeLinkEventData) -> Self {
        Self(data)
    }

    pub fn link_data(&self) -> &TreeLinkEventData {
        &self.0
    }
    pub fn frame(&self) -> &TreeEventFrame {
        self.0.frame()
    }
    pub fn config(&self) -> &ChartConfig {
        self.frame().config()
    }
    pub fn layout(&self) -> &TreeLayout {
        self.frame().layout()
    }
    pub fn theme(&self) -> &Theme {
        self.frame().theme()
    }
    pub fn source(&self) -> &[HierarchyDatum] {
        self.frame().source()
    }
    pub fn activity(&self) -> Option<&TreeActivityState> {
        self.frame().activity()
    }
    pub fn link_index(&self) -> usize {
        self.0.link_index()
    }
    pub fn link(&self) -> &TreeLink {
        self.0.link()
    }
    pub fn source_index(&self) -> usize {
        self.0.source_index()
    }
    pub fn target_index(&self) -> usize {
        self.0.target_index()
    }
    pub fn source_datum(&self) -> &HierarchyDatum {
        self.0.source_datum()
    }
    pub fn target_datum(&self) -> &HierarchyDatum {
        self.0.target_datum()
    }
    pub fn source_node(&self) -> &TreeNode {
        self.0.source_node()
    }
    pub fn target_node(&self) -> &TreeNode {
        self.0.target_node()
    }
    pub fn style(&self) -> &TreeComputedLinkStyle {
        self.0.style()
    }
    pub fn mark(&self) -> Option<&Mark> {
        self.0.mark()
    }
    pub fn is_active(&self) -> Option<bool> {
        self.0.is_active()
    }

    /// Safe plain text, in source-then-target order. Each row uses that
    /// endpoint's computed aggregate; the original signed business values
    /// remain available through the datum getters. No scalar is assigned to
    /// the link. The usual tooltip template/format/budget rules apply.
    pub fn text(&self, options: &TooltipOptions) -> Result<String> {
        tooltip_data_text(
            options,
            &[
                TooltipDatum {
                    id: &self.source_datum().id,
                    label: &self.source_datum().label,
                    series: &self.source_datum().id,
                    value: Some(self.source_node().value),
                },
                TooltipDatum {
                    id: &self.target_datum().id,
                    label: &self.target_datum().label,
                    series: &self.target_datum().id,
                    value: Some(self.target_node().value),
                },
            ],
        )
    }
}

impl From<TreeLinkEventData> for TreeLinkTooltipData {
    fn from(data: TreeLinkEventData) -> Self {
        Self::from_link(data)
    }
}
