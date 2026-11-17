//! A Tree-owned native press keeps automatic paint from replacing its target.
//! Model input continues normally, including panning once the click threshold
//! has been crossed. Explicit accepted publication is a separate cancellation.
use super::*;

#[derive(Clone, Copy)]
pub(super) struct Press {
    id: i32,
    widget: bool,
    dragging: bool,
}

pub(super) fn explicit_change(input: &Input) -> bool {
    matches!(
        input,
        Input::Hidden(_)
            | Input::LegendToggle(_)
            | Input::Selected(_)
            | Input::Activate(_)
            | Input::PlotWidgets(_)
            | Input::PlotWidgetLayout(_)
            | Input::PlotWidgetComposition(_)
            | Input::Scene(..)
            | Input::InteractionScene(_)
            | Input::Layout(_)
            | Input::BlendedAnimation(_)
            | Input::ContourScene(..)
            | Input::Action(
                SurfaceInput::Reset
                    | SurfaceInput::Clear
                    | SurfaceInput::Next
                    | SurfaceInput::Previous
                    | SurfaceInput::First
                    | SurfaceInput::Last
                    | SurfaceInput::Activate
            )
    )
}

impl Engine {
    pub(super) fn tree_press_begin(&self, id: i32, widget: bool) -> Result<()> {
        scene::ensure(self.alive.get(), "browser chart is stopped")?;
        self.retired_native_press.set(None);
        self.native_press.set(Some(Press {
            id,
            widget,
            dragging: false,
        }));
        Ok(())
    }
    pub(super) fn tree_press_active(&self, id: i32) -> bool {
        self.native_press.get().is_some_and(|p| p.id == id)
    }
    pub(super) fn native_press_contact_live(&self) -> bool {
        self.native_press
            .get()
            .is_some_and(|p| !p.widget && self.captured.get() == Some(p.id))
    }
    pub(super) fn tree_press_end(&self, id: i32) -> Result<()> {
        if self.tree_press_active(id) {
            self.native_press.set(None);
            if self.alive.get() {
                self.request()?;
            }
        }
        Ok(())
    }
    pub(super) fn tree_press_cancel(&self) -> Result<()> {
        let Some(press) = self.native_press.take() else {
            return Ok(());
        };
        self.retired_native_press.set(Some(press.id));
        self.queue.borrow_mut().retain(
            |input| !matches!(input, Input::Pointer { id, touch: false, .. } if *id == press.id),
        );
        if self.captured.get() == Some(press.id) {
            self.captured.set(None);
        }
        let surface = if let Ok(mut state) = self.state.try_borrow_mut() {
            if let Some(s) = state.as_mut() {
                // This private hook only retires its primary mouse/pen contact.
                // Public Scene replacement retains its existing cancel contract.
                s.model.apply(SurfaceInput::Cancel { id: press.id })?;
                Some(s.surface.clone())
            } else {
                None
            }
        } else {
            // A native cancellation can be delivered from ordinary UI work.
            // Retire raw transport immediately and resolve the model after that
            // borrow ends, before any later contact queued by the same owner.
            self.try_enqueue(Input::Action(SurfaceInput::Cancel { id: press.id }))?;
            self.outer
                .query_selector(".kartis-surface")
                .map_err(host)?
                .and_then(|node| node.dyn_into::<HtmlElement>().ok())
        };
        if let Some(surface) = surface
            && surface.has_pointer_capture(press.id)
        {
            let _ = surface.release_pointer_capture(press.id);
        }
        if self.alive.get() {
            self.request()?;
        }
        Ok(())
    }
    pub(super) fn hold_native_press_paint(&self, explicit: bool) -> bool {
        let Some(mut press) = self.native_press.get() else {
            return false;
        };
        if explicit {
            return false;
        }
        if !press.widget {
            let state = self.state.borrow();
            if let Some(s) = state.as_ref()
                && s.model.dragging()
                && !s.model.native_click_pending()
            {
                press.dragging = true;
                self.native_press.set(Some(press));
            }
        }
        // Keep the target through Up's event tail too: its model drag is gone
        // before the browser dispatches the native click. The owner releases
        // this lease after that dispatch or its bounded post-input task.
        press.widget || !press.dragging
    }
    pub(super) fn finish_frame_events(&self, events: Vec<BrowserEvent>) -> Result<()> {
        for event in events {
            if !self.alive.get() {
                return Ok(());
            }
            self.emit(event);
        }
        if self.alive.get()
            && (self
                .state
                .borrow()
                .as_ref()
                .is_some_and(|s| s.motion.is_some())
                || !self.queue.borrow().is_empty()
                || self.resize_pending.get()
                || self.pending_reduced.get().is_some())
        {
            self.request()?;
        }
        Ok(())
    }
}
