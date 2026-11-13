use glory::{
    App,
    render::{Event, Navigation, NodeId, PointerPhase, Props},
    style::{Display, Length},
};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};
fn scene() -> Scene {
    let mut s = ChartConfig::default().scene().unwrap();
    for (i, series) in ["alpha", "beta"].into_iter().enumerate() {
        let mut mark = Mark::new(
            format!("datum-{series}"),
            Geometry::Rect {
                x: 10. + i as f64 * 40.,
                y: 10.,
                width: 30.,
                height: 40.,
            },
            Color(30, 90, 150, 255),
        );
        mark.series = series.into();
        mark.label = series.into();
        mark.value = 10. + i as f64;
        s.marks.push(mark);
        s.legend.push(LegendItem {
            id: series.into(),
            label: series.into(),
            color: Color(30, 90, 150, 255),
        });
    }
    s
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = Vec::new();
    while let Some(id) = stack.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        stack.extend(n.children.iter().rev().copied());
    }
    out
}
fn text(app: &App, node: NodeId) -> String {
    let tree = app.tree();
    match tree.node(node).unwrap().descriptor.view().props {
        Props::Text(p) => p.content.clone(),
        Props::Button(p) => p.label.clone(),
        _ => panic!("unexpected text kind"),
    }
}
fn pointer(app: &App, node: NodeId, phase: PointerPhase) {
    app.dispatch(
        node,
        Event::Pointer {
            from: node,
            related: None,
            phase,
        },
    )
    .unwrap();
}
fn pressed(app: &App, node: NodeId) -> bool {
    app.tree()
        .node(node)
        .unwrap()
        .descriptor
        .view()
        .semantics
        .as_ref()
        .unwrap()
        .pressed
        .unwrap()
}

