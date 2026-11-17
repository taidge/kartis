//! Actual Waffle literal planes, application cohorts and retained layer UI.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{EventKind, GraphicProps};
use glory::style::{Length, PointerEvents, Position, Style};
use kartis::browser::*;

use super::*;

#[path = "waffle_svg_layers/widgets.rs"]
mod widgets;

struct Raw {
    epoch: u32,
    status: String,
    note: String,
    metric: f64,
    local: Rc<Cell<u32>>,
}
type Data = WaffleDataFrame<Raw>;
type Handle = BrowserWaffleDataHandle<Raw>;
type Layer = BrowserWaffleDataSvgLayerFrame<Raw>;
type Tip = WaffleDataTooltipFrame<Raw>;
type BatchRefresh = (u32, Box<dyn Fn() -> Value>);
struct LayerRow {
    frame: glory::Bond<Option<Layer>>,
    ctor: u32,
    cleaned: Rc<Cell<bool>>,
}
struct TipRow {
    frame: glory::Bond<Option<Tip>>,
    ctor: u32,
    cleaned: Rc<Cell<bool>>,
}
#[derive(Default)]
struct Metrics {
    owner: RefCell<Option<Handle>>,
    builds: Cell<u32>,
    functions: Cell<u32>,
    cell_functions: Cell<u32>,
    fault: Cell<u8>,
    geometry: Cell<u8>,
    action: Cell<u8>,
    constructors: Cell<u32>,
    cleanups: Cell<u32>,
    tip_constructors: Cell<u32>,
    tip_cleanups: Cell<u32>,
    layers: RefCell<Vec<LayerRow>>,
    tips: RefCell<Vec<TipRow>>,
    batches: RefCell<Vec<BatchRefresh>>,
    frames: RefCell<Vec<Data>>,
    ledger: RefCell<Vec<Value>>,
    historical: RefCell<Vec<Data>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserWaffleData<Raw>,
    metrics: Rc<Metrics>,
    profile: u8,
    mode: u8,
    revision: u64,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}

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
        title: "Owned Waffle layers / 华夫图层".into(),
        ..Default::default()
    }
}
fn options(profile: u8) -> WaffleLayoutOptions {
    WaffleLayoutOptions {
        rows: 3,
        columns: 5,
        total: Some(20.),
        direction: [
            WaffleFillDirection::Top,
            WaffleFillDirection::Right,
            WaffleFillDirection::Bottom,
            WaffleFillDirection::Left,
        ][usize::from(profile)],
        padding: 8.,
        corner_radius: 6.,
        border_width: 2.,
        border_color: InheritedColor::Fixed(Color(30, 40, 50, 255)),
        empty_color: Color(210, 215, 220, 255),
        hidden: BTreeSet::from(["hidden".into()]),
        ..Default::default()
    }
}
fn source(epoch: u32, phase: u8) -> Vec<WaffleDatum<Raw>> {
    if phase == 4 {
        return vec![];
    }
    let values = if phase == 3 {
        [12., 10., 2., -0., 4.]
    } else {
        [5., 4., 2., -0., 1.]
    };
    let mut rows = ["alpha", "beta", "hidden", "zero", "overflow"]
        .into_iter()
        .enumerate()
        .map(|(i, id)| {
            WaffleDatum::new(
                Raw {
                    epoch,
                    status: if phase == 1 { "alert" } else { "ready" }.into(),
                    note: format!("opaque-{i}-{epoch}"),
                    metric: if phase == 2 { 0. } else { -0. },
                    local: Rc::new(Cell::new(100 + i as u32)),
                },
                RadialDatum::new(id, values[i]),
            )
        })
        .collect::<Vec<_>>();
    if phase == 5 {
        rows.swap(0, 1);
    }
    if phase == 6 {
        rows.remove(0);
    }
    rows
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn raw_json(r: &Raw) -> Value {
    json!({"epoch":r.epoch,"status":r.status,"note":r.note,"metricBits":r.metric.to_bits().to_string(),"local":r.local.get(),"address":format!("{r:p}")})
}
fn geometry_json(g: &Geometry) -> Value {
    json!({"kind":match g {Geometry::Rect{..}=>"Rect",Geometry::Circle{..}=>"Circle",Geometry::Path(_)=>"Path"},
        "commands":g.commands().iter().map(|c|match c{
            PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),
            PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!(["Z"])
        }).collect::<Vec<_>>()})
}
fn scene_json(s: &Scene) -> Value {
    json!({"width":s.width,"height":s.height,"background":rgba(s.background),
        "marks":s.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"valueBits":m.value.to_bits().to_string(),"label":m.label,
            "geometry":geometry_json(&m.geometry),"fill":m.fill.map(rgba),"opacity":m.opacity,
            "fillPaint":m.fill_paint.as_ref().map(|p|match p{FillPaint::Pattern(p)=>json!({"kind":"pattern","spacing":p.spacing,"rotation":p.rotation,"offset":p.offset,"foreground":rgba(p.foreground),"background":rgba(p.background)}),_=>json!({"kind":"gradient"})}),
            "stroke":m.stroke.map(|s|json!({"color":rgba(s.color),"width":s.width})),"clip":m.clip.map(|r|[r.x,r.y,r.width,r.height])})).collect::<Vec<_>>(),
        "labels":s.labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"color":rgba(l.color),"size":l.size})).collect::<Vec<_>>(),
        "legend":s.legend.iter().map(|l|json!({"id":l.id,"label":l.label,"color":rgba(l.color)})).collect::<Vec<_>>(),
        "order":s.draw_items().iter().map(|i|match i{DrawItem::Mark(i)=>json!(["M",i]),DrawItem::Label(i)=>json!(["L",i])}).collect::<Vec<_>>()})
}
fn data_json(f: &Data) -> Value {
    let l = f.layout();
    let c = f.computed().config();
    let o = f.computed().options();
    json!({"source":f.source().iter().map(|r|json!({"id":r.projected.id,"valueBits":r.projected.value.to_bits().to_string(),"raw":raw_json(&r.data)})).collect::<Vec<_>>(),
        "config":{"width":c.width,"height":c.height,"margin":[c.margin.left,c.margin.right,c.margin.top,c.margin.bottom]},
        "options":{"rows":o.rows,"columns":o.columns,"total":o.total,"direction":format!("{:?}",o.direction),"allocation":format!("{:?}",o.allocation),"mode":format!("{:?}",o.mode),"padding":o.padding,"radius":o.corner_radius,"hidden":o.hidden},
        "pool":format!("{:p}",f.source().as_ptr()),"total":l.total,"unit":l.unit,"grid":[l.grid.x,l.grid.y,l.grid.width,l.grid.height],"cellSize":l.cell_size,
        "groups":l.groups.iter().map(|g|json!({"id":g.source.id,"sourceIndex":g.source_index,"valueBits":g.source.value.to_bits().to_string(),"formatted":g.formatted_value,"hidden":g.hidden,
            "color":rgba(g.color),"border":rgba(g.border_color),"requested":[g.requested.start,g.requested.end],"occupied":[g.occupied.start,g.occupied.end],"polygons":g.polygons,"areaMarkId":g.area_mark_id})).collect::<Vec<_>>(),
        "cells":l.cells.iter().map(|c|json!({"index":c.index,"row":c.row,"column":c.column,"owner":c.owner,"markId":c.mark_id,"bounds":[c.bounds.x,c.bounds.y,c.bounds.width,c.bounds.height]})).collect::<Vec<_>>(),
        "scene":scene_json(f.scene())})
}
fn frame_id(m: &Metrics, f: &Data) -> usize {
    let mut frames = m.frames.borrow_mut();
    if let Some(i) = frames.iter().position(|old| old == f) {
        i
    } else {
        let i = frames.len();
        frames.push(f.clone());
        i
    }
}
fn layer_json(m: &Metrics, f: &Layer) -> Value {
    json!({"frame":frame_id(m,&f.data),"generation":f.source_generation,"publication":f.publication,"key":f.key,"index":f.index,
        "view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden,"interactive":f.interactive,"plan":plan_json(&f.presentation),"sameTarget":f.presentation.frame()==f.data.computed()})
}
fn plan_json(plan: &WaffleSvgPlan) -> Value {
    json!({"work":plan.work(),"sourceCommands":plan.source_commands(),"layers":plan.layers().iter().map(|o|json!({"index":o.index,"key":o.layer.key,
        "kind":match &o.layer.kind{WaffleSvgLayerKind::Cells=>"cells",WaffleSvgLayerKind::Areas=>"areas",WaffleSvgLayerKind::Legends=>"legends",WaffleSvgLayerKind::Custom(_)=>"custom"},
        "items":o.items().iter().map(|item|match *item{WaffleSvgItem::Cell{cell_index,mark_index}=>json!({"kind":"cell","cellIndex":cell_index,"markIndex":mark_index}),
            WaffleSvgItem::Area{source_index,polygon_index}=>json!({"kind":"area","sourceIndex":source_index,"polygonIndex":polygon_index}),
            WaffleSvgItem::Legend{legend_index,item_index}=>json!({"kind":"legend","legendIndex":legend_index,"itemIndex":item_index})}).collect::<Vec<_>>(),
        "scene":match plan.native_scene(o.index){Ok(s)=>json!({"Ok":scene_json(&s)}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>()})
}
fn tip_json(m: &Metrics, f: &Tip) -> Value {
    json!({"frame":frame_id(m,&f.data),"generation":f.source_generation,"publication":f.publication,"sourceIndex":f.source_index,"polygonIndex":f.polygon_index,
        "occurrence":{"key":f.occurrence.key,"index":f.occurrence.index,"publication":f.occurrence.publication},
        "pointerClient":f.pointer_client,"viewportAnchor":f.viewport_anchor,"requestedAnchor":format!("{:?}",f.requested_anchor),"anchor":format!("{:?}",f.anchor),
        "text":match f.text(){Ok(s)=>json!({"Ok":s}),Err(e)=>json!({"Err":e.to_string()})}})
}
fn owner_json(m: &Metrics, h: &Handle) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"active":s.active,"frame":s.data.as_ref().map(|d|frame_id(m,d)),"generation":s.source_generation,"publication":s.publication,"hovered":s.hovered,
        "view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hidden":s.hidden,"pending":s.pending,"rafPending":s.raf_pending,
        "error":s.error.map(|e|e.to_string()),"uiErrors":s.ui_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"eventErrors":s.event_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "plan":s.presentation.as_ref().map(plan_json),"tooltip":s.tooltip.as_ref().map(|f|tip_json(m,f)),"lastHit":s.last_hit.as_ref().map(|hit|json!({"frame":frame_id(m,&hit.data),"generation":hit.source_generation,"publication":hit.publication,
            "key":hit.key,"index":hit.index,"sourceIndex":hit.source_index,"polygonIndex":hit.polygon_index,"areaId":hit.area_id,"client":hit.client,"viewport":hit.viewport,"scene":hit.scene,"trusted":hit.trusted,"nativeType":hit.native_type}))}),
    )
}
fn current(m: &Metrics) -> Value {
    m.owner
        .borrow()
        .as_ref()
        .map(|h| owner_json(m, h).unwrap_or_else(|e| json!({"Err":e.to_string()})))
        .unwrap_or(Value::Null)
}
fn log(m: &Metrics, kind: &str, value: Value) {
    let mut ledger = m.ledger.borrow_mut();
    let ordinal = ledger.len();
    ledger.push(json!({"ordinal":ordinal,"kind":kind,"value":value}));
}
fn cohort(m: &Metrics, data: Option<&Data>) -> Value {
    m.owner.borrow().as_ref().map(|h|{let o=h.snapshot().unwrap();json!({"data":data.map(|f|o.data.as_ref()==Some(f)),"pool":data.map(|f|o.data.as_ref().is_some_and(|old|old.shares_source(f)))})}).unwrap_or(Value::Null)
}
fn snapshot(f: &Fixture) -> Result<Value> {
    let m = &f.metrics;
    let owner = owner_json(m, &f.owner.handle())?;
    let peers = widgets::peers(m);
    let historical = m
        .historical
        .borrow()
        .iter()
        .map(|d| frame_id(m, d))
        .collect::<Vec<_>>();
    let frames = m
        .frames
        .borrow()
        .iter()
        .enumerate()
        .map(|(i, d)| json!({"id":i,"data":data_json(d)}))
        .collect::<Vec<_>>();
    Ok(
        json!({"owner":owner,"frames":frames,"builds":m.builds.get(),"functions":m.functions.get(),"cellFunctions":m.cell_functions.get(),"geometryMode":m.geometry.get(),
        "constructors":m.constructors.get(),"cleanups":m.cleanups.get(),"tipConstructors":m.tip_constructors.get(),"tipCleanups":m.tip_cleanups.get(),"peers":peers,
        "historical":historical }),
    )
}

fn build(m: &Metrics, c: &BrowserWaffleDataBuildContext<'_, Raw>) -> Result<Data> {
    m.builds.set(m.builds.get() + 1);
    log(
        m,
        "build",
        json!({"number":m.builds.get(),"pool":format!("{:p}",c.source.as_ptr()),"source":c.source.iter().map(|d|json!({"id":d.projected.id,"valueBits":d.projected.value.to_bits().to_string(),"raw":raw_json(&d.data)})).collect::<Vec<_>>()}),
    );
    if m.fault.get() == 1 {
        return Err(ChartError("authored Waffle source rejection".into()));
    }
    if m.fault.get() == 4 {
        return WaffleDataFrame::from_shared(c.config, source(999, 0).into(), c.options);
    }
    let mut color = |ctx: &WaffleDataColorContext<'_, Raw>| {
        m.functions.set(m.functions.get() + 1);
        log(
            m,
            "function",
            json!({"role":"color","id":ctx.computed.datum.id,"raw":raw_json(ctx.data)}),
        );
        let i = match ctx.computed.datum.id.as_str() {
            "alpha" => 0,
            "beta" => 1,
            "hidden" => 2,
            "zero" => 3,
            _ => 4,
        };
        Ok(if ctx.data.status == "alert" {
            [
                Color(220, 120, 20, 255),
                Color(20, 170, 80, 255),
                Color(100, 110, 120, 255),
                Color(160, 80, 190, 255),
                Color(30, 180, 180, 255),
            ][i]
        } else {
            [
                Color(200, 40, 60, 255),
                Color(30, 100, 210, 255),
                Color(100, 110, 120, 255),
                Color(160, 80, 190, 255),
                Color(30, 180, 180, 255),
            ][i]
        })
    };
    let mut formatted = |v: f64| {
        m.functions.set(m.functions.get() + 1);
        log(
            m,
            "function",
            json!({"role":"format","valueBits":v.to_bits().to_string()}),
        );
        Ok(format!("v={v:.1}"))
    };
    let mut border = |ctx: &WaffleDataBorderContext<'_, Raw>| {
        m.functions.set(m.functions.get() + 1);
        log(
            m,
            "function",
            json!({"role":"border","raw":ctx.data.map(raw_json)}),
        );
        Ok(if ctx.data.is_some_and(|d| d.status == "alert") {
            Color(0, 0, 0, 255)
        } else {
            Color(100, 30, 180, 255)
        })
    };
    let mut factories = WaffleDataFactories {
        color: Some(&mut color),
        formatted_value: Some(&mut formatted),
        border_color: Some(&mut border),
    };
    if m.geometry.get() == 0 {
        return WaffleDataFrame::with_shared_factories(
            c.config,
            c.source.clone(),
            c.options,
            &mut factories,
        );
    }
    let mut cell = |ctx: &WaffleDataCellContext<'_, Raw>| {
        m.cell_functions.set(m.cell_functions.get() + 1);
        log(
            m,
            "cell-function",
            json!({"cell":ctx.computed.cell.index,"position":ctx.computed.position,"raw":ctx.data.map(raw_json)}),
        );
        let p = ctx.computed.position;
        let radius = ctx.computed.cell.bounds.width / 3.;
        let mut ink = ctx.computed.inherit(Geometry::Path(vec![
            PathCommand::Move(glory::render::GraphicPoint::new(
                p[0] - radius,
                p[1] + radius,
            )),
            PathCommand::Line(glory::render::GraphicPoint::new(p[0], p[1] - radius)),
            PathCommand::Line(glory::render::GraphicPoint::new(
                p[0] + radius,
                p[1] + radius,
            )),
            PathCommand::Close,
        ]));
        if m.geometry.get() == 2 {
            ink.fill_paint = Some(FillPaint::Pattern(PatternFill {
                spacing: 8.,
                foreground: Color(20, 20, 20, 255),
                background: ctx.computed.fill.unwrap_or(Color(210, 215, 220, 255)),
                ..Default::default()
            }));
        }
        Ok(ink)
    };
    WaffleDataFrame::with_shared_factories_and_cell_factory(
        c.config,
        c.source.clone(),
        c.options,
        &mut factories,
        &mut cell,
    )
}

