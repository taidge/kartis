//! Dedicated real BrowserChart blend fixture, shared by the portable oracle.
use super::*;

struct Fixture {
    chart: BrowserChart,
    root: Element,
    name: &'static str,
    phase: Rc<Cell<u8>>,
    mode: Cell<BlendMode>,
    errors: Rc<RefCell<Vec<String>>>,
}

use web_sys::Element;
thread_local! { static BLENDS: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }

const MODES: [BlendMode; 16] = [
    BlendMode::Normal,
    BlendMode::Multiply,
    BlendMode::Screen,
    BlendMode::Overlay,
    BlendMode::Darken,
    BlendMode::Lighten,
    BlendMode::ColorDodge,
    BlendMode::ColorBurn,
    BlendMode::HardLight,
    BlendMode::SoftLight,
    BlendMode::Difference,
    BlendMode::Exclusion,
    BlendMode::Hue,
    BlendMode::Saturation,
    BlendMode::Color,
    BlendMode::Luminosity,
];

fn rect(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Rect {
        x,
        y,
        width,
        height,
    }
}

fn source(width: f64, height: f64, phase: u8) -> Result<Scene> {
    let mut scene = ChartConfig {
        width,
        height,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        title: "Isolated mark blending".into(),
        ..Default::default()
    }
    .scene()?;
    let mut mixed = Mark::new("mixed", rect(34., 20., 42., 40.), Color(200, 100, 50, 128));
    mixed.series = "mixed-series".into();
    mixed.label = "Mixed fill and stroke".into();
    mixed.value = 12.;
    scene.background = Color(160, 120, 80, 255);
    if phase == 1 {
        scene.background = Color(0, 0, 0, 0);
        scene.marks.push(Mark::new(
            "back",
            rect(20., 20., 40., 40.),
            Color(160, 120, 80, 128),
        ));
        mixed.geometry = rect(40., 20., 40., 40.);
    } else {
        mixed.opacity = 0.5;
        mixed.stroke = Some(GraphicStroke {
            color: Color(40, 200, 240, 128),
            width: 12.,
            cap: StrokeCap::Butt,
        });
        mixed.clip = Some(ClipRect {
            x: 30.,
            y: 8.,
            width: 60.,
            height: 64.,
        });
        mixed.clip_path = Some(Box::new(ClipPath {
            geometry: Geometry::Path(vec![
                PathCommand::Move(GraphicPoint::new(22., 10.)),
                PathCommand::Line(GraphicPoint::new(95., 10.)),
                PathCommand::Line(GraphicPoint::new(95., 70.)),
                PathCommand::Line(GraphicPoint::new(22., 70.)),
                PathCommand::Close,
                PathCommand::Move(GraphicPoint::new(33., 36.)),
                PathCommand::Line(GraphicPoint::new(38., 36.)),
                PathCommand::Line(GraphicPoint::new(38., 44.)),
                PathCommand::Line(GraphicPoint::new(33., 44.)),
                PathCommand::Close,
            ]),
            rule: ClipRule::EvenOdd,
        }));
    }
    scene.marks.push(mixed);
    scene.marks.push(Mark::new(
        "normal",
        rect(98., 20., 10., 10.),
        Color(22, 41, 80, 255),
    ));
    scene.legend.push(LegendItem {
        id: "mixed-series".into(),
        label: "Mixed".into(),
        color: Color(200, 100, 50, 128),
    });
    scene.validate()?;
    Ok(scene)
}

fn update(f: &Fixture, phase: u8, mode: BlendMode) -> Result<()> {
    let state = f.chart.snapshot()?;
    let scene = source(state.width, state.height, phase)?;
    let modes = MarkBlends::new(&scene, [("mixed", mode)])?;
    f.chart.set_layout_with_blends(scene.into(), modes)?;
    f.phase.set(phase);
    f.mode.set(mode);
    Ok(())
}

fn mount(
    document: &web_sys::Document,
    name: &'static str,
    renderer: BrowserRenderer,
) -> std::result::Result<Fixture, JsValue> {
    let root = document.create_element("div")?;
    root.set_id(&format!("blend-{name}"));
    root.set_attribute(
        "style",
        "width:120px;height:80px;background:#5668d0;margin:8px",
    )?;
    let scene = source(120., 80., 0).map_err(js)?;
    let profile = MarkBlends::new(&scene, [("mixed", BlendMode::Multiply)]).map_err(js)?;
    document
        .body()
        .ok_or_else(|| js("missing body"))?
        .append_child(&root)?;
    let chart = match BrowserChart::mount_with_blends(
        root.clone(),
        scene,
        BrowserOptions {
            renderer,
            tooltip: false,
            highlight: kartis::browser::BrowserHighlight::Controlled,
            ..Default::default()
        },
        profile,
    ) {
        Ok(chart) => chart,
        Err(error) => {
            root.remove();
            return Err(js(error));
        }
    };
    let phase = Rc::new(Cell::new(0));
    let resize_phase = phase.clone();
    chart.on_resize(move |w, h| source(w, h, resize_phase.get()));
    let errors = Rc::new(RefCell::new(Vec::new()));
    let event_errors = errors.clone();
    chart.on_event(move |event| {
        if let BrowserEvent::Error(error) = event {
            event_errors.borrow_mut().push(error.0);
        }
    });
    Ok(Fixture {
        chart,
        root,
        name,
        phase,
        mode: Cell::new(BlendMode::Multiply),
        errors,
    })
}

