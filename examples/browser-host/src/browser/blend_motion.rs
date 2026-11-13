//! Actual retained surfaces driven by the host frame callback.
use super::*;
use kartis::browser::{BrowserHighlight, MotionPreference};
use web_sys::Element;

struct Fixture {
    chart: BrowserChart,
    root: Element,
    name: &'static str,
    phase: Rc<Cell<u8>>,
    errors: Rc<RefCell<Vec<String>>>,
}

thread_local! { static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }

fn rect(x: f64, y: f64, width: f64, height: f64) -> Geometry {
    Geometry::Rect {
        x,
        y,
        width,
        height,
    }
}

fn mode(phase: u8) -> BlendMode {
    match phase {
        0 | 1 => BlendMode::Multiply,
        2 => BlendMode::Screen,
        3 => BlendMode::Difference,
        _ => BlendMode::Normal,
    }
}

fn source(w: f64, h: f64, phase: u8, budget: bool) -> Result<(Scene, InteractionBinding)> {
    let mut scene = ChartConfig {
        width: w,
        height: h,
        margin: Margin {
            left: 0.,
            right: 0.,
            top: 0.,
            bottom: 0.,
        },
        title: "Animated blending".into(),
        ..Default::default()
    }
    .scene()?;
    scene.background = Color(160, 120, 80, 255);
    // The frame keeps the tested fill point and the hole fixed across phases.
    let x = if phase == 0 { 16. } else { 24. };
    let mut mark = Mark::new("paint", rect(x, 20., 55., 40.), Color(200, 100, 50, 255));
    mark.value = 12. + 7. * f64::from(phase);
    mark.label = format!("Paint phase {phase}");
    mark.series = "ink".into();
    mark.clip_path = Some(Box::new(ClipPath {
        geometry: Geometry::Path(vec![
            PathCommand::Move(GraphicPoint::new(10., 15.)),
            PathCommand::Line(GraphicPoint::new(90., 15.)),
            PathCommand::Line(GraphicPoint::new(90., 65.)),
            PathCommand::Line(GraphicPoint::new(10., 65.)),
            PathCommand::Close,
            PathCommand::Move(GraphicPoint::new(46., 34.)),
            PathCommand::Line(GraphicPoint::new(54., 34.)),
            PathCommand::Line(GraphicPoint::new(54., 46.)),
            PathCommand::Line(GraphicPoint::new(46., 46.)),
            PathCommand::Close,
        ]),
        rule: ClipRule::EvenOdd,
    }));
    let site = InteractionSite {
        mark_id: mark.id.clone(),
        series: mark.series.clone(),
        label: mark.label.clone(),
        value: Some(mark.value),
        formatted_value: Some(format!("Phase {phase}")),
        color: Color(200, 100, 50, 255),
        position: [x + 27.5, 40.],
    };
    scene.marks.push(mark);
    if budget {
        // Invisible in the ordinary fixture. On the large candidate's
        // viewport this retiring geometry needs >16M scratch pixels.
        let mut mark = Mark::new(
            "budget",
            rect(160., 100., 5000., 4000.),
            Color(55, 65, 75, 255),
        );
        mark.label.clear();
        scene.marks.push(mark);
    }
    scene.legend.push(LegendItem {
        id: "ink".into(),
        label: "Ink".into(),
        color: site.color,
    });
    Ok((
        scene,
        InteractionBinding {
            sites: vec![site],
            ..Default::default()
        },
    ))
}

fn profile(scene: &Scene, phase: u8, budget: bool) -> Result<MarkBlends> {
    let mut modes = vec![("paint", mode(phase))];
    if budget {
        modes.push(("budget", BlendMode::Multiply));
    }
    MarkBlends::new(scene, modes)
}

fn mount(
    document: &web_sys::Document,
    row: &Element,
    name: &'static str,
    renderer: BrowserRenderer,
    kind: u8,
    reduced: bool,
) -> std::result::Result<Fixture, JsValue> {
    let root = document.create_element("div")?;
    root.set_id(&format!("blend-motion-{name}"));
    root.set_attribute("style", "width:120px;height:80px;flex:none")?;
    row.append_child(&root)?;
    let (scene, binding) = source(120., 80., 0, true).map_err(js)?;
    let modes = profile(&scene, 0, true).map_err(js)?;
    let chart = BrowserChart::mount_with_blends(
        root.clone(),
        scene,
        BrowserOptions {
            renderer,
            interaction_data: Some(binding),
            highlight: BrowserHighlight::Controlled,
            tooltip: false,
            motion: if reduced {
                MotionPreference::Reduced
            } else {
                MotionPreference::Animate
            },
            animation: if kind == 0 {
                BrowserAnimation::Scene
            } else {
                BrowserAnimation::Properties(Default::default())
            },
            ..Default::default()
        },
        modes,
    )
    .map_err(js)?;
    let errors = Rc::new(RefCell::new(Vec::new()));
    chart.on_event({
        let errors = errors.clone();
        move |event| {
            if let BrowserEvent::Error(error) = event {
                errors.borrow_mut().push(error.0);
            }
        }
    });
    let phase = Rc::new(Cell::new(0));
    chart.on_resize_with_interaction({
        let phase = phase.clone();
        move |width, height| {
            let (scene, binding) = source(width, height, phase.get(), true)?;
            Ok(SceneInteraction {
                scene,
                interaction: Some(binding),
                plot: None,
            })
        }
    });
    Ok(Fixture {
        chart,
        root,
        name,
        phase,
        errors,
    })
}

