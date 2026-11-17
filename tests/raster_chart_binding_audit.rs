#![cfg(feature = "raster")]
//! Independent Memory-port and CPU-PNG checks. Explicit dispatch below is not
//! evidence of browser/native event transport or physical-device acceptance.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::rc::Rc;

use base64::Engine;
use glory::render::{
    Event, EventKind, NodeId, PlotPointer, PlotPointerDevice, PlotPointerPhase, Props, Rect,
};
use glory::{App, Widget};
use kartis::render::raster::{FontRegistry, RasterOptions};
use kartis::*;

fn source(value: Option<f64>, color: Color) -> SceneInteraction {
    let mut scene = ChartConfig {
        width: 300.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "owner",
        Geometry::Circle {
            x: 150.,
            y: 60.,
            radius: 20.,
        },
        color,
    );
    mark.series = "source".into();
    mark.label = "Owner".into();
    mark.value = value.unwrap_or(0.);
    scene.marks.push(mark);
    SceneInteraction {
        scene,
        plot: Some(ClipRect {
            x: 40.,
            y: 25.,
            width: 220.,
            height: 100.,
        }),
        interaction: Some(InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: "owner".into(),
                series: "source".into(),
                label: "Owner".into(),
                value,
                formatted_value: value.map(|v| format!("raw {v}")),
                color,
                position: [150., 60.],
            }],
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        }),
    }
}
fn large_view() -> ZoomTransform {
    ZoomTransform::new(2., -30., -20.).unwrap()
}
fn small_view() -> ZoomTransform {
    ZoomTransform::new(0.5, 30., 20.).unwrap()
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        stack.extend(node.children.iter().copied());
    }
    found
}
fn first(app: &App, class: &str) -> NodeId {
    let found = nodes(app, class);
    assert_eq!(found.len(), 1, "one actual {class}");
    found[0]
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real button required")
    };
    button.label.clone()
}
fn image(app: &App) -> String {
    let tree = app.tree();
    let Props::Image(image) = tree
        .node(first(app, "kartis-raster-image"))
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("real PNG image required")
    };
    image.source.clone()
}
fn pixel(url: &str, point: [f64; 2], ratio: f64) -> [u8; 4] {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(url.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(
        (info.width, info.height),
        ((300. * ratio) as u32, (150. * ratio) as u32)
    );
    let x = (point[0] * ratio) as usize;
    let y = (point[1] * ratio) as usize;
    pixels[(y * info.width as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}
fn pointer(app: &App, viewport: NodeId, phase: PlotPointerPhase, visual: [f64; 2]) {
    // An explicitly different, nonuniform CSS viewport proves normalization
    // uses the event's logical viewport and never PNG backing pixels/DPR.
    assert!(
        app.dispatch(
            viewport,
            Event::PlotPointer {
                from: viewport,
                pointer: PlotPointer {
                    pointer_id: 17,
                    device: PlotPointerDevice::Pen,
                    phase,
                    x: visual[0] * 2.,
                    y: visual[1] * 2.5,
                    viewport: Rect {
                        x: 0.,
                        y: 0.,
                        width: 600.,
                        height: 375.
                    },
                }
            }
        )
        .unwrap()
    );
}
fn last_hit(hits: &Rc<RefCell<Vec<Option<SurfacePhysicalHit>>>>) -> SurfacePhysicalHit {
    hits.borrow()
        .last()
        .expect("physical observation")
        .clone()
        .expect("material physical hit")
}
fn world(hit: &SurfacePhysicalHit, expected: [f64; 2]) {
    assert_eq!(hit.physical_mark_id, "owner");
    assert_eq!(hit.logical_site_id, "owner");
    assert!(
        hit.scene_point
            .iter()
            .zip(expected)
            .all(|(a, b)| (a - b).abs() < 1e-9),
        "{hit:?}"
    );
}

#[test]
fn published_image_leaf_rejects_plot_pointer_while_its_physical_view_is_a_port() {
    let app = App::memory();
    let rejected = glory::ui::image()
        .source("audit.png")
        .alt("Audit PNG")
        .on(EventKind::PlotPointer, |_| {})
        .mount(app.scope());
    let error = match rejected {
        Err(error) => error.to_string(),
        Ok(_) => panic!("Image is not a physical View"),
    };
    assert!(error.contains("physical View viewport"), "{error}");
    let seen = Rc::new(Cell::new(0));
    let observed = seen.clone();
    let mounted = glory::ui::view()
        .on(EventKind::PlotPointer, move |_| {
            observed.set(observed.get() + 1)
        })
        .fill(glory::ui::image().source("audit.png").alt("Audit PNG"))
        .mount(app.scope())
        .unwrap();
    app.flush().unwrap();
    let viewport = mounted.roots()[0];
    pointer(&app, viewport, PlotPointerPhase::Move, [21.375, 34.625]);
    assert_eq!(seen.get(), 1);
    drop(mounted);
    app.flush().unwrap();
    assert!(
        !app.dispatch(
            viewport,
            Event::PlotPointer {
                from: viewport,
                pointer: PlotPointer {
                    pointer_id: 17,
                    device: PlotPointerDevice::Pen,
                    phase: PlotPointerPhase::Cancel,
                    x: 0.,
                    y: 0.,
                    viewport: Rect::default(),
                }
            }
        )
        .unwrap()
    );
    assert_eq!(seen.get(), 1);
}

#[test]
fn absolute_camera_nonzero_plot_and_three_visual_pixel_click_slop_agree_with_png() {
    let red = Color(210, 40, 50, 255);
    for ratio in [1., 2.] {
        for (view, center) in [(large_view(), [270., 100.]), (small_view(), [105., 50.])] {
            let app = App::memory();
            let hits = Rc::new(RefCell::new(vec![]));
            let activations = Rc::new(RefCell::new(vec![]));
            let (observed, selected) = (hits.clone(), activations.clone());
            app.mount(
                chart_layout(source(None, red))
                    .highlight(false)
                    .raster(
                        FontRegistry::new(),
                        RasterOptions {
                            pixel_ratio: ratio,
                            view,
                            ..Default::default()
                        },
                    )
                    .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
                    .on_select(move |mark| selected.borrow_mut().push(mark)),
            )
            .unwrap();
            let viewport = first(&app, "kartis-raster-plot");
            assert!(matches!(
                app.tree().node(viewport).unwrap().descriptor.view().props,
                Props::View
            ));
            let png = image(&app);
            assert_eq!(pixel(&png, center, ratio), [210, 40, 50, 255]);
            assert_eq!(pixel(&png, [150., 60.], ratio), [255; 4]);
            pointer(&app, viewport, PlotPointerPhase::Move, center);
            world(&last_hit(&hits), [150., 60.]);
            if view.scale == 2. {
                assert!(
                    center[0] > 260.,
                    "valid transformed ink lies outside original plot's screen x extent"
                );
                pointer(&app, viewport, PlotPointerPhase::Move, [305., 100.]);
                assert!(
                    hits.borrow().last().unwrap().is_none(),
                    "cropped PNG ink cannot be picked beyond the image viewport"
                );
            }
            pointer(&app, viewport, PlotPointerPhase::Move, [150., 60.]);
            assert!(
                hits.borrow().last().unwrap().is_none(),
                "old untransformed source coordinate is not image ink"
            );
            pointer(&app, viewport, PlotPointerPhase::Move, center);
            pointer(&app, viewport, PlotPointerPhase::Down, center);
            pointer(&app, viewport, PlotPointerPhase::Up, center);
            assert_eq!(activations.borrow().len(), 1);
            // Both endpoints remain inside real source ink. Only the original
            // visual-space 3px gesture threshold decides whether to activate.
            pointer(&app, viewport, PlotPointerPhase::Down, center);
            let four = [center[0] + 4., center[1]];
            assert_eq!(pixel(&png, four, ratio), [210, 40, 50, 255]);
            pointer(&app, viewport, PlotPointerPhase::Move, four);
            pointer(&app, viewport, PlotPointerPhase::Up, four);
            assert_eq!(
                activations.borrow().len(),
                1,
                "4 visual pixels cannot turn into a small source-space click"
            );
            pointer(&app, viewport, PlotPointerPhase::Down, center);
            let two = [center[0] + 2., center[1]];
            pointer(&app, viewport, PlotPointerPhase::Move, two);
            pointer(&app, viewport, PlotPointerPhase::Up, two);
            assert_eq!(
                activations.borrow().len(),
                2,
                "2 visual pixels cannot turn into a large source-space drag"
            );
            app.clear().unwrap();
        }
    }
}

#[test]
fn view_only_change_repicks_stationary_visual_hover_replaces_token_and_cancels_old_down() {
    for ratio in [1., 2.] {
        let app = App::memory();
        let options = app.scope().cage(RasterOptions {
            pixel_ratio: ratio,
            view: large_view(),
            ..Default::default()
        });
        let hits = Rc::new(RefCell::new(vec![]));
        let selected = Rc::new(RefCell::new(vec![]));
        let (observed, chosen) = (hits.clone(), selected.clone());
        app.mount(
            chart_layout(source(None, Color(210, 40, 50, 255)))
                .highlight(false)
                .raster(FontRegistry::new(), options)
                .on_physical_hover(move |hit| observed.borrow_mut().push(hit))
                .on_select(move |m| chosen.borrow_mut().push(m)),
        )
        .unwrap();
        let viewport = first(&app, "kartis-raster-plot");
        pointer(&app, viewport, PlotPointerPhase::Move, [270., 100.]);
        let before = last_hit(&hits);
        let before_png = image(&app);
        pointer(&app, viewport, PlotPointerPhase::Down, [270., 100.]);
        let observations = hits.borrow().len();
        options.revise(|o| o.view = ZoomTransform::new(2., -28., -20.).unwrap());
        app.flush().unwrap();
        // No new Move was dispatched: the old accepted visual coordinate is
        // repicked into the new camera, and even the same owner has a new token.
        assert!(hits.borrow().len() > observations);
        let after = last_hit(&hits);
        world(&after, [149., 60.]);
        assert_ne!(before.frame_token, after.frame_token);
        assert_ne!(before_png, image(&app));
        assert_eq!(first(&app, "kartis-raster-plot"), viewport);
        pointer(&app, viewport, PlotPointerPhase::Up, [270., 100.]);
        assert!(
            selected.borrow().is_empty(),
            "view change cancels even an unmoved Down over the same owner"
        );
        pointer(&app, viewport, PlotPointerPhase::Down, [272., 100.]);
        pointer(&app, viewport, PlotPointerPhase::Up, [272., 100.]);
        assert_eq!(
            selected.borrow().len(),
            1,
            "fresh gestures still work after cancellation"
        );
        app.clear().unwrap();
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Payload {
    value: Option<f64>,
    note: String,
}
fn slots(value: Option<f64>, note: &str) -> PlotWidgetsData<Payload> {
    PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: Some(ClipRect {
                x: 40.,
                y: 25.,
                width: 220.,
                height: 100.,
            }),
            slots: vec![PlotWidgetSlot {
                mark_id: "owner".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-15., -20.],
                    measured: TextSize {
                        width: 10.,
                        height: 6.,
                    },
                },
            }],
        },
        data: BTreeMap::from([(
            "owner".into(),
            Payload {
                value,
                note: note.into(),
            },
        )]),
    }
}

#[test]
fn failed_png_publication_retains_nullable_typed_child_camera_token_and_active_click() {
    for ratio in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage((
            source(None, Color(210, 40, 50, 255)),
            slots(None, "accepted"),
            RasterOptions {
                pixel_ratio: ratio,
                view: large_view(),
                ..Default::default()
            },
        ));
        let layout = app.scope().bond(move || input.with(|i| i.0.clone()));
        let data = app.scope().bond(move || input.with(|i| i.1.clone()));
        let options = app.scope().bond(move || input.with(|i| i.2.clone()));
        let fail = Rc::new(Cell::new(false));
        let rejected = fail.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let seen_error = errors.clone();
        let hits = Rc::new(RefCell::new(vec![]));
        let seen_hit = hits.clone();
        let selections = Rc::new(RefCell::new(vec![]));
        let selected = selections.clone();
        let disposed = Rc::new(Cell::new(0));
        let cleanup = disposed.clone();
        app.mount(
            chart_layout(layout)
                .highlight(false)
                .raster(FontRegistry::new(), options)
                .raster_image_source(move |png| {
                    if rejected.get() {
                        Err(ChartError("audit PNG publisher refused".into()))
                    } else {
                        png.data_url()
                    }
                })
                .on_reject(move |e| seen_error.borrow_mut().push(e.0))
                .on_physical_hover(move |hit| seen_hit.borrow_mut().push(hit))
                .on_select(move |mark| selected.borrow_mut().push(mark))
                .plot_widgets_with_data(data, move |ctx| {
                    let frame = ctx.frame;
                    let cleanup = cleanup.clone();
                    Ok(glory::ui::widget(move |scope| {
                        let cleanup = cleanup.clone();
                        scope.on_cleanup(move || cleanup.set(cleanup.get() + 1))?;
                        let count = scope.cage(0usize);
                        glory::ui::button()
                            .class("raster-audit-child")
                            .label(scope.bond(move || {
                                frame.with(|f| {
                                    format!(
                                        "{:?}|{}|mark={}|x={}|count={}",
                                        f.data.value,
                                        f.data.note,
                                        f.frame.mark.value,
                                        f.frame.rect.x,
                                        count.get()
                                    )
                                })
                            }))
                            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                            .show_in(scope)
                    }))
                }),
        )
        .unwrap();
        let viewport = first(&app, "kartis-raster-plot");
        let child = first(&app, "raster-audit-child");
        let control = first(&app, "kartis-datum");
        app.dispatch(child, Event::Activate).unwrap();
        assert_eq!(label(&app, child), "None|accepted|mark=0|x=240|count=1");
        assert!(label(&app, control).contains("missing"));
        pointer(&app, viewport, PlotPointerPhase::Move, [270., 100.]);
        let old_hit = last_hit(&hits);
        pointer(&app, viewport, PlotPointerPhase::Down, [270., 100.]);
        let old_png = image(&app);
        let old_tree = format!("{:?}", app.tree());
        let old_hits = hits.borrow().clone();
        fail.set(true);
        input.revise(|i| {
            i.0 = source(Some(42.), Color(30, 90, 190, 255));
            i.1 = slots(Some(42.), "must not leak");
            i.2.view = small_view();
        });
        app.flush().unwrap();
        assert_eq!(errors.borrow().as_slice(), ["audit PNG publisher refused"]);
        assert_eq!(image(&app), old_png);
        assert_eq!(format!("{:?}", app.tree()), old_tree);
        assert_eq!(
            *hits.borrow(),
            old_hits,
            "rejected candidate cannot expose its new physical token"
        );
        assert_eq!(label(&app, child), "None|accepted|mark=0|x=240|count=1");
        // The failed camera proposal cannot cancel the live old-camera Down.
        pointer(&app, viewport, PlotPointerPhase::Up, [271., 100.]);
        assert_eq!(selections.borrow().len(), 1);
        assert_eq!(selections.borrow()[0].value, 0.);
        assert_eq!(last_hit(&hits).frame_token, old_hit.frame_token);
        world(&last_hit(&hits), [150.5, 60.]);
        fail.set(false);
        input.revise(|i| i.1.data.get_mut("owner").unwrap().note = "recovered".into());
        app.flush().unwrap();
        assert_eq!(first(&app, "raster-audit-child"), child);
        assert_eq!(first(&app, "kartis-datum"), control);
        assert_eq!(
            label(&app, child),
            "Some(42.0)|recovered|mark=42|x=97.5|count=1"
        );
        assert!(label(&app, control).contains("42"));
        assert_eq!(pixel(&image(&app), [105., 50.], ratio), [30, 90, 190, 255]);
        assert_eq!(pixel(&image(&app), [270., 100.], ratio), [255; 4]);
        pointer(&app, viewport, PlotPointerPhase::Move, [105., 50.]);
        let recovered = last_hit(&hits);
        world(&recovered, [150., 60.]);
        assert_ne!(recovered.frame_token, old_hit.frame_token);
        assert_eq!(disposed.get(), 0);
        app.clear().unwrap();
        assert_eq!(disposed.get(), 1);
        assert_eq!(app.stats().nodes, 0);
        assert_eq!(app.stats().handlers, 0);
    }
}
