//! Accepted editable selection drives a real reverse-axis domain relayout.
use super::*;

struct Entry {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: Rc<Cell<u8>>,
    changed_plot: Rc<Cell<bool>>,
    reject: Rc<Cell<bool>>,
    accepted: Rc<RefCell<ScaledLineLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
    owner: Weak<()>,
}
thread_local! {
    static VIEWS:RefCell<Vec<Entry>>=const{RefCell::new(Vec::new())};
    static RETIRED:RefCell<Vec<Weak<()>>>=const{RefCell::new(Vec::new())};
}

fn config(width: f64, height: f64, changed: bool) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Brush to reverse domains".into(),
        margin: if changed {
            Margin {
                left: 112.,
                right: 47.,
                top: 55.,
                bottom: 80.,
            }
        } else {
            Margin {
                left: 76.,
                right: 31.,
                top: 35.,
                bottom: 65.,
            }
        },
        ..Default::default()
    }
}
fn layout(
    phase: u8,
    changed: bool,
    width: f64,
    height: f64,
    domains: Option<ResolvedDomains>,
    hidden: BTreeSet<String>,
) -> Result<ScaledLineLayout> {
    if phase > 1 {
        return Err(ChartError("invalid brush viewport phase".into()));
    }
    let phase = f64::from(phase);
    line_layout_with_scale_options(
        &config(width, height, changed),
        &[
            Series::new(
                "alpha",
                vec![
                    Point::new(1., -5.),
                    Point::new(5., 5. + phase * 7.),
                    Point::new(9., 15.),
                ],
            ),
            Series::new(
                "beta",
                vec![
                    Point::new(2., 10.),
                    Point::new(6., -2. + phase * 4.),
                    Point::new(8., 18.),
                ],
            ),
        ],
        &LineOptions {
            x_domain: Some(domains.as_ref().map_or([0., 10.], |d| d.x)),
            y_domain: Some(domains.as_ref().and_then(|d| d.left).unwrap_or([-10., 20.])),
            hidden,
            points: true,
            ..Default::default()
        },
        &LineStyleOptions::default(),
        &LineScaleOptions {
            x: ContinuousScaleOptions {
                reverse: true,
                ..Default::default()
            },
            y: ContinuousScaleOptions {
                reverse: true,
                ..Default::default()
            },
        },
    )
}

#[wasm_bindgen]
pub fn editable_brush_viewport_mount(mode: u8) -> std::result::Result<(), JsValue> {
    let axis = axis(mode).map_err(js)?;
    super::editable_brush_stop();
    RETIRED.with(|s| s.borrow_mut().clear());
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("editable-brush-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let initial = layout(0, false, 640., 400., None, BTreeSet::new()).map_err(js)?;
        let plot = initial.layout.plot;
        let controller = initial
            .viewport_controller(ViewportLimits {
                min_zoom: 1.,
                max_zoom: 20.,
                bounded: true,
            })
            .map_err(js)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                highlight: browser::BrowserHighlight::Controlled,
                editable_brush: Some(EditableBrushOptions {
                    axis,
                    extent: Brush {
                        x0: plot.x,
                        y0: plot.y,
                        x1: plot.x + plot.width,
                        y1: plot.y + plot.height,
                    },
                    handle_size: 10.,
                    modifiers: true,
                }),
                interaction: surface::SurfaceOptions {
                    plot: Some(plot),
                    ..Default::default()
                },
                interaction_data: Some(initial.interaction_binding()),
                ..Default::default()
            },
        )
        .map_err(js)?;
        let phase = Rc::new(Cell::new(0));
        let changed_plot = Rc::new(Cell::new(false));
        let reject = Rc::new(Cell::new(false));
        let accepted = Rc::new(RefCell::new(initial));
        let p = phase.clone();
        let changed = changed_plot.clone();
        let bad = reject.clone();
        let current = accepted.clone();
        let geometry = changed_plot.clone();
        chart
            .on_viewport_with_plot_and_interaction(
                controller,
                ViewportSelection::Both,
                move |w, h| {
                    let c = config(w, h, geometry.get());
                    c.validate()?;
                    Ok(ClipRect::plot(&c))
                },
                move |request| {
                    if bad.get() {
                        return Err(ChartError(
                            "intentional brush viewport layout rejection".into(),
                        ));
                    }
                    let next = layout(
                        p.get(),
                        changed.get(),
                        request.width,
                        request.height,
                        Some(request.domains),
                        request.hidden,
                    )?;
                    let candidate = SceneInteraction {
                        scene: next.layout.scene.clone(),
                        plot: Some(next.layout.plot),
                        interaction: Some(next.interaction_binding()),
                    };
                    *current.borrow_mut() = next;
                    Ok(candidate)
                },
            )
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        let handle = chart.handle();
        let owner = Rc::new(());
        let weak = Rc::downgrade(&owner);
        chart.on_event(move|event|{
            let _keep=&owner;
            let value=match event{
                BrowserEvent::Interaction(SurfaceEvent::EditableBrush{event,ids})=>{
                    if event.phase==BrushPhase::End&&event.source==BrushSource::User&&event.selection.is_some()
                        && let Err(error)=handle.request_editable_brush_viewport(){
                        log.borrow_mut().push(json!({"brushRequestError":error.0}));
                    }
                    json!({"phase":format!("{:?}",event.phase),"source":format!("{:?}",event.source),"selection":event.selection.map(bounds),"ids":ids})
                }
                BrowserEvent::Error(error)=>json!({"error":error.0}),
                other=>json!({"other":format!("{other:?}")}),
            };
            let mut log=log.borrow_mut();if log.len()==1024{log.remove(0);}log.push(value);
        });
        chart.flush().map_err(js)?;
        VIEWS.with(|s| {
            s.borrow_mut().push(Entry {
                chart,
                root,
                renderer: name.into(),
                phase,
                changed_plot,
                reject,
                accepted,
                events,
                owner: weak,
            })
        });
    }
    Ok(())
}

