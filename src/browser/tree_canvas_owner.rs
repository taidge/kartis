//! Retained native renderers for the Tree owner. Painting runs outside Engine
//! state borrows and produces a sealed bitmap before publication.
use super::*;

impl BrowserTree {
    /// Mount a native shared-context Canvas program with automatic Tree
    /// activity. Labels are computed once for every source node, including
    /// labels excluded from the ordinary paint layer.
    pub fn mount_with_canvas(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeCanvasProgram,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(default_canvas_layout),
            None,
            Some(program),
            None,
        )
    }
    /// The builder must use an authoritative `tree_canvas_layout_*` entry.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_and_canvas(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        program: BrowserTreeCanvasProgram,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(build),
            None,
            Some(program),
            None,
        )
    }
    /// Canvas callbacks and retained top-overlay components share one accepted
    /// candidate. Source-order interleaving and ink replacement reject before
    /// any Canvas callback runs.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_canvas_and_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        program: BrowserTreeCanvasProgram,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(default_canvas_layout),
            Some(ComponentMount::components(widgets, placement, factory)),
            Some(program),
            None,
        )
    }
    /// Combine an authoritative activity builder, native renderers and typed
    /// overlay components without replaying the borrowed layout factories.
    #[allow(clippy::too_many_arguments)]
    pub fn mount_with_factory_canvas_and_components<W: glory::Widget + 'static>(
        root: Element,
        source: Vec<HierarchyDatum>,
        config: ChartConfig,
        options: BrowserTreeOptions,
        build: impl FnMut(&BrowserTreeBuildContext<'_>) -> Result<TreeActivityLayout> + 'static,
        program: BrowserTreeCanvasProgram,
        widgets: BrowserTreeWidgetOptions,
        placement: impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>>
        + 'static,
        factory: impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<W>
        + 'static,
    ) -> Result<Self> {
        Self::mount_program(
            root,
            source,
            config,
            options,
            Box::new(build),
            Some(ComponentMount::components(widgets, placement, factory)),
            Some(program),
            None,
        )
    }
}

impl BrowserTreeHandle {
    /// Replace retained callbacks/layers. Even an unchanged Scene receives a
    /// fresh Canvas candidate; a failed paint preserves the accepted bitmap.
    pub fn set_canvas_program(&self, program: BrowserTreeCanvasProgram) -> Result<()> {
        let driver = self.driver()?;
        let slot = driver
            .canvas
            .as_ref()
            .ok_or_else(|| ChartError("Tree was mounted without a native Canvas program".into()))?;
        let accepted = driver.accepted.borrow();
        let frame = TreeCanvasFrame::from_all_computed_activity(
            accepted.request.config.clone(),
            accepted.layout.clone(),
        )?;
        program.admit(&frame)?;
        drop(accepted);
        driver.request()?;
        *slot.borrow_mut() = Some(program);
        driver.canvas_dirty.set(true);
        Ok(())
    }
    /// Export the accepted callback bitmap without reevaluating renderers.
    /// Retained HTML components are outside the exported native Canvas.
    pub fn canvas_png_data_url(&self) -> Result<String> {
        let driver = self
            .driver
            .upgrade()
            .ok_or_else(|| ChartError("Tree owner was dropped".into()))?;
        let core = driver.core()?;
        core.tree_canvas_bitmap()
            .ok_or_else(|| ChartError("Tree has no accepted native Canvas bitmap".into()))?
            .to_png_data_url()
    }
}

/// Stateful, application-owned native Canvas renderers and literal layers.
/// Custom paint changes pixels; picking continues to use the Tree source
/// model (or its node-center mesh), rather than arbitrary callback geometry.
pub struct BrowserTreeCanvasProgram {
    pub layers: Vec<TreeCanvasLayer>,
    pub budget: TreeCanvasBudget,
    pub mesh: TreeCanvasMeshOptions,
    pub render_node: Option<Box<TreeCanvasNodeRenderer<'static>>>,
    pub render_link: Option<Box<TreeCanvasLinkRenderer<'static>>>,
    pub render_label: Option<Box<TreeCanvasLabelRenderer<'static>>>,
    pub custom_layers: BTreeMap<String, Box<TreeCanvasLayerRenderer<'static>>>,
}
impl Default for BrowserTreeCanvasProgram {
    fn default() -> Self {
        Self {
            layers: vec![
                TreeCanvasLayer::Links,
                TreeCanvasLayer::Nodes,
                TreeCanvasLayer::Labels,
                TreeCanvasLayer::Mesh,
            ],
            budget: TreeCanvasBudget::default(),
            mesh: TreeCanvasMeshOptions::default(),
            render_node: None,
            render_link: None,
            render_label: None,
            custom_layers: BTreeMap::new(),
        }
    }
}
impl BrowserTreeCanvasProgram {
    fn admit(&self, frame: &TreeCanvasFrame) -> Result<TreeCanvasPlan> {
        let plan = TreeCanvasPlan::new(frame, self.layers.clone(), self.budget)?;
        for layer in plan.layers() {
            if let TreeCanvasLayer::Custom(key) = &layer.layer {
                scene::ensure(
                    self.custom_layers.contains_key(key),
                    "Tree Canvas custom layer callback is missing",
                )?;
            }
        }
        Ok(plan)
    }
    pub(super) fn prepare(
        &mut self,
        document: &web_sys::Document,
        frame: &TreeCanvasFrame,
        mut options: TreeCanvasOptions,
        checkpoint: &mut dyn FnMut() -> Result<()>,
    ) -> Result<PreparedTreeCanvas> {
        let plan = self.admit(frame)?;
        options.mesh = self.mesh;
        let node_present = self.render_node.is_some();
        let link_present = self.render_link.is_some();
        let label_present = self.render_label.is_some();
        // Short-lived adapters keep the retained 'static closures private and
        // lend them only for this paint's fresh domain/context references.
        let mut node = |ctx: &web_sys::CanvasRenderingContext2d,
                        data: &TreeCanvasNodeContext<'_>| {
            self.render_node.as_mut().expect("present node renderer")(ctx, data)
        };
        let mut link = |ctx: &web_sys::CanvasRenderingContext2d,
                        data: &mut TreeCanvasLinkContext<'_>| {
            self.render_link.as_mut().expect("present link renderer")(ctx, data)
        };
        let mut label = |ctx: &web_sys::CanvasRenderingContext2d,
                         data: &TreeCanvasLabelContext<'_>| {
            self.render_label.as_mut().expect("present label renderer")(ctx, data)
        };
        let mut custom: Vec<_> = self
            .custom_layers
            .iter_mut()
            .map(|(key, callback)| {
                (
                    key.clone(),
                    move |ctx: &web_sys::CanvasRenderingContext2d,
                          data: &mut TreeCanvasLayerContext<'_>| {
                        callback(ctx, data)
                    },
                )
            })
            .collect();
        let mut callbacks = TreeCanvasCallbacks {
            render_node: Some(&mut node),
            render_link: Some(&mut link),
            render_label: Some(&mut label),
            custom_layers: custom
                .iter_mut()
                .map(|(key, callback)| (key.clone(), callback as &mut TreeCanvasLayerRenderer<'_>))
                .collect(),
            checkpoint: Some(checkpoint),
        };
        // Empty renderer slots select the rich Scene fallback, rather than a
        // no-op closure that would suppress the builtin role's paint.
        if !node_present {
            callbacks.render_node = None;
        }
        if !link_present {
            callbacks.render_link = None;
        }
        if !label_present {
            callbacks.render_label = None;
        }
        prepare_tree_canvas(document, frame, &plan, &options, &mut callbacks)
    }
}

