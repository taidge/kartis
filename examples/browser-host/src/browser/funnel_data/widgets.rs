use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, PointerEvents, Position, Style};

use super::*;

fn location(role: &str, ctor: u32) -> Value {
    let d = web_sys::window().unwrap().document().unwrap();
    let selector = format!(".fdr-{role}-{ctor}");
    match d.query_selector(&selector).ok().flatten() {
        None => json!({"connected":false,"accepted":false,"inTip":false}),
        Some(n) => json!({"connected":n.is_connected(),
            "accepted":n.closest(".kartis-funnel-svg-layers[data-funnel-svg-active='true']").ok().flatten().is_some(),
            "inTip":n.closest(".kartis-funnel-tooltip").ok().flatten().is_some(),"html":n.outer_html()}),
    }
}
pub(super) fn peers(m: &Metrics) -> Value {
    json!({"layers":m.rows.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(layer)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "tips":m.tips.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),
        "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(tip)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>()})
}
fn observe(m: &Metrics, role: &str, ctor: u32, own: Value, cohort: Value) {
    log(
        m,
        "effect",
        json!({"role":role,"ctor":ctor,"own":own,"cohort":cohort,"owner":current(m),"peers":peers(m),"location":location(role,ctor)}),
    );
}
fn color(frame: &Frame) -> Color {
    if frame.data(0).is_some_and(|d| !d.revision.is_multiple_of(2)) {
        Color(220, 110, 25, 255)
    } else {
        Color(15, 160, 95, 255)
    }
}
fn drawing(fill: Color) -> GraphicProps {
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
        FunnelSvgLayer::new("parts", FunnelSvgLayerKind::Parts)?,
        FunnelSvgLayer::new("annotations", FunnelSvgLayerKind::Annotations)?,
        FunnelSvgLayer::new("peer", FunnelSvgLayerKind::Custom("peer".into()))?,
    ];
    if mode == 1 {
        layers.retain(|l| l.key != "parts");
    }
    if mode == 2 {
        layers.push(FunnelSvgLayer::new(
            "peer-echo",
            FunnelSvgLayerKind::Custom("peer".into()),
        )?);
    }
    if mode == 3 {
        layers.swap(0, 2);
    }
    BrowserFunnelDataSvgProgram::new(layers)?.custom(
        "peer",
        revision,
        move |ctx: BrowserFunnelDataSvgLayerContext<Raw>| {
            m.constructors.set(m.constructors.get() + 1);
            let ctor = m.constructors.get();
            let frame = ctx.frame;
            if m.fault.get() == 7 {
                return Err(ChartError("raw layer constructor rejected".into()));
            }
            let cleaned = Rc::new(Cell::new(false));
            m.rows.borrow_mut().push(LayerRow {
                ctor,
                frame,
                cleaned: cleaned.clone(),
            });
            let m = m.clone();
            Ok(glory::ui::widget(move |scope| {
                let c = cleaned.clone();
                let cm = m.clone();
                scope.on_cleanup(move || {
                    c.set(true);
                    cm.cleanups.set(cm.cleanups.get() + 1)
                })?;
                let em = m.clone();
                scope.effect(move || {
                    let value = frame.get();
                    observe(
                        &em,
                        "layer",
                        ctor,
                        value.as_ref().map(layer).unwrap_or(Value::Null),
                        value
                            .as_ref()
                            .map(|v| cohort_matches(&em, &v.data))
                            .unwrap_or(Value::Null),
                    )
                });
                let count = scope.cage(0u32);
                glory::ui::graphic()
                    .class(format!("fdr-layer-{ctor} fdr-graphic"))
                    .drawing(scope.bond(move || {
                        drawing(
                            frame
                                .get()
                                .as_ref()
                                .map(|f| color(&f.data))
                                .unwrap_or(Color(0, 0, 0, 0)),
                        )
                    }))
                    .style(
                        Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(16.))
                            .top(Length::Px(12.))
                            .width(Length::Px(20.))
                            .height(Length::Px(16.))
                            .pointer_events(PointerEvents::None),
                    )
                    .show_in(scope)?;
                glory::ui::button()
                    .class(format!("fdr-button fdr-button-{ctor}"))
                    .label(scope.bond(move || {
                        format!(
                            "{}:{}",
                            frame
                                .get()
                                .as_ref()
                                .and_then(|f| f.data.data(0))
                                .map_or(999, |d| d.revision),
                            count.get()
                        )
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                    .style(
                        Style::new()
                            .position(Position::Absolute)
                            .left(Length::Px(235.))
                            .top(Length::Px(310.))
                            .width(Length::Px(68.))
                            .height(Length::Px(24.))
                            .pointer_events(PointerEvents::Auto),
                    )
                    .show_in(scope)?;
                Ok(())
            }))
        },
    )
}
pub(super) fn tooltip(h: &Handle, m: Rc<Metrics>, revision: u64) -> Result<()> {
    h.set_tooltip_options(BrowserFunnelTooltipOptions {
        enabled: true,
        ..Default::default()
    })?;
    h.set_tooltip_component(
        revision,
        move |ctx: BrowserFunnelDataTooltipComponentContext<Raw>| {
            if m.fault.get() == 8 {
                return Err(ChartError("raw tooltip constructor rejected".into()));
            }
            m.tip_constructors.set(m.tip_constructors.get() + 1);
            let ctor = m.tip_constructors.get();
            let frame = ctx.frame;
            let cleaned = Rc::new(Cell::new(false));
            m.tips.borrow_mut().push(TipRow {
                ctor,
                frame,
                cleaned: cleaned.clone(),
            });
            let m = m.clone();
            Ok(glory::ui::widget(move |scope| {
                let c = cleaned.clone();
                let cm = m.clone();
                scope.on_cleanup(move || {
                    c.set(true);
                    cm.tip_cleanups.set(cm.tip_cleanups.get() + 1)
                })?;
                let em = m.clone();
                scope.effect(move || {
                    let value = frame.get();
                    observe(
                        &em,
                        "tip",
                        ctor,
                        value.as_ref().map(tip).unwrap_or(Value::Null),
                        value
                            .as_ref()
                            .map(|v| cohort_matches(&em, &v.data))
                            .unwrap_or(Value::Null),
                    )
                });
                let count = scope.cage(0u32);
                glory::ui::button()
                    .class(format!("fdr-tip-{ctor} fdr-tip"))
                    .label(scope.bond(move || {
                        format!(
                            "{}:{}",
                            frame
                                .get()
                                .as_ref()
                                .and_then(|f| f.data.data(f.source_index))
                                .map_or(999, |d| d.revision),
                            count.get()
                        )
                    }))
                    .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                    .style(Style::new().width(Length::Px(100.)).height(Length::Px(32.)))
                    .show_in(scope)?;
                Ok(())
            }))
        },
    )
}
pub(super) fn hooks(h: &Handle, m: Rc<Metrics>) -> Result<()> {
    let make = || {
        let m = m.clone();
        Box::new(move |e: BrowserFunnelDataMouseEvent<Raw>| {
            let seq = web_sys::js_sys::Reflect::get(e.input.event.as_ref(), &"fdrSeq".into())
                .ok()
                .and_then(|v| v.as_f64());
            let retained = e.data.clone();
            m.historical.borrow_mut().push(retained);
            log(
                &m,
                "native",
                json!({"seq":seq,"kind":format!("{:?}",e.input.kind),"type":e.input.event.type_(),
            "eventPhase":e.input.event.event_phase(),"trusted":e.input.event.is_trusted(),"client":e.input.client,
            "data":data(&e.data),"generation":e.source_generation,"publication":e.publication,"partIndex":e.part_index,
            "sourceIndex":e.source_index,"occurrence":{"key":e.input.occurrence.key,"index":e.input.occurrence.index,"publication":e.input.occurrence.publication},"owner":current(&m),"cohort":cohort_matches(&m,&e.data)}),
            );
            if m.action.replace(0) == 1
                && let Some(h) = m.owner.borrow().as_ref()
            {
                h.set_source(source(77, 0))?;
            }
            Ok(())
        }) as FunnelDataMouseHandler<Raw>
    };
    h.set_mouse_handlers(BrowserFunnelDataMouseHandlers {
        on_mouse_enter: Some(make()),
        on_mouse_move: Some(make()),
        on_mouse_leave: Some(make()),
        on_click: Some(make()),
        ..Default::default()
    })
}
