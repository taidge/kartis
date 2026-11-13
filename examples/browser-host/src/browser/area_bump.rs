//! Application-owned AreaBump re-layout and controlled series focus.
use super::*;
use std::collections::BTreeSet;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    layout: Rc<RefCell<AreaBumpLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static AREAS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn source(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
) -> Result<AreaBumpLayout> {
    if case > 2 || phase > 1 {
        return Err(ChartError("AreaBump fixture case/phase is invalid".into()));
    }
    let mut rows = vec![
        AreaBumpSeries::new(
            "Alpha:area",
            vec![
                AreaBumpDatum::new(0, 6.),
                AreaBumpDatum::new(1, 2.),
                AreaBumpDatum::new(2, if phase == 0 { 9. } else { 5. }),
            ],
        ),
        AreaBumpSeries::new(
            "Beta",
            vec![
                AreaBumpDatum::new(0, 2.),
                AreaBumpDatum::new(1, if phase == 0 { 8. } else { 10. }),
                AreaBumpDatum::new(2, 3.),
            ],
        ),
        AreaBumpSeries::new(
            "Gamma",
            vec![
                AreaBumpDatum::new(0, 0.),
                AreaBumpDatum::new(1, 2.),
                AreaBumpDatum::new(2, 4.),
            ],
        ),
    ];
    rows[0].label = "Alpha <A>".into();
    let mut o = AreaBumpOptions {
        hidden,
        spacing: 8.,
        align: if case == 1 {
            AreaBumpAlign::End
        } else if case == 2 {
            AreaBumpAlign::Start
        } else {
            AreaBumpAlign::Middle
        },
        interpolation: if case == 0 {
            AreaBumpInterpolation::Linear
        } else {
            AreaBumpInterpolation::Smooth
        },
        ..Default::default()
    };
    if case == 2 {
        rows[0].data.remove(1);
        o.missing = AreaBumpMissing::Gap;
        o.x_order = Some((0..4).map(AreaBumpX::from).collect());
    }
    o.style.border_width = 2.;
    if phase == 0 && case == 0 {
        o.styles.insert(
            "Alpha:area".into(),
            AreaBumpSeriesStyle {
                paint: Some(FillPaint::Linear {
                    start: [100., 50.],
                    end: [540., 350.],
                    stops: vec![
                        GradientStop::new(0., Color(30, 110, 190, 255)),
                        GradientStop::new(1., Color(140, 200, 240, 255)),
                    ],
                }),
                ..o.style.clone()
            },
        );
    }
    if phase == 1 {
        o.style.fill_opacity = 0.6;
        o.x_padding = 0.3;
        o.start_labels.text = AreaBumpLabelText::Endpoint(ValueFormat::Fixed(1));
        o.end_labels.text = AreaBumpLabelText::Endpoint(ValueFormat::Fixed(1));
        o.start_labels.rotation = -10.;
        o.end_labels.rotation = 10.;
    }
    layout_area_bump(
        &ChartConfig {
            width,
            height,
            margin: Margin {
                left: 100.,
                right: 100.,
                top: 50.,
                bottom: 50.,
            },
            title: "AreaBump ranked bands".into(),
            ..Default::default()
        },
        &rows,
        &o,
    )
}
fn record(events: &Rc<RefCell<Vec<Value>>>, value: Value) {
    let mut log = events.borrow_mut();
    if log.len() == 64 {
        log.remove(0);
    }
    log.push(value);
}
#[derive(Default)]
struct Peers {
    hover: Option<String>,
    focus: Option<String>,
}
#[wasm_bindgen]
pub fn area_bump_mount(case: u8) -> std::result::Result<(), JsValue> {
    let initial = source(case, 0, 640., 400., BTreeSet::new()).map_err(js)?;
    area_bump_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("area-bump-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            initial.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(400.),
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                tooltip_options: TooltipOptions {
                    value_format: ValueFormat::Fixed(1),
                    ..Default::default()
                },
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(c) => c,
            Err(e) => {
                root.remove();
                area_bump_stop();
                return Err(js(e));
            }
        };
        let phase = Rc::new(Cell::new(0));
        let p = phase.clone();
        let layout = Rc::new(RefCell::new(initial.clone()));
        let cache = layout.clone();
        let viewport = CategoryViewport::new(
            initial.slices.iter().map(|s| s.key.clone()).collect(),
            ClipRect::plot(&ChartConfig {
                width: 640.,
                height: 400.,
                margin: Margin {
                    left: 100.,
                    right: 100.,
                    top: 50.,
                    bottom: 50.,
                },
                ..Default::default()
            }),
            ViewportLimits::default(),
        )
        .map_err(js)?
        .into_controller();
        chart
            .on_viewport(viewport, ViewportSelection::X, move |request| {
                let active = cache.borrow().active_ids.clone();
                let mut next =
                    source(case, p.get(), request.width, request.height, request.hidden)?;
                next.set_active(active)?;
                let scene = next.scene.clone();
                *cache.borrow_mut() = next;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        let cache = layout.clone();
        let weak = chart.handle();
        let peers = Rc::new(RefCell::new(Peers::default()));
        chart.on_event(move|event|{record(&capture,event_value(event.clone()));let base=cache.borrow().clone();let find=|id:&str|base.scene.marks.iter().find(|m|m.id==id&&!m.series.is_empty()).map(|m|m.series.clone());
            let active={let mut state=peers.borrow_mut();match &event{
                BrowserEvent::Interaction(SurfaceEvent::Hover(ids))=>{state.hover=ids.first().and_then(|id|find(id));if let Some(id)=ids.first()&&let Some(m)=base.scene.marks.iter().find(|m|&m.id==id){record(&capture,json!({"type":"area-current","id":m.id,"series":m.series,"value":m.value}));}},
                BrowserEvent::Focus(id)=>state.focus=id.as_deref().and_then(find),_=>return,
            }state.hover.clone().or_else(||state.focus.clone()).into_iter().collect::<BTreeSet<_>>()};
            if active==base.active_ids{return;}match base.with_active(active).and_then(|next|{weak.set_scene(next.scene.clone())?;Ok(next)}){Ok(next)=>*cache.borrow_mut()=next,Err(e)=>record(&capture,json!({"type":"layout-error","message":e.to_string()})),}
        });
        chart.flush().map_err(js)?;
        AREAS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name.into(),
                case,
                phase,
                layout,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn area_bump_phase(phase: u8) -> std::result::Result<(), JsValue> {
    AREAS.with(|slot| {
        for f in slot.borrow().iter() {
            source(f.case, phase, 640., 400., BTreeSet::new()).map_err(js)?;
            f.phase.set(phase);
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn area_bump_flush() -> std::result::Result<(), JsValue> {
    AREAS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
fn target(layout: &AreaBumpLayout, case: u8) -> Option<Value> {
    let preferred = if case == 2 { "Beta" } else { "Alpha:area" };
    let mut candidates = layout
        .cells
        .iter()
        .filter(|c| c.value > 0.)
        .collect::<Vec<_>>();
    candidates.sort_by_key(|c| (c.series_id != preferred, c.column_index != 1));
    for c in candidates {
        let x = c.x_pixel;
        let y = (c.y0 + c.y1) / 2.;
        if let Some(m) = layout.scene.hit_test(x, y)
            && m.series == c.series_id
        {
            return Some(
                json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"sampleValue":c.value,"x":x,"y":y}),
            );
        }
    }
    None
}
#[wasm_bindgen]
pub fn area_bump_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    AREAS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let state=f.chart.snapshot().map_err(js)?;let layout=f.layout.borrow();Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hovered":state.hovered,"selected":state.selected,"hidden":state.hidden,"case":f.case,"phase":f.phase.get(),"target":target(&layout,f.case),"activeIds":layout.active_ids,"maxSum":layout.max_sum,"availableHeight":layout.available_height,"cells":layout.cells.iter().map(|c|json!({"series":c.series_id,"source":c.source_index,"column":c.column_index,"rank":c.rank,"value":c.value,"x":c.x_pixel,"y0":c.y0,"y1":c.y1})).collect::<Vec<_>>(),"paths":layout.series.iter().flat_map(|s|s.mark_ids.iter()).collect::<Vec<_>>(),"owned":layout.scene.label_series.values().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn area_bump_rejected() -> bool {
    AREAS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let layout = f.layout.borrow();
            let mut invalid = layout.clone();
            let accepted = invalid.clone();
            if invalid
                .set_active(BTreeSet::from(["unknown".into()]))
                .is_ok()
                || invalid != accepted
            {
                return false;
            }
            let mut scene = layout.scene.clone();
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn area_bump_stop() {
    let entries = AREAS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
