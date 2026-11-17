//! Independent eight-role Stream function contract acceptance.
use std::cell::{Cell, RefCell};

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::stack::{StackOffset, StackOptions, StackOrder};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 240.,
        margin: Margin {
            left: 31.,
            right: 23.,
            top: 25.,
            bottom: 21.,
        },
        ..Default::default()
    }
}
fn categories() -> Vec<CategoryKey> {
    vec!["one".into(), "two".into(), "three".into()]
}
fn raw() -> Vec<BarSeries> {
    vec![
        BarSeries::new("a", vec![Some(4.), None, Some(-0.)]),
        BarSeries::new("b", vec![Some(2.), Some(3.), None]),
        BarSeries::new("hidden", vec![Some(8.), Some(7.), Some(6.)]),
    ]
}
fn options() -> StreamLayoutOptions {
    let mut o = StreamLayoutOptions {
        stack: StackOptions {
            offset: StackOffset::Zero,
            order: StackOrder::Reverse,
        },
        curve: Curve::Linear,
        dots: Some(StreamDotOptions::default()),
        legend: true,
        ..Default::default()
    };
    o.hidden.insert("hidden".into());
    o.axes.x.visible = false;
    o.axes.x.grid = false;
    o.axes.y.visible = false;
    o.axes.y.grid = false;
    o
}

