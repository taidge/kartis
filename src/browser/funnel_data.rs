//! Typed application data carried by the same private source snapshot as SVG,
//! layer Cages, tooltip Cages and native event admissions.
use std::marker::PhantomData;
use std::sync::Arc;

use super::*;

#[derive(Clone)]
pub(crate) struct SourceEnvelope {
    pub projection: FunnelSvgFrame,
    raw: Option<Rc<dyn Any>>,
    kind: Option<TypeId>,
    pub motion: Option<BrowserFunnelMotionFrame>,
}
impl PartialEq for SourceEnvelope {
    fn eq(&self, other: &Self) -> bool {
        self.projection == other.projection
            && self.motion == other.motion
            && match (&self.raw, &other.raw) {
                (None, None) => true,
                (Some(a), Some(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}
impl SourceEnvelope {
    pub fn legacy(projection: FunnelSvgFrame) -> Self {
        Self {
            projection,
            raw: None,
            kind: None,
            motion: None,
        }
    }
    fn typed<D: 'static>(frame: FunnelDataFrame<D>) -> Self {
        Self {
            projection: frame.computed().clone(),
            raw: Some(Rc::new(frame)),
            kind: Some(TypeId::of::<D>()),
            motion: None,
        }
    }
    pub fn validate_type(&self, expected: Option<TypeId>) -> Result<()> {
        scene::ensure(
            self.kind == expected && self.raw.is_some() == expected.is_some(),
            "Funnel source data type differs from owner",
        )?;
        scene::ensure(
            self.motion
                .as_ref()
                .is_none_or(|frame| frame.data.target() == &self.projection),
            "Funnel motion target differs from source",
        )
    }
    pub fn paint_scene(&self) -> &Scene {
        self.motion
            .as_ref()
            .map_or_else(|| self.projection.scene(), |frame| frame.data.scene())
    }
    pub fn kind(&self) -> Option<TypeId> {
        self.kind
    }
    pub fn typed_frame<D: 'static>(&self) -> Result<FunnelDataFrame<D>> {
        let frame = self
            .raw
            .as_ref()
            .and_then(|raw| raw.downcast_ref::<FunnelDataFrame<D>>())
            .ok_or_else(|| {
                ChartError("Funnel typed source is missing or has a different type".into())
            })?;
        scene::ensure(
            frame.computed() == &self.projection,
            "Funnel raw/projection cohort differs",
        )?;
        Ok(frame.clone())
    }
}

pub struct BrowserFunnelDataBuildContext<'a, D> {
    pub config: &'a ChartConfig,
    pub source: &'a Arc<[FunnelDataDatum<D>]>,
    pub options: &'a FunnelLayoutOptions,
}

fn clone_value<T: Clone>(value: &T) -> T {
    value.clone()
}

macro_rules! typed_frame {
    ($name:ident, $legacy:ty, {$($field:ident: $ty:ty),* $(,)?}) => {
        pub struct $name<D> { pub data: FunnelDataFrame<D>, $(pub $field: $ty,)* }
        impl<D> Clone for $name<D> {
            fn clone(&self) -> Self { Self { data: self.data.clone(), $($field: clone_value(&self.$field),)* } }
        }
        impl<D> PartialEq for $name<D> {
            fn eq(&self, other: &Self) -> bool { self.data == other.data $(&& self.$field == other.$field)* }
        }
        impl<D: 'static> $name<D> {
            pub(super) fn from_legacy(value: $legacy, source: &SourceEnvelope) -> Result<Self> {
                scene::ensure(value.data == source.projection, "Funnel typed presentation projection differs")?;
                Ok(Self { data: source.typed_frame()?, $($field: value.$field,)* })
            }
        }
    };
}
typed_frame!(BrowserFunnelDataSvgLayerFrame, BrowserFunnelSvgLayerFrame, {
    source_generation: u64, publication: u64, key: String, index: usize,
    view: ZoomTransform, hidden: BTreeSet<String>, interactive: bool,
});
typed_frame!(BrowserFunnelDataSvgHit, BrowserFunnelSvgHit, {
    source_generation: u64, publication: u64, key: String, index: usize,
    part_index: usize, source_index: usize, mark_id: String, client: [f64;2],
    viewport: [f64;2], scene: [f64;2], trusted: bool, native_type: String,
});
typed_frame!(BrowserFunnelDataMouseEvent, BrowserFunnelMouseEvent, {
    source_generation: u64, publication: u64, part_index: usize,
    source_index: usize, input: BrowserFunnelMouseInput,
});
typed_frame!(FunnelDataTooltipFrame, tooltip::FunnelTooltipFrame, {
    part_index: usize, source_index: usize, source_generation: u64, publication: u64,
    occurrence: BrowserFunnelMouseOccurrence, pointer_client: [f64;2], viewport_anchor: [f64;2],
    requested_anchor: tooltip::BrowserFunnelTooltipAnchor, anchor: tooltip::BrowserFunnelTooltipAnchor,
});
impl<D> FunnelDataTooltipFrame<D> {
    pub fn datum(&self) -> Result<&D> {
        self.data
            .data(self.source_index)
            .ok_or_else(|| ChartError("Funnel typed tooltip source is missing".into()))
    }
    pub fn part(&self) -> Result<&FunnelPart> {
        self.data
            .computed()
            .parts()
            .get(self.part_index)
            .ok_or_else(|| ChartError("Funnel typed tooltip part is missing".into()))
    }
    pub fn style(&self) -> Result<&FunnelComputedPartStyle> {
        self.data
            .computed()
            .styles()
            .get(self.part_index)
            .ok_or_else(|| ChartError("Funnel typed tooltip style is missing".into()))
    }
    pub fn text(&self) -> Result<String> {
        let p = self.part()?;
        Ok(format!("{}: {}", p.datum.label, p.formatted_value))
    }
}

