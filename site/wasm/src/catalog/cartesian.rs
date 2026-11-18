use glory::render::{GraphicPoint, PathCommand};
use serde_json::json;

use super::{phase, *};

const RANK_SERIES: [&str; 3] = ["North", "South", "West"];
const BAND_SERIES: [&str; 3] = ["Search", "Direct", "Partners"];
const BAND_LABEL_SIZE: f64 = 10.;
const BAND_LABEL_GAP: f64 = 4.;

pub(super) fn reserve_rank_labels(
    c: &mut ChartConfig,
    kind: &str,
    settings: &Settings,
    bump: BumpControls,
) {
    let (names, label_size, gap, start, end) = match kind {
        "bump" => (
            RANK_SERIES,
            c.theme.font_size,
            BumpEndpointLabels::default().padding,
            false,
            bump.end_labels,
        ),
        "area-bump" => (
            BAND_SERIES,
            BAND_LABEL_SIZE,
            BAND_LABEL_GAP,
            settings.get("start-labels") != "off",
            true,
        ),
        _ => return,
    };
    let scale = if settings.get("label-size") == "large" {
        1.25
    } else {
        1.
    };
    let gutter = 5.;
    // Tick labels sit outside the plot at both the top and the bottom.
    let tick_space = 16. + c.theme.font_size * scale + gutter;
    c.margin.top = c.margin.top.max(tick_space);
    c.margin.bottom = c.margin.bottom.max(tick_space);
    if settings.get("label-mode") != "hidden" {
        // The authored Latin names fit a 0.62 em average advance. Reserve it
        // only where an endpoint is enabled so narrow plots keep their columns.
        let longest = names.iter().map(|name| name.len()).max().unwrap_or(0);
        let space = (longest as f64 * 0.62 * label_size * scale + gap + gutter).ceil();
        if start {
            c.margin.left = c.margin.left.max(space);
        }
        if end {
            c.margin.right = c.margin.right.max(space);
        }
    }
}

// D3's basis curve has a short straight lead-in/out and one cubic per
// interval. Express each linear interval with the same command slots so
// the spring can keep the original curve control points even mid-retarget.
fn linear_rank_edge(points: &[GraphicPoint], start: PathCommand, out: &mut Vec<PathCommand>) {
    out.push(start);
    out.push(PathCommand::Line(points[0]));
    for pair in points.windows(2) {
        let [a, b] = [pair[0], pair[1]];
        let third = |from: GraphicPoint, to: GraphicPoint| {
            GraphicPoint::new(from.x + (to.x - from.x) / 3., from.y + (to.y - from.y) / 3.)
        };
        out.push(PathCommand::Cubic {
            first: third(a, b),
            second: third(b, a),
            end: b,
        });
    }
    out.push(PathCommand::Line(points[points.len() - 1]));
}

