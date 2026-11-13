use glory::{
    App,
    render::{Event, NodeId, Props},
    style::{Display, Length, Style},
};
use kartis::*;
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::Rc,
};

fn scene() -> Scene {
    let mut scene = ChartConfig {
        width: 480.,
        height: 360.,
        margin: Margin {
            top: 12.,
            right: 12.,
            bottom: 12.,
            left: 12.,
        },
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.labels.push(Label {
        x: 100.,
        y: 140.,
        size: 100.,
        text: "B".into(),
        rotation: 0.,
        color: Color(30, 90, 150, 255),
        anchor: TextAnchor::Start,
    });
    scene
}

fn owned_scene() -> Scene {
    let mut scene = scene();
    scene.label_series.insert(0, "alpha".into());
    let mut mark = Mark::new(
        "datum",
        Geometry::Circle {
            x: 350.,
            y: 100.,
            radius: 20.,
        },
        Color(30, 90, 150, 255),
    );
    mark.series = "alpha".into();
    scene.marks.push(mark);
    scene.legend.push(LegendItem {
        id: "alpha".into(),
        label: "Alpha".into(),
        color: Color(30, 90, 150, 255),
    });
    scene
}

fn profile() -> SemanticTextProfile {
    SemanticTextProfile {
        line_height: 1.25,
        revision: 0,
    }
}

// Caller-supplied protocol fixtures: Memory stores live descriptors, and does
// no host text layout. Real browser/Apace baseline and pixels are separate gates.
fn metrics() -> SemanticTextMetrics {
    SemanticTextMetrics {
        advance: 60.,
        line_height: 124.984375,
        baseline_from_top: 73.25,
        ink_bounds: Some(LayoutRect {
            x: -8.,
            y: -70.,
            width: 50.,
            height: 90.,
        }),
    }
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut stack = tree.roots().iter().rev().copied().collect::<Vec<_>>();
    let mut output = vec![];
    while let Some(id) = stack.pop() {
        let node = tree.node(id).unwrap();
        if class.is_empty()
            || node
                .descriptor
                .view()
                .classes
                .iter()
                .any(|name| name == class)
        {
            output.push(id);
        }
        stack.extend(node.children.iter().rev().copied());
    }
    output
}

fn style(app: &App, id: NodeId) -> Style {
    app.tree().node(id).unwrap().descriptor.view().style.clone()
}

fn text_children(app: &App, id: NodeId) -> Vec<NodeId> {
    let tree = app.tree();
    tree.node(id)
        .unwrap()
        .children
        .iter()
        .copied()
        .filter(|id| {
            matches!(
                tree.node(*id).unwrap().descriptor.view().props,
                Props::Text(_)
            )
        })
        .collect()
}

fn content(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Text(value) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("ordinary Text required")
    };
    value.content.clone()
}

fn snapshot(app: &App) -> Vec<(NodeId, String)> {
    nodes(app, "")
        .into_iter()
        .map(|id| {
            (
                id,
                format!("{:?}", app.tree().node(id).unwrap().descriptor.view()),
            )
        })
        .collect()
}

