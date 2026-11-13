#![cfg(feature = "raster")]
use kartis::render::raster::*;
use kartis::surface::*;
use kartis::*;
#[path = "contour_common/layout.rs"]
mod common;
use common::*;
fn controls(app: &glory::App) -> Vec<glory::render::NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut output = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "kartis-datum")
        {
            output.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    output
}

#[test]
fn actual_raster_host_uses_typed_threshold_controls_and_rolls_back_failed_image_publication() {
    use std::cell::Cell;
    use std::rc::Rc;
    let c = frame(340., 340.);
    let values = nested();
    let first = contour_layout(&c, grid(&values), &compute(vec![1.]), &teal()).unwrap();
    let next = contour_layout(&c, grid(&values), &compute(vec![1.5]), &teal()).unwrap();
    let failure = Rc::new(Cell::new(false));
    let fail = failure.clone();
    let app = glory::App::memory();
    let surface = RasterSurface::mount_layout_published(
        app.scope(),
        first.scene_interaction(),
        FontRegistry::new(),
        RasterSurfaceOptions {
            highlight: false,
            ..Default::default()
        },
        move |image| {
            if fail.get() {
                Err(ChartError(
                    "isolated contour image publication failure".into(),
                ))
            } else {
                image.data_url()
            }
        },
    )
    .unwrap();
    let handle = surface.handle();
    app.mount(surface.widget()).unwrap();
    app.flush().unwrap();
    let buttons = controls(&app);
    assert_eq!(buttons.len(), 1);
    app.dispatch(buttons[0], glory::render::Event::Activate)
        .unwrap();
    assert_eq!(
        handle.active_interaction().unwrap().unwrap().rows[0].value,
        Some(1.)
    );
    let bounds = RasterSurfaceBounds {
        x: 100.,
        y: 50.,
        width: 680.,
        height: 680.,
    };
    for (x, filled) in [(30., true), (70., false), (110., true), (170., false)] {
        handle
            .dispatch_host(
                SurfaceInput::Move {
                    id: 1,
                    point: (100. + x * 2., 50. + 170. * 2.),
                },
                bounds,
            )
            .unwrap();
        assert_eq!(
            !handle.snapshot().unwrap().hovered.is_empty(),
            filled,
            "real RasterSurface exact hole x{x}"
        );
    }
    let before = handle.snapshot().unwrap();
    let image = handle.frame().unwrap();
    let binding = handle.interaction_data().unwrap();
    failure.set(true);
    assert!(handle.set_layout(next.scene_interaction()).is_err());
    app.flush().unwrap();
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    assert_eq!(handle.interaction_data().unwrap(), binding);
    assert_eq!(controls(&app), buttons);
    failure.set(false);
    handle.set_layout(next.scene_interaction()).unwrap();
    app.flush().unwrap();
    assert_eq!(
        handle.interaction_data().unwrap().unwrap().sites[0].value,
        Some(1.5)
    );
    let new = controls(&app);
    assert_eq!(new.len(), 1);
    assert_ne!(new, buttons);
    surface.stop();
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert!(!handle.is_active());
}
