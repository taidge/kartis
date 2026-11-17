use super::*;
/// Full raw and computed collections at the binding phase. Source indices are
/// the actual original indices; typed IDs are never used to retarget old data.
pub struct FunnelDataAnnotationMatchContext<'a, D> {
    pub data: &'a D,
    pub frame: &'a FunnelDataFrame<D>,
    pub computed: FunnelAnnotationMatchContext<'a>,
}
pub struct FunnelDataAnnotationNoteContext<'a, D> {
    pub data: &'a D,
    pub frame: &'a FunnelDataFrame<D>,
    pub computed: FunnelAnnotationNoteContext<'a>,
}
impl<D> FunnelDataAnnotationNoteContext<'_, D> {
    pub fn scene(&self) -> Result<Scene> {
        self.computed.scene()
    }
}
pub type FunnelDataAnnotationMatcher<'a, D> =
    dyn FnMut(FunnelDataAnnotationMatchContext<'_, D>) -> Result<bool> + 'a;
pub type FunnelDataAnnotationNoteRenderer<'a, D> =
    dyn FnMut(FunnelDataAnnotationNoteContext<'_, D>) -> Result<Scene> + 'a;
pub struct FunnelDataAnnotationFactories<'a, D> {
    pub matcher: Option<&'a mut FunnelDataAnnotationMatcher<'a, D>>,
    pub note: Option<&'a mut FunnelDataAnnotationNoteRenderer<'a, D>>,
}
impl<D> Default for FunnelDataAnnotationFactories<'_, D> {
    fn default() -> Self {
        Self {
            matcher: None,
            note: None,
        }
    }
}
impl<D> FunnelDataFrame<D> {
    pub fn with_annotations(
        &self,
        specs: &[FunnelAnnotationSpec],
        factories: &mut FunnelDataAnnotationFactories<'_, D>,
        style: &FunnelAnnotationStyle,
    ) -> Result<Self> {
        self.with_annotations_with_budget(
            specs,
            factories,
            style,
            FunnelAnnotationBudget::default(),
        )
    }
    pub fn with_annotations_with_budget(
        &self,
        specs: &[FunnelAnnotationSpec],
        factories: &mut FunnelDataAnnotationFactories<'_, D>,
        style: &FunnelAnnotationStyle,
        budget: FunnelAnnotationBudget,
    ) -> Result<Self> {
        let base = if self.computed().annotations().is_none() {
            self.clone()
        } else {
            self.with_computed(self.computed().annotation_source().clone())?
        };
        let FunnelDataAnnotationFactories { matcher, note } = factories;
        let has_matcher = matcher.is_some();
        let has_note = note.is_some();
        let mut match_raw = |computed: FunnelAnnotationMatchContext<'_>| {
            let data = base
                .data(computed.source_index)
                .ok_or_else(|| ChartError("Funnel annotation raw source is missing".into()))?;
            matcher
                .as_deref_mut()
                .expect("configured raw Funnel annotation matcher")(
                FunnelDataAnnotationMatchContext {
                    data,
                    frame: &base,
                    computed,
                },
            )
        };
        let mut draw_note = |computed: FunnelAnnotationNoteContext<'_>| {
            let data = base
                .data(computed.binding.source_index)
                .ok_or_else(|| ChartError("Funnel note raw source is missing".into()))?;
            note.as_deref_mut()
                .expect("configured raw Funnel annotation note")(
                FunnelDataAnnotationNoteContext {
                    data,
                    frame: &base,
                    computed,
                },
            )
        };
        let bundle = funnel_annotations_with_budget(
            base.computed(),
            specs,
            &mut FunnelAnnotationFactories {
                matcher: has_matcher.then_some(&mut match_raw as &mut FunnelAnnotationMatcher<'_>),
                note: has_note.then_some(&mut draw_note as &mut FunnelAnnotationNoteRenderer<'_>),
            },
            style,
            budget,
        )?;
        self.with_annotation_bundle(bundle)
    }
    pub fn with_annotation_bundle(&self, bundle: FunnelAnnotations) -> Result<Self> {
        self.with_computed(self.computed().with_annotation_bundle(bundle)?)
    }
    pub fn annotation_data(&self, binding_index: usize) -> Option<&D> {
        self.computed()
            .annotations()?
            .bindings()
            .get(binding_index)
            .and_then(|b| self.data(b.source_index))
    }
}
