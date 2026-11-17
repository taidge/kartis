//! Actual BrowserNetwork retained components, physics and native input.
use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Style};
use kartis::browser::{
    BrowserNetwork, BrowserNetworkHandle, BrowserNetworkNodeWidgetOptions, BrowserNetworkOptions,
};

use super::*;
use crate::network_widget_model as model;
struct Fixture {
    key: String,
    root: web_sys::Element,
    css: web_sys::Element,
    network: BrowserNetwork,
    failure: Rc<Cell<u8>>,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    registered: Rc<Cell<u32>>,
    placements: Rc<Cell<u32>>,
    links: Rc<Cell<u32>>,
    reentrant: Rc<Cell<u32>>,
    events: Rc<RefCell<Vec<String>>>,
    observations: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn node(n: &ForceNodeState) -> Value {
    json!({"id":n.id,"label":n.label,"radius":n.radius,"position":n.position,"velocity":n.velocity,"fixed":n.fixed,"positionBits":n.position.map(|v|v.to_bits().to_string()),"velocityBits":n.velocity.map(|v|v.to_bits().to_string()),"fixedBits":n.fixed.map(|p|p.map(|v|v.to_bits().to_string()))})
}
fn datum(d: &NetworkNodeWidgetData) -> Value {
    json!({"source":d.source.iter().map(node).collect::<Vec<_>>(),"sourceIndex":d.source_index,"nodeId":d.site.node_id,"world":d.site.world_position,"position":d.site.position,"physicsRadius":d.site.physics_radius,"bounds":[d.site.bounds.x,d.site.bounds.y,d.site.bounds.width,d.site.bounds.height],"worldOffset":d.world_offset,"ticks":d.ticks,"alpha":d.alpha,"alphaBits":d.alpha.to_bits().to_string()})
}
fn rect(r: LayoutRect) -> [f64; 4] {
    [r.x, r.y, r.width, r.height]
}
fn label(
    f: &TypedPlotWidgetFrame<NetworkNodeWidgetData>,
    built: u64,
    count: u32,
    focus: bool,
) -> String {
    json!({"id":f.frame.mark.id,"data":datum(&f.data),"rect":rect(f.frame.rect),"visible":f.frame.visible.map(rect),"interactive":f.frame.interactive,"geometry":format!("{:?}",f.frame.mark.geometry),"paintValue":f.frame.mark.value,"label":f.frame.mark.label,"builtRevision":built,"count":count,"focus":focus}).to_string()
}
#[wasm_bindgen]
pub fn network_widgets_mount(key: &str) -> std::result::Result<(), JsValue> {
    let (profile, r) = key
        .split_once('-')
        .ok_or_else(|| js("bad Network Widget key"))?;
    let profile: u8 = profile.parse().map_err(js)?;
    if profile > 3 {
        return Err(js("bad Network Widget profile"));
    }
    let backend = match r {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("bad renderer")),
    };
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("network-widgets-{key}"));
    root.set_attribute(
        "style",
        "width:600px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#network-widgets-{key} .network-node-widget{{width:100%!important;height:100%!important;min-width:0!important;border:0!important;border-radius:0!important;padding:0!important;margin:0!important;box-shadow:none!important;outline:none!important;appearance:none!important;font:0px monospace!important;display:block!important;overflow:hidden!important}}")));
    document
        .query_selector("head")?
        .unwrap()
        .append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let failure = Rc::new(Cell::new(0u8));
    let calls = Rc::new(Cell::new(0u32));
    let cleanups = Rc::new(Cell::new(0u32));
    let registered = Rc::new(Cell::new(0u32));
    let registered_factory = registered.clone();
    let placements = Rc::new(Cell::new(0u32));
    let links = Rc::new(Cell::new(0u32));
    let reentrant = Rc::new(Cell::new(0u32));
    let events = Rc::new(RefCell::new(vec![]));
    let observations = Rc::new(RefCell::new(Vec::<Value>::new()));
    let observed = observations.clone();
    let owner = Rc::new(RefCell::new(None::<BrowserNetworkHandle>));
    let ph = owner.clone();
    let fh = owner.clone();
    let (fail, pcount, reentry) = (failure.clone(), placements.clone(), reentrant.clone());
    let (lfail, lcount) = (failure.clone(), links.clone());
    let (cfail, called, retired) = (failure.clone(), calls.clone(), cleanups.clone());
    let (sim, glyphs) = model::source(0).map_err(js)?;
    let network = BrowserNetwork::mount_with_link_factory_and_node_widgets(
        root.clone(),
        sim,
        model::config(600., 300.),
        BrowserNetworkOptions {
            auto_start: false,
            chart: BrowserOptions {
                renderer: backend,
                height: Some(300.),
                legend: false,
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
            ..Default::default()
        },
        glyphs,
        BrowserNetworkNodeWidgetOptions {
            revision: 0,
            replace_ink: profile.is_multiple_of(2),
            source_ordered: profile < 2,
        },
        move |ctx| {
            lcount.set(lcount.get() + 1);
            if lfail.get() == 5 {
                return Err(ChartError("Network Widget link rejected".into()));
            }
            // Explicit fixture profile: a wide link has a solid-color interior
            // at DPR1/2 and fractional CSS scale; original thin-edge AA is separate.
            let mut ink = ctx.inherit(ctx.geometry.clone());
            if let Some(stroke) = &mut ink.stroke {
                stroke.width = 6.;
            }
            Ok(ink)
        },
        move |ctx| {
            pcount.set(pcount.get() + 1);
            if ctx.node.id == "b" && fail.get() == 1 {
                return Err(ChartError("Network Widget placement rejected".into()));
            }
            if ctx.node.id == "a"
                && matches!(fail.get(), 6..=8)
                && let Some(h) = ph.borrow().as_ref()
            {
                match fail.get() {
                    6 => {
                        h.snapshot()?;
                        reentry.set(reentry.get() + 1);
                    }
                    7 => h.stop(),
                    8 => {
                        let (s, g) = model::source(1)?;
                        if h.replace_simulation_with_glyphs(s, g).is_err() {
                            reentry.set(reentry.get() + 1);
                        } else {
                            return Err(ChartError("nested Network replacement accepted".into()));
                        }
                    }
                    _ => {}
                }
            }
            if ctx.node.id == "b" && fail.get() == 2 {
                return Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                    x: f64::NAN,
                    y: 30.,
                    width: 32.,
                    height: 22.,
                })));
            }
            model::placement(ctx)
        },
        move |context: PlotWidgetCompositionContext<NetworkNodeWidgetData>| {
            called.set(called.get() + 1);
            let frame = context.frame;
            let built = context.revision;
            let b = frame.with(|f| f.data.site.node_id == "b");
            if b && cfail.get() == 3 {
                return Err(ChartError("Network Widget factory rejected".into()));
            }
            if b && matches!(cfail.get(), 9 | 11)
                && let Some(h) = fh.borrow().as_ref()
            {
                if cfail.get() == 9 {
                    h.stop();
                } else {
                    h.snapshot()?;
                }
            }
            let build_failure = b && cfail.get() == 4;
            let stop = b && cfail.get() == 10;
            let handle = fh.clone();
            let probe = cfail.clone();
            let observed = observed.clone();
            let retired = retired.clone();
            let registered = registered_factory.clone();
            Ok(glory::ui::widget(move |scope| {
                let retired = retired.clone();
                scope.on_cleanup(move || retired.set(retired.get() + 1))?;
                registered.set(registered.get() + 1);
                let probe_handle = handle.clone();
                let probe = probe.clone();
                let observed = observed.clone();
                let count = scope.cage(0u32);
                let focus = scope.cage(false);
                glory::ui::button()
                    .class("network-node-widget")
                    .style(scope.bond(move || {
                        frame.with(|f| {
                            let color = if f.data.ticks.is_multiple_of(2) {
                                Color(0, 100, 255, 255)
                            } else {
                                Color(0, 200, 0, 255)
                            };
                            Style {
                                background: Some(color),
                                color: Some(color),
                                ..Style::new()
                                    .width(Length::Percent(100.))
                                    .height(Length::Percent(100.))
                            }
                        })
                    }))
                    .label(
                        scope.bond(move || {
                            frame.with(|f| {
                                if probe.get()==12 && observed.borrow().len()<2048 && let Some(h)=probe_handle.borrow().as_ref() {
                                    let snapshot=match h.snapshot() {Ok(s)=>json!({"ticks":s.ticks,"alphaBits":s.alpha.to_bits().to_string(),"nodes":s.nodes.iter().map(node).collect::<Vec<_>>(),"chartSize":s.chart.map(|c|[c.width,c.height])}),Err(e)=>json!({"error":e.0})};
                                    observed.borrow_mut().push(json!({"nodeId":f.data.site.node_id,"ticks":f.data.ticks,"alphaBits":f.data.alpha.to_bits().to_string(),"source":f.data.source.iter().map(node).collect::<Vec<_>>(),"snapshot":snapshot}));
                                }
                                label(f, built, count.get(), focus.get())
                            })
                        }),
                    )
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .on(EventKind::FocusChanged, move |e| {
                        if let UiEvent::FocusChanged(v) = e {
                            focus.revise(|f| *f = v)
                        }
                    })
                    .show_in(scope)?;
                if stop && let Some(h) = handle.borrow().as_ref() {
                    h.stop();
                }
                if build_failure {
                    return Err(glory::render::Error::InvalidProps(
                        "Network Widget build rejected",
                    )
                    .into());
                }
                Ok(())
            }))
        },
    );
    let network = match network {
        Ok(n) => n,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    *owner.borrow_mut() = Some(network.handle());
    let errors = events.clone();
    network.on_event(move |event| {
        if let BrowserEvent::Error(e) = event {
            errors.borrow_mut().push(e.0);
        }
    });
    network.flush().map_err(js)?;
    FIXTURES.with(|f| {
        f.borrow_mut().push(Fixture {
            key: key.into(),
            root,
            css,
            network,
            failure,
            calls,
            cleanups,
            registered,
            placements,
            links,
            reentrant,
            events,
            observations,
        })
    });
    Ok(())
}
fn with_entry<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with(|all| {
        let all = all.borrow();
        let e = all
            .iter()
            .find(|e| e.key == key)
            .ok_or_else(|| js("missing Network Widget host"))?;
        f(e)
    })
}
#[wasm_bindgen]
pub fn network_widgets_failure(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
        e.failure.set(mode);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn network_widgets_replace(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
        let (s, g) = model::source(phase).map_err(js)?;
        e.network
            .handle()
            .replace_simulation_with_glyphs(s, g)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_widgets_revision(key: &str, revision: u32) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
        e.network
            .set_node_widget_revision(revision.into())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_widgets_heat(key: &str, alpha: f64) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| e.network.handle().reheat(alpha).map_err(js))
}
#[wasm_bindgen]
pub fn network_widgets_flush(key: &str) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| e.network.flush().map_err(js))
}
#[wasm_bindgen]
pub fn network_widgets_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
        use wasm_bindgen::JsCast;
        e.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))
    })
}
#[wasm_bindgen]
pub fn network_widgets_hidden(key: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
        e.network
            .handle()
            .set_hidden(if hidden {
                ["a".into(), "b".into(), "c".into(), "z".into()].into()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_widgets_view(
    key: &str,
    scale: f64,
    x: f64,
    y: f64,
) -> std::result::Result<(), JsValue> {
    with_entry(key, |e| {
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
pub fn network_widgets_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with_entry(key, |e| {
        let s = e.network.snapshot().map_err(js)?;
        let Some(c) = s.chart else {
            return Ok(json!({"active":false,"calls":e.calls.get(),"cleanups":e.cleanups.get(),"error":s.error.map(|v|v.0)}).to_string());
        };
        let offset = [c.width / 2., c.height / 2.];
        Ok(json!({"active":s.active,"width":c.width,"height":c.height,"revision":c.revision,"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"nodes":s.nodes.iter().map(node).collect::<Vec<_>>(),"offset":offset,"ticks":s.ticks,"alpha":s.alpha,"alphaBits":s.alpha.to_bits().to_string(),"alphaTarget":s.alpha_target,"pending":s.pending,"raf":s.raf_pending,"capture":s.captured,"dragging":s.dragging.map(|v|v.node_id),"suspended":s.suspended,"error":s.error.map(|v|v.0),"selected":c.selected,"hovered":c.hovered,"hidden":c.hidden,"calls":e.calls.get(),"cleanups":e.cleanups.get(),"placements":e.placements.get(),"links":e.links.get(),"reentrant":e.reentrant.get(),"events":e.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn network_widgets_stop(key: &str) -> std::result::Result<u32, JsValue> {
    FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        let i = all
            .iter()
            .position(|e| e.key == key)
            .ok_or_else(|| js("missing Network Widget host"))?;
        let e = all.remove(i);
        e.network.stop();
        e.root.remove();
        e.css.remove();
        Ok(e.cleanups.get())
    })
}

#[wasm_bindgen]
pub fn network_widgets_diagnostics(key: &str) -> std::result::Result<String, JsValue> {
    with_entry(key, |e| {
        Ok(json!({"calls":e.calls.get(),"placements":e.placements.get(),"links":e.links.get(),"cleanups":e.cleanups.get(),"registered":e.registered.get(),"reentrant":e.reentrant.get(),"observations":e.observations.borrow().clone()}).to_string())
    })
}

struct ResizeControl {
    root: web_sys::Element,
    chart: BrowserChart,
}
thread_local! {static RESIZE_CONTROLS:RefCell<BTreeMap<String,ResizeControl>>=const{RefCell::new(BTreeMap::new())};}
#[wasm_bindgen]
pub fn network_widgets_resize_control_mount(key: &str) -> std::result::Result<(), JsValue> {
    let (profile, r) = key.split_once('-').ok_or_else(|| js("bad control key"))?;
    let profile: u8 = profile.parse().map_err(js)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div")?;
    root.set_id(&format!("network-resize-control-{key}"));
    root.set_attribute(
        "style",
        "width:640px;max-width:none;margin:12px 0;transform-origin:0 0;transform:scale(1.25)",
    )?;
    document.body().unwrap().append_child(&root)?;
    let (s, g) = model::source(0).map_err(js)?;
    let l = s
        .scene_with_glyphs(&model::config(640., 300.), &g)
        .map_err(js)?;
    let widgets = network_node_widgets(&l, &s, 0, model::placement).map_err(js)?;
    let replace_ink = if profile.is_multiple_of(2) {
        widgets
            .layout
            .slots
            .iter()
            .map(|s| s.mark_id.clone())
            .collect()
    } else {
        Default::default()
    };
    let frame = network_widget_frame(&l).map_err(js)?;
    let options = BrowserOptions {
        renderer: if r == "svg" {
            BrowserRenderer::Svg
        } else {
            BrowserRenderer::Canvas
        },
        height: Some(300.),
        legend: false,
        interaction_data: frame.interaction,
        motion: browser::MotionPreference::Reduced,
        highlight: browser::BrowserHighlight::Controlled,
        ..Default::default()
    };
    let composition = PlotWidgetsComposition {
        widgets,
        replace_ink,
    };
    let factory = |ctx: PlotWidgetCompositionContext<NetworkNodeWidgetData>| {
        let frame = ctx.frame;
        Ok(glory::ui::widget(move |scope| {
            glory::ui::button()
                .class("network-resize-control-child")
                .label(scope.bond(move || frame.with(|f| label(f, 0, 0, false))))
                .show_in(scope)
        }))
    };
    let chart = if profile < 2 {
        BrowserChart::mount_with_plot_widgets_in_scene(
            root.clone(),
            frame.scene,
            options,
            composition,
            factory,
        )
    } else {
        BrowserChart::mount_with_plot_widgets_composed(
            root.clone(),
            frame.scene,
            options,
            composition,
            factory,
        )
    }
    .map_err(js)?;
    RESIZE_CONTROLS.with(|all| {
        all.borrow_mut()
            .insert(key.into(), ResizeControl { root, chart })
    });
    Ok(())
}
#[wasm_bindgen]
pub fn network_widgets_resize_control_width(
    key: &str,
    width: f64,
) -> std::result::Result<(), JsValue> {
    use wasm_bindgen::JsCast;
    RESIZE_CONTROLS.with(|all| {
        let all = all.borrow();
        let e = all.get(key).ok_or_else(|| js("missing resize control"))?;
        e.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))
    })
}
#[wasm_bindgen]
pub fn network_widgets_resize_control_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    RESIZE_CONTROLS.with(|all|{let all=all.borrow();let e=all.get(key).ok_or_else(||js("missing resize control"))?;e.chart.flush().map_err(js)?;let s=e.chart.snapshot().map_err(js)?;Ok(json!({"width":s.width,"height":s.height,"revision":s.revision,"view":[s.view.scale,s.view.translate_x,s.view.translate_y]}).to_string())})
}
#[wasm_bindgen]
pub fn network_widgets_resize_control_stop(key: &str) -> std::result::Result<(), JsValue> {
    RESIZE_CONTROLS.with(|all| {
        let e = all
            .borrow_mut()
            .remove(key)
            .ok_or_else(|| js("missing resize control"))?;
        e.chart.stop();
        e.root.remove();
        Ok(())
    })
}
