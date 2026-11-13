//! Independent identical SVG owners must never share document fragment IDs.
use super::*;
struct Fixture {
    chart: Option<BrowserChart>,
    handle: BrowserHandle,
    root: web_sys::Element,
    name: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static ISOLATION:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn source(phase: u8) -> Result<Scene> {
    if phase > 2 {
        return Err(ChartError("invalid SVG isolation phase".into()));
    }
    let mut s = ChartConfig {
        width: 600.,
        height: 260.,
        title: "Independent SVG definitions".into(),
        ..Default::default()
    }
    .scene()?;
    let mut linear = Mark::new(
        "linear",
        Geometry::Rect {
            x: 20.,
            y: 25.,
            width: 150.,
            height: 100.,
        },
        Color(80, 140, 180, 255),
    );
    linear.fill_paint = Some(FillPaint::Linear {
        start: [20., 25.],
        end: [170., 25.],
        stops: if phase == 1 {
            vec![
                GradientStop::new(0., Color(20, 180, 80, 255)),
                GradientStop::new(1., Color(180, 40, 200, 255)),
            ]
        } else {
            vec![
                GradientStop::new(0., Color(240, 40, 60, 255)),
                GradientStop::new(1., Color(30, 90, 230, 255)),
            ]
        },
    });
    let mut radial = Mark::new(
        "radial",
        Geometry::Circle {
            x: 270.,
            y: 75.,
            radius: 50.,
        },
        Color(180, 90, 140, 255),
    );
    radial.fill_paint = Some(FillPaint::Radial {
        center: [270., 75.],
        radius: 50.,
        stops: vec![
            GradientStop::new(0., Color(255, 230, 110, 255)),
            GradientStop::new(1., Color(110, 35, 100, 255)),
        ],
    });
    let mut pattern = Mark::new(
        "pattern",
        Geometry::Rect {
            x: 380.,
            y: 25.,
            width: 150.,
            height: 100.,
        },
        Color(90, 150, 100, 255),
    );
    pattern.fill_paint = Some(FillPaint::Pattern(PatternFill {
        shape: if phase == 0 {
            PatternShape::Squares {
                size: 6.,
                stagger: false,
            }
        } else {
            PatternShape::Dots {
                diameter: 5.,
                stagger: true,
            }
        },
        spacing: 12.,
        foreground: Color(25, 135, 70, 255),
        background: Color(240, 250, 245, 255),
        ..Default::default()
    }));
    let mut masked = Mark::new(
        "masked",
        Geometry::Rect {
            x: 25.,
            y: 150.,
            width: 150.,
            height: 75.,
        },
        Color(20, 130, 180, 255),
    );
    masked.fill_paint = linear.fill_paint.clone();
    masked.clip_path = Some(Box::new(ClipPath::circle(100., 188., 35.)));
    s.marks = vec![linear, radial, pattern, masked];
    for m in &mut s.marks {
        m.series = "data".into();
        m.label = format!("{} <safe>", m.id);
        m.value = if phase == 1 { 42. } else { 10. };
        if phase == 2 {
            m.fill_paint = None;
            m.clip_path = None;
        }
    }
    s.labels.push(Label {
        x: 365.,
        y: 197.,
        text: "Masked label <safe>".into(),
        color: Color(60, 30, 120, 255),
        size: 24.,
        anchor: TextAnchor::Start,
        rotation: 0.,
    });
    if phase != 2 {
        s.label_clips.insert(
            0,
            ClipPath {
                geometry: Geometry::Rect {
                    x: 390.,
                    y: 168.,
                    width: 90.,
                    height: 40.,
                },
                rule: ClipRule::NonZero,
            },
        );
    }
    s.legend.push(LegendItem {
        id: "data".into(),
        label: "Data".into(),
        color: Color(80, 140, 180, 255),
    });
    s.validate()?;
    Ok(s)
}
fn mount(
    root: web_sys::Element,
    scene: Scene,
    renderer: BrowserRenderer,
    events: Rc<RefCell<Vec<Value>>>,
) -> Result<BrowserChart> {
    let chart = BrowserChart::mount(
        root,
        scene,
        BrowserOptions {
            renderer,
            height: Some(260.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            tooltip_options: TooltipOptions {
                lines: vec![vec![TooltipPart::Label]],
                ..Default::default()
            },
            interaction: surface::SurfaceOptions {
                pan: false,
                zoom: false,
                pinch: false,
                brush: false,
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    chart.on_event(move |event| {
        let mut e = events.borrow_mut();
        if e.len() == 32 {
            e.remove(0);
        }
        e.push(event_value(event));
    });
    chart.flush()?;
    Ok(chart)
}
#[wasm_bindgen]
pub fn svg_isolation_mount() -> std::result::Result<(), JsValue> {
    let scene = source(0).map_err(js)?;
    svg_isolation_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("first", BrowserRenderer::Svg),
        ("second", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("isolation-{name}"));
        root.set_attribute(
            "style",
            "width:600px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let chart = match mount(root.clone(), scene.clone(), renderer, events.clone()) {
            Ok(c) => c,
            Err(error) => {
                root.remove();
                svg_isolation_stop();
                return Err(js(error));
            }
        };
        let handle = chart.handle();
        ISOLATION.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart: Some(chart),
                handle,
                root,
                name: name.into(),
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn svg_isolation_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase).map_err(js)?;
    ISOLATION.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            if f.handle.is_active() {
                f.handle.set_scene(scene.clone()).map_err(js)?;
            }
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn svg_isolation_first(action: u8) -> std::result::Result<(), JsValue> {
    ISOLATION.with(|slot| {
        let mut entries = slot.borrow_mut();
        let f = entries
            .iter_mut()
            .find(|f| f.name == "first")
            .ok_or_else(|| js("missing first SVG owner"))?;
        match action {
            0 => f.root.set_attribute(
                "style",
                "display:none;width:600px;position:relative;margin:12px 0",
            )?,
            1 => f.root.set_attribute(
                "style",
                "width:600px;max-width:none;position:relative;margin:12px 0",
            )?,
            2 => f.handle.stop(),
            3 => {
                f.chart.take();
            }
            4 => {
                f.chart.take();
                let chart = mount(
                    f.root.clone(),
                    source(f.phase).map_err(js)?,
                    BrowserRenderer::Svg,
                    f.events.clone(),
                )
                .map_err(js)?;
                f.handle = chart.handle();
                f.chart = Some(chart);
            }
            _ => return Err(js("invalid first-owner action")),
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn svg_isolation_flush() -> std::result::Result<(), JsValue> {
    ISOLATION.with(|slot| {
        for f in slot.borrow().iter() {
            if f.handle.is_active() {
                f.handle.flush().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn svg_isolation_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    ISOLATION.with(|slot|{
    let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.name==name)else{return Ok(json!({"active":false}).to_string());};if !f.handle.is_active(){return Ok(json!({"active":false}).to_string());}let s=f.handle.snapshot().map_err(js)?;
    Ok(json!({"active":s.active,"phase":f.phase,"revision":s.revision,"pending":s.pending,"selected":s.selected,"hovered":s.hovered,"target":{"id":"linear","x":80.,"y":70.,"value":if f.phase==1{42.}else{10.}},"events":f.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn svg_isolation_rejected() -> bool {
    ISOLATION.with(|slot| {
        slot.borrow()
            .iter()
            .filter(|f| f.handle.is_active())
            .all(|f| {
                let mut scene = source(f.phase).unwrap();
                scene.marks[0].opacity = f64::NAN;
                f.handle.set_scene(scene).is_err()
            })
    })
}
#[wasm_bindgen]
pub fn svg_isolation_stop() {
    let entries = ISOLATION.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.handle.stop();
        drop(f.chart);
        f.root.remove();
    }
}
