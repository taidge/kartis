//! Owned application data alongside Waffle's validated numeric projection.
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

use crate::scene::ensure;
use crate::*;

/// The application chooses the numeric projection; arbitrary `data` is kept
/// intact, without cloning it for every group, cell, or Widget.
#[derive(Clone, Debug, PartialEq)]
pub struct WaffleDatum<D> {
    pub data: D,
    pub projected: RadialDatum,
}
impl<D> WaffleDatum<D> {
    pub fn new(data: D, projected: RadialDatum) -> Self {
        Self { data, projected }
    }
}

pub struct WaffleDataColorContext<'a, D> {
    pub data: &'a D,
    pub computed: WaffleGroupColorContext<'a>,
}
pub struct WaffleDataBorderContext<'a, D> {
    /// None is the empty style template, never a fabricated zero-valued datum.
    pub data: Option<&'a D>,
    pub computed: WaffleBorderColorContext<'a>,
}
pub struct WaffleDataCellContext<'a, D> {
    /// Empty cells have no source datum.
    pub data: Option<&'a D>,
    pub computed: WaffleCellContext<'a>,
}
pub struct WaffleDataCellWidgetContext<'a, D> {
    pub data: Option<&'a D>,
    pub computed: WaffleCellWidgetContext<'a>,
}

pub type WaffleDataColorFactory<'a, D> =
    dyn FnMut(&WaffleDataColorContext<'_, D>) -> Result<Color> + 'a;
pub type WaffleDataBorderFactory<'a, D> =
    dyn FnMut(&WaffleDataBorderContext<'_, D>) -> Result<Color> + 'a;
pub type WaffleDataCellFactory<'a, D> =
    dyn FnMut(&WaffleDataCellContext<'_, D>) -> Result<ScatterNodeGeometry> + 'a;

/// Borrowed roles for one build. No closure is stored in the resulting frame.
pub struct WaffleDataFactories<'a, D> {
    pub color: Option<&'a mut WaffleDataColorFactory<'a, D>>,
    pub formatted_value: Option<&'a mut WaffleValueFormatter<'a>>,
    pub border_color: Option<&'a mut WaffleDataBorderFactory<'a, D>>,
}
impl<D> Default for WaffleDataFactories<'_, D> {
    fn default() -> Self {
        Self {
            color: None,
            formatted_value: None,
            border_color: None,
        }
    }
}

enum Source<D> {
    // Moving a Vec into a frame preserves the addresses of its elements.
    Owned(Vec<WaffleDatum<D>>),
    Shared(Arc<[WaffleDatum<D>]>),
}
impl<D> Deref for Source<D> {
    type Target = [WaffleDatum<D>];

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(source) => source,
            Self::Shared(source) => source,
        }
    }
}

struct Data<D> {
    source: Source<D>,
    computed: WaffleSvgFrame,
}

