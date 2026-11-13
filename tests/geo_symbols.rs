use std::collections::BTreeMap;

use glory::render::{GraphicPoint, PathCommand};
use kartis::interaction::Interaction;
use kartis::*;

fn land() -> Vec<GeoFeature> {
    vec![GeoFeature {
        id: "land".into(),
        label: "Land".into(),
        value: Some(7.),
        polygons: vec![vec![vec![
            [-30., -15.],
            [30., -15.],
            [30., 15.],
            [-30., 15.],
            [-30., -15.],
        ]]],
    }]
}
fn circle(mark: &Mark) -> (f64, f64, f64) {
    let Geometry::Circle { x, y, radius } = mark.geometry else {
        panic!("expected circle")
    };
    (x, y, radius)
}
fn symbols(scene: &Scene) -> BTreeMap<&str, &Mark> {
    scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("geo-symbol:"))
        .map(|m| (m.id.as_str(), m))
        .collect()
}
fn point(id: &str, value: Option<f64>) -> GeoSymbol {
    GeoSymbol::new(id, [0., 0.], value)
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}

#[test]
fn polygon_and_overlay_share_one_fit_for_all_projections_and_views() {
    let config = ChartConfig::default();
    for projection in [
        GeoProjection::Equirectangular,
        GeoProjection::Mercator,
        GeoProjection::EqualEarth,
        GeoProjection::NaturalEarth,
        GeoProjection::Orthographic,
    ] {
        for fit in [GeoFit::World, GeoFit::Data] {
            let options = GeoMapOptions {
                projection,
                fit,
                center: [12., 8.],
                rotation: 23.,
                padding: 7.,
                ..Default::default()
            };
            let mut layout = choropleth_layout(&config, &land(), &options).unwrap();
            assert_eq!(
                layout.scene,
                choropleth_with_options(&config, &land(), &options).unwrap()
            );
            let context = layout.projection.clone();
            let p = context.project([-30., -15.]).unwrap().unwrap();
            let Geometry::Path(path) = &layout.scene.marks[0].geometry else {
                panic!()
            };
            assert!(path.iter().any(|cmd| match cmd {
                PathCommand::Move(GraphicPoint { x, y })
                | PathCommand::Line(GraphicPoint { x, y }) =>
                    (*x - p[0]).abs() < 1e-8 && (*y - p[1]).abs() < 1e-8,
                _ => false,
            }));
            let data = [
                GeoSymbol::new("corner", [-30., -15.], Some(10.)),
                point("center", Some(5.)),
            ];
            let original_map = layout.scene.marks[0].clone();
            layout
                .add_symbols(
                    &data,
                    &GeoSymbolOptions {
                        clip: false,
                        ..Default::default()
                    },
                )
                .unwrap();
            assert_eq!(layout.projection, context);
            assert_eq!(layout.scene.marks[0], original_map);
            let (x, y, _) = circle(symbols(&layout.scene)["geo-symbol:6:corner"]);
            close(x, p[0]);
            close(y, p[1]);
        }
    }
}

#[test]
fn explicit_scale_translation_roll_and_antimeridian_wrap_are_shared() {
    let options = GeoMapOptions {
        center: [170., 0.],
        rotation: 15.,
        scale: Some(80.),
        translate: Some([200., 140.]),
        fit: GeoFit::Data,
        ..Default::default()
    };
    let mut layout = geographic_map_layout(&ChartConfig::default(), &land(), &options).unwrap();
    close(layout.projection.scale(), 80.);
    assert_eq!(layout.projection.translate(), [200., 140.]);
    let data = [
        GeoSymbol::new("west", [-179., 0.], Some(1.)),
        GeoSymbol::new("east", [179., 0.], Some(1.)),
    ];
    let a = layout
        .projection
        .project(data[0].position)
        .unwrap()
        .unwrap();
    let b = layout
        .projection
        .project(data[1].position)
        .unwrap()
        .unwrap();
    assert!(
        (a[0] - b[0]).abs() < 4.,
        "points beside the seam should wrap about the camera"
    );
    layout
        .add_symbols(
            &data,
            &GeoSymbolOptions {
                clip: false,
                ..Default::default()
            },
        )
        .unwrap();
    close(circle(symbols(&layout.scene)["geo-symbol:4:west"]).0, a[0]);
    close(circle(symbols(&layout.scene)["geo-symbol:4:east"]).1, b[1]);
}

