//! Accepted typed slot data is exercised through real Glory children. These
//! are overlay slots; a rejected child revision retains its old child while
//! that child continues to observe the newly accepted frame.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::style::{Length, Style};
use glory::{App, Widget};
use kartis::*;

#[derive(Clone, Debug, PartialEq)]
struct Payload {
    interval: [f64; 2],
    source_index: Option<usize>,
    text: String,
}

fn packet() -> (Scene, PlotWidgetsData<Payload>) {
    let mut scene = ChartConfig {
        width: 240.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut slots = Vec::new();
    let mut data = BTreeMap::new();
    for (i, id) in ["alpha", "beta"].into_iter().enumerate() {
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x: 50. + i as f64 * 100.,
                y: 55.,
                radius: 10.,
            },
            Color(220, 30, 40, 255),
        );
        mark.series = id.into();
        mark.value = 10. + i as f64;
        mark.label = id.into();
        scene.marks.push(mark);
        slots.push(PlotWidgetSlot {
            mark_id: id.into(),
            revision: 0,
            placement: PlotWidgetPlacement::AtDatum {
                offset: [-20., -10.],
                measured: TextSize {
                    width: 40.,
                    height: 20.,
                },
            },
        });
        data.insert(
            id.into(),
            Payload {
                interval: [-(i as f64) - 3., 10. + i as f64],
                source_index: Some(i),
                text: format!("source {id}"),
            },
        );
    }
    (
        scene,
        PlotWidgetsData {
            layout: PlotWidgetsLayout { plot: None, slots },
            data,
        },
    )
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut result = Vec::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    result
}

fn first(app: &App, class: &str) -> NodeId {
    nodes(app, class)[0]
}

fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(props) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("expected real Glory button")
    };
    props.label.clone()
}

fn style(app: &App, id: NodeId) -> Style {
    app.tree().node(id).unwrap().descriptor.view().style.clone()
}

fn child(context: TypedPlotWidgetContext<Payload>, drops: Rc<Cell<usize>>) -> impl Widget {
    let frame = context.frame;
    let id = context.mark_id;
    let revision = context.revision;
    glory::ui::widget(move |scope| {
        let drops = drops.clone();
        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
        let count = scope.cage(0usize);
        // Memory receives real focus events. Retention is observable in the
        // child's own state; native/browser focus transport is tested elsewhere.
        let focused = scope.cage(false);
        glory::ui::button()
            .class(format!("typed-{id}"))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{} {:?} {:?} mark={} x={} rev={revision} count={} focus={}",
                        f.data.text,
                        f.data.interval,
                        f.data.source_index,
                        f.frame.mark.value,
                        f.frame.rect.x,
                        count.get(),
                        focused.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .on(EventKind::FocusChanged, move |event| {
                if let Event::FocusChanged(value) = event {
                    focused.revise(|old| *old = value);
                }
            })
            .show_in(scope)?;
        if revision == 3 {
            Err(glory::render::Error::InvalidProps("typed child build refused").into())
        } else {
            Ok(())
        }
    })
}

#[test]
fn typed_decorative_slots_do_not_create_source_data_and_plain_slots_keep_their_rule() {
    let (mut scene, data) = packet();
    scene.marks[0].label.clear();
    let app = App::memory();
    let observed = Rc::new(RefCell::new(None));
    let seen = observed.clone();
    app.mount(
        chart(scene.clone()).plot_widgets_with_data(data.clone(), move |ctx| {
            if ctx.mark_id == "alpha" {
                *seen.borrow_mut() = Some(ctx.frame.with(|f| f.frame.mark.clone()));
            }
            Ok(child(ctx, Rc::new(Cell::new(0))))
        }),
    )
    .unwrap();
    assert!(observed.borrow().as_ref().unwrap().label.is_empty());
    assert_eq!(nodes(&app, "typed-alpha").len(), 1);
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    app.dispatch(first(&app, "typed-alpha"), Event::Activate)
        .unwrap();
    assert!(label(&app, first(&app, "typed-alpha")).ends_with("count=1 focus=false"));
    app.clear().unwrap();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let plain = App::memory();
    assert!(
        plain
            .mount(chart(scene).plot_widgets(data.layout, move |_| {
                called.set(called.get() + 1);
                Ok(glory::ui::button().label("plain"))
            }))
            .is_err()
    );
    assert_eq!(calls.get(), 0);
    plain.clear().unwrap();
}

