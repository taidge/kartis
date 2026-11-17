use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, NodeId, Props};
use glory::{App, Widget};
use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;

#[path = "../examples/shared/stream_widgets.rs"]
mod model;

fn config() -> ChartConfig {
    ChartConfig {
        width: 320.,
        height: 220.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn options() -> StreamLayoutOptions {
    let mut o = StreamLayoutOptions {
        stack: StackOptions {
            order: StackOrder::Reverse,
            offset: StackOffset::Diverging,
        },
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        value_format: ValueFormat::Fixed(1),
        ..Default::default()
    };
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    o
}
fn input() -> Vec<BarSeries> {
    vec![
        BarSeries::new("first", vec![None, Some(0.), Some(-2.)]),
        BarSeries::new("second", vec![Some(2.), Some(1.), Some(3.)]),
    ]
}
fn categories() -> Vec<CategoryKey> {
    vec![1_f64.into(), "1".into(), "tail".into()]
}
fn layout() -> StreamLayout {
    stream_layout(&config(), &categories(), &input(), &options()).unwrap()
}
fn measured(_: &StreamDotWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-12., -8.],
        measured: TextSize {
            width: 24.,
            height: 16.,
        },
    }))
}

#[test]
fn borrowed_roles_share_original_nullable_series_and_preserve_default_layout() {
    for position in [
        StreamDotPosition::Start,
        StreamDotPosition::Center,
        StreamDotPosition::End,
    ] {
        for reverse in [false, true] {
            let mut o = options();
            o.dots.as_mut().unwrap().position = position;
            o.y_scale.reverse = reverse;
            let layout = stream_layout(&config(), &categories(), &input(), &o).unwrap();
            let before = layout.clone();
            let mut called = vec![];
            let packet = stream_dot_widgets(&layout, 4, |c| {
                assert!(std::ptr::eq(c.layout, &layout));
                assert!(std::ptr::eq(c.layer, &layout.layers[c.series_index]));
                assert!(std::ptr::eq(
                    c.sample,
                    &c.layer.samples[c.sample.source_index]
                ));
                assert_eq!(c.mark.id, c.sample.mark_id);
                assert_eq!(c.series, &input()[c.series_index]);
                let expected = match position {
                    StreamDotPosition::Start => c.sample.screen_y[0],
                    StreamDotPosition::Center => {
                        c.sample.screen_y[0] * 0.5 + c.sample.screen_y[1] * 0.5
                    }
                    StreamDotPosition::End => c.sample.screen_y[1],
                };
                assert_eq!(c.sample.position[1], expected);
                called.push((c.series_index, c.sample.source_index));
                measured(c)
            })
            .unwrap();
            assert_eq!(called, [(0, 1), (0, 2), (1, 0), (1, 1), (1, 2)]);
            assert_eq!(packet.layout.slots.len(), 5);
            assert_eq!(packet.layout.plot, Some(layout.plot));
            assert_eq!(layout.scene, before.scene);
            assert_eq!(layout.layers, before.layers);
            assert_eq!(layout.stack, before.stack);
            assert_eq!(
                layout.slice_interaction_binding(),
                before.slice_interaction_binding()
            );
            let values: Vec<_> = packet
                .data
                .values()
                .filter(|d| d.series_index == 0)
                .collect();
            assert!(Arc::ptr_eq(&values[0].series, &values[1].series));
            assert_eq!(values[0].series.values, vec![None, Some(0.), Some(-2.)]);
            assert!(packet.data.values().any(|d| d.sample.value == Some(0.)
                && d.sample.formatted_value.as_deref() == Some("0.0")));
            assert!(packet.data.values().all(|d| d.layer_id == d.series.id
                && d.layer_color == layout.layers[d.series_index].color
                && d.stack_index == 1 - d.series_index
                && d.plot == layout.plot));
            let missing = &layout.layers[0].samples[0];
            assert!(!packet.data.contains_key(&missing.mark_id));
            assert_eq!(
                layout
                    .slice_interaction_binding()
                    .sites
                    .iter()
                    .find(|s| s.mark_id == missing.mark_id)
                    .unwrap()
                    .value,
                None
            );
        }
    }
}

