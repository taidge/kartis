use super::*;
struct GeoStreamFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static GEO_STREAMS:RefCell<Vec<GeoStreamFixture>>=const{RefCell::new(Vec::new())};}
fn source(
    case: u8,
    phase: u8,
    invalid: bool,
) -> Result<(GeoMapLayout, GeoStreamMetadata, [f64; 2])> {
    if case > 2 || phase > 1 {
        return Err(ChartError("geo stream fixture case0..2 phase0..1".into()));
    }
    let c = ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 30.,
            right: 30.,
            top: 25.,
            bottom: 25.,
        },
        ..Default::default()
    };
    let view = GeoMapOptions {
        projection: match case {
            0 => GeoProjection::Equirectangular,
            1 => GeoProjection::TransverseMercator,
            _ => {
                if phase == 0 {
                    GeoProjection::Orthographic
                } else {
                    GeoProjection::Gnomonic
                }
            }
        },
        clip_angle: (case == 2).then_some(60.),
        fit: if case == 0 {
            GeoFit::Data
        } else {
            GeoFit::World
        },
        center: if case == 1 && phase == 1 {
            [12., 10.]
        } else {
            [0., 0.]
        },
        rotation: if case == 0 && phase == 1 { 8. } else { 0. },
        scale: if case == 0 {
            None
        } else {
            Some(if phase == 0 { 90. } else { 120. })
        },
        padding: 12.,
        fill: Some(Color(225, 235, 243, 255)),
        border: Some(Color(90, 115, 140, 255)),
        border_width: 1.,
        ..Default::default()
    };
    let regions = if case == 0 {
        vec![
            (
                "east",
                vec![[155., -25.], [178., -25.], [178., 25.], [155., 25.]],
            ),
            (
                "west",
                vec![[-178., -25.], [-155., -25.], [-155., 25.], [-178., 25.]],
            ),
        ]
    } else {
        vec![(
            "land",
            vec![[-35., -30.], [35., -30.], [35., 45.], [-35., 45.]],
        )]
    };
    let features = regions
        .into_iter()
        .map(|(id, ring)| GeoFeature {
            id: id.into(),
            label: id.into(),
            value: Some(1.),
            polygons: vec![vec![ring]],
        })
        .collect::<Vec<_>>();
    let path = GeoPathOptions {
        precision: if invalid { f64::NAN } else { 0.1 },
        ..Default::default()
    };
    let mut map = geographic_map_layout_with_path_options(&c, &features, &view, &path)?;
    let mut grid_style = GeoStreamOptions {
        interactive: false,
        behind: true,
        ..Default::default()
    };
    grid_style.style.stroke.color = Color(170, 184, 201, 255);
    grid_style.style.stroke.width = 0.75;
    map.add_graticule(
        &GeoGraticuleOptions {
            step_minor: [30., 30.],
            ..Default::default()
        },
        &grid_style,
    )?;
    let positions = match case {
        0 => vec![[164., 10.], [-164., 30.]],
        1 => {
            if phase == 0 {
                vec![[-50., -10.], [50., 35.]]
            } else {
                vec![[-40., -20.], [60., 40.]]
            }
        }
        _ => vec![[-80., 0.], [80., 0.]],
    };
    let mut route = GeoLineFeature::new("route", positions.clone());
    route.label = "<Current route>".into();
    route.value = Some(if phase == 0 { 10. } else { 25. });
    let mut opts = GeoStreamOptions::default();
    opts.style.stroke.width = if phase == 0 { 4. } else { 6. };
    opts.style.stroke.color = if phase == 0 {
        Color(200, 40, 65, 255)
    } else {
        Color(35, 120, 195, 255)
    };
    if phase == 1 {
        opts.style.dash = Some(StrokeDash {
            segments: vec![12., 6.],
            offset: 2.,
        });
    }
    let metadata = map.add_lines(&[route], &opts)?;
    map.scene.legend.push(LegendItem {
        id: "route".into(),
        label: "Current route".into(),
        color: opts.style.stroke.color,
    });
    let position = if case == 0 { positions[0] } else { [0., 0.] };
    let projected = map
        .projection
        .project(position)?
        .ok_or_else(|| ChartError("hidden symbol probe".into()))?;
    map.add_symbols(
        &[GeoSymbol::new("shared", position, Some(1.))],
        &GeoSymbolOptions {
            radius_range: [0., 5.],
            color: GeoSymbolColor::Fixed(Color(20, 135, 75, 255)),
            size_legend: None,
            color_legend: None,
            ..Default::default()
        },
    )?;
    map.scene.validate()?;
    Ok((map, metadata, projected))
}
#[wasm_bindgen]
pub fn geo_stream_mount(case: u8) -> std::result::Result<(), JsValue> {
    let (map, _, _) = source(case, 0, false).map_err(js)?;
    geo_stream_stop();
    let d = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = d.create_element("div")?;
        root.set_id(&format!("geo-stream-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        d.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            map.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                highlight: browser::BrowserHighlight::Controlled,
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
        let events = Rc::new(RefCell::new(vec![]));
        let capture = events.clone();
        chart.on_event(move |e| {
            let mut events = capture.borrow_mut();
            if events.len() == 64 {
                events.remove(0);
            }
            events.push(event_value(e));
        });
        chart.flush().map_err(js)?;
        GEO_STREAMS.with(|slot| {
            slot.borrow_mut().push(GeoStreamFixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn geo_stream_phase(phase: u8) -> std::result::Result<(), JsValue> {
    GEO_STREAMS.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            let (m, _, _) = source(f.case, phase, false).map_err(js)?;
            f.chart.set_scene(m.scene).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_stream_flush() -> std::result::Result<(), JsValue> {
    GEO_STREAMS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_stream_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    GEO_STREAMS.with(|slot| {
        let v = slot.borrow();
        let f = v
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing geo renderer"))?;
        f.chart
            .handle()
            .set_hidden(if hidden {
                ["route".into()].into_iter().collect()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
// Select a real segment midpoint whose complete 1px neighborhood still hits
// this route. Trusted Firefox input can quantize a requested position by a
// fraction of a logical pixel; a point beside a foreground symbol is ambiguous.
pub fn geo_stream_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    GEO_STREAMS.with(|slot|{let v=slot.borrow();let Some(f)=v.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let s=f.chart.snapshot().map_err(js)?;let (map,meta,expected)=source(f.case,f.phase,false).map_err(js)?;let route=map.scene.marks.iter().find(|m|m.id=="geo-line:route").unwrap();let commands=route.geometry.commands();let mut target=None;let mut previous=None;for command in &commands{match command{PathCommand::Move(p)=>previous=Some([p.x,p.y]),PathCommand::Line(p)=>{if let Some(a)=previous{let x=(a[0]+p.x)/2.;let y=(a[1]+p.y)/2.;if x>34.&&x<606.&&y>29.&&y<331.&&[-1.,0.,1.].into_iter().all(|dx|[-1.,0.,1.].into_iter().all(|dy|map.scene.hit_test(x+dx,y+dy).is_some_and(|m|m.id==route.id))){target=Some(json!({"x":x,"y":y,"id":route.id,"label":route.label,"value":route.value}));break;}}previous=Some([p.x,p.y]);},_=>{}}}let symbol=map.scene.marks.iter().find(|m|m.id=="geo-symbol:6:shared").unwrap();let Geometry::Circle{x,y,..}=symbol.geometry else{return Err(js("symbol geometry"));};Ok(json!({"active":s.active,"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"hidden":s.hidden,"hovered":s.hovered,"selected":s.selected,"case":f.case,"phase":f.phase,"target":target,"probeMargin":1.,"routeComponents":meta.lines[0].components,"routeCommands":commands.len(),"sampleWork":meta.sampling.work,"pathPrecision":map.projection.path_options().unwrap().precision,"sharedSymbol":[x,y],"expectedSymbol":expected,"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn geo_stream_rejected() -> bool {
    GEO_STREAMS.with(|slot| {
        slot.borrow().iter().all(|f| {
            if source(f.case, f.phase, true).is_ok() {
                return false;
            }
            let Ok((mut map, _, _)) = source(f.case, f.phase, false) else {
                return false;
            };
            let before = map.clone();
            let bad = GeoLineFeature::new("bad", vec![[0., 0.], [180., 0.]]);
            let refused = map.add_lines(&[bad], &Default::default()).is_err() && map == before;
            let mut scene = map.scene;
            scene.marks[0].opacity = f64::NAN;
            refused && f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn geo_stream_stop() {
    let entries = GEO_STREAMS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
