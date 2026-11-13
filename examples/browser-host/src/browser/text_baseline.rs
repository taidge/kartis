//! Ordinary Glory Text aligned using measured HTML alphabetic baselines.
use super::*;
use glory::{Cage, style::Family};
use kartis::browser::BrowserSemanticTextMeasurer;
use std::{collections::BTreeSet, rc::Weak};

struct Fixture {
    charts: Vec<BrowserChart>,
    roots: Vec<web_sys::Element>,
    web: glory::web::WebApp,
    scene: Cage<Scene>,
    profile: Cage<SemanticTextProfile>,
    hidden: Cage<BTreeSet<String>>,
    stale: Rc<Cell<bool>>,
    measurement: Rc<RefCell<Option<SemanticTextMeasurement>>>,
    observations: Rc<RefCell<Vec<Value>>>,
    errors: Rc<RefCell<Vec<String>>>,
    measurer: Weak<BrowserSemanticTextMeasurer>,
    kind: u8,
    anchor: u8,
    collision: bool,
    reference_shift: f64,
}
thread_local! {static FIXTURE:RefCell<Option<Fixture>>=const{RefCell::new(None)};}

fn source(kind: u8, anchor: u8) -> Result<Scene> {
    if kind > 4 || anchor > 2 {
        return Err(ChartError("invalid baseline fixture configuration".into()));
    }
    let mut scene = ChartConfig {
        width: 360.,
        height: 360.,
        title: "Measured ordinary Text baseline".into(),
        ..Default::default()
    }
    .scene()?;
    scene.typography.default = FontOptions {
        family: Some(if kind == 4 {
            vec![
                Family::Named("Kartis Fixture Typography".into()),
                Family::Named("Kartis Baseline ABeeZee".into()),
            ]
        } else {
            vec![Family::Named(
                if kind >= 2 {
                    "Kartis Baseline ABeeZee"
                } else {
                    "Kartis Fixture Typography"
                }
                .into(),
            )]
        }),
        weight: Some(if kind == 1 { 700. } else { 400. }),
    };
    scene.labels.push(Label {
        x: 180.,
        y: 140.,
        text: match kind {
            3 => "ég",
            4 => "Aég",
            _ => "B",
        }
        .into(),
        size: if kind >= 3 { 70. } else { 100. },
        color: Color(0, 0, 0, 255),
        rotation: 0.,
        anchor: match anchor {
            0 => TextAnchor::Start,
            1 => TextAnchor::Middle,
            _ => TextAnchor::End,
        },
    });
    scene.label_series.insert(0, "alpha".into());
    Ok(scene)
}

fn observation(measured: &SemanticTextMeasurement) -> Value {
    let request = measured.request();
    let metrics = measured.metrics();
    json!({"text":request.text,"size":request.size,"family":request.font.css_family(),"weight":request.font.weight,"revision":request.revision,
        "requestedLineHeight":request.line_height,"advance":metrics.advance,"lineHeight":metrics.line_height,"baseline":metrics.baseline_from_top,
        "ink":metrics.ink_bounds.map(|r|[r.x,r.y,r.width,r.height])})
}

#[wasm_bindgen]
pub fn text_baseline_font(kind: u8) -> Vec<u8> {
    match kind {
        0 => include_bytes!("../../../../tests/fonts/KartisFixtureTypography-Regular.ttf").to_vec(),
        1 => include_bytes!("../../../../tests/fonts/KartisFixtureTypography-Bold.ttf").to_vec(),
        _ => include_bytes!("../../../../tests/fonts/ABeeZee-Regular.ttf").to_vec(),
    }
}

