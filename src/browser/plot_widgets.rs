use std::collections::BTreeMap;

use glory::Widget;
use glory::style::Profile;

use super::*;
use crate::widget::plot_widgets::{
    PlotWidgetFactory, PlotWidgetsState, PreparedSlot, SlotSpace, overlay_style, prepare_in_space,
};

mod masks;
use masks::MaskLayer;

pub(super) struct Mount {
    pub layout: PlotWidgetsLayout,
    pub factory: PlotWidgetFactory,
}

pub(super) struct Prepared {
    rows: Vec<PreparedSlot>,
    hidden: BTreeSet<String>,
}

pub(super) fn prepare(
    model: &SurfaceModel,
    layout: &PlotWidgetsLayout,
    require_current_data: bool,
) -> Result<Prepared> {
    scene::ensure(
        layout.slots.len() <= MAX_PLOT_WIDGET_SLOTS,
        "plot Widget slot budget exceeded",
    )?;
    let interaction = model.interaction_data();
    let values = if let Some(binding) = interaction {
        let requested: BTreeSet<_> = layout
            .slots
            .iter()
            .map(|slot| slot.mark_id.as_str())
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
    let rows = prepare_in_space(
        model.scene(),
        layout,
        Profile::HTML,
        SlotSpace {
            plot: Some(model.plot()),
            transform: model.paint_transform(),
            responsive: true,
            require_current_data,
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
    })
}

pub(super) struct Host {
    web: WebApp,
    masks: MaskLayer,
    rows: Cage<Vec<PreparedSlot>>,
    hidden: Cage<BTreeSet<String>>,
    errors: Rc<RefCell<Vec<ChartError>>>,
    alive: Cell<bool>,
    publishing: Cell<bool>,
}

impl Host {
    pub fn launch(
        root: Element,
        scene: &Scene,
        factory: PlotWidgetFactory,
        initial: Prepared,
    ) -> Result<Self> {
        let style = overlay_style(scene, true);
        let (masks, content) = MaskLayer::mount(&root)?;
        masks.publish(&initial.rows);
        let mask_owner = masks.owner.clone();
        let mut cages = None;
        let errors = Rc::new(RefCell::new(Vec::new()));
        let observed = errors.clone();
        let web = glory::web::launch_with(content, |scope| {
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
        .map_err(host)?;
        // A caller cannot subscribe to mount-time errors before it obtains a
        // BrowserChart guard. Abort the unaccepted mount with the real error.
        if let Some(error) = std::mem::take(&mut *errors.borrow_mut()).into_iter().next() {
            let _ = web.app().clear();
            return Err(error);
        }
        let (rows, hidden) =
            cages.ok_or_else(|| ChartError("Glory plot Widgets did not initialize".into()))?;
        Ok(Self {
            web,
            masks,
            rows,
            hidden,
            errors,
            alive: Cell::new(true),
            publishing: Cell::new(false),
        })
    }

    /// Call outside Engine state borrows. Application factory/build/cleanup
    /// closures can synchronously call the chart's weak handle.
    pub fn publish(&self, prepared: Prepared) -> Result<Vec<ChartError>> {
        if !self.alive.get() {
            return Ok(vec![]);
        }
        let changed = self.rows.with(|rows| rows != &prepared.rows);
        let hidden = self.hidden.with(|hidden| hidden != &prepared.hidden);
        if !changed && !hidden {
            return Ok(vec![]);
        }
        self.publishing.set(true);
        if changed {
            self.masks.publish(&prepared.rows);
            self.rows.revise(|rows| *rows = prepared.rows);
        }
        if hidden {
            self.hidden.revise(|hidden| *hidden = prepared.hidden);
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
        if !self.publishing.get() {
            let _ = self.web.app().clear();
        }
    }
}

impl BrowserChart {
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
        mut factory: impl FnMut(PlotWidgetContext) -> Result<W> + 'static,
    ) -> Result<Self> {
        Self::mount_inner(
            root,
            scene,
            options,
            MarkBlends::default(),
            Some(Mount {
                layout,
                factory: Box::new(move |context| Ok(Box::new(factory(context)?))),
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
    /// Queue a replacement of the measured slot layout. New definitions
    /// require a current datum. Subsequent Scene updates retire rows whose
    /// datum is removed. Rejected factory revisions retain the preceding
    /// child and report a BrowserEvent::Error. Application side effects
    /// outside a child scope remain the caller's responsibility.
    pub fn set_plot_widgets(&self, slots: PlotWidgetsLayout) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state.as_ref().expect("active browser chart state");
            state.require_plot_widgets()?;
            prepare(&state.model, &slots, true)?;
        }
        engine.try_enqueue(Input::PlotWidgets(slots))
    }

    /// Atomically replace a Scene, interaction metadata and measured slots.
    /// This uses the plain Scene blend profile, like set_layout.
    pub fn set_layout_with_plot_widgets(
        &self,
        candidate: SceneInteraction,
        slots: PlotWidgetsLayout,
    ) -> Result<()> {
        let engine = self.engine()?;
        {
            let state = engine.state.borrow();
            let state = state.as_ref().expect("active browser chart state");
            state.plot_widget_layout_candidate(candidate.clone(), &slots)?;
        }
        engine.try_enqueue(Input::PlotWidgetLayout(Box::new((candidate, slots))))
    }
}

impl State {
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
        slots: &PlotWidgetsLayout,
    ) -> Result<(SurfaceModel, Vec<SurfaceEvent>)> {
        self.require_plot_widgets()?;
        self.admit_interaction_scene(&candidate.scene, candidate.interaction.as_ref())?;
        let mut model = self.model.clone();
        let changes = model.set_layout(candidate)?;
        prepare(&model, slots, true)?;
        Ok((model, changes))
    }
}
