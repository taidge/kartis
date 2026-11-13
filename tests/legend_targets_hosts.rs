//! Memory-host protocol tests. These do not execute BrowserChart or a browser.
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use glory::App;
#[cfg(feature = "raster")]
use glory::Widget;
use glory::render::{Event, NodeId, Props};
use kartis::*;

fn source() -> Scene {
    let mut scene = ChartConfig {
        width: 320.,
        height: 200.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    for (i, series) in ["canonical-a", "canonical-b"].into_iter().enumerate() {
        let mut mark = Mark::new(
            format!("body-{i}"),
            Geometry::Rect {
                x: 45. + i as f64 * 110.,
                y: 55.,
                width: 65.,
                height: 75.,
            },
            Color(20 + i as u8 * 80, 40, 60, 255),
        );
        mark.series = series.into();
        mark.label = series.into();
        scene.marks.push(mark);
    }
    for (row, target) in [
        ("row-a", "canonical-a"),
        ("row-a-second", "canonical-a"),
        ("row-b", "canonical-b"),
    ] {
        scene.legend.push(LegendItem {
            id: row.into(),
            label: "Same caption".into(),
            color: Color(20, 40, 60, 255),
        });
        scene
            .legend_targets
            .insert(row.into(), LegendTarget::Series(target.into()));
    }
    scene.validate().unwrap();
    scene
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut out = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|c| c == class) {
            out.push(id);
        }
        pending.extend(node.children.iter().rev().copied());
    }
    out
}
fn button_label(app: &App, id: NodeId) -> Option<String> {
    match app.tree().node(id).unwrap().descriptor.view().props {
        Props::Button(p) => Some(p.label.clone()),
        _ => None,
    }
}
fn pressed(app: &App, id: NodeId) -> Option<bool> {
    app.tree()
        .node(id)
        .unwrap()
        .descriptor
        .view()
        .semantics
        .as_ref()
        .and_then(|s| s.pressed)
}
fn texts(app: &App, class: &str) -> Vec<String> {
    let ids = nodes(app, class);
    let tree = app.tree();
    ids.into_iter()
        .map(|id| match tree.node(id).unwrap().descriptor.view().props {
            Props::Text(p) => p.content.clone(),
            _ => panic!("expected text"),
        })
        .collect()
}
fn assert_pressed(app: &App, expected: [bool; 3]) {
    let rows = nodes(app, "kartis-legend-item");
    assert_eq!(rows.len(), 3);
    for (id, expected) in rows.into_iter().zip(expected) {
        assert_eq!(pressed(app, id), Some(expected));
    }
}

#[test]
fn semantic_rows_share_canonical_hidden_restore_and_report_distinct_row_identity() {
    let app = App::memory();
    let proposals = Rc::new(RefCell::new(vec![]));
    let p = proposals.clone();
    let actions = Rc::new(RefCell::new(vec![]));
    let a = actions.clone();
    app.mount(
        chart(source())
            .data_controls(false)
            .on_legend_activate(move |e| a.borrow_mut().push(e))
            .on_hidden_change(move |set| p.borrow_mut().push(set)),
    )
    .unwrap();
    assert_pressed(&app, [true, true, true]);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from(["canonical-a".into()])
    );
    assert_eq!(actions.borrow()[0].item.id, "row-a");
    assert_eq!(
        actions.borrow()[0].resolved_target(),
        LegendTarget::Series("canonical-a".into())
    );
    assert_pressed(&app, [false, false, true]);
    for (index, opacity) in [0.35, 0.35, 1.].into_iter().enumerate() {
        assert_eq!(
            app.tree()
                .node(nodes(&app, "kartis-legend-symbol")[index])
                .unwrap()
                .descriptor
                .view()
                .style
                .opacity,
            Some(opacity)
        );
    }
    app.dispatch(nodes(&app, "kartis-legend-item")[1], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert!(proposals.borrow()[1].is_empty());
    assert_eq!(actions.borrow()[1].item.id, "row-a-second");
    assert_eq!(
        actions.borrow()[1].resolved_target(),
        actions.borrow()[0].resolved_target()
    );
    assert_pressed(&app, [true, true, true]);
}