#[wasm_bindgen]
pub fn blend_motion_mount(kind: u8, reduced: bool) -> std::result::Result<(), JsValue> {
    if kind > 1 {
        return Err(js("unknown blend motion solver"));
    }
    blend_motion_stop();
    let document = web_sys::window()
        .and_then(|w| w.document())
        .ok_or_else(|| js("missing document"))?;
    let row = document.create_element("div")?;
    row.set_id("blend-motion-row");
    row.set_attribute("style", "display:flex;align-items:flex-start;gap:48px")?;
    document
        .body()
        .ok_or_else(|| js("missing body"))?
        .append_child(&row)?;
    let entries = (|| {
        Ok::<_, JsValue>(vec![
            mount(&document, &row, "svg", BrowserRenderer::Svg, kind, reduced)?,
            mount(
                &document,
                &row,
                "canvas",
                BrowserRenderer::Canvas,
                kind,
                reduced,
            )?,
        ])
    })();
    match entries {
        Ok(entries) => FIXTURES.with(|fixtures| {
            *fixtures.borrow_mut() = entries;
            Ok(())
        }),
        Err(error) => {
            row.remove();
            Err(error)
        }
    }
}

#[wasm_bindgen]
pub fn blend_motion_phase(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 4 {
        return Err(js("unknown blend motion phase"));
    }
    FIXTURES.with(|fixtures| {
        for fixture in fixtures.borrow().iter() {
            let current = fixture.chart.snapshot().map_err(js)?;
            let (scene, binding) =
                source(current.width, current.height, phase, true).map_err(js)?;
            let modes = profile(&scene, phase, true).map_err(js)?;
            fixture
                .chart
                .animate_to_with_interaction_and_blends(scene, binding, modes)
                .map_err(js)?;
            fixture.phase.set(phase);
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_motion_rejected() -> std::result::Result<String, JsValue> {
    FIXTURES.with(|fixtures| {
        let list = fixtures.borrow();
        let canvas = list
            .iter()
            .find(|f| f.name == "canvas")
            .ok_or_else(|| js("missing fixture"))?;
        let (scene, binding) = source(5000., 4000., 2, false).map_err(js)?;
        let modes = profile(&scene, 2, false).map_err(js)?;
        let before = canvas.chart.snapshot().map_err(js)?;
        let error = canvas
            .chart
            .animate_to_with_interaction_and_blends(scene, binding, modes)
            .err()
            .map(|e| e.0);
        let after = canvas.chart.snapshot().map_err(js)?;
        Ok(json!({"error":error,"atomic":before == after}).to_string())
    })
}

#[wasm_bindgen]
pub fn blend_motion_viewport() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|fixtures| {
        for fixture in fixtures.borrow().iter() {
            let state = fixture.chart.snapshot().map_err(js)?;
            let viewport = DomainViewport::new(
                ViewportAxes {
                    x: ViewportAxis::new([0., state.width], ScaleKind::Linear),
                    left: Some(ViewportAxis::new([0., state.height], ScaleKind::Linear)),
                    right: None,
                },
                ClipRect {
                    x: 0.,
                    y: 0.,
                    width: state.width,
                    height: state.height,
                },
                ViewportLimits::default(),
            )
            .map_err(js)?;
            let phase = fixture.phase.clone();
            fixture
                .chart
                .on_viewport_with_interaction(viewport, ViewportSelection::Both, move |request| {
                    let (mut scene, mut binding) =
                        source(request.width, request.height, phase.get(), true)?;
                    if let Geometry::Rect { x, width, .. } = &mut scene.marks[0].geometry {
                        let [start, end] = request.domains.x;
                        *x = (*x - start) / (end - start) * request.width;
                        binding.sites[0].position[0] = *x + *width / 2.;
                    }
                    Ok(SceneInteraction {
                        scene,
                        interaction: Some(binding),
                        plot: None,
                    })
                })
                .map_err(js)?;
            fixture
                .chart
                .handle()
                .request_viewport(ViewportRequest::Zoom {
                    factor: 1.2,
                    anchor: (60., 40.),
                    axes: ViewportSelection::X,
                })
                .map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn blend_motion_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|fixtures| {
        let list = fixtures.borrow();
        let Some(fixture) = list.iter().find(|f| f.name == name) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = fixture.chart.snapshot().map_err(js)?;
        Ok(json!({
            "active":state.active,
            "width":state.width,
            "height":state.height,
            "animating":state.animating,
            "pending":state.pending,
            "revision":state.revision,
            "hovered":state.hovered,
            "selected":state.selected,
            "relayout":state.domains.is_some(),
            "values":state.interaction.map(|data| data.rows.iter().map(|row| row.value).collect::<Vec<_>>()),
            "errors":*fixture.errors.borrow(),
        }).to_string())
    })
}

#[wasm_bindgen]
pub fn blend_motion_stop() {
    FIXTURES.with(|fixtures| {
        for fixture in fixtures.borrow_mut().drain(..) {
            fixture.chart.stop();
            fixture.root.remove();
        }
    });
    if let Some(row) = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id("blend-motion-row"))
    {
        row.remove();
    }
}
