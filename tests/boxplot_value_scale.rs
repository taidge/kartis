use glory::render::PathCommand;
#[cfg(feature = "raster")]
use glory::render::{GraphicStroke, StrokeCap};
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

fn samples(values: &[f64]) -> Vec<BoxObservation> {
    values
        .iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(i.to_string(), "Sample", "sensor", Some(*value)))
        .collect()
}

fn near(actual: f64, expected: f64, tolerance: f64) {
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} differs from {expected} (tolerance {tolerance})"
    );
}

fn find<'a>(scene: &'a Scene, id: &str) -> &'a Mark {
    scene.marks.iter().find(|mark| mark.id == id).unwrap()
}

fn endpoints(mark: &Mark, vertical: bool) -> Vec<f64> {
    let Geometry::Path(commands) = &mark.geometry else {
        panic!("statistical strokes use path geometry");
    };
    commands
        .iter()
        .map(|command| {
            let (PathCommand::Move(point) | PathCommand::Line(point)) = command else {
                panic!("expected a straight statistical stroke");
            };
            if vertical { point.y } else { point.x }
        })
        .collect()
}

#[test]
fn actual_nivo_statistics_and_d3_projections_match_six_profiles() {
    let cases = include_str!("fixtures/boxplot-scale-nivo-d3.csv");
    let mut count = 0;
    for row in cases.lines().filter(|line| !line.starts_with('#')) {
        count += 1;
        let fields: Vec<_> = row.split('|').collect();
        let number = |i: usize| fields[i].parse::<f64>().unwrap();
        let list = |i: usize| fields[i].split(',').map(|n| n.parse::<f64>().unwrap());
        let [a, b] = list(2).collect::<Vec<_>>().try_into().unwrap();
        let data = samples(&list(3).collect::<Vec<_>>());
        let vertical = fields[4] == "1";
        let ticks: Vec<(f64, f64)> = fields[13]
            .split(';')
            .map(|pair| {
                let (tick, pixel) = pair.split_once(',').unwrap();
                (tick.parse().unwrap(), pixel.parse().unwrap())
            })
            .collect();
        let mut options = BoxPlotLayoutOptions {
            orientation: if vertical {
                StatisticsOrientation::Vertical
            } else {
                StatisticsOrientation::Horizontal
            },
            domain: Some([a, b]),
            ..Default::default()
        };
        options.value_axis.tick_count = 5;
        let kind = match fields[0] {
            "linear" => ScaleKind::Linear,
            "log" => ScaleKind::Log,
            "symlog" => ScaleKind::SymLog {
                constant: number(1),
            },
            "utc" => ScaleKind::Utc,
            _ => panic!("unexpected reference scale"),
        };
        let profile = ContinuousScaleOptions {
            kind,
            log_base: if kind == ScaleKind::Log {
                number(1)
            } else {
                10.
            },
            reverse: fields[5] == "1",
            round: fields[6] == "1",
            nice: match number(7) as i32 {
                -1 => ScaleNice::Off,
                0 => ScaleNice::Default,
                n => ScaleNice::Count(n as usize),
            },
            ..Default::default()
        };
        let output = boxplot_layout_with_scale_options(&frame(), &data, &options, &profile)
            .unwrap_or_else(|error| panic!("profile {row}: {error}"));
        assert_eq!(output.value_scale.source_domain(), [a, b]);
        assert_eq!(output.value_scale.display_domain(), output.layout.domain);
        for (actual, expected) in output.layout.domain.into_iter().zip(list(8)) {
            near(actual, expected, 1e-10);
        }
        let box_ = &output.layout.boxes[0];
        let tolerance = if kind == ScaleKind::Utc { 0.001 } else { 1e-10 };
        for (actual, expected) in box_.quantiles.into_iter().zip(list(9)) {
            near(actual, expected, tolerance);
        }
        near(box_.mean, number(10), tolerance);
        for (actual, expected) in [box_.rect.x, box_.rect.y, box_.rect.width, box_.rect.height]
            .into_iter()
            .zip(list(11))
        {
            near(actual, expected, 1e-6);
        }
        for (value, expected) in box_.quantiles.into_iter().zip(list(12)) {
            near(output.map_statistic(value).unwrap(), expected, 1e-6);
        }
        let scene = &output.layout.scene;
        let median = find(scene, &format!("{}:median", box_.id));
        assert_eq!(median.value, box_.quantiles[2]);
        assert_eq!(find(scene, &box_.id).value, median.value);
        let median_pixel = list(12).nth(2).unwrap();
        for pixel in endpoints(median, vertical) {
            near(pixel, median_pixel, 1e-6);
        }
        let whiskers = find(scene, &format!("{}:whiskers", box_.id));
        let projection: Vec<_> = list(12).collect();
        for (actual, expected) in endpoints(whiskers, vertical).into_iter().zip([
            projection[0],
            projection[1],
            projection[3],
            projection[4],
        ]) {
            near(actual, expected, 1e-6);
        }
        assert_eq!(
            output.value_scale.ticks(5).len(),
            ticks.len(),
            "profile {row}"
        );
        for ((tick, pixel), actual) in ticks.into_iter().zip(output.value_scale.ticks(5)) {
            near(actual, tick, 1e-10);
            near(output.value_scale.map(actual), pixel, 1e-6);
            assert!(scene.marks.iter().any(|mark| {
                mark.id.starts_with("axis-bar-value-tick-")
                    && (endpoints(mark, vertical)[0] - pixel).abs() < 1e-6
            }));
        }
        let svg = kartis::render::to_svg(scene).unwrap();
        assert!(svg.contains("data-id=\"axis-bar-value"));
        assert!(svg.contains("data-id=\"box:6:Sample:6:sensor\""));
    }
    assert_eq!(count, 6);
}

