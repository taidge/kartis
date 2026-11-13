#![cfg(feature = "raster")]
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::App;
use glory::render::{Event, Props};
use kartis::render::raster::*;
use kartis::surface::{SurfaceEvent, SurfaceInput};
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 200.,
        height: 140.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x) in [("__kartis-editable-brush:selection", 20.), ("body", 100.)] {
        let mut mark = Mark::new(
            id,
            Geometry::Rect {
                x,
                y: 25.,
                width: 35.,
                height: 30.,
            },
            Color(210, 40, 40, 255),
        );
        mark.series = id.into();
        mark.label = id.into();
        scene.marks.push(mark);
    }
    // This catches viewport overlay marks accidentally omitted from an authored
    // non-default order. It also forces an actual namespace collision.
    scene.draw_order = vec![DrawItem::Mark(1), DrawItem::Mark(0)];
    scene
}
fn editor() -> EditableBrushOptions {
    EditableBrushOptions {
        extent: Brush {
            x0: 10.,
            y0: 10.,
            x1: 190.,
            y1: 130.,
        },
        handle_size: 10.,
        ..Default::default()
    }
}
fn selection() -> Brush {
    Brush {
        x0: 70.,
        y0: 50.,
        x1: 130.,
        y1: 90.,
    }
}
fn options(ratio: f64) -> RasterSurfaceOptions {
    RasterSurfaceOptions {
        editable_brush: Some(editor()),
        raster: RasterOptions {
            pixel_ratio: ratio,
            ..Default::default()
        },
        ..Default::default()
    }
}
fn blue(image: &RenderedImage) -> usize {
    image
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| p[2] > p[0].saturating_add(70) && p[2] > p[1].saturating_add(50) && p[3] > 0)
        .count()
}
fn blue_near(image: &RenderedImage, x: f64, y: f64) -> bool {
    let x = (x * image.pixel_ratio) as i32;
    let y = (y * image.pixel_ratio) as i32;
    (-2..=2).any(|dy| {
        (-2..=2).any(|dx| {
            let (x, y) = (x + dx, y + dy);
            if x < 0 || y < 0 || x >= image.width as i32 || y >= image.height as i32 {
                return false;
            }
            let i = (y as usize * image.width as usize + x as usize) * 4;
            image.rgba[i + 2] > image.rgba[i].saturating_add(70)
        })
    })
}

#[test]
fn actual_cpu_overlay_survives_authored_order_id_collision_and_dpr_without_camera_drift() {
    for ratio in [1., 2.] {
        let app = App::memory();
        let surface =
            RasterSurface::mount(app.scope(), scene(), FontRegistry::new(), options(ratio))
                .unwrap();
        let handle = surface.handle();
        assert_eq!(blue(&handle.frame().unwrap()), 0);
        handle
            .dispatch(SurfaceInput::Wheel {
                point: (50., 50.),
                delta: -2_f64.ln() / 0.002,
            })
            .unwrap();
        assert!((handle.snapshot().unwrap().view.scale - 2.).abs() < 1e-12);
        let business = handle.frame().unwrap();
        handle.set_brush_selection(Some(selection())).unwrap();
        let painted = handle.frame().unwrap();
        assert!(blue(&painted) > (100. * ratio * ratio) as usize);
        // Bounds of actual viewport handles, not inverse scene coordinates.
        for (x, y) in [(65., 45.), (125., 45.), (65., 85.), (125., 85.)] {
            assert!(
                blue_near(&painted, x, y),
                "DPR{ratio}: missing actual handle at {x},{y}"
            );
        }
        let brush = handle.editable_brush().unwrap().unwrap();
        assert_eq!(brush.selection, Some(selection()));
        assert_eq!(brush.handles.len(), 8);
        handle.set_brush_selection(None).unwrap();
        assert_eq!(handle.frame().unwrap().rgba, business.rgba);
    }
}

