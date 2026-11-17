//! Typed application data carried by the same private source snapshot as SVG,
//! layer Cages, tooltip Cages and native event admissions.
use std::marker::PhantomData;
use std::sync::Arc;

use super::*;

#[derive(Clone)]
pub(crate) struct SourceEnvelope {
    pub projection: WaffleSvgFrame,
    raw: Option<Rc<dyn Any>>,
    kind: Option<TypeId>,
}
impl PartialEq for SourceEnvelope {
    fn eq(&self, other: &Self) -> bool {
        self.projection == other.projection
            && match (&self.raw, &other.raw) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}
impl SourceEnvelope {
    pub fn legacy(projection: WaffleSvgFrame) -> Self {
        Self {
            projection,
            raw: None,
            kind: None,
        }
    }
    fn typed<D: 'static>(frame: WaffleDataFrame<D>) -> Self {
        Self {
            projection: frame.computed().clone(),
            raw: Some(Rc::new(frame)),
            kind: Some(TypeId::of::<D>()),
        }
    }
    pub fn validate_type(&self, expected: Option<TypeId>) -> Result<()> {
        scene::ensure(
            self.kind == expected && self.raw.is_some() == expected.is_some(),
            "Waffle source data type differs from owner",
        )?;
        Ok(())
    }
    pub fn paint_scene(&self) -> &Scene {
        self.projection.scene()
    }
    pub fn kind(&self) -> Option<TypeId> {
        self.kind
    }
    pub fn typed_frame<D: 'static>(&self) -> Result<WaffleDataFrame<D>> {
        let frame = self
            .raw
            .as_ref()
            .and_then(|raw| raw.downcast_ref::<WaffleDataFrame<D>>())
            .ok_or_else(|| {
                ChartError("Waffle typed source is missing or has a different type".into())
            })?;
        scene::ensure(
            frame.computed() == &self.projection,
            "Waffle raw/projection cohort differs",
        )?;
        Ok(frame.clone())
    }
}

pub struct BrowserWaffleDataBuildContext<'a, D> {
    pub config: &'a ChartConfig,
    pub source: &'a Arc<[WaffleDatum<D>]>,
    pub options: &'a WaffleLayoutOptions,
}

fn clone_value<T: Clone>(value: &T) -> T {
    value.clone()
}

macro_rules! typed_frame {
    ($name:ident, $legacy:ty, {$($field:ident: $ty:ty),* $(,)?}) => {
        pub struct $name<D> { pub data: WaffleDataFrame<D>, $(pub $field: $ty,)* }
        impl<D> Clone for $name<D> {
            fn clone(&self) -> Self { Self { data: self.data.clone(), $($field: clone_value(&self.$field),)* } }
        }
        impl<D> PartialEq for $name<D> {
            fn eq(&self, other: &Self) -> bool { self.data == other.data $(&& self.$field == other.$field)* }
        }
        impl<D: 'static> $name<D> {
            pub(super) fn from_legacy(value: $legacy, source: &SourceEnvelope) -> Result<Self> {
                scene::ensure(value.data == source.projection, "Waffle typed presentation projection differs")?;
                Ok(Self { data: source.typed_frame()?, $($field: value.$field,)* })
            }
        }
    };
}
typed_frame!(BrowserWaffleDataSvgLayerFrame, BrowserWaffleSvgLayerFrame, {
    source_generation: u64, publication: u64, key: String, index: usize,
    presentation: WaffleSvgPlan, view: ZoomTransform, hidden: BTreeSet<String>, interactive: bool,
});
typed_frame!(BrowserWaffleDataSvgHit, BrowserWaffleSvgHit, {
    source_generation: u64, publication: u64, key: String, index: usize,
    polygon_index: usize, source_index: usize, area_id: String, client: [f64;2],
    viewport: [f64;2], scene: [f64;2], trusted: bool, native_type: String,
});
typed_frame!(BrowserWaffleDataMouseEvent, BrowserWaffleMouseEvent, {
    source_generation: u64, publication: u64, polygon_index: usize,
    source_index: usize, input: BrowserWaffleMouseInput,
});
typed_frame!(WaffleDataTooltipFrame, tooltip::WaffleTooltipFrame, {
    polygon_index: usize, source_index: usize, source_generation: u64, publication: u64,
    occurrence: BrowserWaffleMouseOccurrence, pointer_client: [f64;2], viewport_anchor: [f64;2],
    requested_anchor: tooltip::BrowserWaffleTooltipAnchor, anchor: tooltip::BrowserWaffleTooltipAnchor,
});
impl<D> WaffleDataTooltipFrame<D> {
    pub fn datum(&self) -> Result<&D> {
        self.data
            .data(self.source_index)
            .ok_or_else(|| ChartError("Waffle typed tooltip source is missing".into()))
    }
    pub fn group(&self) -> Result<&WaffleGroup> {
        self.data
            .computed()
            .layout()
            .groups
            .get(self.source_index)
            .ok_or_else(|| ChartError("Waffle typed tooltip part is missing".into()))
    }
    pub fn text(&self) -> Result<String> {
        let p = self.group()?;
        Ok(format!("{}: {}", p.source.label, p.formatted_value))
    }
}

