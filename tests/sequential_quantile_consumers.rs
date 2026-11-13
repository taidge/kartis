use kartis::*;

const UNKNOWN: Color = Color(233, 17, 99, 255);
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
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        unknown: UNKNOWN,
        ..Default::default()
    }
}
fn values() -> [Option<f64>; 6] {
    [Some(0.), Some(0.), Some(5.), Some(10.), Some(10.), None]
}
fn expected() -> [Color; 6] {
    [
        Color(64, 64, 64, 255),
        Color(64, 64, 64, 255),
        Color(128, 128, 128, 255),
        Color(255, 255, 255, 255),
        Color(255, 255, 255, 255),
        UNKNOWN,
    ]
}
fn heat_data() -> Vec<HeatCell> {
    values()
        .into_iter()
        .enumerate()
        .map(|(i, value)| HeatCell {
            row: if i < 3 { "early" } else { "late" }.into(),
            column: format!("c{}", i % 3),
            value,
        })
        .collect()
}
fn heat_options() -> HeatmapLayoutOptions {
    HeatmapLayoutOptions {
        colors: colors(),
        axes: HeatmapAxes::none(),
        labels: false,
        ..Default::default()
    }
}
fn calendar_options() -> CalendarLayoutOptions {
    CalendarLayoutOptions {
        colors: colors(),
        start: Some((2024, 1, 1)),
        end: Some((2024, 1, 10)),
        weekday_ticks: vec![],
        month_labels: false,
        year_labels: Some(false),
        year_legend: false,
        day_border: None,
        month_border: None,
        ..Default::default()
    }
}
fn calendar_data() -> Vec<CalendarCell> {
    values()
        .into_iter()
        .enumerate()
        .map(|(i, value)| CalendarCell::new((2024, 1, i as u8 + 1), value))
        .collect()
}
fn features() -> Vec<GeoFeature> {
    values()
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let x = -75. + i as f64 * 25.;
            GeoFeature {
                id: format!("region-{i}"),
                label: format!("Region {i}"),
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
fn map_options() -> GeoMapOptions {
    GeoMapOptions {
        fit: GeoFit::Data,
        border: None,
        ..Default::default()
    }
}

#[test]
fn three_consumers_use_duplicate_ranks_without_replacing_original_values() {
    let h = heatmap_layout(&config(), &heat_data(), &heat_options()).unwrap();
    let c = calendar_layout(&config(), &calendar_data(), &calendar_options()).unwrap();
    let input = features();
    let g = choropleth_with_color_scale(&config(), &input, &map_options(), &colors()).unwrap();
    for scale in [&h.color_scale, &c.color_scale, &g.color_scale] {
        assert_eq!(
            scale.sample_domain(),
            Some([0., 0., 5., 10., 10.].as_slice())
        );
        assert_eq!(scale.rank(0.), Some(0.25));
        assert_eq!(scale.position(0.), Some(0.));
    }
    for (i, color) in expected().into_iter().enumerate() {
        let key = HeatmapCellKey::new(if i < 3 { "early" } else { "late" }, format!("c{}", i % 3));
        let cell = h.cell(&key).unwrap();
        assert_eq!(cell.color, color);
        assert_eq!(cell.tooltip_datum().value, values()[i]);
        assert_eq!(
            h.scene
                .marks
                .iter()
                .find(|m| m.id == cell.mark_id)
                .unwrap()
                .fill,
            Some(color)
        );
        let day = c.day((2024, 1, i as u8 + 1)).unwrap();
        assert_eq!(day.color, color);
        assert_eq!(day.tooltip_datum().value, values()[i]);
        let mark = g
            .scene
            .marks
            .iter()
            .find(|m| m.series == format!("region-{i}"))
            .unwrap();
        assert_eq!(mark.fill, Some(color));
        assert_eq!(mark.label, format!("Region {i}"));
        // Geo's legacy Scene scalar has no nullable representation. Original
        // GeoFeature values remain authoritative for nullable typed bindings.
        assert_eq!(mark.value, values()[i].unwrap_or(0.));
        assert_eq!(input[i].value, values()[i]);
    }
    assert_eq!(c.day((2024, 1, 10)).unwrap().value, None);
    assert_eq!(c.day((2024, 1, 10)).unwrap().color, UNKNOWN);
    assert_eq!(
        h.cell(&HeatmapCellKey::new("early", "c2"))
            .unwrap()
            .tooltip_datum()
            .value,
        Some(5.)
    );
    assert_eq!(h.color_scale.rank(5.), Some(0.5));
}

#[test]
fn heatmap_hiding_rebuilds_the_distribution_but_reordering_does_not_change_colors() {
    let data = heat_data();
    let base = heatmap_layout(&config(), &data, &heat_options()).unwrap();
    let reversed = heatmap_layout(
        &config(),
        &data.iter().cloned().rev().collect::<Vec<_>>(),
        &heat_options(),
    )
    .unwrap();
    for cell in &base.cells {
        assert_eq!(reversed.cell(&cell.key).unwrap().color, cell.color);
        assert_eq!(reversed.cell(&cell.key).unwrap().value, cell.value);
    }
    let mut options = heat_options();
    options.hidden_rows.insert("early".into());
    let hidden = heatmap_layout(&config(), &data, &options).unwrap();
    assert_eq!(
        hidden.color_scale.sample_domain(),
        Some([10., 10.].as_slice())
    );
    let first = hidden.cell(&HeatmapCellKey::new("early", "c2")).unwrap();
    assert!(!first.visible);
    assert_eq!(first.value, Some(5.));
    assert!(!hidden.scene.marks.iter().any(|m| m.id == first.mark_id));
    assert_eq!(
        hidden
            .cell(&HeatmapCellKey::new("late", "c0"))
            .unwrap()
            .color,
        Color(255, 255, 255, 255)
    );
    options.hidden_rows.clear();
    options.hidden_columns.insert("c2".into());
    let hidden = heatmap_layout(&config(), &data, &options).unwrap();
    assert_eq!(
        hidden.color_scale.sample_domain(),
        Some([0., 0., 10., 10.].as_slice())
    );
    assert_eq!(
        hidden
            .cell(&HeatmapCellKey::new("early", "c0"))
            .unwrap()
            .color,
        Color(85, 85, 85, 255)
    );
}

#[test]
fn calendar_missing_dates_are_not_zero_samples_and_hidden_years_rebuild_ranks() {
    let data = values()
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            CalendarCell::new(
                (if i < 3 { 2024 } else { 2025 }, 1, (i % 3) as u8 + 1),
                value,
            )
        })
        .collect::<Vec<_>>();
    let mut options = calendar_options();
    options.end = Some((2025, 1, 3));
    let full = calendar_layout(&config(), &data, &options).unwrap();
    assert_eq!(
        full.color_scale.sample_domain(),
        Some([0., 0., 5., 10., 10.].as_slice())
    );
    assert_eq!(
        full.day((2024, 1, 1)).unwrap().color,
        Color(64, 64, 64, 255)
    );
    options.hidden_years.insert(2024);
    let hidden = calendar_layout(&config(), &data, &options).unwrap();
    assert_eq!(
        hidden.color_scale.sample_domain(),
        Some([10., 10.].as_slice())
    );
    assert!(!hidden.day((2024, 1, 3)).unwrap().visible);
    assert_eq!(hidden.day((2024, 1, 3)).unwrap().value, Some(5.));
    assert_eq!(
        hidden.day((2025, 1, 1)).unwrap().color,
        Color(255, 255, 255, 255)
    );
    let reordered = calendar_layout(
        &config(),
        &data.into_iter().rev().collect::<Vec<_>>(),
        &options,
    )
    .unwrap();
    assert_eq!(hidden.color_scale, reordered.color_scale);
}

