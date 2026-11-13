//! Actual retained hosts. Callbacks enqueue owned observations; flush publishes.
#[path = "../../shared/box_colors_function.rs"]
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
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};
use wasm_bindgen::prelude::*;

type Events = Rc<RefCell<Vec<Value>>>;
enum Observation {
    Physical(Option<SurfacePhysicalHit>),
    Hidden(BTreeSet<String>),
    Event(Value),
}
type Pending = Vec<(u64, Observation)>;
#[derive(Clone)]
struct Queue {
    pending: Rc<RefCell<Pending>>,
    generation: Rc<Cell<u64>>,
    alive: Rc<Cell<bool>>,
    previous_token: Rc<RefCell<Option<SurfaceFrameToken>>>,
}
impl Queue {
    fn new() -> Self {
        Self {
            pending: Rc::new(RefCell::new(Vec::new())),
            generation: Rc::new(Cell::new(0)),
            alive: Rc::new(Cell::new(true)),
            previous_token: Rc::new(RefCell::new(None)),
        }
    }
    fn push(&self, value: Observation) {
        if !self.alive.get() {
            return;
        }
        self.pending
            .borrow_mut()
            .push((self.generation.get(), value));
    }
    fn event(&self, value: Value) {
        self.push(Observation::Event(value));
    }
    fn retire(&self) {
        self.alive.set(false);
        self.pending.borrow_mut().clear();
    }
    fn next(&self) -> std::result::Result<u64, JsValue> {
        self.generation
            .get()
            .checked_add(1)
            .ok_or_else(|| js("fixture generation overflow"))
    }
}
fn physical(queue: &Queue, hit: Option<SurfacePhysicalHit>, backend: &str) {
    if !queue.alive.get() {
        return;
    }
    let same = hit.as_ref().and_then(|h| {
        queue
            .previous_token
            .borrow()
            .as_ref()
            .map(|old| old == &h.frame_token)
    });
    if let Some(hit) = &hit {
        *queue.previous_token.borrow_mut() = Some(hit.frame_token.clone());
    }
    queue.event(json!({"event":"physical-hover","backend":backend,"generation":queue.generation.get(),"sameTokenAsPrevious":same,"hit":hit.as_ref().map(model::hit_json)}));
    queue.push(Observation::Physical(hit));
}
fn activation(value: LegendActivation) -> Value {
    json!({"event":"legend-activation","id":value.item.id,"label":value.item.label,"rgba":model::rgba(value.item.color),"role":format!("{:?}",value.role),"explicitTarget":value.target.as_ref().map(model::target_json),"target":model::target_json(&value.resolved_target())})
}
enum Host {
    Chart {
        web: Option<WebApp>,
        scene: Cage<SceneInteraction>,
        hidden: Cage<BTreeSet<String>>,
    },
    Browser(BrowserChart),
    Raster {
        web: Option<WebApp>,
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
    queue: Queue,
    host_mount_generation: u64,
    actual_mount_count: u64,
    event_history: Vec<Value>,
}
enum RetiredHost {
    Chart,
    Browser(BrowserHandle),
    Raster(RasterSurfaceHandle),
}
struct Retired {
    id: String,
    kind: String,
    reason: String,
    host_mount_generation: u64,
    root: web_sys::Element,
    events: Events,
    events_at_dispose: usize,
    root_children_at_retirement: u32,
    host: RetiredHost,
}
thread_local! {
    static MOUNTS:RefCell<BTreeMap<String,Mounted>>=const {RefCell::new(BTreeMap::new())};
    static RETIRED:RefCell<Vec<Retired>>=const {RefCell::new(Vec::new())};
}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn log(events: &Events, value: Value) {
    events.borrow_mut().push(value);
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
fn ui_error(_: impl std::fmt::Display) -> glory::ui::Error {
    glory::render::Error::InvalidProps("colors-function raster fixture mount rejected").into()
}

fn mount_host(
    root: web_sys::Element,
    kind: &str,
    frame: &model::Frame,
    queue: &Queue,
    actions: bool,
) -> std::result::Result<Host, JsValue> {
    match kind {
        "chart" => {
            let mut source = None;
            let mut hidden_source = None;
            let initial = frame.layout.clone();
            let selected = queue.clone();
            let hovered = queue.clone();
            let focused = queue.clone();
            let errors = queue.clone();
            let observations = queue.clone();
            let hide = queue.clone();
            let actions_queue = queue.clone();
            let document = root
                .owner_document()
                .ok_or_else(|| js("missing host document"))?;
            let measure = BrowserSemanticTextMeasurer::new(&document).map_err(js)?;
            let legend_measure = if frame.case == 5 {
                Some(BrowserSemanticTextMeasurer::new(&document).map_err(js)?)
            } else {
                None
            };
            let measured = queue.clone();
            let web = glory::web::launch_with(root, |scope| {
                let scene = scope.cage(initial);
                source = Some(scene);
                let hidden = scope.cage(BTreeSet::new());
                hidden_source = Some(hidden);
                let chart = chart_layout(scene)
                    .hidden(hidden)
                    .data_control_pointer(false)
                    .highlight(false)
                    .legend(true)
                    .tooltip(false)
                    .semantic_text_measurer(SemanticTextProfile::default(), move |r| {
                        measure.measure(r)
                    })
                    .on_physical_hover(move |hit| physical(&observations, hit, "chart"))
                    .on_select(move |m| {
                        selected.event(json!({"event":"select","id":m.id,"value":m.value}))
                    })
                    .on_hover(move |m| hovered.event(json!({"event":"hover","id":m.map(|m|m.id)})))
                    .on_focus(move |m| focused.event(json!({"event":"focus","id":m.map(|m|m.id)})))
                    .on_reject(move |e| errors.event(json!({"event":"reject","error":e.0})))
                    .on_hidden_change(move |keys| hide.push(Observation::Hidden(keys)));
                let chart = if let Some(measure) = legend_measure {
                    chart
                        .legend_layout(LegendLayoutOptions {
                            bounds: LayoutRect { x: 0., y: 0., width: model::WIDTH, height: 180. },
                            direction: LegendDirection::Vertical,
                            max_columns: 1,
                            ..Default::default()
                        })
                        .font_text_measurer(move |text, style, font| {
                            let request = SemanticTextRequest {
                                text: text.to_string(), size: style.size, font: font.clone(),
                                line_height: style.size * style.line_height, revision: 0,
                            };
                            let measurement = measure.measure(&request)?;
                            let metrics = measurement.metrics();
                            measured.event(json!({"event":"legend-measurement","text":text,"font":format!("{font:?}"),"size":style.size,"lineHeight":request.line_height,"request":format!("{request:?}"),"metrics":format!("{metrics:?}"),"advance":metrics.advance,"measuredLineHeight":metrics.line_height}));
                            Ok(TextSize { width: metrics.advance, height: metrics.line_height })
                        })
                        .legend_item_renderer(|context| {
                            glory::ui::text().class("bylabel-custom-item").content_fn(move || {
                                format!("{} [{}:{}]", context.label.get(), context.target.get().id(), context.hidden.get())
                            })
                        })
                        .legend_symbol_renderer(|_| glory::ui::text().class("bylabel-custom-symbol").content("+"))
                } else {
                    chart
                };
                if actions {
                    chart.on_legend_activate(move |event| actions_queue.event(activation(event)))
                } else {
                    chart
                }
            })
            .map_err(js)?;
            Ok(Host::Chart {
                web: Some(web),
                scene: source.ok_or_else(|| js("missing chart source"))?,
                hidden: hidden_source.ok_or_else(|| js("missing chart hidden source"))?,
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
            let observations = queue.clone();
            chart.on_event(move |event| {
                if let BrowserEvent::Interaction(SurfaceEvent::PhysicalHover(hit)) = &event {
                    physical(&observations, hit.clone(), "browser");
                }
                match event {
                    BrowserEvent::LegendActivated(event) => observations.event(activation(event)),
                    other => {
                        observations.event(json!({"event":"browser","raw":format!("{other:?}")}))
                    }
                }
            });
            chart.flush().map_err(js)?;
            Ok(Host::Browser(chart))
        }
        "raster" => {
            let retained = Rc::new(RefCell::new(None));
            let output = retained.clone();
            let initial = frame.layout.clone();
            let fonts = fonts().map_err(js)?;
            let observations = queue.clone();
            let failures = queue.clone();
            let activations = queue.clone();
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
                    .map_err(ui_error)?;
                    let observations = observations.clone();
                    surface
                        .on_event(move |events| {
                            for event in &events {
                                if let SurfaceEvent::PhysicalHover(hit) = event {
                                    physical(&observations, hit.clone(), "raster");
                                }
                            }
                            observations
                                .event(json!({"event":"raster","raw":format!("{events:?}")}));
                        })
                        .map_err(ui_error)?;
                    let failures = failures.clone();
                    surface
                        .on_error(move |e| {
                            failures.event(json!({"event":"raster-error","error":e.0}))
                        })
                        .map_err(ui_error)?;
                    let activations = activations.clone();
                    surface
                        .on_legend_activate(move |event| activations.event(activation(event)))
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
            Ok(Host::Raster {
                web: Some(web),
                surface,
            })
        }
        _ => Err(js("unknown colors-function host kind")),
    }
}
fn active_web(web: &Option<WebApp>) -> std::result::Result<&WebApp, JsValue> {
    web.as_ref()
        .ok_or_else(|| js("fixture WebApp root owner was retired"))
}
fn flush_host(host: &Host) -> std::result::Result<(), JsValue> {
    match host {
        Host::Chart { web, .. } | Host::Raster { web, .. } => {
            active_web(web)?.app().flush().map_err(js)
        }
        Host::Browser(chart) => chart.flush().map_err(js),
    }
}
fn publish(host: &Host, frame: &model::Frame) -> std::result::Result<(), JsValue> {
    match host {
        Host::Chart { web, scene, .. } => {
            let web = active_web(web)?;
            scene.revise(|old| *old = frame.layout.clone());
            web.app().flush().map_err(js)
        }
        Host::Browser(chart) => {
            chart.set_layout(frame.layout.clone()).map_err(js)?;
            chart.flush().map_err(js)
        }
        Host::Raster { web, surface } => {
            let web = active_web(web)?;
            surface
                .handle()
                .set_layout(frame.layout.clone())
                .map_err(js)?;
            web.app().flush().map_err(js)
        }
    }
}
fn hidden_keys(host: &Host) -> std::result::Result<BTreeSet<String>, JsValue> {
    match host {
        Host::Chart { web, hidden, .. } => {
            active_web(web)?;
            Ok(hidden.get_untracked())
        }
        Host::Browser(chart) => Ok(chart.snapshot().map_err(js)?.hidden),
        Host::Raster { surface, .. } => Ok(surface.handle().snapshot().map_err(js)?.hidden),
    }
}
fn set_hidden(host: &Host, keys: BTreeSet<String>) -> std::result::Result<(), JsValue> {
    match host {
        Host::Chart { web, hidden, .. } => {
            let web = active_web(web)?;
            hidden.revise(|old| *old = keys);
            web.app().flush().map_err(js)
        }
        Host::Browser(chart) => {
            chart.handle().set_hidden(keys).map_err(js)?;
            chart.flush().map_err(js)
        }
        Host::Raster { web, surface } => {
            let web = active_web(web)?;
            surface.handle().set_hidden(keys).map_err(js)?;
            web.app().flush().map_err(js)
        }
    }
}
fn host_facts(host: &Host) -> Value {
    match host {
        Host::Chart { web, hidden, .. } => {
            if web.is_some() {
                json!({"kind":"semantic-chart","retainedWebApp":true,"hidden":hidden.get_untracked()})
            } else {
                json!({"kind":"semantic-chart","retainedWebApp":false,"error":"fixture WebApp root owner was retired"})
            }
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
fn drain(mounted: &mut Mounted) -> std::result::Result<(), JsValue> {
    for _ in 0..24 {
        flush_host(&mounted.host)?;
        let pending = std::mem::take(&mut *mounted.queue.pending.borrow_mut());
        if pending.is_empty() {
            return Ok(());
        }
        for (generation, observation) in pending {
            // Preserve every raw callback observation, including an old-frame
            // intent. Only its controlled side effects are generation-gated.
            let observation = match observation {
                Observation::Event(value) => {
                    log(&mounted.events, value);
                    continue;
                }
                value => value,
            };
            if generation != mounted.queue.generation.get() {
                let payload = match &observation {
                    Observation::Physical(hit) => {
                        json!({"kind":"physical","hit":hit.as_ref().map(model::hit_json)})
                    }
                    Observation::Hidden(keys) => json!({"kind":"hidden","keys":keys}),
                    Observation::Event(value) => value.clone(),
                };
                log(
                    &mounted.events,
                    json!({"event":"retired-pending-observation","generation":generation,"observation":payload}),
                );
                continue;
            }
            match observation {
                Observation::Event(value) => log(&mounted.events, value),
                Observation::Hidden(keys) => {
                    log(
                        &mounted.events,
                        json!({"event":"hidden-proposal","hidden":keys}),
                    );
                    let next = mounted.queue.next()?;
                    mounted.queue.generation.set(next);
                    set_hidden(&mounted.host, keys)?;
                }
                Observation::Physical(hit) => {
                    let mut next = mounted.frame.clone();
                    let changed = match model::accept_physical_hover(&mut next, hit) {
                        Ok(v) => v,
                        Err(e) => {
                            log(
                                &mounted.events,
                                json!({"event":"observed-snapshot-rejected","error":e.0}),
                            );
                            continue;
                        }
                    };
                    if changed {
                        let generation = mounted.queue.next()?;
                        let previous = mounted.queue.generation.replace(generation);
                        if let Err(error) = publish(&mounted.host, &next) {
                            mounted.queue.generation.set(previous);
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
        }
    }
    Err(js("colors-function controlled observations did not settle"))
}

#[wasm_bindgen]
pub fn box_colors_function_mount(
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
        .ok_or_else(|| js("missing colors-function root"))?;
    dispose_internal(id)?;
    let events = Rc::new(RefCell::new(Vec::new()));
    let queue = Queue::new();
    let host = mount_host(root.clone(), kind, &frame, &queue, actions)?;
    log(
        &events,
        json!({"event":"mount","case":case,"phase":phase,"kind":kind,"actions":actions,"hostMountGeneration":0}),
    );
    MOUNTS.with(|mounts| {
        mounts.borrow_mut().insert(
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
                queue,
                host_mount_generation: 0,
                actual_mount_count: 1,
                event_history: Vec::new(),
            },
        )
    });
    Ok(())
}
#[wasm_bindgen]
pub fn box_colors_function_flush(id: &str) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|mounts| {
        let mut mounts = mounts.borrow_mut();
        let m = mounts
            .get_mut(id)
            .ok_or_else(|| js("colors-function root not mounted"))?;
        drain(m)
    })
}
#[wasm_bindgen]
pub fn box_colors_function_update(id: &str, phase: u32) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|mounts| {
        let mut mounts = mounts.borrow_mut();
        let m = mounts
            .get_mut(id)
            .ok_or_else(|| js("colors-function root not mounted"))?;
        drain(m)?;
        let next = model::advance(&m.frame, phase).map_err(js)?;
        let generation = m.queue.next()?;
        let previous = m.queue.generation.replace(generation);
        if let Err(error) = publish(&m.host, &next) {
            m.queue.generation.set(previous);
            return Err(error);
        }
        m.phase = phase;
        m.frame = next;
        log(
            &m.events,
            json!({"event":"source-update","phase":phase,"generation":generation}),
        );
        drain(m)
    })
}
/// Attempt a controller operation without draining queued observations or publishing.
#[wasm_bindgen]
pub fn box_colors_function_try_invalid(
    id: &str,
    mode: u32,
) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|mounts| {let mut mounts=mounts.borrow_mut();let m=mounts.get_mut(id).ok_or_else(||js("colors-function root not mounted"))?;
        // No drain/flush here: a rejected operation cannot accidentally publish.
        let attempt=model::try_invalid(&mut m.frame,mode).map_err(js)?;
        Ok(json!({"id":id,"phase":m.phase,"attempt":attempt,"facts":model::observe(&m.frame),"host":host_facts(&m.host)}).to_string())
    })
}
#[wasm_bindgen]
pub fn box_colors_function_set_hidden(
    id: &str,
    subgroup: &str,
    hidden: bool,
) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|mounts| {let mut mounts=mounts.borrow_mut();let m=mounts.get_mut(id).ok_or_else(||js("colors-function root not mounted"))?;
        if !m.frame.controller.layout.subgroups.iter().any(|k|k==subgroup) {return Err(js("unknown original subgroup"));}
        let mut keys=hidden_keys(&m.host)?;let series=boxplot_series_id(subgroup);
        if hidden {keys.insert(series);} else {keys.remove(&series);}
        set_hidden(&m.host,keys)?;log(&m.events,json!({"event":"explicit-host-hidden","rawSubgroup":subgroup,"canonicalSeriesID":boxplot_series_id(subgroup),"hidden":hidden}));drain(m)
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

fn release_web(web: &mut Option<WebApp>) -> std::result::Result<(), JsValue> {
    if let Some(web) = web.take() {
        let result = web.app().clear().map_err(js);
        // Drop releases Glory's root claim even if clearing returned an error.
        // Deleting the DOM attribute would leave the old owner alive.
        drop(web);
        result
    } else {
        Ok(())
    }
}
fn stop_host(host: &mut Host) -> std::result::Result<RetiredHost, JsValue> {
    Ok(match host {
        Host::Chart { web, .. } => {
            release_web(web)?;
            RetiredHost::Chart
        }
        Host::Browser(chart) => {
            let handle = chart.handle();
            chart.stop();
            RetiredHost::Browser(handle)
        }
        Host::Raster { web, surface } => {
            let handle = surface.handle();
            surface.stop();
            release_web(web)?;
            RetiredHost::Raster(handle)
        }
    })
}
/// Opt-in actions are mount-time host configuration. Retire the old owner,
/// clear its DOM, and explicitly mount the same genuine cached source again.
#[wasm_bindgen]
pub fn box_colors_function_set_actions(
    id: &str,
    actions: bool,
) -> std::result::Result<(), JsValue> {
    MOUNTS.with(|mounts| {
        let mut mounts=mounts.borrow_mut();let m=mounts.get_mut(id).ok_or_else(||js("colors-function root not mounted"))?;
        if m.actions==actions {return Ok(());}
        drain(m)?;
        let hidden=hidden_keys(&m.host)?;
        let next_mount=m.host_mount_generation.checked_add(1).ok_or_else(||js("host mount generation overflow"))?;
        let next_count=m.actual_mount_count.checked_add(1).ok_or_else(||js("actual mount counter overflow"))?;
        let next_publication=m.queue.next()?;
        let before_factory=m.frame.facts["factoryTrace"].clone();
        let before_controller=model::raw_scene(&m.frame.controller.layout.scene);
        m.queue.retire();
        let root_claim_before=m.root.has_attribute("data-gly-app");
        let retired=stop_host(&mut m.host)?;
        let root_claim_after_drop=m.root.has_attribute("data-gly-app");
        if root_claim_after_drop {return Err(js("old fixture root claim survived actual WebApp drop"));}
        m.root.set_inner_html("");
        log(&m.events,json!({"event":"retire-for-actions-remount","oldActions":m.actions,"nextActions":actions,"hostMountGeneration":m.host_mount_generation,"rootClaimBefore":root_claim_before,"rootClaimAfterActualDrop":root_claim_after_drop}));
        let events_at_dispose=m.events.borrow().len();
        RETIRED.with(|r|r.borrow_mut().push(Retired {id:m.id.clone(),kind:m.kind.clone(),reason:"actions-remount".into(),host_mount_generation:m.host_mount_generation,root:m.root.clone(),events:m.events.clone(),events_at_dispose,root_children_at_retirement:m.root.child_element_count(),host:retired}));
        m.event_history.extend(m.events.borrow().iter().cloned());
        let queue=Queue::new();queue.generation.set(next_publication);
        let events=Rc::new(RefCell::new(Vec::new()));
        let host=mount_host(m.root.clone(),&m.kind,&m.frame,&queue,actions)?;
        set_hidden(&host,hidden)?;
        m.host=host;m.queue=queue;m.events=events;m.actions=actions;m.host_mount_generation=next_mount;m.actual_mount_count=next_count;
        log(&m.events,json!({"event":"actions-remount","actions":actions,"hostMountGeneration":next_mount,"actualMountCount":next_count,"controllerSceneExact":before_controller==model::raw_scene(&m.frame.controller.layout.scene),"factoryTraceExact":before_factory==m.frame.facts["factoryTrace"]}));
        drain(m)
    })
}
#[wasm_bindgen]
pub fn box_colors_function_snapshot(id: &str) -> std::result::Result<String, JsValue> {
    MOUNTS.with(|mounts| {
        let mounts=mounts.borrow();let m=mounts.get(id).ok_or_else(||js("colors-function root not mounted"))?;
        let mut events=m.event_history.clone();events.extend(m.events.borrow().iter().cloned());
        Ok(json!({"brand":"kartis","id":m.id,"kind":m.kind,"case":m.case,"phase":m.phase,"actions":m.actions,"hostMountGeneration":m.host_mount_generation,"actualMountCount":m.actual_mount_count,"publicationGeneration":m.queue.generation.get(),"chartCustomMeasured":m.case==5&&m.kind=="chart","facts":model::observe(&m.frame),"host":host_facts(&m.host),"events":events,"currentHostEvents":*m.events.borrow(),"pendingObservations":m.queue.pending.borrow().len(),"rootChildren":m.root.child_element_count(),"rawDOM":m.root.outer_html(),"resources":resources(&m.root)}).to_string())
    })
}
fn dispose_internal(id: &str) -> std::result::Result<(), JsValue> {
    let previous = MOUNTS.with(|mounts| mounts.borrow_mut().remove(id));
    if let Some(mut m) = previous {
        m.queue.retire();
        let retired = stop_host(&mut m.host)?;
        m.root.set_inner_html("");
        log(
            &m.events,
            json!({"event":"dispose","hostMountGeneration":m.host_mount_generation}),
        );
        let events_at_dispose = m.events.borrow().len();
        RETIRED.with(|r| {
            r.borrow_mut().push(Retired {
                id: m.id,
                kind: m.kind,
                reason: "dispose".into(),
                host_mount_generation: m.host_mount_generation,
                root_children_at_retirement: m.root.child_element_count(),
                root: m.root,
                events: m.events,
                events_at_dispose,
                host: retired,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn box_colors_function_dispose(id: &str) -> std::result::Result<(), JsValue> {
    dispose_internal(id)
}
#[wasm_bindgen]
pub fn box_colors_function_retired() -> String {
    RETIRED.with(|r|json!(r.borrow().iter().map(|r| {
        let host=match &r.host {RetiredHost::Chart=>json!({"kind":"semantic-chart","guard":"cleared then dropped"}),RetiredHost::Browser(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)}),RetiredHost::Raster(h)=>json!({"active":h.is_active(),"snapshotError":h.snapshot().err().map(|e|e.0)})};
        json!({"id":r.id,"kind":r.kind,"reason":r.reason,"hostMountGeneration":r.host_mount_generation,"eventsAtDispose":r.events_at_dispose,"events":*r.events.borrow(),"rootChildrenAtRetirement":r.root_children_at_retirement,"rootChildrenNow":r.root.child_element_count(),"host":host})
    }).collect::<Vec<_>>()).to_string())
}
#[wasm_bindgen]
pub fn box_colors_function_schema() -> String {
    json!({"brand":"kartis","schema":"box-colors-function-host-v1","width":model::WIDTH,"height":model::HEIGHT,"plot":[40,30,400,240],"cases":model::CASE_COUNT,"phases":model::PHASE_COUNT,"invalidModes":model::INVALID_COUNT,"caseNames":model::CASE_NAMES,"kinds":["chart","svg","canvas","raster"],"initialActions":{"ordinarySeries":true,"case1":false},"actionsRemountCases":[1],"setActions":"Explicit owner retirement and root lease drop, then remount with same cached controller/Scene; no factory rebuild","hoverSource":"Actual retained-host PhysicalHover and current-frame whole-parts binding, queued outside business callbacks","factoryBorrow":"Actual color and optional legend functions only during builder/source replacement; neither retained","configuredPolicy":"All cases color-built; all except Keys case3 also legend-factory-built, including legendfalse and empty cases","invalidOperations":["color-factory-returned-Err","missing-required-color","missing-required-legend-or-incompatible-Keys-factory","bad-source-empty-group","controller-factor-NaN"],"chartCustomMeasuredCase5":true,"customItemFormat":"caption [target.id:hidden]","customSymbol":"+","customMeasuredBounds":[0,0,480,180],"customSlotOtherHosts":false,"nativeRasterPixelRatio":1,"font":"Lato","strictCompositorAccepted":false}).to_string()
}
