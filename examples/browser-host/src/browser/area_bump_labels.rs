//! Retained AreaBump labels placed from actual Canvas text metrics.
use std::collections::BTreeSet;

use web_sys::{CanvasRenderingContext2d, Element, HtmlCanvasElement};

use super::*;

const WIDTH: f64 = 520.;
const HEIGHT: f64 = 238.;
const OBSTACLE: LayoutRect = LayoutRect {
    x: 391.,
    y: 177.,
    width: 112.,
    height: 9.,
};
const INK: Color = Color(240, 0, 180, 255);

struct Fixture {
    chart: BrowserChart,
    root: Element,
    renderer: &'static str,
    phase: u8,
    layout: AreaBumpLayout,
}

thread_local! {
    static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };
}

fn config() -> ChartConfig {
    ChartConfig {
        width: WIDTH,
        height: HEIGHT,
        title: "AreaBump measured endpoints".into(),
        margin: Margin {
            left: 145.,
            right: 145.,
            top: 34.,
            bottom: 46.,
        },
        ..Default::default()
    }
}

fn measurements(layout: &AreaBumpLayout) -> Result<Vec<AreaBumpMeasuredLabel>> {
    let document = web_sys::window().unwrap().document().unwrap();
    let fail = |_| ChartError("AreaBump Canvas text measurement unavailable".into());
    let canvas = document
        .create_element("canvas")
        .map_err(fail)?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| ChartError("invalid measurement canvas".into()))?;
    let context = canvas
        .get_context("2d")
        .map_err(fail)?
        .ok_or_else(|| ChartError("AreaBump Canvas context unavailable".into()))?
        .dyn_into::<CanvasRenderingContext2d>()
        .map_err(|_| ChartError("invalid measurement context".into()))?;
    layout
        .endpoint_label_candidates()
        .iter()
        .map(|candidate| {
            let label = &candidate.label;
            context.set_font(&format!("{}px sans-serif", label.size));
            context.set_text_baseline("alphabetic");
            context.set_text_align(match label.anchor {
                TextAnchor::Start => "start",
                TextAnchor::Middle => "center",
                TextAnchor::End => "end",
            });
            let metric = context.measure_text(&label.text).map_err(fail)?;
            Ok(candidate.measured(LayoutRect {
                x: -metric.actual_bounding_box_left(),
                y: -metric.actual_bounding_box_ascent(),
                width: metric.actual_bounding_box_left() + metric.actual_bounding_box_right(),
                height: metric.actual_bounding_box_ascent() + metric.actual_bounding_box_descent(),
            }))
        })
        .collect()
}

fn place_options(layout: &AreaBumpLayout) -> AreaBumpLabelPlacementOptions {
    let mut options = AreaBumpLabelPlacementOptions::for_scene(&layout.scene);
    options.collision.forbidden = vec![OBSTACLE];
    options.leader = Some(GraphicStroke {
        color: Color(65, 70, 90, 255),
        width: 1.5,
        cap: StrokeCap::Butt,
    });
    options
}

