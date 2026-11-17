use std::cell::{Cell, RefCell};
use std::rc::Rc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 280.,
        margin: Margin {
            left: 25.,
            right: 25.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<CategoryKey> {
    vec![(-0.).into(), "0".into(), "third".into(), "last".into()]
}
fn source() -> Vec<BarSeries> {
    vec![
        BarSeries::new("a", vec![Some(1.), None, Some(-0.), Some(4.)]),
        BarSeries::new("hidden", vec![Some(2.), Some(3.), Some(5.), Some(6.)]),
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
        legend: true,
        ..Default::default()
    };
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    o
}

#[test]
fn default_bundle_and_inherited_geometry_preserve_complete_legacy_output() {
    let raw = source();
    let categories = categories();
    for position in [
        StreamDotPosition::Start,
        StreamDotPosition::Center,
        StreamDotPosition::End,
    ] {
        for curve in [
            Curve::Linear,
            Curve::Basis,
            Curve::StepBefore,
            Curve::CatmullRom { alpha: 0.5 },
        ] {
            let mut o = options();
            o.curve = curve;
            o.dots.as_mut().unwrap().position = position;
            let old = stream_layout(&config(), &categories, &raw, &o).unwrap();
            let new = stream_layout_with_factories(
                &config(),
                &categories,
                &raw,
                &o,
                &mut Default::default(),
            )
            .unwrap();
            assert_eq!(format!("{old:?}"), format!("{new:?}"));
            assert_eq!(
                render::to_svg(&old.scene).unwrap(),
                render::to_svg(&new.scene).unwrap()
            );
            let inherited = stream_layout_with_factories_and_dot_factory(
                &config(),
                &categories,
                &raw,
                &o,
                &mut Default::default(),
                |c| {
                    Ok(c.inherit(Geometry::Circle {
                        x: c.position[0],
                        y: c.position[1],
                        radius: c.style.size / 2.,
                    }))
                },
            )
            .unwrap();
            let old_custom =
                stream_layout_with_dot_factory(&config(), &categories, &raw, &o, |c| {
                    Ok(c.inherit(Geometry::Circle {
                        x: c.position[0],
                        y: c.position[1],
                        radius: c.style.size / 2.,
                    }))
                })
                .unwrap();
            assert_eq!(format!("{inherited:?}"), format!("{old_custom:?}"));
        }
    }
}

#[test]
fn eight_functions_receive_complete_contexts_and_hidden_samples_format_once() {
    let raw = source();
    let categories = categories();
    let mut o = options();
    o.hidden.insert("hidden".into());
    let log = Rc::new(RefCell::new(vec![]));
    let add = |name: &str| log.borrow_mut().push(name.to_owned());
    let mut format = |v: f64| {
        add(&format!("format:{}", v.to_bits()));
        Ok(format!("value={v}"))
    };
    let mut label = |c: &StreamLayerPathContext<'_>| {
        assert!(std::ptr::eq(c.series, &raw[c.series_index]));
        assert_eq!(c.id, c.series.id);
        assert!(matches!(c.path, Some(Geometry::Path(_))));
        assert_eq!(c.plot, ClipRect::plot(&config()));
        add(&format!("label:{}", c.id));
        Ok(format!("Layer {}", c.id))
    };
    let mut color = |c: &StreamLayerPathContext<'_>| {
        add(&format!("color:{}", c.id));
        Ok(Color(40, 120, 90, 255))
    };
    let full = |c: &StreamDotStyleContext<'_>| {
        assert_eq!(c.layer.samples.len(), 4);
        assert!(
            c.layer
                .samples
                .iter()
                .all(|s| s.value.is_none() == s.formatted_value.is_none())
        );
        assert_eq!(c.layer.label, "Layer a");
        assert_eq!(c.layer.color, Color(40, 120, 90, 255));
        assert!(std::ptr::eq(
            c.sample,
            &c.layer.samples[c.sample.source_index]
        ));
        assert!(std::ptr::eq(c.style, o.dots.as_ref().unwrap()));
        assert_eq!(c.sample.value, raw[0].values[c.sample.source_index]);
    };
    let mut border = |c: &StreamLayerStyleContext<'_>| {
        assert_eq!(c.layer.samples.len(), 4);
        assert_eq!(c.layer.label, "Layer a");
        assert_eq!(c.layer.color, Color(40, 120, 90, 255));
        assert_eq!(c.layer.area_mark_id, Some("stream:area:1:a".into()));
        add("border:a");
        Ok(Color(200, 20, 30, 255))
    };
    let mut size = |c: &StreamDotStyleContext<'_>| {
        full(c);
        add(&format!("size:{}", c.sample.source_index));
        Ok(0.)
    };
    let mut dot_color = |c: &StreamDotStyleContext<'_>| {
        full(c);
        add(&format!("dot_color:{}", c.sample.source_index));
        Ok(Color(10, 80, 200, 255))
    };
    let mut width = |c: &StreamDotStyleContext<'_>| {
        full(c);
        add(&format!("width:{}", c.sample.source_index));
        Ok(0.)
    };
    let mut dot_border = |c: &StreamDotStyleContext<'_>| {
        full(c);
        add(&format!("dot_border:{}", c.sample.source_index));
        Ok(Color(90, 30, 120, 255))
    };
    let layout = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut dot_color),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut dot_border),
        },
    )
    .unwrap();
    let mut expected = vec![
        format!("format:{}", 1_f64.to_bits()),
        format!("format:{}", (-0_f64).to_bits()),
        format!("format:{}", 4_f64.to_bits()),
        "label:a".into(),
        "color:a".into(),
        "border:a".into(),
    ];
    for i in [0, 2, 3] {
        for kind in ["size", "dot_color", "width", "dot_border"] {
            expected.push(format!("{kind}:{i}"));
        }
    }
    for v in [2_f64, 3., 5., 6.] {
        expected.push(format!("format:{}", v.to_bits()));
    }
    expected.extend(["label:hidden".into(), "color:hidden".into()]);
    assert_eq!(*log.borrow(), expected);
    assert_eq!(layout.layers[0].samples[1].formatted_value, None);
    assert_eq!(
        layout
            .interaction_binding()
            .sites
            .iter()
            .find(|s| s.mark_id == "stream:area:1:a")
            .unwrap()
            .formatted_value,
        Some("value=4".into())
    );
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:sample:"))
            .all(|m| matches!(m.geometry, Geometry::Circle { radius: 0., .. }))
    );
    let packet = stream_dot_widgets(&layout, 0, |_| {
        Ok(Some(PlotWidgetPlacement::AtDatum {
            offset: [0., 0.],
            measured: TextSize {
                width: 1.,
                height: 1.,
            },
        }))
    })
    .unwrap();
    assert_eq!(packet.data.len(), 3);
    assert!(
        packet
            .data
            .values()
            .all(|d| d.layer_label == "Layer a" && d.layer_color == Color(40, 120, 90, 255))
    );
    assert_eq!(
        packet.data.values().next().unwrap().series.values,
        raw[0].values
    );
}

