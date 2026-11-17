//! Whole-slot Waffle business areas with independent real painted hit evidence.
use super::*;
#[path = "../../../shared/waffle_regions.rs"]
mod model;

struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}

#[wasm_bindgen]
pub fn waffle_regions_mount() -> std::result::Result<(), JsValue> {
    waffle_regions_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("waffle-regions-{renderer}"));
        root.set_attribute("style", "width:320px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let layout = model::source(0, 320.).map_err(js)?;
        let frame = waffle_area_interaction(&layout).map_err(js)?;
        let chart = BrowserChart::mount(
            root.clone(),
            frame.scene,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(240.),
                legend: false,
                tooltip: false,
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: frame.interaction,
                interaction: surface::SurfaceOptions {
                    plot: frame.plot,
                    pan: false,
                    zoom: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        chart.on_event(move |event| {
            let value = match event {
                BrowserEvent::Interaction(SurfaceEvent::PhysicalHover(hit)) => json!({"kind":"physical","hit":hit.map(|h|json!({"physical":h.physical_mark_id,"logical":h.logical_site_id,"point":h.scene_point}))}),
                BrowserEvent::Interaction(SurfaceEvent::Activated(id)) => json!({"kind":"activated","id":id}),
                BrowserEvent::Error(error) => json!({"kind":"error","error":error.to_string()}),
                _ => json!({"kind":"other","event":format!("{event:?}")}),
            };
            log.borrow_mut().push(value);
        });
        FIXTURES.with(|f| {
            f.borrow_mut().push(Fixture {
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
pub fn waffle_regions_update(
    phase: u8,
    width: f64,
    invalid: u8,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for fixture in f.borrow_mut().iter_mut() {
            let layout = model::source(phase, width).map_err(js)?;
            let mut frame = waffle_area_interaction(&layout).map_err(js)?;
            let binding = frame.interaction.as_mut().unwrap();
            match invalid {
                0 => {}
                1 => binding.regions[0].bounds[0].width = f64::NAN,
                2 => binding.regions[0].mark_id = "missing".into(),
                3 => binding.regions.push(binding.regions[0].clone()),
                4 => binding.picking_profile = PickingProfile::LegacyGeometry,
                _ => return Err(js("unknown invalid Waffle region candidate")),
            }
            let result = fixture.chart.set_layout(frame);
            if invalid == 0 {
                result.map_err(js)?;
                fixture.root.set_attribute(
                    "style",
                    &format!("width:{width}px;max-width:none;margin:12px 0"),
                )?;
                fixture.phase = phase;
            } else if result.is_ok() {
                return Err(js("invalid Waffle region candidate accepted"));
            }
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn waffle_regions_hidden(hidden: bool) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for fixture in f.borrow().iter() {
            fixture
                .chart
                .handle()
                .set_hidden(if hidden {
                    ["alpha".into()].into()
                } else {
                    Default::default()
                })
                .map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn waffle_regions_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for fixture in f.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn waffle_regions_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f| {
        let all=f.borrow();
        let fixture=all.iter().find(|f|f.renderer==renderer).ok_or_else(||js("missing Waffle region fixture"))?;
        let state=fixture.chart.snapshot().map_err(js)?;
        let layout=model::source(fixture.phase,state.width).map_err(js)?;
        let frame=waffle_area_interaction(&layout).map_err(js)?;
        let binding=frame.interaction.as_ref().unwrap();
        let active=state.interaction.map(|a|json!({"ids":a.ids,"anchor":a.anchor,"rows":a.rows.iter().map(|r|json!({"id":r.mark_id,"series":r.series,"label":r.label,"value":r.value,"formatted":r.formatted_value})).collect::<Vec<_>>()}));
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"phase":fixture.phase,
            "hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"view":[state.view.scale,state.view.translate_x,state.view.translate_y],"interaction":active,
            "regions":binding.regions.iter().map(|r|json!({"id":r.mark_id,"bounds":r.bounds.iter().map(|b|[b.x,b.y,b.width,b.height]).collect::<Vec<_>>()})).collect::<Vec<_>>(),
            "sites":binding.sites.iter().map(|r|json!({"id":r.mark_id,"series":r.series,"value":r.value})).collect::<Vec<_>>(),
            "cells":layout.cells.iter().map(|c|json!({"id":c.mark_id,"owner":c.owner,"bounds":[c.bounds.x,c.bounds.y,c.bounds.width,c.bounds.height]})).collect::<Vec<_>>(),
            "svg":kartis::render::to_svg(&layout.scene).map_err(js)?,"events":fixture.events.borrow().clone()}).to_string())
    })
}

#[wasm_bindgen]
pub fn waffle_regions_stop() {
    for fixture in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
