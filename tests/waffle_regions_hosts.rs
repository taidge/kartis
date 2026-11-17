//! Actual Glory Memory event channel; this is not OS input transport.
use std::cell::RefCell;
use std::rc::Rc;

use glory::App;
use glory::render::{Event, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
use kartis::*;
#[path = "../examples/shared/waffle_regions.rs"]
mod model;

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|v| v == class) {
            result.push(id);
        }
        todo.extend(n.children.iter().copied());
    }
    result
}
fn check(phase: u8, raster: bool, dpr: f64) {
    let layout = model::source(phase, 320.).unwrap();
    let frame = waffle_area_interaction(&layout).unwrap();
    let expected = frame.interaction.as_ref().unwrap().sites[0].mark_id.clone();
    let first = frame.interaction.as_ref().unwrap().regions[0].bounds[0];
    let gap = [first.x + 1., first.y + first.height / 2.];
    let center = [first.x + first.width / 2., first.y + first.height / 2.];
    let app = App::memory();
    let logical = Rc::new(RefCell::new(vec![]));
    let hover = logical.clone();
    let physical = Rc::new(RefCell::new(vec![]));
    let ink = physical.clone();
    let selected = Rc::new(RefCell::new(vec![]));
    let select = selected.clone();
    let chart = chart_layout(frame)
        .legend(false)
        .tooltip(false)
        .highlight(false)
        .on_hover(move |m| hover.borrow_mut().push(m.map(|m| m.id)))
        .on_physical_hover(move |h| ink.borrow_mut().push(h))
        .on_select(move |m| select.borrow_mut().push(m.id));
    #[cfg(feature = "raster")]
    let chart = if raster {
        chart.raster(
            kartis::render::raster::FontRegistry::new(),
            kartis::render::raster::RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
    } else {
        chart
    };
    #[cfg(not(feature = "raster"))]
    assert!(!raster && dpr == 1.);
    app.mount(chart).unwrap();
    assert_eq!(
        nodes(&app, "kartis-datum").len(),
        2,
        "one control per occupied group"
    );
    let node = nodes(
        &app,
        if raster {
            "kartis-raster-plot"
        } else {
            "kartis-plot"
        },
    )[0];
    let input = |phase, point: [f64; 2]| {
        assert!(
            app.dispatch(
                node,
                Event::PlotPointer {
                    from: node,
                    pointer: PlotPointer {
                        pointer_id: 91,
                        device: PlotPointerDevice::Pen,
                        phase,
                        x: point[0] * 1.5,
                        y: point[1] * 2.,
                        viewport: Rect {
                            width: 480.,
                            height: 480.,
                            ..Default::default()
                        }
                    }
                }
            )
            .unwrap()
        );
        app.flush().unwrap();
    };
    input(PlotPointerPhase::Move, center);
    input(PlotPointerPhase::Move, gap);
    assert_eq!(logical.borrow().last(), Some(&Some(expected.clone())));
    if phase == 0 {
        assert!(physical.borrow().iter().any(Option::is_some));
        assert_eq!(physical.borrow().last(), Some(&None));
    }
    if phase == 1 {
        assert!(physical.borrow().iter().all(Option::is_none));
    }
    input(PlotPointerPhase::Down, gap);
    input(PlotPointerPhase::Up, gap);
    assert_eq!(*selected.borrow(), [expected]);
    app.clear().unwrap();
}
#[test]
fn semantic_plot_uses_full_slot_regions_without_transparent_ink() {
    for phase in [0, 1, 3, 4] {
        check(phase, false, 1.);
    }
}
#[cfg(feature = "raster")]
#[test]
fn cpu_image_plot_uses_same_typed_regions_at_both_pixel_ratios() {
    for dpr in [1., 2.] {
        for phase in [0, 1, 3, 4] {
            check(phase, true, dpr);
        }
    }
}
