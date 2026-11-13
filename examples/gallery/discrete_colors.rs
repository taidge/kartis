//! The same skewed observations, with quantile and authored threshold breaks.
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let frame = ChartConfig {
        margin: Margin {
            top: 42.,
            right: 40.,
            bottom: 125.,
            left: 85.,
        },
        ..c.clone()
    };
    let values = [
        Some(0.),
        Some(0.),
        Some(1.),
        Some(1.),
        Some(2.),
        Some(3.),
        Some(8.),
        Some(15.),
        Some(40.),
        Some(100.),
        None,
    ];
    let cells = values
        .iter()
        .enumerate()
        .map(|(i, value)| HeatCell {
            row: "Observed".into(),
            column: (i + 1).to_string(),
            value: *value,
        })
        .collect::<Vec<_>>();
    let palette = vec![
        Color(42, 91, 151, 255),
        Color(53, 153, 150, 255),
        Color(153, 199, 122, 255),
        Color(239, 183, 71, 255),
    ];
    let mut result = Vec::new();
    for (name, kind, breaks) in [
        (
            "Heatmap · sample quantiles",
            ColorScaleKind::Quantile,
            vec![],
        ),
        (
            "Heatmap · explicit thresholds",
            ColorScaleKind::Threshold,
            vec![1., 5., 20.],
        ),
    ] {
        let mut layout = heatmap_layout(
            &frame,
            &cells,
            &HeatmapLayoutOptions {
                colors: ColorScaleOptions {
                    kind,
                    colors: palette.clone(),
                    thresholds: breaks,
                    domain: Some([0., 100.]),
                    ..Default::default()
                },
                column_order: Some((1..=values.len()).map(|i| i.to_string()).collect()),
                axes: HeatmapAxes::none(),
                corner_radius: 4.,
                ..Default::default()
            },
        )?;
        append_color_legend(
            &mut layout.scene,
            &layout.color_scale,
            &ColorLegendOptions {
                position: [95., 315.],
                length: 470.,
                thickness: 18.,
                tick_values: Some(vec![0., 20., 100.]),
                unknown_label: Some("Missing".into()),
                ..Default::default()
            },
        )?;
        result.push((name, layout.scene));
    }
    Ok(result)
}