pub struct BrowserWaffleDataSvgLayerContext<D: 'static> {
    pub key: String,
    pub revision: u64,
    pub initial: BrowserWaffleDataSvgLayerFrame<D>,
    pub frame: Bond<Option<BrowserWaffleDataSvgLayerFrame<D>>>,
}
pub struct BrowserWaffleDataTooltipComponentContext<D: 'static> {
    pub revision: u64,
    pub frame: Bond<Option<WaffleDataTooltipFrame<D>>>,
}
pub type WaffleDataMouseHandler<D> =
    Box<dyn FnMut(BrowserWaffleDataMouseEvent<D>) -> Result<()> + 'static>;
pub struct BrowserWaffleDataMouseHandlers<D> {
    pub on_mouse_enter: Option<WaffleDataMouseHandler<D>>,
    pub on_mouse_move: Option<WaffleDataMouseHandler<D>>,
    pub on_mouse_leave: Option<WaffleDataMouseHandler<D>>,
    pub on_click: Option<WaffleDataMouseHandler<D>>,
}

pub struct BrowserWaffleDataSnapshot<D> {
    pub active: bool,
    pub data: Option<WaffleDataFrame<D>>,
    pub presentation: Option<WaffleSvgPlan>,
    pub source_generation: u64,
    pub publication: u64,
    pub hovered: Option<String>,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub pending: bool,
    pub raf_pending: bool,
    pub error: Option<ChartError>,
    pub ui_errors: Vec<ChartError>,
    pub event_errors: Vec<ChartError>,
    pub last_hit: Option<BrowserWaffleDataSvgHit<D>>,
    pub tooltip: Option<WaffleDataTooltipFrame<D>>,
}

