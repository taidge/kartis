use kartis::*;
use std::collections::BTreeSet;
fn frame() -> ChartConfig {
    ChartConfig {
        width: 100.,
        height: 100.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn axis(domain: [f64; 2]) -> ScatterAxisScale {
    ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(domain[0]),
        max: ScatterBound::Fixed(domain[1]),
    }
}
fn bare() -> ScatterLayoutOptions {
    ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis([0., 10.]),
            y: axis([0., 10.]),
        },
        axes: ScatterAxes {
            top: None,
            right: None,
            bottom: None,
            left: None,
        },
        ..Default::default()
    }
}
fn near(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.),
        "{a} != {b}"
    );
}
#[test]
fn old_scatter_radius_contract_and_new_nivo_diameter_remain_distinct() {
    let c = frame();
    let old = scatter(
        &c,
        &[Series::new("A", vec![Point::new(1., 2.)])],
        &ScatterOptions::default(),
    )
    .unwrap();
    assert!(
        old.marks
            .iter()
            .any(|m| matches!(m.geometry, Geometry::Circle { radius: 5., .. }))
    );
    let current = scatter_layout(
        &c,
        &[ScatterSeries::new(
            "A",
            vec![ScatterDatum::new("one", 9., 8.)],
        )],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(current.x_scale.display_domain(), Some([0., 9.]));
    assert_eq!(current.y_scale.display_domain(), Some([0., 8.]));
    assert_eq!(current.sites[0].node_size, 9.);
    assert_eq!(current.sites[0].size_unit, ScatterSizeUnit::CircleDiameter);
    assert!(matches!(
        current.scene.marks[current.sites[0].mark_index].geometry,
        Geometry::Circle { radius: 4.5, .. }
    ));
}
#[test]
fn logical_sites_keep_zero_and_off_plot_centers_but_queries_stay_in_plot() {
    let mut data = vec![
        ScatterDatum::new("outside", -1., 5.),
        ScatterDatum::new("zero", 5., 5.),
    ];
    data[0].size_value = Some(40.);
    data[1].size_value = Some(0.);
    let o = ScatterLayoutOptions {
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Linear {
                    values: [0., 40.],
                    sizes: [0., 40.],
                    clamp: false,
                    missing: ScatterMissingSize::Error,
                },
            },
            ..Default::default()
        },
        ..bare()
    };
    let l = scatter_layout(&frame(), &[ScatterSeries::new("A", data)], &o).unwrap();
    assert_eq!(l.sites.len(), 2);
    assert_eq!(l.sites[0].position, [-10., 50.]);
    assert_eq!(l.sites[0].bounds.width, 40.);
    assert_eq!(l.scene.hit_test(2., 50.).unwrap().id, l.sites[0].mark_id);
    assert!(l.scene.hit_test(50., 50.).is_none());
    assert_eq!(l.nearest(50., 50., 0.).unwrap().unwrap().node_id, "zero");
    assert_eq!(
        l.nearest(2., 50., 100.).unwrap().unwrap().node_id,
        "outside"
    );
    assert!(l.nearest(-2., 50., 100.).unwrap().is_none());
    assert!(l.nearest(50., 50., f64::INFINITY).is_err());
}
#[test]
fn source_indices_and_business_ids_survive_hidden_and_reordered_sources() {
    let data = [
        ScatterSeries::new(
            "A:1",
            vec![
                ScatterDatum::new("x", 1., 2.),
                ScatterDatum::new("y:2", 3., 4.),
            ],
        ),
        ScatterSeries::new("A", vec![ScatterDatum::new("1:x", 5., 6.)]),
    ];
    let a = scatter_layout(&frame(), &data, &bare()).unwrap();
    assert_eq!(
        a.metadata
            .iter()
            .map(|m| m.global_index)
            .collect::<Vec<_>>(),
        [0, 1, 2]
    );
    let b = scatter_layout(
        &frame(),
        &data,
        &ScatterLayoutOptions {
            hidden: BTreeSet::from(["A:1".into()]),
            ..bare()
        },
    )
    .unwrap();
    assert_eq!(b.metadata.len(), 3);
    assert!(b.metadata[0].hidden && b.metadata[0].mark_index.is_none());
    assert_eq!(b.sites[0].global_index, 2);
    assert_eq!(b.sites[0].mark_id, a.sites[2].mark_id);
    let mut reordered = data.clone();
    reordered.reverse();
    reordered[1].data.reverse();
    let r = scatter_layout(&frame(), &reordered, &bare()).unwrap();
    for source in &a.sites {
        let now = r
            .sites
            .iter()
            .find(|s| s.mark_id == source.mark_id)
            .unwrap();
        assert_eq!(now.raw_x, source.raw_x);
        assert_eq!(now.position, source.position);
    }
    assert_ne!(a.sites[0].mark_id, a.sites[2].mark_id);
}
#[test]
fn mixed_numeric_and_reversed_point_y_navigation_preserves_real_anchor_and_lattice() {
    let data = [ScatterSeries::new(
        "A",
        ["a", "b", "c", "d"]
            .into_iter()
            .enumerate()
            .map(|(i, key)| ScatterDatum::new(key, i as f64 * 3., key))
            .collect(),
    )];
    let o = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis([0., 10.]),
            y: ScatterAxisScale::Point {
                options: PointScaleOptions {
                    reverse: true,
                    ..Default::default()
                },
                domain: None,
                window: None,
            },
        },
        ..bare()
    };
    let l = scatter_layout(&frame(), &data, &o).unwrap();
    assert_eq!(l.sites[0].position[1], 100.);
    assert_eq!(l.sites[3].position[1], 0.);
    let mut controller = l.viewport_controller(ViewportLimits::default()).unwrap();
    let domains = controller
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (30., 40.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    near(domains.x[0], 1.5);
    near(domains.x[1], 6.5);
    near(domains.left.unwrap()[0], 0.9);
    near(domains.left.unwrap()[1], 2.4);
    let next = scatter_layout(
        &frame(),
        &data,
        &ScatterLayoutOptions {
            scales: l.viewport_scale_options(domains).unwrap(),
            ..o
        },
    )
    .unwrap();
    near(
        l.x_scale.invert_coordinate(30.).unwrap(),
        next.x_scale.invert_coordinate(30.).unwrap(),
    );
    near(
        l.y_scale.invert_coordinate(40.).unwrap(),
        next.y_scale.invert_coordinate(40.).unwrap(),
    );
    assert_eq!(l.y_scale.category_domain(), next.y_scale.category_domain());
    assert_eq!(next.sites[1].raw_y, CategoryKey::from("b"));
    near(next.sites[1].position[1], 100. * (1. - 0.1 / 1.5));
    let saved = controller.domains();
    controller
        .resize(ClipRect {
            x: 0.,
            y: 0.,
            width: 200.,
            height: 80.,
        })
        .unwrap();
    assert_eq!(controller.domains(), saved);
}
#[test]
fn singleton_point_y_saturates_only_y_and_constant_continuous_navigation_rejects() {
    let data = [ScatterSeries::new(
        "A",
        vec![
            ScatterDatum::new("a", 2., "only"),
            ScatterDatum::new("b", 8., "only"),
        ],
    )];
    let o = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: axis([0., 10.]),
            y: ScatterAxisScale::Point {
                options: Default::default(),
                domain: None,
                window: None,
            },
        },
        ..bare()
    };
    let l = scatter_layout(&frame(), &data, &o).unwrap();
    let mut c = l.viewport_controller(ViewportLimits::default()).unwrap();
    let d = c
        .apply(ViewportRequest::Zoom {
            factor: 10.,
            anchor: (50., 50.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    near(d.x[1] - d.x[0], 1.);
    assert_eq!(d.left, Some([-0.5, 0.5]));
    let constant = scatter_layout(
        &frame(),
        &[ScatterSeries::new(
            "A",
            vec![ScatterDatum::new("a", 2., 3.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis([2., 2.]),
                y: axis([0., 10.]),
            },
            ..bare()
        },
    )
    .unwrap();
    assert!(constant.viewport_controller(Default::default()).is_err());
}
#[test]
fn hidden_negative_log_fallback_and_invalid_unused_sources_remain_checked() {
    let data = [ScatterSeries::new(
        "negative",
        vec![
            ScatterDatum::new("a", -100., 1.),
            ScatterDatum::new("b", -1., 2.),
        ],
    )];
    let o = ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Continuous {
                options: ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    nice: ScaleNice::Default,
                    ..Default::default()
                },
                min: ScatterBound::Auto,
                max: ScatterBound::Auto,
            },
            y: axis([0., 10.]),
        },
        hidden: BTreeSet::from(["negative".into()]),
        ..bare()
    };
    let l = scatter_layout(&frame(), &data, &o).unwrap();
    assert!(l.sites.is_empty());
    assert_eq!(l.x_scale.display_domain(), Some([-100., -1.]));
    let mut bad = data.clone();
    bad[0].data[0].x = CategoryKey::Number(f64::NAN);
    assert!(scatter_layout(&frame(), &bad, &o).is_err());
    bad[0].data[0].x = CategoryKey::from("1");
    assert!(scatter_layout(&frame(), &bad, &o).is_err());
    let underflow = ScatterLayoutOptions {
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::Circle {
                diameter: ScatterSize::Fixed(1e-300),
            },
            ..Default::default()
        },
        ..bare()
    };
    assert!(
        scatter_layout(
            &frame(),
            &[ScatterSeries::new(
                "A",
                vec![ScatterDatum::new("a", 1., 2.)]
            )],
            &underflow
        )
        .is_err()
    );
}
#[test]
fn reusable_layout_queries_reject_stale_geometry_and_paint_budgets_are_checked() {
    let source = [ScatterSeries::new(
        "A",
        vec![ScatterDatum::new("a", 2., 3.)],
    )];
    let mut l = scatter_layout(&frame(), &source, &bare()).unwrap();
    l.scene.marks.clear();
    assert!(l.nearest(20., 70., 100.).is_err());
    let data = [ScatterSeries::new(
        "A",
        (0..65)
            .map(|i| ScatterDatum::new(format!("{i}"), 2., 3.))
            .collect(),
    )];
    let stops = (0..256)
        .map(|i| GradientStop::new(i as f64 / 255., Color(50, 100, 150, 255)))
        .collect();
    let o = ScatterLayoutOptions {
        node: ScatterNodeStyle {
            paint: Some(FillPaint::Linear {
                start: [0., 0.],
                end: [100., 100.],
                stops,
            }),
            ..Default::default()
        },
        ..bare()
    };
    assert!(scatter_layout(&frame(), &data, &o).is_err());
    let log = scatter_layout(
        &frame(),
        &[ScatterSeries::new(
            "A",
            vec![ScatterDatum::new("a", 2., 3.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions {
                        kind: ScaleKind::Log,
                        ..Default::default()
                    },
                    min: ScatterBound::Fixed(1.),
                    max: ScatterBound::Fixed(10.),
                },
                y: axis([0., 10.]),
            },
            ..bare()
        },
    )
    .unwrap();
    assert!(
        log.viewport_scale_options(ResolvedDomains {
            x: [-1., 2.],
            left: Some([0., 10.]),
            right: None
        })
        .is_err()
    );
}
#[test]
fn stroke_only_legend_uses_actual_style_ink_and_node_override_stays_raw() {
    use glory::render::{GraphicStroke, StrokeCap};
    let blue = Color(10, 60, 180, 255);
    let red = Color(220, 30, 40, 255);
    let green = Color(10, 180, 60, 255);
    let mut a = ScatterDatum::new("a", 2., 3.);
    a.color = Some(green);
    let mut row = ScatterSeries::new("A", vec![a, ScatterDatum::new("b", 4., 5.)]);
    row.color = Some(red);
    let o = ScatterLayoutOptions {
        node: ScatterNodeStyle {
            shape: ScatterNodeShape::StrokedGlyph {
                shape: StrokedGlyph::Triangle,
                size: ScatterSize::Fixed(100.),
                rotation: 0.2,
            },
            stroke: Some(GraphicStroke {
                color: blue,
                width: 2.,
                cap: StrokeCap::Round,
            }),
            ..Default::default()
        },
        ..bare()
    };
    let l = scatter_layout(&frame(), std::slice::from_ref(&row), &o).unwrap();
    assert_eq!(l.scene.legend[0].color, blue);
    assert_eq!(l.sites[0].color, green);
    assert_eq!(l.sites[1].color, blue);
    assert!(
        l.sites
            .iter()
            .all(|s| l.scene.marks[s.mark_index].fill.is_none())
    );
    let hidden = scatter_layout(
        &frame(),
        &[row],
        &ScatterLayoutOptions {
            hidden: BTreeSet::from(["A".into()]),
            ..o
        },
    )
    .unwrap();
    assert_eq!(hidden.scene.legend[0].color, blue);
    assert!(hidden.sites.is_empty());
}