#[wasm_bindgen]
pub fn text_baseline_mount(
    kind: u8,
    anchor: u8,
    collision: bool,
) -> std::result::Result<(), JsValue> {
    text_baseline_stop()?;
    let initial = source(kind, anchor).map_err(js)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let mut roots = Vec::new();
    let mut charts = Vec::new();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("text-baseline-{name}"));
        root.set_attribute("style", "width:360px;position:relative;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                legend: false,
                data_controls: false,
                tooltip: false,
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.flush().map_err(js)?;
        roots.push(root);
        charts.push(chart);
    }
    let shell = document.create_element("div")?;
    shell.set_id("text-baseline-semantic");
    shell.set_attribute("style", "width:360px;position:relative;margin:12px 0")?;
    let root = document.create_element("div")?;
    shell.append_child(&root)?;
    document.body().unwrap().append_child(&shell)?;
    let measurer = Rc::new(BrowserSemanticTextMeasurer::new_in(shell.unchecked_ref()).map_err(js)?);
    let weak_measurer = Rc::downgrade(&measurer);
    let stale = Rc::new(Cell::new(false));
    let stale_callback = stale.clone();
    let measurement = Rc::new(RefCell::new(None::<SemanticTextMeasurement>));
    let last = measurement.clone();
    let observations = Rc::new(RefCell::new(Vec::new()));
    let captured = observations.clone();
    let errors = Rc::new(RefCell::new(Vec::new()));
    let rejects = errors.clone();
    let mut inputs = None;
    let web = glory::web::launch_with(root, |scope| {
        let scene = scope.cage(initial);
        let profile = scope.cage(SemanticTextProfile::default());
        let hidden = scope.cage(BTreeSet::new());
        inputs = Some((scene, profile, hidden));
        let chart = chart(scene)
            .legend(false)
            .data_controls(false)
            .tooltip(false)
            .hidden(hidden)
            .semantic_text_measurer(profile, move |request| {
                if stale_callback.get() {
                    return last
                        .borrow()
                        .clone()
                        .ok_or_else(|| ChartError("no stale baseline fixture measurement".into()));
                }
                let measured = measurer.measure(request)?;
                let mut observations = captured.borrow_mut();
                if observations.len() == 64 {
                    observations.remove(0);
                }
                observations.push(observation(&measured));
                *last.borrow_mut() = Some(measured.clone());
                Ok(measured)
            })
            .on_reject(move |error| rejects.borrow_mut().push(error.0));
        if collision {
            chart.label_collision(LabelCollisionOptions {
                viewport: LayoutRect {
                    x: 0.,
                    y: 0.,
                    width: 360.,
                    height: 360.,
                },
                forbidden: vec![LayoutRect {
                    x: 175.,
                    y: 40.,
                    width: 70.,
                    height: 110.,
                }],
                ..Default::default()
            })
        } else {
            chart
        }
    })
    .map_err(js)?;
    let (scene, profile, hidden) = inputs.unwrap();
    roots.push(shell);
    FIXTURE.with(|slot| {
        *slot.borrow_mut() = Some(Fixture {
            charts,
            roots,
            web,
            scene,
            profile,
            hidden,
            stale,
            measurement,
            observations,
            errors,
            measurer: weak_measurer,
            kind,
            anchor,
            collision,
            reference_shift: 0.,
        })
    });
    Ok(())
}

#[wasm_bindgen]
pub fn text_baseline_phase(kind: u8, anchor: u8) -> std::result::Result<(), JsValue> {
    let candidate = source(kind, anchor).map_err(js)?;
    FIXTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let f = slot
            .as_mut()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        for chart in &f.charts {
            chart.set_scene(candidate.clone()).map_err(js)?;
            chart.flush().map_err(js)?;
        }
        f.scene.revise(|scene| *scene = candidate);
        f.web.app().flush().map_err(js)?;
        f.kind = kind;
        f.anchor = anchor;
        f.reference_shift = 0.;
        Ok(())
    })
}

#[wasm_bindgen]
pub fn text_baseline_revision(revision: u64, stale: bool) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let f = slot
            .as_mut()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        f.stale.set(stale);
        f.profile.revise(|profile| profile.revision = revision);
        f.web.app().flush().map_err(js)
    })
}

#[wasm_bindgen]
pub fn text_baseline_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let f = slot
            .as_ref()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        let set = if hidden {
            BTreeSet::from(["alpha".into()])
        } else {
            BTreeSet::new()
        };
        for chart in &f.charts {
            chart.handle().set_hidden(set.clone()).map_err(js)?;
            chart.flush().map_err(js)?;
        }
        f.hidden.revise(|current| *current = set);
        f.web.app().flush().map_err(js)
    })
}

