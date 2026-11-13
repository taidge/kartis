//! Shared data-driven styles, using the public typed layout APIs.
use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;
use std::{
    collections::BTreeMap,
    f64::consts::{FRAC_PI_2, PI, TAU},
};
fn note(scene: &mut Scene, c: &ChartConfig, text: &str) {
    if !scene.draw_order.is_empty() {
        scene.draw_order.push(DrawItem::Label(scene.labels.len()));
    }
    scene.labels.push(Label {
        x: c.margin.left,
        y: 20.,
        text: text.into(),
        color: Color(80, 105, 125, 255),
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
}
fn key(scene: &mut Scene, c: &ChartConfig, items: &[(&str, Color)]) {
    for (i, (id, color)) in items.iter().enumerate() {
        let index = scene.labels.len();
        if !scene.draw_order.is_empty() {
            scene.draw_order.push(DrawItem::Label(index));
        }
        scene.label_series.insert(index, (*id).into());
        scene.labels.push(Label {
            x: c.margin.left + i as f64 * 115.,
            y: 42.,
            text: (*id).into(),
            color: *color,
            size: 12.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
}
fn diverging() -> ColorScaleOptions {
    ColorScaleOptions {
        kind: ColorScaleKind::Diverging { midpoint: 0. },
        domain: Some([-20., 20.]),
        colors: vec![
            Color(125, 170, 210, 255),
            Color(247, 249, 251, 255),
            Color(230, 160, 100, 255),
        ],
        unknown: Color(218, 224, 230, 255),
        ..Default::default()
    }
}
fn legend(
    scene: &mut Scene,
    scale: &ColorScale,
    id: &str,
    title: &str,
    unknown: bool,
) -> Result<()> {
    append_color_legend(
        scene,
        scale,
        &ColorLegendOptions {
            id: id.into(),
            position: [155., 344.],
            length: 330.,
            thickness: 12.,
            title: title.into(),
            format: ValueFormat::Fixed(0),
            text_size: 11.,
            unknown_label: unknown.then(|| "Missing".into()),
            ..Default::default()
        },
    )?;
    Ok(())
}
fn heat_cells(rows: &[&str], columns: &[&str], positive: bool) -> Vec<HeatCell> {
    rows.iter()
        .enumerate()
        .flat_map(|(i, row)| {
            columns.iter().enumerate().map(move |(j, column)| HeatCell {
                row: (*row).into(),
                column: (*column).into(),
                value: if i == 0 && j == 1 {
                    None
                } else {
                    Some(if positive {
                        ((i * 23 + j * 17) % 101) as f64
                    } else {
                        ((i * 13 + j * 7) % 41) as f64 - 20.
                    })
                },
            })
        })
        .collect()
}
pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let mut output = Vec::with_capacity(10);
    let bars_frame = ChartConfig {
        margin: Margin {
            left: 82.,
            right: 55.,
            top: 64.,
            bottom: 52.,
        },
        ..c.clone()
    };
    let categories = ["Q1", "Q2", "Q3", "Q4"].map(str::to_string);
    let data = [
        BarSeries::new("North", vec![Some(28.), Some(42.), Some(34.), Some(58.)]),
        BarSeries::new("South", vec![Some(18.), Some(31.), Some(25.), Some(37.)]),
        BarSeries::new("Online", vec![Some(12.), Some(22.), Some(18.), Some(30.)]),
    ];
    let mut style = BarStyleOptions {
        category_inner_padding: Some(0.16),
        category_outer_padding: Some(0.25),
        inner_padding: 7.,
        labels: BarLabels {
            enabled: true,
            position: BarLabelPosition::End,
            offset: -13.,
            skip_width: 18.,
            skip_height: 22.,
            format: ValueFormat::Fixed(0),
            style: BarTextStyle {
                color: Some(Color(255, 255, 255, 255)),
                size: Some(11.),
                ..Default::default()
            },
            ..Default::default()
        },
        totals: BarTotals {
            enabled: true,
            format: ValueFormat::Affix {
                prefix: "Σ".into(),
                suffix: String::new(),
                precision: 0,
            },
            style: BarTextStyle {
                color: Some(Color(115, 60, 145, 255)),
                size: Some(12.),
                ..Default::default()
            },
            offset: 8.,
            ..Default::default()
        },
        colors: BTreeMap::from([
            ("North".into(), c.theme.color(0)),
            ("South".into(), c.theme.color(1)),
            ("Online".into(), c.theme.color(4)),
        ]),
        ..Default::default()
    };
    let mut grouped = bar_with_style(
        &bars_frame,
        &categories,
        &data,
        &BarOptions {
            corner_radius: 5.,
            value_domain: Some([0., 75.]),
            ..Default::default()
        },
        &style,
    )?;
    note(
        &mut grouped,
        &bars_frame,
        "Independent band padding + pixel gaps · Σ is the raw sum",
    );
    key(
        &mut grouped,
        &bars_frame,
        &[
            ("North", c.theme.color(0)),
            ("South", c.theme.color(1)),
            ("Online", c.theme.color(4)),
        ],
    );
    output.push(("Bar independent padding and totals", grouped));
    let channels = ["Retail", "Business", "Online", "Other"].map(str::to_string);
    let signed = [
        BarSeries::new("Revenue", vec![Some(35.), Some(55.), Some(40.), Some(20.)]),
        BarSeries::new("Returns", vec![Some(-8.), Some(-15.), Some(-5.), Some(-6.)]),
        BarSeries::new("Upsell", vec![Some(12.), Some(18.), Some(10.), Some(8.)]),
    ];
    style.color_by = BarColorBy::Category;
    style.colors = channels
        .iter()
        .cloned()
        .zip([
            c.theme.color(0),
            c.theme.color(4),
            c.theme.color(1),
            c.theme.color(6),
        ])
        .collect();
    style.labels.position = BarLabelPosition::Middle;
    style.labels.offset = 0.;
    style.totals.mode = BarTotalMode::PositiveNegative;
    style.legend = false;
    let mut signed_scene = bar_with_style(
        &bars_frame,
        &channels,
        &signed,
        &BarOptions {
            layout: BarLayout::Stacked,
            horizontal: true,
            corner_radius: 5.,
            value_domain: Some([-25., 90.]),
            ..Default::default()
        },
        &style,
    )?;
    note(
        &mut signed_scene,
        &bars_frame,
        "Category colors · separate positive and negative raw totals",
    );
    output.push(("Signed bar totals and category colors", signed_scene));
    let months = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"].map(str::to_string);
    let percent = [
        BarSeries::new(
            "North",
            vec![
                Some(24.),
                Some(35.),
                Some(40.),
                Some(50.),
                Some(44.),
                Some(62.),
            ],
        ),
        BarSeries::new(
            "South",
            vec![
                Some(16.),
                Some(20.),
                Some(20.),
                Some(15.),
                Some(26.),
                Some(18.),
            ],
        ),
        BarSeries::new(
            "Returns",
            vec![
                Some(-5.),
                Some(-10.),
                Some(-6.),
                Some(-8.),
                Some(-12.),
                Some(-9.),
            ],
        ),
    ];
    style.color_by = BarColorBy::Series;
    style.colors = BTreeMap::from([
        ("North".into(), c.theme.color(0)),
        ("South".into(), c.theme.color(1)),
        ("Returns".into(), c.theme.color(2)),
    ]);
    style.labels.value = BarLabelValue::Displayed;
    style.labels.format = ValueFormat::Percent(0);
    style.labels.skip_width = 25.;
    style.labels.skip_height = 16.;
    style.totals.mode = BarTotalMode::Net;
    style.tooltip_format = Some(ValueFormat::Fixed(0));
    let mut normalized = bar_with_style(
        &bars_frame,
        &months,
        &percent,
        &BarOptions {
            layout: BarLayout::Percent,
            corner_radius: 7.,
            category_window: Some([0.13, 0.87]),
            value_domain: Some([-1., 1.]),
            value_axis: AxisOptions {
                format: ValueFormat::Percent(0),
                grid: true,
                ..Default::default()
            },
            ..Default::default()
        },
        &style,
    )?;
    note(
        &mut normalized,
        &bars_frame,
        "Normalized ink + labels · raw Σ totals · original edge rounding",
    );
    key(
        &mut normalized,
        &bars_frame,
        &[
            ("North", c.theme.color(0)),
            ("South", c.theme.color(1)),
            ("Returns", c.theme.color(2)),
        ],
    );
    output.push(("Percent bar raw totals and window", normalized));

    let heat_frame = ChartConfig {
        margin: Margin {
            left: 82.,
            right: 45.,
            top: 64.,
            bottom: 90.,
        },
        ..c.clone()
    };
    let rows = ["North", "South", "East", "West"];
    let columns = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"];
    let cells = heat_cells(&rows, &columns, false);
    let mut heat = HeatmapLayoutOptions {
        colors: diverging(),
        x_padding: HeatmapBandPadding {
            inner: 0.06,
            outer: 0.03,
        },
        y_padding: HeatmapBandPadding {
            inner: 0.08,
            outer: 0.04,
        },
        corner_radius: 5.,
        border: Some(GraphicStroke {
            color: Color(255, 255, 255, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        }),
        value_format: ValueFormat::Fixed(0),
        empty_label: "—".into(),
        row_order: Some(rows.map(str::to_string).to_vec()),
        column_order: Some(columns.map(str::to_string).to_vec()),
        ..Default::default()
    };
    let mut layout = heatmap_layout(&heat_frame, &cells, &heat)?;
    legend(
        &mut layout.scene,
        &layout.color_scale,
        "heat-diverging",
        "Change · explicit midpoint 0",
        true,
    )?;
    note(
        &mut layout.scene,
        &heat_frame,
        "Diverging scale · independent X/Y padding · explicit missing cell",
    );
    output.push(("Heatmap diverging colors and labels", layout.scene));
    let circle_rows = ["Alpha", "Beta", "Gamma", "Delta", "Epsilon"];
    let circle_columns = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    let circle_cells = heat_cells(&circle_rows, &circle_columns, true);
    let circle_options = HeatmapLayoutOptions {
        shape: HeatmapShape::Circle,
        force_square: true,
        size_variation: Some(HeatmapSizeVariation {
            domain: Some([0., 100.]),
            sizes: [0.15, 1.],
        }),
        colors: ColorScaleOptions {
            domain: Some([0., 100.]),
            colors: vec![
                Color(218, 238, 248, 255),
                Color(80, 155, 190, 255),
                Color(25, 80, 130, 255),
            ],
            ..Default::default()
        },
        labels: false,
        border: Some(GraphicStroke {
            color: Color(255, 255, 255, 255),
            width: 1.,
            cap: StrokeCap::Butt,
        }),
        x_padding: HeatmapBandPadding {
            inner: 0.1,
            outer: 0.1,
        },
        y_padding: HeatmapBandPadding {
            inner: 0.1,
            outer: 0.1,
        },
        row_order: Some(circle_rows.map(str::to_string).to_vec()),
        column_order: Some(circle_columns.map(str::to_string).to_vec()),
        ..Default::default()
    };
    let mut circles = heatmap_layout(&heat_frame, &circle_cells, &circle_options)?;
    legend(
        &mut circles.scene,
        &circles.color_scale,
        "heat-size",
        "Score · diameter varies with value",
        true,
    )?;
    note(
        &mut circles.scene,
        &heat_frame,
        "Circle cells · sizeVariation and color share an explicit domain",
    );
    output.push(("Heatmap circles and size variation", circles.scene));
    heat.active = Some(HeatmapCellKey::new("East", "Apr"));
    heat.hover_target = HeatmapHoverTarget::RowColumn;
    heat.inactive_opacity = 0.15;
    let mut focused = heatmap_layout(&heat_frame, &cells, &heat)?;
    legend(
        &mut focused.scene,
        &focused.color_scale,
        "heat-focus",
        "Shared color scale remains unchanged",
        true,
    )?;
    note(
        &mut focused.scene,
        &heat_frame,
        "Controlled row-column focus: East × Apr · exact related cell set",
    );
    output.push(("Heatmap row-column focus", focused.scene));

    let polar_frame = ChartConfig {
        margin: Margin {
            left: 100.,
            right: 100.,
            top: 62.,
            bottom: 52.,
        },
        ..c.clone()
    };
    let segments = ["Renewal", "New", "Upsell"].map(str::to_string);
    let rings = [
        BarSeries::new("North", vec![Some(50.), Some(25.), Some(15.)]),
        BarSeries::new("South", vec![Some(35.), Some(30.), Some(20.)]),
        BarSeries::new("West", vec![Some(45.), Some(15.), Some(12.)]),
    ];
    let axes = PolarAxesOptions {
        id: "gallery-rings".into(),
        radial_start: Some(PolarAxisStyle {
            text_size: Some(10.),
            ..Default::default()
        }),
        circular_outer: Some(PolarAxisStyle {
            text_size: Some(11.),
            tick_padding: 8.,
            ..Default::default()
        }),
        grid: PolarGridStyle {
            color: Some(Color(155, 175, 190, 100)),
            dash: Some(StrokeDash {
                segments: vec![3., 4.],
                offset: 0.,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    let ring_options = StackedRadialBarOptions {
        start_angle: -FRAC_PI_2,
        end_angle: PI,
        inner_radius: 0.28,
        max_value: Some(100.),
        pad_angle: 0.025,
        track_color: Some(Color(237, 242, 246, 255)),
        colors: BTreeMap::from([
            ("Renewal".into(), c.theme.color(0)),
            ("New".into(), c.theme.color(1)),
            ("Upsell".into(), c.theme.color(4)),
        ]),
        labels: Some(RadialBarLabels {
            text: RadialBarLabelText::Value(ValueFormat::Fixed(0)),
            skip_angle: 0.25,
            color: Some(Color(255, 255, 255, 255)),
            size: Some(11.),
            ..Default::default()
        }),
        axes,
        value_ticks: PolarTickOptions {
            values: Some(vec![0., 25., 50., 75., 100.]),
            ..Default::default()
        },
        ..Default::default()
    };
    let mut radial = stacked_radial_bar(&polar_frame, &segments, &rings, &ring_options)?;
    note(
        &mut radial,
        &polar_frame,
        "Each ring owns a series · category values accumulate in angle",
    );
    key(
        &mut radial,
        &polar_frame,
        &[
            ("Renewal", c.theme.color(0)),
            ("New", c.theme.color(1)),
            ("Upsell", c.theme.color(4)),
        ],
    );
    output.push(("Stacked radial bars with axes", radial));
    let sectors = ["A", "B", "C", "D", "E", "F"].map(str::to_string);
    let values = [
        BarSeries::new(
            "Actual",
            vec![
                Some(20.),
                Some(35.),
                Some(45.),
                Some(30.),
                Some(50.),
                Some(40.),
            ],
        ),
        BarSeries::new(
            "Additional",
            vec![
                Some(15.),
                Some(20.),
                Some(10.),
                Some(25.),
                Some(15.),
                Some(20.),
            ],
        ),
    ];
    let mut axes = PolarAxesOptions {
        id: "gallery-polar".into(),
        ..Default::default()
    };
    axes.circular_outer.as_mut().unwrap().text_size = Some(12.);
    axes.radial_start.as_mut().unwrap().text_size = Some(11.);
    axes.grid.color = Some(Color(140, 165, 185, 130));
    let mut polar = polar_bar_with_axes(
        &polar_frame,
        &sectors,
        &values,
        &PolarBarOptions {
            area_scale: true,
            inner_radius: 0.18,
            pad_angle: 0.06,
            start_angle: 0.,
            end_angle: TAU,
            ..Default::default()
        },
        &axes,
        &PolarTickOptions {
            values: Some(vec![0., 20., 40., 60.]),
            format: ValueFormat::Fixed(0),
            ..Default::default()
        },
    )?;
    note(
        &mut polar,
        &polar_frame,
        "Radius ticks follow the area-preserving square-root mapping",
    );
    key(
        &mut polar,
        &polar_frame,
        &[
            ("Actual", c.theme.color(0)),
            ("Additional", c.theme.color(1)),
        ],
    );
    output.push(("Polar bars with calibrated axes", polar));

    let map_frame = ChartConfig {
        margin: Margin {
            left: 55.,
            right: 55.,
            top: 52.,
            bottom: 100.,
        },
        ..c.clone()
    };
    let regions = [
        ("West", [-130., 10., -70., 55.], Some(-18.)),
        ("Central", [-45., 10., 15., 55.], Some(-5.)),
        ("East", [45., 10., 105., 55.], Some(15.)),
        ("Southwest", [-105., -45., -45., -5.], Some(0.)),
        ("Southeast", [0., -45., 60., -5.], Some(8.)),
        ("Unknown", [85., -45., 135., -5.], None),
    ];
    let features = regions
        .iter()
        .map(|(id, [x0, y0, x1, y1], value)| GeoFeature {
            id: (*id).into(),
            label: (*id).into(),
            value: *value,
            polygons: vec![vec![vec![[*x0, *y0], [*x1, *y0], [*x1, *y1], [*x0, *y1]]]],
        })
        .collect::<Vec<_>>();
    let mut mapped = choropleth_with_color_scale(
        &map_frame,
        &features,
        &GeoMapOptions {
            projection: GeoProjection::EqualEarth,
            fit: GeoFit::Data,
            padding: 8.,
            border: Some(Color(135, 150, 165, 255)),
            border_width: 1.,
            ..Default::default()
        },
        &diverging(),
    )?;
    for (id, [x0, y0, x1, y1], _) in &regions {
        if let Some(p) = mapped
            .projection
            .project([(x0 + x1) / 2., (y0 + y1) / 2.])?
        {
            let index = mapped.scene.labels.len();
            if !mapped.scene.draw_order.is_empty() {
                mapped.scene.draw_order.push(DrawItem::Label(index));
            }
            mapped.scene.label_series.insert(index, (*id).into());
            mapped.scene.labels.push(Label {
                x: p[0],
                y: p[1],
                text: (*id).into(),
                color: Color(35, 50, 65, 255),
                size: 11.,
                anchor: TextAnchor::Middle,
                rotation: 0.,
            });
        }
    }
    legend(
        &mut mapped.scene,
        &mapped.color_scale,
        "map-shared",
        "Change · same diverging scale as Heatmap",
        true,
    )?;
    note(
        &mut mapped.scene,
        &map_frame,
        "Equal Earth polygons · zero and unknown remain distinct",
    );
    output.push(("Choropleth shared diverging scale", mapped.scene));
    let calendar_frame = ChartConfig {
        margin: Margin {
            left: 64.,
            right: 35.,
            top: 55.,
            bottom: 90.,
        },
        ..c.clone()
    };
    let mut days = vec![];
    let mut ordinal = 0usize;
    for year in [2023, 2024] {
        for (month, length) in [
            31,
            if year == 2024 { 29 } else { 28 },
            31,
            30,
            31,
            30,
            31,
            31,
            30,
            31,
            30,
            31,
        ]
        .into_iter()
        .enumerate()
        {
            for day in 1..=length {
                if !ordinal.is_multiple_of(7) {
                    days.push(CalendarDate::new(
                        year,
                        (month + 1) as u8,
                        day as u8,
                        ((ordinal * 17) % 101) as f64,
                    ));
                }
                ordinal += 1;
            }
        }
    }
    let colors = ColorScaleOptions {
        kind: ColorScaleKind::Quantize,
        domain: Some([0., 100.]),
        colors: vec![
            Color(221, 240, 238, 255),
            Color(160, 216, 204, 255),
            Color(85, 180, 160, 255),
            Color(40, 140, 130, 255),
            Color(35, 100, 105, 255),
        ],
        unknown: Color(228, 232, 237, 255),
        ..Default::default()
    };
    let mut calendar = calendar_range_with_color_scale(
        &calendar_frame,
        &days,
        &CalendarRangeOptions {
            years: Some((2023, 2024)),
            year_gap: 28.,
            ..Default::default()
        },
        &colors,
    )?;
    append_color_legend(
        &mut calendar.scene,
        &calendar.color_scale,
        &ColorLegendOptions {
            id: "calendar-bins".into(),
            position: [155., 344.],
            length: 330.,
            thickness: 12.,
            tick_values: Some(vec![0., 20., 40., 60., 80., 100.]),
            format: ValueFormat::Fixed(0),
            text_size: 11.,
            title: "Fixed bins shared across years".into(),
            unknown_label: Some("Missing".into()),
            ..Default::default()
        },
    )?;
    note(
        &mut calendar.scene,
        &calendar_frame,
        "Quantized colors · one data domain for 2023 and leap-year 2024",
    );
    output.push(("Calendar shared quantized scale", calendar.scene));
    for (_, scene) in &output {
        scene.validate()?;
    }
    Ok(output)
}