#[test]
fn identical_captions_do_not_merge_targets_or_overwrite_current_hidden_state() {
    let app = App::memory();
    let proposals = Rc::new(RefCell::new(vec![]));
    let p = proposals.clone();
    app.mount(chart(source()).on_hidden_change(move |set| p.borrow_mut().push(set)))
        .unwrap();
    app.dispatch(nodes(&app, "kartis-legend-item")[2], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from(["canonical-b".into()])
    );
    assert_pressed(&app, [true, true, false]);
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        proposals.borrow()[1],
        BTreeSet::from(["canonical-a".into(), "canonical-b".into()])
    );
    assert_pressed(&app, [false, false, false]);
}

#[test]
fn controlled_hidden_proposal_uses_canonical_target_and_waits_for_acceptance() {
    let app = App::memory();
    let hidden = app.scope().cage(BTreeSet::<String>::new());
    let proposals = Rc::new(RefCell::new(vec![]));
    let p = proposals.clone();
    app.mount(
        chart(source())
            .hidden(hidden)
            .on_hidden_change(move |set| p.borrow_mut().push(set)),
    )
    .unwrap();
    app.dispatch(nodes(&app, "kartis-legend-item")[1], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from(["canonical-a".into()])
    );
    assert!(hidden.get_untracked().is_empty());
    assert_pressed(&app, [true, true, true]);
    hidden.revise(|old| *old = proposals.borrow()[0].clone());
    app.flush().unwrap();
    assert_pressed(&app, [false, false, true]);
    app.dispatch(nodes(&app, "kartis-legend-item")[2], Event::Activate)
        .unwrap();
    assert_eq!(
        proposals.borrow()[1],
        BTreeSet::from(["canonical-a".into(), "canonical-b".into()])
    );
    assert_eq!(
        hidden.get_untracked(),
        BTreeSet::from(["canonical-a".into()])
    );
}

#[test]
fn empty_raw_category_is_static_without_handler_and_actionable_without_pressed_when_enabled() {
    for enabled in [false, true] {
        let app = App::memory();
        let mut scene = source();
        scene
            .legend_targets
            .insert("row-b".into(), LegendTarget::Category(String::new()));
        scene.legend[2].label = "Category <Å & é>".into();
        let actions = Rc::new(RefCell::new(vec![]));
        let a = actions.clone();
        let changes = Rc::new(Cell::new(0));
        let h = changes.clone();
        let widget = chart(scene)
            .default_hidden(BTreeSet::from(["canonical-a".into(), "row-b".into()]))
            .on_hidden_change(move |_| h.set(h.get() + 1));
        let widget = if enabled {
            widget.on_legend_activate(move |e| a.borrow_mut().push(e))
        } else {
            widget
        };
        app.mount(widget).unwrap();
        let row = nodes(&app, "kartis-legend-item")[2];
        assert_eq!(button_label(&app, row).is_some(), enabled);
        assert_eq!(pressed(&app, row), None);
        assert_eq!(
            app.tree()
                .node(nodes(&app, "kartis-legend-symbol")[2])
                .unwrap()
                .descriptor
                .view()
                .style
                .opacity,
            Some(1.)
        );
        app.dispatch(row, Event::Activate).unwrap();
        assert_eq!(changes.get(), 0);
        assert_eq!(actions.borrow().len(), usize::from(enabled));
        if enabled {
            assert_eq!(actions.borrow()[0].item.id, "row-b");
            assert_eq!(
                actions.borrow()[0].resolved_target(),
                LegendTarget::Category(String::new())
            );
        }
    }
}

