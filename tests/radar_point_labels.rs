use std::cell::RefCell;
use std::rc::Rc;

use kartis::*;

fn axes() -> [String; 3] {
    ["speed", "cost <&> Å café", "risk"].map(str::to_owned)
}
fn data() -> [RadarSeries; 2] {
    [
        RadarSeries::new("Revenue", vec![7., 7., 3.]),
        RadarSeries::new("Costs", vec![7., 2., 0.]),
    ]
}
fn config() -> ChartConfig {
    let mut c = ChartConfig::default();
    c.theme.colors = vec![Color(220, 20, 30, 255), Color(20, 40, 220, 255)];
    c
}
fn options() -> RadarLayoutOptions {
    let mut o = RadarLayoutOptions {
        domain: Some([0., 10.]),
        axis_labels: false,
        ..Default::default()
    };
    o.grid.line.enabled = false;
    o.grid.spokes.enabled = false;
    o.points.labels = true;
    o.points.value_format = ValueFormat::Fixed(1);
    o
}
fn geometry(c: &RadarPointContext<'_>) -> Result<RadarPointGeometry> {
    Ok(c.inherit(Geometry::Circle {
        x: c.position[0],
        y: c.position[1],
        radius: 3.,
    }))
}

#[test]
#[cfg(feature = "geojson")]
fn actual_original_accessor_and_dot_point_data_match_typed_native_contexts() {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/radar-point-label/values.json")).unwrap();
    let mut checked = 0;
    for profile in oracle["cases"].as_array().unwrap() {
        let name = profile["name"].as_str().unwrap();
        let mut calls = 0;
        let scene = radar_with_point_label_factory(&config(), &axes(), &data(), &options(), |c| {
            let original = profile["labels"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["input"]["index"] == c.index && d["input"]["key"] == c.key)
                .unwrap();
            assert_eq!(c.point.axis_id, c.index);
            assert_eq!(c.point.series.id, c.key);
            assert_eq!(original["input"]["value"].as_f64().unwrap(), c.value);
            assert_eq!(original["input"]["formattedValue"], c.formatted_value);
            let text = match name {
                "index" => c.index.to_owned(),
                "key" => c.key.to_owned(),
                "value" => c.value.to_string(),
                "formattedValue" | "public-default" => c.formatted_value.to_owned(),
                "color" => format!(
                    "rgb({},{},{})",
                    c.point.color.0, c.point.color.1, c.point.color.2
                ),
                "missing" => String::new(),
                "typed-function" => format!("{}/{}: {}", c.key, c.index, c.formatted_value),
                "empty" => String::new(),
                "unicode-escaping" => format!("<&> Å café {}", c.index),
                _ => panic!(),
            };
            let expected = &original["label"];
            if let Some(s) = expected.as_str() {
                assert_eq!(text, s);
            } else if expected.is_number() {
                assert_eq!(text, expected.to_string());
            } else {
                assert!(text.is_empty());
            }
            calls += 1;
            checked += 1;
            Ok(text)
        })
        .unwrap();
        assert_eq!(calls, 6);
        assert_eq!(scene.labels.len(), 6);
    }
    assert_eq!(checked, 60);
}

#[test]
fn borrowed_owned_text_changes_only_owned_text_and_escapes_in_renderers() {
    let axes = axes();
    let mut o = options();
    o.axis_labels_by_id
        .insert(axes[0].clone(), "Speed caption".into());
    let d = data();
    let base = radar_layout(&config(), &axes, &d, &o, &Default::default()).unwrap();
    let mut prefix = String::from("<&> Å café");
    let mut calls = 0;
    let custom =
        radar_layout_with_point_label_factory(&config(), &axes, &d, &o, &Default::default(), |c| {
            calls += 1;
            if c.index == "speed" {
                assert_eq!(c.point.axis_label, "Speed caption");
            }
            Ok(format!(
                "{prefix} {}/{}: {}",
                c.key, c.index, c.formatted_value
            ))
        })
        .unwrap();
    prefix.push('!');
    assert_eq!(calls, 6);
    assert_eq!(base.interaction_binding(), custom.interaction_binding());
    assert_eq!(base.scene.marks, custom.scene.marks);
    assert_eq!(base.slices, custom.slices);
    assert_eq!(base.anchors, custom.anchors);
    let mut restored = custom.clone();
    for (actual, original) in restored.scene.labels.iter_mut().zip(&base.scene.labels) {
        assert_eq!(actual.x, original.x);
        assert_eq!(actual.y, original.y);
        actual.text.clone_from(&original.text);
    }
    assert_eq!(restored, base);
    let svg = render::to_svg(&custom.scene).unwrap();
    assert!(svg.contains("&lt;&amp;&gt; Å café"));
    assert!(!svg.contains("<&> Å café"));
    assert!(
        render::to_canvas_script(&custom.scene, "radar")
            .unwrap()
            .contains("Å café")
    );
}

