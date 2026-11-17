//! Independent source-provenance and real Glory child acceptance checks.
//! Memory events are explicit port dispatch, not physical host transport.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use glory::render::{Event, EventKind, GraphicPoint as P, NodeId, PathCommand as C, Props};
use glory::{App, Widget};
use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 260.,
        height: 180.,
        margin: Margin {
            left: 31.,
            top: 23.,
            right: 19.,
            bottom: 17.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<BarSeries> {
    vec![
        BarSeries::new("first", vec![Some(4.), None, Some(0.)]),
        BarSeries::new("second", vec![Some(3.), Some(2.), Some(5.)]),
        BarSeries::new("hidden", vec![Some(9.), Some(8.), Some(7.)]),
    ]
}
fn options() -> StreamLayoutOptions {
    let mut o = StreamLayoutOptions {
        stack: StackOptions {
            order: StackOrder::Reverse,
            offset: StackOffset::Zero,
        },
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        ..Default::default()
    };
    o.hidden.insert("hidden".into());
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    o
}
fn build(o: &StreamLayoutOptions) -> StreamLayout {
    stream_layout(
        &config(),
        &["one".into(), "two".into(), "three".into()],
        &raw(),
        o,
    )
    .unwrap()
}
fn placement(_: &StreamDotWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [1., -3.],
        measured: TextSize {
            width: 8.,
            height: 6.,
        },
    }))
}

#[test]
fn eligible_callbacks_keep_raw_nullable_source_and_original_indices_under_reverse_stack() {
    let layout = build(&options());
    let original = layout.scene.clone();
    let mut calls = vec![];
    let packet = stream_dot_widgets(&layout, 17, |ctx| {
        assert_eq!(ctx.series, &raw()[ctx.series_index]);
        assert_eq!(ctx.series_index, ctx.layer.original_index);
        assert_eq!(ctx.sample.value, ctx.series.values[ctx.sample.source_index]);
        assert_eq!(ctx.mark.id, ctx.sample.mark_id);
        calls.push((ctx.series_index, ctx.sample.source_index));
        placement(ctx)
    })
    .unwrap();
    assert_eq!(calls, [(0, 0), (0, 2), (1, 0), (1, 1), (1, 2)]);
    assert_ne!(
        layout.layers[0].original_index,
        layout.layers[0].stack_index
    );
    assert_eq!(packet.layout.slots.len(), 5);
    assert!(packet.layout.slots.iter().all(|s| s.revision == 17));
    let first = packet
        .data
        .values()
        .filter(|d| d.series_index == 0)
        .collect::<Vec<_>>();
    assert_eq!(first.len(), 2);
    assert!(Arc::ptr_eq(&first[0].series, &first[1].series));
    assert_eq!(first[0].series.values, [Some(4.), None, Some(0.)]);
    assert!(packet.data.values().any(|d| d.sample.value == Some(0.)));
    assert_eq!(layout.scene, original);
}

#[test]
fn disabled_null_size_zero_and_transparent_custom_roles_have_distinct_eligibility() {
    let mut o = options();
    o.dots = None;
    let disabled = build(&o);
    let mut calls = 0;
    let packet = stream_dot_widgets(&disabled, 0, |_| {
        calls += 1;
        Ok(None)
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert!(packet.data.is_empty());
    assert!(
        disabled.layers[0].samples.iter().all(|s| disabled
            .scene
            .marks
            .iter()
            .any(|m| m.id == s.mark_id))
    );
    for position in [
        StreamDotPosition::Start,
        StreamDotPosition::Center,
        StreamDotPosition::End,
    ] {
        o.dots = Some(StreamDotOptions {
            size: 0.,
            position,
            ..Default::default()
        });
        let layout = build(&o);
        let mut count = 0;
        stream_dot_widgets(&layout, 0, |ctx| {
            count += 1;
            let expected = match position {
                StreamDotPosition::Start => ctx.sample.screen_y[0],
                StreamDotPosition::Center => {
                    0.5 * ctx.sample.screen_y[0] + 0.5 * ctx.sample.screen_y[1]
                }
                StreamDotPosition::End => ctx.sample.screen_y[1],
            };
            assert_eq!(ctx.sample.position[1], expected);
            assert!(matches!(
                ctx.mark.geometry,
                Geometry::Circle { radius: 0., .. }
            ));
            Ok(None)
        })
        .unwrap();
        assert_eq!(count, 5);
    }
    let layout = stream_layout_with_dot_factory(
        &config(),
        &["one".into(), "two".into(), "three".into()],
        &raw(),
        &options(),
        |ctx| {
            let mut dot = ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 3.,
            });
            dot.opacity = 0.;
            dot.fill = Some(Color(10, 20, 30, 0));
            Ok(dot)
        },
    )
    .unwrap();
    let mut count = 0;
    stream_dot_widgets(&layout, 0, |ctx| {
        count += 1;
        assert_eq!(ctx.mark.opacity, 0.);
        placement(ctx)
    })
    .unwrap();
    assert_eq!(count, 5);
}

