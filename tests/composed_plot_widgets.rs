use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

#[derive(Clone, PartialEq)]
struct Data {
    text: String,
}
fn frame(value: f64, revision: u64) -> (Scene, PlotWidgetsComposition<Data>) {
    let mut scene = ChartConfig {
        width: 240.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (id, x, color) in [
        ("alpha", 50., Color(220, 30, 40, 255)),
        ("beta", 150., Color(20, 160, 70, 255)),
    ] {
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x,
                y: 55.,
                radius: 10.,
            },
            color,
        );
        mark.series = id.into();
        mark.value = value;
        scene.marks.push(mark);
    }
    let widgets = PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: ["alpha", "beta"]
                .into_iter()
                .map(|id| PlotWidgetSlot {
                    mark_id: id.into(),
                    revision,
                    placement: PlotWidgetPlacement::AtDatum {
                        offset: [-20., -10.],
                        measured: TextSize {
                            width: 40.,
                            height: 20.,
                        },
                    },
                })
                .collect(),
        },
        data: ["alpha", "beta"]
            .into_iter()
            .map(|id| {
                (
                    id.into(),
                    Data {
                        text: format!("{id}:{value}"),
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    };
    (
        scene,
        PlotWidgetsComposition {
            widgets,
            replace_ink: BTreeSet::from(["alpha".into()]),
        },
    )
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    out
}
fn first(app: &App, class: &str) -> NodeId {
    nodes(app, class)[0]
}
fn live_tree(app: &App) -> String {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut rows = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        rows.push(format!("{id:?}: {node:?}"));
        pending.extend(node.children.iter().rev().copied());
    }
    format!("{:?} {rows:?}", tree.roots())
}
fn label(app: &App, id: NodeId) -> String {
    match app.tree().node(id).unwrap().descriptor.view().props {
        Props::Button(p) => p.label.clone(),
        _ => panic!("button expected"),
    }
}
fn factory(ctx: PlotWidgetCompositionContext<Data>, drops: Rc<Cell<usize>>) -> Result<impl Widget> {
    if ctx.revision == 1 {
        return Err(ChartError("strict factory refused".into()));
    }
    let frame = ctx.frame;
    let id = ctx.mark_id;
    let revision = ctx.revision;
    Ok(glory::ui::widget(move |scope| {
        let drops = drops.clone();
        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
        let count = scope.cage(0usize);
        glory::ui::button()
            .class(format!("composed-{id}"))
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{} {} clicks={}",
                        f.data.text,
                        f.frame.mark.value,
                        count.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .show_in(scope)?;
        if revision == 2 {
            return Err(glory::render::Error::InvalidProps("strict child build refused").into());
        }
        Ok(())
    }))
}

#[test]
fn semantic_replacement_keeps_original_data_and_retained_widget_state() {
    let app = App::memory();
    let mut initial = frame(10., 0);
    initial.0.labels.push(Label {
        x: 20.,
        y: 125.,
        text: "Original caption".into(),
        size: 12.,
        color: Color(0, 0, 0, 255),
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    initial.0.label_series.insert(0, "alpha".into());
    let input = app.scope().cage(initial);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    let drops = Rc::new(Cell::new(0));
    let clean = drops.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .plot_widgets_composed(slots, move |ctx| factory(ctx, clean.clone())),
    )
    .unwrap();
    assert_eq!(nodes(&app, "kartis-mark").len(), 1);
    assert!(nodes(&app,"kartis-plot").iter().any(|_| {
        let tree=app.tree();let mut pending=tree.roots().to_vec();while let Some(id)=pending.pop(){let node=tree.node(id).unwrap();if matches!(node.descriptor.view().props,Props::Text(text) if text.content=="Original caption"){return true;}pending.extend(node.children.iter().copied());}false
    }));
    let alpha = first(&app, "composed-alpha");
    let beta = first(&app, "composed-beta");
    app.dispatch(alpha, Event::Activate).unwrap();
    assert!(label(&app, alpha).contains("clicks=1"));
    input.revise(|p| p.1.widgets.data.get_mut("alpha").unwrap().text = "metadata only".into());
    app.flush().unwrap();
    assert_eq!(first(&app, "composed-alpha"), alpha);
    assert!(label(&app, alpha).contains("metadata only 10 clicks=1"));
    input.revise(|p| {
        p.0.marks[0].value = 42.;
        p.1.widgets.data.get_mut("alpha").unwrap().text = "next".into();
    });
    app.flush().unwrap();
    assert_eq!(first(&app, "composed-beta"), beta);
    assert!(label(&app, alpha).contains("next 42 clicks=1"));
    assert_eq!(input.get().0.marks[0].fill, Some(Color(220, 30, 40, 255)));
    assert_eq!(input.get().0.marks[0].opacity, 1.);
    input.revise(|p| p.1.replace_ink.clear());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-mark").len(), 2);
    assert_eq!(first(&app, "composed-alpha"), alpha);
    input.revise(|p| p.1.replace_ink.insert("alpha".into()));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-mark").len(), 1);
    app.clear().unwrap();
    assert_eq!(drops.get(), 2);
    assert_eq!(app.stats().nodes, 0);
}

#[test]
fn semantic_finite_pointer_ignores_replaced_source_ink_and_mode_only_restores_it() {
    use glory::render::{PlotPointer, PlotPointerDevice, PlotPointerPhase, Rect};
    let app = App::memory();
    let (scene, composition) = frame(10., 0);
    let binding = InteractionBinding {
        sites: scene
            .marks
            .iter()
            .map(|mark| InteractionSite {
                mark_id: mark.id.clone(),
                series: mark.series.clone(),
                label: mark.label.clone(),
                value: Some(mark.value),
                formatted_value: None,
                color: mark.fill.unwrap(),
                position: if mark.id == "alpha" {
                    [50., 55.]
                } else {
                    [150., 55.]
                },
            })
            .collect(),
        picking_profile: PickingProfile::PaintedGeometry,
        ..Default::default()
    };
    let input = app.scope().cage((
        SceneInteraction {
            scene,
            interaction: Some(binding),
            plot: Some(ClipRect {
                x: 0.,
                y: 0.,
                width: 240.,
                height: 150.,
            }),
        },
        composition,
    ));
    let source = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    let hovered = Rc::new(RefCell::new(vec![]));
    let seen = hovered.clone();
    app.mount(
        chart_layout(source)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_hover(move |mark| seen.borrow_mut().push(mark.map(|m| m.id)))
            .plot_widgets_composed(slots, |ctx| factory(ctx, Rc::new(Cell::new(0)))),
    )
    .unwrap();
    let pointer = |x| {
        let plot = first(&app, "kartis-plot");
        app.dispatch(
            plot,
            Event::PlotPointer {
                from: plot,
                pointer: PlotPointer {
                    pointer_id: 1,
                    device: PlotPointerDevice::Pen,
                    phase: PlotPointerPhase::Move,
                    x,
                    y: 55.,
                    viewport: Rect {
                        width: 240.,
                        height: 150.,
                        ..Default::default()
                    },
                },
            },
        )
        .unwrap();
        app.flush().unwrap();
    };
    pointer(150.);
    assert_eq!(hovered.borrow().last().unwrap().as_deref(), Some("beta"));
    pointer(50.);
    assert_eq!(hovered.borrow().last(), Some(&None));
    input.revise(|p| p.1.replace_ink.clear());
    app.flush().unwrap();
    pointer(50.);
    assert_eq!(hovered.borrow().last().unwrap().as_deref(), Some("alpha"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[test]
fn factory_and_build_failure_reject_entire_frame_including_retained_data() {
    let app = App::memory();
    let input = app.scope().cage(frame(10., 0));
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    let drops = Rc::new(Cell::new(0));
    let clean = drops.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let seen = errors.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| seen.borrow_mut().push(e.0))
            .plot_widgets_composed(slots, move |ctx| factory(ctx, clean.clone())),
    )
    .unwrap();
    let alpha = first(&app, "composed-alpha");
    let accepted = live_tree(&app);
    let accepted_nodes = app.stats().nodes;
    for revision in [1, 2] {
        input.revise(|p| *p = frame(99., revision));
        app.flush().unwrap();
        assert_eq!(first(&app, "composed-alpha"), alpha);
        assert_eq!(label(&app, alpha), "alpha:10 10 clicks=0");
        assert_eq!(live_tree(&app), accepted);
        assert_eq!(app.stats().nodes, accepted_nodes);
    }
    assert_eq!(errors.borrow().len(), 2);
    assert_eq!(drops.get(), 1);
    input.revise(|p| *p = frame(25., 3));
    app.flush().unwrap();
    assert_ne!(first(&app, "composed-alpha"), alpha);
    assert!(label(&app, first(&app, "composed-alpha")).contains("alpha:25 25"));
    app.clear().unwrap();
    assert_eq!(drops.get(), 5);
}

#[test]
fn zero_source_group_hides_and_inerts_widget_without_losing_its_state() {
    let app = App::memory();
    let mut initial = frame(10., 0);
    initial.0.opacity_groups.push(MarkOpacityGroup {
        id: "group".into(),
        members: vec!["alpha".into()],
        opacity: 0.,
    });
    let input = app.scope().cage(initial);
    let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(
        chart(scene)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .plot_widgets_composed(slots, |ctx| factory(ctx, Rc::new(Cell::new(0)))),
    )
    .unwrap();
    let alpha = first(&app, "composed-alpha");
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    assert!(label(&app, alpha).contains("clicks=0"));
    input.revise(|p| p.0.opacity_groups[0].opacity = 0.5);
    app.flush().unwrap();
    assert_eq!(first(&app, "composed-alpha"), alpha);
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    assert!(label(&app, alpha).contains("clicks=1"));
    input.revise(|p| p.0.opacity_groups[0].opacity = 0.);
    app.flush().unwrap();
    assert_eq!(first(&app, "composed-alpha"), alpha);
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    assert!(label(&app, alpha).contains("clicks=1"));
    assert_eq!(input.get().0.marks[0].opacity, 1.);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
fn image_source(app: &App) -> String {
    match app
        .tree()
        .node(first(app, "kartis-raster-image"))
        .unwrap()
        .descriptor
        .view()
        .props
    {
        Props::Image(p) => p.source.clone(),
        _ => panic!("image expected"),
    }
}
#[cfg(feature = "raster")]
fn pixel(source: &str, x: usize, y: usize) -> [u8; 4] {
    use base64::Engine;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(source.strip_prefix("data:image/png;base64,").unwrap())
        .unwrap();
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
        .read_info()
        .unwrap();
    let mut rgba = vec![0; reader.output_buffer_size().unwrap()];
    let info = reader.next_frame(&mut rgba).unwrap();
    rgba[(y * info.width as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}

#[cfg(feature = "raster")]
#[test]
fn png_publication_gates_new_children_source_data_and_mode_changes() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage(frame(10., 0));
        let scene = app.scope().bond(move || input.with(|p| p.0.clone()));
        let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
        let rejecting = Rc::new(Cell::new(false));
        let reject = rejecting.clone();
        let calls = Rc::new(Cell::new(0));
        let publications = calls.clone();
        let drops = Rc::new(Cell::new(0));
        let clean = drops.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let seen = errors.clone();
        app.mount(
            raster_chart(
                scene,
                FontRegistry::new(),
                RasterOptions {
                    pixel_ratio: dpr,
                    ..Default::default()
                },
            )
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .raster_image_source(move |image| {
                publications.set(publications.get() + 1);
                if reject.get() {
                    Err(ChartError("strict PNG publication refused".into()))
                } else {
                    image.data_url()
                }
            })
            .on_reject(move |e| seen.borrow_mut().push(e.0))
            .plot_widgets_composed(slots, move |ctx| factory(ctx, clean.clone())),
        )
        .unwrap();
        let alpha = first(&app, "composed-alpha");
        let original = image_source(&app);
        assert_eq!(
            pixel(&original, (50. * dpr) as usize, (55. * dpr) as usize),
            [255, 255, 255, 255]
        );
        assert_eq!(
            pixel(&original, (150. * dpr) as usize, (55. * dpr) as usize),
            [20, 160, 70, 255]
        );
        let tree = live_tree(&app);
        let accepted_nodes = app.stats().nodes;
        let before = calls.get();
        rejecting.set(true);
        input.revise(|p| *p = frame(90., 3));
        app.flush().unwrap();
        assert_eq!(errors.borrow().len(), 1);
        assert_eq!(calls.get(), before + 1);
        assert_eq!(image_source(&app), original);
        assert_eq!(live_tree(&app), tree);
        assert_eq!(app.stats().nodes, accepted_nodes);
        assert_eq!(first(&app, "composed-alpha"), alpha);
        assert!(label(&app, alpha).contains("alpha:10 10"));
        assert_eq!(drops.get(), 2);
        rejecting.set(false);
        input.revise(|p| p.1.widgets.data.get_mut("alpha").unwrap().text = "retry".into());
        app.flush().unwrap();
        assert_ne!(first(&app, "composed-alpha"), alpha);
        assert!(label(&app, first(&app, "composed-alpha")).contains("retry 90"));
        input.revise(|p| p.1.replace_ink.clear());
        app.flush().unwrap();
        assert_eq!(
            pixel(
                &image_source(&app),
                (50. * dpr) as usize,
                (55. * dpr) as usize
            ),
            [220, 30, 40, 255]
        );
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}

#[cfg(feature = "raster")]
#[test]
fn initial_child_failure_precedes_any_png_publisher_call() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    let app = App::memory();
    let input = frame(10., 1);
    let calls = Rc::new(Cell::new(0));
    let seen = calls.clone();
    assert!(
        app.mount(
            raster_chart(input.0, FontRegistry::new(), RasterOptions::default())
                .raster_image_source(move |image| {
                    seen.set(seen.get() + 1);
                    image.data_url()
                })
                .plot_widgets_composed(input.1, |ctx| factory(ctx, Rc::new(Cell::new(0))))
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert_eq!(app.stats().nodes, 0);
}