fn pressed(app: &App, id: NodeId) -> bool {
    app.tree()
        .node(id)
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
fn ordinary_text_uses_real_caller_baseline_and_three_advance_anchors() {
    let mut frame = scene();
    frame.labels = [TextAnchor::Start, TextAnchor::Middle, TextAnchor::End]
        .into_iter()
        .enumerate()
        .map(|(index, anchor)| Label {
            x: 100. + index as f64 * 120.,
            anchor,
            ..frame.labels[0].clone()
        })
        .collect();
    let app = App::memory();
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    app.mount(
        chart(frame).semantic_text_measurer(profile(), move |request| {
            counted.set(counted.get() + 1);
            assert_eq!(request.line_height, 125.);
            request.measured(metrics())
        }),
    )
    .unwrap();
    assert_eq!(calls.get(), 3);
    let labels = nodes(&app, "kartis-label");
    assert_eq!(labels.len(), 3);
    for (index, left) in [100., 190., 280.].into_iter().enumerate() {
        let outer = style(&app, labels[index]);
        assert_eq!(outer.left, Some(Length::Px(left)));
        assert_eq!(outer.top, Some(Length::Px(66.75)));
        assert_eq!(outer.width, Some(Length::Px(60.)));
        assert_eq!(outer.height, Some(Length::Px(124.984375)));
        let children = text_children(&app, labels[index]);
        assert_eq!(children.len(), 1);
        assert_eq!(content(&app, children[0]), "B");
        let text = style(&app, children[0]);
        assert_eq!(text.display, Some(Display::Block));
        assert_eq!(text.font_size, Some(100.));
        assert_eq!(text.line_height, Some(125.));
        assert_eq!(text.letter_spacing, Some(0.));
        text.validate().unwrap();
    }
    assert!(nodes(&app, "kartis-outline-glyph").is_empty());
}

#[test]
fn cache_reuses_position_anchor_and_paint_but_remeasures_each_resource_input() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let resource = app.scope().cage(profile());
    let calls = Rc::new(RefCell::new(vec![]));
    let captured = calls.clone();
    app.mount(
        chart(source).semantic_text_measurer(resource, move |request| {
            captured.borrow_mut().push(request.clone());
            request.measured(metrics())
        }),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label")[0];
    let text = text_children(&app, outer)[0];
    source.revise(|scene| {
        scene.labels[0].x += 7.;
        scene.labels[0].y += 5.;
        scene.labels[0].anchor = TextAnchor::End;
        scene.labels[0].color = Color(120, 40, 80, 128);
    });
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 1);
    assert_eq!(style(&app, outer).left, Some(Length::Px(47.)));
    assert_eq!(style(&app, outer).top, Some(Length::Px(71.75)));
    source.revise(|scene| scene.typography.default.family = Some(vec![FontFamily::Mono]));
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 2);
    source.revise(|scene| scene.typography.default.weight = Some(700.));
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 3);
    source.revise(|scene| scene.labels[0].text = "AB".into());
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 4);
    source.revise(|scene| scene.labels[0].size = 80.);
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 5);
    resource.revise(|profile| profile.line_height = 1.5);
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 6);
    resource.revise(|profile| profile.revision += 1);
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 7);
    assert_eq!(calls.borrow().last().unwrap().revision, 1);
    assert_eq!(calls.borrow().last().unwrap().line_height, 120.);
    assert_eq!(nodes(&app, "kartis-label"), vec![outer]);
    assert_eq!(text_children(&app, outer), vec![text]);
    assert_eq!(content(&app, text), "AB");
    assert_eq!(style(&app, text).font_weight, Some(700.));
    source.revise(|scene| scene.labels[0].color.3 = 0);
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 7);
    source.revise(|scene| scene.labels[0].color.3 = 255);
    app.flush().unwrap();
    assert_eq!(calls.borrow().len(), 8);
    assert_eq!(text_children(&app, outer), vec![text]);
}

#[test]
fn stale_revision_rejects_complete_scene_profile_and_layout_until_correct_retry() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let resource = app.scope().cage(profile());
    let seed = Rc::new(RefCell::new(None::<SemanticTextMeasurement>));
    let saved = seed.clone();
    let stale = Rc::new(Cell::new(false));
    let mode = stale.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 480.,
                    height: 360.,
                },
                ..Default::default()
            })
            .semantic_text_measurer(resource, move |request| {
                if mode.get() {
                    return Ok(saved.borrow().as_ref().unwrap().clone());
                }
                let measured = request.measured(metrics())?;
                *saved.borrow_mut() = Some(measured.clone());
                Ok(measured)
            })
            .on_reject(move |error| rejected.borrow_mut().push(error.0)),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label")[0];
    let text = text_children(&app, outer)[0];
    let accepted = snapshot(&app);
    stale.set(true);
    source.revise(|scene| {
        scene.width = 540.;
        scene.labels[0].x += 30.;
    });
    resource.revise(|profile| {
        profile.line_height = 1.5;
        profile.revision = 1;
    });
    app.flush().unwrap();
    assert!(!errors.borrow().is_empty());
    assert_eq!(snapshot(&app), accepted);
    stale.set(false);
    resource.revise(|profile| profile.revision = 2);
    app.flush().unwrap();
    assert_eq!(nodes(&app, "kartis-label"), vec![outer]);
    assert_eq!(text_children(&app, outer), vec![text]);
    assert_eq!(style(&app, outer).left, Some(Length::Px(130.)));
    assert_eq!(style(&app, text).line_height, Some(150.));
    assert_ne!(snapshot(&app), accepted);
    assert_eq!(seed.borrow().as_ref().unwrap().request().revision, 2);
}

