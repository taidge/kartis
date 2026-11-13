use glory::{
    App,
    render::{Event, NodeId, Props},
    style::{Display, Length, Position},
};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
fn rect(x: f64, y: f64, w: f64, h: f64) -> LayoutRect {
    LayoutRect {
        x,
        y,
        width: w,
        height: h,
    }
}
fn scene() -> Scene {
    let mut s = ChartConfig {
        width: 240.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (index, id) in ["a", "b", "c"].into_iter().enumerate() {
        let mut m = Mark::new(
            id,
            Geometry::Circle {
                x: 220.,
                y: 140.,
                radius: 5.,
            },
            Color(20, 30, 40, 255),
        );
        m.series = id.into();
        m.label = id.into();
        m.value = index as f64 + 1.;
        s.marks.push(m);
        s.legend.push(LegendItem {
            id: id.into(),
            label: id.into(),
            color: Color(20, 30, 40, 255),
        });
    }
    for text in ["Low", "High"] {
        s.labels.push(Label {
            x: 100.,
            y: 100.,
            text: text.into(),
            size: 12.,
            color: Color(20, 30, 40, 255),
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
    s
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut found = Vec::new();
    while let Some(id) = pending.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            found.push(id);
        }
        pending.extend(n.children.iter().rev().copied());
    }
    found
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
        Props::Button(p) => p.label.clone(),
        _ => panic!("expected text"),
    }
}

#[test]
fn explicit_legend_measurements_drive_actual_multicolumn_widget_positions() {
    let app = App::memory();
    app.mount(
        chart(scene())
            .text_measurer(|_, _| {
                Ok(TextSize {
                    width: 40.,
                    height: 18.,
                })
            })
            .legend_layout(LegendLayoutOptions {
                bounds: rect(0., 0., 200., 80.),
                max_columns: 2,
                row_gap: 4.,
                column_gap: 8.,
                ..Default::default()
            }),
    )
    .unwrap();
    let ids = nodes(&app, "kartis-legend-item");
    let tree = app.tree();
    let first = tree.node(ids[0]).unwrap().descriptor.view().style;
    let second = tree.node(ids[1]).unwrap().descriptor.view().style;
    let third = tree.node(ids[2]).unwrap().descriptor.view().style;
    assert_eq!(first.position, Some(Position::Absolute));
    assert_eq!(first.width, Some(Length::Px(58.)));
    assert_eq!(second.left, Some(Length::Px(66.)));
    assert_eq!(third.left, Some(Length::Px(0.)));
    assert_eq!(third.top, Some(Length::Px(22.)));
}
#[test]
fn tooltip_avoids_obstacles_and_uses_measured_clamped_style_in_actual_widget() {
    let app = App::memory();
    let hover = app.scope().cage(Some("a".into()));
    app.mount(
        chart(scene())
            .hovered(hover)
            .text_measurer(|_, _| {
                Ok(TextSize {
                    width: 70.,
                    height: 30.,
                })
            })
            .tooltip_layout(TooltipLayoutOptions {
                viewport: rect(0., 0., 240., 160.),
                forbidden: vec![rect(180., 110., 60., 50.)],
                ..Default::default()
            }),
    )
    .unwrap();
    let tree = app.tree();
    let style = tree
        .node(nodes(&app, "kartis-tooltip")[0])
        .unwrap()
        .descriptor
        .view()
        .style;
    assert_eq!(style.position, Some(Position::Absolute));
    assert_eq!(style.width, Some(Length::Px(70.)));
    assert_eq!(style.height, Some(Length::Px(30.)));
    let (Some(Length::Px(x)), Some(Length::Px(y))) = (style.left, style.top) else {
        panic!("placed")
    };
    let r = rect(x, y, 70., 30.);
    assert!(rect(4., 4., 232., 152.).contains(r));
    assert_eq!(r.overlap(rect(180., 110., 60., 50.)), 0.);
}
#[test]
fn label_collision_priorities_and_alternatives_change_real_label_nodes() {
    let app = App::memory();
    app.mount(
        chart(scene())
            .text_measurer(|_, _| {
                Ok(TextSize {
                    width: 40.,
                    height: 20.,
                })
            })
            .label_collision(LabelCollisionOptions {
                viewport: rect(0., 90., 240., 24.),
                padding: 2.,
                ..Default::default()
            })
            .label_priorities(BTreeMap::from([(1, 10)])),
    )
    .unwrap();
    let ids = nodes(&app, "kartis-label");
    let tree = app.tree();
    assert_eq!(
        tree.node(ids[0]).unwrap().descriptor.view().style.display,
        Some(Display::None)
    );
    assert_ne!(
        tree.node(ids[1]).unwrap().descriptor.view().style.display,
        Some(Display::None)
    );
    assert_eq!(
        tree.node(ids[1]).unwrap().descriptor.view().style.width,
        Some(Length::Px(40.))
    );
}
#[test]
fn custom_rust_widgets_bind_current_legend_and_tooltip_data_and_keep_controlled_toggle() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let hovered = app.scope().cage(Some("a".into()));
    let hidden = app.scope().cage(BTreeSet::new());
    let proposals = Rc::new(RefCell::new(Vec::new()));
    let received = proposals.clone();
    app.mount(
        chart(source)
            .hovered(hovered)
            .hidden(hidden)
            .on_hidden_change(move |h| received.borrow_mut().push(h))
            .legend_item_renderer(|ctx| glory::ui::text().class("custom-legend").content(ctx.label))
            .legend_symbol_renderer(|ctx| {
                glory::ui::text()
                    .class("custom-symbol")
                    .content_fn(move || {
                        ctx.hidden
                            .with(|h| if *h { "off".into() } else { "on".into() })
                    })
            })
            .tooltip_formatter(|marks| {
                Ok(format!("<b>{}:{:.1}</b>", marks[0].label, marks[0].value))
            })
            .tooltip_renderer(|ctx| {
                glory::ui::view().fill((
                    glory::ui::text()
                        .class("custom-tooltip-text")
                        .content(ctx.text),
                    glory::ui::text()
                        .class("custom-tooltip-id")
                        .content_fn(move || {
                            ctx.marks
                                .with(|m| m.first().map(|m| m.id.clone()).unwrap_or_default())
                        }),
                ))
            }),
    )
    .unwrap();
    assert_eq!(text(&app, "custom-legend"), "a");
    assert_eq!(text(&app, "custom-symbol"), "on");
    assert_eq!(text(&app, "custom-tooltip-text"), "<b>a:1.0</b>");
    assert_eq!(text(&app, "custom-tooltip-id"), "a");
    source.revise(|s| {
        s.legend[0].label = "Updated legend".into();
        s.marks[0].label = "Updated datum".into();
        s.marks[0].value = 12.5;
    });
    app.flush().unwrap();
    assert_eq!(text(&app, "custom-legend"), "Updated legend");
    assert_eq!(
        text(&app, "custom-tooltip-text"),
        "<b>Updated datum:12.5</b>"
    );
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(text(&app, "custom-symbol"), "on");
    assert_eq!(proposals.borrow().len(), 1);
    hidden.revise(|h| *h = proposals.borrow()[0].clone());
    app.flush().unwrap();
    assert_eq!(text(&app, "custom-symbol"), "off");
    assert_eq!(text(&app, "custom-tooltip-id"), "");
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}
#[test]
fn custom_renderer_ssr_escapes_text_and_retains_accessible_button_names() {
    let html = glory::ssr::render(
        chart(scene())
            .hovered(Some("a".into()))
            .legend_item_renderer(|ctx| {
                glory::ui::text().content_fn(move || ctx.label.with(|s| format!("<i>{s}</i>")))
            })
            .tooltip_formatter(|_| Ok("<script>safe text</script>".into()))
            .tooltip_renderer(|ctx| glory::ui::text().content(ctx.text)),
    )
    .unwrap();
    assert!(!html.contains("<script>"));
    assert!(html.contains("&lt;script&gt;safe text&lt;/script&gt;"));
    assert!(html.contains("&lt;i&gt;a&lt;/i&gt;"));
    assert!(html.contains("aria-label=\"Hide a\""));
}
#[test]
fn failed_measurement_keeps_previous_scene_layout_and_controlled_visibility() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let hidden = app.scope().cage(BTreeSet::new());
    let fail = Rc::new(Cell::new(false));
    let f = fail.clone();
    let errors = Rc::new(Cell::new(0));
    let observed = errors.clone();
    app.mount(
        chart(source)
            .hidden(hidden)
            .legend_layout(LegendLayoutOptions::default())
            .text_measurer(move |_, _| {
                if f.get() {
                    Err(ChartError("measurement failed".into()))
                } else {
                    Ok(TextSize {
                        width: 40.,
                        height: 20.,
                    })
                }
            })
            .on_reject(move |_| observed.set(observed.get() + 1)),
    )
    .unwrap();
    let original = nodes(&app, "kartis-legend-item")[0];
    fail.set(true);
    source.revise(|s| {
        s.legend[0].label = "Rejected".into();
        s.marks[0].value = 99.;
    });
    hidden.revise(|h| {
        h.insert("a".into());
    });
    app.flush().unwrap();
    assert_eq!(errors.get(), 1);
    assert_eq!(text(&app, "kartis-legend-item"), "Hide a");
    assert_eq!(text(&app, "kartis-datum"), "a: 1");
    assert_eq!(nodes(&app, "kartis-legend-item")[0], original);
    assert_eq!(nodes(&app, "kartis-datum").len(), 3);
    fail.set(false);
    source.revise(|s| s.title = "retry".into());
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-legend-item"), "Show Rejected");
    assert_eq!(nodes(&app, "kartis-datum").len(), 2);
}
#[test]
fn custom_formatter_error_clears_current_content_and_recovers() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let errors = Rc::new(Cell::new(0));
    let count = errors.clone();
    app.mount(
        chart(source)
            .hovered(Some("a".into()))
            .tooltip_formatter(|m| {
                if m[0].value > 10. {
                    Err(ChartError("custom failure".into()))
                } else {
                    Ok(m[0].value.to_string())
                }
            })
            .on_reject(move |_| count.set(count.get() + 1)),
    )
    .unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "1");
    source.revise(|s| s.marks[0].value = 20.);
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "");
    assert_eq!(errors.get(), 1);
    source.revise(|s| s.marks[0].value = 3.);
    app.flush().unwrap();
    assert_eq!(text(&app, "kartis-tooltip"), "3");
}
#[test]
fn factories_formatter_and_measurement_captures_drop_on_cleanup() {
    struct Guard(Rc<Cell<usize>>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.set(self.0.get() + 1);
        }
    }
    let app = App::memory();
    let source = app.scope().cage(scene());
    let drops = Rc::new(Cell::new(0));
    let one = Guard(drops.clone());
    let two = Guard(drops.clone());
    let three = Guard(drops.clone());
    let four = Guard(drops.clone());
    let five = Guard(drops.clone());
    app.mount(
        chart(source)
            .hovered(Some("a".into()))
            .text_measurer(move |_, _| {
                let _ = &one;
                Ok(TextSize {
                    width: 20.,
                    height: 20.,
                })
            })
            .legend_item_renderer(move |ctx| {
                let _ = &two;
                glory::ui::text().content(ctx.label)
            })
            .legend_symbol_renderer(move |_| {
                let _ = &three;
                glory::ui::text().content("*".to_string())
            })
            .tooltip_renderer(move |ctx| {
                let _ = &four;
                glory::ui::text().content(ctx.text)
            })
            .tooltip_formatter(move |_| {
                let _ = &five;
                Ok("text".into())
            }),
    )
    .unwrap();
    app.clear().unwrap();
    assert_eq!(drops.get(), 5);
    source.revise(|s| s.title = "retired".into());
    app.flush().unwrap();
    assert_eq!(drops.get(), 5);
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}
#[test]
fn automatic_layout_requires_explicit_measurement_and_no_silent_raster_noop() {
    let app = App::memory();
    assert!(
        app.mount(chart(scene()).legend_layout(LegendLayoutOptions::default()))
            .is_err()
    );
    assert_eq!(app.stats().nodes, 0);
    assert!(
        app.mount(
            chart(scene())
                .plot(false)
                .label_collision(LabelCollisionOptions::default())
                .text_measurer(|_, _| Ok(TextSize {
                    width: 10.,
                    height: 10.
                }))
        )
        .is_err()
    );
}

