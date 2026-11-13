//! Signed power and connected reciprocal profiles through the shared host.
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let c = ChartConfig {
        margin: Margin {
            left: 76.,
            right: 35.,
            top: 35.,
            bottom: 65.,
        },
        ..c.clone()
    };
    let cases = [
        (
            "Scales · signed square and square root",
            [-16., 16.],
            [-16., 16.],
            ScaleKind::Sqrt,
            ScaleKind::Pow { exponent: 2. },
            vec![
                Point::new(-16., -16.),
                Point::new(-9., -4.),
                Point::new(-4., 0.),
                Point::new(0., 4.),
                Point::new(4., 9.),
                Point::new(9., 12.),
                Point::new(16., 16.),
            ],
        ),
        (
            "Scales · reciprocal branch and square root",
            [1., 16.],
            [0., 16.],
            ScaleKind::Pow { exponent: -1. },
            ScaleKind::Sqrt,
            vec![
                Point::new(1., 0.),
                Point::new(2., 1.),
                Point::new(4., 4.),
                Point::new(8., 9.),
                Point::new(16., 16.),
            ],
        ),
    ];
    cases
        .into_iter()
        .map(|(name, x, y, x_kind, y_kind, points)| {
            let layout = line_layout_with_scale_options(
                &c,
                &[Series::new("Observed", points)],
                &LineOptions {
                    x_domain: Some(x),
                    y_domain: Some(y),
                    axes: CartesianAxes {
                        x: AxisOptions {
                            tick_values: Some(if x[0] < 0. {
                                vec![-16., -4., 0., 4., 16.]
                            } else {
                                vec![1., 2., 4., 8., 16.]
                            }),
                            ..Default::default()
                        },
                        y: AxisOptions {
                            tick_count: 5,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &LineStyleOptions {
                    series: LineSeriesStyle {
                        point_radius: 5.,
                        width: 3.,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                &LineScaleOptions {
                    x: ContinuousScaleOptions {
                        kind: x_kind,
                        ..Default::default()
                    },
                    y: ContinuousScaleOptions {
                        kind: y_kind,
                        ..Default::default()
                    },
                },
            )?;
            Ok((name, layout.layout.scene))
        })
        .collect()
}
