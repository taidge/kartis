//! Independent consistency checks, not an immutable original-input contract.
use std::sync::Arc;

use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 300.,
        height: 200.,
        margin: Margin {
            left: 29.,
            right: 21.,
            top: 17.,
            bottom: 23.,
        },
        ..Default::default()
    }
}
fn raw() -> Vec<ScatterSeries> {
    let datum = |id: &str, x, y, size| {
        let mut d = ScatterDatum::new(id, x, y);
        d.size_value = size;
        d.label = Some(format!("raw {id}"));
        d
    };
    vec![
        ScatterSeries::new("hidden", vec![datum("hidden", 5., 5., Some(1.))]),
        ScatterSeries::new(
            "alpha",
            vec![
                datum("zero", 2., 3., Some(0.)),
                datum("skip", 3., 4., None),
                datum("inside", 4., 6., Some(6.)),
                datum("outside", 20., 5., Some(8.)),
            ],
        ),
        ScatterSeries::new("transparent", vec![datum("point", 6., 7., Some(4.))]),
    ]
}
fn options() -> ScatterLayoutOptions {
    let axis = ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions {
            nice: ScaleNice::Off,
            ..Default::default()
        },
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(10.),
    };
    let mut o = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis.clone(),
            y: axis,
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Linear {
                    values: [0., 8.],
                    sizes: [0., 16.],
                    clamp: false,
                    missing: ScatterMissingSize::Skip,
                },
            },
            ..Default::default()
        },
        ..Default::default()
    };
    o.hidden.insert("hidden".into());
    let mut transparent = o.node.clone();
    transparent.opacity = 0.;
    o.series_styles.insert("transparent".into(), transparent);
    o
}
fn build() -> ScatterLayout {
    scatter_layout(&config(), &raw(), &options()).unwrap()
}
fn measured(_: &ScatterNodeWidgetContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    Ok(Some(PlotWidgetPlacement::AtDatum {
        offset: [-4., -4.],
        measured: TextSize {
            width: 8.,
            height: 8.,
        },
    }))
}

#[test]
fn actual_sites_keep_zero_transparent_and_off_plot_roles_with_full_raw_series() {
    let source = raw();
    let layout = build();
    let scene = layout.scene.clone();
    let mut calls = vec![];
    let packet = scatter_node_widgets(&layout, &source, 9, |c| {
        assert!(std::ptr::eq(c.layout, &layout));
        assert!(std::ptr::eq(c.series, &source[c.site.series_index]));
        assert!(std::ptr::eq(
            c.datum,
            &source[c.site.series_index].data[c.site.source_index]
        ));
        assert_eq!(&layout.scene.marks[c.site.mark_index], c.mark);
        calls.push((
            c.site.series_index,
            c.site.source_index,
            c.site.global_index,
        ));
        measured(c)
    })
    .unwrap();
    assert_eq!(calls, [(1, 0, 1), (1, 2, 3), (1, 3, 4), (2, 0, 5)]);
    assert_eq!(packet.data.len(), 4);
    assert_eq!(layout.metadata.len(), 6);
    assert_eq!(layout.scene, scene);
    assert!(packet.layout.slots.iter().all(|s| s.revision == 9));
    let alpha = packet
        .data
        .values()
        .filter(|d| d.site.series_id == "alpha")
        .collect::<Vec<_>>();
    assert_eq!(alpha.len(), 3);
    for d in &alpha {
        assert_eq!(d.series.as_ref(), &source[1]);
        assert!(Arc::ptr_eq(&alpha[0].series, &d.series));
    }
    assert_eq!(
        alpha[0].series.data[1].size_value, None,
        "omitted raw datum is retained in full series"
    );
    assert!(packet.data.values().any(|d| d.site.node_size == 0.));
    let outside = packet
        .data
        .values()
        .find(|d| d.site.node_id == "outside")
        .unwrap();
    assert!(outside.site.position[0] > layout.plot.x + layout.plot.width);
    let transparent = packet
        .data
        .values()
        .find(|d| d.site.series_id == "transparent")
        .unwrap();
    assert_eq!(layout.scene.marks[transparent.site.mark_index].opacity, 0.);
}

