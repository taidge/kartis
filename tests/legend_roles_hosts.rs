use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
#[cfg(feature = "raster")]
use glory::Widget;
use glory::render::{Event, NodeId, Props};
use kartis::*;

fn scene(category: bool) -> Scene {
    let mut scene = ChartConfig {
        width: 240.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for id in ["same", "other"] {
        let mut mark = Mark::new(
            id,
            Geometry::Circle {
                x: 100.,
                y: 80.,
                radius: 6.,
            },
            Color(10, 20, 30, 255),
        );
        mark.series = id.into();
        mark.label = id.into();
        scene.marks.push(mark);
        scene.legend.push(LegendItem {
            id: id.into(),
            label: format!("<{id}>"),
            color: Color(10, 20, 30, 255),
        });
    }
    if category {
        scene
            .legend_roles
            .insert("same".into(), LegendRole::Category);
    }
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        todo.extend(n.children.iter().rev().copied());
    }
    out
}
fn button(app: &App, id: NodeId) -> Option<String> {
    match app.tree().node(id).unwrap().descriptor.view().props {
        Props::Button(p) => Some(p.label.clone()),
        _ => None,
    }
}
fn text(app: &App, class: &str) -> String {
    let tree = app.tree();
    match tree
        .node(nodes(app, class)[0])
        .unwrap()
        .descriptor
        .view()
        .props
    {
        Props::Text(p) => p.content.clone(),
        _ => panic!("text"),
    }
}
#[test]
fn category_is_static_readable_and_not_dimmed_by_same_named_series() {
    let app = App::memory();
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    app.mount(
        chart(scene(true))
            .default_hidden(BTreeSet::from(["same".into()]))
            .on_hidden_change(move |_| count.set(count.get() + 1)),
    )
    .unwrap();
    let items = nodes(&app, "kartis-legend-item");
    assert!(button(&app, items[0]).is_none());
    assert_eq!(button(&app, items[1]).as_deref(), Some("Hide <other>"));
    assert_eq!(
        app.tree()
            .node(nodes(&app, "kartis-legend-symbol")[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .opacity,
        Some(1.)
    );
    app.dispatch(items[0], Event::Activate).unwrap();
    assert_eq!(changes.get(), 0);
    app.dispatch(items[1], Event::Activate).unwrap();
    assert_eq!(changes.get(), 1);
}
#[test]
fn explicit_action_delivers_current_original_identity_and_never_hidden_proposal() {
    let app = App::memory();
    let source = app.scope().cage(scene(true));
    let seen = Rc::new(RefCell::new(vec![]));
    let output = seen.clone();
    let hidden = Rc::new(RefCell::new(vec![]));
    let h = hidden.clone();
    app.mount(
        chart(source)
            .on_legend_activate(move |a| output.borrow_mut().push(a))
            .on_hidden_change(move |p| h.borrow_mut().push(p)),
    )
    .unwrap();
    source.revise(|s| {
        s.legend[0].label = "Renamed, not identity".into();
        s.legend[0].color = Color(40, 50, 60, 70);
    });
    app.flush().unwrap();
    let items = nodes(&app, "kartis-legend-item");
    assert_eq!(
        button(&app, items[0]).as_deref(),
        Some("Renamed, not identity")
    );
    app.dispatch(items[0], Event::Activate).unwrap();
    assert!(hidden.borrow().is_empty());
    assert_eq!(
        seen.borrow()[0],
        LegendActivation {
            item: source.get_untracked().legend[0].clone(),
            role: LegendRole::Category,
            target: None,
        }
    );
    app.dispatch(items[1], Event::Activate).unwrap();
    assert_eq!(hidden.borrow().len(), 1);
    assert_eq!(seen.borrow()[1].role, LegendRole::Series);
    assert_eq!(seen.borrow()[1].item.id, "other");
}
#[test]
fn role_only_updates_rebuild_same_id_and_refresh_custom_context() {
    let app = App::memory();
    let source = app.scope().cage(scene(false));
    let changes = Rc::new(Cell::new(0));
    let count = changes.clone();
    app.mount(
        chart(source)
            .on_hidden_change(move |_| count.set(count.get() + 1))
            .legend_symbol_renderer(|ctx| {
                glory::ui::text()
                    .class("role-context")
                    .content_fn(move || format!("{:?}:{}", ctx.role.get(), ctx.hidden.get()))
            }),
    )
    .unwrap();
    let old = nodes(&app, "kartis-legend-item")[0];
    assert!(button(&app, old).is_some());
    source.revise(|s| {
        s.legend_roles.insert("same".into(), LegendRole::Category);
    });
    app.flush().unwrap();
    let static_id = nodes(&app, "kartis-legend-item")[0];
    assert_ne!(old, static_id);
    assert!(button(&app, static_id).is_none());
    assert_eq!(text(&app, "role-context"), "Category:false");
    assert!(app.tree().node(old).is_err());
    app.dispatch(static_id, Event::Activate).unwrap();
    assert_eq!(changes.get(), 0);
    source.revise(|s| s.legend_roles.clear());
    app.flush().unwrap();
    let current = nodes(&app, "kartis-legend-item")[0];
    assert!(button(&app, current).is_some());
    assert_ne!(static_id, current);
    app.dispatch(current, Event::Activate).unwrap();
    assert_eq!(changes.get(), 1);
}
#[test]
fn typed_action_role_change_keeps_category_unpressed_in_actual_ssr() {
    let s = scene(true);
    let html = glory::ssr::render(chart(s.clone()).plot(false).data_controls(false)).unwrap();
    assert!(html.contains("&lt;same&gt;"));
    assert!(!html.contains("Hide &lt;same&gt;"));
    assert!(html.contains("Hide &lt;other&gt;"));
    let html = glory::ssr::render(
        chart(s)
            .plot(false)
            .data_controls(false)
            .on_legend_activate(|_| {}),
    )
    .unwrap();
    let category = html
        .split("class=\"kartis-legend-item\"")
        .nth(1)
        .unwrap()
        .split("</button>")
        .next()
        .unwrap();
    assert!(category.contains("&lt;same&gt;"));
    assert!(!category.contains("aria-pressed"));
    assert!(!category.contains("Hide "));
}
#[test]
fn measure_plain_category_and_live_role_fonts_before_publication() {
    let app = App::memory();
    let source = app.scope().cage(scene(true));
    let seen = Rc::new(RefCell::new(vec![]));
    let output = seen.clone();
    app.mount(
        chart(source)
            .legend_layout(LegendLayoutOptions {
                bounds: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 240.,
                    height: 160.,
                },
                ..Default::default()
            })
            .text_measurer(move |text, font| {
                output.borrow_mut().push((text.to_owned(), font.clone()));
                Ok(TextSize {
                    width: 45.,
                    height: 16.,
                })
            }),
    )
    .unwrap();
    assert!(seen.borrow().iter().any(|(s, _)| s == "<same>"));
    assert!(!seen.borrow().iter().any(|(s, _)| s == "Hide <same>"));
    source.revise(|s| s.typography.legend.weight = Some(650.));
    app.flush().unwrap();
    assert!(
        seen.borrow()
            .iter()
            .any(|(s, f)| s == "<same>" && f.weight == 650.)
    );
    assert_eq!(
        app.tree()
            .node(nodes(&app, "kartis-legend-item")[0])
            .unwrap()
            .descriptor
            .view()
            .style
            .font_weight,
        Some(650.)
    );
}
#[test]
fn bad_role_update_keeps_old_controls_then_valid_role_recovers() {
    let app = App::memory();
    let source = app.scope().cage(scene(false));
    let failures = Rc::new(Cell::new(0));
    let f = failures.clone();
    app.mount(chart(source).on_reject(move |_| f.set(f.get() + 1)))
        .unwrap();
    let first = nodes(&app, "kartis-legend-item")[0];
    source.revise(|s| {
        s.legend_roles
            .insert("missing".into(), LegendRole::Category);
        s.legend[0].label = "rejected".into();
    });
    app.flush().unwrap();
    assert_eq!(failures.get(), 1);
    assert_eq!(first, nodes(&app, "kartis-legend-item")[0]);
    assert_eq!(button(&app, first).as_deref(), Some("Hide <same>"));
    source.revise(|s| {
        s.legend_roles.clear();
        s.legend_roles.insert("same".into(), LegendRole::Category);
    });
    app.flush().unwrap();
    assert!(button(&app, nodes(&app, "kartis-legend-item")[0]).is_none());
}
#[cfg(feature = "raster")]
fn raster(app: &App, enabled: bool) -> RasterSurface {
    let output = Rc::new(RefCell::new(None));
    let out = output.clone();
    app.mount(glory::ui::widget(move |scope| {
        let surface = RasterSurface::mount(
            scope,
            scene(true),
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                legend_actions: enabled,
                ..Default::default()
            },
        )
        .unwrap();
        let widget = surface.widget();
        *out.borrow_mut() = Some(surface);
        widget.show_in(scope)
    }))
    .unwrap();
    output.borrow_mut().take().unwrap()
}
#[test]
#[cfg(feature = "raster")]
fn raster_subscription_is_not_implicit_action_enablement() {
    let app = App::memory();
    let surface = raster(&app, false);
    let seen = Rc::new(Cell::new(0));
    let s = seen.clone();
    surface
        .on_legend_activate(move |_| s.set(s.get() + 1))
        .unwrap();
    let id = nodes(&app, "kartis-legend-item")[0];
    assert!(button(&app, id).is_none());
    let before = surface.handle().snapshot().unwrap();
    app.dispatch(id, Event::Activate).unwrap();
    assert_eq!(seen.get(), 0);
    assert_eq!(before, surface.handle().snapshot().unwrap());
}
#[test]
#[cfg(feature = "raster")]
fn raster_explicit_category_action_and_reentrant_subscriber_replacement() {
    let app = App::memory();
    let surface = raster(&app, true);
    let count = Rc::new(Cell::new(0));
    let output = count.clone();
    let h = surface.handle();
    surface
        .on_legend_activate(move |a| {
            assert_eq!(a.role, LegendRole::Category);
            assert_eq!(a.item.id, "same");
            output.set(output.get() + 1);
            let second = output.clone();
            h.on_legend_activate(move |_| second.set(second.get() + 10))
                .unwrap();
        })
        .unwrap();
    let before = surface.handle().snapshot().unwrap();
    let id = nodes(&app, "kartis-legend-item")[0];
    assert_eq!(button(&app, id).as_deref(), Some("<same>"));
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::Activate).unwrap();
    assert_eq!(count.get(), 11);
    assert_eq!(before, surface.handle().snapshot().unwrap());
    surface.stop();
    assert!(surface.on_legend_activate(|_| {}).is_err());
}
#[test]
fn series_activation_intent_precedes_hidden_proposal() {
    let app = App::memory();
    let order = Rc::new(RefCell::new(vec![]));
    let a = order.clone();
    let h = order.clone();
    app.mount(
        chart(scene(false))
            .on_legend_activate(move |_| a.borrow_mut().push("intent"))
            .on_hidden_change(move |_| h.borrow_mut().push("hidden")),
    )
    .unwrap();
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(*order.borrow(), vec!["intent", "hidden"]);
}
#[test]
fn reentrant_category_or_removed_source_does_not_receive_old_series_toggle() {
    for remove in [false, true] {
        let app = Rc::new(App::memory());
        let source = app.scope().cage(scene(false));
        let changes = Rc::new(Cell::new(0));
        let h = changes.clone();
        let nested = app.clone();
        app.mount(
            chart(source)
                .on_legend_activate(move |a| {
                    assert_eq!(a.role, LegendRole::Series);
                    source.revise(|s| {
                        if remove {
                            s.legend.retain(|i| i.id != "same");
                        } else {
                            s.legend_roles.insert("same".into(), LegendRole::Category);
                        }
                    });
                    nested.flush().unwrap();
                })
                .on_hidden_change(move |_| h.set(h.get() + 1)),
        )
        .unwrap();
        app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
            .unwrap();
        assert_eq!(changes.get(), 0);
    }
}
