//! Viewport-space editing; scene projection is confined to data-anchor queries.
use super::*;
use crate::brush::{
    BrushReconfigure, EditableBrush, EditableBrushEvent, EditableBrushInput, EditableBrushOptions,
};

impl SurfaceModel {
    pub fn editable_brush(&self) -> Option<&EditableBrush> {
        self.editable_brush.as_ref()
    }

    /// Opt in independently of the legacy `SurfaceOptions::brush` flag. While
    /// configured, editable creation supersedes legacy Shift-drag creation;
    /// outside the editor's extent, ordinary pan/click remains available.
    /// Extents are explicit viewport pixels inside the current plot. On plot
    /// shrink/movement the extent is clamped to that plot; expansion requires
    /// an explicit reconfiguration. Resizing never reinterprets pixels as data.
    pub fn configure_editable_brush(
        &mut self,
        options: Option<EditableBrushOptions>,
        policy: BrushReconfigure,
    ) -> Result<Vec<SurfaceEvent>> {
        if options.is_none() && self.editable_brush.is_none() {
            return Ok(Vec::new());
        }
        let mut next = self.clone();
        let events = if let Some(options) = options {
            next.validate_editable_extent(options.extent)?;
            if let Some(editor) = &mut next.editable_brush {
                editor.configure(options, policy)?
            } else {
                next.editable_brush = Some(EditableBrush::new(options)?);
                Vec::new()
            }
        } else if let Some(mut editor) = next.editable_brush.take() {
            let mut events = editor.cancel()?;
            events.extend(editor.set_selection(None)?);
            events
        } else {
            Vec::new()
        };
        if next.editable_brush != self.editable_brush {
            next.drag = None;
            next.brush = None;
            next.touches.clear();
            next.pinch = None;
        }
        let mut events = next.editable_events(events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }

    pub fn set_brush_selection(&mut self, selection: Option<Brush>) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.clone();
        let events = next
            .editable_brush
            .as_mut()
            .ok_or_else(|| crate::ChartError("editable brush is not configured".into()))?
            .set_selection(selection)?;
        if next.editable_brush != self.editable_brush {
            next.drag = None;
            next.brush = None;
            next.touches.clear();
            next.pinch = None;
        }
        let mut events = next.editable_events(events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }

    pub fn apply_editable_brush(&mut self, input: EditableBrushInput) -> Result<Vec<SurfaceEvent>> {
        // A second input system cannot steal a live pan/pinch/click session.
        if matches!(input, EditableBrushInput::Begin { .. })
            && (self.drag.is_some() || !self.touches.is_empty() || self.pinch.is_some())
        {
            return Ok(Vec::new());
        }
        let mut next = self.clone();
        let events = next
            .editable_brush
            .as_mut()
            .ok_or_else(|| crate::ChartError("editable brush is not configured".into()))?
            .apply(input)?;
        if !events.is_empty()
            && matches!(
                input,
                EditableBrushInput::Nudge { .. } | EditableBrushInput::Clear
            )
        {
            next.drag = None;
            next.brush = None;
            next.touches.clear();
            next.pinch = None;
        }
        let mut events = next.editable_events(events)?;
        if next
            .editable_brush
            .as_ref()
            .is_some_and(|brush| brush.active())
        {
            next.hover_point = None;
            next.hover(Vec::new(), &mut events);
        }
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }

    /// Owner/blur cancellation restores the selection accepted before Begin.
    pub fn cancel_editable_brush(&mut self) -> Result<Vec<SurfaceEvent>> {
        let mut next = self.clone();
        let events = next
            .editable_brush
            .as_mut()
            .map(EditableBrush::cancel)
            .transpose()?
            .unwrap_or_default();
        let mut events = next.editable_events(events)?;
        next.physical_events(self, &mut events);
        *self = next;
        Ok(events)
    }

    pub fn editable_brush_ids(&self) -> Result<Vec<String>> {
        self.editable_selection_ids(self.editable_brush.as_ref().and_then(|b| b.selection()))
    }

    /// Convert the accepted viewport rectangle to a plot-local domain request.
    /// Subtract the plot origin once; do not inverse-project the Scene camera.
    /// Empty temporary selections do not request a zero-span data domain.
    pub fn editable_brush_viewport_request(&self) -> Result<Option<crate::ViewportRequest>> {
        let Some(editor) = &self.editable_brush else {
            return Ok(None);
        };
        let Some(selection) = editor.selection() else {
            return Ok(None);
        };
        selection.validate()?;
        let axis = editor.options().axis;
        if (axis != crate::BrushAxis::Y && selection.x0 == selection.x1)
            || (axis != crate::BrushAxis::X && selection.y0 == selection.y1)
        {
            return Ok(None);
        }
        let plot = self.plot();
        Ok(Some(crate::ViewportRequest::Brush {
            bounds: Brush::from_points(
                (selection.x0 - plot.x, selection.y0 - plot.y),
                (selection.x1 - plot.x, selection.y1 - plot.y),
            )?,
            axes: match axis {
                crate::BrushAxis::X => crate::ViewportSelection::X,
                crate::BrushAxis::Y => crate::ViewportSelection::Y,
                crate::BrushAxis::XY => crate::ViewportSelection::Both,
            },
        }))
    }

    fn validate_editable_extent(&self, extent: Brush) -> Result<()> {
        extent.validate()?;
        let plot = self.plot();
        ensure(
            extent.x0 >= plot.x
                && extent.y0 >= plot.y
                && extent.x1 <= plot.x + plot.width
                && extent.y1 <= plot.y + plot.height,
            "editable brush extent must be inside the current viewport plot",
        )
    }

    fn editable_events(&self, events: Vec<EditableBrushEvent>) -> Result<Vec<SurfaceEvent>> {
        events
            .into_iter()
            .map(|event| {
                Ok(SurfaceEvent::EditableBrush {
                    ids: self.editable_selection_ids(event.selection)?,
                    event,
                })
            })
            .collect()
    }

    fn editable_selection_ids(&self, selection: Option<Brush>) -> Result<Vec<String>> {
        let Some(selection) = selection else {
            return Ok(Vec::new());
        };
        selection.validate()?;
        let transform = self.paint_transform();
        let mut ids = Vec::new();
        for mark in &self.scene.marks {
            if !self.pointer_data(mark) {
                continue;
            }
            let Some([x, y]) = self.current_anchor(mark) else {
                continue;
            };
            if mark.clip.is_some_and(|clip| !clip.contains(x, y))
                || mark
                    .clip_path
                    .as_ref()
                    .is_some_and(|clip| !clip.contains(x, y))
            {
                continue;
            }
            let screen = transform.project((x, y))?;
            if self.plot().contains(screen.0, screen.1) && selection.contains(screen) {
                ids.push(mark.id.clone());
            }
        }
        Ok(ids)
    }

    /// Retire stale baselines before accepting a changed host plot. The caller
    /// already works on a cloned model, so any failure preserves its old frame.
    pub(super) fn fit_editable_brush(&mut self) -> Result<Vec<SurfaceEvent>> {
        let Some(editor) = &self.editable_brush else {
            return Ok(Vec::new());
        };
        let mut options = *editor.options();
        let plot = self.plot();
        let old = options.extent;
        options.extent = Brush {
            x0: old.x0.clamp(plot.x, plot.x + plot.width),
            x1: old.x1.clamp(plot.x, plot.x + plot.width),
            y0: old.y0.clamp(plot.y, plot.y + plot.height),
            y1: old.y1.clamp(plot.y, plot.y + plot.height),
        };
        if options.extent == old {
            return Ok(Vec::new());
        }
        let events = self
            .editable_brush
            .as_mut()
            .expect("checked editor")
            .configure(options, BrushReconfigure::Clamp)?;
        self.editable_events(events)
    }
}