pub struct BrowserFunnelDataSvgLayerContext<D: 'static> {
    pub key: String,
    pub revision: u64,
    pub initial: BrowserFunnelDataSvgLayerFrame<D>,
    pub frame: Bond<Option<BrowserFunnelDataSvgLayerFrame<D>>>,
    pub current: BrowserFunnelCurrentPart,
}
pub struct BrowserFunnelDataTooltipComponentContext<D: 'static> {
    pub revision: u64,
    pub frame: Bond<Option<FunnelDataTooltipFrame<D>>>,
}
pub type FunnelDataMouseHandler<D> =
    Box<dyn FnMut(BrowserFunnelDataMouseEvent<D>) -> Result<()> + 'static>;
pub struct BrowserFunnelDataMouseHandlers<D> {
    pub on_mouse_enter: Option<FunnelDataMouseHandler<D>>,
    pub on_mouse_move: Option<FunnelDataMouseHandler<D>>,
    pub on_mouse_leave: Option<FunnelDataMouseHandler<D>>,
    pub on_click: Option<FunnelDataMouseHandler<D>>,
    pub activity: BrowserFunnelMouseActivity,
}

pub struct BrowserFunnelDataSnapshot<D> {
    pub active: bool,
    pub data: Option<FunnelDataFrame<D>>,
    pub source_generation: u64,
    pub publication: u64,
    pub current: Option<CategoryKey>,
    pub view: ZoomTransform,
    pub hidden: BTreeSet<String>,
    pub pending: bool,
    pub raf_pending: bool,
    pub error: Option<ChartError>,
    pub ui_errors: Vec<ChartError>,
    pub event_errors: Vec<ChartError>,
    pub last_hit: Option<BrowserFunnelDataSvgHit<D>>,
    pub tooltip: Option<FunnelDataTooltipFrame<D>>,
}

