//! A real application for deterministic and automatic Tree animation checks.
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserTree, BrowserTreeAnimatedWidgetContext, BrowserTreeAnimatedWidgetFrame,
    BrowserTreeHandle, BrowserTreeLinkMouseHandlers, BrowserTreeLinkTooltipOptions,
    BrowserTreeMotionClock, BrowserTreeMouseHandlers, BrowserTreeNodeMouseHandlers,
    BrowserTreeOptions, BrowserTreeSvgAnimatedLayerContext, BrowserTreeSvgBuildContext,
    BrowserTreeSvgProgram, BrowserTreeTooltipOptions, BrowserTreeWidgetOptions, MotionPreference,
    TreeMotionWidgetProps,
};
use kartis::motion::PropertyState;

use super::*;

#[derive(Default)]
struct Metrics {
    phase: Cell<u8>,
    fault: Cell<u8>,
    builds: Cell<u32>,
    factories: Cell<u32>,
    cleanups: Cell<u32>,
    renders: Cell<u32>,
    layer_factories: Cell<u32>,
    layer_cleanups: Cell<u32>,
    effects: RefCell<Vec<Value>>,
    events: RefCell<Vec<Value>>,
    owner: RefCell<Option<BrowserTreeHandle>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserTree,
    metrics: Rc<Metrics>,
}
thread_local! { static FIXTURES: RefCell<BTreeMap<String, Fixture>> = const { RefCell::new(BTreeMap::new()) }; }
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    let mut rows = vec![
        HierarchyDatum::new("r", None, -0.),
        HierarchyDatum::new("a", Some("r".into()), 3.),
        HierarchyDatum::new("b", Some("r".into()), 5.),
    ];
    if phase == 2 {
        rows.retain(|row| row.id != "b");
    }
    if phase == 3 {
        rows.iter_mut().find(|row| row.id == "a").unwrap().value = 0.;
        rows.push(HierarchyDatum::new("c", Some("a".into()), 7.));
    }
    if phase == 4 {
        rows.reverse();
    }
    rows
}
fn config(phase: u8) -> ChartConfig {
    ChartConfig {
        width: 520.,
        height: 340.,
        margin: Margin {
            left: if phase == 0 { 40. } else { 100. },
            right: 40.,
            top: 30.,
            bottom: 30.,
        },
        ..Default::default()
    }
}
fn tree_options() -> TreeOptions {
    let mut tree = TreeOptions::default();
    tree.style.links.curve = TreeLinkCurve::Straight;
    tree.style.labels.position = TreeLabelPosition::Center;
    tree
}
fn build(
    m: &Metrics,
    c: &ChartConfig,
    s: &[HierarchyDatum],
    o: &TreeOptions,
    a: &TreeActivityOptions,
    layers: Option<&[TreeSvgLayer]>,
) -> Result<TreeActivityLayout> {
    m.builds.set(m.builds.get() + 1);
    if m.fault.get() == 1 {
        return Err(ChartError("authored Tree build rejection".into()));
    }
    let phase = m.phase.get();
    let mut size = |_: &TreeNodeFunctionContext<'_>| Ok(if phase == 0 { 20. } else { 32. });
    let mut color = |_: &TreeNodeFunctionContext<'_>| {
        Ok(if phase == 0 {
            Color(30, 80, 180, 255)
        } else {
            Color(180, 100, 40, 255)
        })
    };
    let mut width = |_: &TreeLinkFunctionContext<'_>| {
        Ok(if phase == 5 {
            0.
        } else if phase == 0 {
            4.
        } else {
            8.
        })
    };
    let mut factories = TreeFactories {
        node_size: Some(&mut size),
        node_color: Some(&mut color),
        link_thickness: Some(&mut width),
        ..Default::default()
    };
    let mut layout = match layers {
        Some(layers) => tree_svg_layout_with_activity(
            c,
            s,
            o,
            layers,
            &mut factories,
            a,
            &mut TreeActivityFactories::default(),
        ),
        None => tree_layout_with_activity(
            c,
            s,
            o,
            &mut factories,
            a,
            &mut TreeActivityFactories::default(),
        ),
    }?;
    if matches!(phase, 6 | 7) {
        let index = layout.layout.scene.labels.len();
        layout.layout.scene.labels.push(Label {
            x: 260.,
            y: 18.,
            text: "Tree annotation / 树注释".into(),
            size: 14.,
            color: Color(90, 20, 120, 255),
            anchor: TextAnchor::Middle,
            rotation: 0.,
        });
        if !layout.layout.scene.draw_order.is_empty() {
            layout.layout.scene.draw_order.push(DrawItem::Label(index));
        }
        layout.layout.scene.title = "Tree title / 树标题".into();
        if phase == 7 {
            let mut order = layout.layout.scene.draw_items();
            let extra = order.pop().unwrap();
            order.insert(1, extra);
            layout.layout.scene.draw_order = order;
        }
    }
    Ok(layout)
}
fn states<const N: usize>(values: &[PropertyState; N]) -> Value {
    json!(
        values
            .iter()
            .map(|v| [v.value, v.target, v.velocity])
            .collect::<Vec<_>>()
    )
}
fn capture(frame: &TreeMotionFrame) -> Value {
    json!({"targetSerial":frame.target_serial(),"sampleSerial":frame.sample_serial(),"plot":[frame.target().plot().x,frame.target().plot().y,frame.target().plot().width,frame.target().plot().height],
      "raw":frame.target().source().iter().map(|d|json!({"id":d.id,"parent":d.parent,"bits":d.value.to_bits().to_string()})).collect::<Vec<_>>(),
      "nodes":frame.nodes().iter().map(|r|json!({"key":r.key(),"instance":r.instance(),"phase":format!("{:?}",r.phase()),"props":[r.props().x,r.props().y,r.props().diameter],"color":format!("{:?}",r.props().color),"states":states(r.states()),"dataBits":r.data().datum().value.to_bits().to_string()})).collect::<Vec<_>>(),
      "links":frame.links().iter().map(|r|json!({"key":[r.key().source_id,r.key().target_id],"instance":r.instance(),"phase":format!("{:?}",r.phase()),"source":r.props().source,"target":r.props().target,"width":r.props().thickness,"states":states(r.states())})).collect::<Vec<_>>(),
      "labels":frame.labels().iter().map(|r|json!({"key":r.key(),"instance":r.instance(),"phase":format!("{:?}",r.phase()),"props":[r.props().x,r.props().y,r.props().rotation],"states":states(r.states())})).collect::<Vec<_>>()})
}
fn widget_frame(frame: &BrowserTreeAnimatedWidgetFrame) -> Value {
    let (role, props) = match frame.animated_props {
        TreeMotionWidgetProps::Node(p) => ("node", json!([p.x, p.y, p.diameter])),
        TreeMotionWidgetProps::Link(p) => ("link", json!([p.source, p.target, p.thickness])),
        TreeMotionWidgetProps::Label(p) => ("label", json!([p.x, p.y, p.rotation])),
    };
    let data = match &frame.data {
        kartis::browser::BrowserTreeAnimatedWidgetData::Node(d) => {
            json!({"id":d.datum().id,"index":d.source_index(),"bits":d.datum().value.to_bits().to_string(),"targetPosition":d.node().position})
        }
        kartis::browser::BrowserTreeAnimatedWidgetData::Link(d) => {
            json!({"source":d.source_datum().id,"target":d.target_datum().id,"sourceIndex":d.source_index(),"targetIndex":d.target_index(),"sourceBits":d.source_datum().value.to_bits().to_string(),"targetBits":d.target_datum().value.to_bits().to_string(),"width":d.style().thickness})
        }
        kartis::browser::BrowserTreeAnimatedWidgetData::Label(d) => {
            json!({"id":d.datum().id,"index":d.source_index(),"bits":d.datum().value.to_bits().to_string(),"labelIndex":d.label_index(),"text":d.label().text,"font":format!("{:?}",d.font())})
        }
    };
    json!({"key":frame.key,"role":role,"props":props,"data":data,"phase":format!("{:?}",frame.phase),"instance":frame.instance,
        "generation":frame.source_generation,"dataGeneration":frame.data_generation,"occurrence":frame.occurrence.key,"visible":frame.visible,"interactive":frame.interactive,
        "publication":frame.occurrence.publication,"sampleSerial":frame.motion.sample_serial(),"targetSerial":frame.motion.target_serial()})
}
fn construct(m: Rc<Metrics>, context: BrowserTreeAnimatedWidgetContext) -> Result<impl Widget> {
    m.factories.set(m.factories.get() + 1);
    let ctor = m.factories.get();
    scene_initial_none(&context)?;
    if m.fault.get() == 2 {
        return Err(ChartError("authored animated component rejection".into()));
    }
    let frame = context.frame;
    let props = context.animated_props;
    Ok(glory::ui::widget(move |scope| {
        let clean = m.clone();
        scope.on_cleanup(move || clean.cleanups.set(clean.cleanups.get() + 1))?;
        let watch = m.clone();
        scope.effect(move||{
            let own=frame.get().map(|f|widget_frame(&f));
            let owner=watch.owner.borrow().as_ref().map(|h|h.motion_frame().map(|f|f.map(|f|json!({"generation":f.source_generation,"publication":f.publication,"sampleSerial":f.data.sample_serial(),"targetSerial":f.data.target_serial()}))));
            watch.effects.borrow_mut().push(json!({"ctor":ctor,"own":own,"owner":owner.as_ref().and_then(|r|r.as_ref().ok()),"error":owner.as_ref().and_then(|r|r.as_ref().err()).map(ToString::to_string)}));
        });
        let count = scope.cage(0u32);
        glory::ui::button().class("tm-widget")
            .style(scope.bond(move||{
                let (x,y,w,h,color)=match props.get() {
                    Some(TreeMotionWidgetProps::Node(p))=>(p.x-p.diameter/2.,p.y-p.diameter/2.,p.diameter,p.diameter,p.color),
                    Some(TreeMotionWidgetProps::Link(p))=>((p.source[0]+p.target[0])/2.-5.,(p.source[1]+p.target[1])/2.-3.,10.,6.,p.color),
                    Some(TreeMotionWidgetProps::Label(p))=>(p.x+20.,p.y-6.,12.,12.,Color(180,100,40,255)),
                    None=>(0.,0.,0.,0.,Color(0,0,0,0)),
                };
                Style{background:Some(color),..Style::new().position(Position::Absolute).left(Length::Px(x)).top(Length::Px(y)).width(Length::Px(w)).height(Length::Px(h))}
            }))
            .label(scope.bond(move||json!({"ctor":ctor,"count":count.get(),"frame":frame.get().map(|f|widget_frame(&f))}).to_string()))
            .on(EventKind::Activate,move|_|count.revise(|v|*v+=1)).show_in(scope)
    }))
}
fn scene_initial_none(context: &BrowserTreeAnimatedWidgetContext) -> Result<()> {
    if !context
        .frame
        .try_with_untracked(Option::is_none)
        .map_err(|e| ChartError(e.to_string()))?
    {
        return Err(ChartError(
            "animated frame was published during construction".into(),
        ));
    }
    Ok(())
}
fn program(m: Rc<Metrics>, repeat: bool) -> Result<BrowserTreeSvgProgram> {
    if !repeat {
        return Ok(BrowserTreeSvgProgram::defaults());
    }
    let mut layers = TreeSvgPlan::default_layers();
    layers.push(TreeSvgLayer::new("nodes-again", TreeSvgLayerKind::Nodes)?);
    layers.push(TreeSvgLayer::new(
        "motion-panel",
        TreeSvgLayerKind::Custom("panel".into()),
    )?);
    BrowserTreeSvgProgram::new(layers)?.custom_animated("panel",0,move|context:BrowserTreeSvgAnimatedLayerContext| {
        m.layer_factories.set(m.layer_factories.get()+1);
        let frame=context.frame; let motion=context.animated_props;
        let m=m.clone();
        Ok(glory::ui::widget(move|scope| {
            let cleaned=m.clone();scope.on_cleanup(move||cleaned.layer_cleanups.set(cleaned.layer_cleanups.get()+1))?;
            let watched=m.clone();
            scope.effect(move||{
                let own=frame.get().zip(motion.get()).map(|(f,m)|json!({"generation":f.source_generation,"publication":f.publication,"sampleSerial":m.sample_serial(),"targetSerial":m.target_serial(),"raw":f.data.source().iter().map(|d|d.id.clone()).collect::<Vec<_>>()}));
                let owner=watched.owner.borrow().as_ref().map(|h|h.motion_frame().map(|f|f.map(|f|json!({"generation":f.source_generation,"publication":f.publication,"sampleSerial":f.data.sample_serial(),"targetSerial":f.data.target_serial()}))));
                watched.effects.borrow_mut().push(json!({"kind":"layer","own":own,"owner":owner.as_ref().and_then(|r|r.as_ref().ok()),"error":owner.as_ref().and_then(|r|r.as_ref().err()).map(ToString::to_string)}));
            });
            glory::ui::button().class("tm-layer").label(scope.bond(move||frame.get().zip(motion.get()).map(|(f,m)|json!({"generation":f.source_generation,"publication":f.publication,"sampleSerial":m.sample_serial(),"targetSerial":m.target_serial()}).to_string()).unwrap_or_default())).show_in(scope)
        }))
    })
}
#[wasm_bindgen]
pub fn tree_motion_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("tree-motion-{key}"));
    root.set_attribute("style", "width:520px;height:340px;position:relative")?;
    doc.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let authored = m.clone();
    let mut options = BrowserTreeOptions {
        tree: tree_options(),
        ..Default::default()
    };
    options.chart.motion = MotionPreference::Animate;
    options.chart.renderer = BrowserRenderer::Svg;
    options.picking = kartis::browser::BrowserTreePicking::Geometry;
    let owner = if profile == 0 {
        BrowserTree::mount_with_factory_and_motion(
            root.clone(),
            raw(0),
            config(0),
            options,
            move |c| build(&authored, c.config, c.source, c.tree, c.activity, None),
            TreeMotionOptions::default(),
        )
    } else if profile == 1 {
        let factory = m.clone();
        BrowserTree::mount_with_factory_svg_layers_motion_and_animated_components(
            root.clone(),
            raw(0),
            config(0),
            options,
            BrowserTreeSvgProgram::defaults(),
            move |c: &BrowserTreeSvgBuildContext<'_>| {
                build(
                    &authored,
                    c.config,
                    c.source,
                    c.tree,
                    c.activity,
                    Some(c.layers),
                )
            },
            TreeMotionOptions::default(),
            BrowserTreeWidgetOptions::default(),
            move |c| construct(factory.clone(), c),
        )
    } else {
        let rendered = m.clone();
        BrowserTree::mount_with_motion_renderer(
            root.clone(),
            raw(0),
            config(0),
            options,
            BrowserTreeSvgProgram::defaults(),
            move |c| {
                build(
                    &authored,
                    c.config,
                    c.source,
                    c.tree,
                    c.activity,
                    Some(c.layers),
                )
            },
            TreeMotionOptions::default(),
            move |frame| {
                rendered.renders.set(rendered.renders.get() + 1);
                if rendered.fault.get() == 3 {
                    Err(ChartError("authored motion renderer rejection".into()))
                } else {
                    tree_motion_paint(frame)
                }
            },
        )
    }
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(owner.handle());
    let nodes = m.clone();
    let links = m.clone();
    owner.set_mouse_handlers(BrowserTreeMouseHandlers {
        nodes:BrowserTreeNodeMouseHandlers {on_mouse_move:Some(Box::new(move|event| {
            nodes.events.borrow_mut().push(json!({"role":"node","id":event.data.datum().id,"index":event.data.source_index(),"bits":event.data.datum().value.to_bits().to_string(),"targetPosition":event.data.node().position,"generation":event.source_generation,"trusted":event.input.event.is_trusted(),"scene":event.input.scene}));Ok(())
        })),..Default::default()},
        links:BrowserTreeLinkMouseHandlers {on_mouse_move:Some(Box::new(move|event| {
            links.events.borrow_mut().push(json!({"role":"link","source":event.data.source_datum().id,"target":event.data.target_datum().id,"width":event.data.style().thickness,"generation":event.source_generation,"trusted":event.input.event.is_trusted(),"scene":event.input.scene}));Ok(())
        })),..Default::default()},
        ..Default::default()
    }).map_err(js)?;
    owner
        .handle()
        .set_node_tooltip_options(BrowserTreeTooltipOptions::default())
        .map_err(js)?;
    owner
        .handle()
        .set_link_tooltip_options(BrowserTreeLinkTooltipOptions::default())
        .map_err(js)?;
    owner
        .handle()
        .set_motion_clock(BrowserTreeMotionClock::Manual)
        .map_err(js)?;
    owner.flush().map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                owner,
                metrics: m,
            },
        );
    });
    Ok(())
}
#[wasm_bindgen]
pub fn tree_motion_operation(
    key: &str,
    operation: &str,
    value: f64,
) -> std::result::Result<(), JsValue> {
    FIXTURES.with_borrow(|all| {
        let f = all
            .get(key)
            .ok_or_else(|| js("missing Tree motion fixture"))?;
        let h = f.owner.handle();
        match operation {
            "phase" => {
                let phase = value as u8;
                f.metrics.phase.set(phase);
                h.set_source(raw(phase)).map_err(js)?;
                h.set_config(config(phase)).map_err(js)?;
                h.flush().map_err(js)
            }
            "tick" => h.advance_motion(value).map_err(js),
            "fault" => {
                f.metrics.fault.set(value as u8);
                Ok(())
            }
            "refresh" => {
                h.refresh().map_err(js)?;
                h.flush().map_err(js)
            }
            "disable" => {
                h.set_motion_options(None).map_err(js)?;
                h.flush().map_err(js)
            }
            "enable" => {
                h.set_motion_options(Some(TreeMotionOptions::default()))
                    .map_err(js)?;
                h.flush().map_err(js)
            }
            "enable_phase" => {
                h.set_motion_options(Some(TreeMotionOptions::default()))
                    .map_err(js)?;
                let phase = value as u8;
                f.metrics.phase.set(phase);
                h.set_source(raw(phase)).map_err(js)?;
                h.set_config(config(phase)).map_err(js)?;
                h.flush().map_err(js)
            }
            "program" => {
                h.set_svg_program(program(f.metrics.clone(), value != 0.).map_err(js)?)
                    .map_err(js)?;
                h.flush().map_err(js)
            }
            "zoom" => {
                h.dispatch(SurfaceInput::Reset).map_err(js)?;h.flush().map_err(js)?;
                if value!=0. {h.dispatch(SurfaceInput::Wheel{point:(260.,180.),delta:-50.}).map_err(js)?;}
                h.flush().map_err(js)
            },
            "css" => {f.root.set_attribute("style",if value!=0.{"width:520px;height:340px;position:relative;transform-origin:0 0;transform:scale(1.25,.8)"}else{"width:520px;height:340px;position:relative"})?;h.flush().map_err(js)},
            "hidden" => {let mut hidden=std::collections::BTreeSet::new();if value!=0.{hidden.insert("b".into());}h.set_hidden(hidden).map_err(js)?;h.flush().map_err(js)},
            "config" => {
                let mut next = TreeMotionOptions::default();
                next.spring.stiffness = value;
                h.set_motion_options(Some(next)).map_err(js)?;
                h.flush().map_err(js)
            }
            "automatic" => h
                .set_motion_clock(BrowserTreeMotionClock::Automatic)
                .map_err(js),
            "flush" => h.flush().map_err(js),
            "manual" => h
                .set_motion_clock(BrowserTreeMotionClock::Manual)
                .map_err(js),
            "stop" => {
                f.owner.stop();
                Ok(())
            }
            _ => Err(js("bad Tree motion operation")),
        }
    })
}
#[wasm_bindgen]
pub fn tree_motion_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with_borrow(|all|{
        let f=all.get(key).ok_or_else(||js("missing Tree motion fixture"))?;
        let state=f.owner.snapshot().map_err(js)?;
        let motion=f.owner.motion_frame();
        let node_tip=f.owner.handle().node_tooltip().map_err(js)?;
        let link_tip=f.owner.handle().link_tooltip().map_err(js)?;
        Ok(json!({"active":state.active,"rafPending":state.raf_pending,"pending":state.pending,"view":state.chart.as_ref().map(|s|[s.view.scale,s.view.translate_x,s.view.translate_y]),
            "error":state.error.map(|e|e.to_string()),"motion":motion.as_ref().ok().and_then(|f|f.as_ref()).map(|f|json!({"generation":f.source_generation,"publication":f.publication,"configuration":f.configuration,"frame":capture(&f.data)})),
            "motionError":motion.err().map(|e|e.to_string()),
            "nodeTooltip":node_tip.map(|t|json!({"id":t.data.datum().id,"targetPosition":t.data.node().position,"generation":t.source_generation,"anchor":t.viewport_anchor})),
            "linkTooltip":link_tip.map(|t|json!({"source":t.data.link_data().source_datum().id,"target":t.data.link_data().target_datum().id,"width":t.data.link_data().style().thickness,"generation":t.source_generation,"anchor":t.viewport_anchor})),
            "builds":f.metrics.builds.get(),"factories":f.metrics.factories.get(),"renders":f.metrics.renders.get(),"cleanups":f.metrics.cleanups.get(),"layerFactories":f.metrics.layer_factories.get(),"layerCleanups":f.metrics.layer_cleanups.get(),"effects":*f.metrics.effects.borrow(),"events":*f.metrics.events.borrow(),"connected":f.root.is_connected()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_motion_drop(key: &str) -> bool {
    FIXTURES.with_borrow_mut(|all| {
        all.remove(key)
            .map(|f| {
                f.root.remove();
                true
            })
            .unwrap_or(false)
    })
}