#[wasm_bindgen]
pub fn text_baseline_reference_shift(dy: f64) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let mut slot = slot.borrow_mut();
        let f = slot
            .as_mut()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        let mut scene = source(f.kind, f.anchor).map_err(js)?;
        scene.labels[0].y += dy;
        for chart in &f.charts {
            chart.set_scene(scene.clone()).map_err(js)?;
            chart.flush().map_err(js)?;
        }
        f.reference_shift = dy;
        Ok(())
    })
}

#[wasm_bindgen]
pub fn text_baseline_draw(
    canvas: web_sys::HtmlCanvasElement,
    ratio: f64,
) -> std::result::Result<(), JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let f = slot
            .as_ref()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        let mut scene = source(f.kind, f.anchor).map_err(js)?;
        scene.labels[0].y += f.reference_shift;
        kartis::render::draw_canvas(&scene, &canvas, ratio).map_err(js)
    })
}

#[wasm_bindgen]
pub fn text_baseline_probe(text: &str, line_height: f64) -> std::result::Result<String, JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let f = slot
            .as_ref()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        let measurer = f
            .measurer
            .upgrade()
            .ok_or_else(|| js("baseline measurer released"))?;
        let scene = source(f.kind, f.anchor).map_err(js)?;
        let mut request = SemanticTextProfile {
            line_height,
            revision: 99,
        }
        .request(&scene.labels[0], &scene.resolved_label_font(0))
        .map_err(js)?;
        request.text = text.into();
        let measured = measurer.measure(&request).map_err(js)?;
        let over_budget = measurer
            .measure_many(&vec![request; MAX_MARKS + 1])
            .is_err();
        Ok(
            json!({"measurement":observation(&measured),"overBudgetRejected":over_budget})
                .to_string(),
        )
    })
}

#[wasm_bindgen]
pub fn text_baseline_measure_in(
    parent: web_sys::HtmlElement,
    text: &str,
) -> std::result::Result<String, JsValue> {
    FIXTURE.with(|slot| {
        let slot = slot.borrow();
        let f = slot
            .as_ref()
            .ok_or_else(|| js("baseline fixture stopped"))?;
        let scene = source(f.kind, f.anchor).map_err(js)?;
        let mut request = SemanticTextProfile {
            line_height: 1.,
            revision: 99,
        }
        .request(&scene.labels[0], &scene.resolved_label_font(0))
        .map_err(js)?;
        request.text = text.into();
        let measurer = BrowserSemanticTextMeasurer::new_in(&parent).map_err(js)?;
        let measured = measurer.measure(&request).map_err(js)?;
        Ok(observation(&measured).to_string())
    })
}

#[wasm_bindgen]
pub fn text_baseline_snapshot() -> String {
    FIXTURE.with(|slot| { let slot = slot.borrow(); let Some(f) = slot.as_ref() else {return json!({"active":false}).to_string()};
        json!({"active":true,"kind":f.kind,"anchor":f.anchor,"collision":f.collision,"errors":f.errors.borrow().clone(),
            "measurement":f.measurement.borrow().as_ref().map(observation),"observations":f.observations.borrow().clone(),"nodes":f.web.app().stats().nodes}).to_string()
    })
}

#[wasm_bindgen]
pub fn text_baseline_stop() -> std::result::Result<String, JsValue> {
    let Some(f) = FIXTURE.with(|slot| slot.borrow_mut().take()) else {
        return Ok(
            json!({"active":false,"nodes":0,"handlers":0,"measurerReleased":true}).to_string(),
        );
    };
    for chart in f.charts {
        chart.stop();
    }
    f.web.app().clear().map_err(js)?;
    let stats = f.web.app().stats();
    let released = f.measurer.upgrade().is_none();
    for root in f.roots {
        root.remove();
    }
    Ok(json!({"active":false,"nodes":stats.nodes,"handlers":stats.handlers,"measurerReleased":released}).to_string())
}