#[wasm_bindgen]
pub fn waffle_svg_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 {
        return Err(js("unknown Waffle profile"));
    }
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    root.set_id(&format!("waffle-svg-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform-origin:0 0",
    )?;
    document.body().unwrap().append_child(&root)?;
    let metrics = Rc::new(Metrics::default());
    let builder = metrics.clone();
    let owner = BrowserWaffleData::mount_with_factory_and_svg_layers(
        root.clone(),
        source(0, 0),
        config(),
        options(profile),
        widgets::program(0, 0, metrics.clone()).map_err(js)?,
        move |c| build(&builder, c),
    )
    .map_err(js)?;
    *metrics.owner.borrow_mut() = Some(owner.handle());
    widgets::hooks(&owner.handle(), metrics.clone()).map_err(js)?;
    owner.flush().map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                owner,
                metrics,
                profile,
                mode: 0,
                revision: 0,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn waffle_svg_operation(
    key: &str,
    op: &str,
    a: f64,
    b: f64,
) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all|{
        let f=all.get_mut(key).ok_or_else(||js("missing Waffle fixture"))?;let h=f.owner.handle();
        let result:Result<()>=match op {
            "source"=>h.set_source(source(a as u32,b as u8)),
            "refresh"=>h.refresh(),
            "program"=>{f.mode=a as u8;f.revision=b as u64;widgets::program(f.mode,f.revision,f.metrics.clone()).and_then(|p|h.set_svg_program(p))},
            "view"=>h.set_view(ZoomTransform{scale:a,translate_x:b,translate_y:-b}),
            "hidden"=>h.set_hidden(if a==0.{BTreeSet::new()}else if a==1.{BTreeSet::from(["alpha".into(),"hidden".into()])}else{BTreeSet::from(["hidden".into()])}),
            "resize"=>h.resize(a,b),
            "geometry"=>{f.metrics.geometry.set(a as u8);h.refresh()},
            "mode"=>{let mut o=options(f.profile);o.mode=if a==0.{WaffleMode::Cells}else{WaffleMode::Areas};h.set_options(o)},
            "tooltip"=>widgets::tooltip(&h,f.metrics.clone(),a as u64),
            "standard-tip"=>h.clear_tooltip_component().and_then(|_|h.set_tooltip_options(BrowserWaffleTooltipOptions{enabled:a!=0.,..Default::default()})),
            "reset-tip"=>h.reset_tooltip(),
            "fault"=>{f.metrics.fault.set(a as u8);Ok(())},
            "action"=>{f.metrics.action.set(a as u8);Ok(())},
            "css"=>f.root.set_attribute("style",&format!("position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform:translate({b}px,0) scale({a});transform-origin:0 0")).map_err(|e|ChartError(format!("{e:?}"))),
            "visibility"=>f.root.clone().dyn_into::<web_sys::HtmlElement>().map_err(|e|ChartError(format!("{e:?}"))).and_then(|n|n.style().set_property("display",if a==0.{"none"}else{""}).map_err(|e|ChartError(format!("{e:?}")))),
            "stop"=>{h.stop();Ok(())},
            _=>Err(ChartError("unknown Waffle operation".into())),
        };
        let result=result.and_then(|_|if op=="stop" {Ok(())}else{h.flush()});
        Ok(json!({"Ok":result.is_ok(),"error":result.err().map(|e|e.to_string()),"snapshot":snapshot(f).map_err(js)?}).to_string())
    })
}
#[wasm_bindgen]
pub fn waffle_svg_read(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        snapshot(all.get(key).ok_or_else(|| js("missing Waffle fixture"))?)
            .map(|v| v.to_string())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn waffle_svg_owner(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("missing Waffle fixture"))?;
        owner_json(&f.metrics, &f.owner.handle())
            .map(|v| v.to_string())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn waffle_svg_drain(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("missing Waffle fixture"))?;
        Ok(serde_json::to_string(&std::mem::take(&mut *f.metrics.ledger.borrow_mut())).unwrap())
    })
}
#[wasm_bindgen]
pub fn waffle_svg_batch_refresh(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("missing Waffle fixture"))?;
        let batches = f.metrics.batches.borrow();
        let (_, batch) = batches
            .iter()
            .find(|(ctor, _)| widgets::location("layer", *ctor)["accepted"] == true)
            .ok_or_else(|| js("missing accepted Waffle Scope"))?;
        Ok(batch().to_string())
    })
}
#[wasm_bindgen]
pub fn waffle_svg_drop(key: &str) {
    FIXTURES.with_borrow_mut(|all| {
        if let Some(f) = all.remove(key) {
            f.owner.stop();
            f.root.remove();
        }
    });
}
