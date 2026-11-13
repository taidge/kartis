//! Atomic semantic plot state. The paint Scene is never replaced by the
//! controls-only projection, and physical hit data always comes from Surface.
use glory::render::{PlotPointer, PlotPointerDevice, PlotPointerPhase};

use super::*;
use crate::surface::{SurfaceEvent, SurfaceInput, SurfaceModel, SurfaceOptions};
use crate::{PickingProfile, SceneInteraction, SurfacePhysicalHit};

#[derive(Clone)]
pub(super) struct BoundPlot {
    pub model: Rc<RefCell<SurfaceModel>>,
    layout: Rc<RefCell<SceneInteraction>>,
    controls: Rc<RefCell<Vec<Mark>>>,
    revision: Cage<bool>,
    lookup: Rc<RefCell<BTreeMap<String, usize>>>,
}
pub(super) struct Prepared {
    model: SurfaceModel,
    layout: SceneInteraction,
    controls: Vec<Mark>,
    pub events: Vec<SurfaceEvent>,
}
impl Prepared {
    pub fn data_labels(&self) -> BTreeMap<String, String> {
        self.model.data_control_labels()
    }
}
fn projection(model: &SurfaceModel) -> Vec<Mark> {
    let Some(binding) = model.interaction_data() else {
        return vec![];
    };
    let ids: BTreeMap<_, _> = binding
        .sites
        .iter()
        .map(|site| (site.mark_id.as_str(), site))
        .collect();
    // Explicitly controls-only. Actual painting keeps the unmodified Scene.
    let scene = model.controls_scene();
    scene
        .paint_marks()
        .filter(|mark| ids.contains_key(mark.id.as_str()) && !model.hidden().contains(&mark.series))
        .map(|mark| {
            let mut mark = mark.clone();
            let site = ids[mark.id.as_str()];
            if let Some(value) = site.value {
                mark.value = value;
            }
            mark.label.clone_from(&site.label);
            mark
        })
        .collect()
}
fn changes(before: &SurfaceModel, after: &SurfaceModel) -> Vec<SurfaceEvent> {
    let mut events = vec![];
    if before.hovered() != after.hovered() {
        events.push(SurfaceEvent::Hover(after.hovered().to_vec()));
    }
    if before.selected() != after.selected() {
        events.push(SurfaceEvent::Selected(after.selected().map(str::to_string)));
    }
    if before.view() != after.view() {
        events.push(SurfaceEvent::View(after.view()));
    }
    if before.hovered_physical_hit() != after.hovered_physical_hit() {
        events.push(SurfaceEvent::PhysicalHover(
            after.hovered_physical_hit().cloned(),
        ));
    }
    events
}
/// Shared finite input translation for semantic and raster hosts. Callers
/// operate on an uncommitted candidate model, then atomically accept it.
pub(crate) fn apply_pointer(
    model: &mut SurfaceModel,
    pointer: PlotPointer,
) -> Result<Vec<SurfaceEvent>> {
    pointer
        .validate()
        .map_err(|error| ChartError(error.to_string()))?;
    let old = model.clone();
    let terminal = matches!(
        pointer.phase,
        PlotPointerPhase::Leave | PlotPointerPhase::Cancel
    );
    let point = if terminal {
        (0., 0.)
    } else {
        (
            pointer.x * model.scene().width / pointer.viewport.width,
            pointer.y * model.scene().height / pointer.viewport.height,
        )
    };
    let id = pointer.pointer_id;
    let accepted = if pointer.device == PlotPointerDevice::Touch {
        match pointer.phase {
            PlotPointerPhase::Move => model.apply(SurfaceInput::TouchMove { id, point })?,
            PlotPointerPhase::Down => model.apply(SurfaceInput::TouchStart { id, point })?,
            PlotPointerPhase::Up => {
                let events = model.apply(SurfaceInput::TouchEnd { id, point })?;
                // A tap may activate a datum, but touch is not persistent mouse hover.
                model.apply(SurfaceInput::Leave)?;
                events
            }
            PlotPointerPhase::Leave | PlotPointerPhase::Cancel => {
                model.apply(SurfaceInput::TouchCancel { id })?;
                model.apply(SurfaceInput::Leave)?
            }
        }
    } else {
        match pointer.phase {
            PlotPointerPhase::Move => model.apply(SurfaceInput::Move { id, point })?,
            PlotPointerPhase::Down => model.apply(SurfaceInput::Down {
                id,
                point,
                brush: false,
            })?,
            PlotPointerPhase::Up => model.apply(SurfaceInput::Up { id, point })?,
            PlotPointerPhase::Leave | PlotPointerPhase::Cancel => {
                model.apply(SurfaceInput::Cancel { id })?;
                model.apply(SurfaceInput::Leave)?
            }
        }
    };
    let mut events = changes(&old, model);
    for event in accepted {
        if matches!(event, SurfaceEvent::Activated(_)) {
            let index = events
                .iter()
                .position(|e| matches!(e, SurfaceEvent::PhysicalHover(_)))
                .unwrap_or(events.len());
            events.insert(index, event);
        }
    }
    Ok(events)
}
pub(super) fn validate(layout: &SceneInteraction) -> Result<()> {
    layout.validate()?;
    ensure(
        layout
            .interaction
            .as_ref()
            .is_some_and(|b| b.picking_profile == PickingProfile::PaintedGeometry),
        "chart_layout requires an explicit PaintedGeometry interaction binding",
    )
}
impl BoundPlot {
    pub fn new(scope: &Scope, layout: SceneInteraction, hidden: &BTreeSet<String>) -> Result<Self> {
        validate(&layout)?;
        let mut model = SurfaceModel::new_with_interaction(
            layout.scene.clone(),
            SurfaceOptions {
                plot: layout.plot,
                zoom: false,
                pan: false,
                pinch: false,
                brush: false,
                ..Default::default()
            },
            layout.interaction.clone(),
        )?;
        model.set_hidden(hidden.clone())?;
        let controls = projection(&model);
        let lookup = controls
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id.clone(), i))
            .collect();
        Ok(Self {
            model: Rc::new(RefCell::new(model)),
            layout: Rc::new(RefCell::new(layout)),
            controls: Rc::new(RefCell::new(controls)),
            revision: scope.cage(false),
            lookup: Rc::new(RefCell::new(lookup)),
        })
    }
    pub fn prepare(&self, layout: SceneInteraction, hidden: &BTreeSet<String>) -> Result<Prepared> {
        validate(&layout)?;
        let old = self.model.borrow();
        let mut model = old.clone();
        if *self.layout.borrow() != layout {
            model.set_layout(layout.clone())?;
        }
        if model.hidden() != hidden {
            model.set_hidden(hidden.clone())?;
        }
        let events = changes(&old, &model);
        let controls = projection(&model);
        Ok(Prepared {
            model,
            layout,
            controls,
            events,
        })
    }
    pub fn prepare_hidden(&self, hidden: &BTreeSet<String>) -> Result<Prepared> {
        self.prepare(self.layout.borrow().clone(), hidden)
    }
    pub fn commit(&self, next: Prepared) -> Vec<SurfaceEvent> {
        let Prepared {
            model,
            layout,
            controls,
            events,
        } = next;
        let changed = *self.controls.borrow() != controls;
        *self.model.borrow_mut() = model;
        *self.layout.borrow_mut() = layout;
        if changed {
            *self.lookup.borrow_mut() = controls
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id.clone(), i))
                .collect();
            *self.controls.borrow_mut() = controls;
            // An invalidation bit, not a source frame identity or sequence.
            self.revision.revise(|value| *value = !*value);
        }
        events
    }
    pub fn marks(&self) -> Vec<Mark> {
        let _ = self.revision.get();
        self.controls.borrow().clone()
    }
    pub fn data_labels(&self) -> BTreeMap<String, String> {
        self.model.borrow().data_control_labels()
    }
    pub fn lookup(&self, id: &str) -> Option<Mark> {
        let _ = self.revision.get();
        let index = *self.lookup.borrow().get(id)?;
        let mut mark = self.model.borrow().logical_datum(id)?.clone();
        let controls = self.controls.borrow();
        let source = &controls[index];
        mark.value = source.value;
        mark.label.clone_from(&source.label);
        Some(mark)
    }
    pub fn apply(&self, pointer: PlotPointer) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.model.borrow().clone();
        let events = apply_pointer(&mut next, pointer)?;
        *self.model.borrow_mut() = next;
        Ok(events)
    }
    pub fn activate(&self, id: &str) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.model.borrow().clone();
        let events = next.activate(id)?;
        *self.model.borrow_mut() = next;
        Ok(events)
    }
    pub fn clear_pointer(&self) -> Result<Vec<SurfaceEvent>> {
        let old = self.model.borrow();
        let mut next = old.clone();
        next.cancel_drag();
        next.apply(SurfaceInput::Leave)?;
        let events = changes(&old, &next);
        drop(old);
        *self.model.borrow_mut() = next;
        Ok(events)
    }
    pub fn current_hit(&self, hit: &SurfacePhysicalHit) -> bool {
        self.model.borrow().is_current_hit(hit)
    }
}
impl State {
    pub(super) fn bound_pointer(&self, pointer: PlotPointer) {
        if !self.selection.alive.get() {
            return;
        }
        let Some(bound) = &self.bound_plot else {
            return;
        };
        let result = bound.apply(pointer);
        match result {
            Ok(events) => self.bound_events(events),
            Err(error) => self.bound_error(error),
        }
    }
    pub(super) fn bound_error(&self, error: ChartError) {
        if !self.selection.alive.get() {
            return;
        }
        if self.has_rejection {
            self.rejection.emit(error);
        } else if let Some(scope) = self.owner.upgrade() {
            scope.report_error(ui_error(error));
        }
    }
    pub(super) fn bound_events(&self, events: Vec<SurfaceEvent>) {
        let Some(bound) = &self.bound_plot else {
            return;
        };
        let token = bound.model.borrow().frame_token();
        for event in events {
            if !self.selection.alive.get() || bound.model.borrow().frame_token() != token {
                break;
            }
            match event {
                SurfaceEvent::Hover(ids) => {
                    let next = ids.first().cloned();
                    let previous = self.hovered.get_untracked();
                    if !self.controlled_hover && previous != next {
                        self.hovered.revise(|old| *old = next.clone());
                    }
                    if previous != next {
                        self.hover
                            .emit(next.as_deref().and_then(|id| bound.lookup(id)));
                    }
                }
                SurfaceEvent::Selected(next) => {
                    if !self.controlled_selection {
                        self.selected.revise(|old| *old = next.clone());
                    }
                    self.selection_change
                        .emit(next.as_deref().and_then(|id| bound.lookup(id)));
                }
                SurfaceEvent::Activated(id) => {
                    if let Some(mark) = bound.lookup(&id) {
                        self.selection.emit(mark);
                    }
                }
                SurfaceEvent::PhysicalHover(hit)
                    if hit.as_ref().is_none_or(|h| bound.current_hit(h)) =>
                {
                    self.physical_hover.emit(hit);
                }
                _ => {}
            }
        }
    }
}
