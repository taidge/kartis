//! Opaque application data through the actual dedicated Funnel owner.
use std::collections::BTreeSet;

use kartis::browser::*;

use super::*;

#[path = "funnel_data/build.rs"]
mod build;
#[path = "funnel_data/widgets.rs"]
mod widgets;

// Deliberately no Clone, Debug, PartialEq or Serialize implementation. Rc/Cell
// also makes this an actual non-Send/non-Sync application type.
struct Raw {
    region: String,
    revision: u32,
    metric: f64,
    nested: Vec<String>,
    local: Rc<Cell<u32>>,
}
type Handle = BrowserFunnelDataHandle<Raw>;
type Frame = FunnelDataFrame<Raw>;
type LayerFrame = BrowserFunnelDataSvgLayerFrame<Raw>;
type TipFrame = FunnelDataTooltipFrame<Raw>;
type LayerBond = glory::Bond<Option<LayerFrame>>;
type TipBond = glory::Bond<Option<TipFrame>>;
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
    owner: RefCell<Option<Handle>>,
    builds: Cell<u32>,
    fault: Cell<u8>,
    action: Cell<u8>,
    constructors: Cell<u32>,
    cleanups: Cell<u32>,
    tip_constructors: Cell<u32>,
    tip_cleanups: Cell<u32>,
    rows: RefCell<Vec<LayerRow>>,
    tips: RefCell<Vec<TipRow>>,
    ledger: RefCell<Vec<Value>>,
    historical: RefCell<Vec<Frame>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserFunnelData<Raw>,
    metrics: Rc<Metrics>,
    mode: u8,
    revision: u64,
}
thread_local! { static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const {RefCell::new(BTreeMap::new())}; }