#[test]
fn area_and_radius_encodings_missing_and_zero_are_distinct() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    let data = [
        point("one", Some(1.)),
        point("four", Some(4.)),
        point("zero", Some(0.)),
        point("missing", None),
    ];
    let options = GeoSymbolOptions {
        radius_range: [0., 20.],
        domain: Some([0., 4.]),
        labels: true,
        missing: GeoMissingSymbol::Draw {
            radius: 3.,
            color: Color(1, 2, 3, 255),
            label: "No data".into(),
        },
        ..Default::default()
    };
    layout.add_symbols(&data, &options).unwrap();
    let marks = symbols(&layout.scene);
    assert_eq!(marks.len(), 3);
    close(circle(marks["geo-symbol:3:one"]).2, 10.);
    close(circle(marks["geo-symbol:4:four"]).2, 20.);
    assert_eq!(circle(marks["geo-symbol:7:missing"]).2, 3.);
    assert!(marks["geo-symbol:7:missing"].label.contains("No data"));
    assert_eq!(layout.scene.label_series.len(), 3);
    let mut radial =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    radial
        .add_symbols(
            &data,
            &GeoSymbolOptions {
                size: GeoSymbolSize::Radius,
                zero_radius: 2.,
                ..options
            },
        )
        .unwrap();
    let marks = symbols(&radial.scene);
    close(circle(marks["geo-symbol:3:one"]).2, 5.);
    close(circle(marks["geo-symbol:4:zero"]).2, 2.);
}

#[test]
fn globe_backface_and_mercator_cap_cull_before_paint() {
    for projection in [GeoProjection::Orthographic, GeoProjection::Mercator] {
        let mut layout = geographic_map_layout(
            &ChartConfig::default(),
            &[],
            &GeoMapOptions {
                projection,
                ..Default::default()
            },
        )
        .unwrap();
        let hidden = if projection == GeoProjection::Orthographic {
            [150., 0.]
        } else {
            [0., 89.]
        };
        layout
            .add_symbols(
                &[
                    point("front", Some(1.)),
                    GeoSymbol::new("hidden", hidden, Some(100.)),
                ],
                &GeoSymbolOptions {
                    clip: false,
                    labels: true,
                    ..Default::default()
                },
            )
            .unwrap();
        assert_eq!(symbols(&layout.scene).len(), 1);
        assert_eq!(layout.scene.labels.len(), 1);
    }
}

#[test]
fn numeric_color_bins_and_both_legends_have_real_scene_geometry() {
    let mut layout =
        choropleth_layout(&ChartConfig::default(), &land(), &Default::default()).unwrap();
    let options = GeoSymbolOptions {
        domain: Some([0., 4.]),
        radius_range: [0., 20.],
        color: GeoSymbolColor::Value {
            domain: Some([0., 4.]),
            colors: vec![Color(10, 20, 30, 255), Color(70, 80, 90, 255)],
        },
        size_legend: Some(GeoSizeLegend {
            layout: GeoLegendOptions {
                title: "Magnitude".into(),
                ..Default::default()
            },
            values: vec![1., 4.],
        }),
        color_legend: Some(GeoLegendOptions {
            position: [120., 20.],
            title: "Bands".into(),
            ..Default::default()
        }),
        ..Default::default()
    };
    layout
        .add_symbols(
            &[
                point("lower", Some(1.)),
                point("boundary", Some(2.)),
                point("max", Some(4.)),
            ],
            &options,
        )
        .unwrap();
    let marks = symbols(&layout.scene);
    assert_eq!(
        marks["geo-symbol:5:lower"].fill,
        Some(Color(10, 20, 30, 255))
    );
    assert_eq!(
        marks["geo-symbol:8:boundary"].fill,
        Some(Color(70, 80, 90, 255))
    );
    assert_eq!(marks["geo-symbol:3:max"].fill, Some(Color(70, 80, 90, 255)));
    let sizes: Vec<_> = layout
        .scene
        .marks
        .iter()
        .filter(|m| m.id.starts_with("geo-size-legend:"))
        .collect();
    assert_eq!(sizes.len(), 2);
    close(circle(sizes[0]).2, 10.);
    close(circle(sizes[1]).2, 20.);
    assert_eq!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-color-legend:"))
            .count(),
        2
    );
    assert!(layout.scene.labels.iter().any(|l| l.text == "[0, 2)"));
    assert!(layout.scene.labels.iter().any(|l| l.text == "[2, 4]"));
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains("Magnitude") && svg.contains("Bands") && svg.contains("geo-size-legend"));
}

