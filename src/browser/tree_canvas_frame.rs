//! Atomic transport of already-sealed native Tree pixels. User renderers never
//! run in this module or while Engine State is borrowed.
use super::*;
use crate::widget::plot_widgets::PlotWidgetsPacket;

pub(super) type Repaint =
    Rc<dyn Fn(&TreeCanvasFrame, TreeCanvasOptions) -> Result<PreparedTreeCanvas>>;
use crate::widget::plot_widgets::composition::CompositionStage;

pub(super) struct Candidate {
    pub layout: SceneInteraction,
    pub bitmap: PreparedTreeCanvas,
    pub composition: Option<(Rc<PlotWidgetsPacket>, Rc<CompositionStage>)>,
}

pub(super) fn policy(options: &BrowserOptions, packet: Option<&PlotWidgetsPacket>) -> Result<()> {
    scene::ensure(
        options.renderer == BrowserRenderer::Canvas,
        "raw Tree Canvas requires Canvas renderer",
    )?;
    scene::ensure(
        options.highlight == BrowserHighlight::Controlled && options.crosshair.is_none(),
        "raw Tree Canvas requires controlled highlighting without a source crosshair",
    )?;
    if let Some(packet) = packet {
        scene::ensure(
            packet.is_composition()
                && !packet.is_source_ordered()
                && packet.replace_ink().is_empty(),
            "raw Tree Canvas requires strict top-overlay components without source ink replacement",
        )?;
    }
    Ok(())
}
pub(super) fn validate_bitmap(scene: &Scene, bitmap: &PreparedTreeCanvas) -> Result<()> {
    scene::ensure(
        scene == &bitmap.frame().layout().scene,
        "raw Tree Canvas bitmap differs from candidate Scene",
    )?;
    let expected =
        [scene.width, scene.height].map(|v| (v * bitmap.options().pixel_ratio).ceil() as u32);
    scene::ensure(
        bitmap.backing_size() == expected,
        "raw Tree Canvas bitmap dimensions differ",
    )
}
impl BrowserChart {
    pub(super) fn mount_tree_canvas(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        bitmap: PreparedTreeCanvas,
        mount: Option<plot_widgets::Mount>,
    ) -> Result<Self> {
        policy(&options, mount.as_ref().map(|m| &m.layout))?;
        validate_bitmap(&scene, &bitmap)?;
        Self::mount_core(
            root,
            scene,
            options,
            MarkBlends::default(),
            mount,
            Some(bitmap),
        )
    }
}
impl Engine {
    pub(super) fn set_tree_canvas_repaint(&self, callback: Repaint) {
        *self.tree_canvas_repaint.borrow_mut() = Some(callback);
    }
    /// Temporarily expose only the accepted model while application renderers
    /// prepare pixels for a native input's candidate view/hidden/DPR.
    pub(super) fn repaint_tree_canvas_if_needed(
        &self,
        baseline: Option<&(GestureState, Option<String>)>,
    ) -> Result<()> {
        let work = {
            let state = self.state.borrow();
            let Some(s) = state.as_ref() else {
                return Ok(());
            };
            let Some(bitmap) = &s.tree_canvas else {
                return Ok(());
            };
            let mut options = bitmap.options().clone();
            options.view = s.model.paint_transform();
            options.pixel_ratio = blends::ratio(self.window.device_pixel_ratio(), s.model.scene());
            options.hidden = s.model.hidden().clone();
            if options.view == bitmap.options().view
                && options.pixel_ratio == bitmap.options().pixel_ratio
                && options.hidden == bitmap.options().hidden
            {
                return Ok(());
            }
            // Accepted raw and visible primary backing remain live while the
            // pending/seal pair is prepared. Both allocations are reserved.
            let pixels = u64::from(bitmap.backing_size()[0]) * u64::from(bitmap.backing_size()[1]);
            options.reserved_pixels = pixels.saturating_mul(2);
            (
                s.gesture(),
                s.focused.clone(),
                bitmap.frame().clone(),
                options,
            )
        };
        let producer =
            self.tree_canvas_repaint.borrow().clone().ok_or_else(|| {
                ChartError("raw Tree Canvas repaint producer is unavailable".into())
            })?;
        let (candidate, focused, frame, options) = work;
        let (accepted, accepted_focus) = baseline
            .ok_or_else(|| ChartError("raw Tree Canvas repaint checkpoint is missing".into()))?;
        {
            let mut state = self.state.borrow_mut();
            let s = state
                .as_mut()
                .ok_or_else(|| ChartError("browser chart stopped".into()))?;
            s.restore_gesture(accepted.clone());
            s.focused = accepted_focus.clone();
        }
        let result = producer(&frame, options.clone());
        scene::ensure(self.alive.get(), "raw Tree Canvas stopped during repaint")?;
        let mut bitmap = result?;
        scene::ensure(
            bitmap.frame() == &frame
                && bitmap.options().view == options.view
                && bitmap.options().pixel_ratio == options.pixel_ratio
                && bitmap.options().hidden == options.hidden,
            "raw Tree Canvas repaint producer returned another candidate",
        )?;
        validate_bitmap(candidate.model.scene(), &bitmap)?;
        bitmap.inherit_admission(
            candidate
                .tree_canvas
                .as_ref()
                .expect("candidate raw bitmap"),
        );
        let mut state = self.state.borrow_mut();
        let s = state
            .as_mut()
            .ok_or_else(|| ChartError("browser chart stopped".into()))?;
        s.restore_gesture(candidate);
        s.focused = focused;
        s.tree_canvas = Some(bitmap);
        Ok(())
    }
    pub(super) fn raw_bitmap_matches(&self, bitmap: &PreparedTreeCanvas) -> bool {
        self.state
            .try_borrow()
            .ok()
            .and_then(|s| {
                s.as_ref()
                    .and_then(|s| s.tree_canvas.as_ref().map(|old| old.same_admission(bitmap)))
            })
            .unwrap_or(false)
    }
    pub(super) fn tree_canvas_bitmap(&self) -> Option<PreparedTreeCanvas> {
        self.state
            .try_borrow()
            .ok()
            .and_then(|s| s.as_ref().and_then(|s| s.tree_canvas.clone()))
    }
}
impl State {
    pub(super) fn process_tree_canvas(
        &mut self,
        candidate: Candidate,
    ) -> Result<Vec<SurfaceEvent>> {
        let Candidate {
            layout,
            bitmap,
            composition,
        } = candidate;
        policy(&self.options, composition.as_ref().map(|c| c.0.as_ref()))?;
        validate_bitmap(&layout.scene, &bitmap)?;
        scene::ensure(
            self.motion.is_none() && self.current_blends().is_empty() && self.blends.is_empty(),
            "raw Tree Canvas does not support animation or blends",
        )?;
        let (model, changes) = if let Some((packet, stage)) = &composition {
            stage.validate()?;
            self.plot_widget_layout_candidate(layout, packet)?
        } else {
            self.admit_interaction_scene(&layout.scene, layout.interaction.as_ref())?;
            let mut model = self.model.clone();
            let changes = model.set_layout(layout)?;
            (model, changes)
        };
        // All fallible model/slot checks precede the accepted State mutation.
        self.model = model;
        self.tree_canvas = Some(bitmap);
        if let Some((packet, stage)) = composition {
            self.plot_widgets = Some(packet);
            self.staged_widgets = Some(stage);
        }
        self.blends = Rc::default();
        self.motion = None;
        self.last_time = None;
        self.contour_bindings.clear();
        Ok(changes)
    }
    pub(super) fn validate_tree_canvas_paint(&self, dpr: f64) -> Result<()> {
        if let Some(raw) = &self.tree_canvas {
            policy(&self.options, self.plot_widgets.as_deref())?;
            validate_bitmap(self.model.scene(), raw)?;
            scene::ensure(
                self.model.paint_transform() == raw.options().view
                    && blends::ratio(dpr, self.model.scene()) == raw.options().pixel_ratio
                    && self.model.hidden() == &raw.options().hidden,
                "raw Tree Canvas requires a fresh bitmap for view, DPR or hidden changes",
            )?;
        }
        Ok(())
    }
}
