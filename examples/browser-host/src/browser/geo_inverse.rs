//! Application-owned geographic inverse picking and marker round trips.
use std::rc::Weak;

use wasm_bindgen::JsCast;
use wasm_bindgen::closure::Closure;

use super::*;

#[wasm_bindgen]
extern "C" {
    type PreciseGeoPointer;
    #[wasm_bindgen(method, getter, js_name = clientX)]
    fn client_x(this: &PreciseGeoPointer) -> f64;
    #[wasm_bindgen(method, getter, js_name = clientY)]
    fn client_y(this: &PreciseGeoPointer) -> f64;
}

#[derive(Clone)]
struct Source {
    scene: Scene,
    projection: GeoProjectionContext,
    view: GeoMapOptions,
    anchor: [f64; 2],
    probe: [f64; 2],
}
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    surface: web_sys::Element,
    renderer: String,
    case: u8,
    phase: u8,
    source: Rc<RefCell<Source>>,
    receipt: Rc<RefCell<Value>>,
    events: Rc<RefCell<Vec<Value>>>,
    listener: Closure<dyn FnMut(web_sys::PointerEvent)>,
    callback: Weak<()>,
}
thread_local! {
    static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};
    static RETIRED:RefCell<Vec<(String,BrowserHandle,Weak<()>)>>=const{RefCell::new(Vec::new())};
}

