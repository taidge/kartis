#![cfg(feature = "raster")]
use kartis::{render::raster::*, *};
use std::{cell::Cell, rc::Rc};

#[test]
fn initial_publisher_may_retire_the_build_owner_without_panicking_or_leaking() {
    let app = Rc::new(glory::App::memory());
    let weak = Rc::downgrade(&app);
    let calls = Rc::new(Cell::new(0));
    let count = calls.clone();
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        app.scope().mount_scoped(|_| {
            raster_chart(
                ChartConfig::default().scene().unwrap(),
                FontRegistry::new(),
                RasterOptions::default(),
            )
            .raster_image_source(move |frame| {
                count.set(count.get() + 1);
                weak.upgrade().unwrap().scope().owner().dispose();
                frame.data_url()
            })
        })
    }));
    assert!(
        outcome.is_ok(),
        "retired initial publication must reject the build, not panic while creating owner cages"
    );
    assert!(outcome.unwrap().is_err());
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
