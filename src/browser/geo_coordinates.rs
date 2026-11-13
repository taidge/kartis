//! Public client-coordinate conversion shared with the host's own input chain.
use super::*;

impl BrowserHandle {
    /// Convert precise CSS client coordinates to accepted Scene coordinates.
    /// Reuses the host surface bounds and removes its current paint camera
    /// exactly once. Relayout mode already paints Scene coordinates directly.
    /// This conversion does not itself test plot or geographic visibility.
    pub fn scene_point_at_client(&self, client: (f64, f64)) -> Result<(f64, f64)> {
        let engine = self.engine()?;
        let state = engine.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
        state
            .model
            .paint_transform()
            .unproject(state.coordinates(client)?)
    }

    /// Invert the accepted Scene point within a fitted projection's intrinsic
    /// footprint. The caller retains the projection with its accepted layout;
    /// feature-polygon coverage and the viewport plot are not inferred here.
    pub fn invert_geo_at_client(
        &self,
        projection: &GeoProjectionContext,
        client: (f64, f64),
    ) -> Result<Option<[f64; 2]>> {
        let engine = self.engine()?;
        let state = engine.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
        scene::ensure(
            projection.dimensions() == [state.model.scene().width, state.model.scene().height],
            "geographic context dimensions do not match the accepted Scene",
        )?;
        let point = state
            .model
            .paint_transform()
            .unproject(state.coordinates(client)?)?;
        projection.invert([point.0, point.1])
    }

    /// Geographic picking restricted to both the visible viewport plot and
    /// the projection's authored Scene plot. Uses the same paint-camera chain
    /// as `scene_point_at_client`; the projection camera is removed by its own
    /// inverse and is distinct from the Scene pan/zoom camera.
    pub fn invert_geo_in_plot_at_client(
        &self,
        projection: &GeoProjectionContext,
        client: (f64, f64),
    ) -> Result<Option<[f64; 2]>> {
        let engine = self.engine()?;
        let state = engine.state.borrow();
        let state = state
            .as_ref()
            .ok_or_else(|| ChartError("browser chart is stopped".into()))?;
        scene::ensure(
            projection.dimensions() == [state.model.scene().width, state.model.scene().height],
            "geographic context dimensions do not match the accepted Scene",
        )?;
        let point = state.coordinates(client)?;
        if !state.model.plot().contains(point.0, point.1) {
            return Ok(None);
        }
        let scene = state.model.paint_transform().unproject(point)?;
        projection.invert_in_plot([scene.0, scene.1])
    }
}