pub struct BrowserFunnelDataSvgProgram<D> {
    pub(super) inner: BrowserFunnelSvgProgram,
    marker: PhantomData<fn() -> D>,
}
impl<D> Clone for BrowserFunnelDataSvgProgram<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}
impl<D: 'static> BrowserFunnelDataSvgProgram<D> {
    pub fn new(layers: Vec<FunnelSvgLayer>) -> Result<Self> {
        Ok(Self {
            inner: BrowserFunnelSvgProgram::new(layers)?,
            marker: PhantomData,
        })
    }
    pub fn defaults() -> Self {
        Self {
            inner: BrowserFunnelSvgProgram::defaults(),
            marker: PhantomData,
        }
    }
    pub fn custom<W: glory::Widget + 'static>(
        mut self,
        registry: impl Into<String>,
        revision: u64,
        mut factory: impl FnMut(BrowserFunnelDataSvgLayerContext<D>) -> Result<W> + 'static,
    ) -> Result<Self> {
        self.inner =
            self.inner
                .custom_internal(registry, revision, Some(TypeId::of::<D>()), move |c| {
                    let initial = BrowserFunnelDataSvgLayerFrame::from_legacy(
                        c.initial.frame.clone(),
                        &c.initial.source,
                    )?;
                    let bond = c.scope.bond(move || {
                        c.frame.get().map(|value| {
                            BrowserFunnelDataSvgLayerFrame::from_legacy(value.frame, &value.source)
                                .expect("validated typed layer cohort")
                        })
                    });
                    Ok(Box::new(factory(BrowserFunnelDataSvgLayerContext {
                        key: c.key,
                        revision: c.revision,
                        initial,
                        frame: bond,
                        current: c.current,
                    })?))
                })?;
        Ok(self)
    }
    pub fn with_budget(mut self, budget: FunnelSvgBudget) -> Result<Self> {
        self.inner = self.inner.with_budget(budget)?;
        Ok(self)
    }
    pub fn with_extras(mut self, policy: FunnelSvgExtrasPolicy) -> Self {
        self.inner = self.inner.with_extras(policy);
        self
    }
    pub fn interactive(mut self, value: bool) -> Self {
        self.inner = self.inner.interactive(value);
        self
    }
    pub fn layers(&self) -> &[FunnelSvgLayer] {
        self.inner.layers()
    }
    pub fn budget(&self) -> FunnelSvgBudget {
        self.inner.budget()
    }
}
impl<D: 'static> Default for BrowserFunnelDataSvgProgram<D> {
    fn default() -> Self {
        Self::defaults()
    }
}

