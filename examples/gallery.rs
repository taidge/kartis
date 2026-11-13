//! cargo run --example gallery -- target/gallery
use kartis::*;
use std::{fmt::Write, path::PathBuf};
#[path = "gallery/advanced_numeric.rs"]
mod advanced_numeric;
#[path = "gallery/blend_demo.rs"]
mod blend_demo;
#[path = "gallery/boxplot_profiles.rs"]
mod boxplot_profiles;
#[cfg(feature = "raster")]
#[path = "gallery/color_fonts.rs"]
mod color_fonts;
#[path = "gallery/color_interpolation.rs"]
mod color_interpolation;
#[path = "gallery/configurable.rs"]
mod configurable;
#[path = "gallery/configured_paint.rs"]
mod configured_paint;
#[path = "shared/geo_conic.rs"]
pub mod conic_model;
#[path = "gallery/contour_density.rs"]
mod contour_density;
#[path = "gallery/data_styles.rs"]
mod data_styles;
#[path = "gallery/discrete_colors.rs"]
mod discrete_colors;
#[path = "gallery/extended_scales.rs"]
mod extended_scales;
#[path = "gallery/flow_layouts.rs"]
mod flow_layouts;
#[path = "gallery/geo_glyphs.rs"]
mod geo_glyphs;
#[path = "gallery/interactions.rs"]
mod interactions;
#[path = "gallery/layout_models.rs"]
mod layout_models;
#[path = "gallery/line_custom.rs"]
mod line_custom;
#[cfg(feature = "raster")]
#[path = "gallery/measured_sunburst.rs"]
mod measured_sunburst;
#[path = "gallery/p1.rs"]
mod p1;
#[path = "gallery/p2.rs"]
mod p2;
#[path = "gallery/power_scales.rs"]
mod power_scales;
#[path = "gallery/presentation.rs"]
mod presentation;
#[path = "gallery/radar_scales.rs"]
mod radar_scales;
#[path = "gallery/radial_focus.rs"]
mod radial_focus;
#[path = "gallery/radial_label_gate.rs"]
mod radial_label_gate;
#[path = "gallery/scale_glyphs.rs"]
mod scale_glyphs;
#[path = "gallery/scatter.rs"]
mod scatter;
#[path = "gallery/scatter_custom.rs"]
mod scatter_custom;
#[path = "gallery/sequential_quantile.rs"]
mod sequential_quantile;
#[path = "shared/geo_spherical.rs"]
mod spherical_model;
#[path = "gallery/styles.rs"]
mod styles;
#[cfg(feature = "raster")]
#[path = "gallery/typography.rs"]
mod typography;
#[path = "gallery/upgrades.rs"]
mod upgrades;