#[test]
fn explicit_empty_labels_are_preserved_and_last_null_summary_is_not_reformatted() {
    let categories = categories();
    let raw = vec![BarSeries::new(
        "a",
        vec![Some(1.), Some(2.), Some(3.), None],
    )];
    let o = options();
    let calls = Cell::new(0);
    let mut format = |v: f64| {
        calls.set(calls.get() + 1);
        Ok(format!("formatted{v}"))
    };
    let mut empty = |_: &StreamLayerPathContext<'_>| Ok(String::new());
    let layout = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            label: Some(&mut empty),
            formatted_value: Some(&mut format),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(calls.get(), 3);
    assert_eq!(layout.layers[0].label, "");
    assert_eq!(layout.scene.legend[0].label, "");
    let area = layout
        .scene
        .marks
        .iter()
        .find(|m| m.id == "stream:area:1:a")
        .unwrap();
    assert_eq!(area.label, "");
    assert!(
        !layout
            .interaction_binding()
            .sites
            .iter()
            .any(|s| s.mark_id == area.id)
    );
    assert!(!layout.interaction_binding().anchors.contains_key(&area.id));
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:sample:"))
            .all(|m| m.label.starts_with(" · "))
    );
    assert_eq!(
        layout
            .slice_interaction_binding()
            .sites
            .iter()
            .find(|s| s.value.is_none())
            .unwrap()
            .formatted_value,
        None
    );
    layout.clone().into_scene_interaction().unwrap();
    layout.into_slice_interaction().unwrap();
    let mut o = options();
    o.labels.insert("a".into(), "".into());
    let legacy = stream_layout(&config(), &categories, &raw, &o).unwrap();
    assert_eq!(
        legacy.layers[0].label, "a",
        "static empty-label fallback remains unchanged"
    );
}

