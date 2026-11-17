//! Original interval endpoints remain observable in retained Bullet Widgets.
//! Memory events below are explicit dispatch, not browser/OS input evidence.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

#[derive(Clone, Copy, Debug)]
enum Case {
    Range,
    Measure,
    SyntheticRange,
    Marker,
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 280.,
        height: 180.,
        ..Default::default()
    }
}
fn layout(case: Case, negative: bool) -> BulletLayout {
    let zero = if negative { -0. } else { 0. };
    let (ranges, measures, markers) = match case {
        Case::Range => (vec![zero, 3.], vec![], vec![]),
        Case::Measure => (vec![], vec![zero, 3.], vec![]),
        Case::SyntheticRange => (vec![zero], vec![], vec![]),
        Case::Marker => (vec![zero, 3.], vec![], vec![4.]),
    };
    let mut options = BulletLayoutOptions {
        min: Some(0.),
        max: Some(5.),
        legend: false,
        ..Default::default()
    };
    options.axis.visible = false;
    options.axis.grid = false;
    options.titles.insert("row".into(), String::new());
    bullet_layout(
        &config(),
        &[BulletDatum {
            id: "row".into(),
            ranges,
            measures,
            markers,
        }],
        &options,
    )
    .unwrap()
}
fn selected(case: Case, part: &BulletPart) -> bool {
    match case {
        Case::Range => part.kind == BulletPartKind::Range && part.source_index == Some(1),
        Case::Measure => part.kind == BulletPartKind::Measure && part.source_index == Some(1),
        Case::SyntheticRange => part.kind == BulletPartKind::Range && part.source_index.is_none(),
        Case::Marker => part.kind == BulletPartKind::Marker,
    }
}
fn packet(case: Case, l: &BulletLayout) -> PlotWidgetsData<BulletPartWidgetData> {
    bullet_part_widgets(l, 4, |c| {
        Ok(
            selected(case, c.part).then_some(PlotWidgetPlacement::Rect(LayoutRect {
                x: 80.,
                y: 50.,
                width: 40.,
                height: 20.,
            })),
        )
    })
    .unwrap()
}
fn source(case: Case, negative: bool) -> (SceneInteraction, PlotWidgetsData<BulletPartWidgetData>) {
    let l = layout(case, negative);
    let packet = packet(case, &l);
    let frame = SceneInteraction {
        scene: l.scene,
        interaction: Some(InteractionBinding {
            picking_profile: PickingProfile::PaintedGeometry,
            ..Default::default()
        }),
        plot: Some(ClipRect::plot(&config())),
    };
    frame.validate().unwrap();
    (frame, packet)
}
fn children(app: &App) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = queue.pop() {
        let node = tree.node(id).unwrap();
        if node
            .descriptor
            .view()
            .classes
            .iter()
            .any(|c| c == "endpoint-child")
        {
            found.push(id);
        }
        queue.extend(node.children.iter().copied());
    }
    found
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real Button expected");
    };
    p.label.clone()
}
fn retained(case: Case) {
    let a = source(case, true);
    let b = source(case, false);
    assert_eq!(a.0, b.0);
    assert_eq!(
        render::to_svg(&a.0.scene).unwrap(),
        render::to_svg(&b.0.scene).unwrap()
    );
    assert_eq!(a.1.layout, b.1.layout);
    assert_eq!(a.1.data.len(), 1);
    let old = a.1.data.values().next().unwrap();
    let new = b.1.data.values().next().unwrap();
    assert_eq!(
        old.part, new.part,
        "ordinary BulletPart numeric equality must stay unchanged"
    );
    assert_eq!(old.part.raw_value.to_bits(), new.part.raw_value.to_bits());
    assert_eq!(old.part.interval[0].to_bits(), (-0_f64).to_bits());
    assert_eq!(new.part.interval[0].to_bits(), 0_f64.to_bits());
    assert_eq!(
        a.1,
        source(case, true).1,
        "fresh equal source allocations must remain equal"
    );
    let app = App::memory();
    let input = app.scope().cage(a);
    let frame = app.scope().bond(move || input.with(|s| s.0.clone()));
    let widgets = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_with_data(widgets, move |c| {
                called.set(called.get() + 1);
                let typed = c.frame;
                Ok(glory::ui::widget(move |scope| {
                    let count = scope.cage(0);
                    let focus = scope.cage(false);
                    glory::ui::button()
                        .class("endpoint-child")
                        .label(scope.bond(move || {
                            typed.with(|f| {
                                format!(
                                    "{:016x}/count={}/focus={}",
                                    f.data.part.interval[0].to_bits(),
                                    count.get(),
                                    focus.get()
                                )
                            })
                        }))
                        .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                        .on(EventKind::FocusChanged, move |e| {
                            if let Event::FocusChanged(v) = e {
                                focus.revise(|f| *f = v);
                            }
                        })
                        .show_in(scope)
                }))
            }),
    )
    .unwrap();
    let child = children(&app)[0];
    app.dispatch(child, Event::Activate).unwrap();
    app.dispatch(child, Event::FocusChanged(true)).unwrap();
    assert_eq!(text(&app, child), "8000000000000000/count=1/focus=true");
    input.revise(|s| *s = b);
    app.flush().unwrap();
    assert_eq!(children(&app), [child]);
    assert_eq!(calls.get(), 1);
    assert_eq!(
        text(&app, child),
        "0000000000000000/count=1/focus=true",
        "{case:?}: previous original endpoint lost"
    );
    let mut denied = source(case, true);
    denied.1.layout.slots[0].placement = PlotWidgetPlacement::Rect(LayoutRect {
        x: f64::NAN,
        y: 50.,
        width: 40.,
        height: 20.,
    });
    input.revise(|s| *s = denied);
    app.flush().unwrap();
    assert_eq!(errors.borrow().len(), 1);
    assert_eq!(text(&app, child), "0000000000000000/count=1/focus=true");
    input.revise(|s| *s = source(case, true));
    app.flush().unwrap();
    assert_eq!(text(&app, child), "8000000000000000/count=1/focus=true");
    assert_eq!(children(&app), [child]);
    assert_eq!(calls.get(), 1);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}
