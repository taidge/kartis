use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let axes = ["Response", "Capacity", "Noise", "Throughput"].map(str::to_owned);
    let log = radar_layout_with_value_scale(
        c,
        &axes,
        &[
            RadarSeries::new("Measured", vec![10., 100., 1., 1000.]),
            RadarSeries::new("Expected", vec![100., 25., 50., 250.]),
        ],
        &RadarLayoutOptions {
            domain: Some([1., 1000.]),
            grid: RadarGridOptions {
                levels: 3,
                tick_labels: true,
                tick_format: ValueFormat::Fixed(0),
                ..Default::default()
            },
            ..Default::default()
        },
        &RadarValueScaleOptions {
            kind: RadarValueScaleKind::Log { base: 10. },
            ..Default::default()
        },
        &RadarInteractionOptions::default(),
    )?;
    let signed = radar_layout_with_value_scale(
        c,
        &axes,
        &[
            RadarSeries::new("Primary", vec![-40., 0., 10., 40.]),
            RadarSeries::new("Secondary", vec![0., -10., 10., 30.]),
        ],
        &RadarLayoutOptions {
            domain: Some([-40., 40.]),
            grid: RadarGridOptions {
                values: Some(vec![-40., -10., 0., 10., 40.]),
                tick_labels: true,
                tick_format: ValueFormat::Fixed(0),
                tick_label_axis: 1,
                ..Default::default()
            },
            ..Default::default()
        },
        &RadarValueScaleOptions {
            kind: RadarValueScaleKind::SymLog { constant: 2. },
            ..Default::default()
        },
        &RadarInteractionOptions {
            direction: PolarSliceDirection::CounterClockwise,
            ..Default::default()
        },
    )?;
    Ok(vec![
        ("Radar · logarithmic decades", log.layout.scene),
        ("Radar · signed SymLog zero ring", signed.layout.scene),
    ])
}
