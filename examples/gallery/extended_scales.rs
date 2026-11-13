use glory::render::{GraphicStroke, StrokeCap};
use kartis::*;

pub fn scenes(c: &ChartConfig) -> Result<Vec<(&'static str, Scene)>> {
    let groups = ["North", "East", "South", "West"].map(str::to_owned);
    let bars = [
        BarSeries::new("Planned", vec![Some(3.3), Some(7.2), Some(5.6), Some(8.7)]),
        BarSeries::new("Actual", vec![Some(4.9), Some(5.1), Some(6.9), Some(9.3)]),
    ];
    let reversed = bar_layout_with_scale_options(
        c,
        &groups,
        &bars,
        &BarOptions {
            horizontal: true,
            value_domain: Some([0.13, 9.87]),
            value_axis: AxisOptions {
                grid: true,
                ..Default::default()
            },
            ..Default::default()
        },
        &BarStyleOptions::default(),
        &ContinuousScaleOptions {
            nice: ScaleNice::Count(5),
            reverse: true,
            round: true,
            ..Default::default()
        },
    )?;
    let signed = bar_layout_with_scale_options(
        c,
        &groups,
        &[
            BarSeries::new("Sales", vec![Some(95.), Some(45.), Some(85.), Some(120.)]),
            BarSeries::new("Growth", vec![Some(25.), Some(18.), Some(-15.), Some(20.)]),
            BarSeries::new(
                "Returns",
                vec![Some(-14.), Some(-32.), Some(-29.), Some(-8.)],
            ),
        ],
        &BarOptions {
            layout: BarLayout::Stacked,
            value_domain: Some([-180., 180.]),
            corner_radius: 5.,
            ..Default::default()
        },
        &BarStyleOptions {
            totals: BarTotals {
                enabled: true,
                mode: BarTotalMode::PositiveNegative,
                ..Default::default()
            },
            ..Default::default()
        },
        &ContinuousScaleOptions {
            kind: ScaleKind::SymLog { constant: 12. },
            ..Default::default()
        },
    )?;

    let cohorts = ["Trial", "Standard", "Plus"];
    let mut values = Vec::new();
    for (cohort_index, group) in cohorts.into_iter().enumerate() {
        for (index, value) in [2., 4., 4., 4., 8., 16.].into_iter().enumerate() {
            let mut point = SwarmPoint::new(
                format!("{group}-{index}"),
                group,
                value * 2_f64.powi(cohort_index as i32),
            );
            point.size = Some(155. + index as f64 * 34.);
            values.push(point);
        }
    }
    let logarithmic = swarm_layout(
        c,
        &values,
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            groups: Some(cohorts.into_iter().map(SwarmGroup::new).collect()),
            group_gap: 12.,
            spacing: 5.,
            value_scale: ContinuousScaleOptions {
                kind: ScaleKind::Log,
                log_base: 2.,
                ..Default::default()
            },
            value_axis: Some(AxisOptions {
                grid: true,
                legend: Some("Latency (ms)".into()),
                ..Default::default()
            }),
            node: SwarmNodeStyle {
                shape: SwarmNodeShape::FilledGlyph {
                    shape: FilledGlyph::Wye,
                    area: 200.,
                    rotation: 0.2,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    const FEB28: f64 = 1_709_078_400_000.;
    const DAY: f64 = 86_400_000.;
    let dates = ["Spring", "Summer", "Fall"]
        .into_iter()
        .enumerate()
        .flat_map(|(cohort, name)| {
            [0., 1., 1., 2.].into_iter().enumerate().map(move |(i, d)| {
                SwarmPoint::new(
                    format!("{name}-{i}"),
                    name,
                    FEB28 + (d + cohort as f64) * DAY,
                )
            })
        })
        .collect::<Vec<_>>();
    let date_format = ValueFormat::date_time("%b %-d", &TimeLocale::default(), TimeZone::Utc)?;
    let temporal = swarm_layout(
        c,
        &dates,
        &SwarmLayoutOptions {
            groups: Some(
                ["Spring", "Summer", "Fall"]
                    .into_iter()
                    .map(SwarmGroup::new)
                    .collect(),
            ),
            value_scale: ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                reverse: true,
                ..Default::default()
            },
            value_axis: Some(AxisOptions {
                tick_values: Some((0..5).map(|i| FEB28 + i as f64 * DAY).collect()),
                format: date_format.clone(),
                grid: true,
                ..Default::default()
            }),
            value_format: date_format,
            node: SwarmNodeStyle {
                shape: SwarmNodeShape::StrokedGlyph {
                    shape: StrokedGlyph::Diamond,
                    size: 180.,
                    rotation: 0.35,
                },
                stroke: Some(GraphicStroke {
                    color: Color(28, 89, 140, 255),
                    width: 2.,
                    cap: StrokeCap::Round,
                }),
                ..Default::default()
            },
            ..Default::default()
        },
    )?;

    let mut network = ForceSimulation::new(
        &[
            NetworkNode::new("Hub"),
            NetworkNode::new("North"),
            NetworkNode::new("West"),
            NetworkNode::new("East"),
        ],
        &[
            NetworkLink::new("Hub", "North", 4.),
            NetworkLink::new("Hub", "West", 2.),
            NetworkLink::new("Hub", "East", 3.),
        ],
        ForceOptions {
            labels: true,
            ..Default::default()
        },
    )?;
    network.set_position("Hub", [0., 12.])?;
    network.set_position("North", [-4., -92.])?;
    network.set_position("West", [-135., 63.])?;
    network.set_position("East", [148., 64.])?;
    let glyphs = NetworkGlyphOptions {
        nodes: [
            (
                "Hub".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Filled(FilledGlyph::Star),
                        size: 2200.,
                        rotation: 0.2,
                    },
                    ..Default::default()
                },
            ),
            (
                "West".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Filled(FilledGlyph::Wye),
                        size: 1100.,
                        rotation: 0.4,
                    },
                    ..Default::default()
                },
            ),
            (
                "East".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Stroked(StrokedGlyph::Triangle),
                        size: 1600.,
                        rotation: 0.31,
                    },
                    stroke: Some(GraphicStroke {
                        color: Color(28, 89, 140, 255),
                        width: 3.,
                        cap: StrokeCap::Round,
                    }),
                    ..Default::default()
                },
            ),
        ]
        .into(),
    };
    let glyph_scene = network.scene_with_glyphs(c, &glyphs)?;

    let months = ["Jan", "Feb", "Mar"];
    let combo_series = |id, kind, values: [f64; 3]| {
        ComboSeries::new(
            id,
            kind,
            months
                .into_iter()
                .zip(values)
                .map(|(month, value)| ComboPoint::new(month, value))
                .collect(),
        )
    };
    let mut orders = combo_series("Orders", ComboKind::Line, [100., 1000., 10_000.]);
    orders.axis = YAxisSide::Right;
    let combo_frame = ChartConfig {
        margin: Margin {
            right: 76.,
            ..c.margin
        },
        ..c.clone()
    };
    let mixed = combo_layout_with_scale_options(
        &combo_frame,
        &[
            combo_series("Retail", ComboKind::Bar, [2., -3., 4.]),
            combo_series("Wholesale", ComboKind::Bar, [3., -1., 2.]),
            orders,
        ],
        &ComboOptions {
            x: ComboXAxis::Categories {
                values: months.map(str::to_owned).to_vec(),
                padding: 0.16,
            },
            category_window: Some([0.12, 0.94]),
            right: YAxisOptions {
                axis: AxisOptions {
                    tick_values: Some(vec![100., 300., 1000., 3000., 10_000.]),
                    ..Default::default()
                },
                ..Default::default()
            },
            bar_layout: BarLayout::Stacked,
            bar_corner_radius: 5.,
            ..Default::default()
        },
        &ComboScaleOptions {
            left: Some(ContinuousScaleOptions {
                kind: ScaleKind::SymLog { constant: 2. },
                reverse: true,
                round: true,
                nice: ScaleNice::Count(5),
                ..Default::default()
            }),
            right: Some(ContinuousScaleOptions {
                kind: ScaleKind::Log,
                nice: ScaleNice::Default,
                ..Default::default()
            }),
            ..Default::default()
        },
    )?;
    Ok(vec![
        ("Bars · reversed rounded value scale", reversed.layout.scene),
        ("Bars · signed SymLog stacks", signed.layout.scene),
        ("Swarm · logarithmic Wye packing", logarithmic.scene),
        ("Swarm · UTC hollow symbols", temporal.scene),
        (
            "Network · independent force and glyph sizes",
            glyph_scene.scene().clone(),
        ),
        ("Combo · independent SymLog and Log axes", mixed.scene),
    ])
}
