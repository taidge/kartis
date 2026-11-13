//! Actual browser font selection and font-aware semantic measurements.
use std::collections::BTreeSet;

use glory::Cage;
use glory::style::Family;

use super::*;

const FAMILY: &str = "Kartis Fixture Typography";
const QUOTED_FAMILY: &str = "Chart \"Quoted\"";
struct Fixture {
    charts: Vec<BrowserChart>,
    roots: Vec<web_sys::Element>,
    semantic: glory::web::WebApp,
    source: Cage<Scene>,
    hidden: Cage<BTreeSet<String>>,
    phase: u8,
    errors: Rc<RefCell<Vec<String>>>,
    measurements: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static FIXTURE:RefCell<Option<Fixture>>=const{RefCell::new(None)};}

fn source(phase: u8) -> Result<Scene> {
    if phase > 6 {
        return Err(ChartError("unsupported typography phase".into()));
    }
    let mut scene = ChartConfig {
        width: 360.,
        height: 220.,
        title: "Shared typography".into(),
        ..Default::default()
    }
    .scene()?;
    if phase != 4 {
        scene.typography.default = FontOptions {
            family: Some(vec![Family::Named(
                if phase == 2 { QUOTED_FAMILY } else { FAMILY }.into(),
            )]),
            weight: Some(match phase {
                1 => 700.,
                5 => 500.,
                6 => 600.,
                _ => 400.,
            }),
        };
    }
    scene.typography.tooltip.weight = Some(700.);
    scene.typography.labels.insert(
        1,
        FontOptions {
            family: None,
            weight: Some(400.),
        },
    );
    for (y, size) in [(85., 50.), (155., 40.)] {
        scene.labels.push(Label {
            x: 40.,
            y,
            size,
            text: "AB".into(),
            color: Color(0, 0, 0, 255),
            rotation: 0.,
            anchor: TextAnchor::Start,
        });
        scene
            .label_series
            .insert(scene.labels.len() - 1, "alpha".into());
    }
    let mut mark = Mark::new(
        "datum",
        Geometry::Circle {
            x: 285.,
            y: 70.,
            radius: 16.,
        },
        Color(32, 102, 188, 255),
    );
    mark.series = "alpha".into();
    mark.label = "AB".into();
    mark.value = 12.;
    mark.clip = Some(ClipRect {
        x: 280.,
        y: 65.,
        width: 20.,
        height: 10.,
    });
    scene.marks.push(mark);
    scene.legend.push(LegendItem {
        id: "alpha".into(),
        label: "AB".into(),
        color: Color(32, 102, 188, 255),
    });
    if phase == 3 {
        scene.typography.default.weight = Some(1001.);
    }
    Ok(scene)
}

#[wasm_bindgen]
pub fn typography_font_bytes(bold: bool) -> Vec<u8> {
    if bold {
        include_bytes!("../../../../tests/fonts/KartisFixtureTypography-Bold.ttf").to_vec()
    } else {
        include_bytes!("../../../../tests/fonts/KartisFixtureTypography-Regular.ttf").to_vec()
    }
}

fn root(
    document: &web_sys::Document,
    name: &str,
) -> std::result::Result<web_sys::Element, JsValue> {
    let root = document.create_element("div")?;
    root.set_id(&format!("typography-{name}"));
    root.set_attribute(
        "style",
        "width:360px;position:relative;margin:12px 0;font-weight:900;font-family:serif",
    )?;
    document.body().unwrap().append_child(&root)?;
    Ok(root)
}