#[wasm_bindgen]
pub fn blend_mount() -> std::result::Result<(), JsValue> {
    blend_stop();
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| js("missing document"))?;
    let svg = mount(&document, "svg", BrowserRenderer::Svg)?;
    let canvas = match mount(&document, "canvas", BrowserRenderer::Canvas) {
        Ok(canvas) => canvas,
        Err(error) => {
            svg.chart.stop();
            svg.root.remove();
            return Err(error);
        }
    };
    BLENDS.with(|state| *state.borrow_mut() = vec![svg, canvas]);
    Ok(())
}

#[wasm_bindgen]
pub fn blend_mode(index: usize) -> std::result::Result<(), JsValue> {
    let mode = *MODES
        .get(index)
        .ok_or_else(|| js("unknown blend fixture mode"))?;
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            update(f, f.phase.get(), mode).map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 2 {
        return Err(js("unknown blend fixture phase"));
    }
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            if phase == 2 {
                let state = f.chart.snapshot().map_err(js)?;
                f.chart
                    .set_layout(source(state.width, state.height, 0).map_err(js)?.into())
                    .map_err(js)?;
                f.phase.set(0);
                f.mode.set(BlendMode::Normal);
            } else {
                update(f, phase, f.mode.get()).map_err(js)?;
            }
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_zoom() -> std::result::Result<(), JsValue> {
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            f.chart
                .handle()
                .dispatch(SurfaceInput::Wheel {
                    point: (59., 45.),
                    delta: -175.,
                })
                .map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_viewport() -> std::result::Result<(), JsValue> {
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            let s = f.chart.snapshot().map_err(js)?;
            let viewport = DomainViewport::new(
                ViewportAxes {
                    x: ViewportAxis::new([0., s.width], ScaleKind::Linear),
                    left: Some(ViewportAxis::new([0., s.height], ScaleKind::Linear)),
                    right: None,
                },
                ClipRect {
                    x: 0.,
                    y: 0.,
                    width: s.width,
                    height: s.height,
                },
                ViewportLimits::default(),
            )
            .map_err(js)?;
            let phase = f.phase.clone();
            f.chart
                .on_viewport(viewport, ViewportSelection::Both, move |request| {
                    source(request.width, request.height, phase.get())
                })
                .map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_reject() -> std::result::Result<String, JsValue> {
    let mut stale = true;
    let mut animation = true;
    let mut budget = false;
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            let state = f.chart.snapshot().map_err(js)?;
            let scene = source(state.width, state.height, 0).map_err(js)?;
            let modes = MarkBlends::new(&scene, [("mixed", BlendMode::Multiply)]).map_err(js)?;
            let mut wrong = scene.clone();
            wrong.marks.retain(|mark| mark.id != "mixed");
            stale &= f.chart.set_layout_with_blends(wrong.into(), modes).is_err();
            animation &= f.chart.animate_to(scene).is_err();
            if f.name == "canvas" {
                let mut huge = ChartConfig {
                    width: 5000.,
                    height: 4000.,
                    ..Default::default()
                }
                .scene()
                .map_err(js)?;
                huge.marks.push(Mark::new(
                    "huge",
                    rect(0., 0., 5000., 4000.),
                    Color(20, 40, 50, 255),
                ));
                let modes = MarkBlends::new(&huge, [("huge", BlendMode::Screen)]).map_err(js)?;
                budget = f.chart.set_layout_with_blends(huge.into(), modes).is_err();
            }
        }
        Ok::<_, JsValue>(())
    })?;
    Ok(json!({"stale":stale,"animation":animation,"budget":budget}).to_string())
}

#[wasm_bindgen]
pub fn blend_flush() -> std::result::Result<(), JsValue> {
    BLENDS.with(|entries| {
        for f in entries.borrow().iter() {
            f.chart.handle().flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    BLENDS.with(|entries| {
        let entries = entries.borrow();
        let Some(f) = entries.iter().find(|f| f.name == name) else { return Ok(json!({"active":false}).to_string()); };
        let s = f.chart.snapshot().map_err(js)?;
        Ok(json!({"active":s.active,"revision":s.revision,"pending":s.pending,"width":s.width,"height":s.height,
            "view":[s.view.scale,s.view.translate_x,s.view.translate_y],"relayout":s.domains.is_some(),
            "hovered":s.hovered,"hidden":s.hidden,"errors":f.errors.borrow().clone()}).to_string())
    })
}

#[wasm_bindgen]
pub fn blend_static(index: usize, phase: u8) -> std::result::Result<String, JsValue> {
    let mode = *MODES
        .get(index)
        .ok_or_else(|| js("unknown blend fixture mode"))?;
    let scene = source(120., 80., phase).map_err(js)?;
    let profile = MarkBlends::new(&scene, [("mixed", mode)]).map_err(js)?;
    let svg =
        render::to_svg_with_namespace_and_blends(&scene, "blend-static", &profile).map_err(js)?;
    let script =
        render::to_canvas_script_with_blends(&scene, "blend-script", &profile).map_err(js)?;
    Ok(json!({"svg":svg,"script":script}).to_string())
}

#[wasm_bindgen]
pub fn blend_stop() {
    BLENDS.with(|entries| {
        for f in entries.borrow_mut().drain(..) {
            f.chart.stop();
            f.root.remove();
        }
    });
}
