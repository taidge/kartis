use std::collections::BTreeMap;

use super::HierarchyFocusController;
use crate::crosshair::mark_center;
use crate::motion::{PropertyMotion, PropertyMotionOptions};
use crate::{
    ChartConfig, ChartError, HierarchyDatum, HierarchyLayout, HierarchyView, InteractionBinding,
    Result, Scene, SceneInteraction,
};

/// Animated data-space focus for treemap, circle packing and icicle layouts.
/// Retargeting preserves the sampled geometry and velocity of compatible marks.
#[derive(Clone)]
pub struct HierarchyFocusMotion {
    controller: HierarchyFocusController,
    target: HierarchyLayout,
    binding: InteractionBinding,
    motion: PropertyMotion,
    reduced_motion: bool,
}

impl HierarchyFocusMotion {
    pub fn new(
        c: &ChartConfig,
        data: &[HierarchyDatum],
        view: &HierarchyView,
        options: PropertyMotionOptions,
    ) -> Result<Self> {
        let mut controller = HierarchyFocusController::new(data)?;
        let target = controller.layout(c, view)?;
        let binding = target.interaction_binding()?;
        let motion = PropertyMotion::new(target.scene.clone(), options)?;
        Ok(Self {
            controller,
            target,
            binding,
            motion,
            reduced_motion: false,
        })
    }

    pub fn controller(&self) -> &HierarchyFocusController {
        &self.controller
    }

    pub fn target_layout(&self) -> &HierarchyLayout {
        &self.target
    }

    pub fn scene(&self) -> &Scene {
        self.motion.scene()
    }

    /// Supplies the sampled Scene to a SurfaceModel together with accepted raw
    /// data and live anchors. Retiring ink is excluded from the typed binding.
    pub fn frame_interaction(&self) -> Result<SceneInteraction> {
        let scene = self.scene();
        let mut binding = self.binding.clone();
        if !self.is_settled() {
            let marks = scene
                .marks
                .iter()
                .map(|mark| (mark.id.as_str(), mark))
                .collect::<BTreeMap<_, _>>();
            for site in &mut binding.sites {
                site.position = marks
                    .get(site.mark_id.as_str())
                    .and_then(|mark| mark_center(mark))
                    .ok_or_else(|| ChartError("missing hierarchy motion datum anchor".into()))?;
            }
        }
        Ok(SceneInteraction {
            scene: scene.clone(),
            interaction: Some(binding),
            plot: Some(self.target.plot()),
        })
    }

    pub fn is_settled(&self) -> bool {
        self.motion.is_settled()
    }

    pub fn tick(&mut self, dt: f64) -> Result<&Scene> {
        self.motion.tick(dt)
    }

    pub fn finish(&mut self) {
        self.motion.finish();
    }

    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
        self.motion.set_reduced_motion(reduced);
    }

    /// Retarget a new layout. A viewport size or plot change is applied at once.
    pub fn relayout(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.layout(c, view))
    }

    pub fn focus(
        &mut self,
        c: &ChartConfig,
        id: &str,
        view: &HierarchyView,
    ) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.focus(c, id, view))
    }

    pub fn parent(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.parent(c, view))
    }

    pub fn back(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.back(c, view))
    }

    pub fn root(&mut self, c: &ChartConfig, view: &HierarchyView) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.root(c, view))
    }

    pub fn replace(
        &mut self,
        c: &ChartConfig,
        data: &[HierarchyDatum],
        view: &HierarchyView,
    ) -> Result<&HierarchyLayout> {
        self.retarget(|controller| controller.replace(c, data, view))
    }

    fn retarget(
        &mut self,
        layout: impl FnOnce(&mut HierarchyFocusController) -> Result<HierarchyLayout>,
    ) -> Result<&HierarchyLayout> {
        let mut controller = self.controller.clone();
        let target = layout(&mut controller)?;
        let binding = target.interaction_binding()?;
        if target.plot() == self.target.plot()
            && target.scene.width == self.target.scene.width
            && target.scene.height == self.target.scene.height
        {
            self.motion.retarget(target.scene.clone())?;
        } else {
            let mut motion =
                PropertyMotion::new(target.scene.clone(), self.motion.options().clone())?;
            motion.set_reduced_motion(self.reduced_motion);
            self.motion = motion;
        }
        self.controller = controller;
        self.target = target;
        self.binding = binding;
        Ok(&self.target)
    }
}
