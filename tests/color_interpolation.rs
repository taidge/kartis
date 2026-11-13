use kartis::*;

#[test]
fn eight_interpolation_profiles_match_the_independent_d3_queries() {
    use ColorInterpolation::*;
    use HueInterpolation::{Direct, Shortest};

    let rows = include_str!("fixtures/color-interpolation-d3.csv");
    let mut count = 0;
    for row in rows.lines().skip(2) {
        let columns: Vec<_> = row.split('|').collect();
        let number = |i: usize| columns[i].parse::<f64>().unwrap();
        let profile = match columns[0] {
            "rgb-2.2" => RgbGamma { gamma: 2.2 },
            "lab" => Lab,
            "hcl-short" => Hcl { hue: Shortest },
            "hcl-direct" => Hcl { hue: Direct },
            "hsl-short" => Hsl { hue: Shortest },
            "hsl-direct" => Hsl { hue: Direct },
            "helix-short" => Cubehelix {
                hue: Shortest,
                gamma: 1.,
            },
            "helix-direct-2.2" => Cubehelix {
                hue: Direct,
                gamma: 2.2,
            },
            _ => panic!("unknown profile in {row}"),
        };
        let colors = columns[6]
            .split(';')
            .map(|encoded| {
                let parts: Vec<_> = encoded
                    .split(':')
                    .map(|part| part.parse::<u8>().unwrap())
                    .collect();
                Color(parts[0], parts[1], parts[2], parts[3])
            })
            .collect();
        let options = ColorScaleOptions {
            kind: if columns[1] == "sequential" {
                ColorScaleKind::Sequential
            } else {
                ColorScaleKind::Diverging {
                    midpoint: number(4),
                }
            },
            domain: Some([number(2), number(3)]),
            colors,
            clamp: columns[5] == "1",
            ..Default::default()
        };
        let scale = options.resolve_with_interpolation([], profile).unwrap();
        let Color(r, g, b, a) = scale.map(Some(number(7)));
        for (index, actual) in [r, g, b, a].into_iter().enumerate() {
            assert!(
                actual.abs_diff(number(8 + index) as u8) <= 1,
                "channel {index} of {row}; got {actual}"
            );
        }
        assert_eq!(scale.profile(), profile.into());
        count += 1;
    }
    assert_eq!(count, 288);
}

#[test]
fn preserves_typed_anchors_and_default_gamma_and_rejects_invalid_profiles() {
    let options = ColorScaleOptions {
        kind: ColorScaleKind::Diverging { midpoint: 2. },
        domain: Some([10., -10.]),
        colors: vec![
            Color(250, 25, 75, 0),
            Color(0, 0, 0, 160),
            Color(30, 110, 240, 255),
        ],
        ..Default::default()
    };
    let profiles = [
        ColorInterpolation::RgbGamma { gamma: 2.2 },
        ColorInterpolation::Lab,
        ColorInterpolation::Hcl {
            hue: HueInterpolation::Shortest,
        },
        ColorInterpolation::Hsl {
            hue: HueInterpolation::Direct,
        },
        ColorInterpolation::Cubehelix {
            hue: HueInterpolation::Shortest,
            gamma: 0.7,
        },
    ];
    for profile in profiles {
        let scale = options.resolve_with_interpolation([], profile).unwrap();
        assert_eq!(scale, scale.clone());
        for (i, value) in [10., 2., -10.].into_iter().enumerate() {
            assert_eq!(scale.map(Some(value)), options.colors[i]);
        }
        assert_eq!(scale.map(Some(100.)), options.colors[0]);
        assert_eq!(scale.map(None), options.unknown);
        assert_eq!(scale.map(Some(f64::NAN)), options.unknown);
        assert!(
            scale
                .stops(3)
                .unwrap()
                .iter()
                .any(|stop| stop.value == 2. && stop.color == options.colors[1])
        );
    }
    let srgb = options.resolve([]).unwrap();
    let gamma = options
        .resolve_with_interpolation([], ColorInterpolation::RgbGamma { gamma: 1. })
        .unwrap();
    for value in [-9.9, -3.6, 0., 1.7, 2.4, 4., 8.8] {
        assert_eq!(gamma.map(Some(value)), srgb.map(Some(value)));
    }
    for invalid in [0., -1., 0.001, 101., f64::NAN, f64::INFINITY] {
        assert!(
            options
                .resolve_with_interpolation([], ColorInterpolation::RgbGamma { gamma: invalid })
                .is_err()
        );
    }
    let quantized = ColorScaleOptions {
        transform: ColorScaleTransform::Linear,
        kind: ColorScaleKind::Quantize,
        ..options
    };
    assert!(
        quantized
            .resolve_with_interpolation([], ColorInterpolation::Lab)
            .is_err()
    );
}

