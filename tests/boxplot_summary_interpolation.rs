use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 360.,
        margin: Margin {
            top: 20.,
            right: 20.,
            bottom: 20.,
            left: 20.,
        },
        ..Default::default()
    }
}
fn source(x: f64, y: f64) -> [BoxObservation; 2] {
    [
        BoxObservation::new("left", "A", "", Some(x)),
        BoxObservation::new("right", "A", "", Some(y)),
    ]
}
fn summary() -> BoxPlotLayoutOptions {
    BoxPlotLayoutOptions {
        orientation: StatisticsOrientation::Horizontal,
        auto_domain_source: BoxPlotAutoDomainSource::SummaryQuantiles,
        ..Default::default()
    }
}

#[test]
fn original_nivo_1046_narrow_date_and_scalar_profiles_match_statistic_bits_and_projection() {
    let mut count = 0;
    for line in include_str!("fixtures/boxplot-summary-interpolation.csv")
        .lines()
        .skip(1)
    {
        count += 1;
        let f: Vec<_> = line.split('|').collect();
        let nums = |i: usize| {
            f[i].split(',')
                .map(|v| v.parse::<f64>().unwrap())
                .collect::<Vec<_>>()
        };
        let bound = |i: usize| {
            if f[i] == "auto" {
                None
            } else {
                Some(f[i].parse().unwrap())
            }
        };
        let (x, y) = (f[1].parse().unwrap(), f[2].parse().unwrap());
        let mut o = summary();
        o.quantiles = nums(3).try_into().unwrap();
        o.domain_bounds = Some(BoxPlotDomainBounds {
            min: bound(4),
            max: bound(5),
        });
        let kind = if f[0] == "time" {
            ScaleKind::Utc
        } else {
            ScaleKind::Linear
        };
        let l = boxplot_layout_with_scale_options(
            &frame(),
            &source(x, y),
            &o,
            &ContinuousScaleOptions {
                kind,
                ..Default::default()
            },
        )
        .unwrap();
        for (actual, expected) in l.layout.boxes[0].quantiles.into_iter().zip(nums(6)) {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "case{count} statistic for{x}..{y}: {actual} != {expected}"
            );
        }
        for (actual, expected) in l.value_scale.source_domain().into_iter().zip(nums(7)) {
            if kind == ScaleKind::Utc {
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "case{count} Date domain"
                );
            } else {
                assert_eq!(actual, expected, "case{count} numeric domain");
            }
        }
        for (value, expected) in l.layout.boxes[0].quantiles.into_iter().zip(nums(8)) {
            let actual = l.map_statistic(value).unwrap();
            assert!(
                (actual - expected).abs() < 1e-9,
                "case{count} query{value}: {actual} != {expected}"
            );
        }
    }
    assert_eq!(count, 1046);
}

#[test]
fn supplied_root_witnesses_keep_one_millisecond_and_sixty_four_pixel_distinctions() {
    let base = 8_000_000_000_000_000.;
    let o = summary();
    let p = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let first =
        boxplot_layout_with_scale_options(&frame(), &source(base, base + 5.), &o, &p).unwrap();
    assert_eq!(first.layout.boxes[0].quantiles[4], base + 4.);
    assert_eq!(first.value_scale.source_domain(), [base, base + 4.]);
    assert_eq!(first.map_statistic(base + 4.).unwrap(), 340.);
    let second =
        boxplot_layout_with_scale_options(&frame(), &source(base + 1., base + 4.), &o, &p).unwrap();
    assert_eq!(second.layout.boxes[0].quantiles[0], base + 1.);
    assert_eq!(second.value_scale.source_domain()[0], base + 1.);
}
