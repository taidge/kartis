//! Real owned Funnel springs, source data and retained application components.
use std::collections::BTreeSet;

use kartis::browser::*;

use super::*;

#[path = "funnel_motion/widgets.rs"]
mod widgets;
struct Raw {
    revision: u32,
    note: String,
    metric: f64,
    local: Rc<Cell<u32>>,
}
type Handle = BrowserFunnelDataHandle<Raw>;
type Data = FunnelDataFrame<Raw>;
type Layer = BrowserFunnelDataSvgLayerFrame<Raw>;
type Tip = FunnelDataTooltipFrame<Raw>;
type Motion = BrowserFunnelMotionFrame;
type MotionBond = glory::Bond<Option<Motion>>;
struct LayerRow {
    frame: glory::Bond<Option<Layer>>,
    motion: MotionBond,
    ctor: u32,
    cleaned: Rc<Cell<bool>>,
}
struct TipRow {
    frame: glory::Bond<Option<Tip>>,
    motion: MotionBond,
    ctor: u32,
    cleaned: Rc<Cell<bool>>,
}
#[derive(Default)]
struct Metrics {
    owner: RefCell<Option<Handle>>,
    builds: Cell<u32>,
    factories: Cell<u32>,
    fault: Cell<u8>,
    callback_step: Cell<bool>,
    constructors: Cell<u32>,
    cleanups: Cell<u32>,
    tip_constructors: Cell<u32>,
    tip_cleanups: Cell<u32>,
    layers: RefCell<Vec<LayerRow>>,
    tips: RefCell<Vec<TipRow>>,
    ledger: RefCell<Vec<Value>>,
    historical: RefCell<Vec<(Data, Option<Motion>)>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserFunnelData<Raw>,
    metrics: Rc<Metrics>,
    profile: u8,
    mode: u8,
    revision: u64,
}
thread_local! { static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const {RefCell::new(BTreeMap::new())}; }
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
        title: "Owned Funnel springs / 漏斗动画".into(),
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
        border_width: 3.,
        border_color: InheritedColor::Fixed(Color(20, 30, 40, 255)),
        labels: Some(FunnelLabelOptions::default()),
        before: Some(FunnelSeparatorOptions {
            length: 18.,
            offset: 4.,
            ..Default::default()
        }),
        after: Some(FunnelSeparatorOptions {
            length: 18.,
            offset: 4.,
            ..Default::default()
        }),
        current_size_extension: 14.,
        current_border_width: Some(7.),
        legend: false,
        ..Default::default()
    }
}
fn source(epoch: u32, phase: u8) -> Vec<FunnelDataDatum<Raw>> {
    let changed = phase != 0;
    let mut rows = [
        (CategoryKey::from("1"), 180., Color(190, 40, 50, 255)),
        (CategoryKey::from(1.), 110., Color(30, 90, 210, 255)),
        (CategoryKey::from("tail"), 60., Color(20, 160, 70, 255)),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (id, size, color))| {
        let mut p = FunnelDatum::new(
            id,
            if changed {
                [13., 7., 2.][i]
            } else {
                8. - i as f64 * 2.
            },
        );
        p.label = format!("stage-{i}-phase{}", u8::from(changed));
        p.size = Some(if changed { [90., 170., 130.][i] } else { size });
        p.color = Some(if changed {
            [
                Color(40, 130, 200, 255),
                Color(210, 110, 20, 255),
                Color(160, 40, 170, 255),
            ][i]
        } else {
            color
        });
        FunnelDataDatum::new(
            Raw {
                revision: epoch,
                note: format!("opaque-{i}-{epoch}"),
                metric: if epoch.is_multiple_of(2) { -0. } else { 0. },
                local: Rc::new(Cell::new(100 + i as u32)),
            },
            p,
        )
    })
    .collect::<Vec<_>>();
    if phase == 7 {
        rows.remove(0);
    }
    if phase == 2 {
        rows.swap(0, 1);
    }
    if phase == 3 {
        rows.remove(1);
        let mut p = FunnelDatum::new("new", 3.);
        p.size = Some(80.);
        p.color = Some(Color(60, 80, 100, 255));
        rows.push(FunnelDataDatum::new(
            Raw {
                revision: epoch,
                note: format!("new-{epoch}"),
                metric: -0.,
                local: Rc::new(Cell::new(303)),
            },
            p,
        ));
    }
    if phase == 4 || phase == 5 {
        for row in &mut rows {
            row.projected.size = Some(0.);
        }
    }
    if phase == 6 {
        rows.remove(0);
    }
    rows
}
fn specs() -> Vec<FunnelAnnotationSpec> {
    use FunnelAnnotationPosition::Relative as R;
    [
        (CategoryKey::from("1"), FunnelAnnotationKind::Dot),
        (CategoryKey::from(1.), FunnelAnnotationKind::Circle),
        (CategoryKey::from("tail"), FunnelAnnotationKind::Rect),
    ]
    .into_iter()
    .map(|(id, kind)| {
        let mut a = FunnelAnnotationSpec::new(
            FunnelAnnotationMatch::Id(id),
            kind,
            FunnelAnnotationNote::Text("note".into()),
            R(80.),
            R(-16.),
        );
        a.note_width = 30.;
        if kind != FunnelAnnotationKind::Rect {
            a.size = Some(if kind == FunnelAnnotationKind::Dot {
                10.
            } else {
                40.
            });
        }
        a.offset = 3.;
        a
    })
    .collect()
}
fn build(m: &Metrics, c: &BrowserFunnelDataBuildContext<'_, Raw>) -> Result<Data> {
    m.builds.set(m.builds.get() + 1);
    if m.fault.get() == 1 {
        return Err(ChartError("authored Funnel motion build rejection".into()));
    }
    let mut border = |ctx: &FunnelDataStyleContext<'_, Raw>| {
        m.factories.set(m.factories.get() + 1);
        Ok(if ctx.data.revision == 0 {
            Color(20, 30, 40, 255)
        } else {
            Color(100, 70, 20, 255)
        })
    };
    let mut label = |ctx: &FunnelDataStyleContext<'_, Raw>| {
        m.factories.set(m.factories.get() + 1);
        Ok(if ctx.data.revision == 0 {
            Color(255, 255, 255, 255)
        } else {
            Color(30, 30, 30, 255)
        })
    };
    let f = FunnelDataFrame::with_shared_factories(
        c.config,
        c.source.clone(),
        c.options,
        &mut FunnelDataFactories {
            border_color: Some(&mut border),
            label_color: Some(&mut label),
            ..Default::default()
        },
    )?;
    f.with_annotations(
        &specs(),
        &mut FunnelDataAnnotationFactories::default(),
        &FunnelAnnotationStyle::default(),
    )
}
fn color(c: Color) -> [u8; 4] {
    [c.0, c.1, c.2, c.3]
}
fn geometry(g: &Geometry) -> Value {
    let kind = match g {
        Geometry::Rect { .. } => "Rect",
        Geometry::Circle { .. } => "Circle",
        Geometry::Path(_) => "Path",
    };
    let values = match g {
        Geometry::Circle { x, y, radius } => Some(vec![*x, *y, *radius]),
        Geometry::Rect {
            x,
            y,
            width,
            height,
        } => Some(vec![*x, *y, *width, *height]),
        Geometry::Path(_) => None,
    };
    json!({"kind":kind,"values":values,"commands":g.commands().iter().map(|c|match c{
        PathCommand::Move(p)=>json!(["M",p.x,p.y]),PathCommand::Line(p)=>json!(["L",p.x,p.y]),
        PathCommand::Cubic{first,second,end}=>json!(["C",first.x,first.y,second.x,second.y,end.x,end.y]),PathCommand::Close=>json!(["Z"])
    }).collect::<Vec<_>>()})
}
fn scene_data(s: &Scene) -> Value {
    json!({"width":s.width,"height":s.height,"background":color(s.background),
        "marks":s.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"geometry":geometry(&m.geometry),"fill":m.fill.map(color),"opacity":m.opacity,
            "stroke":m.stroke.as_ref().map(|s|json!({"color":color(s.color),"width":s.width,"cap":format!("{:?}",s.cap)})),"clip":m.clip.map(|r|[r.x,r.y,r.width,r.height])})).collect::<Vec<_>>(),
        "labels":s.labels.iter().map(|l|json!({"text":l.text,"position":[l.x,l.y],"color":color(l.color),"size":l.size,"rotate":l.rotation})).collect::<Vec<_>>(),
        "order":s.draw_items().iter().map(|i|match i{DrawItem::Mark(i)=>json!(["M",i]),DrawItem::Label(i)=>json!(["L",i])}).collect::<Vec<_>>()})
}
fn data(f: &Data) -> Value {
    json!({"computed":super::funnel_svg_layers::data_json(f.computed()),"scene":scene_data(f.scene()),"pool":format!("{:p}",f.source().as_ptr()),
    "raw":f.source().iter().map(|r|json!({"revision":r.data.revision,"note":r.data.note,"metricBits":r.data.metric.to_bits().to_string(),"local":r.data.local.get(),"address":format!("{:p}",&r.data)})).collect::<Vec<_>>()})
}
fn state(s: kartis::motion::PropertyState) -> [f64; 3] {
    [s.value, s.velocity, s.target]
}
fn motion(f: &Motion) -> Value {
    use FunnelMotionLabelChannel as L;
    use FunnelMotionMarkChannel as M;
    let dormant = (0..f.data.target().parts().len()).map(|i| {
        let mut row = serde_json::Map::new();
        for role in [FunnelMotionPartRole::Body, FunnelMotionPartRole::Border] {
            let channels = [M::FillRed,M::FillGreen,M::FillBlue,M::FillAlpha,M::StrokeRed,M::StrokeGreen,M::StrokeBlue,M::StrokeAlpha,M::StrokeWidth]
                .into_iter().filter_map(|channel| f.data.part_property(i,role,channel).unwrap().map(|s| (format!("{channel:?}"),json!(state(s)))))
                .collect::<serde_json::Map<String,Value>>();
            row.insert(format!("{role:?}"),json!({"channels":channels,"progress":f.data.part_path_progress(i,role).unwrap().map(state),
                "geometry":f.data.part_geometry_states(i,role).unwrap().map(|ss|ss.iter().copied().map(state).collect::<Vec<_>>())}));
        }
        Value::Object(row)
    }).collect::<Vec<_>>();
    json!({"generation":f.source_generation,"publication":f.publication,"configuration":f.configuration,
        "dormant":dormant,
        "target":super::funnel_svg_layers::data_json(f.data.target()),"targetScene":scene_data(f.data.target().scene()),"scene":scene_data(f.data.scene()),"settled":f.data.is_settled(),"targetSerial":f.data.target_serial(),"sampleSerial":f.data.sample_serial(),
        "roles":(0..f.data.scene().marks.len()).map(|i|match f.data.mark_role(i).unwrap(){Some(FunnelSvgMarkRole::Body{part_index})=>json!({"kind":"body","partIndex":part_index}),Some(FunnelSvgMarkRole::Border{part_index})=>json!({"kind":"border","partIndex":part_index}),Some(FunnelSvgMarkRole::Separator{separator_index})=>json!({"kind":"separator","separatorIndex":separator_index}),None=>Value::Null}).collect::<Vec<_>>(),
        "labelParts":(0..f.data.scene().labels.len()).map(|i|f.data.label_part_index(i).unwrap()).collect::<Vec<_>>(),
        "annotations":f.data.target().annotations().map(|a|json!({"baseMarks":f.data.target().annotation_source().scene().marks.len(),"baseLabels":f.data.target().annotation_source().scene().labels.len(),"order":a.scene().draw_items().iter().map(|i|match i{DrawItem::Mark(i)=>json!(["M",i]),DrawItem::Label(i)=>json!(["L",i])}).collect::<Vec<_>>(),
            "bindings":a.bindings().iter().map(|b|json!({"kind":format!("{:?}",b.kind),"id":super::funnel_svg_layers::key_json(&b.id),"specIndex":b.spec_index,"partIndex":b.part_index,"sourceIndex":b.source_index,"center":b.center,"size":b.size,"width":b.width,"height":b.height,"radius":b.border_radius,"points":b.computed.points,"text":b.computed.text})).collect::<Vec<_>>()})),
        "marks":f.data.scene().marks.iter().map(|m|json!({"id":m.id,"progress":f.data.path_progress(&m.id).map(state),"geometry":f.data.geometry_states(&m.id).map(|ss|ss.iter().copied().map(state).collect::<Vec<_>>()),
            "channels":([M::FillRed,M::FillGreen,M::FillBlue,M::FillAlpha,M::StrokeRed,M::StrokeGreen,M::StrokeBlue,M::StrokeAlpha,M::StrokeWidth].iter().filter_map(|c|f.data.mark_property(&m.id,*c).map(|s|(format!("{c:?}"),json!(state(s))))).collect::<serde_json::Map<String,Value>>())})).collect::<Vec<_>>(),
        "labels":(0..f.data.scene().labels.len()).map(|i|[L::X,L::Y,L::Red,L::Green,L::Blue,L::Alpha].iter().filter_map(|c|f.data.label_property(i,*c).map(|s|(format!("{c:?}"),json!(state(s))))).collect::<serde_json::Map<String,Value>>()).collect::<Vec<_>>()})
}
fn layer(f: &Layer) -> Value {
    json!({"data":data(&f.data),"generation":f.source_generation,"publication":f.publication,"key":f.key,"index":f.index})
}
fn tip(f: &Tip) -> Value {
    json!({"data":data(&f.data),"generation":f.source_generation,"publication":f.publication,"partIndex":f.part_index,"sourceIndex":f.source_index})
}
fn current(m: &Metrics) -> Value {
    m.owner
        .borrow()
        .as_ref()
        .map(|h| owner(h).unwrap_or_else(|e| json!({"Err":e.to_string()})))
        .unwrap_or(Value::Null)
}
fn owner(h: &Handle) -> Result<Value> {
    let s = h.snapshot()?;
    let ms = h.motion_snapshot()?;
    Ok(
        json!({"active":s.active,"data":s.data.as_ref().map(data),"generation":s.source_generation,"publication":s.publication,
        "current":s.current.as_ref().map(super::funnel_svg_layers::key_json),"pending":s.pending,"rafPending":s.raf_pending,
        "hidden":s.hidden,"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"error":s.error.map(|e|e.to_string()),
        "uiErrors":s.ui_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),"eventErrors":s.event_errors.iter().map(ToString::to_string).collect::<Vec<_>>(),
        "motion":h.motion_frame()?.as_ref().map(motion),"motionState":{"active":ms.active,"enabled":ms.enabled,"clock":format!("{:?}",ms.clock),"preference":format!("{:?}",ms.preference),"reduced":ms.reduced,"configuration":ms.configuration,"rafPending":ms.raf_pending},
        "tooltip":s.tooltip.as_ref().map(tip),"lastHit":s.last_hit.as_ref().map(|hit|json!({"data":data(&hit.data),"generation":hit.source_generation,"publication":hit.publication,"partIndex":hit.part_index,"scene":hit.scene,"client":hit.client}))}),
    )
}
fn log(m: &Metrics, kind: &str, value: Value) {
    let mut l = m.ledger.borrow_mut();
    let ordinal = l.len();
    l.push(json!({"ordinal":ordinal,"kind":kind,"value":value}));
}
fn cohort(m: &Metrics, d: Option<&Data>, p: Option<&Motion>) -> Value {
    let h = m.owner.borrow();
    let Some(h) = h.as_ref() else {
        return Value::Null;
    };
    let accepted = h.snapshot().unwrap().data;
    let paint = h.motion_frame().unwrap();
    json!({"data":d.map(|d|accepted.as_ref()==Some(d)),"pool":d.map(|d|accepted.as_ref().is_some_and(|a|a.shares_source(d))),"motion":paint.as_ref()==p,
        "target":p.map(|p|d.is_some_and(|d|d.computed()==p.data.target()))})
}
fn snapshot(f: &Fixture) -> Result<Value> {
    let m = &f.metrics;
    Ok(
        json!({"owner":owner(&f.owner.handle())?,"builds":m.builds.get(),"factories":m.factories.get(),
    "constructors":m.constructors.get(),"cleanups":m.cleanups.get(),"tipConstructors":m.tip_constructors.get(),"tipCleanups":m.tip_cleanups.get(),"peers":widgets::peers(m),
    "historical":m.historical.borrow().iter().map(|(d,p)|json!({"data":data(d),"motion":p.as_ref().map(motion)})).collect::<Vec<_>>(),"ledgerCount":m.ledger.borrow().len()}),
    )
}

