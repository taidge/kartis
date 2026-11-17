use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, PointerEvents, Position, Style};

use super::*;

fn location(role: &str, ctor: u32) -> Value {
    let d = web_sys::window().unwrap().document().unwrap();
    match d
        .query_selector(&format!(".fm-{role}-{ctor}"))
        .ok()
        .flatten()
    {
        None => json!({"connected":false,"accepted":false,"inTip":false}),
        Some(n) => {
            json!({"connected":n.is_connected(),"accepted":n.closest(".kartis-funnel-svg-layers[data-funnel-svg-active='true']").ok().flatten().is_some(),
            "inTip":n.closest(".kartis-funnel-tooltip").ok().flatten().is_some(),"html":n.outer_html()})
        }
    }
}
pub(super) fn peers(m: &Metrics) -> Value {
    json!({"layers":m.layers.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(layer)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})},
        "motion":match r.motion.try_with_untracked(|v|v.as_ref().map(motion)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "tips":m.tips.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(tip)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})},
        "motion":match r.motion.try_with_untracked(|v|v.as_ref().map(motion)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>()})
}
fn drawing(p: Option<&Motion>) -> GraphicProps {
    let fill = p
        .and_then(|p| {
            p.data
                .target()
                .body_mark(0)
                .ok()
                .flatten()
                .and_then(|body| p.data.scene().marks.iter().find(|m| m.id == body.id))
                .and_then(|m| m.fill)
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
) -> Result<BrowserFunnelDataSvgProgram<Raw>> {
    let mut layers = vec![
        FunnelSvgLayer::new("separators", FunnelSvgLayerKind::Separators)?,
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("labels", FunnelSvgLayerKind::Labels)?,
        FunnelSvgLayer::new("annotations", FunnelSvgLayerKind::Annotations)?,
        FunnelSvgLayer::new("peer", FunnelSvgLayerKind::Custom("peer".into()))?,
    ];
    if mode == 1 {
        layers.retain(|l| l.key != "parts");
    }
    if mode == 2 {
        layers.push(FunnelSvgLayer::new(
            "parts-echo",
            FunnelSvgLayerKind::Parts,
        )?);
        layers.push(FunnelSvgLayer::new(
            "peer-echo",
            FunnelSvgLayerKind::Custom("peer".into()),
        )?);
    }
    if mode == 3 {
        layers.swap(0, 4);
    }
    BrowserFunnelDataSvgProgram::new(layers)?.custom_animated("peer",revision,move|ctx:BrowserFunnelDataSvgLayerContext<Raw>,paint:MotionBond|{
        m.constructors.set(m.constructors.get()+1);let ctor=m.constructors.get();
        if m.fault.get()==2{return Err(ChartError("authored animated layer rejection".into()));}
        let frame=ctx.frame;let cleaned=Rc::new(Cell::new(false));m.layers.borrow_mut().push(LayerRow{frame,motion:paint,ctor,cleaned:cleaned.clone()});let m=m.clone();
        Ok(glory::ui::widget(move|scope|{
            let c=cleaned.clone();let cm=m.clone();scope.on_cleanup(move||{c.set(true);cm.cleanups.set(cm.cleanups.get()+1)})?;
            let em=m.clone();scope.effect(move||{let d=frame.get();let p=paint.get();log(&em,"effect",json!({"role":"layer","ctor":ctor,"own":d.as_ref().map(layer),"motion":p.as_ref().map(motion),
                "cohort":cohort(&em,d.as_ref().map(|d|&d.data),p.as_ref()),"owner":current(&em),"peers":peers(&em),"location":location("layer",ctor)}));});
            let count=scope.cage(0u32);
            glory::ui::graphic().class(format!("fm-layer-{ctor} fm-graphic")).drawing(scope.bond(move||drawing(paint.get().as_ref())))
                .style(Style::new().position(Position::Absolute).left(Length::Px(16.)).top(Length::Px(12.)).width(Length::Px(20.)).height(Length::Px(16.)).pointer_events(PointerEvents::None)).show_in(scope)?;
            glory::ui::button().class(format!("fm-button fm-button-{ctor}")).label(scope.bond(move||format!("{}:{}",frame.get().as_ref().and_then(|f|f.data.data(0)).map_or(999,|r|r.revision),count.get())))
                .on(EventKind::Activate,move |_|count.revise(|n|*n+=1)).style(Style::new().position(Position::Absolute).left(Length::Px(235.)).top(Length::Px(310.)).width(Length::Px(68.)).height(Length::Px(24.)).pointer_events(PointerEvents::Auto)).show_in(scope)?;
            Ok(())
        }))
    })
}
pub(super) fn tooltip(h: &Handle, m: Rc<Metrics>, revision: u64) -> Result<()> {
    h.set_tooltip_options(BrowserFunnelTooltipOptions {
        enabled: true,
        ..Default::default()
    })?;
    h.set_tooltip_component_with_motion(revision,move|ctx:BrowserFunnelDataTooltipComponentContext<Raw>,paint:MotionBond|{
        m.tip_constructors.set(m.tip_constructors.get()+1);let ctor=m.tip_constructors.get();if m.fault.get()==3{return Err(ChartError("authored animated tooltip rejection".into()));}
        let frame=ctx.frame;let cleaned=Rc::new(Cell::new(false));m.tips.borrow_mut().push(TipRow{frame,motion:paint,ctor,cleaned:cleaned.clone()});let m=m.clone();
        Ok(glory::ui::widget(move|scope|{
            let c=cleaned.clone();let cm=m.clone();scope.on_cleanup(move||{c.set(true);cm.tip_cleanups.set(cm.tip_cleanups.get()+1)})?;
            let em=m.clone();scope.effect(move||{let d=frame.get();let p=paint.get();log(&em,"effect",json!({"role":"tip","ctor":ctor,"own":d.as_ref().map(tip),"motion":p.as_ref().map(motion),
                "cohort":cohort(&em,d.as_ref().map(|d|&d.data),p.as_ref()),"owner":current(&em),"peers":peers(&em),"location":location("tip",ctor)}));});
            let count=scope.cage(0u32);glory::ui::button().class(format!("fm-tip fm-tip-{ctor}"))
                .label(scope.bond(move||format!("{}:{}",frame.get().as_ref().and_then(|f|f.data.data(f.source_index)).map_or(999,|r|r.revision),count.get())))
                .on(EventKind::Activate,move |_|count.revise(|n|*n+=1)).style(Style::new().width(Length::Px(100.)).height(Length::Px(32.))).show_in(scope)?;Ok(())
        }))
    })
}
pub(super) fn hooks(h: &Handle, m: Rc<Metrics>) -> Result<()> {
    let make = || {
        let m = m.clone();
        Box::new(
            move |e: BrowserFunnelDataMouseEvent<Raw>, p: Option<Motion>| {
                let seq = web_sys::js_sys::Reflect::get(e.input.event.as_ref(), &"fmSeq".into())
                    .ok()
                    .and_then(|v| v.as_f64());
                m.historical.borrow_mut().push((e.data.clone(), p.clone()));
                log(
                    &m,
                    "native",
                    json!({"kind":format!("{:?}",e.input.kind),"data":data(&e.data),"motion":p.as_ref().map(motion),"generation":e.source_generation,"publication":e.publication,
            "partIndex":e.part_index,"sourceIndex":e.source_index,"seq":seq,"type":e.input.event.type_(),"trusted":e.input.event.is_trusted(),"eventPhase":e.input.event.event_phase(),"client":e.input.client,"scene":e.input.scene,
            "occurrence":{"key":e.input.occurrence.key,"index":e.input.occurrence.index},"cohort":cohort(&m,Some(&e.data),p.as_ref()),"owner":current(&m)}),
                );
                if e.input.kind == BrowserFunnelMouseKind::Click
                    && m.callback_step.get()
                    && let Some(h) = m.owner.borrow().as_ref()
                {
                    let before = h.motion_frame()?;
                    let result = h.step_motion(0.01);
                    log(
                        &m,
                        "callbackStep",
                        json!({"rejected":result.is_err(),"error":result.err().map(|e|e.to_string()),"unchanged":before==h.motion_frame()?}),
                    );
                }
                Ok(())
            },
        ) as Box<dyn FnMut(BrowserFunnelDataMouseEvent<Raw>, Option<Motion>) -> Result<()>>
    };
    h.set_mouse_handlers_with_motion(BrowserFunnelDataMotionMouseHandlers {
        on_mouse_enter: Some(make()),
        on_mouse_move: Some(make()),
        on_mouse_leave: Some(make()),
        on_click: Some(make()),
        ..Default::default()
    })
}