#[test]
fn public_source_forgeries_and_late_invalid_marks_fail_before_any_callback() {
    type Mutate = fn(&mut StreamLayout);
    let mutations: &[(&str, Mutate)] = &[
        ("plot", |l| l.plot.x += 1.),
        ("stack order", |l| l.stack.order.reverse()),
        ("stack baseline", |l| l.stack.baseline[2] += 1.),
        ("stack domain", |l| l.stack.domain[1] += 1.),
        ("stack point", |l| l.stack.layers[2].points[2].column = 1),
        ("x scale", |l| {
            let mut o = options();
            o.x_scale.reverse = true;
            l.x_scale = build(&o).x_scale;
        }),
        ("layer identity", |l| l.layers[2].id = "forged".into()),
        ("label", |l| l.layers[1].label = "forged".into()),
        ("original index", |l| l.layers[1].original_index = 0),
        ("stack index", |l| l.layers[1].stack_index = 0),
        ("hidden", |l| l.layers[2].hidden = false),
        ("layer color", |l| l.layers[1].color = Color(1, 2, 3, 255)),
        ("area owner", |l| l.layers[1].area_mark_id = None),
        ("sample owner", |l| {
            l.layers[1].samples[2].mark_id = l.layers[0].samples[0].mark_id.clone()
        }),
        ("sample index", |l| l.layers[1].samples[2].source_index = 0),
        ("category", |l| {
            l.layers[1].samples[2].category = "forged".into()
        }),
        ("raw value", |l| l.layers[1].samples[2].value = None),
        ("signed zero source", |l| {
            l.layers[0].samples[2].value = Some(-0.)
        }),
        ("color scale", |l| {
            l.colors = OrdinalColorOptions::default()
                .resolve(["forged".into()])
                .unwrap();
        }),
        ("formatted", |l| {
            l.layers[1].samples[2].formatted_value = Some("forged".into())
        }),
        ("stack y0", |l| l.layers[1].samples[2].y0 += 1.),
        ("stack y1", |l| l.layers[1].samples[2].y1 += 1.),
        ("screen boundary", |l| {
            l.layers[1].samples[2].screen_y[0] += 1.
        }),
        ("position", |l| l.layers[1].samples[2].position[0] += 1.),
        ("slice source", |l| l.slices[2].source_index = 0),
        ("slice category", |l| l.slices[2].category = "forged".into()),
        ("slice x", |l| l.slices[2].x += 1.),
        ("slice layers", |l| l.slices[2].layer_indices.reverse()),
        ("late paint", |l| {
            l.scene.marks.last_mut().unwrap().opacity = 0.5
        }),
        ("late geometry", |l| {
            l.scene.marks.last_mut().unwrap().geometry = Geometry::Circle {
                x: 2.,
                y: 3.,
                radius: 1.,
            }
        }),
        ("late invalid", |l| {
            l.scene.marks.last_mut().unwrap().value = f64::NAN
        }),
        ("removed mark", |l| {
            l.scene.marks.pop();
        }),
    ];
    for (name, mutate) in mutations {
        let mut layout = build(&options());
        mutate(&mut layout);
        let mut calls = 0;
        let result = stream_dot_widgets(&layout, 0, |_| {
            calls += 1;
            Ok(None)
        });
        assert!(result.is_err(), "{name} accepted");
        assert_eq!(calls, 0, "{name} invoked user code before full validation");
    }
}

