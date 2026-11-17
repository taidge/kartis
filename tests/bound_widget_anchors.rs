//! Bound Chart Widgets use the source anchor, independently of displaced ink.
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::Rc;

use glory::render::{Event, NodeId, PathCommand, Props};
use glory::style::Length;
use glory::{App, Bond, Widget};
use kartis::*;

fn source(x: f64) -> SceneInteraction {
    let mut scene = ChartConfig {
        width: 240.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "offset",
        Geometry::Path(vec![
            // A separate Move is an explicit support point, with no paint ink.
            PathCommand::Move(glory::render::GraphicPoint::new(x, 45.)),
            PathCommand::Move(glory::render::GraphicPoint::new(x + 65., 70.)),
            PathCommand::Line(glory::render::GraphicPoint::new(x + 85., 70.)),
            PathCommand::Line(glory::render::GraphicPoint::new(x + 85., 90.)),
            PathCommand::Close,
        ]),
        Color(30, 80, 190, 255),
    );
    mark.series = "source".into();
    mark.label = "Offset paint".into();
    mark.value = 13.;
    let anchor = InteractionAnchor::path_points(&mark.geometry, [x, 45.], &[0]).unwrap();
    scene.marks.push(mark);
    SceneInteraction {
        scene,
        plot: None,
        interaction: Some(InteractionBinding {
            sites: vec![InteractionSite {
                mark_id: "offset".into(),
                series: "source".into(),
                label: "Offset paint".into(),
                value: None,
                formatted_value: None,
                color: Color(30, 80, 190, 255),
                position: [x, 45.],
            }],
            anchors: BTreeMap::from([("offset".into(), anchor)]),
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        }),
    }
}
fn packet() -> PlotWidgetsData<String> {
    PlotWidgetsData {
        layout: PlotWidgetsLayout {
            plot: None,
            slots: vec![PlotWidgetSlot {
                mark_id: "offset".into(),
                revision: 0,
                placement: PlotWidgetPlacement::AtDatum {
                    offset: [-8., -6.],
                    measured: TextSize {
                        width: 16.,
                        height: 12.,
                    },
                },
            }],
        },
        data: BTreeMap::from([("offset".into(), "original".into())]),
    }
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = queue.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        queue.extend(&node.children);
    }
    result
}
fn button(frame: Bond<TypedPlotWidgetFrame<String>>, drops: Rc<Cell<usize>>) -> impl Widget {
    glory::ui::widget(move |scope| {
        let drops = drops.clone();
        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
        let count = scope.cage(0u32);
        glory::ui::button()
            .class("anchored-child")
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{} live={} x={} count={}",
                        f.data,
                        f.frame.mark.value,
                        f.frame.rect.x,
                        count.get()
                    )
                })
            }))
            .on(glory::render::EventKind::Activate, move |_| {
                count.revise(|v| *v += 1)
            })
            .show_in(scope)
    })
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real button expected")
    };
    p.label.clone()
}

#[test]
fn typed_path_anchor_positions_overlay_and_strict_children_and_updates_atomically() {
    for strict in [false, true] {
        let app = App::memory();
        let input = app.scope().cage(source(40.));
        let drops = Rc::new(Cell::new(0));
        let retired = drops.clone();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let chart = chart_layout(input).legend(false).tooltip(false);
        if strict {
            app.mount(chart.plot_widgets_composed(
                PlotWidgetsComposition {
                    widgets: packet(),
                    replace_ink: ["offset".into()].into(),
                },
                move |context| {
                    called.set(called.get() + 1);
                    Ok(button(context.frame, retired.clone()))
                },
            ))
            .unwrap();
        } else {
            app.mount(chart.plot_widgets_with_data(packet(), move |context| {
                called.set(called.get() + 1);
                Ok(button(context.frame, retired.clone()))
            }))
            .unwrap();
        }
        let child = nodes(&app, "anchored-child")[0];
        let clip = nodes(&app, "kartis-plot-widget-clip")[0];
        let tree = app.tree();
        let style = &tree.node(clip).unwrap().descriptor.view().style;
        assert_eq!(style.left, Some(Length::Px(32.)));
        assert_eq!(style.top, Some(Length::Px(39.)));
        drop(tree);
        assert!(
            label(&app, child).contains("live=13 x=32"),
            "anchor binding must not overwrite current paint scalars"
        );
        app.dispatch(child, Event::Activate).unwrap();
        input.revise(|frame| *frame = source(60.));
        app.flush().unwrap();
        assert_eq!(nodes(&app, "anchored-child"), [child]);
        assert!(label(&app, child).contains("x=52 count=1"));
        assert_eq!(calls.get(), 1);
        app.clear().unwrap();
        assert_eq!(drops.get(), 1);
    }
}

#[test]
fn scene_only_path_has_no_invented_anchor_and_stale_bound_anchor_preserves_live_child() {
    let app = App::memory();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    assert!(
        app.mount(
            chart(source(40.).scene).plot_widgets_with_data(packet(), move |context| {
                called.set(called.get() + 1);
                Ok(button(context.frame, Rc::new(Cell::new(0))))
            })
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(nodes(&app, "anchored-child").is_empty());
    let app = App::memory();
    let input = app.scope().cage(source(40.));
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(input)
            .plot_widgets_with_data(packet(), |context| {
                Ok(button(context.frame, Rc::new(Cell::new(0))))
            })
            .on_reject(move |e| rejected.borrow_mut().push(e.0)),
    )
    .unwrap();
    let child = nodes(&app, "anchored-child")[0];
    app.dispatch(child, Event::Activate).unwrap();
    let before = label(&app, child);
    input.revise(|frame| {
        frame.scene.marks[0].geometry = Geometry::Circle {
            x: 110.,
            y: 80.,
            radius: 10.,
        }
    });
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_eq!(nodes(&app, "anchored-child"), [child]);
    assert_eq!(label(&app, child), before);
    app.clear().unwrap();
}
