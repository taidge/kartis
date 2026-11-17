//! Actual BrowserChart input and retained typed source for custom Swarm ink.
use super::*;
#[path = "../../../shared/swarm_custom.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    chart: BrowserChart,
    layout: SwarmLayout,
    phase: u8,
    vertical: bool,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! { static FIXTURES: RefCell<BTreeMap<&'static str, Fixture>> = const { RefCell::new(BTreeMap::new()) }; }

#[wasm_bindgen]
pub fn swarm_custom_mount(vertical: bool, nearest: bool) -> std::result::Result<(), JsValue> {
    swarm_custom_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let layout = model::build(640., 360., vertical, 0, 0).map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("swarm-custom-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;margin:12px 0;position:relative",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(layout.interaction_binding()),
                interaction: surface::SurfaceOptions {
                    plot: Some(layout.plot),
                    picking: if nearest {
                        surface::Picking::Nearest { distance: 3. }
                    } else {
                        surface::Picking::Geometry
                    },
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
        let log = events.clone();
        chart.on_event(move |event| {
            let mut log = log.borrow_mut();
            if log.len() >= 128 {
                log.remove(0);
            }
            log.push(event_value(event));
        });
        chart.flush().map_err(js)?;
        FIXTURES.with(|s| {
            s.borrow_mut().insert(
                name,
                Fixture {
                    root,
                    chart,
                    layout,
                    phase: 0,
                    vertical,
                    events,
                },
            )
        });
    }
    Ok(())
}

/// Build every candidate before touching either mounted renderer. A factory
/// failure therefore publishes neither partial geometry nor partial source data.
#[wasm_bindgen]
pub fn swarm_custom_update(phase: u8, width: f64, invalid: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|s| {
        let mut fixtures = s.borrow_mut();
        let next: Vec<_> = fixtures
            .iter()
            .map(|(&name, f)| {
                model::build(width, 360., f.vertical, phase, invalid).map(|layout| (name, layout))
            })
            .collect::<Result<_>>()
            .map_err(js)?;
        for (name, layout) in next {
            let f = fixtures.get_mut(name).unwrap();
            f.chart.set_layout(layout.clone().into()).map_err(js)?;
            f.root.set_attribute(
                "style",
                &format!("width:{width}px;max-width:none;margin:12px 0;position:relative"),
            )?;
            f.chart.flush().map_err(js)?;
            f.layout = layout;
            f.phase = phase;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn swarm_custom_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|s| {
        for f in s.borrow().values() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn swarm_custom_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|s| {
        let fixtures = s.borrow();
        let f = fixtures.get(renderer).ok_or_else(|| js("custom Swarm is unmounted"))?;
        let state = f.chart.snapshot().map_err(js)?;
        Ok(json!({"phase":f.phase,"vertical":f.vertical,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"events":*f.events.borrow(),"sites":f.layout.sites.iter().map(|site|json!({"id":site.mark_id,"node":site.node_id,"position":site.position,"probe":model::ink_probe(site),"raw":site.raw_value,"sourceIndex":site.source_index,"inkRadius":site.ink_radius,"label":site.label})).collect::<Vec<_>>(),"metadata":f.layout.metadata.iter().map(|m|json!({"node":m.node_id,"hidden":m.hidden,"site":m.site_index,"sourceIndex":m.source_index,"raw":m.raw_value})).collect::<Vec<_>>()}).to_string())
    })
}

#[wasm_bindgen]
pub fn swarm_custom_stop() {
    for (_, f) in FIXTURES.with(|s| std::mem::take(&mut *s.borrow_mut())) {
        f.chart.stop();
        f.root.remove();
    }
}