#[test]
fn resolved_circle_diameter_and_paint_reach_actual_mark_and_typed_bindings() {
    let raw = source();
    let categories = categories();
    let mut o = options();
    o.dots.as_mut().unwrap().position = StreamDotPosition::End;
    o.style.border_width = 2.;
    let mut layer_color = |_: &StreamLayerPathContext<'_>| Ok(Color(70, 120, 30, 255));
    let mut border = |_: &StreamLayerStyleContext<'_>| Ok(Color(220, 20, 50, 255));
    let mut size =
        |c: &StreamDotStyleContext<'_>| Ok(if c.sample.value == Some(0.) { 0. } else { 12. });
    let mut color = |_: &StreamDotStyleContext<'_>| Ok(Color(5, 140, 210, 255));
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut stroke = |_: &StreamDotStyleContext<'_>| Ok(Color(120, 10, 170, 255));
    let l = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            color: Some(&mut layer_color),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut color),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut stroke),
            ..Default::default()
        },
    )
    .unwrap();
    let slice = l.slice_interaction_binding();
    let exact = l.interaction_binding();
    for layer in &l.layers {
        assert_eq!(layer.color, Color(70, 120, 30, 255));
        assert_eq!(
            l.scene
                .legend
                .iter()
                .find(|e| e.id == layer.id)
                .unwrap()
                .color,
            layer.color
        );
        let area = l
            .scene
            .marks
            .iter()
            .find(|m| Some(&m.id) == layer.area_mark_id.as_ref())
            .unwrap();
        assert_eq!(area.fill, Some(layer.color));
        assert_eq!(area.stroke.unwrap().color, Color(220, 20, 50, 255));
        for sample in &layer.samples {
            let mark = l
                .scene
                .marks
                .iter()
                .find(|m| m.id == sample.mark_id)
                .unwrap();
            assert_eq!(sample.position[1], sample.screen_y[1]);
            if let Some(value) = sample.value {
                assert_eq!(
                    mark.geometry,
                    Geometry::Circle {
                        x: sample.position[0],
                        y: sample.position[1],
                        radius: if value == 0. { 0. } else { 6. }
                    }
                );
                assert_eq!(mark.fill, Some(Color(5, 140, 210, 255)));
                assert_eq!(mark.stroke.unwrap().color, Color(120, 10, 170, 255));
                assert_eq!(
                    slice
                        .sites
                        .iter()
                        .find(|s| s.mark_id == sample.mark_id)
                        .unwrap()
                        .color,
                    Color(5, 140, 210, 255)
                );
                assert!(
                    exact.sites.iter().any(|s| s.mark_id == sample.mark_id),
                    "positive border retains actual zero-radius dot ink eligibility"
                );
            } else {
                assert!(mark.stroke.is_none());
                assert_eq!(
                    slice
                        .sites
                        .iter()
                        .find(|s| s.mark_id == sample.mark_id)
                        .unwrap()
                        .value,
                    None
                );
            }
        }
    }
}

fn transparent() -> (ChartConfig, StreamLayoutOptions) {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let mut o = options();
    let contrast = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    o.style.color = Some(Color(0, 0, 0, 0));
    o.style.border_width = 2.;
    o.style.border_color = contrast.clone();
    let d = o.dots.as_mut().unwrap();
    d.color = contrast.clone();
    d.border_width = 2.;
    d.border_color = contrast;
    (c, o)
}