#[test]
fn projection_clipping_changes_ink_without_changing_supplied_feature_distribution() {
    let mut data = features();
    data.push(GeoFeature {
        id: "behind".into(),
        label: "Behind".into(),
        value: Some(100.),
        polygons: vec![vec![vec![
            [160., 0.],
            [170., 0.],
            [170., 10.],
            [160., 10.],
            [160., 0.],
        ]]],
    });
    let mut options = GeoMapOptions {
        projection: GeoProjection::Orthographic,
        border: None,
        ..Default::default()
    };
    let front = choropleth_with_color_scale(&config(), &data, &options, &colors()).unwrap();
    assert_eq!(
        front.color_scale.sample_domain(),
        Some([0., 0., 5., 10., 10., 100.].as_slice())
    );
    assert!(!front.scene.marks.iter().any(|m| m.series == "behind"));
    assert_eq!(
        front
            .scene
            .marks
            .iter()
            .find(|m| m.series == "region-2")
            .unwrap()
            .fill,
        Some(Color(102, 102, 102, 255))
    );
    options.center = [165., 0.];
    let back = choropleth_with_color_scale(&config(), &data, &options, &colors()).unwrap();
    assert!(back.scene.marks.iter().any(|m| m.series == "behind"));
    assert_eq!(front.color_scale, back.color_scale);
}