#[test]
fn source_layer_color_stays_distinct_from_custom_dot_paint() {
    let mut o = options();
    o.style.color = Some(Color(40, 70, 90, 255));
    let layout = stream_layout_with_dot_factory(&config(), &categories(), &input(), &o, |c| {
        let mut dot = c.inherit(Geometry::Circle {
            x: c.position[0],
            y: c.position[1],
            radius: 3.,
        });
        dot.fill = Some(Color(5, 11, 17, 255));
        Ok(dot)
    })
    .unwrap();
    let packet = stream_dot_widgets(&layout, 0, measured).unwrap();
    assert_eq!(packet.data.len(), 5);
    for (id, datum) in packet.data {
        assert_eq!(datum.layer_color, Color(40, 70, 90, 255));
        let mark = layout.scene.marks.iter().find(|m| m.id == id).unwrap();
        assert_eq!(mark.fill, Some(Color(5, 11, 17, 255)));
        assert_ne!(Some(datum.layer_color), mark.fill);
    }
}

#[test]
fn placeholders_are_not_components_but_zero_size_and_zero_opacity_are() {
    let mut o = options();
    o.dots = None;
    let disabled = stream_layout(&config(), &categories(), &input(), &o).unwrap();
    assert!(
        stream_dot_widgets(&disabled, 0, |_| panic!("disabled dots called user code"))
            .unwrap()
            .data
            .is_empty()
    );
    assert_eq!(disabled.slice_interaction_binding().sites.len(), 6);
    o.dots = Some(StreamDotOptions {
        size: 0.,
        ..Default::default()
    });
    o.hidden.insert("second".into());
    let zero = stream_layout(&config(), &categories(), &input(), &o).unwrap();
    assert_eq!(
        stream_dot_widgets(&zero, 0, measured).unwrap().data.len(),
        2
    );
    o.hidden.clear();
    let opacity = stream_layout_with_dot_factory(&config(), &categories(), &input(), &o, |c| {
        let mut dot = c.inherit(Geometry::Circle {
            x: c.position[0],
            y: c.position[1],
            radius: 4.,
        });
        dot.opacity = 0.;
        Ok(dot)
    });
    assert!(opacity.is_ok());
    assert_eq!(
        stream_dot_widgets(&opacity.unwrap(), 0, measured)
            .unwrap()
            .data
            .len(),
        5
    );
}

#[test]
fn complete_public_source_preflight_rejects_late_forgery_before_callbacks() {
    type Mutate = Box<dyn FnOnce(&mut StreamLayout)>;
    let changes: Vec<Mutate> = vec![
        Box::new(|l| l.layers[1].samples[2].value = Some(700.)),
        Box::new(|l| l.layers[1].samples[2].formatted_value = Some("forged".into())),
        Box::new(|l| l.layers[1].samples[2].source_index = 0),
        Box::new(|l| l.layers[1].samples[2].position[1] = f64::NAN),
        Box::new(|l| l.layers[1].samples[2].y0 += 1.),
        Box::new(|l| l.layers[1].samples[2].category = "fake".into()),
        Box::new(|l| l.layers[1].label = "fake".into()),
        Box::new(|l| l.layers[1].original_index = 0),
        Box::new(|l| l.layers[1].hidden = true),
        Box::new(|l| l.layers.reverse()),
        Box::new(|l| l.stack.layers[1].points[2].value = Some(999.)),
        Box::new(|l| l.slices[2].layer_indices.clear()),
        Box::new(|l| l.plot.width -= 1.),
        Box::new(|l| {
            l.y_scale = ResolvedContinuousScale::new(
                l.y_scale.source_domain(),
                l.y_scale.range(),
                &ContinuousScaleOptions {
                    clamp: true,
                    nice: ScaleNice::Off,
                    ..Default::default()
                },
            )
            .unwrap();
        }),
        Box::new(|l| l.scene.marks.last_mut().unwrap().series = "fake".into()),
        Box::new(|l| l.scene.marks.last_mut().unwrap().value = 999.),
        Box::new(|l| l.scene.marks.last_mut().unwrap().opacity = 0.4),
        Box::new(|l| {
            l.scene.marks.pop();
        }),
        Box::new(|l| {
            let mut mark = l.scene.marks.last().unwrap().clone();
            mark.id = "stream:orphan".into();
            l.scene.marks.push(mark);
        }),
    ];
    for change in changes {
        let mut layout = layout();
        change(&mut layout);
        let calls = Cell::new(0);
        assert!(
            stream_dot_widgets(&layout, 0, |_| {
                calls.set(calls.get() + 1);
                Ok(None)
            })
            .is_err()
        );
        assert_eq!(calls.get(), 0);
    }
    let mut layout = layout();
    layout.scene.marks.push(Mark::new(
        "decoration",
        Geometry::Circle {
            x: 1.,
            y: 1.,
            radius: 1.,
        },
        Color(0, 0, 0, 255),
    ));
    assert_eq!(
        stream_dot_widgets(&layout, 0, measured).unwrap().data.len(),
        5,
        "unrelated decorations do not forge Stream roles"
    );
}

