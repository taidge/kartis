//! Retained Conic map controls with independent per-surface publication.
use crate::conic_model::*;
use kartis::{browser::*, *};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, closure::Closure, prelude::*};
fn js(e: impl ToString) -> JsValue {
    JsValue::from_str(&e.to_string())
}
#[wasm_bindgen]
extern "C" {
    type PrecisePointer;
    #[wasm_bindgen(method,getter,js_name=clientX)]
    fn px(this: &PrecisePointer) -> f64;
    #[wasm_bindgen(method,getter,js_name=clientY)]
    fn py(this: &PrecisePointer) -> f64;
}
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    surface: web_sys::Element,
    name: String,
    source: Rc<RefCell<Source>>,
    errors: Rc<Cell<usize>>,
    pending: Rc<RefCell<Option<Source>>>,
    receipt: Rc<RefCell<Value>>,
    events: Rc<RefCell<Vec<String>>>,
    callback: Weak<()>,
    listener: Closure<dyn FnMut(web_sys::PointerEvent)>,
}
struct MountRoot(Option<web_sys::Element>);
impl Drop for MountRoot {
    fn drop(&mut self) {
        if let Some(root) = self.0.take() {
            root.remove();
        }
    }
}
thread_local! {static ENTRIES:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};static RETIRED:RefCell<Vec<(BrowserHandle,Weak<()>)>>=const{RefCell::new(Vec::new())};}
fn commit(
    handle: &BrowserHandle,
    accepted: &Rc<RefCell<Source>>,
    errors: &Rc<Cell<usize>>,
    next: Source,
) -> Result<()> {
    let old = errors.get();
    handle.set_layout(next.published.clone())?;
    handle.flush()?;
    if errors.get() != old {
        return Err(ChartError(
            "Conic proposal publication failed; previous source/context retained".into(),
        ));
    }
    *accepted.borrow_mut() = next;
    Ok(())
}
#[wasm_bindgen]
pub fn conic_mount() -> std::result::Result<(), JsValue> {
    mount_settings(Settings::default())
}
#[wasm_bindgen]
pub fn conic_reference_mount(profile: u8, camera: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 || camera > 2 {
        return Err(js("unknown reference profile"));
    }
    let mut s = Settings {
        width: 760.,
        height: 320.,
        diagnostic: true,
        fit_data: true,
        ..Default::default()
    };
    s.family = if profile == 3 { 0 } else { profile };
    s.parallels = if profile == 2 {
        [30., 30.]
    } else if profile == 3 {
        [29.5, 45.5]
    } else {
        [0., 60.]
    };
    s.center = [[0., 0.], [-35., -20.], [80., -30.]][camera as usize];
    s.roll = [0., -25., 30.][camera as usize];
    mount_settings(s)
}
fn mount_settings(settings: Settings) -> std::result::Result<(), JsValue> {
    let result = mount_settings_inner(settings);
    if result.is_err() {
        conic_stop();
    }
    result
}
fn mount_settings_inner(settings: Settings) -> std::result::Result<(), JsValue> {
    let first = build(&settings).map_err(js)?;
    conic_stop();
    RETIRED.with(|r| r.borrow_mut().clear());
    let doc = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = doc.create_element("section")?;
        root.set_id(&format!("conic-{name}"));
        root.set_attribute(
            "style",
            &format!(
                "width:{}px;max-width:none;position:relative;background:white;margin:12px 0",
                settings.width
            ),
        )?;
        doc.get_element_by_id("conic-charts")
            .unwrap_or_else(|| doc.body().unwrap().into())
            .append_child(&root)?;
        let mut mounting = MountRoot(Some(root.clone()));
        let chart = BrowserChart::mount(
            root.clone(),
            first.published.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(settings.height),
                legend: false,
                motion: MotionPreference::Reduced,
                highlight: BrowserHighlight::Controlled,
                interaction_data: first.published.interaction.clone(),
                interaction: surface::SurfaceOptions {
                    plot: first.published.plot,
                    pan: true,
                    zoom: true,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.flush().map_err(js)?;
        let source = Rc::new(RefCell::new(first.clone()));
        let errors = Rc::new(Cell::new(0usize));
        let pending = Rc::new(RefCell::new(None::<Source>));
        let receipt = Rc::new(RefCell::new(json!({"count":0})));
        let events = Rc::new(RefCell::new(Vec::new()));
        let token = Rc::new(());
        let callback = Rc::downgrade(&token);
        let (accepted, p, error, keeper) = (
            source.clone(),
            pending.clone(),
            errors.clone(),
            token.clone(),
        );
        let resized_receipt = receipt.clone();
        let resized_root = root.clone();
        let observed_events = events.clone();
        chart.on_event(move |event| {
            let _keep = &keeper;
            let mut log = observed_events.borrow_mut();
            if log.len() == 256 {
                log.remove(0);
            }
            log.push(format!("{event:?}"));
            drop(log);
            match event {
                BrowserEvent::Error(_) => {
                    error.set(error.get() + 1);
                    p.borrow_mut().take();
                }
                BrowserEvent::Resized { width, height } => {
                    if let Some(next) = p.borrow_mut().take()
                        && next.settings.width == width
                        && next.settings.height == height
                    {
                        *accepted.borrow_mut() = next;
                        *resized_receipt.borrow_mut() = json!({"count":0});
                        if let Ok(Some(output)) = resized_root.query_selector(".conic-readout") {
                            output.set_text_content(Some(
                                "Move over the map to read longitude and latitude.",
                            ));
                        }
                    }
                }
                _ => {}
            }
        });
        let (accepted, p, keeper) = (source.clone(), pending.clone(), token.clone());
        chart.on_resize_with_interaction(move |width, height| {
            let _keep = &keeper;
            let mut s = accepted.borrow().settings.clone();
            s.width = width;
            s.height = height;
            let next = build(&s)?;
            let out = next.published.clone();
            *p.borrow_mut() = Some(next);
            Ok(out)
        });
        let surface = root.query_selector(".kartis-surface")?.unwrap();
        let output = doc.create_element("output")?;
        output.set_attribute("aria-live", "polite")?;
        output.set_attribute("aria-label", "Geographic coordinates")?;
        output.set_class_name("conic-readout");
        output.set_text_content(Some("Move over the map to read longitude and latitude."));
        root.append_child(&output)?;
        let (handle, current, error, observed) = (
            chart.handle(),
            source.clone(),
            errors.clone(),
            receipt.clone(),
        );
        let listened_surface = surface.clone();
        let listener = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
            let _keep = &token;
            // BrowserChart receives pointerup at window. Observe it after that
            // listener has queued Up; publishing a marker before Up cancels
            // the ongoing Surface drag baseline and loses ordinary clicks.
            if event.type_() == "pointerup"
                && !event
                    .target()
                    .and_then(|t| t.dyn_into::<web_sys::Node>().ok())
                    .is_some_and(|n| listened_surface.contains(Some(&n)))
            {
                return;
            }
            if event.buttons() != 0 {
                return;
            }
            let precise: &PrecisePointer = event.unchecked_ref();
            let client = (precise.px(), precise.py());
            let result = (|| -> Result<Value> {
                handle.flush()?;
                let mut next = current.borrow().clone();
                let point = handle.scene_point_at_client(client)?;
                let ll = handle.invert_geo_in_plot_at_client(&next.context, client)?;
                let projected = ll.map(|ll| next.context.project(ll)).transpose()?.flatten();
                if next
                    .published
                    .scene
                    .marks
                    .last()
                    .is_some_and(|m| m.id == "conic-inverse-marker")
                {
                    let index = next.published.scene.marks.len() - 1;
                    next.published.scene.marks.pop();
                    next.published
                        .scene
                        .draw_order
                        .retain(|item| *item != DrawItem::Mark(index));
                }
                if let Some([x, y]) = projected {
                    let mut m = Mark::new(
                        "conic-inverse-marker",
                        Geometry::Circle { x, y, radius: 5. },
                        Color(225, 35, 75, 255),
                    );
                    m.label.clear();
                    m.clip = Some(next.context.plot());
                    m.clip_path = next.context.projection_clip_path(0.25, 4096)?.map(Box::new);
                    if !next.published.scene.draw_order.is_empty() {
                        next.published
                            .scene
                            .draw_order
                            .push(DrawItem::Mark(next.published.scene.marks.len()));
                    }
                    next.published.scene.marks.push(m);
                }
                commit(&handle, &current, &error, next)?;
                output.set_text_content(Some(&ll.map_or_else(
                    || "Outside the visible projection.".into(),
                    |[x, y]| format!("Longitude {x:.6}°, latitude {y:.6}°"),
                )));
                Ok(
                    json!({"count":observed.borrow().get("count").and_then(Value::as_u64).unwrap_or(0)+1,"trusted":event.is_trusted(),"type":event.type_(),"client":[client.0,client.1],"scene":[point.0,point.1],"position":ll,"marker":projected}),
                )
            })();
            if result.is_err() {
                output.set_text_content(Some("This point cannot be resolved at the current view."));
            }
            *observed.borrow_mut() = result.unwrap_or_else(|e| json!({"error":e.0}));
        }) as Box<dyn FnMut(_)>);
        surface
            .add_event_listener_with_callback("pointermove", listener.as_ref().unchecked_ref())?;
        web_sys::window()
            .unwrap()
            .add_event_listener_with_callback("pointerup", listener.as_ref().unchecked_ref())?;
        ENTRIES.with(|r| {
            r.borrow_mut().push(Entry {
                chart,
                root,
                surface,
                name: name.into(),
                source,
                errors,
                pending,
                receipt,
                events,
                callback,
                listener,
            })
        });
        mounting.0.take();
    }
    Ok(())
}
#[wasm_bindgen]
pub fn conic_update(
    family: u8,
    p0: f64,
    p1: f64,
    fit_data: bool,
    center: &[f64],
    roll: f64,
    hidden: bool,
) -> std::result::Result<(), JsValue> {
    if center.len() != 2 {
        return Err(js("camera needs two coordinates"));
    }
    let entries = ENTRIES.with(|r| {
        r.borrow()
            .iter()
            .map(|e| {
                (
                    e.chart.handle(),
                    e.source.clone(),
                    e.errors.clone(),
                    e.root.clone(),
                    e.receipt.clone(),
                )
            })
            .collect::<Vec<_>>()
    });
    let next = entries
        .iter()
        .map(|(_, source, _, _, _)| {
            let mut s = source.borrow().settings.clone();
            s.family = family;
            s.parallels = [p0, p1];
            s.fit_data = fit_data;
            s.center = [center[0], center[1]];
            s.roll = roll;
            s.hidden = hidden;
            build(&s)
        })
        .collect::<Result<Vec<_>>>()
        .map_err(js)?;
    for ((handle, source, errors, root, receipt), next) in entries.iter().zip(next) {
        commit(handle, source, errors, next).map_err(js)?;
        *receipt.borrow_mut() = json!({"count":0});
        if let Some(output) = root.query_selector(".conic-readout")? {
            output.set_text_content(Some("Move over the map to read longitude and latitude."));
        }
    }
    Ok(())
}
#[wasm_bindgen]
pub fn conic_flush() -> std::result::Result<(), JsValue> {
    let handles = ENTRIES.with(|r| {
        r.borrow()
            .iter()
            .map(|e| e.chart.handle())
            .collect::<Vec<_>>()
    });
    for h in handles {
        h.flush().map_err(js)?;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn conic_resize(width: f64) -> std::result::Result<(), JsValue> {
    if !width.is_finite() || !(400. ..=800.).contains(&width) {
        return Err(js("invalid Conic width"));
    }
    let roots = ENTRIES.with(|r| {
        r.borrow()
            .iter()
            .map(|e| e.root.clone())
            .collect::<Vec<_>>()
    });
    for root in roots {
        root.dyn_ref::<web_sys::HtmlElement>()
            .unwrap()
            .style()
            .set_property("width", &format!("{width}px"))?;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn conic_snapshot(name: &str) -> std::result::Result<String, JsValue> {
    ENTRIES.with(|r|{let entries=r.borrow();let Some(e)=entries.iter().find(|e|e.name==name)else{return Ok(RETIRED.with(|r|json!({"active":false,"retired":r.borrow().len(),"callbacksReleased":r.borrow().iter().all(|(_,w)|w.upgrade().is_none()),"stopped":r.borrow().iter().all(|(h,_)|!h.is_active())}).to_string()));};let s=e.source.borrow();let state=e.chart.snapshot().map_err(js)?;Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"errors":e.errors.get(),"events":e.events.borrow().clone(),"pendingResize":e.pending.borrow().is_some(),"family":s.settings.family,"diagnostic":s.settings.diagnostic,"parallels":s.settings.parallels,"fitData":s.settings.fit_data,"center":s.settings.center,"roll":s.settings.roll,"hidden":s.settings.hidden,"view":{"scale":state.view.scale,"x":state.view.translate_x,"y":state.view.translate_y},"plot":[s.context.plot().x,s.context.plot().y,s.context.plot().width,s.context.plot().height],"scale":s.context.scale(),"translate":s.context.translate(),"rawCenter":s.context.fitted_raw_center(),"maskStatus":s.mask_status,"authoritativeSVG":render::to_svg(&s.published.scene).map_err(js)?,"source":s.features.iter().map(|f|json!({"id":f.id,"value":f.value,"polygons":f.polygons})).collect::<Vec<_>>(),"sites":s.published.interaction.as_ref().unwrap().sites.iter().map(|v|json!({"id":v.mark_id,"value":v.value,"label":v.label,"position":v.position})).collect::<Vec<_>>(),"interaction":state.interaction.map(|i|json!({"ids":i.ids,"rows":i.rows.iter().map(|v|json!({"id":v.mark_id,"value":v.value})).collect::<Vec<_>>()})),"hovered":state.hovered,"selected":state.selected,"receipt":e.receipt.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn conic_stop() {
    let old = ENTRIES.with(|r| std::mem::take(&mut *r.borrow_mut()));
    for e in old {
        let _ = e.surface.remove_event_listener_with_callback(
            "pointermove",
            e.listener.as_ref().unchecked_ref(),
        );
        let _ = web_sys::window()
            .unwrap()
            .remove_event_listener_with_callback("pointerup", e.listener.as_ref().unchecked_ref());
        let h = e.chart.handle();
        e.chart.stop();
        e.root.remove();
        RETIRED.with(|r| r.borrow_mut().push((h, e.callback)));
    }
}