#[test]
fn callback_error_stops_without_publishing_partial_packet_and_none_skips_only_selection() {
    let layout = build(&options());
    let mut calls = 0;
    let result = stream_dot_widgets(&layout, 0, |ctx| {
        calls += 1;
        if calls == 3 {
            Err(ChartError("placement refuses third role".into()))
        } else {
            placement(ctx)
        }
    });
    assert!(result.is_err());
    assert_eq!(calls, 3);
    let packet = stream_dot_widgets(&layout, 0, |ctx| {
        if ctx.sample.value == Some(0.) {
            placement(ctx)
        } else {
            Ok(None)
        }
    })
    .unwrap();
    assert_eq!(packet.data.len(), 1);
    assert_eq!(packet.data.values().next().unwrap().sample.source_index, 2);
}

#[test]
fn replacing_value_scale_with_same_stops_but_different_clamp_is_rejected() {
    let mut layout = build(&options());
    let original = layout.y_scale.clone();
    layout.y_scale = ResolvedContinuousScale::new(
        original.source_domain(),
        original.range(),
        &ContinuousScaleOptions {
            clamp: true,
            nice: ScaleNice::Off,
            ..Default::default()
        },
    )
    .unwrap();
    // A scale with the same visible knots can still change extrapolation.
    // Preserve display endpoints, including any nice expansion, for this probe.
    if original.display_domain() != original.source_domain() {
        layout.y_scale = ResolvedContinuousScale::new(
            original.source_domain(),
            original.range(),
            &ContinuousScaleOptions {
                clamp: true,
                nice: options().y_scale.nice,
                ..Default::default()
            },
        )
        .unwrap();
    }
    assert_eq!(
        layout.y_scale.source_domain_stops(),
        original.source_domain_stops()
    );
    assert_eq!(
        layout.y_scale.display_domain_stops(),
        original.display_domain_stops()
    );
    assert_eq!(layout.y_scale.range_stops(), original.range_stops());
    let mut calls = 0;
    assert!(
        stream_dot_widgets(&layout, 0, |_| {
            calls += 1;
            Ok(None)
        })
        .is_err(),
        "changed clamp configuration was accepted"
    );
    assert_eq!(calls, 0);
}

fn path_layout() -> StreamLayout {
    stream_layout_with_dot_factory(
        &config(),
        &["one".into(), "two".into(), "three".into()],
        &raw(),
        &options(),
        |ctx| {
            let [x, y] = ctx.position;
            Ok(ctx.inherit(Geometry::Path(vec![
                C::Move(P {
                    x: x + 8.,
                    y: y - 3.,
                }),
                C::Line(P {
                    x: x + 14.,
                    y: y - 3.,
                }),
                C::Line(P {
                    x: x + 14.,
                    y: y + 3.,
                }),
                C::Line(P {
                    x: x + 8.,
                    y: y + 3.,
                }),
                C::Close,
            ])))
        },
    )
    .unwrap()
}
fn nodes(app: &App, class: &str) -> Vec<NodeId> {
    let tree = app.tree();
    let mut todo = tree.roots().to_vec();
    let mut result = vec![];
    while let Some(id) = todo.pop() {
        let n = tree.node(id).unwrap();
        if n.descriptor.view().classes.iter().any(|c| c == class) {
            result.push(id);
        }
        todo.extend(n.children.iter().copied());
    }
    result
}
fn button_label(app: &App, id: NodeId) -> String {
    let tree = app.tree();
    let Props::Button(button) = tree.node(id).unwrap().descriptor.view().props else {
        panic!("real button required")
    };
    button.label.clone()
}
fn child(ctx: TypedPlotWidgetContext<StreamDotWidgetData>) -> impl Widget {
    glory::ui::widget(move |scope| {
        let frame = ctx.frame;
        let count = scope.cage(0usize);
        glory::ui::button()
            .class("audit-stream-dot")
            .label(scope.bond(move || {
                frame.with(|f| {
                    format!(
                        "{}:{}:{:?}:rect={:?}:count={}",
                        f.data.layer_id,
                        f.data.sample.source_index,
                        f.data.sample.value,
                        f.frame.rect,
                        count.get()
                    )
                })
            }))
            .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
            .show_in(scope)
    })
}
fn selected(layout: &StreamLayout) -> PlotWidgetsData<StreamDotWidgetData> {
    stream_dot_widgets(layout, 0, |ctx| {
        if ctx.series_index == 1 && ctx.sample.source_index == 1 {
            placement(ctx)
        } else {
            Ok(None)
        }
    })
    .unwrap()
}