#[test]
fn placement_errors_and_selected_slot_budget_return_no_partial_packet() {
    let layout = layout();
    let scene = layout.scene.clone();
    let mut calls = 0;
    assert!(
        stream_dot_widgets(&layout, 0, |c| {
            calls += 1;
            if calls == 4 {
                Err(ChartError("refused".into()))
            } else {
                measured(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 4);
    assert_eq!(layout.scene, scene);
    let categories = (0..130)
        .map(|v| CategoryKey::from(v as f64))
        .collect::<Vec<_>>();
    let large = stream_layout(
        &config(),
        &categories,
        &[BarSeries::new("large", vec![Some(1.); 130])],
        &options(),
    )
    .unwrap();
    let mut calls = 0;
    assert!(
        stream_dot_widgets(&large, 0, |c| {
            calls += 1;
            measured(c)
        })
        .unwrap_err()
        .0
        .contains("budget")
    );
    assert_eq!(calls, 129);
    assert_eq!(
        stream_dot_widgets(&large, 0, |c| {
            if c.sample.source_index < 128 {
                measured(c)
            } else {
                Ok(None)
            }
        })
        .unwrap()
        .data
        .len(),
        128
    );
    assert!(
        stream_dot_widgets(&large, 0, |_| Ok(None))
            .unwrap()
            .data
            .is_empty()
    );
}

#[test]
fn shared_profiles_and_actual_path_widgets_keep_the_typed_sample_anchor() {
    for (phase, expected) in [
        (0, 7),
        (1, 8),
        (2, 3),
        (3, 7),
        (4, 7),
        (5, 7),
        (6, 6),
        (7, 0),
    ] {
        for width in [320., 420.] {
            let source = model::source(phase, width, 0).unwrap();
            assert_eq!(source.composition.widgets.data.len(), expected);
            assert_eq!(source.composition.widgets.layout.plot, source.frame.plot);
            assert_eq!(
                source.frame.interaction.as_ref().unwrap().sites.len(),
                if phase == 2 { 4 } else { 8 }
            );
        }
    }
    let source = model::source(3, 320., 0).unwrap();
    let app = App::memory();
    let snapshots = Rc::new(RefCell::new(vec![]));
    let seen = snapshots.clone();
    app.mount(
        chart_layout(source.frame)
            .legend(false)
            .plot_widgets_composed(source.composition, move |context| {
                seen.borrow_mut().push(context.frame);
                Ok(glory::ui::button().label("Path dot"))
            }),
    )
    .unwrap();
    for frame in snapshots.borrow().iter() {
        frame.with(|frame| {
            let [x, y] = frame.data.sample.position;
            assert_eq!(frame.frame.rect.x, x - 18.);
            assert_eq!(frame.frame.rect.y, y - 12.);
            let Geometry::Path(commands) = &frame.frame.mark.geometry else {
                panic!("Path expected")
            };
            let glory::render::PathCommand::Move(first) = commands[0] else {
                panic!("Move expected")
            };
            assert_eq!(first.x, x + 10.);
            assert_ne!(frame.frame.rect.x, first.x - 18.);
        });
    }
    assert_eq!(snapshots.borrow().len(), 7);
    app.clear().unwrap();
}

fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut queue = tree.roots().to_vec();
    let mut found = vec![];
    while let Some(id) = queue.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|s| s == class) {
            found.push(id);
        }
        queue.extend(n.children.iter().copied());
    }
    found
}
fn label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(p) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("button expected")
    };
    p.label.clone()
}
fn tree(app: &App) -> String {
    let t = app.tree();
    let mut q = t.roots().to_vec();
    let mut out = vec![];
    while let Some(id) = q.pop() {
        let n = t.node(id).unwrap();
        out.push(format!("{id:?}:{n:?}"));
        q.extend(n.children.iter().copied());
    }
    format!("{:?}:{out:?}", t.roots())
}
fn child(
    ctx: PlotWidgetCompositionContext<StreamDotWidgetData>,
    cleanup: Rc<Cell<usize>>,
) -> Result<impl Widget> {
    if ctx.revision == 90 {
        return Err(ChartError("dot factory refused".into()));
    }
    let frame = ctx.frame;
    let revision = ctx.revision;
    Ok(glory::ui::widget(move |scope| {
        let cleanup = cleanup.clone();
        scope.on_cleanup(move || cleanup.set(cleanup.get() + 1))?;
        let count = scope.cage(0);
        let focus = scope.cage(false);
        glory::ui::button()
            .class(
                frame.with(|f| {
                    format!("dot-{}-{}", f.data.series_index, f.data.sample.source_index)
                }),
            )
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{}|raw={:?}|formatted={:?}|count={}|focus={}",
                        f.data.layer_label,
                        f.data.sample.value,
                        f.data.sample.formatted_value,
                        count.get(),
                        focus.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
            .on(EventKind::FocusChanged, move |event| {
                if let Event::FocusChanged(value) = event {
                    focus.revise(|v| *v = value);
                }
            })
            .show_in(scope)?;
        if revision == 91 {
            return Err(glory::render::Error::InvalidProps("dot build refused").into());
        }
        Ok(())
    }))
}

