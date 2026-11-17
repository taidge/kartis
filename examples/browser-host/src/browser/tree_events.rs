//! Real native typed Tree mouse-hook acceptance application.
use glory::Widget;
use glory::render::EventKind;
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeCanvasProgram,
    BrowserTreeDoubleClickBehavior, BrowserTreeHandle, BrowserTreeLinkMouseEvent,
    BrowserTreeLinkMouseHandlers, BrowserTreeMouseBinding, BrowserTreeMouseHandlers,
    BrowserTreeMouseInput, BrowserTreeNodeMouseActivity, BrowserTreeNodeMouseEvent,
    BrowserTreeNodeMouseHandlers, BrowserTreeOptions, BrowserTreePicking,
    BrowserTreeSvgBuildContext, BrowserTreeSvgProgram, BrowserTreeTooltipOptions,
    BrowserTreeWidgetOptions, MotionPreference, TreeTooltipFrame,
};

use super::*;

type SourceBond = glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>;
struct Row {
    ctor: u32,
    key: String,
    frame: SourceBond,
}
#[derive(Default)]
struct Metrics {
    phase: Cell<u8>,
    mode: Cell<u8>,
    fault: Cell<u8>,
    action: Cell<u8>,
    handler_revision: Cell<u32>,
    builds: Cell<u32>,
    functions: Cell<u32>,
    placements: Cell<u32>,
    factories: Cell<u32>,
    raw_calls: Cell<u32>,
    rejected_program_factories: Cell<u32>,
    rejected_program_inputs: RefCell<Vec<Value>>,
    cleanups: Cell<u32>,
    owner: RefCell<Option<BrowserTreeHandle>>,
    rows: RefCell<Vec<Row>>,
    hooks: RefCell<Vec<Value>>,
    returns: RefCell<Vec<Value>>,
    actions: RefCell<Vec<Value>>,
    errors: RefCell<Vec<String>>,
    updates: Cell<u32>,
}
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    m: Rc<Metrics>,
    profile: u8,
    bindings: RefCell<Vec<BrowserTreeMouseBinding>>,
    nested: RefCell<Option<BrowserTree>>,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Rc<Fixture>>>=const{RefCell::new(BTreeMap::new())};}
