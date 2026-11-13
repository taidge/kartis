//! Actual D3 scale options shared by line ink, axes and typed input sites.
use super::*;
use std::collections::BTreeSet;
mod point;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<ScaledLineLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static SCALES:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Resolved scale profiles".into(),
        margin: Margin {
            left: 76.,
            right: 30.,
            top: 35.,
            bottom: 65.,
        },
        ..Default::default()
    }
}
fn profile(
    case: u8,
    phase: u8,
) -> Result<(Vec<Series>, LineOptions, LineStyleOptions, LineScaleOptions)> {
    if case > 5 || case == 3 || phase > 1 {
        return Err(ChartError("invalid scale case/phase".into()));
    }
    let p = f64::from(phase);
    let mut opts = LineOptions::default();
    let mut scales = LineScaleOptions::default();
    let data = match case {
        0 => {
            opts.x_domain = Some([0.13, 9.2]);
            opts.y_domain = Some([0.3, 8.1]);
            scales.x = ContinuousScaleOptions {
                nice: ScaleNice::Count(if phase == 0 { 5 } else { 3 }),
                reverse: phase == 1,
                round: phase == 1,
                clamp: phase == 1,
                ..Default::default()
            };
            scales.y = scales.x;
            vec![
                Series::new(
                    "alpha",
                    vec![
                        Point::new(-3., 11.5 + p),
                        Point::new(3.3, 4.25 + p * 2.5),
                        Point::new(7.2, 7.4),
                    ],
                ),
                Series::new(
                    "beta",
                    vec![
                        Point::new(1.4, 2.),
                        Point::new(4.6, 3.),
                        Point::new(8.2, 5.),
                    ],
                ),
            ]
        }
        1 => {
            let sign = if phase == 0 { 1. } else { -1. };
            opts.x_domain = Some(if phase == 0 { [0.7, 90.] } else { [-90., -0.7] });
            opts.y_domain = Some(if phase == 0 { [-13., 41.] } else { [-41., 13.] });
            scales.x = ContinuousScaleOptions {
                kind: ScaleKind::Log,
                nice: ScaleNice::Default,
                log_base: if phase == 0 { 2. } else { std::f64::consts::E },
                ..Default::default()
            };
            scales.y = ContinuousScaleOptions {
                kind: ScaleKind::SymLog {
                    constant: if phase == 0 { 2. } else { 0.5 },
                },
                nice: ScaleNice::Count(5),
                ..Default::default()
            };
            vec![
                Series::new(
                    "alpha",
                    vec![
                        Point::new(sign * 1.3, -8. + p * 2.),
                        Point::new(sign * 9., 6. - p * 17.),
                        Point::new(sign * 63., 31. - p * 43.),
                    ],
                ),
                Series::new(
                    "beta",
                    vec![
                        Point::new(sign * 2.5, 13. - p * 15.),
                        Point::new(sign * 17., -3. - p * 15.),
                        Point::new(sign * 80., 23. - p * 28.),
                    ],
                ),
            ]
        }
        2 => {
            const FEB27: f64 = 1_708_992_000_000.;
            const DAY: f64 = 86_400_000.;
            opts.x_domain = Some([FEB27 + DAY * 0.5, FEB27 + DAY * 5.125]);
            opts.y_domain = Some([0.4, 9.3]);
            scales.x = ContinuousScaleOptions {
                kind: ScaleKind::Utc,
                nice: ScaleNice::Count(4),
                reverse: phase == 1,
                round: phase == 1,
                ..Default::default()
            };
            scales.y.nice = ScaleNice::Default;
            vec![
                Series::new(
                    "alpha",
                    vec![
                        Point::new(FEB27 + DAY * 1.5, 3.),
                        Point::new(FEB27 + DAY * 3.25, 4.25 + p * 2.5),
                        Point::new(FEB27 + DAY * 4.5, 8.),
                    ],
                ),
                Series::new(
                    "beta",
                    vec![
                        Point::new(FEB27 + DAY * 0.75, 7.),
                        Point::new(FEB27 + DAY * 2.75, 2.),
                        Point::new(FEB27 + DAY * 4.8, 5.),
                    ],
                ),
            ]
        }
        _ => {
            let reciprocal = case == 5;
            let sign = if reciprocal && phase == 1 { -1. } else { 1. };
            opts.x_domain = Some(if reciprocal {
                if phase == 0 { [1., 16.] } else { [-16., -1.] }
            } else {
                [0., 16.]
            });
            opts.y_domain = Some([0., 16.]);
            scales.x = ContinuousScaleOptions {
                kind: if reciprocal {
                    ScaleKind::Pow { exponent: -1. }
                } else {
                    ScaleKind::Sqrt
                },
                reverse: phase == 1,
                round: phase == 1,
                clamp: !reciprocal && phase == 1,
                ..Default::default()
            };
            scales.y = ContinuousScaleOptions {
                kind: if reciprocal {
                    ScaleKind::Sqrt
                } else {
                    ScaleKind::Pow { exponent: 2. }
                },
                reverse: phase == 1,
                round: phase == 1,
                clamp: phase == 1,
                ..Default::default()
            };
            vec![
                Series::new(
                    "alpha",
                    vec![
                        Point::new(sign * if reciprocal { 1. } else { 0. }, 1.),
                        Point::new(
                            sign * if reciprocal { 3. } else { 4. + p * 5. },
                            (if reciprocal { 4. } else { 8. }) + p * 5.,
                        ),
                        Point::new(sign * 16., 16.),
                    ],
                ),
                Series::new(
                    "beta",
                    vec![
                        Point::new(sign * 2., 9.),
                        Point::new(sign * 8., 2.),
                        Point::new(sign * 12., 12.),
                    ],
                ),
            ]
        }
    };
    opts.points = true;
    opts.axes.x.tick_count = if case == 1 { 5 } else { 4 };
    opts.axes.y.tick_count = 5;
    opts.axes.y.format = ValueFormat::Fixed(2);
    if case == 2 {
        opts.axes.x.format = ValueFormat::UtcDate;
        opts.axes.x.tick_rotation = -20.;
    }
    let style = LineStyleOptions {
        series: LineSeriesStyle {
            width: 3.,
            point_radius: 6.,
            point_border: Some(GraphicStroke {
                color: Color(255, 255, 255, 255),
                width: 1.,
                cap: StrokeCap::Round,
            }),
            ..Default::default()
        },
        ..Default::default()
    };
    Ok((data, opts, style, scales))
}
fn proposal(
    case: u8,
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
    domains: Option<ResolvedDomains>,
) -> Result<ScaledLineLayout> {
    let (data, mut opts, style, mut scales) = profile(case, phase)?;
    let c = config(width, height);
    opts.hidden = hidden;
    if let Some(domains) = domains {
        let base_x = ResolvedContinuousScale::new(opts.x_domain.unwrap(), [0., 1.], &scales.x)?
            .display_domain();
        let base_y = ResolvedContinuousScale::new(opts.y_domain.unwrap(), [0., 1.], &scales.y)?
            .display_domain();
        if domains.x != base_x {
            opts.x_domain = Some(domains.x);
            scales.x.nice = ScaleNice::Off;
        }
        if let Some(y) = domains.left
            && y != base_y
        {
            opts.y_domain = Some(y);
            scales.y.nice = ScaleNice::Off;
        }
    }
    line_layout_with_scale_options(&c, &data, &opts, &style, &scales)
}
fn bind(f: &Fixture) -> Result<()> {
    let snapshot = f.chart.snapshot()?;
    let next = proposal(
        f.case,
        f.phase.get(),
        snapshot.width,
        snapshot.height,
        snapshot.hidden,
        None,
    )?;
    let controller = next.viewport_controller(ViewportLimits {
        min_zoom: 1.,
        max_zoom: 8.,
        bounded: true,
    })?;
    let case = f.case;
    let phase = f.phase.clone();
    let accepted = f.accepted.clone();
    f.chart
        .on_viewport_with_interaction(controller, ViewportSelection::Both, move |request| {
            let next = proposal(
                case,
                phase.get(),
                request.width,
                request.height,
                request.hidden,
                Some(request.domains),
            )?;
            // Application-owned typed tooltip rows include formatted X as well
            // as the shared formatted Y, including real UTC dates.
            let mut binding = next.interaction_binding();
            for row in &mut binding.sites {
                let site = next
                    .layout
                    .sites
                    .iter()
                    .find(|s| s.mark_id == row.mark_id)
                    .unwrap();
                row.label = format!("{} · {}", site.series_id, site.formatted_x);
            }
            let published = SceneInteraction {
                scene: next.layout.scene.clone(),
                plot: Some(next.layout.plot),
                interaction: Some(binding),
            };
            *accepted.borrow_mut() = next;
            Ok(published)
        })
}
#[wasm_bindgen]
pub fn scale_profiles_mount(case: u8) -> std::result::Result<(), JsValue> {
    scale_profiles_stop();
    if case == 3 {
        return point::mount();
    }
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("scale-profiles-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = proposal(case, 0, 640., 400., Default::default(), None).map_err(js)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.layout.scene.clone(),
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions::default(),
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        let accepted = Rc::new(RefCell::new(initial));
        let layout = accepted.clone();
        let handle = chart.handle();
        chart.on_event(move |event| {
            log.borrow_mut().push(json!({"event":format!("{event:?}")}));
            if let BrowserEvent::Interaction(SurfaceEvent::Brush {
                bounds,
                complete: true,
                ..
            }) = event
            {
                log.borrow_mut()
                    .push(json!({"brush":[bounds.x0,bounds.y0,bounds.x1,bounds.y1]}));
                let plot = layout.borrow().layout.plot;
                let bounds = Brush {
                    x0: bounds.x0 - plot.x,
                    y0: bounds.y0 - plot.y,
                    x1: bounds.x1 - plot.x,
                    y1: bounds.y1 - plot.y,
                };
                if let Err(error) = handle.request_viewport(ViewportRequest::Brush {
                    bounds,
                    axes: ViewportSelection::Both,
                }) {
                    log.borrow_mut().push(json!({"brushError":error.0}));
                }
            }
        });
        let f = Fixture {
            chart,
            root,
            renderer: renderer.into(),
            case,
            phase: Rc::new(Cell::new(0)),
            accepted,
            events,
        };
        bind(&f).map_err(js)?;
        SCALES.with(|slot| slot.borrow_mut().push(f));
    }
    Ok(())
}
#[wasm_bindgen]
pub fn scale_profiles_phase(phase: u8) -> std::result::Result<(), JsValue> {
    point::phase(phase)?;
    SCALES.with(|slot| {
        for f in slot.borrow().iter() {
            profile(f.case, phase).map_err(js)?;
            if f.phase.replace(phase) != phase {
                bind(f).map_err(js)?;
            } else {
                f.chart.handle().refresh_viewport().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scale_profiles_flush() -> std::result::Result<(), JsValue> {
    point::flush()?;
    SCALES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scale_profiles_reset() -> std::result::Result<(), JsValue> {
    point::reset()?;
    SCALES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart
                .handle()
                .request_viewport(ViewportRequest::Reset)
                .map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn scale_profiles_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    if let Some(snapshot) = point::snapshot(renderer)? {
        return Ok(snapshot);
    }
    SCALES.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string());};let snapshot=f.chart.snapshot().map_err(js)?;let l=f.accepted.borrow();let scene=&l.layout.scene;
        let preferred=if f.case==0&&f.phase.get()==1{0}else{1};
        let target=l.layout.sites.iter().filter(|s|s.series_id=="alpha"&&s.source_index==preferred).chain(l.layout.sites.iter().filter(|s|s.series_id=="alpha"&&s.source_index!=preferred)).chain(l.layout.sites.iter().filter(|s|s.series_id!="alpha")).find_map(|site|{
            let [x,y]=site.position;
            for (dx,dy)in [(0.,0.),(2.,0.),(-2.,0.),(2.5,2.5),(-2.5,2.5),(2.5,-2.5),(-2.5,-2.5)]{
                let p=(x+dx,y+dy);let safe=[-1.,0.,1.].into_iter().all(|a|[-1.,0.,1.].into_iter().all(|b|scene.hit_test(p.0+a,p.1+b).is_some_and(|m|m.id==site.mark_id)));
                if safe {let m=scene.marks.iter().find(|m|m.id==site.mark_id).unwrap();return Some(json!({"id":site.mark_id,"label":m.label,"x":p.0,"y":p.1,"rawX":site.raw_x,"value":site.raw_y,"formattedX":site.formatted_x,"formattedY":site.formatted_y,"mapped":site.position,"source":site.source_index}));}
            }None
        });
        let scale=|s:&ResolvedContinuousScale|json!({"source":s.source_domain(),"display":s.display_domain(),"range":s.range(),"kind":format!("{:?}",s.chart_scale().kind()),"round":s.chart_scale().is_rounded(),"clamp":s.chart_scale().is_clamped(),"ticks":s.ticks(if f.case==1{5}else{4}),"mappedSites":l.layout.sites.iter().map(|p|if s.chart_scale().kind()==ScaleKind::Utc{json!(s.map(p.raw_x))}else{Value::Null}).collect::<Vec<_>>()});
        let sites=l.layout.sites.iter().map(|p|json!({"id":p.mark_id,"series":p.series_id,"source":p.source_index,"raw":[p.raw_x,p.raw_y],"position":p.position,"mapped":[l.x_scale.map(p.raw_x),l.y_scale.map(p.raw_y)]})).collect::<Vec<_>>();
        Ok(json!({"active":snapshot.active,"revision":snapshot.revision,"pending":snapshot.pending,"width":snapshot.width,"height":snapshot.height,"case":f.case,"phase":f.phase.get(),"hidden":snapshot.hidden,"hovered":snapshot.hovered,"selected":snapshot.selected,"domains":snapshot.domains.map(|d|json!({"x":d.x,"y":d.left})),"view":{"scale":snapshot.view.scale,"x":snapshot.view.translate_x,"y":snapshot.view.translate_y},"plot":[l.layout.plot.x,l.layout.plot.y,l.layout.plot.width,l.layout.plot.height],"xScale":scale(&l.x_scale),"yScale":scale(&l.y_scale),"sites":sites,"labels":scene.labels.iter().map(|l|json!({"text":l.text,"x":l.x,"y":l.y})).collect::<Vec<_>>(),"owned":scene.label_series.values().cloned().collect::<Vec<_>>(),"target":target,"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn scale_profiles_rejected() -> bool {
    if let Some(rejected) = point::rejected() {
        return rejected;
    }
    SCALES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let bad = ResolvedContinuousScale::new(
                [0., 1.],
                [0., 100.],
                &ContinuousScaleOptions {
                    kind: ScaleKind::Log,
                    ..Default::default()
                },
            )
            .is_err();
            let mut scene = f.accepted.borrow().layout.scene.clone();
            scene.marks[0].opacity = f64::NAN;
            let mut binding = f.accepted.borrow().interaction_binding();
            binding.sites[0].position[0] = f64::NAN;
            bad && f.chart.set_scene(scene).is_err()
                && f.chart
                    .set_scene_with_interaction(f.accepted.borrow().layout.scene.clone(), binding)
                    .is_err()
        })
    })
}
#[wasm_bindgen]
pub fn scale_profiles_stop() {
    point::stop();
    let entries = SCALES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