#[test]
fn physical_host_input_rebases_once_and_programmatic_updates_retire_the_old_session() {
    let app = App::memory();
    let surface =
        RasterSurface::mount(app.scope(), scene(), FontRegistry::new(), options(2.)).unwrap();
    let handle = surface.handle();
    let bounds = RasterSurfaceBounds {
        x: 50.,
        y: 80.,
        width: 400.,
        height: 280.,
    };
    handle
        .dispatch_editable_brush_host(
            EditableBrushInput::Begin {
                id: 7,
                point: (90., 120.),
                target: BrushTarget::Overlay,
                modifiers: Default::default(),
            },
            bounds,
        )
        .unwrap();
    let session = handle.editable_brush().unwrap().unwrap().session.unwrap();
    handle
        .dispatch_editable_brush_host(
            EditableBrushInput::Move {
                session,
                id: 7,
                point: (290., 260.),
            },
            bounds,
        )
        .unwrap();
    assert_eq!(
        handle.editable_brush().unwrap().unwrap().selection,
        Some(Brush {
            x0: 20.,
            y0: 20.,
            x1: 120.,
            y1: 90.
        })
    );
    handle.set_brush_selection(Some(selection())).unwrap();
    assert!(!handle.editable_brush().unwrap().unwrap().active);
    let image = handle.frame().unwrap();
    assert!(
        handle
            .dispatch_editable_brush_host(
                EditableBrushInput::End {
                    session,
                    id: 7,
                    point: (400., 320.)
                },
                bounds
            )
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        handle.editable_brush().unwrap().unwrap().selection,
        Some(selection())
    );
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    assert_eq!(
        handle
            .dispatch_editable_brush_host(
                EditableBrushInput::Begin {
                    id: 8,
                    point: (90., 120.),
                    target: BrushTarget::Overlay,
                    modifiers: Default::default()
                },
                RasterSurfaceBounds {
                    width: 0.,
                    ..bounds
                }
            )
            .unwrap_err()
            .0,
        "invalid raster host input bounds"
    );
}

#[test]
fn publisher_failure_preserves_complete_selection_session_frame_and_revision_then_retries() {
    let app = App::memory();
    let reject = Rc::new(Cell::new(false));
    let gate = reject.clone();
    let surface = RasterSurface::mount_published(
        app.scope(),
        scene(),
        FontRegistry::new(),
        options(1.),
        move |_| {
            if gate.get() {
                Err(ChartError("publisher refused new frame".into()))
            } else {
                Ok("image://accepted".into())
            }
        },
    )
    .unwrap();
    let handle = surface.handle();
    handle.set_brush_selection(Some(selection())).unwrap();
    handle
        .dispatch_editable_brush(EditableBrushInput::Begin {
            id: 1,
            point: (130., 90.),
            target: BrushTarget::Handle(BrushHandle::Se),
            modifiers: Default::default(),
        })
        .unwrap();
    let before = handle.snapshot().unwrap();
    let image = handle.frame().unwrap();
    let session = before.editable_brush.as_ref().unwrap().session.unwrap();
    let movement = EditableBrushInput::Move {
        session,
        id: 1,
        point: (155., 110.),
    };
    reject.set(true);
    assert!(handle.dispatch_editable_brush(movement).is_err());
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&handle.frame().unwrap(), &image));
    reject.set(false);
    handle.dispatch_editable_brush(movement).unwrap();
    assert_eq!(
        handle.editable_brush().unwrap().unwrap().selection,
        Some(Brush {
            x1: 155.,
            y1: 110.,
            ..selection()
        })
    );
}

#[test]
fn actual_observer_replacement_retires_callback_session_and_releases_owner_capture() {
    let app = App::memory();
    let surface =
        RasterSurface::mount(app.scope(), scene(), FontRegistry::new(), options(1.)).unwrap();
    let handle = surface.handle();
    let nested = handle.clone();
    let capture = Rc::new(());
    let weak = Rc::downgrade(&capture);
    let calls = Rc::new(RefCell::new(Vec::new()));
    let observed = calls.clone();
    surface
        .on_event(move |events| {
            let _ = &capture;
            if events.iter().any(|event| {
                matches!(
                    event,
                    SurfaceEvent::EditableBrush {
                        event: EditableBrushEvent {
                            phase: BrushPhase::Start,
                            source: BrushSource::User,
                            ..
                        },
                        ..
                    }
                )
            }) {
                observed.borrow_mut().push("replace");
                nested.set_brush_selection(Some(selection())).unwrap();
            }
        })
        .unwrap();
    handle
        .dispatch_editable_brush(EditableBrushInput::Begin {
            id: 2,
            point: (20., 20.),
            target: BrushTarget::Overlay,
            modifiers: Default::default(),
        })
        .unwrap();
    assert_eq!(*calls.borrow(), ["replace"]);
    assert!(!handle.editable_brush().unwrap().unwrap().active);
    assert_eq!(
        handle.editable_brush().unwrap().unwrap().selection,
        Some(selection())
    );
    surface.stop();
    assert!(weak.upgrade().is_none());
    assert!(!handle.is_active());
}