#[test]
fn point_domain_omissions_hidden_and_missing_size_do_not_create_component_roles() {
    let mut o = options();
    o.scales.x = ScatterAxisScale::Point {
        options: PointScaleOptions::default(),
        domain: Some(vec![2_f64.into(), 6_f64.into()]),
        window: None,
    };
    let source = raw();
    let layout = scatter_layout(&config(), &source, &o).unwrap();
    let mut calls = vec![];
    let packet = scatter_node_widgets(&layout, &source, 0, |c| {
        calls.push(c.site.global_index);
        measured(c)
    })
    .unwrap();
    assert_eq!(calls, [1, 5]);
    assert_eq!(packet.data.len(), 2);
    assert!(
        layout
            .metadata
            .iter()
            .filter(|m| m.mark_index.is_none())
            .all(|m| !packet.data.contains_key(&m.mark_id))
    );
}

#[test]
fn all_source_and_metadata_conflicts_are_preflighted_before_placement_callbacks() {
    type Change = fn(&mut ScatterLayout, &mut Vec<ScatterSeries>);
    let cases: &[(&str, Change)] = &[
        ("missing source series", |_, s| {
            s.pop();
        }),
        ("extra source datum", |_, s| {
            s[2].data.push(ScatterDatum::new("extra", 1., 2.));
        }),
        ("source series id", |_, s| s[2].id = "forged".into()),
        ("source series label", |_, s| s[2].label = "forged".into()),
        ("source datum id", |_, s| s[2].data[0].id = "forged".into()),
        ("source x", |_, s| s[2].data[0].x = 9_f64.into()),
        ("source y", |_, s| s[2].data[0].y = 9_f64.into()),
        ("source size", |_, s| s[2].data[0].size_value = Some(7.)),
        ("source invalid size", |_, s| {
            s[2].data[0].size_value = Some(f64::NAN)
        }),
        ("source datum label", |_, s| {
            s[2].data[0].label = Some("forged".into())
        }),
        ("metadata count", |l, _| {
            l.metadata.pop();
        }),
        ("metadata order", |l, _| l.metadata.swap(0, 1)),
        ("metadata owner", |l, _| {
            l.metadata[5].mark_id = "forged".into()
        }),
        ("metadata node", |l, _| {
            l.metadata[5].node_id = "forged".into()
        }),
        ("metadata series", |l, _| {
            l.metadata[5].series_id = "forged".into()
        }),
        ("metadata series index", |l, _| {
            l.metadata[5].series_index = 0
        }),
        ("metadata source index", |l, _| {
            l.metadata[5].source_index = 1
        }),
        ("metadata global index", |l, _| {
            l.metadata[5].global_index = 0
        }),
        ("metadata raw x", |l, _| l.metadata[5].raw_x = 8_f64.into()),
        ("metadata raw y", |l, _| l.metadata[5].raw_y = 8_f64.into()),
        ("metadata size value", |l, _| {
            l.metadata[5].size_value = None
        }),
        ("metadata hidden", |l, _| l.metadata[5].hidden = true),
        ("metadata position", |l, _| l.metadata[5].position = None),
        ("metadata mark index", |l, _| {
            l.metadata[5].mark_index = None
        }),
        ("metadata node size", |l, _| l.metadata[5].node_size = None),
        ("site count", |l, _| {
            l.sites.pop();
        }),
        ("site owner", |l, _| l.sites[3].mark_id = "forged".into()),
        ("site node id", |l, _| l.sites[3].node_id = "forged".into()),
        ("site series", |l, _| l.sites[3].series_id = "forged".into()),
        ("site series label", |l, _| {
            l.sites[3].series_label = "forged".into()
        }),
        ("site label", |l, _| l.sites[3].label = "forged".into()),
        ("site series index", |l, _| l.sites[3].series_index = 0),
        ("site source index", |l, _| l.sites[3].source_index = 1),
        ("site global index", |l, _| l.sites[3].global_index = 0),
        ("site raw x", |l, _| l.sites[3].raw_x = 8_f64.into()),
        ("site raw y", |l, _| l.sites[3].raw_y = 8_f64.into()),
        ("site size value", |l, _| l.sites[3].size_value = None),
        ("site node size", |l, _| l.sites[3].node_size = 999.),
        ("site position", |l, _| l.sites[3].position[0] += 1.),
        ("site bounds", |l, _| l.sites[3].bounds.width += 1.),
        ("site mark index", |l, _| l.sites[3].mark_index = 0),
        ("site label index", |l, _| l.sites[3].label_index = Some(0)),
        ("mark label", |l, _| {
            l.scene.marks.last_mut().unwrap().label = "forged".into()
        }),
        ("mark series", |l, _| {
            l.scene.marks.last_mut().unwrap().series = "forged".into()
        }),
        ("mark value", |l, _| {
            l.scene.marks.last_mut().unwrap().value = 999.
        }),
        ("mark geometry", |l, _| {
            l.scene.marks.last_mut().unwrap().geometry = Geometry::Circle {
                x: 1.,
                y: 1.,
                radius: 2.,
            }
        }),
        ("late invalid mark", |l, _| {
            l.scene.marks.last_mut().unwrap().opacity = f64::NAN
        }),
        ("plot", |l, _| l.plot.x += 1.),
        ("resolved scale", |l, _| {
            let mut o = options();
            if let ScatterAxisScale::Continuous { options, .. } = &mut o.scales.x {
                options.reverse = true;
            }
            l.x_scale = scatter_layout(&config(), &raw(), &o).unwrap().x_scale;
        }),
        ("authored scale config", |l, _| {
            if let ScatterAxisScale::Continuous { options, .. } = &mut l.scale_options.x {
                options.clamp = true;
            }
        }),
    ];
    let mut accepted_conflicts = vec![];
    for (name, change) in cases {
        let mut source = raw();
        let mut layout = build();
        change(&mut layout, &mut source);
        let mut count = 0;
        let result = scatter_node_widgets(&layout, &source, 0, |_| {
            count += 1;
            Ok(None)
        });
        if result.is_ok() {
            accepted_conflicts.push((*name, count));
        } else {
            assert_eq!(
                count, 0,
                "{name} invoked user placement before complete preflight"
            );
        }
    }
    assert!(
        accepted_conflicts.is_empty(),
        "accepted conflicts (name, callback count): {accepted_conflicts:?}"
    );
}

