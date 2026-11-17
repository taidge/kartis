use std::collections::BTreeMap;

use glory::Widget;
use glory::style::Profile;

use super::*;
use crate::widget::plot_widgets::composition::{CompositionManager, CompositionStage};
use crate::widget::plot_widgets::{
    PlotWidgetFactory, PlotWidgetsPacket, PlotWidgetsState, PreparedSlot, SlotSpace,
    composition_factory, overlay_style, plain_factory, prepare_in_space_packet,
    scene_composition_factory, typed_factory,
};

mod masks;
pub(super) mod svg_occurrences;
use masks::MaskLayer;

pub(super) struct Mount {
    pub layout: PlotWidgetsPacket,
    pub factory: PlotWidgetFactory,
}

pub(super) struct Prepared {
    rows: Vec<PreparedSlot>,
    hidden: BTreeSet<String>,
    composed: bool,
    ordered: bool,
}

pub(super) struct OrderedFrame {
    paint: Rc<super::ordered_widgets::PaintStage>,
    widgets: Rc<CompositionStage>,
    rows: Vec<PreparedSlot>,
}

pub(super) fn prepare(
    model: &SurfaceModel,
    packet: &PlotWidgetsPacket,
    require_current_data: bool,
) -> Result<Prepared> {
    packet.validate()?;
    let layout = packet.scene_layout();
    scene::ensure(
        layout.slots.len() <= MAX_PLOT_WIDGET_SLOTS,
        "plot Widget slot budget exceeded",
    )?;
    let interaction = model.interaction_data();
    let values = if let Some(binding) = interaction {
        let requested: BTreeSet<_> = layout
            .slots
            .iter()
            .filter_map(|slot| match &slot.target {
                SceneWidgetTarget::Mark(id) => Some(id.as_str()),
                SceneWidgetTarget::Label(_) => None,
            })
            .collect();
        binding
            .sites
            .iter()
            .filter(|site| requested.contains(site.mark_id.as_str()))
            .filter_map(|site| Some((site.mark_id.as_str(), site.value?)))
            .collect::<BTreeMap<_, _>>()
    } else {
        BTreeMap::new()
    };
    let rows = prepare_in_space_packet(
        model.scene(),
        packet,
        Profile::HTML,
        SlotSpace {
            plot: Some(model.plot()),
            transform: model.paint_transform(),
            responsive: true,
            require_current_data: require_current_data || packet.is_composition(),
        },
        |mark| match interaction {
            Some(binding) => binding.anchor_for(mark),
            None => Ok(crate::crosshair::mark_center(mark)),
        },
        |mark| values.get(mark.id.as_str()).copied(),
    )?;
    Ok(Prepared {
        rows,
        hidden: model.hidden().clone(),
        composed: packet.is_composition(),
        ordered: packet.is_source_ordered(),
    })
}

pub(super) struct Host {
    web: WebApp,
    masks: MaskLayer,
    rows: Option<Cage<Vec<PreparedSlot>>>,
    hidden: Option<Cage<BTreeSet<String>>>,
    composition: Option<CompositionManager>,
    ordered: Option<super::ordered_widgets::OrderedPaint>,
    accepted_masks: RefCell<Vec<PreparedSlot>>,
    errors: Rc<RefCell<Vec<ChartError>>>,
    alive: Cell<bool>,
    publishing: Cell<bool>,
}

