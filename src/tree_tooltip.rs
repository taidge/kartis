//! Complete Tree tooltip data, independent of painted Mark eligibility.
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;

#[derive(Clone, Debug)]
struct Data {
    config: ChartConfig,
    layout: TreeActivityLayout,
    source_index: usize,
}

/// An immutable, validated Tree node and its original source/activity cohort.
/// A zero-radius, transparent or empty-label node is valid without a painted
/// Mark. Constructing data does not select, hover or activate that source.
#[derive(Clone, Debug)]
pub struct TreeTooltipData(Arc<Data>);

impl PartialEq for TreeTooltipData {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
            || (self.config() == other.config()
                && self.0.source_index == other.0.source_index
                && self.activity_layout() == other.activity_layout()
                && self.layout().computed_link_styles() == other.layout().computed_link_styles()
                && self.layout().computed_canvas_labels()
                    == other.layout().computed_canvas_labels())
    }
}

impl TreeTooltipData {
    /// Capture an actual original source ID, never a prefixed Mark ID or a
    /// guessed bounds center. Geometry and capture provenance validate first.
    pub fn new(config: ChartConfig, layout: TreeActivityLayout, source_id: &str) -> Result<Self> {
        config.validate()?;
        tree_activity_frame(&layout)?;
        ensure(
            config.width == layout.layout.scene.width
                && config.height == layout.layout.scene.height
                && ClipRect::plot(&config) == layout.layout.plot,
            "Tree tooltip config differs from layout dimensions or plot",
        )?;
        let source_index = layout
            .layout
            .source()
            .iter()
            .position(|datum| datum.id == source_id)
            .ok_or_else(|| ChartError("Tree tooltip references an unknown source ID".into()))?;
        Ok(Self(Arc::new(Data {
            config,
            layout,
            source_index,
        })))
    }
    pub fn config(&self) -> &ChartConfig {
        &self.0.config
    }
    pub fn layout(&self) -> &TreeLayout {
        &self.0.layout.layout
    }
    pub fn activity_layout(&self) -> &TreeActivityLayout {
        &self.0.layout
    }
    pub fn activity(&self) -> &TreeActivityState {
        &self.0.layout.activity
    }
    pub fn source(&self) -> &[HierarchyDatum] {
        self.layout().source()
    }
    pub fn source_index(&self) -> usize {
        self.0.source_index
    }
    pub fn datum(&self) -> &HierarchyDatum {
        &self.source()[self.source_index()]
    }
    pub fn node(&self) -> &TreeNode {
        &self.layout().nodes[self.source_index()]
    }
    pub fn is_active(&self) -> Option<bool> {
        self.activity().nodes[self.source_index()]
    }
    /// Original computed node center in outer logical Scene coordinates.
    pub fn anchor(&self) -> [f64; 2] {
        self.node().position
    }
    /// Safe plain-text formatting. The default numeric value is the computed
    /// aggregate, matching the existing Tree Mark tooltip. `datum().value`
    /// separately retains the raw business value, including its zero sign.
    pub fn text(&self, options: &TooltipOptions) -> Result<String> {
        tooltip_data_text(
            options,
            &[TooltipDatum {
                id: &self.datum().id,
                label: &self.datum().label,
                series: &self.datum().id,
                value: Some(self.node().value),
            }],
        )
    }
}
