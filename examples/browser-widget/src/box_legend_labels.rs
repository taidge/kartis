//! Actual four-host caption controls. Legend item identity never comes from display text.
#[path = "../../shared/box_legend_labels.rs"]
mod model;

use glory::{Cage, Widget, web::WebApp};
use kartis::{browser::*, raster_surface::*, render::raster::*, *};
use serde_json::{Value, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

type Events = Rc<RefCell<Vec<Value>>>;
enum Host {
    Chart {
        web: WebApp,
        scene: Cage<Scene>,
        hidden: Cage<BTreeSet<String>>,
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
    actions: bool,
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
    rows.push(value);
}
fn activation(event: LegendActivation) -> Value {
    json!({"event":"legend-activation","id":event.item.id,"label":event.item.label,"rgba":model::rgba(event.item.color),"role":format!("{:?}",event.role)})
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
    glory::render::Error::InvalidProps("box-legend-labels raster fixture mount rejected").into()
}
fn mount_host(
    root: web_sys::Element,
    kind: &str,
    frame: &model::Frame,
    events: &Events,
    actions: bool,
) -> std::result::Result<Host, JsValue> {
    match kind {
        "chart" => {
            let mut source = None;
            let mut hidden_source = None;
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
                let hidden = scope.cage(BTreeSet::new());
                hidden_source = Some(hidden);
                let hidden_events = events.clone();
                let mut chart = chart(scene)
                    .hidden(hidden)
                    .highlight(false)
                    .legend(true)
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
                    .on_hidden_change(move |proposal| {
                        log(
                            &hidden_events,
                            json!({"event":"hidden-proposal","hidden":proposal}),
                        );
                        hidden.revise(|old| *old = proposal);
                    });
                if actions {
                    let activations = events.clone();
                    chart =
                        chart.on_legend_activate(move |event| log(&activations, activation(event)));
                }
                chart
            })
            .map_err(js)?;
            Ok(Host::Chart {
                web,
                scene: source.ok_or_else(|| js("missing chart source"))?,
                hidden: hidden_source.ok_or_else(|| js("missing semantic hidden state"))?,
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
                    legend: true,
                    legend_actions: actions,
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
                log(
                    &observed,
                    match e {
                        BrowserEvent::LegendActivated(event) => activation(event),
                        other => json!({"event":"browser","raw":format!("{other:?}")}),
                    },
                )
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
            let legend_events = events.clone();
            let web = glory::web::launch_with(root, move |_| {
                glory::ui::widget(move |scope| {
                    let surface = RasterSurface::mount_layout(
                        scope,
                        initial.clone(),
                        fonts.clone(),
                        RasterSurfaceOptions {
                            highlight: false,
                            legend_actions: actions,
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
                    let activations = legend_events.clone();
                    surface
                        .on_legend_activate(move |event| log(&activations, activation(event)))
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
        _ => Err(js("unknown box-legend-labels host kind")),
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
        Host::Chart { web, scene, .. } => {
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
        Host::Chart { hidden, .. } => {
            json!({"kind":"semantic-chart","retainedWebApp":true,"hidden":hidden.get_untracked()})
        }
        Host::Browser(chart) => match chart.snapshot() {
            Ok(s) => {
                json!({"kind":"browser","active":s.active,"renderer":format!("{:?}",s.renderer),"width":s.width,"height":s.height,"revision":s.revision,"pending":s.pending,"animating":s.animating,"selected":s.selected,"hovered":s.hovered,"hidden":s.hidden,"raw":format!("{s:?}")})
            }
            Err(e) => json!({"error":e.0}),
        },
        Host::Raster { surface, .. } => match surface.handle().snapshot() {
            Ok(s) => {
                json!({"kind":"raster","active":surface.handle().is_active(),"width":s.width,"height":s.height,"revision":s.revision,"selected":s.selected,"hovered":s.hovered,"hidden":s.hidden,"raw":format!("{s:?}")})
            }
            Err(e) => json!({"error":e.0}),
        },
    }
}
#[wasm_bindgen]
pub fn box_legend_labels_mount(
    id: &str,
    kind: &str,
    case: u32,
    phase: u32,
    actions: bool,
) -> std::result::Result<(), JsValue> {
    let frame = model::build(case, phase).map_err(js)?;
    let root = web_sys::window()
        .and_then(|w| w.document())
        .and_then(|d| d.get_element_by_id(id))
        .ok_or_else(|| js("missing box-legend-labels root"))?;
    dispose_internal(id)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let host = mount_host(root.clone(), kind, &frame, &events, actions)?;
    log(
        &events,
        json!({"event":"mount","case":case,"phase":phase,"kind":kind,"actions":actions}),
    );
    MOUNTS.with(|m| {
        m.borrow_mut().insert(
            id.into(),
            Mounted {
                id: id.into(),
                kind: kind.into(),
                case,
                phase,
                actions,
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
pub fn box_legend_labels_update(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let mut m = m.borrow_mut();
        let mounted = m
            .get_mut(id)
            .ok_or_else(|| js("box-legend-labels root is not mounted"))?;
        let next = model::advance(&mounted.frame, phase).map_err(js)?;
        publish(&mounted.host, &next)?;
        mounted.phase = phase;
        mounted.frame = next;
        log(&mounted.events, json!({"event":"update","phase":phase}));
        Ok(())
    })
}
#[wasm_bindgen]
pub fn box_legend_labels_try_invalid(id: &str, mode: u32) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let mut m=m.borrow_mut();
        let mounted=m.get_mut(id).ok_or_else(||js("box-legend-labels root is not mounted"))?;
        // The real retained controller receives the operation. There is no host
        // publication on rejection; snapshots independently expose its state.
        let raw=model::try_invalid(&mut mounted.frame,mode).map_err(js)?;
        log(&mounted.events,json!({"event":"invalid-controller-operation","mode":mode,"error":raw["error"]}));
        Ok(json!({"id":id,"phase":mounted.phase,"attempt":raw,"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host)}).to_string())
    })
}
#[wasm_bindgen]
pub fn box_legend_labels_flush(id: &str) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|m| {
        let m = m.borrow();
        let mounted = m
            .get(id)
            .ok_or_else(|| js("box-legend-labels root is not mounted"))?;
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
pub fn box_legend_labels_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|m| {
        let m=m.borrow();let mounted=m.get(id).ok_or_else(||js("box-legend-labels root is not mounted"))?;
        Ok(json!({"id":mounted.id,"kind":mounted.kind,"case":mounted.case,"phase":mounted.phase,"actions":mounted.actions,"publishedPhase":mounted.frame.facts["phase"],"facts":model::observe(&mounted.frame),"host":host_facts(&mounted.host),"events":*mounted.events.borrow(),"rootChildren":mounted.root.child_element_count(),"rawDOM":mounted.root.outer_html(),"resources":resources(&mounted.root)}).to_string())
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
pub fn box_legend_labels_dispose(id: &str) -> std::result::Result<(), JsValue> {
    dispose_internal(id)
}
#[wasm_bindgen]
pub fn box_legend_labels_retired() -> String {
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
pub fn box_legend_labels_schema() -> String {
    json!({"width":model::WIDTH,"height":model::HEIGHT,"size":model::SIZE,"cases":model::CASE_COUNT,"phases":model::PHASE_COUNT,"invalidModes":model::INVALID_COUNT,"caseNames":model::CASE_NAMES,"kinds":["chart","svg","canvas","raster"],"actionsProfiles":[false,true],"legendVisible":true,"nativeRasterPixelRatio":1,"font":"Lato","captionCallbacks":false,"hostHidden":"Separate accepted visibility; never inferred from caption text"}).to_string()
}

#[wasm_bindgen]
pub fn box_legend_labels_set_hidden(
    id: &str,
    subgroup: &str,
    hidden: bool,
) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|mounts| {
        let mounts = mounts.borrow();
        let mounted = mounts.get(id).ok_or_else(||js("box-legend-labels root is not mounted"))?;
        if !mounted.frame.controller.layout.subgroups.iter().any(|key|key==subgroup) {
            return Err(js("unknown original subgroup in legend fixture"));
        }
        let series = boxplot_series_id(subgroup);
        let mut keys: BTreeSet<String> = match &mounted.host {
            Host::Chart { hidden, .. } => hidden.get_untracked(),
            Host::Browser(chart) => chart.snapshot().map_err(js)?.hidden,
            Host::Raster {surface, ..} => surface.handle().snapshot().map_err(js)?.hidden,
        };
        if hidden { keys.insert(series); } else { keys.remove(&series); }
        match &mounted.host {
            Host::Chart {hidden, web, ..} => { hidden.revise(|old|*old=keys);web.app().flush().map_err(js)?; }
            Host::Browser(chart) => {chart.handle().set_hidden(keys).map_err(js)?;chart.flush().map_err(js)?;}
            Host::Raster {surface,web} => {surface.handle().set_hidden(keys).map_err(js)?;web.app().flush().map_err(js)?;}
        }
        log(&mounted.events,json!({"event":"explicit-host-hidden","rawSubgroup":subgroup,"canonicalSeriesID":boxplot_series_id(subgroup),"hidden":hidden}));
        Ok(())
    })
}