#[test]
fn typed_path_uses_original_data_anchor_not_offset_ink_and_rejects_binding_only_forgery() {
    let layout = path_layout();
    let packet = selected(&layout);
    let id = packet.layout.slots[0].mark_id.clone();
    let expected = packet.data[&id].sample.position;
    let mut source = layout.into_scene_interaction().unwrap();
    source.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
    source.validate().unwrap();
    let app = App::memory();
    let input = app.scope().cage((source.clone(), packet.clone()));
    let frame = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    let observations = Rc::new(RefCell::new(vec![]));
    let seen = observations.clone();
    let errors = Rc::new(Cell::new(0));
    let rejected = errors.clone();
    app.mount(
        chart_layout(frame)
            .on_reject(move |e| {
                eprintln!("Stream Memory rejection: {e}");
                rejected.set(rejected.get() + 1);
            })
            .plot_widgets_with_data(slots, move |ctx| {
                seen.borrow_mut().push(ctx.frame.get());
                Ok(child(ctx))
            }),
    )
    .unwrap();
    let observed = &observations.borrow()[0];
    assert_eq!(observed.frame.rect.x, expected[0] + 1.);
    assert_eq!(observed.frame.rect.y, expected[1] - 3.);
    assert!(matches!(observed.frame.mark.geometry, Geometry::Path(_)));
    let node = nodes(&app, "audit-stream-dot")[0];
    app.dispatch(node, Event::Activate).unwrap();
    let before = format!("{:?}", app.tree());
    input.revise(|p| {
        let mark = p.0.scene.marks.iter().find(|m| m.id == id).unwrap();
        let wrong = InteractionAnchor::path_points(
            &mark.geometry,
            [expected[0] + 11., expected[1]],
            &[0, 1, 2, 3],
        )
        .unwrap();
        p.0.interaction
            .as_mut()
            .unwrap()
            .anchors
            .insert(id.clone(), wrong);
        p.1.layout.slots[0].revision = 1;
    });
    app.flush().unwrap();
    assert_eq!(errors.get(), 1);
    assert_eq!(observations.borrow().len(), 1);
    assert_eq!(format!("{:?}", app.tree()), before);
    assert!(button_label(&app, node).ends_with("count=1"));
    app.clear().unwrap();
    let plain = App::memory();
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    assert!(
        plain
            .mount(
                chart(source.scene).plot_widgets_with_data(packet, move |ctx| {
                    called.set(called.get() + 1);
                    Ok(child(ctx))
                })
            )
            .is_err()
    );
    assert_eq!(calls.get(), 0);
    plain.clear().unwrap();
}

#[test]
fn nullable_to_zero_adds_only_the_new_dot_and_keeps_existing_real_child_state() {
    fn frame(zero: bool) -> (SceneInteraction, PlotWidgetsData<StreamDotWidgetData>) {
        let mut data = raw();
        if zero {
            data[0].values[1] = Some(0.);
        }
        let layout = stream_layout(
            &config(),
            &["one".into(), "two".into(), "three".into()],
            &data,
            &options(),
        )
        .unwrap();
        let packet = stream_dot_widgets(&layout, 0, placement).unwrap();
        let mut source = layout.into_scene_interaction().unwrap();
        source.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
        (source, packet)
    }
    let app = App::memory();
    let input = app.scope().cage(frame(false));
    let source = app.scope().bond(move || input.with(|p| p.0.clone()));
    let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
    app.mount(chart_layout(source).plot_widgets_with_data(slots, |ctx| Ok(child(ctx))))
        .unwrap();
    let old = nodes(&app, "audit-stream-dot");
    assert_eq!(old.len(), 5);
    // Edge slots may be clipped/inert. Exercise state on the known interior
    // second-series middle sample rather than bypassing clip eligibility.
    let active = *old
        .iter()
        .find(|n| button_label(&app, **n).starts_with("second:1:"))
        .unwrap();
    assert!(app.dispatch(active, Event::Activate).unwrap());
    assert!(button_label(&app, active).ends_with("count=1"));
    input.revise(|p| *p = frame(true));
    app.flush().unwrap();
    let new = nodes(&app, "audit-stream-dot");
    assert_eq!(new.len(), 6);
    for node in &old {
        assert!(new.contains(node));
        assert!(button_label(&app, *node).ends_with(if *node == active {
            "count=1"
        } else {
            "count=0"
        }));
    }
    let added = new.iter().find(|n| !old.contains(n)).unwrap();
    assert!(button_label(&app, *added).starts_with("first:1:Some(0.0)"));
    assert!(button_label(&app, *added).ends_with("count=0"));
    input.revise(|p| *p = frame(false));
    app.flush().unwrap();
    assert_eq!(nodes(&app, "audit-stream-dot"), old);
    assert!(app.tree().node(*added).is_err());
    app.clear().unwrap();
    assert_eq!(app.stats().nodes, 0);
}

