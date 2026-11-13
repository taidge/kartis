//! Independent real SVG/Canvas Delaunay controllers and inspectable topology.
use super::*;
struct DelaunayFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    case: u8,
    phase: u8,
    graph: Delaunay,
    diagram: DelaunayDiagram,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static DELAUNAY:RefCell<Vec<DelaunayFixture>>=const{RefCell::new(Vec::new())};}

fn source(case: u8, phase: u8) -> Result<(Delaunay, DelaunayDiagram)> {
    if phase > 1 {
        return Err(ChartError("Delaunay phase must be 0 or 1".into()));
    }
    let step = f64::from(phase);
    let mut points = match case {
        0 => vec![
            DelaunaySite::new("b", 0., 0.),
            DelaunaySite::new("c", 100., 0.),
            DelaunaySite::new("d", 0., 100.),
            DelaunaySite::new("target <E>", 30. + step * 8., 35.),
            DelaunaySite::new("f", 90., 80.),
        ],
        1 => vec![
            DelaunaySite::new("p0", 50., 0.),
            DelaunaySite::new("p1", 40., 30.),
            DelaunaySite::new("target <E>", 0., 50. + step * 5.),
            DelaunaySite::new("p3", -40., 30.),
            DelaunaySite::new("p4", -50., 0.),
            DelaunaySite::new("p5", 0., -50.),
            DelaunaySite::new("zz-duplicate", 50., -0.),
        ],
        2 => vec![
            DelaunaySite::new("left", 0., 0.),
            DelaunaySite::new("quarter", 25., 0.),
            DelaunaySite::new("target <E>", 50., step * 20.),
            DelaunaySite::new("three-quarter", 75., 0.),
            DelaunaySite::new("right", 100., 0.),
        ],
        _ => return Err(ChartError("Delaunay case must be 0..=2".into())),
    };
    if case == 0 && phase == 1 {
        points.push(DelaunaySite::new("a", -30., 30.));
    }
    let graph = Delaunay::new(points)?;
    let diagram = graph.diagram(
        &ChartConfig {
            width: 640.,
            height: 360.,
            title: "Delaunay adjacency".into(),
            margin: Margin {
                left: 55.,
                right: 85.,
                top: 45.,
                bottom: 45.,
            },
            ..Default::default()
        },
        &DelaunayDiagramOptions {
            site_radius: 7.,
            site_color: Color(35, 105, 195, 255),
            link_width: 2.5,
            link_color: Color(60, 85, 105, 255),
            triangle_fill: Some(Color(80, 145, 210, 35)),
            labels: true,
            label_size: 13.,
            ..Default::default()
        },
    )?;
    Ok((graph, diagram))
}

#[wasm_bindgen]
pub fn delaunay_mount(case: u8) -> std::result::Result<(), JsValue> {
    let (graph, diagram) = source(case, 0).map_err(js)?;
    delaunay_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("delaunay-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            diagram.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
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
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                delaunay_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut logs = capture.borrow_mut();
            let sequence = logs
                .last()
                .and_then(|event: &Value| event.get("sequence"))
                .and_then(Value::as_u64)
                .unwrap_or(0)
                .saturating_add(1);
            if logs.len() == 48 {
                logs.remove(0);
            }
            let mut value = event_value(event);
            value["sequence"] = json!(sequence);
            logs.push(value);
        });
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            delaunay_stop();
            return Err(js(error));
        }
        DELAUNAY.with(|slot| {
            slot.borrow_mut().push(DelaunayFixture {
                chart,
                root,
                renderer: name,
                case,
                phase: 0,
                graph: graph.clone(),
                diagram: diagram.clone(),
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn delaunay_phase(phase: u8) -> std::result::Result<(), JsValue> {
    DELAUNAY.with(|slot| {
        let mut entries = slot.borrow_mut();
        for fixture in entries.iter_mut() {
            let (graph, diagram) = source(fixture.case, phase).map_err(js)?;
            fixture.chart.set_scene(diagram.scene.clone()).map_err(js)?;
            fixture.graph = graph;
            fixture.diagram = diagram;
            fixture.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn delaunay_flush() -> std::result::Result<(), JsValue> {
    DELAUNAY.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn delaunay_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    DELAUNAY.with(|slot| {
        let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer) else {return Ok(json!({"active":false}).to_string());};
        let state=f.chart.snapshot().map_err(js)?;
        let target=f.diagram.sites.iter().find(|s|s.id=="target <E>").unwrap();
        let mark=f.diagram.scene.marks.iter().find(|m|m.id==target.mark_id).unwrap();
        let sites=f.graph.sites().iter().map(|s|json!({"id":s.id,"x":s.x,"y":s.y})).collect::<Vec<_>>();
        let triangles=f.graph.triangles().as_chunks::<3>().0.iter().map(|t|t.map(|i|f.graph.sites()[i].id.clone())).collect::<Vec<_>>();
        let links=f.graph.links().iter().map(|p|p.map(|i|f.graph.sites()[i].id.clone())).collect::<Vec<_>>();
        let hull=f.graph.hull().iter().map(|&i|f.graph.sites()[i].id.clone()).collect::<Vec<_>>();
        let raw=f.graph.raw_sites().iter().enumerate().map(|(i,s)|json!({"id":s.id,"owner":f.graph.sites()[f.graph.raw_owners()[i]].id})).collect::<Vec<_>>();
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"case":f.case,"phase":f.phase,"sites":sites,"raw":raw,"triangles":triangles,"links":links,"hull":hull,"target":{"id":target.mark_id,"raw_id":target.id,"x":target.x,"y":target.y,"value":mark.value,"label":mark.label,"neighbors":f.graph.neighbors("target <E>").unwrap().iter().map(|s|s.id.clone()).collect::<Vec<_>>()},"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn delaunay_rejected() -> bool {
    DELAUNAY.with(|slot| {
        slot.borrow().iter().all(|f| {
            let mut raw = f.graph.raw_sites().to_vec();
            raw[0].x = f64::NAN;
            if Delaunay::new(raw).is_ok() {
                return false;
            }
            let mut invalid = f.diagram.scene.clone();
            invalid.marks[0].opacity = f64::NAN;
            f.chart.set_scene(invalid).is_err() && source(f.case, 2).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn delaunay_stop() {
    let entries = DELAUNAY.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for fixture in entries {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
