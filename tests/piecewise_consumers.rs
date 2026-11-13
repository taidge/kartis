use glory::render::{GraphicPoint as P, PathCommand as C};
use kartis::{surface::*, *};

fn frame() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 60.,
            right: 20.,
            top: 20.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn profile(y: bool) -> PiecewiseScaleOptions {
    PiecewiseScaleOptions {
        domain: if y {
            vec![0., 5., 10.]
        } else {
            vec![0., 2., 10.]
        },
        range: if y {
            vec![0., 0.2, 1.]
        } else {
            vec![0., 0.75, 1.]
        },
        ..Default::default()
    }
}
fn points() -> Vec<Point> {
    vec![
        Point::new(1., 2.),
        Point::new(2., 5.),
        Point::new(6., 8.),
        Point::new(9., 9.),
    ]
}
fn line_options() -> LineOptions {
    LineOptions {
        axes: CartesianAxes {
            x: AxisOptions {
                visible: false,
                ..Default::default()
            },
            y: AxisOptions {
                visible: false,
                ..Default::default()
            },
            ..Default::default()
        },
        ..Default::default()
    }
}
fn scales() -> PiecewiseLineScaleOptions {
    PiecewiseLineScaleOptions {
        x: Some(profile(false)),
        y: Some(profile(true)),
        ..Default::default()
    }
}
fn line() -> PiecewiseLineLayout {
    line_layout_with_piecewise_scale_options(
        &frame(),
        &[Series::new("A", points())],
        &line_options(),
        &LineStyleOptions::default(),
        &scales(),
    )
    .unwrap()
}
fn scatter_options() -> ScatterLayoutOptions {
    ScatterLayoutOptions {
        scales: ScatterScaleOptions {
            x: ScatterAxisScale::Piecewise {
                options: profile(false),
            },
            y: ScatterAxisScale::Piecewise {
                options: profile(true),
            },
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
fn scatter_data() -> Vec<ScatterSeries> {
    vec![ScatterSeries::new(
        "A",
        points()
            .into_iter()
            .enumerate()
            .map(|(i, p)| ScatterDatum::new(i.to_string(), p.x, p.y.unwrap()))
            .collect(),
    )]
}
fn near(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
fn expected() -> [[f64; 2]; 4] {
    // Original d3.scaleLinear([0,2,10],[60,315,400]) and
    // d3.scaleLinear([0,5,10],[260,212,20]) at the four raw observations.
    [[187.5, 240.8], [315., 212.], [357.5, 96.8], [389.375, 58.4]]
}

#[test]
fn real_line_scatter_geometry_and_typed_picking_share_all_breakpoints() {
    let l = line();
    let s = scatter_layout(&frame(), &scatter_data(), &scatter_options()).unwrap();
    assert_eq!(l.layout.sites.len(), 4);
    assert_eq!(s.sites.len(), 4);
    for (i, target) in expected().iter().enumerate() {
        for point in [l.layout.sites[i].position, s.sites[i].position] {
            near(point[0], target[0]);
            near(point[1], target[1]);
        }
        assert_eq!(l.layout.sites[i].raw_x, points()[i].x);
        assert_eq!(s.sites[i].raw_x, CategoryKey::Number(points()[i].x));
    }
    for (scene, binding, plot, id) in [
        (
            l.layout.scene.clone(),
            l.interaction_binding(),
            l.layout.plot,
            l.layout.sites[0].mark_id.clone(),
        ),
        (
            s.scene.clone(),
            s.interaction_binding(),
            s.plot,
            s.sites[0].mark_id.clone(),
        ),
    ] {
        let mut model = SurfaceModel::new_with_interaction(
            scene,
            SurfaceOptions {
                plot: Some(plot),
                picking: Picking::Geometry,
                ..Default::default()
            },
            Some(binding),
        )
        .unwrap();
        model
            .apply(SurfaceInput::Move {
                id: 1,
                point: (187.5, 240.8),
            })
            .unwrap();
        assert!(model.hovered().contains(&id));
        model
            .apply(SurfaceInput::Move {
                id: 1,
                point: (94., 240.8),
            })
            .unwrap();
        assert!(
            model.hovered().is_empty(),
            "obsolete two-endpoint pixel remained pickable"
        );
    }
}

#[test]
fn scatter_viewport_reflow_keeps_all_knots_source_ids_and_hidden_state() {
    let baseline = scatter_layout(&frame(), &scatter_data(), &scatter_options()).unwrap();
    let mut controller = baseline
        .viewport_controller(ViewportLimits::default())
        .unwrap();
    let domains = controller
        .apply(ViewportRequest::Zoom {
            factor: 2.,
            anchor: (120., 90.),
            axes: ViewportSelection::Both,
        })
        .unwrap();
    assert!(
        baseline.viewport_scale_options(domains).is_err(),
        "endpoint-only helper silently flattened a piecewise scale"
    );
    let stops = controller.piecewise_domains().unwrap();
    let options = baseline
        .viewport_scale_options_with_piecewise(domains, &stops)
        .unwrap();
    let next = scatter_layout(
        &frame(),
        &scatter_data(),
        &ScatterLayoutOptions {
            scales: options,
            ..scatter_options()
        },
    )
    .unwrap();
    assert_eq!(
        next.x_scale.chart_scale().unwrap().domain_stops(),
        stops.x.as_ref().unwrap()
    );
    assert_eq!(
        next.y_scale.chart_scale().unwrap().domain_stops(),
        stops.left.as_ref().unwrap()
    );
    for site in &next.sites {
        let old = baseline
            .sites
            .iter()
            .find(|old| old.node_id == site.node_id)
            .unwrap();
        assert_eq!(site.mark_id, old.mark_id);
        assert_eq!(site.raw_x, old.raw_x);
        assert_eq!(site.raw_y, old.raw_y);
    }
    let hidden = scatter_layout(
        &frame(),
        &scatter_data(),
        &ScatterLayoutOptions {
            hidden: ["A".into()].into_iter().collect(),
            ..scatter_options()
        },
    )
    .unwrap();
    assert!(hidden.sites.is_empty());
    assert_eq!(
        hidden.x_scale.chart_scale().unwrap().domain_stops(),
        [0., 2., 10.]
    );
}

#[test]
fn turning_range_draws_with_explicit_inverse_branches_and_rejects_ambiguous_viewport() {
    let turning = PiecewiseScaleOptions {
        domain: vec![0., 2., 6., 10.],
        range: vec![0., 1., 0.2, 1.],
        ..Default::default()
    };
    let l = line_layout_with_piecewise_scale_options(
        &frame(),
        &[Series::new("A", points())],
        &line_options(),
        &Default::default(),
        &PiecewiseLineScaleOptions {
            x: Some(turning.clone()),
            ..Default::default()
        },
    )
    .unwrap();
    assert!(l.viewport_controller(ViewportLimits::default()).is_err());
    assert!(l.x_scale.invert(230.).is_err());
    near(l.x_scale.invert_segment(0, 230.).unwrap(), 1.);
    near(l.x_scale.invert_segment(1, 230.).unwrap(), 4.5);
    let s = scatter_layout(
        &frame(),
        &scatter_data(),
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Piecewise { options: turning },
                ..scatter_options().scales
            },
            ..scatter_options()
        },
    )
    .unwrap();
    assert!(s.viewport_controller(ViewportLimits::default()).is_err());
    assert_eq!(s.sites.len(), 4);
}

#[test]
fn full_stop_invalid_updates_are_atomic_and_plot_fraction_contract_is_explicit() {
    let l = line();
    let mut controller = l.viewport_controller(ViewportLimits::default()).unwrap();
    let before = controller.clone();
    assert!(
        controller
            .apply(ViewportRequest::TransformXY {
                transform: ViewportTransform {
                    scale_x: f64::NAN,
                    scale_y: 1.,
                    translate_x: 0.,
                    translate_y: 0.
                },
                axes: ViewportSelection::Both,
            })
            .is_err()
    );
    assert_eq!(controller, before);
    let wrong = ChartScale::with_piecewise_options(&PiecewiseScaleOptions {
        domain: vec![1., 3., 9.],
        ..profile(false)
    })
    .unwrap();
    assert!(
        controller
            .clone()
            .with_piecewise_scales(Some(&wrong), None, None)
            .is_err()
    );
    assert_eq!(controller, before);
    let mut invalid = scales();
    invalid.x.as_mut().unwrap().range = vec![0.1, 0.5, 1.];
    assert!(
        line_layout_with_piecewise_scale_options(
            &frame(),
            &[],
            &line_options(),
            &Default::default(),
            &invalid
        )
        .is_err()
    );
    assert!(
        line_layout_with_piecewise_scale_options(
            &frame(),
            &[],
            &LineOptions {
                x_domain: Some([-1., 10.]),
                ..line_options()
            },
            &Default::default(),
            &scales()
        )
        .is_err()
    );
}

#[test]
fn custom_line_geometry_receives_original_piecewise_data_anchors() {
    let mut positions = Vec::new();
    let l = line_layout_with_piecewise_scale_options_and_point_factory(
        &frame(),
        &[Series::new("A", points())],
        &line_options(),
        &Default::default(),
        &scales(),
        None,
        |context| {
            positions.push(context.position);
            let x = context.position[0] + 12.;
            let y = context.position[1] + 8.;
            Ok(context.inherit(Geometry::Path(vec![
                C::Move(P::new(x, y)),
                C::Line(P::new(x + 8., y)),
                C::Line(P::new(x + 8., y + 6.)),
                C::Line(P::new(x, y + 6.)),
                C::Close,
            ])))
        },
    )
    .unwrap();
    assert_eq!(positions.len(), 4);
    for ((point, site), expected) in positions.iter().zip(&l.layout.sites).zip(expected()) {
        near(point[0], expected[0]);
        near(point[1], expected[1]);
        assert_eq!(
            *point, site.position,
            "offset custom ink replaced the typed data anchor"
        );
    }
    l.interaction_binding().validate(&l.layout.scene).unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn actual_cpu_line_and_scatter_ink_uses_piecewise_pixels_at_both_dprs() {
    use render::raster::*;
    let l = line();
    let s = scatter_layout(&frame(), &scatter_data(), &scatter_options()).unwrap();
    for scene in [&l.layout.scene, &s.scene] {
        assert!(scene.labels.is_empty());
        for ratio in [1., 2.] {
            let image = render_raster(
                scene,
                &FontRegistry::new(),
                &RasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                },
            )
            .unwrap();
            let pixel = |p: [f64; 2]| {
                let x = (p[0] * ratio).floor() as usize;
                let y = (p[1] * ratio).floor() as usize;
                &image.rgba
                    [(y * image.width as usize + x) * 4..(y * image.width as usize + x) * 4 + 4]
            };
            let Color(r, g, b, a) = frame().theme.color(0);
            assert_eq!(pixel(expected()[0]), [r, g, b, a]);
            assert_eq!(pixel([94., 240.8]), [255, 255, 255, 255]);
        }
    }
}
