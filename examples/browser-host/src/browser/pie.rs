//! Real rounded Pie controllers; raw IDs and owned leader labels survive updates.
use super::*;
struct PieFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static PIES:RefCell<Vec<PieFixture>>=const{RefCell::new(Vec::new())};}
fn options(phase: u8) -> Result<PieOptions> {
    if phase > 2 {
        return Err(ChartError("pie fixture phase must be 0..2".into()));
    }
    let mut o = PieOptions {
        inner_radius: if phase == 2 { 0.65 } else { 0.45 },
        pad_angle: 0.03,
        start_angle: if phase == 0 { 0. } else { 0.2 },
        ..Default::default()
    };
    o.end_angle = o.start_angle
        + if phase == 2 {
            -std::f64::consts::TAU
        } else {
            std::f64::consts::TAU
        };
    o.style.corner_radius = if phase == 1 { 24. } else { 12. };
    o.style.border = Some(GraphicStroke {
        color: Color(30, 50, 80, 255),
        width: 3.,
        cap: StrokeCap::Round,
    });
    o.style.fill_opacity = if phase == 1 { 0.45 } else { 1. };
    if phase == 1 {
        o.style.active_ids.insert("alpha".into());
        o.style.active_inner_radius_offset = 10.;
        o.style.active_outer_radius_offset = 12.;
    }
    o.style.arc_labels.legacy_fit = false;
    o.style.arc_labels.skip_angle = if phase == 2 { 1.8 } else { 0. };
    o.style.arc_labels.text = match phase {
        0 => PieLabelText::Value(ValueFormat::Fixed(0)),
        1 => PieLabelText::Percent(ValueFormat::Percent(0)),
        _ => PieLabelText::Id,
    };
    o.style.arc_labels.style.color = Some(Color(20, 30, 50, 255));
    o.style.arc_labels.style.size = Some(15.);
    o.style.link_labels.enabled = true;
    o.style.link_labels.text = PieLabelText::Label;
    o.style.link_labels.color = PieLinkColor::Slice;
    o.style.link_labels.thickness = 2.;
    o.style.link_labels.diagonal_length = 15.;
    o.style.link_labels.straight_length = 20.;
    o.style.link_labels.style.size = Some(14.);
    Ok(o)
}
fn data(phase: u8) -> Vec<RadialDatum> {
    [
        ("alpha", if phase == 1 { 40. } else { 25. }, "Alpha <A>"),
        ("beta", if phase == 1 { 30. } else { 35. }, "Beta"),
        ("gamma", if phase == 1 { 30. } else { 40. }, "Gamma"),
    ]
    .into_iter()
    .map(|(id, value, label)| {
        let mut d = RadialDatum::new(id, value);
        d.label = label.into();
        d
    })
    .collect()
}
fn source(phase: u8) -> Result<Scene> {
    let mut s = kartis::pie(
        &ChartConfig {
            width: 640.,
            height: 360.,
            margin: Margin {
                left: 120.,
                right: 120.,
                top: 60.,
                bottom: 60.,
            },
            title: "Rounded pie".into(),
            ..Default::default()
        },
        &data(phase),
        &options(phase)?,
    )?;
    if phase == 0 {
        s.marks
            .iter_mut()
            .find(|m| m.id == "alpha")
            .unwrap()
            .fill_paint = Some(FillPaint::Linear {
            start: [320., 60.],
            end: [440., 180.],
            stops: vec![
                GradientStop {
                    offset: 0.,
                    color: Color(40, 120, 200, 255),
                },
                GradientStop {
                    offset: 1.,
                    color: Color(60, 180, 130, 255),
                },
            ],
        });
    }
    s.validate()?;
    Ok(s)
}
#[wasm_bindgen]
pub fn pie_mount() -> std::result::Result<(), JsValue> {
    let scene = source(0).map_err(js)?;
    pie_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("pie-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
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
        ) {
            Ok(chart) => chart,
            Err(e) => {
                root.remove();
                pie_stop();
                return Err(js(e));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut events = capture.borrow_mut();
            if events.len() == 32 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        if let Err(e) = chart.flush() {
            chart.stop();
            root.remove();
            pie_stop();
            return Err(js(e));
        }
        PIES.with(|slot| {
            slot.borrow_mut().push(PieFixture {
                chart,
                root,
                renderer: name.into(),
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn pie_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase).map_err(js)?;
    PIES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn pie_flush() -> std::result::Result<(), JsValue> {
    PIES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn pie_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    PIES.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let state=f.chart.snapshot().map_err(js)?;let scene=source(f.phase).map_err(js)?;let o=options(f.phase).map_err(js)?;let fraction=if f.phase==1 {0.4}else{0.25};let direction=(o.end_angle-o.start_angle).signum();let angle=o.start_angle+direction*(o.pad_angle/2.+(std::f64::consts::TAU-3.*o.pad_angle)*fraction/2.);let radius=if f.phase==1 {88.}else{120.*(1.+o.inner_radius)/2.};let x=320.+radius*angle.sin();let y=180.-radius*angle.cos();let leader=scene.marks.iter().find(|m|m.series=="alpha"&&m.label.is_empty()).unwrap();let p=leader.geometry.commands();let PathCommand::Line(end)=p[2]else{unreachable!()};Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"phase":f.phase,"sceneHit":scene.hit_test(x,y).map(|m|m.id.clone()),"target":{"id":"alpha","label":"Alpha <A>","value":if f.phase==1 {40.}else{25.},"x":x,"y":y},"leader":{"id":leader.id,"x":end.x,"y":end.y},"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn pie_rejected() -> bool {
    PIES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let Ok(mut bad) = options(f.phase) else {
                return false;
            };
            bad.style.corner_radius = f64::NAN;
            if kartis::pie(&ChartConfig::default(), &data(f.phase), &bad).is_ok() {
                return false;
            }
            let Ok(mut scene) = source(f.phase) else {
                return false;
            };
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn pie_stop() {
    let entries = PIES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
