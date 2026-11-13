use super::*;
struct PointFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: Rc<Cell<u8>>,
    accepted: Rc<RefCell<PointLineLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static POINTS:RefCell<Vec<PointFixture>>=const{RefCell::new(Vec::new())};}
fn key(key: &CategoryKey) -> Value {
    match key {
        CategoryKey::String(s) => json!({"type":"string","value":s}),
        CategoryKey::Number(n) => json!({"type":"number","value":n}),
    }
}
fn proposal(
    phase: u8,
    width: f64,
    height: f64,
    hidden: BTreeSet<String>,
    domains: Option<ResolvedDomains>,
) -> Result<PointLineLayout> {
    if phase > 1 {
        return Err(ChartError("invalid point scale phase".into()));
    }
    let keys = vec![
        CategoryKey::from("1"),
        1_f64.into(),
        "March".into(),
        "April".into(),
    ];
    let alpha = keys
        .iter()
        .cloned()
        .zip([3., 4.25 + 2.5 * f64::from(phase), 7., 5.])
        .map(|(x, y)| PointLineDatum::new(x, y))
        .collect();
    let beta = keys
        .iter()
        .cloned()
        .zip([7., 3., 2., 8.])
        .map(|(x, y)| PointLineDatum::new(x, y))
        .collect();
    let mut line = LineOptions {
        hidden,
        y_domain: Some(domains.and_then(|d| d.left).unwrap_or([0., 10.])),
        ..Default::default()
    };
    line.axes.y.format = ValueFormat::Fixed(2);
    line.axes.x.grid = true;
    point_line_layout(
        &config(width, height),
        &[
            PointLineSeries::new("alpha", alpha),
            PointLineSeries::new("beta", beta),
        ],
        &PointLineOptions {
            line,
            point: PointScaleOptions {
                padding: if phase == 0 { 0. } else { 0.5 },
                align: if phase == 0 { 0.5 } else { 0.8 },
                round: phase == 1,
                reverse: phase == 1,
            },
            domain: Some(keys),
            window: domains.map(|d| d.x),
            ..Default::default()
        },
        &LineStyleOptions {
            series: LineSeriesStyle {
                width: 3.,
                point_radius: 6.,
                ..Default::default()
            },
            ..Default::default()
        },
    )
}
fn bind(f: &PointFixture) -> Result<()> {
    let snapshot = f.chart.snapshot()?;
    let initial = proposal(
        f.phase.get(),
        snapshot.width,
        snapshot.height,
        snapshot.hidden,
        None,
    )?;
    let controller = initial.viewport_controller(ViewportLimits {
        min_zoom: 1.,
        max_zoom: 8.,
        bounded: true,
    })?;
    let phase = f.phase.clone();
    let accepted = f.accepted.clone();
    f.chart
        .on_viewport_with_interaction(controller, ViewportSelection::Both, move |request| {
            let next = proposal(
                phase.get(),
                request.width,
                request.height,
                request.hidden,
                Some(request.domains),
            )?;
            let publish = next.clone().into();
            *accepted.borrow_mut() = next;
            Ok(publish)
        })
}
pub(super) fn mount() -> std::result::Result<(), JsValue> {
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("scale-profiles-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let initial = proposal(0, 640., 400., Default::default(), None).map_err(js)?;
        let accepted = Rc::new(RefCell::new(initial.clone()));
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene,
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(400.),
                motion: browser::MotionPreference::Reduced,
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
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
                let p = layout.borrow().plot;
                let bounds = Brush {
                    x0: bounds.x0 - p.x,
                    y0: bounds.y0 - p.y,
                    x1: bounds.x1 - p.x,
                    y1: bounds.y1 - p.y,
                };
                if let Err(e) = handle.request_viewport(ViewportRequest::Brush {
                    bounds,
                    axes: ViewportSelection::Both,
                }) {
                    log.borrow_mut().push(json!({"brushError":e.0}));
                }
            }
        });
        let f = PointFixture {
            chart,
            root,
            renderer: renderer.into(),
            phase: Rc::new(Cell::new(0)),
            accepted,
            events,
        };
        bind(&f).map_err(js)?;
        POINTS.with(|slot| slot.borrow_mut().push(f));
    }
    Ok(())
}
pub(super) fn phase(phase: u8) -> std::result::Result<(), JsValue> {
    POINTS.with(|slot| {
        for f in slot.borrow().iter() {
            proposal(phase, 640., 400., Default::default(), None).map_err(js)?;
            if f.phase.replace(phase) != phase {
                bind(f).map_err(js)?;
            } else {
                f.chart.handle().refresh_viewport().map_err(js)?;
            }
        }
        Ok(())
    })
}
pub(super) fn flush() -> std::result::Result<(), JsValue> {
    POINTS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
pub(super) fn reset() -> std::result::Result<(), JsValue> {
    POINTS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart
                .handle()
                .request_viewport(ViewportRequest::Reset)
                .map_err(js)?;
        }
        Ok(())
    })
}
pub(super) fn snapshot(renderer: &str) -> std::result::Result<Option<String>, JsValue> {
    POINTS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(None);};let s=f.chart.snapshot().map_err(js)?;let l=f.accepted.borrow();let target=l.sites.iter().find(|p|p.series_id=="alpha"&&p.source_index==1).or_else(||l.sites.iter().find(|p|p.series_id=="alpha")).and_then(|p|{let [x,y]=p.position;[-1.,0.,1.].into_iter().all(|a|[-1.,0.,1.].into_iter().all(|b|l.scene.hit_test(x+a,y+b).is_some_and(|m|m.id==p.mark_id))).then(||json!({"id":p.mark_id,"label":l.scene.marks.iter().find(|m|m.id==p.mark_id).unwrap().label,"x":x,"y":y,"rawX":key(&p.raw_x),"value":p.raw_y,"formattedX":p.formatted_x,"formattedY":p.formatted_y,"source":p.source_index,"mapped":p.position}))});
        let xr=l.x_scale.range();let sites=l.sites.iter().map(|p|json!({"id":p.mark_id,"series":p.series_id,"source":p.source_index,"raw":[key(&p.raw_x),json!(p.raw_y)],"position":p.position,"mapped":[l.x_scale.map(&p.raw_x).unwrap(),l.y_scale.map(p.raw_y)]})).collect::<Vec<_>>();
        Ok(Some(json!({"active":s.active,"revision":s.revision,"pending":s.pending,"width":s.width,"height":s.height,"case":3,"phase":f.phase.get(),"hidden":s.hidden,"hovered":s.hovered,"selected":s.selected,"domains":s.domains.map(|d|json!({"x":d.x,"y":d.left})),"view":{"scale":s.view.scale,"x":s.view.translate_x,"y":s.view.translate_y},"plot":[l.plot.x,l.plot.y,l.plot.width,l.plot.height],"xScale":{"kind":"Point","display":l.x_scale.window(),"source":[0.,3.],"range":xr,"round":f.phase.get()==1,"domain":l.x_scale.domain().iter().map(key).collect::<Vec<_>>(),"positions":l.x_scale.domain().iter().map(|k|l.x_scale.map(k).unwrap()).collect::<Vec<_>>(),"identities":l.x_scale.domain().iter().map(|k|k.identity().unwrap()).collect::<Vec<_>>()},"yScale":{"kind":"Linear","source":l.y_scale.source_domain(),"display":l.y_scale.display_domain(),"range":l.y_scale.range(),"round":false,"clamp":false},"sites":sites,"labels":l.scene.labels.iter().map(|l|json!({"text":l.text,"x":l.x,"y":l.y})).collect::<Vec<_>>(),"owned":l.scene.label_series.values().cloned().collect::<Vec<_>>(),"target":target,"events":f.events.borrow().clone()}).to_string()))})
}
pub(super) fn rejected() -> Option<bool> {
    POINTS.with(|slot| {
        let entries = slot.borrow();
        if entries.is_empty() {
            return None;
        }
        Some(entries.iter().all(|f| {
            let bad = PointScale::new(
                vec![CategoryKey::Number(f64::NAN)],
                [0., 100.],
                &Default::default(),
            )
            .is_err();
            let mut scene = f.accepted.borrow().scene.clone();
            scene.marks[0].opacity = f64::NAN;
            bad && f.chart.set_scene(scene).is_err()
        }))
    })
}
pub(super) fn stop() {
    let entries = POINTS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
