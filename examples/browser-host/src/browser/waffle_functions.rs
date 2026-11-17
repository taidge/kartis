//! Application-owned raw Waffle data with borrowed functions and real live UI.
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, Style};

use super::*;

struct RawPayload {
    status: String,
    raw_metric: f64,
    note: u32,
}
type Payload = WaffleDataCellWidgetData<RawPayload>;
type Row = TypedPlotWidgetFrame<Payload>;
#[derive(Default)]
struct Metrics {
    factories: Cell<u32>,
    scopes: Cell<u32>,
    cleanups: Cell<u32>,
    fault: Cell<u8>,
    calls: RefCell<Vec<Value>>,
    probes: RefCell<Vec<Value>>,
    rows: RefCell<Vec<(u32, glory::Bond<Row>)>>,
}
struct Fixture {
    root: web_sys::Element,
    chart: BrowserChart,
    frame: WaffleDataFrame<RawPayload>,
    metrics: Rc<Metrics>,
    areas: bool,
    phase: u8,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn color(data: &RawPayload, index: usize) -> Color {
    match (data.status.as_str(), index) {
        ("alert", 0) => Color(220, 120, 20, 255),
        ("alert", 1) => Color(20, 170, 80, 255),
        (_, 0) => Color(200, 40, 60, 255),
        (_, 1) => Color(30, 100, 210, 255),
        (_, 2) => Color(100, 110, 120, 255),
        (_, 3) => Color(160, 80, 190, 255),
        _ => Color(30, 180, 180, 255),
    }
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn raw_json(data: &RawPayload) -> Value {
    json!({"status":data.status,"metricBits":data.raw_metric.to_bits().to_string(),"note":data.note})
}
fn group_json(group: &WaffleGroup) -> Value {
    json!({"id":group.source.id,"sourceIndex":group.source_index,"valueBits":group.source.value.to_bits().to_string(),"formatted":group.formatted_value,"hidden":group.hidden,"color":rgba(group.color),"border":rgba(group.border_color),"requested":[group.requested.start,group.requested.end],"occupied":[group.occupied.start,group.occupied.end],"polygons":group.polygons,"areaMarkId":group.area_mark_id})
}
fn row_json(row: &Row) -> Value {
    let computed = row.data.computed();
    json!({"index":computed.cell.index,"row":computed.cell.row,"column":computed.cell.column,"owner":computed.cell.owner,"raw":row.data.data().map(raw_json),"group":computed.group.as_ref().map(|g|group_json(g)),"cohort":row.data.frame().source().iter().map(|d|json!({"id":d.projected.id,"raw":raw_json(&d.data),"valueBits":d.projected.value.to_bits().to_string()})).collect::<Vec<_>>(),"visible":row.frame.visible.map(|r|[r.x,r.y,r.width,r.height]),"interactive":row.frame.interactive})
}
fn make(
    phase: u8,
    areas: bool,
    revision: u64,
    m: &Metrics,
) -> Result<(WaffleDataFrame<RawPayload>, PlotWidgetsComposition<Payload>)> {
    let config = ChartConfig {
        width: 340.,
        height: 240.,
        margin: Margin {
            left: 20.,
            right: 20.,
            top: 20.,
            bottom: 20.,
        },
        title: "Waffle functions / 函数".into(),
        ..Default::default()
    };
    let values = if phase == 3 {
        [1.5, 1.5, 0., 0., 3.]
    } else {
        [1., 1., 1., -0., 0.]
    };
    let source = ["alpha", "beta", "hidden", "zero", "overflow"]
        .into_iter()
        .enumerate()
        .map(|(index, id)| {
            WaffleDatum::new(
                RawPayload {
                    status: if matches!(phase, 1 | 2) {
                        "alert"
                    } else {
                        "ready"
                    }
                    .into(),
                    raw_metric: if phase == 2 { 0. } else { -0. },
                    note: if phase == 2 { 77 } else { 0 },
                },
                RadialDatum::new(id, values[index]),
            )
        })
        .collect();
    let options = WaffleLayoutOptions {
        rows: 2,
        columns: 3,
        total: Some(6.),
        direction: WaffleFillDirection::Bottom,
        mode: if areas {
            WaffleMode::Areas
        } else {
            WaffleMode::Cells
        },
        padding: 8.,
        border_width: 2.,
        empty_color: Color(210, 210, 210, 255),
        hidden: std::collections::BTreeSet::from(["hidden".into()]),
        ..Default::default()
    };
    let mut resolve_color = |c: &WaffleDataColorContext<'_, RawPayload>| {
        m.calls
            .borrow_mut()
            .push(json!({"role":"color","id":c.computed.datum.id,"raw":raw_json(c.data)}));
        if m.fault.get() == 1 && c.computed.source_index == 1 {
            return Err(ChartError("authored Waffle color rejection".into()));
        }
        Ok(color(c.data, c.computed.source_index))
    };
    let mut format = |value: f64| {
        m.calls
            .borrow_mut()
            .push(json!({"role":"format","bits":value.to_bits().to_string()}));
        Ok(if m.fault.get() == 2 {
            "长".repeat(400)
        } else {
            format!("v={value:.1}")
        })
    };
    let mut border = |c: &WaffleDataBorderContext<'_, RawPayload>| match c.computed.target {
        WaffleBorderTarget::Group(group) => {
            m.calls.borrow_mut().push(json!({"role":"border","id":group.source.id,"raw":c.data.map(raw_json),"color":rgba(group.color),"formatted":group.formatted_value,"placeholder":rgba(group.border_color),"polygons":group.polygons.len()}));
            Ok(if c.data.unwrap().status == "alert" {
                Color(0, 0, 0, 255)
            } else {
                Color(100, 30, 180, 255)
            })
        }
        WaffleBorderTarget::Empty(_) => {
            m.calls
                .borrow_mut()
                .push(json!({"role":"empty","raw":c.data.map(raw_json)}));
            if m.fault.get() == 3 {
                return Err(ChartError("authored Waffle empty border rejection".into()));
            }
            Ok(Color(70, 70, 70, 255))
        }
    };
    let mut factories = WaffleDataFactories {
        color: Some(&mut resolve_color),
        formatted_value: Some(&mut format),
        border_color: Some(&mut border),
    };
    let frame = if phase == 4 || m.fault.get() == 4 {
        WaffleDataFrame::with_factories_and_cell_factory(
            &config,
            source,
            &options,
            &mut factories,
            &mut |c: &WaffleDataCellContext<'_, RawPayload>| {
                m.calls.borrow_mut().push(json!({"role":"cell","index":c.computed.cell.index,"raw":c.data.map(raw_json),"fill":c.computed.fill.map(rgba),"stroke":c.computed.stroke.map(|s|rgba(s.color))}));
                if m.fault.get() == 4 {
                    return Err(ChartError("authored Waffle cell rejection".into()));
                }
                if let Some(group) = c.computed.group {
                    scene_style(
                        c.computed.fill == Some(color(c.data.unwrap(), group.source_index)),
                    )?;
                }
                Ok(c.computed.inherit(Geometry::Circle {
                    x: c.computed.position[0],
                    y: c.computed.position[1],
                    radius: 30.,
                }))
            },
        )?
    } else {
        WaffleDataFrame::with_factories(&config, source, &options, &mut factories)?
    };
    let packet = frame.cell_widgets(revision, |c| {
        Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
            x: c.computed.cell.bounds.x + 6.,
            y: c.computed.cell.bounds.y + 6.,
            width: 20.,
            height: 18.,
        })))
    })?;
    Ok((
        frame,
        PlotWidgetsComposition {
            widgets: packet,
            replace_ink: Default::default(),
        },
    ))
}
fn scene_style(ok: bool) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(ChartError(
            "custom cell did not receive final Waffle style".into(),
        ))
    }
}
#[wasm_bindgen]
pub fn waffle_functions_mount(
    key: &str,
    renderer: &str,
    areas: bool,
) -> std::result::Result<(), JsValue> {
    let backend = match renderer {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("unknown Waffle renderer")),
    };
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("waffle-functions-{key}"));
    root.set_attribute("style", "position:relative;width:340px;height:240px")?;
    doc.body().unwrap().append_child(&root)?;
    let metrics = Rc::new(Metrics::default());
    let (frame, composition) = make(0, areas, 0, &metrics).map_err(js)?;
    let captured = metrics.clone();
    let document = doc.clone();
    let chart = BrowserChart::mount_with_plot_widgets_in_scene(
        root.clone(),
        frame.scene().clone(),
        BrowserOptions {
            renderer: backend,
            height: Some(240.),
            motion: browser::MotionPreference::Reduced,
            highlight: browser::BrowserHighlight::Controlled,
            interaction_data: Some(frame.layout().interaction.clone()),
            interaction: surface::SurfaceOptions {
                plot: composition.widgets.layout.plot,
                ..Default::default()
            },
            ..Default::default()
        },
        composition,
        move |context: PlotWidgetCompositionContext<Payload>| {
            captured.factories.set(captured.factories.get() + 1);
            let ctor = captured.factories.get();
            let frame = context.frame;
            let index = frame.with(|f| f.data.computed().cell.index);
            if captured.fault.get() == 5 && index == 4 {
                return Err(ChartError("authored Waffle Widget rejection".into()));
            }
            captured.rows.borrow_mut().push((ctor, frame));
            let m = captured.clone();
            let document = document.clone();
            Ok(glory::ui::widget(move |scope| {
                m.scopes.set(m.scopes.get() + 1);
                let retired = m.clone();
                scope.on_cleanup(move || retired.cleanups.set(retired.cleanups.get() + 1))?;
                let count = scope.cage(0u32);
                glory::ui::button()
                    .class(format!("wf-widget wf-ctor-{ctor}"))
                    .style(scope.bond(move || {
                        Style {
                            background: Some(if frame.with(|f| f.data.data().is_some()) {
                                Color(130, 40, 170, 255)
                            } else {
                                Color(255, 220, 0, 255)
                            }),
                            ..Style::new()
                                .width(Length::Percent(100.))
                                .height(Length::Percent(100.))
                        }
                    }))
                    .label(scope.bond(move || {
                        json!({"ctor":ctor,"count":count.get(),"value":frame.with(row_json)})
                            .to_string()
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                    .show_in(scope)?;
                let watch = m.clone();
                let ordinal = Cell::new(0u32);
                let document = document.clone();
                scope.effect(move || {
                    let observation = ordinal.get();
                    ordinal.set(observation + 1);
                    let own = frame.get();
                    let rows = watch
                        .rows
                        .borrow()
                        .iter()
                        .map(|(id, bond)| {
                            let connected = document
                                .query_selector(&format!(".wf-ctor-{id}"))
                                .ok()
                                .flatten()
                                .is_some_and(|n| n.is_connected());
                            match bond.try_with_untracked(row_json) {
                                Ok(value) => json!({"ctor":id,"connected":connected,"value":value}),
                                Err(e) => {
                                    json!({"ctor":id,"connected":connected,"error":e.to_string()})
                                }
                            }
                        })
                        .collect::<Vec<_>>();
                    watch.probes.borrow_mut().push(
                        json!({"ctor":ctor,"ordinal":observation,"own":row_json(&own),"rows":rows}),
                    );
                });
                Ok(())
            }))
        },
    )
    .map_err(js)?;
    chart.flush().map_err(js)?;
    FIXTURES.with_borrow_mut(|f| {
        f.insert(
            key.into(),
            Fixture {
                root,
                chart,
                frame,
                metrics,
                areas,
                phase: 0,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn waffle_functions_update(
    key: &str,
    phase: u8,
    revision: u32,
    fault: u8,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all
            .get_mut(key)
            .ok_or_else(|| js("missing Waffle functions fixture"))?;
        f.metrics.fault.set(fault);
        let (frame, composition) =
            make(phase, f.areas, u64::from(revision), &f.metrics).map_err(js)?;
        f.chart
            .set_layout_with_plot_widgets_in_scene(frame.interaction(), composition)
            .map_err(js)?;
        f.chart.flush().map_err(js)?;
        f.frame = frame;
        f.phase = phase;
        Ok(())
    })
}
#[wasm_bindgen]
pub fn waffle_functions_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{let f=all.get(key).ok_or_else(||js("missing Waffle functions fixture"))?;let state=f.chart.snapshot().map_err(js)?;
        Ok(json!({"active":state.active,"revision":state.revision,"phase":f.phase,"factories":f.metrics.factories.get(),"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get(),"calls":*f.metrics.calls.borrow(),"probes":*f.metrics.probes.borrow(),"groups":f.frame.layout().groups.iter().map(group_json).collect::<Vec<_>>(),"raw":f.frame.source().iter().map(|d|json!({"id":d.projected.id,"valueBits":d.projected.value.to_bits().to_string(),"raw":raw_json(&d.data)})).collect::<Vec<_>>(),"cells":f.frame.layout().cells.iter().map(|c|json!({"index":c.index,"owner":c.owner,"markId":c.mark_id,"bounds":[c.bounds.x,c.bounds.y,c.bounds.width,c.bounds.height]})).collect::<Vec<_>>(),"sites":f.frame.layout().interaction.sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"color":rgba(s.color),"formatted":s.formatted_value,"value":s.value})).collect::<Vec<_>>(),"legend":f.frame.scene().legend.iter().map(|l|json!({"id":l.id,"color":rgba(l.color)})).collect::<Vec<_>>(),"sceneDebug":format!("{:?}",f.frame.scene())}).to_string())
    })
}
#[wasm_bindgen]
pub fn waffle_functions_stop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all|{let f=all.remove(key).ok_or_else(||js("missing Waffle functions fixture"))?;f.chart.stop();f.root.remove();Ok(json!({"factories":f.metrics.factories.get(),"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get()}).to_string())})
}