pub struct BrowserFunnelData<D> {
    pub(super) inner: BrowserFunnel,
    marker: PhantomData<fn() -> D>,
}
pub struct BrowserFunnelDataHandle<D> {
    pub(super) inner: BrowserFunnelHandle,
    marker: PhantomData<fn() -> D>,
}
impl<D> Clone for BrowserFunnelDataHandle<D> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            marker: PhantomData,
        }
    }
}
impl<D: 'static> BrowserFunnelData<D> {
    pub fn mount_with_svg_layers(
        root: Element,
        source: Vec<FunnelDataDatum<D>>,
        config: ChartConfig,
        options: FunnelLayoutOptions,
        program: BrowserFunnelDataSvgProgram<D>,
    ) -> Result<Self> {
        Self::mount_with_factory_and_svg_layers(root, source, config, options, program, |c| {
            FunnelDataFrame::from_shared(c.config, c.source.clone(), c.options)
        })
    }
    pub fn mount_with_factory_and_svg_layers(
        root: Element,
        source: Vec<FunnelDataDatum<D>>,
        config: ChartConfig,
        options: FunnelLayoutOptions,
        program: BrowserFunnelDataSvgProgram<D>,
        mut builder: impl FnMut(&BrowserFunnelDataBuildContext<'_, D>) -> Result<FunnelDataFrame<D>>
        + 'static,
    ) -> Result<Self> {
        let projected = source.iter().map(|d| d.projected.clone()).collect();
        let source: Arc<[FunnelDataDatum<D>]> = source.into();
        let inner = BrowserFunnel::mount_erased(
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
                    .and_then(|raw| raw.downcast_ref::<Arc<[FunnelDataDatum<D>]>>())
                    .ok_or_else(|| ChartError("Funnel typed request type differs".into()))?;
                let frame = builder(&BrowserFunnelDataBuildContext {
                    config: c.config,
                    source,
                    options: c.options,
                })?;
                scene::ensure(
                    Arc::ptr_eq(&frame.shared_source(), source),
                    "Funnel typed builder returned another raw source",
                )?;
                Ok(SourceEnvelope::typed(frame))
            },
        )?;
        Ok(Self {
            inner,
            marker: PhantomData,
        })
    }
    pub fn handle(&self) -> BrowserFunnelDataHandle<D> {
        BrowserFunnelDataHandle {
            inner: self.inner.handle(),
            marker: PhantomData,
        }
    }
    pub fn snapshot(&self) -> Result<BrowserFunnelDataSnapshot<D>> {
        self.handle().snapshot()
    }
    pub fn flush(&self) -> Result<()> {
        self.inner.flush()
    }
    pub fn stop(&self) {
        self.inner.stop();
    }
    pub fn set_source(&self, v: Vec<FunnelDataDatum<D>>) -> Result<()> {
        self.handle().set_source(v)
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.handle().set_config(v)
    }
    pub fn set_options(&self, v: FunnelLayoutOptions) -> Result<()> {
        self.handle().set_options(v)
    }
    pub fn set_svg_program(&self, v: BrowserFunnelDataSvgProgram<D>) -> Result<()> {
        self.handle().set_svg_program(v)
    }
    pub fn set_current_part(&self, id: Option<&CategoryKey>) -> Result<()> {
        self.handle().set_current_part(id)
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
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserFunnelDataSvgLayerFrame<D>>> {
        self.handle().svg_layer_frame(key)
    }
    pub fn set_mouse_handlers(&self, v: BrowserFunnelDataMouseHandlers<D>) -> Result<()> {
        self.handle().set_mouse_handlers(v)
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.handle().clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, v: tooltip::BrowserFunnelTooltipOptions) -> Result<()> {
        self.handle().set_tooltip_options(v)
    }
    pub fn set_tooltip_component<W: glory::Widget + 'static>(
        &self,
        revision: u64,
        factory: impl FnMut(BrowserFunnelDataTooltipComponentContext<D>) -> Result<W> + 'static,
    ) -> Result<()> {
        self.handle().set_tooltip_component(revision, factory)
    }
    pub fn clear_tooltip_component(&self) -> Result<()> {
        self.handle().clear_tooltip_component()
    }
    pub fn reset_tooltip(&self) -> Result<()> {
        self.handle().reset_tooltip()
    }
    pub fn tooltip(&self) -> Result<Option<FunnelDataTooltipFrame<D>>> {
        self.handle().tooltip()
    }
}
impl<D: 'static> BrowserFunnelDataHandle<D> {
    fn owner(&self) -> Result<Rc<Driver>> {
        let owner = self.inner.owner()?;
        scene::ensure(
            owner.raw_type == Some(TypeId::of::<D>()),
            "Funnel typed handle data type differs",
        )?;
        Ok(owner)
    }
    pub fn snapshot(&self) -> Result<BrowserFunnelDataSnapshot<D>> {
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
            .map(|(hit, source)| BrowserFunnelDataSvgHit::from_legacy(hit.clone(), source))
            .transpose()?;
        Ok(BrowserFunnelDataSnapshot {
            active: legacy.active,
            data,
            source_generation: legacy.source_generation,
            publication: legacy.publication,
            current: legacy.current,
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
    pub fn set_source(&self, source: Vec<FunnelDataDatum<D>>) -> Result<()> {
        let projected = source.iter().map(|d| d.projected.clone()).collect();
        let source: Arc<[FunnelDataDatum<D>]> = source.into();
        self.owner()?
            .queue(Command::TypedSource(projected, Rc::new(source)))
    }
    pub fn set_config(&self, v: ChartConfig) -> Result<()> {
        self.inner.set_config(v)
    }
    pub fn set_options(&self, v: FunnelLayoutOptions) -> Result<()> {
        self.inner.set_options(v)
    }
    pub fn set_svg_program(&self, v: BrowserFunnelDataSvgProgram<D>) -> Result<()> {
        self.inner.set_svg_program(v.inner)
    }
    pub fn set_current_part(&self, id: Option<&CategoryKey>) -> Result<()> {
        self.inner.set_current_part(id)
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
    pub fn svg_layer_frame(&self, key: &str) -> Result<Option<BrowserFunnelDataSvgLayerFrame<D>>> {
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
            .map(|(frame, source)| BrowserFunnelDataSvgLayerFrame::from_legacy(frame, &source))
            .transpose()
    }
    pub fn set_mouse_handlers(&self, handlers: BrowserFunnelDataMouseHandlers<D>) -> Result<()> {
        let owner = self.owner()?;
        owner.require_mutable()?;
        owner.events.replace_typed(handlers);
        Ok(())
    }
    pub fn clear_mouse_handlers(&self) -> Result<()> {
        self.inner.clear_mouse_handlers()
    }
    pub fn set_tooltip_options(&self, v: tooltip::BrowserFunnelTooltipOptions) -> Result<()> {
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
        mut factory: impl FnMut(BrowserFunnelDataTooltipComponentContext<D>) -> Result<W> + 'static,
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
                        FunnelDataTooltipFrame::from_legacy(value.frame, &value.source)
                            .expect("validated typed tooltip cohort")
                    })
                });
                factory(BrowserFunnelDataTooltipComponentContext {
                    revision: c.revision,
                    frame,
                })
            },
        ));
        owner.queue(Command::TooltipConfig(config))
    }
    pub fn tooltip(&self) -> Result<Option<FunnelDataTooltipFrame<D>>> {
        let owner = self.owner()?;
        if !owner.alive.get() {
            return Ok(None);
        }
        owner
            .tooltip
            .internal_frame()
            .map(|value| FunnelDataTooltipFrame::from_legacy(value.frame, &value.source))
            .transpose()
    }
}

