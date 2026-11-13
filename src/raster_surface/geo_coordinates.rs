//! Read-only geographic picking through the native image's accepted input chain.
use super::*;

impl RasterSurfaceHandle {
    /// Convert host input in the supplied image bounds to accepted Scene pixels.
    /// Bounds and input share units and origin; image DPR is already accounted
    /// for by those bounds. The current paint camera is removed exactly once.
    /// This conversion does not itself test plot or geographic visibility.
    pub fn scene_point_at_host(
        &self,
        host: (f64, f64),
        bounds: RasterSurfaceBounds,
    ) -> Result<(f64, f64)> {
        with_host_point(self, host, bounds, |model, point| {
            model.paint_transform().unproject(point)
        })
    }

    /// Invert a host point within a fitted projection's intrinsic footprint.
    /// Retain the projection together with its accepted layout. Equal dimensions
    /// cannot identify a different camera or a stale same-sized layout; feature
    /// coverage and the viewport plot are not inferred by this method.
    pub fn invert_geo_at_host(
        &self,
        projection: &GeoProjectionContext,
        host: (f64, f64),
        bounds: RasterSurfaceBounds,
    ) -> Result<Option<[f64; 2]>> {
        with_host_point(self, host, bounds, |model, point| {
            matching_dimensions(projection, model)?;
            let point = model.paint_transform().unproject(point)?;
            projection.invert([point.0, point.1])
        })
    }

    /// Geographic picking restricted to both the current viewport plot and the
    /// projection's authored Scene plot. The Scene pan/zoom camera is removed
    /// once; the projection inverse removes its distinct spherical camera.
    pub fn invert_geo_in_plot_at_host(
        &self,
        projection: &GeoProjectionContext,
        host: (f64, f64),
        bounds: RasterSurfaceBounds,
    ) -> Result<Option<[f64; 2]>> {
        with_host_point(self, host, bounds, |model, point| {
            matching_dimensions(projection, model)?;
            if !model.plot().contains(point.0, point.1) {
                return Ok(None);
            }
            let point = model.paint_transform().unproject(point)?;
            projection.invert_in_plot([point.0, point.1])
        })
    }
}

fn matching_dimensions(projection: &GeoProjectionContext, model: &SurfaceModel) -> Result<()> {
    scene::ensure(
        projection.dimensions() == [model.scene().width, model.scene().height],
        "geographic context dimensions do not match the accepted Scene",
    )
}

fn with_host_point<T>(
    handle: &RasterSurfaceHandle,
    host: (f64, f64),
    bounds: RasterSurfaceBounds,
    convert: impl FnOnce(&SurfaceModel, (f64, f64)) -> Result<T>,
) -> Result<T> {
    let engine = handle.engine()?;
    let data = engine.data.borrow();
    let data = data
        .as_ref()
        .ok_or_else(|| ChartError("raster surface stopped".into()))?;
    let point = bounds.point(host, data.model.scene())?;
    convert(&data.model, point)
}
