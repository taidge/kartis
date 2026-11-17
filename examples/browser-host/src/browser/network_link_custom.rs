use kartis::browser::{BrowserNetwork, BrowserNetworkHandle, BrowserNetworkOptions};

use super::*;
#[path = "../../../shared/network_link_custom.rs"]
mod model;

struct Fixture {
    root: web_sys::Element,
    network: BrowserNetwork,
    renderer: &'static str,
    mode: Rc<Cell<u8>>,
    calls: Rc<Cell<u32>>,
    drops: Rc<Cell<u32>>,
    reentrant: Rc<Cell<u32>>,
}
struct Lifetime(Rc<Cell<u32>>);
impl Drop for Lifetime {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}

#[wasm_bindgen]
pub fn network_link_custom_mount() -> std::result::Result<(), JsValue> {
    network_link_custom_stop();
    let d = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = d.create_element("div")?;
        root.set_id(&format!("network-link-custom-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        d.body().unwrap().append_child(&root)?;
        let (simulation, glyphs) = model::source(0).map_err(js)?;
        let mode = Rc::new(Cell::new(0));
        let calls = Rc::new(Cell::new(0));
        let drops = Rc::new(Cell::new(0));
        let reentrant = Rc::new(Cell::new(0));
        let (fail, count, reentry) = (mode.clone(), calls.clone(), reentrant.clone());
        let handle = Rc::new(RefCell::new(None::<BrowserNetworkHandle>));
        let owner = handle.clone();
        let lifetime = Lifetime(drops.clone());
        let network = BrowserNetwork::mount_with_link_factory(
            root.clone(),
            simulation,
            model::config(640., 360.),
            BrowserNetworkOptions {
                auto_start: false,
                chart: BrowserOptions {
                    renderer: backend,
                    height: Some(360.),
                    highlight: browser::BrowserHighlight::Controlled,
                    motion: browser::MotionPreference::Reduced,
                    ..Default::default()
                },
                ..Default::default()
            },
            glyphs,
            move |ctx| {
                let _keep = &lifetime;
                count.set(count.get() + 1);
                if ctx.link_index == 1 && fail.get() == 1 {
                    return Err(ChartError(
                        "deliberate retained link factory rejection".into(),
                    ));
                }
                if ctx.link_index == 0
                    && fail.get() == 3
                    && let Some(h) = owner.borrow().as_ref()
                {
                    h.stop();
                }
                if ctx.link_index == 0
                    && fail.get() == 4
                    && let Some(h) = owner.borrow().as_ref()
                {
                    h.snapshot()?;
                    reentry.set(reentry.get() + 1);
                }
                if ctx.link_index == 0
                    && fail.get() == 5
                    && let Some(h) = owner.borrow().as_ref()
                {
                    let (s, _) = model::source(1)?;
                    if h.replace_simulation(s).is_err() {
                        reentry.set(reentry.get() + 1);
                    } else {
                        return Err(ChartError("reentrant link rendering was accepted".into()));
                    }
                }
                let mut g = model::ink(ctx)?;
                if ctx.link_index == 1 && fail.get() == 2 {
                    g.opacity = f64::NAN;
                }
                Ok(g)
            },
        )
        .map_err(js)?;
        *handle.borrow_mut() = Some(network.handle());
        network.flush().map_err(js)?;
        FIXTURES.with(|f| {
            f.borrow_mut().push(Fixture {
                root,
                network,
                renderer,
                mode,
                calls,
                drops,
                reentrant,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn network_link_custom_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        let mut failure = None;
        for entry in f.borrow().iter() {
            if entry.network.handle().is_active()
                && let Err(error) = entry.network.flush()
            {
                failure.get_or_insert(error);
            }
        }
        failure.map_or(Ok(()), |error| Err(js(error)))
    })
}
#[wasm_bindgen]
pub fn network_link_custom_mode(mode: u8) {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry.mode.set(mode);
        }
    });
}
#[wasm_bindgen]
pub fn network_link_custom_phase(phase: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            let (s, g) = model::source(phase).map_err(js)?;
            entry
                .network
                .handle()
                .replace_simulation_with_glyphs(s, g)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn network_link_custom_tick() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|f| {
        for entry in f.borrow().iter() {
            entry.network.handle().reheat(0.5).map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn network_link_custom_stop_in_preflight(renderer: &str) -> bool {
    FIXTURES.with(|f| {
        let f = f.borrow();
        let entry = f.iter().find(|e| e.renderer == renderer).unwrap();
        entry.mode.set(3);
        let (s, g) = model::source(1).unwrap();
        entry
            .network
            .handle()
            .replace_simulation_with_glyphs(s, g)
            .is_err()
    })
}
#[wasm_bindgen]
pub fn network_link_custom_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|f| {
        let f=f.borrow();let Some(e)=f.iter().find(|e|e.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};
        let s=e.network.snapshot().map_err(js)?;let Some(chart)=s.chart else{return Ok(json!({"active":false,"drops":e.drops.get(),"calls":e.calls.get()}).to_string());};
        let c=model::config(chart.width,chart.height);let offset=[c.margin.left+c.inner_width()/2.,c.margin.top+c.inner_height()/2.];
        let nodes=s.nodes.iter().map(|n|json!({"id":n.id,"label":n.label,"world":n.position,"anchor":[n.position[0]+offset[0],n.position[1]+offset[1]],"fixed":n.fixed})).collect::<Vec<_>>();
        Ok(json!({"active":s.active,"calls":e.calls.get(),"drops":e.drops.get(),"reentrant":e.reentrant.get(),"mode":e.mode.get(),"nodes":nodes,"width":chart.width,"height":chart.height,"offset":offset,"ticks":s.ticks,"pending":s.pending,"raf":s.raf_pending,"revision":chart.revision,"view":[chart.view.scale,chart.view.translate_x,chart.view.translate_y],"hovered":chart.hovered,"selected":chart.selected,"capture":s.captured,"dragging":s.dragging.map(|s|s.node_id),"error":s.error.map(|s|s.0)}).to_string())
    })
}
#[wasm_bindgen]
pub fn network_link_custom_stop() {
    for f in FIXTURES.with(|f| std::mem::take(&mut *f.borrow_mut())) {
        f.network.stop();
        f.root.remove();
    }
}
