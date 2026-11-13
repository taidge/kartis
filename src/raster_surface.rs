//! Owner-scoped CPU raster plot/controller for explicit native host integration.
//! This has no window, clock, filesystem, font lookup or implicit image loader.
//! Hosts supply pointer rectangles in the same coordinate system as their input,
//! then call `dispatch_host`; Glory data/legend controls bridge automatically.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::{Rc, Weak};

use glory::style::{Length, Position, Style};
use glory::{Cage, Scope, Widget};

use crate::render::raster::{FontRegistry, RasterOptions, RenderedImage, render_raster};
use crate::surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
use crate::*;
mod editable_brush;
mod geo_coordinates;
mod plot_pointer;
pub use editable_brush::RasterEditableBrushSnapshot;

#[derive(Clone, Debug, PartialEq)]
pub struct RasterSurfaceOptions {
    pub interaction: SurfaceOptions,
    pub raster: RasterOptions,
    pub legend: LegendOptions,
    /// Enable legend action controls at mount. Default categories are static.
    /// on_legend_activate subscribes without changing this explicit opt-in.
    pub legend_actions: bool,
    pub tooltip: TooltipOptions,
    pub highlight: bool,
    /// Persistent viewport-pixel editing, independent of legacy Shift-drag.
    pub editable_brush: Option<EditableBrushOptions>,
}
impl Default for RasterSurfaceOptions {
    fn default() -> Self {
        Self {
            interaction: SurfaceOptions::default(),
            raster: RasterOptions::default(),
            legend: LegendOptions::default(),
            legend_actions: false,
            tooltip: TooltipOptions::default(),
            highlight: true,
            editable_brush: None,
        }
    }
}
impl RasterSurfaceOptions {
    pub fn validate(&self) -> Result<()> {
        self.interaction.validate()?;
        self.raster.validate()?;
        self.legend.validate()?;
        self.tooltip.validate()?;
        if let Some(brush) = self.editable_brush {
            brush.validate()?;
        }
        scene::ensure(
            self.raster.view == ZoomTransform::default(),
            "RasterSurface manages its own view transform",
        )
    }
}
/// Host input and this rectangle must use the same units and origin. An image
/// drawn at an offset or under DPI scaling is mapped to logical scene pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RasterSurfaceBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}
impl RasterSurfaceBounds {
    fn point(self, p: (f64, f64), scene: &Scene) -> Result<(f64, f64)> {
        scene::ensure(
            [self.x, self.y, self.width, self.height, p.0, p.1]
                .into_iter()
                .all(f64::is_finite)
                && self.width > 0.
                && self.height > 0.,
            "invalid raster host input bounds",
        )?;
        let point = (
            (p.0 - self.x) / self.width * scene.width,
            (p.1 - self.y) / self.height * scene.height,
        );
        scene::ensure(
            [point.0, point.1]
                .into_iter()
                .all(|v| v.is_finite() && v.abs() <= 1e9),
            "raster host pointer conversion overflow",
        )?;
        Ok(point)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RasterSurfaceSnapshot {
    pub revision: u64,
    pub width: f64,
    pub height: f64,
    pub selected: Option<String>,
    pub hovered: Vec<String>,
    pub hidden: BTreeSet<String>,
    pub view: ZoomTransform,
    pub touches: usize,
    pub dragging: bool,
    pub editable_brush: Option<RasterEditableBrushSnapshot>,
}
type Publisher = Box<dyn FnMut(&RenderedImage) -> Result<String>>;
type Observer = Box<dyn FnMut(Vec<SurfaceEvent>)>;
type LegendObserver = Box<dyn FnMut(LegendActivation)>;
type ErrorObserver = Box<dyn FnMut(ChartError)>;
#[derive(Clone, Debug, PartialEq)]
struct ImageFrame {
    source: String,
    width: f64,
    height: f64,
}
struct Data {
    model: SurfaceModel,
    options: RasterSurfaceOptions,
    fonts: FontRegistry,
    image: Rc<RenderedImage>,
    visual: Scene,
    revision: u64,
}
struct Engine {
    alive: Cell<bool>,
    scope: Scope,
    ui_scope: RefCell<Option<Scope>>,
    built: Cell<bool>,
    data: RefCell<Option<Data>>,
    scene: Cage<Scene>,
    data_labels: Cage<BTreeMap<String, String>>,
    data_pointer: Cage<bool>,
    metadata_version: Cage<u64>,
    hidden: Cage<BTreeSet<String>>,
    selected: Cage<Option<String>>,
    hovered: Cage<Option<String>>,
    tooltip: Cage<Vec<String>>,
    frame: Cage<ImageFrame>,
    brush_frame: Cage<Option<RasterEditableBrushSnapshot>>,
    publisher: Rc<RefCell<Option<Publisher>>>,
    observer: RefCell<Option<Observer>>,
    legend_observer: RefCell<Option<LegendObserver>>,
    legend_generation: Cell<u64>,
    errors: RefCell<Option<ErrorObserver>>,
}
/// Retain this guard for the mounted lifetime. Drop/stop retire the image,
/// controls, callbacks and model. Weak handles never keep a document alive.
#[must_use = "retain RasterSurface while its image and controls are mounted"]
pub struct RasterSurface {
    engine: Rc<Engine>,
}
#[derive(Clone)]
pub struct RasterSurfaceHandle {
    engine: Weak<Engine>,
}
pub struct RasterSurfaceWidget {
    engine: Weak<Engine>,
}
struct MountAdmission {
    scope: Scope,
    armed: bool,
}
impl Drop for MountAdmission {
    fn drop(&mut self) {
        if self.armed {
            self.scope.owner().dispose();
        }
    }
}
impl RasterSurface {
    pub fn mount(
        scope: &Scope,
        scene: Scene,
        fonts: FontRegistry,
        options: RasterSurfaceOptions,
    ) -> Result<Self> {
        Self::mount_published(scope, scene, fonts, options, |image| image.data_url())
    }
    /// Explicit native provider integration. It receives complete PNG-capable
    /// RGBA frames and returns an application-owned image source. The publisher
    /// must bound its own encoded-byte/cache lifetime and perform no nested UI
    /// flush. Async host image decode errors are separate host observations.
    pub fn mount_published(
        scope: &Scope,
        scene: Scene,
        fonts: FontRegistry,
        options: RasterSurfaceOptions,
        publish: impl FnMut(&RenderedImage) -> Result<String> + 'static,
    ) -> Result<Self> {
        Self::mount_layout_published(scope, scene.into(), fonts, options, publish)
    }
    /// Mount a layout together with its original values, nullable samples and
    /// explicit interaction anchors. The layout's plot takes precedence over
    /// the interaction options when supplied.
    pub fn mount_layout(
        scope: &Scope,
        layout: SceneInteraction,
        fonts: FontRegistry,
        options: RasterSurfaceOptions,
    ) -> Result<Self> {
        Self::mount_layout_published(scope, layout, fonts, options, |image| image.data_url())
    }
    /// Publish a typed layout through an application-owned image provider.
    /// Layout validation and initial painting finish before publication.
    pub fn mount_layout_published(
        scope: &Scope,
        layout: SceneInteraction,
        fonts: FontRegistry,
        mut options: RasterSurfaceOptions,
        publish: impl FnMut(&RenderedImage) -> Result<String> + 'static,
    ) -> Result<Self> {
        layout.validate()?;
        if let Some(plot) = layout.plot {
            options.interaction.plot = Some(plot);
        }
        let scene = layout.scene;
        options.validate()?;
        scene.validate()?;
        for item in &scene.legend {
            legend_label(item, &options.legend)?;
        }
        let scope = scope.child().map_err(error)?;
        let mut admission = MountAdmission {
            scope: scope.clone(),
            armed: true,
        };
        let publisher: Rc<RefCell<Option<Publisher>>> =
            Rc::new(RefCell::new(Some(Box::new(publish))));
        let cleanup = publisher.clone();
        scope
            .on_cleanup(move || {
                cleanup.borrow_mut().take();
            })
            .map_err(error)?;
        let mut model = SurfaceModel::new_with_interaction(
            scene.clone(),
            options.interaction.clone(),
            layout.interaction,
        )?;
        plot_pointer::admit(&scope, &model, &options)?;
        model.configure_editable_brush(options.editable_brush, BrushReconfigure::Preserve)?;
        let visual = visual(&model, options.highlight)?;
        let image = Rc::new(render_raster(&visual, &fonts, &options.raster)?);
        let mut callback = publisher
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("raster publisher retired".into()))?;
        let source = callback(&image)?;
        scope.owner().check_write().map_err(error)?;
        *publisher.borrow_mut() = Some(callback);
        validate_source(&source)?;
        let engine = Rc::new(Engine {
            alive: Cell::new(true),
            ui_scope: RefCell::new(None),
            built: Cell::new(false),
            scene: scope.cage(model.controls_scene()),
            data_labels: scope.cage(model.data_control_labels()),
            data_pointer: scope.cage(!plot_pointer::strict(&model)),
            metadata_version: scope.cage(0),
            hidden: scope.cage(BTreeSet::new()),
            selected: scope.cage(None),
            hovered: scope.cage(None),
            tooltip: scope.cage(Vec::new()),
            frame: scope.cage(ImageFrame {
                source,
                width: scene.width,
                height: scene.height,
            }),
            brush_frame: scope.cage(editable_brush::snapshot(&model)?),
            data: RefCell::new(Some(Data {
                model,
                options,
                fonts,
                image,
                visual,
                revision: 0,
            })),
            publisher,
            observer: RefCell::new(None),
            legend_observer: RefCell::new(None),
            legend_generation: Cell::new(0),
            errors: RefCell::new(None),
            scope,
        });
        let weak = Rc::downgrade(&engine);
        engine
            .scope
            .on_cleanup(move || {
                if let Some(engine) = weak.upgrade() {
                    engine.stop();
                }
            })
            .map_err(error)?;
        admission.armed = false;
        Ok(Self { engine })
    }
    pub fn handle(&self) -> RasterSurfaceHandle {
        RasterSurfaceHandle {
            engine: Rc::downgrade(&self.engine),
        }
    }
    /// Mount once into the same runtime. The image and normal Glory controls
    /// are owned by a separate child scope and disappear when this guard stops.
    pub fn widget(&self) -> RasterSurfaceWidget {
        RasterSurfaceWidget {
            engine: Rc::downgrade(&self.engine),
        }
    }
    pub fn on_event(&self, callback: impl FnMut(Vec<SurfaceEvent>) + 'static) -> Result<()> {
        self.engine.check()?;
        *self.engine.observer.borrow_mut() = Some(Box::new(callback));
        Ok(())
    }
    /// Subscribe to explicitly enabled legend actions. Configure
    /// RasterSurfaceOptions::legend_actions before mount; installing a callback
    /// alone does not turn static categories into buttons.
    pub fn on_legend_activate(
        &self,
        callback: impl FnMut(LegendActivation) + 'static,
    ) -> Result<()> {
        self.handle().on_legend_activate(callback)
    }
    pub fn stop(&self) {
        self.engine.stop();
    }
    /// Receives rejected proposals originating from the mounted Glory controls.
    /// Explicit handle operations return their errors directly to the caller.
    pub fn on_error(&self, callback: impl FnMut(ChartError) + 'static) -> Result<()> {
        self.engine.check()?;
        *self.engine.errors.borrow_mut() = Some(Box::new(callback));
        Ok(())
    }
}
impl Drop for RasterSurface {
    fn drop(&mut self) {
        self.stop();
    }
}
fn error(value: impl std::fmt::Display) -> ChartError {
    ChartError(value.to_string())
}
fn ui_error(_: ChartError) -> glory::ui::Error {
    glory::render::Error::InvalidProps("invalid or retired raster surface").into()
}
fn validate_source(source: &str) -> Result<()> {
    scene::ensure(
        !source.is_empty() && source.len() <= crate::raster_widget::MAX_RASTER_IMAGE_SOURCE_BYTES,
        "raster surface image source budget exceeded",
    )
}
fn visual(model: &SurfaceModel, highlight: bool) -> Result<Scene> {
    let mut scene = model.scene().without_series(model.hidden());
    if highlight {
        let active = model
            .hovered()
            .first()
            .map(String::as_str)
            .or(model.selected());
        if let Some(id) = active {
            for mark in &mut scene.marks {
                if !mark.label.is_empty() && mark.id != id {
                    mark.opacity *= 0.35;
                }
            }
        }
    }
    editable_brush::append_visual(&mut scene, model)?;
    Ok(scene)
}
impl Engine {
    fn check(&self) -> Result<()> {
        scene::ensure(
            self.alive.get() && self.scope.owner().is_alive(),
            "raster surface stopped",
        )
    }
    fn stop(&self) {
        if !self.alive.replace(false) {
            return;
        }
        let observer = self.observer.borrow_mut().take();
        let legend_observer = self.legend_observer.borrow_mut().take();
        let errors = self.errors.borrow_mut().take();
        let publisher = self.publisher.borrow_mut().take();
        let data = self.data.borrow_mut().take();
        let ui = self.ui_scope.borrow_mut().take();
        if let Some(ui) = ui {
            ui.owner().dispose();
        }
        self.scope.owner().dispose();
        drop((observer, legend_observer, errors, publisher, data));
    }
    fn activate_legend(&self, activation: LegendActivation) {
        if self.check().is_err() {
            return;
        }
        let current = {
            let data = self.data.borrow();
            data.as_ref().and_then(|data| {
                let scene = data.model.scene();
                if !data.options.legend_actions
                    || scene.legend_role(&activation.item.id) != Some(activation.role)
                    || scene.legend_target(&activation.item.id)
                        != Some(activation.resolved_target())
                {
                    return None;
                }
                scene
                    .legend
                    .iter()
                    .find(|item| item.id == activation.item.id)
                    .map(|item| LegendActivation {
                        item: item.clone(),
                        role: activation.role,
                        target: scene.legend_targets.get(&item.id).cloned(),
                    })
            })
        };
        let Some(current) = current else {
            return;
        };
        let generation = self.legend_generation.get();
        let callback = self.legend_observer.borrow_mut().take();
        if let Some(mut callback) = callback {
            callback(current);
            if self.alive.get() && self.legend_generation.get() == generation {
                *self.legend_observer.borrow_mut() = Some(callback);
            }
        }
    }
    fn toggle_legend(&self, activation: LegendActivation) -> Result<Vec<SurfaceEvent>> {
        self.check()?;
        let target = activation.resolved_target();
        let current = {
            let data = self.data.borrow();
            data.as_ref().is_some_and(|data| {
                activation.role == LegendRole::Series
                    && data.model.scene().legend_target(&activation.item.id) == Some(target.clone())
            })
        };
        if !current {
            return Ok(vec![]);
        }
        let LegendTarget::Series(series) = target else {
            return Ok(vec![]);
        };
        self.update(|model, _| {
            let mut hidden = model.hidden().clone();
            if !hidden.remove(&series) {
                hidden.insert(series);
            }
            model.set_hidden(hidden)
        })
    }
    fn update(
        &self,
        mutate: impl FnOnce(&mut SurfaceModel, &mut RasterSurfaceOptions) -> Result<Vec<SurfaceEvent>>,
    ) -> Result<Vec<SurfaceEvent>> {
        self.check()?;
        let (mut model, mut options, fonts, old_image, old_visual, revision, old_raster) = {
            let data = self.data.borrow();
            let data = data
                .as_ref()
                .ok_or_else(|| ChartError("raster surface stopped".into()))?;
            (
                data.model.clone(),
                data.options.clone(),
                data.fonts.clone(),
                data.image.clone(),
                data.visual.clone(),
                data.revision,
                {
                    let mut raster = data.options.raster.clone();
                    raster.view = data.model.paint_transform();
                    raster
                },
            )
        };
        let events = mutate(&mut model, &mut options)?;
        options.editable_brush = model.editable_brush().map(|brush| *brush.options());
        options.validate()?;
        plot_pointer::admit(&self.scope, &model, &options)?;
        for item in &model.scene().legend {
            legend_label(item, &options.legend)?;
        }
        let visual = visual(&model, options.highlight)?;
        let mut raster = options.raster.clone();
        raster.view = model.paint_transform();
        let needs_render = visual != old_visual || raster != old_raster;
        let (image, source) = if needs_render {
            let image = Rc::new(render_raster(&visual, &fonts, &raster)?);
            let mut callback = self
                .publisher
                .borrow_mut()
                .take()
                .ok_or_else(|| ChartError("raster surface publisher busy or retired".into()))?;
            let result = callback(&image);
            if self.alive.get() {
                *self.publisher.borrow_mut() = Some(callback);
            }
            let source = result?;
            validate_source(&source)?;
            (image, Some(source))
        } else {
            (old_image, None)
        };
        self.check()?;
        scene::ensure(
            self.data
                .borrow()
                .as_ref()
                .is_some_and(|data| data.revision == revision),
            "raster surface changed during publication",
        )?;
        let revision = revision
            .checked_add(1)
            .ok_or_else(|| ChartError("raster surface revision exhausted".into()))?;
        let metadata_changed = {
            let data = self.data.borrow();
            let old = data
                .as_ref()
                .expect("checked live surface")
                .model
                .interaction_data();
            match (old, model.interaction_data()) {
                (Some(old), Some(next)) => !std::ptr::eq(old, next),
                (None, None) => false,
                _ => true,
            }
        };
        let brush_frame = editable_brush::snapshot(&model)?;
        self.scope
            .batch(|| {
                let enabled = !plot_pointer::strict(&model);
                if self.data_pointer.get_untracked() != enabled {
                    self.data_pointer.revise(|old| *old = enabled);
                }
                let scene = model.controls_scene();
                if self.scene.get_untracked() != scene {
                    self.scene.revise(|old| *old = scene);
                }
                let labels = model.data_control_labels();
                if self.data_labels.get_untracked() != labels {
                    self.data_labels.revise(|old| *old = labels);
                }
                if metadata_changed {
                    self.metadata_version.revise(|version| *version = revision);
                }
                let hidden = model.hidden().clone();
                if self.hidden.get_untracked() != hidden {
                    self.hidden.revise(|old| *old = hidden);
                }
                let selected = model.selected().map(str::to_owned);
                if self.selected.get_untracked() != selected {
                    self.selected.revise(|old| *old = selected);
                }
                let hovered = model.hovered().first().cloned();
                if self.hovered.get_untracked() != hovered {
                    self.hovered.revise(|old| *old = hovered);
                }
                let tooltip = model.hovered().to_vec();
                if self.tooltip.get_untracked() != tooltip {
                    self.tooltip.revise(|old| *old = tooltip);
                }
                if let Some(source) = source {
                    self.frame.revise(|old| {
                        *old = ImageFrame {
                            source,
                            width: model.scene().width,
                            height: model.scene().height,
                        }
                    });
                }
                if self.brush_frame.get_untracked() != brush_frame {
                    self.brush_frame.revise(|old| *old = brush_frame);
                }
                *self.data.borrow_mut() = Some(Data {
                    model,
                    options,
                    fonts,
                    image,
                    visual,
                    revision,
                });
            })
            .map_err(error)?;
        let callback = self.observer.borrow_mut().take();
        if let Some(mut callback) = callback {
            callback(events.clone());
            if self.alive.get() {
                *self.observer.borrow_mut() = Some(callback);
            }
        }
        Ok(events)
    }
}
impl RasterSurfaceHandle {
    fn report(&self, result: Result<Vec<SurfaceEvent>>) {
        if let Err(error) = result
            && let Ok(engine) = self.engine()
        {
            let callback = engine.errors.borrow_mut().take();
            if let Some(mut callback) = callback {
                callback(error);
                if engine.alive.get() {
                    *engine.errors.borrow_mut() = Some(callback);
                }
            }
        }
    }
    fn engine(&self) -> Result<Rc<Engine>> {
        let engine = self
            .engine
            .upgrade()
            .ok_or_else(|| ChartError("raster surface dropped".into()))?;
        engine.check()?;
        Ok(engine)
    }
    /// Replace the legend action subscriber. This does not change mount-time
    /// RasterSurfaceOptions::legend_actions or the current hidden-series set.
    pub fn on_legend_activate(
        &self,
        callback: impl FnMut(LegendActivation) + 'static,
    ) -> Result<()> {
        let engine = self.engine()?;
        engine
            .legend_generation
            .set(engine.legend_generation.get().wrapping_add(1));
        *engine.legend_observer.borrow_mut() = Some(Box::new(callback));
        Ok(())
    }
    pub fn is_active(&self) -> bool {
        self.engine().is_ok()
    }
    pub fn stop(&self) {
        if let Some(engine) = self.engine.upgrade() {
            engine.stop();
        }
    }
    pub fn snapshot(&self) -> Result<RasterSurfaceSnapshot> {
        let engine = self.engine()?;
        let data = engine.data.borrow();
        let data = data
            .as_ref()
            .ok_or_else(|| ChartError("raster surface stopped".into()))?;
        let model = &data.model;
        Ok(RasterSurfaceSnapshot {
            revision: data.revision,
            width: model.scene().width,
            height: model.scene().height,
            selected: model.selected().map(str::to_owned),
            hovered: model.hovered().to_vec(),
            hidden: model.hidden().clone(),
            view: model.view(),
            touches: model.touch_count(),
            dragging: model.dragging(),
            editable_brush: editable_brush::snapshot(model)?,
        })
    }
    pub fn frame(&self) -> Result<Rc<RenderedImage>> {
        let engine = self.engine()?;
        let image = engine
            .data
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("raster surface stopped".into()))?
            .image
            .clone();
        Ok(image)
    }
    /// Business rows at the current hover, or the selected datum when there
    /// is no hover. Positions stay in logical Scene coordinates.
    pub fn active_interaction(&self) -> Result<Option<ActiveInteraction>> {
        let engine = self.engine()?;
        let data = engine.data.borrow();
        let model = &data
            .as_ref()
            .ok_or_else(|| ChartError("raster surface stopped".into()))?
            .model;
        let selected;
        let ids = if model.hovered().is_empty() {
            selected = model
                .selected()
                .map(str::to_owned)
                .into_iter()
                .collect::<Vec<_>>();
            &selected
        } else {
            model.hovered()
        };
        Ok(model.active_interaction(ids))
    }
    /// Copy the accepted binding for application-owned detail panels.
    pub fn interaction_data(&self) -> Result<Option<InteractionBinding>> {
        let engine = self.engine()?;
        let data = engine.data.borrow();
        Ok(data
            .as_ref()
            .ok_or_else(|| ChartError("raster surface stopped".into()))?
            .model
            .interaction_data()
            .cloned())
    }
    pub fn dispatch(&self, input: SurfaceInput) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, _| model.apply(input))
    }
    /// Rebase every pointer variant through the supplied host image rectangle;
    /// semantic keyboard commands have no coordinates and pass through directly.
    pub fn dispatch_host(
        &self,
        input: SurfaceInput,
        bounds: RasterSurfaceBounds,
    ) -> Result<Vec<SurfaceEvent>> {
        let engine = self.engine()?;
        let scene = engine
            .data
            .borrow()
            .as_ref()
            .ok_or_else(|| ChartError("raster surface stopped".into()))?
            .model
            .scene()
            .clone();
        let input = match input {
            SurfaceInput::Move { id, point } => SurfaceInput::Move {
                id,
                point: bounds.point(point, &scene)?,
            },
            SurfaceInput::Down { id, point, brush } => SurfaceInput::Down {
                id,
                point: bounds.point(point, &scene)?,
                brush,
            },
            SurfaceInput::Up { id, point } => SurfaceInput::Up {
                id,
                point: bounds.point(point, &scene)?,
            },
            SurfaceInput::Wheel { point, delta } => SurfaceInput::Wheel {
                point: bounds.point(point, &scene)?,
                delta,
            },
            SurfaceInput::TouchStart { id, point } => SurfaceInput::TouchStart {
                id,
                point: bounds.point(point, &scene)?,
            },
            SurfaceInput::TouchMove { id, point } => SurfaceInput::TouchMove {
                id,
                point: bounds.point(point, &scene)?,
            },
            SurfaceInput::TouchEnd { id, point } => SurfaceInput::TouchEnd {
                id,
                point: bounds.point(point, &scene)?,
            },
            other => other,
        };
        engine.update(|model, _| model.apply(input))
    }
    pub fn set_scene(&self, scene: Scene) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, _| model.set_scene(scene))
    }
    /// Replace the plot, paint and business binding as one admitted frame.
    /// Invalid bindings, render failures and publisher errors retain all three.
    pub fn set_layout(&self, layout: SceneInteraction) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, options| {
            let plot = layout.plot;
            let events = model.set_layout(layout)?;
            if let Some(plot) = plot {
                options.interaction.plot = Some(plot);
            }
            Ok(events)
        })
    }
    pub fn set_hidden(&self, hidden: BTreeSet<String>) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, _| model.set_hidden(hidden))
    }
    pub fn set_selected(&self, id: Option<String>) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, _| model.set_selected(id))
    }
    pub fn set_hovered(&self, ids: Vec<String>) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|model, _| model.set_hovered(ids))
    }
    pub fn set_raster_options(&self, raster: RasterOptions) -> Result<Vec<SurfaceEvent>> {
        self.engine()?.update(|_, options| {
            options.raster = raster;
            Ok(Vec::new())
        })
    }
}
impl Widget for RasterSurfaceWidget {
    fn build(&mut self, scope: &Scope) -> glory::ui::Result {
        let engine = self
            .engine
            .upgrade()
            .ok_or_else(|| ui_error(ChartError("raster surface dropped".into())))?;
        engine.check().map_err(ui_error)?;
        scene::ensure(
            scope.runtime() == engine.scope.runtime() && !engine.built.replace(true),
            "raster surface widget must mount once in its owning runtime",
        )
        .map_err(ui_error)?;
        let child = scope.child()?;
        let handle = RasterSurfaceHandle {
            engine: Rc::downgrade(&engine),
        };
        let options = engine
            .data
            .borrow()
            .as_ref()
            .expect("live surface")
            .options
            .clone();
        let hidden = handle.clone();
        let selected = handle.clone();
        let activate = handle.clone();
        let hover = handle.clone();
        let focus = handle.clone();
        let tooltip_engine = Rc::downgrade(&engine);
        let tooltip_options = options.tooltip.clone();
        let controls = chart(engine.scene)
            .plot(false)
            .data_control_labels(engine.data_labels)
            .data_control_pointer(engine.data_pointer)
            .hidden(engine.hidden)
            .selected(engine.selected)
            .hovered(engine.hovered)
            .tooltip_marks(engine.tooltip)
            .legend_options(options.legend)
            .tooltip_options(options.tooltip)
            .tooltip_formatter(move |marks| {
                let engine = tooltip_engine
                    .upgrade()
                    .ok_or_else(|| ChartError("raster surface dropped".into()))?;
                engine.check()?;
                let _ = engine.metadata_version.get();
                let data = engine.data.borrow();
                let model = &data
                    .as_ref()
                    .ok_or_else(|| ChartError("raster surface stopped".into()))?
                    .model;
                let ids = marks.iter().map(|mark| mark.id.clone()).collect::<Vec<_>>();
                if let Some(active) = model.active_interaction(&ids) {
                    interaction_tooltip_text(&tooltip_options, &active)
                } else {
                    tooltip_text(&tooltip_options, &marks.iter().collect::<Vec<_>>())
                }
            })
            .on_legend_toggle(move |activation| {
                let result = hidden
                    .engine()
                    .and_then(|engine| engine.toggle_legend(activation));
                hidden.report(result);
            })
            .on_selection_change(move |mark| {
                let result = selected.set_selected(mark.map(|m| m.id));
                selected.report(result);
            })
            .on_select(move |mark| {
                let result = activate
                    .engine()
                    .and_then(|e| e.update(|model, _| model.activate(&mark.id)));
                activate.report(result);
            })
            .on_hover(move |mark| {
                let result = hover.engine().and_then(|engine| {
                    // A queued control-origin hover cannot replace a physical
                    // hit after the accepted layout switches to PaintedGeometry.
                    if engine
                        .data
                        .borrow()
                        .as_ref()
                        .is_some_and(|data| plot_pointer::strict(&data.model))
                    {
                        return Ok(vec![]);
                    }
                    engine.update(|model, _| {
                        model.set_hovered(mark.into_iter().map(|m| m.id).collect())
                    })
                });
                hover.report(result);
            })
            .on_focus(move |mark| {
                let result = focus.set_hovered(mark.into_iter().map(|m| m.id).collect());
                focus.report(result);
            });
        let controls = if options.legend_actions {
            let weak = Rc::downgrade(&engine);
            controls.on_legend_activate(move |activation| {
                if let Some(engine) = weak.upgrade() {
                    engine.activate_legend(activation);
                }
            })
        } else {
            controls
        };
        let result = glory::ui::view()
            .class("kartis-raster-surface")
            .style(Style::new().position(Position::Relative))
            .fill((
                editable_brush::ImageAndControls {
                    engine: Rc::downgrade(&engine),
                },
                controls,
            ))
            .show_in(&child);
        if result.is_ok() {
            *engine.ui_scope.borrow_mut() = Some(child);
        } else {
            child.owner().dispose();
            engine.stop();
        }
        result
    }
}
