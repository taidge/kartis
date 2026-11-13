//! CPU-rendered PNG images inside the standard Glory chart Widget.
//!
//! The default source is an in-memory PNG data URL, verified in Glory's web
//! image host. Native applications must explicitly supply an image provider
//! and a source publisher: a String image source grants no native I/O rights.
//! Transactional retention covers rendering, encoding and source publication.
//! A custom provider's later asynchronous decode/transport failures are host
//! observations; they cannot roll back the accepted application model.
//! This module does not install plot-coordinate pointer listeners. Applications
//! can use `SurfaceModel` with their host's explicitly supplied coordinates.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::{Cage, Scope, Vary};

use crate::render::raster::{FontRegistry, RasterOptions, RenderedImage, render_raster};
use crate::scene::ensure;
use crate::{Chart, ChartError, Result, Scene, chart};

/// Maximum encoded image-source bytes retained by one widget frame.
pub const MAX_RASTER_IMAGE_SOURCE_BYTES: usize = 32 * 1024 * 1024;
pub(crate) type ImageSource = Box<dyn FnMut(&RenderedImage) -> Result<String>>;
pub(crate) struct RasterInput {
    pub fonts: FontRegistry,
    pub options: Vary<RasterOptions>,
}

/// Use a real PNG-backed Glory image and the normal chart controls.
/// Fonts are registered explicitly; neither this widget nor the raster backend
/// searches the host filesystem or system font registry.
pub fn raster_chart(
    scene: impl Into<Vary<Scene>>,
    fonts: FontRegistry,
    options: impl Into<Vary<RasterOptions>>,
) -> Chart {
    chart(scene).raster(fonts, options)
}
impl Chart {
    pub fn raster(mut self, fonts: FontRegistry, options: impl Into<Vary<RasterOptions>>) -> Self {
        self.raster = Some(RasterInput {
            fonts,
            options: options.into(),
        });
        self
    }
    /// Explicit image publication, for example an application-owned native PNG
    /// provider. This callback must not perform a nested Glory flush. Its captures
    /// are released on owner cleanup; the application owns any provider cache.
    /// Returning Err retains the previous image, scene, hidden state and controls.
    pub fn raster_image_source(
        mut self,
        publish: impl FnMut(&RenderedImage) -> Result<String> + 'static,
    ) -> Self {
        self.raster_source = Some(Box::new(publish));
        self
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RasterFrame {
    pub source: String,
    pub width: f64,
    pub height: f64,
}
#[derive(Clone)]
struct Publisher {
    callback: Rc<RefCell<Option<ImageSource>>>,
    alive: Rc<Cell<bool>>,
    custom: bool,
}
impl Publisher {
    fn new(callback: Option<ImageSource>) -> Self {
        let custom = callback.is_some();
        Self {
            callback: Rc::new(RefCell::new(callback)),
            alive: Rc::new(Cell::new(true)),
            custom,
        }
    }
    fn publish(&self, image: &RenderedImage) -> Result<String> {
        ensure(self.alive.get(), "raster widget owner retired")?;
        if !self.custom {
            return image.data_url();
        }
        let mut callback = self
            .callback
            .borrow_mut()
            .take()
            .ok_or_else(|| ChartError("reentrant raster image publication".into()))?;
        let result = callback(image);
        if self.alive.get() {
            *self.callback.borrow_mut() = Some(callback);
        }
        ensure(self.alive.get(), "raster widget owner retired")?;
        result
    }
    fn clear(&self) {
        self.alive.set(false);
        self.callback.borrow_mut().take();
    }
}
#[derive(Clone)]
pub(crate) struct RasterState {
    fonts: FontRegistry,
    publisher: Publisher,
    pub options: Cage<RasterOptions>,
    pub frame: Cage<RasterFrame>,
}
impl RasterState {
    pub fn new(
        scope: &Scope,
        input: &RasterInput,
        options: RasterOptions,
        scene: &Scene,
        hidden: &BTreeSet<String>,
        source: Option<ImageSource>,
    ) -> Result<Self> {
        let publisher = Publisher::new(source);
        // Initial publication can run application code before Chart installs
        // its remaining callbacks. Tie the publisher to this owner first.
        let cleanup = publisher.clone();
        scope
            .on_cleanup(move || cleanup.clear())
            .map_err(|error| ChartError(error.to_string()))?;
        let frame = prepare(&input.fonts, &publisher, scene, hidden, &options)?;
        scope
            .owner()
            .check_write()
            .map_err(|error| ChartError(error.to_string()))?;
        Ok(Self {
            fonts: input.fonts.clone(),
            publisher,
            options: scope.cage(options),
            frame: scope.cage(frame),
        })
    }
    pub fn prepare(
        &self,
        scene: &Scene,
        hidden: &BTreeSet<String>,
        options: &RasterOptions,
    ) -> Result<RasterFrame> {
        prepare(&self.fonts, &self.publisher, scene, hidden, options)
    }
    pub fn commit(&self, frame: RasterFrame, options: RasterOptions) {
        if self.frame.with(|old| old != &frame) {
            self.frame.revise(|old| *old = frame);
        }
        if self.options.with(|old| old != &options) {
            self.options.revise(|old| *old = options);
        }
    }
    pub fn clear(&self) {
        self.publisher.clear();
    }
}
fn prepare(
    fonts: &FontRegistry,
    publisher: &Publisher,
    scene: &Scene,
    hidden: &BTreeSet<String>,
    options: &RasterOptions,
) -> Result<RasterFrame> {
    let visible = scene.without_series(hidden);
    let image = render_raster(&visible, fonts, options)?;
    let source = publisher.publish(&image)?;
    ensure(
        !source.is_empty() && source.len() <= MAX_RASTER_IMAGE_SOURCE_BYTES,
        "raster image source byte budget exceeded or empty source",
    )?;
    Ok(RasterFrame {
        source,
        width: scene.width,
        height: scene.height,
    })
}