#[wasm_bindgen]
pub fn funnel_motion_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 3 {
        return Err(js("unknown motion profile"));
    }
    let d = web_sys::window().unwrap().document().unwrap();
    let root = d.create_element("section")?;
    root.set_id(&format!("funnel-motion-{key}"));
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
        move |c| build(&bm, c),
    )
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(own.handle());
    let h = own.handle();
    h.set_motion_clock(BrowserFunnelMotionClock::Manual)
        .map_err(js)?;
    h.enable_motion(FunnelMotionOptions::default())
        .map_err(js)?;
    h.flush().map_err(js)?;
    widgets::hooks(&h, m.clone()).map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                owner: own,
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
pub fn funnel_motion_operation(
    key: &str,
    op: &str,
    a: f64,
    b: f64,
) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow_mut(|all|{let f=all.get_mut(key).ok_or_else(||js("motion fixture missing"))?;let h=f.owner.handle();
        let result:Result<()>=match op{
            "source"=>h.set_source(source(a as u32,b as u8)),
            "step"=>h.step_motion(a),
            "clock"=>h.set_motion_clock(if a==0.{BrowserFunnelMotionClock::Manual}else{BrowserFunnelMotionClock::Automatic}),
            "preference"=>h.set_motion_preference(match a as u8{0=>MotionPreference::System,1=>MotionPreference::Reduced,_=>MotionPreference::Animate}),
            "enable"=>h.enable_motion(FunnelMotionOptions::default()),"disable"=>h.set_motion_options(None),
            "spring"=>h.enable_motion(FunnelMotionOptions{spring:kartis::motion::SpringConfig{stiffness:a,damping:b,..Default::default()},..Default::default()}),
            "budget"=>h.enable_motion(FunnelMotionOptions{max_marks:a as usize,..Default::default()}),
            "current"=>h.set_current_part((a!=0.).then_some(&CategoryKey::from("1"))),
            "curve"=>{let mut o=options(f.profile);o.interpolation=if a==0.{FunnelInterpolation::Linear}else{FunnelInterpolation::Smooth};h.set_options(o)},
            "dormant-border"=>{let mut o=options(f.profile);o.border_width=a;o.border_opacity=b;h.set_options(o)},
            "program"=>{f.mode=a as u8;f.revision=b as u64;widgets::program(f.mode,f.revision,f.metrics.clone()).and_then(|p|h.set_svg_program(p))},
            "tooltip"=>widgets::tooltip(&h,f.metrics.clone(),a as u64),
            "view"=>h.set_view(ZoomTransform{scale:a,translate_x:b,translate_y:-b}),
            "hidden"=>h.snapshot().and_then(|snapshot|{let mut s=BTreeSet::new();if a!=0. && let Some(data)=snapshot.data && let Some(part)=data.computed().parts().first(){s.insert(part.series.clone());}h.set_hidden(s)}),
            "fault"=>{f.metrics.fault.set(a as u8);Ok(())},"refresh"=>h.refresh(),
            "callback"=>{f.metrics.callback_step.set(a!=0.);Ok(())},
            "css"=>f.root.set_attribute("style",&format!("position:relative;width:600px;height:420px;margin:24px 0 0 80px;transform:translate({b}px,0) scale({a});transform-origin:0 0")).map_err(|e|ChartError(format!("{e:?}"))),
            "visibility"=>f.root.clone().dyn_into::<web_sys::HtmlElement>().map_err(|e|ChartError(format!("{e:?}"))).and_then(|root|root.style().set_property("display",if a==0.{"none"}else{""}).map_err(|e|ChartError(format!("{e:?}")))),
            "stop"=>{h.stop();Ok(())},_=>Err(ChartError("unknown motion operation".into()))};
        let result=result.and_then(|_|if op=="step"||op=="stop"{Ok(())}else{h.flush()});
        Ok(json!({"Ok":result.is_ok(),"error":result.err().map(|e|e.to_string()),"snapshot":snapshot(f).map_err(js)?}).to_string())})
}
#[wasm_bindgen]
pub fn funnel_motion_read(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        snapshot(all.get(key).ok_or_else(|| js("missing motion fixture"))?)
            .map(|v| v.to_string())
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_motion_owner(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        owner(
            &all.get(key)
                .ok_or_else(|| js("missing motion fixture"))?
                .owner
                .handle(),
        )
        .map(|v| v.to_string())
        .map_err(js)
    })
}
#[wasm_bindgen]
pub fn funnel_motion_drain(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("missing motion fixture"))?;
        Ok(json!(f.metrics.ledger.borrow_mut().drain(..).collect::<Vec<_>>()).to_string())
    })
}
#[wasm_bindgen]
pub fn funnel_motion_drop(key: &str) {
    FIXTURES.with_borrow_mut(|all| {
        if let Some(f) = all.remove(key) {
            f.root.remove();
        }
    });
    if let Some(n) = web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(&format!("fm-canvas-{key}"))
    {
        n.remove();
    }
}

#[wasm_bindgen]
pub fn funnel_motion_canvas(key: &str) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all.get(key).ok_or_else(|| js("missing motion fixture"))?;
        let h = f.owner.handle();
        let scene = if let Some(p) = h.motion_frame().map_err(js)? {
            p.data.scene().clone()
        } else {
            h.snapshot()
                .map_err(js)?
                .data
                .ok_or_else(|| js("missing source"))?
                .scene()
                .clone()
        };
        let w = web_sys::window().unwrap();
        let d = w.document().unwrap();
        let id = format!("fm-canvas-{key}");
        let canvas = match d.get_element_by_id(&id) {
            Some(n) => n,
            None => {
                let n = d.create_element("canvas")?;
                n.set_id(&id);
                n.set_attribute(
                    "style",
                    "position:absolute;left:720px;top:24px;width:600px;height:420px",
                )?;
                d.body().unwrap().append_child(&n)?;
                n
            }
        }
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|e| js(format!("{e:?}")))?;
        kartis::render::draw_canvas(&scene, &canvas, w.device_pixel_ratio()).map_err(js)
    })
}