impl Host {
    pub fn has_ordered_frame(&self) -> bool {
        self.ordered
            .as_ref()
            .is_some_and(|ordered| ordered.has_frame())
    }
    pub fn prepare_ordered(
        &self,
        request: super::ordered_widgets::Request,
        prepared: Prepared,
        stage: Option<Rc<CompositionStage>>,
    ) -> Result<OrderedFrame> {
        scene::ensure(self.alive.get(), "ordered Widget host retired")?;
        let manager = self
            .composition
            .as_ref()
            .ok_or_else(|| ChartError("ordered Widget manager missing".into()))?;
        let widgets = match stage {
            Some(stage) => manager.retarget(&stage, prepared.rows.clone(), prepared.hidden)?,
            None => manager.prepare(prepared.rows.clone(), prepared.hidden)?,
        };
        let ordered = self
            .ordered
            .as_ref()
            .ok_or_else(|| ChartError("ordered paint host missing".into()))?;
        let paint = ordered.prepare(&self.web, request)?;
        Ok(OrderedFrame {
            paint,
            widgets,
            rows: prepared.rows,
        })
    }
    pub fn commit_ordered(&self, frame: OrderedFrame) -> Result<Vec<ChartError>> {
        scene::ensure(self.alive.get(), "ordered Widget host retired")?;
        self.masks.publish(&frame.rows)?;
        self.publishing.set(true);
        let result = self.ordered.as_ref().expect("ordered host").commit(
            self.composition.as_ref().expect("ordered manager"),
            &frame.widgets,
            frame.paint,
        );
        if let Err(error) = result {
            let _ = self.masks.publish(&self.accepted_masks.borrow());
            self.publishing.set(false);
            return Err(error);
        }
        *self.accepted_masks.borrow_mut() = frame.rows;
        // Dynamic child bindings are ordinary post-commit UI observations.
        let errors = match self.web.app().pump() {
            Ok(_) => vec![],
            Err(error) => vec![host(error)],
        };
        self.publishing.set(false);
        Ok(errors)
    }
    pub fn commit_ordered_with_adopt(
        &self,
        frame: OrderedFrame,
        adopt: &mut dyn FnMut(),
    ) -> Result<Vec<ChartError>> {
        scene::ensure(self.alive.get(), "ordered Widget host retired")?;
        self.masks.publish(&frame.rows)?;
        self.publishing.set(true);
        let result = self
            .ordered
            .as_ref()
            .expect("ordered host")
            .commit_with_adopt(
                self.composition.as_ref().expect("ordered manager"),
                &frame.widgets,
                frame.paint,
                adopt,
            );
        if let Err(error) = result {
            let _ = self.masks.publish(&self.accepted_masks.borrow());
            self.publishing.set(false);
            return Err(error);
        }
        *self.accepted_masks.borrow_mut() = frame.rows;
        // Dynamic child bindings are ordinary post-commit UI observations.
        let errors = match self.web.app().pump() {
            Ok(_) => vec![],
            Err(error) => vec![host(error)],
        };
        self.publishing.set(false);
        Ok(errors)
    }
    pub fn flush_cleanup(&self) -> Result<()> {
        if self.alive.get() {
            self.web.app().pump().map(|_| ()).map_err(host)
        } else {
            Ok(())
        }
    }