#[test]
fn permutation_keeps_ids_palette_tooltips_order_and_owned_categorical_legends() {
    let mut data = vec![point("z", Some(2.)), point("a", Some(1.))];
    data[0].series = "Zulu".into();
    data[1].series = "Alpha".into();
    let options = GeoSymbolOptions {
        labels: true,
        color_legend: Some(Default::default()),
        ..Default::default()
    };
    let mut first =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    let mut second = first.clone();
    first.add_symbols(&data, &options).unwrap();
    data.reverse();
    second.add_symbols(&data, &options).unwrap();
    assert_eq!(first.scene, second.scene);
    assert_eq!(first.scene.legend.len(), 2);
    assert_eq!(first.scene.legend[0].id, "Alpha");
    assert!(first.scene.label_series.values().any(|id| id == "Zulu"));
    assert!(
        symbols(&first.scene)["geo-symbol:1:a"]
            .label
            .contains("a: 1")
    );
}

#[test]
fn custom_paint_order_is_preserved_then_overlay_items_are_interleaved() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &land(), &Default::default()).unwrap();
    layout.scene.draw_order = vec![DrawItem::Mark(0)];
    layout
        .add_symbols(
            &[point("a", Some(1.))],
            &GeoSymbolOptions {
                labels: true,
                color_legend: Some(Default::default()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        layout.scene.draw_order,
        vec![
            DrawItem::Mark(0),
            DrawItem::Mark(1),
            DrawItem::Label(0),
            DrawItem::Mark(2),
            DrawItem::Label(1)
        ]
    );
    layout.scene.validate().unwrap();
}

#[test]
fn plot_clip_keeps_partly_visible_circles_and_omits_fully_outside_circles() {
    let config = ChartConfig::default();
    let mut layout = geographic_map_layout(&config, &[], &Default::default()).unwrap();
    let projected = layout.projection.project([180., 0.]).unwrap().unwrap();
    let plot = layout.projection.plot();
    close(projected[0], plot.x + plot.width);
    layout
        .add_symbols(
            &[GeoSymbol::new("edge", [180., 0.], Some(1.))],
            &GeoSymbolOptions::default(),
        )
        .unwrap();
    assert_eq!(symbols(&layout.scene).len(), 1);
    assert_eq!(symbols(&layout.scene)["geo-symbol:4:edge"].clip, Some(plot));
    let mut outside = geographic_map_layout(
        &config,
        &[],
        &GeoMapOptions {
            translate: Some([-5000., -5000.]),
            ..Default::default()
        },
    )
    .unwrap();
    outside
        .add_symbols(&[point("outside", Some(1.))], &Default::default())
        .unwrap();
    assert!(symbols(&outside.scene).is_empty());
}

#[test]
fn malformed_inputs_budget_identity_and_candidate_failures_are_atomic() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &land(), &Default::default()).unwrap();
    let before = layout.clone();
    for value in [Some(-1.), Some(f64::NAN), Some(f64::INFINITY)] {
        assert!(
            layout
                .add_symbols(&[point("bad", value)], &Default::default())
                .is_err()
        );
        assert_eq!(layout, before);
    }
    assert!(
        layout
            .add_symbols(
                &[GeoSymbol::new("bad", [181., 0.], Some(1.))],
                &Default::default()
            )
            .is_err()
    );
    assert!(
        layout
            .add_symbols(
                &[point("same", Some(1.)), point("same", Some(2.))],
                &Default::default()
            )
            .is_err()
    );
    assert!(
        layout
            .add_symbols(
                &vec![point("many", None); MAX_GEO_SYMBOLS + 1],
                &Default::default()
            )
            .is_err()
    );
    for options in [
        GeoSymbolOptions {
            domain: Some([1., 1.]),
            ..Default::default()
        },
        GeoSymbolOptions {
            color: GeoSymbolColor::Series(vec![]),
            ..Default::default()
        },
        GeoSymbolOptions {
            radius_range: [0., f64::NAN],
            ..Default::default()
        },
        GeoSymbolOptions {
            size_legend: Some(GeoSizeLegend {
                layout: Default::default(),
                values: vec![1.; 33],
            }),
            ..Default::default()
        },
        GeoSymbolOptions {
            color: GeoSymbolColor::Value {
                domain: None,
                colors: vec![Color(1, 2, 3, 255); 65],
            },
            ..Default::default()
        },
        GeoSymbolOptions {
            color_legend: Some(GeoLegendOptions {
                position: [999999., 0.],
                ..Default::default()
            }),
            ..Default::default()
        },
    ] {
        assert!(
            layout
                .add_symbols(&[point("bad", Some(1.))], &options)
                .is_err()
        );
        assert_eq!(layout, before);
    }
    let mut oversized = layout.scene.clone();
    oversized.width += 1.;
    let oversized_before = oversized.clone();
    assert!(
        layout
            .projection
            .add_symbols(
                &mut oversized,
                &[point("bad", Some(1.))],
                &Default::default()
            )
            .is_err()
    );
    assert_eq!(oversized, oversized_before);
    layout
        .add_symbols(&[point("one", Some(1.))], &Default::default())
        .unwrap();
    let after = layout.clone();
    assert!(
        layout
            .add_symbols(&[point("one", Some(1.))], &Default::default())
            .is_err()
    );
    assert_eq!(layout, after);
}

