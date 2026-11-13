#![cfg(feature = "raster")]
use std::cell::{Cell, RefCell};
use std::io::Cursor;
use std::rc::Rc;

use base64::Engine;
use glory::render::{Event, EventKind, NodeId, Props};
use glory::style::{Display, Length, PointerEvents, Style};
use glory::{App, Widget};
use kartis::render::raster::*;
use kartis::*;

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (i, id) in ["alpha", "beta"].into_iter().enumerate() {
        let color = if i == 0 {
            Color(220, 30, 40, 255)
        } else {
            Color(20, 160, 70, 255)
        };
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x: 50. + i as f64 * 100.,
                y: 55.,
                radius: 10.,
            },
            color,
        );
        mark.series = id.into();
        mark.value = 10. + i as f64;
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color,
        });
    }
    scene
}
fn layout() -> PlotWidgetsLayout {
    PlotWidgetsLayout {
        plot: Some(ClipRect {
            x: 5.,
            y: 5.,
            width: 225.,
            height: 130.,
        }),
        slots: ["alpha", "beta"]
            .into_iter()
            .map(|id| PlotWidgetSlot {
                mark_id: id.into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-20., -10.],
                    measured: TextSize {
                        width: 40.,
                        height: 20.,
                    },
                },
            })
            .collect(),
    }
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut found = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        stack.extend(node.children.iter().rev().copied());
    }
    found
}
fn first(app: &App, class: &str) -> NodeId {
    nodes(app, class)[0]
}
fn style(app: &App, id: NodeId) -> Style {
    app.tree().node(id).unwrap().descriptor.view().style.clone()
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("expected real Glory button")
    };
    p.label.clone()
}
fn source(app: &App) -> String {
    let tree = app.tree();
    let Props::Image(p) = tree
        .node(first(app, "kartis-raster-image"))
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("expected CPU image")
    };
    p.source.clone()
}
fn decode(source: &str) -> (u32, u32, Vec<u8>) {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    pixels.truncate(info.buffer_size());
    (info.width, info.height, pixels)
}
fn pixel(source: &str, x: usize, y: usize) -> [u8; 4] {
    let (width, _, pixels) = decode(source);
    pixels[((y * width as usize + x) * 4)..][..4]
        .try_into()
        .unwrap()
}

