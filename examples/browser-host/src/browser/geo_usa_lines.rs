//! Actual per-component line ink with one canonical browser data control.
use super::*;

struct Source {
    scene: Scene,
    metadata: GeoUsaStreamMetadata,
    projection: GeoProjectionContext,
    phase: u8,
}
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    source: Source,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static ENTRIES:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};}
fn source(phase: u8) -> Result<Source> {
    if phase > 1 {
        return Err(ChartError("USA line phase must be 0..1".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    };
    let centers = [[-122.4194, 37.7749], [-149.9, 61.2], [-157.8583, 21.3069]];
    let features = centers
        .iter()
        .enumerate()
        .map(|(i, [x, y])| GeoFeature {
            id: format!("land{i}"),
            label: String::new(),
            value: None,
            polygons: vec![vec![vec![
                [x - 3., y - 2.],
                [x - 3., y + 2.],
                [x + 3., y + 2.],
                [x + 3., y - 2.],
            ]]],
        })
        .collect::<Vec<_>>();
    let mut map = geographic_usa_map_layout(
        &config,
        &features,
        &GeoUsaMapOptions {
            fit: if phase == 0 {
                GeoFit::World
            } else {
                GeoFit::Data
            },
            padding: 12.,
            border: None,
            fill: Some(Color(225, 235, 244, 255)),
            ..Default::default()
        },
    )?;
    map.add_usa_graticule(
        &Default::default(),
        &GeoStreamOptions {
            behind: true,
            style: GeoLineStyle {
                stroke: GraphicStroke {
                    color: Color(190, 195, 205, 255),
                    width: 0.8,
                    cap: StrokeCap::Butt,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    let invisible = GeoLineFeature::new("outside", vec![[1., 47.], [3., 49.]]);
    let mut route = GeoLineFeature::new("route", vec![]);
    route.label = "Regional route".into();
    route.series = String::new();
    route.value = if phase == 0 { None } else { Some(42.) };
    route.lines = centers
        .iter()
        .map(|[x, y]| vec![[x - 2., *y], [x + 2., *y]])
        .collect();
    let metadata = map.add_usa_lines(
        &[invisible, route],
        &GeoStreamOptions {
            style: GeoLineStyle {
                stroke: GraphicStroke {
                    color: Color(210, 35, 80, 255),
                    width: 7.,
                    cap: StrokeCap::Round,
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    Ok(Source {
        scene: map.scene,
        metadata,
        projection: map.projection,
        phase,
    })
}
fn probe(geometry: &Geometry) -> [f64; 2] {
    let Geometry::Path(commands) = geometry else {
        unreachable!()
    };
    let mut last = None;
    let mut best = ([0., 0.], -1.);
    for command in commands {
        match command {
            PathCommand::Move(p) => last = Some([p.x, p.y]),
            PathCommand::Line(p) => {
                if let Some(a) = last {
                    let length = (p.x - a[0]).hypot(p.y - a[1]);
                    if length > best.1 {
                        best = ([(a[0] + p.x) / 2., (a[1] + p.y) / 2.], length);
                    }
                }
                last = Some([p.x, p.y]);
            }
            _ => {}
        }
    }
    best.0
}
#[wasm_bindgen]
pub fn usa_lines_mount() -> std::result::Result<(), JsValue> {
    usa_lines_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, kind) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let source = source(0).map_err(js)?;
        let binding = source
            .metadata
            .interaction_binding(&source.scene)
            .map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("usa-lines-{renderer}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            source.scene.clone(),
            BrowserOptions {
                renderer: kind,
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(binding),
                interaction: surface::SurfaceOptions {
                    plot: Some(source.projection.plot()),
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                tooltip_options: TooltipOptions {
                    missing: MissingValue::Placeholder("not supplied".into()),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut v = capture.borrow_mut();
            if v.len() >= 64 {
                v.remove(0);
            }
            v.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        ENTRIES.with(|s| {
            s.borrow_mut().push(Entry {
                chart,
                root,
                renderer,
                source,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn usa_lines_phase(phase: u8) -> std::result::Result<(), JsValue> {
    ENTRIES.with(|s| {
        for e in s.borrow_mut().iter_mut() {
            let next = source(phase).map_err(js)?;
            let frame = next
                .metadata
                .clone()
                .into_scene_interaction(next.scene.clone())
                .map_err(js)?;
            e.chart.set_layout(frame).map_err(js)?;
            e.chart.flush().map_err(js)?;
            e.source = next;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn usa_lines_flush() -> std::result::Result<(), JsValue> {
    ENTRIES.with(|s| {
        for e in s.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn usa_lines_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    ENTRIES.with(|s|{
    let entries=s.borrow();let Some(e)=entries.iter().find(|e|e.renderer==renderer) else{return Ok(json!({"active":false}).to_string());};
    let state=e.chart.snapshot().map_err(js)?;let line=&e.source.metadata.lines[1];
    let active=state.interaction.as_ref().map(|a|json!({"anchor":a.anchor,"ids":a.ids,"rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"series":r.series,"label":r.label,"value":r.value})).collect::<Vec<_>>()}));
    let binding=e.source.metadata.interaction_binding(&e.source.scene).map_err(js)?;
    Ok(json!({"active":state.active,"phase":e.source.phase,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"interaction":active,"id":line.mark_id,"sourceIndex":line.source_index,"series":line.series,"sourceSeries":line.source_series,"value":line.value,"anchor":binding.sites[0].position,"controls":binding.sites.len(),"aliases":binding.ink_owners.len(),"parts":line.parts.iter().map(|p|json!({"region":format!("{:?}",p.region),"id":p.mark_id,"probe":probe(&p.geometry)})).collect::<Vec<_>>(),"events":e.events.borrow().clone()}).to_string())
})
}
#[wasm_bindgen]
pub fn usa_lines_rejected() -> bool {
    ENTRIES.with(|s| {
        s.borrow().iter().all(|e| {
            let mut invalid = e.source.scene.clone();
            let id = &e.source.metadata.lines[1].mark_id;
            let mark = invalid.marks.iter_mut().find(|m| &m.id == id).unwrap();
            if let Geometry::Path(path) = &mut mark.geometry {
                path.remove(0);
            }
            let binding = e
                .source
                .metadata
                .interaction_binding(&e.source.scene)
                .unwrap();
            e.source.metadata.interaction_binding(&invalid).is_err()
                && e.chart
                    .set_layout(SceneInteraction {
                        scene: invalid,
                        interaction: Some(binding),
                        plot: Some(e.source.projection.plot()),
                    })
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn usa_lines_stop() {
    let entries = ENTRIES.with(|s| std::mem::take(&mut *s.borrow_mut()));
    for e in entries {
        e.chart.stop();
        e.root.remove();
    }
}
