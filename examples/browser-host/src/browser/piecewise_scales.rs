//! Real full-breakpoint line/scatter consumers and domain relayout.
use super::*;
mod model;
use model::{Layout, config, proposal, scale_json};
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    case: u8,
    scatter: bool,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<Layout>>,
    events: Rc<RefCell<Vec<Value>>>,
    reject: Rc<Cell<bool>>,
    calls: Rc<Cell<u32>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn bind(f: &Fixture) -> Result<()> {
    let snapshot = f.chart.snapshot()?;
    let baseline = proposal(
        f.case,
        f.phase.get(),
        f.scatter,
        &config(snapshot.width, snapshot.height),
        snapshot.hidden,
        None,
    )?;
    if f.case == 6 {
        f.chart.set_layout(baseline.interaction())?;
        *f.accepted.borrow_mut() = baseline;
        return Ok(());
    }
    let controller = baseline.controller()?;
    let (case, scatter, phase) = (f.case, f.scatter, f.phase.get());
    let accepted = f.accepted.clone();
    let reject = f.reject.clone();
    let calls = f.calls.clone();
    f.chart
        .on_viewport_with_interaction(controller, ViewportSelection::Both, move |request| {
            calls.set(calls.get() + 1);
            if reject.get() {
                return Err(ChartError("piecewise fixture layout rejection".into()));
            }
            let next = proposal(
                case,
                phase,
                scatter,
                &config(request.width, request.height),
                request.hidden.clone(),
                Some((&baseline, &request)),
            )?;
            let published = next.interaction();
            *accepted.borrow_mut() = next;
            Ok(published)
        })
}
#[wasm_bindgen]
pub fn piecewise_scales_mount(
    case: u8,
    scatter: bool,
    phase: u8,
) -> std::result::Result<(), JsValue> {
    model::profiles(case, phase).map_err(js)?;
    piecewise_scales_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("piecewise-scales-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = proposal(
            case,
            phase,
            scatter,
            &config(640., 400.),
            Default::default(),
            None,
        )
        .map_err(js)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene().clone(),
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.set_layout(initial.interaction()).map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial));
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        let layout = accepted.clone();
        let handle = chart.handle();
        chart.on_event(move |event| {
            log.borrow_mut().push(json!({"event":format!("{event:?}")}));
            if let BrowserEvent::Interaction(SurfaceEvent::Brush {
                bounds,
                complete: true,
                ..
            }) = event
            {
                log.borrow_mut()
                    .push(json!({"brush":[bounds.x0,bounds.y0,bounds.x1,bounds.y1]}));
                let p = layout.borrow().plot();
                if let Err(error) = handle.request_viewport(ViewportRequest::Brush {
                    bounds: Brush {
                        x0: bounds.x0 - p.x,
                        y0: bounds.y0 - p.y,
                        x1: bounds.x1 - p.x,
                        y1: bounds.y1 - p.y,
                    },
                    axes: ViewportSelection::Both,
                }) {
                    log.borrow_mut().push(json!({"brushError":error.0}));
                }
            }
        });
        let f = Fixture {
            chart,
            root,
            renderer,
            case,
            scatter,
            phase: Rc::new(Cell::new(phase)),
            accepted,
            events,
            reject: Rc::new(Cell::new(false)),
            calls: Rc::new(Cell::new(0)),
        };
        bind(&f).map_err(js)?;
        FIXTURES.with(|s| s.borrow_mut().push(f));
    }
    Ok(())
}
#[wasm_bindgen]
pub fn piecewise_scales_phase(phase: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with(|s| {
        for f in s.borrow().iter() {
            model::profiles(f.case, phase).map_err(js)?;
            f.phase.set(phase);
            bind(f).map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn piecewise_scales_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|s| {
        for f in s.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn piecewise_scales_reset() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|s| {
        for f in s.borrow().iter().filter(|f| f.case != 6) {
            f.chart
                .handle()
                .request_viewport(ViewportRequest::Reset)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn piecewise_scales_reject(reject: bool) {
    FIXTURES.with(|s| {
        for f in s.borrow().iter() {
            f.reject.set(reject);
        }
    });
}
#[wasm_bindgen]
pub fn piecewise_scales_invalid() -> bool {
    FIXTURES.with(|s| {
        s.borrow().iter().all(|f| {
            let mut profile = model::profiles(f.case, f.phase.get()).unwrap();
            profile.x.as_mut().unwrap().range[0] = 0.1;
            let invalid_profile = line_layout_with_piecewise_scale_options(
                &config(640., 400.),
                &[],
                &Default::default(),
                &Default::default(),
                &profile,
            )
            .is_err();
            let mut scene = f.accepted.borrow().scene().clone();
            scene.marks[0].opacity = f64::NAN;
            let mut binding = f.accepted.borrow().interaction().interaction.unwrap();
            binding.sites[0].position[0] = f64::NAN;
            invalid_profile
                && f.chart.set_scene(scene).is_err()
                && f.chart
                    .set_scene_with_interaction(f.accepted.borrow().scene().clone(), binding)
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn piecewise_scales_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|slot| {
        let fixtures=slot.borrow();let Some(f)=fixtures.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};
        let snapshot=f.chart.snapshot().map_err(js)?;let l=f.accepted.borrow();let p=l.plot();let scene=l.scene();
        let baseline=proposal(f.case,f.phase.get(),f.scatter,&config(snapshot.width,snapshot.height),snapshot.hidden.clone(),None).map_err(js)?;
        let controller=f.chart.handle().viewport_controller().map_err(js)?;
        let transform=controller.as_ref().map(|c|{let t=c.transform_xy();json!({"kx":t.scale_x,"ky":t.scale_y,"tx":t.translate_x,"ty":t.translate_y})});
        let stops=controller.as_ref().map(|c|c.piecewise_domains()).transpose().map_err(js)?;
        let sites=l.sites();
        let target=sites.iter().find_map(|site| {
            let x=site["position"][0].as_f64()?;let y=site["position"][1].as_f64()?;
            if x<p.x+10.||x>p.x+p.width-10.||y<p.y+10.||y>p.y+p.height-10. {return None;}
            for (dx,dy) in [(0.,0.),(2.,0.),(-2.,0.),(2.,2.),(-2.,-2.)] {
                if [-0.75,0.,0.75].into_iter().all(|a|[-0.75,0.,0.75].into_iter().all(|b|scene.hit_test(x+dx+a,y+dy+b).is_some_and(|m|m.id==site["id"].as_str().unwrap()))) {
                    let mut target=site.clone();target["point"]=json!([x+dx,y+dy]);target["label"]=json!(scene.marks.iter().find(|m|m.id==site["id"].as_str().unwrap()).unwrap().label);return Some(target);
                }
            }None
        });
        Ok(json!({"active":snapshot.active,"revision":snapshot.revision,"pending":snapshot.pending,"width":snapshot.width,"height":snapshot.height,
            "case":f.case,"scatter":f.scatter,"phase":f.phase.get(),"calls":f.calls.get(),"hidden":snapshot.hidden,"hovered":snapshot.hovered,"selected":snapshot.selected,
            "plot":[p.x,p.y,p.width,p.height],"view":{"scale":snapshot.view.scale,"x":snapshot.view.translate_x,"y":snapshot.view.translate_y},
            "transform":transform,"stops":stops.map(|s|json!({"x":s.x,"y":s.left})),"viewportRejected":baseline.controller().is_err(),
            "xScale":scale_json(l.scale(false)),"yScale":scale_json(l.scale(true)),"baseX":scale_json(baseline.scale(false)),"baseY":scale_json(baseline.scale(true)),
            "sites":sites,"target":target,"labels":scene.labels.iter().map(|l|json!({"text":l.text,"x":l.x,"y":l.y})).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn piecewise_scales_stop() {
    let fixtures = FIXTURES.with(|s| std::mem::take(&mut *s.borrow_mut()));
    for f in fixtures {
        f.chart.stop();
        f.root.remove();
    }
}