#[test]
fn cpu_image_and_retained_widgets_follow_geometry_reorder_view_and_logical_dpr() {
    let app = App::memory();
    let scene = app.scope().cage(scene());
    let slots = app.scope().cage(layout());
    let options = app.scope().cage(RasterOptions::default());
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    app.mount(
        raster_chart(scene, FontRegistry::new(), options).plot_widgets(slots, move |ctx| {
            called.set(called.get() + 1);
            let id = ctx.mark_id;
            let frame = ctx.frame;
            Ok(glory::ui::widget(move |scope| {
                let count = scope.cage(0usize);
                glory::ui::button()
                    .class(format!("slot-{id}"))
                    .label(scope.bond(move || {
                        frame.with(|f| format!("{} count {}", f.mark.value, count.get()))
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                    .show_in(scope)
            }))
        }),
    )
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    let beta = first(&app, "slot-beta");
    let clips = nodes(&app, "kartis-plot-widget-clip");
    let image = first(&app, "kartis-raster-image");
    assert_eq!(pixel(&source(&app), 50, 55), [220, 30, 40, 255]);
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    assert_eq!(label(&app, alpha), "10 count 1");
    scene.revise(|s| {
        s.width = 300.;
        s.marks[0].geometry = Geometry::Circle {
            x: 80.,
            y: 62.,
            radius: 12.,
        };
        s.marks[0].value = 37.;
        s.marks.reverse();
    });
    slots.revise(|s| s.slots.reverse());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-plot-widget-clip"), [clips[1], clips[0]]);
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(first(&app, "slot-beta"), beta);
    assert_eq!(label(&app, alpha), "37 count 1");
    assert_eq!(style(&app, clips[0]).left, Some(Length::Px(60.)));
    assert_eq!(style(&app, clips[0]).top, Some(Length::Px(52.)));
    options.revise(|o| o.view = ZoomTransform::new(1.5, -10., -5.).unwrap());
    app.flush().unwrap();
    let projected = style(&app, clips[0]);
    assert_eq!(projected.left, Some(Length::Px(80.)));
    assert_eq!(projected.top, Some(Length::Px(73.)));
    assert_eq!(projected.width, Some(Length::Px(60.)));
    assert_eq!(projected.height, Some(Length::Px(30.)));
    assert_eq!(pixel(&source(&app), 110, 88), [220, 30, 40, 255]);
    let before_dpr = source(&app);
    options.revise(|o| o.pixel_ratio = 2.);
    app.flush().unwrap();
    assert_eq!(style(&app, clips[0]), projected);
    let dpr = source(&app);
    assert_ne!(dpr, before_dpr);
    assert_eq!((decode(&dpr).0, decode(&dpr).1), (600, 300));
    assert_eq!(pixel(&dpr, 220, 176), [220, 30, 40, 255]);
    assert_eq!(first(&app, "kartis-raster-image"), image);
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(label(&app, alpha), "37 count 1");
    assert_eq!(calls.get(), 2);
}

#[test]
fn rectangle_clips_are_exact_partial_widgets_inert_and_whole_masks_retain_children() {
    let app = App::memory();
    let mut initial = scene();
    initial.marks[0].clip = Some(ClipRect {
        x: 35.,
        y: 20.,
        width: 60.,
        height: 95.,
    });
    let scene = app.scope().cage(initial);
    let slots = app.scope().cage(layout());
    let presses = Rc::new(Cell::new(0));
    let selected = presses.clone();
    app.mount(
        raster_chart(scene, FontRegistry::new(), RasterOptions::default()).plot_widgets(
            slots,
            move |ctx| {
                let presses = selected.clone();
                Ok(glory::ui::button()
                    .class(format!("slot-{}", ctx.mark_id))
                    .label(ctx.mark_id)
                    .on(EventKind::Activate, move |_| presses.set(presses.get() + 1)))
            },
        ),
    )
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    let clip = first(&app, "kartis-plot-widget-clip");
    let content = first(&app, "kartis-plot-widget-slot");
    assert_eq!(style(&app, clip).left, Some(Length::Px(35.)));
    assert_eq!(style(&app, clip).width, Some(Length::Px(35.)));
    assert_eq!(style(&app, content).left, Some(Length::Px(-5.)));
    assert_eq!(
        style(&app, content).pointer_events,
        Some(PointerEvents::None)
    );
    assert!(app.scope().is_inert(alpha).unwrap());
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    scene.revise(|s| {
        s.marks[0].clip_path = Some(Box::new(ClipPath {
            geometry: Geometry::Rect {
                x: 40.,
                y: 48.,
                width: 20.,
                height: 10.,
            },
            rule: ClipRule::NonZero,
        }))
    });
    app.flush().unwrap();
    assert_eq!(style(&app, clip).left, Some(Length::Px(40.)));
    assert_eq!(style(&app, clip).top, Some(Length::Px(48.)));
    assert_eq!(style(&app, clip).width, Some(Length::Px(20.)));
    assert_eq!(style(&app, clip).height, Some(Length::Px(10.)));
    assert_eq!(style(&app, content).left, Some(Length::Px(-10.)));
    assert!(app.scope().is_inert(alpha).unwrap());
    assert_eq!(pixel(&source(&app), 50, 50), [220, 30, 40, 255]);
    assert_eq!(pixel(&source(&app), 50, 47), [255, 255, 255, 255]);
    slots.revise(|s| {
        s.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
            x: 45.,
            y: 49.,
            width: 10.,
            height: 5.,
        })
    });
    app.flush().unwrap();
    assert!(!app.scope().is_inert(alpha).unwrap());
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    scene.revise(|s| {
        s.marks[0].clip = None;
        s.marks[0].clip_path = Some(Box::new(ClipPath::circle(50., 55., 40.)));
    });
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert!(!app.scope().is_inert(alpha).unwrap());
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    scene.revise(|s| s.marks[0].clip_path = Some(Box::new(ClipPath::circle(15., 15., 5.))));
    app.flush().unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    assert!(!app.scope().is_active(alpha).unwrap());
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    assert_eq!(pixel(&source(&app), 50, 55), [255, 255, 255, 255]);
    scene.revise(|s| s.marks[0].clip_path = None);
    app.flush().unwrap();
    slots.revise(|s| {
        s.plot = Some(ClipRect {
            x: 50.,
            y: 45.,
            width: 180.,
            height: 50.,
        })
    });
    app.flush().unwrap();
    assert_eq!(style(&app, clip).left, Some(Length::Px(50.)));
    assert_eq!(style(&app, clip).width, Some(Length::Px(5.)));
    assert!(app.scope().is_inert(alpha).unwrap());
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    slots.revise(|s| s.plot = layout().plot);
    app.flush().unwrap();
    let legend = first(&app, "kartis-legend-item");
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    slots.revise(|s| {
        s.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
            x: 260.,
            y: 49.,
            width: 10.,
            height: 5.,
        })
    });
    app.flush().unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    assert_eq!(presses.get(), 3);
}

