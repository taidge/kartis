//! Actual literal Funnel SVG layers and retained application-owned Glory UI.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserFunnel, BrowserFunnelBuildContext, BrowserFunnelCurrentPart, BrowserFunnelHandle,
    BrowserFunnelSvgLayerContext, BrowserFunnelSvgLayerFrame, BrowserFunnelSvgProgram,
};

use super::*;

type FrameBond = glory::Bond<Option<BrowserFunnelSvgLayerFrame>>;
type BatchControl = (u32, Box<dyn Fn() -> Value>);
struct Row {
    key: String,
    ctor: u32,
    frame: FrameBond,
    current: BrowserFunnelCurrentPart,
}
#[derive(Default)]
struct Metrics {
    builds: Cell<u32>,
    functions: Cell<u32>,
    constructors: Cell<u32>,
    scopes: Cell<u32>,
    cleanups: Cell<u32>,
    fault: Cell<u8>,
    action: Cell<u8>,
    owner: RefCell<Option<BrowserFunnelHandle>>,
    rows: RefCell<Vec<Row>>,
    calls: RefCell<Vec<Value>>,
    probes: RefCell<Vec<Value>>,
    actions: RefCell<Vec<Value>>,
    batches: RefCell<Vec<BatchControl>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserFunnel,
    metrics: Rc<Metrics>,
    profile: u8,
    mode: u8,
    revision: u64,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn rgba(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
pub(super) fn key_json(k: &CategoryKey) -> Value {
    match k {
        CategoryKey::String(s) => {
            json!({"kind":"string","value":s,"identity":k.identity().unwrap()})
        }
        CategoryKey::Number(n) => {
            json!({"kind":"number","value":n,"bits":n.to_bits().to_string(),"identity":k.identity().unwrap()})
        }
    }
}
pub(super) fn config() -> ChartConfig {
    ChartConfig {
        width: 420.,
        height: 300.,
        margin: Margin {
            left: 20.,
            top: 20.,
            right: 20.,
            bottom: 20.,
        },
        title: "Funnel SVG layers / 漏斗函数层".into(),
        ..Default::default()
    }
}
pub(super) fn options(profile: u8) -> FunnelLayoutOptions {
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
        fill_opacity: 0.5,
        border_width: 2.,
        border_opacity: 1.,
        before: Some(FunnelSeparatorOptions {
            length: 12.,
            offset: 3.,
            ..Default::default()
        }),
        after: Some(FunnelSeparatorOptions {
            length: 12.,
            offset: 3.,
            ..Default::default()
        }),
        current_size_extension: 12.,
        current_border_width: Some(5.),
        legend: false,
        ..Default::default()
    }
}
pub(super) fn source(phase: u8) -> Vec<FunnelDatum> {
    let mut rows = vec![
        FunnelDatum::new("1", if phase == 1 { 10. } else { 8. }),
        FunnelDatum::new(1., if phase == 1 { 3. } else { 4. }),
        FunnelDatum::new("zero", if phase == 1 { 0. } else { -0. }),
        FunnelDatum::new("transparent", 2.),
        FunnelDatum::new("last-zero", if phase == 4 { -0. } else { 0. }),
    ];
    for (i, row) in rows.iter_mut().enumerate() {
        row.label = format!("stage-{i}{}", if phase == 1 { "-updated" } else { "" });
        row.size = Some([180., 100., 0., 100., 0.][i]);
        row.color = Some(
            [
                Color(200, 30, 40, 255),
                Color(30, 90, 210, 255),
                Color(30, 170, 80, 255),
                Color(40, 80, 120, 0),
                Color(90, 100, 110, 255),
            ][i],
        );
    }
    if phase == 2 {
        rows.swap(0, 1);
    }
    if phase == 3 {
        rows.remove(1);
    }
    rows
}
fn datum_json(d: &FunnelDatum) -> Value {
    json!({"id":key_json(&d.id),"valueBits":d.value.to_bits().to_string(),"label":d.label,"sizeBits":d.size.map(|v|v.to_bits().to_string()),"color":d.color.map(rgba)})
}
fn geometry_json(g: &Geometry) -> Value {
    Value::Array(
        g.commands()
            .iter()
            .map(|c| match c {
                PathCommand::Move(p) => json!(["M", p.x, p.y]),
                PathCommand::Line(p) => json!(["L", p.x, p.y]),
                PathCommand::Cubic { first, second, end } => {
                    json!(["C", first.x, first.y, second.x, second.y, end.x, end.y])
                }
                PathCommand::Close => json!(["Z"]),
            })
            .collect(),
    )
}
pub(super) fn data_json(f: &FunnelSvgFrame) -> Value {
    let layout = f.layout();
    let plot = f.plot();
    json!({
        "source":f.source().iter().map(datum_json).collect::<Vec<_>>(),
        "config":[f.config().width,f.config().height],"plot":[plot.x,plot.y,plot.width,plot.height],"innerSize":f.inner_size(),
        "direction":format!("{:?}",f.options().direction),"interpolation":format!("{:?}",f.options().interpolation),"current":f.options().current.as_ref().map(key_json),
        "parts":f.parts().iter().enumerate().map(|(i,p)|json!({"index":p.source_index,"datum":datum_json(&p.datum),"center":p.center,"bounds":[p.bounds.x,p.bounds.y,p.bounds.width,p.bounds.height],"color":rgba(p.color),"formatted":p.formatted_value,"current":p.is_current,"series":p.series,"markId":p.mark_id,"profile":p.profile.iter().map(|s|json!({"before":s.before,"after":s.after})).collect::<Vec<_>>(),"body":f.body_mark(i).unwrap().map(|m|m.id.clone()),"border":f.border_mark(i).unwrap().map(|m|m.id.clone()),"label":f.label(i).unwrap().map(|l|l.text.clone()),"originalBody":f.original_body_geometry(i).unwrap().map(|g|geometry_json(&g))})).collect::<Vec<_>>(),
        "styles":f.styles().iter().map(|s|json!({"color":rgba(s.color),"fillOpacity":s.fill_opacity,"borderColor":rgba(s.border_color),"borderWidth":s.border_width,"borderOpacity":s.border_opacity,"labelColor":s.label_color.map(rgba)})).collect::<Vec<_>>(),
        "separators":f.separators().iter().map(|s|json!({"part":s.part_id.as_ref().map(key_json),"side":format!("{:?}",s.side),"from":s.from,"to":s.to})).collect::<Vec<_>>(),
        "marks":f.scene().marks.iter().map(|m|json!({"id":m.id,"series":m.series,"valueBits":m.value.to_bits().to_string(),"color":m.fill.map(rgba),"opacity":m.opacity,"path":geometry_json(&m.geometry)})).collect::<Vec<_>>(),
        "labels":f.scene().labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"color":rgba(l.color)})).collect::<Vec<_>>(),
        "sites":layout.interaction_binding().sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"position":s.position})).collect::<Vec<_>>(),
        "extras":f.extras().iter().map(|i|format!("{i:?}")).collect::<Vec<_>>()
    })
}
pub(super) fn layer_json(f: &BrowserFunnelSvgLayerFrame) -> Value {
    json!({"data":data_json(&f.data),"generation":f.source_generation,"publication":f.publication,"key":f.key,"index":f.index,"view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden,"interactive":f.interactive})
}
fn cohort_json(f: &BrowserFunnelSvgLayerFrame) -> Value {
    let plot = f.data.plot();
    json!({"generation":f.source_generation,"publication":f.publication,"key":f.key,"index":f.index,
        "view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden,"interactive":f.interactive,
        "source":f.data.source().iter().map(datum_json).collect::<Vec<_>>(),"current":f.data.options().current.as_ref().map(key_json),
        "plot":[plot.x,plot.y,plot.width,plot.height],"config":[f.data.config().width,f.data.config().height]})
}
fn owner_cohort_json(h: &BrowserFunnelHandle) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"active":s.active,"generation":s.source_generation,"publication":s.publication,"current":s.current.as_ref().map(key_json),
        "view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hidden":s.hidden,
        "source":s.data.as_ref().map(|f|f.source().iter().map(datum_json).collect::<Vec<_>>()),
        "config":s.data.as_ref().map(|f|[f.config().width,f.config().height]),"plot":s.data.as_ref().map(|f|{let p=f.plot();[p.x,p.y,p.width,p.height]})}),
    )
}
fn scope_location(document: &web_sys::Document, ctor: u32) -> Value {
    let node = document
        .query_selector(&format!(".fsl-ctor-{ctor}"))
        .ok()
        .flatten();
    let stage = node
        .as_ref()
        .and_then(|n| n.closest(".kartis-funnel-svg-layers").ok().flatten());
    json!({"connected":node.as_ref().is_some_and(|n|n.is_connected()),
        "accepted":stage.as_ref().is_some_and(|n|n.get_attribute("data-funnel-svg-active").as_deref()==Some("true")),
        "publication":stage.as_ref().and_then(|n|n.get_attribute("data-funnel-svg-publication")),
        "state":stage.as_ref().and_then(|n|n.get_attribute("data-funnel-svg-state")),
        "inert":node.as_ref().is_some_and(|n|n.closest("[inert]").ok().flatten().is_some())})
}
pub(super) fn owner_json(h: &BrowserFunnelHandle) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"active":s.active,"generation":s.source_generation,"publication":s.publication,"current":s.current.as_ref().map(key_json),"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hidden":s.hidden,"pending":s.pending,"rafPending":s.raf_pending,"error":s.error.map(|e|e.to_string()),"uiErrors":s.ui_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"data":s.data.as_ref().map(data_json),"lastHit":s.last_hit.as_ref().map(|h|json!({"generation":h.source_generation,"publication":h.publication,"key":h.key,"index":h.index,"partIndex":h.part_index,"sourceIndex":h.source_index,"markId":h.mark_id,"client":h.client,"viewport":h.viewport,"scene":h.scene,"trusted":h.trusted,"nativeType":h.native_type,"data":data_json(&h.data)}))}),
    )
}
fn layer_factory(
    m: Rc<Metrics>,
) -> impl FnMut(BrowserFunnelSvgLayerContext) -> Result<Box<dyn Widget>> {
    move |c| {
        m.constructors.set(m.constructors.get() + 1);
        let ctor = m.constructors.get();
        let frame = c.frame;
        let initial = c.initial;
        let key = c.key;
        if m.fault.get() == 3 && key == "middle" {
            return Err(ChartError(
                "authored Funnel layer constructor rejection".into(),
            ));
        }
        let revision = c.revision;
        let cap = c.current;
        m.rows.borrow_mut().push(Row {
            key: key.clone(),
            ctor,
            frame,
            current: cap.clone(),
        });
        let m = m.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.scopes.set(m.scopes.get() + 1);
            let retired = m.clone();
            scope.on_cleanup(move || retired.cleanups.set(retired.cleanups.get() + 1))?;
            let batch_scope = scope.downgrade();
            let batch_metrics = Rc::downgrade(&m);
            m.batches.borrow_mut().push((ctor, Box::new(move || {
                let Some(scope) = batch_scope.upgrade() else { return json!({"invoked":false,"scopeGone":true}); };
                let Some(metrics) = batch_metrics.upgrade() else { return json!({"invoked":false,"metricsGone":true}); };
                let owner = metrics.owner.borrow().as_ref().unwrap().clone();
                let invoked = Cell::new(false);
                let result = scope.batch(|| {
                    invoked.set(true);
                    owner.refresh()?;
                    owner.flush()?;
                    let document = web_sys::window().unwrap().document().unwrap();
                    Ok::<_, ChartError>(json!({"owner":owner_json(&owner)?,"location":scope_location(&document,ctor)}))
                });
                match result {
                    Ok(Ok(value)) => json!({"invoked":invoked.get(),"value":value,"error":null}),
                    Ok(Err(error)) => json!({"invoked":invoked.get(),"error":error.to_string()}),
                    Err(error) => json!({"invoked":invoked.get(),"error":error.to_string()}),
                }
            })));
            let count = scope.cage(0u32);
            let observed = m.clone();
            let ordinal = Cell::new(0u32);
            scope.effect(move || {
                let own = frame.get();
                let n = ordinal.get(); ordinal.set(n + 1);
                let bootstrap=observed.owner.borrow().is_none();
                let owner = observed.owner.borrow().as_ref().map(|h|owner_cohort_json(h).map_err(|e|e.to_string()));
                let document = web_sys::window().unwrap().document().unwrap();
                let rows = observed.rows.borrow().iter().map(|r|{
                    let location=scope_location(&document,r.ctor);
                    let connected=location["connected"].as_bool().unwrap();
                    match r.frame.try_with_untracked(|f|f.as_ref().map(cohort_json)) {
                        Ok(value)=>json!({"key":r.key,"ctor":r.ctor,"connected":connected,"location":location,"value":value}),
                        Err(e)=>json!({"key":r.key,"ctor":r.ctor,"connected":connected,"location":location,"error":e.to_string()}),
                    }
                }).collect::<Vec<_>>();
                observed.probes.borrow_mut().push(json!({"ctor":ctor,"ordinal":n,"bootstrap":bootstrap,"location":scope_location(&document,ctor),"own":own.as_ref().map(cohort_json),"owner":owner.as_ref().and_then(|r|r.as_ref().ok()),"ownerError":owner.and_then(|r|r.err()),"rows":rows}));
            });
            let label_key = key.clone();
            let label_initial = initial.clone();
            let styled_initial = initial.clone();
            let styled = m.clone();
            let active = m.clone();
            let action_key = key.clone();
            let cap = cap.clone();
            glory::ui::button()
                .class(format!("fsl-button fsl-key-{key} fsl-ctor-{ctor}"))
                .style(scope.bond(move || {
                    let f=frame.get().unwrap_or_else(||styled_initial.clone());
                    let p=f.data.inner_position(f.data.parts()[0].center).unwrap();
                    Style {background:Some(Color(130,40,170,255)),..Style::new().position(Position::Absolute).left(Length::Px(p[0]-10.)).top(Length::Px(p[1]-10.)).width(Length::Px(if styled.fault.get()==5 { f64::NAN } else {20.})).height(Length::Px(20.))}
                }))
                .label(scope.bond(move ||json!({"key":label_key,"ctor":ctor,"revision":revision,"count":count.get(),"initial":layer_json(&label_initial),"frame":frame.get().as_ref().map(layer_json)}).to_string()))
                .on(EventKind::Activate,move |_| {
                    count.revise(|v|*v+=1);
                    let action=active.action.get();
                    if action!=0 {
                        let target=match action {1=>Some(CategoryKey::from(1.)),2=>None,_=>Some(CategoryKey::from("missing"))};
                        let result=cap.set_current_part(target.as_ref());
                        active.actions.borrow_mut().push(json!({"key":action_key,"action":action,"ok":result.is_ok(),"error":result.err().map(|e|e.to_string())}));
                    }
                }).show_in(scope)?;
            let graphic_key = key.clone();
            glory::ui::graphic()
                .class(format!("fsl-graphic fsl-graphic-{key}"))
                .drawing(scope.bond(move || GraphicProps {
                    width: 24.,
                    height: 18.,
                    commands: vec![
                        PathCommand::Move(GraphicPoint::new(0., 0.)),
                        PathCommand::Line(GraphicPoint::new(24., 0.)),
                        PathCommand::Line(GraphicPoint::new(24., 18.)),
                        PathCommand::Line(GraphicPoint::new(0., 18.)),
                        PathCommand::Close,
                    ],
                    fill: Some(if graphic_key == "middle" {
                        Color(170, 60, 200, 255)
                    } else {
                        Color(20, 170, 90, 255)
                    }),
                    ..Default::default()
                }))
                .style(
                    Style::new()
                        .position(Position::Absolute)
                        .left(Length::Px(22.))
                        .top(Length::Px(10.))
                        .width(Length::Px(24.))
                        .height(Length::Px(18.)),
                )
                .show_in(scope)?;
            Ok(())
        })) as Box<dyn Widget>)
    }
}
fn program(mode: u8, revision: u64, m: Rc<Metrics>) -> Result<BrowserFunnelSvgProgram> {
    let layer = |key, kind| FunnelSvgLayer::new(key, kind);
    let mut layers = vec![
        layer("under", FunnelSvgLayerKind::Custom("back".into()))?,
        layer("separators", FunnelSvgLayerKind::Separators)?,
    ];
    if mode != 1 {
        layers.push(layer("parts", FunnelSvgLayerKind::Parts)?);
    }
    layers.push(layer("middle", FunnelSvgLayerKind::Custom("front".into()))?);
    if mode == 2 {
        layers.push(layer("parts-front", FunnelSvgLayerKind::Parts)?);
    }
    layers.push(layer("labels", FunnelSvgLayerKind::Labels)?);
    layers.push(layer("extras", FunnelSvgLayerKind::SceneExtras)?);
    if mode == 3 {
        layers.reverse();
    }
    if mode == 4 {
        layers.retain(|l| l.key != "middle");
    }
    if mode == 5 {
        layers.push(layer("echo", FunnelSvgLayerKind::Custom("back".into()))?);
    }
    if mode == 90 {
        layers.push(layer("parts", FunnelSvgLayerKind::Parts)?);
    }
    if mode == 91 {
        layers.push(layer(
            "missing",
            FunnelSvgLayerKind::Custom("missing".into()),
        )?);
    }
    if mode == 7 {
        layers.clear();
    }
    let mut p = BrowserFunnelSvgProgram::new(layers)?
        .custom("back", 0, layer_factory(m.clone()))?
        .custom("front", revision, layer_factory(m))?
        .interactive(mode != 6);
    if mode == 7 {
        p = p.with_extras(FunnelSvgExtrasPolicy::Omit);
    }
    if mode == 92 || mode == 93 {
        p = p.with_budget(FunnelSvgBudget {
            components: if mode == 92 { 2 } else { 3 },
            ..Default::default()
        })?;
    }
    Ok(p)
}
#[wasm_bindgen]
pub fn funnel_svg_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 {
        return Err(js("unknown Funnel SVG profile"));
    }
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("funnel-svg-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:420px;height:300px;transform-origin:0 0",
    )?;
    doc.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let built = m.clone();
    let owner=BrowserFunnel::mount_with_factory_and_svg_layers(root.clone(),source(0),config(),options(profile),program(0,0,m.clone()).map_err(js)?,move |c:&BrowserFunnelBuildContext<'_>|{
        built.builds.set(built.builds.get()+1);
        if built.fault.get()==1 {return Err(ChartError("authored Funnel builder rejection".into()));}
        let mut color=|ctx:&FunnelColorContext<'_>|{built.functions.set(built.functions.get()+1);built.calls.borrow_mut().push(json!({"role":"color","sourceIndex":ctx.source_index,"datum":datum_json(ctx.datum)}));if built.fault.get()==2 && ctx.source_index==1 {return Err(ChartError("authored Funnel color rejection".into()));}Ok(ctx.default_color)};
        let mut format=|v:f64|{built.functions.set(built.functions.get()+1);built.calls.borrow_mut().push(json!({"role":"format","bits":v.to_bits().to_string()}));Ok(format!("v={v:.1}"))};
        let mut border=|ctx:&FunnelPartStyleContext<'_>|{built.functions.set(built.functions.get()+1);built.calls.borrow_mut().push(json!({"role":"border","sourceIndex":ctx.part.source_index}));Ok(Color(30,30,30,255))};
        let mut label=|ctx:&FunnelPartStyleContext<'_>|{built.functions.set(built.functions.get()+1);built.calls.borrow_mut().push(json!({"role":"label","sourceIndex":ctx.part.source_index}));Ok(Color(70,50,20,255))};
        let mut layout=funnel_layout_with_factories(c.config,c.source,c.options,&mut FunnelFactories {color:Some(&mut color),formatted_value:Some(&mut format),border_color:Some(&mut border),label_color:Some(&mut label)})?;
        layout.scene.labels.push(Label{x:c.config.width/2.,y:12.,text:"Funnel note / 漏斗注释".into(),size:10.,color:Color(40,40,40,255),anchor:TextAnchor::Middle,rotation:0.});
        FunnelSvgFrame::from_layout(layout)
    }).map_err(js)?;
    *m.owner.borrow_mut() = Some(owner.handle());
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                owner,
                metrics: m,
                profile,
                mode: 0,
                revision: 0,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn funnel_svg_operation(key: &str, op: &str, value: f64) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all
            .get_mut(key)
            .ok_or_else(|| js("missing Funnel SVG fixture"))?;
        let h = f.owner.handle();
        let n = value as u8;
        match op {
            "source" => h.set_source(source(n)).map_err(js)?,
            "program" => {
                h.set_svg_program(program(n, f.revision, f.metrics.clone()).map_err(js)?)
                    .map_err(js)?;
                h.flush().map_err(js)?;
                f.mode = n;
                return Ok(());
            }
            "revision" => {
                let r = value as u64;
                h.set_svg_program(program(f.mode, r, f.metrics.clone()).map_err(js)?)
                    .map_err(js)?;
                h.flush().map_err(js)?;
                f.revision = r;
                return Ok(());
            }
            "fault" => {
                f.metrics.fault.set(n);
                return Ok(());
            }
            "action" => {
                f.metrics.action.set(n);
                return Ok(());
            }
            "current" => {
                let k = match n {
                    0 => None,
                    1 => Some(CategoryKey::from("1")),
                    2 => Some(CategoryKey::from(1.)),
                    _ => Some(CategoryKey::from("missing")),
                };
                h.set_current_part(k.as_ref()).map_err(js)?;
            }
            "view" => h
                .set_view(ZoomTransform {
                    scale: if n == 0 { 1. } else { 1.1 },
                    translate_x: if n == 0 { 0. } else { 8. },
                    translate_y: if n == 0 { 0. } else { 5. },
                })
                .map_err(js)?,
            "hidden" => {
                let mut hidden = BTreeSet::new();
                if n != 0 {
                    hidden.insert(format!(
                        "funnel:{}",
                        CategoryKey::from(1.).identity().map_err(js)?
                    ));
                }
                h.set_hidden(hidden).map_err(js)?;
            }
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
            "bad-batch" => {
                h.set_source(source(1)).map_err(js)?;
                h.set_svg_program(program(91, f.revision, f.metrics.clone()).map_err(js)?)
                    .map_err(js)?;
            }
            _ => return Err(js("unknown Funnel SVG operation")),
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_svg_capability(key: &str, ctor: u32, target: u8) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("missing Funnel SVG fixture"))?;
        let rows = f.metrics.rows.borrow();
        let row = rows
            .iter()
            .find(|r| r.ctor == ctor)
            .ok_or_else(|| js("missing Funnel SVG constructor"))?;
        let id = match target {
            0 => None,
            1 => Some(CategoryKey::from("1")),
            2 => Some(CategoryKey::from(1.)),
            _ => Some(CategoryKey::from("missing")),
        };
        row.current.set_current_part(id.as_ref()).map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_svg_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{
        let f=all.get(key).ok_or_else(||js("missing Funnel SVG fixture"))?;
        Ok(json!({"owner":owner_json(&f.owner.handle()).map_err(js)?,"builds":f.metrics.builds.get(),"functions":f.metrics.functions.get(),"constructors":f.metrics.constructors.get(),"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get(),"rows":f.metrics.rows.borrow().iter().map(|r|json!({"key":r.key,"ctor":r.ctor,"frame":r.frame.try_with_untracked(|f|f.as_ref().map(cohort_json)).ok().flatten()})).collect::<Vec<_>>(),"pendingObservations":{"calls":f.metrics.calls.borrow().len(),"probes":f.metrics.probes.borrow().len(),"actions":f.metrics.actions.borrow().len()}}).to_string())
    })
}
#[wasm_bindgen]
pub fn funnel_svg_take_observations(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{
        let f=all.get(key).ok_or_else(||js("missing Funnel SVG fixture"))?;
        Ok(json!({"calls":std::mem::take(&mut *f.metrics.calls.borrow_mut()),"probes":std::mem::take(&mut *f.metrics.probes.borrow_mut()),"actions":std::mem::take(&mut *f.metrics.actions.borrow_mut())}).to_string())
    })
}
#[wasm_bindgen]
pub fn funnel_svg_drop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all|{
        let f=all.remove(key).ok_or_else(||js("missing Funnel SVG fixture"))?;f.owner.stop();f.root.remove();
        Ok(json!({"constructors":f.metrics.constructors.get(),"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get()}).to_string())
    })
}

#[wasm_bindgen]
pub fn funnel_svg_batch_refresh(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let fixture = all
            .get(key)
            .ok_or_else(|| js("missing Funnel SVG fixture"))?;
        let document = web_sys::window().unwrap().document().unwrap();
        let batches = fixture.metrics.batches.borrow();
        let (_, batch) = batches
            .iter()
            .find(|(ctor, _)| scope_location(&document, *ctor)["accepted"] == true)
            .ok_or_else(|| js("missing accepted batch Scope"))?;
        Ok(batch().to_string())
    })
}
