use glory::render::PathCommand;
use kartis::*;

fn frame() -> ChartConfig {
    ChartConfig {
        width: 300.,
        height: 250.,
        margin: Margin {
            left: 40.25,
            right: 50.25,
            top: 35.25,
            bottom: 45.25,
        },
        ..Default::default()
    }
}

fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
}

#[test]
fn nice_round_and_reverse_share_tick_and_bar_positions() {
    let config = frame();
    let options = BarOptions {
        horizontal: true,
        value_domain: Some([0.13, 9.87]),
        value_axis: AxisOptions {
            tick_values: Some(vec![2.5]),
            ..Default::default()
        },
        ..Default::default()
    };
    let result = bar_layout_with_scale_options(
        &config,
        &["Quarter".into()],
        &[BarSeries::new("Series", vec![Some(2.5)])],
        &options,
        &BarStyleOptions::default(),
        &ContinuousScaleOptions {
            nice: ScaleNice::Count(5),
            reverse: true,
            round: true,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(result.value_scale.source_domain(), [0.13, 9.87]);
    assert_eq!(result.value_scale.display_domain(), [0., 10.]);
    assert_eq!(result.layout.value_domain, [0., 10.]);
    let datum = &result.layout.bars[0];
    let expected = (249.75_f64 * 0.75 + 40.25 * 0.25).round();
    assert_eq!(result.value_scale.map(2.5), expected);
    assert_eq!(datum.visible_rect.x, expected);
    near(datum.visible_rect.x + datum.visible_rect.width, 249.75);
    let tick = result
        .layout
        .scene
        .marks
        .iter()
        .find(|mark| mark.id == "axis-bar-value-tick-0")
        .unwrap();
    assert!(matches!(&tick.geometry, Geometry::Path(commands)
        if matches!(commands[0], PathCommand::Move(p) if p.x == expected)));
    let picked = result
        .layout
        .scene
        .hit_test(
            expected + 3.,
            datum.visible_rect.y + datum.visible_rect.height / 2.,
        )
        .unwrap();
    assert_eq!(picked.id, datum.id);
    assert_eq!(picked.value, 2.5);
}

#[test]
fn descending_symlog_stacks_and_totals_follow_both_axis_directions() {
    let categories = ["Quarter".into()];
    let series = [
        BarSeries::new("A", vec![Some(2.)]),
        BarSeries::new("B", vec![Some(4.)]),
        BarSeries::new("C", vec![Some(-4.)]),
    ];
    let style = BarStyleOptions {
        totals: BarTotals {
            enabled: true,
            mode: BarTotalMode::PositiveNegative,
            offset: 7.,
            ..Default::default()
        },
        ..Default::default()
    };
    let config = frame();
    for horizontal in [false, true] {
        let options = BarOptions {
            horizontal,
            value_domain: Some([10., -10.]),
            layout: BarLayout::Stacked,
            ..Default::default()
        };
        for reverse in [false, true] {
            let output = bar_layout_with_scale_options(
                &config,
                &categories,
                &series,
                &options,
                &style,
                &ContinuousScaleOptions {
                    kind: ScaleKind::SymLog { constant: 2. },
                    reverse,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(output.layout.value_domain, [10., -10.]);
            let mut range = if horizontal {
                [40.25, 249.75]
            } else {
                [204.75, 35.25]
            };
            if reverse {
                range.reverse();
            }
            let map = |value: f64| {
                let t =
                    (6_f64.ln() - value.signum() * (value.abs() / 2.).ln_1p()) / (2. * 6_f64.ln());
                range[0] * (1. - t) + range[1] * t
            };
            let second = &output.layout.bars[1];
            assert_eq!(second.interval, [2., 6.]);
            let p = map(2.);
            let q = map(6.);
            let r = second.visible_rect;
            let (start, size) = if horizontal {
                (r.x, r.width)
            } else {
                (r.y, r.height)
            };
            near(start, p.min(q));
            near(size, (p - q).abs());
            let positive_direction = (range[0] - range[1]).signum();
            for (total, value, direction) in [
                (&output.layout.totals[0], 6., positive_direction),
                (&output.layout.totals[1], -4., -positive_direction),
            ] {
                assert_eq!(total.anchor_value, value);
                let position = total.position.unwrap();
                near(
                    position[usize::from(!horizontal)],
                    map(value) + direction * 7.,
                );
                if horizontal {
                    let label = &output.layout.scene.labels[total.label_index.unwrap()];
                    assert_eq!(
                        label.anchor,
                        if direction > 0. {
                            TextAnchor::Start
                        } else {
                            TextAnchor::End
                        }
                    );
                }
            }
        }
    }
}

#[test]
fn clipping_retains_unclamped_rounded_endpoints_and_raw_metadata() {
    let config = frame();
    let mut options = BarOptions {
        horizontal: true,
        padding: 0.,
        corner_radius: 16.,
        value_domain: Some([3., 1.]),
        ..Default::default()
    };
    options.value_axis.visible = false;
    options.value_axis.grid = false;
    options.category_axis.visible = false;
    let data = [BarSeries::new("Outlier", vec![Some(f64::MAX)])];
    let categories = ["Quarter".into()];
    let render = |clamp| {
        bar_layout_with_scale_options(
            &config,
            &categories,
            &data,
            &options,
            &BarStyleOptions::default(),
            &ContinuousScaleOptions {
                clamp,
                ..Default::default()
            },
        )
        .unwrap()
    };
    let unclamped = render(false);
    let clamped = render(true);
    assert_eq!(clamped.layout, unclamped.layout);
    assert!(unclamped.value_scale.map(f64::MAX).is_nan());
    assert_eq!(clamped.value_scale.map(f64::MAX), 40.25);
    let datum = &clamped.layout.bars[0];
    assert_eq!(datum.visible_rect, ClipRect::plot(&config));
    assert_eq!(datum.raw_value, f64::MAX);
    let scene = &clamped.layout.scene;
    assert!(
        scene.hit_test(41., 36.).is_some(),
        "the crop must not round off an artificial end"
    );
    assert!(
        scene.hit_test(40., 36.).is_none(),
        "ink outside the plot must stay clipped"
    );
    assert!(render::to_svg(scene).unwrap().contains(&datum.id));
}

#[test]
fn zero_baseline_rejects_log_and_utc_profiles() {
    for kind in [ScaleKind::Log, ScaleKind::Utc] {
        let error = bar_layout_with_scale_options(
            &frame(),
            &["Quarter".into()],
            &[BarSeries::new("Series", vec![Some(2.)])],
            &BarOptions::default(),
            &BarStyleOptions::default(),
            &ContinuousScaleOptions {
                kind,
                ..Default::default()
            },
        )
        .unwrap_err();
        assert!(error.to_string().contains("zero-baseline"));
    }
}