pub(super) fn snapshot(renderer: &str) -> std::result::Result<Option<String>, JsValue> {
    VIEWS.with(|slot|{
        let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(None);};
        let state=f.chart.snapshot().map_err(js)?;let accepted=f.accepted.borrow();let plot=accepted.layout.plot;
        let editor=state.editable_brush.map(|b|json!({"selection":b.selection.map(bounds),"ids":b.ids,"extent":bounds(b.options.extent),"axis":format!("{:?}",b.options.axis),"active":b.active,"captured":b.captured_pointer}));
        Ok(Some(json!({"active":state.active,"renderer":f.renderer,"width":state.width,"height":state.height,"editor":editor,"domains":state.domains.map(|d|json!({"x":d.x,"y":d.left})),"phase":f.phase.get(),"changedPlot":f.changed_plot.get(),"plot":[plot.x,plot.y,plot.width,plot.height],"revision":state.revision,"pending":state.pending,"dropped":state.dropped,"view":[state.view.scale,state.view.translate_x,state.view.translate_y],"sites":accepted.layout.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series_id,"source":s.source_index,"raw":[s.raw_x,s.raw_y],"position":s.position})).collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string()))
    })
}

fn with(
    renderer: &str,
    action: impl FnOnce(&Entry) -> Result<()>,
) -> std::result::Result<(), JsValue> {
    VIEWS.with(|s| {
        let entries = s.borrow();
        let f = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js(ChartError("brush viewport fixture is stopped".into())))?;
        action(f).map_err(js)
    })
}
#[wasm_bindgen]
pub fn editable_brush_viewport_phase(
    renderer: &str,
    phase: u8,
    changed_plot: bool,
) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js(ChartError("invalid brush viewport phase".into())));
    }
    with(renderer, |f| {
        f.phase.set(phase);
        f.changed_plot.set(changed_plot);
        f.chart.handle().refresh_viewport()
    })
}
#[wasm_bindgen]
pub fn editable_brush_viewport_reject(
    renderer: &str,
    reject: bool,
) -> std::result::Result<(), JsValue> {
    with(renderer, |f| {
        f.reject.set(reject);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn editable_brush_viewport_hidden(
    renderer: &str,
    hide: bool,
) -> std::result::Result<(), JsValue> {
    with(renderer, |f| {
        f.chart.handle().set_hidden(if hide {
            BTreeSet::from(["alpha".into()])
        } else {
            BTreeSet::new()
        })
    })
}
#[wasm_bindgen]
pub fn editable_brush_viewport_reset(renderer: &str) -> std::result::Result<(), JsValue> {
    with(renderer, |f| f.chart.handle().reset_view())
}
#[wasm_bindgen]
pub fn editable_brush_viewport_retired() -> bool {
    RETIRED.with(|s| s.borrow().iter().all(|weak| weak.upgrade().is_none()))
}
pub(super) fn flush() -> std::result::Result<(), JsValue> {
    VIEWS.with(|s| {
        for f in s.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
pub(super) fn stop() {
    let entries = VIEWS.with(|s| std::mem::take(&mut *s.borrow_mut()));
    if entries.is_empty() {
        return;
    }
    let mut retired = Vec::with_capacity(entries.len());
    for f in entries {
        f.chart.stop();
        f.root.remove();
        retired.push(f.owner);
    }
    RETIRED.with(|s| *s.borrow_mut() = retired);
}