#[test]
fn coherent_current_paint_edits_remain_supported_and_callback_errors_stop() {
    let source = raw();
    let mut layout = build();
    let original_mark = layout.scene.marks[layout.sites[1].mark_index].clone();
    layout.sites[1].color = Color(12, 34, 56, 255);
    let packet = scatter_node_widgets(&layout, &source, 0, measured).unwrap();
    assert_eq!(
        packet.data[&layout.sites[1].mark_id].site.color,
        Color(12, 34, 56, 255)
    );
    assert_eq!(
        layout.scene.marks[layout.sites[1].mark_index], original_mark,
        "computed metadata color is not actual paint"
    );
    let site = &mut layout.sites[1];
    site.color = Color(12, 34, 56, 255);
    let mark = &mut layout.scene.marks[site.mark_index];
    mark.fill = Some(site.color);
    mark.opacity = 0.25;
    let site_id = site.mark_id.clone();
    let packet = scatter_node_widgets(&layout, &source, 0, measured).unwrap();
    assert_eq!(packet.data[&site_id].site.color, Color(12, 34, 56, 255));
    let mut calls = 0;
    assert!(
        scatter_node_widgets(&layout, &source, 0, |c| {
            calls += 1;
            if calls == 3 {
                Err(ChartError("audit placement refused".into()))
            } else {
                measured(c)
            }
        })
        .is_err()
    );
    assert_eq!(calls, 3);
    let only = scatter_node_widgets(&layout, &source, 0, |c| {
        if c.site.node_size == 0. {
            measured(c)
        } else {
            Ok(None)
        }
    })
    .unwrap();
    assert_eq!(only.data.len(), 1);
    let mut coherent_source = source;
    coherent_source[1].label = "current coherent series".into();
    for site in layout
        .sites
        .iter_mut()
        .filter(|site| site.series_index == 1)
    {
        site.series_label = coherent_source[1].label.clone();
        site.label = format!(
            "{} · {}: x: {}, y: {}",
            site.series_label,
            coherent_source[1].data[site.source_index]
                .label
                .as_ref()
                .unwrap(),
            site.formatted_x,
            site.formatted_y
        );
        layout.scene.marks[site.mark_index].label = site.label.clone();
    }
    layout
        .scene
        .legend
        .iter_mut()
        .find(|item| item.id == "alpha")
        .unwrap()
        .label = coherent_source[1].label.clone();
    let current = scatter_node_widgets(&layout, &coherent_source, 0, measured).unwrap();
    assert!(
        current
            .data
            .values()
            .filter(|d| d.site.series_id == "alpha")
            .all(|d| d.series.label == "current coherent series")
    );
}