#[test]
fn eligibility_is_actual_owned_label_generation_and_invalid_zero_size_rejects() {
    let c = config();
    let axes = axes();
    let d = data();
    for mode in 0..4 {
        let mut o = options();
        let expected = match mode {
            0 => {
                o.points.labels = false;
                0
            }
            1 => {
                o.hidden.insert("Revenue".into());
                3
            }
            2 => {
                o.points.enabled = false;
                0
            }
            _ => {
                o.series_styles.insert(
                    "Costs".into(),
                    RadarSeriesStyle {
                        points: Some(RadarPointOptions {
                            labels: false,
                            ..o.points.clone()
                        }),
                        ..Default::default()
                    },
                );
                3
            }
        };
        let mut calls = 0;
        let _ = radar_with_point_label_factory(&c, &axes, &d, &o, |_| {
            calls += 1;
            Ok("owned".into())
        })
        .unwrap();
        assert_eq!(calls, expected);
    }
    let mut o = options();
    o.points.size = 0.;
    let mut calls = 0;
    assert!(
        radar_with_point_label_factory(&c, &axes, &d, &o, |_| {
            calls += 1;
            Ok("not called".into())
        })
        .is_err()
    );
    assert_eq!(calls, 0);
    // With a real glyph override the legacy diameter is unused. Existing
    // builders generate owned labels even when that unused diameter is zero.
    o.points.glyph = Some(GlyphOptions::default());
    let glyph = radar_with_point_label_factory(&c, &axes, &d, &o, |_| {
        calls += 1;
        Ok("real glyph, owned text".into())
    })
    .unwrap();
    assert_eq!(calls, 6);
    assert_eq!(glyph.labels.len(), 6);
    o.points.glyph = Some(GlyphOptions {
        size: 0.,
        ..Default::default()
    });
    o.points.fill = Some(Color(1, 2, 3, 0));
    o.points.label_style.color = Some(Color(1, 2, 3, 0));
    let mut transparent_calls = 0;
    let transparent = radar_with_point_label_factory(&c, &axes, &d, &o, |_| {
        transparent_calls += 1;
        Ok("an owned slot even with transparent zero ink".into())
    })
    .unwrap();
    assert_eq!(transparent_calls, 6);
    assert_eq!(transparent.labels.len(), 6);
    // A valid caller-authored zero-ink mark still has an owned label. This is
    // distinct from an invalid configured size0 and is the pre-existing rule.
    let mut calls = 0;
    let l = radar_layout_with_point_and_label_factories(
        &c,
        &axes,
        &d,
        &options(),
        &Default::default(),
        |p| {
            Ok(p.inherit(Geometry::Circle {
                x: p.position[0],
                y: p.position[1],
                radius: 0.,
            }))
        },
        |_| {
            calls += 1;
            Ok("zero ink, owned text".into())
        },
    )
    .unwrap();
    assert_eq!(calls, 6);
    assert_eq!(l.scene.labels.len(), 6);
}