fn source(epoch: u32, phase: u8) -> Vec<FunnelDataDatum<Raw>> {
    let mut rows: Vec<_> = [
        (CategoryKey::from("1"), 8., 180., Color(190, 40, 50, 255)),
        (CategoryKey::from(1.), 4., 120., Color(30, 90, 210, 255)),
        (CategoryKey::from("zero"), -0., 0., Color(20, 160, 70, 255)),
        (
            CategoryKey::from("transparent"),
            2.,
            90.,
            Color(80, 90, 120, 0),
        ),
        (CategoryKey::from("tail"), 0., 60., Color(180, 100, 30, 255)),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, value, size, color))| {
        let mut projected = FunnelDatum::new(id, if phase == 1 && i == 0 { 10. } else { value });
        projected.label = format!("source-{i}");
        projected.color = Some(color);
        projected.size = Some(size);
        FunnelDataDatum::new(
            Raw {
                region: format!("region-{i}"),
                revision: epoch,
                metric: if epoch.is_multiple_of(2) { -0. } else { 0. },
                nested: vec![
                    format!("business-{i}"),
                    format!("epoch-{epoch}"),
                    "<raw & opaque>".into(),
                ],
                local: Rc::new(Cell::new(100 + i as u32)),
            },
            projected,
        )
    })
    .collect();
    if phase == 2 {
        rows.swap(0, 1);
    }
    if phase == 3 {
        rows.remove(1);
    }
    rows
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
        title: "Opaque Funnel application data / 原始业务字段".into(),
        ..Default::default()
    }
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
        current_size_extension: 12.,
        before: None,
        after: None,
        labels: None,
        legend: false,
        ..Default::default()
    }
}
fn raw(d: &Raw) -> Value {
    json!({"region":d.region,"revision":d.revision,"metricBits":d.metric.to_bits().to_string(),
        "nested":d.nested,"local":d.local.get(),"address":format!("{d:p}")})
}
fn data(f: &Frame) -> Value {
    // Every byte comes from the public typed frame, never a desired-source Cell.
    let source: Vec<_> = f.source().iter().map(|d| raw(&d.data)).collect();
    let bindings =
        f.computed().annotations().map(|a| {
            a.bindings().iter().map(|b| json!({
        "sourceIndex":b.source_index,"partIndex":b.part_index,"specIndex":b.spec_index,
        "raw":raw(f.data(b.source_index).unwrap()),"center":b.center
    })).collect::<Vec<_>>()
        });
    json!({"raw":source,"poolAddress":format!("{:p}",f.source().as_ptr()),
        "computed":super::funnel_svg_layers::data_json(f.computed()),"annotations":bindings})
}
fn layer(f: &LayerFrame) -> Value {
    json!({"data":data(&f.data),"generation":f.source_generation,"publication":f.publication,
        "key":f.key,"index":f.index,"view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden,"interactive":f.interactive})
}
fn tip(f: &TipFrame) -> Value {
    json!({"data":data(&f.data),"generation":f.source_generation,"publication":f.publication,
        "sourceIndex":f.source_index,"partIndex":f.part_index,"client":f.pointer_client,
        "occurrence":{"key":f.occurrence.key,"index":f.occurrence.index,"publication":f.occurrence.publication}})
}
fn owner(h: &Handle) -> Result<Value> {
    let s = h.snapshot()?;
    Ok(
        json!({"active":s.active,"data":s.data.as_ref().map(data),"generation":s.source_generation,
        "publication":s.publication,"current":s.current.as_ref().map(super::funnel_svg_layers::key_json),
        "view":[s.view.scale,s.view.translate_x,s.view.translate_y],"hidden":s.hidden,"pending":s.pending,
        "error":s.error.map(|e|e.to_string()),"uiErrors":s.ui_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "eventErrors":s.event_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"tooltip":s.tooltip.as_ref().map(tip),
        "lastHit":s.last_hit.as_ref().map(|hit|json!({"data":data(&hit.data),"generation":hit.source_generation,
            "publication":hit.publication,"partIndex":hit.part_index,"sourceIndex":hit.source_index,"markId":hit.mark_id,
            "client":hit.client,"viewport":hit.viewport,"scene":hit.scene,"trusted":hit.trusted,"type":hit.native_type,
            "occurrence":{"key":hit.key,"index":hit.index}}))}),
    )
}
fn current(m: &Metrics) -> Value {
    m.owner
        .borrow()
        .as_ref()
        .map(|h| match owner(h) {
            Ok(v) => v,
            Err(e) => json!({"Err":e.to_string()}),
        })
        .unwrap_or(Value::Null)
}
fn cohort_matches(m: &Metrics, frame: &Frame) -> Value {
    let h = m.owner.borrow();
    let Some(h) = h.as_ref() else {
        return Value::Null;
    };
    match h.snapshot() {
        Ok(snapshot) => snapshot
            .data
            .as_ref()
            .map(|accepted| {
                json!({
                    "fullFrame":accepted==frame,"rawPool":accepted.shares_source(frame),
                    "computed":accepted.computed()==frame.computed()
                })
            })
            .unwrap_or(Value::Null),
        Err(error) => json!({"Err":error.to_string()}),
    }
}
fn log(m: &Metrics, kind: &str, value: Value) {
    let mut ledger = m.ledger.borrow_mut();
    let ordinal = ledger.len();
    ledger.push(json!({"ordinal":ordinal,"kind":kind,"value":value}));
}
fn location(role: &str, ctor: u32) -> Value {
    let d = web_sys::window().unwrap().document().unwrap();
    let q = if role == "layer" {
        format!(".fdr-layer-{ctor}")
    } else {
        format!(".fdr-tip-{ctor}")
    };
    let e = d.query_selector(&q).unwrap();
    let connected = e.as_ref().is_some_and(|element| element.is_connected());
    let accepted = e
        .as_ref()
        .and_then(|e| {
            e.closest(".kartis-funnel-svg-layers[data-funnel-svg-active=\"true\"]")
                .unwrap()
        })
        .is_some();
    let in_tip = e
        .as_ref()
        .and_then(|e| e.closest(".kartis-funnel-tooltip-component").unwrap())
        .is_some();
    json!({"connected":connected,"accepted":accepted,"inTip":in_tip})
}
fn snapshot(f: &Fixture) -> Result<Value> {
    let m = &f.metrics;
    Ok(
        json!({"owner":owner(&f.owner.handle())?,"builds":m.builds.get(),"constructors":m.constructors.get(),
        "cleanups":m.cleanups.get(),"tipConstructors":m.tip_constructors.get(),"tipCleanups":m.tip_cleanups.get(),
        "layers":m.rows.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("layer",r.ctor),
            "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(layer)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "tips":m.tips.borrow().iter().map(|r|json!({"ctor":r.ctor,"cleaned":r.cleaned.get(),"location":location("tip",r.ctor),
            "frame":match r.frame.try_with_untracked(|v|v.as_ref().map(tip)){Ok(v)=>json!({"Ok":v}),Err(e)=>json!({"Err":e.to_string()})}})).collect::<Vec<_>>(),
        "historical":m.historical.borrow().iter().map(data).collect::<Vec<_>>(),"count":m.ledger.borrow().len()}),
    )
}

