use kartis::*;
fn scale(transform: ColorScaleTransform, domain: [f64; 2], kind: ColorScaleKind) -> ColorScale {
    ColorScaleOptions {
        transform,
        domain: Some(domain),
        kind,
        colors: if matches!(kind, ColorScaleKind::Diverging { .. }) {
            vec![
                Color(0, 0, 255, 255),
                Color(0, 255, 0, 255),
                Color(255, 0, 0, 255),
            ]
        } else {
            vec![Color(0, 0, 255, 255), Color(255, 0, 0, 255)]
        },
        ..Default::default()
    }
    .resolve([])
    .unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-9, "{a} != {b}");
}
#[test]
fn actual_legend_ticks_follow_transformed_axis_in_both_orientations_and_directions() {
    for reverse in [false, true] {
        for orientation in [
            ColorLegendOrientation::Horizontal,
            ColorLegendOrientation::Vertical,
        ] {
            let domain = if reverse { [1000., 1.] } else { [1., 1000.] };
            let s = scale(
                ColorScaleTransform::Log,
                domain,
                ColorScaleKind::Diverging { midpoint: 10. },
            );
            let mut scene = ChartConfig::default().scene().unwrap();
            let legend = append_color_legend(
                &mut scene,
                &s,
                &ColorLegendOptions {
                    position: [20., 30.],
                    length: 300.,
                    orientation,
                    tick_values: Some(vec![1., 10., 100., 1000.]),
                    ..Default::default()
                },
            )
            .unwrap();
            for (i, tick) in legend.ticks.iter().enumerate() {
                let t = if reverse {
                    1. - i as f64 / 3.
                } else {
                    i as f64 / 3.
                };
                if orientation == ColorLegendOrientation::Horizontal {
                    close(tick.position[0], 20. + 300. * t);
                } else {
                    close(tick.position[1], 30. + 300. * (1. - t));
                }
            }
            let midpoint = legend.stops.iter().find(|s| s.value == 10.).unwrap();
            close(midpoint.offset, if reverse { 2. / 3. } else { 1. / 3. });
            assert_eq!(midpoint.color, Color(0, 255, 0, 255));
            scene.validate().unwrap();
            let snapshot = scene.clone();
            assert!(
                append_color_legend(
                    &mut scene,
                    &s,
                    &ColorLegendOptions {
                        tick_values: Some(vec![0.]),
                        ..Default::default()
                    }
                )
                .is_err()
            );
            assert_eq!(scene, snapshot);
        }
    }
}
#[test]
fn reciprocal_and_signed_axes_preserve_monotone_samples_and_lookup_hard_edges() {
    for (transform, domain, midpoint) in [
        (ColorScaleTransform::Pow { exponent: -1. }, [1., 100.], 10.),
        (ColorScaleTransform::Sqrt, [-9., 16.], 0.),
        (
            ColorScaleTransform::SymLog { constant: 2. },
            [-100., 20.],
            0.,
        ),
        (ColorScaleTransform::Log, [-1000., -1.], -10.),
    ] {
        for kind in [
            ColorScaleKind::Sequential,
            ColorScaleKind::Diverging { midpoint },
        ] {
            let s = scale(transform, domain, kind);
            let stops = s.stops(17).unwrap();
            assert!(stops.windows(2).all(|v| v[0].offset <= v[1].offset));
            for stop in &stops {
                close(s.position(stop.value).unwrap(), stop.offset);
                assert_eq!(stop.color, s.map(Some(stop.value)));
            }
            let lookup = ColorScaleOptions {
                transform,
                domain: Some(domain),
                kind,
                ..Default::default()
            }
            .resolve_with_profile([], ColorScheme::Viridis)
            .unwrap();
            let stops = lookup.stops(2).unwrap();
            assert_eq!(stops.len(), 512);
            for pair in stops.as_chunks::<2>().0 {
                assert_eq!(pair[0].color, pair[1].color);
                close(lookup.position(pair[0].value).unwrap(), pair[0].offset);
                close(lookup.position(pair[1].value).unwrap(), pair[1].offset);
            }
        }
    }
}
#[cfg(feature = "raster")]
#[test]
fn actual_gradient_pixels_follow_nonlinear_values_and_diverging_nonuniform_center() {
    use render::raster::*;
    for kind in [
        ColorScaleKind::Sequential,
        ColorScaleKind::Diverging { midpoint: 10. },
    ] {
        let s = scale(ColorScaleTransform::Log, [1., 1000.], kind);
        let mut scene = ChartConfig {
            width: 360.,
            height: 100.,
            ..Default::default()
        }
        .scene()
        .unwrap();
        append_color_legend(
            &mut scene,
            &s,
            &ColorLegendOptions {
                position: [20., 20.],
                length: 301.,
                thickness: 20.,
                steps: 256,
                tick_count: 0,
                ..Default::default()
            },
        )
        .unwrap();
        for ratio in [1., 2.] {
            let image = render_raster(
                &scene,
                &FontRegistry::new(),
                &RasterOptions {
                    pixel_ratio: ratio,
                    ..Default::default()
                },
            )
            .unwrap();
            for x in [50., 100., 150., 200., 250.] {
                let px = ((20. + x) * ratio).floor() as usize;
                let py = (25. * ratio) as usize;
                let pixel = &image.rgba[(py * image.width as usize + px) * 4
                    ..(py * image.width as usize + px) * 4 + 4];
                let value = s
                    .value_at((px as f64 / ratio + 0.5 / ratio - 20.) / 301.)
                    .unwrap();
                let Color(r, g, b, a) = s.map(Some(value));
                assert!(
                    pixel
                        .iter()
                        .zip([r, g, b, a])
                        .all(|(a, b)| a.abs_diff(b) <= 1),
                    "{kind:?} ratio={ratio} x={x}: {pixel:?} != {:?}",
                    [r, g, b, a]
                );
            }
        }
    }
}

#[test]
fn endpoint_midpoints_keep_d3_degenerate_direction_and_lookup_colors() {
    for (transform, domain) in [
        (ColorScaleTransform::Log, [1000., 1.]),
        (ColorScaleTransform::Pow { exponent: -1. }, [1., 100.]),
        (ColorScaleTransform::Sqrt, [9., -4.]),
        (ColorScaleTransform::SymLog { constant: 1. }, [20., -50.]),
    ] {
        for midpoint in domain {
            let lookup = ColorScaleOptions {
                transform,
                domain: Some(domain),
                kind: ColorScaleKind::Diverging { midpoint },
                ..Default::default()
            }
            .resolve_with_profile([], ColorScheme::Viridis)
            .unwrap();
            let stops = lookup.stops(17).unwrap();
            if midpoint == domain[0] {
                assert_eq!(stops.len(), 2);
                assert_eq!(stops[0].color, stops[1].color);
            }
            for pair in stops.windows(2).filter(|p| p[0].offset < p[1].offset) {
                let offset = (pair[0].offset + pair[1].offset) / 2.;
                assert_eq!(
                    lookup.map(Some(lookup.value_at(offset).unwrap())),
                    pair[0].color
                );
            }
        }
    }
}