#[test]
fn complete_layer_contexts_and_eight_roles_observe_null_hidden_zero_order_without_extra_summary_format()
 {
    let events = RefCell::new(vec![]);
    let format_calls = Cell::new(0);
    let mut format = |v: f64| {
        let n = format_calls.get() + 1;
        format_calls.set(n);
        events
            .borrow_mut()
            .push(format!("format:{:x}", v.to_bits()));
        Ok(format!("call{n}"))
    };
    let mut label = |c: &StreamLayerPathContext<'_>| {
        assert_eq!(c.series, &raw()[c.series_index]);
        assert_eq!(c.id, c.series.id);
        assert!(c.path.is_some());
        assert_eq!(c.plot, ClipRect::plot(&config()));
        events.borrow_mut().push(format!("label:{}", c.id));
        Ok(format!("label-{}", c.id))
    };
    let mut color = |c: &StreamLayerPathContext<'_>| {
        events.borrow_mut().push(format!("color:{}", c.id));
        Ok(Color(20 + c.series_index as u8, 90, 140, 255))
    };
    let mut border = |c: &StreamLayerStyleContext<'_>| {
        assert!(!c.layer.hidden);
        assert!(c.layer.area_mark_id.is_some());
        assert_eq!(c.layer.samples.len(), 3);
        assert!(
            c.layer
                .samples
                .iter()
                .filter(|s| s.value.is_some())
                .all(|s| s.formatted_value.is_some())
        );
        assert_eq!(c.layer.label, format!("label-{}", c.layer.id));
        events.borrow_mut().push(format!("border:{}", c.layer.id));
        Ok(Color(230, 110, 20, 255))
    };
    let record = |name: &str, c: &StreamDotStyleContext<'_>| {
        assert_eq!(
            c.layer.samples.len(),
            3,
            "full layer, not construction prefix"
        );
        assert!(!c.layer.hidden);
        assert!(c.sample.value.is_some());
        assert_eq!(c.sample.position, c.position);
        assert_eq!(
            c.sample.formatted_value,
            c.layer.samples[c.sample.source_index].formatted_value
        );
        events
            .borrow_mut()
            .push(format!("{name}:{}:{}", c.layer.id, c.sample.source_index));
    };
    let mut size = |c: &StreamDotStyleContext<'_>| {
        record("size", c);
        Ok(0.)
    };
    let mut fill = |c: &StreamDotStyleContext<'_>| {
        record("fill", c);
        Ok(Color(30, 60, 180, 0))
    };
    let mut width = |c: &StreamDotStyleContext<'_>| {
        record("width", c);
        Ok(0.)
    };
    let mut stroke = |c: &StreamDotStyleContext<'_>| {
        record("stroke", c);
        Ok(Color(60, 30, 180, 0))
    };
    let layout = stream_layout_with_factories(
        &config(),
        &categories(),
        &raw(),
        &options(),
        &mut StreamFactories {
            label: Some(&mut label),
            color: Some(&mut color),
            formatted_value: Some(&mut format),
            border_color: Some(&mut border),
            dot_size: Some(&mut size),
            dot_color: Some(&mut fill),
            dot_border_width: Some(&mut width),
            dot_border_color: Some(&mut stroke),
        },
    )
    .unwrap();
    assert_eq!(
        format_calls.get(),
        7,
        "all Some once, hidden included, None excluded, no area summary call"
    );
    let e = events.borrow();
    for id in ["a", "b", "hidden"] {
        let l = e.iter().position(|x| x == &format!("label:{id}")).unwrap();
        assert_eq!(e[l + 1], format!("color:{id}"));
    }
    assert!(
        !e.iter()
            .any(|x| x.contains("border:hidden") || x.contains(":hidden:"))
    );
    let dots = e
        .iter()
        .filter(|x| {
            ["size:", "fill:", "width:", "stroke:"]
                .iter()
                .any(|prefix| x.starts_with(prefix))
        })
        .collect::<Vec<_>>();
    assert_eq!(dots.len(), 16);
    for chunk in dots.chunks(4) {
        assert!(
            chunk[0].starts_with("size:")
                && chunk[1].starts_with("fill:")
                && chunk[2].starts_with("width:")
                && chunk[3].starts_with("stroke:")
        );
    }
    let a = layout.layers.iter().find(|l| l.id == "a").unwrap();
    let b = layout.layers.iter().find(|l| l.id == "b").unwrap();
    assert_eq!(a.samples[2].value.unwrap().to_bits(), (-0f64).to_bits());
    assert!(b.samples[2].formatted_value.is_none());
    let binding = layout.interaction_binding();
    let area_a = binding
        .sites
        .iter()
        .find(|s| s.mark_id == a.area_mark_id.as_ref().unwrap().as_str())
        .unwrap();
    assert_eq!(area_a.formatted_value, a.samples[2].formatted_value);
    let area_b = binding
        .sites
        .iter()
        .find(|s| s.mark_id == b.area_mark_id.as_ref().unwrap().as_str())
        .unwrap();
    assert!(area_b.value.is_none() && area_b.formatted_value.is_none());
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:area"))
            .all(|m| m.stroke.is_none()),
        "callback does not enable disabled border ink"
    );
    assert_eq!(
        layout
            .scene
            .legend
            .iter()
            .map(|l| l.label.as_str())
            .collect::<Vec<_>>(),
        layout
            .layers
            .iter()
            .map(|l| l.label.as_str())
            .collect::<Vec<_>>()
    );
}

