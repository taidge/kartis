//! Bounded data-space hierarchy navigation. Every transition computes and
//! validates the actual proposed layout before committing data/focus/history.
use std::sync::Arc;

use crate::hierarchy_layout::{HierarchyModel, circle_packing_model, icicle_model};
use crate::scene::ensure;
use crate::*;

mod motion;
pub use motion::HierarchyFocusMotion;

pub const MAX_HIERARCHY_FOCUS_HISTORY: usize = 128;
#[derive(Clone, Debug)]
pub struct HierarchyFocusController {
    model: Arc<HierarchyModel>,
    focus: Option<String>,
    history: Vec<String>,
    treemap: TreemapLayoutState,
    revision: u64,
}
impl HierarchyFocusController {
    pub fn new(data: &[HierarchyDatum]) -> Result<Self> {
        let model = Arc::new(HierarchyModel::new(data)?);
        Ok(Self {
            focus: model.root().map(String::from),
            model,
            history: vec![],
            treemap: Default::default(),
            revision: 0,
        })
    }
    pub fn current(&self) -> Option<&str> {
        self.focus.as_deref()
    }
    pub fn root_id(&self) -> Option<&str> {
        self.model.root()
    }
    pub fn data(&self) -> &[HierarchyDatum] {
        &self.model.data
    }
    pub fn history(&self) -> &[String] {
        &self.history
    }
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn treemap_state(&self) -> &TreemapLayoutState {
        &self.treemap
    }
    pub fn ancestors(&self) -> Vec<String> {
        self.focus
            .as_deref()
            .map(|id| self.model.ancestors(id).unwrap_or_default())
            .unwrap_or_default()
    }
    fn build(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<HierarchyLayout> {
        match view {
            HierarchyView::Treemap(o) => {
                self.treemap
                    .layout_model(c, &self.model, self.focus.as_deref(), o)
            }
            HierarchyView::CirclePacking(o) => {
                circle_packing_model(c, &self.model, self.focus.as_deref(), o)
            }
            HierarchyView::Icicle(o) => icicle_model(c, &self.model, self.focus.as_deref(), o),
        }
    }
    pub fn layout(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<HierarchyLayout> {
        let mut next = self.clone();
        let result = next.build(c, view)?;
        *self = next;
        Ok(result)
    }
    fn changed(&mut self) -> Result<()> {
        self.revision = self
            .revision
            .checked_add(1)
            .ok_or_else(|| ChartError("hierarchy focus revision exhausted".into()))?;
        Ok(())
    }
    fn push(&mut self, id: String) {
        if self.history.last() != Some(&id) {
            if self.history.len() == MAX_HIERARCHY_FOCUS_HISTORY {
                self.history.remove(0);
            }
            self.history.push(id);
        }
    }
    pub fn focus(
        &mut self,
        c: &ChartConfig,
        id: &str,
        view: &HierarchyView,
    ) -> Result<HierarchyLayout> {
        ensure(
            self.model.index.contains_key(id),
            "unknown hierarchy focus ID",
        )?;
        let mut next = self.clone();
        if next.current() != Some(id) {
            if let Some(previous) = next.focus.take() {
                next.push(previous);
            }
            next.focus = Some(id.into());
            next.changed()?;
        }
        let result = next.build(c, view)?;
        *self = next;
        Ok(result)
    }
    pub fn root(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<HierarchyLayout> {
        if let Some(id) = self.root_id().map(String::from) {
            self.focus(c, &id, view)
        } else {
            self.layout(c, view)
        }
    }
    pub fn parent(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<HierarchyLayout> {
        if let Some(parent) = self.ancestors().last().cloned() {
            self.focus(c, &parent, view)
        } else {
            self.layout(c, view)
        }
    }
    pub fn back(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<HierarchyLayout> {
        let mut next = self.clone();
        while let Some(id) = next.history.pop() {
            if next.model.index.contains_key(&id) && next.current() != Some(id.as_str()) {
                next.focus = Some(id);
                next.changed()?;
                break;
            }
        }
        let result = next.build(c, view)?;
        *self = next;
        Ok(result)
    }
    /// Keep a surviving focus ID; otherwise recover its nearest surviving old
    /// ancestor, then the new root. Empty input clears focus and history.
    pub fn replace(
        &mut self,
        c: &ChartConfig,
        data: &[HierarchyDatum],
        view: &HierarchyView,
    ) -> Result<HierarchyLayout> {
        let model = Arc::new(HierarchyModel::new(data)?);
        let mut next = self.clone();
        let focus = self
            .current()
            .filter(|id| model.index.contains_key(*id))
            .map(String::from)
            .or_else(|| {
                self.ancestors()
                    .into_iter()
                    .rev()
                    .find(|id| model.index.contains_key(id))
            })
            .or_else(|| model.root().map(String::from));
        next.model = model;
        next.focus = focus;
        next.history.retain(|id| next.model.index.contains_key(id));
        next.history.dedup();
        while next.history.last() == next.focus.as_ref() && !next.history.is_empty() {
            next.history.pop();
        }
        next.treemap.reconcile(&next.model);
        next.changed()?;
        let result = next.build(c, view)?;
        *self = next;
        Ok(result)
    }
}
