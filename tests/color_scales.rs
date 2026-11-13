use kartis::*;
fn options(kind: ColorScaleKind) -> ColorScaleOptions {
    ColorScaleOptions {
        transform: ColorScaleTransform::Linear,
        kind,
        domain: Some([-10., 10.]),
        colors: vec![Color(0, 0, 0, 255), Color(255, 255, 255, 255)],
        unknown: Color(123, 45, 67, 255),
        clamp: true,
        thresholds: Vec::new(),
    }
}
#[test]
fn independent_d3_queries_cover_constant_missing_center_bins_and_rgba() {
    let fixture = include_str!("fixtures/color-scale-d3.csv");
    let mut count = 0;
    for row in fixture.lines().skip(2) {
        let cells = row.split('|').collect::<Vec<_>>();
        let number = |i: usize| cells[i].parse::<f64>().unwrap();
        let kind = match cells[0] {
            "sequential" => ColorScaleKind::Sequential,
            "diverging" => ColorScaleKind::Diverging {
                midpoint: number(3),
            },
            _ => ColorScaleKind::Quantize,
        };
        let colors = cells[5]
            .split(';')
            .map(|c| {
                let v = c
                    .split(':')
                    .map(|n| n.parse::<u8>().unwrap())
                    .collect::<Vec<_>>();
                Color(v[0], v[1], v[2], v[3])
            })
            .collect();
        let o = ColorScaleOptions {
            transform: ColorScaleTransform::Linear,
            kind,
            domain: Some([number(1), number(2)]),
            colors,
            clamp: cells[4] == "1",
            thresholds: Vec::new(),
            unknown: Color(123, 45, 67, 255),
        };
        let scale = o.resolve([]).unwrap();
        let value = match cells[6] {
            "missing" => None,
            "nan" => Some(f64::NAN),
            _ => Some(number(6)),
        };
        let expected = Color(
            number(7) as u8,
            number(8) as u8,
            number(9) as u8,
            number(10) as u8,
        );
        assert_eq!(scale.map(value), expected, "{row}");
        count += 1;
    }
    assert_eq!(count, 300);
}
#[test]
fn exact_quantize_boundaries_reverse_direction_and_hard_edge_stops() {
    let colors = vec![
        Color(20, 40, 60, 255),
        Color(80, 100, 120, 255),
        Color(140, 160, 180, 255),
    ];
    let mut o = options(ColorScaleKind::Quantize);
    o.domain = Some([0., 1.]);
    o.colors = colors.clone();
    let s = o.resolve([]).unwrap();
    let t = s.thresholds()[0];
    assert_eq!(s.map(Some(f64::from_bits(t.to_bits() - 1))), colors[0]);
    assert_eq!(s.map(Some(t)), colors[1]);
    assert_eq!(
        s.stops(2)
            .unwrap()
            .iter()
            .map(|s| s.offset)
            .collect::<Vec<_>>(),
        vec![0., 1. / 3., 1. / 3., 2. / 3., 2. / 3., 1.]
    );
    o.domain = Some([1., 0.]);
    let reverse = o.resolve([]).unwrap();
    assert_eq!(reverse.map(Some(1.)), colors[0]);
    assert_eq!(reverse.map(Some(0.)), colors[2]);
    assert_eq!(reverse.map(Some(reverse.thresholds()[0])), colors[1]);
    assert!(reverse.bins().windows(2).all(|b| b[0].end == b[1].start));
    o.clamp = false;
    let s = o.resolve([]).unwrap();
    assert_eq!(s.map(Some(2.)), o.unknown);
    o.domain = Some([4., 4.]);
    let s = o.resolve([]).unwrap();
    assert_eq!(s.map(Some(4.)), colors[2]);
    assert_eq!(s.bins().len(), 1);
}
#[test]
fn diverging_center_is_a_data_value_and_palette_kinks_survive_sampling() {
    let mut o = options(ColorScaleKind::Diverging { midpoint: 2. });
    o.colors = vec![
        Color(0, 0, 100, 255),
        Color(200, 200, 200, 255),
        Color(100, 0, 0, 255),
    ];
    let s = o.resolve([]).unwrap();
    assert_eq!(s.map(Some(2.)), o.colors[1]);
    assert_eq!(s.map(Some(-4.)), Color(100, 100, 150, 255));
    assert_eq!(s.map(Some(6.)), Color(150, 100, 100, 255));
    assert!(
        s.stops(2)
            .unwrap()
            .iter()
            .any(|t| t.value == 2. && t.offset == 0.6 && t.color == o.colors[1])
    );
    o.domain = Some([10., -10.]);
    let s = o.resolve([]).unwrap();
    assert_eq!(s.map(Some(10.)), o.colors[0]);
    assert_eq!(s.map(Some(2.)), o.colors[1]);
    assert_eq!(s.map(Some(-10.)), o.colors[2]);
    o.kind = ColorScaleKind::Sequential;
    o.colors = vec![
        Color(0, 0, 0, 255),
        Color(255, 0, 0, 255),
        Color(0, 0, 255, 255),
    ];
    let s = o.resolve([]).unwrap();
    assert!(
        s.stops(2)
            .unwrap()
            .iter()
            .any(|v| v.offset == 0.5 && v.color == o.colors[1])
    );
}
#[test]
fn auto_empty_constant_extreme_and_extrapolation_profiles_are_finite() {
    let mut o = ColorScaleOptions::default();
    let empty = o.resolve([None, None]).unwrap();
    assert_eq!(empty.domain(), [0., 1.]);
    assert_eq!(empty.map(None), o.unknown);
    let constant = o.resolve([Some(7.), Some(7.)]).unwrap();
    assert_eq!(constant.domain(), [7., 7.]);
    assert_eq!(constant.map(Some(-1e99)), Color(133, 172, 211, 255));
    o.domain = Some([-f64::MAX, f64::MAX]);
    let huge = o.resolve([]).unwrap();
    assert_eq!(huge.map(Some(0.)), constant.map(Some(7.)));
    assert!(huge.stops(256).unwrap().iter().all(|s| s.value.is_finite()));
    o.domain = Some([f64::from_bits(f64::MAX.to_bits() - 1), f64::MAX]);
    o.kind = ColorScaleKind::Quantize;
    o.colors = vec![Color(1, 2, 3, 255); 256];
    let huge = o.resolve([]).unwrap();
    assert!(huge.thresholds().iter().all(|v| v.is_finite()));
    o.kind = ColorScaleKind::Sequential;
    o.colors = vec![Color(10, 30, 20, 255), Color(20, 30, 40, 255)];
    o.domain = Some([0., f64::MIN_POSITIVE]);
    o.clamp = false;
    let s = o.resolve([]).unwrap();
    assert_eq!(s.map(Some(f64::MAX)), Color(255, 30, 255, 255));
    assert_eq!(s.map(Some(f64::NEG_INFINITY)), o.unknown);
}
#[test]
fn invalid_sources_and_options_are_rejected_without_rewriting_the_caller() {
    let original = options(ColorScaleKind::Sequential);
    for case in 0..6 {
        let mut o = original.clone();
        match case {
            0 => o.colors.clear(),
            1 => o.colors = vec![Color(0, 0, 0, 255); 257],
            2 => o.domain = Some([0., f64::NAN]),
            3 => o.kind = ColorScaleKind::Diverging { midpoint: 0. },
            4 => {
                o.kind = ColorScaleKind::Diverging { midpoint: 20. };
                o.colors.push(Color(0, 0, 0, 255));
            }
            _ => o.domain = Some([f64::INFINITY, 0.]),
        };
        assert!(o.resolve([]).is_err(), "{case}");
    }
    assert!(original.resolve([Some(f64::NAN)]).is_err());
    assert!(original.resolve((0..20001).map(|_| None)).is_err());
    let s = original.resolve([]).unwrap();
    assert!(s.stops(1).is_err());
    assert!(s.stops(257).is_err());
    assert_eq!(original, options(ColorScaleKind::Sequential));
}
#[test]
fn quantize_thresholds_remain_ordered_and_inside_tight_finite_domains() {
    for low in [1_f64, 1e-300, 1e300, -1., -1e-300, -1e300] {
        for distance in [1, 2, 5, 16] {
            let high = f64::from_bits(if low > 0. {
                low.to_bits() + distance
            } else {
                low.to_bits() - distance
            });
            for domain in [[low, high], [high, low]] {
                for count in [2, 3, 7, 16, 63, 128, 256] {
                    let s = ColorScaleOptions {
                        kind: ColorScaleKind::Quantize,
                        domain: Some(domain),
                        colors: vec![Color(0, 0, 0, 255); count],
                        ..Default::default()
                    }
                    .resolve([])
                    .unwrap();
                    assert!(
                        s.thresholds().iter().all(|v| *v >= low && *v <= high),
                        "outside {domain:?} n={count}: {:?}",
                        s.thresholds()
                    );
                    assert!(
                        s.thresholds()
                            .windows(2)
                            .all(|v| if domain[0] <= domain[1] {
                                v[0] <= v[1]
                            } else {
                                v[0] >= v[1]
                            }),
                        "nonmonotone {domain:?} n={count}: {:?}",
                        s.thresholds()
                    );
                }
            }
        }
    }
}
#[test]
fn shared_calendar_and_map_consumers_preserve_missing_zero_projection_and_geometry() {
    let c = ChartConfig::default();
    let colors = ColorScaleOptions {
        domain: Some([-2., 2.]),
        colors: vec![Color(0, 0, 255, 255), Color(255, 0, 0, 255)],
        unknown: Color(0, 255, 0, 255),
        ..Default::default()
    };
    let dates = [
        CalendarDate::new(2024, 2, 28, -2.),
        CalendarDate::new(2024, 2, 29, 0.),
    ];
    let options = CalendarRangeOptions {
        start: Some((2024, 2, 28)),
        end: Some((2024, 3, 1)),
        ..Default::default()
    };
    let old = calendar_range(&c, &dates, &options).unwrap();
    let new = calendar_range_with_color_scale(&c, &dates, &options, &colors).unwrap();
    for (a, b) in old.marks.iter().zip(&new.scene.marks) {
        assert_eq!(a.geometry, b.geometry);
        assert_eq!(a.id, b.id);
    }
    assert_eq!(new.scene.marks[0].fill, Some(Color(0, 0, 255, 255)));
    assert_eq!(new.scene.marks[1].fill, Some(Color(128, 0, 128, 255)));
    assert_eq!(new.scene.marks[2].fill, Some(colors.unknown));
    let features = (0..3)
        .map(|i| GeoFeature {
            id: format!("f{i}"),
            label: format!("F{i}"),
            value: if i == 2 { None } else { Some(i as f64 - 1.) },
            polygons: vec![vec![vec![
                [-20. + i as f64 * 12., 0.],
                [-10. + i as f64 * 12., 0.],
                [-10. + i as f64 * 12., 10.],
                [-20. + i as f64 * 12., 10.],
            ]]],
        })
        .collect::<Vec<_>>();
    let old = geographic_map_layout(&c, &features, &Default::default()).unwrap();
    let new = choropleth_with_color_scale(&c, &features, &Default::default(), &colors).unwrap();
    assert_eq!(old.projection, new.projection);
    for (a, b) in old.scene.marks.iter().zip(&new.scene.marks) {
        assert_eq!(a.geometry, b.geometry);
        assert_eq!(
            b.fill,
            Some(
                new.color_scale
                    .map(features.iter().find(|f| f.id == b.series).unwrap().value)
            )
        );
    }
}