#[test]
fn legacy_calendar_adapter_and_authored_window_keep_full_source_samples() {
    let dates = values()
        .into_iter()
        .enumerate()
        .filter_map(|(i, value)| value.map(|value| CalendarDate::new(2024, 1, i as u8 + 1, value)))
        .collect::<Vec<_>>();
    let options = CalendarRangeOptions {
        start: Some((2024, 1, 1)),
        end: Some((2024, 1, 6)),
        ..Default::default()
    };
    let old = calendar_range_with_color_scale(&config(), &dates, &options, &colors()).unwrap();
    for (i, color) in expected().into_iter().enumerate() {
        let id = format!("2024-01-{:02}", i + 1);
        assert_eq!(
            old.scene.marks.iter().find(|m| m.id == id).unwrap().fill,
            Some(color)
        );
    }
    let window = ColorScaleOptions {
        domain: Some([0., 1.]),
        clamp: false,
        ..colors()
    };
    let h = heatmap_layout(
        &config(),
        &heat_data(),
        &HeatmapLayoutOptions {
            colors: window,
            ..heat_options()
        },
    )
    .unwrap();
    assert_eq!(
        h.color_scale.sample_domain(),
        Some([0., 0., 5., 10., 10.].as_slice())
    );
    assert_eq!(
        h.cell(&HeatmapCellKey::new("early", "c0")).unwrap().color,
        Color(64, 64, 64, 255)
    );
    assert_eq!(
        h.cell(&HeatmapCellKey::new("early", "c2")).unwrap().color,
        UNKNOWN
    );
    assert_eq!(
        h.cell(&HeatmapCellKey::new("early", "c2")).unwrap().value,
        Some(5.)
    );
}

#[test]
fn optional_exact_legends_reject_unrepresentable_output_without_losing_valid_cells() {
    let data = (0..1000)
        .map(|i| HeatCell::new("r", format!("c{i}"), i as f64))
        .collect::<Vec<_>>();
    let h = heatmap_layout(
        &config(),
        &data,
        &HeatmapLayoutOptions {
            colors: ColorScaleOptions {
                colors: vec![Color(0, 0, 0, 255), Color(255, 254, 253, 255)],
                ..colors()
            },
            ..heat_options()
        },
    )
    .unwrap();
    assert_eq!(h.color_scale.sample_domain().unwrap().len(), 1000);
    let mut scene = h.scene.clone();
    let before = scene.clone();
    assert!(
        append_color_legend(&mut scene, &h.color_scale, &ColorLegendOptions::default()).is_err()
    );
    assert_eq!(scene, before);
    let close = [0., 0.5, 0.500000001, 1.]
        .into_iter()
        .enumerate()
        .map(|(i, v)| HeatCell::new("r", format!("c{i}"), v))
        .collect::<Vec<_>>();
    let h = heatmap_layout(&config(), &close, &heat_options()).unwrap();
    assert_eq!(h.cells[1].color, Color(85, 85, 85, 255));
    assert_eq!(h.cells[2].color, Color(170, 170, 170, 255));
    let mut scene = h.scene.clone();
    let before = scene.clone();
    assert!(
        append_color_legend(&mut scene, &h.color_scale, &ColorLegendOptions::default()).is_err()
    );
    assert_eq!(scene, before);
}

#[cfg(feature = "raster")]
#[test]
fn actual_three_consumer_cpu_pixels_have_original_rank_colors_at_both_dprs() {
    use render::raster::*;
    let h = heatmap_layout(&config(), &heat_data(), &heat_options()).unwrap();
    let c = calendar_layout(&config(), &calendar_data(), &calendar_options()).unwrap();
    let g = choropleth_with_color_scale(&config(), &features(), &map_options(), &colors()).unwrap();
    let hp = (0..6)
        .map(|i| {
            h.cell(&HeatmapCellKey::new(
                if i < 3 { "early" } else { "late" },
                format!("c{}", i % 3),
            ))
            .unwrap()
            .center
            .unwrap()
        })
        .collect::<Vec<_>>();
    let cp = (1..=6)
        .map(|i| {
            let d = c.day((2024, 1, i)).unwrap();
            [d.rect.x + d.rect.width / 2., d.rect.y + d.rect.height / 2.]
        })
        .collect::<Vec<_>>();
    let gp = (0..6)
        .map(|i| {
            g.projection
                .project([-69. + i as f64 * 25., 6.])
                .unwrap()
                .unwrap()
        })
        .collect::<Vec<_>>();
    for (scene, points) in [(&h.scene, hp), (&c.scene, cp), (&g.scene, gp)] {
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
            for (point, Color(r, g, b, a)) in points.iter().zip(expected()) {
                let (x, y) = (
                    (point[0] * ratio).floor() as usize,
                    (point[1] * ratio).floor() as usize,
                );
                assert_eq!(
                    &image.rgba[(y * image.width as usize + x) * 4
                        ..(y * image.width as usize + x) * 4 + 4],
                    [r, g, b, a],
                    "ratio={ratio} point={point:?}"
                );
            }
        }
    }
}
