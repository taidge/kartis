use kartis::*;

fn palette(n: usize) -> Vec<Color> {
    (0..n).map(|i| Color(i as u8, 30, 200, 255)).collect()
}
fn options(kind: ColorScaleKind, count: usize) -> ColorScaleOptions {
    ColorScaleOptions {
        kind,
        colors: palette(count),
        unknown: Color(255, 0, 0, 255),
        ..Default::default()
    }
}
fn list(value: &str) -> Vec<f64> {
    value
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.parse().unwrap())
        .collect()
}

#[test]
fn independent_d3_quantiles_threshold_queries_and_inverse_intervals() {
    let mut count = 0;
    for row in include_str!("fixtures/discrete-color-d3.csv")
        .lines()
        .skip(2)
    {
        let c = row.split('|').collect::<Vec<_>>();
        let mut o = options(
            if c[0] == "quantile" {
                ColorScaleKind::Quantile
            } else {
                ColorScaleKind::Threshold
            },
            c[3].parse().unwrap(),
        );
        o.thresholds = list(c[2]);
        let samples = c[1].split(',').filter(|s| !s.is_empty()).map(|s| {
            if s == "missing" {
                None
            } else {
                Some(s.parse::<f64>().unwrap())
            }
        });
        let s = o.resolve(samples).unwrap();
        match c[4] {
            "map" => {
                let value = if c[5] == "missing" {
                    None
                } else {
                    Some(c[5].parse().unwrap())
                };
                let expected: i32 = c[6].parse().unwrap();
                let color = if expected < 0 {
                    o.unknown
                } else {
                    o.colors[expected as usize]
                };
                assert_eq!(s.map(value), color, "{row}");
            }
            "cut" => assert_eq!(
                s.thresholds()[c[5].parse::<usize>().unwrap()],
                c[6].parse::<f64>().unwrap(),
                "{row}"
            ),
            "extent" => {
                let bound = |v: &str| {
                    if v == "unbounded" {
                        None
                    } else {
                        Some(v.parse::<f64>().unwrap())
                    }
                };
                let expected = ColorScaleExtent {
                    start: bound(c[6]),
                    end: bound(c[7]),
                };
                let i = c[5].parse::<usize>().unwrap();
                assert_eq!(s.bin_extent(i), Some(expected), "{row}");
                assert_eq!(s.invert_extent(o.colors[i]), Some(expected), "{row}");
            }
            _ => panic!("unexpected oracle operation"),
        }
        count += 1;
    }
    assert!(count > 1000, "truncated independent oracle: {count}");
}

#[test]
fn finite_legend_window_does_not_replace_sample_distribution_or_threshold_tails() {
    let mut q = options(ColorScaleKind::Quantile, 4);
    q.domain = Some([2., 8.]);
    let s = q
        .resolve([Some(0.), Some(0.), Some(4.), Some(12.)])
        .unwrap();
    assert_eq!(s.sample_extent(), Some([0., 12.]));
    assert_eq!(s.thresholds(), &[0., 2., 6.]);
    assert_eq!(
        s.bin_extent(0),
        Some(ColorScaleExtent {
            start: Some(0.),
            end: Some(0.)
        })
    );
    assert_eq!(
        s.bins()
            .iter()
            .map(|b| (b.start, b.end, b.color))
            .collect::<Vec<_>>(),
        vec![(2., 6., q.colors[2]), (6., 8., q.colors[3])]
    );
    assert_eq!(s.map(Some(-100.)), q.colors[0]);
    q.clamp = false;
    assert_eq!(
        q.resolve([Some(0.), Some(12.)]).unwrap().map(Some(-1.)),
        q.unknown
    );

    let mut t = options(ColorScaleKind::Threshold, 4);
    t.thresholds = vec![0., 0., 5.];
    t.domain = Some([-10., 10.]);
    let s = t.resolve([]).unwrap();
    assert_eq!(s.map(Some(0.)), t.colors[2]);
    assert_eq!(s.map(Some(f64::MAX)), t.colors[3]);
    assert_eq!(s.bin_extent(0).unwrap().start, None);
    assert_eq!(s.bin_extent(3).unwrap().end, None);
    assert_eq!(s.bins().len(), 3);
    let stops = s.stops(2).unwrap();
    assert_eq!(stops.len(), 6);
    assert_eq!(stops[1].offset, stops[2].offset);
    assert_ne!(stops[1].color, stops[2].color);
}