#[test]
fn positive_range_retains_previous_original_zero_endpoint() {
    retained(Case::Range);
}
#[test]
fn positive_measure_retains_previous_original_zero_endpoint() {
    retained(Case::Measure);
}
#[test]
fn synthetic_none_range_retains_previous_original_zero_endpoint() {
    retained(Case::SyntheticRange);
}
#[test]
fn marker_has_no_previous_array_endpoint_and_keeps_its_owned_data_equal() {
    let a = source(Case::Marker, true);
    let b = source(Case::Marker, false);
    assert_eq!(a.0, b.0);
    assert_eq!(a.1, b.1);
    let data = a.1.data.values().next().unwrap();
    assert_eq!(data.part.interval, [4., 4.]);
    assert_eq!(data.part.raw_value.to_bits(), 4_f64.to_bits());
    assert_eq!(data.part.source_index, Some(0));
}
#[test]
fn visible_interval_and_ordinary_geometry_keep_numeric_equality() {
    let l = layout(Case::Range, true);
    assert_eq!(l, layout(Case::Range, false));
    let p = packet(Case::Range, &l);
    let a = p.data.values().next().unwrap();
    let mut b = a.clone();
    b.part.visible_interval[0] = if b.part.visible_interval[0].is_sign_negative() {
        0.
    } else {
        -0.
    };
    assert_eq!(
        a, &b,
        "visible projection is not an original endpoint bit contract"
    );
    let mut upper = a.clone();
    upper.part.interval = [1., -0.];
    let mut other = upper.clone();
    other.part.interval[1] = 0.;
    assert_ne!(upper, other, "both original interval endpoints participate");
}
