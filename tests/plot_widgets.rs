use glory::render::{Event, EventKind, NodeId, Props};
use glory::style::{Display, Length, PointerEvents, Style};
use glory::{App, Widget};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

fn datum_scene() -> Scene {
    let mut s = ChartConfig {
        width: 240.,
        height: 150.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (index, id) in ["alpha", "beta"].into_iter().enumerate() {
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x: 50. + index as f64 * 100.,
                y: 55.,
                radius: 10.,
            },
            Color(40, 90, 180, 255),
        );
        mark.series = id.into();
        mark.value = 10. + index as f64;
        s.marks.push(mark);
        s.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color: Color(40, 90, 180, 255),
        });
    }
    s
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
    let mut result = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        stack.extend(node.children.iter().rev().copied());
    }
    result
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
        panic!("expected a Button")
    };
    p.label.clone()
}
fn is_inert(app: &App, id: NodeId) -> bool {
    app.scope().is_inert(id).unwrap()
}

#[test]
fn measured_widget_is_retained_on_live_geometry_value_and_reorder() {
    let app = App::memory();
    let scene = app.scope().cage(datum_scene());
    let slots = app.scope().cage(layout());
    let calls = Rc::new(Cell::new(0));
    let selected = Rc::new(Cell::new(0));
    let select = selected.clone();
    let called = calls.clone();
    app.mount(
        chart(scene)
            .on_select(move |_| select.set(select.get() + 1))
            .plot_widgets(slots, move |context| {
                called.set(called.get() + 1);
                let id = context.mark_id;
                let frame = context.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0usize);
                    glory::ui::button()
                        .class(format!("slot-{id}"))
                        .label(scope.bond(move || {
                            frame
                                .with(|datum| format!("{} count {}", datum.mark.value, count.get()))
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    assert_eq!(calls.get(), 2);
    let alpha = first(&app, "slot-alpha");
    let beta = first(&app, "slot-beta");
    let initial_clip = nodes(&app, "kartis-plot-widget-clip");
    assert_eq!(style(&app, initial_clip[0]).left, Some(Length::Px(30.)));
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    assert_eq!(label(&app, alpha), "10 count 1");
    assert_eq!(selected.get(), 0);
    scene.revise(|scene| {
        scene.width = 300.;
        scene.marks[0].geometry = Geometry::Circle {
            x: 80.,
            y: 62.,
            radius: 12.,
        };
        scene.marks[0].value = 37.;
        scene.marks.reverse();
    });
    slots.revise(|layout| layout.slots.reverse());
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(first(&app, "slot-beta"), beta);
    assert_eq!(
        nodes(&app, "kartis-plot-widget-clip"),
        [initial_clip[1], initial_clip[0]]
    );
    assert_eq!(style(&app, initial_clip[0]).left, Some(Length::Px(60.)));
    assert_eq!(style(&app, initial_clip[0]).top, Some(Length::Px(52.)));
    assert_eq!(label(&app, alpha), "37 count 1");
    assert_eq!(calls.get(), 2);
}

#[test]
fn partial_clip_is_visible_but_inert_and_hidden_retains_identity() {
    let app = App::memory();
    let mut initial = datum_scene();
    initial.marks[0].clip = Some(ClipRect {
        x: 35.,
        y: 20.,
        width: 60.,
        height: 95.,
    });
    let slots = app.scope().cage(layout());
    let presses = Rc::new(Cell::new(0));
    let counter = presses.clone();
    app.mount(chart(initial).plot_widgets(slots, move |ctx| {
        let presses = counter.clone();
        Ok(glory::ui::button()
            .class(format!("slot-{}", ctx.mark_id))
            .label(ctx.mark_id)
            .on(EventKind::Activate, move |_| presses.set(presses.get() + 1)))
    }))
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    let clip = first(&app, "kartis-plot-widget-clip");
    let wrapper = first(&app, "kartis-plot-widget-slot");
    assert_eq!(style(&app, clip).left, Some(Length::Px(35.)));
    assert_eq!(style(&app, clip).width, Some(Length::Px(35.)));
    assert_eq!(style(&app, wrapper).left, Some(Length::Px(-5.)));
    assert_eq!(
        style(&app, wrapper).pointer_events,
        Some(PointerEvents::None)
    );
    assert!(is_inert(&app, alpha));
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    slots.revise(|layout| {
        layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
            x: 45.,
            y: 45.,
            width: 40.,
            height: 20.,
        })
    });
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert!(!is_inert(&app, alpha));
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    let legend = first(&app, "kartis-legend-item");
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(style(&app, clip).display, Some(Display::None));
    assert!(!app.scope().is_active(alpha).unwrap());
    assert!(!app.dispatch(alpha, Event::Activate).unwrap());
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert!(app.dispatch(alpha, Event::Activate).unwrap());
    assert_eq!(presses.get(), 2);
}

#[test]
fn failed_factory_and_child_build_keep_preceding_widget_and_siblings() {
    let app = App::memory();
    let scene = app.scope().cage(datum_scene());
    let slots = app.scope().cage(layout());
    let errors = Rc::new(RefCell::new(Vec::new()));
    let capture = errors.clone();
    let drops = Rc::new(Cell::new(0));
    let drop_probe = drops.clone();
    app.mount(
        chart(scene)
            .on_reject(move |error| capture.borrow_mut().push(error.0))
            .plot_widgets(slots, move |ctx| {
                if ctx.revision == 1 {
                    return Err(ChartError("factory refused".into()));
                }
                let drops = drop_probe.clone();
                let id = ctx.mark_id;
                let revision = ctx.revision;
                Ok(glory::ui::widget(move |scope| {
                    let drops = drops.clone();
                    scope.on_cleanup(move || drops.set(drops.get() + 1))?;
                    glory::ui::button()
                        .class(format!("slot-{id}"))
                        .label(format!("{id} revision {revision}"))
                        .show_in(scope)?;
                    if revision == 2 {
                        Err(
                            glory::render::Error::InvalidProps("fixture child build refused")
                                .into(),
                        )
                    } else {
                        Ok(())
                    }
                }))
            }),
    )
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    let beta = first(&app, "slot-beta");
    for revision in [1, 2] {
        slots.revise(|layout| layout.slots[0].revision = revision);
        app.flush().unwrap();
        assert_eq!(nodes(&app, "slot-alpha"), [alpha]);
        assert_eq!(nodes(&app, "slot-beta"), [beta]);
    }
    assert_eq!(drops.get(), 1);
    assert!(errors.borrow()[0].contains("factory refused"));
    assert!(errors.borrow()[1].contains("fixture child build refused"));
    slots.revise(|layout| layout.slots[0].revision = 0);
    app.flush().unwrap();
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert_eq!(first(&app, "slot-beta"), beta);
    assert_eq!(drops.get(), 1);
    slots.revise(|layout| layout.slots[0].revision = 3);
    app.flush().unwrap();
    let replacement = first(&app, "slot-alpha");
    assert_ne!(replacement, alpha);
    assert_eq!(label(&app, replacement), "alpha revision 3");
    assert_eq!(first(&app, "slot-beta"), beta);
    assert_eq!(drops.get(), 2);
    scene.revise(|scene| {
        scene.marks.remove(0);
    });
    slots.revise(|layout| {
        layout.slots.remove(0);
    });
    app.flush().unwrap();
    assert!(nodes(&app, "slot-alpha").is_empty());
    assert_eq!(first(&app, "slot-beta"), beta);
    assert_eq!(drops.get(), 3);
    app.clear().unwrap();
    assert_eq!(drops.get(), 4);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[test]
fn invalid_layout_rejects_the_entire_scene_update_and_admission() {
    let app = App::memory();
    let scene = app.scope().cage(datum_scene());
    let slots = app.scope().cage(layout());
    let errors = Rc::new(RefCell::new(Vec::new()));
    let capture = errors.clone();
    app.mount(
        chart(scene)
            .on_reject(move |error| capture.borrow_mut().push(error.0))
            .plot_widgets(slots, move |ctx| {
                Ok(glory::ui::button()
                    .class(format!("slot-{}", ctx.mark_id))
                    .label(ctx.mark_id))
            }),
    )
    .unwrap();
    let alpha = first(&app, "slot-alpha");
    let plot = first(&app, "kartis-plot");
    slots.revise(|layout| layout.slots.push(layout.slots[0].clone()));
    scene.revise(|scene| scene.width = 300.);
    app.flush().unwrap();
    assert_eq!(style(&app, plot).width, Some(Length::Px(240.)));
    assert_eq!(first(&app, "slot-alpha"), alpha);
    assert!(
        errors
            .borrow()
            .last()
            .unwrap()
            .contains("duplicate plot Widget datum")
    );
    slots.revise(|layout| {
        layout.slots.pop();
    });
    app.flush().unwrap();
    assert_eq!(style(&app, plot).width, Some(Length::Px(300.)));
    assert_eq!(first(&app, "slot-alpha"), alpha);

    let reject = |scene: Scene, layout: PlotWidgetsLayout, plot: bool| {
        let app = App::memory();
        let calls = Rc::new(Cell::new(0));
        let captured = calls.clone();
        assert!(
            app.mount(chart(scene).plot(plot).plot_widgets(layout, move |_| {
                captured.set(captured.get() + 1);
                Ok(glory::ui::button().label("unexpected"))
            }))
            .is_err()
        );
        assert_eq!(calls.get(), 0);
        assert_eq!(app.stats().nodes, 0);
    };
    reject(datum_scene(), layout(), false);
    let mut invalid = layout();
    invalid.slots[0].mark_id = "unknown".into();
    reject(datum_scene(), invalid, true);
    let mut invalid = layout();
    invalid
        .slots
        .resize(MAX_PLOT_WIDGET_SLOTS + 1, invalid.slots[0].clone());
    reject(datum_scene(), invalid, true);
    let mut masked = datum_scene();
    masked.marks[0].clip_path = Some(Box::new(ClipPath::circle(50., 55., 10.)));
    reject(masked, layout(), true);
}

#[cfg(feature = "raster")]
#[test]
fn raster_chart_rejects_partial_path_masks_before_factory_or_image_publication() {
    let app = App::memory();
    let mut scene = datum_scene();
    scene.marks[0].clip_path = Some(Box::new(ClipPath::circle(50., 55., 10.)));
    let result = app.mount(
        chart(scene)
            .raster(
                kartis::render::raster::FontRegistry::new(),
                kartis::render::raster::RasterOptions::default(),
            )
            .raster_image_source(|_| panic!("raster publisher ran for a rejected plot"))
            .plot_widgets(layout(), |_| -> kartis::Result<glory::ui::Button> {
                panic!("Widget factory ran for a rejected plot")
            }),
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("partial plot Widget path masks")
    );
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
