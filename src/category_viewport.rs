//! Category navigation retains the complete category domain. Fractional band
//! windows zoom/pan smoothly; layouts keep IDs, colors and bar stacks stable.
use crate::{scene::ensure, *};
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryViewport {
    categories: Vec<String>,
    controller: DomainViewport,
}
#[derive(Clone, Debug, PartialEq)]
pub struct CategoryWindow {
    pub normalized: [f64; 2],
    /// Index-space edges, not a list of remapped indices.
    pub domain: [f64; 2],
    pub categories: Vec<String>,
}
impl CategoryViewport {
    pub fn new(categories: Vec<String>, plot: ClipRect, limits: ViewportLimits) -> Result<Self> {
        Self::with_value_axes(categories, plot, limits, None, None)
    }
    pub fn with_value_axes(
        categories: Vec<String>,
        plot: ClipRect,
        mut limits: ViewportLimits,
        left: Option<ViewportAxis>,
        right: Option<ViewportAxis>,
    ) -> Result<Self> {
        ensure(
            !categories.is_empty() && categories.len() <= 10_000,
            "category viewport needs 1..=10000 categories",
        )?;
        crate::scales::BandScale::new(categories.clone(), [0., 1.], 0.)?;
        limits.validate()?;
        ensure(limits.bounded, "category viewport must be bounded")?;
        if left.is_none() && right.is_none() {
            limits.max_zoom = limits.max_zoom.min(categories.len() as f64);
            limits.min_zoom = limits.min_zoom.min(limits.max_zoom);
        }
        let controller = DomainViewport::new(
            ViewportAxes {
                x: ViewportAxis::new([0., categories.len() as f64], ScaleKind::Linear),
                left,
                right,
            },
            plot,
            limits,
        )?
        .with_x_max_zoom(categories.len() as f64)?;
        Ok(Self {
            categories,
            controller,
        })
    }
    pub fn controller(&self) -> &DomainViewport {
        &self.controller
    }
    pub fn into_controller(self) -> DomainViewport {
        self.controller
    }
    pub fn categories(&self) -> &[String] {
        &self.categories
    }
    pub fn window(&self) -> Result<CategoryWindow> {
        Self::resolve(&self.categories, self.controller.domains().x)
    }
    pub fn resolve(categories: &[String], domain: [f64; 2]) -> Result<CategoryWindow> {
        ensure(
            categories.len() <= 10_000,
            "category viewport category budget exceeded",
        )?;
        crate::scales::BandScale::new(categories.to_vec(), [0., 1.], 0.)?;
        let n = categories.len() as f64;
        ensure(
            n > 0.
                && domain.iter().all(|v| v.is_finite())
                && domain[0] >= 0.
                && domain[1] <= n
                && domain[0] < domain[1],
            "category viewport range is outside original categories",
        )?;
        Ok(CategoryWindow {
            normalized: [domain[0] / n, domain[1] / n],
            domain,
            categories: categories
                [domain[0].floor() as usize..(domain[1].ceil() as usize).min(categories.len())]
                .to_vec(),
        })
    }
    pub fn apply(&mut self, request: ViewportRequest) -> Result<CategoryWindow> {
        let mut next = self.clone();
        next.controller.apply(request)?;
        let window = next.window()?;
        *self = next;
        Ok(window)
    }
    pub fn resize(&mut self, plot: ClipRect) -> Result<CategoryWindow> {
        self.controller.resize(plot)?;
        self.window()
    }
}