pub struct BrowserWaffleDataSvgProgram<D> {
    pub(super) inner: BrowserWaffleSvgProgram,
    marker: PhantomData<fn() -> D>,
}
impl<D> Clone for BrowserWaffleDataSvgProgram<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}
impl<D: 'static> BrowserWaffleDataSvgProgram<D> {
    pub fn new(layers: Vec<WaffleSvgLayer>) -> Result<Self> {
        Ok(Self {
            inner: BrowserWaffleSvgProgram::new(layers)?,
            marker: PhantomData,
        })
    }
    pub fn defaults() -> Self {
        Self {
            inner: BrowserWaffleSvgProgram::defaults(),
            marker: PhantomData,
        }
    }
    pub fn custom<W: glory::Widget + 'static>(
        mut self,
        registry: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(BrowserWaffleDataSvgLayerContext<D>) -> Result<W> + 'static,
    ) -> Result<Self> {
        self.inner =
            self.inner
                .custom_internal(registry, revision, Some(TypeId::of::<D>()), move |c| {
                    let initial = BrowserWaffleDataSvgLayerFrame::from_legacy(
                        c.initial.frame.clone(),
                        &c.initial.source,
                    )?;
                    let bond = c.scope.bond(move || {
                        c.frame.get().map(|value| {
                            BrowserWaffleDataSvgLayerFrame::from_legacy(value.frame, &value.source)
                                .expect("validated typed layer cohort")
                        })
                    });
                    Ok(Box::new(factory(BrowserWaffleDataSvgLayerContext {
                        key: c.key,
                        revision: c.revision,
                        initial,
                        frame: bond,
                    })?))
                })?;
        Ok(self)
    }
    pub fn with_budget(mut self, budget: WaffleSvgBudget) -> Result<Self> {
        self.inner = self.inner.with_budget(budget)?;
        Ok(self)
    }
    pub fn with_legends(mut self, legends: Vec<WaffleSvgLegendSpec>) -> Result<Self> {
        self.inner = self.inner.with_legends(legends)?;
        Ok(self)
    }
    pub fn interactive(mut self, value: bool) -> Self {
        self.inner = self.inner.interactive(value);
        self
    }
    pub fn layers(&self) -> &[WaffleSvgLayer] {
        self.inner.layers()
    }
    pub fn budget(&self) -> WaffleSvgBudget {
        self.inner.budget()
    }
}
impl<D: 'static> Default for BrowserWaffleDataSvgProgram<D> {
    fn default() -> Self {
        Self::defaults()
    }
}

