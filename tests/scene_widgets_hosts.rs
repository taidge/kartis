//! Real Memory host protocol, checked native affine rejection and CPU source ink.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::*;

fn source(phase: u32, revision: u64, keep_other: bool) -> (Scene, SceneWidgetsComposition<u32>) {
    let mut scene = ChartConfig {
        width: 300. + f64::from(phase),
        height: 180.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "node",
        Geometry::Rect {
            x: 40.,
            y: 50.,
            width: 20.,
            height: 20.,
        },
        if phase == 0 {
            Color(20, 80, 180, 255)
        } else {
            Color(180, 50, 20, 255)
        },
    );
    mark.label = "original business label".into();
    mark.series = "owner".into();
    mark.value = f64::from(phase + 1);
    scene.marks.push(mark);
    scene.labels.push(Label {
        rotation: 0.,
        x: 120. + f64::from(phase),
        y: 70.,
        text: format!("source-label-{phase}"),
        color: Color(70, 20, 160, 255),
        size: 14.,
        anchor: TextAnchor::Start,
    });
    scene.label_series.insert(0, "owner".into());
    let label_index = if keep_other {
        scene.labels.push(Label {
            rotation: 0.,
            x: 180.,
            y: 70.,
            text: "retained-source-label".into(),
            color: Color(20, 40, 80, 255),
            size: 14.,
            anchor: TextAnchor::Start,
        });
        scene.label_series.insert(1, "other".into());
        if phase > 0 {
            scene.labels.reverse();
            scene.label_series = BTreeMap::from([(0, "other".into()), (1, "owner".into())]);
            1
        } else {
            0
        }
    } else {
        0
    };
    scene.draw_order = vec![DrawItem::Mark(0)];
    scene
        .draw_order
        .extend((0..scene.labels.len()).map(DrawItem::Label));
    scene.validate().unwrap();
    let slots = vec![
        SceneWidgetSlot {
            key: "component-mark".into(),
            target: SceneWidgetTarget::Mark("node".into()),
            revision,
            placement: PlotWidgetPlacement::Rect(LayoutRect {
                x: 70.,
                y: 100.,
                width: 40.,
                height: 20.,
            }),
        },
        SceneWidgetSlot {
            key: "component-label".into(),
            target: SceneWidgetTarget::Label(label_index),
            revision,
            placement: PlotWidgetPlacement::AtDatum {
                offset: [-10., -10.],
                measured: TextSize {
                    width: 40.,
                    height: 20.,
                },
            },
        },
    ];
    (
        scene,
        SceneWidgetsComposition {
            widgets: SceneWidgetsData {
                layout: SceneWidgetsLayout { plot: None, slots },
                data: BTreeMap::from([
                    ("component-mark".into(), phase + 10),
                    ("component-label".into(), phase + 10),
                ]),
            },
            replace_ink: BTreeSet::from(["component-label".into()]),
        },
    )
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if node.descriptor.view().classes.iter().any(|v| v == class) {
            found.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    found
}
fn text(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("actual Button required");
    };
    button.label.clone()
}
fn tree_snapshot(app: &App) -> String {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        result.push(format!("{id:?}:{node:?}"));
        pending.extend(node.children.iter().copied());
    }
    format!("{:?}:{result:?}", tree.roots())
}
fn source_texts(app: &App) -> Vec<String> {
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if let Props::Text(text) = node.descriptor.view().props {
            result.push(text.content.clone());
        }
        pending.extend(node.children.iter().copied());
    }
    result
}

fn differently_styled_label_projection() -> (Scene, SceneWidgetsComposition<u32>) {
    let (mut scene, composition) = source(0, 0, true);
    scene.labels[0].text = "REMOVE".into();
    scene.labels[1].text = "KEEP  white\tspace".into();
    scene.typography.labels.insert(
        1,
        FontOptions {
            family: Some(vec![FontFamily::Serif]),
            weight: Some(700.),
        },
    );
    scene
        .typography
        .label_whitespace
        .insert(1, LabelWhitespace::Preserve);
    scene.label_clips.insert(
        1,
        ClipPath {
            geometry: Geometry::Rect {
                x: 170.,
                y: 55.,
                width: 120.,
                height: 30.,
            },
            rule: ClipRule::NonZero,
        },
    );
    scene.validate().unwrap();
    (scene, composition)
}

