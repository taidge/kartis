//! The public gallery uses the same owned chart surfaces as a Rust application.
#![cfg(target_arch = "wasm32")]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use kartis::browser::{BrowserChart, BrowserOptions, BrowserRenderer};
use kartis::*;
use wasm_bindgen::prelude::*;

const KINDS: [&str; 6] = ["line", "bar", "pie", "scatter", "waffle", "heatmap"];
thread_local! {
    static GALLERY: RefCell<Option<Gallery>> = const { RefCell::new(None) };
    // Author data outlives the owned DOM surfaces during BFCache suspension.
    static DATASET: Rc<Cell<u32>> = Rc::new(Cell::new(0));
}
struct Gallery {
    charts: Vec<BrowserChart>,
    seed: Rc<Cell<u32>>,
}
fn js(error: impl ToString) -> JsValue {
    JsValue::from_str(&error.to_string())
}
fn root(kind: &str) -> std::result::Result<web_sys::Element, JsValue> {
    web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(&format!("chart-{kind}")))
        .ok_or_else(|| js(format!("Missing gallery surface: {kind}")))
}
fn scene(kind: &str, width: f64, height: f64, seed: u32) -> Result<Scene> {
    let phase = f64::from(seed % 7);
    let mut config = ChartConfig {
        width: width.clamp(220., 1800.),
        height: height.clamp(220., 600.),
        margin: Margin {
            top: 22.,
            right: 24.,
            bottom: 38.,
            left: 44.,
        },
        title: format!("Kartis {kind} example"),
        theme: Theme {
            colors: vec![
                Color(34, 126, 119, 255),
                Color(232, 137, 86, 255),
                Color(113, 127, 187, 255),
                Color(211, 173, 78, 255),
            ],
            text: Color(108, 117, 124, 255),
            grid: Color(233, 236, 235, 255),
            font_size: 11.,
            ..Default::default()
        },
    };
    match kind {
        "line" => {
            let series = ["This year", "Last year"]
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
                                    22. + x * (1.8 - j as f64 * 0.55)
                                        + (x * 0.75 + phase + j as f64).sin() * 8.
                                        + j as f64 * 6.,
                                )
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            line(
                &config,
                &series,
                &LineOptions {
                    curve: Curve::MonotoneX,
                    area: true,
                    y_domain: Some([0., 70.]),
                    ..Default::default()
                },
            )
        }
        "bar" => {
            let categories = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat"].map(str::to_owned);
            let series = ["Studio", "Online"]
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    BarSeries::new(
                        id,
                        (0..6)
                            .map(|i| Some(24. + ((i * 7 + j * 11 + seed as usize * 3) % 35) as f64))
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            bar(
                &config,
                &categories,
                &series,
                &BarOptions {
                    corner_radius: 4.,
                    ..Default::default()
                },
            )
        }
        "pie" => {
            config.margin = Margin {
                top: 18.,
                right: 28.,
                bottom: 18.,
                left: 28.,
            };
            pie(
                &config,
                &[
                    RadialDatum::new("Direct", 42. + phase * 2.),
                    RadialDatum::new("Search", 28. - phase),
                    RadialDatum::new("Community", 19. + phase),
                    RadialDatum::new("Other", 11.),
                ],
                &PieOptions {
                    inner_radius: 0.65,
                    pad_angle: 0.035,
                    labels: false,
                    ..Default::default()
                },
            )
        }
        "scatter" => {
            let series = ["North", "South", "West"]
                .into_iter()
                .enumerate()
                .map(|(j, id)| {
                    Series::new(
                        id,
                        (0..14)
                            .map(|i| {
                                let x = 8. + ((i * 17 + j * 23) % 88) as f64;
                                Point::new(
                                    x,
                                    12. + x * 0.6
                                        + (i as f64 * 2.3 + phase).sin() * 16.
                                        + j as f64 * 4.,
                                )
                            })
                            .collect(),
                    )
                })
                .collect::<Vec<_>>();
            scatter(
                &config,
                &series,
                &ScatterOptions {
                    radius: 4.5,
                    x_domain: Some([0., 100.]),
                    y_domain: Some([0., 100.]),
                    ..Default::default()
                },
            )
        }
        "waffle" => {
            config.margin = Margin {
                top: 14.,
                right: 16.,
                bottom: 14.,
                left: 16.,
            };
            Ok(waffle_layout(
                &config,
                &[
                    RadialDatum::new("Complete", 35. + phase),
                    RadialDatum::new("In progress", 20. - phase),
                    RadialDatum::new("Planned", 10.),
                ],
                &WaffleLayoutOptions {
                    rows: 6,
                    columns: 10,
                    total: Some(70.),
                    padding: 5.,
                    corner_radius: 4.,
                    empty_color: Color(235, 239, 237, 255),
                    ..Default::default()
                },
            )?
            .scene)
        }
        "heatmap" => {
            let days = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
            let cells = days
                .into_iter()
                .enumerate()
                .flat_map(|(d, day)| {
                    (0..8).map(move |hour| {
                        HeatCell::new(
                            format!("{:02}:00", 8 + hour * 2),
                            day,
                            ((d * 13 + hour * 9 + seed as usize * 7) % 64) as f64,
                        )
                    })
                })
                .collect::<Vec<_>>();
            heatmap(
                &config,
                &cells,
                &HeatmapOptions {
                    padding: 0.12,
                    domain: Some([0., 64.]),
                    low: Color(234, 244, 236, 255),
                    high: Color(34, 126, 119, 255),
                    ..Default::default()
                },
            )
        }
        _ => Err(ChartError("Unknown gallery chart".into())),
    }
}
/// Changing renderers disposes the previous owners and their native listeners.
#[wasm_bindgen]
pub fn mount_gallery(renderer: &str) -> std::result::Result<(), JsValue> {
    let renderer = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("Unknown renderer")),
    };
    let seed = DATASET.with(Rc::clone);
    stop_gallery();
    let mut charts = Vec::new();
    for kind in KINDS {
        let root = root(kind)?;
        let bounds = root.get_bounding_client_rect();
        // Leave room for the real Glory legend below the owned surface.
        let height = if kind == "line" {
            288.
        } else if kind == "heatmap" {
            264.
        } else {
            250.
        };
        let chart = BrowserChart::mount(
            root,
            scene(kind, bounds.width(), height, seed.get()).map_err(js)?,
            BrowserOptions {
                renderer,
                data_controls: false,
                legend: kind != "heatmap",
                height: Some(height),
                ..Default::default()
            },
        )
        .map_err(js)?;
        let current = seed.clone();
        chart.on_resize(move |w, h| scene(kind, w, h, current.get()));
        charts.push(chart);
    }
    GALLERY.with(|g| *g.borrow_mut() = Some(Gallery { charts, seed }));
    Ok(())
}
#[wasm_bindgen]
pub fn next_dataset() -> std::result::Result<(), JsValue> {
    GALLERY.with(|g| {
        let g = g.borrow();
        let g = g.as_ref().ok_or_else(|| js("Gallery is not mounted"))?;
        g.seed.set(g.seed.get().wrapping_add(1));
        for (kind, chart) in KINDS.into_iter().zip(&g.charts) {
            let snapshot = chart.snapshot().map_err(js)?;
            chart
                .set_scene(scene(kind, snapshot.width, snapshot.height, g.seed.get()).map_err(js)?)
                .map_err(js)?;
        }
        Ok(())
    })
}
/// Export the author dataset with the current responsive chart dimensions.
#[wasm_bindgen]
pub fn export_svg(kind: &str) -> std::result::Result<String, JsValue> {
    GALLERY.with(|g| {
        let g = g.borrow();
        let g = g.as_ref().ok_or_else(|| js("Gallery is not mounted"))?;
        let index = KINDS
            .iter()
            .position(|k| *k == kind)
            .ok_or_else(|| js("Unknown chart"))?;
        let snapshot = g.charts[index].snapshot().map_err(js)?;
        render::to_svg(&scene(kind, snapshot.width, snapshot.height, g.seed.get()).map_err(js)?)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn stop_gallery() {
    GALLERY.with(|g| {
        g.borrow_mut().take();
    });
}