#[test]
fn reentrant_target_change_or_row_removal_cannot_receive_the_old_series_toggle() {
    for remove in [false, true] {
        let app = Rc::new(App::memory());
        let source = app.scope().cage(source());
        let nested = app.clone();
        let first = Rc::new(Cell::new(true));
        let only_once = first.clone();
        let actions = Rc::new(RefCell::new(vec![]));
        let a = actions.clone();
        let proposals = Rc::new(RefCell::new(vec![]));
        let p = proposals.clone();
        app.mount(
            chart(source)
                .on_legend_activate(move |e| {
                    a.borrow_mut().push(e);
                    if only_once.replace(false) {
                        source.revise(|scene| {
                            if remove {
                                scene.legend.retain(|row| row.id != "row-a");
                                scene.legend_targets.remove("row-a");
                            } else {
                                scene.legend_targets.insert(
                                    "row-a".into(),
                                    LegendTarget::Series("canonical-b".into()),
                                );
                            }
                        });
                        nested.flush().unwrap();
                    }
                })
                .on_hidden_change(move |set| p.borrow_mut().push(set)),
        )
        .unwrap();
        app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
            .unwrap();
        assert!(proposals.borrow().is_empty());
        assert_eq!(actions.borrow()[0].item.id, "row-a");
        assert_eq!(
            actions.borrow()[0].resolved_target(),
            LegendTarget::Series("canonical-a".into())
        );
        if !remove {
            app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
                .unwrap();
            assert_eq!(
                proposals.borrow()[0],
                BTreeSet::from(["canonical-b".into()])
            );
        }
    }
}

#[test]
fn caption_only_reentrant_update_preserves_target_and_can_toggle() {
    let app = Rc::new(App::memory());
    let source = app.scope().cage(source());
    let nested = app.clone();
    let actions = Rc::new(RefCell::new(vec![]));
    let a = actions.clone();
    let proposals = Rc::new(RefCell::new(vec![]));
    let p = proposals.clone();
    app.mount(
        chart(source)
            .on_legend_activate(move |e| {
                a.borrow_mut().push(e);
                source.revise(|scene| scene.legend[0].label = "Renamed <Å & é>".into());
                nested.flush().unwrap();
            })
            .on_hidden_change(move |set| p.borrow_mut().push(set)),
    )
    .unwrap();
    let original_row = nodes(&app, "kartis-legend-item")[0];
    app.dispatch(original_row, Event::Activate).unwrap();
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from(["canonical-a".into()])
    );
    assert_eq!(actions.borrow()[0].item.label, "Same caption");
    assert_eq!(nodes(&app, "kartis-legend-item")[0], original_row);
    assert_eq!(
        button_label(&app, original_row).as_deref(),
        Some("Show Renamed <Å & é>")
    );
}

#[test]
fn target_only_update_retires_old_row_owner_and_new_row_toggles_new_canonical_series() {
    let app = App::memory();
    let source = app.scope().cage(source());
    let proposals = Rc::new(RefCell::new(vec![]));
    let p = proposals.clone();
    app.mount(chart(source).on_hidden_change(move |set| p.borrow_mut().push(set)))
        .unwrap();
    let old = nodes(&app, "kartis-legend-item")[0];
    source.revise(|scene| {
        scene
            .legend_targets
            .insert("row-a".into(), LegendTarget::Series("canonical-b".into()));
    });
    app.flush().unwrap();
    let current = nodes(&app, "kartis-legend-item")[0];
    assert_ne!(old, current);
    assert!(app.tree().node(old).is_err());
    assert!(!app.dispatch(old, Event::Activate).unwrap());
    assert!(proposals.borrow().is_empty());
    app.dispatch(current, Event::Activate).unwrap();
    app.flush().unwrap();
    assert_eq!(
        proposals.borrow()[0],
        BTreeSet::from(["canonical-b".into()])
    );
    assert_pressed(&app, [false, true, false]);
}

