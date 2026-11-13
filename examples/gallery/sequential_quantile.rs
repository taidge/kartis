//! Empirical ranks determine colors; legends retain the original data axis.
use kartis::*;

fn frame(c: &ChartConfig) -> ChartConfig {
    ChartConfig {
        margin: Margin {
            left: 80.,
            right: 35.,
            top: 45.,
            bottom: 105.,
        },
        ..c.clone()
    }
}
fn colors() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::SequentialQuantile,
        colors: vec![Color(224, 241, 246, 255), Color(19, 87, 132, 255)],
        unknown: Color(225, 229, 234, 255),
        ..Default::default()
    }
}
fn legend(scene: &mut Scene, scale: &ColorScale, c: &ChartConfig) -> Result<()> {
    let [a, b] = scale.domain();
    append_color_legend(
        scene,
        scale,
        &ColorLegendOptions {
            position: [105., c.height - 55.],
            length: c.width - 245.,
            thickness: 14.,
            tick_values: Some(if a == b {
                vec![a]
            } else {
                vec![a, (a + b) / 2., b]
            }),
            title: "Data value (color uses upper sample rank)".into(),
            unknown_label: Some("Missing".into()),
            format: ValueFormat::Auto,
            ..Default::default()
        },
    )?;
    Ok(())
}
fn note(scene: &mut Scene, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: scene.width / 2.,
        y: 20.,
        text: text.into(),
        color: Color(50, 61, 72, 255),
        size: 12.,
        anchor: TextAnchor::Middle,
        rotation: 0.,
    });
}
fn heat(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let raw = [
        Some(0.),
        Some(0.),
        Some(1.),
        Some(1.),
        Some(5.),
        Some(10.),
        Some(10.),
        None,
    ];
    let cells = ["Early", "Middle", "Late"]
        .into_iter()
        .flat_map(|row| {
            raw.into_iter().enumerate().map(move |(i, value)| HeatCell {
                row: row.into(),
                column: format!("v{}", i + 1),
                value,
            })
        })
        .collect::<Vec<_>>();
    let mut layout = heatmap_layout(
        &c,
        &cells,
        &HeatmapLayoutOptions {
            colors: colors(),
            labels: false,
            corner_radius: 4.,
            row_order: Some(vec!["Early".into(), "Middle".into(), "Late".into()]),
            column_order: Some((1..=raw.len()).map(|i| format!("v{i}")).collect()),
            axes: HeatmapAxes::none(),
            x_padding: HeatmapBandPadding {
                inner: 0.08,
                outer: 0.,
            },
            y_padding: HeatmapBandPadding {
                inner: 0.12,
                outer: 0.,
            },
            ..Default::default()
        },
    )?;
    for cell in &layout.cells {
        let Some([x, y]) = cell.center else { continue };
        let color = cell.color;
        let luminance =
            299u32 * u32::from(color.0) + 587u32 * u32::from(color.1) + 114u32 * u32::from(color.2);
        if !layout.scene.draw_order.is_empty() {
            layout
                .scene
                .draw_order
                .push(DrawItem::Label(layout.scene.labels.len()));
        }
        layout.scene.labels.push(Label {
            x,
            y: y + 4.,
            text: cell
                .value
                .map_or_else(|| "NA".into(), |v| format!("{v:.0}")),
            color: if luminance < 140_000 {
                Color(255, 255, 255, 255)
            } else {
                Color(40, 50, 60, 255)
            },
            size: 13.,
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
    }
    legend(&mut layout.scene, &layout.color_scale, &c)?;
    note(
        &mut layout.scene,
        "Repeated values share the upper rank; missing cells stay unknown",
    );
    layout.scene.validate()?;
    Ok(layout.scene)
}
fn calendar(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let raw = [
        Some(0.),
        Some(0.),
        Some(1.),
        Some(1.),
        Some(2.),
        Some(5.),
        Some(10.),
        Some(20.),
        Some(100.),
        None,
    ];
    let data = (1..=28u8)
        .map(|day| CalendarCell::new((2024, 1, day), raw[(day as usize - 1) % raw.len()]))
        .collect::<Vec<_>>();
    let mut layout = calendar_layout(
        &c,
        &data,
        &CalendarLayoutOptions {
            start: Some((2024, 1, 1)),
            end: Some((2024, 1, 28)),
            colors: colors(),
            day_spacing: 4.,
            day_radius: 3.,
            weekday_ticks: vec![],
            month_labels: false,
            year_labels: Some(false),
            year_legend: false,
            day_border: None,
            month_border: None,
            ..Default::default()
        },
    )?;
    legend(&mut layout.scene, &layout.color_scale, &c)?;
    note(
        &mut layout.scene,
        "January observations: a skewed distribution, an original-value legend",
    );
    layout.scene.validate()?;
    Ok(layout.scene)
}
fn geo(c: &ChartConfig) -> Result<Scene> {
    let c = frame(c);
    let raw = [
        Some(0.),
        Some(0.),
        Some(1.),
        Some(5.),
        Some(10.),
        Some(100.),
        None,
        Some(10.),
    ];
    let data = raw
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            let x = -70. + (i % 4) as f64 * 35.;
            let y = 8. + (i / 4) as f64 * 25.;
            GeoFeature {
                id: format!("region-{i}"),
                label: format!("Region {}", i + 1),
                value,
                polygons: vec![vec![vec![
                    [x, y],
                    [x + 28., y + 2.],
                    [x + 25., y + 20.],
                    [x + 3., y + 18.],
                    [x, y],
                ]]],
            }
        })
        .collect::<Vec<_>>();
    let mut layout = choropleth_with_color_scale(
        &c,
        &data,
        &GeoMapOptions {
            fit: GeoFit::Data,
            border: Some(Color(255, 255, 255, 255)),
            border_width: 2.,
            ..Default::default()
        },
        &colors(),
    )?;
    legend(&mut layout.scene, &layout.color_scale, &c)?;
    note(
        &mut layout.scene,
        "All supplied regional values establish one shared rank distribution",
    );
    layout.scene.validate()?;
    Ok(layout.scene)
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    Ok(vec![
        ("Sequential quantile · duplicate HeatMap", heat(c)?),
        ("Sequential quantile · observed calendar", calendar(c)?),
        ("Sequential quantile · regional colors", geo(c)?),
    ])
}
