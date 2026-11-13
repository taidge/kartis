//! Checked planar contours, immutable weighted density fields and typed Scene adapters.
//! Original algorithm licenses are retained in `docs/third-party-notices.md`
//! and `tests/fixtures/contour-density-d3-LICENSE.txt`.
mod core;
mod density;
mod layout;
mod scatter;

use crate::{ChartError, Result};
pub use core::{
    Grid as ContourGrid, Level as ContourLevel, Limits as ContourLimits, Options as ContourOptions,
    Point as ContourPoint, Polygon as ContourPolygon, Ring as ContourRing,
    Thresholds as ContourThresholds,
};
pub use density::{
    DensityField, DensityOptions, DensityPoint, DensityThresholds, estimate_density,
};
pub use layout::{
    ContourLayout, ContourLayoutOptions, ContourLevelMetadata, ContourSceneLimits, contour_layout,
    contour_levels_layout, density_layout,
};
pub use scatter::{
    DensityObservation, DensityWeights, ScatterDensityLayout, ScatterDensityOptions,
    scatter_density_layout,
};

pub(crate) fn checked(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
pub(crate) fn algorithm_error(error: core::Error) -> ChartError {
    ChartError(error.0)
}

/// Extract one finite threshold from a row-major planar grid.
/// `None` cells consistently behave as low/NaN samples. Nonfinite present
/// samples reject. Rings retain their winding, islands and explicit holes.
pub fn contour(
    grid: ContourGrid<'_>,
    threshold: f64,
    smooth: bool,
    limits: ContourLimits,
) -> Result<ContourLevel> {
    core::contour(grid, threshold, smooth, limits).map_err(algorithm_error)
}
/// Extract sorted explicit or automatic thresholds under cumulative budgets.
pub fn contours(grid: ContourGrid<'_>, options: &ContourOptions) -> Result<Vec<ContourLevel>> {
    core::contours(grid, options).map_err(algorithm_error)
}