fn source(phase: u8) -> Result<AreaBumpLayout> {
    let labels = AreaBumpLabels {
        padding: 14.,
        size: Some(19.),
        color: Some(INK),
        ..Default::default()
    };
    let data = [
        ("Revenue", 90., 90. - 6. * f64::from(phase)),
        ("Waiting", 5., 5. + 3. * f64::from(phase)),
        ("On Hold", 5., 5. + 3. * f64::from(phase)),
    ]
    .map(|(id, a, b)| {
        AreaBumpSeries::new(
            id,
            vec![AreaBumpDatum::new("Start", a), AreaBumpDatum::new("End", b)],
        )
    });
    let mut raw = layout_area_bump(
        &config(),
        &data,
        &AreaBumpOptions {
            tie_break: AreaBumpTieBreak::SeriesId,
            start_labels: labels.clone(),
            end_labels: labels,
            axis_top: None,
            axis_bottom: None,
            grid: false,
            style: AreaBumpSeriesStyle {
                fill_opacity: 1.,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let mut obstacle = Mark::new(
        "area-label-obstacle",
        Geometry::Rect {
            x: OBSTACLE.x,
            y: OBSTACLE.y,
            width: OBSTACLE.width,
            height: OBSTACLE.height,
        },
        Color(249, 240, 209, 255),
    );
    obstacle.label.clear();
    raw.scene.marks.push(obstacle);
    raw.place_endpoint_labels(&measurements(&raw)?, &place_options(&raw))
}

#[wasm_bindgen]
pub fn area_label_mount() -> std::result::Result<(), JsValue> {
    area_label_stop();
    let layout = source(0).map_err(js)?;
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("area-label-{renderer}"));
        root.set_attribute("style", "width:520px;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer: backend,
                height: Some(HEIGHT),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                ..Default::default()
            },
        )
        .map_err(|error| {
            root.remove();
            area_label_stop();
            js(error)
        })?;
        FIXTURES.with(|entries| {
            entries.borrow_mut().push(Fixture {
                chart,
                root,
                renderer,
                phase: 0,
                layout: layout.clone(),
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn area_label_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js("unknown AreaBump endpoint fixture phase"));
    }
    let layout = source(phase).map_err(js)?;
    FIXTURES.with(|entries| {
        for fixture in entries.borrow_mut().iter_mut() {
            fixture.chart.set_scene(layout.scene.clone()).map_err(js)?;
            fixture.layout = layout.clone();
            fixture.phase = phase;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn area_label_focus(on: bool) -> std::result::Result<(), JsValue> {
    let active = if on {
        ["Waiting".into()].into()
    } else {
        BTreeSet::new()
    };
    FIXTURES.with(|entries| {
        for fixture in entries.borrow_mut().iter_mut() {
            let layout = fixture.layout.with_active(active.clone()).map_err(js)?;
            fixture.chart.set_scene(layout.scene.clone()).map_err(js)?;
            fixture.layout = layout;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn area_label_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|entries| {
        for fixture in entries.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn area_label_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|entries| {
        let entries = entries.borrow();
        let Some(fixture) = entries.iter().find(|fixture| fixture.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = fixture.chart.snapshot().map_err(js)?;
        let layout = &fixture.layout;
        let labels: Vec<_> = layout
            .endpoint_label_candidates()
            .iter()
            .zip(layout.endpoint_label_placements())
            .map(|(c, p)| {
                let label = p.label_index.map(|index| &layout.scene.labels[index]);
                let leader = p.leader_index.map(|index| &layout.scene.marks[index]);
                json!({
                    "id": c.id, "series": c.series,
                    "side": if c.side == AreaBumpLabelSide::Start { "start" } else { "end" },
                    "original":[c.label.x, c.label.y], "endpoint":c.endpoint,
                    "position":label.map(|l| [l.x,l.y]), "text": c.label.text,
                    "bounds":p.bounds.map(|r| [r.x,r.y,r.width,r.height]),
                    "leader":leader.map(|m| &m.id), "opacity":leader.map(|m| m.opacity),
                })
            })
            .collect();
        Ok(json!({
            "active":state.active, "revision":state.revision, "pending":state.pending,
            "phase":fixture.phase, "width":WIDTH, "height":HEIGHT,
            "obstacle":[OBSTACLE.x,OBSTACLE.y,OBSTACLE.width,OBSTACLE.height],
            "labels":labels, "focus":layout.active_ids,
        })
        .to_string())
    })
}

#[wasm_bindgen]
pub fn area_label_rejected() -> bool {
    FIXTURES.with(|entries| {
        let entries = entries.borrow();
        entries.len() == 2
            && entries.iter().all(|fixture| {
                let layout = &fixture.layout;
                let Ok(mut measured) = measurements(layout) else {
                    return false;
                };
                let Some(first) = measured.first_mut() else {
                    return false;
                };
                first.signature.push('x');
                layout
                    .place_endpoint_labels(&measured, &place_options(layout))
                    .is_err()
            })
    })
}

#[wasm_bindgen]
pub fn area_label_stop() {
    FIXTURES.with(|entries| {
        for fixture in entries.borrow_mut().drain(..) {
            fixture.chart.stop();
            fixture.root.remove();
        }
    });
}
