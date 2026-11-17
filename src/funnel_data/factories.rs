use super::*;
/// Raw source is available in its entirety. `computed` retains the original
/// role's evaluation phase; it does not invent future styled parts.
pub struct FunnelDataColorContext<'a, D> {
    pub data: &'a D,
    pub source: &'a [FunnelDataDatum<D>],
    pub computed: FunnelColorContext<'a>,
}
pub struct FunnelDataStyleContext<'a, D> {
    pub data: &'a D,
    pub source: &'a [FunnelDataDatum<D>],
    pub computed: FunnelPartStyleContext<'a>,
}
pub struct FunnelDataSizeContext<'a, D> {
    pub data: &'a D,
    pub source: &'a [FunnelDataDatum<D>],
    pub computed: FunnelSizeContext<'a>,
}
pub struct FunnelDataStageContext<'a, D> {
    pub data: &'a D,
    pub source: &'a [FunnelDataDatum<D>],
    pub computed: FunnelStageContext<'a>,
}
pub type FunnelDataColorFactory<'a, D> =
    dyn FnMut(&FunnelDataColorContext<'_, D>) -> Result<Color> + 'a;
pub type FunnelDataStyleFactory<'a, D> =
    dyn FnMut(&FunnelDataStyleContext<'_, D>) -> Result<Color> + 'a;
pub type FunnelDataSizeFactory<'a, D> = dyn FnMut(FunnelDataSizeContext<'_, D>) -> Result<f64> + 'a;
pub type FunnelDataStageFactory<'a, D> =
    dyn FnMut(&FunnelDataStageContext<'_, D>) -> Result<FunnelStageGeometry> + 'a;
pub struct FunnelDataFactories<'a, D> {
    pub color: Option<&'a mut FunnelDataColorFactory<'a, D>>,
    /// The original number-only formatter contract is retained.
    pub formatted_value: Option<&'a mut FunnelValueFormatter<'a>>,
    pub border_color: Option<&'a mut FunnelDataStyleFactory<'a, D>>,
    pub label_color: Option<&'a mut FunnelDataStyleFactory<'a, D>>,
    pub size: Option<&'a mut FunnelDataSizeFactory<'a, D>>,
    pub stage: Option<&'a mut FunnelDataStageFactory<'a, D>>,
}
impl<D> Default for FunnelDataFactories<'_, D> {
    fn default() -> Self {
        Self {
            color: None,
            formatted_value: None,
            border_color: None,
            label_color: None,
            size: None,
            stage: None,
        }
    }
}
pub(super) fn build<D>(
    config: &ChartConfig,
    projected: &[FunnelDatum],
    options: &FunnelLayoutOptions,
    source: &[FunnelDataDatum<D>],
    factories: &mut FunnelDataFactories<'_, D>,
) -> Result<FunnelSvgFrame> {
    let FunnelDataFactories {
        color,
        formatted_value,
        border_color,
        label_color,
        size,
        stage,
    } = factories;
    let has_formatter = formatted_value.is_some();
    let has_color = color.is_some();
    let has_border = border_color.is_some();
    let has_label = label_color.is_some();
    let has_size = size.is_some();
    let has_stage = stage.is_some();
    let mut resolve_color = |computed: &FunnelColorContext<'_>| {
        color.as_deref_mut().expect("configured raw Funnel color")(&FunnelDataColorContext {
            data: &source[computed.source_index].data,
            source,
            computed: *computed,
        })
    };
    let mut format = |value| {
        formatted_value
            .as_deref_mut()
            .expect("configured raw Funnel formatter")(value)
    };
    let mut resolve_border = |computed: &FunnelPartStyleContext<'_>| {
        border_color
            .as_deref_mut()
            .expect("configured raw Funnel border")(&FunnelDataStyleContext {
            data: &source[computed.part.source_index].data,
            source,
            computed: *computed,
        })
    };
    let mut resolve_label = |computed: &FunnelPartStyleContext<'_>| {
        label_color
            .as_deref_mut()
            .expect("configured raw Funnel label")(&FunnelDataStyleContext {
            data: &source[computed.part.source_index].data,
            source,
            computed: *computed,
        })
    };
    let mut resolve_size = |computed: FunnelSizeContext<'_>| {
        size.as_deref_mut().expect("configured raw Funnel size")(FunnelDataSizeContext {
            data: &source[computed.source_index].data,
            source,
            computed,
        })
    };
    let mut resolve_stage = |computed: &FunnelStageContext<'_>| {
        stage.as_deref_mut().expect("configured raw Funnel stage")(&FunnelDataStageContext {
            data: &source[computed.source_index].data,
            source,
            computed: *computed,
        })
    };
    let mut base = FunnelFactories {
        color: has_color.then_some(&mut resolve_color as &mut FunnelColorFactory<'_>),
        formatted_value: has_formatter.then_some(&mut format as &mut FunnelValueFormatter<'_>),
        border_color: has_border.then_some(&mut resolve_border as &mut FunnelStyleColorFactory<'_>),
        label_color: has_label.then_some(&mut resolve_label as &mut FunnelStyleColorFactory<'_>),
    };
    match (has_size, has_stage) {
        (false, false) => FunnelSvgFrame::with_factories(config, projected, options, &mut base),
        (true, false) => FunnelSvgFrame::with_size_and_factories(
            config,
            projected,
            options,
            &mut resolve_size,
            &mut base,
        ),
        (false, true) => FunnelSvgFrame::with_factories_and_stage_factory(
            config,
            projected,
            options,
            &mut base,
            &mut resolve_stage,
        ),
        (true, true) => FunnelSvgFrame::with_size_factories_and_stage_factory(
            config,
            projected,
            options,
            &mut resolve_size,
            &mut base,
            &mut resolve_stage,
        ),
    }
}