pub struct BrowserWaffleData<D> {
    pub(super) inner: BrowserWaffle,
    marker: PhantomData<fn() -> D>,
}
pub struct BrowserWaffleDataHandle<D> {
    pub(super) inner: BrowserWaffleHandle,
    marker: PhantomData<fn() -> D>,
}
impl<D> Clone for BrowserWaffleDataHandle<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}
impl<D: 'static> BrowserWaffleData<D> {
    pub fn mount_with_svg_layers(
        root: Element,
        source: Vec<WaffleDatum<D>>,
        config: ChartConfig,
        options: WaffleLayoutOptions,
        program: BrowserWaffleDataSvgProgram<D>,
    ) -> Result<Self> {
        Self::mount_with_factory_and_svg_layers(root, source, config, options, program, |c| {
            WaffleDataFrame::from_shared(c.config, c.source.clone(), c.options)
        })
    }
    pub fn mount_with_factory_and_svg_layers(
        root: Element,
        source: Vec<WaffleDatum<D>>,
        config: ChartConfig,
        options: WaffleLayoutOptions,
        program: BrowserWaffleDataSvgProgram<D>,
        mut builder: impl FnMut(&BrowserWaffleDataBuildContext<'_, D>) -> Result<WaffleDataFrame<D>>
        + 'static,
    ) -> Result<Self> {
        let projected = source.iter().map(|d| d.projected.clone()).collect();
        let source: Arc<[WaffleDatum<D>]> = source.into();
        let inner = BrowserWaffle::mount_erased(
            root,
            projected,
            config,
            options,
            program.inner,
            RawMount {
                source: Some(Rc::new(source)),
                kind: Some(TypeId::of::<D>()),
            },
            move |c, raw| {
                let source = raw
                    .and_then(|raw| raw.downcast_ref::<Arc<[WaffleDatum<D>]>>())
                    .ok_or_else(|| ChartError("Waffle typed request type differs".into()))?;
                let frame = builder(&BrowserWaffleDataBuildContext {
                    config: c.config,
                    source,
                    options: c.options,
                })?;
                scene::ensure(
                    frame
                        .shared_source()
                        .is_some_and(|shared| Arc::ptr_eq(&shared, source)),
                    "Waffle typed builder returned another raw source",
                )?;
                Ok(SourceEnvelope::typed(frame))
            },
        )?;
        Ok(Self {
            inner,
            marker: PhantomData,
        })
    }
    pub fn handle(&self) -> BrowserWaffleDataHandle<D> {
        BrowserWaffleDataHandle {
            inner: self.inner.handle(),
            marker: PhantomData,
        }
    }
    pub fn snapshot(&self) -> Result<BrowserWaffleDataSnapshot<D>> {
        self.handle().snapshot()
    }
    pub fn flush(&self) -> Result<()> {
        self.inner.flush()
    }
    pub fn stop(&self) {
        self.inner.stop();
    }
    pub fn set_source(&self, v: Vec<WaffleDatum<D>>) -> Result<()> {
        self.handle().set_source(v)
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.handle().set_config(v)
    }
    pub fn set_options(&self, v: WaffleLayoutOptions) -> Result<()> {
        self.handle().set_options(v)
    }
    pub fn set_svg_program(&self, v: BrowserWaffleDataSvgProgram<D>) -> Result<()> {
        self.handle().set_svg_program(v)
    }
    pub fn set_view(&self, v: ZoomTransform) -> Result<()> {
        self.handle().set_view(v)
    }
    pub fn set_hidden(&self, v: BTreeSet<String>) -> Result<()> {
        self.handle().set_hidden(v)
    }
    pub fn resize(&self, w: f64, h: f64) -> Result<()> {
        self.handle().resize(w, h)
    }
    pub fn refresh(&self) -> Result<()> {
        self.handle().refresh()
    }
    pub fn source_generation(&self) -> Result<u64> {
        self.handle().source_generation()
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserWaffleDataSvgLayerFrame<D>>> {
        self.handle().svg_layer_frame(key)
    }
    pub fn set_mouse_handlers(&self, v: BrowserWaffleDataMouseHandlers<D>) -> Result<()> {
        self.handle().set_mouse_handlers(v)
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.handle().clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, v: tooltip::BrowserWaffleTooltipOptions) -> Result<()> {
        self.handle().set_tooltip_options(v)
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserWaffleDataTooltipComponentContext<D>) -> Result<W> + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component(revision, factory)
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        self.handle().clear_tooltip_component()
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.handle().reset_tooltip()
    }
    pub fn tooltip(&self) -> Result<Option<WaffleDataTooltipFrame<D>>> {
        self.handle().tooltip()
    }
}
impl<D: 'static> BrowserWaffleDataHandle<D> {
    fn owner(&self) -> Result<Rc<Driver>> {
        let owner = self.inner.owner()?;
        scene::ensure(
            owner.raw_type == Some(TypeId::of::<D>()),
            "Waffle typed handle data type differs",
        )?;
        Ok(owner)
    }
    pub fn snapshot(&self) -> Result<BrowserWaffleDataSnapshot<D>> {
        let owner = self.owner()?;
        let legacy = owner.snapshot();
        let data = owner
            .accepted
            .borrow()
            .as_ref()
            .map(|a| a.source.typed_frame())
            .transpose()?;
        let last_hit = owner
            .last_hit
            .borrow()
            .as_ref()
            .map(|(hit, source)| BrowserWaffleDataSvgHit::from_legacy(hit.clone(), source))
            .transpose()?;
        Ok(BrowserWaffleDataSnapshot {
            active: legacy.active,
            data,
            presentation: legacy.presentation,
            source_generation: legacy.source_generation,
            publication: legacy.publication,
            hovered: legacy.hovered,
            view: legacy.view,
            hidden: legacy.hidden,
            pending: legacy.pending,
            raf_pending: legacy.raf_pending,
            error: legacy.error,
            ui_errors: legacy.ui_errors,
            event_errors: legacy.event_errors,
            last_hit,
            tooltip: self.tooltip()?,
        })
    }
    pub fn set_source(&self, source: Vec<WaffleDatum<D>>) -> Result<()> {
        let projected = source.iter().map(|d| d.projected.clone()).collect();
        let source: Arc<[WaffleDatum<D>]> = source.into();
        self.owner()?
            .queue(Command::TypedSource(projected, Rc::new(source)))
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.inner.set_config(v)
    }
    pub fn set_options(&self, v: WaffleLayoutOptions) -> Result<()> {
        self.inner.set_options(v)
    }
    pub fn set_svg_program(&self, v: BrowserWaffleDataSvgProgram<D>) -> Result<()> {
        self.inner.set_svg_program(v.inner)
    }
    pub fn set_view(&self, v: ZoomTransform) -> Result<()> {
        self.inner.set_view(v)
    }
    pub fn set_hidden(&self, v: BTreeSet<String>) -> Result<()> {
        self.inner.set_hidden(v)
    }
    pub fn resize(&self, w: f64, h: f64) -> Result<()> {
        self.inner.resize(w, h)
    }
    pub fn refresh(&self) -> Result<()> {
        self.inner.refresh()
    }
    pub fn flush(&self) -> Result<()> {
        self.inner.flush()
    }
    pub fn stop(&self) {
        self.inner.stop()
    }
    pub fn source_generation(&self) -> Result<u64> {
        self.inner.source_generation()
    }
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserWaffleDataSvgLayerFrame<D>>> {
        let owner = self.owner()?;
        owner.require_alive()?;
        owner
            .accepted
            .borrow()
            .as_ref()
            .and_then(|a| {
                a.stage
                    .frames
                    .get(key)
                    .map(|frame| (frame.clone(), a.source.clone()))
            })
            .map(|(frame, source)| BrowserWaffleDataSvgLayerFrame::from_legacy(frame, &source))
            .transpose()
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserWaffleDataMouseHandlers<D>) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        owner.events.replace_typed(handlers);
        Ok(())
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.inner.clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, v: tooltip::BrowserWaffleTooltipOptions) -> Result<()> {
        self.inner.set_tooltip_options(v)
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        self.inner.clear_tooltip_component()
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.inner.reset_tooltip()
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        mut factory: impl FnMut(BrowserWaffleDataTooltipComponentContext<D>) -> Result<W> + 'static,
    ) -> Result<()> {
        let owner = self.owner()?;
        let mut config = owner.tooltip_config()?;
        config.options.get_or_insert_with(Default::default);
        config.kind = Some(TypeId::of::<D>());
        config.component = Some(tooltip::Component::new(
            revision,
            move |c: tooltip::InternalContext| {
                let source = c.frame;
                let frame = c.scope()?.bond(move || {
                    source.get().map(|value| {
                        WaffleDataTooltipFrame::from_legacy(value.frame, &value.source)
                            .expect("validated typed tooltip cohort")
                    })
                });
                factory(BrowserWaffleDataTooltipComponentContext {
                    revision: c.revision,
                    frame,
                })
            },
        ));
        owner.queue(Command::Tooltip(config))
    }
    pub fn tooltip(&self) -> Result<Option<WaffleDataTooltipFrame<D>>> {
        let owner = self.owner()?;
        if !owner.alive.get() {
            return Ok(None);
        }
        owner
            .tooltip
            .internal_frame()
            .map(|value| WaffleDataTooltipFrame::from_legacy(value.frame, &value.source))
            .transpose()
    }
}

