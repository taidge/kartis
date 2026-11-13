use kartis::*;
fn config() -> ChartConfig {
    ChartConfig {
        width: 400.,
        height: 240.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    }
}
fn colors() -> ColorScaleOptions {
    ColorScaleOptions {
        transform: ColorScaleTransform::Log,
        domain: Some([1., 1000.]),
        colors: vec![Color(5, 30, 90, 255), Color(245, 220, 20, 255)],
        unknown: Color(123, 45, 67, 255),
        ..Default::default()
    }
}
fn expected(query: &str) -> Color {
    let row = include_str!("fixtures/nonlinear-color-d3.csv")
        .lines()
        .find(|line| {
            let c: Vec<_> = line.split('|').collect();
            c.first() == Some(&"C")
                && c[1] == "sequential-log-1-1-forward-span-clamp"
                && c[10] == query
        })
        .unwrap();
    let c: Vec<_> = row.split('|').collect();
    let rgb: Vec<u8> = c[13].split(':').map(|x| x.parse().unwrap()).collect();
    Color(rgb[0], rgb[1], rgb[2], rgb[3])
}
fn heat() -> HeatmapLayout {
    let data = [
        HeatCell::new("r", "a", 1.),
        HeatCell::new("r", "b", 10.),
        HeatCell::new("r", "c", 100.),
        HeatCell {
            row: "r".into(),
            column: "d".into(),
            value: None,
        },
    ];
    heatmap_layout(
        &config(),
        &data,
        &HeatmapLayoutOptions {
            colors: colors(),
            axes: HeatmapAxes::none(),
            labels: false,
            ..Default::default()
        },
    )
    .unwrap()
}
fn calendar() -> CalendarLayout {
    let data = [Some(1.), Some(10.), Some(100.), None]
        .into_iter()
        .enumerate()
        .map(|(i, value)| CalendarCell::new((2024, 1, i as u8 + 1), value))
        .collect::<Vec<_>>();
    calendar_layout(
        &config(),
        &data,
        &CalendarLayoutOptions {
            colors: colors(),
            weekday_ticks: vec![],
            month_labels: false,
            year_labels: Some(false),
            year_legend: false,
            day_border: None,
            month_border: None,
            ..Default::default()
        },
    )
    .unwrap()
}
fn features() -> Vec<GeoFeature> {
    [Some(1.), Some(10.), Some(100.), None]
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let x = i as f64 * 20. - 40.;
            GeoFeature {
                id: format!("f{i}"),
                label: format!("feature{i}"),
                value,
                polygons: vec![vec![vec![
                    [x, 0.],
                    [x + 12., 0.],
                    [x + 12., 12.],
                    [x, 12.],
                    [x, 0.],
                ]]],
            }
        })
        .collect()
}
fn map() -> ColorMappedGeoLayout {
    choropleth_with_color_scale(
        &config(),
        &features(),
        &GeoMapOptions {
            fit: GeoFit::Data,
            border: None,
            ..Default::default()
        },
        &colors(),
    )
    .unwrap()
}
#[test]
fn existing_heatmap_calendar_choropleth_consume_the_transform_and_retain_data_geometry() {
    let h = heat();
    let c = calendar();
    let g = map();
    for (i, query) in ["1", "10", "100", "missing"].into_iter().enumerate() {
        let color = expected(query);
        assert_eq!(h.cells[i].color, color);
        assert_eq!(
            h.scene
                .marks
                .iter()
                .find(|m| m.id == h.cells[i].mark_id)
                .unwrap()
                .fill,
            Some(color)
        );
        assert_eq!(c.days[i].color, color);
        assert_eq!(c.days[i].value, h.cells[i].value);
        assert_eq!(
            g.scene
                .marks
                .iter()
                .find(|m| m.series == format!("f{i}"))
                .unwrap()
                .fill,
            Some(color)
        );
    }
    assert_eq!(h.cells[3].tooltip_datum().value, None);
    assert_ne!(
        h.cells[1].color,
        ColorScaleOptions {
            transform: ColorScaleTransform::Linear,
            ..colors()
        }
        .resolve([])
        .unwrap()
        .map(Some(10.))
    );
    let old = geographic_map_layout(
        &config(),
        &features(),
        &GeoMapOptions {
            fit: GeoFit::Data,
            border: None,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(old.projection, g.projection);
    assert!(
        old.scene
            .marks
            .iter()
            .zip(&g.scene.marks)
            .all(|(a, b)| a.id == b.id && a.geometry == b.geometry && a.value == b.value)
    );
    let legacy = calendar_range_with_color_scale(
        &config(),
        &[CalendarDate::new(2024, 1, 1, 10.)],
        &CalendarRangeOptions {
            start: Some((2024, 1, 1)),
            end: Some((2024, 1, 2)),
            ..Default::default()
        },
        &colors(),
    )
    .unwrap();
    assert_eq!(legacy.scene.marks[0].fill, Some(expected("10")));
    assert_eq!(legacy.scene.marks[1].fill, Some(expected("missing")));
}
#[cfg(feature = "raster")]
#[test]
fn actual_consumer_cpu_pixels_match_original_d3_at_dpr_one_and_two() {
    use render::raster::*;
    let h = heat();
    let c = calendar();
    let g = map();
    let heat_points = h
        .cells
        .iter()
        .map(|c| c.center.unwrap())
        .collect::<Vec<_>>();
    let calendar_points = c
        .days
        .iter()
        .map(|d| [d.rect.x + d.rect.width / 2., d.rect.y + d.rect.height / 2.])
        .collect::<Vec<_>>();
    let geo_points = (0..4)
        .map(|i| {
            g.projection
                .project([i as f64 * 20. - 34., 6.])
                .unwrap()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for (scene, points) in [
        (&h.scene, heat_points),
        (&c.scene, calendar_points),
        (&g.scene, geo_points),
    ] {
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
            for (i, point) in points.iter().enumerate() {
                let x = (point[0] * ratio).floor() as usize;
                let y = (point[1] * ratio).floor() as usize;
                let pixel = &image.rgba
                    [(y * image.width as usize + x) * 4..(y * image.width as usize + x) * 4 + 4];
                let Color(r, g, b, a) = expected(["1", "10", "100", "missing"][i]);
                assert_eq!(pixel, [r, g, b, a], "ratio={ratio} point={point:?}");
            }
        }
    }
}
