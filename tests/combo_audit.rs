//! Independent audit regressions for mixed chart domains and clipping.
use kartis::*;
fn numeric(domain: [f64; 2], y: [f64; 2]) -> ComboOptions {
    ComboOptions {
        x: ComboXAxis::Numeric {
            domain: Some(domain),
            scale: ScaleKind::Linear,
        },
        left: YAxisOptions {
            domain: Some(y),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn bars(id: &str, site: f64, value: f64) -> ComboSeries {
    ComboSeries::new(id, ComboKind::Bar, vec![ComboPoint::new(site, value)])
}
fn bar_width(scene: &Scene, id: &str) -> f64 {
    scene
        .marks
        .iter()
        .find_map(|mark| {
            if mark.series == id {
                if let Geometry::Rect { width, .. } = mark.geometry {
                    Some(width)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap()
}
#[test]
fn grouped_finite_bars_do_not_require_a_representable_stack_total() {
    let data = [
        bars("a", 0., f64::MAX * 0.75),
        bars("b", 0., f64::MAX * 0.75),
    ];
    let result = combo(
        &ChartConfig::default(),
        &data,
        &numeric([-1., 1.], [0., f64::MAX]),
    );
    assert!(
        result.is_ok(),
        "grouped bars never add their raw values: {result:?}"
    );
}
#[test]
fn unrelated_scatter_does_not_shrink_a_single_numeric_bar_site() {
    let c = ChartConfig::default();
    let options = numeric([-5., 5.], [0., 10.]);
    let bar = bars("bars", 0., 5.);
    let alone = combo(&c, std::slice::from_ref(&bar), &options).unwrap();
    let scatter = ComboSeries::new("scatter", ComboKind::Scatter, vec![ComboPoint::new(1., 5.)]);
    let mixed = combo(&c, &[bar, scatter], &options).unwrap();
    assert_eq!(
        bar_width(&alone, "bars"),
        bar_width(&mixed, "bars"),
        "auto bar width must depend only on bar sites"
    );
}
#[test]
fn numeric_bar_with_an_outside_center_still_draws_its_visible_clipped_extent() {
    let c = ChartConfig::default();
    let options = ComboOptions {
        numeric_bar_width: Some(100.),
        ..numeric([0., 10.], [0., 10.])
    };
    let scene = combo(&c, &[bars("overlap", -0.25, 5.)], &options).unwrap();
    let visible = scene.marks.iter().any(|mark| {
        mark.series == "overlap"
            && match mark.geometry {
                Geometry::Rect {
                    x, width, height, ..
                } => x < c.width - c.margin.right && x + width > c.margin.left && height > 0.,
                _ => false,
            }
    });
    assert!(
        visible,
        "the bar overlaps the plot even though its numeric center is outside the x domain"
    );
}
#[test]
fn scatter_clipping_is_independent_of_the_unrelated_line_curve_option() {
    let c = ChartConfig::default();
    let data = [ComboSeries::new(
        "scatter",
        ComboKind::Scatter,
        vec![
            ComboPoint::new(0., 1.),
            ComboPoint::new(0.5, 5.),
            ComboPoint::new(f64::MAX, 5.),
        ],
    )];
    let linear = combo(&c, &data, &numeric([0., 1.], [0., 10.])).unwrap();
    let curved = combo(
        &c,
        &data,
        &ComboOptions {
            curve: Curve::Natural,
            ..numeric([0., 1.], [0., 10.])
        },
    );
    assert!(
        curved.is_ok(),
        "scatter never paints a curve, so discarded line projection must not reject it: {curved:?}"
    );
    let curved = curved.unwrap();
    assert_eq!(
        linear
            .marks
            .iter()
            .filter(|mark| mark.series == "scatter")
            .count(),
        curved
            .marks
            .iter()
            .filter(|mark| mark.series == "scatter")
            .count()
    );
}

#[test]
fn hiding_last_negative_log_x_series_does_not_invalidate_previously_valid_data() {
    let c = ChartConfig::default();
    let data = [ComboSeries::new(
        "negative",
        ComboKind::Line,
        vec![ComboPoint::new(-10., 1.), ComboPoint::new(-1., 2.)],
    )];
    let options = ComboOptions {
        x: ComboXAxis::Numeric {
            domain: None,
            scale: ScaleKind::Log,
        },
        ..Default::default()
    };
    combo(&c, &data, &options).unwrap();
    let hidden = combo(
        &c,
        &data,
        &ComboOptions {
            hidden: ["negative".into()].into_iter().collect(),
            ..options
        },
    );
    assert!(
        hidden.is_ok(),
        "hiding valid negative-log x data must not validate it against the empty positive fallback domain: {hidden:?}"
    );
}
#[test]
fn hiding_last_negative_log_y_series_does_not_invalidate_previously_valid_data() {
    let c = ChartConfig::default();
    let data = [ComboSeries::new(
        "negative",
        ComboKind::Line,
        vec![ComboPoint::new(0., -10.), ComboPoint::new(1., -1.)],
    )];
    let options = ComboOptions {
        left: YAxisOptions {
            scale: ScaleKind::Log,
            ..Default::default()
        },
        ..Default::default()
    };
    combo(&c, &data, &options).unwrap();
    let hidden = combo(
        &c,
        &data,
        &ComboOptions {
            hidden: ["negative".into()].into_iter().collect(),
            ..options
        },
    );
    assert!(
        hidden.is_ok(),
        "hiding valid negative-log y data must not validate it against the empty positive fallback domain: {hidden:?}"
    );
}