#[test]
fn geometry_then_label_order_preserves_legacy_geometry_when_labels_off() {
    let events = Rc::new(RefCell::new(vec![]));
    let a = events.clone();
    let b = events.clone();
    let l = radar_layout_with_point_and_label_factories(
        &config(),
        &axes(),
        &data(),
        &options(),
        &RadarInteractionOptions {
            direction: PolarSliceDirection::CounterClockwise,
            ..Default::default()
        },
        move |c| {
            a.borrow_mut()
                .push(format!("g:{}/{}", c.series.id, c.axis_id));
            geometry(c)
        },
        move |c| {
            b.borrow_mut().push(format!("l:{}/{}", c.key, c.index));
            Ok(c.formatted_value.into())
        },
    )
    .unwrap();
    let expected = data()
        .iter()
        .flat_map(|d| {
            [0, 2, 1].into_iter().flat_map(move |i| {
                [
                    format!("g:{}/{}", d.id, axes()[i]),
                    format!("l:{}/{}", d.id, axes()[i]),
                ]
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(*events.borrow(), expected);
    assert_eq!(l.scene.labels.len(), 6);
    let mut o = options();
    o.points.labels = false;
    let mut geom_calls = 0;
    let mut label_calls = 0;
    radar_layout_with_point_and_label_factories(
        &config(),
        &axes(),
        &data(),
        &o,
        &Default::default(),
        |c| {
            geom_calls += 1;
            geometry(c)
        },
        |_| {
            label_calls += 1;
            Ok("not called".into())
        },
    )
    .unwrap();
    assert_eq!((geom_calls, label_calls), (6, 0));
}

#[test]
fn callback_errors_are_atomic_and_geometry_error_skips_the_label() {
    let mut labels = 0;
    let e = radar_layout_with_point_and_label_factories(
        &config(),
        &axes(),
        &data(),
        &options(),
        &Default::default(),
        |_| Err(ChartError("geometry sentinel".into())),
        |_| {
            labels += 1;
            Ok("not called".into())
        },
    )
    .unwrap_err();
    assert_eq!(e.0, "geometry sentinel");
    assert_eq!(labels, 0);
    let mut labels = 0;
    let e = radar_with_point_label_factory(&config(), &axes(), &data(), &options(), |_| {
        labels += 1;
        Err(ChartError("label sentinel".into()))
    })
    .unwrap_err();
    assert_eq!(e.0, "label sentinel");
    assert_eq!(labels, 1);
    let mut bad = data();
    bad[1].values[2] = f64::NAN;
    let mut labels = 0;
    assert!(
        radar_with_point_label_factory(&config(), &axes(), &bad, &options(), |_| {
            labels += 1;
            Ok("not called".into())
        })
        .is_err()
    );
    assert_eq!(labels, 0);
    let mut bad = options();
    bad.points.value_format = ValueFormat::Fixed(10000);
    assert!(
        radar_with_point_label_factory(&config(), &axes(), &data(), &bad, |_| {
            labels += 1;
            Ok("not called".into())
        })
        .is_err()
    );
    assert_eq!(labels, 0);
}

#[test]
fn empty_and_utf8_byte_budget_follow_existing_scene_contract() {
    for text in [String::new(), "é".repeat(512)] {
        let scene = radar_with_point_label_factory(&config(), &axes(), &data(), &options(), |_| {
            Ok(text.clone())
        })
        .unwrap();
        assert!(scene.labels.iter().all(|l| l.text == text));
        scene.validate().unwrap();
    }
    for text in ["x".repeat(1025), "é".repeat(513)] {
        let e = radar_with_point_label_factory(&config(), &axes(), &data(), &options(), |_| {
            Ok(text.clone())
        })
        .unwrap_err();
        assert!(e.0.contains("point label text budget"));
    }
}

#[test]
fn shared_scale_and_combined_geometry_keep_raw_units_and_binding() {
    let d = [RadarSeries::new("Revenue", vec![1., 7., 25.])];
    let mut o = options();
    o.domain = Some([1., 100.]);
    for kind in [
        RadarValueScaleKind::Linear,
        RadarValueScaleKind::Log { base: 10. },
        RadarValueScaleKind::SymLog { constant: 1. },
        RadarValueScaleKind::Pow { exponent: 2. },
    ] {
        let scale = RadarValueScaleOptions {
            kind,
            ..Default::default()
        };
        let base = radar_layout_with_value_scale_and_point_factory(
            &config(),
            &axes(),
            &d,
            &o,
            &scale,
            &Default::default(),
            geometry,
        )
        .unwrap();
        let mut positions = vec![];
        let custom = radar_layout_with_value_scale_and_point_and_label_factories(
            &config(),
            &axes(),
            &d,
            &o,
            &scale,
            &Default::default(),
            geometry,
            |c| {
                positions.push((c.value, c.point.position));
                Ok(c.formatted_value.into())
            },
        )
        .unwrap();
        assert_eq!(format!("{custom:?}"), format!("{base:?}"));
        for (value, p) in positions {
            let radius = custom.value_scale.radius_at(value).unwrap();
            assert!(
                ((p[0] - custom.layout.center[0]).hypot(p[1] - custom.layout.center[1]) - radius)
                    .abs()
                    < 1e-8
            );
        }
        let labels = radar_layout_with_value_scale_and_point_label_factory(
            &config(),
            &axes(),
            &d,
            &o,
            &scale,
            &Default::default(),
            |c| Ok(format!("{} {}", c.index, c.formatted_value)),
        )
        .unwrap();
        assert_eq!(
            labels.layout.interaction_binding(),
            custom.layout.interaction_binding()
        );
        let scene = radar_with_value_scale_and_point_label_factory(
            &config(),
            &axes(),
            &d,
            &o,
            &scale,
            |c| Ok(c.formatted_value.into()),
        )
        .unwrap();
        assert_eq!(
            scene,
            radar_with_value_scale(&config(), &axes(), &d, &o, &scale).unwrap()
        );
    }
}

#[test]
fn formatted_callback_exactly_preserves_existing_scene_and_layout() {
    let base = radar_with_options(&config(), &axes(), &data(), &options()).unwrap();
    let labels = radar_with_point_label_factory(&config(), &axes(), &data(), &options(), |c| {
        Ok(c.formatted_value.into())
    })
    .unwrap();
    assert_eq!(labels, base);
    assert_eq!(
        render::to_svg(&labels).unwrap(),
        render::to_svg(&base).unwrap()
    );
    assert_eq!(
        render::to_canvas_script(&labels, "radar").unwrap(),
        render::to_canvas_script(&base, "radar").unwrap()
    );
    let old = radar_layout_with_point_factory(
        &config(),
        &axes(),
        &data(),
        &options(),
        &Default::default(),
        geometry,
    )
    .unwrap();
    let combined = radar_layout_with_point_and_label_factories(
        &config(),
        &axes(),
        &data(),
        &options(),
        &Default::default(),
        geometry,
        |c| Ok(c.formatted_value.into()),
    )
    .unwrap();
    assert_eq!(combined, old);
}

#[test]
fn actual_surface_hover_never_calls_the_borrowed_builder_again() {
    use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
    let mut calls = 0;
    let layout = radar_layout_with_point_label_factory(
        &config(),
        &axes(),
        &data(),
        &options(),
        &RadarInteractionOptions {
            slices: false,
            ..Default::default()
        },
        |c| {
            calls += 1;
            Ok(format!("{}/{}", c.key, c.index))
        },
    )
    .unwrap();
    assert_eq!(calls, 6);
    let point = layout.slices[0].rows[0].position;
    let id = layout.slices[0].rows[0].point_mark_id.clone();
    let mut surface = SurfaceModel::new_with_interaction(
        layout.scene.clone(),
        SurfaceOptions {
            picking: Picking::Nearest { distance: 1. },
            plot: Some(ClipRect::plot(&config())),
            ..Default::default()
        },
        Some(layout.interaction_binding()),
    )
    .unwrap();
    for _ in 0..3 {
        surface
            .apply(SurfaceInput::Move {
                id: 1,
                point: (point[0], point[1]),
            })
            .unwrap();
        assert!(surface.hovered().contains(&id));
    }
    assert_eq!(calls, 6);
}

#[test]
fn empty_source_has_no_invented_context_and_known_command_budget_precedes_callbacks() {
    let mut calls = 0;
    let scene = radar_with_point_label_factory(&config(), &axes(), &[], &options(), |_| {
        calls += 1;
        Ok("not called".into())
    })
    .unwrap();
    assert_eq!(calls, 0);
    assert!(scene.labels.is_empty());
    let axes = (0..128).map(|i| format!("axis{i}")).collect::<Vec<_>>();
    let data = (0..128)
        .map(|i| RadarSeries::new(format!("series{i}"), vec![1.; 128]))
        .collect::<Vec<_>>();
    let mut o = options();
    o.points.shape = RadarPointShape::Cross;
    let e = radar_with_point_label_factory(&config(), &axes, &data, &o, |_| {
        calls += 1;
        Ok("not called".into())
    })
    .unwrap_err();
    assert!(e.0.contains("output budget"));
    assert_eq!(calls, 0);
}

#[test]
fn known_later_tooltip_and_disabled_site_errors_precede_new_callbacks_only() {
    let mut o = options();
    o.axis_labels_by_id
        .insert(axes()[2].clone(), "x".repeat(1024));
    let mut legacy_geometry = 0;
    assert!(
        radar_layout_with_point_factory(
            &config(),
            &axes(),
            &data(),
            &o,
            &Default::default(),
            |c| {
                legacy_geometry += 1;
                geometry(c)
            }
        )
        .is_err()
    );
    assert_eq!(
        legacy_geometry, 2,
        "legacy geometry failure timing is preserved"
    );
    let mut new_geometry = 0;
    let mut labels = 0;
    assert!(
        radar_layout_with_point_and_label_factories(
            &config(),
            &axes(),
            &data(),
            &o,
            &Default::default(),
            |c| {
                new_geometry += 1;
                geometry(c)
            },
            |_| {
                labels += 1;
                Ok("not called".into())
            }
        )
        .is_err()
    );
    assert_eq!((new_geometry, labels), (0, 0));
    assert!(
        radar_with_point_label_factory(&config(), &axes(), &data(), &o, |_| {
            labels += 1;
            Ok("not called".into())
        })
        .is_err()
    );
    assert_eq!(labels, 0);
    let mut d = data();
    d[1].label = "y".repeat(1024);
    let mut o = options();
    o.series_styles.insert(
        "Costs".into(),
        RadarSeriesStyle {
            points: Some(RadarPointOptions {
                enabled: false,
                ..o.points.clone()
            }),
            ..Default::default()
        },
    );
    assert!(
        radar_layout_with_point_label_factory(
            &config(),
            &axes(),
            &d,
            &o,
            &Default::default(),
            |_| {
                labels += 1;
                Ok("not called".into())
            }
        )
        .is_err()
    );
    assert_eq!(
        labels, 0,
        "disabled typed sites also need known-tooltip admission"
    );
    let scene = radar_with_point_label_factory(&config(), &axes(), &d, &o, |_| {
        labels += 1;
        Ok("enabled scene point".into())
    })
    .unwrap();
    assert_eq!(scene.labels.len(), 3);
    assert_eq!(
        labels, 3,
        "Scene-only builds do not invent disabled interaction sites"
    );
}

#[test]
fn combined_preflight_keeps_real_custom_budget_instead_of_unused_cross_symbols() {
    let axes = (0..128).map(|i| format!("axis{i}")).collect::<Vec<_>>();
    let data = (0..128)
        .map(|i| RadarSeries::new(format!("series{i}"), vec![1.; 128]))
        .collect::<Vec<_>>();
    let mut o = options();
    o.points.shape = RadarPointShape::Cross;
    assert!(
        radar_with_options(&config(), &axes, &data, &o).is_err(),
        "ordinary Cross path budget exceeds 200k"
    );
    let mut geometry_calls = 0;
    let mut labels = 0;
    let layout = radar_layout_with_point_and_label_factories(
        &config(),
        &axes,
        &data,
        &o,
        &RadarInteractionOptions {
            slices: false,
            ..Default::default()
        },
        |p| {
            geometry_calls += 1;
            Ok(p.inherit(Geometry::Circle {
                x: p.position[0],
                y: p.position[1],
                radius: 1.,
            }))
        },
        |_| {
            labels += 1;
            Ok(String::new())
        },
    )
    .unwrap();
    assert_eq!((geometry_calls, labels), (16384, 16384));
    assert_eq!(layout.scene.labels.len(), 16384);
}
