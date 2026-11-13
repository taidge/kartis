use std::collections::BTreeMap;

use glory::render::PathCommand;
use kartis::surface::{Picking, SurfaceInput, SurfaceModel, SurfaceOptions};
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 400.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn profile(kind: ScaleKind) -> ContinuousScaleOptions {
    ContinuousScaleOptions {
        kind,
        ..Default::default()
    }
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-8, "{a} != {b}");
}
fn pick(scene: Scene, binding: InteractionBinding, position: [f64; 2], id: &str) {
    let mut model = SurfaceModel::new_with_interaction(
        scene,
        SurfaceOptions {
            plot: Some(ClipRect::plot(&frame())),
            picking: Picking::Nearest { distance: 1. },
            ..Default::default()
        },
        Some(binding),
    )
    .unwrap();
    model
        .apply(SurfaceInput::Move {
            id: 1,
            point: (position[0], position[1]),
        })
        .unwrap();
    assert_eq!(model.hovered(), &[id.to_string()]);
}
fn tick(scene: &Scene, id: &str) -> [f64; 2] {
    let m = scene.marks.iter().find(|m| m.id == id).unwrap();
    let Geometry::Path(path) = &m.geometry else {
        panic!()
    };
    let PathCommand::Move(p) = path[0] else {
        panic!()
    };
    [p.x, p.y]
}