#[test]
fn invalid_configuration_and_scene_resize_are_atomic_and_do_not_reinterpret_pixels_as_data() {
    let app = App::memory();
    let surface =
        RasterSurface::mount(app.scope(), scene(), FontRegistry::new(), options(1.)).unwrap();
    let handle = surface.handle();
    handle.set_brush_selection(Some(selection())).unwrap();
    let before = handle.snapshot().unwrap();
    let image = handle.frame().unwrap();
    assert!(
        handle
            .configure_editable_brush(
                Some(EditableBrushOptions {
                    extent: Brush {
                        x1: 250.,
                        ..editor().extent
                    },
                    ..editor()
                }),
                BrushReconfigure::Preserve
            )
            .is_err()
    );
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    let mut small = scene();
    small.width = 110.;
    handle.set_scene(small).unwrap();
    let state = handle.editable_brush().unwrap().unwrap();
    assert_eq!(state.options.extent.x1, 110.);
    assert_eq!(state.selection.unwrap().x1, 110.);
    assert!(!state.active);
}

#[test]
fn named_intrinsic_keyboard_controls_share_axis_availability_and_current_business_ids() {
    let app = App::memory();
    let surface =
        RasterSurface::mount(app.scope(), scene(), FontRegistry::new(), options(1.)).unwrap();
    let handle = surface.handle();
    app.mount(surface.widget()).unwrap();
    let find = |class: &str| {
        let tree = app.tree();
        let mut pending = tree.roots().to_vec();
        while let Some(id) = pending.pop() {
            let n = tree.node(id).unwrap();
            if n.descriptor.view().classes.iter().any(|c| c == class) {
                return id;
            }
            pending.extend(n.children.iter().copied());
        }
        panic!("missing native brush control {class}");
    };
    let enabled = |class: &str| {
        let id = find(class);
        let tree = app.tree();
        let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
            panic!("expected intrinsic button")
        };
        button.enabled
    };
    assert!(!enabled("kartis-raster-brush-target-n"));
    assert!(!enabled("kartis-raster-brush-nudge-right"));
    let selected = Brush {
        x0: 70.,
        y0: 20.,
        x1: 140.,
        y1: 70.,
    };
    handle.set_brush_selection(Some(selected)).unwrap();
    app.flush().unwrap();
    assert_eq!(handle.editable_brush().unwrap().unwrap().ids, ["body"]);
    assert!(enabled("kartis-raster-brush-target-n"));
    assert!(enabled("kartis-raster-brush-nudge-right"));
    handle
        .set_hidden(std::collections::BTreeSet::from(["body".into()]))
        .unwrap();
    assert!(handle.editable_brush().unwrap().unwrap().ids.is_empty());
    handle.set_hidden(Default::default()).unwrap();
    assert_eq!(handle.editable_brush().unwrap().unwrap().ids, ["body"]);
    // Retire a focused top-edge choice when switching to X; directional buttons
    // deliberately edit the remaining selection rather than emit an invalid N handle.
    app.dispatch(find("kartis-raster-brush-target-n"), Event::Activate)
        .unwrap();
    handle
        .configure_editable_brush(
            Some(EditableBrushOptions {
                axis: BrushAxis::X,
                ..editor()
            }),
            BrushReconfigure::Preserve,
        )
        .unwrap();
    app.flush().unwrap();
    assert!(!enabled("kartis-raster-brush-target-n"));
    assert!(enabled("kartis-raster-brush-target-e"));
    let before = handle.editable_brush().unwrap().unwrap().selection.unwrap();
    app.dispatch(find("kartis-raster-brush-nudge-right"), Event::Activate)
        .unwrap();
    assert_eq!(
        handle.editable_brush().unwrap().unwrap().selection,
        Some(Brush {
            x0: before.x0 + 1.,
            x1: before.x1 + 1.,
            ..before
        })
    );
    let top = find("kartis-raster-brush-handle-n");
    assert_eq!(
        app.tree()
            .node(top)
            .unwrap()
            .descriptor
            .view()
            .name
            .as_deref(),
        Some("Brush top edge")
    );
    surface.stop();
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
