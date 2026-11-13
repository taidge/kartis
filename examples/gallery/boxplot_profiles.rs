use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

fn observations(group: &str, values: impl IntoIterator<Item = f64>) -> Vec<BoxObservation> {
    values
        .into_iter()
        .enumerate()
        .map(|(i, value)| BoxObservation::new(format!("{group}-{i}"), group, "", Some(value)))
        .collect()
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut log_samples = observations("Read", [2., 4., 8., 16., 32., 64., 128.]);
    log_samples.extend(observations("Write", [3., 6., 12., 24., 48., 96., 120.]));
    let log = boxplot_layout_with_scale_options(
        c,
        &log_samples,
        &BoxPlotLayoutOptions {
            orientation: StatisticsOrientation::Horizontal,
            domain: Some([3., 110.]),
            colors: [("".into(), Color(37, 94, 153, 255))].into(),
            border_radius: 8.,
            border: Some(GraphicStroke {
                color: Color(26, 52, 88, 255),
                width: 2.,
                cap: StrokeCap::Round,
            }),
            outlier_radius: 7.,
            outlier_shape: BoxOutlierShape::Diamond,
            outlier_fill: Some(Color(174, 42, 83, 255)),
            value_axis: AxisOptions {
                grid: true,
                legend: Some("Latency in milliseconds · base 2".into()),
                ..Default::default()
            },
            legend: false,
            ..Default::default()
        },
        &ContinuousScaleOptions {
            kind: ScaleKind::Log,
            log_base: 2.,
            nice: ScaleNice::Default,
            reverse: true,
            clamp: true,
            ..Default::default()
        },
    )?;

    let midnight = 1_709_164_800_000.;
    let mut utc_samples = observations(
        "Edge A",
        [-2., -1., 0., 1., 4., 6.].map(|value| midnight + value),
    );
    utc_samples.extend(observations(
        "Edge B",
        [-1., 0., 2., 3., 6., 8.].map(|value| midnight + value),
    ));
    let utc = boxplot_layout_with_scale_options(
        c,
        &utc_samples,
        &BoxPlotLayoutOptions {
            orientation: StatisticsOrientation::Horizontal,
            domain: Some([midnight - 2., midnight + 8.]),
            colors: [("".into(), Color(39, 133, 113, 255))].into(),
            border_radius: 8.,
            median: GraphicStroke {
                color: Color(245, 188, 46, 255),
                width: 3.,
                cap: StrokeCap::Butt,
            },
            value_axis: AxisOptions {
                grid: true,
                tick_values: Some(
                    [-2., 0., 2., 4., 6., 8.]
                        .map(|value| midnight + value)
                        .to_vec(),
                ),
                format: ValueFormat::UtcTime,
                legend: Some("UTC at the start of 2024-02-29".into()),
                ..Default::default()
            },
            legend: false,
            ..Default::default()
        },
        &ContinuousScaleOptions {
            kind: ScaleKind::Utc,
            ..Default::default()
        },
    )?;

    Ok(vec![
        ("BoxPlot · reversed logarithmic samples", log.layout.scene),
        ("BoxPlot · fractional UTC median", utc.layout.scene),
    ])
}