#[test]
fn finite_extreme_magnitudes_and_subnormals_have_valid_area_ratios() {
    for max in [f64::MAX, 1e-300] {
        let mut layout =
            geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
        layout
            .add_symbols(
                &[point("quarter", Some(max / 4.)), point("max", Some(max))],
                &GeoSymbolOptions {
                    radius_range: [0., 20.],
                    ..Default::default()
                },
            )
            .unwrap();
        close(
            circle(symbols(&layout.scene)["geo-symbol:7:quarter"]).2,
            10.,
        );
        close(circle(symbols(&layout.scene)["geo-symbol:3:max"]).2, 20.);
        layout.scene.validate().unwrap();
    }
}

#[test]
fn overlay_source_union_budget_is_checked_before_mutation() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    layout.scene.marks = (0..MAX_MARKS)
        .map(|i| {
            Mark::new(
                format!("base{i}"),
                Geometry::Circle {
                    x: 0.,
                    y: 0.,
                    radius: 1.,
                },
                Color(0, 0, 0, 255),
            )
        })
        .collect();
    let before = layout.clone();
    assert!(
        layout
            .add_symbols(&[point("one", Some(1.))], &Default::default())
            .is_err()
    );
    assert_eq!(layout, before);
}

#[test]
fn ten_thousand_symbols_are_bounded_and_legend_samples_are_not_interactive_data() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    let data: Vec<_> = (0..MAX_GEO_SYMBOLS)
        .map(|i| GeoSymbol::new(format!("city{i}"), [0., 0.], Some(1.)))
        .collect();
    let options = GeoSymbolOptions {
        size_legend: Some(GeoSizeLegend {
            layout: Default::default(),
            values: vec![1.],
        }),
        color_legend: Some(GeoLegendOptions {
            position: [20., 80.],
            ..Default::default()
        }),
        ..Default::default()
    };
    layout.add_symbols(&data, &options).unwrap();
    assert_eq!(symbols(&layout.scene).len(), MAX_GEO_SYMBOLS);
    let mut interaction = Interaction::default();
    for mark in layout
        .scene
        .marks
        .iter()
        .filter(|m| !m.id.starts_with("geo-symbol:"))
    {
        assert!(mark.label.is_empty());
        let (x, y) = match mark.geometry {
            Geometry::Circle { x, y, .. } | Geometry::Rect { x, y, .. } => (x + 1., y + 1.),
            _ => panic!(),
        };
        assert_eq!(interaction.pointer(&layout.scene, x, y), None);
    }
    let projected = layout.projection.project([0., 0.]).unwrap().unwrap();
    assert!(
        interaction
            .pointer(&layout.scene, projected[0], projected[1])
            .is_some()
    );
}

#[test]
fn color_legend_conflicts_reject_the_whole_layer_and_missing_row_is_drawn() {
    let mut layout =
        geographic_map_layout(&ChartConfig::default(), &[], &Default::default()).unwrap();
    layout.scene.legend.push(LegendItem {
        id: "symbols".into(),
        label: "Conflicting".into(),
        color: Color(1, 1, 1, 255),
    });
    let before = layout.clone();
    let options = GeoSymbolOptions {
        color_legend: Some(Default::default()),
        ..Default::default()
    };
    assert!(
        layout
            .add_symbols(&[point("one", Some(1.))], &options)
            .is_err()
    );
    assert_eq!(layout, before);
    layout.scene.legend.clear();
    layout
        .add_symbols(
            &[point("missing", None)],
            &GeoSymbolOptions {
                missing: GeoMissingSymbol::Draw {
                    radius: 4.,
                    color: Color(4, 5, 6, 255),
                    label: "Unavailable".into(),
                },
                ..options
            },
        )
        .unwrap();
    assert!(layout.scene.labels.iter().any(|l| l.text == "Unavailable"));
    assert_eq!(
        layout
            .scene
            .marks
            .iter()
            .filter(|m| m.id.starts_with("geo-color-legend:"))
            .count(),
        2
    );
    assert_eq!(
        symbols(&layout.scene)["geo-symbol:7:missing"].fill,
        Some(Color(4, 5, 6, 255))
    );
}