#[test]
fn utc_calendar_ticks_and_fractional_median_preserve_numeric_positions() {
    let data = samples(&[0., 1.]);
    let profile = ContinuousScaleOptions {
        kind: ScaleKind::Utc,
        ..Default::default()
    };
    let options = BoxPlotLayoutOptions::default();
    let result = boxplot_layout_with_scale_options(&frame(), &data, &options, &profile).unwrap();
    let box_ = &result.layout.boxes[0];
    assert_eq!(box_.quantiles, [0.1, 0.25, 0.5, 0.75, 0.9]);
    assert_eq!(result.map_statistic(0.5).unwrap(), 180.);
    assert!(result.value_scale.map(0.5).is_nan());
    assert!(result.map_statistic(f64::INFINITY).is_err());
    assert!(result.map_statistic(MAX_TIMESTAMP_MS as f64 + 1.).is_err());
    assert_eq!(
        find(&result.layout.scene, &format!("{}:median", box_.id)).value,
        0.5
    );
    assert!(
        result
            .layout
            .scene
            .marks
            .iter()
            .any(|mark| mark.label.contains("1970-01-01"))
    );
    assert!(
        boxplot_layout_with_scale_options(&frame(), &samples(&[0., 0.5]), &options, &profile)
            .is_err()
    );
    let fractional_domain = BoxPlotLayoutOptions {
        domain: Some([0.5, 1.]),
        ..options
    };
    assert!(
        boxplot_layout_with_scale_options(&frame(), &data, &fractional_domain, &profile).is_err()
    );
    let day = 86_400_000.;
    let leap = 1_709_164_800_000.;
    let daily = BoxPlotLayoutOptions {
        value_axis: AxisOptions {
            tick_count: 5,
            tick_calendar: Some(TimeCalendar {
                interval: Some(TimeInterval::new(TimeUnit::Day, 1).unwrap()),
                ..Default::default()
            }),
            format: ValueFormat::UtcDate,
            ..Default::default()
        },
        ..Default::default()
    };
    let leap_samples = samples(&[leap - day, leap - 100., leap + 100., leap + day]);
    let result =
        boxplot_layout_with_scale_options(&frame(), &leap_samples, &daily, &profile).unwrap();
    assert!(
        result
            .layout
            .scene
            .labels
            .iter()
            .any(|label| label.text == "2024-02-29")
    );
}

#[test]
fn extrema_and_hidden_log_domains_preserve_raw_values_and_admit_only_one_sign() {
    let data = samples(&[7., 12., 30., 40., 97.]);
    let scale = ContinuousScaleOptions {
        kind: ScaleKind::Log,
        nice: ScaleNice::Default,
        clamp: true,
        ..Default::default()
    };
    let mut options = BoxPlotLayoutOptions::default();
    let result = boxplot_layout_with_scale_options(&frame(), &data, &options, &scale).unwrap();
    assert_eq!(result.value_scale.source_domain(), [7., 97.]);
    assert_eq!(result.layout.domain, [1., 100.]);
    assert_eq!(result.layout.boxes[0].extrema, [7., 97.]);
    let mut signed = data;
    signed.push(BoxObservation::new(
        "negative",
        "Sample",
        "hidden",
        Some(-8.),
    ));
    options.domain = Some([1., 100.]);
    assert!(boxplot_layout_with_scale_options(&frame(), &signed, &options, &scale).is_err());
    options.hidden.insert(boxplot_series_id("hidden"));
    let result = boxplot_layout_with_scale_options(&frame(), &signed, &options, &scale).unwrap();
    assert_eq!(result.layout.boxes.len(), 1);
    assert!(
        result
            .layout
            .scene
            .legend
            .iter()
            .any(|entry| entry.label == "hidden")
    );

    let negatives = samples(&[-8., -4., -2.]);
    let hidden = BoxPlotLayoutOptions {
        hidden: [boxplot_series_id("sensor")].into(),
        ..Default::default()
    };
    let result = boxplot_layout_with_scale_options(&frame(), &negatives, &hidden, &scale).unwrap();
    assert_eq!(result.value_scale.source_domain(), [-10., -1.]);
    assert!(result.layout.boxes.is_empty());
}