#[test]
fn explicit_color_overrides_do_not_resolve_unused_contrast_and_geometry_inherits_paint() {
    let (c, o) = transparent();
    let raw = source();
    let categories = categories();
    assert!(stream_layout(&c, &categories, &raw, &o).is_err());
    let events = Rc::new(RefCell::new(vec![]));
    let add = |s| events.borrow_mut().push(s);
    let mut border = |_: &StreamLayerStyleContext<'_>| {
        add("border");
        Ok(Color(210, 30, 40, 255))
    };
    let mut dot = |_: &StreamDotStyleContext<'_>| {
        add("color");
        Ok(Color(10, 130, 170, 255))
    };
    let mut width = |_: &StreamDotStyleContext<'_>| {
        add("width");
        Ok(3.)
    };
    let mut stroke = |_: &StreamDotStyleContext<'_>| {
        add("stroke");
        Ok(Color(120, 20, 170, 255))
    };
    let mut size = |_: &StreamDotStyleContext<'_>| {
        add("size");
        Ok(12.)
    };
    let layout = stream_layout_with_factories_and_dot_factory(
        &c,
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut dot),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut stroke),
            ..Default::default()
        },
        |ctx| {
            add("geometry");
            assert!(matches!(ctx.style.color, InheritedColor::Contrast { .. }));
            assert_eq!(ctx.style.size, 6.);
            assert_eq!(ctx.fill, Some(Color(10, 130, 170, 255)));
            assert_eq!(ctx.stroke.unwrap().width, 3.);
            let [x, y] = ctx.position;
            Ok(ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 20., y - 5.)),
                C::Line(P::new(x + 30., y - 5.)),
                C::Line(P::new(x + 30., y + 5.)),
                C::Line(P::new(x + 20., y + 5.)),
                C::Close,
            ])))
        },
    )
    .unwrap();
    assert_eq!(
        events.borrow().iter().filter(|e| **e == "geometry").count(),
        7
    );
    assert_eq!(
        events.borrow().iter().filter(|e| **e == "border").count(),
        2
    );
    for layer in &layout.layers {
        for sample in &layer.samples {
            if sample.value.is_some() {
                let m = layout
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == sample.mark_id)
                    .unwrap();
                assert_eq!(m.fill, Some(Color(10, 130, 170, 255)));
                assert_eq!(m.stroke.unwrap().color, Color(120, 20, 170, 255));
                assert_eq!(
                    layout.slice_interaction_binding().anchor_for(m).unwrap(),
                    Some(sample.position)
                );
            }
        }
    }
    let mut zero = options();
    zero.dots.as_mut().unwrap().border_width = 0.;
    zero.dots.as_mut().unwrap().border_color = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    zero.style.color = Some(Color(0, 0, 0, 0));
    let mut border_calls = 0;
    let mut explicit = |_: &StreamDotStyleContext<'_>| {
        border_calls += 1;
        Ok(Color(1, 2, 3, 255))
    };
    stream_layout_with_factories(
        &c,
        &categories,
        &raw,
        &zero,
        &mut StreamFactories {
            dot_border_color: Some(&mut explicit),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        border_calls, 7,
        "explicit width0 border color is still a function role"
    );
}

#[test]
fn glyph_size_conflict_is_preflight_and_dynamic_stroke_width_is_checked() {
    let raw = source();
    let categories = categories();
    let mut o = options();
    o.dots.as_mut().unwrap().glyph = Some(GlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Plus),
        size: 30.,
        rotation: 0.,
    });
    let calls = Cell::new(0);
    let mut label = |_: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("a".into())
    };
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut size = |_: &StreamDotStyleContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(20.)
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &o,
            &mut StreamFactories {
                label: Some(&mut label),
                dot_size: Some(&mut size),
                dot_border_width: Some(&mut width),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    assert!(
        stream_layout(&config(), &categories, &raw, &o).is_err(),
        "unchanged static stroke-only width0 is invalid"
    );
    let layout = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            dot_border_width: Some(&mut width),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:sample:") && m.value != 0.)
            .all(|m| m.stroke.unwrap().width == 2.)
    );
    let geometry_calls = Cell::new(0);
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(0.);
    assert!(
        stream_layout_with_factories_and_dot_factory(
            &config(),
            &categories,
            &raw,
            &o,
            &mut StreamFactories {
                dot_border_width: Some(&mut width),
                ..Default::default()
            },
            |c| {
                geometry_calls.set(geometry_calls.get() + 1);
                Ok(c.inherit(Geometry::Circle {
                    x: c.position[0],
                    y: c.position[1],
                    radius: 0.,
                }))
            }
        )
        .is_err()
    );
    assert_eq!(geometry_calls.get(), 0);
    let mut o = o.clone();
    o.hidden = ["a".into(), "hidden".into()].into();
    stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            dot_border_width: Some(&mut width),
            ..Default::default()
        },
    )
    .unwrap();
}

