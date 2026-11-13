#![cfg(feature = "raster")]
use glory::{
    App, Widget,
    render::{Event, NodeId},
};
use kartis::{
    render::raster::*,
    surface::{SurfaceEvent, SurfaceInput},
    *,
};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 200.,
        height: 120.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x) in [("a", 20.), ("b", 100.)] {
        let mut m = Mark::new(
            id,
            Geometry::Rect {
                x,
                y: 20.,
                width: 50.,
                height: 40.,
            },
            Color(200, 30, 40, 255),
        );
        m.label = id.into();
        m.series = id.into();
        m.value = 1.25;
        scene.marks.push(m);
        scene.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color: Color(200, 30, 40, 255),
        });
    }
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut found = Vec::new();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        pending.extend(n.children.iter().copied());
    }
    found.reverse();
    found
}
fn mounted(app: &App) -> RasterSurface {
    let installed = Rc::new(RefCell::new(None));
    let output = installed.clone();
    app.mount(glory::ui::widget(move |scope| {
        let surface =
            RasterSurface::mount(scope, scene(), FontRegistry::new(), Default::default()).unwrap();
        let widget = surface.widget();
        *output.borrow_mut() = Some(surface);
        widget.show_in(scope)
    }))
    .unwrap();
    installed.borrow_mut().take().unwrap()
}