#[test]
fn empty_constant_single_color_and_duplicate_colors_have_explicit_semantics() {
    let q = options(ColorScaleKind::Quantile, 4);
    let empty = q.resolve([None, None]).unwrap();
    assert_eq!(empty.map(Some(0.)), q.unknown);
    assert_eq!(empty.bin_extent(0), None);
    assert!(empty.stops(2).unwrap().iter().all(|s| s.color == q.unknown));
    let constant = q.resolve([Some(3.), Some(3.)]).unwrap();
    assert_eq!(constant.map(Some(3.)), q.colors[3]);
    assert_eq!(constant.bins().len(), 1);
    assert_eq!(constant.bins()[0].color, q.colors[3]);
    for kind in [ColorScaleKind::Quantile, ColorScaleKind::Threshold] {
        let mut o = options(kind, 1);
        let s = o.resolve([Some(2.), Some(9.)]).unwrap();
        assert_eq!(s.map(Some(-1.)), o.colors[0]);
        o.colors = vec![o.colors[0]; 3];
        if kind == ColorScaleKind::Threshold {
            o.thresholds = vec![3., 5.];
        }
        let s = o.resolve([Some(2.), Some(9.)]).unwrap();
        assert_eq!(s.invert_extent(o.colors[0]), s.bin_extent(0));
        assert_ne!(s.bin_extent(0), s.bin_extent(2));
        assert_eq!(s.bin_extent(3), None);
        assert_eq!(s.invert_extent(o.unknown), None);
    }
}

#[test]
fn invalid_breaks_profiles_sources_and_budgets_reject_before_publication() {
    let mut t = options(ColorScaleKind::Threshold, 3);
    for breaks in [
        vec![],
        vec![2., 1.],
        vec![0., f64::NAN],
        vec![0., f64::INFINITY],
        vec![0.; 256],
    ] {
        t.thresholds = breaks;
        assert!(t.resolve([]).is_err());
    }
    t.thresholds = vec![0., 5.];
    assert!(t.resolve_with_profile([], ColorInterpolation::Lab).is_err());
    assert!(t.resolve_with_profile([], ColorScheme::Viridis).is_err());
    t.domain = Some([10., -10.]);
    assert!(t.resolve([]).is_err());
    for kind in [
        ColorScaleKind::Sequential,
        ColorScaleKind::Quantize,
        ColorScaleKind::Quantile,
    ] {
        let mut o = options(kind, 3);
        o.thresholds = vec![1., 2.];
        assert!(o.resolve([]).is_err());
    }
    let q = options(ColorScaleKind::Quantile, 3);
    assert!(q.resolve([Some(f64::NAN)]).is_err());
    assert!(q.resolve(std::iter::repeat_n(None, MAX_MARKS + 1)).is_err());
    let extreme = q.resolve([Some(-f64::MAX), Some(f64::MAX)]).unwrap();
    assert!(extreme.thresholds().iter().all(|v| v.is_finite()));
    assert!(extreme.thresholds().windows(2).all(|w| w[0] <= w[1]));
    for v in [
        Some(f64::NAN),
        Some(f64::INFINITY),
        Some(f64::NEG_INFINITY),
        None,
    ] {
        assert_eq!(extreme.map(v), q.unknown);
    }
}

