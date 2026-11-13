//! Transactional domain relayout, with callbacks outside controller borrows.
use super::*;
type PlotLayout = Box<dyn FnMut(f64, f64) -> Result<ClipRect>>;

#[derive(Clone, Debug, PartialEq)]
pub struct BrowserViewportLayout {
    pub width: f64,
    pub height: f64,
    pub plot: ClipRect,
    pub domains: ResolvedDomains,
    /// Complete authored-order stops for axes using a piecewise mapping.
    pub piecewise_domains: PiecewiseViewportDomains,
    pub hidden: BTreeSet<String>,
}
pub(super) struct ViewportBinding {
    pub controller: DomainViewport,
    axes: ViewportSelection,
    insets: [f64; 4],
    plot_layout: Option<PlotLayout>,
    callback: Box<dyn FnMut(BrowserViewportLayout) -> Result<SceneInteraction>>,
}

impl BrowserChart {
    /// Relayout data and axes from accepted numeric domains, preserving fixed
    /// plot insets on resize. This takes precedence over on_resize while active.
    /// The returned Scene must use the request's dimensions and plot rectangle.
    pub fn on_viewport(
        &self,
        controller: DomainViewport,
        axes: ViewportSelection,
        mut layout: impl FnMut(BrowserViewportLayout) -> Result<Scene> + 'static,
    ) -> Result<()> {
        self.configure_viewport(controller, axes, None, move |request| {
            layout(request).map(Into::into)
        })
    }
    pub fn on_viewport_with_interaction(
        &self,
        controller: DomainViewport,
        axes: ViewportSelection,
        layout: impl FnMut(BrowserViewportLayout) -> Result<SceneInteraction> + 'static,
    ) -> Result<()> {
        self.configure_viewport(controller, axes, None, layout)
    }
    /// Resolve plot margins from current dimensions and application state on
    /// every relayout. Both callbacks run outside host/controller borrows.
    pub fn on_viewport_with_plot(
        &self,
        controller: DomainViewport,
        axes: ViewportSelection,
        plot: impl FnMut(f64, f64) -> Result<ClipRect> + 'static,
        mut layout: impl FnMut(BrowserViewportLayout) -> Result<Scene> + 'static,
    ) -> Result<()> {
        self.configure_viewport(controller, axes, Some(Box::new(plot)), move |request| {
            layout(request).map(Into::into)
        })
    }
    pub fn on_viewport_with_plot_and_interaction(
        &self,
        controller: DomainViewport,
        axes: ViewportSelection,
        plot: impl FnMut(f64, f64) -> Result<ClipRect> + 'static,
        layout: impl FnMut(BrowserViewportLayout) -> Result<SceneInteraction> + 'static,
    ) -> Result<()> {
        self.configure_viewport(controller, axes, Some(Box::new(plot)), layout)
    }
    fn configure_viewport(
        &self,
        controller: DomainViewport,
        axes: ViewportSelection,
        plot_layout: Option<PlotLayout>,
        layout: impl FnMut(BrowserViewportLayout) -> Result<SceneInteraction> + 'static,
    ) -> Result<()> {
        let engine = &self.engine;
        scene::ensure(engine.alive.get(), "browser chart is stopped")?;
        let (width, height) = {
            let state = engine.state.borrow();
            let model = &state.as_ref().unwrap().model;
            (model.scene().width, model.scene().height)
        };
        let p = controller.plot();
        let limits = controller.limits();
        scene::ensure(
            p.x >= 0.
                && p.y >= 0.
                && p.width > 0.
                && p.height > 0.
                && p.x + p.width <= width
                && p.y + p.height <= height,
            "browser viewport plot must fit scene",
        )?;
        scene::ensure(
            limits.bounded && limits.min_zoom >= 1. && limits.max_zoom <= 100.,
            "browser viewport requires bounded zoom extent within 1..=100",
        )?;
        scene::ensure(
            axes != ViewportSelection::Y
                || controller.original_axes().left.is_some()
                || controller.original_axes().right.is_some(),
            "viewport has no value axis",
        )?;
        engine.try_enqueue(Input::ViewportRefresh)?;
        engine
            .viewport_generation
            .set(engine.viewport_generation.get().wrapping_add(1));
        *engine.viewport.borrow_mut() = Some(ViewportBinding {
            controller,
            axes,
            insets: [p.x, p.y, width - p.x - p.width, height - p.y - p.height],
            plot_layout,
            callback: Box::new(layout),
        });
        engine.viewport_configured.set(true);
        Ok(())
    }
}
impl BrowserHandle {
    /// Navigate from the accepted editable selection, converting viewport
    /// pixels to plot-local pixels exactly once. An empty selection is a no-op.
    pub fn request_editable_brush_viewport(&self) -> Result<()> {
        let engine = self.engine()?;
        scene::ensure(
            engine.viewport_configured.get(),
            "no data viewport configured",
        )?;
        let request = engine
            .state
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("browser chart retired".into()))?
            .model
            .editable_brush_viewport_request()?;
        if let Some(request) = request {
            engine.try_enqueue(Input::ViewportRequest(request))?;
        }
        Ok(())
    }

    /// Queue axis-selective navigation. The controller, layout and published
    /// frame commit together on RAF; callback failures keep the accepted view.
    pub fn request_viewport(&self, request: ViewportRequest) -> Result<()> {
        let engine = self.engine()?;
        scene::ensure(
            engine.viewport_configured.get(),
            "no data viewport configured",
        )?;
        engine.try_enqueue(Input::ViewportRequest(request))
    }
    /// Copy the accepted controller without reconstructing domains from a
    /// rounded pixel transform. Also available inside layout callbacks.
    pub fn viewport_controller(&self) -> Result<Option<DomainViewport>> {
        let engine = self.engine()?;
        let controller = engine.viewport_controller.borrow().clone();
        Ok(controller)
    }
    /// Recompute with current application data without resetting the view.
    pub fn refresh_viewport(&self) -> Result<()> {
        let engine = self.engine()?;
        scene::ensure(
            engine.viewport_configured.get(),
            "no data viewport configured",
        )?;
        engine.try_enqueue(Input::ViewportRefresh)
    }
}
impl Engine {
    pub(super) fn relayout_viewport(
        &self,
        proposed: Option<GestureState>,
        size: Option<(f64, f64)>,
    ) -> Result<Option<Vec<SurfaceEvent>>> {
        self.relayout_viewport_request(proposed, size, None)
    }
    pub(super) fn relayout_viewport_request(
        &self,
        proposed: Option<GestureState>,
        size: Option<(f64, f64)>,
        request: Option<ViewportRequest>,
    ) -> Result<Option<Vec<SurfaceEvent>>> {
        let generation = self.viewport_generation.get();
        let Some(mut binding) = self.viewport.borrow_mut().take() else {
            return Ok(None);
        };
        let result = (|| {
            let mut candidate = binding.controller.clone();
            let mut state = proposed
                .clone()
                .unwrap_or_else(|| self.state.borrow().as_ref().unwrap().gesture());
            let (width, height) =
                size.unwrap_or((state.model.scene().width, state.model.scene().height));
            let target_plot = if let Some(plot) = &mut binding.plot_layout {
                let outcome = plot(width, height);
                if !self.alive.get() || self.viewport_generation.get() != generation {
                    return Ok(None);
                }
                let plot = outcome?;
                scene::ensure(
                    plot.x >= 0.
                        && plot.y >= 0.
                        && plot.x + plot.width <= width
                        && plot.y + plot.height <= height,
                    "dynamic viewport plot must fit scene",
                )?;
                Some(plot)
            } else if size.is_some() {
                let [left, top, right, bottom] = binding.insets;
                Some(ClipRect {
                    x: left,
                    y: top,
                    width: width - left - right,
                    height: height - top - bottom,
                })
            } else {
                None
            };
            if let Some(request) = request {
                candidate.apply(request)?;
            } else if let Some(input) = &proposed {
                let gesture = {
                    let previous = self.state.borrow();
                    input
                        .model
                        .relayout_gesture(&previous.as_ref().unwrap().model)?
                };
                candidate.apply_gesture(gesture, binding.axes)?;
            }
            // Inputs are expressed in the currently accepted plot's local
            // pixels. A responsive margin change must preserve that requested
            // data range rather than reinterpret the old pointer in new pixels.
            if let Some(plot) = target_plot {
                candidate.resize(plot)?;
            }
            let layout_request = BrowserViewportLayout {
                width,
                height,
                plot: candidate.plot(),
                domains: candidate.domains(),
                piecewise_domains: candidate.piecewise_domains()?,
                hidden: state.model.hidden().clone(),
            };
            // A scalar virtual gesture baseline is sufficient to describe the
            // next relative wheel/pan/pinch. Actual X/Y windows live only in the
            // controller and painting stays in relayout coordinates.
            let transform = match candidate.transform_for(binding.axes) {
                Ok(transform) => transform,
                Err(_) => {
                    let (kx, ky) = candidate.zoom();
                    let k = kx.min(ky);
                    let w = candidate.normalized_windows();
                    ZoomTransform::new(
                        k,
                        -w.x[0] * candidate.plot().width * k,
                        -w.y[0] * candidate.plot().height * k,
                    )?
                }
            };
            let outcome = (binding.callback)(layout_request);
            if !self.alive.get() || self.viewport_generation.get() != generation {
                return Ok(None);
            }
            let SceneInteraction {
                scene,
                interaction,
                plot,
            } = outcome?;
            scene::ensure(
                plot.is_none_or(|p| p == candidate.plot()),
                "typed viewport layout plot differs from requested plot",
            )?;
            self.state
                .borrow()
                .as_ref()
                .unwrap()
                .admit_interaction_scene(&scene, interaction.as_ref())?;
            self.state.borrow().as_ref().unwrap().admit_scene(&scene)?;
            let modes = state.blends.for_scene(&scene);
            blends::admit(
                self.state.borrow().as_ref().unwrap().options.renderer,
                &self.window,
                &scene,
                &modes,
                ZoomTransform::default(),
            )?;
            scene::ensure(
                scene.width == width && scene.height == height,
                "viewport callback must preserve requested scene dimensions",
            )?;
            let old_plot = state.model.plot();
            if request == Some(ViewportRequest::Reset) {
                state.model.apply(SurfaceInput::Reset)?;
            }
            let preserve_gestures = proposed.is_some()
                && size.is_none()
                && request.is_none()
                && old_plot == candidate.plot();
            let changes = state.model.relayout_with_interaction(
                scene,
                candidate.plot(),
                transform,
                // Virtual input transforms must allow either direction even
                // when just one data axis has reached its limit. The controller
                // independently applies actual limits, including UTC 1 ms.
                [0.000001, 1_000_000.],
                preserve_gestures,
                interaction,
            )?;
            state.motion = None;
            state.contour_bindings.clear();
            state.last_time = None;
            self.state
                .borrow_mut()
                .as_mut()
                .unwrap()
                .restore_gesture(state);
            if !preserve_gestures {
                let surface = self.state.borrow().as_ref().unwrap().surface.clone();
                self.cancel_captures(&surface);
            }
            binding.controller = candidate;
            self.viewport_domains
                .set(Some(binding.controller.domains()));
            *self.viewport_controller.borrow_mut() = Some(binding.controller.clone());
            Ok(Some(changes))
        })();
        if self.alive.get() && self.viewport_generation.get() == generation {
            *self.viewport.borrow_mut() = Some(binding);
        }
        result
    }
}
