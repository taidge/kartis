//! Explicit retained web hosts for the shared inherited-border application.
#[path = "../../shared/radial_box_stroke_roles.rs"]
mod model;

use glory::{Cage, Widget, web::WebApp};
use kartis::{browser::*, raster_surface::*, render::raster::*, *};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};
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
    glory::render::Error::InvalidProps("radial-box-stroke-role raster fixture mount rejected")
        .into()
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
        _ => Err(js("unknown radial-box-stroke-role host kind")),
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
pub fn radial_box_roles_mount(
    id: &str,
    kind: &str,
    case: u32,
    phase: u32,
) -> std::result::Result<(), JsValue> {
    let frame = model::build(case, phase).map_err(js)?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .ok_or_else(|| js("missing radial-box-stroke-role root"))?;
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
pub fn radial_box_roles_update(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let mut m = m.borrow_mut();
        let mounted = m
            .get_mut(id)
            .ok_or_else(|| js("radial-box-stroke-role root is not mounted"))?;
        let next = model::build(mounted.case, phase).map_err(js)?;
        publish(&mounted.host, &next)?;
        mounted.phase = phase;
        mounted.frame = next;
        log(&mounted.events, json!({"event":"update","phase":phase}));
        Ok(())
    })
}
#[wasm_bindgen]
pub fn radial_box_roles_try_invalid_profile(
    id: &str,
    mode: u32,
) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let m=m.borrow();let mounted=m.get(id).ok_or_else(||js("radial-box-stroke-role root is not mounted"))?;
        // No host publication occurs until a complete public builder succeeds.
        // Returning the actual error (or null) lets the independent driver
        // detect an unexpectedly accepted invalid profile.
        let result=model::build_invalid(mounted.case,mounted.phase,mode);
        let error=result.err().map(|e|e.0);
        log(&mounted.events,json!({"event":"invalid-profile","mode":mode,"error":error}));
        Ok(json!({"id":id,"mode":mode,"error":error,"phase":mounted.phase,"facts":mounted.frame.facts,"host":host_facts(&mounted.host)}).to_string())
    })
}
#[wasm_bindgen]
pub fn radial_box_roles_flush(id: &str) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let m = m.borrow();
        let mounted = m
            .get(id)
            .ok_or_else(|| js("radial-box-stroke-role root is not mounted"))?;
        flush_host(&mounted.host)
    })
}
#[wasm_bindgen]
pub fn radial_box_roles_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let m=m.borrow();let mounted=m.get(id).ok_or_else(||js("radial-box-stroke-role root is not mounted"))?;
        Ok(json!({"id":mounted.id,"kind":mounted.kind,"case":mounted.case,"phase":mounted.phase,"facts":mounted.frame.facts,"host":host_facts(&mounted.host),"events":*mounted.events.borrow(),"rootChildren":mounted.root.child_element_count()}).to_string())
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
pub fn radial_box_roles_dispose(id: &str) -> std::result::Result<(), JsValue> {
    dispose_internal(id)
}
#[wasm_bindgen]
pub fn radial_box_roles_retired() -> String {
    RETIRED.with(|r|json!(r.borrow().iter().map(|r| {
        let host=match &r.host {
            RetiredHost::Chart=>json!({"kind":"semantic-chart","guard":"dropped after clear"}),
            RetiredHost::Browser(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)}),
            RetiredHost::Raster(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)}),
        };
        json!({"id":r.id,"kind":r.kind,"eventsAtDispose":r.events_at_dispose,"events":*r.events.borrow(),"rootChildrenNow":r.root.child_element_count(),"host":host})
    }).collect::<Vec<_>>()).to_string())
}
