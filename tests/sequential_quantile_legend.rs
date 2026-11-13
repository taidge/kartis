use kartis::*;
fn options() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 128, 0, 255)],
        ..Default::default()
    }
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 160.,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        ..Default::default()
    }
}
fn legend() -> ColorLegendOptions {
    ColorLegendOptions {
        position: [20., 20.],
        length: 200.,
        thickness: 20.,
        tick_count: 0,
        ..Default::default()
    }
}
#[test]
fn exact_data_axis_jumps_ignore_steps_and_preserve_isolated_endpoint() {
    let scale = options().resolve([Some(0.), Some(10.)]).unwrap();
    let stops = scale.stops(2).unwrap();
    assert_eq!(stops, scale.stops(256).unwrap());
    assert_eq!(
        stops.iter().map(|s| s.offset).collect::<Vec<_>>(),
        vec![0., 1., 1.]
    );
    assert_eq!(
        stops.iter().map(|s| s.value).collect::<Vec<_>>(),
        vec![0., 10., 10.]
    );
    assert_eq!(
        stops.iter().map(|s| s.color).collect::<Vec<_>>(),
        vec![scale.palette()[0], scale.palette()[0], scale.palette()[1]]
    );
    let mut scene = config().scene().unwrap();
    let output = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            tick_values: Some(vec![0., 5., 10.]),
            ..legend()
        },
    )
    .unwrap();
    assert_eq!(output.stops, stops);
    assert_eq!(output.ticks[1].position, [120., 40.]);
    assert_eq!(output.ticks[1].color, scale.palette()[0]);
    assert_eq!(output.ticks[2].color, scale.palette()[1]);
    let paint = scene.marks[0].fill_paint.as_ref().unwrap();
    assert_eq!(
        paint.stops().iter().map(|s| s.offset).collect::<Vec<_>>(),
        vec![0., 1., 1.]
    );
    let svg = render::to_svg(&scene).unwrap();
    assert_eq!(svg.matches("<stop offset=\"1\"").count(), 2);
    let canvas = render::to_canvas_script(&scene, "plot").unwrap();
    assert_eq!(canvas.matches("gradient.addColorStop(1,").count(), 2);
}
#[test]
fn duplicate_rank_and_equal_rgba_plateaus_keep_surviving_boundaries() {
    let scale = ColorScaleOptions {
        domain: Some([-4., 12.]),
        ..options()
    }
    .resolve([Some(-2.), Some(0.), Some(0.), Some(3.), Some(10.)])
    .unwrap();
    let stops = scale.stops(16).unwrap();
    assert_eq!(
        stops.iter().map(|s| s.offset).collect::<Vec<_>>(),
        vec![0., 0.25, 0.25, 7. / 16., 7. / 16., 14. / 16., 14. / 16., 1.]
    );
    assert_eq!(stops[2].color, scale.map(Some(0.)));
    assert_eq!(stops[1].color, scale.map(Some(-1.)));
    // Backend conversion need only preserve the ordering of true jumps;
    // a normal fraction such as 1/3 is valid without exact f32 equality.
    let thirds = options().resolve([Some(0.), Some(1.), Some(3.)]).unwrap();
    let third_stops = thirds.stops(2).unwrap();
    assert_eq!(third_stops[1].offset, 1. / 3.);
    assert_ne!(third_stops[1].offset, (third_stops[1].offset as f32) as f64);
    let a = Color(40, 50, 60, 128);
    let b = Color(160, 30, 40, 128);
    let plateaus = ColorScaleOptions {
        colors: vec![a, a, b, a, a],
        ..options()
    }
    .resolve((0..5).map(|n| Some(n as f64)))
    .unwrap();
    let p = plateaus.stops(2).unwrap();
    assert_eq!(
        p.iter().map(|s| s.offset).collect::<Vec<_>>(),
        vec![0., 0.5, 0.5, 0.75, 0.75, 1.]
    );
    assert_eq!(
        p.iter().map(|s| s.color).collect::<Vec<_>>(),
        vec![a, a, b, b, a, a]
    );
    let flat = ColorScaleOptions {
        colors: vec![a, a],
        ..options()
    }
    .resolve((0..20_000).map(|n| Some(n as f64)))
    .unwrap();
    assert_eq!(flat.stops(16).unwrap().len(), 2);
}
#[test]
fn exact_512_budget_and_unrepresentable_legends_fail_transactionally() {
    let full = ColorScaleOptions {
        domain: Some([0., 256.]),
        ..options()
    }
    .resolve((0..256).map(|n| Some(n as f64)))
    .unwrap();
    assert_eq!(full.stops(16).unwrap().len(), 512);
    let mut scene = config().scene().unwrap();
    append_color_legend(&mut scene, &full, &legend()).unwrap();
    assert_eq!(
        scene.marks[0].fill_paint.as_ref().unwrap().stops().len(),
        512
    );
    let excessive = options().resolve((0..257).map(|n| Some(n as f64))).unwrap();
    assert_eq!(excessive.sample_domain().unwrap().len(), 257);
    assert_ne!(excessive.map(Some(100.)), excessive.unknown());
    assert!(excessive.stops(16).is_err());
    let accepted = scene.clone();
    assert!(append_color_legend(&mut scene, &excessive, &legend()).is_err());
    assert_eq!(scene, accepted);
    for values in [
        vec![0., 0.5, 0.50000001, 1.],
        vec![0., f64::from_bits(1), 1.],
        vec![0., 1. - f64::EPSILON, 1.],
        vec![-f64::MAX, 0., f64::from_bits(1), f64::MAX],
    ] {
        let scale = options().resolve(values.into_iter().map(Some)).unwrap();
        assert!(scale.stops(2).is_err());
        assert!(append_color_legend(&mut scene, &scale, &legend()).is_err());
        assert_eq!(scene, accepted);
    }
}
#[test]
fn orientation_ownership_and_scene_total_stop_budget_are_preserved() {
    let scale = options().resolve([Some(0.), Some(1.), Some(2.)]).unwrap();
    let mut scene = config().scene().unwrap();
    let original = Mark::new(
        "business",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 2.,
            height: 2.,
        },
        Color(1, 2, 3, 255),
    );
    scene.marks.push(original.clone());
    scene.draw_order.push(DrawItem::Mark(0));
    let layout = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            orientation: ColorLegendOrientation::Vertical,
            length: 100.,
            ..legend()
        },
    )
    .unwrap();
    assert_eq!(scene.marks[0], original);
    assert_eq!(scene.draw_order.len(), 2);
    assert_eq!(layout.stops.len(), 5);
    match scene.marks[1].fill_paint.as_ref().unwrap() {
        FillPaint::Linear { start, end, .. } => {
            assert_eq!(*start, [40., 120.]);
            assert_eq!(*end, [40., 20.]);
        }
        _ => panic!("expected one gradient"),
    };
    assert!(scene.marks[1].label.is_empty());
    let mut full = config().scene().unwrap();
    for i in 0..32 {
        let mut mark = original.clone();
        mark.id = format!("base-{i}");
        mark.fill_paint = Some(FillPaint::Linear {
            start: [0., 0.],
            end: [1., 0.],
            stops: (0..512)
                .map(|j| GradientStop::new(j as f64 / 511., Color(j as u8, 0, 0, 255)))
                .collect(),
        });
        full.marks.push(mark);
    }
    full.validate().unwrap();
    let previous = full.clone();
    assert!(append_color_legend(&mut full, &scale, &legend()).is_err());
    assert_eq!(full, previous);
}
#[cfg(feature = "raster")]
#[test]
fn cpu_plateau_pixels_match_actual_data_colors_without_alpha_seams() {
    use render::raster::*;
    let scale = ColorScaleOptions {
        domain: Some([-4., 12.]),
        colors: vec![Color(0, 0, 0, 128), Color(255, 128, 0, 128)],
        ..options()
    }
    .resolve([Some(-2.), Some(0.), Some(0.), Some(3.), Some(10.)])
    .unwrap();
    let mut scene = config().scene().unwrap();
    scene.background = Color(255, 255, 255, 255);
    append_color_legend(&mut scene, &scale, &legend()).unwrap();
    let values = [-3., 1., 5., 11.];
    for (i, v) in values.iter().enumerate() {
        scene.marks.push(Mark::new(
            format!("data-{i}"),
            Geometry::Rect {
                x: 280.,
                y: 20. + i as f64 * 30.,
                width: 30.,
                height: 20.,
            },
            scale.map(Some(*v)),
        ));
    }
    for dpr in [1., 2.] {
        let im = render_raster(
            &scene,
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        let at = |x: f64, y: f64| {
            let i =
                (((y * dpr).floor() as usize) * im.width as usize + (x * dpr).floor() as usize) * 4;
            im.rgba[i..i + 4].to_vec()
        };
        for (i, v) in values.iter().enumerate() {
            let x = 20. + scale.position(*v).unwrap() * 200.;
            assert_eq!(at(x, 30.), at(290., 30. + i as f64 * 30.));
        }
        // Integer-aligned empirical jumps remain hard at both pixel ratios;
        // translucent colors have neither a blended band nor a doubled-alpha seam.
        for (left, right, left_plateau, right_plateau) in [(69., 70., 0, 1), (194., 195., 2, 3)] {
            assert_eq!(at(left, 30.), at(290., 30. + left_plateau as f64 * 30.));
            assert_eq!(at(right, 30.), at(290., 30. + right_plateau as f64 * 30.));
            assert_ne!(at(left, 30.), at(right, 30.));
        }
        let mut wrong = scene.clone();
        if let Some(FillPaint::Linear { stops, .. }) = &mut wrong.marks[0].fill_paint {
            *stops = vec![
                GradientStop::new(0., scale.palette()[0]),
                GradientStop::new(1., scale.palette()[1]),
            ];
        }
        let bad = render_raster(
            &wrong,
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        assert_ne!(bad.rgba, im.rgba, "smoothing negative must be detectable");
    }
}
#[cfg(feature = "raster")]
#[test]
fn cpu_n2_endpoint_keeps_last_in_bar_pixel_on_the_left_plateau() {
    use render::raster::*;
    let scale = options().resolve([Some(0.), Some(10.)]).unwrap();
    let mut scene = config().scene().unwrap();
    scene.background = Color(255, 255, 255, 255);
    append_color_legend(&mut scene, &scale, &legend()).unwrap();
    for dpr in [1., 2.] {
        let im = render_raster(
            &scene,
            &FontRegistry::new(),
            &RasterOptions {
                pixel_ratio: dpr,
                ..Default::default()
            },
        )
        .unwrap();
        for x in (20. * dpr) as usize..(220. * dpr) as usize {
            let i = ((30. * dpr) as usize * im.width as usize + x) * 4;
            assert_eq!(&im.rgba[i..i + 4], &[0, 0, 0, 255]);
        }
        let i = ((30. * dpr) as usize * im.width as usize + (220. * dpr) as usize) * 4;
        assert_eq!(&im.rgba[i..i + 4], &[255; 4]);
    }
    assert_eq!(scale.map(Some(10.)), scale.palette()[1]);
}
