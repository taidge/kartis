use super::*;
pub struct FunnelDataWidgetContext<'a, D> {
    /// Closing separators have no raw source, preserving the legacy role.
    pub data: Option<&'a D>,
    pub frame: &'a FunnelDataFrame<D>,
    pub computed: FunnelWidgetContext<'a>,
}
pub struct FunnelDataWidgetData<D> {
    frame: FunnelDataFrame<D>,
    computed: FunnelWidgetData,
}
impl<D> Clone for FunnelDataWidgetData<D> {
    fn clone(&self) -> Self {
        Self {
            frame: self.frame.clone(),
            computed: self.computed.clone(),
        }
    }
}
impl<D> PartialEq for FunnelDataWidgetData<D> {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame && self.computed == other.computed
    }
}
impl<D> fmt::Debug for FunnelDataWidgetData<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FunnelDataWidgetData")
            .field("frame", &self.frame)
            .field("computed", &self.computed)
            .finish()
    }
}
impl<D> FunnelDataWidgetData<D> {
    pub fn computed(&self) -> &FunnelWidgetData {
        &self.computed
    }
    pub fn frame(&self) -> &FunnelDataFrame<D> {
        &self.frame
    }
    pub fn data(&self) -> Option<&D> {
        self.computed.source_index.and_then(|i| self.frame.data(i))
    }
}
impl<D: 'static> FunnelDataFrame<D> {
    /// Existing native Mark roles use the unannotated source projection.
    /// Bound annotations remain independent decoration on the complete frame;
    /// no annotation or closing separator is fabricated into an owned datum.
    pub fn widgets(
        &self,
        revision: u64,
        mut placement: impl FnMut(
            &FunnelDataWidgetContext<'_, D>,
        ) -> Result<Option<PlotWidgetPlacement>>,
    ) -> Result<PlotWidgetsData<FunnelDataWidgetData<D>>> {
        let base = self.computed().annotation_source();
        let packet = funnel_widgets(base.layout(), base.source(), revision, |computed| {
            let data = computed.source_index.and_then(|i| self.data(i));
            placement(&FunnelDataWidgetContext {
                data,
                frame: self,
                computed: *computed,
            })
        })?;
        Ok(PlotWidgetsData {
            layout: packet.layout,
            data: packet
                .data
                .into_iter()
                .map(|(key, computed)| {
                    (
                        key,
                        FunnelDataWidgetData {
                            frame: self.clone(),
                            computed,
                        },
                    )
                })
                .collect(),
        })
    }
}