/// A captured cohort. Equality is capture identity so raw-only updates remain
/// observable even when an application's own PartialEq omits extra fields or
/// treats positive and negative zero as equal. D needs no Clone/PartialEq bound.
pub struct WaffleDataFrame<D>(Arc<Data<D>>);
impl<D> Clone for WaffleDataFrame<D> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<D> PartialEq for WaffleDataFrame<D> {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl<D> Eq for WaffleDataFrame<D> {}
impl<D> fmt::Debug for WaffleDataFrame<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WaffleDataFrame")
            .field("source_len", &self.0.source.len())
            .field("layout", self.layout())
            .finish_non_exhaustive()
    }
}
impl<D> WaffleDataFrame<D> {
    pub fn new(
        config: &ChartConfig,
        source: Vec<WaffleDatum<D>>,
        options: &WaffleLayoutOptions,
    ) -> Result<Self> {
        Self::build(
            config,
            Source::Owned(source),
            options,
            &mut WaffleDataFactories::default(),
            None,
            false,
        )
    }
    pub fn from_shared(
        config: &ChartConfig,
        source: Arc<[WaffleDatum<D>]>,
        options: &WaffleLayoutOptions,
    ) -> Result<Self> {
        Self::with_shared_factories(config, source, options, &mut WaffleDataFactories::default())
    }
    pub fn with_factories(
        config: &ChartConfig,
        source: Vec<WaffleDatum<D>>,
        options: &WaffleLayoutOptions,
        factories: &mut WaffleDataFactories<'_, D>,
    ) -> Result<Self> {
        Self::build(
            config,
            Source::Owned(source),
            options,
            factories,
            None,
            false,
        )
    }
    pub fn with_shared_factories(
        config: &ChartConfig,
        source: Arc<[WaffleDatum<D>]>,
        options: &WaffleLayoutOptions,
        factories: &mut WaffleDataFactories<'_, D>,
    ) -> Result<Self> {
        Self::build(
            config,
            Source::Shared(source),
            options,
            factories,
            None,
            true,
        )
    }
    pub fn with_factories_and_cell_factory(
        config: &ChartConfig,
        source: Vec<WaffleDatum<D>>,
        options: &WaffleLayoutOptions,
        factories: &mut WaffleDataFactories<'_, D>,
        cell: &mut impl FnMut(&WaffleDataCellContext<'_, D>) -> Result<ScatterNodeGeometry>,
    ) -> Result<Self> {
        Self::build(
            config,
            Source::Owned(source),
            options,
            factories,
            Some(cell),
            false,
        )
    }
    pub fn with_shared_factories_and_cell_factory(
        config: &ChartConfig,
        source: Arc<[WaffleDatum<D>]>,
        options: &WaffleLayoutOptions,
        factories: &mut WaffleDataFactories<'_, D>,
        cell: &mut impl FnMut(&WaffleDataCellContext<'_, D>) -> Result<ScatterNodeGeometry>,
    ) -> Result<Self> {
        Self::build(
            config,
            Source::Shared(source),
            options,
            factories,
            Some(cell),
            true,
        )
    }
    fn build(
        config: &ChartConfig,
        source: Source<D>,
        options: &WaffleLayoutOptions,
        factories: &mut WaffleDataFactories<'_, D>,
        cell: Option<&mut WaffleDataCellFactory<'_, D>>,
        nivo: bool,
    ) -> Result<Self> {
        ensure(
            source.len() <= MAX_MARKS,
            "Waffle raw source budget exceeded",
        )?;
        let projected: Vec<_> = source.iter().map(|datum| datum.projected.clone()).collect();
        let computed = {
            let WaffleDataFactories {
                color,
                formatted_value,
                border_color,
            } = factories;
            let has_color = color.is_some();
            let has_formatter = formatted_value.is_some();
            let has_border = border_color.is_some();
            let mut resolve_color = |computed: &WaffleGroupColorContext<'_>| {
                let data = &source
                    .get(computed.source_index)
                    .ok_or_else(|| ChartError("Waffle color source index is missing".into()))?
                    .data;
                color.as_deref_mut().expect("configured Waffle raw color")(
                    &WaffleDataColorContext {
                        data,
                        computed: *computed,
                    },
                )
            };
            let mut format = |value| {
                formatted_value
                    .as_deref_mut()
                    .expect("configured Waffle raw formatter")(value)
            };
            let mut resolve_border = |computed: &WaffleBorderColorContext<'_>| {
                let data = match computed.target {
                    WaffleBorderTarget::Group(group) => Some(
                        &source
                            .get(group.source_index)
                            .ok_or_else(|| {
                                ChartError("Waffle border source index is missing".into())
                            })?
                            .data,
                    ),
                    WaffleBorderTarget::Empty(_) => None,
                };
                border_color
                    .as_deref_mut()
                    .expect("configured Waffle raw border")(
                    &WaffleDataBorderContext {
                        data,
                        computed: *computed,
                    },
                )
            };
            let mut base = WaffleFactories {
                color: if has_color {
                    Some(&mut resolve_color)
                } else {
                    None
                },
                formatted_value: if has_formatter {
                    Some(&mut format)
                } else {
                    None
                },
                border_color: if has_border {
                    Some(&mut resolve_border)
                } else {
                    None
                },
            };
            if let Some(cell) = cell {
                let mut customize = |computed: &WaffleCellContext<'_>| {
                    let data = computed.group.map(|g| &source[g.source_index].data);
                    cell(&WaffleDataCellContext {
                        data,
                        computed: *computed,
                    })
                };
                if nivo {
                    WaffleSvgFrame::with_factories_and_cell_factory(
                        config,
                        &projected,
                        options,
                        &mut base,
                        &mut customize,
                    )?
                } else {
                    WaffleSvgFrame::legacy(
                        config,
                        &projected,
                        options,
                        &mut base,
                        Some(&mut customize),
                    )?
                }
            } else {
                if nivo {
                    WaffleSvgFrame::with_factories(config, &projected, options, &mut base)?
                } else {
                    WaffleSvgFrame::legacy(config, &projected, options, &mut base, None)?
                }
            }
        };
        Ok(Self(Arc::new(Data { source, computed })))
    }
    pub fn source(&self) -> &[WaffleDatum<D>] {
        &self.0.source
    }
    pub fn data(&self, source_index: usize) -> Option<&D> {
        self.0.source.get(source_index).map(|d| &d.data)
    }
    pub fn layout(&self) -> &WaffleLayout {
        self.0.computed.layout()
    }
    pub fn scene(&self) -> &Scene {
        self.0.computed.scene()
    }
    pub fn interaction(&self) -> SceneInteraction {
        self.0.computed.interaction()
    }
    pub fn area_interaction(&self) -> Result<SceneInteraction> {
        waffle_area_interaction(self.layout())
    }
    pub fn computed(&self) -> &WaffleSvgFrame {
        &self.0.computed
    }
    pub fn config(&self) -> &ChartConfig {
        self.computed().config()
    }
    pub fn options(&self) -> &WaffleLayoutOptions {
        self.computed().options()
    }
    pub fn shares_source(&self, other: &Self) -> bool {
        match (&self.0.source, &other.0.source) {
            (Source::Shared(left), Source::Shared(right)) => Arc::ptr_eq(left, right),
            (Source::Owned(_), Source::Owned(_)) => Arc::ptr_eq(&self.0, &other.0),
            _ => false,
        }
    }
    #[cfg(all(feature = "web", target_arch = "wasm32"))]
    pub(crate) fn shared_source(&self) -> Option<Arc<[WaffleDatum<D>]>> {
        match &self.0.source {
            Source::Shared(source) => Some(source.clone()),
            Source::Owned(_) => None,
        }
    }
}

