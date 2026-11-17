//! Bound annotations, real literal SVG occurrences and retained peer Widgets.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, PointerEvents, Position, Style};
use kartis::browser::{
    BrowserFunnel, BrowserFunnelHandle, BrowserFunnelMouseEvent, BrowserFunnelMouseHandlers,
    BrowserFunnelSvgLayerContext, BrowserFunnelSvgLayerFrame, BrowserFunnelSvgProgram,
    FunnelTooltipFrame,
};

use super::*;

type LayerBond = glory::Bond<Option<BrowserFunnelSvgLayerFrame>>;
type TipBond = glory::Bond<Option<FunnelTooltipFrame>>;
struct LayerRow {
    ctor: u32,
    frame: LayerBond,
    cleaned: Rc<Cell<bool>>,
}
struct TipRow {
    ctor: u32,
    frame: TipBond,
    cleaned: Rc<Cell<bool>>,
}
#[derive(Default)]
struct Metrics {
    owner: RefCell<Option<BrowserFunnelHandle>>,
    builds: Cell<u32>,
    matcher: Cell<u32>,
    notes: Cell<u32>,
    constructors: Cell<u32>,
    cleanups: Cell<u32>,
    tip_constructors: Cell<u32>,
    tip_cleanups: Cell<u32>,
    recipe: Cell<u8>,
    fault: Cell<u8>,
    rows: RefCell<Vec<LayerRow>>,
    tips: RefCell<Vec<TipRow>>,
    ledger: RefCell<Vec<Value>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserFunnel,
    metrics: Rc<Metrics>,
    mode: u8,
    revision: u64,
}
thread_local! { static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const {RefCell::new(BTreeMap::new())}; }
fn log(m: &Metrics, kind: &str, value: Value) {
    let mut l = m.ledger.borrow_mut();
    let ordinal = l.len();
    l.push(json!({"ordinal":ordinal,"kind":kind,"value":value}));
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 420.,
        margin: Margin {
            left: 40.,
            right: 30.,
            top: 28.,
            bottom: 32.,
        },
        title: "Bound Funnel annotations / 注解".into(),
        ..Default::default()
    }
}
fn source(phase: u8) -> Vec<FunnelDatum> {
    let mut r = vec![
        FunnelDatum::new("1", if phase == 1 { 10. } else { 8. }),
        FunnelDatum::new(1., 4.),
        FunnelDatum::new("zero", if phase == 1 { 0. } else { -0. }),
        FunnelDatum::new("transparent", 2.),
        FunnelDatum::new("tail", 0.),
    ];
    for (i, d) in r.iter_mut().enumerate() {
        d.label = format!("bound-{i}{}", if phase == 1 { "-new" } else { "" });
        d.size = Some([180., 120., 0., 90., 60.][i]);
        d.color = Some(
            [
                Color(190, 40, 50, 255),
                Color(30, 90, 210, 255),
                Color(20, 160, 70, 255),
                Color(80, 90, 120, 0),
                Color(180, 100, 30, 255),
            ][i],
        );
    }
    if phase == 2 {
        r.swap(0, 1);
    }
    if phase == 3 {
        r.remove(1);
    }
    r
}
fn options(profile: u8) -> FunnelLayoutOptions {
    FunnelLayoutOptions {
        direction: if profile.is_multiple_of(2) {
            FunnelDirection::Vertical
        } else {
            FunnelDirection::Horizontal
        },
        interpolation: if profile < 2 {
            FunnelInterpolation::Linear
        } else {
            FunnelInterpolation::Smooth
        },
        size: FunnelSize::DatumPixels,
        shape_blending: 0.5,
        fill_opacity: 1.,
        border_width: 0.,
        before: None,
        after: None,
        labels: None,
        legend: false,
        current_size_extension: 12.,
        ..Default::default()
    }
}
fn specs(recipe: u8) -> Vec<FunnelAnnotationSpec> {
    use FunnelAnnotationKind as K;
    use FunnelAnnotationMatch as M;
    use FunnelAnnotationNote as N;
    use FunnelAnnotationPosition as P;
    let make = |matcher, kind, note, x, y| {
        FunnelAnnotationSpec::new(matcher, kind, note, P::Relative(x), P::Relative(y))
    };
    if recipe == 0 {
        return vec![];
    }
    if recipe == 2 {
        return [
            (0., -100.),
            (100., -100.),
            (100., 0.),
            (100., 100.),
            (0., 100.),
            (-100., 100.),
            (-100., 0.),
            (-100., -100.),
        ]
        .into_iter()
        .map(|(x, y)| {
            let mut s = make(M::Id("1".into()), K::Rect, N::Text("eight".into()), x, y);
            s.offset = 4.;
            s.note_width = 24.;
            s
        })
        .collect();
    }
    let mut dot = make(
        M::Id("1".into()),
        K::Dot,
        N::Text("dot <&>".into()),
        60.,
        -10.,
    );
    dot.size = Some(12.);
    dot.offset = 91.;
    dot.note_width = 48.;
    let mut circle = make(
        M::Id(CategoryKey::from(1.)),
        K::Circle,
        N::Text("circle".into()),
        -70.,
        -12.,
    );
    circle.size = Some(46.);
    circle.offset = 3.;
    circle.note_width = 42.;
    let mut rect = make(
        M::Id("zero".into()),
        K::Rect,
        N::Text("rect".into()),
        60.,
        14.,
    );
    rect.offset = 6.;
    rect.width = Some(777.);
    rect.height = Some(555.);
    rect.x = Some(999.);
    rect.y = Some(888.);
    rect.border_radius = 3.;
    rect.note_width = 36.;
    let mut custom = make(
        M::Predicate("note-target".into()),
        K::Dot,
        N::Custom("marker".into()),
        -80.,
        -16.,
    );
    custom.size = Some(0.);
    custom.note_width = 32.;
    let mut absolute = make(
        M::Id("tail".into()),
        K::Circle,
        N::Text(String::new()),
        0.,
        0.,
    );
    absolute.note_x = P::Absolute(500.);
    absolute.note_y = P::Absolute(370.);
    absolute.size = Some(0.);
    absolute.offset = 2.;
    absolute.note_width = 22.;
    let no_match = make(
        M::Predicate("none".into()),
        K::Dot,
        N::Text("absent".into()),
        30.,
        0.,
    );
    let multi = make(
        M::Predicate("multi".into()),
        K::Dot,
        N::Text("multi".into()),
        25.,
        18.,
    );
    let mut result = vec![dot, circle, rect, custom, absolute, no_match, multi];
    // Dot offset is ignored; the multi result uses fixed small independent ink.
    result[6].size = Some(6.);
    result[6].note_width = 18.;
    result
}
fn style() -> FunnelAnnotationStyle {
    let mut s = FunnelAnnotationStyle::default();
    s.outline.stroke = Some(Color(20, 60, 180, 255));
    s.outline.stroke_width = 2.;
    s.outline.outline_width = 1.;
    s.symbol.fill = Some(Color(240, 30, 80, 255));
    s.link.stroke = Some(Color(0, 120, 100, 255));
    s.link.stroke_width = 2.;
    s.link.outline_width = 0.;
    s.text.outline_width = 1.;
    s
}
fn key(k: &CategoryKey) -> Value {
    super::funnel_svg_layers::key_json(k)
}
fn geometry(g: &Geometry) -> Value {
    match g {
        Geometry::Circle { x, y, radius } => json!({"kind":"circle","x":x,"y":y,"radius":radius}),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => json!({"kind":"rect","x":x,"y":y,"width":width,"height":height}),
        Geometry::Path(p) => {
            json!({"kind":"path","commands":p.iter().map(|c|match c {PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!(["Z"])}).collect::<Vec<_>>()})
        }
    }
}
fn frame(f: &FunnelSvgFrame) -> Value {
    let mut value = super::funnel_svg_layers::data_json(f);
    value["annotations"]=f.annotations().map(|a|json!({"source":super::funnel_svg_layers::data_json(a.source()),"bindings":a.bindings().iter().map(|b|json!({"specIndex":b.spec_index,"partIndex":b.part_index,"sourceIndex":b.source_index,"id":key(&b.id),"kind":format!("{:?}",b.kind),"note":match &b.note{FunnelAnnotationNote::Text(t)=>json!({"text":t}),FunnelAnnotationNote::Custom(k)=>json!({"custom":k})},"center":b.center,"size":b.size,"width":b.width,"height":b.height,"borderRadius":b.border_radius,"noteWidth":b.note_width,"textOffset":b.note_text_offset,"points":b.computed.points,"text":b.computed.text,"angle":b.computed.angle})).collect::<Vec<_>>(),"marks":a.scene().marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"geometry":geometry(&m.geometry),"fill":m.fill.map(|c|[c.0,c.1,c.2,c.3]),"opacity":m.opacity})).collect::<Vec<_>>(),"labels":a.scene().labels.iter().map(|l|json!({"x":l.x,"y":l.y,"text":l.text,"size":l.size,"anchor":format!("{:?}",l.anchor)})).collect::<Vec<_>>()})).unwrap_or(Value::Null);
    value
}
fn layer(f: &BrowserFunnelSvgLayerFrame) -> Value {
    json!({"data":frame(&f.data),"generation":f.source_generation,"publication":f.publication,"key":f.key,"index":f.index,"view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden})
}
fn tip(f: &FunnelTooltipFrame) -> Value {
    json!({"data":frame(&f.data),"generation":f.source_generation,"publication":f.publication,"sourceIndex":f.source_index,"partIndex":f.part_index,"client":f.pointer_client,"occurrence":{"key":f.occurrence.key,"index":f.occurrence.index,"publication":f.occurrence.publication}})
}
fn accepted(h: &BrowserFunnelHandle) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"active":s.active,"data":s.data.as_ref().map(frame),"generation":s.source_generation,"publication":s.publication,"current":s.current.as_ref().map(key),"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hidden":s.hidden,"pending":s.pending,"error":s.error.map(|e|e.to_string()),"uiErrors":s.ui_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"eventErrors":s.event_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"tooltip":h.tooltip()?.as_ref().map(tip)}),
    )
}
fn owner(m: &Metrics) -> Value {
    match m.owner.borrow().as_ref() {
        None => Value::Null,
        Some(h) => match accepted(h) {
            Ok(s) => json!({"Ok":s}),
            Err(e) => json!({"Err":e.to_string()}),
        },
    }
}
fn location(role: &str, ctor: u32) -> Value {
    let d = web_sys::window().unwrap().document().unwrap();
    let n = d
        .query_selector(&format!(".faa-{role}-ctor-{ctor}"))
        .ok()
        .flatten();
    let outer = n
        .as_ref()
        .and_then(|n| n.closest(".kartis-funnel-svg-layers").ok().flatten());
    json!({"connected":n.as_ref().is_some_and(|n|n.is_connected()),"accepted":outer.as_ref().is_some_and(|n|n.get_attribute("data-funnel-svg-active").as_deref()==Some("true")),"inTip":n.as_ref().is_some_and(|n|n.closest(".kartis-funnel-tooltip").ok().flatten().is_some())})
}
fn peers(m: &Metrics) -> Value {
    json!({
        "layers": m.rows.borrow().iter().map(|r| json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),"frame":match r.frame.try_with_untracked(|v|v.as_ref().map(layer)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "tips": m.tips.borrow().iter().map(|r| json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),"frame":match r.frame.try_with_untracked(|v|v.as_ref().map(tip)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>()
    })
}
fn observe(m: &Metrics, role: &str, ctor: u32, own: Value) {
    log(
        m,
        "effect",
        json!({"role":role,"ctor":ctor,"own":own,"location":location(role,ctor),"owner":owner(m),"peers":peers(m)}),
    );
}

fn program(mode: u8, revision: u64, m: Rc<Metrics>) -> Result<BrowserFunnelSvgProgram> {
    let make = |key, kind| FunnelSvgLayer::new(key, kind);
    let mut ls = vec![make("parts", FunnelSvgLayerKind::Parts)?];
    if mode != 1 {
        ls.push(make("annotations", FunnelSvgLayerKind::Annotations)?);
    }
    if mode == 2 {
        ls.push(make("annotations-echo", FunnelSvgLayerKind::Annotations)?);
    }
    if mode == 3 {
        ls.reverse();
    }
    ls.push(make("peer", FunnelSvgLayerKind::Custom("peer".into()))?);
    if mode == 4 {
        ls.retain(|l| l.key != "parts");
    }
    BrowserFunnelSvgProgram::new(ls)?.custom(
        "peer",
        revision,
        move |ctx: BrowserFunnelSvgLayerContext| {
            m.constructors.set(m.constructors.get() + 1);
            let ctor = m.constructors.get();
            let f = ctx.frame;
            let cleaned = Rc::new(Cell::new(false));
            m.rows.borrow_mut().push(LayerRow {
                ctor,
                frame: f,
                cleaned: cleaned.clone(),
            });
            let m = m.clone();
            Ok(glory::ui::widget(move |scope| {
                let c = cleaned.clone();
                let clean = m.clone();
                scope.on_cleanup(move || {
                    c.set(true);
                    clean.cleanups.set(clean.cleanups.get() + 1)
                })?;
                let em = m.clone();
                scope.effect(move || {
                    observe(
                        &em,
                        "layer",
                        ctor,
                        f.get().as_ref().map(layer).unwrap_or(Value::Null),
                    )
                });
                let count = scope.cage(0u32);
                glory::ui::graphic()
                    .class(format!("faa-graphic faa-layer-ctor-{ctor}"))
                    .drawing(GraphicProps {
                        width: 20.,
                        height: 16.,
                        commands: vec![
                            PathCommand::Move(glory::render::GraphicPoint::new(0., 0.)),
                            PathCommand::Line(glory::render::GraphicPoint::new(20., 0.)),
                            PathCommand::Line(glory::render::GraphicPoint::new(20., 16.)),
                            PathCommand::Line(glory::render::GraphicPoint::new(0., 16.)),
                            PathCommand::Close,
                        ],
                        fill: Some(Color(15, 160, 95, 255)),
                        ..Default::default()
                    })
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
                    .class("faa-peer-button")
                    .label(scope.bond(move || count.get().to_string()))
                    .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
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
fn install_tip(h: &BrowserFunnelHandle, m: Rc<Metrics>) -> Result<()> {
    h.set_tooltip_component(0, move |ctx| {
        m.tip_constructors.set(m.tip_constructors.get() + 1);
        let ctor = m.tip_constructors.get();
        let f = ctx.frame;
        let cleaned = Rc::new(Cell::new(false));
        m.tips.borrow_mut().push(TipRow {
            ctor,
            frame: f,
            cleaned: cleaned.clone(),
        });
        let m = m.clone();
        Ok(glory::ui::widget(move |scope| {
            let clean = m.clone();
            let c = cleaned.clone();
            scope.on_cleanup(move || {
                c.set(true);
                clean.tip_cleanups.set(clean.tip_cleanups.get() + 1)
            })?;
            let em = m.clone();
            scope.effect(move || {
                observe(
                    &em,
                    "tip",
                    ctor,
                    f.get().as_ref().map(tip).unwrap_or(Value::Null),
                )
            });
            glory::ui::button()
                .class(format!("faa-tip faa-tip-ctor-{ctor}"))
                .label("bound tooltip")
                .style(Style::new().width(Length::Px(100.)).height(Length::Px(32.)))
                .show_in(scope)?;
            Ok(())
        }))
    })
}
fn install_hooks(h: &BrowserFunnelHandle, m: Rc<Metrics>) -> Result<()> {
    let handler = move || {
        let m = m.clone();
        Box::new(move |e: BrowserFunnelMouseEvent| {
            let id = web_sys::js_sys::Reflect::get(e.input.event.as_ref(), &"faaSeq".into())
                .ok()
                .and_then(|v| v.as_f64());
            log(
                &m,
                "native",
                json!({"kind":format!("{:?}",e.input.kind),"seq":id,"eventPhase":e.input.event.event_phase(),"trusted":e.input.event.is_trusted(),"type":e.input.event.type_(),"client":e.input.client,"data":frame(&e.data),"generation":e.source_generation,"publication":e.publication,"sourceIndex":e.source_index,"partIndex":e.part_index,"occurrence":{"key":e.input.occurrence.key,"index":e.input.occurrence.index,"publication":e.input.occurrence.publication},"owner":owner(&m)}),
            );
            Ok(())
        }) as kartis::browser::FunnelMouseHandler
    };
    h.set_mouse_handlers(BrowserFunnelMouseHandlers {
        on_mouse_enter: Some(handler()),
        on_mouse_move: Some(handler()),
        on_mouse_leave: Some(handler()),
        on_click: Some(handler()),
        ..Default::default()
    })
}
fn snapshot(f: &Fixture) -> Result<Value> {
    let m = &f.metrics;
    Ok(
        json!({"owner":accepted(&f.owner.handle())?,"builds":m.builds.get(),"matcher":m.matcher.get(),"notes":m.notes.get(),"constructors":m.constructors.get(),"cleanups":m.cleanups.get(),"tipConstructors":m.tip_constructors.get(),"tipCleanups":m.tip_cleanups.get(),"layers":m.rows.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),"frame":match r.frame.try_with_untracked(|v|v.as_ref().map(layer)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),"tips":m.tips.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),"frame":match r.frame.try_with_untracked(|v|v.as_ref().map(tip)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),"count":m.ledger.borrow().len()}),
    )
}
fn build(
    m: &Metrics,
    ctx: &kartis::browser::BrowserFunnelBuildContext<'_>,
) -> Result<FunnelSvgFrame> {
    m.builds.set(m.builds.get() + 1);
    let base = FunnelSvgFrame::new(ctx.config, ctx.source, ctx.options)?;
    let mut matcher = |c: FunnelAnnotationMatchContext<'_>| {
        m.matcher.set(m.matcher.get() + 1);
        log(
            m,
            "match",
            json!({"key":c.key,"specIndex":c.spec_index,"sourceIndex":c.source_index,"partIndex":c.part_index,"id":key(&c.part.datum.id),"rawBits":c.part.datum.value.to_bits().to_string(),"source":super::funnel_svg_layers::data_json(c.frame)}),
        );
        if m.fault.get() == 1 {
            return Err(ChartError("annotation predicate rejected".into()));
        }
        Ok(match c.key {
            "note-target" => c.part.datum.id == CategoryKey::from("transparent"),
            "multi" => c.source_index < 2,
            _ => false,
        })
    };
    let mut note = |c: FunnelAnnotationNoteContext<'_>| {
        m.notes.set(m.notes.get() + 1);
        log(
            m,
            "note",
            json!({"key":c.key,"sourceIndex":c.binding.source_index,"specIndex":c.binding.spec_index,"rawBits":c.datum.value.to_bits().to_string(),"x":c.x,"y":c.y}),
        );
        if m.fault.get() == 2 {
            return Err(ChartError("annotation note rejected".into()));
        }
        let mut scene = c.scene()?;
        scene.marks.push(Mark {
            id: "author-note-marker".into(),
            series: String::new(),
            label: String::new(),
            value: 0.,
            geometry: Geometry::Rect {
                x: c.x,
                y: c.y - 10.,
                width: 14.,
                height: 12.,
            },
            fill: Some(Color(20, 180, 210, 255)),
            fill_paint: None,
            stroke: None,
            stroke_dash: None,
            clip_path: None,
            opacity: 1.,
            clip: None,
        });
        Ok(scene)
    };
    let mut factories = FunnelAnnotationFactories {
        matcher: Some(&mut matcher),
        note: Some(&mut note),
    };
    let mut budget = FunnelAnnotationBudget::default();
    if m.fault.get() == 3 {
        budget.bindings = 0;
    }
    base.with_annotations_with_budget(&specs(m.recipe.get()), &mut factories, &style(), budget)
}
#[wasm_bindgen]
pub fn funnel_annotation_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 {
        return Err(js("unknown annotation profile"));
    }
    let d = web_sys::window().unwrap().document().unwrap();
    let root = d.create_element("section")?;
    root.set_id(&format!("funnel-annotation-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform-origin:0 0",
    )?;
    d.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let b = m.clone();
    let own = BrowserFunnel::mount_with_factory_and_svg_layers(
        root.clone(),
        source(0),
        config(),
        options(profile),
        program(0, 0, m.clone()).map_err(js)?,
        move |ctx| build(&b, ctx),
    )
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(own.handle());
    install_hooks(&own.handle(), m.clone()).map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                owner: own,
                metrics: m,
                mode: 0,
                revision: 0,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn funnel_annotation_operation(
    key: &str,
    op: &str,
    value: f64,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all
            .get_mut(key)
            .ok_or_else(|| js("annotation fixture missing"))?;
        let h = f.owner.handle();
        let n = value as u8;
        match op {
            "recipe" => {
                f.metrics.recipe.set(n);
                h.refresh().map_err(js)?;
            }
            "source" => h.set_source(source(n)).map_err(js)?,
            "fault" => {
                f.metrics.fault.set(n);
                return Ok(());
            }
            "refresh" => h.refresh().map_err(js)?,
            "program" => {
                h.set_svg_program(program(n, f.revision, f.metrics.clone()).map_err(js)?)
                    .map_err(js)?;
                h.flush().map_err(js)?;
                f.mode = n;
                return Ok(());
            }
            "revision" => {
                h.set_svg_program(program(f.mode, value as u64, f.metrics.clone()).map_err(js)?)
                    .map_err(js)?;
                h.flush().map_err(js)?;
                f.revision = value as u64;
                return Ok(());
            }
            "current" => {
                let id = CategoryKey::from(1.);
                h.set_current_part(if n == 0 { None } else { Some(&id) })
                    .map_err(js)?;
            }
            "hidden" => {
                let mut hs = BTreeSet::new();
                if n != 0 {
                    hs.insert(format!(
                        "funnel:{}",
                        CategoryKey::from("1").identity().map_err(js)?
                    ));
                }
                h.set_hidden(hs).map_err(js)?;
            }
            "view" => h
                .set_view(if n == 0 {
                    ZoomTransform::default()
                } else {
                    ZoomTransform {
                        scale: 1.1,
                        translate_x: 8.,
                        translate_y: 5.,
                    }
                })
                .map_err(js)?,
            "tip" => install_tip(&h, f.metrics.clone()).map_err(js)?,
            "stop" => {
                h.stop();
                return Ok(());
            }
            "flush" => {}
            _ => return Err(js("unknown annotation operation")),
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_annotation_owner(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("annotation fixture missing"))?;
        accepted(&f.owner.handle())
            .map(|v| v.to_string())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_annotation_delta(key: &str, start: usize) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("annotation fixture missing"))?;
        let s = snapshot(f).map_err(js)?;
        let l = f.metrics.ledger.borrow();
        if start > l.len() {
            return Err(js("annotation delta start out of range"));
        }
        Ok(json!({"start":start,"total":l.len(),"snapshot":s,"entries":&l[start..]}).to_string())
    })
}
#[wasm_bindgen]
pub fn funnel_annotation_full(key: &str) -> std::result::Result<String, JsValue> {
    funnel_annotation_delta(key, 0)
}
#[wasm_bindgen]
pub fn funnel_annotation_drop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all
            .remove(key)
            .ok_or_else(|| js("annotation fixture missing"))?;
        f.owner.stop();
        let value = snapshot(&f).map_err(js)?;
        if f.metrics.rows.borrow().iter().any(|r| !r.cleaned.get())
            || f.metrics.tips.borrow().iter().any(|r| !r.cleaned.get())
        {
            return Err(js("annotation Scope cleanup incomplete"));
        }
        f.root.remove();
        Ok(value.to_string())
    })
}