fn match_rank_curve_commands(mark: &mut Mark) {
    let Geometry::Path(commands) = &mut mark.geometry else {
        return;
    };
    let closed = commands.last() == Some(&PathCommand::Close);
    let vertices = if closed {
        &commands[..commands.len() - 1]
    } else {
        &commands[..]
    };
    let Some(points) = vertices
        .iter()
        .map(|cmd| match cmd {
            PathCommand::Move(p) | PathCommand::Line(p) => Some(*p),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    // An Area bump band has the same set of columns on both edges, with
    // the upper edge traversed in reverse. Keep the two caps straight.
    let edge_len = if closed {
        points.len() / 2
    } else {
        points.len()
    };
    if edge_len < 3 || (closed && !points.len().is_multiple_of(2)) {
        return;
    }
    commands.clear();
    linear_rank_edge(&points[..edge_len], PathCommand::Move(points[0]), commands);
    if closed {
        linear_rank_edge(
            &points[edge_len..],
            PathCommand::Line(points[edge_len]),
            commands,
        );
        commands.push(PathCommand::Close);
    }
}

pub(super) fn categories() -> Vec<String> {
    ["Jan", "Feb", "Mar", "Apr", "May", "Jun"]
        .map(str::to_owned)
        .to_vec()
}
pub(super) fn bars(seed: u32) -> Vec<BarSeries> {
    ["Studio", "Online", "Partners"]
        .into_iter()
        .enumerate()
        .map(|(j, id)| {
            BarSeries::new(
                id,
                (0..6)
                    .map(|i| {
                        Some(
                            20. + j as f64 * 4.
                                + (i as f64 * 0.9 + j as f64 + phase(seed)).sin() * 12.,
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}
fn lines(seed: u32) -> Vec<Series> {
    ["This year", "Last year"]
        .into_iter()
        .enumerate()
        .map(|(j, id)| {
            Series::new(
                id,
                (0..18)
                    .map(|i| {
                        let x = f64::from(i);
                        Point::new(
                            x,
                            24. + x * (1.6 - j as f64 * 0.5)
                                + (x * 0.65 + phase(seed) + j as f64).sin() * 8.
                                + j as f64 * 7.,
                        )
                    })
                    .collect(),
            )
        })
        .collect()
}
pub(super) fn sample(
    kind: &str,
    c: &ChartConfig,
    seed: u32,
    variant: &str,
    bump: BumpControls,
    settings: &Settings,
) -> Result<Sample> {
    let p = phase(seed);
    let (mut scene, data, code) = match kind {
        "line" | "area" => {
            let source = lines(seed);
            let data=json!(source.iter().map(|s|json!({"id":s.id,"data":s.data.iter().map(|p|json!({"x":p.x,"y":p.y})).collect::<Vec<_>>()})).collect::<Vec<_>>());
            let curve = match variant {
                "linear" => Curve::Linear,
                "step" => Curve::Step,
                "basis" => Curve::Basis,
                "natural" => Curve::Natural,
                _ => Curve::MonotoneX,
            };
            (
                line(
                    c,
                    &source,
                    &LineOptions {
                        curve,
                        area: kind == "area",
                        points: settings.get("points") != "off",
                        y_scale: cart_controls::scale(settings.get("y-scale")),
                        y_domain: Some([0., 75.]),
                        ..Default::default()
                    },
                )?,
                data,
                match (kind, variant) {
                    ("area", "basis") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Basis, area: true, ..Default::default() })?"
                    }
                    ("area", "natural") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Natural, area: true, ..Default::default() })?"
                    }
                    (_, "basis") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Basis, area: false, ..Default::default() })?"
                    }
                    (_, "natural") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Natural, area: false, ..Default::default() })?"
                    }
                    ("area", "linear") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Linear, area: true, ..Default::default() })?"
                    }
                    ("area", "step") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Step, area: true, ..Default::default() })?"
                    }
                    ("area", _) => {
                        "line(&config, &series, &LineOptions { curve: Curve::MonotoneX, area: true, ..Default::default() })?"
                    }
                    (_, "linear") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Linear, area: false, ..Default::default() })?"
                    }
                    (_, "step") => {
                        "line(&config, &series, &LineOptions { curve: Curve::Step, area: false, ..Default::default() })?"
                    }
                    _ => {
                        "line(&config, &series, &LineOptions { curve: Curve::MonotoneX, area: false, ..Default::default() })?"
                    }
                },
            )
        }
        "bar" | "stream" | "marimekko" => {
            let categories = categories();
            let source = bars(seed);
            let data = json!({"categories":categories,"series":source.iter().map(|s|json!({"id":s.id,"values":s.values})).collect::<Vec<_>>()});
            if kind == "bar" {
                (
                    bar(
                        c,
                        &categories,
                        &source,
                        &BarOptions {
                            corner_radius: if settings.get("corners") == "square" {
                                0.
                            } else {
                                3.
                            },
                            padding: if settings.get("padding") == "wide" {
                                0.5
                            } else {
                                0.2
                            },
                            layout: if matches!(variant, "stacked" | "horizontal-stacked") {
                                BarLayout::Stacked
                            } else {
                                BarLayout::Grouped
                            },
                            horizontal: matches!(variant, "horizontal" | "horizontal-stacked"),
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
                        "horizontal-stacked" => {
                            "bar(&config, &categories, &series, &BarOptions { layout: BarLayout::Stacked, horizontal: true, corner_radius: 3., ..Default::default() })?"
                        }
                        "stacked" => {
                            "bar(&config, &categories, &series, &BarOptions { layout: BarLayout::Stacked, horizontal: false, corner_radius: 3., ..Default::default() })?"
                        }
                        "horizontal" => {
                            "bar(&config, &categories, &series, &BarOptions { layout: BarLayout::Grouped, horizontal: true, corner_radius: 3., ..Default::default() })?"
                        }
                        _ => {
                            "bar(&config, &categories, &series, &BarOptions { layout: BarLayout::Grouped, horizontal: false, corner_radius: 3., ..Default::default() })?"
                        }
                    },
                )
            } else if kind == "stream" {
                macro_rules! stream_offset {
                    ($offset:ident) => {
                        (
                            stack::StackOffset::$offset,
                            concat!("stream_layout(&config, &categories, &series, &StreamLayoutOptions { stack: stack::StackOptions { order: stack::StackOrder::InsideOut, offset: stack::StackOffset::", stringify!($offset), " }, curve: Curve::Basis, legend: true, ..Default::default() })?.scene"),
                        )
                    };
                }
                let (offset, code) = match variant {
                    "silhouette" => stream_offset!(Silhouette),
                    "expand" => stream_offset!(Expand),
                    "zero" => stream_offset!(Zero),
                    _ => stream_offset!(Wiggle),
                };
                (
                    stream_layout(
                        c,
                        &categories
                            .into_iter()
                            .map(CategoryKey::from)
                            .collect::<Vec<_>>(),
                        &source,
                        &StreamLayoutOptions {
                            stack: stack::StackOptions {
                                order: stack::StackOrder::InsideOut,
                                offset,
                            },
                            curve: if settings.get("curve") == "linear" {
                                Curve::Linear
                            } else {
                                Curve::Basis
                            },
                            dots: (settings.get("dots") == "on").then(StreamDotOptions::default),
                            legend: true,
                            ..Default::default()
                        },
                    )?
                    .scene,
                    data,
                    code,
                )
            } else {
                (
                    marimekko(
                        c,
                        &categories,
                        &source,
                        &MarimekkoOptions {
                            horizontal: variant == "horizontal",
                            padding: if variant == "spaced" { 6. } else { 0. },
                            labels: settings.get("labels") == "on",
                        },
                    )?,
                    data,
                    match variant {
                        "horizontal" => {
                            "marimekko(&config, &categories, &series, &MarimekkoOptions { horizontal: true, ..Default::default() })?"
                        }
                        "spaced" => {
                            "marimekko(&config, &categories, &series, &MarimekkoOptions { padding: 6., ..Default::default() })?"
                        }
                        _ => {
                            "marimekko(&config, &categories, &series, &MarimekkoOptions::default())?"
                        }
                    },
                )
            }
        }
        "scatter" => {
            let source = RANK_SERIES
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    Series::new(
                        id,
                        (0..16)
                            .map(|i| {
                                let x = 8.
                                    + ((i * 17 + j * 23) % 84) as f64
                                    + (p + i as f64).sin() * 3.;
                                Point::new(
                                    x,
                                    12. + x * 0.55
                                        + (i as f64 * 2.3 + p).sin() * 14.
                                        + j as f64 * 4.,
                                )
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            let data=json!(source.iter().map(|s|json!({"id":s.id,"data":s.data.iter().map(|p|json!({"x":p.x,"y":p.y})).collect::<Vec<_>>()})).collect::<Vec<_>>());
            (
                scatter(
                    c,
                    &source,
                    &ScatterOptions {
                        radius: match variant {
                            "small" => 2.5,
                            "large" => 8.,
                            _ => 4.5,
                        },
                        x_scale: cart_controls::scale(settings.get("x-scale")),
                        y_scale: cart_controls::scale(settings.get("y-scale")),
                        x_domain: Some([0., 100.]),
                        y_domain: Some([0., 100.]),
                        ..Default::default()
                    },
                )?,
                data,
                match variant {
                    "small" => {
                        "scatter(&config, &series, &ScatterOptions { radius: 2.5, ..Default::default() })?"
                    }
                    "large" => {
                        "scatter(&config, &series, &ScatterOptions { radius: 8., ..Default::default() })?"
                    }
                    _ => {
                        "scatter(&config, &series, &ScatterOptions { radius: 4.5, ..Default::default() })?"
                    }
                },
            )
        }
        "radar" => {
            let dimensions = ["Speed", "Quality", "Reach", "Cost", "Growth"].map(str::to_owned);
            let source = [
                RadarSeries::new(
                    "North",
                    (0..5)
                        .map(|i| 65. + (p + f64::from(i)).sin() * 22.)
                        .collect(),
                ),
                RadarSeries::new(
                    "South",
                    (0..5)
                        .map(|i| 63. + (p * 0.8 + f64::from(i) * 1.8).cos() * 24.)
                        .collect(),
                ),
            ];
            let data = json!({"dimensions":dimensions,"series":source.iter().map(|s|json!({"id":s.id,"values":s.values})).collect::<Vec<_>>()});
            (
                radar(
                    c,
                    &dimensions,
                    &source,
                    &RadarOptions {
                        levels: if variant == "coarse" { 3 } else { 5 },
                        fill_opacity: if settings.get("fill-opacity") == "strong" {
                            0.65
                        } else {
                            0.2
                        },
                        max_value: (settings.get("maximum") == "fixed").then_some(120.),
                        rotation: if variant == "rotated" {
                            36_f64.to_radians()
                        } else {
                            0.
                        },
                    },
                )?,
                data,
                match variant {
                    "coarse" => {
                        "radar(&config, &dimensions, &series, &RadarOptions { levels: 3, ..Default::default() })?"
                    }
                    "rotated" => {
                        "radar(&config, &dimensions, &series, &RadarOptions { rotation: 36_f64.to_radians(), ..Default::default() })?"
                    }
                    _ => "radar(&config, &dimensions, &series, &RadarOptions::default())?",
                },
            )
        }
        "bump" => {
            if !(1. ..=12.).contains(&bump.line_width) {
                return Err(ChartError(
                    "Bump line width must be between 1 and 12 pixels".into(),
                ));
            }
            let source = ["North", "South", "West"]
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    BumpSeries::new(
                        id,
                        (0..6)
                            .map(|i| {
                                BumpDatum::new(
                                    format!("Q{}", i + 1),
                                    1. + ((i + j + seed as usize % 3) % 3) as f64,
                                )
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            let data=json!(source.iter().map(|s|json!({"id":s.id,"data":s.data.iter().map(|d|json!({"x":key_json(&d.x),"rank":d.rank})).collect::<Vec<_>>()})).collect::<Vec<_>>());
            macro_rules! bump_options {
                ($options:expr) => {
                    (
                        $options,
                        concat!("let mut options = ", stringify!($options), ";"),
                    )
                };
            }
            let (mut options, code) = match variant {
                "linear" => bump_options!(BumpLayoutOptions {
                    interpolation: BumpInterpolation::Linear,
                    ..Default::default()
                }),
                "no-plateau" => bump_options!(BumpLayoutOptions {
                    x_padding: 0.,
                    ..Default::default()
                }),
                "large-points" => bump_options!(BumpLayoutOptions {
                    style: BumpSeriesStyle {
                        point: BumpPointStyle {
                            size: 14.,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                "line-only" => bump_options!(BumpLayoutOptions {
                    style: BumpSeriesStyle {
                        point: BumpPointStyle {
                            size: 0.,
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                _ => bump_options!(BumpLayoutOptions::default()),
            };
            options.style.width = bump.line_width;
            options.labels.end = bump.end_labels;
            (bump_layout(c, &source, &options)?.scene, data, code)
        }
        "area-bump" => {
            let source = BAND_SERIES
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    AreaBumpSeries::new(
                        id,
                        (0..6)
                            .map(|i| {
                                AreaBumpDatum::new(
                                    format!("Q{}", i + 1),
                                    23. + (i as f64 * 1.4 + j as f64 * 2. + p).sin() * 14.,
                                )
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            let data=json!(source.iter().map(|s|json!({"id":s.id,"data":s.data.iter().map(|d|json!({"x":match &d.x {AreaBumpX::Category(x)=>json!(x),AreaBumpX::Number(x)=>json!(x)},"value":d.value})).collect::<Vec<_>>()})).collect::<Vec<_>>());
            (
                area_bump(
                    c,
                    &source,
                    &AreaBumpOptions {
                        align: if variant == "start" {
                            AreaBumpAlign::Start
                        } else {
                            AreaBumpAlign::Middle
                        },
                        interpolation: if variant == "linear" {
                            AreaBumpInterpolation::Linear
                        } else {
                            AreaBumpInterpolation::Smooth
                        },
                        spacing: if variant == "spaced" { 8. } else { 0. },
                        style: AreaBumpSeriesStyle {
                            fill_opacity: if settings.get("fill-opacity") == "light" {
                                0.35
                            } else {
                                0.8
                            },
                            ..Default::default()
                        },
                        start_labels: AreaBumpLabels {
                            enabled: settings.get("start-labels") != "off",
                            padding: BAND_LABEL_GAP,
                            size: Some(BAND_LABEL_SIZE),
                            ..Default::default()
                        },
                        end_labels: AreaBumpLabels {
                            padding: BAND_LABEL_GAP,
                            size: Some(BAND_LABEL_SIZE),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?,
                data,
                match variant {
                    "start" => {
                        "area_bump(&config, &series, &AreaBumpOptions { align: AreaBumpAlign::Start, ..Default::default() })?"
                    }
                    "linear" => {
                        "area_bump(&config, &series, &AreaBumpOptions { interpolation: AreaBumpInterpolation::Linear, ..Default::default() })?"
                    }
                    "spaced" => {
                        "area_bump(&config, &series, &AreaBumpOptions { spacing: 8., ..Default::default() })?"
                    }
                    _ => {
                        "area_bump(&config, &series, &AreaBumpOptions { align: AreaBumpAlign::Middle, ..Default::default() })?"
                    }
                },
            )
        }
        "parallel-coordinates" => {
            let dimensions = ["Speed", "Memory", "Quality", "Cost"].map(str::to_owned);
            let source = (0..9)
                .map(|i| ParallelRecord {
                    id: format!("Model {}", i + 1),
                    values: (0..4)
                        .map(|j| Some(40. + (i as f64 * 1.7 + j as f64 * 1.2 + p).sin() * 25.))
                        .collect(),
                })
                .collect::<Vec<_>>();
            let data = json!({"dimensions":dimensions,"records":source.iter().map(|r|json!({"id":r.id,"values":r.values})).collect::<Vec<_>>()});
            if variant == "horizontal" {
                (
                    parallel_coordinates(c, &dimensions, &source)?,
                    data,
                    "parallel_coordinates(&config, &dimensions, &records)?",
                )
            } else {
                let dimensions = dimensions.map(ParallelDimension::new);
                let rows = source
                    .into_iter()
                    .map(ParallelDatum::from)
                    .collect::<Vec<_>>();
                let (options, code) = if variant == "vertical" {
                    (
                        ParallelCoordinatesOptions {
                            layout: ParallelLayout::Vertical,
                            ..Default::default()
                        },
                        "parallel_coordinates_with_options(&config, &dimensions, &rows, &ParallelCoordinatesOptions { layout: ParallelLayout::Vertical, ..Default::default() })?",
                    )
                } else {
                    (
                        ParallelCoordinatesOptions {
                            curve: Curve::Basis,
                            ..Default::default()
                        },
                        "parallel_coordinates_with_options(&config, &dimensions, &rows, &ParallelCoordinatesOptions { curve: Curve::Basis, ..Default::default() })?",
                    )
                };
                (
                    parallel_coordinates_with_options(c, &dimensions, &rows, &options)?,
                    data,
                    code,
                )
            }
        }
        "heatmap" => {
            let source = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
                .into_iter()
                .enumerate()
                .flat_map(|(d, day)| {
                    (0..8).map(move |hour| {
                        HeatCell::new(
                            day,
                            format!("{:02}:00", 8 + hour * 2),
                            ((d * 13 + hour * 9 + seed as usize % 64 * 7) % 64) as f64,
                        )
                    })
                })
                .collect::<Vec<_>>();
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"row":d.row,"column":d.column,"value":d.value}))
                    .collect::<Vec<_>>()
            );
            (
                heatmap(
                    c,
                    &source,
                    &HeatmapOptions {
                        padding: if variant == "spaced" { 0.4 } else { 0.1 },
                        square: variant == "square",
                        domain: Some([0., 64.]),
                        low: Color(234, 244, 236, 255),
                        high: if settings.get("highColor") == "berry" {
                            Color(160, 58, 106, 255)
                        } else {
                            Color(34, 126, 119, 255)
                        },
                        labels: settings.get("cellLabels") == "show",
                        ..Default::default()
                    },
                )?,
                data,
                match variant {
                    "square" => {
                        "heatmap(&config, &cells, &HeatmapOptions { square: true, padding: 0.1, ..options })?"
                    }
                    "spaced" => {
                        "heatmap(&config, &cells, &HeatmapOptions { padding: 0.4, ..options })?"
                    }
                    _ => {
                        "heatmap(&config, &cells, &HeatmapOptions { square: false, padding: 0.1, ..options })?"
                    }
                },
            )
        }
        "boxplot" => {
            let source = ["North", "South", "West", "East"]
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    BoxSample::new(
                        id,
                        (0..20)
                            .map(|i| {
                                25. + j as f64 * 5.
                                    + (i as f64 * 1.8 + p + j as f64).sin() * 14.
                                    + f64::from(i % 4) * 3.
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            let data = json!(
                source
                    .iter()
                    .map(|s| json!({"id":s.id,"values":s.values}))
                    .collect::<Vec<_>>()
            );
            let observations: Vec<_> = source
                .iter()
                .flat_map(|series| {
                    series.values.iter().enumerate().map(move |(i, &value)| {
                        BoxObservation::new(
                            format!("{}-{i}", series.id),
                            &series.id,
                            "",
                            Some(value),
                        )
                    })
                })
                .collect();
            let mut options = BoxPlotLayoutOptions::default();
            if settings.get("medianWidth") == "bold" {
                options.median.width = 4.;
            }
            if settings.get("whiskerCaps") == "full" {
                options.cap_size = 1.;
            }
            let code = match variant {
                "horizontal" => {
                    options.orientation = StatisticsOrientation::Horizontal;
                    "boxplot_layout(&config, &observations, &BoxPlotLayoutOptions { orientation: StatisticsOrientation::Horizontal, ..Default::default() })?.scene"
                }
                "extrema" => {
                    options.whiskers = BoxWhiskers::Extrema;
                    "boxplot_layout(&config, &observations, &BoxPlotLayoutOptions { whiskers: BoxWhiskers::Extrema, ..Default::default() })?.scene"
                }
                _ => {
                    "boxplot_layout(&config, &observations, &BoxPlotLayoutOptions::default())?.scene"
                }
            };
            (
                boxplot_layout(c, &observations, &options)?.scene,
                data,
                code,
            )
        }
        "bullet" => {
            let source = ["Revenue", "Profit", "Customers"]
                .into_iter()
                .enumerate()
                .map(|(i, id)| BulletDatum {
                    id: id.into(),
                    ranges: vec![100., 75., 50.],
                    measures: vec![66. + (p + i as f64).sin() * 18.],
                    markers: vec![85. - i as f64 * 5.],
                })
                .collect::<Vec<_>>();
            let data=json!(source.iter().map(|s|json!({"id":s.id,"ranges":s.ranges,"measures":s.measures,"markers":s.markers})).collect::<Vec<_>>());
            let mut options = BulletLayoutOptions {
                spacing: 8.,
                measure_size: if settings.get("measureSize") == "wide" {
                    0.7
                } else {
                    0.4
                },
                marker_width: if settings.get("markerWidth") == "bold" {
                    5.
                } else {
                    2.
                },
                ..Default::default()
            };
            let code = match variant {
                "vertical" => {
                    options.orientation = StatisticsOrientation::Vertical;
                    "bullet_layout(&config, &rows, &BulletLayoutOptions { orientation: StatisticsOrientation::Vertical, spacing: 8., ..Default::default() })?.scene"
                }
                "reverse" => {
                    options.reverse = true;
                    "bullet_layout(&config, &rows, &BulletLayoutOptions { reverse: true, spacing: 8., ..Default::default() })?.scene"
                }
                _ => {
                    "bullet_layout(&config, &rows, &BulletLayoutOptions { spacing: 8., ..Default::default() })?.scene"
                }
            };
            (bullet_layout(c, &source, &options)?.scene, data, code)
        }
        "calendar" => {
            let source = (1..=12)
                .flat_map(|month| {
                    (1..=28).map(move |day| CalendarDay {
                        month,
                        day,
                        value: 30. + ((f64::from(month) + f64::from(day)) * 0.7 + p).sin() * 25.,
                    })
                })
                .collect::<Vec<_>>();
            let data = json!({"year":2026,"days":source.iter().map(|d|json!({"month":d.month,"day":d.day,"value":d.value})).collect::<Vec<_>>()});
            let days: Vec<_> = source
                .iter()
                .map(|d| CalendarCell::new((2026, d.month, d.day), Some(d.value)))
                .collect();
            let mut options = CalendarLayoutOptions {
                mode: CalendarLayoutMode::FullYears,
                day_spacing: 0.5,
                weekday_space: 12.,
                ..Default::default()
            };
            if settings.get("dayBorders") == "off" {
                options.day_border = None;
            }
            if settings.get("monthBorders") == "off" {
                options.month_border = None;
            }
            let code = match variant {
                "monday" => {
                    options.first_weekday = CalendarWeekday::Monday;
                    "calendar_layout(&config, &days, &CalendarLayoutOptions { mode: CalendarLayoutMode::FullYears, first_weekday: CalendarWeekday::Monday, ..options })?.scene"
                }
                "vertical" => {
                    options.orientation = CalendarOrientation::Vertical;
                    "calendar_layout(&config, &days, &CalendarLayoutOptions { mode: CalendarLayoutMode::FullYears, orientation: CalendarOrientation::Vertical, ..options })?.scene"
                }
                _ => {
                    "calendar_layout(&config, &days, &CalendarLayoutOptions { mode: CalendarLayoutMode::FullYears, ..options })?.scene"
                }
            };
            (calendar_layout(c, &days, &options)?.scene, data, code)
        }
        "time-range" => {
            let source = (1..=3)
                .flat_map(|month| {
                    (1..=28).map(move |day| {
                        CalendarCell::new(
                            (2026, month, day),
                            Some(30. + ((f64::from(month) + f64::from(day)) * 0.7 + p).sin() * 25.),
                        )
                    })
                })
                .collect::<Vec<_>>();
            let data = json!({"start":[2026,1,1],"end":[2026,3,31],"days":source.iter().map(|d|json!({"date":d.date,"value":d.value})).collect::<Vec<_>>()});
            (
                calendar_layout(
                    c,
                    &source,
                    &CalendarLayoutOptions {
                        start: Some((2026, 1, 1)),
                        end: Some((2026, 3, 31)),
                        day_radius: if settings.get("dayCorners") == "square" {
                            0.
                        } else {
                            2.
                        },
                        day_opacity: if settings.get("dayOpacity") == "muted" {
                            0.55
                        } else {
                            1.
                        },
                        orientation: if variant == "vertical" {
                            CalendarOrientation::Vertical
                        } else {
                            CalendarOrientation::Horizontal
                        },
                        first_weekday: if variant == "sunday" {
                            CalendarWeekday::Sunday
                        } else {
                            CalendarWeekday::Monday
                        },
                        colors: ColorScaleOptions {
                            colors: vec![Color(234, 244, 236, 255), Color(34, 126, 119, 255)],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?
                .scene,
                data,
                match variant {
                    "sunday" => {
                        "calendar_layout(&config, &days, &CalendarLayoutOptions { first_weekday: CalendarWeekday::Sunday, ..options })?.scene"
                    }
                    "vertical" => {
                        "calendar_layout(&config, &days, &CalendarLayoutOptions { orientation: CalendarOrientation::Vertical, first_weekday: CalendarWeekday::Monday, ..options })?.scene"
                    }
                    _ => {
                        "calendar_layout(&config, &days, &CalendarLayoutOptions { first_weekday: CalendarWeekday::Monday, ..options })?.scene"
                    }
                },
            )
        }
        "swarm" => {
            let radius = (c.inner_width().min(c.inner_height()) / 48.).clamp(2., 4.);
            let source = (0..48)
                .map(|i| SwarmDatum {
                    id: format!("sample-{i}"),
                    group: if i < 24 { "North" } else { "South" }.into(),
                    value: 30. + (f64::from(i) * 1.9 + p).sin() * 19.,
                    radius,
                })
                .collect::<Vec<_>>();
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"id":d.id,"group":d.group,"value":d.value,"radius":d.radius}))
                    .collect::<Vec<_>>()
            );
            let mut scene = swarmplot(
                c,
                &source,
                &SwarmOptions {
                    horizontal: variant != "vertical",
                    gap: if variant == "compact" { 0. } else { 1. },
                    ..Default::default()
                },
            )?;
            let outlined = settings.get("nodeBorder") == "outline";
            let muted = settings.get("nodeOpacity") == "muted";
            for node in &mut scene.marks {
                if outlined {
                    node.stroke = Some(glory::render::GraphicStroke {
                        color: Color(36, 58, 70, 255),
                        width: 1.5,
                        cap: glory::render::StrokeCap::Butt,
                    });
                }
                if muted {
                    node.opacity = 0.55;
                }
            }
            (
                scene,
                data,
                match variant {
                    "vertical" => {
                        "swarmplot(&config, &observations, &SwarmOptions { horizontal: false, ..Default::default() })?"
                    }
                    "compact" => {
                        "swarmplot(&config, &observations, &SwarmOptions { gap: 0., ..Default::default() })?"
                    }
                    _ => "swarmplot(&config, &observations, &SwarmOptions::default())?",
                },
            )
        }
        _ => return Err(ChartError(format!("Unknown Cartesian chart: {kind}"))),
    };
    if matches!(kind, "bump" | "area-bump") && variant == "linear" {
        for mark in &mut scene.marks {
            if mark.id.starts_with("bump-line:") || mark.id.starts_with("area-bump:") {
                match_rank_curve_commands(mark);
            }
        }
    }
    cart_controls::style_marks(kind, &mut scene, settings);
    Ok(Sample { scene, data, code })
}
