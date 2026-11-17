use serde_json::json;

use super::{phase, *};

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
pub(super) fn sample(kind: &str, c: &ChartConfig, seed: u32, variant: &str) -> Result<Sample> {
    let p = phase(seed);
    let (scene, data, code) = match kind {
        "line" | "area" => {
            let source = lines(seed);
            let data=json!(source.iter().map(|s|json!({"id":s.id,"data":s.data.iter().map(|p|json!({"x":p.x,"y":p.y})).collect::<Vec<_>>()})).collect::<Vec<_>>());
            let curve = match variant {
                "linear" => Curve::Linear,
                "step" => Curve::Step,
                _ => Curve::MonotoneX,
            };
            (
                line(
                    c,
                    &source,
                    &LineOptions {
                        curve,
                        area: kind == "area",
                        y_domain: Some([0., 75.]),
                        ..Default::default()
                    },
                )?,
                data,
                match (kind, variant) {
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
                            corner_radius: 3.,
                            layout: if variant == "stacked" {
                                BarLayout::Stacked
                            } else {
                                BarLayout::Grouped
                            },
                            horizontal: variant == "horizontal",
                            ..Default::default()
                        },
                    )?,
                    data,
                    match variant {
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
                                offset: stack::StackOffset::Wiggle,
                            },
                            curve: Curve::Basis,
                            legend: true,
                            ..Default::default()
                        },
                    )?
                    .scene,
                    data,
                    "stream_layout(&config, &categories, &series, &StreamLayoutOptions::default())?.scene",
                )
            } else {
                (
                    marimekko(c, &categories, &source, &MarimekkoOptions::default())?,
                    data,
                    "marimekko(&config, &categories, &series, &MarimekkoOptions::default())?",
                )
            }
        }
        "scatter" => {
            let source = ["North", "South", "West"]
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
                        radius: 4.5,
                        x_domain: Some([0., 100.]),
                        y_domain: Some([0., 100.]),
                        ..Default::default()
                    },
                )?,
                data,
                "scatter(&config, &series, &ScatterOptions { radius: 4.5, ..Default::default() })?",
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
                radar(c, &dimensions, &source, &RadarOptions::default())?,
                data,
                "radar(&config, &dimensions, &series, &RadarOptions::default())?",
            )
        }
        "bump" => {
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
            (
                bump_layout(c, &source, &BumpLayoutOptions::default())?.scene,
                data,
                "bump_layout(&config, &rank_series, &BumpLayoutOptions::default())?.scene",
            )
        }
        "area-bump" => {
            let source = ["Search", "Direct", "Partners"]
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
                        start_labels: AreaBumpLabels {
                            padding: 4.,
                            size: Some(10.),
                            ..Default::default()
                        },
                        end_labels: AreaBumpLabels {
                            padding: 4.,
                            size: Some(10.),
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?,
                data,
                "area_bump(&config, &series, &AreaBumpOptions::default())?",
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
            (
                parallel_coordinates(c, &dimensions, &source)?,
                data,
                "parallel_coordinates(&config, &dimensions, &records)?",
            )
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
                        padding: 0.1,
                        domain: Some([0., 64.]),
                        low: Color(234, 244, 236, 255),
                        high: Color(34, 126, 119, 255),
                        ..Default::default()
                    },
                )?,
                data,
                "heatmap(&config, &cells, &HeatmapOptions::default())?",
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
            (boxplot(c, &source)?, data, "boxplot(&config, &samples)?")
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
            (bullet(c, &source)?, data, "bullet(&config, &rows)?")
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
            (
                calendar(c, 2026, &source)?,
                data,
                "calendar(&config, 2026, &daily_values)?",
            )
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
                        day_radius: 2.,
                        first_weekday: CalendarWeekday::Monday,
                        colors: ColorScaleOptions {
                            colors: vec![Color(234, 244, 236, 255), Color(34, 126, 119, 255)],
                            ..Default::default()
                        },
                        ..Default::default()
                    },
                )?
                .scene,
                data,
                "calendar_layout(&config, &days, &CalendarLayoutOptions { start: Some((2026, 1, 1)), end: Some((2026, 3, 31)), ..Default::default() })?.scene",
            )
        }
        "swarm" => {
            let source = (0..48)
                .map(|i| {
                    SwarmDatum::new(
                        format!("sample-{i}"),
                        if i < 24 { "North" } else { "South" },
                        30. + (f64::from(i) * 1.9 + p).sin() * 19.,
                    )
                })
                .collect::<Vec<_>>();
            let data = json!(
                source
                    .iter()
                    .map(|d| json!({"id":d.id,"group":d.group,"value":d.value}))
                    .collect::<Vec<_>>()
            );
            (
                swarmplot(c, &source, &SwarmOptions::default())?,
                data,
                "swarmplot(&config, &observations, &SwarmOptions::default())?",
            )
        }
        _ => return Err(ChartError(format!("Unknown Cartesian chart: {kind}"))),
    };
    Ok(Sample { scene, data, code })
}