fn fixture(key: &str) -> std::result::Result<Rc<Fixture>, JsValue> {
    FIXTURES.with_borrow(|all| {
        all.get(key)
            .cloned()
            .ok_or_else(|| js("missing Tree events fixture"))
    })
}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    let mut rows = [
        ("a", Some("r"), 0.),
        ("c", Some("a"), if phase == 1 { 35. } else { 30. }),
        ("r", None, 0.),
        ("b", Some("r"), if phase == 4 { 0. } else { -0. }),
        ("d", Some("a"), 10.),
    ]
    .into_iter()
    .map(|(id, parent, value)| {
        let mut d = HierarchyDatum::new(id, parent.map(str::to_owned), value);
        d.label = if phase == 3 && id == "b" {
            String::new()
        } else {
            format!("source-{id}")
        };
        d
    })
    .collect::<Vec<_>>();
    if phase == 2 {
        rows.reverse();
    }
    rows
}
fn config() -> ChartConfig {
    let mut c = ChartConfig {
        width: 480.,
        height: 320.,
        margin: Margin {
            left: 31.,
            top: 23.,
            right: 47.,
            bottom: 37.,
        },
        ..Default::default()
    };
    c.theme.colors = vec![
        Color(100, 20, 180, 255),
        Color(0, 180, 200, 255),
        Color(255, 220, 0, 255),
    ];
    c
}
fn tree_options(phase: u8) -> TreeOptions {
    let mut o = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    o.style.links.enabled = phase != 5;
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.width = 6.;
    o.style.links.color = TreeLinkColor::Fixed(Color(230, 110, 20, 255));
    o.style.labels.position = TreeLabelPosition::Center;
    o.style.labels.size = Some(12.);
    o
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn datum(d: &HierarchyDatum) -> Value {
    json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value,"bits":d.value.to_bits().to_string()})
}
fn mark(m: &Mark) -> Value {
    json!({"id":m.id,"geometry":format!("{:?}",m.geometry),"label":m.label,"value":m.value,"bits":m.value.to_bits().to_string(),"fill":m.fill.map(rgba),"opacity":m.opacity})
}
fn frame(f: &TreeEventFrame) -> Value {
    let a = f.activity();
    json!({"config":[f.config().width,f.config().height],"margin":[f.config().margin.left,f.config().margin.top,f.config().margin.right,f.config().margin.bottom],"plot":[f.plot().x,f.plot().y,f.plot().width,f.plot().height],"raw":f.source().iter().map(datum).collect::<Vec<_>>(),"pool":format!("{:p}",f.source().as_ptr()),"nodes":f.nodes().iter().enumerate().map(|(i,n)|json!({"index":i,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"value":n.value,"rawBits":f.source()[i].value.to_bits().to_string(),"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":f.links().iter().enumerate().map(|(i,l)|json!({"index":i,"source":l.source_index,"target":l.target_index,"sourcePosition":f.nodes()[l.source_index].position,"targetPosition":f.nodes()[l.target_index].position,"position":l.position,"markId":l.mark_id,"width":f.computed_link_styles()[i].thickness,"color":rgba(f.computed_link_styles()[i].color)})).collect::<Vec<_>>(),"labels":f.scene().labels.iter().map(|l|json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"rotation":l.rotation,"anchor":format!("{:?}",l.anchor),"color":rgba(l.color)})).collect::<Vec<_>>(),"activity":a.map(|a|json!({"focus":a.focus_id,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links]})),"theme":{"background":rgba(f.theme().background),"text":rgba(f.theme().text),"colors":f.theme().colors.iter().copied().map(rgba).collect::<Vec<_>>(),"fontSize":f.theme().font_size,"font":format!("{:?}",f.theme().font)},"marks":f.scene().marks.iter().map(mark).collect::<Vec<_>>()})
}
fn element(e: Option<web_sys::EventTarget>) -> Value {
    e.and_then(|v|v.dyn_into::<web_sys::Element>().ok()).map(|e|json!({"tag":e.tag_name(),"namespace":e.namespace_uri(),"class":e.get_attribute("class"),"id":e.id(),"key":e.closest("[data-tree-svg-key]").ok().flatten().and_then(|e|e.get_attribute("data-tree-svg-key")),"mesh":e.closest("[data-tree-svg-mesh]").ok().flatten().is_some()})).unwrap_or(Value::Null)
}
fn input(i: &BrowserTreeMouseInput) -> Result<Value> {
    let window =
        web_sys::window().ok_or_else(|| ChartError("event fixture window missing".into()))?;
    let lookup =
        web_sys::js_sys::Reflect::get(&window, &JsValue::from_str("__treeEventsNativeSequence"))
            .map_err(|e| ChartError(format!("native event identity lookup: {e:?}")))?
            .dyn_into::<web_sys::js_sys::Function>()
            .map_err(|_| ChartError("native event identity function missing".into()))?;
    let sequence = lookup
        .call1(&JsValue::UNDEFINED, i.event.as_ref())
        .map_err(|e| ChartError(format!("native event identity lookup failed: {e:?}")))?
        .as_f64()
        .ok_or_else(|| ChartError("native event sequence is not numeric".into()))?;
    Ok(
        json!({"nativeSequence":sequence,"eventPhase":i.event.event_phase(),"currentTargetIsWindow":i.event.current_target().is_some_and(|target|JsValue::from(target)==JsValue::from(window.clone())),"kind":format!("{:?}",i.kind),"nativeType":i.event.type_(),"trusted":i.event.is_trusted(),"timestamp":i.event.time_stamp(),"button":i.event.button(),"buttons":i.event.buttons(),"detail":i.event.detail(),"modifiers":[i.event.alt_key(),i.event.ctrl_key(),i.event.meta_key(),i.event.shift_key()],"client":i.client,"viewport":i.viewport,"scene":i.scene,"pointer":i.pointer.as_ref().map(|p|json!({"id":p.id,"kind":p.kind,"primary":p.primary,"captured":p.captured})),"origin":format!("{:?}",i.origin),"occurrence":i.occurrence.as_ref().map(|o|json!({"key":o.key,"index":o.index,"publication":o.publication})),"target":element(i.event.target()),"related":element(i.event.related_target())}),
    )
}
fn source_data(d: &TreeActivityComponentData) -> Value {
    let (source, nodes, links, labels, role, index) = match &d.component {
        TreeComponentData::Node(v) => (
            &v.source,
            &v.nodes,
            &v.links,
            &v.labels,
            "node",
            v.source_index,
        ),
        TreeComponentData::Link(v) => (
            &v.source,
            &v.nodes,
            &v.links,
            &v.labels,
            "link",
            v.link_index,
        ),
        TreeComponentData::Label(v) => (
            &v.source,
            &v.nodes,
            &v.links,
            &v.labels,
            "label",
            v.source_index,
        ),
    };
    json!({"role":role,"index":index,"raw":source.iter().map(datum).collect::<Vec<_>>(),"nodes":nodes.iter().enumerate().map(|(i,n)|json!({"index":i,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"value":n.value,"rawBits":n.raw_value.to_bits().to_string(),"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":links.iter().map(|l|json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"position":l.position,"markId":l.mark_id})).collect::<Vec<_>>(),"labels":labels.labels.iter().map(|l|json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"rotation":l.rotation,"anchor":format!("{:?}",l.anchor),"color":rgba(l.color)})).collect::<Vec<_>>(),"activity":{"focus":d.activity.focus_id,"nodes":d.activity.nodes,"links":d.activity.links,"flags":[d.activity.highlight.ancestor_nodes,d.activity.highlight.descendant_nodes,d.activity.highlight.ancestor_links,d.activity.highlight.descendant_links]},"active":d.is_active})
}
fn source_frame(v: &TypedSceneWidgetFrame<TreeActivityComponentData>) -> Value {
    let r = v.frame.rect();
    json!({"rect":[r.x,r.y,r.width,r.height],"visible":v.frame.visible().map(|r|[r.x,r.y,r.width,r.height]),"data":source_data(&v.data)})
}
fn source_rows(m: &Metrics) -> Vec<Value> {
    m.rows.borrow().iter().map(|r|{let read=r.frame.try_with_untracked(source_frame);json!({"ctor":r.ctor,"key":r.key,"alive":r.frame.is_alive(),"value":read.as_ref().ok(),"error":read.as_ref().err().map(|e|format!("{e:?}"))})}).collect()
}
fn tooltip_frame(t: &TreeTooltipFrame) -> Result<Value> {
    let data =
        TreeEventFrame::with_activity(t.data.config().clone(), t.data.activity_layout().clone())?;
    Ok(
        json!({"id":t.data.datum().id,"index":t.data.source_index(),"datum":datum(t.data.datum()),"value":t.data.node().value,"active":t.data.is_active(),"sourceGeneration":t.source_generation,"pointerClient":t.pointer_client,"viewportAnchor":t.viewport_anchor,"frame":frame(&data)}),
    )
}
fn owner_data(h: &BrowserTreeHandle) -> Result<Value> {
    let s = h.snapshot()?;
    let data = TreeEventFrame::with_activity(s.config.clone(), s.layout.clone())?;
    let tooltip = h.node_tooltip()?.as_ref().map(tooltip_frame).transpose()?;
    let generation = h.source_generation();
    if s.active {
        generation.as_ref().map_err(Clone::clone)?;
    }
    let mut layer = None;
    let mut layers = BTreeMap::new();
    let mut layer_errors = BTreeMap::new();
    for key in ["nodes", "links", "labels", "mesh", "nodes-again", "panel"] {
        match h.svg_layer_frame(key) {
            Ok(Some(l)) => {
                let certificate = json!({"key":l.key,"index":l.index,"sourceGeneration":l.source_generation,"publication":l.publication,"view":[l.view.scale,l.view.translate_x,l.view.translate_y]});
                if layer.is_none() {
                    let source =
                        TreeEventFrame::new(l.data.config().clone(), l.data.layout().clone())?;
                    let mut data = frame(&source);
                    data["activity"]=l.data.activity().map(|a|json!({"focus":a.focus_id,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links]})).unwrap_or(Value::Null);
                    let mut full = certificate.clone();
                    full["frame"] = data;
                    layer = Some(full);
                }
                layers.insert(key, Some(certificate));
                layer_errors.insert(key, None::<String>);
            }
            Ok(None) => {
                layers.insert(key, None);
                layer_errors.insert(key, None);
            }
            Err(error) => {
                if s.active {
                    return Err(error);
                }
                layers.insert(key, None);
                layer_errors.insert(key, Some(error.to_string()));
            }
        }
    }
    Ok(
        json!({"data":frame(&data),"sourceGeneration":generation.as_ref().ok(),"sourceGenerationError":generation.as_ref().err().map(|e|e.to_string()),"publicLayer":layer,"publicLayers":layers,"publicLayerErrors":layer_errors,"tooltip":tooltip,"tooltipGetterPhase":"actual accepted public h.node_tooltip during snapshot","active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"desiredFocus":s.desired_focus,"error":s.error.map(|e|e.to_string()),"revision":s.widget_revision,"chart":s.chart.map(|s|json!({"view":[s.view.scale,s.view.translate_x,s.view.translate_y],"selected":s.selected,"hovered":s.hovered,"revision":s.revision}))}),
    )
}
fn action(m: &Rc<Metrics>) -> Result<()> {
    let action = m.action.replace(0);
    if action == 0 {
        return Ok(());
    }
    let h = m
        .owner
        .borrow()
        .clone()
        .ok_or_else(|| ChartError("event fixture owner absent".into()))?;
    let before = owner_data(&h)?;
    let outcome = match action {
        1 => Err(ChartError("tree hook application rejected".into())),
        2 => {
            m.phase.set(1);
            h.set_source(raw(1)).and_then(|_| h.flush())
        }
        3 => {
            m.fault.set(1);
            m.phase.set(1);
            h.set_source(raw(1)).and_then(|_| h.flush())
        }
        4 => {
            h.stop();
            Ok(())
        }
        5 => {
            m.handler_revision.set(2);
            h.set_mouse_handlers(handlers(m.clone(), 2, false, false))
        }
        6 => h.flush(),
        7 => h.clear_mouse_handlers(),
        _ => Err(ChartError("unknown tree hook action".into())),
    };
    let after = owner_data(&h);
    m.actions.borrow_mut().push(json!({"action":action,"before":before,"ok":outcome.is_ok(),"error":outcome.as_ref().err().map(|e|e.to_string()),"after":after.as_ref().ok(),"afterError":after.err().map(|e|e.to_string())}));
    outcome
}
fn record_node(m: &Rc<Metrics>, revision: u32, e: BrowserTreeNodeMouseEvent) -> Result<()> {
    let data = &e.data;
    let h = m.owner.borrow().clone();
    let observed = h.as_ref().map(owner_data).transpose()?;
    m.hooks.borrow_mut().push(json!({"family":"node","registration":revision,"sourceGeneration":e.source_generation,"input":input(&e.input)?,"frame":frame(data.frame()),"index":data.source_index(),"datum":datum(data.datum()),"active":data.is_active(),"mark":data.mark().map(mark),"owner":observed,"sourceRows":source_rows(m)}));
    let result = action(m);
    let after = h.as_ref().map(owner_data).transpose();
    m.returns.borrow_mut().push(json!({"family":"node","registration":revision,"nativeSequence":input(&e.input)?["nativeSequence"],"kind":format!("{:?}",e.input.kind),"eventPhase":e.input.event.event_phase(),"eventSourceGeneration":e.source_generation,"eventFrameAfter":frame(data.frame()),"ok":result.is_ok(),"error":result.as_ref().err().map(|e|e.to_string()),"ownerAfter":after.as_ref().ok(),"readError":after.as_ref().err().map(|e|e.to_string())}));
    let _ = after?;
    result
}
fn record_link(m: &Rc<Metrics>, revision: u32, e: BrowserTreeLinkMouseEvent) -> Result<()> {
    let data = &e.data;
    let h = m.owner.borrow().clone();
    let observed = h.as_ref().map(owner_data).transpose()?;
    m.hooks.borrow_mut().push(json!({"family":"link","registration":revision,"sourceGeneration":e.source_generation,"input":input(&e.input)?,"frame":frame(data.frame()),"index":data.link_index(),"sourceIndex":data.source_index(),"targetIndex":data.target_index(),"sourceDatum":datum(data.source_datum()),"targetDatum":datum(data.target_datum()),"active":data.is_active(),"style":{"width":data.style().thickness,"color":rgba(data.style().color)},"mark":data.mark().map(mark),"owner":observed,"sourceRows":source_rows(m)}));
    let result = action(m);
    let after = h.as_ref().map(owner_data).transpose();
    m.returns.borrow_mut().push(json!({"family":"link","registration":revision,"nativeSequence":input(&e.input)?["nativeSequence"],"kind":format!("{:?}",e.input.kind),"eventPhase":e.input.event.event_phase(),"eventSourceGeneration":e.source_generation,"eventFrameAfter":frame(data.frame()),"ok":result.is_ok(),"error":result.as_ref().err().map(|e|e.to_string()),"ownerAfter":after.as_ref().ok(),"readError":after.as_ref().err().map(|e|e.to_string())}));
    let _ = after?;
    result
}
fn handlers(
    m: Rc<Metrics>,
    revision: u32,
    links: bool,
    callbacks_only: bool,
) -> BrowserTreeMouseHandlers {
    let mut n = BrowserTreeNodeMouseHandlers::default();
    let mut l = BrowserTreeLinkMouseHandlers::default();
    macro_rules! install {
        ($field:ident) => {{
            let node = m.clone();
            n.$field = Some(Box::new(move |e| record_node(&node, revision, e)));
            if links {
                let link = m.clone();
                l.$field = Some(Box::new(move |e| record_link(&link, revision, e)));
            }
        }};
    }
    install!(on_mouse_enter);
    install!(on_mouse_move);
    install!(on_mouse_leave);
    install!(on_mouse_down);
    install!(on_mouse_up);
    install!(on_click);
    install!(on_double_click);
    BrowserTreeMouseHandlers {
        nodes: n,
        links: l,
        double_click: if callbacks_only {
            BrowserTreeDoubleClickBehavior::CallbacksOnlyOnHandledTarget
        } else {
            BrowserTreeDoubleClickBehavior::KeepChartReset
        },
        ..Default::default()
    }
}
fn build(
    m: &Rc<Metrics>,
    c: &ChartConfig,
    source: &[HierarchyDatum],
    o: &TreeOptions,
    a: &TreeActivityOptions,
    layers: Option<&[TreeSvgLayer]>,
    canvas: bool,
) -> Result<TreeActivityLayout> {
    m.builds.set(m.builds.get() + 1);
    if m.fault.get() == 1 {
        return Err(ChartError("tree event builder rejected".into()));
    }
    let phase = m.phase.get();
    let count = m.clone();
    let mut size = move |c: &TreeNodeFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(if phase == 3 && c.datum.id == "c" {
            0.
        } else {
            24. + 4. * c.depth as f64
        })
    };
    let count = m.clone();
    let mut color = move |c: &TreeNodeFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(match c.datum.id.as_str() {
            "r" => Color(160, 80, 30, 255),
            "a" => Color(0, 100, 255, 255),
            "b" => Color(0, 200, 0, 255),
            "c" => Color(230, 110, 20, 255),
            _ => Color(100, 20, 180, 255),
        })
    };
    let count = m.clone();
    let mut thickness = move |_: &TreeLinkFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(6.)
    };
    let count = m.clone();
    let mut link_color = move |_: &TreeLinkFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(Color(230, 110, 20, 255))
    };
    let count = m.clone();
    let mut label = move |c: &TreeLabelFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(format!("paint-{}", c.datum.id))
    };
    let mut base = TreeFactories {
        node_size: Some(&mut size),
        node_color: Some(&mut color),
        link_thickness: Some(&mut thickness),
        link_color: Some(&mut link_color),
        label: Some(&mut label),
    };
    let mut modifiers = TreeActivityFactories::default();
    let mut result = if let Some(layers) = layers {
        tree_svg_layout_with_activity(c, source, o, layers, &mut base, a, &mut modifiers)
    } else if canvas {
        tree_canvas_layout_with_activity(c, source, o, &mut base, a, &mut modifiers)
    } else {
        tree_layout_with_activity(c, source, o, &mut base, a, &mut modifiers)
    }?;
    if phase == 3 {
        for mark in &mut result.layout.scene.marks {
            if mark.id == "node:a" {
                mark.opacity = 0.;
            }
        }
    }
    Ok(result)
}
fn specs(mode: u8) -> Result<Vec<TreeSvgLayer>> {
    use TreeSvgLayerKind as K;
    let mut rows = vec![
        ("links", K::Links),
        ("nodes", K::Nodes),
        ("labels", K::Labels),
    ];
    match mode {
        1 => rows.insert(2, ("nodes-again", K::Nodes)),
        2 => rows.retain(|(_, k)| !matches!(k, K::Links)),
        3 => {
            rows.push(("mesh", K::Mesh));
            rows.push(("panel", K::Custom("panel".into())));
        }
        4 => {
            rows.push(("panel", K::Custom("panel".into())));
            rows.push(("mesh", K::Mesh));
        }
        5 => rows.retain(|(_, k)| !matches!(k, K::Nodes)),
        7 => {}
        _ => rows.push(("mesh", K::Mesh)),
    }
    rows.into_iter()
        .map(|(key, kind)| TreeSvgLayer::new(key, kind))
        .collect()
}
fn program(mode: u8) -> Result<BrowserTreeSvgProgram> {
    BrowserTreeSvgProgram::new(specs(mode)?)?
        .custom("panel", 0, move |_| {
            Ok(glory::ui::button()
                .class("te-custom-button")
                .label("independent custom UI")
                .style(Style {
                    background: Some(Color(100, 20, 180, 255)),
                    ..Style::new()
                        .position(Position::Absolute)
                        .left(Length::Px(354.))
                        .top(Length::Px(46.4))
                        .width(Length::Px(48.))
                        .height(Length::Px(40.))
                }))
        })
        .map(|p| p.interactive(mode != 6))
}
fn placement(
    m: Rc<Metrics>,
) -> impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    move |c| {
        m.placements.set(m.placements.get() + 1);
        Ok(match c.component {
            TreeComponentContext::Node(n) if n.datum.id == "b" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [24., 24.],
                    measured: TextSize {
                        width: 64.,
                        height: 24.,
                    },
                })
            }
            TreeComponentContext::Link(l) if l.layout.source()[l.link.target_index].id == "c" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-32., -24.],
                    measured: TextSize {
                        width: 64.,
                        height: 24.,
                    },
                })
            }
            TreeComponentContext::Label(l) if l.datum.id == "d" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-24., 20.],
                    measured: TextSize {
                        width: 48.,
                        height: 20.,
                    },
                })
            }
            _ => None,
        })
    }
}
fn source_factory(
    m: Rc<Metrics>,
) -> impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<Box<dyn Widget>>
{
    move |c| {
        m.factories.set(m.factories.get() + 1);
        let ctor = m.factories.get();
        let key = c.key;
        let frame = c.frame;
        let m = m.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.rows.borrow_mut().push(Row {
                ctor,
                key: key.clone(),
                frame,
            });
            let clean = m.clone();
            scope.on_cleanup(move || clean.cleanups.set(clean.cleanups.get() + 1))?;
            let count = scope.cage(0u32);
            let label_key = key.clone();
            glory::ui::button().class(format!("te-source-button te-source-{ctor}")).style(Style{background:Some(Color(100,20,180,255)),..Style::new().width(Length::Percent(100.)).height(Length::Percent(100.))}).label(scope.bond(move||json!({"ctor":ctor,"key":label_key,"count":count.get(),"active":frame.with(|f|f.data.is_active)}).to_string())).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).show_in(scope)
        })) as Box<dyn Widget>)
    }
}
#[wasm_bindgen]
pub fn tree_events_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if key.is_empty()
        || key.len() > 32
        || !key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
        || profile > 8
    {
        return Err(js("invalid Tree event fixture profile"));
    }
    tree_events_stop(key)?;
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("section")?;
    root.set_id(&format!("tree-events-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:480px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = doc.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-events-{key} .te-source-button,#tree-events-{key} .te-custom-button{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;overflow:hidden;min-width:0;box-shadow:none;pointer-events:auto!important}}")));
    doc.body().unwrap().append_child(&css)?;
    doc.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let canvas = (4..=6).contains(&profile);
    let nearest = matches!(profile, 1 | 3 | 5 | 6);
    let options = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer: if canvas {
                BrowserRenderer::Canvas
            } else {
                BrowserRenderer::Svg
            },
            legend: false,
            tooltip: profile == 8,
            highlight: BrowserHighlight::Controlled,
            motion: MotionPreference::Reduced,
            ..Default::default()
        },
        tree: tree_options(0),
        picking: if nearest {
            BrowserTreePicking::Nearest { distance: 64. }
        } else {
            BrowserTreePicking::Geometry
        },
        activity: TreeActivityOptions {
            highlight: TreeRelationOptions {
                ancestor_nodes: true,
                ancestor_links: true,
                ..Default::default()
            },
            ..Default::default()
        },
    };
    let built = m.clone();
    let normal_build = move |c: &BrowserTreeBuildContext<'_>| {
        build(
            &built,
            c.config,
            c.source,
            c.tree,
            c.activity,
            None,
            profile == 6,
        )
    };
    let built = m.clone();
    let svg_build = move |c: &BrowserTreeSvgBuildContext<'_>| {
        build(
            &built,
            c.config,
            c.source,
            c.tree,
            c.activity,
            Some(c.layers),
            false,
        )
    };
    let mounted = match profile {
        2 | 3 => BrowserTree::mount_with_factory_and_svg_layers(
            root.clone(),
            raw(0),
            config(),
            options,
            program(0).map_err(js)?,
            svg_build,
        ),
        7 => BrowserTree::mount_with_factory_and_svg_layers_and_components(
            root.clone(),
            raw(0),
            config(),
            options,
            program(0).map_err(js)?,
            svg_build,
            BrowserTreeWidgetOptions {
                source_ordered: true,
                revision: 0,
                replace_ink: false,
            },
            placement(m.clone()),
            source_factory(m.clone()),
        ),
        6 => {
            let mut p = BrowserTreeCanvasProgram::default();
            let count = m.clone();
            p.custom_layers.insert(
                "probe".into(),
                Box::new(move |ctx, _| {
                    count.raw_calls.set(count.raw_calls.get() + 1);
                    ctx.set_fill_style_str("rgb(0,180,200)");
                    ctx.fill_rect(120., 230., 20., 12.);
                    Ok(())
                }),
            );
            p.layers.push(TreeCanvasLayer::Custom("probe".into()));
            BrowserTree::mount_with_factory_and_canvas(
                root.clone(),
                raw(0),
                config(),
                options,
                normal_build,
                p,
            )
        }
        _ => BrowserTree::mount_with_factory(root.clone(), raw(0), config(), options, normal_build),
    };
    let owner = match mounted {
        Ok(o) => o,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    *m.owner.borrow_mut() = Some(owner.handle());
    let errors = m.clone();
    owner.on_event(move |e| errors.errors.borrow_mut().push(format!("{e:?}")));
    let updates = m.clone();
    owner.on_update(move |_| updates.updates.set(updates.updates.get() + 1));
    let f = Rc::new(Fixture {
        root,
        css,
        owner,
        m,
        profile,
        bindings: RefCell::new(vec![]),
        nested: RefCell::new(None),
    });
    if profile == 8 {
        f.owner
            .handle()
            .set_node_tooltip_options(BrowserTreeTooltipOptions::default())
            .map_err(js)?;
    }
    f.owner.flush().map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(key.to_owned(), f);
    });
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_handlers(
    key: &str,
    revision: u32,
    links: bool,
    callbacks_only: bool,
) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.owner
        .handle()
        .set_mouse_handlers(handlers(f.m.clone(), revision, links, callbacks_only))
        .map_err(js)?;
    f.m.handler_revision.set(revision);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_fidelity(key: &str, native_component: bool) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let mut h = handlers(
        f.m.clone(),
        f.m.handler_revision.get(),
        !(4..=6).contains(&f.profile),
        false,
    );
    h.node_activity = if native_component {
        BrowserTreeNodeMouseActivity::NativeComponent
    } else {
        BrowserTreeNodeMouseActivity::Owner
    };
    f.owner.handle().set_mouse_handlers(h).map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_current(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    let h = fixture(key)?.owner.handle();
    h.set_activity_options(TreeActivityOptions {
        focus_id: (!id.is_empty()).then(|| id.to_owned()),
        highlight: TreeRelationOptions {
            ancestor_nodes: true,
            ancestor_links: true,
            ..Default::default()
        },
        ..Default::default()
    })
    .map_err(js)?;
    h.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_clear(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?
        .owner
        .handle()
        .clear_mouse_handlers()
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_action(key: &str, action: u8) -> std::result::Result<(), JsValue> {
    if action > 7 {
        return Err(js("invalid tree event action"));
    }
    fixture(key)?.m.action.set(action);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 5 {
        return Err(js("invalid tree event source phase"));
    }
    let f = fixture(key)?;
    f.m.phase.set(phase);
    let h = f.owner.handle();
    h.set_tree_options(tree_options(phase)).map_err(js)?;
    h.set_source(raw(phase)).map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_fault(key: &str, value: bool) -> std::result::Result<(), JsValue> {
    fixture(key)?.m.fault.set(u8::from(value));
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_program(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let p = program(mode).map_err(js)?;
    f.owner.handle().set_svg_program(p).map_err(js)?;
    f.m.mode.set(mode);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_reject_program(key: &str) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let calls = f.m.clone();
    let candidate=BrowserTreeSvgProgram::new(vec![TreeSvgLayer::new("links",TreeSvgLayerKind::Links).map_err(js)?,TreeSvgLayer::new("nodes",TreeSvgLayerKind::Nodes).map_err(js)?,TreeSvgLayer::new("labels",TreeSvgLayerKind::Labels).map_err(js)?,TreeSvgLayer::new("reject",TreeSvgLayerKind::Custom("reject".into())).map_err(js)?]).map_err(js)?.custom("reject",991,move|context|->Result<Box<dyn Widget>>{
        calls.rejected_program_factories.set(calls.rejected_program_factories.get()+1);
        let initial_none=context.frame.try_with_untracked(Option::is_none).map_err(|error|ChartError(format!("rejected program constructor frame: {error:?}")))?;
        calls.rejected_program_inputs.borrow_mut().push(json!({"key":context.key,"revision":context.revision,"index":context.initial.index,"sourceGeneration":context.initial.source_generation,"publication":context.initial.publication,"initialNone":initial_none}));
        Err(ChartError("tree hook program factory rejected".into()))
    }).map_err(js)?;
    // This is a desired candidate, not the fixture's accepted authored mode.
    f.owner.handle().set_svg_program(candidate).map_err(js)?;
    f.owner.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_bind(key: &str, kind: &str, ctor: u32) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let element = f
        .root
        .query_selector(&format!(".te-source-{ctor}"))?
        .ok_or_else(|| js("missing actual source component Element"))?;
    let guard = match kind {
        "node" => f.owner.handle().bind_node_mouse_element("b", element),
        "link" => f.owner.handle().bind_link_mouse_element("c", element),
        _ => return Err(js("unknown explicit Tree binding family")),
    }
    .map_err(js)?;
    f.bindings.borrow_mut().push(guard);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_unbind(key: &str) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    for b in f.bindings.borrow().iter() {
        b.unbind();
        b.unbind();
    }
    f.bindings.borrow_mut().clear();
    Ok(())
}
#[wasm_bindgen]
pub fn tree_events_flush(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?.owner.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_refresh(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?.owner.refresh().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    let h = fixture(key)?.owner.handle();
    h.set_widget_revision(revision).map_err(js)?;
    h.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_hidden(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?
        .owner
        .handle()
        .set_hidden(if id.is_empty() {
            Default::default()
        } else {
            [id.to_owned()].into_iter().collect()
        })
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
    let h = fixture(key)?.owner.handle();
    h.dispatch(SurfaceInput::Reset).map_err(js)?;
    if zoom {
        h.dispatch(SurfaceInput::Wheel {
            point: (232., 132.),
            delta: -50.,
        })
        .map_err(js)?;
    }
    h.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_dispatch(key: &str, x: f64, y: f64) -> std::result::Result<(), JsValue> {
    let h = fixture(key)?.owner.handle();
    h.dispatch(SurfaceInput::Move {
        id: 991,
        point: (x, y),
    })
    .map_err(js)?;
    h.flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_events_nested(key: &str) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let doc = web_sys::window().unwrap().document().unwrap();
    let element = doc.create_element("section")?;
    element.set_attribute("class", "te-nested")?;
    element.set_attribute(
        "style",
        "position:absolute;right:0;top:0;width:160px;height:120px;z-index:5",
    )?;
    f.root.append_child(&element)?;
    let node = HierarchyDatum::new("nested", None, 1.);
    let c = ChartConfig {
        width: 160.,
        height: 120.,
        margin: Margin {
            left: 10.,
            right: 10.,
            top: 10.,
            bottom: 10.,
        },
        ..Default::default()
    };
    let owner = BrowserTree::mount(
        element,
        vec![node],
        c,
        BrowserTreeOptions {
            chart: BrowserOptions {
                legend: false,
                tooltip: false,
                motion: MotionPreference::Reduced,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .map_err(js)?;
    *f.nested.borrow_mut() = Some(owner);
    Ok(())
}
fn snapshot_state(f: &Fixture) -> Result<Value> {
    let owner = owner_data(&f.owner.handle())?;
    let rows = source_rows(&f.m);
    Ok(
        json!({"owner":owner,"profile":f.profile,"phase":f.m.phase.get(),"mode":f.m.mode.get(),"registration":f.m.handler_revision.get(),"actions":f.m.actions.borrow().clone(),"rejectedProgramInputs":f.m.rejected_program_inputs.borrow().clone(),"events":f.m.errors.borrow().clone(),"rows":rows,"metrics":{"builds":f.m.builds.get(),"functions":f.m.functions.get(),"placements":f.m.placements.get(),"factories":f.m.factories.get(),"rawCalls":f.m.raw_calls.get(),"rejectedProgramFactories":f.m.rejected_program_factories.get(),"cleanups":f.m.cleanups.get(),"updates":f.m.updates.get()}}),
    )
}
/// Direct accepted getters, without reading either callback history.
#[wasm_bindgen]
pub fn tree_events_owner_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    let f = fixture(key)?;
    owner_data(&f.owner.handle())
        .map(|owner| owner.to_string())
        .map_err(js)
}
/// Compatibility full history; the observation driver checks it at bounded checkpoints.
#[wasm_bindgen]
pub fn tree_events_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    let f = fixture(key)?;
    let mut value = snapshot_state(&f).map_err(js)?;
    value["hooks"] = Value::Array(f.m.hooks.borrow().clone());
    value["returns"] = Value::Array(f.m.returns.borrow().clone());
    Ok(value.to_string())
}
/// Read actual contiguous suffixes. Invalid starts are errors, never clamped.
#[wasm_bindgen]
pub fn tree_events_snapshot_delta(
    key: &str,
    hook_start: u32,
    return_start: u32,
) -> std::result::Result<String, JsValue> {
    let f = fixture(key)?;
    let (hook_count, hooks) = {
        let values = f.m.hooks.borrow();
        let suffix = values
            .get(hook_start as usize..)
            .ok_or_else(|| js("Tree hook delta hook cursor exceeds history"))?;
        (values.len(), suffix.to_vec())
    };
    let (return_count, returns) = {
        let values = f.m.returns.borrow();
        let suffix = values
            .get(return_start as usize..)
            .ok_or_else(|| js("Tree hook delta return cursor exceeds history"))?;
        (values.len(), suffix.to_vec())
    };
    let mut value = snapshot_state(&f).map_err(js)?;
    value["hookStart"] = json!(hook_start);
    value["returnStart"] = json!(return_start);
    value["hookCount"] = json!(hook_count);
    value["returnCount"] = json!(return_count);
    value["hooks"] = Value::Array(hooks);
    value["returns"] = Value::Array(returns);
    Ok(value.to_string())
}
#[wasm_bindgen]
pub fn tree_events_stop(key: &str) -> std::result::Result<bool, JsValue> {
    let f = FIXTURES.with_borrow_mut(|all| all.remove(key));
    let Some(f) = f else {
        return Ok(false);
    };
    f.owner.stop();
    if let Some(nested) = f.nested.borrow_mut().take() {
        nested.stop();
    }
    f.bindings.borrow_mut().clear();
    if f.m.rows.borrow().iter().any(|r| r.frame.is_alive()) {
        return Err(js("Tree hook stop retained live Source frame"));
    }
    if f.m.cleanups.get() != f.m.rows.borrow().len() as u32 {
        return Err(js("Tree hook stop missed Source cleanup"));
    }
    f.root.remove();
    f.css.remove();
    Ok(true)
}
