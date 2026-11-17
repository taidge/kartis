use super::*;

pub(super) fn location(role: &str, ctor: u32) -> Value {
    let document = web_sys::window().unwrap().document().unwrap();
    match document
        .query_selector(&format!(".ws-{role}-{ctor}"))
        .ok()
        .flatten()
    {
        None => json!({"connected":false,"accepted":false,"inTip":false}),
        Some(n) => {
            let component = n.closest(".kartis-waffle-tooltip-component").ok().flatten().map(|node| {
                let computed = web_sys::window().unwrap().get_computed_style(&node).ok().flatten();
                json!({"class":node.class_name(),"style":node.get_attribute("style"),"display":computed.as_ref().and_then(|s|s.get_property_value("display").ok()),
                    "visibility":computed.as_ref().and_then(|s|s.get_property_value("visibility").ok()),"inert":node.has_attribute("inert"),"connected":node.is_connected(),
                    "childClass":node.query_selector(".ws-tip").ok().flatten().map(|n|n.class_name())})
            });
            json!({"connected":n.is_connected(),"accepted":n.closest(".kartis-waffle-svg-layers[data-waffle-svg-active='true']").ok().flatten().is_some(),
            "inTip":n.closest(".kartis-waffle-tooltip").ok().flatten().is_some(),"component":component,"html":n.outer_html()})
        }
    }
}
pub(super) fn peers(m: &Metrics) -> Value {
    json!({"layers":m.layers.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(|f|layer_json(m,f))){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "tips":m.tips.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(|f|tip_json(m,f))){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>()})
}
fn drawing(frame: Option<&Layer>) -> GraphicProps {
    let fill = frame
        .and_then(|f| {
            f.data
                .layout()
                .groups
                .iter()
                .find(|g| !g.hidden && !g.occupied.is_empty())
                .map(|g| g.color)
        })
        .unwrap_or(Color(0, 0, 0, 0));
    GraphicProps {
        width: 20.,
        height: 16.,
        fill: Some(fill),
        commands: vec![
            PathCommand::Move(glory::render::GraphicPoint::new(0., 0.)),
            PathCommand::Line(glory::render::GraphicPoint::new(20., 0.)),
            PathCommand::Line(glory::render::GraphicPoint::new(20., 16.)),
            PathCommand::Line(glory::render::GraphicPoint::new(0., 16.)),
            PathCommand::Close,
        ],
        ..Default::default()
    }
}
pub(super) fn program(
    mode: u8,
    revision: u64,
    m: Rc<Metrics>,
) -> Result<BrowserWaffleDataSvgProgram<Raw>> {
    let mut layers = vec![
        WaffleSvgLayer::new("cells", WaffleSvgLayerKind::Cells)?,
        WaffleSvgLayer::new("areas", WaffleSvgLayerKind::Areas)?,
        WaffleSvgLayer::new("legends", WaffleSvgLayerKind::Legends)?,
        WaffleSvgLayer::new("peer", WaffleSvgLayerKind::Custom("peer".into()))?,
    ];
    match mode {
        1 => layers.retain(|l| l.key != "areas"),
        2 => layers.swap(0, 1),
        3 => {
            layers.push(WaffleSvgLayer::new(
                "cells-echo",
                WaffleSvgLayerKind::Cells,
            )?);
            layers.push(WaffleSvgLayer::new(
                "areas-echo",
                WaffleSvgLayerKind::Areas,
            )?);
            layers.push(WaffleSvgLayer::new(
                "peer-echo",
                WaffleSvgLayerKind::Custom("peer".into()),
            )?);
        }
        4 => {
            let peer = layers.remove(3);
            layers.insert(1, peer);
        }
        5 => layers.retain(|l| l.key != "cells"),
        _ => {}
    }
    let program = BrowserWaffleDataSvgProgram::new(layers)?.interactive(mode != 6);
    let program = if mode == 7 || mode == 8 {
        let bounds = if mode == 8 {
            let owner = m.owner.borrow();
            let frame = owner
                .as_ref()
                .ok_or_else(|| ChartError("Waffle owner missing".into()))?
                .snapshot()?
                .data
                .ok_or_else(|| ChartError("Waffle source missing".into()))?;
            let slot = frame.layout().cells[1].bounds;
            LayoutRect {
                x: slot.x,
                y: slot.y,
                width: 530.,
                height: 28.,
            }
        } else {
            LayoutRect {
                x: 40.,
                y: 385.,
                width: 530.,
                height: 28.,
            }
        };
        let legend = WaffleSvgLegendSpec::new(
            "labels",
            LegendOptions::default(),
            LegendLayoutOptions {
                bounds,
                ..Default::default()
            },
            TextSize {
                width: 82.,
                height: 24.,
            },
        )?;
        program.with_legends(vec![legend])?
    } else {
        program
    };
    let program = if mode == 90 {
        program.with_budget(WaffleSvgBudget {
            work: 0,
            ..Default::default()
        })?
    } else {
        program
    };
    program.custom("peer",revision,move|ctx:BrowserWaffleDataSvgLayerContext<Raw>|{
        m.constructors.set(m.constructors.get()+1);let ctor=m.constructors.get();
        if m.fault.get()==2{return Err(ChartError("authored Waffle layer constructor rejection".into()));}
        let initial=ctx.frame.get();log(&m,"constructor",json!({"kind":"layer","ctor":ctor,"initialNone":initial.is_none(),"key":ctx.key,"revision":ctx.revision}));
        let frame=ctx.frame;let cleaned=Rc::new(Cell::new(false));m.layers.borrow_mut().push(LayerRow{frame,ctor,cleaned:cleaned.clone()});let m=m.clone();
        Ok(glory::ui::widget(move|scope|{
            let c=cleaned.clone();let cm=m.clone();scope.on_cleanup(move||{c.set(true);cm.cleanups.set(cm.cleanups.get()+1)})?;
            let batch_scope=scope.downgrade();let batch_metrics=Rc::downgrade(&m);
            m.batches.borrow_mut().push((ctor,Box::new(move||{
                let Some(scope)=batch_scope.upgrade() else{return json!({"invoked":false,"scopeGone":true})};
                let Some(metrics)=batch_metrics.upgrade() else{return json!({"invoked":false,"metricsGone":true})};
                let owner=metrics.owner.borrow().as_ref().unwrap().clone();let invoked=Cell::new(false);
                let result=scope.batch(||{invoked.set(true);owner.refresh()?;owner.flush()?;owner_json(&metrics,&owner)});
                match result{Ok(Ok(value))=>json!({"invoked":invoked.get(),"value":value,"error":null}),Ok(Err(e))=>json!({"invoked":invoked.get(),"error":e.to_string()}),Err(e)=>json!({"invoked":invoked.get(),"error":e.to_string()})}
            })));
            let em=m.clone();scope.effect(move||{let own=frame.get();log(&em,"effect",json!({"kind":"layer","ctor":ctor,"own":own.as_ref().map(|f|layer_json(&em,f)),
                "cohort":cohort(&em,own.as_ref().map(|f|&f.data)),"owner":current(&em),"peers":peers(&em),"location":location("layer",ctor)}));});
            glory::ui::graphic().class(format!("ws-layer-{ctor} ws-graphic")).drawing(scope.bond(move||drawing(frame.get().as_ref())))
                .style(Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(6.)).width(Length::Px(20.)).height(Length::Px(16.)).pointer_events(PointerEvents::None)).show_in(scope)?;
            let count=scope.cage(0u32);let activate=m.clone();let style_m=m.clone();
            glory::ui::button().class(format!("ws-button ws-button-{ctor}"))
                .label(scope.bond(move||format!("{}:{}",frame.get().as_ref().and_then(|f|f.data.data(0)).map_or(999,|d|d.epoch),count.get())))
                .on(EventKind::Activate,move|_|{count.revise(|n|*n+=1);if let Some(h)=activate.owner.borrow().as_ref(){let r=match activate.action.get(){1=>h.refresh(),2=>h.set_hidden(BTreeSet::from(["alpha".into(),"hidden".into()])),_=>Ok(())};log(&activate,"action",json!({"Ok":r.is_ok(),"error":r.err().map(|e|e.to_string())}));}})
                .style(scope.bond(move||{
                    let point=frame.get().as_ref().and_then(|f|f.data.layout().cells.first().map(|c|[c.bounds.x+c.bounds.width/2.,c.bounds.y+c.bounds.height/2.])).unwrap_or([300.,210.]);
                    Style::new().position(Position::Absolute).left(Length::Px(point[0]-30.)).top(Length::Px(point[1]-12.)).width(Length::Px(if style_m.fault.get()==5{f64::NAN}else{60.})).height(Length::Px(24.)).pointer_events(PointerEvents::Auto)
                })).show_in(scope)?;
            Ok(())
        }))
    })
}
pub(super) fn tooltip(h: &Handle, m: Rc<Metrics>, revision: u64) -> Result<()> {
    h.set_tooltip_options(BrowserWaffleTooltipOptions {
        enabled: true,
        ..Default::default()
    })?;
    h.set_tooltip_component(revision,move|ctx:BrowserWaffleDataTooltipComponentContext<Raw>|{
        m.tip_constructors.set(m.tip_constructors.get()+1);let ctor=m.tip_constructors.get();
        if m.fault.get()==3{return Err(ChartError("authored Waffle tooltip constructor rejection".into()));}
        log(&m,"constructor",json!({"kind":"tip","ctor":ctor,"revision":ctx.revision,"initialNone":ctx.frame.get().is_none()}));
        let frame=ctx.frame;let cleaned=Rc::new(Cell::new(false));m.tips.borrow_mut().push(TipRow{frame,ctor,cleaned:cleaned.clone()});let m=m.clone();
        Ok(glory::ui::widget(move|scope|{
            let c=cleaned.clone();let cm=m.clone();scope.on_cleanup(move||{c.set(true);cm.tip_cleanups.set(cm.tip_cleanups.get()+1)})?;
            let em=m.clone();scope.effect(move||{let own=frame.get();log(&em,"effect",json!({"kind":"tip","ctor":ctor,"own":own.as_ref().map(|f|tip_json(&em,f)),
                "cohort":cohort(&em,own.as_ref().map(|f|&f.data)),"owner":current(&em),"peers":peers(&em),"location":location("tip",ctor)}));});
            let count=scope.cage(0u32);
            glory::ui::button().class(format!("ws-tip ws-tip-{ctor}")).label(scope.bond(move||format!("{}:{}",frame.get().as_ref().and_then(|f|f.data.data(f.source_index)).map_or(999,|d|d.epoch),count.get())))
                .on(EventKind::Activate,move|_|count.revise(|n|*n+=1)).style(Style::new().width(Length::Px(100.)).height(Length::Px(32.))).show_in(scope)?;
            Ok(())
        }))
    })
}
pub(super) fn hooks(h: &Handle, m: Rc<Metrics>) -> Result<()> {
    let make = || {
        let m = m.clone();
        Box::new(move |e: BrowserWaffleDataMouseEvent<Raw>| {
            let seq = web_sys::js_sys::Reflect::get(e.input.event.as_ref(), &"wsSeq".into())
                .ok()
                .and_then(|v| v.as_f64());
            m.historical.borrow_mut().push(e.data.clone());
            log(
                &m,
                "native",
                json!({"kind":format!("{:?}",e.input.kind),"frame":frame_id(&m,&e.data),"generation":e.source_generation,"publication":e.publication,
            "sourceIndex":e.source_index,"polygonIndex":e.polygon_index,"client":e.input.client,"viewport":e.input.viewport,"scene":e.input.scene,
            "occurrence":{"key":e.input.occurrence.key,"index":e.input.occurrence.index,"publication":e.input.occurrence.publication},
            "seq":seq,"trusted":e.input.event.is_trusted(),"nativeType":e.input.event.type_(),"phase":e.input.event.event_phase(),"owner":current(&m),"peers":peers(&m)}),
            );
            Ok(())
        }) as Box<dyn FnMut(BrowserWaffleDataMouseEvent<Raw>) -> Result<()>>
    };
    h.set_mouse_handlers(BrowserWaffleDataMouseHandlers {
        on_mouse_enter: Some(make()),
        on_mouse_move: Some(make()),
        on_mouse_leave: Some(make()),
        on_click: Some(make()),
    })
}
