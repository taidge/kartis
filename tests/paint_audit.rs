use kartis::*;

#[test]
fn rounded_combo_bars_clip_original_caps_instead_of_rounding_the_cut_edge() {
    let config = ChartConfig::default();
    let data = [ComboSeries::new(
        "bars",
        ComboKind::Bar,
        vec![ComboPoint::new(1., 20.), ComboPoint::new(2., 5.)],
    )];
    let options = ComboOptions {
        numeric_bar_width: Some(40.),
        left: YAxisOptions {
            domain: Some([0., 10.]),
            ..Default::default()
        },
        bar_corner_radius: 10.,
        ..Default::default()
    };
    let base = combo(
        &config,
        &data,
        &ComboOptions {
            bar_corner_radius: 0.,
            ..options.clone()
        },
    )
    .unwrap();
    let rounded = combo(&config, &data, &options).unwrap();
    let base = base
        .marks
        .iter()
        .find(|mark| mark.series == "bars" && mark.value == 20.)
        .unwrap();
    let Geometry::Rect { x, y, width, .. } = base.geometry else {
        panic!("expected unrounded bar")
    };
    let mut isolated = config.scene().unwrap();
    isolated.marks.push(
        rounded
            .marks
            .iter()
            .find(|mark| mark.id == base.id)
            .unwrap()
            .clone(),
    );
    // Value 20 extends far above explicit [0,10]. Its true rounded cap lies
    // outside the viewport, so the cut through the stem has square corners.
    assert_eq!(y, config.margin.top);
    assert!(width >= 20.);
    assert!(
        isolated.hit_test(x + 0.5, y + 0.5).is_some(),
        "rounding a clipped endpoint creates a false cap at the plot boundary"
    );
}

#[test]
fn rounded_category_window_keeps_the_original_bar_edge_outside_the_clip() {
    let config = ChartConfig::default();
    let options = BarOptions {
        category_window: Some([0.1, 0.8]),
        padding: 0.,
        corner_radius: 10.,
        ..Default::default()
    };
    let rounded = bar(
        &config,
        &["A".into(), "B".into()],
        &[BarSeries::new("bars", vec![Some(5.), Some(10.)])],
        &options,
    )
    .unwrap();
    let mark = rounded
        .marks
        .iter()
        .find(|mark| mark.series == "bars" && mark.value == 5.)
        .unwrap();
    let Geometry::Path(path) = &mark.geometry else {
        panic!("expected rounded path")
    };
    let glory::render::PathCommand::Move(point) = path[0] else {
        unreachable!()
    };
    let mut isolated = config.scene().unwrap();
    isolated.marks.push(mark.clone());
    assert!(
        isolated
            .hit_test(config.margin.left + 0.5, point.y + 0.5)
            .is_some(),
        "a partial category's rounded top-left corner lies left of the visible window"
    );
}

fn inside_round_rect(point: [f64; 2], rect: [f64; 4], radius: f64) -> bool {
    let [x, y, w, h] = rect;
    if point[0] < x || point[0] > x + w || point[1] < y || point[1] > y + h {
        return false;
    }
    let radius = radius.min(w / 2.).min(h / 2.);
    let cx = point[0].clamp(x + radius, x + w - radius);
    let cy = point[1].clamp(y + radius, y + h - radius);
    (point[0] - cx).hypot(point[1] - cy) <= radius
}

#[test]
fn cropped_positive_negative_grouped_stacked_bars_match_original_shape_in_both_orientations() {
    let config = ChartConfig::default();
    let categories = vec!["A".into(), "B".into()];
    let clip = ClipRect::plot(&config);
    for horizontal in [false, true] {
        for layout in [BarLayout::Grouped, BarLayout::Stacked] {
            for sign in [-1., 1.] {
                let data = [
                    BarSeries::new("one", vec![Some(3. * sign), Some(20. * sign)]),
                    BarSeries::new("two", vec![Some(2. * sign), Some(10. * sign)]),
                ];
                let options = BarOptions {
                    horizontal,
                    layout,
                    padding: 0.,
                    category_window: Some([0.1, 0.9]),
                    value_domain: Some([-5., 5.]),
                    corner_radius: 30.,
                    ..Default::default()
                };
                let scene = bar(&config, &categories, &data, &options).unwrap();
                let base = bar(
                    &config,
                    &categories,
                    &data,
                    &BarOptions {
                        corner_radius: 0.,
                        ..options.clone()
                    },
                )
                .unwrap();
                assert_eq!(
                    scene.marks.iter().map(|m| &m.id).collect::<Vec<_>>(),
                    base.marks.iter().map(|m| &m.id).collect::<Vec<_>>()
                );
                let range = if horizontal {
                    [clip.y, clip.y + clip.height]
                } else {
                    [clip.x, clip.x + clip.width]
                };
                let bands = scales::BandScale::new(categories.clone(), range, 0.)
                    .unwrap()
                    .windowed([0.1, 0.9])
                    .unwrap();
                let value_range = if horizontal {
                    [clip.x, clip.x + clip.width]
                } else {
                    [clip.y + clip.height, clip.y]
                };
                let scale = scales::LinearScale::new([-5., 5.], value_range).unwrap();
                for mark in scene
                    .marks
                    .iter()
                    .filter(|m| m.series == "one" || m.series == "two")
                {
                    let si = usize::from(mark.series == "two");
                    let i = if mark.id.ends_with("-0") { 0 } else { 1 };
                    let width =
                        bands.bandwidth / if layout == BarLayout::Grouped { 2. } else { 1. };
                    let position = bands.at(i).unwrap()
                        + if layout == BarLayout::Grouped {
                            si as f64 * width
                        } else {
                            0.
                        };
                    let start = if layout == BarLayout::Stacked && si == 1 {
                        data[0].values[i].unwrap()
                    } else {
                        0.
                    };
                    let a = scale.map(start);
                    let b = scale.map(start + mark.value);
                    let rect = if horizontal {
                        [a.min(b), position, (a - b).abs(), width]
                    } else {
                        [position, a.min(b), width, (a - b).abs()]
                    };
                    let visible = [
                        rect[0].max(clip.x),
                        rect[1].max(clip.y),
                        (rect[0] + rect[2]).min(clip.x + clip.width),
                        (rect[1] + rect[3]).min(clip.y + clip.height),
                    ];
                    let mut isolated = config.scene().unwrap();
                    isolated.marks.push(mark.clone());
                    for x in [
                        visible[0] + 0.5,
                        (visible[0] + visible[2]) / 2.,
                        visible[2] - 0.5,
                    ] {
                        for y in [
                            visible[1] + 0.5,
                            (visible[1] + visible[3]) / 2.,
                            visible[3] - 0.5,
                        ] {
                            let expected =
                                clip.contains(x, y) && inside_round_rect([x, y], rect, 30.);
                            assert_eq!(
                                isolated.hit_test(x, y).is_some(),
                                expected,
                                "{horizontal:?}/{layout:?}/{sign}: {} at {x},{y}",
                                mark.id
                            );
                        }
                    }
                    let old = base.marks.iter().find(|m| m.id == mark.id).unwrap();
                    assert_eq!(
                        (&mark.series, &mark.label, mark.value, mark.fill),
                        (&old.series, &old.label, old.value, old.fill)
                    );
                }
            }
        }
    }
}