#[test]
fn line_axes_points_sites_and_viewport_share_power_coordinates() {
    let out = line_layout_with_scale_options(
        &frame(),
        &[Series::new(
            "s",
            vec![Point::new(0., 0.), Point::new(4., 4.), Point::new(16., 16.)],
        )],
        &LineOptions {
            x_domain: Some([0., 16.]),
            y_domain: Some([0., 16.]),
            axes: CartesianAxes {
                x: AxisOptions {
                    tick_values: Some(vec![4.]),
                    ..Default::default()
                },
                y: AxisOptions {
                    tick_values: Some(vec![4.]),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &LineScaleOptions {
            x: profile(ScaleKind::Sqrt),
            y: profile(ScaleKind::Pow { exponent: 2. }),
        },
    )
    .unwrap();
    let site = &out.layout.sites[1];
    assert_eq!((site.raw_x, site.raw_y), (4., 4.));
    assert_eq!(site.position, [200., 340.]);
    assert_eq!(tick(&out.layout.scene, "axis-x-tick-0")[0], 200.);
    assert_eq!(tick(&out.layout.scene, "axis-y-tick-0")[1], 340.);
    let mark = out
        .layout
        .scene
        .marks
        .iter()
        .find(|m| m.id == site.mark_id)
        .unwrap();
    assert!(matches!(
        mark.geometry,
        Geometry::Circle {
            x: 200.,
            y: 340.,
            ..
        }
    ));
    pick(
        out.layout.scene.clone(),
        out.interaction_binding(),
        site.position,
        &site.mark_id,
    );
    let mut v = out.viewport_controller(ViewportLimits::default()).unwrap();
    let d = v
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (160., 160.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    close(d.x[0], 1.);
    close(d.x[1], 9.);
    close(d.left.unwrap()[0], 8.);
    close(d.left.unwrap()[1], (192_f64).sqrt());
}

#[test]
fn bar_geometry_uses_positive_powers_for_both_orientations_and_signed_stacks() {
    for horizontal in [false, true] {
        for (kind, delta) in [
            (ScaleKind::Sqrt, 160.),
            (ScaleKind::Pow { exponent: 2. }, 20.),
        ] {
            let o = BarOptions {
                horizontal,
                value_domain: Some([0., 16.]),
                value_scale: kind,
                ..Default::default()
            };
            let data = [BarSeries::new("A", vec![Some(4.)])];
            let out = bar_layout_with_scale_options(
                &frame(),
                &["Q".into()],
                &data,
                &o,
                &BarStyleOptions::default(),
                &profile(kind),
            )
            .unwrap();
            let rect = out.layout.bars[0].visible_rect;
            if horizontal {
                close(rect.x, 40.);
                close(rect.width, delta);
            } else {
                close(rect.y, 360. - delta);
                close(rect.height, delta);
            }
            let old = bar(&frame(), &["Q".into()], &data, &o).unwrap();
            let m = old
                .marks
                .iter()
                .find(|m| m.series == "A" && !m.label.is_empty())
                .unwrap();
            let Geometry::Rect { width, height, .. } = m.geometry else {
                panic!()
            };
            close(if horizontal { width } else { height }, delta);
        }
    }
    let out = bar_layout_with_scale_options(
        &frame(),
        &["Q".into()],
        &[
            BarSeries::new("A", vec![Some(4.)]),
            BarSeries::new("B", vec![Some(-4.)]),
        ],
        &BarOptions {
            layout: BarLayout::Stacked,
            value_domain: Some([-16., 16.]),
            ..Default::default()
        },
        &BarStyleOptions::default(),
        &profile(ScaleKind::Pow { exponent: 2. }),
    )
    .unwrap();
    for b in &out.layout.bars {
        close(b.visible_rect.height, 10.);
    }
    assert!(
        bar_layout_with_scale_options(
            &frame(),
            &["Q".into()],
            &[BarSeries::new("A", vec![Some(4.)])],
            &BarOptions::default(),
            &BarStyleOptions::default(),
            &profile(ScaleKind::Pow { exponent: -1. })
        )
        .is_err()
    );
}

#[test]
fn mixed_numeric_bars_and_reciprocal_line_preserve_independent_values_and_anchors() {
    let mut line = ComboSeries::new(
        "reciprocal",
        ComboKind::Line,
        vec![
            ComboPoint::new(4., 4.).identified("middle"),
            ComboPoint::new(16., 16.),
        ],
    );
    line.axis = YAxisSide::Right;
    let bars = ComboSeries::new(
        "bars",
        ComboKind::Bar,
        vec![
            ComboPoint::new(4., 4.).identified("middle"),
            ComboPoint::new(16., 16.),
        ],
    );
    let out = combo_layout_with_scale_options(
        &frame(),
        &[bars, line],
        &ComboOptions {
            x: ComboXAxis::Numeric {
                domain: Some([0., 16.]),
                scale: ScaleKind::Linear,
            },
            left: YAxisOptions {
                domain: Some([0., 16.]),
                ..Default::default()
            },
            right: YAxisOptions {
                domain: Some([1., 16.]),
                ..Default::default()
            },
            numeric_bar_width: Some(16.),
            ..Default::default()
        },
        &ComboScaleOptions {
            x: Some(profile(ScaleKind::Sqrt)),
            left: Some(profile(ScaleKind::Pow { exponent: 2. })),
            right: Some(profile(ScaleKind::Pow { exponent: -1. })),
        },
    )
    .unwrap();
    let bar = out
        .sites
        .iter()
        .find(|s| s.series_id == "bars" && s.source_index == 0)
        .unwrap();
    assert_eq!(bar.raw_y, 4.);
    assert_eq!(bar.stack_interval, Some([0., 4.]));
    close(bar.value_position[0], 200.);
    close(bar.value_position[1], 340.);
    let l = out
        .sites
        .iter()
        .find(|s| s.series_id == "reciprocal" && s.source_index == 0)
        .unwrap();
    close(l.position[0], 200.);
    close(l.position[1], 104.);
    pick(
        out.scene.clone(),
        out.interaction_binding(),
        l.position,
        &l.mark_id,
    );
}

#[test]
fn scatter_parallel_and_swarm_paint_the_shared_transformed_positions() {
    let axis = |kind| ScatterAxisScale::Continuous {
        options: profile(kind),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(16.),
    };
    let scatter = scatter_layout(
        &frame(),
        &[ScatterSeries::new(
            "A",
            vec![ScatterDatum::new("four", 4., 4.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis(ScaleKind::Sqrt),
                y: axis(ScaleKind::Pow { exponent: 2. }),
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(scatter.sites[0].position, [200., 340.]);
    pick(
        scatter.scene.clone(),
        scatter.interaction_binding(),
        scatter.sites[0].position,
        &scatter.sites[0].mark_id,
    );
    let profiles = BTreeMap::from([
        (
            "sqrt".into(),
            ParallelScaleOptions {
                bounds: [Some(0.), Some(16.)],
                scale: profile(ScaleKind::Sqrt),
            },
        ),
        (
            "square".into(),
            ParallelScaleOptions {
                bounds: [Some(0.), Some(16.)],
                scale: profile(ScaleKind::Pow { exponent: 2. }),
            },
        ),
    ]);
    let parallel = parallel_layout_with_scale_options(
        &frame(),
        &[
            ParallelDimension::new("sqrt"),
            ParallelDimension::new("square"),
        ],
        &[ParallelDatum::new("row", vec![Some(4.), Some(4.)])],
        &ParallelCoordinatesOptions::default(),
        &profiles,
    )
    .unwrap();
    close(parallel.rows[0].points[0].unwrap()[1], 200.);
    close(parallel.rows[0].points[1].unwrap()[1], 340.);
    for (kind, expected) in [
        (ScaleKind::Sqrt, 200.),
        (ScaleKind::Pow { exponent: 2. }, 340.),
    ] {
        let out = swarm_layout(
            &frame(),
            &[SwarmPoint::new("four", "A", 4.)],
            &SwarmLayoutOptions {
                orientation: SwarmOrientation::Vertical,
                value_domain: Some([0., 16.]),
                value_scale: profile(kind),
                value_fit: SwarmValueFit::Clip,
                ..Default::default()
            },
        )
        .unwrap();
        close(out.sites[0].position[1], expected);
        pick(
            out.scene.clone(),
            out.interaction_binding(),
            out.sites[0].position,
            &out.sites[0].mark_id,
        );
    }
}

#[test]
fn boxplot_bullet_radar_and_stream_keep_raw_statistics_and_project_real_ink() {
    let observations = [0., 4., 16.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| BoxObservation::new(i.to_string(), "G", "S", Some(v)))
        .collect::<Vec<_>>();
    for (kind, expected) in [
        (ScaleKind::Sqrt, 200.),
        (ScaleKind::Pow { exponent: 2. }, 340.),
    ] {
        let boxes = boxplot_layout_with_scale_options(
            &frame(),
            &observations,
            &BoxPlotLayoutOptions {
                domain: Some([0., 16.]),
                ..Default::default()
            },
            &profile(kind),
        )
        .unwrap();
        assert_eq!(boxes.layout.boxes[0].quantiles[2], 4.);
        close(boxes.map_statistic(4.).unwrap(), expected);
        let median = boxes
            .layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == format!("{}:median", boxes.layout.boxes[0].id))
            .unwrap();
        let Geometry::Path(path) = &median.geometry else {
            panic!()
        };
        assert!(matches!(path[0],PathCommand::Move(p) if (p.y-expected).abs()<1e-8));

        let bullets = bullet_layout_with_scale_options(
            &frame(),
            &[BulletDatum {
                id: "row".into(),
                ranges: vec![16.],
                measures: vec![4.],
                markers: vec![4.],
            }],
            &BulletLayoutOptions {
                min: Some(0.),
                max: Some(16.),
                ..Default::default()
            },
            &profile(kind),
        )
        .unwrap();
        let part = bullets.layout.rows[0]
            .parts
            .iter()
            .find(|p| p.kind == BulletPartKind::Measure)
            .unwrap();
        assert_eq!(part.raw_value, 4.);
        assert_eq!(part.interval, [0., 4.]);
        let Geometry::Rect { x, width, .. } = part.geometry else {
            panic!()
        };
        close(x, 40.);
        close(width, 360. - expected);

        let radar = radar_layout_with_value_scale(
            &frame(),
            &["a".into(), "b".into(), "c".into()],
            &[RadarSeries::new("S", vec![4., 4., 4.])],
            &RadarLayoutOptions {
                domain: Some([0., 16.]),
                radius_ratio: 1.,
                ..Default::default()
            },
            &RadarValueScaleOptions {
                kind: if kind == ScaleKind::Sqrt {
                    RadarValueScaleKind::Sqrt
                } else {
                    RadarValueScaleKind::Pow { exponent: 2. }
                },
                ..Default::default()
            },
            &RadarInteractionOptions::default(),
        )
        .unwrap();
        let radius = (360. - expected) / 2.;
        close(radar.value_scale.radius_at(4.).unwrap(), radius);
        close(radar.layout.slices[0].rows[0].position[1], 200. - radius);
        assert_eq!(radar.layout.slices[0].rows[0].raw_value, 4.);

        let stream = stream_layout(
            &frame(),
            &[CategoryKey::from("a"), CategoryKey::from("b")],
            &[BarSeries::new("S", vec![Some(4.), Some(16.)])],
            &StreamLayoutOptions {
                y_bounds: [Some(0.), Some(16.)],
                y_scale: profile(kind),
                stack: stack::StackOptions {
                    offset: stack::StackOffset::Zero,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();
        let sample = &stream.layers[0].samples[0];
        assert_eq!((sample.value, sample.y0, sample.y1), (Some(4.), 0., 4.));
        close(sample.screen_y[0], 360.);
        close(sample.screen_y[1], expected);
        stream.scene.validate().unwrap();
    }
}

#[test]
fn reciprocal_y_line_and_empty_automatic_scales_do_not_invent_zero_domains() {
    let reciprocal = profile(ScaleKind::Pow { exponent: -1. });
    let out = line_layout_with_scale_options(
        &frame(),
        &[Series::new(
            "s",
            vec![Point::new(0., 1.), Point::new(4., 4.), Point::new(16., 16.)],
        )],
        &LineOptions {
            x_domain: Some([0., 16.]),
            y_domain: Some([1., 16.]),
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &LineScaleOptions {
            x: profile(ScaleKind::Linear),
            y: reciprocal,
        },
    )
    .unwrap();
    close(out.layout.sites[1].position[1], 104.);
    let empty = line_layout_with_scale_options(
        &frame(),
        &[],
        &LineOptions::default(),
        &LineStyleOptions::default(),
        &LineScaleOptions {
            x: reciprocal,
            y: reciprocal,
        },
    )
    .unwrap();
    assert_eq!(empty.x_scale.source_domain(), [1., 10.]);
    assert_eq!(empty.y_scale.source_domain(), [1., 10.]);
    let axis = || ScatterAxisScale::Continuous {
        options: reciprocal,
        min: ScatterBound::Auto,
        max: ScatterBound::Auto,
    };
    let scatter = scatter_layout(
        &frame(),
        &[],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis(),
                y: axis(),
            },
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(scatter.x_scale.source_domain(), Some([1., 10.]));
    let parallel = parallel_layout_with_scale_options(
        &frame(),
        &[ParallelDimension::new("x"), ParallelDimension::new("y")],
        &[],
        &ParallelCoordinatesOptions::default(),
        &BTreeMap::from([(
            "x".into(),
            ParallelScaleOptions {
                scale: reciprocal,
                ..Default::default()
            },
        )]),
    )
    .unwrap();
    assert_eq!(parallel.dimensions[0].source_domain, [1., 10.]);
    let swarm = swarm_layout(
        &frame(),
        &[],
        &SwarmLayoutOptions {
            value_scale: reciprocal,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(swarm.value_scale.source_domain(), [1., 10.]);
    let bullet = bullet_layout_with_scale_options(
        &frame(),
        &[],
        &BulletLayoutOptions::default(),
        &reciprocal,
    )
    .unwrap();
    assert_eq!(
        bullet.shared_value_scale.unwrap().source_domain(),
        [1., 10.]
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_paints_the_power_position_instead_of_the_linear_fallback() {
    use render::raster::*;
    let mut c = frame();
    c.theme.colors = vec![Color(21, 145, 155, 255)];
    let axis = |kind| ScatterAxisScale::Continuous {
        options: profile(kind),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(16.),
    };
    let out = scatter_layout(
        &c,
        &[ScatterSeries::new(
            "A",
            vec![ScatterDatum::new("four", 4., 4.)],
        )],
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis(ScaleKind::Pow { exponent: 2. }),
                y: axis(ScaleKind::Sqrt),
            },
            axes: ScatterAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            ..Default::default()
        },
    )
    .unwrap();
    let image = render_raster(&out.scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let start = (y * 400 + x) * 4;
        &image.rgba[start..start + 4]
    };
    assert_eq!(pixel(60, 200), &[21, 145, 155, 255]);
    assert_eq!(pixel(120, 280), &[255, 255, 255, 255]);
}