    pub fn launch(
        root: Element,
        scene: &Scene,
        factory: PlotWidgetFactory,
        initial: Prepared,
    ) -> Result<Self> {
        let ordered_mode = initial.ordered;
        let initial_masks = initial.rows.clone();
        let mut style = overlay_style(scene, true);
        if ordered_mode {
            style.opacity = Some(0.);
        }

        let (masks, content) = MaskLayer::mount(&root)?;
        masks.publish(&initial.rows)?;
        let ordered = if ordered_mode {
            Some(super::ordered_widgets::OrderedPaint::new(content.clone())?)
        } else {
            None
        };
        let mask_owner = masks.owner.clone();
        let mut cages = None;
        let mut composition = None;
        let errors = Rc::new(RefCell::new(Vec::new()));
        let observed = errors.clone();
        let web = if initial.composed {
            let result = Rc::new(RefCell::new(None));
            let captured = result.clone();
            let mut factory = Some(factory);
            let mut initial = Some(initial);
            let web = glory::web::launch_with(content, move |_| {
                glory::ui::widget(move |scope| {
                    let initial = initial.take().expect("composition builds once");
                    let manager = CompositionManager::new(
                        scope,
                        Profile::HTML,
                        factory.take().expect("composition factory builds once"),
                        style.clone().into(),
                        Some(mask_owner.clone()),
                    )
                    .map_err(|error| glory::render::Error::Backend(error.0))?;
                    if !initial.ordered {
                        let stage = manager
                            .prepare(initial.rows, initial.hidden)
                            .map_err(|error| glory::render::Error::Backend(error.0))?;
                        manager
                            .commit(&stage)
                            .map_err(|error| glory::render::Error::Backend(error.0))?;
                    }
                    *captured.borrow_mut() = Some(manager);
                    Ok(())
                })
            })
            .map_err(host)?;
            composition = result.borrow_mut().take();
            scene::ensure(composition.is_some(), "composition host did not initialize")?;
            web
        } else {
            glory::web::launch_with(content, |scope| {
                let state = PlotWidgetsState::new(scope, initial.rows, Profile::HTML, factory)
                    .with_mask_owner(mask_owner);
                let hidden = scope.cage(initial.hidden);
                cages = Some((state.rows, hidden));
                glory::ui::widget(move |scope| {
                    let cleanup = state.clone();
                    scope.on_cleanup(move || cleanup.clear())?;
                    let errors = observed.clone();
                    state
                        .clone()
                        .show(scope, style.clone(), hidden, move |error| {
                            errors.borrow_mut().push(error);
                        })
                })
            })
            .map_err(host)?
        };
        if let Some(error) = std::mem::take(&mut *errors.borrow_mut()).into_iter().next() {
            let _ = web.app().clear();
            return Err(error);
        }
        let (rows, hidden) = match cages {
            Some((rows, hidden)) => (Some(rows), Some(hidden)),
            None if composition.is_some() => (None, None),
            None => return Err(ChartError("Glory plot Widgets did not initialize".into())),
        };
        Ok(Self {
            web,
            masks,
            rows,
            hidden,
            composition,
            ordered,
            accepted_masks: RefCell::new(initial_masks),
            errors,
            alive: Cell::new(true),
            publishing: Cell::new(false),
        })
    }

    /// Prepare hidden candidate children outside every Engine State borrow.
    pub fn stage(&self, prepared: Prepared) -> Result<Rc<CompositionStage>> {
        scene::ensure(self.alive.get(), "plot Widget host retired")?;
        self.composition
            .as_ref()
            .ok_or_else(|| ChartError("strict composition requires a composed mount".into()))?
            .prepare(prepared.rows, prepared.hidden)
    }

    /// Application cleanup/build callbacks run outside Engine State borrows.
    pub fn publish(
        &self,
        prepared: Prepared,
        stage: Option<Rc<CompositionStage>>,
    ) -> Result<Vec<ChartError>> {
        if !self.alive.get() {
            return Ok(vec![]);
        }
        if let Some(manager) = &self.composition {
            let stage = match stage {
                Some(stage) => manager.retarget(&stage, prepared.rows.clone(), prepared.hidden)?,
                None => manager.prepare(prepared.rows.clone(), prepared.hidden)?,
            };
            self.masks.publish(&prepared.rows)?;
            self.publishing.set(true);
            let result = manager.commit(&stage);
            if result.is_err() {
                let _ = self.masks.publish(&self.accepted_masks.borrow());
            }
            let errors = if result.is_ok() {
                *self.accepted_masks.borrow_mut() = prepared.rows;
                // Data is already adopted before ordinary dynamic UI bindings
                // run. Match the ordered host: report those errors on the
                // accepted frame rather than calling publication a rejection.
                self.web.app().pump().err().map(host).into_iter().collect()
            } else {
                vec![]
            };
            self.publishing.set(false);
            if !self.alive.get() {
                let _ = self.web.app().clear();
                return Ok(vec![]);
            }
            result?;
            return Ok(errors);
        }
        let rows = self.rows.expect("overlay rows");
        let hidden = self.hidden.expect("overlay hidden");
        let changed = rows.with(|rows| rows != &prepared.rows);
        let hidden_changed = hidden.with(|hidden| hidden != &prepared.hidden);
        if !changed && !hidden_changed {
            return Ok(vec![]);
        }
        if changed {
            self.masks.publish(&prepared.rows)?;
        }
        self.publishing.set(true);
        if changed {
            rows.revise(|rows| *rows = prepared.rows);
        }
        if hidden_changed {
            hidden.revise(|hidden| *hidden = prepared.hidden);
        }
        let result = self.web.app().pump().map_err(host);
        self.publishing.set(false);
        if !self.alive.get() {
            let _ = self.web.app().clear();
            return Ok(vec![]);
        }
        result?;
        Ok(std::mem::take(&mut *self.errors.borrow_mut()))
    }