#[test]
fn complete_static_preflight_rejects_late_source_options_projection_before_callbacks() {
    let original = source();
    let categories = categories();
    for case in 0..8 {
        let mut raw = original.clone();
        let mut o = options();
        let mut categories = categories.clone();
        match case {
            0 => raw[1].values[3] = Some(f64::NAN),
            1 => raw[1].values.pop().map(|_| ()).unwrap(),
            2 => raw[1].id = "a".into(),
            3 => {
                o.labels.insert("unknown".into(), "x".into());
            }
            4 => categories[3] = categories[0].clone(),
            5 => o.dots.as_mut().unwrap().size = f64::NAN,
            6 => o.y_scale.kind = ScaleKind::Utc,
            7 => o.colors.palette = OrdinalColorPalette::Custom(vec![]),
            _ => unreachable!(),
        };
        let calls = Cell::new(0);
        let mut label = |_: &StreamLayerPathContext<'_>| {
            calls.set(calls.get() + 1);
            Ok("x".into())
        };
        let mut formatter = |_: f64| {
            calls.set(calls.get() + 1);
            Ok("x".into())
        };
        assert!(
            stream_layout_with_factories(
                &config(),
                &categories,
                &raw,
                &o,
                &mut StreamFactories {
                    label: Some(&mut label),
                    formatted_value: Some(&mut formatter),
                    ..Default::default()
                }
            )
            .is_err(),
            "case{case}"
        );
        assert_eq!(calls.get(), 0, "case{case}");
    }
    // The last original boundary overflows before any prior layer callback.
    let mut o = options();
    o.y_bounds = [Some(0.), Some(f64::MIN_POSITIVE)];
    let calls = Cell::new(0);
    let mut label = |_: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("x".into())
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &original,
            &o,
            &mut StreamFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn invalid_dynamic_outputs_stop_without_returning_partial_layout() {
    let raw = source();
    let categories = categories();
    let original = stream_layout(&config(), &categories, &raw, &options()).unwrap();
    for invalid in [f64::NAN, f64::INFINITY, -1., 4096.0001] {
        let calls = Cell::new(0);
        let mut size = |_: &StreamDotStyleContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(invalid)
        };
        assert!(
            stream_layout_with_factories(
                &config(),
                &categories,
                &raw,
                &options(),
                &mut StreamFactories {
                    dot_size: Some(&mut size),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
    for invalid in [f64::NAN, -1., 1e-9, 256.0001] {
        let calls = Cell::new(0);
        let mut width = |_: &StreamDotStyleContext<'_>| {
            calls.set(calls.get() + 1);
            Ok(invalid)
        };
        assert!(
            stream_layout_with_factories(
                &config(),
                &categories,
                &raw,
                &options(),
                &mut StreamFactories {
                    dot_border_width: Some(&mut width),
                    ..Default::default()
                }
            )
            .is_err()
        );
        assert_eq!(calls.get(), 1);
    }
    let mut huge = |_: f64| Ok("字".repeat(342));
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &options(),
            &mut StreamFactories {
                formatted_value: Some(&mut huge),
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut exact = |_: f64| Ok("字".repeat(341) + "x");
    let accepted = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &options(),
        &mut StreamFactories {
            formatted_value: Some(&mut exact),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        accepted.layers[0].samples[0]
            .formatted_value
            .as_ref()
            .unwrap()
            .len(),
        1024
    );
    let mut huge_label = |_: &StreamLayerPathContext<'_>| Ok("x".repeat(1024));
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &options(),
            &mut StreamFactories {
                label: Some(&mut huge_label),
                ..Default::default()
            }
        )
        .is_err(),
        "full sample label includes category suffix"
    );
    let calls = Cell::new(0);
    let mut late = |_: &StreamDotStyleContext<'_>| {
        calls.set(calls.get() + 1);
        if calls.get() == 6 {
            Err(ChartError("late dot rejected".into()))
        } else {
            Ok(Color(1, 2, 3, 255))
        }
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &options(),
            &mut StreamFactories {
                dot_color: Some(&mut late),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 6);
    assert_eq!(
        stream_layout(&config(), &categories, &raw, &options())
            .unwrap()
            .scene,
        original.scene
    );
}

#[test]
fn empty_layers_still_compute_label_color_but_have_no_border_or_dot_roles() {
    let raw = vec![BarSeries::new("a", vec![]), BarSeries::new("b", vec![])];
    let calls = Rc::new(RefCell::new(vec![]));
    let mut label = |c: &StreamLayerPathContext<'_>| {
        assert_eq!(c.path, None);
        calls.borrow_mut().push(format!("label:{}", c.id));
        Ok(c.id.into())
    };
    let mut color = |c: &StreamLayerPathContext<'_>| {
        assert_eq!(c.path, None);
        calls.borrow_mut().push(format!("color:{}", c.id));
        Ok(Color(1, 2, 3, 255))
    };
    let mut formatter = |_: f64| panic!("no sample");
    let mut border = |_: &StreamLayerStyleContext<'_>| panic!("no area");
    let mut dot = |_: &StreamDotStyleContext<'_>| panic!("no dot");
    let layout = stream_layout_with_factories(
        &config(),
        &[],
        &raw,
        &options(),
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut formatter),
            border_color: Some(&mut border),
            dot_size: Some(&mut dot),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        *calls.borrow(),
        ["label:a", "color:a", "label:b", "color:b"]
    );
    assert_eq!(layout.layers.len(), 2);
    assert_eq!(layout.scene.legend.len(), 2);
}

