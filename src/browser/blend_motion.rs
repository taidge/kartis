use super::*;

/// One explicit request to change the Scene and its blend profile together.
pub(super) struct Animation {
    pub scene: Scene,
    pub interaction: Option<InteractionBinding>,
    pub contours: Option<motion::ContourBindings>,
    pub modes: MarkBlends,
}

impl BrowserChart {
    /// Animate geometry and artistic blend modes together. Use an empty profile
    /// to fade from the current artistic modes to normal compositing.
    pub fn animate_to_with_blends(&self, scene: Scene, modes: MarkBlends) -> Result<()> {
        self.handle().animate_to_with_blends(scene, modes)
    }

    /// Animate a new Scene and immediately bind its current business data.
    pub fn animate_to_with_interaction_and_blends(
        &self,
        scene: Scene,
        interaction: InteractionBinding,
        modes: MarkBlends,
    ) -> Result<()> {
        self.handle()
            .animate_to_with_interaction_and_blends(scene, interaction, modes)
    }

    /// Animate a Scene with explicit contour correspondence and blend modes.
    /// Requires a Properties animation profile.
    pub fn animate_to_with_contours_and_blends(
        &self,
        scene: Scene,
        contours: motion::ContourBindings,
        modes: MarkBlends,
    ) -> Result<()> {
        self.handle()
            .animate_to_with_contours_and_blends(scene, contours, modes)
    }
}

impl BrowserHandle {
    /// Retarget geometry and artistic blend modes together.
    pub fn animate_to_with_blends(&self, scene: Scene, modes: MarkBlends) -> Result<()> {
        self.enqueue_blended_animation(Animation {
            scene,
            interaction: None,
            contours: None,
            modes,
        })
    }

    /// Retarget geometry, blend modes, and interaction together.
    pub fn animate_to_with_interaction_and_blends(
        &self,
        scene: Scene,
        interaction: InteractionBinding,
        modes: MarkBlends,
    ) -> Result<()> {
        self.enqueue_blended_animation(Animation {
            scene,
            interaction: Some(interaction),
            contours: None,
            modes,
        })
    }

    /// Retarget explicit contour correspondence and blend modes together.
    pub fn animate_to_with_contours_and_blends(
        &self,
        scene: Scene,
        contours: motion::ContourBindings,
        modes: MarkBlends,
    ) -> Result<()> {
        self.enqueue_blended_animation(Animation {
            scene,
            interaction: None,
            contours: Some(contours),
            modes,
        })
    }

    fn enqueue_blended_animation(&self, request: Animation) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
            state.admit_blended_animation(&engine.window, &request)?;
        }
        engine.try_enqueue(Input::BlendedAnimation(Box::new(request)))
    }
}

impl State {
    pub(super) fn current_blends(&self) -> &MarkBlends {
        self.motion
            .as_ref()
            .map_or(&self.blends, RunningMotion::blends)
    }

    pub(super) fn freeze_motion(&mut self) {
        if let Some(motion) = self.motion.take() {
            // Leave the accepted physical Scene and its actual ghost paint
            // intact when a gesture interrupts animation.
            if motion.blends() != self.blends.as_ref() {
                self.blends = Rc::new(motion.blends().clone());
            }
        }
        self.last_time = None;
    }

    fn admit_blended_animation(
        &self,
        window: &Window,
        request: &Animation,
    ) -> Result<Option<RunningMotion>> {
        if request.contours.is_some() {
            scene::ensure(
                !matches!(self.options.animation, BrowserAnimation::Scene),
                "contour bindings require Properties animation",
            )?;
        }
        let empty = motion::ContourBindings::new();
        self.admit_interaction_scene_with_contours(
            &request.scene,
            request.interaction.as_ref(),
            request.contours.as_ref().unwrap_or(&empty),
        )?;
        let renderer = self.options.renderer;
        let view = self.model.paint_transform();
        blends::admit(renderer, window, &request.scene, &request.modes, view)?;
        if self.reduced {
            return Ok(None);
        }
        let candidate = self.motion_candidate_with_blends(
            request.scene.clone(),
            request.contours.clone(),
            request.interaction.as_ref(),
            request.modes.clone(),
        )?;
        let modes = candidate.blends();
        if renderer == BrowserRenderer::Canvas
            && let Some(frames) = candidate.blend_admission_frames()?
        {
            // At retarget, an entering mark may have zero presence. Admit
            // both poses of the resident union with presence armed, including
            // authored Property enter/exit geometries. Actual frames are
            // admitted separately because spring overshoot can grow the ROI.
            for frame in &frames {
                blends::admit(renderer, window, frame, modes, view)?;
            }
        }
        Ok(Some(candidate))
    }

    pub(super) fn accept_blended_animation(
        &mut self,
        window: &Window,
        request: Animation,
    ) -> Result<Vec<SurfaceEvent>> {
        // Recheck against the accepted frame after earlier queued operations.
        let candidate = self.admit_blended_animation(window, &request)?;
        let mut model = self.model.clone();
        let events = if let Some(motion) = &candidate {
            let changes = model.set_interaction_frame(
                &request.scene,
                request.interaction,
                motion.scene().clone(),
            )?;
            model.cancel_drag();
            changes
        } else {
            model.set_scene_with_interaction(request.scene, request.interaction)?
        };
        if self.motion.is_none() || candidate.is_none() {
            self.last_time = None;
        }
        if candidate.is_none() {
            self.contour_bindings = request.contours.unwrap_or_default();
        }
        self.blends = Rc::new(request.modes);
        self.motion = candidate;
        self.model = model;
        Ok(events)
    }
}