#[test]
fn actual_memory_children_retain_state_update_owned_data_and_reject_strict_candidates() {
    let app = App::memory();
    let source = app.scope().cage(model::source(0, 320., 0).unwrap());
    let frame = app.scope().bond(move || source.with(|s| s.frame.clone()));
    let packet = app
        .scope()
        .bond(move || source.with(|s| s.composition.clone()));
    let hidden = app.scope().cage(BTreeSet::new());
    let cleanup = Rc::new(Cell::new(0));
    let clean = cleanup.clone();
    let errors = Rc::new(RefCell::new(vec![]));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .legend(false)
            .tooltip_options(TooltipOptions {
                missing: MissingValue::Placeholder("missing".into()),
                ..Default::default()
            })
            .hidden(hidden)
            .on_reject(move |e| rejected.borrow_mut().push(e.0))
            .plot_widgets_composed(packet, move |ctx| child(ctx, clean.clone())),
    )
    .unwrap_or_else(|e| panic!("{e:?}: {:?}", errors.borrow()));
    assert!(nodes(&app, "dot-0-0").is_empty(), "None has no component");
    assert!(
        nodes(&app, "kartis-datum")
            .iter()
            .any(|id| label(&app, *id).ends_with(": missing"))
    );
    let id = nodes(&app, "dot-0-2")[0];
    app.dispatch(id, Event::Activate).unwrap();
    app.dispatch(id, Event::FocusChanged(true)).unwrap();
    source.revise(|s| *s = model::source(1, 360., 0).unwrap());
    app.flush().unwrap();
    assert_eq!(nodes(&app, "dot-0-2"), [id]);
    assert!(label(&app, id).contains("Some(5.0)"));
    assert!(label(&app, id).contains("count=1|focus=true"));
    assert_eq!(
        nodes(&app, "dot-0-0").len(),
        1,
        "None-to-zero mounts the newly eligible role"
    );
    hidden.revise(|v| {
        v.insert("north".into());
    });
    app.flush().unwrap();
    assert_eq!(nodes(&app, "dot-0-2"), [id]);
    assert!(app.tree().inert(id));
    hidden.revise(BTreeSet::clear);
    app.flush().unwrap();
    assert!(!app.tree().inert(id));
    for revision in [90, 91] {
        let before = tree(&app);
        source.revise(|s| *s = model::source(0, 320., revision).unwrap());
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(nodes(&app, "dot-0-2"), [id]);
    }
    assert_eq!(errors.borrow().len(), 2);
    source.revise(|s| *s = model::source(1, 320., 1).unwrap());
    app.flush().unwrap();
    let next = nodes(&app, "dot-0-2")[0];
    assert_ne!(next, id);
    assert!(label(&app, next).contains("count=0|focus=false"));
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
    assert_eq!(
        cleanup.get(),
        17,
        "eight old, one failed build, eight new children cleaned exactly once"
    );
}