#[test]
fn scene_slots_options_and_publisher_rejection_rollback_then_retry_as_one_frame() {
    let app = App::memory();
    let scene = app.scope().cage(scene());
    let slots = app.scope().cage(layout());
    let options = app.scope().cage(RasterOptions::default());
    let reject = Rc::new(Cell::new(false));
    let flag = reject.clone();
    let publications = Rc::new(Cell::new(0));
    let published = publications.clone();
    let factories = Rc::new(Cell::new(0));
    let called = factories.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        raster_chart(scene, FontRegistry::new(), options)
            .raster_image_source(move |image| {
                published.set(published.get() + 1);
                if flag.get() {
                    Err(ChartError("source denied candidate".into()))
                } else {
                    image.data_url()
                }
            })
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets(slots, move |ctx| {
                called.set(called.get() + 1);
                let frame = ctx.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    glory::ui::button()
                        .class(format!("slot-{}", ctx.mark_id))
                        .label(scope.bond(move || {
                            frame.with(|f| format!("{} count {}", f.mark.value, count.get()))
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    app.dispatch(alpha, Event::Activate).unwrap();
    let clip = first(&app, "kartis-plot-widget-clip");
    let old_style = style(&app, clip);
    let old_image = source(&app);
    reject.set(true);
    scene.revise(|s| {
        s.width = 300.;
        s.marks[0].value = 37.;
        s.marks[0].geometry = Geometry::Circle {
            x: 80.,
            y: 62.,
            radius: 12.,
        };
    });
    slots.revise(|s| {
        s.slots[0].placement = PlotWidgetPlacement::AtDatum {
            offset: [-10., -8.],
            measured: TextSize {
                width: 30.,
                height: 16.,
            },
        }
    });
    options.revise(|o| {
        o.view = ZoomTransform::new(1.25, 4., 3.).unwrap();
        o.pixel_ratio = 2.;
    });
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(style(&app, clip), old_style);
    assert_eq!(source(&app), old_image);
    assert_eq!(label(&app, alpha), "10 count 1");
    assert_eq!(factories.get(), 2);
    assert!(
        errors
            .borrow()
            .iter()
            .any(|s| s.contains("source denied candidate"))
    );
    reject.set(false);
    slots.revise(|s| s.slots.reverse());
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(label(&app, alpha), "37 count 1");
    assert_eq!(style(&app, clip).left, Some(Length::Px(91.5)));
    assert_eq!(style(&app, clip).top, Some(Length::Px(70.5)));
    assert_eq!(style(&app, clip).width, Some(Length::Px(37.5)));
    let accepted = source(&app);
    assert_ne!(accepted, old_image);
    assert_eq!((decode(&accepted).0, decode(&accepted).1), (600, 300));
    let accepted_style = style(&app, clip);
    let before_invalid = publications.get();
    slots.revise(|s| s.slots.push(s.slots[0].clone()));
    options.revise(|o| o.pixel_ratio = f64::NAN);
    scene.revise(|s| s.marks[0].value = 99.);
    app.flush().unwrap();
    assert_eq!(publications.get(), before_invalid);
    assert_eq!(source(&app), accepted);
    assert_eq!(style(&app, clip), accepted_style);
    assert_eq!(label(&app, alpha), "37 count 1");
    slots.revise(|s| {
        s.slots.pop();
    });
    options.revise(|o| o.pixel_ratio = 2.);
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(label(&app, alpha), "99 count 1");
    assert_eq!(factories.get(), 2);
    let accepted = source(&app);
    let before_mask = publications.get();
    scene.revise(|s| s.marks[0].clip_path = Some(Box::new(ClipPath::circle(80., 62., 6.))));
    app.flush().unwrap();
    assert_eq!(source(&app), accepted);
    assert_eq!(publications.get(), before_mask);
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(style(&app, clip), accepted_style);
    assert!(
        errors
            .borrow()
            .iter()
            .any(|e| e.contains("partial plot Widget path masks"))
    );
    scene.revise(|s| s.marks[0].clip_path = Some(Box::new(ClipPath::circle(80., 62., 50.))));
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(factories.get(), 2);
}

#[test]
fn cleanup_retires_publisher_factory_child_state_and_pending_reactive_callbacks() {
    let app = App::memory();
    let scene = app.scope().cage(scene());
    let slots = app.scope().cage(layout());
    let options = app.scope().cage(RasterOptions::default());
    let factory_token = Rc::new(());
    let weak_factory = Rc::downgrade(&factory_token);
    let publisher_token = Rc::new(());
    let weak_publisher = Rc::downgrade(&publisher_token);
    let factories = Rc::new(Cell::new(0));
    let called = factories.clone();
    let publications = Rc::new(Cell::new(0));
    let published = publications.clone();
    let children = Rc::new(Cell::new(0));
    let cleaned = children.clone();
    app.mount(
        raster_chart(scene, FontRegistry::new(), options)
            .raster_image_source(move |image| {
                let _ = &publisher_token;
                published.set(published.get() + 1);
                image.data_url()
            })
            .plot_widgets(slots, move |ctx| {
                let _ = &factory_token;
                called.set(called.get() + 1);
                let cleaned = cleaned.clone();
                Ok(glory::ui::widget(move |scope| {
                    let cleaned = cleaned.clone();
                    scope.on_cleanup(move || cleaned.set(cleaned.get() + 1))?;
                    let count = scope.cage(0usize);
                    glory::ui::button()
                        .class(format!("slot-{}", ctx.mark_id))
                        .label(scope.bond(move || count.get().to_string()))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    app.dispatch(first(&app, "slot-alpha"), Event::Activate)
        .unwrap();
    assert_eq!(label(&app, first(&app, "slot-alpha")), "1");
    assert_eq!(factories.get(), 2);
    assert_eq!(publications.get(), 1);
    assert!(weak_factory.upgrade().is_some());
    assert!(weak_publisher.upgrade().is_some());
    app.clear().unwrap();
    assert_eq!(children.get(), 2);
    assert!(weak_factory.upgrade().is_none());
    assert!(weak_publisher.upgrade().is_none());
    scene.revise(|s| s.marks[0].value = 77.);
    slots.revise(|s| s.slots[0].revision += 1);
    options.revise(|o| o.pixel_ratio = 2.);
    app.flush().unwrap();
    assert_eq!(factories.get(), 2);
    assert_eq!(publications.get(), 1);
    assert_eq!(children.get(), 2);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