#[test]
fn defaults_preserve_entire_layout_and_explicit_empty_labels_do_not_invent_area_controls() {
    for dots in [false, true] {
        for position in [
            StreamDotPosition::Start,
            StreamDotPosition::Center,
            StreamDotPosition::End,
        ] {
            let mut o = options();
            o.dots = dots.then_some(StreamDotOptions {
                position,
                ..Default::default()
            });
            let before = stream_layout(&config(), &categories(), &raw(), &o).unwrap();
            let after = stream_layout_with_factories(
                &config(),
                &categories(),
                &raw(),
                &o,
                &mut StreamFactories::default(),
            )
            .unwrap();
            assert_eq!(format!("{before:?}"), format!("{after:?}"));
            assert_eq!(
                render::to_svg(&before.scene).unwrap(),
                render::to_svg(&after.scene).unwrap()
            );
        }
    }
    let mut empty = |_: &StreamLayerPathContext<'_>| Ok(String::new());
    let layout = stream_layout_with_factories(
        &config(),
        &categories(),
        &raw(),
        &options(),
        &mut StreamFactories {
            label: Some(&mut empty),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(layout.layers.iter().all(|l| l.label.is_empty()));
    assert!(layout.scene.legend.iter().all(|l| l.label.is_empty()));
    assert!(
        layout
            .interaction_binding()
            .sites
            .iter()
            .all(|s| !s.mark_id.starts_with("stream:area"))
    );
    assert!(
        layout
            .slice_interaction_binding()
            .sites
            .iter()
            .all(|s| s.label.starts_with(" · "))
    );
}

#[test]
fn explicit_functions_override_unused_contrast_and_geometry_sees_resolved_paint_once() {
    let mut c = config();
    c.theme.background = Color(0, 0, 0, 0);
    let mut o = options();
    o.style.border_color = InheritedColor::Contrast {
        dark: Color(0, 0, 0, 255),
        light: Color(255, 255, 255, 255),
    };
    o.style.border_width = 3.;
    o.dots.as_mut().unwrap().color = o.style.border_color.clone();
    o.dots.as_mut().unwrap().border_color = o.style.border_color.clone();
    o.dots.as_mut().unwrap().border_width = 2.;
    let mut layer = |_: &StreamLayerPathContext<'_>| Ok(Color(0, 0, 0, 0));
    let mut border = |_: &StreamLayerStyleContext<'_>| Ok(Color(230, 110, 20, 255));
    let mut fill = |_: &StreamDotStyleContext<'_>| Ok(Color(20, 90, 140, 255));
    let calls = Cell::new(0);
    let mut stroke = |_: &StreamDotStyleContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(Color(80, 30, 150, 255))
    };
    let mut geometry_calls = 0;
    let result = stream_layout_with_factories_and_dot_factory(
        &c,
        &categories(),
        &raw(),
        &o,
        &mut StreamFactories {
            color: Some(&mut layer),
            border_color: Some(&mut border),
            dot_color: Some(&mut fill),
            dot_border_color: Some(&mut stroke),
            ..Default::default()
        },
        |ctx| {
            geometry_calls += 1;
            assert_eq!(ctx.fill, Some(Color(20, 90, 140, 255)));
            assert_eq!(ctx.stroke.unwrap().color, Color(80, 30, 150, 255));
            let [x, y] = ctx.position;
            Ok(ctx.inherit(Geometry::Path(vec![
                C::Move(P::new(x + 8., y - 3.)),
                C::Line(P::new(x + 14., y - 3.)),
                C::Line(P::new(x + 14., y + 3.)),
                C::Close,
            ])))
        },
    )
    .unwrap();
    assert_eq!(geometry_calls, 4);
    assert_eq!(calls.get(), 4);
    let sites = result.slice_interaction_binding();
    for sample in result
        .layers
        .iter()
        .filter(|l| !l.hidden)
        .flat_map(|l| &l.samples)
        .filter(|s| s.value.is_some())
    {
        let mark = result
            .scene
            .marks
            .iter()
            .find(|m| m.id == sample.mark_id)
            .unwrap();
        assert_eq!(sites.anchor_for(mark).unwrap().unwrap(), sample.position);
    }
}

#[test]
fn late_invalid_source_glyph_units_budget_and_errors_stop_without_partial_candidate() {
    let mut invalid = raw();
    invalid[2].values[2] = Some(f64::NAN);
    let calls = Cell::new(0);
    let mut label = |_: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok("x".into())
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &invalid,
            &options(),
            &mut StreamFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut o = options();
    o.dots.as_mut().unwrap().glyph = Some(GlyphOptions {
        shape: GlyphShape::Filled(FilledGlyph::Circle),
        size: 64.,
        rotation: 0.,
    });
    let mut size = |_: &StreamDotStyleContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(8.)
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &o,
            &mut StreamFactories {
                label: Some(&mut label),
                dot_size: Some(&mut size),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(calls.get(), 0);
    let mut formats = 0;
    let mut format = |_: f64| {
        formats += 1;
        if formats == 3 {
            Err(ChartError("independent format stop".into()))
        } else {
            Ok("good".into())
        }
    };
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &options(),
            &mut StreamFactories {
                formatted_value: Some(&mut format),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert_eq!(formats, 3);
    for value in [f64::NAN, -1., 1e9] {
        let mut size = |_: &StreamDotStyleContext<'_>| Ok(value);
        assert!(
            stream_layout_with_factories(
                &config(),
                &categories(),
                &raw(),
                &options(),
                &mut StreamFactories {
                    dot_size: Some(&mut size),
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    let max = "界".repeat(341) + "a";
    assert_eq!(max.len(), 1024);
    let mut format = |_: f64| Ok(max.clone());
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &options(),
            &mut StreamFactories {
                formatted_value: Some(&mut format),
                ..Default::default()
            }
        )
        .is_ok()
    );
    let mut format = |_: f64| Ok(max.clone() + "b");
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &options(),
            &mut StreamFactories {
                formatted_value: Some(&mut format),
                ..Default::default()
            }
        )
        .is_err()
    );
}

#[test]
fn stroked_glyph_dynamic_width_is_resolved_before_geometry_and_never_invents_fill() {
    let mut o = options();
    let dots = o.dots.as_mut().unwrap();
    dots.glyph = Some(GlyphOptions {
        shape: GlyphShape::Stroked(StrokedGlyph::Plus),
        size: 64.,
        rotation: 0.,
    });
    dots.border_width = 0.;
    let mut width = |_: &StreamDotStyleContext<'_>| Ok(2.);
    let mut color = |_: &StreamDotStyleContext<'_>| Ok(Color(50, 80, 120, 255));
    let mut border = |_: &StreamDotStyleContext<'_>| Ok(Color(90, 30, 180, 255));
    let mut geometry_calls = 0;
    let layout = stream_layout_with_factories_and_dot_factory(
        &config(),
        &categories(),
        &raw(),
        &o,
        &mut StreamFactories {
            dot_border_width: Some(&mut width),
            dot_color: Some(&mut color),
            dot_border_color: Some(&mut border),
            ..Default::default()
        },
        |ctx| {
            geometry_calls += 1;
            assert!(ctx.fill.is_none());
            assert_eq!(ctx.stroke.unwrap().width, 2.);
            assert_eq!(ctx.stroke.unwrap().color, Color(90, 30, 180, 255));
            Ok(ctx.inherit(Geometry::Circle {
                x: ctx.position[0],
                y: ctx.position[1],
                radius: 4.,
            }))
        },
    )
    .unwrap();
    assert_eq!(geometry_calls, 4);
    assert!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("stream:sample") && m.stroke.is_some())
            .all(|m| m.fill.is_none())
    );
    let mut zero = |_: &StreamDotStyleContext<'_>| Ok(0.);
    let geometry_calls = Cell::new(0);
    assert!(
        stream_layout_with_factories_and_dot_factory(
            &config(),
            &categories(),
            &raw(),
            &o,
            &mut StreamFactories {
                dot_border_width: Some(&mut zero),
                ..Default::default()
            },
            |ctx| {
                geometry_calls.set(geometry_calls.get() + 1);
                Ok(ctx.inherit(Geometry::Circle {
                    x: ctx.position[0],
                    y: ctx.position[1],
                    radius: 4.,
                }))
            }
        )
        .is_err()
    );
    assert_eq!(geometry_calls.get(), 0);
}

#[test]
fn explicit_color_is_independent_of_missing_default_lookup_but_not_invalid_configuration() {
    let mut o = options();
    o.colors.domain = Some(vec!["unrelated".into()]);
    o.colors.unknown = OrdinalColorUnknown::Missing;
    assert!(stream_layout(&config(), &categories(), &raw(), &o).is_err());
    let calls = RefCell::new(vec![]);
    let mut color = |c: &StreamLayerPathContext<'_>| {
        calls.borrow_mut().push(c.id.to_owned());
        Ok(Color(20 + c.series_index as u8, 90, 140, 255))
    };
    let layout = stream_layout_with_factories(
        &config(),
        &categories(),
        &raw(),
        &o,
        &mut StreamFactories {
            color: Some(&mut color),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(*calls.borrow(), ["a", "b", "hidden"]);
    assert_eq!(layout.colors.domain(), &[CategoryKey::from("unrelated")]);
    for layer in &layout.layers {
        assert_eq!(
            layout.colors.map(&CategoryKey::from(layer.id.clone())),
            None,
            "function output must not masquerade as an ordinal mapping cache"
        );
        assert_eq!(
            layer.color,
            Color(20 + layer.original_index as u8, 90, 140, 255)
        );
        assert_eq!(
            layout
                .scene
                .legend
                .iter()
                .find(|l| l.id == layer.id)
                .unwrap()
                .color,
            layer.color
        );
        if let Some(id) = &layer.area_mark_id {
            assert_eq!(
                layout
                    .scene
                    .marks
                    .iter()
                    .find(|m| &m.id == id)
                    .unwrap()
                    .fill,
                Some(layer.color)
            );
        }
        for sample in &layer.samples {
            if let Some(site) = layout
                .slice_interaction_binding()
                .sites
                .iter()
                .find(|s| s.mark_id == sample.mark_id)
            {
                assert_eq!(site.color, layer.color);
            }
        }
    }
    calls.borrow_mut().clear();
    o.colors.palette = OrdinalColorPalette::Custom(vec![]);
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &o,
            &mut StreamFactories {
                color: Some(&mut color),
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        calls.borrow().is_empty(),
        "invalid palette never reaches explicit override"
    );
}

#[test]
fn hidden_layer_labels_use_metadata_budget_while_visible_sample_labels_include_category_suffix() {
    let max = "界".repeat(341) + "a";
    assert_eq!(max.len(), 1024);
    let mut o = options();
    o.labels.insert("hidden".into(), max.clone());
    let ordinary = stream_layout(&config(), &categories(), &raw(), &o)
        .expect("hidden metadata label never creates sample Mark text");
    let via_default = stream_layout_with_factories(
        &config(),
        &categories(),
        &raw(),
        &o,
        &mut StreamFactories::default(),
    )
    .unwrap();
    assert_eq!(ordinary.scene, via_default.scene);
    assert_eq!(ordinary.layers, via_default.layers);
    assert_eq!(
        via_default
            .layers
            .iter()
            .find(|l| l.id == "hidden")
            .unwrap()
            .label,
        max
    );
    assert!(via_default.scene.marks.iter().all(|m| m.series != "hidden"));
    let calls = Cell::new(0);
    let mut label = |ctx: &StreamLayerPathContext<'_>| {
        calls.set(calls.get() + 1);
        Ok(if ctx.id == "hidden" {
            max.clone()
        } else {
            ctx.id.into()
        })
    };
    let explicit = stream_layout_with_factories(
        &config(),
        &categories(),
        &raw(),
        &options(),
        &mut StreamFactories {
            label: Some(&mut label),
            ..Default::default()
        },
    )
    .expect("hidden callback output has metadata-only1024byte budget");
    assert_eq!(calls.get(), 3);
    assert_eq!(
        explicit
            .layers
            .iter()
            .find(|l| l.id == "hidden")
            .unwrap()
            .label,
        max
    );
    assert!(explicit.scene.marks.iter().all(|m| m.series != "hidden"));
    o.hidden.clear();
    assert!(
        stream_layout(&config(), &categories(), &raw(), &o).is_err(),
        "visible composed sample label exceeds1024"
    );
    let mut all_visible = options();
    all_visible.hidden.clear();
    let mut label = |_: &StreamLayerPathContext<'_>| Ok(max.clone());
    assert!(
        stream_layout_with_factories(
            &config(),
            &categories(),
            &raw(),
            &all_visible,
            &mut StreamFactories {
                label: Some(&mut label),
                ..Default::default()
            }
        )
        .is_err()
    );
}