#[test]
fn custom_and_measured_legend_contexts_keep_row_target_label_and_hidden_separate() {
    let app = App::memory();
    let source = app.scope().cage(source());
    let hidden = app.scope().cage(BTreeSet::<String>::new());
    let measured = Rc::new(RefCell::new(vec![]));
    let m = measured.clone();
    app.mount(
        chart(source)
            .hidden(hidden)
            .legend_layout(LegendLayoutOptions {
                bounds: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 320.,
                    height: 130.,
                },
                ..Default::default()
            })
            .text_measurer(move |text, _| {
                m.borrow_mut().push(text.to_owned());
                Ok(TextSize {
                    width: 55.,
                    height: 14.,
                })
            })
            .legend_item_renderer(|ctx| {
                glory::ui::text()
                    .class("target-item-context")
                    .content_fn(move || {
                        format!(
                            "{}->{:?}:{}:{}",
                            ctx.item.get().id,
                            ctx.target.get(),
                            ctx.hidden.get(),
                            ctx.label.get()
                        )
                    })
            })
            .legend_symbol_renderer(|ctx| {
                glory::ui::text()
                    .class("target-symbol-context")
                    .content_fn(move || format!("{:?}:{:?}", ctx.role.get(), ctx.target.get()))
            }),
    )
    .unwrap();
    assert_eq!(
        texts(&app, "target-item-context")[0],
        "row-a->Series(\"canonical-a\"):false:Same caption"
    );
    assert!(measured.borrow().iter().any(|s| s == "Hide Same caption"));
    hidden.revise(|set| {
        set.insert("canonical-a".into());
    });
    app.flush().unwrap();
    assert_eq!(
        texts(&app, "target-item-context")[1],
        "row-a-second->Series(\"canonical-a\"):true:Same caption"
    );
    assert!(measured.borrow().iter().any(|s| s == "Show Same caption"));
    source.revise(|scene| {
        scene
            .legend_targets
            .insert("row-a".into(), LegendTarget::Category(String::new()));
        scene.legend[0].label = "Category caption".into();
    });
    app.flush().unwrap();
    assert_eq!(
        texts(&app, "target-item-context")[0],
        "row-a->Category(\"\"):false:Category caption"
    );
    assert_eq!(
        texts(&app, "target-symbol-context")[0],
        "Category:Category(\"\")"
    );
    assert!(button_label(&app, nodes(&app, "kartis-legend-item")[0]).is_none());
}

#[cfg(feature = "raster")]
fn raster(app: &App, scene: Scene, actions: bool, publications: Rc<Cell<usize>>) -> RasterSurface {
    let installed = Rc::new(RefCell::new(None));
    let output = installed.clone();
    app.mount(glory::ui::widget(move |scope| {
        let count = publications.clone();
        let surface = RasterSurface::mount_published(
            scope,
            scene.clone(),
            render::raster::FontRegistry::new(),
            RasterSurfaceOptions {
                legend_actions: actions,
                ..Default::default()
            },
            move |image| {
                count.set(count.get() + 1);
                image.data_url()
            },
        )
        .unwrap();
        let widget = surface.widget();
        *output.borrow_mut() = Some(surface);
        widget.show_in(scope)
    }))
    .unwrap();
    installed.borrow_mut().take().unwrap()
}