    pub fn publish_with_adopt(
        &self,
        prepared: Prepared,
        stage: Option<Rc<CompositionStage>>,
        adopt: &mut dyn FnMut(),
    ) -> Result<Vec<ChartError>> {
        if !self.alive.get() {
            return Ok(vec![]);
        }
        if let Some(manager) = &self.composition {
            let stage = match stage {
                Some(stage) => manager.retarget(&stage, prepared.rows.clone(), prepared.hidden)?,
                None => manager.prepare(prepared.rows.clone(), prepared.hidden)?,
            };
            self.masks.publish(&prepared.rows)?;
            self.publishing.set(true);
            let result = manager.commit_with_adopt(&stage, adopt);
            if result.is_err() {
                let _ = self.masks.publish(&self.accepted_masks.borrow());
            }
            let errors = if result.is_ok() {
                *self.accepted_masks.borrow_mut() = prepared.rows;
                // Data is already adopted before ordinary dynamic UI bindings
                // run. Match the ordered host: report those errors on the
                // accepted frame rather than calling publication a rejection.
                self.web.app().pump().err().map(host).into_iter().collect()
            } else {
                vec![]
            };
            self.publishing.set(false);
            if !self.alive.get() {
                let _ = self.web.app().clear();
                return Ok(vec![]);
            }
            result?;
            return Ok(errors);
        }
        let rows = self.rows.expect("overlay rows");
        let hidden = self.hidden.expect("overlay hidden");
        let changed = rows.with(|rows| rows != &prepared.rows);
        let hidden_changed = hidden.with(|hidden| hidden != &prepared.hidden);
        if !changed && !hidden_changed {
            adopt();
            return Ok(vec![]);
        }
        if changed {
            self.masks.publish(&prepared.rows)?;
        }
        self.publishing.set(true);
        adopt();
        if changed {
            rows.revise(|rows| *rows = prepared.rows);
        }
        if hidden_changed {
            hidden.revise(|hidden| *hidden = prepared.hidden);
        }
        let result = self.web.app().pump().map_err(host);
        self.publishing.set(false);
        if !self.alive.get() {
            let _ = self.web.app().clear();
            return Ok(vec![]);
        }
        result?;
        Ok(std::mem::take(&mut *self.errors.borrow_mut()))
    }

    pub fn stop(&self) {
        self.alive.set(false);
        if let Some(manager) = &self.composition {
            manager.clear();
        }
        if let Some(ordered) = &self.ordered {
            ordered.clear();
        }
        if !self.publishing.get() {
            let _ = self.web.app().clear();
        }
    }
}