#[test]
fn clipping_keeps_partially_visible_outlier_centers_and_exact_picking() {
    let c = frame();
    let data = samples(&[-0.1, 2., 4., 6., 8., 10.1]);
    let options = BoxPlotLayoutOptions {
        orientation: StatisticsOrientation::Horizontal,
        domain: Some([0., 10.]),
        outlier_radius: 8.,
        outlier_shape: BoxOutlierShape::Diamond,
        ..Default::default()
    };
    let profile = ContinuousScaleOptions {
        clamp: true,
        ..Default::default()
    };
    let result = boxplot_layout_with_scale_options(&c, &data, &options, &profile).unwrap();
    let box_ = &result.layout.boxes[0];
    let clip = ClipRect::plot(&c);
    let scene = &result.layout.scene;
    let first = find(scene, &format!("{}:outlier:1:0", box_.id));
    let last = find(scene, &format!("{}:outlier:1:5", box_.id));
    assert_eq!((first.value, last.value), (-0.1, 10.1));
    assert_eq!(result.value_scale.map(first.value), clip.x);
    assert_eq!(first.clip, Some(clip));
    assert_eq!(last.clip, Some(clip));
    let diamond_x = |mark: &Mark| {
        let Geometry::Path(path) = &mark.geometry else {
            panic!("expected a diamond path");
        };
        let PathCommand::Move(top) = path[0] else {
            panic!("expected diamond top vertex");
        };
        top.x
    };
    near(diamond_x(first), 16.8, 1e-10);
    near(diamond_x(last), 343.2, 1e-10);
    assert_eq!(scene.hit_test(20.5, box_.center).unwrap().id, first.id);
    assert_eq!(scene.hit_test(339.5, box_.center).unwrap().id, last.id);
    assert_ne!(
        scene.hit_test(19.5, box_.center).map(|mark| &mark.id),
        Some(&first.id)
    );
    assert_ne!(
        scene.hit_test(340.5, box_.center).map(|mark| &mark.id),
        Some(&last.id)
    );
}

#[test]
fn finite_extremes_saturate_beyond_clip_without_changing_box_summaries() {
    let c = frame();
    let data = samples(&[-f64::MAX, -2., 3., 6., f64::MAX]);
    let options = BoxPlotLayoutOptions {
        domain: Some([0., 10.]),
        border_radius: 10.,
        ..Default::default()
    };
    let result =
        boxplot_layout_with_scale_options(&c, &data, &options, &Default::default()).unwrap();
    let box_ = &result.layout.boxes[0];
    assert_eq!(box_.extrema, [-f64::MAX, f64::MAX]);
    assert_eq!(box_.quantiles[1..=3], [-2., 3., 6.]);
    assert!(box_.rect.y + box_.rect.height > ClipRect::plot(&c).y + ClipRect::plot(&c).height);
    assert_eq!(
        box_.outliers
            .iter()
            .map(|point| point.value)
            .collect::<Vec<_>>(),
        [-f64::MAX, f64::MAX]
    );
    result.layout.scene.validate().unwrap();
}

#[cfg(feature = "raster")]
#[test]
fn cpu_clips_rounded_box_without_inventing_a_border_at_the_crop() {
    use kartis::render::raster::*;
    let c = frame();
    let options = BoxPlotLayoutOptions {
        orientation: StatisticsOrientation::Horizontal,
        domain: Some([0., 8.]),
        border_radius: 24.,
        border: Some(GraphicStroke {
            color: Color(0, 0, 0, 255),
            width: 4.,
            cap: StrokeCap::Butt,
        }),
        colors: [("sensor".into(), Color(160, 55, 25, 255))].into(),
        outlier_radius: 0.,
        value_axis: AxisOptions {
            visible: false,
            grid: false,
            ..Default::default()
        },
        group_axis: CategoryAxisOptions {
            visible: false,
            ..Default::default()
        },
        legend: false,
        ..Default::default()
    };
    let scale = ContinuousScaleOptions {
        clamp: true,
        ..Default::default()
    };
    let result = boxplot_layout_with_scale_options(
        &c,
        &samples(&[-18., -4., 4., 9., 10.]),
        &options,
        &scale,
    )
    .unwrap();
    let box_ = &result.layout.boxes[0];
    assert!(box_.rect.x < 20. && box_.rect.x + box_.rect.width > 340.);
    let image = render_raster(
        &result.layout.scene,
        &FontRegistry::default(),
        &RasterOptions::default(),
    )
    .unwrap();
    let pixel = |x: usize, y: usize| &image.rgba[(y * image.width as usize + x) * 4..][..4];
    assert_eq!(pixel(19, 185), &[255, 255, 255, 255]);
    assert_eq!(pixel(20, 185), &[160, 55, 25, 255]);
    assert_eq!(pixel(339, 185), &[160, 55, 25, 255]);
    assert_eq!(pixel(340, 185), &[255, 255, 255, 255]);
}