fn assert_remaining_semantic_label(app: &App) {
    use glory::style::{Length, Overflow, WhiteSpace};
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if let Props::Text(text) = node.descriptor.view().props
            && text.content.starts_with("KEEP")
        {
            found.push(id);
        }
        pending.extend(node.children.iter().copied());
    }
    assert_eq!(found.len(), 1, "one actual remaining source Text node");
    let descriptor = tree.node(found[0]).unwrap().descriptor.view();
    let Props::Text(text) = descriptor.props else {
        unreachable!()
    };
    // Preserve keeps the two spaces and maps a control separator to one space;
    // the source Label and its measurement key retain the original tab.
    assert_eq!(text.content, "KEEP  white space");
    assert_eq!(descriptor.style.font_weight, Some(700.));
    assert_eq!(descriptor.style.font_family, Some(vec![FontFamily::Serif]));
    assert_eq!(descriptor.style.white_space, Some(WhiteSpace::Pre));
    drop(tree);
    let clips = nodes(app, "kartis-label-clip");
    assert_eq!(clips.len(), 1);
    let tree = app.tree();
    let style = tree.node(clips[0]).unwrap().descriptor.view().style;
    assert_eq!(style.left, Some(Length::Px(170.)));
    assert_eq!(style.top, Some(Length::Px(55.)));
    assert_eq!(style.width, Some(Length::Px(120.)));
    assert_eq!(style.height, Some(Length::Px(30.)));
    assert_eq!(style.overflow_x, Some(Overflow::Hidden));
    assert_eq!(style.overflow_y, Some(Overflow::Hidden));
}

#[test]
fn native_label_projection_keeps_remaining_font_whitespace_and_clip() {
    let (scene, composition) = differently_styled_label_projection();
    let app = App::memory();
    app.mount(
        chart(scene)
            .legend(false)
            .tooltip(false)
            .scene_widgets_composed(composition, |_| {
                Ok(glory::ui::button().label("real replacement"))
            }),
    )
    .unwrap();
    assert_remaining_semantic_label(&app);
}

#[test]
fn native_label_projection_keeps_remaining_measurement_identity() {
    let (scene, composition) = differently_styled_label_projection();
    let app = App::memory();
    let requests = Rc::new(RefCell::new(vec![]));
    let measured = requests.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .tooltip(false)
            .semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                measured.borrow_mut().push(request.clone());
                request.measured(SemanticTextMetrics {
                    advance: 100.,
                    line_height: 18.,
                    baseline_from_top: 14.,
                    ink_bounds: Some(LayoutRect {
                        x: 0.,
                        y: -12.,
                        width: 80.,
                        height: 16.,
                    }),
                })
            })
            .scene_widgets_composed(composition, |_| {
                Ok(glory::ui::button().label("real replacement"))
            }),
    )
    .unwrap();
    assert_remaining_semantic_label(&app);
    let requests = requests.borrow();
    assert_eq!(requests.len(), 1, "removed source Label is not measured");
    assert_eq!(requests[0].text, "KEEP  white\tspace");
    assert_eq!(requests[0].font.weight, 700.);
    assert_eq!(requests[0].font.family, vec![FontFamily::Serif]);
    assert_eq!(requests[0].whitespace, LabelWhitespace::Preserve);
}

#[test]
fn native_label_projection_remaps_original_collision_priority_to_actual_remaining_text() {
    use glory::style::Display;
    let (mut scene, composition) = source(0, 0, true);
    scene.labels[0].text = "REMOVE".into();
    let low = &mut scene.labels[1];
    low.text = "LOW".into();
    low.x = 180.;
    low.y = 70.;
    low.size = 10.;
    let mut high = low.clone();
    high.text = "HIGH".into();
    scene.labels.push(high);
    scene.label_series.insert(2, "other-high".into());
    scene.draw_order.push(DrawItem::Label(2));
    scene.validate().unwrap();
    let requests = Rc::new(RefCell::new(vec![]));
    let measured = requests.clone();
    let app = App::memory();
    app.mount(
        chart(scene)
            .legend(false)
            .tooltip(false)
            .text_measurer(move |text, _| {
                measured.borrow_mut().push(text.to_owned());
                Ok(TextSize {
                    width: 60.,
                    height: 10.,
                })
            })
            .label_collision(LabelCollisionOptions {
                // Both labels fit here only at their original position;
                // every alternative vertical displacement is outside.
                viewport: LayoutRect {
                    x: 180.,
                    y: 62.,
                    width: 60.,
                    height: 10.,
                },
                padding: 0.,
                ..Default::default()
            })
            .label_priorities(BTreeMap::from([(2, 100)]))
            .scene_widgets_composed(composition, |_| {
                Ok(glory::ui::button().label("real replacement"))
            }),
    )
    .unwrap();
    assert_eq!(&*requests.borrow(), &["LOW", "HIGH"]);
    let tree = app.tree();
    let mut pending = tree.roots().to_vec();
    let mut visible = BTreeMap::new();
    while let Some(id) = pending.pop() {
        let node = tree.node(id).unwrap();
        if let Props::Text(text) = node.descriptor.view().props
            && matches!(text.content.as_str(), "LOW" | "HIGH")
        {
            let parent = tree.node(node.parent.unwrap()).unwrap();
            visible.insert(text.content.clone(), parent.descriptor.view().style.display);
        }
        pending.extend(node.children.iter().copied());
    }
    assert_eq!(visible.len(), 2);
    assert_eq!(visible["LOW"], Some(Display::None));
    assert_ne!(visible["HIGH"], Some(Display::None));
}