#[test]
fn metadata_only_and_complete_frames_keep_child_identity_and_local_state() {
    let app = App::memory();
    let input = app.scope().cage(packet());
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let data = app.scope().bond(move || input.with(|p| p.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let drops = Rc::new(Cell::new(0));
    let cleanup = drops.clone();
    app.mount(chart(scene).plot_widgets_with_data(data, move |context| {
        called.set(called.get() + 1);
        Ok(child(context, cleanup.clone()))
    }))
    .unwrap();
    let alpha = first(&app, "typed-alpha");
    let beta = first(&app, "typed-beta");
    let clips = nodes(&app, "kartis-plot-widget-clip");
    app.dispatch(alpha, Event::Activate).unwrap();
    app.dispatch(alpha, Event::FocusChanged(true)).unwrap();
    let original_scene = input.with(|p| p.0.clone());
    input.revise(|p| {
        let datum = p.1.data.get_mut("alpha").unwrap();
        datum.interval = [-99., 999.];
        datum.source_index = None;
        datum.text = "synthetic <range> 中文".into();
    });
    app.flush().unwrap();
    assert_eq!(input.with(|p| p.0.clone()), original_scene);
    assert_eq!(first(&app, "typed-alpha"), alpha);
    assert_eq!(
        label(&app, alpha),
        "synthetic <range> 中文 [-99.0, 999.0] None mark=10 x=30 rev=0 count=1 focus=true"
    );
    assert_eq!(calls.get(), 2);
    input.revise(|p| {
        p.0.width = 300.;
        p.0.marks[0].value = 37.;
        p.0.marks[0].geometry = Geometry::Circle {
            x: 80.,
            y: 62.,
            radius: 12.,
        };
        p.0.marks.reverse();
        p.1.layout.slots.reverse();
        p.1.data.get_mut("alpha").unwrap().text = "accepted update".into();
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-plot-widget-clip"), [clips[1], clips[0]]);
    assert_eq!(first(&app, "typed-alpha"), alpha);
    assert_eq!(first(&app, "typed-beta"), beta);
    assert!(label(&app, alpha).contains("accepted update [-99.0, 999.0] None mark=37 x=60"));
    assert!(label(&app, alpha).ends_with("count=1 focus=true"));
    assert_eq!(
        style(&app, first(&app, "kartis-plot")).width,
        Some(Length::Px(300.))
    );
    assert_eq!(calls.get(), 2);
    input.revise(|p| {
        p.0.marks.retain(|m| m.id != "alpha");
        p.1.layout.slots.retain(|s| s.mark_id != "alpha");
        p.1.data.remove("alpha");
    });
    app.flush().unwrap();
    assert!(nodes(&app, "typed-alpha").is_empty());
    assert_eq!(first(&app, "typed-beta"), beta);
    assert_eq!(drops.get(), 1);
    app.clear().unwrap();
    assert_eq!(drops.get(), 2);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[test]
fn invalid_candidate_keeps_accepted_scene_payload_and_children_but_revision_errors_are_local() {
    let app = App::memory();
    let input = app.scope().cage(packet());
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let data = app.scope().bond(move || input.with(|p| p.1.clone()));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let rejected = errors.clone();
    let drops = Rc::new(Cell::new(0));
    let cleanup = drops.clone();
    app.mount(
        chart(scene)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_with_data(data, move |context| {
                if context.revision == 1 {
                    return Err(ChartError("typed child revision refused".into()));
                }
                Ok(child(context, cleanup.clone()))
            }),
    )
    .unwrap();
    let accepted = input.get();
    let alpha = first(&app, "typed-alpha");
    let beta = first(&app, "typed-beta");
    app.dispatch(alpha, Event::Activate).unwrap();
    let old_tree = format!("{:?}", app.tree());
    for invalid in 0..4 {
        let mut proposed = accepted.clone();
        proposed.0.width = 320.;
        proposed.1.data.get_mut("alpha").unwrap().text = "must not leak".into();
        match invalid {
            0 => {
                proposed.1.data.remove("alpha");
            }
            1 => {
                proposed
                    .1
                    .data
                    .insert("unknown".into(), accepted.1.data["alpha"].clone());
            }
            2 => {
                proposed.1.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: -1.,
                    height: 10.,
                })
            }
            _ => proposed.0.marks[0].value = f64::NAN,
        }
        input.revise(|old| *old = proposed);
        app.flush().unwrap();
        assert_eq!(
            format!("{:?}", app.tree()),
            old_tree,
            "invalid cohort {invalid}"
        );
        assert_eq!(first(&app, "typed-alpha"), alpha);
        assert_eq!(first(&app, "typed-beta"), beta);
        assert!(!label(&app, alpha).contains("must not leak"));
    }
    assert_eq!(errors.borrow().len(), 4);
    input.revise(|old| *old = accepted.clone());
    app.flush().unwrap();
    input.revise(|p| {
        p.0.width = 300.;
        p.0.marks[0].value = 42.;
        p.1.data.get_mut("alpha").unwrap().text = "accepted despite child refusal".into();
        p.1.layout.slots[0].revision = 1;
    });
    app.flush().unwrap();
    assert_eq!(errors.borrow().len(), 5);
    assert!(
        errors
            .borrow()
            .last()
            .unwrap()
            .contains("typed child revision refused")
    );
    assert_eq!(first(&app, "typed-alpha"), alpha);
    assert_eq!(first(&app, "typed-beta"), beta);
    assert!(label(&app, alpha).contains("accepted despite child refusal"));
    assert!(label(&app, alpha).contains("mark=42"));
    assert!(label(&app, alpha).ends_with("rev=0 count=1 focus=false"));
    assert_eq!(
        style(&app, first(&app, "kartis-plot")).width,
        Some(Length::Px(300.))
    );
    assert_eq!(drops.get(), 0);
    input.revise(|p| {
        p.1.layout.slots[0].revision = 3;
        p.1.data.get_mut("alpha").unwrap().text = "accepted despite child build refusal".into();
    });
    app.flush().unwrap();
    assert_eq!(errors.borrow().len(), 6);
    assert!(
        errors
            .borrow()
            .last()
            .unwrap()
            .contains("typed child build refused")
    );
    assert_eq!(nodes(&app, "typed-alpha"), [alpha]);
    assert!(label(&app, alpha).contains("accepted despite child build refusal"));
    assert!(label(&app, alpha).ends_with("rev=0 count=1 focus=false"));
    // The partially mounted candidate was cleaned up; the old live child was not.
    assert_eq!(drops.get(), 1);
    // Returning to the prior accepted revision retains the child; a new
    // revision explicitly retries the factory and replaces only that child.
    input.revise(|p| p.1.layout.slots[0].revision = 0);
    app.flush().unwrap();
    assert_eq!(first(&app, "typed-alpha"), alpha);
    input.revise(|p| p.1.layout.slots[0].revision = 2);
    app.flush().unwrap();
    assert_ne!(first(&app, "typed-alpha"), alpha);
    assert_eq!(first(&app, "typed-beta"), beta);
    assert_eq!(drops.get(), 2);
}

#[cfg(feature = "raster")]
fn image_source(app: &App) -> String {
    let tree = app.tree();
    let Props::Image(props) = tree
        .node(first(app, "kartis-raster-image"))
        .unwrap()
        .descriptor
        .view()
        .props
    else {
        panic!("expected actual PNG image")
    };
    props.source.clone()
}

#[cfg(feature = "raster")]
fn png_witness(source: &str, x: usize, y: usize) -> ((u32, u32), [u8; 4]) {
    use base64::Engine;
    let encoded = source.strip_prefix("data:image/png;base64,").unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut pixels).unwrap();
    let offset = (y * info.width as usize + x) * 4;
    (
        (info.width, info.height),
        pixels[offset..offset + 4].try_into().unwrap(),
    )
}

#[cfg(feature = "raster")]
#[test]
fn raster_publisher_rejection_keeps_typed_frame_and_real_png_at_both_dprs() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage(packet());
        let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
        let data = app.scope().bond(move || input.with(|p| p.1.clone()));
        let rejecting = Rc::new(Cell::new(false));
        let reject = rejecting.clone();
        let errors = Rc::new(RefCell::new(Vec::new()));
        let captured = errors.clone();
        let drops = Rc::new(Cell::new(0));
        let cleanup = drops.clone();
        app.mount(
            chart(scene)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |image| {
                    if reject.get() {
                        return Err(ChartError("typed PNG publisher refused".into()));
                    }
                    image.data_url()
                })
                .on_reject(move |e| captured.borrow_mut().push(e.0))
                .plot_widgets_with_data(data, move |context| Ok(child(context, cleanup.clone()))),
        )
        .unwrap();
        let alpha = first(&app, "typed-alpha");
        app.dispatch(alpha, Event::Activate).unwrap();
        let old_image = image_source(&app);
        let old_tree = format!("{:?}", app.tree());
        assert!(old_image.starts_with("data:image/png;base64,"));
        assert_eq!(
            png_witness(&old_image, (50. * dpr) as usize, (55. * dpr) as usize),
            (
                ((240. * dpr) as u32, (150. * dpr) as u32),
                [220, 30, 40, 255]
            )
        );
        rejecting.set(true);
        input.revise(|p| {
            p.0.marks[0].geometry = Geometry::Circle {
                x: 80.,
                y: 62.,
                radius: 12.,
            };
            p.0.marks[0].fill = Some(Color(10, 170, 70, 255));
            p.0.marks[0].value = 42.;
            p.1.data.get_mut("alpha").unwrap().text = "typed PNG candidate".into();
        });
        app.flush().unwrap();
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(image_source(&app), old_image);
        assert_eq!(format!("{:?}", app.tree()), old_tree);
        assert_eq!(first(&app, "typed-alpha"), alpha);
        assert!(!label(&app, alpha).contains("typed PNG candidate"));
        rejecting.set(false);
        input.revise(|p| p.1.data.get_mut("alpha").unwrap().text = "typed PNG recovered".into());
        app.flush().unwrap();
        assert_ne!(image_source(&app), old_image);
        assert_eq!(
            png_witness(
                &image_source(&app),
                (80. * dpr) as usize,
                (62. * dpr) as usize
            )
            .1,
            [10, 170, 70, 255]
        );
        assert_eq!(first(&app, "typed-alpha"), alpha);
        assert!(label(&app, alpha).contains("typed PNG recovered"));
        assert!(label(&app, alpha).contains("mark=42 x=60"));
        assert!(label(&app, alpha).ends_with("count=1 focus=false"));
        app.clear().unwrap();
        assert_eq!(drops.get(), 2);
        assert_eq!(app.stats().nodes, 0);
        assert_eq!(app.stats().handlers, 0);
    }
}