#[test]
fn collision_translates_line_box_using_negative_bearing_and_descender_ink() {
    let app = App::memory();
    app.mount(
        chart(scene())
            .semantic_text_measurer(profile(), |request| request.measured(metrics()))
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 480.,
                    height: 360.,
                },
                forbidden: vec![LayoutRect {
                    x: 91.,
                    y: 69.,
                    width: 52.,
                    height: 92.,
                }],
                ..Default::default()
            }),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label")[0];
    let value = style(&app, outer);
    assert_eq!(value.display, Some(Display::Flex));
    assert_eq!(value.left, Some(Length::Px(100.)));
    // The upward option would leave the viewport. The accepted downward ink
    // shift is the ink height 90 + padding 2 + clearance 2; line-box baseline
    // stays 73.25 below its own top, independently of its measured height.
    assert_eq!(value.top, Some(Length::Px(160.75)));
    assert_eq!(value.width, Some(Length::Px(60.)));
    assert_eq!(value.height, Some(Length::Px(124.984375)));
    let world_baseline = 160.75 + 73.25;
    assert_eq!(world_baseline - 70., 164.);
    assert_eq!(world_baseline + 20., 254.);
    assert_ne!(value.top, Some(Length::Px(164.)));
    assert_ne!(value.width, Some(Length::Px(50.)));
}

#[test]
fn broad_obstacle_retains_the_original_line_box_alternative_for_shorter_ink() {
    // Preserve the browser B reproduction: these supplied host metrics have a
    // 100px line box, 72px ink and a bearing on both sides of the 60px advance.
    let mut frame = scene();
    frame.width = 360.;
    frame.labels[0].x = 210.;
    frame.labels[0].anchor = TextAnchor::Middle;
    let app = App::memory();
    app.mount(
        chart(frame)
            .semantic_text_measurer(SemanticTextProfile::default(), |request| {
                request.measured(SemanticTextMetrics {
                    advance: 60.,
                    line_height: 100.,
                    baseline_from_top: 95.,
                    ink_bounds: Some(LayoutRect {
                        x: -1.,
                        y: -71.,
                        width: 62.,
                        height: 72.,
                    }),
                })
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 360.,
                    height: 360.,
                },
                forbidden: vec![LayoutRect {
                    x: 175.,
                    y: 40.,
                    width: 70.,
                    height: 110.,
                }],
                ..Default::default()
            }),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label")[0];
    let child = text_children(&app, outer)[0];
    let placed = style(&app, outer);
    assert_eq!(placed.display, Some(Display::Flex));
    assert_eq!(placed.left, Some(Length::Px(180.)));
    // ±76px ink offsets fail: the upper ink leaves the viewport and the lower
    // padded ink still intersects the exact original obstacle. The host-line
    // alternative moves by 100 + padding 2 + clearance 2, preserving its box.
    assert_eq!(placed.top, Some(Length::Px(149.)));
    assert_eq!(placed.width, Some(Length::Px(60.)));
    assert_eq!(placed.height, Some(Length::Px(100.)));
    assert_eq!(content(&app, child), "B");
    assert_eq!(style(&app, child).font_size, Some(100.));
    assert_eq!(style(&app, child).line_height, Some(100.));
    assert_eq!(style(&app, child).display, Some(Display::Block));
    assert!(nodes(&app, "kartis-outline-glyph").is_empty());
}

#[test]
fn collision_preflight_rejects_valid_ink_whose_displaced_line_box_would_overflow() {
    let mut frame = scene();
    frame.labels[0].y = 999_800.;
    let app = App::memory();
    let source = app.scope().cage(frame);
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .semantic_text_measurer(profile(), move |request| {
                counted.set(counted.get() + 1);
                request.measured(metrics())
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 999_750.,
                    width: 480.,
                    height: 250.,
                },
                forbidden: vec![LayoutRect {
                    x: 91.,
                    y: 999_789.,
                    width: 52.,
                    height: 92.,
                }],
                ..Default::default()
            })
            .on_reject(move |error| rejected.borrow_mut().push(error.0)),
    )
    .unwrap();
    let before = snapshot(&app);
    source.revise(|scene| {
        scene.width = 540.;
        scene.labels[0].y = 999_860.;
    });
    app.flush().unwrap();
    // New ink's only collision-free location ends at 999974; its distinct
    // 124.984375-pixel line box would end past the one-million coordinate cap.
    assert!(
        errors
            .borrow()
            .iter()
            .any(|error| error.contains("rectangle"))
    );
    assert_eq!(snapshot(&app), before);
    assert_eq!(calls.get(), 1);
    source.revise(|scene| scene.labels[0].y = 999_800.);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert_ne!(snapshot(&app), before);
}