#[test]
fn explicit_layer_color_overrides_missing_fixed_domain_without_mutating_ordinal_mapping() {
    let raw = source();
    let categories = categories();
    let mut o = options();
    o.colors.domain = Some(vec!["declared-only".into()]);
    o.colors.unknown = OrdinalColorUnknown::Missing;
    assert!(
        stream_layout(&config(), &categories, &raw, &o).is_err(),
        "old missing default color remains an error without a function"
    );
    let calls = Cell::new(0);
    let mut color = |c: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(if c.id == "a" {
            Color(10, 90, 150, 255)
        } else {
            Color(90, 20, 140, 255)
        })
    };
    let layout = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
    .expect("explicit color does not need a missing static default");
    assert_eq!(calls.get(), 2);
    assert_eq!(
        layout.colors.domain(),
        &[CategoryKey::String("declared-only".into())]
    );
    for layer in &layout.layers {
        assert_eq!(
            layout.colors.map(&CategoryKey::String(layer.id.clone())),
            None
        );
        assert_eq!(
            layout
                .scene
                .legend
                .iter()
                .find(|e| e.id == layer.id)
                .unwrap()
                .color,
            layer.color
        );
        assert_eq!(
            layout
                .scene
                .marks
                .iter()
                .find(|m| Some(&m.id) == layer.area_mark_id.as_ref())
                .unwrap()
                .fill,
            Some(layer.color)
        );
    }
    let mut invalid = o;
    invalid.colors.palette = OrdinalColorPalette::Custom(vec![]);
    calls.set(0);
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &invalid,
            &mut StreamFactories {
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(
        calls.get(),
        0,
        "explicit callback cannot bypass invalid static palette configuration"
    );
}

#[test]
fn hidden_layer_full_label_budget_does_not_require_an_unrendered_sample_suffix() {
    let raw = source();
    let categories = categories();
    let mut o = options();
    o.hidden.insert("hidden".into());
    o.labels.insert("hidden".into(), "x".repeat(1024));
    let layout=stream_layout(&config(),&categories,&raw,&o).expect("hidden metadata has a bounded label, but there is no generated sample Mark label to concatenate");
    assert_eq!(layout.layers[1].label.len(), 1024);
    assert_eq!(layout.scene.legend[1].label.len(), 1024);
    let mut label = |c: &StreamLayerPathContext<'_>| {
        Ok(if c.id == "hidden" {
            "x".repeat(1024)
        } else {
            "a".into()
        })
    };
    let layout = stream_layout_with_factories(
        &config(),
        &categories,
        &raw,
        &o,
        &mut StreamFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(layout.layers[1].label.len(), 1024);
    o.hidden.clear();
    assert!(
        stream_layout(&config(), &categories, &raw, &o).is_err(),
        "visible sample text still must fit its full generated label budget"
    );
}

#[test]
fn disabled_hidden_area_does_not_generate_unused_curve_controls() {
    let values = [0., 0., 3740., 3740., 0., 0., 3740., 0.];
    let raw = [BarSeries::new(
        "hidden",
        values.into_iter().map(Some).collect(),
    )];
    let categories: Vec<_> = (0..values.len())
        .map(|i| CategoryKey::Number(i as f64))
        .collect();
    let mut o = options();
    o.curve = Curve::Natural;
    o.y_bounds = [Some(0.), Some(1.)];
    o.hidden.insert("hidden".into());
    let layout = stream_layout(&config(), &categories, &raw, &o).expect(
        "hidden sample projections are valid; no area rendering needs its Natural control hull",
    );
    assert!(
        layout.layers[0]
            .samples
            .iter()
            .all(|s| s.screen_y.iter().all(|v| v.abs() <= 900_000.))
    );
    assert!(layout.layers[0].area_mark_id.is_none());
    let calls = Cell::new(0);
    let mut label = |_: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("label".into())
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories,
            &raw,
            &o,
            &mut StreamFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err(),
        "an explicit geometry context must obey its real control-coordinate budget"
    );
    assert_eq!(
        calls.get(),
        0,
        "all requested original path geometry is preflighted before callbacks"
    );
    o.hidden.clear();
    assert!(
        stream_layout(&config(), &categories, &raw, &o).is_err(),
        "visible area admission is not relaxed"
    );
}