fn palette() -> ColorScaleOptions {
    ColorScaleOptions {
        domain: Some([0., 100.]),
        colors: vec![Color(240, 43, 95, 255), Color(50, 70, 216, 255)],
        unknown: Color(120, 140, 160, 255),
        ..Default::default()
    }
}

#[test]
fn chart_cells_and_legend_stops_share_the_selected_interpolation() {
    let profile = ColorInterpolation::Hcl {
        hue: HueInterpolation::Shortest,
    };
    let scale = palette().resolve_with_interpolation([], profile).unwrap();
    let expected = scale.map(Some(50.));
    assert_ne!(expected, palette().resolve([]).unwrap().map(Some(50.)));
    let c = ChartConfig::default();
    let mut heat = heatmap_layout_with_color_profile(
        &c,
        &[
            HeatCell {
                row: "North".into(),
                column: "Jan".into(),
                value: Some(50.),
            },
            HeatCell {
                row: "North".into(),
                column: "Feb".into(),
                value: None,
            },
        ],
        &HeatmapLayoutOptions {
            colors: palette(),
            ..Default::default()
        },
        profile,
    )
    .unwrap();
    assert_eq!(heat.color_scale, scale);
    for cell in &heat.cells {
        let mark = heat
            .scene
            .marks
            .iter()
            .find(|m| m.id == cell.mark_id)
            .unwrap();
        assert_eq!(mark.fill, Some(scale.map(cell.value)));
        assert_eq!(cell.color, scale.map(cell.value));
    }
    let legend = append_color_legend(
        &mut heat.scene,
        &heat.color_scale,
        &ColorLegendOptions {
            steps: 3,
            tick_values: Some(vec![0., 50., 100.]),
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(legend.stops[1].value, 50.);
    assert_eq!(legend.stops[1].color, expected);
    assert_eq!(legend.ticks[1].color, expected);

    let date = (2024, 2, 29);
    let calendar = calendar_layout_with_color_profile(
        &c,
        &[CalendarCell::new(date, Some(50.))],
        &CalendarLayoutOptions {
            colors: palette(),
            start: Some(date),
            end: Some((2024, 3, 1)),
            ..Default::default()
        },
        profile,
    )
    .unwrap();
    assert_eq!(calendar.color_scale, scale);
    assert_eq!(calendar.day(date).unwrap().color, expected);
    assert_eq!(calendar.day((2024, 3, 1)).unwrap().color, scale.unknown());

    let full_calendar = calendar_range_with_color_profile(
        &c,
        &[CalendarDate::new(2024, 2, 29, 50.)],
        &CalendarRangeOptions {
            start: Some(date),
            end: Some((2024, 3, 1)),
            ..Default::default()
        },
        &palette(),
        profile,
    )
    .unwrap();
    assert_eq!(full_calendar.color_scale, scale);
    assert_eq!(full_calendar.scene.marks[0].fill, Some(expected));

    let features = [GeoFeature {
        id: "example".into(),
        label: "Example".into(),
        value: Some(50.),
        polygons: vec![vec![vec![[-20., 0.], [-10., 0.], [-10., 10.], [-20., 10.]]]],
    }];
    let map =
        choropleth_with_color_profile(&c, &features, &Default::default(), &palette(), profile)
            .unwrap();
    assert_eq!(map.color_scale, scale);
    assert_eq!(
        map.scene
            .marks
            .iter()
            .find(|mark| mark.series == "example")
            .unwrap()
            .fill,
        Some(expected)
    );

    let svg = render::to_svg(&heat.scene).unwrap();
    let Color(r, g, b, _) = expected;
    assert!(svg.contains(&format!("rgba({r},{g},{b},1)")));
}

#[test]
fn named_lookup_ramp_keeps_actual_bins_and_authored_domain() {
    let scheme = ColorScheme::Viridis;
    let options = ColorScaleOptions {
        colors: vec![],
        domain: Some([256., 0.]),
        ..Default::default()
    };
    let scale = options.resolve_with_profile([], scheme).unwrap();
    assert_eq!(scale.profile(), scheme.into());
    let stops = scale.stops(2).unwrap();
    assert_eq!(stops.len(), 512);
    for (index, bin) in stops.as_chunks::<2>().0.iter().enumerate() {
        let value = 256. - index as f64;
        assert_eq!([bin[0].value, bin[1].value], [value, value - 1.]);
        assert_eq!(bin[0].color, scale.map(Some(value)));
        assert_eq!(bin[1].color, scale.map(Some(value - 0.5)));
    }

    for (midpoint, count) in [(192., 512), (256., 256), (0., 257)] {
        let diverging = ColorScaleOptions {
            transform: ColorScaleTransform::Linear,
            kind: ColorScaleKind::Diverging { midpoint },
            ..options.clone()
        }
        .resolve_with_profile([], scheme)
        .unwrap();
        let stops = diverging.stops(4).unwrap();
        assert_eq!(stops.len(), count);
        assert_eq!(stops[0].offset, 0.);
        assert_eq!(stops.last().unwrap().offset, 1.);
        assert_eq!(stops[0].color, diverging.map(Some(256.)));
        assert_eq!(stops.last().unwrap().color, diverging.map(Some(0.)));
        assert!(
            stops
                .iter()
                .rev()
                .find(|stop| stop.value == midpoint)
                .is_some_and(|stop| stop.color == scheme.sample(0.5).unwrap())
        );
    }

    let mut heat = heatmap_layout_with_color_profile(
        &Default::default(),
        &[HeatCell {
            row: "a".into(),
            column: "b".into(),
            value: Some(128.),
        }],
        &HeatmapLayoutOptions {
            colors: options.clone(),
            ..Default::default()
        },
        scheme,
    )
    .unwrap();
    assert_eq!(heat.cells[0].color, scheme.sample(0.5).unwrap());
    let legend =
        append_color_legend(&mut heat.scene, &heat.color_scale, &Default::default()).unwrap();
    assert_eq!(legend.stops, stops);
    let svg = render::to_svg(&heat.scene).unwrap();
    assert_eq!(svg.matches("<stop ").count(), 512);

    for (kind, clamp) in [
        (ColorScaleKind::Quantize, true),
        (ColorScaleKind::Sequential, false),
    ] {
        assert!(
            ColorScaleOptions {
                transform: ColorScaleTransform::Linear,
                kind,
                clamp,
                ..options.clone()
            }
            .resolve_with_profile([], scheme)
            .is_err()
        );
    }
}

#[cfg(feature = "raster")]
#[test]
fn perceptual_legend_and_the_same_sample_reach_cpu_pixels() {
    use render::raster::{FontRegistry, render_raster};

    let scale = palette()
        .resolve_with_interpolation([], ColorInterpolation::Lab)
        .unwrap();
    let mut scene = ChartConfig {
        width: 240.,
        height: 100.,
        ..Default::default()
    }
    .scene()
    .unwrap();
    append_color_legend(
        &mut scene,
        &scale,
        &ColorLegendOptions {
            position: [20., 20.],
            length: 101.,
            thickness: 20.,
            steps: 3,
            tick_count: 0,
            ..Default::default()
        },
    )
    .unwrap();
    let expected = scale.map(Some(50.));
    scene.marks.push(Mark::new(
        "same-sample",
        Geometry::Rect {
            x: 150.,
            y: 20.,
            width: 30.,
            height: 20.,
        },
        expected,
    ));
    let frame = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize| &frame.rgba[(25 * 240 + x) * 4..(25 * 240 + x) * 4 + 4];
    let Color(r, g, b, a) = expected;
    assert_eq!(pixel(160), &[r, g, b, a]);
    assert!(
        pixel(70)
            .iter()
            .zip(pixel(160))
            .all(|(a, b)| a.abs_diff(*b) <= 1),
        "ramp at fractional sample join {:?}, same value {:?}",
        pixel(70),
        pixel(160)
    );
}