#[test]
fn controlled_show_failure_keeps_hidden_scene_and_measurement_state_atomic() {
    let app = App::memory();
    let source = app.scope().cage(owned_scene());
    let hidden = app.scope().cage(BTreeSet::from(["alpha".into()]));
    let resource = app.scope().cage(profile());
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let fail = Rc::new(Cell::new(false));
    let failing = fail.clone();
    let proposals = Rc::new(RefCell::new(vec![]));
    let proposed = proposals.clone();
    let errors = Rc::new(Cell::new(0));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .hidden(hidden)
            .semantic_text_measurer(resource, move |request| {
                counted.set(counted.get() + 1);
                if failing.get() {
                    return Err(ChartError("host font unavailable".into()));
                }
                request.measured(metrics())
            })
            .on_hidden_change(move |value| proposed.borrow_mut().push(value))
            .on_reject(move |_| rejected.set(rejected.get() + 1)),
    )
    .unwrap();
    let legend = nodes(&app, "kartis-legend-item")[0];
    let outer = nodes(&app, "kartis-label")[0];
    assert_eq!(calls.get(), 0);
    assert_eq!(style(&app, outer).display, Some(Display::None));
    app.dispatch(legend, Event::Activate).unwrap();
    assert_eq!(proposals.borrow().as_slice(), &[BTreeSet::new()]);
    assert!(!pressed(&app, legend));
    assert_eq!(calls.get(), 0);
    let before = snapshot(&app);
    fail.set(true);
    source.revise(|scene| scene.width = 540.);
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    assert!(errors.get() > 0);
    assert_eq!(snapshot(&app), before);
    fail.set(false);
    resource.revise(|profile| profile.revision += 1);
    app.flush().unwrap();
    assert_eq!(calls.get(), 2);
    assert!(pressed(&app, legend));
    assert_eq!(style(&app, outer).display, Some(Display::Flex));
    assert_eq!(nodes(&app, "kartis-label"), vec![outer]);
}

#[test]
fn uncontrolled_legend_hiding_skips_metrics_and_show_retries_without_losing_nodes() {
    let app = App::memory();
    let source = app.scope().cage(owned_scene());
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    let fail = Rc::new(Cell::new(false));
    let failing = fail.clone();
    let errors = Rc::new(Cell::new(0));
    let rejected = errors.clone();
    app.mount(
        chart(source)
            .semantic_text_measurer(profile(), move |request| {
                counted.set(counted.get() + 1);
                if failing.get() {
                    return Err(ChartError("measurement resource not ready".into()));
                }
                request.measured(metrics())
            })
            .on_reject(move |_| rejected.set(rejected.get() + 1)),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label")[0];
    let text = text_children(&app, outer)[0];
    let legend = nodes(&app, "kartis-legend-item")[0];
    assert_eq!(calls.get(), 1);
    app.dispatch(legend, Event::Activate).unwrap();
    assert!(!pressed(&app, legend));
    source.revise(|scene| scene.labels[0].x += 10.);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    let hidden = snapshot(&app);
    fail.set(true);
    app.dispatch(legend, Event::Activate).unwrap();
    assert!(errors.get() > 0);
    assert_eq!(snapshot(&app), hidden);
    assert_eq!(calls.get(), 2);
    fail.set(false);
    app.dispatch(legend, Event::Activate).unwrap();
    assert!(pressed(&app, legend));
    assert_eq!(calls.get(), 3);
    assert_eq!(nodes(&app, "kartis-label"), vec![outer]);
    assert_eq!(text_children(&app, outer), vec![text]);
    assert_eq!(style(&app, outer).left, Some(Length::Px(110.)));
}

#[test]
fn zero_ink_empty_text_and_whitespace_keep_positive_requested_css_line_height() {
    let mut frame = scene();
    frame.labels = ["", " "]
        .into_iter()
        .map(|text| Label {
            text: text.into(),
            ..frame.labels[0].clone()
        })
        .collect();
    let app = App::memory();
    app.mount(
        chart(frame)
            .semantic_text_measurer(profile(), |request| {
                request.measured(SemanticTextMetrics {
                    advance: 0.,
                    line_height: 0.,
                    baseline_from_top: 0.,
                    ink_bounds: None,
                })
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 480.,
                    height: 360.,
                },
                ..Default::default()
            }),
    )
    .unwrap();
    let labels = nodes(&app, "kartis-label");
    assert_eq!(labels.len(), 2);
    for (index, id) in labels.into_iter().enumerate() {
        assert_eq!(style(&app, id).height, Some(Length::Px(0.)));
        let child = text_children(&app, id)[0];
        assert_eq!(content(&app, child), if index == 0 { "" } else { " " });
        assert_eq!(style(&app, child).line_height, Some(125.));
        style(&app, child).validate().unwrap();
    }
}

