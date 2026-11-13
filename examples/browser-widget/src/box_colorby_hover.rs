//! Explicit retained hosts for the shared controlled BoxPlot application.
#[path = "../../shared/box_colorby_hover.rs"]
mod model;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use glory::web::WebApp;
use glory::{Cage, Widget};
use kartis::browser::*;
use kartis::raster_surface::*;
use kartis::render::raster::*;
use kartis::*;
use serde_json::{Value, json};
use wasm_bindgen::prelude::*;

type Events = Rc<RefCell<Vec<Value>>>;
enum Host {
    Chart { web: WebApp, scene: Cage<Scene> },
    Browser(BrowserChart),
    Raster { web: WebApp, surface: RasterSurface },
}
struct Mounted {
    id: String,
    kind: String,
    case: u32,
    phase: u32,
    root: web_sys::Element,
    frame: model::Frame,
    host: Host,
    events: Events,
}
enum RetiredHost {
    Chart,
    Browser(BrowserHandle),
    Raster(RasterSurfaceHandle),
}
struct Retired {
    id: String,
    kind: String,
    root: web_sys::Element,
    events: Events,
    events_at_dispose: usize,
    host: RetiredHost,
}
thread_local! {
    static MOUNTS: RefCell<BTreeMap<String, Mounted>> = const { RefCell::new(BTreeMap::new()) };
    static RETIRED: RefCell<Vec<Retired>> = const { RefCell::new(Vec::new()) };
}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn log(events: &Events, value: Value) {
    let mut rows = events.borrow_mut();
    if rows.len() == 512 {
        rows.remove(0);
    }
    rows.push(value);
}
fn fonts() -> Result<FontRegistry> {
    let mut fonts = FontRegistry::new();
    let face = fonts.register(
        include_bytes!("../../../tests/fonts/Lato-Regular.ttf").as_slice(),
        0,
    )?;
    fonts.bind_face(
        face,
        FaceDescriptor {
            family: FontFamily::Named("Lato".into()),
            weight: 400.,
        },
    )?;
    Ok(fonts)
}
fn ui_error(error: impl std::fmt::Display) -> glory::ui::Error {
    // Keep the actual error in the event log before returning Glory's typed
    // mount error; the static message never pretends the candidate succeeded.
    let _ = error;
    glory::render::Error::InvalidProps("box-colorby-hover raster fixture mount rejected").into()
}
fn mount_host(
    root: web_sys::Element,
    kind: &str,
    frame: &model::Frame,
    events: &Events,
) -> std::result::Result<Host, JsValue> {
    match kind {
        "chart" => {
            let mut source = None;
            let initial = frame.layout.scene.clone();
            let selected = events.clone();
            let hovered = events.clone();
            let focused = events.clone();
            let errors = events.clone();
            let document = root
                .owner_document()
                .ok_or_else(|| js("missing host document"))?;
            let measure = BrowserSemanticTextMeasurer::new(&document).map_err(js)?;
            let web = glory::web::launch_with(root, |scope| {
                let scene = scope.cage(initial);
                source = Some(scene);
                chart(scene)
                    .highlight(false)
                    .legend(false)
                    .tooltip(false)
                    .semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                        measure.measure(request)
                    })
                    .on_select(move |m| {
                        log(
                            &selected,
                            json!({"event":"select","id":m.id,"value":m.value}),
                        )
                    })
                    .on_hover(move |m| log(&hovered, json!({"event":"hover","id":m.map(|m|m.id)})))
                    .on_focus(move |m| log(&focused, json!({"event":"focus","id":m.map(|m|m.id)})))
                    .on_reject(move |e| log(&errors, json!({"event":"reject","error":e.0})))
            })
            .map_err(js)?;
            Ok(Host::Chart {
                web,
                scene: source.ok_or_else(|| js("missing chart source"))?,
            })
        }
        "svg" | "canvas" => {
            let chart = BrowserChart::mount(
                root,
                frame.layout.scene.clone(),
                BrowserOptions {
                    renderer: if kind == "svg" {
                        BrowserRenderer::Svg
                    } else {
                        BrowserRenderer::Canvas
                    },
                    motion: MotionPreference::Reduced,
                    highlight: BrowserHighlight::Controlled,
                    legend: false,
                    tooltip: false,
                    interaction_data: frame.layout.interaction.clone(),
                    interaction: surface::SurfaceOptions {
                        plot: frame.layout.plot,
                        zoom: false,
                        pan: false,
                        pinch: false,
                        brush: false,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .map_err(js)?;
            let observed = events.clone();
            chart.on_event(move |e| {
                log(&observed, json!({"event":"browser","raw":format!("{e:?}")}))
            });
            chart.flush().map_err(js)?;
            Ok(Host::Browser(chart))
        }
        "raster" => {
            let retained = Rc::new(RefCell::new(None));
            let output = retained.clone();
            let initial = frame.layout.clone();
            let fonts = fonts().map_err(js)?;
            let observations = events.clone();
            let failures = events.clone();
            let mounts = events.clone();
            let web = glory::web::launch_with(root, move |_| {
                glory::ui::widget(move |scope| {
                    let surface = RasterSurface::mount_layout(
                        scope,
                        initial.clone(),
                        fonts.clone(),
                        RasterSurfaceOptions {
                            highlight: false,
                            ..Default::default()
                        },
                    )
                    .map_err(|e| {
                        log(&mounts, json!({"event":"raster-mount-error","error":e.0}));
                        ui_error("raster mount")
                    })?;
                    let observations = observations.clone();
                    surface
                        .on_event(move |e| {
                            log(
                                &observations,
                                json!({"event":"raster","raw":format!("{e:?}")}),
                            )
                        })
                        .map_err(ui_error)?;
                    let failures = failures.clone();
                    surface
                        .on_error(move |e| {
                            log(&failures, json!({"event":"raster-error","error":e.0}))
                        })
                        .map_err(ui_error)?;
                    let widget = surface.widget();
                    *output.borrow_mut() = Some(surface);
                    widget.show_in(scope)
                })
            })
            .map_err(js)?;
            let surface = retained
                .borrow_mut()
                .take()
                .ok_or_else(|| js("missing retained raster surface"))?;
            Ok(Host::Raster { web, surface })
        }
        _ => Err(js("unknown box-colorby-hover host kind")),
    }
}
fn flush_host(host: &Host) -> std::result::Result<(), JsValue> {
    match host {
        Host::Chart { web, .. } | Host::Raster { web, .. } => web.app().flush().map_err(js),
        Host::Browser(chart) => chart.flush().map_err(js),
    }
}
fn publish(host: &Host, frame: &model::Frame) -> std::result::Result<(), JsValue> {
    match host {
        Host::Chart { web, scene } => {
            scene.revise(|old| *old = frame.layout.scene.clone());
            web.app().flush().map_err(js)
        }
        Host::Browser(chart) => {
            chart.set_layout(frame.layout.clone()).map_err(js)?;
            chart.flush().map_err(js)
        }
        Host::Raster { web, surface } => {
            surface
                .handle()
                .set_layout(frame.layout.clone())
                .map_err(js)?;
            web.app().flush().map_err(js)
        }
    }
}
fn host_facts(host: &Host) -> Value {
    match host {
        Host::Chart { .. } => json!({"kind":"semantic-chart","retainedWebApp":true}),
        Host::Browser(chart) => match chart.snapshot() {
            Ok(s) => {
                json!({"kind":"browser","active":s.active,"renderer":format!("{:?}",s.renderer),"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"animating":s.animating,"selected":s.selected,"hovered":s.hovered,"raw":format!("{s:?}")})
            }
            Err(e) => json!({"error":e.0}),
        },
        Host::Raster { surface, .. } => match surface.handle().snapshot() {
            Ok(s) => {
                json!({"kind":"raster","active":surface.handle().is_active(),"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"hovered":s.hovered,"raw":format!("{s:?}")})
            }
            Err(e) => json!({"error":e.0}),
        },
    }
}
#[wasm_bindgen]
pub fn box_colorby_hover_mount(
    id: &str,
    kind: &str,
    case: u32,
    phase: u32,
) -> std::result::Result<(), JsValue> {
    let frame = model::build(case, phase).map_err(js)?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .ok_or_else(|| js("missing box-colorby-hover root"))?;
    dispose_internal(id)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let host = mount_host(root.clone(), kind, &frame, &events)?;
    log(
        &events,
        json!({"event":"mount","case":case,"phase":phase,"kind":kind}),
    );
    MOUNTS.with(|m| {
        m.borrow_mut().insert(
            id.into(),
            Mounted {
                id: id.into(),
                kind: kind.into(),
                case,
                phase,
                root,
                frame,
                host,
                events,
            },
        )
    });
    Ok(())
}
#[wasm_bindgen]
pub fn box_colorby_hover_update(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let mut m = m.borrow_mut();
        let mounted = m
            .get_mut(id)
            .ok_or_else(|| js("box-colorby-hover root is not mounted"))?;
        let next = model::advance(&mounted.frame, phase).map_err(js)?;
        publish(&mounted.host, &next)?;
        mounted.phase = phase;
        mounted.frame = next;
        log(&mounted.events, json!({"event":"update","phase":phase}));
        Ok(())
    })
}
#[wasm_bindgen]
pub fn box_colorby_hover_try_invalid(id: &str, mode: u32) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let mut m=m.borrow_mut();
        let mounted=m.get_mut(id).ok_or_else(||js("box-colorby-hover root is not mounted"))?;
        // The real retained controller receives the operation. There is no host
        // publication on rejection; snapshots independently expose its state.
        let raw=model::try_invalid(&mut mounted.frame,mode).map_err(js)?;
        log(&mounted.events,json!({"event":"invalid-controller-operation","mode":mode,"error":raw["error"]}));
        Ok(json!({"id":id,"phase":mounted.phase,"attempt":raw,"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host)}).to_string())
    })
}
#[wasm_bindgen]
pub fn box_colorby_hover_flush(id: &str) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let m = m.borrow();
        let mounted = m
            .get(id)
            .ok_or_else(|| js("box-colorby-hover root is not mounted"))?;
        flush_host(&mounted.host)
    })
}
// This is a raw resource inventory. The independent driver owns canonical
// DOM normalization, resource hashes and actual backing-buffer readback.
fn resources(root: &web_sys::Element) -> Value {
    fn collect(node: &web_sys::Element, path: &str, out: &mut Vec<Value>) {
        let names = [
            "id",
            "class",
            "data-id",
            "data-series",
            "data-opacity-group",
            "opacity",
            "style",
            "src",
            "href",
            "width",
            "height",
            "viewBox",
            "d",
            "clip-path",
        ];
        let attrs = names
            .into_iter()
            .filter_map(|name| {
                node.get_attribute(name)
                    .map(|value| (name.to_string(), Value::String(value)))
            })
            .collect::<serde_json::Map<String, Value>>();
        out.push(json!({"path":path,"tag":node.tag_name(),"attributes":attrs}));
        let mut child = node.first_element_child();
        let mut i = 0;
        while let Some(current) = child {
            collect(&current, &format!("{path}/{i}"), out);
            child = current.next_element_sibling();
            i += 1;
        }
    }
    let mut result = Vec::new();
    collect(root, "root", &mut result);
    json!(result)
}
#[wasm_bindgen]
pub fn box_colorby_hover_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let m=m.borrow();let mounted=m.get(id).ok_or_else(||js("box-colorby-hover root is not mounted"))?;
        Ok(json!({"id":mounted.id,"kind":mounted.kind,"case":mounted.case,"phase":mounted.phase,"publishedPhase":mounted.frame.facts["phase"],"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host),"events":*mounted.events.borrow(),"rootChildren":mounted.root.child_element_count(),"rawDOM":mounted.root.outer_html(),"resources":resources(&mounted.root)}).to_string())
    })
}
fn dispose_internal(id: &str) -> std::result::Result<(), JsValue> {
    let previous = MOUNTS.with(|m| m.borrow_mut().remove(id));
    if let Some(mounted) = previous {
        let host = match &mounted.host {
            Host::Chart { web, .. } => {
                web.app().clear().map_err(js)?;
                RetiredHost::Chart
            }
            Host::Browser(chart) => {
                let weak = chart.handle();
                chart.stop();
                RetiredHost::Browser(weak)
            }
            Host::Raster { web, surface } => {
                let weak = surface.handle();
                surface.stop();
                web.app().clear().map_err(js)?;
                RetiredHost::Raster(weak)
            }
        };
        log(&mounted.events, json!({"event":"dispose"}));
        let events_at_dispose = mounted.events.borrow().len();
        RETIRED.with(|r| {
            r.borrow_mut().push(Retired {
                id: mounted.id,
                kind: mounted.kind,
                root: mounted.root,
                events: mounted.events,
                events_at_dispose,
                host,
            })
        });
        // mounted.host drops here; no retired record owns a WebApp or strong
        // BrowserChart/RasterSurface guard.
    }
    Ok(())
}
#[wasm_bindgen]
pub fn box_colorby_hover_dispose(id: &str) -> std::result::Result<(), JsValue> {
    dispose_internal(id)
}
#[wasm_bindgen]
pub fn box_colorby_hover_retired() -> String {
    RETIRED.with(|r|json!(r.borrow().iter().map(|r| {
        let host=match &r.host {
            RetiredHost::Chart=>json!({"kind":"semantic-chart","guard":"dropped after clear"}),
            RetiredHost::Browser(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)}),
            RetiredHost::Raster(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)}),
        };
        json!({"id":r.id,"kind":r.kind,"eventsAtDispose":r.events_at_dispose,"events":*r.events.borrow(),"rootChildrenNow":r.root.child_element_count(),"host":host})
    }).collect::<Vec<_>>()).to_string())
}

#[wasm_bindgen]
pub fn box_colorby_hover_schema() -> String {
    json!({"width":model::WIDTH,"height":model::HEIGHT,"cases":model::CASE_COUNT,"phases":model::PHASE_COUNT,"invalidModes":model::INVALID_COUNT,"caseNames":model::CASE_NAMES,"kinds":["chart","svg","canvas","raster"]}).to_string()
}