impl BrowserChart {
    /// Replace real Scene marks or labels with retained Glory components at
    /// their authored draw-item positions. Label targets keep their source
    /// identity and never create a synthetic Mark or a canonical datum.
    pub fn mount_with_scene_widgets_in_scene<T: Clone + PartialEq + 'static, W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        composition: SceneWidgetsComposition<T>,
        factory: impl FnMut(SceneWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::scene_in_scene(composition),
                factory: scene_composition_factory(factory),
            }),
        )
    }

    pub fn set_scene_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle().set_scene_widgets_in_scene(composition)
    }

    pub fn set_layout_with_scene_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle()
            .set_layout_with_scene_widgets_in_scene(candidate, composition)
    }

    /// Mount a strict Scene component composition above the native SVG or
    /// Canvas paint. Successful label replacements omit only their source
    /// label ink. The live Glory subtree is not rasterized into Canvas/export.
    pub fn mount_with_scene_widgets_composed<T: Clone + PartialEq + 'static, W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        composition: SceneWidgetsComposition<T>,
        factory: impl FnMut(SceneWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::scene_composition(composition),
                factory: scene_composition_factory(factory),
            }),
        )
    }

    pub fn set_scene_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle().set_scene_widgets_composed(composition)
    }

    pub fn set_layout_with_scene_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle()
            .set_layout_with_scene_widgets_composed(candidate, composition)
    }

    /// Insert real Widgets at their authored source Mark positions, including
    /// shared opacity groups. Native pixel-shaped hit targets prevent later
    /// renderer ink from clicking through to a covered Widget.
    pub fn mount_with_plot_widgets_in_scene<T: Clone + PartialEq + 'static, W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        composition: PlotWidgetsComposition<T>,
        factory: impl FnMut(PlotWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::in_scene(composition),
                factory: composition_factory(factory),
            }),
        )
    }
    pub fn set_plot_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle().set_plot_widgets_in_scene(composition)
    }
    pub fn set_layout_with_plot_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle()
            .set_layout_with_plot_widgets_in_scene(candidate, composition)
    }
    /// Mount a strict Widget composition. Real children are built while hidden
    /// before the first filtered paint. Replaced physical mark ink is omitted;
    /// original Scene labels, source metadata and canonical controls remain.
    /// Widgets occupy the top layer, independently of the source paint order.
    pub fn mount_with_plot_widgets_composed<T: Clone + PartialEq + 'static, W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        composition: PlotWidgetsComposition<T>,
        factory: impl FnMut(PlotWidgetCompositionContext<T>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::composition(composition),
                factory: composition_factory(factory),
            }),
        )
    }

    pub fn set_plot_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle().set_plot_widgets_composed(composition)
    }

    pub fn set_layout_with_plot_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.handle()
            .set_layout_with_plot_widgets_composed(candidate, composition)
    }

    /// Mount measured Glory Widgets with owned, typed metadata. Each child's
    /// frame and data follow the same accepted slot row without remounting.
    /// Subsequent packets must keep this metadata type, including empty packets.
    /// Child construction errors retain the old child, as in the untyped API;
    /// they do not roll back an already accepted Scene or packet.
    pub fn mount_with_plot_widgets_data<T: Clone + PartialEq + 'static, W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        data: PlotWidgetsData<T>,
        factory: impl FnMut(TypedPlotWidgetContext<T>) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::typed(data),
                factory: typed_factory(factory),
            }),
        )
    }

    pub fn set_plot_widgets_data<T: Clone + PartialEq + 'static>(
        &self,
        data: PlotWidgetsData<T>,
    ) -> Result<()> {
        self.handle().set_plot_widgets_data(data)
    }

    pub fn set_layout_with_plot_widgets_data<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        data: PlotWidgetsData<T>,
    ) -> Result<()> {
        self.handle()
            .set_layout_with_plot_widgets_data(candidate, data)
    }

    /// Mount real Glory Widgets in caller-measured slots above either browser
    /// renderer. The Widget layer is a sibling of the input surface; its
    /// interactive children own pointer and keyboard events themselves.
    /// Browser camera transforms move and resize the measured boxes. Datum
    /// anchors use live geometry, including typed Path anchors when supplied.
    /// Partial slots paint through their rectangular or path clips and are inert. Mounted
    /// revisions persist across geometry changes and legend filtering. Inline
    /// content is clipped by the slot; Glory popups use their own host policy.
    pub fn mount_with_plot_widgets<W: Widget>(
        root: Element,
        scene: Scene,
        options: BrowserOptions,
        layout: PlotWidgetsLayout,
        factory: impl FnMut(PlotWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout: PlotWidgetsPacket::plain(layout),
                factory: plain_factory(factory),
            }),
        )
    }

    pub fn set_plot_widgets(&self, layout: PlotWidgetsLayout) -> Result<()> {
        self.handle().set_plot_widgets(layout)
    }

    pub fn set_layout_with_plot_widgets(
        &self,
        candidate: SceneInteraction,
        slots: PlotWidgetsLayout,
    ) -> Result<()> {
        self.handle().set_layout_with_plot_widgets(candidate, slots)
    }
}

