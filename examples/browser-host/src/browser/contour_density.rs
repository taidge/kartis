//! Application-owned contours and weighted density using public Kartis APIs.
use kartis::browser::{
    BrowserChart, BrowserEvent, BrowserHandle, BrowserHighlight, BrowserOptions, BrowserRenderer,
    MotionPreference,
};
use kartis::surface::{SurfaceEvent, SurfaceOptions};
use kartis::*;
use serde_json::{Value, json};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeSet,
    rc::{Rc, Weak},
};
use wasm_bindgen::prelude::*;

#[derive(Clone)]
struct Settings {
    mode: u8,
    inserted: bool,
    hidden: bool,
    hide_source: bool,
    weight: f64,
    bandwidth: f64,
    width: f64,
    height: f64,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: 0,
            inserted: false,
            hidden: false,
            hide_source: false,
            weight: 1.,
            bandwidth: 14.,
            width: 640.,
            height: 360.,
        }
    }
}
#[derive(Clone)]
struct Frame {
    candidate: SceneInteraction,
    levels: Vec<ContourLevelMetadata>,
    field: Option<DensityField>,
    observations: Vec<DensityObservation>,
    settings: Settings,
    generation: u64,
}
#[derive(Clone, PartialEq)]
struct CacheKey {
    width: f64,
    height: f64,
    weight: f64,
    bandwidth: f64,
    hide_source: bool,
}
#[derive(Clone)]
struct Cache {
    key: CacheKey,
    layout: ScatterDensityLayout,
    generation: u64,
}
struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    settings: Rc<RefCell<Settings>>,
    frame: Rc<RefCell<Frame>>,
    cache: Rc<RefCell<Option<Cache>>>,
    events: Rc<RefCell<Vec<Value>>>,
    publish_errors: Rc<Cell<usize>>,
    callback: Weak<()>,
}
thread_local! {
    static ENTRIES:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};
    static RETIRED:RefCell<Vec<(String,BrowserHandle,Weak<()>)>>=const{RefCell::new(Vec::new())};
}
fn checked(ok: bool, message: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ChartError(message.into()))
    }
}
fn js(error: impl std::fmt::Display) -> JsValue {
    error.to_string().into()
}
fn config(s: &Settings) -> ChartConfig {
    ChartConfig {
        width: s.width,
        height: s.height,
        margin: Margin {
            left: 32.,
            right: 24.,
            top: 24.,
            bottom: 60.,
        },
        ..Default::default()
    }
}
fn levels(s: &Settings) -> Vec<f64> {
    if s.mode == 0 {
        if s.inserted { vec![0.5, 1.] } else { vec![1.] }
    } else if s.inserted {
        vec![0.0005, 0.001]
    } else {
        vec![0.001]
    }
}
fn source_scatter(c: &ChartConfig, s: &Settings) -> Result<(ScatterLayout, DensityWeights)> {
    let mut weights = DensityWeights::new();
    let data = (0..2)
        .map(|group| {
            let data = (0..18)
                .map(|i| {
                    let angle = i as f64 * 2.399963229728653;
                    let radius = (i as f64 / 18.).sqrt();
                    let x = if group == 0 { 30. } else { 72. } + radius * 10. * angle.cos();
                    let y = if group == 0 { 65. } else { 32. } + radius * 15. * angle.sin();
                    let id = format!("sample-{i}");
                    weights.insert(
                        (format!("cloud-{group}"), id.clone()),
                        if group == 0 { 1. } else { s.weight },
                    );
                    let mut point = ScatterDatum::new(id, x, y);
                    point.size_value = Some(999.);
                    point
                })
                .collect();
            ScatterSeries::new(format!("cloud-{group}"), data)
        })
        .collect::<Vec<_>>();
    let axis = || ScatterAxisScale::Continuous {
        options: ContinuousScaleOptions::default(),
        min: ScatterBound::Fixed(0.),
        max: ScatterBound::Fixed(100.),
    };
    let scatter = scatter_layout(
        c,
        &data,
        &ScatterLayoutOptions {
            scales: ScatterScaleOptions {
                x: axis(),
                y: axis(),
            },
            hidden: if s.hide_source {
                ["cloud-1".into()].into()
            } else {
                BTreeSet::new()
            },
            node: ScatterNodeStyle {
                shape: ScatterNodeShape::Circle {
                    diameter: ScatterSize::Fixed(6.),
                },
                ..Default::default()
            },
            ..Default::default()
        },
    )?;
    Ok((scatter, weights))
}
fn build(s: &Settings, cache: &mut Option<Cache>) -> Result<Frame> {
    checked(
        s.mode <= 1 && s.weight.is_finite() && s.bandwidth.is_finite(),
        "invalid contour/density settings",
    )?;
    let c = config(s);
    c.validate()?;
    let mut options = ContourLayoutOptions {
        colors: ColorScaleOptions {
            colors: vec![Color(20, 135, 155, 255); 2],
            domain: Some(if s.mode == 0 { [0., 2.] } else { [0., 0.002] }),
            ..Default::default()
        },
        ..Default::default()
    };
    let (candidate, metadata, field, observations, generation) = if s.mode == 0 {
        let values = (0usize..225)
            .map(|i| {
                let (x, y) = (i % 15, i / 15);
                Some(if x.min(y).min(14 - x).min(14 - y) / 2 % 2 == 0 {
                    2.
                } else {
                    0.
                })
            })
            .collect::<Vec<_>>();
        let mut layout = contour_layout(
            &c,
            ContourGrid {
                width: 15,
                height: 15,
                values: &values,
            },
            &ContourOptions {
                thresholds: ContourThresholds::Values(levels(s)),
                ..Default::default()
            },
            &options,
        )?;
        if s.hidden {
            options.hidden = layout.levels.iter().map(|l| l.id.clone()).collect();
            layout = contour_layout(
                &c,
                ContourGrid {
                    width: 15,
                    height: 15,
                    values: &values,
                },
                &ContourOptions {
                    thresholds: ContourThresholds::Values(levels(s)),
                    ..Default::default()
                },
                &options,
            )?;
        }
        (layout.scene_interaction(), layout.levels, None, vec![], 0)
    } else {
        let key = CacheKey {
            width: s.width,
            height: s.height,
            weight: s.weight,
            bandwidth: s.bandwidth,
            hide_source: s.hide_source,
        };
        if cache.as_ref().is_none_or(|cache| cache.key != key) {
            let (scatter, weights) = source_scatter(&c, s)?;
            let layout = scatter_density_layout(
                &c,
                &scatter,
                &weights,
                ScatterDensityOptions {
                    bandwidth: s.bandwidth,
                    cell_size: 4.,
                    ..Default::default()
                },
                &DensityThresholds::Values(levels(s)),
                &options,
            )?;
            let generation = cache.as_ref().map_or(1, |cache| cache.generation + 1);
            *cache = Some(Cache {
                key,
                layout,
                generation,
            });
        }
        let cache = cache.as_ref().unwrap();
        let mut combined = cache.layout.recontour(
            &DensityThresholds::Values(levels(s)),
            ContourLimits::default(),
            &options,
        )?;
        if s.hidden {
            options.hidden = combined
                .levels
                .iter()
                .map(|level| level.id.clone())
                .collect();
            combined = cache.layout.recontour(
                &DensityThresholds::Values(levels(s)),
                ContourLimits::default(),
                &options,
            )?;
        }
        (
            combined.scene_interaction(),
            combined.levels,
            Some(combined.field),
            combined.observations,
            cache.generation,
        )
    };
    candidate.validate()?;
    Ok(Frame {
        candidate,
        levels: metadata,
        field,
        observations,
        settings: s.clone(),
        generation,
    })
}
fn event(event: BrowserEvent) -> Value {
    match event {
        BrowserEvent::Focus(id) => json!({"type":"focus","id":id}),
        BrowserEvent::Error(error) => json!({"type":"error","message":error.0}),
        BrowserEvent::LegendActivated(event) => json!({
            "type":"legend", "id":event.item.id, "label":event.item.label,
            "role":format!("{:?}",event.role),
        }),
        BrowserEvent::Resized { width, height } => {
            json!({"type":"resized","width":width,"height":height})
        }
        BrowserEvent::Interaction(SurfaceEvent::Hover(ids)) => json!({"type":"hover","ids":ids}),
        BrowserEvent::Interaction(SurfaceEvent::Activated(id)) => {
            json!({"type":"activated","id":id})
        }
        BrowserEvent::Interaction(SurfaceEvent::Selected(id)) => json!({"type":"selected","id":id}),
        BrowserEvent::Interaction(other) => {
            json!({"type":"interaction","value":format!("{other:?}")})
        }
    }
}
#[wasm_bindgen]
pub fn contour_density_mount(mode: u8) -> std::result::Result<(), JsValue> {
    let settings = Settings {
        mode,
        ..Default::default()
    };
    let first = build(&settings, &mut None).map_err(js)?;
    contour_density_stop();
    RETIRED.with(|slot| slot.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, kind) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let settings = Rc::new(RefCell::new(settings.clone()));
        let cache = Rc::new(RefCell::new(None));
        let initial = build(&settings.borrow(), &mut cache.borrow_mut()).map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("contour-density-{renderer}"));
        root.set_attribute("class", "chart")?;
        root.set_attribute("style", "width:640px;max-width:none;position:relative")?;
        document
            .get_element_by_id("contour-density-charts")
            .unwrap()
            .append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            first.candidate.scene.clone(),
            BrowserOptions {
                renderer: kind,
                height: Some(settings.borrow().height),
                highlight: BrowserHighlight::Controlled,
                motion: MotionPreference::Reduced,
                interaction: SurfaceOptions {
                    plot: initial.candidate.plot,
                    pan: false,
                    zoom: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        chart.set_layout(initial.candidate.clone()).map_err(js)?;
        chart.flush().map_err(js)?;
        let frame = Rc::new(RefCell::new(initial));
        let events = Rc::new(RefCell::new(Vec::new()));
        let token = Rc::new(());
        let weak = Rc::downgrade(&token);
        let publish_errors = Rc::new(Cell::new(0usize));
        let pending_resize = Rc::new(RefCell::new(None::<(Settings, Frame, Option<Cache>)>));
        let capture = events.clone();
        let keeper = token.clone();
        let errors = publish_errors.clone();
        let pending = pending_resize.clone();
        let state = settings.clone();
        let accepted = frame.clone();
        let field = cache.clone();
        chart.on_event(move |value| {
            let _keep = &keeper;
            match &value {
                BrowserEvent::Error(_) => {
                    errors.set(errors.get().saturating_add(1));
                    pending.borrow_mut().take();
                }
                BrowserEvent::Resized { width, height } => {
                    if let Some((next, candidate, proposed_cache)) = pending.borrow_mut().take()
                        && next.width == *width
                        && next.height == *height
                    {
                        *state.borrow_mut() = next;
                        *accepted.borrow_mut() = candidate;
                        *field.borrow_mut() = proposed_cache;
                    }
                }
                _ => {}
            }
            let mut log = capture.borrow_mut();
            if log.len() >= 80 {
                log.remove(0);
            }
            log.push(event(value));
        });
        let state = settings.clone();
        let field = cache.clone();
        chart.on_resize_with_interaction(move |width, height| {
            let _keep = &token;
            let mut next = state.borrow().clone();
            next.width = width;
            next.height = height;
            let mut proposed_cache = field.borrow().clone();
            let candidate = build(&next, &mut proposed_cache)?;
            let output = candidate.candidate.clone();
            *pending_resize.borrow_mut() = Some((next, candidate, proposed_cache));
            Ok(output)
        });
        ENTRIES.with(|slot| {
            slot.borrow_mut().push(Entry {
                chart,
                root,
                renderer: renderer.into(),
                settings,
                frame,
                cache,
                events,
                publish_errors,
                callback: weak,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn contour_density_update(
    inserted: bool,
    hidden: bool,
    hide_source: bool,
    weight: f64,
    bandwidth: f64,
) -> std::result::Result<(), JsValue> {
    ENTRIES.with(|slot| {
        let entries = slot.borrow();
        let mut candidates = Vec::new();
        for entry in entries.iter() {
            let mut next = entry.settings.borrow().clone();
            next.inserted = inserted;
            next.hidden = hidden;
            next.hide_source = hide_source;
            next.weight = weight;
            next.bandwidth = bandwidth;
            let mut cache = entry.cache.borrow().clone();
            let candidate = build(&next, &mut cache).map_err(js)?;
            candidates.push((next, candidate, cache));
        }
        for (entry, (next, candidate, cache)) in entries.iter().zip(candidates) {
            let errors_before = entry.publish_errors.get();
            entry
                .chart
                .set_layout(candidate.candidate.clone())
                .map_err(js)?;
            entry.chart.flush().map_err(js)?;
            if entry.publish_errors.get() != errors_before {
                return Err(js(
                    "Contour proposal was not published; accepted frame retained",
                ));
            }
            *entry.settings.borrow_mut() = next;
            *entry.frame.borrow_mut() = candidate;
            *entry.cache.borrow_mut() = cache;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn contour_density_flush() -> std::result::Result<(), JsValue> {
    ENTRIES.with(|slot| {
        for e in slot.borrow().iter() {
            e.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn contour_density_resize(width: f64) -> std::result::Result<(), JsValue> {
    checked(
        width.is_finite() && (400. ..=800.).contains(&width),
        "invalid chart width",
    )
    .map_err(js)?;
    ENTRIES.with(|slot| {
        for e in slot.borrow().iter() {
            e.root.set_attribute(
                "style",
                &format!("width:{width}px;max-width:none;position:relative"),
            )?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn contour_density_rejected() -> bool {
    ENTRIES.with(|slot| {
        slot.borrow().iter().all(|entry| {
            let mut invalid = entry.frame.borrow().candidate.clone();
            if let Some(binding) = &mut invalid.interaction {
                if let Some(site) = binding.sites.first_mut() {
                    site.value = Some(f64::NAN);
                } else {
                    invalid.scene.width = f64::NAN;
                }
            }
            entry.chart.set_layout(invalid).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn contour_density_rejected_scene() -> bool {
    ENTRIES.with(|slot| {
        slot.borrow().iter().all(|entry| {
            let mut invalid = entry.frame.borrow().candidate.clone();
            invalid.scene.width = f64::NAN;
            entry.chart.set_layout(invalid).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn contour_density_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    ENTRIES.with(|slot|{let entries=slot.borrow();let Some(e)=entries.iter().find(|e|e.renderer==renderer)else{return Ok(RETIRED.with(|slot|{let entries=slot.borrow();let old=entries.iter().find(|e|e.0==renderer);json!({"active":false,"retired":old.is_some(),"stopped":old.is_none_or(|e|!e.1.is_active()),"callbacksReleased":old.is_none_or(|e|e.2.upgrade().is_none())}).to_string()}));};
        let state=e.chart.snapshot().map_err(js)?;let f=e.frame.borrow();let plot=f.candidate.plot.unwrap();let binding=f.candidate.interaction.as_ref().unwrap();
        let sites=binding.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"position":s.position})).collect::<Vec<_>>();
        let levels=f.levels.iter().map(|l|json!({"id":l.id,"value":l.threshold,"rank":l.duplicate_rank,"polygons":l.polygons,"holes":l.holes,"hidden":l.hidden,"mark":l.mark_id})).collect::<Vec<_>>();
        let observations=f.observations.iter().map(|o|json!({"id":o.mark_id,"series":o.series_id,"node":o.node_id,"rawX":match o.raw_x{CategoryKey::Number(x)=>Some(x),_=>None},"rawY":match o.raw_y{CategoryKey::Number(y)=>Some(y),_=>None},"position":o.position,"weight":o.weight})).collect::<Vec<_>>();
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"mode":f.settings.mode,"inserted":f.settings.inserted,"hidden":f.settings.hidden,"hideSource":f.settings.hide_source,"weight":f.settings.weight,"bandwidth":f.settings.bandwidth,"plot":[plot.x,plot.y,plot.width,plot.height],"sites":sites,"levels":levels,"observations":observations,"generation":f.generation,"storage":f.field.as_ref().map(|f|f.values().as_ptr() as usize),"fieldBits":f.field.as_ref().map(|f|f.values().iter().map(|v|v.to_bits()).collect::<Vec<_>>()),"fieldMax":f.field.as_ref().map(DensityField::max),"events":e.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn contour_density_stop() {
    let old = ENTRIES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for e in old {
        let handle = e.chart.handle();
        e.chart.stop();
        e.root.remove();
        RETIRED.with(|slot| slot.borrow_mut().push((e.renderer, handle, e.callback)));
    }
}