#[test]
fn measurement_closures_release_after_cleanup_and_initial_rejection() {
    let app = App::memory();
    let source = app.scope().cage(scene());
    let resource = app.scope().cage(profile());
    let payload = Rc::new(());
    let weak = Rc::downgrade(&payload);
    let captured = payload.clone();
    let calls = Rc::new(Cell::new(0));
    let counted = calls.clone();
    app.mount(
        chart(source).semantic_text_measurer(resource, move |request| {
            let _keep = &captured;
            counted.set(counted.get() + 1);
            request.measured(metrics())
        }),
    )
    .unwrap();
    drop(payload);
    assert!(weak.upgrade().is_some());
    app.clear().unwrap();
    assert!(weak.upgrade().is_none());
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
    app.mount(
        chart(scene()).semantic_text_measurer(profile(), |request| request.measured(metrics())),
    )
    .unwrap();
    assert_eq!(nodes(&app, "kartis-label").len(), 1);
    app.clear().unwrap();
    source.revise(|scene| scene.labels[0].text = "AB".into());
    resource.revise(|profile| profile.revision += 1);
    app.flush().unwrap();
    assert_eq!(calls.get(), 1);
    let app = App::memory();
    let payload = Rc::new(());
    let weak = Rc::downgrade(&payload);
    let captured = payload.clone();
    let result = app.mount(chart(scene()).semantic_text_measurer(profile(), move |_| {
        let _keep = &captured;
        Err(ChartError("initial host metric failure".into()))
    }));
    assert!(result.is_err());
    drop(payload);
    assert!(weak.upgrade().is_none());
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(app.stats().handlers, 0);
}

#[cfg(feature = "raster")]
#[test]
fn rotated_outline_and_ordinary_fallback_use_separate_metrics_without_losing_cache() {
    use kartis::render::raster::FontRegistry;
    let mut frame = scene();
    frame.labels.push(Label {
        text: "j".into(),
        x: 300.,
        rotation: 25.,
        ..frame.labels[0].clone()
    });
    let mut fonts = FontRegistry::new();
    fonts
        .register(include_bytes!("fonts/ABeeZee-Regular.ttf").as_slice(), 0)
        .unwrap();
    let app = App::memory();
    let source = app.scope().cage(frame);
    let resource = app.scope().cage(profile());
    let calls = Rc::new(RefCell::new(vec![]));
    let captured = calls.clone();
    app.mount(
        chart(source)
            .outline_labels(fonts, OutlineLabelOptions::default())
            .semantic_text_measurer(resource, move |request| {
                captured.borrow_mut().push(request.text.clone());
                request.measured(metrics())
            }),
    )
    .unwrap();
    let outer = nodes(&app, "kartis-label");
    assert_eq!(outer.len(), 2);
    assert_eq!(calls.borrow().as_slice(), &["B"]);
    let glyphs = nodes(&app, "kartis-outline-glyph");
    assert_eq!(glyphs.len(), 1);
    assert_eq!(style(&app, outer[0]).top, Some(Length::Px(66.75)));
    assert_eq!(
        style(&app, text_children(&app, outer[0])[0]).line_height,
        Some(125.)
    );
    let rotated_safe_text = text_children(&app, outer[1])[0];
    assert_eq!(style(&app, rotated_safe_text).font_size, Some(1.));
    source.revise(|scene| scene.labels[1].x += 10.);
    app.flush().unwrap();
    assert_eq!(calls.borrow().as_slice(), &["B"]);
    assert_eq!(nodes(&app, "kartis-outline-glyph"), glyphs);
    resource.revise(|profile| profile.revision += 1);
    app.flush().unwrap();
    assert_eq!(calls.borrow().as_slice(), &["B", "B"]);
    source.revise(|scene| scene.labels[1].rotation = 0.);
    app.flush().unwrap();
    assert_eq!(calls.borrow().as_slice(), &["B", "B", "j"]);
    assert!(nodes(&app, "kartis-outline-glyph").is_empty());
    assert_eq!(text_children(&app, outer[1]), vec![rotated_safe_text]);
    assert_eq!(style(&app, rotated_safe_text).font_size, Some(100.));
    assert_eq!(style(&app, rotated_safe_text).line_height, Some(125.));
}
