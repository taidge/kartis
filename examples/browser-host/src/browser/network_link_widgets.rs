//! Real mixed node/link component stages on BrowserNetwork.
use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Style};
use kartis::browser::{
    BrowserNetwork, BrowserNetworkHandle, BrowserNetworkOptions, BrowserNetworkWidgetOptions,
};

use super::*;
#[path = "../../../shared/network_link_widgets.rs"]
mod model;
struct Fixture {
    key: String,
    root: web_sys::Element,
    css: web_sys::Element,
    network: BrowserNetwork,
    failure: Rc<Cell<u8>>,
    calls: Rc<Cell<u32>>,
    placements: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    registered: Rc<Cell<u32>>,
    observations: Rc<RefCell<Vec<Value>>>,
}
thread_local! { static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }
fn node(n: &ForceNodeState) -> Value {
    json!({"id":n.id,"label":n.label,"radius":n.radius,"position":n.position,"velocity":n.velocity,"fixed":n.fixed,"positionBits":n.position.map(|v|v.to_bits().to_string()),"velocityBits":n.velocity.map(|v|v.to_bits().to_string()),"fixedBits":n.fixed.map(|p|p.map(|v|v.to_bits().to_string()))})
}
fn datum(data: &NetworkWidgetData) -> Value {
    match data {
        NetworkWidgetData::Node(d) => {
            json!({"role":"node","source":d.source.iter().map(node).collect::<Vec<_>>(),"sourceIndex":d.source_index,"position":d.site.position,"worldOffset":d.world_offset,"ticks":d.ticks,"alphaBits":d.alpha.to_bits().to_string()})
        }
        NetworkWidgetData::Link(d) => {
            json!({"role":"link","source":d.source.iter().map(node).collect::<Vec<_>>(),"edges":d.edges.iter().map(|e|json!({"source":e.source,"target":e.target,"value":e.value,"valueBits":e.value.to_bits().to_string()})).collect::<Vec<_>>(),"sourceIndex":d.source_index,"position":d.site.position,"sourcePosition":d.site.source_position,"targetPosition":d.site.target_position,"bounds":[d.site.bounds.x,d.site.bounds.y,d.site.bounds.width,d.site.bounds.height],"worldOffset":d.world_offset,"ticks":d.ticks,"alphaBits":d.alpha.to_bits().to_string()})
        }
    }
}
fn rect(r: LayoutRect) -> [f64; 4] {
    [r.x, r.y, r.width, r.height]
}
fn key<'a>(ctx: &'a NetworkWidgetContext<'_>) -> &'a str {
    match ctx {
        NetworkWidgetContext::Node(n) => &n.site.mark_id,
        NetworkWidgetContext::Link(l) => &l.site.mark_id,
    }
}
#[wasm_bindgen]
pub fn network_link_widgets_mount(key_name: &str) -> std::result::Result<(), JsValue> {
    let (profile, backend) = key_name
        .split_once('-')
        .ok_or_else(|| js("bad component key"))?;
    let profile: u8 = profile.parse().map_err(js)?;
    if profile > 3 {
        return Err(js("bad component profile"));
    }
    let renderer = match backend {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("bad renderer")),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("network-link-widgets-{key_name}"));
    root.set_attribute(
        "style",
        "width:600px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#network-link-widgets-{key_name} .network-component{{width:100%!important;height:100%!important;min-width:0!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:0px monospace!important;display:block!important;overflow:hidden!important}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let failure = Rc::new(Cell::new(0));
    let calls = Rc::new(Cell::new(0));
    let placements = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let registered = Rc::new(Cell::new(0));
    let observations = Rc::new(RefCell::new(Vec::<Value>::new()));
    let owner = Rc::new(RefCell::new(None::<BrowserNetworkHandle>));
    let (pf, pc, ph) = (failure.clone(), placements.clone(), owner.clone());
    let lf = failure.clone();
    let (ff, fc, fh) = (failure.clone(), calls.clone(), owner.clone());
    let (retired, reg, obs) = (cleanups.clone(), registered.clone(), observations.clone());
    let (sim, glyphs) = model::source(0).map_err(js)?;
    let result = BrowserNetwork::mount_with_link_factory_and_widgets(
        root.clone(),
        sim,
        model::config(600., 300.),
        BrowserNetworkOptions {
            auto_start: false,
            chart: BrowserOptions {
                renderer,
                height: Some(300.),
                legend: false,
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
            ..Default::default()
        },
        glyphs,
        BrowserNetworkWidgetOptions {
            revision: 0,
            replace_ink: profile.is_multiple_of(2),
            source_ordered: profile < 2,
        },
        move |ctx| {
            if lf.get() == 5 {
                return Err(ChartError("Mixed link geometry rejected".into()));
            }
            model::ink(ctx)
        },
        move |ctx| {
            pc.set(pc.get() + 1);
            let id = key(ctx);
            if (pf.get() == 1 && id == "force-link:2") || (pf.get() == 2 && id == "force-node:a") {
                return Err(ChartError("Mixed placement rejected".into()));
            }
            if pf.get() == 6 && id == "force-link:0" {
                return Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                    x: f64::NAN,
                    y: 30.,
                    width: 32.,
                    height: 22.,
                })));
            }
            if pf.get() == 7
                && id == "force-link:0"
                && let Some(h) = ph.borrow().as_ref()
            {
                h.stop();
            }
            model::placement(ctx)
        },
        move |context: PlotWidgetCompositionContext<NetworkWidgetData>| {
            fc.set(fc.get() + 1);
            let id = context.mark_id.clone();
            if ff.get() == 3 && id == "force-link:2" {
                return Err(ChartError("Mixed factory rejected".into()));
            }
            let reject = ff.get() == 4 && id == "force-node:a";
            let (retired, reg, obs, probe, handle) = (
                retired.clone(),
                reg.clone(),
                obs.clone(),
                ff.clone(),
                fh.clone(),
            );
            let frame = context.frame;
            let built = context.revision;
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                reg.set(reg.get() + 1);
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                let (probe, handle, obs) = (probe.clone(), handle.clone(), obs.clone());
                glory::ui::button().class("network-component")
                    .style(scope.bond(move || frame.with(|f| {
                        let data=datum(&f.data); let color=if data["ticks"].as_u64().unwrap().is_multiple_of(2) { Color(0,100,255,255) } else { Color(0,200,0,255) };
                        Style { background:Some(color),color:Some(color),..Style::new().width(Length::Percent(100.)).height(Length::Percent(100.)) }
                    })))
                    .label(scope.bond(move ||frame.with(|f| {
                        let d=datum(&f.data);
                        if probe.get()==8 && let Some(h)=handle.borrow().as_ref() && let Ok(s)=h.snapshot() {
                            obs.borrow_mut().push(json!({"id":f.frame.mark.id,"data":d,"ownerSource":s.nodes.iter().map(node).collect::<Vec<_>>(),"ownerTicks":s.ticks,"ownerAlphaBits":s.alpha.to_bits().to_string()}));
                        }
                        json!({"id":f.frame.mark.id,"data":d,"rect":rect(f.frame.rect),"visible":f.frame.visible.map(rect),"interactive":f.frame.interactive,"paintValue":f.frame.mark.value,"geometry":format!("{:?}",f.frame.mark.geometry),"builtRevision":built,"count":count.get(),"focus":focus.get()}).to_string()
                    })))
                    .on(EventKind::Activate, move |_|count.revise(|v|*v+=1))
                    .on(EventKind::FocusChanged,move |e| { if let UiEvent::FocusChanged(v)=e { focus.revise(|f|*f=v); } }).show_in(scope)?;
                if reject {
                    return Err(glory::render::Error::InvalidProps("Mixed build rejected").into());
                }
                Ok(())
            }))
        },
    );
    let network = match result {
        Ok(n) => n,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    *owner.borrow_mut() = Some(network.handle());
    network.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().push(Fixture {
            key: key_name.into(),
            root,
            css,
            network,
            failure,
            calls,
            placements,
            cleanups,
            registered,
            observations,
        })
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with(|all| {
        let all = all.borrow();
        f(all
            .iter()
            .find(|e| e.key == key)
            .ok_or_else(|| js("missing mixed host"))?)
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_failure(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.failure.set(mode);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |e| e.network.flush().map_err(js))
}
#[wasm_bindgen]
pub fn network_link_widgets_revision(key: &str, rev: u32) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.network.set_widget_revision(rev.into()).map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_heat(key: &str, alpha: f64) -> std::result::Result<(), JsValue> {
    with(key, |e| e.network.handle().reheat(alpha).map_err(js))
}
#[wasm_bindgen]
pub fn network_link_widgets_replace(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        let (s, g) = model::source(phase).map_err(js)?;
        e.network
            .handle()
            .replace_simulation_with_glyphs(s, g)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_view(
    key: &str,
    scale: f64,
    x: f64,
    y: f64,
) -> std::result::Result<(), JsValue> {
    with(key, |e| {
        e.network
            .handle()
            .set_view(ZoomTransform {
                scale,
                translate_x: x,
                translate_y: y,
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |e| {
        let s = e.network.snapshot().map_err(js)?;
        let c = s.chart;
        Ok(json!({"active":s.active,"chart":c.as_ref().map(|c|json!({"width":c.width,"height":c.height,"revision":c.revision,"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"selected":c.selected})),"nodes":s.nodes.iter().map(node).collect::<Vec<_>>(),"ticks":s.ticks,"alphaBits":s.alpha.to_bits().to_string(),"pending":s.pending,"raf":s.raf_pending,"dragging":s.dragging.map(|d|d.node_id),"capture":s.captured,"calls":e.calls.get(),"placements":e.placements.get(),"cleanups":e.cleanups.get(),"registered":e.registered.get(),"observations":if e.failure.get()==8 {e.observations.borrow().clone()} else {vec![]}}).to_string())
    })
}
#[wasm_bindgen]
pub fn network_link_widgets_stop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        let index = all
            .iter()
            .position(|e| e.key == key)
            .ok_or_else(|| js("missing mixed host"))?;
        let e = all.remove(index);
        e.network.stop();
        e.root.remove();
        e.css.remove();
        Ok(json!({"cleanups":e.cleanups.get(),"registered":e.registered.get()}).to_string())
    })
}