#[test]
fn huge_finite_values_are_bounded_and_wholly_invisible_stacks_are_removed() {
    let config = ChartConfig::default();
    for sign in [-1., 1.] {
        for scale in [ScaleKind::Linear, ScaleKind::SymLog { constant: 0.01 }] {
            let domain = if sign > 0. { [0., 10.] } else { [-10., 0.] };
            let data = [BarSeries::new("huge", vec![Some(sign * f64::MAX)])];
            for horizontal in [false, true] {
                let scene = bar(
                    &config,
                    &["A".into()],
                    &data,
                    &BarOptions {
                        horizontal,
                        value_scale: scale,
                        value_domain: Some(domain),
                        corner_radius: 10.,
                        ..Default::default()
                    },
                )
                .unwrap();
                scene.validate().unwrap();
                render::to_svg(&scene).unwrap();
                let mark = scene.marks.iter().find(|m| m.series == "huge").unwrap();
                assert_eq!(mark.value, sign * f64::MAX);
            }
            let scene = combo(
                &config,
                &[ComboSeries::new(
                    "huge",
                    ComboKind::Bar,
                    vec![ComboPoint::new(1., sign * f64::MAX)],
                )],
                &ComboOptions {
                    left: YAxisOptions {
                        domain: Some(domain),
                        scale,
                        ..Default::default()
                    },
                    bar_corner_radius: 10.,
                    ..Default::default()
                },
            )
            .unwrap();
            scene.validate().unwrap();
            render::to_svg(&scene).unwrap();
            assert_eq!(
                scene
                    .marks
                    .iter()
                    .find(|m| m.series == "huge")
                    .unwrap()
                    .value,
                sign * f64::MAX
            );
        }
    }
    let scene = combo(
        &config,
        &[
            ComboSeries::new("one", ComboKind::Bar, vec![ComboPoint::new(1., 20.)]),
            ComboSeries::new("two", ComboKind::Bar, vec![ComboPoint::new(1., 20.)]),
        ],
        &ComboOptions {
            bar_layout: BarLayout::Stacked,
            left: YAxisOptions {
                domain: Some([0., 10.]),
                ..Default::default()
            },
            bar_corner_radius: 10.,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(scene.marks.iter().any(|m| m.series == "one"));
    assert!(!scene.marks.iter().any(|m| m.series == "two"));
}

#[cfg(feature = "raster")]
#[test]
fn transparent_colored_stops_match_actual_browser_srgb_alpha_interpolation() {
    use kartis::render::raster::*;
    let mut scene = ChartConfig {
        width: 100.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    let mut mark = Mark::new(
        "gradient",
        Geometry::Rect {
            x: 0.,
            y: 0.,
            width: 100.,
            height: 100.,
        },
        Color(0, 0, 0, 255),
    );
    mark.fill_paint = Some(FillPaint::Linear {
        start: [0., 0.],
        end: [100., 0.],
        stops: vec![
            GradientStop::new(0., Color(255, 0, 0, 0)),
            GradientStop::new(1., Color(0, 0, 255, 255)),
        ],
    });
    scene.marks.push(mark);
    let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
    let pixel: [u8; 4] = image.rgba[(5 * 100 + 50) * 4..(5 * 100 + 50) * 4 + 4]
        .try_into()
        .unwrap();
    // Independently sampled in Chromium from actual CanvasGradient and an
    // sRGB SVG linearGradient; both give [190,126,191,255] at x=50.5.
    assert!(
        pixel[0].abs_diff(190) <= 2
            && pixel[1].abs_diff(126) <= 2
            && pixel[2].abs_diff(191) <= 2
            && pixel[3] == 255,
        "colored transparency changed gradient semantics: {pixel:?}"
    );
}