#[test]
fn uncontrolled_legend_filters_marks_and_data_and_updates_toggle_semantics() {
    let app = App::memory();
    let changes = Rc::new(RefCell::new(Vec::new()));
    let captured = changes.clone();
    app.mount(chart(scene()).on_hidden_change(move |h| captured.borrow_mut().push(h)))
        .unwrap();
    let legends = nodes(&app, "kartis-legend-item");
    assert!(pressed(&app, legends[0]));
    app.dispatch(legends[0], Event::Activate).unwrap();
    assert!(!pressed(&app, legends[0]));
    assert_eq!(text(&app, legends[0]), "Show alpha");
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
    assert_eq!(changes.borrow()[0], BTreeSet::from(["alpha".into()]));
    let wrappers = nodes(&app, "kartis-mark-clip");
    assert_eq!(
        app.tree()
            .node(wrappers[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    );
    app.dispatch(legends[0], Event::Activate).unwrap();
    assert!(pressed(&app, legends[0]));
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}
#[test]
fn controlled_legend_emits_a_proposal_until_the_caller_accepts_it() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::new());
    let proposal = Rc::new(RefCell::new(None));
    let captured = proposal.clone();
    app.mount(
        chart(scene())
            .hidden(hidden)
            .on_hidden_change(move |h| *captured.borrow_mut() = Some(h)),
    )
    .unwrap();
    let legend = nodes(&app, "kartis-legend-item")[0];
    app.dispatch(legend, Event::Activate).unwrap();
    assert!(pressed(&app, legend));
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    hidden.revise(|value| *value = proposal.borrow_mut().take().unwrap());
    app.flush().unwrap();
    assert!(!pressed(&app, legend));
    assert_eq!(nodes(&app, "kartis-datum").len(), 1);
}
#[test]
fn hover_focus_tooltips_highlights_and_callbacks_follow_the_current_datum() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let hover = Rc::new(RefCell::new(Vec::new()));
    let focus = Rc::new(RefCell::new(Vec::new()));
    let h = hover.clone();
    let f = focus.clone();
    app.mount(
        chart(source)
            .on_hover(move |m| h.borrow_mut().push(m.map(|m| m.value)))
            .on_focus(move |m| f.borrow_mut().push(m.map(|m| m.id))),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum");
    let tooltip = nodes(&app, "kartis-tooltip")[0];
    pointer(&app, data[0], PointerPhase::Enter);
    assert_eq!(text(&app, tooltip), "alpha: 10");
    assert_eq!(*hover.borrow(), [Some(10.)]);
    let graphics = nodes(&app, "kartis-mark");
    assert_eq!(
        app.tree()
            .node(graphics[1])
            .unwrap()
            .descriptor
            .view()
            .style
            .opacity,
        Some(0.35)
    );
    app.dispatch(data[1], Event::FocusChanged(true)).unwrap();
    assert_eq!(text(&app, tooltip), "alpha: 10");
    pointer(&app, data[0], PointerPhase::Leave);
    assert_eq!(text(&app, tooltip), "beta: 11");
    source.revise(|s| s.marks[1].value = 42.);
    app.flush().unwrap();
    assert_eq!(text(&app, tooltip), "beta: 42");
    app.dispatch(data[1], Event::FocusChanged(false)).unwrap();
    assert!(text(&app, tooltip).is_empty());
    assert_eq!(*focus.borrow(), [Some("datum-beta".into()), None]);
}
#[test]
fn selected_state_is_controlled_and_activation_reads_current_values() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let selected = app.scope().cage(None);
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let captured = proposals.clone();
    let activated = Rc::new(Cell::new(0.));
    let a = activated.clone();
    app.mount(
        chart(source)
            .selected(selected)
            .on_selection_change(move |m| captured.borrow_mut().push(m.map(|m| m.id)))
            .on_select(move |m| a.set(m.value)),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum");
    source.revise(|s| s.marks[0].value = 35.);
    app.flush().unwrap();
    app.dispatch(data[0], Event::Activate).unwrap();
    assert_eq!(activated.get(), 35.);
    assert!(!pressed(&app, data[0]));
    selected.revise(|s| *s = proposals.borrow()[0].clone());
    app.flush().unwrap();
    assert!(pressed(&app, data[0]));
}
#[test]
fn arrow_navigation_proposes_current_visible_destination_without_synthetic_focus() {
    let app = App::memory();
    let destinations = Rc::new(RefCell::new(Vec::new()));
    let capture = destinations.clone();
    let focused = Rc::new(Cell::new(0));
    let f = focused.clone();
    app.mount(
        chart(scene())
            .on_navigate(move |m| capture.borrow_mut().push(m.id))
            .on_focus(move |_| f.set(f.get() + 1)),
    )
    .unwrap();
    let data = nodes(&app, "kartis-datum");
    assert!(
        app.tree()
            .node(data[0])
            .unwrap()
            .descriptor
            .view()
            .semantics
            .as_ref()
            .unwrap()
            .navigation_item
    );
    app.dispatch(
        data[0],
        Event::Navigate {
            from: data[0],
            command: Navigation::Next,
        },
    )
    .unwrap();
    assert_eq!(*destinations.borrow(), ["datum-beta"]);
    assert_eq!(focused.get(), 0);
    app.dispatch(data[1], Event::FocusChanged(true)).unwrap();
    assert_eq!(focused.get(), 1);
}
#[test]
fn removed_data_clears_selection_and_stale_origins_cannot_emit() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let changes = Rc::new(RefCell::new(Vec::new()));
    let capture = changes.clone();
    app.mount(
        chart(source).on_selection_change(move |m| capture.borrow_mut().push(m.map(|m| m.id))),
    )
    .unwrap();
    let node = nodes(&app, "kartis-datum")[0];
    app.dispatch(node, Event::Activate).unwrap();
    source.revise(|s| {
        s.marks.remove(0);
    });
    app.flush().unwrap();
    assert_eq!(*changes.borrow(), [Some("datum-alpha".into()), None]);
    assert!(!app.dispatch(node, Event::Activate).unwrap_or(false));
    assert_eq!(changes.borrow().len(), 2);
}
#[test]
fn selection_callback_can_dispose_the_chart_without_followup_callbacks() {
    let app = Rc::new(App::memory());
    let weak = Rc::downgrade(&app);
    let called = Rc::new(Cell::new(0));
    let c = called.clone();
    app.mount(
        chart(scene())
            .on_selection_change(move |_| weak.upgrade().unwrap().clear().unwrap())
            .on_select(move |_| c.set(c.get() + 1)),
    )
    .unwrap();
    let node = nodes(&app, "kartis-datum")[0];
    app.dispatch(node, Event::Activate).unwrap();
    assert_eq!(called.get(), 0);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
#[test]
fn hover_loss_during_source_reconciliation_can_dispose_without_reading_dead_state() {
    let app = Rc::new(App::memory());
    let source = app.scope().cage(scene());
    let weak = Rc::downgrade(&app);
    app.mount(chart(source).on_hover(move |m| {
        if m.is_none() {
            let result = weak.upgrade().unwrap().clear();
            // Disposal succeeds, while Glory deliberately rejects the nested
            // flush. The enclosing flush completes the queued retirement.
            assert!(matches!(
                result,
                Err(glory::ui::Error::Access(
                    glory::core::AccessError::ReentrantFlush
                ))
            ));
        }
    }))
    .unwrap();
    let node = nodes(&app, "kartis-datum")[0];
    pointer(&app, node, PointerPhase::Enter);
    source.revise(|s| s.marks.clear());
    app.flush().unwrap();
    assert_eq!(app.stats().nodes, 0);
}
#[test]
fn clipped_graphics_retain_scene_viewbox_with_negative_inner_offsets() {
    let app = App::memory();
    let mut s = scene();
    s.marks[0].clip = Some(ClipRect {
        x: 50.,
        y: 60.,
        width: 100.,
        height: 120.,
    });
    app.mount(chart(s)).unwrap();
    let wrappers = nodes(&app, "kartis-mark-clip");
    let graphics = nodes(&app, "kartis-mark");
    let tree = app.tree();
    let wrapper = tree.node(wrappers[0]).unwrap().descriptor.view();
    assert_eq!(wrapper.style.left, Some(Length::Px(50.)));
    assert_eq!(wrapper.style.width, Some(Length::Px(100.)));
    let graphic = tree.node(graphics[0]).unwrap().descriptor.view();
    assert_eq!(graphic.style.left, Some(Length::Px(-50.)));
    assert_eq!(graphic.style.top, Some(Length::Px(-60.)));
    assert_eq!(graphic.style.width, Some(Length::Px(800.)));
}
#[test]
fn invalid_controlled_updates_retain_previous_model_and_report_original_diagnostic() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::new());
    let errors = Rc::new(RefCell::new(Vec::new()));
    let e = errors.clone();
    app.mount(
        chart(scene())
            .hidden(hidden)
            .on_reject(move |error| e.borrow_mut().push(error.0)),
    )
    .unwrap();
    hidden.revise(|s| {
        s.insert(String::new());
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    assert!(errors.borrow()[0].contains("hidden-series"));
}

#[test]
fn uncontrolled_legend_rejects_hidden_state_growth_at_the_budget() {
    let app = App::memory();
    let hidden = (0..256).map(|i| format!("unknown-{i}")).collect();
    let rejections = Rc::new(Cell::new(0));
    let captured = rejections.clone();
    app.mount(
        chart(scene())
            .default_hidden(hidden)
            .on_reject(move |_| captured.set(captured.get() + 1)),
    )
    .unwrap();
    let legend = nodes(&app, "kartis-legend-item")[0];
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(rejections.get(), 1);
    assert!(pressed(&app, legend));
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}

#[test]
fn widget_paints_mixed_items_in_order_and_reorders_existing_dom_identities() {
    let app = App::memory();
    let mut s = scene();
    s.labels = vec![
        Label {
            rotation: 0.,
            x: 20.,
            y: 30.,
            text: "alpha caption".into(),
            color: Color(0, 0, 0, 255),
            size: 12.,
            anchor: TextAnchor::Start,
        },
        Label {
            rotation: 0.,
            x: 60.,
            y: 30.,
            text: "shared axis".into(),
            color: Color(0, 0, 0, 255),
            size: 12.,
            anchor: TextAnchor::Start,
        },
    ];
    s.label_series.insert(0, "alpha".into());
    s.draw_order = vec![
        DrawItem::Mark(1),
        DrawItem::Label(0),
        DrawItem::Mark(0),
        DrawItem::Label(1),
    ];
    let source = app.scope().cage(s);
    app.mount(chart(source)).unwrap();
    let classes = |app: &App| {
        nodes(app, "kartis-paint-item")
            .iter()
            .map(|id| {
                let tree = app.tree();
                if tree
                    .node(*id)
                    .unwrap()
                    .descriptor
                    .view()
                    .classes
                    .iter()
                    .any(|c| c == "kartis-label")
                {
                    "label"
                } else {
                    "mark"
                }
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(classes(&app), ["mark", "label", "mark", "label"]);
    let original = nodes(&app, "kartis-paint-item");
    source.revise(|s| {
        s.draw_order = vec![
            DrawItem::Label(1),
            DrawItem::Mark(0),
            DrawItem::Label(0),
            DrawItem::Mark(1),
        ]
    });
    app.flush().unwrap();
    assert_eq!(
        nodes(&app, "kartis-paint-item"),
        [original[3], original[2], original[1], original[0]]
    );
    let legend = nodes(&app, "kartis-legend-item")[0];
    app.dispatch(legend, Event::Activate).unwrap();
    let labels = nodes(&app, "kartis-label");
    assert_ne!(
        app.tree()
            .node(labels[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    ); // shared axis
    assert_eq!(
        app.tree()
            .node(labels[1])
            .unwrap()
            .descriptor
            .view()
            .style
            .display,
        Some(Display::None)
    ); // owned alpha
}

#[test]
fn external_plot_mode_keeps_controls_without_allocating_semantic_graphics() {
    let app = App::memory();
    let mut s = scene();
    s.marks[0].geometry = Geometry::Path(
        std::iter::once(glory::render::PathCommand::Move(
            glory::render::GraphicPoint::new(0., 0.),
        ))
        .chain((0..300).map(|i| {
            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(i as f64, 50.))
        }))
        .collect(),
    );
    app.mount(chart(s).plot(false)).unwrap();
    assert!(nodes(&app, "kartis-plot").is_empty());
    assert!(nodes(&app, "kartis-mark").is_empty());
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
    assert_eq!(nodes(&app, "kartis-legend-item").len(), 2);
}
#[test]
fn externally_controlled_hover_updates_tooltips_and_button_events_only_propose() {
    let app = App::memory();
    let hovered = app.scope().cage(Some("datum-alpha".to_string()));
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let captured = proposals.clone();
    app.mount(
        chart(scene())
            .plot(false)
            .hovered(hovered)
            .on_hover(move |mark| captured.borrow_mut().push(mark.map(|m| m.id))),
    )
    .unwrap();
    let tooltip = nodes(&app, "kartis-tooltip")[0];
    assert_eq!(text(&app, tooltip), "alpha: 10");
    let beta = nodes(&app, "kartis-datum")[1];
    pointer(&app, beta, PointerPhase::Enter);
    assert_eq!(text(&app, tooltip), "alpha: 10");
    assert_eq!(*proposals.borrow(), [Some("datum-beta".into())]);
    hovered.revise(|id| *id = proposals.borrow()[0].clone());
    app.flush().unwrap();
    assert_eq!(text(&app, tooltip), "beta: 11");
}