#[test]
fn heatmap_calendar_and_choropleth_share_real_discrete_paint_and_missing_data() {
    let c = ChartConfig::default();
    for kind in [ColorScaleKind::Quantile, ColorScaleKind::Threshold] {
        let mut colors = options(kind, 3);
        if kind == ColorScaleKind::Threshold {
            colors.thresholds = vec![0., 5.];
        }
        let cells = [
            HeatCell {
                row: "r".into(),
                column: "a".into(),
                value: Some(-2.),
            },
            HeatCell {
                row: "r".into(),
                column: "b".into(),
                value: Some(0.),
            },
            HeatCell {
                row: "r".into(),
                column: "c".into(),
                value: Some(8.),
            },
            HeatCell {
                row: "r".into(),
                column: "d".into(),
                value: None,
            },
        ];
        let layout = heatmap_layout(
            &c,
            &cells,
            &HeatmapLayoutOptions {
                colors: colors.clone(),
                ..Default::default()
            },
        )
        .unwrap();
        for cell in &layout.cells {
            let mark = layout
                .scene
                .marks
                .iter()
                .find(|m| m.id == cell.key.mark_id())
                .unwrap();
            assert_eq!(mark.fill, Some(layout.color_scale.map(cell.value)));
        }
        assert_eq!(layout.color_scale.map(None), colors.unknown);
        let dates = [
            CalendarDate::new(2024, 2, 28, -2.),
            CalendarDate::new(2024, 2, 29, 0.),
            CalendarDate::new(2024, 3, 1, 8.),
        ];
        let calendar = calendar_range_with_color_scale(
            &c,
            &dates,
            &CalendarRangeOptions {
                start: Some((2024, 2, 28)),
                end: Some((2024, 3, 2)),
                ..Default::default()
            },
            &colors,
        )
        .unwrap();
        for (id, value) in [
            ("2024-02-28", Some(-2.)),
            ("2024-02-29", Some(0.)),
            ("2024-03-01", Some(8.)),
            ("2024-03-02", None),
        ] {
            assert_eq!(
                calendar
                    .scene
                    .marks
                    .iter()
                    .find(|m| m.id == id)
                    .unwrap()
                    .fill,
                Some(calendar.color_scale.map(value))
            );
        }
        let features = (0..4)
            .map(|i| GeoFeature {
                id: format!("f{i}"),
                label: format!("Region {i}"),
                value: cells[i].value,
                polygons: vec![vec![vec![
                    [-30. + i as f64 * 15., 0.],
                    [-20. + i as f64 * 15., 0.],
                    [-20. + i as f64 * 15., 10.],
                    [-30. + i as f64 * 15., 10.],
                ]]],
            })
            .collect::<Vec<_>>();
        let map =
            choropleth_with_color_scale(&c, &features, &GeoMapOptions::default(), &colors).unwrap();
        assert_eq!(
            map.color_scale.thresholds(),
            layout.color_scale.thresholds()
        );
        for mark in &map.scene.marks {
            let feature = features.iter().find(|f| f.id == mark.series).unwrap();
            assert_eq!(mark.fill, Some(map.color_scale.map(feature.value)));
        }
        assert_eq!(
            calendar.color_scale.thresholds(),
            map.color_scale.thresholds()
        );
        assert!(render::to_svg(&layout.scene).unwrap().contains("<svg"));
        assert!(render::to_canvas_script(&calendar.scene, "chart").is_ok());
        let app = glory::App::memory();
        app.mount(chart(map.scene)).unwrap();
    }
}

#[test]
fn visibility_recomputes_quantiles_but_authored_thresholds_stay_fixed() {
    let cells = [0., 0., 4., 12.]
        .into_iter()
        .enumerate()
        .map(|(i, value)| HeatCell {
            row: "r".into(),
            column: i.to_string(),
            value: Some(value),
        })
        .collect::<Vec<_>>();
    let config = ChartConfig::default();
    for kind in [ColorScaleKind::Quantile, ColorScaleKind::Threshold] {
        let mut colors = options(kind, 2);
        if kind == ColorScaleKind::Threshold {
            colors.thresholds = vec![2.];
        }
        let mut o = HeatmapLayoutOptions {
            colors,
            ..Default::default()
        };
        let before = heatmap_layout(&config, &cells, &o).unwrap();
        assert_eq!(before.color_scale.thresholds(), &[2.]);
        o.hidden_columns.insert("3".into());
        let hidden = heatmap_layout(&config, &cells, &o).unwrap();
        assert_eq!(
            hidden.color_scale.thresholds(),
            if kind == ColorScaleKind::Quantile {
                &[0.]
            } else {
                &[2.]
            }
        );
        assert_eq!(
            hidden
                .cells
                .iter()
                .find(|cell| cell.key.column == "3")
                .unwrap()
                .value,
            Some(12.)
        );
        o.hidden_columns.clear();
        let restored = heatmap_layout(&config, &cells, &o).unwrap();
        assert_eq!(restored.scene, before.scene);
        assert_eq!(restored.color_scale, before.color_scale);
    }
}