#[test]
fn offset_custom_path_keeps_original_anchor_and_late_anchor_forgery_calls_no_user_code() {
    let source = raw();
    let mut layout = scatter_layout_with_node_factory(&config(), &source, &options(), |c| {
        let [x, y] = c.position;
        Ok(ScatterNodeGeometry::inherit(
            Geometry::Path(vec![
                C::Move(P::new(x + 10., y - 6.)),
                C::Line(P::new(x + 22., y - 6.)),
                C::Line(P::new(x + 22., y + 6.)),
                C::Line(P::new(x + 10., y + 6.)),
                C::Close,
            ]),
            c,
        ))
    })
    .unwrap();
    let packet = scatter_node_widgets(&layout, &source, 0, |c| {
        assert_eq!(
            layout.interaction_binding().anchor_for(c.mark)?,
            Some(c.site.position)
        );
        measured(c)
    })
    .unwrap();
    assert_eq!(packet.data.len(), 4);
    let last = layout.sites.last().unwrap();
    let mark = &layout.scene.marks[last.mark_index];
    let wrong = InteractionAnchor::path_points(
        &mark.geometry,
        [last.position[0] + 16., last.position[1]],
        &[0, 1, 2, 3],
    )
    .unwrap();
    layout.anchors.insert(last.mark_id.clone(), wrong);
    let mut count = 0;
    assert!(
        scatter_node_widgets(&layout, &source, 0, |_| {
            count += 1;
            Ok(None)
        })
        .is_err()
    );
    assert_eq!(count, 0);
}

#[test]
fn signed_zero_raw_values_keep_bits_and_only_coherent_source_edits_are_accepted() {
    fn bits(key: &CategoryKey) -> u64 {
        let CategoryKey::Number(value) = key else {
            panic!("original numeric variant required")
        };
        value.to_bits()
    }
    let mut datum = ScatterDatum::new("signed", -0., -0.);
    datum.size_value = Some(0.);
    let source = vec![ScatterSeries::new("sign", vec![datum])];
    let layout = scatter_layout(&config(), &source, &options()).unwrap();
    assert_eq!(layout.sites.len(), 1);
    let packet = scatter_node_widgets(&layout, &source, 0, measured).unwrap();
    let owned = packet.data.values().next().unwrap();
    for key in [
        &layout.metadata[0].raw_x,
        &layout.metadata[0].raw_y,
        &layout.sites[0].raw_x,
        &layout.sites[0].raw_y,
        &owned.series.data[0].x,
        &owned.series.data[0].y,
        &owned.site.raw_x,
        &owned.site.raw_y,
    ] {
        assert_eq!(bits(key), (-0_f64).to_bits());
    }
    let mut compatibility_frame = layout.clone();
    compatibility_frame.scene.marks[0].value = 0.;
    let compatible = scatter_node_widgets(&compatibility_frame, &source, 0, measured).unwrap();
    assert_eq!(
        bits(&compatible.data.values().next().unwrap().series.data[0].y),
        (-0_f64).to_bits(),
        "compatibility paint scalar is not original typed data"
    );
    for axis in [0, 1] {
        for edits in [1, 2, 4, 6] {
            let mut source = source.clone();
            let mut layout = layout.clone();
            if edits & 1 != 0 {
                if axis == 0 {
                    source[0].data[0].x = 0_f64.into();
                } else {
                    source[0].data[0].y = 0_f64.into();
                }
            }
            if edits & 2 != 0 {
                if axis == 0 {
                    layout.metadata[0].raw_x = 0_f64.into();
                } else {
                    layout.metadata[0].raw_y = 0_f64.into();
                }
            }
            if edits & 4 != 0 {
                if axis == 0 {
                    layout.sites[0].raw_x = 0_f64.into();
                } else {
                    layout.sites[0].raw_y = 0_f64.into();
                }
            }
            let mut calls = 0;
            assert!(
                scatter_node_widgets(&layout, &source, 0, |_| {
                    calls += 1;
                    Ok(None)
                })
                .is_err(),
                "axis={axis} edit-mask={edits} normalized inconsistent raw zero"
            );
            assert_eq!(calls, 0);
        }
    }
    let mut source = source;
    let mut layout = layout;
    source[0].data[0].x = 0_f64.into();
    source[0].data[0].y = 0_f64.into();
    layout.metadata[0].raw_x = 0_f64.into();
    layout.metadata[0].raw_y = 0_f64.into();
    layout.sites[0].raw_x = 0_f64.into();
    layout.sites[0].raw_y = 0_f64.into();
    let packet = scatter_node_widgets(&layout, &source, 0, measured).unwrap();
    let owned = packet.data.values().next().unwrap();
    for key in [
        &owned.series.data[0].x,
        &owned.series.data[0].y,
        &owned.site.raw_x,
        &owned.site.raw_y,
    ] {
        assert_eq!(
            bits(key),
            0_f64.to_bits(),
            "coherent public raw edit is supported"
        );
    }
}
