//! Real native Funnel hooks and a source-coherent retained tooltip.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, PointerEvents, Style};
use kartis::browser::{
    BrowserFunnel, BrowserFunnelHandle, BrowserFunnelMouseActivity, BrowserFunnelMouseEvent,
    BrowserFunnelMouseHandlers, BrowserFunnelSvgLayerContext, BrowserFunnelSvgProgram,
    BrowserFunnelTooltipComponentContext, BrowserFunnelTooltipOptions, FunnelMouseHandler,
    FunnelTooltipFrame,
};

use super::funnel_svg_layers::{config, data_json, layer_json, options, owner_json};
use super::*;

type TipBond = glory::Bond<Option<FunnelTooltipFrame>>;
type BatchControl = (u32, Box<dyn Fn() -> Value>);
type LayerBond = glory::Bond<Option<kartis::browser::BrowserFunnelSvgLayerFrame>>;
#[derive(Default)]
struct Metrics {
    owner: RefCell<Option<BrowserFunnelHandle>>,
    builds: Cell<u32>,
    calls: Cell<u32>,
    fault: Cell<u8>,
    action: Cell<u8>,
    constructors: Cell<u32>,
    scopes: Cell<u32>,
    cleanups: Cell<u32>,
    tip_constructors: Cell<u32>,
    tip_scopes: Cell<u32>,
    tip_cleanups: Cell<u32>,
    rows: RefCell<Vec<(u32, LayerBond)>>,
    tips: RefCell<Vec<(u32, TipBond)>>,
    events: RefCell<Vec<Value>>,
    observations: RefCell<Vec<Value>>,
    actions: RefCell<Vec<Value>>,
    batches: RefCell<Vec<BatchControl>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserFunnel,
    metrics: Rc<Metrics>,
    profile: u8,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn source(phase: u8) -> Vec<FunnelDatum> {
    let mut rows = super::funnel_svg_layers::source(phase);
    if phase == 5 {
        rows[0].label = "<img src=x onerror=alert(1)> & 漏斗".into();
        rows[1].label = String::new();
    }
    rows
}
fn tooltip_json(f: &FunnelTooltipFrame) -> Value {
    json!({"data":data_json(&f.data),"partIndex":f.part_index,"sourceIndex":f.source_index,
        "generation":f.source_generation,"publication":f.publication,
        "occurrence":{"key":f.occurrence.key,"index":f.occurrence.index,"publication":f.occurrence.publication},
        "client":f.pointer_client,"viewportAnchor":f.viewport_anchor,
        "requestedAnchor":format!("{:?}",f.requested_anchor),"anchor":format!("{:?}",f.anchor)})
}
fn read_result(value: std::result::Result<Option<Value>, String>) -> Value {
    match value {
        Ok(v) => json!({"Ok":v}),
        Err(e) => json!({"Err":e}),
    }
}
fn snapshot(h: &BrowserFunnelHandle, m: &Metrics) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"owner":owner_json(h)?,"tooltip":h.tooltip()?.as_ref().map(tooltip_json),
        "eventErrors":s.event_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "builds":m.builds.get(),"functions":m.calls.get(),"constructors":m.constructors.get(),"scopes":m.scopes.get(),"cleanups":m.cleanups.get(),
        "tipConstructors":m.tip_constructors.get(),"tipScopes":m.tip_scopes.get(),"tipCleanups":m.tip_cleanups.get(),
        "layers":m.rows.borrow().iter().map(|(ctor,r)|json!({"ctor":ctor,"location":location("layer",*ctor),"frame":read_result(r.try_with_untracked(|v|v.as_ref().map(layer_json)).map_err(|e|e.to_string()))})).collect::<Vec<_>>(),
        "tips":m.tips.borrow().iter().map(|(ctor,r)|json!({"ctor":ctor,"location":location("tooltip",*ctor),"frame":read_result(r.try_with_untracked(|v|v.as_ref().map(tooltip_json)).map_err(|e|e.to_string()))})).collect::<Vec<_>>() }),
    )
}
fn location(role: &str, ctor: u32) -> Value {
    let document = web_sys::window().unwrap().document().unwrap();
    let node = document
        .query_selector(&format!(".fie-{role}-ctor-{ctor}"))
        .ok()
        .flatten();
    let connected = node.as_ref().is_some_and(|n| n.is_connected());
    let visible = node.as_ref().is_some_and(|n| {
        n.get_bounding_client_rect().width() > 0.
            && n.get_bounding_client_rect().height() > 0.
            && n.closest("[inert]").ok().flatten().is_none()
    });
    let stage = node
        .as_ref()
        .and_then(|n| n.closest(".kartis-funnel-svg-layers").ok().flatten());
    let record = node
        .as_ref()
        .and_then(|n| n.closest(".kartis-funnel-tooltip-component").ok().flatten());
    let accepted = if role == "layer" {
        stage
            .as_ref()
            .is_some_and(|n| n.get_attribute("data-funnel-svg-active").as_deref() == Some("true"))
    } else {
        record.as_ref().is_some_and(|n| {
            n.parent_element()
                .is_some_and(|p| p.class_list().contains("kartis-funnel-tooltip"))
                && (n.dyn_ref::<web_sys::HtmlElement>().is_some_and(|n| {
                    n.style().get_property_value("display").ok().as_deref() != Some("none")
                }) || n.has_attribute("inert"))
        })
    };
    json!({"connected":connected,"visible":visible,"accepted":accepted,"publication":stage.and_then(|n|n.get_attribute("data-funnel-svg-publication"))})
}
fn observe(m: &Metrics, role: &str, ctor: u32, own: Value) {
    let owner = m.owner.borrow().as_ref().cloned();
    let sampled = owner.as_ref().map(|h| match snapshot(h, m) {
        Ok(v) => json!({"Ok":v}),
        Err(e) => json!({"Err":e.to_string()}),
    });
    m.observations.borrow_mut().push(
        json!({"role":role,"ctor":ctor,"location":location(role,ctor),"own":own,"owner":sampled}),
    );
}
fn program(mode: u8, m: Rc<Metrics>) -> Result<BrowserFunnelSvgProgram> {
    let mut layers = vec![FunnelSvgLayer::new(
        "separators",
        FunnelSvgLayerKind::Separators,
    )?];
    if mode != 1 {
        layers.push(FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?);
    }
    layers.push(FunnelSvgLayer::new(
        "overlay",
        FunnelSvgLayerKind::Custom("overlay".into()),
    )?);
    if mode == 2 {
        layers.push(FunnelSvgLayer::new("parts-top", FunnelSvgLayerKind::Parts)?);
    }
    layers.push(FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?);
    BrowserFunnelSvgProgram::new(layers)?
        .custom("overlay", 0, move |ctx: BrowserFunnelSvgLayerContext| {
            m.constructors.set(m.constructors.get() + 1);
            let frame = ctx.frame;
            let ctor = m.constructors.get();
            m.rows.borrow_mut().push((ctor, frame));
            let m = m.clone();
            Ok(glory::ui::widget(move |scope| {
                m.scopes.set(m.scopes.get() + 1);
                let cleanup = m.clone();
                scope.on_cleanup(move || cleanup.cleanups.set(cleanup.cleanups.get() + 1))?;
                let observed = m.clone();
                scope.effect(move || {
                    observe(
                        &observed,
                        "layer",
                        ctor,
                        frame.get().as_ref().map(layer_json).unwrap_or(Value::Null),
                    )
                });
                glory::ui::graphic()
                    .class(format!("fie-layer-graphic fie-layer-ctor-{ctor}"))
                    .drawing(scope.bond(move || GraphicProps {
                        width: 24.,
                        height: 18.,
                        commands: vec![
                            glory::render::PathCommand::Move(glory::render::GraphicPoint::new(
                                0., 0.,
                            )),
                            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(
                                24., 0.,
                            )),
                            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(
                                24., 18.,
                            )),
                            glory::render::PathCommand::Line(glory::render::GraphicPoint::new(
                                0., 18.,
                            )),
                            glory::render::PathCommand::Close,
                        ],
                        fill: Some(Color(15, 160, 95, 255)),
                        ..Default::default()
                    }))
                    .style(
                        Style::new()
                            .position(glory::style::Position::Absolute)
                            .left(Length::Px(22.))
                            .top(Length::Px(10.))
                            .width(Length::Px(24.))
                            .height(Length::Px(18.))
                            .pointer_events(PointerEvents::None),
                    )
                    .show_in(scope)?;
                Ok(())
            }))
        })
        .and_then(|p| {
            let p = p.interactive(mode != 3);
            if mode == 20 || mode == 21 {
                p.with_budget(FunnelSvgBudget {
                    components: if mode == 20 { 2 } else { 3 },
                    ..Default::default()
                })
            } else {
                Ok(p)
            }
        })
}
fn handler(m: Rc<Metrics>, registration: u8) -> FunnelMouseHandler {
    Box::new(move |e: BrowserFunnelMouseEvent| {
        let h = m
            .owner
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| ChartError("fixture owner missing".into()))?;
        let target = e
            .input
            .event
            .target()
            .and_then(|t| t.dyn_into::<web_sys::Element>().ok());
        let event_id = web_sys::js_sys::Reflect::get(e.input.event.as_ref(), &"fieEventId".into())
            .ok()
            .and_then(|v| v.as_f64());
        let rectangle = target
            .as_ref()
            .and_then(|t| t.closest(".kartis-funnel").ok().flatten())
            .map(|n| {
                let r = n.get_bounding_client_rect();
                [r.x(), r.y(), r.width(), r.height()]
            });
        let target_publication = target
            .as_ref()
            .and_then(|t| t.closest(".kartis-funnel-svg-layers").ok().flatten())
            .and_then(|n| n.get_attribute("data-funnel-svg-publication"));
        m.events.borrow_mut().push(json!({"registration":registration,"kind":format!("{:?}",e.input.kind),"data":data_json(&e.data),"generation":e.source_generation,"publication":e.publication,"partIndex":e.part_index,"sourceIndex":e.source_index,
            "rootRect":rectangle,"targetPublication":target_publication,"pointer":e.input.pointer.as_ref().map(|p|json!({"id":p.id,"kind":p.kind,"primary":p.primary,"captured":p.captured})),"eventPhase":e.input.event.event_phase(),"currentTargetWindow":e.input.event.current_target().as_ref().is_some_and(|t|web_sys::js_sys::Object::is(t.as_ref(),web_sys::window().unwrap().as_ref())),"nativeType":e.input.event.type_(),"trusted":e.input.event.is_trusted(),"eventId":event_id,"button":e.input.event.button(),"buttons":e.input.event.buttons(),"detail":e.input.event.detail(),
            "client":e.input.client,"viewport":e.input.viewport,"scene":e.input.scene,"occurrence":{"key":e.input.occurrence.key,"index":e.input.occurrence.index,"publication":e.input.occurrence.publication},
            "targetId":target.as_ref().and_then(|t|t.get_attribute("data-id")),"connected":target.as_ref().is_some_and(|t|t.is_connected()),
            "owner":snapshot(&h,&m)?}));
        match m.action.get() {
            1 => return Err(ChartError("authored Funnel handler rejection".into())),
            2 => {
                h.set_source(source(1))?;
                let result = h.flush();
                m.actions.borrow_mut().push(json!({"kind":"reentrant-flush","rejected":result.is_err(),"error":result.err().map(|e|e.to_string())}));
            }
            3 => h.stop(),
            4 => h.clear_mouse_handlers()?,
            5 => {
                m.action.set(0);
                if let Some(target) = target {
                    let function = web_sys::js_sys::Function::new_with_args(
                        "target,x,y",
                        "target.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,clientX:x,clientY:y,pointerType:'mouse',isPrimary:true,pointerId:1}))",
                    );
                    let result = function.call3(
                        &JsValue::NULL,
                        target.as_ref(),
                        &e.input.client[0].into(),
                        &e.input.client[1].into(),
                    );
                    m.actions
                        .borrow_mut()
                        .push(json!({"kind":"recursive-native","threw":result.is_err()}));
                }
            }
            6 => {
                m.action.set(0);
                install(
                    &h,
                    m.clone(),
                    BrowserFunnelMouseActivity::NativeComponent,
                    7,
                )?;
            }
            _ => {}
        }
        Ok(())
    })
}
fn install(
    h: &BrowserFunnelHandle,
    m: Rc<Metrics>,
    activity: BrowserFunnelMouseActivity,
    registration: u8,
) -> Result<()> {
    h.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_mouse_enter: Some(handler(m.clone(), registration)),
        on_mouse_move: Some(handler(m.clone(), registration)),
        on_mouse_leave: Some(handler(m.clone(), registration)),
        on_click: Some(handler(m, registration)),
        activity,
    })
}
fn component(h: &BrowserFunnelHandle, m: Rc<Metrics>, revision: u64) -> Result<()> {
    h.set_tooltip_component(revision,move |ctx:BrowserFunnelTooltipComponentContext| {
        m.tip_constructors.set(m.tip_constructors.get()+1);
        if m.fault.get()==2 {return Err(ChartError("authored Funnel tooltip constructor rejection".into()));}
        let frame=ctx.frame;let ctor=m.tip_constructors.get();m.tips.borrow_mut().push((ctor,frame));let m=m.clone();
        Ok(glory::ui::widget(move |scope| {
            m.tip_scopes.set(m.tip_scopes.get()+1);let cleanup=m.clone();scope.on_cleanup(move ||cleanup.tip_cleanups.set(cleanup.tip_cleanups.get()+1))?;
            let batch_scope=scope.downgrade();let batch_metrics=Rc::downgrade(&m);
            m.batches.borrow_mut().push((ctor,Box::new(move || {
                let Some(scope)=batch_scope.upgrade() else{return json!({"invoked":false,"scopeGone":true});};
                let Some(metrics)=batch_metrics.upgrade() else{return json!({"invoked":false,"metricsGone":true});};
                let handle=metrics.owner.borrow().as_ref().cloned().unwrap();let invoked=Cell::new(false);
                let result=scope.batch(|| {invoked.set(true);handle.refresh()?;handle.flush()?;snapshot(&handle,&metrics)});
                match result {Ok(Ok(value))=>json!({"invoked":invoked.get(),"error":null,"value":value}),Ok(Err(e))=>json!({"invoked":invoked.get(),"error":e.to_string()}),Err(e)=>json!({"invoked":invoked.get(),"error":e.to_string()})}
            })));
            let count=scope.cage(0u32);let observed=m.clone();scope.effect(move ||observe(&observed,"tooltip",ctor,frame.get().as_ref().map(tooltip_json).unwrap_or(Value::Null)));
            let styled=m.clone();
            glory::ui::button().class(format!("fie-tooltip-button fie-tooltip-ctor-{ctor}")).label(scope.bond(move ||json!({"revision":revision,"count":count.get(),"frame":frame.get().as_ref().map(tooltip_json)}).to_string()))
                .style(scope.bond(move || {let _accepted=frame.get();Style{background:Some(Color(170,60,200,255)),..Style::new().width(Length::Px(if styled.fault.get()==3 {f64::NAN}else{90.})).height(Length::Px(32.)).pointer_events(PointerEvents::None)}}))
                .on(EventKind::Activate,move |_|{count.revise(|v|*v+=1);}).show_in(scope)?;
            Ok(())
        }))
    })
}
#[wasm_bindgen]
pub fn funnel_interaction_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        if all.contains_key(key) {
            return Err(js("duplicate interaction fixture"));
        }
        let doc = web_sys::window().unwrap().document().unwrap();
        let root = doc.create_element("div")?;
        root.set_id(&format!("funnel-interaction-{key}"));
        root.set_attribute("style", "position:relative;width:420px;height:300px")?;
        doc.body().unwrap().append_child(&root)?;
        let m = Rc::new(Metrics::default());
        let built = m.clone();
        let owner = BrowserFunnel::mount_with_factory_and_svg_layers(
            root.clone(),
            source(0),
            config(),
            options(profile),
            program(0, m.clone()).map_err(js)?,
            move |c| {
                built.builds.set(built.builds.get() + 1);
                if built.fault.get() == 1 {
                    return Err(ChartError(
                        "authored Funnel interaction source rejection".into(),
                    ));
                }
                let mut color = |ctx: &FunnelColorContext<'_>| {
                    built.calls.set(built.calls.get() + 1);
                    Ok(ctx.default_color)
                };
                let mut format = |value: f64| Ok(format!("value={value:.1}"));
                let mut border = |_: &FunnelPartStyleContext<'_>| Ok(Color(30, 30, 30, 255));
                let mut label = |_: &FunnelPartStyleContext<'_>| Ok(Color(70, 50, 20, 255));
                FunnelSvgFrame::with_factories(
                    c.config,
                    c.source,
                    c.options,
                    &mut FunnelFactories {
                        color: Some(&mut color),
                        formatted_value: Some(&mut format),
                        border_color: Some(&mut border),
                        label_color: Some(&mut label),
                    },
                )
            },
        )
        .map_err(js)?;
        *m.owner.borrow_mut() = Some(owner.handle());
        install(
            &owner.handle(),
            m.clone(),
            BrowserFunnelMouseActivity::NativeComponent,
            0,
        )
        .map_err(js)?;
        all.insert(
            key.into(),
            Fixture {
                root,
                owner,
                metrics: m,
                profile,
            },
        );
        Ok(())
    })
}
#[wasm_bindgen]
pub fn funnel_interaction_operation(
    key: &str,
    operation: &str,
    value: f64,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all
            .get_mut(key)
            .ok_or_else(|| js("missing Funnel interaction fixture"))?;
        let h = f.owner.handle();
        let n = value as u8;
        match operation {
            "source" => h.set_source(source(n)).map_err(js)?,
            "queue-current" => {
                h.set_current_part(None).map_err(js)?;
                return Ok(());
            }
            "program" => h
                .set_svg_program(program(n, f.metrics.clone()).map_err(js)?)
                .map_err(js)?,
            "tip" => {
                let opts = BrowserFunnelTooltipOptions {
                    enabled: n != 0,
                    ..Default::default()
                };
                h.set_tooltip_options(opts).map_err(js)?;
            }
            "anchor" => {
                use kartis::browser::BrowserFunnelTooltipAnchor as A;
                h.set_tooltip_options(BrowserFunnelTooltipOptions {
                    anchor: [
                        A::Top,
                        A::Right,
                        A::Bottom,
                        A::Left,
                        A::Center,
                        A::TopRight,
                        A::BottomRight,
                        A::BottomLeft,
                        A::TopLeft,
                    ][n as usize % 9],
                    ..Default::default()
                })
                .map_err(js)?;
            }
            "component" => component(&h, f.metrics.clone(), value as u64).map_err(js)?,
            "replace-with-source" => {
                h.set_source(source(1)).map_err(js)?;
                component(&h, f.metrics.clone(), value as u64).map_err(js)?;
            }
            "standard" => h.clear_tooltip_component().map_err(js)?,
            "reset-tip" => h.reset_tooltip().map_err(js)?,
            "handlers" => {
                if n == 0 {
                    h.clear_mouse_handlers().map_err(js)?;
                } else {
                    install(
                        &h,
                        f.metrics.clone(),
                        if n == 2 {
                            BrowserFunnelMouseActivity::Owner
                        } else {
                            BrowserFunnelMouseActivity::NativeComponent
                        },
                        n,
                    )
                    .map_err(js)?;
                }
            }
            "action" => {
                f.metrics.action.set(n);
                return Ok(());
            }
            "fault" => {
                f.metrics.fault.set(n);
                return Ok(());
            }
            "current" => {
                let current = match n {
                    0 => None,
                    1 => Some(CategoryKey::from("1")),
                    _ => Some(CategoryKey::from(1.)),
                };
                h.set_current_part(current.as_ref()).map_err(js)?;
            }
            "hidden" => {
                let mut hidden = BTreeSet::new();
                if n != 0 {
                    hidden.insert("funnel:s1".into());
                }
                h.set_hidden(hidden).map_err(js)?;
            }
            "view" => h
                .set_view(if n == 0 {
                    ZoomTransform::default()
                } else {
                    ZoomTransform {
                        scale: 1.2,
                        translate_x: 4.,
                        translate_y: 7.,
                    }
                })
                .map_err(js)?,
            "labels" => {
                let mut o = options(f.profile);
                if n == 0 {
                    o.labels = None;
                }
                h.set_options(o).map_err(js)?;
            }
            "refresh" => h.refresh().map_err(js)?,
            "flush" => {}
            "stop" => {
                h.stop();
                return Ok(());
            }
            _ => return Err(js("unknown Funnel interaction operation")),
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_interaction_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("missing Funnel interaction fixture"))?;
        snapshot(&f.owner.handle(), &f.metrics)
            .map(|v| v.to_string())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_interaction_take(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{let f=all.get(key).ok_or_else(||js("missing Funnel interaction fixture"))?;let m=&f.metrics;Ok(json!({"events":std::mem::take(&mut *m.events.borrow_mut()),"observations":std::mem::take(&mut *m.observations.borrow_mut()),"actions":std::mem::take(&mut *m.actions.borrow_mut())}).to_string())})
}
#[wasm_bindgen]
pub fn funnel_interaction_drop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all|{let f=all.remove(key).ok_or_else(||js("missing Funnel interaction fixture"))?;f.owner.stop();f.root.remove();Ok(json!({"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get(),"tipScopes":f.metrics.tip_scopes.get(),"tipCleanups":f.metrics.tip_cleanups.get()}).to_string())})
}

#[wasm_bindgen]
pub fn funnel_interaction_batch_refresh(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("missing Funnel interaction fixture"))?;
        let controls = f.metrics.batches.borrow();
        let (_, batch) = controls
            .iter()
            .find(|(ctor, _)| location("tooltip", *ctor)["accepted"] == true)
            .ok_or_else(|| js("no accepted tooltip Scope"))?;
        Ok(batch().to_string())
    })
}