impl BrowserHandle {
    pub fn set_scene_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(None, PlotWidgetsPacket::scene_in_scene(composition))
    }

    pub fn set_layout_with_scene_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(
            Some(candidate),
            PlotWidgetsPacket::scene_in_scene(composition),
        )
    }

    pub fn set_scene_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(None, PlotWidgetsPacket::scene_composition(composition))
    }

    pub fn set_layout_with_scene_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: SceneWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(
            Some(candidate),
            PlotWidgetsPacket::scene_composition(composition),
        )
    }

    pub fn set_plot_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(None, PlotWidgetsPacket::in_scene(composition))
    }
    pub fn set_layout_with_plot_widgets_in_scene<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(Some(candidate), PlotWidgetsPacket::in_scene(composition))
    }
    /// Preflight real candidate children before enqueueing. Factory/build errors
    /// reject this entire candidate. Existing children keep their accepted frame.
    /// Candidate-build context is tentative until paint and composition commit.
    pub fn set_plot_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(None, PlotWidgetsPacket::composition(composition))
    }

    pub fn set_layout_with_plot_widgets_composed<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        composition: PlotWidgetsComposition<T>,
    ) -> Result<()> {
        self.set_composition_packet(Some(candidate), PlotWidgetsPacket::composition(composition))
    }

    fn set_composition_packet(
        &self,
        candidate: Option<SceneInteraction>,
        packet: PlotWidgetsPacket,
    ) -> Result<()> {
        self.enqueue_composition_packet(candidate, packet)
            .map(|_| ())
    }

    /// Return the original shared adoption token without changing the public
    /// setters. A coordinated owner can expose candidate snapshots precisely
    /// when its retained child rows become live, including retargeted stages.
    pub(super) fn enqueue_composition_packet(
        &self,
        candidate: Option<SceneInteraction>,
        packet: PlotWidgetsPacket,
    ) -> Result<Rc<CompositionStage>> {
        let engine = self.engine()?;
        let prepared = {
            let state = engine.state.borrow();
            let state = state.as_ref().expect("active browser chart state");
            state.require_plot_widget_type(&packet)?;
            let mut model = match &candidate {
                Some(candidate) => {
                    state
                        .plot_widget_layout_candidate(candidate.clone(), &packet)?
                        .0
                }
                None => state.model.clone(),
            };
            model.set_widget_replacements(packet.replace_ink().clone())?;
            prepare(&model, &packet, true)?
        };
        let host = engine
            .plot_widgets
            .borrow()
            .clone()
            .ok_or_else(|| ChartError("strict composition host retired".into()))?;
        let stage = host.stage(prepared)?;
        scene::ensure(
            engine.alive.get(),
            "browser chart retired during composition build",
        )?;
        engine.try_enqueue(Input::PlotWidgetComposition(Box::new((
            candidate,
            packet,
            stage.clone(),
        ))))?;
        Ok(stage)
    }

    /// Queue typed metadata and placement together. Payload keys must match the
    /// measured slot IDs exactly. Metadata-only updates retain mounted children.
    pub fn set_plot_widgets_data<T: Clone + PartialEq + 'static>(
        &self,
        data: PlotWidgetsData<T>,
    ) -> Result<()> {
        self.set_plot_widget_packet(PlotWidgetsPacket::typed(data))
    }

    /// Admit Scene, interaction binding, slots and typed data as one candidate.
    /// Failed model/slot validation leaves the accepted frame and data intact.
    pub fn set_layout_with_plot_widgets_data<T: Clone + PartialEq + 'static>(
        &self,
        candidate: SceneInteraction,
        data: PlotWidgetsData<T>,
    ) -> Result<()> {
        self.set_layout_with_plot_widget_packet(candidate, PlotWidgetsPacket::typed(data))
    }

    fn set_plot_widget_packet(&self, slots: PlotWidgetsPacket) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state.as_ref().expect("active browser chart state");
            state.require_plot_widget_type(&slots)?;
            prepare(&state.model, &slots, true)?;
        }
        engine.try_enqueue(Input::PlotWidgets(slots))
    }

    fn set_layout_with_plot_widget_packet(
        &self,
        candidate: SceneInteraction,
        slots: PlotWidgetsPacket,
    ) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state.as_ref().expect("active browser chart state");
            state.plot_widget_layout_candidate(candidate.clone(), &slots)?;
        }
        engine.try_enqueue(Input::PlotWidgetLayout(Box::new((candidate, slots))))
    }

    /// Queue a replacement of the measured slot layout. New definitions
    /// require a current datum. Subsequent Scene updates retire rows whose
    /// datum is removed. Rejected factory revisions retain the preceding
    /// child and report a BrowserEvent::Error. Application side effects
    /// outside a child scope remain the caller's responsibility.
    pub fn set_plot_widgets(&self, slots: PlotWidgetsLayout) -> Result<()> {
        self.set_plot_widget_packet(PlotWidgetsPacket::plain(slots))
    }

    /// Atomically replace a Scene, interaction metadata and measured slots.
    /// This uses the plain Scene blend profile, like set_layout.
    pub fn set_layout_with_plot_widgets(
        &self,
        candidate: SceneInteraction,
        slots: PlotWidgetsLayout,
    ) -> Result<()> {
        self.set_layout_with_plot_widget_packet(candidate, PlotWidgetsPacket::plain(slots))
    }
}

