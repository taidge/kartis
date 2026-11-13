use glory::render::PathCommand;
use kartis::*;

fn config() -> ChartConfig {
    ChartConfig {
        width: 360.,
        height: 360.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

fn part(row: &BulletRowLayout, kind: BulletPartKind, source: Option<usize>) -> &BulletPart {
    row.parts
        .iter()
        .find(|part| part.kind == kind && part.source_index == source)
        .unwrap()
}

fn start(geometry: &Geometry) -> [f64; 2] {
    let Geometry::Path(path) = geometry else {
        panic!("expected path");
    };
    let PathCommand::Move(point) = path[0] else {
        panic!("expected start");
    };
    [point.x, point.y]
}

fn numbers(values: &str) -> Vec<f64> {
    values
        .split(',')
        .map(|value| value.parse().unwrap())
        .collect()
}

#[test]
fn configured_coordinates_match_the_actual_nivo_and_d3_profiles() {
    let mut profiles = 0;
    for row in include_str!("fixtures/bullet-scale-nivo-d3.csv")
        .lines()
        .skip(3)
    {
        let cols = row.split('|').collect::<Vec<_>>();
        let param = cols[1].parse().unwrap();
        let source: [f64; 2] = numbers(cols[2]).try_into().unwrap();
        let data = [BulletDatum {
            id: "oracle".into(),
            ranges: numbers(cols[3]),
            measures: numbers(cols[4]),
            markers: numbers(cols[5]),
        }];
        let vertical = cols[6] == "1";
        let coordinate = usize::from(vertical);
        let options = BulletLayoutOptions {
            orientation: if vertical {
                StatisticsOrientation::Vertical
            } else {
                StatisticsOrientation::Horizontal
            },
            min: Some(source[0]),
            max: Some(source[1]),
            reverse: cols[7] == "1",
            axis: AxisOptions {
                tick_count: 4,
                ..Default::default()
            },
            ..Default::default()
        };
        let scale = ContinuousScaleOptions {
            kind: match cols[0] {
                "log" => ScaleKind::Log,
                "symlog" => ScaleKind::SymLog { constant: param },
                "utc" => ScaleKind::Utc,
                _ => ScaleKind::Linear,
            },
            log_base: if cols[0] == "log" { param } else { 10. },
            nice: match cols[10] {
                "-1" => ScaleNice::Off,
                "0" => ScaleNice::Default,
                n => ScaleNice::Count(n.parse().unwrap()),
            },
            reverse: cols[8] == "1",
            round: cols[9] == "1",
            clamp: cols[0] == "nivo",
        };
        let result = bullet_layout_with_scale_options(&config(), &data, &options, &scale)
            .unwrap_or_else(|error| panic!("{}: {error}", cols[0]));
        let layout = &result.layout;
        let row = &layout.rows[0];
        let scale = result.shared_value_scale.unwrap();
        assert_eq!(scale.source_domain(), source);
        assert_eq!(row.domain.as_slice(), numbers(cols[11]));
        for (kind, expected) in [
            (BulletPartKind::Range, cols[12]),
            (BulletPartKind::Measure, cols[13]),
        ] {
            let rects = expected.split(';').map(numbers).collect::<Vec<_>>();
            let parts = row
                .parts
                .iter()
                .filter(|part| part.kind == kind)
                .collect::<Vec<_>>();
            assert_eq!(parts.len(), rects.len());
            for (actual, expected) in parts.into_iter().zip(rects) {
                let Geometry::Rect {
                    x,
                    y,
                    width,
                    height,
                } = actual.geometry
                else {
                    panic!("expected interval rectangle");
                };
                assert_eq!(actual.interval.as_slice(), &expected[..2]);
                for (actual, expected) in [x, y, width, height].into_iter().zip(&expected[2..]) {
                    near(actual, *expected);
                }
            }
        }
        for (index, expected) in numbers(cols[14]).into_iter().enumerate() {
            let marker = part(row, BulletPartKind::Marker, Some(index));
            near(start(&marker.geometry)[coordinate], expected);
        }
        let oracle_ticks = cols[15].split(';').map(numbers).collect::<Vec<_>>();
        let ticks = scale.ticks(4);
        let tick_marks = layout
            .scene
            .marks
            .iter()
            .filter(|mark| mark.id.starts_with("bullet-axis:6:oracle-tick-"))
            .collect::<Vec<_>>();
        assert_eq!(tick_marks.len(), oracle_ticks.len());
        assert_eq!(ticks.len(), oracle_ticks.len());
        for ((tick, mark), expected) in ticks.into_iter().zip(tick_marks).zip(oracle_ticks) {
            near(tick, expected[0]);
            near(start(&mark.geometry)[coordinate], expected[1]);
        }
        profiles += 1;
    }
    assert_eq!(profiles, 7);
}

fn positive() -> [BulletDatum; 1] {
    [BulletDatum {
        id: "Operations".into(),
        ranges: vec![100., 10., 0.],
        measures: vec![30., 5.],
        markers: vec![100., 2000., 0.],
    }]
}

fn positive_options() -> BulletLayoutOptions {
    BulletLayoutOptions {
        min: Some(3.),
        max: Some(760.),
        axis: AxisOptions {
            tick_values: Some(vec![1., 10., 100., 1000.]),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn nice_log() -> ContinuousScaleOptions {
    ContinuousScaleOptions {
        kind: ScaleKind::Log,
        nice: ScaleNice::Default,
        round: true,
        ..Default::default()
    }
}

#[test]
fn log_nice_uses_display_bounds_for_real_ink_ticks_and_raw_hit_values() {
    let scaled =
        bullet_layout_with_scale_options(&config(), &positive(), &positive_options(), &nice_log())
            .unwrap();
    let layout = &scaled.layout;
    let row = &layout.rows[0];
    let scale = scaled.shared_value_scale.as_ref().unwrap();
    assert_eq!(scale.source_domain(), [3., 760.]);
    assert_eq!(scale.display_domain(), [1., 1000.]);
    assert_eq!(layout.shared_domain, Some([1., 1000.]));
    assert_eq!(row.domain, [1., 1000.]);
    assert_eq!(scaled.row_value_scales[&row.id].map(100.), 233.);
    assert_eq!(scale.map(100.), 233.);
    assert!(scale.map(2000.) > 340.);
    assert!(!scale.chart_scale().is_clamped());
    let first = part(row, BulletPartKind::Measure, Some(1));
    assert_eq!(first.interval, [1., 5.]);
    assert_eq!(first.visible_interval, [1., 5.]);
    assert!(matches!(
        first.geometry,
        Geometry::Rect {
            x: 20.,
            width: 75.,
            ..
        }
    ));
    let last = part(row, BulletPartKind::Range, None);
    assert_eq!(last.interval, [100., 1000.]);
    assert!(matches!(
        last.geometry,
        Geometry::Rect {
            x: 233.,
            width: 107.,
            ..
        }
    ));

    for (i, value) in [1., 10., 100., 1000.].into_iter().enumerate() {
        let tick = layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == format!("bullet-axis:10:Operations-tick-{i}"))
            .unwrap();
        assert_eq!(start(&tick.geometry)[0], scale.map(value));
    }
    let observed = part(row, BulletPartKind::Marker, Some(0));
    let hit = layout.scene.hit_test(233., 180.).unwrap();
    assert_eq!(hit.id, observed.id);
    assert_eq!(hit.value, 100.);
    assert_eq!(hit.clip, Some(row.track));
    let svg = render::to_svg(&layout.scene).unwrap();
    assert!(svg.contains(r#"data-id="bullet:10:Operations:marker:0""#));
    for (index, projected) in [(1, 340.), (2, 20.)] {
        let missing = part(row, BulletPartKind::Marker, Some(index));
        assert_eq!(start(&missing.geometry)[0], projected);
        assert!(!layout.scene.marks.iter().any(|m| m.id == missing.id));
    }
}

#[test]
fn log_per_row_keeps_sign_direction_ids_and_owned_axes_on_reflow() {
    let data = [
        BulletDatum {
            id: "negative".into(),
            ranges: vec![-60., -3.],
            measures: vec![-16.],
            markers: vec![-8.],
        },
        BulletDatum {
            id: "positive".into(),
            ranges: vec![3., 61.],
            measures: vec![16.],
            markers: vec![8.],
        },
    ];
    let mut options = BulletLayoutOptions {
        orientation: StatisticsOrientation::Vertical,
        reverse: true,
        domain_mode: BulletDomainMode::PerRow,
        axis_position: BulletPosition::Before,
        axis: AxisOptions {
            tick_values: Some(vec![-64., -8., -2., 2., 8., 64.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Log,
        log_base: 2.,
        reverse: true,
        nice: ScaleNice::Default,
        ..Default::default()
    };
    let result = bullet_layout_with_scale_options(&config(), &data, &options, &scale).unwrap();
    assert!(result.shared_value_scale.is_none());
    assert_eq!(result.layout.shared_domain, None);
    let negative = &result.layout.rows[0];
    let positive = &result.layout.rows[1];
    assert_eq!(negative.domain, [-64., -2.]);
    assert_eq!(positive.domain, [2., 64.]);
    assert_eq!(
        result.row_value_scales["negative"].source_domain(),
        [-60., -3.]
    );
    assert_eq!(
        result.row_value_scales["positive"].source_domain(),
        [3., 61.]
    );
    assert_eq!(result.row_value_scales["positive"].range(), [340., 20.]);
    near(
        start(&part(negative, BulletPartKind::Marker, Some(0)).geometry)[1],
        148.,
    );
    near(
        start(&part(positive, BulletPartKind::Marker, Some(0)).geometry)[1],
        212.,
    );
    let marker = part(positive, BulletPartKind::Marker, Some(0));
    let center_x = positive.track.x + positive.track.width / 2.;
    assert_eq!(
        result.layout.scene.hit_test(center_x, 212.).unwrap().id,
        marker.id
    );
    assert!(
        result
            .layout
            .scene
            .labels
            .iter()
            .enumerate()
            .any(|(i, label)| {
                label.text == "-8"
                    && result.layout.scene.label_series.get(&i).map(String::as_str)
                        == Some("negative")
            })
    );

    options.hidden.insert("negative".into());
    let filtered = bullet_layout_with_scale_options(&config(), &data, &options, &scale).unwrap();
    assert_eq!(filtered.row_value_scales.len(), 1);
    let row = &filtered.layout.rows[0];
    assert_eq!(row.track.width, 320.);
    assert_eq!(part(row, BulletPartKind::Marker, Some(0)).id, marker.id);
    assert_eq!(filtered.layout.scene.legend.len(), 2);
    assert!(
        !filtered
            .layout
            .scene
            .label_series
            .values()
            .any(|id| id == "negative")
    );
}

#[test]
fn rounded_collapsed_interval_keeps_metadata_without_creating_a_mark() {
    let data = [BulletDatum {
        id: "rounded".into(),
        ranges: vec![1000.],
        measures: vec![0.0001, 50.],
        markers: vec![0.],
    }];
    let result = bullet_layout_with_scale_options(
        &config(),
        &data,
        &BulletLayoutOptions::default(),
        &ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 2. },
            reverse: true,
            round: true,
            ..Default::default()
        },
    )
    .unwrap();
    let row = &result.layout.rows[0];
    let tiny = part(row, BulletPartKind::Measure, Some(0));
    assert_eq!(tiny.visible_interval, [0., 0.0001]);
    assert!(matches!(
        tiny.geometry,
        Geometry::Rect {
            x: 340.,
            width: 0.,
            ..
        }
    ));
    assert!(!result.layout.scene.marks.iter().any(|m| m.id == tiny.id));
    let mark = part(row, BulletPartKind::Marker, Some(0));
    assert_eq!(start(&mark.geometry)[0], 340.);
}

#[test]
fn utc_calendar_ticks_and_leap_day_marker_share_the_actual_time_axis() {
    let first = 1_709_078_400_000.;
    let day = 86_400_000.;
    let mut data = [BulletDatum {
        id: "UTC".into(),
        ranges: vec![first, first + day * 2.],
        measures: vec![first + day * 1.5],
        markers: vec![first + day],
    }];
    let options = BulletLayoutOptions {
        axis: AxisOptions {
            tick_calendar: Some(TimeCalendar {
                interval: Some(TimeInterval::new(TimeUnit::Day, 1).unwrap()),
                ..Default::default()
            }),
            format: ValueFormat::UtcDate,
            ..Default::default()
        },
        value_format: ValueFormat::UtcDate,
        ..Default::default()
    };
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let result = bullet_layout_with_scale_options(&config(), &data, &options, &scale).unwrap();
    let row = &result.layout.rows[0];
    let marker = part(row, BulletPartKind::Marker, Some(0));
    assert_eq!(start(&marker.geometry)[0], 180.);
    let hit = result.layout.scene.hit_test(180., 180.).unwrap();
    assert_eq!(hit.id, marker.id);
    assert_eq!(hit.value, first + day);
    assert!(hit.label.contains("2024-02-29"));
    assert!(
        result
            .layout
            .scene
            .labels
            .iter()
            .any(|l| l.text == "2024-02-29" && l.x == 180.)
    );
    assert!(bullet_layout(&config(), &data, &options).is_err());
    data[0].markers[0] += 0.5;
    assert!(bullet_layout_with_scale_options(&config(), &data, &options, &scale).is_err());
}

#[test]
fn empty_log_uses_authored_bounds_then_ticks_and_rejects_invalid_scale_options() {
    let c = config();
    let mut options = BulletLayoutOptions::default();
    let scale = nice_log();
    let empty = bullet_layout_with_scale_options(&c, &[], &options, &scale).unwrap();
    assert_eq!(empty.layout.shared_domain, Some([1., 10.]));
    assert!(empty.row_value_scales.is_empty());
    options.axis.tick_values = Some(vec![-100., -1.]);
    let empty = bullet_layout_with_scale_options(&c, &[], &options, &scale).unwrap();
    assert_eq!(empty.layout.shared_domain, Some([-100., -1.]));
    options.min = Some(-5.);
    options.max = Some(3.);
    assert!(bullet_layout_with_scale_options(&c, &[], &options, &scale).is_err());
    options.domain_mode = BulletDomainMode::PerRow;
    assert!(bullet_layout_with_scale_options(&c, &[], &options, &scale).is_err());
    assert!(
        bullet_layout_with_scale_options(
            &c,
            &[],
            &BulletLayoutOptions::default(),
            &ContinuousScaleOptions {
                log_base: 1.,
                ..scale
            },
        )
        .is_err()
    );
}

#[cfg(feature = "raster")]
#[test]
fn cpu_draws_log_marker_and_ranges_at_nonlinear_coordinates() {
    use kartis::render::raster::{FontRegistry, render_raster};

    let mut options = positive_options();
    options.axis.visible = false;
    options.range_colors = vec![Color(30, 60, 90, 255)];
    options.measure_colors = vec![Color(20, 50, 170, 255)];
    options.marker_colors = vec![Color(220, 55, 35, 255)];
    let result =
        bullet_layout_with_scale_options(&config(), &positive(), &options, &nice_log()).unwrap();
    let mut scene = result.layout.scene;
    scene.labels.clear();
    scene.label_series.clear();
    scene.legend.clear();
    let image = render_raster(&scene, &FontRegistry::default(), &Default::default()).unwrap();
    let pixel = |x: usize, y: usize| {
        let index = (y * image.width as usize + x) * 4;
        &image.rgba[index..index + 4]
    };
    assert_eq!(pixel(233, 180), &[220, 55, 35, 255]);
    assert_eq!(pixel(61, 180), &[20, 50, 170, 255]);
    assert_eq!(pixel(150, 40), &[30, 60, 90, 255]);
}
