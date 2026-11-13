//! Configured radar acceptance fixtures, using the actual shared Scene.
use super::*;
struct RadarFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static RADARS:RefCell<Vec<RadarFixture>>=const{RefCell::new(Vec::new())};}
fn source(phase: u8) -> Result<Scene> {
    let c = ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 80.,
            right: 80.,
            top: 40.,
            bottom: 40.,
        },
        title: "Configured radar".into(),
        ..Default::default()
    };
    let axes = ["speed", "quality", "cost", "safety", "uptime"].map(str::to_string);
    let mut options = RadarLayoutOptions {
        domain: Some([0., 100.]),
        curve: if phase == 0 {
            Curve::cardinal_closed(0.5)
        } else {
            Curve::CatmullRomClosed { alpha: 0.5 }
        },
        ..Default::default()
    };
    options.grid.shape = if phase == 0 {
        RadarGridShape::Circle
    } else {
        RadarGridShape::Polygon
    };
    options.grid.values = Some(vec![25., 50., 75., 100.]);
    options.grid.tick_labels = true;
    options.grid.tick_label_axis = 2;
    options.grid.tick_style.color = Some(Color(80, 90, 100, 255));
    options.grid.tick_style.size = Some(11.);
    options.grid.tick_format = ValueFormat::Affix {
        prefix: String::new(),
        suffix: "%".into(),
        precision: 0,
    };
    options.axis_labels_by_id = BTreeMap::from([
        ("speed".into(), "Speed <safe>".into()),
        ("quality".into(), "Quality".into()),
        ("cost".into(), "Cost".into()),
        ("safety".into(), "Safety".into()),
        ("uptime".into(), "Uptime".into()),
    ]);
    options.axis_label_style.size = Some(17.);
    options.axis_label_style.color = Some(Color(20, 50, 80, 255));
    options.series.fill_opacity = 0.18;
    options.series.border.width = 4.;
    options.series.border.dash = Some(StrokeDash {
        segments: vec![10., 4.],
        offset: 2.,
    });
    options.points.shape = RadarPointShape::Diamond;
    options.points.size = 14.;
    options.points.fill = Some(Color(255, 255, 255, 255));
    options.points.border.enabled = true;
    options.points.border.width = 2.;
    options.points.labels = true;
    options.points.value_format = ValueFormat::Affix {
        prefix: String::new(),
        suffix: "%".into(),
        precision: 0,
    };
    options.points.label_style.color = Some(Color(20, 40, 100, 255));
    options.points.label_offset = [0., -16.];
    if phase > 1 {
        options.domain = Some([100., 0.]);
    }
    let mut alpha = RadarSeries::new(
        "alpha",
        if phase == 0 {
            vec![80., 65., 60., 70., 90.]
        } else {
            vec![55., 80., 65., 95., 75.]
        },
    );
    alpha.label = "Alpha <A>".into();
    let mut beta = RadarSeries::new("beta", vec![50., 40., 75., 50., 55.]);
    beta.label = "Beta group".into();
    radar_with_options(&c, &axes, &[alpha, beta], &options)
}
#[wasm_bindgen]
pub fn radar_mount() -> std::result::Result<(), JsValue> {
    radar_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("radar-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            source(0).map_err(js)?,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        chart.on_event(move |event| log.borrow_mut().push(json!({"event":format!("{event:?}")})));
        RADARS.with(|slot| {
            slot.borrow_mut().push(RadarFixture {
                chart,
                root,
                renderer: renderer.into(),
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn radar_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase).map_err(js)?;
    RADARS.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radar_flush() -> std::result::Result<(), JsValue> {
    RADARS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radar_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    RADARS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};
    let state=f.chart.snapshot().map_err(js)?;let scene=source(f.phase).map_err(js)?;let m=scene.marks.iter().find(|m|m.id=="radar-point:5:alpha:5:speed").unwrap();
    let (x,y)=match &m.geometry {Geometry::Circle{x,y,..}=>(*x,*y),Geometry::Path(path)=>{let vertices=path.iter().filter_map(|c|match c{glory::render::PathCommand::Move(p)|glory::render::PathCommand::Line(p)=>Some(*p),_=>None}).collect::<Vec<_>>();(vertices.iter().map(|p|p.x).sum::<f64>()/vertices.len() as f64,vertices.iter().map(|p|p.y).sum::<f64>()/vertices.len() as f64)},_=>unreachable!()};
    Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hidden":state.hidden,"hovered":state.hovered,"selected":state.selected,"phase":f.phase,"target":{"id":m.id,"value":m.value,"label":m.label,"x":x,"y":y},"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn radar_rejected() -> bool {
    source(2).is_err()
        && RADARS.with(|slot| {
            slot.borrow().iter().all(|f| {
                let Ok(mut s) = source(f.phase) else {
                    return false;
                };
                s.marks[0].opacity = f64::NAN;
                f.chart.set_scene(s).is_err()
            })
        })
}
#[wasm_bindgen]
pub fn radar_stop() {
    let entries = RADARS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