#[wasm_bindgen]
pub fn funnel_data_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 {
        return Err(js("unknown data profile"));
    }
    let d = web_sys::window().unwrap().document().unwrap();
    let root = d.create_element("section")?;
    root.set_id(&format!("funnel-data-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform-origin:0 0",
    )?;
    d.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let bm = m.clone();
    let own = BrowserFunnelData::mount_with_factory_and_svg_layers(
        root.clone(),
        source(0, 0),
        config(),
        options(profile),
        widgets::program(0, 0, m.clone()).map_err(js)?,
        move |ctx| build::build(&bm, ctx),
    )
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(own.handle());
    widgets::hooks(&own.handle(), m.clone()).map_err(js)?;
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
pub fn funnel_data_operation(
    key: &str,
    operation: &str,
    a: f64,
    b: f64,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow_mut(|all|{
        let f=all.get_mut(key).ok_or_else(||js("unknown data fixture"))?;let h=f.owner.handle();
        let result=match operation{
            "source"=>h.set_source(source(a as u32,b as u8)),
            "view"=>h.set_view(ZoomTransform{scale:a,translate_x:b,translate_y:5.}),
            "hidden"=>h.set_hidden(if a==0.{BTreeSet::new()}else{BTreeSet::from(["funnel:s1".into()])}),
            "program"=>{f.mode=a as u8;f.revision=b as u64;widgets::program(f.mode,f.revision,f.metrics.clone()).and_then(|p|h.set_svg_program(p))},
            "budget"=>widgets::program(0,b as u64,f.metrics.clone()).and_then(|p|p.with_budget(FunnelSvgBudget{components:a as usize,..Default::default()})).and_then(|p|h.set_svg_program(p)),
            "current"=>{let id=CategoryKey::from("1");h.set_current_part(if a==0.{None}else{Some(&id)})},
            "fault"=>{f.metrics.fault.set(a as u8);Ok(())},
            "action"=>{f.metrics.action.set(a as u8);Ok(())},
            "tip"=>widgets::tooltip(&h,f.metrics.clone(),a as u64),
            "standard-tip"=>h.clear_tooltip_component().and_then(|()|h.set_tooltip_options(BrowserFunnelTooltipOptions{enabled:true,..Default::default()})),
            "disable-tip"=>h.set_tooltip_options(BrowserFunnelTooltipOptions{enabled:false,..Default::default()}),
            "css"=>{f.root.set_attribute("style",&format!("position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform-origin:0 0;transform:scale({a},{b})"))?;Ok(())},
            "refresh"=>h.refresh(),
            "flush"=>h.flush(),
            "stop"=>{h.stop();Ok(())},
            _=>return Err(js("unknown data operation")),
        };
        result.map_err(js)?;if operation!="stop"&&operation!="fault"&&operation!="action"{h.flush().map_err(js)?;}Ok(())
    })
}
#[wasm_bindgen]
pub fn funnel_data_full(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("unknown data fixture"))?;
        Ok(
            json!({"snapshot":snapshot(f).map_err(js)?,"ledger":&*f.metrics.ledger.borrow()})
                .to_string(),
        )
    })
}
#[wasm_bindgen]
pub fn funnel_data_delta(key: &str, start: usize) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("unknown data fixture"))?;
        let l = f.metrics.ledger.borrow();
        Ok(
            json!({"snapshot":snapshot(f).map_err(js)?,"entries":&l[start.min(l.len())..]})
                .to_string(),
        )
    })
}
#[wasm_bindgen]
pub fn funnel_data_owner(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        owner(
            &all.get(key)
                .ok_or_else(|| js("unknown data fixture"))?
                .owner
                .handle(),
        )
        .map(|v| v.to_string())
        .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_data_drop(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let f = all.remove(key).ok_or_else(|| js("unknown data fixture"))?;
        f.owner.stop();
        let value = snapshot(&f).map_err(js)?.to_string();
        f.root.remove();
        Ok(value)
    })
}