#[cfg(feature = "raster")]
#[test]
fn raster_duplicate_rows_share_one_canonical_visibility_state_and_restore_bitmap() {
    let app = App::memory();
    let surface = raster(&app, source(), true, Rc::new(Cell::new(0)));
    let handle = surface.handle();
    let original = handle.frame().unwrap();
    let actions = Rc::new(RefCell::new(vec![]));
    let a = actions.clone();
    surface
        .on_legend_activate(move |e| a.borrow_mut().push(e))
        .unwrap();
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        handle.snapshot().unwrap().hidden,
        BTreeSet::from(["canonical-a".into()])
    );
    assert_eq!(actions.borrow()[0].item.id, "row-a");
    assert_eq!(
        actions.borrow()[0].resolved_target(),
        LegendTarget::Series("canonical-a".into())
    );
    assert_pressed(&app, [false, false, true]);
    assert_ne!(handle.frame().unwrap().rgba, original.rgba);
    app.dispatch(nodes(&app, "kartis-legend-item")[1], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert!(handle.snapshot().unwrap().hidden.is_empty());
    assert_eq!(handle.frame().unwrap().rgba, original.rgba);
    assert_pressed(&app, [true, true, true]);
    app.dispatch(nodes(&app, "kartis-legend-item")[2], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    assert_eq!(
        handle.snapshot().unwrap().hidden,
        BTreeSet::from(["canonical-b".into()])
    );
    assert_pressed(&app, [true, true, false]);
}

#[cfg(feature = "raster")]
#[test]
fn raster_empty_category_subscription_does_not_enable_actions_or_hide_any_series() {
    for actions in [false, true] {
        let app = App::memory();
        let mut scene = source();
        scene
            .legend_targets
            .insert("row-b".into(), LegendTarget::Category(String::new()));
        let publications = Rc::new(Cell::new(0));
        let surface = raster(&app, scene, actions, publications.clone());
        let seen = Rc::new(RefCell::new(vec![]));
        let s = seen.clone();
        surface
            .on_legend_activate(move |e| s.borrow_mut().push(e))
            .unwrap();
        let row = nodes(&app, "kartis-legend-item")[2];
        let before = surface.handle().snapshot().unwrap();
        let frame = surface.handle().frame().unwrap();
        let count = publications.get();
        assert_eq!(button_label(&app, row).is_some(), actions);
        assert_eq!(pressed(&app, row), None);
        app.dispatch(row, Event::Activate).unwrap();
        app.flush().unwrap();
        assert_eq!(seen.borrow().len(), usize::from(actions));
        if actions {
            assert_eq!(
                seen.borrow()[0].resolved_target(),
                LegendTarget::Category(String::new())
            );
        }
        assert_eq!(surface.handle().snapshot().unwrap(), before);
        assert!(Rc::ptr_eq(&frame, &surface.handle().frame().unwrap()));
        assert_eq!(publications.get(), count);
    }
}

#[cfg(feature = "raster")]
#[test]
fn raster_reentrant_retarget_rejects_stale_private_toggle_before_publisher_or_frame_commit() {
    let app = App::memory();
    let publications = Rc::new(Cell::new(0));
    let surface = raster(&app, source(), true, publications.clone());
    let handle = surface.handle();
    let replace = handle.clone();
    let accepted = Rc::new(RefCell::new(None));
    let saved = accepted.clone();
    let count = publications.clone();
    let observed = Rc::new(RefCell::new(vec![]));
    let obs = observed.clone();
    surface
        .on_legend_activate(move |e| {
            obs.borrow_mut().push(e);
            let mut next = source();
            next.legend_targets
                .insert("row-a".into(), LegendTarget::Series("canonical-b".into()));
            replace.set_scene(next).unwrap();
            *saved.borrow_mut() = Some((
                replace.snapshot().unwrap(),
                replace.frame().unwrap(),
                count.get(),
            ));
        })
        .unwrap();
    app.dispatch(nodes(&app, "kartis-legend-item")[0], Event::Activate)
        .unwrap();
    app.flush().unwrap();
    let (snapshot, frame, count_after_replacement) = accepted.borrow().as_ref().cloned().unwrap();
    assert_eq!(
        observed.borrow()[0].resolved_target(),
        LegendTarget::Series("canonical-a".into())
    );
    assert!(snapshot.hidden.is_empty());
    assert_eq!(handle.snapshot().unwrap(), snapshot);
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(handle.frame().unwrap().rgba, frame.rgba);
    assert_eq!(publications.get(), count_after_replacement);
}

#[cfg(feature = "raster")]
#[test]
fn raster_invalid_target_keeps_whole_frame_then_target_only_update_retires_old_row() {
    let app = App::memory();
    let publications = Rc::new(Cell::new(0));
    let surface = raster(&app, source(), true, publications.clone());
    let handle = surface.handle();
    let old_row = nodes(&app, "kartis-legend-item")[0];
    let before = handle.snapshot().unwrap();
    let frame = handle.frame().unwrap();
    let count = publications.get();
    let mut bad = source();
    bad.legend_targets.insert(
        "missing-row".into(),
        LegendTarget::Series("canonical-a".into()),
    );
    assert!(handle.set_scene(bad).is_err());
    assert_eq!(handle.snapshot().unwrap(), before);
    assert!(Rc::ptr_eq(&frame, &handle.frame().unwrap()));
    assert_eq!(publications.get(), count);
    assert_eq!(nodes(&app, "kartis-legend-item")[0], old_row);
    let mut next = source();
    next.legend_targets
        .insert("row-a".into(), LegendTarget::Series("canonical-b".into()));
    handle.set_scene(next).unwrap();
    app.flush().unwrap();
    let current = nodes(&app, "kartis-legend-item")[0];
    assert_ne!(current, old_row);
    assert!(app.tree().node(old_row).is_err());
    app.dispatch(current, Event::Activate).unwrap();
    app.flush().unwrap();
    assert_eq!(
        handle.snapshot().unwrap().hidden,
        BTreeSet::from(["canonical-b".into()])
    );
    assert_pressed(&app, [false, true, false]);
}
