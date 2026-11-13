//! Explicit retained hosts for the shared controlled BoxPlot application.
#[path = "../../shared/box_parts_pointer.rs"]
mod model;

use glory::{Cage, Widget, web::WebApp};
use kartis::{
    browser::*,
    raster_surface::*,
    render::raster::*,
    surface::{SurfaceEvent, SurfaceFrameToken, SurfacePhysicalHit},
    *,
};
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};
use wasm_bindgen::prelude::*;

type Events = Rc<RefCell<Vec<Value>>>;
type PendingHits = Vec<(u64, Option<SurfacePhysicalHit>)>;
#[derive(Clone)]
struct PointerState {
    pending: Rc<RefCell<PendingHits>>,
    generation: Rc<Cell<u64>>,
    previous_token: Rc<RefCell<Option<SurfaceFrameToken>>>,
}
impl PointerState {
    fn new() -> Self {
        Self {
            pending: Rc::new(RefCell::new(Vec::new())),
            generation: Rc::new(Cell::new(0)),
            previous_token: Rc::new(RefCell::new(None)),
        }
    }
}
fn physical(
    events: &Events,
    pointer: &PointerState,
    hit: Option<SurfacePhysicalHit>,
    backend: &str,
) {
    let same_token = hit.as_ref().and_then(|h| {
        pointer
            .previous_token
            .borrow()
            .as_ref()
            .map(|old| old == &h.frame_token)
    });
    if let Some(hit) = &hit {
        *pointer.previous_token.borrow_mut() = Some(hit.frame_token.clone());
    }
    log(
        events,
        json!({"event":"physical-hover","backend":backend,"generation":pointer.generation.get(),"sameTokenAsPrevious":same_token,"hit":hit.as_ref().map(model::hit_json)}),
    );
    let mut pending = pointer.pending.borrow_mut();
    if pending.len() >= 1024 {
        log(events, json!({"event":"fixture-pending-overflow"}));
        return;
    }
    pending.push((pointer.generation.get(), hit));
}
enum Host {
    Chart {
        web: WebApp,
        scene: Cage<SceneInteraction>,
    },
    Browser(BrowserChart),
    Raster {
        web: WebApp,
        surface: RasterSurface,
    },
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
    pointer: PointerState,
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
    glory::render::Error::InvalidProps("box-parts-pointer raster fixture mount rejected").into()
}
fn mount_host(
    root: web_sys::Element,
    kind: &str,
    frame: &model::Frame,
    events: &Events,
    pointer: &PointerState,
) -> std::result::Result<Host, JsValue> {
    match kind {
        "chart" => {
            let mut source = None;
            let initial = frame.layout.clone();
            let selected = events.clone();
            let hovered = events.clone();
            let focused = events.clone();
            let errors = events.clone();
            let physical_events = events.clone();
            let physical_state = pointer.clone();
            let document = root
                .owner_document()
                .ok_or_else(|| js("missing host document"))?;
            let measure = BrowserSemanticTextMeasurer::new(&document).map_err(js)?;
            let web = glory::web::launch_with(root, |scope| {
                let scene = scope.cage(initial);
                source = Some(scene);
                chart_layout(scene)
                    .data_control_pointer(false)
                    .highlight(false)
                    .legend(false)
                    .tooltip(false)
                    .semantic_text_measurer(SemanticTextProfile::default(), move |request| {
                        measure.measure(request)
                    })
                    .on_physical_hover(move |hit| {
                        physical(&physical_events, &physical_state, hit, "chart")
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
            let physical_state = pointer.clone();
            chart.on_event(move |e| {
                if let BrowserEvent::Interaction(SurfaceEvent::PhysicalHover(hit)) = &e {
                    physical(&observed, &physical_state, hit.clone(), "browser");
                }
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
            let physical_state = pointer.clone();
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
                    let physical_state = physical_state.clone();
                    surface
                        .on_event(move |e| {
                            for event in &e {
                                if let SurfaceEvent::PhysicalHover(hit) = event {
                                    physical(&observations, &physical_state, hit.clone(), "raster");
                                }
                            }
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
        _ => Err(js("unknown box-parts-pointer host kind")),
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
            scene.revise(|old| *old = frame.layout.clone());
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

fn next_generation(pointer: &PointerState) -> std::result::Result<u64, JsValue> {
    pointer
        .generation
        .get()
        .checked_add(1)
        .ok_or_else(|| js("fixture publication generation overflow"))
}
fn drain(mounted: &mut Mounted) -> std::result::Result<(), JsValue> {
    // The callbacks only queue owned snapshots. No MOUNTS/model borrow crosses
    // a callback or drives controlled updates recursively from that callback.
    for _ in 0..16 {
        flush_host(&mounted.host)?;
        let pending = std::mem::take(&mut *mounted.pointer.pending.borrow_mut());
        if pending.is_empty() {
            return Ok(());
        }
        for (generation, hit) in pending {
            if generation != mounted.pointer.generation.get() {
                log(
                    &mounted.events,
                    json!({"event":"retired-pending-observation","generation":generation,"hit":hit.as_ref().map(model::hit_json)}),
                );
                continue;
            }
            let mut next = mounted.frame.clone();
            let changed = match model::accept_physical_hover(&mut next, hit) {
                Ok(changed) => changed,
                Err(error) => {
                    log(
                        &mounted.events,
                        json!({"event":"observed-snapshot-rejected","error":error.0}),
                    );
                    continue;
                }
            };
            if changed {
                let generation = next_generation(&mounted.pointer)?;
                let previous = mounted.pointer.generation.replace(generation);
                if let Err(error) = publish(&mounted.host, &next) {
                    mounted.pointer.generation.set(previous);
                    return Err(error);
                }
                log(
                    &mounted.events,
                    json!({"event":"pointer-owner-publication","generation":generation,"active":next.facts["active"]}),
                );
            }
            mounted.frame = next;
        }
    }
    Err(js("fixture pointer publication did not settle"))
}
#[wasm_bindgen]
pub fn box_parts_pointer_mount(
    id: &str,
    kind: &str,
    case: u32,
    phase: u32,
) -> std::result::Result<(), JsValue> {
    let frame = model::build(case, phase).map_err(js)?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .ok_or_else(|| js("missing whole-parts root"))?;
    dispose_internal(id)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let pointer = PointerState::new();
    let host = mount_host(root.clone(), kind, &frame, &events, &pointer)?;
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
                pointer,
            },
        )
    });
    Ok(())
}
#[wasm_bindgen]
pub fn box_parts_pointer_flush(id: &str) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let mut m = m.borrow_mut();
        let mounted = m
            .get_mut(id)
            .ok_or_else(|| js("whole-parts root not mounted"))?;
        drain(mounted)
    })
}
#[wasm_bindgen]
pub fn box_parts_pointer_update(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let mut m = m.borrow_mut();
        let mounted = m
            .get_mut(id)
            .ok_or_else(|| js("whole-parts root not mounted"))?;
        drain(mounted)?;
        let next = model::advance(&mounted.frame, phase).map_err(js)?;
        let generation = next_generation(&mounted.pointer)?;
        let previous = mounted.pointer.generation.replace(generation);
        if let Err(error) = publish(&mounted.host, &next) {
            mounted.pointer.generation.set(previous);
            return Err(error);
        }
        mounted.phase = phase;
        mounted.frame = next;
        log(
            &mounted.events,
            json!({"event":"source-update","phase":phase,"generation":generation}),
        );
        drain(mounted)
    })
}
#[wasm_bindgen]
pub fn box_parts_pointer_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m|{let m=m.borrow();let mounted=m.get(id).ok_or_else(||js("whole-parts root not mounted"))?;
        Ok(json!({"id":mounted.id,"kind":mounted.kind,"case":mounted.case,"phase":mounted.phase,"publicationGeneration":mounted.pointer.generation.get(),"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host),"events":*mounted.events.borrow(),"pendingObservations":mounted.pointer.pending.borrow().len(),"rootChildren":mounted.root.child_element_count(),"rawDOM":mounted.root.outer_html(),"resources":resources(&mounted.root)}).to_string())
    })
}
#[wasm_bindgen]
pub fn box_parts_pointer_try_invalid(id: &str, mode: u32) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m|{let mut m=m.borrow_mut();let mounted=m.get_mut(id).ok_or_else(||js("whole-parts root not mounted"))?;
        let before=mounted.frame.controller.layout.clone();
        let result=match mode {
            0=>mounted.frame.controller.set_hovered_mark(Some("not-an-owned-source-mark")),
            1=>{let options=*mounted.frame.controller.options();mounted.frame.controller.set_options(&BoxPlotInteractionOptions{opacity:f64::NAN,..options})},
            _=>return Err(js("unknown whole-parts invalid operation")),
        };
        let error=result.err().map(|e|e.0);
        let unchanged=mounted.frame.controller.layout==before;
        log(&mounted.events,json!({"event":"invalid-controller-operation","mode":mode,"error":error}));
        Ok(json!({"mode":mode,"error":error,"layoutEqualsBefore":unchanged,"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host)}).to_string())
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
pub fn box_parts_pointer_dispose(id: &str) -> std::result::Result<(), JsValue> {
    dispose_internal(id)
}
#[wasm_bindgen]
pub fn box_parts_pointer_retired() -> String {
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
pub fn box_parts_pointer_schema() -> String {
    json!({"width":model::WIDTH,"height":model::HEIGHT,"cases":model::CASE_COUNT,"phases":model::PHASE_COUNT,"caseNames":model::CASE_NAMES,"kinds":["chart","svg","canvas","raster"],"invalidModes":2,"profile":"PaintedGeometry","hoverSource":"actual retained-host PhysicalHover only; flush drains controlled owner changes"}).to_string()
}
