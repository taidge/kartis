//! Planar grid topology and a weighted density layer behind raw observations.
use kartis::*;

fn frame(c: &ChartConfig) -> ChartConfig {
    ChartConfig {
        margin: Margin {
            left: 70.,
            right: 30.,
            top: 30.,
            bottom: 110.,
        },
        ..c.clone()
    }
}

fn legend(c: &ChartConfig, title: &str, format: ValueFormat) -> ColorLegendOptions {
    ColorLegendOptions {
        position: [110., c.height - 48.],
        length: c.width - 220.,
        thickness: 12.,
        tick_count: 4,
        format,
        title: title.into(),
        ..Default::default()
    }
}

fn label(scene: &mut Scene, label: Label) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(label);
}

fn y_title(scene: &mut Scene, c: &ChartConfig, text: &str) {
    label(
        scene,
        Label {
            x: 18.,
            y: (c.margin.top + c.height - c.margin.bottom) / 2.,
            text: text.into(),
            color: c.theme.text,
            size: c.theme.font_size,
            anchor: TextAnchor::Middle,
            rotation: -90.,
        },
    );
}

fn nested_grid(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let values = (0usize..225)
        .map(|index| {
            let (x, y) = (index % 15, index / 15);
            Some(if x.min(y).min(14 - x).min(14 - y) / 2 % 2 == 0 {
                2.
            } else {
                0.
            })
        })
        .collect::<Vec<_>>();
    let mut layout = contour_layout(
        &c,
        ContourGrid {
            width: 15,
            height: 15,
            values: &values,
        },
        &ContourOptions {
            thresholds: ContourThresholds::Values(vec![0.5, 1., 1.5]),
            ..Default::default()
        },
        &ContourLayoutOptions {
            colors: ColorScaleOptions {
                colors: vec![Color(205, 236, 234, 255), Color(12, 104, 127, 255)],
                domain: Some([0., 2.]),
                ..Default::default()
            },
            color_legend: Some(legend(&c, "Contour threshold", ValueFormat::Fixed(1))),
            ..Default::default()
        },
    )?;
    let plot = layout.plot;
    for (side, range, name, id) in [
        (
            AxisSide::Bottom,
            [plot.x, plot.x + plot.width],
            "Grid column",
            "grid-column",
        ),
        (
            AxisSide::Left,
            [plot.y, plot.y + plot.height],
            "Grid row",
            "grid-row",
        ),
    ] {
        axis(
            &mut layout.scene,
            &c,
            &ChartScale::new(ScaleKind::Linear, [0., 15.], range)?,
            side,
            &AxisOptions {
                tick_values: Some(vec![0., 5., 10., 15.]),
                legend: (side == AxisSide::Bottom).then(|| name.into()),
                ..Default::default()
            },
            id,
        )?;
    }
    y_title(&mut layout.scene, &c, "Grid row");
    layout.scene.validate()?;
    Ok(layout.scene)
}

fn weighted_density(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let mut weights = DensityWeights::new();
    let data = (0..2)
        .map(|group| {
            let series = if group == 0 { "Coastal" } else { "Inland" };
            let points = (0..100)
                .map(|i| {
                    let angle = i as f64 * 2.399963229728653;
                    let radius = (i as f64 / 100.).sqrt();
                    let x = if group == 0 { 30. } else { 73. } + radius * 12. * angle.cos();
                    let y = if group == 0 { 65. } else { 31. } + radius * 18. * angle.sin();
                    let id = format!("sample-{i}");
                    weights.insert(
                        (series.into(), id.clone()),
                        if group == 0 { 1. } else { 0.65 },
                    );
                    let mut point = ScatterDatum::new(id, x, y);
                    point.color = Some(if group == 0 {
                        Color(32, 91, 156, 255)
                    } else {
                        Color(219, 111, 35, 255)
                    });
                    point
                })
                .collect();
            ScatterSeries::new(series, points)
        })
        .collect::<Vec<_>>();
    let axis = || ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(100.),
    };
    let scatter = scatter_layout(
        &c,
        &data,
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis(),
                y: axis(),
            },
            axes: ScatterAxes {
                bottom: Some(AxisOptions {
                    legend: Some("Observed x".into()),
                    ..Default::default()
                }),
                left: Some(AxisOptions {
                    ..Default::default()
                }),
                ..Default::default()
            },
            node: ScatterNodeStyle {
                shape: ScatterNodeShape::Circle {
                    diameter: ScatterSize::Fixed(5.),
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let mut scene = scatter_density_layout(
        &c,
        &scatter,
        &weights,
        ScatterDensityOptions {
            bandwidth: 18.,
            cell_size: 4.,
            ..Default::default()
        },
        &DensityThresholds::Count(6),
        &ContourLayoutOptions {
            colors: ColorScaleOptions {
                colors: vec![Color(243, 237, 226, 255), Color(125, 98, 161, 255)],
                ..Default::default()
            },
            color_legend: Some(legend(&c, "Weight per square pixel", ValueFormat::Fixed(3))),
            ..Default::default()
        },
    )?
    .scene;
    y_title(&mut scene, &c, "Observed y");
    label(
        &mut scene,
        Label {
            x: c.width / 2.,
            y: 18.,
            text: "Observation weights: Coastal 1.00, Inland 0.65".into(),
            color: c.theme.text,
            size: c.theme.font_size,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        },
    );
    scene.validate()?;
    Ok(scene)
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    Ok(vec![
        ("Contour · nested grid holes", nested_grid(c)?),
        (
            "Density · weighted scatter observations",
            weighted_density(c)?,
        ),
    ])
}
