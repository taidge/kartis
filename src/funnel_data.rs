//! Owned application data with Funnel's checked projection and source cohort.
//! Borrowed factories run during one build; no function or browser payload is
//! stored in the existing FunnelSvgFrame. D has no blanket trait requirements.
use std::fmt;
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;
mod factories;
pub use factories::*;
mod annotations;
pub use annotations::*;
mod widgets;
pub use widgets::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FunnelDataDatum<D> {
    pub data: D,
    pub projected: FunnelDatum,
}
impl<D> FunnelDataDatum<D> {
    pub fn new(data: D, projected: FunnelDatum) -> Self {
        Self { data, projected }
    }
}
struct Cohort<D> {
    source: Arc<[FunnelDataDatum<D>]>,
    computed: FunnelSvgFrame,
}
pub struct FunnelDataFrame<D>(Arc<Cohort<D>>);
impl<D> Clone for FunnelDataFrame<D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<D> PartialEq for FunnelDataFrame<D> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<D> Eq for FunnelDataFrame<D> {}
impl<D> fmt::Debug for FunnelDataFrame<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunnelDataFrame")
            .field("source_len", &self.0.source.len())
            .field("computed", &self.0.computed)
            .finish_non_exhaustive()
    }
}
impl<D> FunnelDataFrame<D> {
    pub fn new(
        config: &ChartConfig,
        source: Vec<FunnelDataDatum<D>>,
        options: &FunnelLayoutOptions,
    ) -> Result<Self> {
        Self::from_shared(config, source.into(), options)
    }
    /// Shared ownership makes rebuilds possible without D:Clone. A new capture
    /// is always distinct, even when the raw pool and numeric Scene are equal.
    pub fn from_shared(
        config: &ChartConfig,
        source: Arc<[FunnelDataDatum<D>]>,
        options: &FunnelLayoutOptions,
    ) -> Result<Self> {
        Self::with_shared_factories(config, source, options, &mut FunnelDataFactories::default())
    }
    pub fn with_factories(
        config: &ChartConfig,
        source: Vec<FunnelDataDatum<D>>,
        options: &FunnelLayoutOptions,
        factories: &mut FunnelDataFactories<'_, D>,
    ) -> Result<Self> {
        Self::with_shared_factories(config, source.into(), options, factories)
    }
    pub fn with_shared_factories(
        config: &ChartConfig,
        source: Arc<[FunnelDataDatum<D>]>,
        options: &FunnelLayoutOptions,
        factories: &mut FunnelDataFactories<'_, D>,
    ) -> Result<Self> {
        ensure(
            source.len() <= MAX_MARKS,
            "Funnel raw source budget exceeded",
        )?;
        let projected: Vec<_> = source.iter().map(|d| d.projected.clone()).collect();
        let computed = factories::build(config, &projected, options, &source, factories)?;
        Ok(Self(Arc::new(Cohort { source, computed })))
    }
    pub fn source(&self) -> &[FunnelDataDatum<D>] {
        &self.0.source
    }
    pub fn data(&self, source_index: usize) -> Option<&D> {
        self.0.source.get(source_index).map(|d| &d.data)
    }
    pub fn computed(&self) -> &FunnelSvgFrame {
        &self.0.computed
    }
    pub fn config(&self) -> &ChartConfig {
        self.computed().config()
    }
    pub fn options(&self) -> &FunnelLayoutOptions {
        self.computed().options()
    }
    pub fn layout(&self) -> &FunnelLayout {
        self.computed().layout()
    }
    pub fn scene(&self) -> &Scene {
        self.computed().scene()
    }
    /// Distinct config/annotation captures may share one unchanged raw pool.
    /// This does not assert their computed frames or full cohorts are equal.
    pub fn shares_source(&self, other: &Self) -> bool {
        self.same_source(other)
    }
    pub(crate) fn shared_source(&self) -> Arc<[FunnelDataDatum<D>]> {
        self.0.source.clone()
    }
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0.source, &other.0.source)
    }
    /// Preserve an application's legal Scene ink/extra edits from a genuinely
    /// constructed Funnel frame. Every original projected source field remains
    /// bit-exact, including nested paint coordinates; no raw datum is inferred.
    pub fn with_computed(&self, computed: FunnelSvgFrame) -> Result<Self> {
        ensure(
            computed.source().len() == self.source().len()
                && computed
                    .source()
                    .iter()
                    .zip(self.source())
                    .all(|(a, b)| datum_equal(a, &b.projected)),
            "Funnel raw projection differs from its source",
        )?;
        Ok(Self(Arc::new(Cohort {
            source: self.shared_source(),
            computed,
        })))
    }
}
fn datum_equal(a: &FunnelDatum, b: &FunnelDatum) -> bool {
    crate::widget_source_eq::category(&a.id, &b.id)
        && a.label == b.label
        && crate::widget_source_eq::number(a.value, b.value)
        && crate::widget_source_eq::optional(a.size, b.size)
        && a.color == b.color
        && crate::widget_source_eq::paint(a.paint.as_ref(), b.paint.as_ref())
}
