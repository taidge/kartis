use kartis::*;

#[test]
fn full_continuous_budget_fits_in_one_ramp_without_discarding_anchors() {
    let scale = ColorScaleOptions {
        domain: Some([0., 1.]),
        colors: (0..=255).map(|n| Color(n, 255 - n, n, 255)).collect(),
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    let mut scene = config().scene().unwrap();
    let legend = append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            steps: 255,
            tick_count: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(legend.stops.len(), 509);
    assert_eq!(scene.marks.len(), 1);
    let paint = scene.marks[0].fill_paint.as_ref().unwrap();
    assert_eq!(paint.stops().len(), legend.stops.len());
    for (source, rendered) in legend.stops.iter().zip(paint.stops()) {
        assert_eq!(source.offset, rendered.offset);
        assert_eq!(source.color, rendered.color);
    }
    assert!(scene.validate().is_ok());
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 220.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn scale(kind: ColorScaleKind) -> ColorScale {
    ColorScaleOptions {
        transform: ColorScaleTransform::Linear,
        kind,
        domain: Some([0., 9.]),
        colors: vec![
            Color(255, 0, 0, 255),
            Color(0, 255, 0, 255),
            Color(0, 0, 255, 255),
        ],
        unknown: Color(238, 238, 238, 255),
        clamp: true,
        thresholds: Vec::new(),
    }
    .resolve([])
    .unwrap()
}
#[test]
fn hard_edges_real_gradient_stops_ticks_and_noninteractive_swatch_match_the_scale() {
    let mut scene = config().scene().unwrap();
    let s = scale(ColorScaleKind::Quantize);
    let o = ColorLegendOptions {
        position: [30., 40.],
        length: 180.,
        thickness: 20.,
        steps: 2,
        tick_values: Some(vec![0., 3., 6., 9.]),
        format: ValueFormat::Fixed(1),
        unknown_label: Some("Missing".into()),
        title: "Magnitude".into(),
        ..Default::default()
    };
    let l = append_color_legend(&mut scene, &s, &o).unwrap();
    let bands = scene
        .marks
        .iter()
        .filter(|m| m.id.contains(":band:"))
        .collect::<Vec<_>>();
    assert_eq!(bands.len(), 3);
    assert!(
        bands
            .iter()
            .all(|m| m.fill_paint.is_none() && m.label.is_empty())
    );
    assert_eq!(
        bands.iter().map(|m| m.fill.unwrap()).collect::<Vec<_>>(),
        s.palette()
    );
    assert_eq!(
        l.ticks.iter().map(|t| t.label.as_str()).collect::<Vec<_>>(),
        ["0.0", "3.0", "6.0", "9.0"]
    );
    assert_eq!(l.ticks[1].color, s.palette()[1]);
    assert_eq!(l.ticks[1].position, [90., 60.]);
    assert_eq!(l.unknown.unwrap().x, 226.);
    assert!(
        scene
            .marks
            .iter()
            .all(|m| m.label.is_empty() && m.series.is_empty())
    );
    let mut continuous = config().scene().unwrap();
    let l = append_color_legend(
        &mut continuous,
        &scale(ColorScaleKind::Diverging { midpoint: 3. }),
        &ColorLegendOptions {
            steps: 2,
            tick_count: 0,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(
        l.stops.iter().map(|s| s.offset).collect::<Vec<_>>(),
        [0., 1. / 3., 1.]
    );
    assert_eq!(continuous.marks.len(), 1);
    let paint = continuous.marks[0].fill_paint.as_ref().unwrap();
    assert_eq!(paint.stops().len(), 3);
    assert_eq!(paint.stops()[1].color, l.stops[1].color);
    assert_eq!(
        render::to_svg(&continuous)
            .unwrap()
            .matches("<linearGradient ")
            .count(),
        1
    );
}
#[test]
fn vertical_and_constant_layouts_preserve_explicit_paint_order_and_ownership() {
    let mut scene = config().scene().unwrap();
    scene.marks.push(Mark::new(
        "datum",
        Geometry::Circle {
            x: 10.,
            y: 10.,
            radius: 2.,
        },
        Color(0, 0, 0, 255),
    ));
    scene.labels.push(Label {
        x: 10.,
        y: 10.,
        text: "Owned".into(),
        color: Color(0, 0, 0, 255),
        size: 10.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scene.label_series.insert(0, "data".into());
    scene.draw_order = vec![DrawItem::Label(0), DrawItem::Mark(0)];
    let l = append_color_legend(
        &mut scene,
        &scale(ColorScaleKind::Quantize),
        &ColorLegendOptions {
            position: [20., 20.],
            length: 120.,
            orientation: ColorLegendOrientation::Vertical,
            tick_values: Some(vec![0., 9.]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(l.ticks[0].position, [34., 140.]);
    assert_eq!(l.ticks[1].position, [34., 20.]);
    assert_eq!(
        &scene.draw_order[..2],
        &[DrawItem::Label(0), DrawItem::Mark(0)]
    );
    assert_eq!(scene.label_series[&0], "data");
    scene.validate().unwrap();
    let s = ColorScaleOptions {
        domain: Some([4., 4.]),
        ..Default::default()
    }
    .resolve([])
    .unwrap();
    let mut scene = config().scene().unwrap();
    let l = append_color_legend(&mut scene, &s, &Default::default()).unwrap();
    assert_eq!(l.ticks.len(), 1);
    assert_eq!(l.ticks[0].value, 4.);
    assert_eq!(
        scene
            .marks
            .iter()
            .filter(|m| m.id.contains(":band:"))
            .count(),
        1
    );
}
#[test]
fn invalid_legend_or_identity_collision_leaves_original_scene_exactly_accepted() {
    let s = scale(ColorScaleKind::Quantize);
    let mut scene = config().scene().unwrap();
    append_color_legend(&mut scene, &s, &Default::default()).unwrap();
    let accepted = scene.clone();
    assert!(append_color_legend(&mut scene, &s, &Default::default()).is_err());
    assert_eq!(scene, accepted);
    for case in 0..6 {
        let mut o = ColorLegendOptions {
            id: "other".into(),
            ..Default::default()
        };
        match case {
            0 => o.length = f64::NAN,
            1 => o.tick_values = Some(vec![10.]),
            2 => o.tick_values = Some(vec![3., 3.]),
            3 => o.steps = 257,
            4 => o.tick_count = 33,
            _ => o.text_size = 0.,
        };
        assert!(append_color_legend(&mut scene, &s, &o).is_err());
        assert_eq!(scene, accepted);
    }
}
#[cfg(feature = "raster")]
#[test]
fn actual_cpu_legend_has_discrete_boundaries_and_continuous_intermediate_colors() {
    use render::raster::*;
    let render = |kind| {
        let mut scene = config().scene().unwrap();
        append_color_legend(
            &mut scene,
            &scale(kind),
            &ColorLegendOptions {
                position: [20., 20.],
                length: 180.,
                thickness: 20.,
                steps: 2,
                tick_count: 0,
                ..Default::default()
            },
        )
        .unwrap();
        render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap()
    };
    let image = render(ColorScaleKind::Quantize);
    let pixel = |image: &RenderedImage, x: usize, y: usize| {
        image.rgba[(y * image.width as usize + x) * 4..(y * image.width as usize + x) * 4 + 4]
            .to_vec()
    };
    assert_eq!(pixel(&image, 79, 25), [255, 0, 0, 255]);
    assert_eq!(pixel(&image, 80, 25), [0, 255, 0, 255]);
    assert_eq!(pixel(&image, 140, 25), [0, 0, 255, 255]);
    assert_eq!(pixel(&image, 20, 19), [255, 255, 255, 255]);
    let image = render(ColorScaleKind::Sequential);
    let p = pixel(&image, 65, 25);
    assert!(p[0].abs_diff(126) <= 2 && p[1].abs_diff(129) <= 2 && p[2] == 0);
    assert_eq!(pixel(&image, 20, 19), [255; 4]);
}