#[wasm_bindgen]
pub fn typography_mount() -> std::result::Result<(), JsValue> {
    typography_stop()?;
    let document = web_sys::window().unwrap().document().unwrap();
    let initial = source(0).map_err(js)?;
    let mut roots = Vec::new();
    let mut charts = Vec::new();
    let errors = Rc::new(RefCell::new(Vec::new()));
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = root(&document, name)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.clone(),
            BrowserOptions {
                renderer,
                height: Some(220.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let captured = errors.clone();
        chart.on_event(move |event| {
            if let BrowserEvent::Error(error) = event {
                captured.borrow_mut().push(error.0);
            }
        });
        chart.flush().map_err(js)?;
        roots.push(root);
        charts.push(chart);
    }
    let semantic_root = root(&document, "semantic")?;
    roots.push(semantic_root.clone());
    let mut inputs = None;
    let measurements = Rc::new(RefCell::new(Vec::new()));
    let calls = measurements.clone();
    let rejects = errors.clone();
    let canvas = document
        .create_element("canvas")?
        .dyn_into::<web_sys::HtmlCanvasElement>()?;
    let context = canvas
        .get_context("2d")?
        .ok_or_else(|| js("Canvas measurement unavailable"))?
        .dyn_into::<web_sys::CanvasRenderingContext2d>()?;
    let semantic = glory::web::launch_with(semantic_root, |scope| {
        let scene = scope.cage(initial);
        let hidden = scope.cage(BTreeSet::new());
        inputs = Some((scene, hidden));
        chart(scene)
            .hidden(hidden)
            .tooltip_marks(vec!["datum".into()])
            .tooltip_formatter(|_| Ok("AB".into()))
            .legend_options(LegendOptions {
                text: PresentationText { size: 20., ..Default::default() },
                ..Default::default()
            })
            .tooltip_options(TooltipOptions {
                text: PresentationText { size: 20., ..Default::default() },
                ..Default::default()
            })
            .legend_layout(LegendLayoutOptions {
                bounds: LayoutRect { x: 5., y: 180., width: 345., height: 35. },
                ..Default::default()
            })
            .tooltip_layout(TooltipLayoutOptions {
                viewport: LayoutRect { x: 0., y: 0., width: 360., height: 220. },
                ..Default::default()
            })
            .label_collision(LabelCollisionOptions {
                viewport: LayoutRect { x: 0., y: 0., width: 360., height: 220. },
                ..Default::default()
            })
            .font_text_measurer(move |text, style, font| {
                context.set_font(&font.canvas_font(style.size));
                let metrics = context.measure_text(text).map_err(|_| ChartError("font measurement failed".into()))?;
                let width = metrics.width();
                let mut calls = calls.borrow_mut();
                if calls.len() == 256 { calls.remove(0); }
                calls.push(json!({"text":text,"weight":font.weight,"family":font.css_family(),"size":style.size,"width":width,"canvasFont":context.font()}));
                Ok(TextSize { width, height: style.size * style.line_height })
            })
            .on_reject(move |error| rejects.borrow_mut().push(error.0))
    })
    .map_err(js)?;
    let (source, hidden) = inputs.unwrap();
    FIXTURE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            charts,
            roots,
            semantic,
            source,
            hidden,
            phase: 0,
            errors,
            measurements,
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn typography_phase(phase: u8) -> std::result::Result<String, JsValue> {
    let candidate = source(phase).map_err(js)?;
    FIXTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let fixture = slot
            .as_mut()
            .ok_or_else(|| js("typography fixture stopped"))?;
        let mut rejected = Vec::new();
        for chart in &fixture.charts {
            let result = chart.set_scene(candidate.clone());
            if phase == 3 {
                rejected.push(result.is_err());
            } else {
                result.map_err(js)?;
                chart.flush().map_err(js)?;
            }
        }
        fixture.source.revise(|scene| *scene = candidate);
        fixture.semantic.app().flush().map_err(js)?;
        if phase != 3 {
            fixture.phase = phase;
        }
        Ok(
            json!({"browserRejected":rejected,"errors":fixture.errors.borrow().clone()})
                .to_string(),
        )
    })
}

#[wasm_bindgen]
pub fn typography_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let fixture = slot
            .as_ref()
            .ok_or_else(|| js("typography fixture stopped"))?;
        let hidden = if hidden {
            BTreeSet::from(["alpha".into()])
        } else {
            BTreeSet::new()
        };
        for chart in &fixture.charts {
            chart.handle().set_hidden(hidden.clone()).map_err(js)?;
            chart.flush().map_err(js)?;
        }
        fixture.hidden.revise(|set| *set = hidden);
        fixture.semantic.app().flush().map_err(js)
    })
}

#[wasm_bindgen]
pub fn typography_snapshot() -> std::result::Result<String, JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let Some(fixture) = slot.as_ref() else { return Ok(json!({"active":false}).to_string()); };
        let charts = fixture.charts.iter().map(|chart| {
            let state = chart.snapshot()?;
            Ok(json!({"revision":state.revision,"pending":state.pending,"hidden":state.hidden,"hovered":state.hovered,"selected":state.selected}))
        }).collect::<Result<Vec<_>>>().map_err(js)?;
        Ok(json!({"active":true,"phase":fixture.phase,"charts":charts,"measurements":fixture.measurements.borrow().clone(),"errors":fixture.errors.borrow().clone()}).to_string())
    })
}

#[wasm_bindgen]
pub fn typography_stop() -> std::result::Result<String, JsValue> {
    let Some(fixture) = FIXTURE.with(|slot| slot.borrow_mut().take()) else {
        return Ok(json!({"active":false,"nodes":0,"handlers":0}).to_string());
    };
    for chart in fixture.charts {
        chart.stop();
    }
    fixture.semantic.app().clear().map_err(js)?;
    let stats = fixture.semantic.app().stats();
    for root in fixture.roots {
        root.remove();
    }
    Ok(json!({"active":false,"nodes":stats.nodes,"handlers":stats.handlers}).to_string())
}