pub(super) type ErasedMouseHandler =
    Box<dyn FnMut(BrowserWaffleMouseEvent, &SourceEnvelope) -> Result<()> + 'static>;
impl<D> Default for BrowserWaffleDataMouseHandlers<D> {
    fn default() -> Self {
        Self {
            on_mouse_enter: None,
            on_mouse_move: None,
            on_mouse_leave: None,
            on_click: None,
        }
    }
}
impl<D: 'static> BrowserWaffleDataMouseHandlers<D> {
    pub(super) fn erase(self) -> BTreeMap<BrowserWaffleMouseKind, ErasedMouseHandler> {
        let mut slots: BTreeMap<BrowserWaffleMouseKind, ErasedMouseHandler> = BTreeMap::new();
        for (kind, slot) in [
            (BrowserWaffleMouseKind::Enter, self.on_mouse_enter),
            (BrowserWaffleMouseKind::Move, self.on_mouse_move),
            (BrowserWaffleMouseKind::Leave, self.on_mouse_leave),
            (BrowserWaffleMouseKind::Click, self.on_click),
        ] {
            if let Some(mut callback) = slot {
                slots.insert(
                    kind,
                    Box::new(move |event, source| {
                        callback(BrowserWaffleDataMouseEvent::from_legacy(event, source)?)
                    }),
                );
            }
        }
        slots
    }
}