pub(super) type ErasedMouseHandler =
    Box<dyn FnMut(BrowserFunnelMouseEvent, &SourceEnvelope) -> Result<()> + 'static>;
impl<D> Default for BrowserFunnelDataMouseHandlers<D> {
    fn default() -> Self {
        Self {
            on_mouse_enter: None,
            on_mouse_move: None,
            on_mouse_leave: None,
            on_click: None,
            activity: BrowserFunnelMouseActivity::default(),
        }
    }
}
impl<D: 'static> BrowserFunnelDataMouseHandlers<D> {
    pub(super) fn erase(
        self,
    ) -> (
        BrowserFunnelMouseActivity,
        BTreeMap<BrowserFunnelMouseKind, ErasedMouseHandler>,
    ) {
        let mut slots: BTreeMap<BrowserFunnelMouseKind, ErasedMouseHandler> = BTreeMap::new();
        for (kind, slot) in [
            (BrowserFunnelMouseKind::Enter, self.on_mouse_enter),
            (BrowserFunnelMouseKind::Move, self.on_mouse_move),
            (BrowserFunnelMouseKind::Leave, self.on_mouse_leave),
            (BrowserFunnelMouseKind::Click, self.on_click),
        ] {
            if let Some(mut callback) = slot {
                slots.insert(
                    kind,
                    Box::new(move |event, source| {
                        callback(BrowserFunnelDataMouseEvent::from_legacy(event, source)?)
                    }),
                );
            }
        }
        (self.activity, slots)
    }
}