impl Driver {
    pub(super) fn prepare_canvas_candidate(
        &self,
        candidate: &Accepted,
    ) -> Result<Option<PreparedTreeCanvas>> {
        if self.canvas.is_none() {
            return Ok(None);
        }
        let frame = TreeCanvasFrame::from_all_computed_activity(
            candidate.request.config.clone(),
            candidate.layout.clone(),
        )?;
        let core = self.core()?;
        let (model, reserved_pixels) = {
            let state = core.state.borrow();
            let state = state
                .as_ref()
                .ok_or_else(|| ChartError("Tree chart is stopped".into()))?;
            let pixels = state
                .canvas
                .as_ref()
                .map_or(0, |c| u64::from(c.width()) * u64::from(c.height()));
            let mut model = state.model.clone();
            model.set_layout(tree_activity_frame(&candidate.layout)?)?;
            (model, pixels.saturating_mul(2))
        };
        let options = TreeCanvasOptions {
            pixel_ratio: blends::ratio(
                self.window.device_pixel_ratio(),
                &candidate.layout.layout.scene,
            ),
            view: model.paint_transform(),
            hidden: model.hidden().clone(),
            reserved_pixels,
            ..Default::default()
        };
        self.repaint_canvas(&frame, options).map(Some)
    }
    pub(super) fn repaint_canvas(
        &self,
        frame: &TreeCanvasFrame,
        options: TreeCanvasOptions,
    ) -> Result<PreparedTreeCanvas> {
        let slot = self
            .canvas
            .as_ref()
            .ok_or_else(|| ChartError("Tree has no native Canvas program".into()))?;
        let core = self.core()?;
        let document = self
            .surface
            .owner_document()
            .ok_or_else(|| ChartError("Tree surface has no document".into()))?;
        let mut program = slot
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("Tree Canvas program is already in use".into()))?;
        let previous_painting = self.painting.replace(true);
        let result = program.prepare(&document, frame, options, &mut || {
            scene::ensure(
                self.alive.get() && core.alive.get(),
                "Tree stopped during native Canvas rendering",
            )
        });
        self.painting.set(previous_painting);
        if self.alive.get() {
            *slot.borrow_mut() = Some(program);
        }
        result
    }
}

pub(super) fn default_canvas_layout(
    context: &BrowserTreeBuildContext<'_>,
) -> Result<TreeActivityLayout> {
    tree_canvas_layout_with_activity(
        context.config,
        context.source,
        context.tree,
        &mut TreeFactories::default(),
        context.activity,
        &mut TreeActivityFactories::default(),
    )
}

pub(super) fn admit_canvas_policy(
    options: &BrowserOptions,
    components: Option<&BrowserTreeWidgetOptions>,
) -> Result<()> {
    scene::ensure(
        options.renderer == BrowserRenderer::Canvas,
        "Tree native Canvas renderers require the Canvas renderer",
    )?;
    scene::ensure(
        options.highlight == BrowserHighlight::Controlled && options.crosshair.is_none(),
        "Tree native Canvas renderers require controlled highlighting without a source crosshair",
    )?;
    options.interaction.validate()?;
    options.legend_options.validate()?;
    options.tooltip_options.validate()?;
    if let Some(layout) = &options.tooltip_layout {
        layout.validate()?;
    }
    match &options.animation {
        BrowserAnimation::Properties(options)
        | BrowserAnimation::PropertiesWithContours { options, .. } => options.validate()?,
        BrowserAnimation::Scene => {}
    }
    scene::ensure(
        options
            .height
            .is_none_or(|height| height.is_finite() && (1. ..=16384.).contains(&height)),
        "invalid browser surface height",
    )?;
    scene::ensure(
        components.is_none_or(|w| !w.source_ordered && !w.replace_ink),
        "Tree native Canvas renderers require top-overlay components with retained source ink",
    )
}
