//! The actual Image/View coordinate channel. No display-only mark or hit rect
//! is added; real coordinates still have to pass Surface's painted-ink picker.
use super::*;
use glory::render::{Event, EventKind, PlotPointer};

pub(super) fn strict(model: &SurfaceModel) -> bool {
    model
        .interaction_data()
        .is_some_and(|b| b.picking_profile == PickingProfile::PaintedGeometry)
}
pub(super) fn admit(
    scope: &Scope,
    model: &SurfaceModel,
    options: &RasterSurfaceOptions,
) -> Result<()> {
    if !strict(model) {
        return Ok(());
    }
    scene::ensure(
        options.editable_brush.is_none(),
        "strict raster plot pointer does not support the editable-brush input plane",
    )?;
    let input = scope.capabilities().map_err(error)?.input;
    scene::ensure(
        !input || scope.supports_plot_pointer().map_err(error)?,
        "raster plot requires finite pointer coordinates from its interactive host",
    )
}
/// Called on every candidate, before pixel encoding/publication. Registration
/// capability cannot change while the RasterSurface owner remains mounted.
pub(super) fn dispatch(engine: &Rc<Engine>, pointer: PlotPointer) -> Result<Vec<SurfaceEvent>> {
    pointer.validate().map_err(|e| ChartError(e.to_string()))?;
    engine.update(|model, _| {
        // Profile is rechecked at delivery, including events queued before a
        // Legacy/Painted layout replacement. Never interpret legacy controls.
        if !strict(model) {
            return Ok(vec![]);
        }
        crate::widget::plot_pointer::apply_pointer(model, pointer)
    })
}
/// Install an owner-scoped interest on the existing image-host View only
/// while the current accepted frame explicitly selects PaintedGeometry.
/// No new View/Group is inserted on the legacy path.
pub(super) fn image_channel(
    scope: &Scope,
    node: Cage<Option<glory::render::NodeId>>,
    engine: Weak<Engine>,
) -> glory::ui::Result {
    let listener = Rc::new(RefCell::new(None::<Scope>));
    let cleanup = listener.clone();
    scope.on_cleanup(move || {
        if let Some(owner) = cleanup.borrow_mut().take() {
            owner.owner().dispose();
        }
    })?;
    let weak = scope.downgrade();
    scope.structural(move || {
        let Some(scope) = weak.upgrade() else {
            return;
        };
        let Some(engine) = engine.upgrade() else {
            return;
        };
        let enabled = !engine.data_pointer.get();
        if !enabled {
            if let Some(owner) = listener.borrow_mut().take() {
                owner.owner().dispose();
            }
            return;
        }
        if listener.borrow().is_some() {
            return;
        }
        let Some(node) = node.get() else {
            return;
        };
        let result = (|| -> glory::ui::Result<Scope> {
            let child = scope.child()?;
            let observed = Rc::downgrade(&engine);
            if let Err(error) = child.on(node, EventKind::PlotPointer, move |event| {
                let Event::PlotPointer { pointer, .. } = event else {
                    return;
                };
                if let Some(engine) = observed.upgrade() {
                    if !engine.alive.get() {
                        return;
                    }
                    let result = dispatch(&engine, pointer);
                    RasterSurfaceHandle {
                        engine: Rc::downgrade(&engine),
                    }
                    .report(result);
                }
            }) {
                child.owner().dispose();
                return Err(error);
            }
            Ok(child)
        })();
        match result {
            Ok(child) => *listener.borrow_mut() = Some(child),
            Err(error) => scope.report_error(error),
        }
    });
    Ok(())
}
