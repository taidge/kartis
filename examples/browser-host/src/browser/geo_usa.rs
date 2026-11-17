//! Real browser events and editable labels in the fixed USA component frame.
use std::rc::Weak;

use super::*;

#[wasm_bindgen]
extern "C" {
    type UsaPointer;
    #[wasm_bindgen(method,getter,js_name=clientX)]
    fn client_x(this: &UsaPointer) -> f64;
    #[wasm_bindgen(method,getter,js_name=clientY)]
    fn client_y(this: &UsaPointer) -> f64;
}

#[derive(Clone)]
struct Source {
    scene: Scene,
    projection: GeoProjectionContext,
    phase: u8,
    labels: bool,
    masked: bool,
}
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    surface: web_sys::Element,
    renderer: String,
    source: Source,
    region: Rc<Cell<GeoUsaRegion>>,
    receipt: Rc<RefCell<Value>>,
    events: Rc<RefCell<Vec<Value>>>,
    listener: Closure<dyn FnMut(web_sys::PointerEvent)>,
    callback: Weak<()>,
}
thread_local! {
    static ENTRIES:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};
    static RETIRED:RefCell<Vec<(String,BrowserHandle,Weak<()>)>>=const{RefCell::new(Vec::new())};
}
fn positions() -> [[f64; 2]; 4] {
    [
        [-122.4194, 37.7749],
        [-149.9, 61.2],
        [-157.8583, 21.3069],
        [-114.5, 27.],
    ]
}
fn source(phase: u8, labels: bool, masked: bool) -> Result<Source> {
    if phase > 2 {
        return Err(ChartError("USA phase must be 0..2".into()));
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
    let features = positions()[..3]
        .iter()
        .enumerate()
        .map(|(i, [x, y])| GeoFeature {
            id: format!("land-{i}"),
            label: String::new(),
            value: Some(i as f64),
            polygons: vec![vec![vec![
                [x - 2., y - 1.],
                [x - 2., y + 1.],
                [x + 2., y + 1.],
                [x + 2., y - 1.],
            ]]],
        })
        .collect::<Vec<_>>();
    let mut layout = geographic_usa_map_layout(
        &config,
        &features,
        &GeoUsaMapOptions {
            fit: if phase == 2 {
                GeoFit::Data
            } else {
                GeoFit::World
            },
            padding: 12.,
            scale: (phase == 1 || labels).then_some(550.),
            translate: (phase == 1 || labels).then_some([320., 190.]),
            fill: Some(Color(225, 235, 244, 255)),
            border: None,
            ..Default::default()
        },
    )?;
    layout.scene.legend.clear();
    if labels {
        layout.scene.marks.clear();
    }
    let symbols = if labels {
        vec![GeoSymbol::new(
            "HI <native & editable>",
            positions()[2],
            Some(7.),
        )]
    } else {
        positions()[..3]
            .iter()
            .enumerate()
            .map(|(i, p)| GeoSymbol::new(format!("place-{i}"), *p, Some(7.)))
            .collect()
    };
    layout.add_symbols(
        &symbols,
        &GeoSymbolOptions {
            radius_range: if labels { [60., 60.] } else { [6., 6.] },
            color: GeoSymbolColor::Fixed(Color(30, 90, 160, 255)),
            labels,
            label_color: Color(235, 20, 150, 255),
            label_size: 18.,
            clip_to_projection: masked,
            clip_labels_to_projection: masked,
            ..Default::default()
        },
    )?;
    Ok(Source {
        scene: layout.scene,
        projection: layout.projection,
        phase,
        labels,
        masked,
    })
}
fn inverse_value(value: Option<GeoUsaInverse>) -> Value {
    value.map_or(
        Value::Null,
        |v| json!({"position":v.position,"region":format!("{:?}",v.region)}),
    )
}
fn mount(phase: u8, labels: bool, masked: bool) -> std::result::Result<(), JsValue> {
    let original = source(phase, labels, masked).map_err(js)?;
    usa_stop();
    RETIRED.with(|s| s.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, kind) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("usa-{renderer}"));
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
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let receipt = Rc::new(RefCell::new(json!({"count":0})));
        let events = Rc::new(RefCell::new(Vec::new()));
        let token = Rc::new(());
        let weak = Rc::downgrade(&token);
        let event_token = token.clone();
        let log = events.clone();
        chart.on_event(move |event| {
            let _keep = &event_token;
            let mut log = log.borrow_mut();
            if log.len() >= 64 {
                log.remove(0);
            }
            log.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        let surface = root
            .query_selector(".kartis-surface")?
            .ok_or_else(|| js("missing USA surface"))?;
        let readout = document.create_element("output")?;
        readout.set_attribute("aria-label", "USA geographic coordinates")?;
        readout.set_attribute("aria-live", "polite")?;
        readout.set_text_content(Some("Move over the map."));
        root.append_child(&readout)?;
        let handle = chart.handle();
        let projection = original.projection.clone();
        let region = Rc::new(Cell::new(GeoUsaRegion::Lower48));
        let requested = region.clone();
        let output = receipt.clone();
        let listener = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
            let _keep = &token;
            if event.buttons() != 0 {
                return;
            }
            let precise: &UsaPointer = event.unchecked_ref();
            let client = (precise.client_x(), precise.client_y());
            let result = (|| -> Result<Value> {
                handle.flush()?;
                let point = handle.scene_point_at_client(client)?;
                let default = handle.invert_usa_at_client(&projection, client)?;
                let known =
                    handle.invert_usa_in_region_at_client(&projection, requested.get(), client)?;
                let general = handle.invert_geo_at_client(&projection, client)?;
                let in_plot = handle.invert_geo_in_plot_at_client(&projection, client)?;
                let count = output.borrow()["count"].as_u64().unwrap_or(0) + 1;
                readout.set_text_content(Some(&known.map_or_else(
                    || "Outside the selected region.".into(),
                    |p| format!("{:?}: {:.6}, {:.6}", requested.get(), p[0], p[1]),
                )));
                Ok(
                    json!({"count":count,"trusted":event.is_trusted(),"eventType":event.type_(),"pointerType":event.pointer_type(),"client":[client.0,client.1],"scene":[point.0,point.1],"default":inverse_value(default),"known":known,"requested":format!("{:?}",requested.get()),"general":general,"inPlot":in_plot}),
                )
            })();
            *output.borrow_mut() = result.unwrap_or_else(|e| json!({"error":e.to_string()}));
        }) as Box<dyn FnMut(_)>);
        for name in ["pointermove", "pointerup"] {
            surface.add_event_listener_with_callback(name, listener.as_ref().unchecked_ref())?;
        }
        ENTRIES.with(|s| {
            s.borrow_mut().push(Entry {
                chart,
                root,
                surface,
                renderer: renderer.into(),
                source: original.clone(),
                region,
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
pub fn usa_mount(phase: u8) -> std::result::Result<(), JsValue> {
    mount(phase, false, true)
}
#[wasm_bindgen]
pub fn usa_labels_mount(masked: bool) -> std::result::Result<(), JsValue> {
    mount(0, true, masked)
}
#[wasm_bindgen]
pub fn usa_region(index: u8) -> std::result::Result<(), JsValue> {
    let region = *GeoUsaRegion::ALL
        .get(usize::from(index))
        .ok_or_else(|| js("invalid USA region"))?;
    ENTRIES.with(|s| {
        for e in s.borrow().iter() {
            e.region.set(region);
        }
    });
    Ok(())
}
#[wasm_bindgen]
pub fn usa_flush() -> std::result::Result<(), JsValue> {
    ENTRIES.with(|s| {
        for e in s.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn usa_reset(renderer: &str) -> std::result::Result<(), JsValue> {
    ENTRIES.with(|s| {
        s.borrow()
            .iter()
            .find(|e| e.renderer == renderer)
            .ok_or_else(|| js("missing USA renderer"))?
            .chart
            .handle()
            .reset_view()
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn usa_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    ENTRIES.with(|s|{
        let entries=s.borrow();
        let Some(e)=entries.iter().find(|e|e.renderer==renderer) else {
            return Ok(RETIRED.with(|s|{let retired=s.borrow();let old=retired.iter().find(|e|e.0==renderer);json!({"active":false,"stopped":old.is_none_or(|e|!e.1.is_active()),"released":old.is_none_or(|e|e.2.upgrade().is_none())}).to_string()}));
        };
        let snapshot=e.chart.snapshot().map_err(js)?;
        let core=e.source.projection.usa_projection().unwrap();
        let plot=e.source.projection.plot();
        let probes=positions().iter().map(|p|Ok(match core.project(*p)? {Some(q)=>json!({"source":p,"point":q.position,"region":format!("{:?}",q.region)}),None=>Value::Null})).collect::<Result<Vec<_>>>().map_err(js)?;
        Ok(json!({"active":snapshot.active,"phase":e.source.phase,"labels":e.source.labels,"masked":e.source.masked,"width":snapshot.width,"height":snapshot.height,"revision":snapshot.revision,"pending":snapshot.pending,"hovered":snapshot.hovered,"selected":snapshot.selected,"view":{"scale":snapshot.view.scale,"x":snapshot.view.translate_x,"y":snapshot.view.translate_y},"plot":[plot.x,plot.y,plot.width,plot.height],"scale":core.scale(),"origin":core.translate(),"fitCenter":e.source.projection.fitted_raw_center(),"clips":GeoUsaRegion::ALL.map(|r|{let c=core.region_clip(r);json!({"region":format!("{:?}",r),"rect":[c.x,c.y,c.width,c.height]})}),"probes":probes,"receipt":e.receipt.borrow().clone(),"events":e.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn usa_rejected() -> bool {
    ENTRIES.with(|s| {
        s.borrow().iter().all(|e| {
            let mut invalid = e.source.scene.clone();
            invalid.marks[0].opacity = f64::NAN;
            let h = e.chart.handle();
            let ordinary = geographic_map_layout(
                &ChartConfig {
                    width: 640.,
                    height: 400.,
                    ..Default::default()
                },
                &[],
                &Default::default(),
            )
            .unwrap()
            .projection;
            e.chart.set_scene(invalid).is_err()
                && h.invert_usa_at_client(&e.source.projection, (f64::NAN, 0.))
                    .is_err()
                && h.invert_usa_in_region_at_client(
                    &e.source.projection,
                    GeoUsaRegion::Hawaii,
                    (f64::NAN, 0.),
                )
                .is_err()
                && h.invert_usa_at_client(&ordinary, (0., 0.)).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn usa_stop() {
    let entries = ENTRIES.with(|s| std::mem::take(&mut *s.borrow_mut()));
    for e in entries {
        for name in ["pointermove", "pointerup"] {
            let _ = e
                .surface
                .remove_event_listener_with_callback(name, e.listener.as_ref().unchecked_ref());
        }
        let handle = e.chart.handle();
        e.chart.stop();
        e.root.remove();
        RETIRED.with(|s| s.borrow_mut().push((e.renderer, handle, e.callback)));
    }
}