#[test]
fn initial_measurement_and_widget_factory_may_retire_the_owner_without_dead_writes() {
    for factory in [false, true] {
        let app = Rc::new(App::memory());
        let weak = Rc::downgrade(&app);
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if factory {
                app.mount(chart(scene()).legend_item_renderer(move |_| {
                    weak.upgrade().unwrap().scope().owner().dispose();
                    glory::ui::text().content("retired".to_string())
                }))
            } else {
                app.mount(
                    chart(scene())
                        .legend_layout(LegendLayoutOptions::default())
                        .text_measurer(move |_, _| {
                            weak.upgrade().unwrap().scope().owner().dispose();
                            Ok(TextSize {
                                width: 20.,
                                height: 20.,
                            })
                        }),
                )
            }
        }));
        assert!(
            outcome.is_ok(),
            "custom initial callback panicked after retiring owner"
        );
        // Initial measurement is synchronous construction; legend factories
        // run in Each's structural phase, where retiring the owner can be a
        // successful empty commit. Both must leave no nodes or handlers.
        if !factory {
            assert!(outcome.unwrap().is_err());
        }
        assert_eq!(app.stats().nodes, 0);
        assert_eq!(app.stats().handlers, 0);
    }
}

#[test]
fn formatter_can_retire_during_reactive_update_and_uncontrolled_layout_failure_keeps_toggle() {
    let app = Rc::new(App::memory());
    let source = app.scope().cage(scene());
    let weak = Rc::downgrade(&app);
    app.mount(
        chart(source)
            .hovered(Some("a".into()))
            .tooltip_formatter(move |marks| {
                if marks[0].value > 10. {
                    assert!(matches!(
                        weak.upgrade().unwrap().clear(),
                        Err(glory::ui::Error::Access(
                            glory::core::AccessError::ReentrantFlush
                        ))
                    ));
                }
                Ok("text".into())
            }),
    )
    .unwrap();
    source.revise(|s| s.marks[0].value = 11.);
    app.flush().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    let app = App::memory();
    let fail = Rc::new(Cell::new(false));
    let flag = fail.clone();
    let rejected = Rc::new(Cell::new(0));
    let count = rejected.clone();
    let proposals = Rc::new(Cell::new(0));
    let proposed = proposals.clone();
    app.mount(
        chart(scene())
            .legend_layout(LegendLayoutOptions::default())
            .text_measurer(move |_, _| {
                if flag.get() {
                    Err(ChartError("no metrics".into()))
                } else {
                    Ok(TextSize {
                        width: 20.,
                        height: 20.,
                    })
                }
            })
            .on_reject(move |_| count.set(count.get() + 1))
            .on_hidden_change(move |_| proposed.set(proposed.get() + 1)),
    )
    .unwrap();
    fail.set(true);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    assert_eq!(rejected.get(), 1);
    assert_eq!(proposals.get(), 0);
    assert_eq!(text(&app, "kartis-legend-item"), "Hide a");
    assert_eq!(nodes(&app, "kartis-datum").len(), 3);
}
