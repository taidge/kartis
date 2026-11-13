use kartis::*;
#[cfg(feature = "raster")]
#[test]
fn continuous_legend_rgba_matches_its_scale_at_the_same_value() {
    use render::raster::*;
    let scale = ColorScaleOptions {
        domain: Some([0., 1.]),
        colors: vec![Color(255, 0, 0, 0), Color(0, 0, 255, 255)],
        ..Default::default()
    }
    .resolve([])
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
            steps: 2,
            tick_count: 0,
            ..Default::default()
        },
    )
    .unwrap();
    scene.marks.push(Mark::new(
        "same-value",
        Geometry::Rect {
            x: 150.,
            y: 20.,
            width: 30.,
            height: 20.,
        },
        scale.map(Some(0.5)),
    ));
    let image = render_raster(&scene, &FontRegistry::new(), &Default::default()).unwrap();
    let pixel = |x: usize| image.rgba[(25 * 240 + x) * 4..(25 * 240 + x) * 4 + 4].to_vec();
    let legend = pixel(70);
    let datum = pixel(160);
    assert!(
        legend.iter().zip(&datum).all(|(a, b)| a.abs_diff(*b) <= 1),
        "legend={legend:?}; datum={datum:?}; scale={:?}",
        scale.map(Some(0.5))
    );
}

#[test]
fn automatic_legend_ticks_do_not_repeat_the_same_representable_value() {
    let low = 1_f64;
    let high = f64::from_bits(low.to_bits() + 1);
    for domain in [[low, high], [high, low]] {
        let scale = ColorScaleOptions {
            domain: Some(domain),
            ..Default::default()
        }
        .resolve([])
        .unwrap();
        let mut scene = ChartConfig::default().scene().unwrap();
        let legend = append_color_legend(
            &mut scene,
            &scale,
            &ColorLegendOptions {
                tick_count: 5,
                ..Default::default()
            },
        )
        .unwrap();
        let values = legend
            .ticks
            .iter()
            .map(|t| t.value.to_bits())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            values.len(),
            legend.ticks.len(),
            "auto ticks repeatedly overpaint the same positions: {:?}",
            legend.ticks
        );
    }
}