// A comparison uses the same fixed domains, plot, axes and source points.
// Additional profiles append only their series geometry to the first scene.
fn compare_curves(
    frame: &ChartConfig,
    data: &[Point],
    profiles: &[(&str, Curve)],
    domains: [[f64; 2]; 2],
) -> Result<Scene> {
    let mut output = frame.scene()?;
    for (i, (name, curve)) in profiles.iter().enumerate() {
        let mut colored = frame.clone();
        let color = frame.theme.color(i);
        colored.theme.colors = vec![color];
        let mut scene = line(
            &colored,
            &[Series::new(*name, data.to_vec())],
            &LineOptions {
                curve: *curve,
                points: i == 0,
                x_domain: Some(domains[0]),
                y_domain: Some(domains[1]),
                ..Default::default()
            },
        )?;
        if i == 0 {
            output.labels.append(&mut scene.labels);
        }
        for mut mark in scene.marks {
            if i > 0 && mark.series.is_empty() {
                continue;
            }
            mark.id = format!("comparison-{i}-{}", mark.id);
            output.marks.push(mark);
        }
        output.legend.append(&mut scene.legend);
        output.labels.push(Label {
            x: frame.margin.left + i as f64 * 190.,
            y: 20.,
            text: (*name).into(),
            color,
            size: 12.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
    }
    output.validate()?;
    Ok(output)
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/gallery"));
    std::fs::create_dir_all(&output)?;
    let c = ChartConfig {
        width: 720.,
        height: 400.,
        ..Default::default()
    };
    let points = vec![
        Series::new(
            "North",
            (0..12)
                .map(|i| Point::new(i as f64, 20. + (i as f64 * 0.7).sin() * 12. + i as f64))
                .collect(),
        ),
        Series::new(
            "South",
            (0..12)
                .map(|i| Point::new(i as f64, 12. + (i as f64 * 0.5).cos() * 7.))
                .collect(),
        ),
    ];
    let categories: Vec<String> = ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    let bars = vec![
        BarSeries::new(
            "North",
            vec![
                Some(32.),
                Some(44.),
                Some(-18.),
                Some(52.),
                Some(36.),
                Some(45.),
            ],
        ),
        BarSeries::new(
            "South",
            vec![
                Some(21.),
                Some(28.),
                Some(-14.),
                Some(32.),
                Some(26.),
                Some(19.),
            ],
        ),
    ];
    let positive = vec![
        BarSeries::new(
            "North",
            vec![
                Some(32.),
                Some(44.),
                Some(18.),
                Some(52.),
                Some(36.),
                Some(45.),
            ],
        ),
        BarSeries::new(
            "South",
            vec![
                Some(21.),
                Some(28.),
                Some(14.),
                Some(32.),
                Some(26.),
                Some(19.),
            ],
        ),
    ];
    let radial: Vec<_> = [("North", 38.), ("South", 27.), ("East", 21.), ("West", 14.)]
        .into_iter()
        .map(|(id, value)| RadialDatum::new(id, value))
        .collect();
    let hierarchy = vec![
        HierarchyDatum::new("All", None, 0.),
        HierarchyDatum::new("Europe", Some("All".into()), 0.),
        HierarchyDatum::new("Asia", Some("All".into()), 0.),
        HierarchyDatum::new("DE", Some("Europe".into()), 24.),
        HierarchyDatum::new("FR", Some("Europe".into()), 16.),
        HierarchyDatum::new("CN", Some("Asia".into()), 38.),
        HierarchyDatum::new("JP", Some("Asia".into()), 22.),
    ];
    let mut scenes = vec![
        (
            "Line",
            line(
                &c,
                &points,
                &LineOptions {
                    curve: Curve::Monotone,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Area",
            line(
                &c,
                &points,
                &LineOptions {
                    area: true,
                    curve: Curve::Monotone,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Grouped bar",
            bar(&c, &categories, &bars, &BarOptions::default())?,
        ),
        (
            "Diverging stacked bar",
            bar(
                &c,
                &categories,
                &bars,
                &BarOptions {
                    layout: BarLayout::Stacked,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Horizontal bar",
            bar(
                &c,
                &categories,
                &bars,
                &BarOptions {
                    horizontal: true,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Percent bar",
            bar(
                &c,
                &categories,
                &positive,
                &BarOptions {
                    layout: BarLayout::Percent,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Marimekko",
            marimekko(&c, &categories, &positive, &MarimekkoOptions::default())?,
        ),
        (
            "SwarmPlot",
            swarmplot(
                &c,
                &(0..80)
                    .map(|i| {
                        SwarmDatum::new(
                            format!("sample-{i}"),
                            if i < 40 { "North" } else { "South" },
                            12. + (i % 5) as f64 * 4.,
                        )
                    })
                    .collect::<Vec<_>>(),
                &SwarmOptions::default(),
            )?,
        ),
        (
            "Polar bar",
            polar_bar(&c, &categories, &positive, &PolarBarOptions::default())?,
        ),
        ("Scatter", scatter(&c, &points, &ScatterOptions::default())?),
        (
            "Stream",
            stream(
                &c,
                &categories,
                &positive,
                &StreamOptions {
                    offset: StreamOffset::Center,
                    ..Default::default()
                },
            )?,
        ),
        ("Pie", pie(&c, &radial, &PieOptions::default())?),
        (
            "Donut",
            pie(
                &c,
                &radial,
                &PieOptions {
                    inner_radius: 0.65,
                    ..Default::default()
                },
            )?,
        ),
        (
            "Radial bar",
            radial_bar(&c, &radial, &RadialBarOptions::default())?,
        ),
        (
            "Radar",
            radar(
                &c,
                &[
                    "Speed".into(),
                    "Quality".into(),
                    "Reach".into(),
                    "Cost".into(),
                    "Growth".into(),
                ],
                &[
                    RadarSeries::new("North", vec![80., 65., 90., 55., 72.]),
                    RadarSeries::new("South", vec![55., 85., 65., 80., 60.]),
                ],
                &RadarOptions::default(),
            )?,
        ),
        ("Funnel", funnel(&c, &radial, &FunnelOptions::default())?),
        ("Waffle", waffle(&c, &radial, &WaffleOptions::default())?),
        (
            "Treemap",
            treemap(&c, &hierarchy, &HierarchyOptions::default())?,
        ),
        (
            "Sunburst",
            sunburst(&c, &hierarchy, &HierarchyOptions::default())?,
        ),
        (
            "Icicle",
            icicle(&c, &hierarchy, &HierarchyOptions::default())?,
        ),
        ("Tree", tree(&c, &hierarchy, &HierarchyOptions::default())?),
        (
            "Circle packing",
            circle_packing(&c, &hierarchy, &HierarchyOptions::default())?,
        ),
        (
            "Chord",
            chord(
                &c,
                &["North".into(), "South".into(), "East".into()],
                &[vec![8., 20., 10.], vec![15., 5., 18.], vec![12., 22., 4.]],
                0.05,
            )?,
        ),
        (
            "Calendar",
            calendar(
                &c,
                2026,
                &(1..=12)
                    .flat_map(|month| {
                        (1..=28).map(move |day| CalendarDay {
                            month,
                            day,
                            value: ((month as f64 + day as f64) * 0.7).sin() * 25. + 30.,
                        })
                    })
                    .collect::<Vec<_>>(),
            )?,
        ),
        (
            "Box plot",
            boxplot(
                &c,
                &[
                    BoxSample::new("North", vec![4., 7., 12., 14., 18., 22., 25., 65.]),
                    BoxSample::new("South", vec![9., 11., 16., 19., 24., 30., 34., 41.]),
                ],
            )?,
        ),
        (
            "Bullet",
            bullet(
                &c,
                &[
                    BulletDatum {
                        id: "Revenue".into(),
                        ranges: vec![100., 75., 50.],
                        measures: vec![82.],
                        markers: vec![90.],
                    },
                    BulletDatum {
                        id: "Profit".into(),
                        ranges: vec![100., 65., 35.],
                        measures: vec![64.],
                        markers: vec![75.],
                    },
                ],
            )?,
        ),
        (
            "Parallel coordinates",
            parallel_coordinates(
                &c,
                &[
                    "Speed".into(),
                    "Memory".into(),
                    "Quality".into(),
                    "Cost".into(),
                ],
                &(0..8)
                    .map(|i| ParallelRecord {
                        id: format!("Model {}", i + 1),
                        values: (0..4)
                            .map(|j| Some(((i * 13 + j * 7) % 50 + 10) as f64))
                            .collect(),
                    })
                    .collect::<Vec<_>>(),
            )?,
        ),
    ];
    let control_points = [14., 38., 18., 45., 24., 42., 19., 34.]
        .into_iter()
        .enumerate()
        .map(|(i, y)| Point::new(i as f64, y))
        .collect::<Vec<_>>();
    for (name, curve) in [
        ("Basis curve", Curve::Basis),
        ("Catmull-Rom curve", Curve::CatmullRom { alpha: 0.5 }),
        ("Bundle curve", Curve::Bundle { beta: 0.25 }),
    ] {
        scenes.push((
            name,
            line(
                &c,
                &[Series::new("Control points", control_points.clone())],
                &LineOptions {
                    curve,
                    ..Default::default()
                },
            )?,
        ));
    }
    scenes.push((
        "Open spline comparison",
        compare_curves(
            &c,
            &control_points,
            &[
                ("Basis open", Curve::BasisOpen),
                ("Cardinal open", Curve::CardinalOpen),
                ("Catmull-Rom open", Curve::CatmullRomOpen { alpha: 0.5 }),
            ],
            [[0., 7.], [0., 55.]],
        )?,
    ));
    scenes.push((
        "Cardinal tension comparison",
        compare_curves(
            &c,
            &control_points,
            &[
                ("Tension 0", Curve::cardinal(0.)),
                ("Tension 0.6", Curve::cardinal(0.6)),
                ("Tension 1", Curve::cardinal(1.)),
            ],
            [[0., 7.], [0., 55.]],
        )?,
    ));
    let loop_points = [
        (1., 3.),
        (2.5, 7.),
        (6., 8.),
        (8., 5.),
        (7., 1.),
        (4., 2.),
        (1.5, 1.),
    ]
    .into_iter()
    .map(|(x, y)| Point::new(x, y))
    .collect::<Vec<_>>();
    scenes.push((
        "Periodic closed splines",
        compare_curves(
            &c,
            &loop_points,
            &[
                ("Basis closed", Curve::BasisClosed),
                ("Cardinal closed", Curve::CardinalClosed),
                ("Catmull-Rom closed", Curve::CatmullRomClosed { alpha: 0.5 }),
            ],
            [[0., 10.], [0., 10.]],
        )?,
    ));
    let long_loop = (0..700)
        .map(|i| {
            let a = i as f64 * std::f64::consts::TAU / 700.;
            let r = 1. + 0.14 * (5. * a).sin();
            Point::new(5. + 3.5 * r * a.cos(), 5. + 3.5 * r * a.sin())
        })
        .collect();
    let mut long_area = line(
        &c,
        &[Series::new("Continuous contour", long_loop)],
        &LineOptions {
            curve: Curve::CatmullRomClosed { alpha: 0.5 },
            area: true,
            points: false,
            x_domain: Some([0., 10.]),
            y_domain: Some([0., 10.]),
            ..Default::default()
        },
    )?;
    assert_eq!(
        long_area
            .marks
            .iter()
            .filter(|m| m.id.starts_with("area-"))
            .count(),
        1
    );
    assert!(
        long_area
            .marks
            .iter()
            .any(|m| m.id.starts_with("area-") && m.geometry.commands().len() > 240)
    );
    long_area.labels.push(Label {
        x: c.margin.left,
        y: 20.,
        text: "700 samples · one continuous closed contour".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Long closed area", long_area));
    scenes.push((
        "Bump X and Y",
        compare_curves(
            &c,
            &control_points,
            &[
                ("Bump X · horizontal tangents", Curve::BumpX),
                ("Bump Y · vertical tangents", Curve::BumpY),
            ],
            [[0., 7.], [0., 55.]],
        )?,
    ));
    let mut monotone_y = line(
        &c,
        &[Series::new(
            "Vertical progression",
            vec![
                Point::new(30., 0.),
                Point::new(70., 15.),
                Point::new(20., 35.),
                Point::new(80., 60.),
                Point::new(45., 80.),
                Point::new(60., 100.),
            ],
        )],
        &LineOptions {
            curve: Curve::MonotoneY,
            x_domain: Some([0., 100.]),
            y_domain: Some([0., 100.]),
            ..Default::default()
        },
    )?;
    monotone_y.labels.push(Label {
        x: c.margin.left,
        y: 20.,
        text: "Monotonic in Y; X can change direction".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Monotone Y curve", monotone_y));
    let axis_frame = ChartConfig {
        margin: Margin {
            top: 64.,
            right: 96.,
            bottom: 48.,
            left: 64.,
        },
        ..c.clone()
    };
    scenes.push((
        "Top and right axes",
        line(
            &axis_frame,
            &points,
            &LineOptions {
                axes: CartesianAxes {
                    x_side: AxisSide::Top,
                    y_side: AxisSide::Right,
                    x: AxisOptions {
                        legend: Some("Month index".into()),
                        tick_count: 5,
                        ..Default::default()
                    },
                    y: AxisOptions {
                        grid: true,
                        format: ValueFormat::Fixed(0),
                        ..Default::default()
                    },
                },
                ..Default::default()
            },
        )?,
    ));
    let rotated_frame = ChartConfig {
        margin: Margin {
            bottom: 100.,
            ..c.margin
        },
        ..c.clone()
    };
    scenes.push((
        "Rotated category labels",
        bar(
            &rotated_frame,
            &[
                "January".into(),
                "February".into(),
                "March".into(),
                "April".into(),
                "May".into(),
                "June".into(),
            ],
            &[BarSeries::new(
                "Revenue",
                vec![
                    Some(30.),
                    Some(42.),
                    Some(35.),
                    Some(58.),
                    Some(47.),
                    Some(64.),
                ],
            )],
            &BarOptions {
                category_axis: CategoryAxisOptions {
                    tick_rotation: 40.,
                    ..Default::default()
                },
                ..Default::default()
            },
        )?,
    ));
    let euro = kartis::format::FormatLocale {
        decimal: ",".into(),
        thousands: ".".into(),
        currency: (String::new(), " €".into()),
        minus: "-".into(),
        ..Default::default()
    };
    let locale_frame = ChartConfig {
        margin: Margin {
            left: 116.,
            right: 24.,
            top: 32.,
            bottom: 52.,
        },
        ..c.clone()
    };
    scenes.push((
        "Locale number format",
        line(
            &locale_frame,
            &[Series::new(
                "Revenue EUR",
                [8400.50, 9600., 9300.25, 12750.80, 12100.50, 14720.75]
                    .into_iter()
                    .enumerate()
                    .map(|(i, y)| Point::new((i + 1) as f64, y))
                    .collect(),
            )],
            &LineOptions {
                curve: Curve::Monotone,
                axes: CartesianAxes {
                    x: AxisOptions {
                        format: ValueFormat::Affix {
                            prefix: "M".into(),
                            suffix: String::new(),
                            precision: 0,
                        },
                        tick_values: Some((1..=6).map(|i| i as f64).collect()),
                        ..Default::default()
                    },
                    y: AxisOptions {
                        format: ValueFormat::specifier("$,.2f", &euro)?,
                        grid: true,
                        tick_count: 4,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )?,
    ));
    let dual_frame = ChartConfig {
        margin: Margin {
            left: 88.,
            right: 104.,
            top: 32.,
            bottom: 56.,
        },
        ..c.clone()
    };
    scenes.push((
        "Dual value axes",
        dual_axis(
            &dual_frame,
            &[
                DualAxisSeries::new(
                    Series::new(
                        "Temperature",
                        [18., 23., 21., 28., 25., 30.]
                            .into_iter()
                            .enumerate()
                            .map(|(i, y)| Point::new((i + 1) as f64, y))
                            .collect(),
                    ),
                    YAxisSide::Left,
                ),
                DualAxisSeries {
                    series: Series::new(
                        "Revenue",
                        [3000., 8500., 6500., 14000., 11000., 18000.]
                            .into_iter()
                            .enumerate()
                            .map(|(i, y)| Point::new((i + 1) as f64, y))
                            .collect(),
                    ),
                    axis: YAxisSide::Right,
                    area: true,
                },
            ],
            &DualAxisOptions {
                x_domain: Some([1., 6.]),
                x_axis: AxisOptions {
                    legend: Some("Month".into()),
                    format: ValueFormat::Fixed(0),
                    tick_values: Some((1..=6).map(|i| i as f64).collect()),
                    ..Default::default()
                },
                left: YAxisOptions {
                    domain: Some([0., 40.]),
                    axis: AxisOptions {
                        format: ValueFormat::Affix {
                            prefix: String::new(),
                            suffix: " °C".into(),
                            precision: 0,
                        },
                        grid: true,
                        tick_values: Some(vec![0., 10., 20., 30., 40.]),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                right: YAxisOptions {
                    domain: Some([0., 20000.]),
                    axis: AxisOptions {
                        format: ValueFormat::specifier(
                            "$.2s",
                            &kartis::format::FormatLocale::default(),
                        )?,
                        tick_values: Some(vec![0., 5000., 10000., 15000., 20000.]),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                curve: Curve::Monotone,
                ..Default::default()
            },
        )?,
    ));
    let combo_months: Vec<String> = ["Jan", "Feb", "Mar", "Apr"]
        .into_iter()
        .map(String::from)
        .collect();
    let combo_data = |values: &[f64]| {
        combo_months
            .iter()
            .zip(values)
            .map(|(month, value)| ComboPoint::new(month.clone(), *value))
            .collect()
    };
    let mut temperature = ComboSeries::new(
        "Temperature",
        ComboKind::Line,
        combo_data(&[14., 19., 22., 18.]),
    );
    temperature.axis = YAxisSide::Right;
    scenes.push((
        "Mixed Cartesian chart",
        combo(
            &axis_frame,
            &[
                ComboSeries::new("Budget", ComboKind::Area, combo_data(&[60., 70., 50., 80.])),
                ComboSeries::new("Revenue", ComboKind::Bar, combo_data(&[44., 60., 48., 64.])),
                temperature,
                ComboSeries::new(
                    "Forecast",
                    ComboKind::Scatter,
                    combo_data(&[51., 66., 55., 70.]),
                ),
            ],
            &ComboOptions {
                x: ComboXAxis::Categories {
                    values: combo_months.clone(),
                    padding: 0.25,
                },
                left: YAxisOptions {
                    domain: Some([0., 100.]),
                    axis: AxisOptions {
                        grid: true,
                        format: ValueFormat::Affix {
                            prefix: "$".into(),
                            suffix: "k".into(),
                            precision: 0,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                right: YAxisOptions {
                    domain: Some([0., 30.]),
                    axis: AxisOptions {
                        format: ValueFormat::Affix {
                            prefix: String::new(),
                            suffix: " °C".into(),
                            precision: 0,
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
                curve: Curve::Monotone,
                line_points: false,
                series_order: vec![
                    "Budget".into(),
                    "Revenue".into(),
                    "Temperature".into(),
                    "Forecast".into(),
                ],
                ..Default::default()
            },
        )?,
    ));
    scenes.push((
        "Logarithmic axes",
        line(
            &c,
            &[Series::new(
                "Growth",
                vec![
                    Point::new(1., 1.),
                    Point::new(10., 10.),
                    Point::new(100., 1000.),
                    Point::new(1000., 10000.),
                ],
            )],
            &LineOptions {
                x_scale: ScaleKind::Log,
                y_scale: ScaleKind::Log,
                axes: CartesianAxes {
                    x: AxisOptions {
                        tick_values: Some(vec![1., 10., 100., 1000.]),
                        ..Default::default()
                    },
                    y: AxisOptions {
                        format: ValueFormat::Compact(1),
                        grid: true,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )?,
    ));
    scenes.push((
        "UTC time axis",
        line(
            &c,
            &[Series::new(
                "Daily",
                (0..6)
                    .map(|i| {
                        Point::new(
                            1708992000000. + i as f64 * 86400000.,
                            10. + (i as f64 * 0.9).sin() * 5.,
                        )
                    })
                    .collect(),
            )],
            &LineOptions {
                x_scale: ScaleKind::Utc,
                axes: CartesianAxes {
                    x: AxisOptions {
                        tick_count: 3,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
        )?,
    ));
    scenes.push((
        "Clipped domains",
        line(
            &c,
            &[Series::new(
                "Clipped",
                vec![Point::new(-2., 0.7), Point::new(3., 0.3)],
            )],
            &LineOptions {
                x_domain: Some([0., 1.]),
                y_domain: Some([0., 1.]),
                ..Default::default()
            },
        )?,
    ));
    let date_frame = ChartConfig {
        margin: Margin {
            left: 64.,
            right: 64.,
            top: 48.,
            bottom: 64.,
        },
        ..c.clone()
    };
    let day = 86_400_000.;
    let february_28 = 1_709_078_400_000.;
    let chinese_ticks = (0..4)
        .map(|i| february_28 + i as f64 * day)
        .collect::<Vec<_>>();
    let mut chinese_dates = line(
        &date_frame,
        &[Series::new(
            "Daily volume",
            chinese_ticks
                .iter()
                .zip([24., 38., 31., 46.])
                .map(|(x, y)| Point::new(*x, y))
                .collect(),
        )],
        &LineOptions {
            x_scale: ScaleKind::Utc,
            x_domain: Some([chinese_ticks[0], chinese_ticks[3]]),
            y_domain: Some([0., 60.]),
            axes: CartesianAxes {
                x: AxisOptions {
                    tick_values: Some(chinese_ticks),
                    format: ValueFormat::DateTime(Box::new(DateTimeFormat::utc(
                        "%-m月%-d日 %a",
                        &TimeLocale::zh_cn(),
                    )?)),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    chinese_dates.labels.push(Label {
        x: 64.,
        y: 22.,
        text: "中文日期 · 2024 闰年 · UTC".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Chinese date labels", chinese_dates));
    let clock_start = 1_709_208_000_000.; // 2024-02-29 12:00 UTC / 20:00 UTC+08:00.
    let clock_ticks = (0..5)
        .map(|i| clock_start + i as f64 * 10_800_000.)
        .collect::<Vec<_>>();
    let clock_format = DateTimeFormat::new(
        "%m-%d %H:%M",
        &TimeLocale::zh_cn(),
        TimeZone::fixed_offset(480)?,
    )?;
    let mut offset_clock = line(
        &date_frame,
        &[Series::new(
            "Evening traffic",
            clock_ticks
                .iter()
                .zip([32., 55., 27., 18., 41.])
                .map(|(x, y)| Point::new(*x, y))
                .collect(),
        )],
        &LineOptions {
            x_scale: ScaleKind::Utc,
            x_domain: Some([clock_ticks[0], clock_ticks[4]]),
            y_domain: Some([0., 60.]),
            axes: CartesianAxes {
                x: AxisOptions {
                    tick_values: Some(clock_ticks),
                    format: ValueFormat::DateTime(Box::new(clock_format)),
                    ..Default::default()
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    offset_clock.labels.push(Label {
        x: 64.,
        y: 22.,
        text: "Fixed UTC+08:00 · same instants, explicit local clock".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Fixed-offset clock", offset_clock));
    for (name, start, end, unit, step) in [
        (
            "Local calendar day ticks",
            1_709_049_600_000.,
            1_709_568_000_000.,
            TimeUnit::Day,
            2,
        ),
        (
            "Local calendar month ticks",
            1_705_248_000_000.,
            1_715_702_400_000.,
            TimeUnit::Month,
            1,
        ),
    ] {
        let mut calendar = TimeCalendar::new(TimeZone::fixed_offset(480)?);
        calendar.interval = Some(TimeInterval::new(unit, step)?);
        let data = Series::new(
            "Activity",
            (0..=12)
                .map(|i| {
                    Point::new(
                        start + (end - start) * f64::from(i) / 12.,
                        30. + (f64::from(i) * 0.8).sin() * 15.,
                    )
                })
                .collect(),
        );
        scenes.push((
            name,
            line(
                &date_frame,
                &[data],
                &LineOptions {
                    x_scale: ScaleKind::Utc,
                    x_domain: Some([start, end]),
                    y_domain: Some([0., 60.]),
                    axes: CartesianAxes {
                        x: AxisOptions {
                            tick_calendar: Some(calendar),
                            format: ValueFormat::date_time(
                                "%m-%d %H:%M",
                                &TimeLocale::default(),
                                TimeZone::fixed_offset(480)?,
                            )?,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?,
        ));
    }
    let viewport_plot = ClipRect::plot(&c);
    let mut domain_view = DomainViewport::new(
        ViewportAxes {
            x: ViewportAxis::new([0., 100.], ScaleKind::Linear),
            left: Some(ViewportAxis::new([0., 100.], ScaleKind::Linear)),
            right: None,
        },
        viewport_plot,
        ViewportLimits::default(),
    )?;
    let viewport_data = [Series::new(
        "Measured",
        [20., 35., 60., 45., 70., 40., 55., 30., 50., 65., 25.]
            .into_iter()
            .enumerate()
            .map(|(i, y)| Point::new(i as f64 * 10., y))
            .collect(),
    )];
    let render_domains = |domains: ResolvedDomains, caption: &str| -> Result<Scene> {
        let mut scene = line(
            &c,
            &viewport_data,
            &LineOptions {
                x_domain: Some(domains.x),
                y_domain: domains.left,
                ..Default::default()
            },
        )?;
        scene.labels.push(Label {
            x: c.margin.left,
            y: 20.,
            text: caption.into(),
            color: c.theme.text,
            size: 12.,
            anchor: TextAnchor::Start,
            rotation: 0.,
        });
        Ok(scene)
    };
    let original_domains = domain_view.domains();
    let zoomed_domains = domain_view.apply(ViewportRequest::Zoom {
        factor: 2.,
        anchor: (viewport_plot.width / 2., viewport_plot.height / 2.),
        axes: ViewportSelection::X,
    })?;
    scenes.push((
        "Viewport original domains",
        render_domains(
            original_domains,
            "X: 0–100 · Y: 0–100 · fixed plot and point radius",
        )?,
    ));
    scenes.push((
        "Viewport relayout domains",
        render_domains(
            zoomed_domains,
            "X: 25–75 · Y: 0–100 · data relaid out inside the same axes",
        )?,
    ));
    scenes.push((
        "Wiggle stream",
        stream(
            &c,
            &categories,
            &positive,
            &StreamOptions {
                offset: StreamOffset::Wiggle,
                order: kartis::stack::StackOrder::InsideOut,
                ..Default::default()
            },
        )?,
    ));
    scenes.push((
        "Squarify",
        treemap_with_tile(
            &c,
            &hierarchy,
            &HierarchyOptions::default(),
            TreemapTile::Squarify,
        )?,
    ));
    scenes.push((
        "Tidy radial tree",
        tree_with_options(
            &c,
            &hierarchy,
            &TreeOptions {
                orientation: TreeOrientation::Radial,
                ..Default::default()
            },
        )?,
    ));
    let mut voronoi = scatter(&c, &points, &ScatterOptions::default())?;
    let mesh = kartis::mesh::PointMesh::from_scene(&voronoi)?;
    let mut cells = vec![];
    for cell in mesh.voronoi(ClipRect::plot(&c))? {
        if cell.polygon.is_empty() {
            continue;
        }
        let mut path = vec![];
        for (i, (x, y)) in cell.polygon.into_iter().enumerate() {
            let p = glory::render::GraphicPoint::new(x, y);
            path.push(if i == 0 {
                glory::render::PathCommand::Move(p)
            } else {
                glory::render::PathCommand::Line(p)
            });
        }
        path.push(glory::render::PathCommand::Close);
        let mut mark = Mark::new(
            format!("mesh:{}", cell.id),
            Geometry::Path(path),
            Color(0, 0, 0, 0),
        );
        mark.fill = None;
        mark.label.clear();
        mark.stroke = Some(glory::render::GraphicStroke {
            color: c.theme.grid,
            width: 1.,
            cap: glory::render::StrokeCap::Butt,
        });
        mark.clip = Some(ClipRect::plot(&c));
        cells.push(mark);
    }
    cells.append(&mut voronoi.marks);
    voronoi.marks = cells;
    scenes.push(("Voronoi mesh", voronoi));
    let mut layered = c.scene()?;
    for (i, (id, x, y, width, height)) in [
        ("Blue", 90., 70., 420., 220.),
        ("Orange", 350., 150., 260., 180.),
    ]
    .iter()
    .enumerate()
    {
        let mut mark = Mark::new(
            *id,
            Geometry::Rect {
                x: *x,
                y: *y,
                width: *width,
                height: *height,
            },
            c.theme.color(i),
        );
        mark.series = (*id).into();
        layered.marks.push(mark);
        layered.legend.push(LegendItem {
            id: (*id).into(),
            label: (*id).into(),
            color: c.theme.color(i),
        });
    }
    layered.labels.push(Label {
        rotation: 0.,
        x: 130.,
        y: 190.,
        text: "This text is behind the orange block".into(),
        color: Color(255, 255, 255, 255),
        size: 22.,
        anchor: TextAnchor::Start,
    });
    layered.labels.push(Label {
        rotation: 0.,
        x: 480.,
        y: 255.,
        text: "Front label".into(),
        color: Color(255, 255, 255, 255),
        size: 22.,
        anchor: TextAnchor::Middle,
    });
    layered.label_series.insert(0, "Blue".into());
    layered.label_series.insert(1, "Orange".into());
    layered.draw_order = vec![
        DrawItem::Mark(0),
        DrawItem::Label(0),
        DrawItem::Mark(1),
        DrawItem::Label(1),
    ];
    scenes.push(("Custom layers", layered));
    let map_features = vec![
        GeoFeature {
            id: "West".into(),
            label: "West".into(),
            value: Some(32.),
            polygons: vec![vec![vec![
                [-70., -20.],
                [-15., -20.],
                [-15., 50.],
                [-70., 50.],
                [-70., -20.],
            ]]],
        },
        GeoFeature {
            id: "East".into(),
            label: "East".into(),
            value: Some(77.),
            polygons: vec![vec![vec![
                [20., -30.],
                [65., -30.],
                [65., 45.],
                [20., 45.],
                [20., -30.],
            ]]],
        },
    ];
    let symbol_frame = ChartConfig {
        margin: Margin {
            left: 32.,
            right: 232.,
            top: 46.,
            bottom: 32.,
        },
        ..c.clone()
    };
    let mut symbol_map = geographic_map_layout(
        &symbol_frame,
        &map_features,
        &GeoMapOptions {
            projection: GeoProjection::EqualEarth,
            fit: GeoFit::Data,
            padding: 14.,
            fill: Some(Color(232, 239, 244, 255)),
            border: Some(Color(177, 195, 208, 255)),
            ..Default::default()
        },
    )?;
    let fitted_context = symbol_map.projection.clone();
    let symbol_data = [
        ("Harbor", [-50., 0.], Some(25.)),
        ("Hill", [-40., 25.], Some(64.)),
        ("North", [-25., 40.], Some(100.)),
        ("South", [28., -10.], Some(16.)),
        ("Central", [40., 18.], Some(49.)),
        ("East", [55., 35.], Some(81.)),
        ("No report", [-55., 42.], None),
    ]
    .into_iter()
    .map(|(id, position, value)| GeoSymbol::new(id, position, value))
    .collect::<Vec<_>>();
    symbol_map.add_symbols(
        &symbol_data,
        &GeoSymbolOptions {
            domain: Some([0., 100.]),
            radius_range: [0., 18.],
            labels: true,
            label_size: 11.,
            value_format: ValueFormat::Affix {
                prefix: String::new(),
                suffix: "k".into(),
                precision: 0,
            },
            color: GeoSymbolColor::Value {
                domain: Some([0., 100.]),
                colors: vec![
                    Color(117, 111, 187, 255),
                    Color(30, 157, 151, 255),
                    Color(230, 142, 67, 255),
                ],
            },
            missing: GeoMissingSymbol::Draw {
                radius: 4.,
                color: Color(148, 157, 167, 255),
                label: "No report".into(),
            },
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [510., 48.],
                    title: "Volume · circle area (k)".into(),
                    ..Default::default()
                },
                values: vec![25., 64., 100.],
            }),
            color_legend: Some(GeoLegendOptions {
                position: [510., 216.],
                title: "Volume · color bands (k)".into(),
                format: ValueFormat::Fixed(0),
                ..Default::default()
            }),
            ..Default::default()
        },
    )?;
    assert_eq!(
        symbol_map.projection, fitted_context,
        "symbols reuse the exact Data fit"
    );
    symbol_map.scene.labels.push(Label {
        x: 32.,
        y: 20.,
        text: "One fitted map · area and color encode the same magnitude".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Geographic symbol legends", symbol_map.scene));
    let mut globe = geographic_map_layout(
        &symbol_frame,
        &map_features,
        &GeoMapOptions {
            projection: GeoProjection::Orthographic,
            fill: Some(Color(217, 231, 221, 255)),
            border: Some(Color(161, 191, 170, 255)),
            ..Default::default()
        },
    )?;
    let [globe_x, globe_y] = globe.projection.translate();
    let mut sphere = Mark::new(
        "globe-silhouette",
        Geometry::Circle {
            x: globe_x,
            y: globe_y,
            radius: globe.projection.scale(),
        },
        Color(238, 245, 251, 255),
    );
    sphere.label.clear();
    sphere.stroke = Some(glory::render::GraphicStroke {
        color: Color(179, 199, 214, 255),
        width: 1.,
        cap: glory::render::StrokeCap::Butt,
    });
    globe.scene.marks.insert(0, sphere);
    let globe_data = [
        ("West port", [-35., 15.], 64., "Western ports"),
        ("Equator", [0., 0.], 25., "Western ports"),
        ("East port", [45., 10.], 81., "Eastern ports"),
        ("South port", [-10., -35.], 49., "Eastern ports"),
        ("Back face", [150., 10.], 100., "Eastern ports"),
    ]
    .into_iter()
    .map(|(id, position, value, series)| {
        let mut symbol = GeoSymbol::new(id, position, Some(value));
        symbol.series = series.into();
        symbol
    })
    .collect::<Vec<_>>();
    assert_eq!(globe.projection.project([150., 10.])?, None);
    globe.add_symbols(
        &globe_data,
        &GeoSymbolOptions {
            domain: Some([0., 100.]),
            radius_range: [0., 18.],
            labels: true,
            label_size: 11.,
            color: GeoSymbolColor::Series(vec![Color(78, 121, 167, 255), Color(230, 142, 67, 255)]),
            size_legend: Some(GeoSizeLegend {
                layout: GeoLegendOptions {
                    position: [510., 48.],
                    title: "Traffic · circle area".into(),
                    ..Default::default()
                },
                values: vec![25., 64., 100.],
            }),
            color_legend: Some(GeoLegendOptions {
                position: [510., 235.],
                title: "Port region".into(),
                ..Default::default()
            }),
            ..Default::default()
        },
    )?;
    assert!(
        !globe
            .scene
            .marks
            .iter()
            .any(|m| m.id.ends_with(":Back face"))
    );
    globe.scene.labels.push(Label {
        x: 32.,
        y: 20.,
        text: "Orthographic view · four visible ports, one behind the globe".into(),
        color: c.theme.text,
        size: 12.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    scenes.push(("Globe symbol legends", globe.scene));
    for (name, projection) in [
        ("Equal Earth map", GeoProjection::EqualEarth),
        ("Natural Earth map", GeoProjection::NaturalEarth),
        ("Orthographic map", GeoProjection::Orthographic),
    ] {
        scenes.push((
            name,
            choropleth_with_options(
                &c,
                &map_features,
                &GeoMapOptions {
                    projection,
                    ..Default::default()
                },
            )?,
        ));
    }
    let seam = vec![GeoFeature {
        id: "Seam".into(),
        label: "Antimeridian".into(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [165., -30.],
            [-165., -30.],
            [-165., 30.],
            [165., 30.],
            [165., -30.],
        ]]],
    }];
    scenes.push((
        "Antimeridian map",
        geographic_map(&c, &seam, &GeoMapOptions::default())?,
    ));
    let days: Vec<_> = (2024..=2026)
        .flat_map(|year| {
            (1..=12).flat_map(move |month| {
                (1..=28).map(move |day| {
                    CalendarDate::new(
                        year,
                        month,
                        day,
                        ((year + month as i32 + day as i32) % 31) as f64,
                    )
                })
            })
        })
        .collect();
    scenes.push((
        "Multi-year calendar",
        calendar_range(&c, &days, &CalendarRangeOptions::default())?,
    ));
    let force_nodes: Vec<_> = (0..30).map(|i| NetworkNode::new(format!("N{i}"))).collect();
    let force_links: Vec<_> = (1..30)
        .map(|i| NetworkLink::new(format!("N{}", (i - 1) / 2), format!("N{i}"), 1.))
        .collect();
    let mut simulation = ForceSimulation::new(&force_nodes, &force_links, ForceOptions::default())?;
    simulation.step(160)?;
    scenes.push(("Barnes-Hut network", simulation.scene(&c)?));
    let heat: Vec<_> = (0..5)
        .flat_map(|r| {
            (0..8).map(move |col| {
                HeatCell::new(
                    format!("Row {}", r + 1),
                    format!("C{}", col + 1),
                    ((r * 7 + col * 3) % 23) as f64,
                )
            })
        })
        .collect();
    scenes.push((
        "Heatmap",
        heatmap(
            &c,
            &heat,
            &HeatmapOptions {
                labels: true,
                ..Default::default()
            },
        )?,
    ));
    let mut resquarify_data = vec![HierarchyDatum::new("Portfolio", None, 0.)];
    for (i, value) in [24., 20., 16., 14., 12., 10., 8., 6., 5., 4., 3., 2.]
        .into_iter()
        .enumerate()
    {
        resquarify_data.push(HierarchyDatum::new(
            format!("Asset {}", i + 1),
            Some("Portfolio".into()),
            value,
        ));
    }
    let mut resquarifier = Resquarify::new(1.6)?;
    scenes.push((
        "Resquarify initial",
        resquarifier.layout(&c, &resquarify_data, &HierarchyOptions::default())?,
    ));
    for (i, datum) in resquarify_data.iter_mut().skip(1).enumerate() {
        datum.value *= if i % 2 == 0 { 0.55 } else { 1.75 };
    }
    scenes.push((
        "Resquarify updated",
        resquarifier.layout(&c, &resquarify_data, &HierarchyOptions::default())?,
    ));
    let ranks = vec![
        Series::new(
            "North",
            vec![
                Point::new(0., 1.),
                Point::new(1., 2.),
                Point::new(2., 1.),
                Point::new(3., 3.),
            ],
        ),
        Series::new(
            "South",
            vec![
                Point::new(0., 2.),
                Point::new(1., 1.),
                Point::new(2., 3.),
                Point::new(3., 2.),
            ],
        ),
        Series::new(
            "East",
            vec![
                Point::new(0., 3.),
                Point::new(1., 3.),
                Point::new(2., 2.),
                Point::new(3., 1.),
            ],
        ),
    ];
    scenes.push(("Bump", bump(&c, &ranks, &LineOptions::default())?));
    let nodes: Vec<_> = ["Source", "Process A", "Process B", "Output"]
        .into_iter()
        .map(FlowNode::new)
        .collect();
    let links = vec![
        FlowLink {
            source: "Source".into(),
            target: "Process A".into(),
            value: 60.,
        },
        FlowLink {
            source: "Source".into(),
            target: "Process B".into(),
            value: 40.,
        },
        FlowLink {
            source: "Process A".into(),
            target: "Output".into(),
            value: 60.,
        },
        FlowLink {
            source: "Process B".into(),
            target: "Output".into(),
            value: 40.,
        },
    ];
    scenes.push((
        "Sankey",
        sankey(&c, &nodes, &links, &SankeyOptions::default())?,
    ));
    // Unequal path lengths expose every alignment policy: the late source can
    // move toward Stage C, early outputs can move to the final column, and the
    // short stage distinguishes right alignment from centered source alignment.
    let alignment_nodes = [
        "Supply",
        "Stage A",
        "Stage B",
        "Stage C",
        "Main output",
        "Late source",
        "Early output",
        "Short stage",
        "Short output",
    ]
    .into_iter()
    .map(FlowNode::new)
    .collect::<Vec<_>>();
    let alignment_links = [
        ("Supply", "Stage A", 80.),
        ("Stage A", "Stage B", 60.),
        ("Stage B", "Stage C", 60.),
        ("Stage C", "Main output", 90.),
        ("Late source", "Stage C", 30.),
        ("Stage A", "Early output", 20.),
        ("Supply", "Short stage", 20.),
        ("Short stage", "Short output", 20.),
    ]
    .into_iter()
    .map(|(source, target, value)| FlowLink {
        source: source.into(),
        target: target.into(),
        value,
    })
    .collect::<Vec<_>>();
    let alignment_frame = ChartConfig {
        margin: Margin {
            left: 96.,
            right: 104.,
            top: 32.,
            bottom: 40.,
        },
        ..c.clone()
    };
    for (name, alignment) in [
        ("Sankey left alignment", SankeyAlignment::Left),
        ("Sankey right alignment", SankeyAlignment::Right),
        ("Sankey center alignment", SankeyAlignment::Center),
        ("Sankey justify alignment", SankeyAlignment::Justify),
    ] {
        scenes.push((
            name,
            sankey_with_layout(
                &alignment_frame,
                &alignment_nodes,
                &alignment_links,
                &SankeyLayoutOptions {
                    alignment,
                    appearance: SankeyOptions {
                        node_width: 18.,
                        node_padding: 18.,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )?,
        ));
    }
    let nodes: Vec<_> = (0..14).map(|i| NetworkNode::new(format!("N{i}"))).collect();
    let links: Vec<_> = (1..14)
        .map(|i| NetworkLink::new(format!("N{}", (i - 1) / 2), format!("N{i}"), 2.))
        .collect();
    scenes.push((
        "Network",
        network(&c, &nodes, &links, &NetworkOptions::default())?,
    ));
    let land = vec![
        GeoFeature {
            id: "West".into(),
            label: "West".into(),
            value: Some(32.),
            polygons: vec![vec![vec![
                [-125., 15.],
                [-65., 15.],
                [-65., 60.],
                [-125., 60.],
            ]]],
        },
        GeoFeature {
            id: "East".into(),
            label: "East".into(),
            value: Some(77.),
            polygons: vec![vec![vec![
                [10., -20.],
                [110., -20.],
                [110., 55.],
                [10., 55.],
            ]]],
        },
    ];
    scenes.push(("Choropleth", choropleth(&c, &land, Projection::Mercator)?));
    scenes.extend(upgrades::scenes(&c)?);
    scenes.extend(styles::scenes(&c)?);
    scenes.extend(configurable::scenes(&c)?);
    scenes.extend(presentation::scenes(&c)?);
    scenes.extend(data_styles::scenes(&c)?);
    scenes.extend(discrete_colors::scenes(&c)?);
    scenes.extend(power_scales::scenes(&c)?);
    scenes.extend(advanced_numeric::scenes(&c)?);
    scenes.extend(p1::scenes(&c)?);
    scenes.extend(p2::scenes(&c)?);
    scenes.extend(interactions::scenes(&c)?);
    scenes.extend(scale_glyphs::scenes(&c)?);
    scenes.extend(geo_glyphs::scenes(&c)?);
    scenes.extend(scatter::scenes(&c)?);
    scenes.extend(extended_scales::scenes(&c)?);
    scenes.push(("Scatter · custom datum badges", scatter_custom::scene(&c)?));
    let (blend_scene, blends) = blend_demo::scene(&c)?;
    scenes.push((blend_demo::NAME, blend_scene));
    scenes.extend(color_interpolation::scenes(&c)?);
    scenes.extend(radar_scales::scenes(&c)?);
    scenes.extend(configured_paint::scenes(&c)?);
    scenes.extend(boxplot_profiles::scenes(&c)?);
    scenes.push(line_custom::scene(&c)?);
    scenes.extend(radial_focus::scenes(&c)?);
    scenes.extend(layout_models::scenes(&c)?);
    scenes.extend(flow_layouts::scenes(&c)?);
    scenes.extend(radial_label_gate::scenes(&c)?);
    #[cfg(feature = "raster")]
    scenes.extend(measured_sunburst::scenes(&c)?);
    #[cfg(feature = "raster")]
    scenes.extend(color_fonts::scenes(&c)?);
    #[cfg(feature = "raster")]
    scenes.extend(typography::scenes(&c)?);
    scenes.extend(contour_density::scenes(&c)?);
    scenes.extend(sequential_quantile::scenes(&c)?);
    scenes.extend(conic_model::gallery_scenes(&c)?);
    scenes.extend(spherical_model::gallery_scenes(&c)?);
    let mut html = String::from(
        r#"<!doctype html><html lang="en"><meta charset="utf-8"><link rel="icon" href="data:,"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Kartis — Rust charts for Glory</title><style>
*{box-sizing:border-box}body{margin:0;background:#eef2f6;color:#233247;font:15px system-ui,sans-serif}header{padding:40px max(24px,calc((100vw - 1540px)/2));background:#142639;color:white}h1{margin:8px 0;font-size:42px;letter-spacing:-1.5px}header p{color:#bad0e2;max-width:760px;line-height:1.6}small{letter-spacing:2px;text-transform:uppercase;color:#66d6ba}main{max-width:1600px;margin:auto;padding:24px;display:grid;grid-template-columns:repeat(2,minmax(0,1fr));gap:22px}article{background:white;border:1px solid #dce4ec;border-radius:12px;overflow:hidden;box-shadow:0 3px 12px #20304005}article>div.heading{display:flex;align-items:center;justify-content:space-between;padding:15px 20px;border-bottom:1px solid #edf0f4}h2{font-size:17px;margin:0}button,select{font:inherit;padding:6px 10px;border:1px solid #ced9e2;border-radius:6px;background:white;color:#324e68;cursor:pointer}button:focus-visible{outline:3px solid #468fcc}.plot{position:relative}.svg-view>svg,.canvas-view>canvas{display:block;width:100%!important;height:auto!important;aspect-ratio:720/400}.status{font-size:12px;color:#516b80;min-height:32px;padding:6px 20px;background:#f8fafc}.canvas-view{display:none}article.canvas-mode .svg-view{display:none}article.canvas-mode .canvas-view{display:block}.controls{display:flex;gap:8px}footer{padding:25px;text-align:center;color:#516b80}@media(max-width:950px){main{grid-template-columns:1fr}h1{font-size:34px}}
</style><header><small>Kartis / Glory</small><h1>One scene. Two renderers.</h1><p>Deterministic Rust chart layouts rendered with SVG and Canvas. Switch each card to compare the same geometry. Point at a mark to inspect its value. No d3, React, or network dependencies.</p></header><main>"#,
    );
    let scene_count = scenes.len();
    for (i, (name, mut scene)) in scenes.into_iter().enumerate() {
        scene.title = name.into();
        let id = format!("canvas-{i}");
        // Standalone exports own their document. Inline cards share one DOM:
        // isolate generated resource IDs so toggling an earlier identical SVG
        // to display:none cannot retire a later card's gradient/pattern/clip.
        let namespace = format!("gallery-{i}");
        let (svg, inline_svg, script) = if name == blend_demo::NAME {
            (
                render::to_svg_with_blends(&scene, &blends)?,
                render::to_svg_with_namespace_and_blends(&scene, &namespace, &blends)?,
                render::to_canvas_script_with_blends(&scene, &id, &blends)?,
            )
        } else {
            (
                render::to_svg(&scene)?,
                render::to_svg_with_namespace(&scene, &namespace)?,
                render::to_canvas_script(&scene, &id)?,
            )
        };
        std::fs::write(output.join(format!("chart-{i:02}.svg")), &svg)?;
        write!(
            html,
            r#"<article data-index="{i}"><div class="heading"><h2>{name}</h2><div class="controls"><button class="toggle" aria-pressed="false">SVG → Canvas</button><a href="chart-{i:02}.svg" aria-label="Download {name} SVG">SVG ↗</a></div></div><div class="plot"><div class="svg-view">{inline_svg}</div><div class="canvas-view"><canvas id="{id}"></canvas></div></div><div class="status" aria-live="polite">{} marks · shared Rust scene</div></article><script>{script}</script>"#,
            scene.marks.len()
        )?;
    }
    html.push_str("</main><footer>Kartis 0.1 · Rust / SVG / Canvas · Layouts generated locally</footer><script>");
    html.push_str(include_str!("gallery/interaction.js"));
    html.push_str("</script></html>");
    std::fs::write(output.join("index.html"), html)?;
    println!(
        "Gallery: {} ({scene_count} scenes)",
        output.join("index.html").display()
    );
    Ok(())
}