fn projection(case: u8) -> Result<GeoProjection> {
    Ok(match case {
        0 => GeoProjection::Equirectangular,
        1 => GeoProjection::Mercator,
        2 => GeoProjection::TransverseMercator,
        3 => GeoProjection::EqualEarth,
        4 => GeoProjection::NaturalEarth,
        5 => GeoProjection::Orthographic,
        6 => GeoProjection::AzimuthalEqualArea,
        7 => GeoProjection::AzimuthalEquidistant,
        8 => GeoProjection::Stereographic,
        9 => GeoProjection::Gnomonic,
        _ => {
            return Err(ChartError(
                "geographic projection index must be 0..9".into(),
            ));
        }
    })
}
fn source(case: u8, phase: u8) -> Result<Source> {
    if phase > 2 {
        return Err(ChartError("geographic inverse phase must be 0..2".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 400.,
        margin: Margin {
            left: 43.,
            right: 29.,
            top: 31.,
            bottom: 27.,
        },
        ..Default::default()
    };
    let center = if phase == 0 { [0., 0.] } else { [17., -12.] };
    let view = GeoMapOptions {
        projection: projection(case)?,
        center,
        rotation: if phase == 0 { 0. } else { 23. },
        fit: if phase == 2 {
            GeoFit::Data
        } else {
            GeoFit::World
        },
        scale: (phase == 1).then_some(88.),
        translate: (phase == 1).then_some([310., 190.]),
        padding: 16.,
        fill: Some(Color(224, 235, 244, 255)),
        border: Some(Color(70, 105, 140, 255)),
        ..Default::default()
    };
    let [left, right, bottom, top] = if phase == 2 {
        [-14., 28., -10., 22.]
    } else {
        [-23., 23., -18., 18.]
    };
    let features = vec![GeoFeature {
        id: "land".into(),
        label: String::new(),
        value: Some(1.),
        polygons: vec![vec![vec![
            [center[0] + left, center[1] + bottom],
            [center[0] + right, center[1] + bottom],
            [center[0] + right, center[1] + top],
            [center[0] + left, center[1] + top],
        ]]],
    }];
    let mut map = geographic_map_layout(&config, &features, &view)?;
    map.scene.legend.clear();
    for mark in &mut map.scene.marks {
        mark.label.clear();
        mark.series.clear();
    }
    let anchor = center;
    let probe = [center[0] + 7., center[1] + 6.];
    let mut symbol = GeoSymbol::new("anchor", anchor, Some(25.));
    symbol.label = "Geographic anchor".into();
    map.add_symbols(
        &[symbol],
        &GeoSymbolOptions {
            radius_range: [0., 7.],
            color: GeoSymbolColor::Fixed(Color(25, 130, 80, 255)),
            size_legend: None,
            color_legend: None,
            ..Default::default()
        },
    )?;
    map.scene.validate()?;
    Ok(Source {
        scene: map.scene,
        projection: map.projection,
        view,
        anchor,
        probe,
    })
}

#[wasm_bindgen]
pub fn geo_inverse_mount(case: u8, phase: u8) -> std::result::Result<(), JsValue> {
    let original = source(case, phase).map_err(js)?;
    geo_inverse_stop();
    RETIRED.with(|slot| slot.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, kind) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("geo-inverse-{renderer}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            original.scene.clone(),
            BrowserOptions {
                renderer: kind,
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                interaction: surface::SurfaceOptions {
                    plot: Some(original.projection.plot()),
                    brush: false,
                    pinch: false,
                    pan: true,
                    zoom: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let accepted = Rc::new(RefCell::new(original.clone()));
        let receipt = Rc::new(RefCell::new(json!({"count":0})));
        let events = Rc::new(RefCell::new(Vec::new()));
        let token = Rc::new(());
        let weak = Rc::downgrade(&token);
        let pointer_token = token.clone();
        let callback_events = events.clone();
        chart.on_event(move |event| {
            let _keep = &token;
            let mut events = callback_events.borrow_mut();
            if events.len() >= 64 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        let surface = root
            .query_selector(".kartis-surface")?
            .ok_or_else(|| js("missing geographic surface"))?;
        let readout = document.create_element("output")?;
        readout.set_class_name("kartis-geo-inverse-status");
        readout.set_attribute("aria-label", "Geographic coordinates")?;
        readout.set_attribute("aria-live", "polite")?;
        readout.set_attribute("aria-atomic", "true")?;
        readout.set_attribute(
            "style",
            "display:block;margin:8px 0;font-variant-numeric:tabular-nums",
        )?;
        readout.set_text_content(Some("Move over the map to read longitude and latitude."));
        root.append_child(&readout)?;
        let handle = chart.handle();
        let current = accepted.clone();
        let output = receipt.clone();
        let listener = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
            let _keep_listener = &pointer_token;
            // Replacing the hover annotation during a live drag would retire
            // the host's drag authority. Keep the accepted location while the
            // user pans, then refresh from the accepted camera on pointerup.
            if event.buttons() != 0 {
                return;
            }
            let precise: &PreciseGeoPointer = event.unchecked_ref();
            let client = (precise.client_x(), precise.client_y());
            let result = (|| -> Result<Value> {
                handle.flush()?;
                let source = current.borrow();
                let scene = handle.scene_point_at_client(client)?;
                let position = handle.invert_geo_in_plot_at_client(&source.projection, client)?;
                let intrinsic = handle.invert_geo_at_client(&source.projection, client)?;
                let projected = position
                    .map(|position| source.projection.project(position))
                    .transpose()?
                    .flatten();
                let mut candidate = source.scene.clone();
                if let Some([x, y]) = projected {
                    let mut marker = Mark::new(
                        "geo-inverse-marker",
                        Geometry::Circle { x, y, radius: 5. },
                        Color(215, 35, 75, 255),
                    );
                    marker.label.clear();
                    marker.clip = Some(source.projection.plot());
                    candidate.marks.push(marker);
                }
                handle.set_scene(candidate)?;
                readout.set_text_content(Some(&position.map_or_else(
                    || "Outside the visible map.".into(),
                    |[longitude, latitude]| {
                        format!("Longitude {longitude:.5}°, latitude {latitude:.5}°")
                    },
                )));
                let count = output.borrow()["count"].as_u64().unwrap_or(0) + 1;
                Ok(
                    json!({"count":count,"trusted":event.is_trusted(),"eventType":event.type_(),"client":[client.0,client.1],"scene":[scene.0,scene.1],"position":position,"intrinsic":intrinsic,"marker":projected,"pointerType":event.pointer_type()}),
                )
            })();
            *output.borrow_mut() =
                result.unwrap_or_else(|error| json!({"error":error.to_string()}));
        }) as Box<dyn FnMut(_)>);
        surface
            .add_event_listener_with_callback("pointermove", listener.as_ref().unchecked_ref())?;
        surface.add_event_listener_with_callback("pointerup", listener.as_ref().unchecked_ref())?;
        FIXTURES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                surface,
                renderer: renderer.into(),
                case,
                phase,
                source: accepted,
                receipt,
                events,
                listener,
                callback: weak,
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn geo_inverse_phase(phase: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|slot| {
        for fixture in slot.borrow_mut().iter_mut() {
            let next = source(fixture.case, phase).map_err(js)?;
            fixture.chart.set_scene(next.scene.clone()).map_err(js)?;
            fixture.chart.flush().map_err(js)?;
            *fixture.source.borrow_mut() = next;
            fixture.phase = phase;
            *fixture.receipt.borrow_mut() = json!({"count":0});
            if let Some(readout) = fixture.root.query_selector(".kartis-geo-inverse-status")? {
                readout.set_text_content(Some("Move over the map to read longitude and latitude."));
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_inverse_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn geo_inverse_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let Some(fixture) = entries.iter().find(|f| f.renderer == renderer) else {
            return Ok(RETIRED.with(|slot| {
                let entries = slot.borrow();
                let old = entries.iter().find(|f| f.0 == renderer);
                json!({"active":false,"retired":old.is_some(),"handleStopped":old.is_none_or(|f|!f.1.is_active()),"callbackReleased":old.is_none_or(|f|f.2.upgrade().is_none())}).to_string()
            }));
        };
        let snapshot = fixture.chart.snapshot().map_err(js)?;
        let source = fixture.source.borrow();
        let p = source.projection.plot();
        Ok(json!({"active":snapshot.active,"case":fixture.case,"phase":fixture.phase,"width":snapshot.width,"height":snapshot.height,"revision":snapshot.revision,"pending":snapshot.pending,"hovered":snapshot.hovered,"selected":snapshot.selected,"view":{"scale":snapshot.view.scale,"x":snapshot.view.translate_x,"y":snapshot.view.translate_y},"plot":[p.x,p.y,p.width,p.height],"scale":source.projection.scale(),"translate":source.projection.translate(),"fittedRawCenter":source.projection.fitted_raw_center(),"center":source.view.center,"roll":source.view.rotation,"fit":format!("{:?}",source.view.fit),"anchor":source.anchor,"probe":source.probe,"anchorPoint":source.projection.project(source.anchor).map_err(js)?,"probePoint":source.projection.project(source.probe).map_err(js)?,"receipt":fixture.receipt.borrow().clone(),"events":fixture.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn geo_inverse_rejected() -> bool {
    FIXTURES.with(|slot| {
        slot.borrow().iter().all(|fixture| {
            let source = fixture.source.borrow();
            let mut invalid = source.scene.clone();
            invalid.marks[0].opacity = f64::NAN;
            fixture.chart.set_scene(invalid).is_err()
                && source.projection.invert([f64::NAN, 0.]).is_err()
                && source.projection.invert([1e7, 0.]).is_err()
                && fixture
                    .chart
                    .handle()
                    .scene_point_at_client((f64::NAN, 0.))
                    .is_err()
                && fixture
                    .chart
                    .handle()
                    .invert_geo_at_client(&source.projection, (f64::NAN, 0.))
                    .is_err()
                && fixture
                    .chart
                    .handle()
                    .invert_geo_in_plot_at_client(&source.projection, (f64::NAN, 0.))
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn geo_inverse_reset_view(renderer: &str) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let fixture = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing geographic renderer"))?;
        fixture.chart.handle().reset_view().map_err(js)
    })
}
#[wasm_bindgen]
pub fn geo_inverse_stop() {
    let fixtures = FIXTURES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for fixture in fixtures {
        let _ = fixture.surface.remove_event_listener_with_callback(
            "pointermove",
            fixture.listener.as_ref().unchecked_ref(),
        );
        let _ = fixture.surface.remove_event_listener_with_callback(
            "pointerup",
            fixture.listener.as_ref().unchecked_ref(),
        );
        let handle = fixture.chart.handle();
        fixture.chart.stop();
        fixture.root.remove();
        RETIRED.with(|slot| {
            slot.borrow_mut()
                .push((fixture.renderer, handle, fixture.callback))
        });
    }
}