fn child(
    context: SceneWidgetCompositionContext<u32>,
    calls: Rc<Cell<usize>>,
    drops: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    calls.set(calls.get() + 1);
    if context.key == "component-label" && context.revision == 4 {
        return Err(ChartError("public mixed Label factory refused".into()));
    }
    let reject = context.key == "component-label" && context.revision == 5;
    let frame = context.frame;
    let key = context.key;
    Ok(glory::ui::widget(move |scope| {
        let drops = drops.clone();
        scope.on_cleanup(move || drops.set(drops.get() + 1))?;
        let count = scope.cage(0u32);
        let focused = scope.cage(false);
        glory::ui::button()
            .class(key.clone())
            .label(scope.bond(move || {
                frame.with(|value| {
                    let source = match &value.frame {
                        SceneWidgetFrame::Mark(mark) => format!("Mark:{}", mark.mark.value),
                        SceneWidgetFrame::Label(label) => format!(
                            "Label:{}:{}:{}",
                            label.label_index, label.label.text, label.label.x
                        ),
                    };
                    format!(
                        "{source};data={};count={};focus={}",
                        value.data,
                        count.get(),
                        focused.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .on(EventKind::FocusChanged, move |event| {
                if let Event::FocusChanged(value) = event {
                    focused.revise(|v| *v = value);
                }
            })
            .show_in(scope)?;
        if reject {
            return Err(
                glory::render::Error::InvalidProps("public mixed Label build refused").into(),
            );
        }
        Ok(())
    }))
}

#[test]
fn real_memory_mixed_children_follow_label_index_and_keep_business_controls_on_rejection() {
    let app = App::memory();
    let input = app.scope().cage(source(0, 0, true));
    let scene = app.scope().bond(move || input.with(|s| s.0.clone()));
    let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
    let calls = Rc::new(Cell::new(0));
    let drops = Rc::new(Cell::new(0));
    let called = calls.clone();
    let disposed = drops.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .tooltip(false)
            .on_reject(move |error| rejected.borrow_mut().push(error.0))
            .scene_widgets_composed(packet, move |context| {
                child(context, called.clone(), disposed.clone())
            }),
    )
    .unwrap();
    let label = nodes(&app, "component-label")[0];
    let mark = nodes(&app, "component-mark")[0];
    let controls = nodes(&app, "kartis-datum");
    assert_eq!(
        controls.len(),
        1,
        "Label role creates no extra business datum"
    );
    let texts = source_texts(&app);
    assert!(texts.iter().any(|s| s == "retained-source-label"));
    assert!(!texts.iter().any(|s| s == "source-label-0"));
    app.dispatch(label, Event::Activate).unwrap();
    app.dispatch(label, Event::FocusChanged(true)).unwrap();
    input.revise(|s| *s = source(1, 0, true));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "component-label"), [label]);
    assert_eq!(nodes(&app, "component-mark"), [mark]);
    assert_eq!(nodes(&app, "kartis-datum"), controls);
    assert_eq!(
        text(&app, label),
        "Label:1:source-label-1:121;data=11;count=1;focus=true"
    );
    assert_eq!(text(&app, mark), "Mark:2;data=11;count=0;focus=false");
    assert_eq!(calls.get(), 2);
    let old = tree_snapshot(&app);
    for revision in [4, 5] {
        input.revise(|s| *s = source(2, revision, true));
        app.flush().unwrap();
        assert_eq!(tree_snapshot(&app), old);
    }
    assert_eq!(errors.borrow().len(), 2);
    assert_eq!(
        drops.get(),
        3,
        "only the real failed candidate scopes retire"
    );
    let mut missing = source(3, 0, true);
    missing.1.widgets.data.remove("component-label");
    input.revise(|s| *s = missing);
    app.flush().unwrap();
    assert_eq!(tree_snapshot(&app), old);
    assert_eq!(errors.borrow().len(), 3);
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(drops.get(), 5);
}

#[test]
fn native_automatic_rotated_label_is_rejected_before_any_factory() {
    let app = App::memory();
    let (mut scene, composition) = source(0, 0, false);
    scene.labels[0].rotation = 35.;
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    let result = app.mount(chart(scene).legend(false).scene_widgets_composed(
        composition,
        move |_| {
            called.set(called.get() + 1);
            Ok(glory::ui::button().label("must not be built"))
        },
    ));
    assert!(result.is_err());
    assert_eq!(calls.get(), 0);
    app.clear().unwrap();
}

#[test]
fn legacy_mark_counteroffset_is_checked_by_final_rectangle_not_an_added_offset_cap() {
    let app = App::memory();
    let mut scene = ChartConfig {
        width: 240.,
        height: 160.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    scene.marks.push(Mark::new(
        "legacy",
        Geometry::Circle {
            x: -999_999.,
            y: 50.,
            radius: 1.,
        },
        Color(20, 40, 80, 255),
    ));
    scene.validate().unwrap();
    let packet = PlotWidgetsComposition {
        widgets: PlotWidgetsData {
            layout: PlotWidgetsLayout {
                plot: None,
                slots: vec![PlotWidgetSlot {
                    mark_id: "legacy".into(),
                    revision: 0,
                    placement: PlotWidgetPlacement::AtDatum {
                        offset: [1_000_001., 0.],
                        measured: TextSize {
                            width: 20.,
                            height: 10.,
                        },
                    },
                }],
            },
            data: BTreeMap::from([("legacy".into(), 7u32)]),
        },
        replace_ink: BTreeSet::new(),
    };
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    app.mount(
        chart(scene)
            .legend(false)
            .plot_widgets_composed(packet, move |context| {
                called.set(called.get() + 1);
                let value = context.frame.get();
                assert_eq!(value.frame.rect.x, 2.);
                assert_eq!(value.frame.rect.width, 20.);
                assert_eq!(value.data, 7);
                Ok(glory::ui::button()
                    .class("legacy-counteroffset")
                    .label("admitted"))
            }),
    )
    .unwrap();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        text(&app, nodes(&app, "legacy-counteroffset")[0]),
        "admitted"
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_dpr_label_ink_projection_and_publisher_rejection_keep_complete_old_mixed_frame() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let input = app.scope().cage(source(0, 0, false));
        let scene = app.scope().bond(move || input.with(|s| s.0.clone()));
        let packet = app.scope().bond(move || input.with(|s| s.1.clone()));
        let denied = Rc::new(Cell::new(false));
        let deny = denied.clone();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        app.mount(
            chart(scene)
                .legend(false)
                .tooltip(false)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |png| {
                    if deny.get() {
                        Err(ChartError("Scene Label PNG refused".into()))
                    } else {
                        png.data_url()
                    }
                })
                .on_reject(move |error| rejected.borrow_mut().push(error.0))
                .scene_widgets_composed(packet, move |context| {
                    child(context, called.clone(), Rc::new(Cell::new(0)))
                }),
        )
        .unwrap();
        let label = nodes(&app, "component-label")[0];
        app.dispatch(label, Event::Activate).unwrap();
        app.dispatch(label, Event::FocusChanged(true)).unwrap();
        let image_id = nodes(&app, "kartis-raster-image")[0];
        let image = || {
            let tree = app.tree();
            let Props::Image(image) = tree.node(image_id).unwrap().descriptor.view().props else {
                panic!("actual Image")
            };
            image.source.clone()
        };
        let old_png = image();
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(old_png.split_once(',').unwrap().1)
            .unwrap();
        let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut rgba = vec![0; decoder.output_buffer_size().unwrap()];
        let info = decoder.next_frame(&mut rgba).unwrap();
        assert_eq!(
            [info.width, info.height],
            [(300. * dpr) as u32, (180. * dpr) as u32]
        );
        let offset = 4 * ((60. * dpr) as usize * info.width as usize + (50. * dpr) as usize);
        assert_eq!(
            &rgba[offset..offset + 4],
            &[20, 80, 180, 255],
            "the node's source paint remains"
        );
        let old_tree = tree_snapshot(&app);
        let old_label = text(&app, label);
        denied.set(true);
        input.revise(|s| *s = source(1, 0, false));
        app.flush().unwrap();
        assert_eq!(image(), old_png);
        assert_eq!(tree_snapshot(&app), old_tree);
        assert_eq!(text(&app, label), old_label);
        assert_eq!(errors.borrow().len(), 1);
        denied.set(false);
        input.revise(|s| *s = source(2, 0, false));
        app.flush().unwrap();
        assert_ne!(image(), old_png);
        assert_eq!(nodes(&app, "component-label"), [label]);
        assert!(text(&app, label).ends_with("data=12;count=1;focus=true"));
        assert_eq!(calls.get(), 2);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