#[test]
fn cropped_legends_keep_partition_colors_and_extreme_quantiles_stay_monotone() {
    let mut o = options(ColorScaleKind::Threshold, 4);
    o.thresholds = vec![-5., 0., 5.];
    for (domain, color) in [([-100., -20.], 0), ([1., 3.], 2), ([20., 100.], 3)] {
        o.domain = Some(domain);
        let s = o.resolve([]).unwrap();
        assert_eq!(s.bins().len(), 1);
        assert_eq!(
            (s.bins()[0].start, s.bins()[0].end, s.bins()[0].color),
            (domain[0], domain[1], o.colors[color])
        );
        let stops = s.stops(2).unwrap();
        assert_eq!((stops[0].offset, stops[1].offset), (0., 1.));
    }
    for count in [1, 2, 3, 7, 128, 256] {
        let q = options(ColorScaleKind::Quantile, count);
        for samples in [
            vec![-f64::MAX, f64::MAX],
            vec![1., 1_f64.next_up()],
            vec![f64::from_bits(1), f64::from_bits(2)],
        ] {
            let s = q.resolve(samples.into_iter().map(Some)).unwrap();
            let extent = s.sample_extent().unwrap();
            assert_eq!(s.thresholds().len(), count - 1);
            assert!(s.thresholds().windows(2).all(|w| w[0] <= w[1]));
            assert!(
                s.thresholds()
                    .iter()
                    .all(|v| v.is_finite() && *v >= extent[0] && *v <= extent[1])
            );
            assert!(
                s.stops(2)
                    .unwrap()
                    .iter()
                    .all(|v| v.offset.is_finite() && (0. ..=1.).contains(&v.offset))
            );
        }
    }
}

#[cfg(feature = "raster")]
#[test]
fn legend_pixels_are_exact_discrete_colors_and_append_rejection_is_atomic() {
    use render::raster::*;
    for kind in [ColorScaleKind::Quantile, ColorScaleKind::Threshold] {
        let mut o = options(kind, 3);
        o.colors = vec![
            Color(30, 80, 170, 255),
            Color(20, 160, 120, 255),
            Color(230, 180, 40, 255),
        ];
        o.domain = Some([-10., 10.]);
        if kind == ColorScaleKind::Threshold {
            o.thresholds = vec![-5., 0.];
        }
        let s = o.resolve([Some(-5.), Some(8.)]).unwrap();
        let c = ChartConfig {
            width: 300.,
            height: 100.,
            ..Default::default()
        };
        let mut scene = c.scene().unwrap();
        let opts = ColorLegendOptions {
            position: [20., 20.],
            length: 200.,
            thickness: 20.,
            tick_count: 0,
            ..Default::default()
        };
        let legend = append_color_legend(&mut scene, &s, &opts).unwrap();
        assert_eq!(scene.marks.len(), 3);
        assert!(
            scene
                .marks
                .iter()
                .all(|m| m.fill_paint.is_none() && m.series.is_empty())
        );
        assert_eq!(legend.stops.len(), 6);
        let image = render_raster(&scene, &FontRegistry::new(), &RasterOptions::default()).unwrap();
        for x in [35_usize, 85, 135, 185, 215] {
            let value = -10. + (x as f64 + 0.5 - 20.) / 200. * 20.;
            let color = s.map(Some(value));
            let offset = (30 * 300 + x) * 4;
            assert_eq!(
                &image.rgba[offset..offset + 4],
                &[color.0, color.1, color.2, color.3],
                "{kind:?} x={x}"
            );
        }
        let accepted = scene.clone();
        assert!(append_color_legend(&mut scene, &s, &opts).is_err());
        assert_eq!(scene, accepted);
        let svg = render::to_svg(&scene).unwrap();
        assert!(!svg.contains("linearGradient"));
    }
}
