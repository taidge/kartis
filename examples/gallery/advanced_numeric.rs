//! Shared nonlinear colors, full-breakpoint axes and fitted geographic inverse.
use kartis::*;

fn colors(c: &ChartConfig, logarithmic: bool) -> Result<Scene> {
    let frame = ChartConfig {
        margin: Margin {
            left: 60.,
            right: 35.,
            top: 45.,
            bottom: 110.,
        },
        ..c.clone()
    };
    let raw = if logarithmic {
        vec![
            Some(1.),
            Some(3.),
            Some(10.),
            Some(30.),
            Some(100.),
            Some(300.),
            Some(1000.),
            None,
        ]
    } else {
        vec![
            Some(-100.),
            Some(-10.),
            Some(0.),
            Some(5.),
            Some(25.),
            Some(100.),
            Some(500.),
            None,
        ]
    };
    let cells = raw
        .iter()
        .enumerate()
        .map(|(i, value)| HeatCell {
            row: "Observed".into(),
            column: (i + 1).to_string(),
            value: *value,
        })
        .collect::<Vec<_>>();
    let mut layout = heatmap_layout(
        &frame,
        &cells,
        &HeatmapLayoutOptions {
            colors: ColorScaleOptions {
                kind: if logarithmic {
                    ColorScaleKind::Sequential
                } else {
                    ColorScaleKind::Diverging { midpoint: 5. }
                },
                transform: if logarithmic {
                    ColorScaleTransform::Log
                } else {
                    ColorScaleTransform::SymLog { constant: 2. }
                },
                domain: Some(if logarithmic {
                    [1., 1000.]
                } else {
                    [-100., 500.]
                }),
                colors: if logarithmic {
                    vec![Color(230, 244, 250, 255), Color(18, 95, 135, 255)]
                } else {
                    vec![
                        Color(40, 100, 175, 255),
                        Color(245, 245, 235, 255),
                        Color(220, 105, 35, 255),
                    ]
                },
                ..Default::default()
            },
            column_order: Some((1..=raw.len()).map(|i| i.to_string()).collect()),
            axes: HeatmapAxes::none(),
            corner_radius: 5.,
            ..Default::default()
        },
    )?;
    append_color_legend(
        &mut layout.scene,
        &layout.color_scale,
        &ColorLegendOptions {
            position: [90., c.height - 70.],
            length: c.width - 210.,
            thickness: 18.,
            tick_values: Some(if logarithmic {
                vec![1., 10., 100., 1000.]
            } else {
                vec![-100., 0., 5., 25., 500.]
            }),
            unknown_label: Some("Missing".into()),
            ..Default::default()
        },
    )?;
    Ok(layout.scene)
}