#[cfg(feature = "raster")]
#[test]
fn real_png_publisher_failure_retains_typed_binding_data_options_controls_and_children() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    for dpr in [1., 2.] {
        let app = App::memory();
        let source = app.scope().cage((
            model::source(0, 320., 0).unwrap(),
            RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        ));
        let frame = app.scope().bond(move || source.with(|s| s.0.frame.clone()));
        let packet = app
            .scope()
            .bond(move || source.with(|s| s.0.composition.clone()));
        let options = app.scope().bond(move || source.with(|s| s.1.clone()));
        let fail = Rc::new(Cell::new(false));
        let refused = fail.clone();
        let cleanup = Rc::new(Cell::new(0));
        let clean = cleanup.clone();
        let errors = Rc::new(RefCell::new(vec![]));
        let rejected = errors.clone();
        app.mount(
            chart_layout(frame)
                .raster(FontRegistry::new(), options)
                .legend(false)
                .highlight(false)
                .tooltip_options(TooltipOptions {
                    missing: MissingValue::Placeholder("missing".into()),
                    ..Default::default()
                })
                .on_reject(move |e| rejected.borrow_mut().push(e.0))
                .raster_image_source(move |image| {
                    if refused.get() {
                        Err(ChartError("dot PNG refused".into()))
                    } else {
                        image.data_url()
                    }
                })
                .plot_widgets_composed(packet, move |ctx| child(ctx, clean.clone())),
        )
        .unwrap();
        let image = |app: &App| {
            let t = app.tree();
            let mut q = t.roots().to_vec();
            while let Some(id) = q.pop() {
                let n = t.node(id).unwrap();
                if let Props::Image(p) = n.descriptor.view().props {
                    return p.source.clone();
                }
                q.extend(n.children.iter().copied());
            }
            panic!("Image expected")
        };
        let id = nodes(&app, "dot-0-2")[0];
        app.dispatch(id, Event::Activate).unwrap();
        app.dispatch(id, Event::FocusChanged(true)).unwrap();
        let before = tree(&app);
        let png = image(&app);
        {
            use base64::Engine;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(png.split_once(',').unwrap().1)
                .unwrap();
            let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
                .read_info()
                .unwrap();
            assert_eq!(reader.info().width, (320. * dpr) as u32);
            assert_eq!(reader.info().height, (240. * dpr) as u32);
            let mut decoded = vec![0; reader.output_buffer_size().unwrap()];
            let info = reader.next_frame(&mut decoded).unwrap();
            assert_eq!(info.color_type, png::ColorType::Rgba);
            assert_eq!(&decoded[..4], &[255, 255, 255, 255]);
        }
        fail.set(true);
        source.revise(|s| {
            s.0 = model::source(1, 360., 0).unwrap();
            s.1.view = ZoomTransform::new(1.25, -12., -8.).unwrap();
        });
        app.flush().unwrap();
        assert_eq!(tree(&app), before);
        assert_eq!(image(&app), png);
        assert!(label(&app, id).contains("Some(3.0)"));
        assert!(nodes(&app, "dot-0-0").is_empty());
        assert_eq!(errors.borrow().len(), 1);
        fail.set(false);
        source.revise(|s| s.0 = model::source(1, 360., 0).unwrap());
        app.flush().unwrap();
        assert_eq!(nodes(&app, "dot-0-2"), [id]);
        assert!(label(&app, id).contains("Some(5.0)"));
        assert!(label(&app, id).contains("count=1|focus=true"));
        assert!(!nodes(&app, "dot-0-0").is_empty());
        assert_ne!(image(&app), png);
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