impl State {
    pub(super) fn source_ordered(&self) -> bool {
        self.plot_widgets
            .as_ref()
            .is_some_and(|packet| packet.is_source_ordered())
    }
    pub(super) fn require_plot_widget_type(&self, packet: &PlotWidgetsPacket) -> Result<()> {
        self.require_plot_widgets()?;
        scene::ensure(
            self.plot_widgets.as_ref().unwrap().data_type() == packet.data_type()
                && self.plot_widgets.as_ref().unwrap().is_composition() == packet.is_composition()
                && self.plot_widgets.as_ref().unwrap().is_source_ordered()
                    == packet.is_source_ordered(),
            "plot Widget metadata type differs from the mounted factory",
        )
    }

    pub(super) fn prepare_widgets_for(&self, model: &SurfaceModel) -> Result<Option<Prepared>> {
        self.plot_widgets
            .as_ref()
            .map(|layout| prepare(model, layout, false))
            .transpose()
    }

    pub(super) fn require_plot_widgets(&self) -> Result<()> {
        scene::ensure(
            self.plot_widgets.is_some(),
            "plot Widgets require mount_with_plot_widgets",
        )
    }

    pub(super) fn plot_widget_layout_candidate(
        &self,
        candidate: SceneInteraction,
        slots: &PlotWidgetsPacket,
    ) -> Result<(SurfaceModel, Vec<SurfaceEvent>)> {
        self.require_plot_widget_type(slots)?;
        self.admit_interaction_scene(&candidate.scene, candidate.interaction.as_ref())?;
        let mut model = self.model.clone();
        let mut changes = model.set_layout(candidate)?;
        prepare(&model, slots, true)?;
        changes.extend(model.set_widget_replacements(slots.replace_ink().clone())?);
        Ok((model, changes))
    }
}