#[test]
fn explicit_offset_and_dpi_coordinates_drive_actual_model_and_single_frame() {
    let app = App::memory();
    let surface = mounted(&app);
    let handle = surface.handle();
    let bounds = RasterSurfaceBounds {
        x: 50.,
        y: 80.,
        width: 400.,
        height: 240.,
    };
    handle
        .dispatch_host(
            SurfaceInput::Move {
                id: 1,
                point: (110., 140.),
            },
            bounds,
        )
        .unwrap();
    assert_eq!(handle.snapshot().unwrap().hovered, ["a"]);
    handle
        .dispatch_host(
            SurfaceInput::Down {
                id: 1,
                point: (110., 140.),
                brush: false,
            },
            bounds,
        )
        .unwrap();
    handle
        .dispatch_host(
            SurfaceInput::Up {
                id: 1,
                point: (110., 140.),
            },
            bounds,
        )
        .unwrap();
    assert_eq!(handle.snapshot().unwrap().selected.as_deref(), Some("a"));
    assert_eq!(handle.frame().unwrap().width, 200);
    let before = handle.snapshot().unwrap();
    assert!(
        handle
            .dispatch_host(
                SurfaceInput::Move {
                    id: 1,
                    point: (100., 100.)
                },
                RasterSurfaceBounds {
                    width: 0.,
                    ..bounds
                }
            )
            .is_err()
    );
    assert_eq!(handle.snapshot().unwrap(), before);
}
#[test]
fn control_callbacks_feed_one_model_and_keep_legend_and_tooltip_current() {
    let app = App::memory();
    let surface = mounted(&app);
    let handle = surface.handle();
    let events = Rc::new(RefCell::new(Vec::new()));
    let seen = events.clone();
    surface
        .on_event(move |e| seen.borrow_mut().extend(e))
        .unwrap();
    let data = nodes(&app, "kartis-datum");
    app.dispatch(data[0], Event::Activate).unwrap();
    assert_eq!(handle.snapshot().unwrap().selected.as_deref(), Some("a"));
    assert!(
        events
            .borrow()
            .iter()
            .any(|e| matches!(e,SurfaceEvent::Activated(id)if id=="a"))
    );
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert!(handle.snapshot().unwrap().hidden.contains("a"));
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
}
#[test]
fn render_and_publication_failures_do_not_commit_model_controls_or_frame() {
    let app = App::memory();
    let owner = app.scope().child().unwrap();
    let fail = Rc::new(Cell::new(false));
    let f = fail.clone();
    let surface = RasterSurface::mount_published(
        &owner,
        scene(),
        FontRegistry::new(),
        Default::default(),
        move |image| {
            if f.get() {
                Err(ChartError("publication denied".into()))
            } else {
                image.data_url()
            }
        },
    )
    .unwrap();
    app.mount(surface.widget()).unwrap();
    let handle = surface.handle();
    let old = handle.snapshot().unwrap();
    let frame = handle.frame().unwrap();
    fail.set(true);
    assert!(handle.set_hidden(BTreeSet::from(["a".into()])).is_err());
    assert_eq!(handle.snapshot().unwrap(), old);
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    assert!(
        handle
            .set_raster_options(RasterOptions {
                pixel_ratio: f64::NAN,
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(handle.snapshot().unwrap(), old);
}
#[test]
fn resize_and_zoom_publish_current_dimensions_and_transform() {
    let app = App::memory();
    let surface = mounted(&app);
    let handle = surface.handle();
    handle
        .dispatch(SurfaceInput::Wheel {
            point: (100., 60.),
            delta: -200.,
        })
        .unwrap();
    assert!(handle.snapshot().unwrap().view.scale > 1.);
    let mut scene = scene();
    scene.width = 240.;
    handle.set_scene(scene).unwrap();
    app.flush().unwrap();
    assert_eq!(handle.frame().unwrap().logical_width, 240.);
    handle
        .set_raster_options(RasterOptions {
            pixel_ratio: 2.,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(handle.frame().unwrap().width, 480);
}
#[test]
fn drop_and_parent_cleanup_retire_controls_weak_handles_and_captures() {
    struct Guard(Rc<Cell<usize>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let app = App::memory();
    let surface = mounted(&app);
    let handle = surface.handle();
    let drops = Rc::new(Cell::new(0));
    let marker = Guard(drops.clone());
    surface
        .on_event(move |_| {
            let _ = &marker;
        })
        .unwrap();
    drop(surface);
    app.flush().unwrap();
    assert!(!handle.is_active());
    assert_eq!(drops.get(), 1);
    assert!(nodes(&app, "kartis-raster-surface-image").is_empty());
    assert_eq!(app.stats().handlers, 0);
    assert!(handle.dispatch(SurfaceInput::Next).is_err());
    let surface = mounted(&app);
    let handle = surface.handle();
    app.clear().unwrap();
    assert!(!handle.is_active());
    drop(surface);
}
#[test]
fn observer_may_stop_or_reenter_without_borrow_panics() {
    let app = App::memory();
    let surface = mounted(&app);
    let handle = surface.handle();
    let weak = handle.clone();
    surface.on_event(move |_| weak.stop()).unwrap();
    handle.dispatch(SurfaceInput::Next).unwrap();
    assert!(!handle.is_active());
    app.flush().unwrap();
    assert_eq!(app.stats().handlers, 0);
    let surface = mounted(&app);
    let handle = surface.handle();
    let weak = handle.clone();
    let first = Rc::new(Cell::new(true));
    surface
        .on_event(move |_| {
            if first.replace(false) {
                weak.set_hidden(BTreeSet::from(["b".into()])).unwrap();
            }
        })
        .unwrap();
    handle.dispatch(SurfaceInput::Next).unwrap();
    assert!(handle.snapshot().unwrap().hidden.contains("b"));
}

#[test]
fn control_publication_errors_are_observed_and_budget_changes_revalidate_render() {
    let app = App::memory();
    let failure = Rc::new(Cell::new(false));
    let fail = failure.clone();
    let surface = RasterSurface::mount_published(
        app.scope(),
        scene(),
        FontRegistry::new(),
        Default::default(),
        move |image| {
            if fail.get() {
                Err(ChartError("source rejected".into()))
            } else {
                image.data_url()
            }
        },
    )
    .unwrap();
    app.mount(surface.widget()).unwrap();
    let handle = surface.handle();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let output = errors.clone();
    surface
        .on_error(move |error| output.borrow_mut().push(error.0))
        .unwrap();
    let before = handle.snapshot().unwrap();
    failure.set(true);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(handle.snapshot().unwrap(), before);
    assert_eq!(*errors.borrow(), ["source rejected"]);
    failure.set(false);
    assert!(
        handle
            .set_raster_options(RasterOptions {
                max_operations: 1,
                ..Default::default()
            })
            .is_err()
    );
    assert_eq!(handle.snapshot().unwrap(), before);
}

#[test]
fn publication_reentrance_and_owner_retirement_cannot_commit_stale_frames() {
    let app = App::memory();
    let installed: Rc<RefCell<Option<RasterSurfaceHandle>>> = Rc::new(RefCell::new(None));
    let access = installed.clone();
    let retire = Rc::new(Cell::new(false));
    let flag = retire.clone();
    let surface = RasterSurface::mount_published(
        app.scope(),
        scene(),
        FontRegistry::new(),
        Default::default(),
        move |frame| {
            if flag.get() {
                access.borrow().as_ref().unwrap().stop();
            }
            frame.data_url()
        },
    )
    .unwrap();
    let handle = surface.handle();
    *installed.borrow_mut() = Some(handle.clone());
    app.mount(surface.widget()).unwrap();
    retire.set(true);
    assert!(handle.set_hidden(BTreeSet::from(["a".into()])).is_err());
    app.flush().unwrap();
    assert!(!handle.is_active());
    assert!(nodes(&app, "kartis-raster-surface-image").is_empty());
    let app = Rc::new(App::memory());
    let weak = Rc::downgrade(&app);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        RasterSurface::mount_published(
            app.scope(),
            scene(),
            FontRegistry::new(),
            Default::default(),
            move |frame| {
                weak.upgrade().unwrap().scope().owner().dispose();
                frame.data_url()
            },
        )
    }));
    assert!(result.is_ok() && result.unwrap().is_err());
}

#[test]
fn failed_initial_admission_drops_publisher_captures_before_parent_cleanup() {
    struct Guard(Rc<Cell<usize>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let app = App::memory();
    let drops = Rc::new(Cell::new(0));
    let marker = Guard(drops.clone());
    let result = RasterSurface::mount_published(
        app.scope(),
        scene(),
        FontRegistry::new(),
        Default::default(),
        move |_| {
            let _ = &marker;
            Err(ChartError("no source".into()))
        },
    );
    assert!(result.is_err());
    assert_eq!(drops.get(), 1);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

fn typed_scene(missing: bool) -> SceneInteraction {
    let mut scene = scene();
    scene.marks[0].value = 0.;
    let mut decoration = scene.marks[0].clone();
    decoration.id = "decoration".into();
    decoration.geometry = Geometry::Circle {
        x: 180.,
        y: 90.,
        radius: 5.,
    };
    scene.marks.push(decoration);
    let interaction = InteractionBinding {
        sites: scene.marks[..2]
            .iter()
            .enumerate()
            .map(|(i, mark)| InteractionSite {
                mark_id: mark.id.clone(),
                series: mark.series.clone(),
                label: format!("source {}", mark.id),
                value: if i == 0 && missing {
                    None
                } else {
                    Some(mark.value)
                },
                formatted_value: Some(format!("raw {}", mark.value)),
                color: mark.fill.unwrap(),
                position: [45. + i as f64 * 80., 40.],
            })
            .collect(),
        ..Default::default()
    };
    SceneInteraction {
        scene,
        interaction: Some(interaction),
        plot: Some(ClipRect {
            x: 10.,
            y: 10.,
            width: 180.,
            height: 100.,
        }),
    }
}

fn button_text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let glory::render::Props::Button(button) = tree.node(id).unwrap().descriptor.view().props
    else {
        panic!("expected button");
    };
    button.label.clone()
}

#[test]
fn typed_raster_controls_and_tooltips_preserve_missing_values_and_source_rows() {
    let app = App::memory();
    let surface = RasterSurface::mount_layout(
        app.scope(),
        typed_scene(true),
        FontRegistry::new(),
        RasterSurfaceOptions {
            tooltip: TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let handle = surface.handle();
    app.mount(surface.widget()).unwrap();
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(
        controls.len(),
        2,
        "unbound decoration must not become a data control"
    );
    assert_eq!(button_text(&app, controls[0]), "a: missing");
    app.dispatch(controls[0], Event::Activate).unwrap();
    let active = handle.active_interaction().unwrap().unwrap();
    assert_eq!(active.ids, ["a"]);
    assert_eq!(active.anchor, [45., 40.]);
    assert_eq!(active.rows[0].value, None);
    assert_eq!(active.rows[0].label, "source a");
    handle
        .dispatch(SurfaceInput::Move {
            id: 1,
            point: (45., 40.),
        })
        .unwrap();
    app.flush().unwrap();
    let tree = app.tree();
    let tooltip = nodes(&app, "kartis-tooltip");
    assert!(
        tooltip.iter().any(|id| {
            matches!(tree.node(*id).unwrap().descriptor.view().props,
            glory::render::Props::Text(text) if text.content == "source a: missing")
        }),
        "typed tooltip must use nullable source metadata"
    );
    drop(tree);
    handle.set_layout(typed_scene(false)).unwrap();
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    assert_eq!(button_text(&app, controls[0]), "a: 0");
    assert_eq!(
        handle.active_interaction().unwrap().unwrap().rows[0].value,
        Some(0.)
    );
    assert_eq!(
        handle.interaction_data().unwrap().unwrap().sites[0].value,
        Some(0.)
    );
}

#[test]
fn typed_raster_layout_rejection_retains_binding_plot_image_and_control_identity() {
    let app = App::memory();
    let failed = Rc::new(Cell::new(false));
    let failure = failed.clone();
    let publications = Rc::new(Cell::new(0));
    let published = publications.clone();
    let surface = RasterSurface::mount_layout_published(
        app.scope(),
        typed_scene(true),
        FontRegistry::new(),
        Default::default(),
        move |image| {
            published.set(published.get() + 1);
            if failure.get() {
                Err(ChartError("typed publication rejected".into()))
            } else {
                image.data_url()
            }
        },
    )
    .unwrap();
    let handle = surface.handle();
    app.mount(surface.widget()).unwrap();
    let controls = nodes(&app, "kartis-datum");
    let image = handle.frame().unwrap();
    let before = handle.snapshot().unwrap();
    let binding = handle.interaction_data().unwrap();
    let mut invalid = typed_scene(false);
    invalid.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
    assert!(handle.set_layout(invalid).is_err());
    assert_eq!(
        publications.get(),
        1,
        "reject binding before invoking publisher"
    );
    assert_eq!(handle.snapshot().unwrap(), before);
    assert_eq!(handle.interaction_data().unwrap(), binding);
    let mut resized = typed_scene(false);
    resized.scene.width = 240.;
    resized.scene.marks[0].fill = Some(Color(20, 140, 70, 255));
    resized.plot.as_mut().unwrap().width = 220.;
    failed.set(true);
    assert!(handle.set_layout(resized.clone()).is_err());
    app.flush().unwrap();
    assert_eq!(handle.snapshot().unwrap(), before);
    assert_eq!(handle.interaction_data().unwrap(), binding);
    assert!(Rc::ptr_eq(&image, &handle.frame().unwrap()));
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    assert_eq!(button_text(&app, controls[0]), "a: missing");
    failed.set(false);
    handle.set_layout(resized).unwrap();
    app.flush().unwrap();
    assert_eq!(handle.snapshot().unwrap().width, 240.);
    assert_eq!(
        handle.interaction_data().unwrap().unwrap().sites[0].value,
        Some(0.)
    );
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    let pixels = handle.frame().unwrap();
    let offset = (40 * pixels.width as usize + 45) * 4;
    assert_eq!(&pixels.rgba[offset..offset + 4], &[20, 140, 70, 255]);
    let published = publications.get();
    handle.set_layout(typed_scene(true)).unwrap();
    app.flush().unwrap();
    assert_eq!(handle.snapshot().unwrap().width, 200.);
    assert_eq!(publications.get(), published + 1);
    let image = handle.frame().unwrap();
    handle.set_layout(typed_scene(false)).unwrap();
    app.flush().unwrap();
    assert!(
        Rc::ptr_eq(&image, &handle.frame().unwrap()),
        "metadata-only update retains encoded pixels"
    );
    assert_eq!(publications.get(), published + 1);
    handle.set_scene(scene()).unwrap();
    app.flush().unwrap();
    assert_eq!(handle.interaction_data().unwrap(), None);
    assert!(handle.active_interaction().unwrap().is_none());
}
