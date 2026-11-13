#![allow(dead_code)]
use kartis::*;
pub(super) fn frame(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        ..Default::default()
    }
}
pub(super) fn nested() -> Vec<Option<f64>> {
    (0usize..225)
        .map(|i| {
            let (x, y) = (i % 15, i / 15);
            Some(if x.min(y).min(14 - x).min(14 - y) / 2 % 2 == 0 {
                2.
            } else {
                0.
            })
        })
        .collect()
}
pub(super) fn teal() -> ContourLayoutOptions {
    ContourLayoutOptions {
        colors: ColorScaleOptions {
            colors: vec![Color(10, 130, 140, 255); 2],
            ..Default::default()
        },
        ..Default::default()
    }
}
pub(super) fn grid<'a>(values: &'a [Option<f64>]) -> ContourGrid<'a> {
    ContourGrid {
        width: 15,
        height: 15,
        values,
    }
}
pub(super) fn compute(thresholds: Vec<f64>) -> ContourOptions {
    ContourOptions {
        thresholds: ContourThresholds::Values(thresholds),
        ..Default::default()
    }
}

pub(super) fn exact_scatter(c: &ChartConfig, data: &[ScatterSeries]) -> ScatterLayout {
    scatter_layout(
        c,
        data,
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(32.),
                },
                y: ScatterAxisScale::Continuous {
                    options: ContinuousScaleOptions::default(),
                    min: ScatterBound::Fixed(0.),
                    max: ScatterBound::Fixed(24.),
                },
            },
            axes: ScatterAxes {
                top: None,
                right: None,
                bottom: None,
                left: None,
            },
            ..Default::default()
        },
    )
    .unwrap()
}
