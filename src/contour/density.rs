pub use core::{DensityOptions, DensityThresholds, WeightedPoint as DensityPoint};
use std::sync::Arc;

use super::{ContourLevel, ContourLimits, algorithm_error, core};
use crate::Result;

/// Immutable, shareable aggregate. No business samples or historical fields are
/// retained. Raw cells preserve the original Float32 accumulation; thresholds
/// and `max` use weight per square logical pixel, independent of renderer DPR.
#[derive(Clone, Debug)]
pub struct DensityField {
    inner: Arc<core::DensityField>,
    size: [f64; 2],
    origin: [f64; 2],
    bandwidth: f64,
    requested_cell_size: f64,
    sample_count: usize,
}
impl DensityField {
    pub fn values(&self) -> &[f32] {
        self.inner.values()
    }
    pub fn dimensions(&self) -> [usize; 2] {
        self.inner.dimensions()
    }
    pub fn cell_size(&self) -> usize {
        self.inner.cell_size()
    }
    pub fn requested_cell_size(&self) -> f64 {
        self.requested_cell_size
    }
    pub fn origin(&self) -> [f64; 2] {
        self.origin
    }
    pub fn size(&self) -> [f64; 2] {
        self.size
    }
    pub fn bandwidth(&self) -> f64 {
        self.bandwidth
    }
    pub fn sample_count(&self) -> usize {
        self.sample_count
    }
    pub fn max(&self) -> f64 {
        self.inner.max()
    }
    pub fn contour(&self, value: f64, limits: ContourLimits) -> Result<ContourLevel> {
        self.inner.contour(value, limits).map_err(algorithm_error)
    }
    pub fn contours(
        &self,
        thresholds: &DensityThresholds,
        limits: ContourLimits,
    ) -> Result<Vec<ContourLevel>> {
        self.inner
            .contours(thresholds, limits)
            .map_err(algorithm_error)
    }
}
pub fn estimate_density(points: &[DensityPoint], options: DensityOptions) -> Result<DensityField> {
    let inner = core::density(points, options).map_err(algorithm_error)?;
    let radius = ((4. * options.bandwidth * options.bandwidth + 1.).sqrt() - 1.) / 2.;
    let offset = radius * 3.;
    Ok(DensityField {
        inner: Arc::new(inner),
        size: options.size,
        origin: [-offset, -offset],
        bandwidth: options.bandwidth,
        requested_cell_size: options.cell_size,
        sample_count: points.len(),
    })
}
