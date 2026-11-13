use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let frame = ChartConfig {
        margin: Margin {
            top: 42.,
            right: 32.,
            bottom: 110.,
            left: 80.,
        },
        ..c.clone()
    };
    let rows = ["Increasing", "Decreasing", "Offset"];
    let columns: Vec<_> = (0..=10).map(|index| (index * 10).to_string()).collect();
    let mut cells = Vec::new();
    for (row, id) in rows.into_iter().enumerate() {
        for (i, column) in columns.iter().enumerate() {
            let value = match row {
                0 => i * 10,
                1 => (10 - i) * 10,
                _ => (i * 10 + 20).min(100),
            };
            cells.push(HeatCell {
                row: id.into(),
                column: column.clone(),
                value: Some(value as f64),
            });
        }
    }
    let options = HeatmapLayoutOptions {
        colors: ColorScaleOptions {
            domain: Some([0., 100.]),
            colors: vec![Color(240, 43, 95, 255), Color(50, 70, 216, 255)],
            ..Default::default()
        },
        row_order: Some(rows.map(str::to_string).to_vec()),
        column_order: Some(columns),
        x_padding: HeatmapBandPadding {
            inner: 0.05,
            outer: 0.03,
        },
        y_padding: HeatmapBandPadding {
            inner: 0.09,
            outer: 0.05,
        },
        corner_radius: 4.,
        ..Default::default()
    };
    let mut scenes = Vec::new();
    for (name, profile) in [
        (
            "Heatmap · HCL shortest hue",
            ColorScaleProfile::from(ColorInterpolation::Hcl {
                hue: HueInterpolation::Shortest,
            }),
        ),
        (
            "Heatmap · HCL direct hue",
            ColorInterpolation::Hcl {
                hue: HueInterpolation::Direct,
            }
            .into(),
        ),
        ("Heatmap · Viridis", ColorScheme::Viridis.into()),
    ] {
        let mut chart = heatmap_layout_with_color_profile(&frame, &cells, &options, profile)?;
        append_color_legend(
            &mut chart.scene,
            &chart.color_scale,
            &ColorLegendOptions {
                position: [120., 344.],
                length: 480.,
                thickness: 16.,
                steps: 33,
                tick_values: Some(vec![0., 25., 50., 75., 100.]),
                ..Default::default()
            },
        )?;
        scenes.push((name, chart.scene));
    }
    Ok(scenes)
}