/// Owned typed Widget payload. The numeric role and raw cohort cannot be
/// independently replaced through its accessors. Empty cells retain None.
pub struct WaffleDataCellWidgetData<D> {
    computed: WaffleCellWidgetData,
    frame: WaffleDataFrame<D>,
}
impl<D> Clone for WaffleDataCellWidgetData<D> {
    fn clone(&self) -> Self {
        Self {
            computed: self.computed.clone(),
            frame: self.frame.clone(),
        }
    }
}
impl<D> PartialEq for WaffleDataCellWidgetData<D> {
    fn eq(&self, other: &Self) -> bool {
        self.frame == other.frame && self.computed == other.computed
    }
}
impl<D> fmt::Debug for WaffleDataCellWidgetData<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WaffleDataCellWidgetData")
            .field("computed", &self.computed)
            .field("frame", &self.frame)
            .finish()
    }
}
impl<D> WaffleDataCellWidgetData<D> {
    pub fn computed(&self) -> &WaffleCellWidgetData {
        &self.computed
    }
    pub fn frame(&self) -> &WaffleDataFrame<D> {
        &self.frame
    }
    pub fn data(&self) -> Option<&D> {
        self.computed
            .group
            .as_ref()
            .and_then(|group| self.frame.data(group.source_index))
    }
}

impl<D: 'static> WaffleDataFrame<D> {
    pub fn cell_widgets(
        &self,
        revision: u64,
        mut placement: impl FnMut(
            &WaffleDataCellWidgetContext<'_, D>,
        ) -> Result<Option<PlotWidgetPlacement>>,
    ) -> Result<PlotWidgetsData<WaffleDataCellWidgetData<D>>> {
        let packet = waffle_cell_widgets(self.layout(), revision, |computed| {
            let data = computed
                .group
                .and_then(|group| self.data(group.source_index));
            placement(&WaffleDataCellWidgetContext {
                data,
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
                        WaffleDataCellWidgetData {
                            computed,
                            frame: self.clone(),
                        },
                    )
                })
                .collect(),
        })
    }
}