fn piecewise_line(c: &ChartConfig) -> Result<Scene> {
    let data = [Series::new(
        "Observed",
        vec![
            Point::new(0., 0.),
            Point::new(1., 2.),
            Point::new(2., 5.),
            Point::new(4., 6.),
            Point::new(6., 8.),
            Point::new(9., 9.),
            Point::new(10., 10.),
        ],
    )];
    let layout = line_layout_with_piecewise_scale_options(
        c,
        &data,
        &LineOptions {
            area: true,
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &PiecewiseLineScaleOptions {
            x: Some(PiecewiseScaleOptions {
                domain: vec![0., 2., 10.],
                range: vec![0., 0.75, 1.],
                ..Default::default()
            }),
            y: Some(PiecewiseScaleOptions {
                domain: vec![0., 5., 10.],
                range: vec![0., 0.2, 1.],
                ..Default::default()
            }),
            ..Default::default()
        },
    )?;
    Ok(layout.layout.scene)
}

fn piecewise_scatter(c: &ChartConfig) -> Result<Scene> {
    let data = [ScatterSeries::new(
        "Observations",
        (0..60)
            .map(|i| {
                let x = 1. + i as f64 * 1.65;
                ScatterDatum::new(format!("sample-{i}"), x, (i as f64 * 0.7).sin() * 8.)
            })
            .collect(),
    )];
    Ok(scatter_layout(
        c,
        &data,
        &ScatterLayoutOptions {
            axes: ScatterAxes {
                bottom: Some(AxisOptions {
                    grid: true,
                    tick_values: Some(vec![1., 2., 5., 10., 20., 50., 100.]),
                    ..Default::default()
                }),
                ..Default::default()
            },
            scales: ScatterScaleOptions {
                x: ScatterAxisScale::Piecewise {
                    options: PiecewiseScaleOptions {
                        domain: vec![1., 10., 100.],
                        range: vec![0., 0.6, 1.],
                        continuous: ContinuousScaleOptions {
                            kind: ScaleKind::Log,
                            ..Default::default()
                        },
                    },
                },
                y: ScatterAxisScale::Piecewise {
                    options: PiecewiseScaleOptions {
                        domain: vec![-10., 0., 10.],
                        range: vec![0., 0.3, 1.],
                        continuous: ContinuousScaleOptions {
                            kind: ScaleKind::SymLog { constant: 2. },
                            ..Default::default()
                        },
                    },
                },
            },
            ..Default::default()
        },
    )?
    .scene)
}

fn inverse_map(c: &ChartConfig, globe: bool) -> Result<Scene> {
    let (ring, positions) = if globe {
        (
            vec![
                [-45., -35.],
                [60., -35.],
                [60., 45.],
                [-45., 45.],
                [-45., -35.],
            ],
            [[-25., -10.], [15., 5.], [45., 25.]],
        )
    } else {
        (
            vec![[40., 22.], [52., 22.], [52., 30.], [40., 30.], [40., 22.]],
            [[42., 24.], [46., 26.], [50., 28.]],
        )
    };
    let mut layout = geographic_map_layout(
        c,
        &[GeoFeature {
            id: "region".into(),
            label: "Region".into(),
            value: Some(1.),
            polygons: vec![vec![ring]],
        }],
        &GeoMapOptions {
            projection: if globe {
                GeoProjection::Orthographic
            } else {
                GeoProjection::EqualEarth
            },
            center: if globe { [15., 10.] } else { [35., 20.] },
            rotation: 25.,
            fit: if globe { GeoFit::World } else { GeoFit::Data },
            fill: Some(Color(222, 235, 242, 255)),
            border: Some(Color(100, 135, 155, 255)),
            ..Default::default()
        },
    )?;
    let mut symbols = Vec::new();
    for (i, source) in positions.into_iter().enumerate() {
        let point = layout
            .projection
            .project(source)?
            .ok_or_else(|| ChartError("example point is outside projection".into()))?;
        let geographic = layout
            .projection
            .invert_in_plot(point)?
            .ok_or_else(|| ChartError("example inverse is outside plot".into()))?;
        let mut symbol = GeoSymbol::new(format!("readback-{i}"), geographic, Some((i + 1) as f64));
        symbol.series = "Readback".into();
        symbol.label = format!("{:.2}°, {:.2}°", geographic[0], geographic[1]);
        symbols.push(symbol);
    }
    layout.add_symbols(
        &symbols,
        &GeoSymbolOptions {
            domain: Some([0., 3.]),
            radius_range: [6., 10.],
            labels: true,
            label_size: 12.,
            ..Default::default()
        },
    )?;
    Ok(layout.scene)
}

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    Ok(vec![
        ("Heatmap · logarithmic colors and legend", colors(c, true)?),
        (
            "Heatmap · symmetric-log diverging center",
            colors(c, false)?,
        ),
        ("Line · full-breakpoint value axes", piecewise_line(c)?),
        (
            "Scatter · piecewise log and symlog axes",
            piecewise_scatter(c)?,
        ),
        ("Geo · fitted inverse readback", inverse_map(c, false)?),
        ("Geo · globe inverse readback", inverse_map(c, true)?),
    ])
}