#[cfg(feature = "raster")]
#[test]
fn real_png_rejection_keeps_path_anchor_and_typed_source_then_recovers_same_child() {
    use kartis::render::raster::{FontRegistry, RasterOptions};
    fn witness(url: &str, dpr: f64) -> [u8; 4] {
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(url.strip_prefix("data:image/png;base64,").unwrap())
            .unwrap();
        let mut reader = png::Decoder::new(std::io::Cursor::new(bytes))
            .read_info()
            .unwrap();
        let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
        let info = reader.next_frame(&mut pixels).unwrap();
        assert_eq!(
            (info.width, info.height),
            ((260. * dpr) as u32, (180. * dpr) as u32)
        );
        let x = dpr as usize;
        let y = dpr as usize;
        pixels[(y * info.width as usize + x) * 4..][..4]
            .try_into()
            .unwrap()
    }
    for dpr in [1., 2.] {
        let layout = path_layout();
        let packet = selected(&layout);
        let app = App::memory();
        let mut source = layout.into_scene_interaction().unwrap();
        source.interaction.as_mut().unwrap().picking_profile = PickingProfile::PaintedGeometry;
        source.validate().unwrap();
        let input = app.scope().cage((source, packet));
        let frame = app.scope().bond(move || input.with(|p| p.0.clone()));
        let slots = app.scope().bond(move || input.with(|p| p.1.clone()));
        let fail = Rc::new(Cell::new(false));
        let failing = fail.clone();
        let errors = Rc::new(Cell::new(0));
        let rejected = errors.clone();
        app.mount(
            chart_layout(frame)
                .raster(
                    FontRegistry::new(),
                    RasterOptions {
                        pixel_ratio: dpr,
                        ..Default::default()
                    },
                )
                .raster_image_source(move |png| {
                    if failing.get() {
                        Err(ChartError("audit Stream PNG refused".into()))
                    } else {
                        png.data_url()
                    }
                })
                .on_reject(move |_| rejected.set(rejected.get() + 1))
                .plot_widgets_with_data(slots, |ctx| Ok(child(ctx))),
        )
        .unwrap();
        let node = nodes(&app, "audit-stream-dot")[0];
        app.dispatch(node, Event::Activate).unwrap();
        let image = |app: &App| {
            let tree = app.tree();
            let id = nodes(app, "kartis-raster-image")[0];
            let Props::Image(png) = tree.node(id).unwrap().descriptor.view().props else {
                panic!("PNG required")
            };
            png.source.clone()
        };
        let before_png = image(&app);
        let before = format!("{:?}", app.tree());
        assert!(before_png.starts_with("data:image/png;base64,"));
        assert_eq!(
            witness(&before_png, dpr),
            input.with(|p| {
                let c = p.0.scene.background;
                [c.0, c.1, c.2, c.3]
            })
        );
        fail.set(true);
        input.revise(|p| {
            p.0.scene.background = Color(4, 5, 6, 255);
            p.1.data.values_mut().next().unwrap().layer_label = "candidate source data".into();
        });
        app.flush().unwrap();
        assert_eq!(errors.get(), 1);
        assert_eq!(image(&app), before_png);
        assert_ne!(witness(&image(&app), dpr), [4, 5, 6, 255]);
        assert_eq!(format!("{:?}", app.tree()), before);
        fail.set(false);
        input.revise(|p| p.1.data.values_mut().next().unwrap().layer_id = "recovered".into());
        app.flush().unwrap();
        assert_ne!(image(&app), before_png);
        assert_eq!(nodes(&app, "audit-stream-dot"), [node]);
        assert_eq!(witness(&image(&app), dpr), [4, 5, 6, 255]);
        assert!(button_label(&app, node).starts_with("recovered:"));
        assert!(button_label(&app, node).ends_with("count=1"));
        app.clear().unwrap();
        assert_eq!(app.stats().nodes, 0);
    }
}
