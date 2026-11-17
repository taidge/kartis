//! Real retained SVG link-tooltip acceptance application, independent of older fixtures.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeHandle,
    BrowserTreeLinkMouseEvent, BrowserTreeLinkMouseHandlers,
    BrowserTreeLinkTooltipComponentContext, BrowserTreeLinkTooltipOptions,
    BrowserTreeMouseHandlers, BrowserTreeMouseInput, BrowserTreeOptions, BrowserTreePicking,
    BrowserTreeSvgBuildContext, BrowserTreeSvgLayerContext, BrowserTreeSvgLayerFrame,
    BrowserTreeSvgProgram, BrowserTreeTooltipAnchor, BrowserTreeTooltipComponentContext,
    BrowserTreeTooltipOptions, BrowserTreeWidgetOptions, MotionPreference, TreeLinkTooltipFrame,
    TreeTooltipFrame,
};

use super::*;
type SourceBond = glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>;
struct SourceRow {
    ctor: u32,
    born: u64,
    key: String,
    frame: SourceBond,
}
struct NodeRow {
    ctor: u32,
    born: u64,
    frame: glory::Bond<Option<TreeTooltipFrame>>,
}
struct LinkRow {
    ctor: u32,
    born: u64,
    frame: glory::Bond<Option<TreeLinkTooltipFrame>>,
}
struct LayerRow {
    ctor: u32,
    born: u64,
    key: String,
    frame: glory::Bond<Option<BrowserTreeSvgLayerFrame>>,
}
#[derive(Default)]
struct Metrics {
    phase: Cell<u8>,
    mode: Cell<u8>,
    fault: Cell<u8>,
    operation: Cell<u32>,
    builds: Cell<u32>,
    functions: Cell<u32>,
    placements: Cell<u32>,
    source_factories: Cell<u32>,
    node_factories: Cell<u32>,
    link_factories: Cell<u32>,
    layer_factories: Cell<u32>,
    cleanups: Cell<u32>,
    updates: Cell<u32>,
    callback_flush: Cell<bool>,
    owner: RefCell<Option<BrowserTreeHandle>>,
    root: RefCell<Option<web_sys::Element>>,
    sources: RefCell<Vec<SourceRow>>,
    nodes: RefCell<Vec<NodeRow>>,
    links: RefCell<Vec<LinkRow>>,
    layers: RefCell<Vec<LayerRow>>,
    retired: RefCell<BTreeSet<String>>,
    records: RefCell<Vec<Value>>,
    returns: RefCell<Vec<Value>>,
    probes: RefCell<Vec<Value>>,
    errors: RefCell<Vec<String>>,
}
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    m: Rc<Metrics>,
    profile: u8,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Rc<Fixture>>>=const{RefCell::new(BTreeMap::new())};}
fn fixture(key: &str) -> std::result::Result<Rc<Fixture>, JsValue> {
    FIXTURES.with_borrow(|all| {
        all.get(key)
            .cloned()
            .ok_or_else(|| js("missing Tree link tooltip fixture"))
    })
}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    if phase == 6 {
        return Vec::new();
    }
    let mut values = [
        ("a", Some("r"), 0.),
        ("c", Some("a"), if phase == 1 { 35. } else { 30. }),
        ("r", None, 0.),
        ("b", Some("r"), -0.),
        ("d", Some("a"), 10.),
    ]
    .into_iter()
    .map(|(id, parent, value)| {
        let mut d = HierarchyDatum::new(id, parent.map(str::to_owned), value);
        d.label = if phase == 7 && id == "b" {
            String::new()
        } else {
            format!("source-{id}")
        };
        d
    })
    .collect::<Vec<_>>();
    if phase == 2 {
        values.reverse();
    }
    values
}
fn config() -> ChartConfig {
    let mut c = ChartConfig {
        width: 520.,
        height: 360.,
        margin: Margin {
            left: 43.,
            top: 29.,
            right: 61.,
            bottom: 47.,
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
fn options(phase: u8) -> TreeOptions {
    let mut o = TreeOptions {
        node_radius: 22.,
        ..Default::default()
    };
    o.style.links.enabled = phase != 5;
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.width = 8.;
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
    json!({"id":m.id,"geometry":format!("{:?}",m.geometry),"label":m.label,"value":m.value,"bits":m.value.to_bits().to_string(),"fill":m.fill.map(rgba),"series":m.series,"stroke":m.stroke.map(|stroke|json!({"width":stroke.width,"color":rgba(stroke.color),"cap":format!("{:?}",stroke.cap)})),"opacity":m.opacity})
}
fn frame(f: &TreeEventFrame) -> Value {
    let a = f.activity();
    json!({"config":[f.config().width,f.config().height],"margin":[f.config().margin.left,f.config().margin.top,f.config().margin.right,f.config().margin.bottom],"plot":[f.plot().x,f.plot().y,f.plot().width,f.plot().height],"raw":f.source().iter().map(datum).collect::<Vec<_>>(),"pool":format!("{:p}",f.source().as_ptr()),"nodes":f.nodes().iter().enumerate().map(|(i,n)|json!({"index":i,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"value":n.value,"rawBits":f.source()[i].value.to_bits().to_string(),"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":f.links().iter().enumerate().map(|(i,l)|json!({"index":i,"source":l.source_index,"target":l.target_index,"sourcePosition":f.nodes()[l.source_index].position,"targetPosition":f.nodes()[l.target_index].position,"position":l.position,"markId":l.mark_id,"width":f.computed_link_styles()[i].thickness,"color":rgba(f.computed_link_styles()[i].color)})).collect::<Vec<_>>(),"labels":f.scene().labels.iter().map(|l|json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"rotation":l.rotation,"anchor":format!("{:?}",l.anchor),"color":rgba(l.color)})).collect::<Vec<_>>(),"activity":a.map(|a|json!({"focus":a.focus_id,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links]})),"theme":{"background":rgba(f.theme().background),"text":rgba(f.theme().text),"colors":f.theme().colors.iter().copied().map(rgba).collect::<Vec<_>>(),"fontSize":f.theme().font_size,"font":format!("{:?}",f.theme().font)},"marks":f.scene().marks.iter().map(mark).collect::<Vec<_>>()})
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

fn node_frame(t: &TreeTooltipFrame) -> Result<Value> {
    let f =
        TreeEventFrame::with_activity(t.data.config().clone(), t.data.activity_layout().clone())?;
    Ok(
        json!({"id":t.data.datum().id,"index":t.data.source_index(),"datum":datum(t.data.datum()),"value":t.data.node().value,"active":t.data.is_active(),"sourceGeneration":t.source_generation,"pointerClient":t.pointer_client,"viewportAnchor":t.viewport_anchor,"anchor":format!("{:?}",t.anchor),"frame":frame(&f)}),
    )
}
fn link_frame(t: &TreeLinkTooltipFrame) -> Result<Value> {
    let d = t.data.link_data();
    Ok(
        json!({"index":d.link_index(),"sourceIndex":d.source_index(),"targetIndex":d.target_index(),"sourceDatum":datum(d.source_datum()),"targetDatum":datum(d.target_datum()),"active":d.is_active(),"style":{"width":d.style().thickness,"color":rgba(d.style().color)},"mark":d.mark().map(mark),"sourceGeneration":t.source_generation,"pointerClient":t.pointer_client,"viewportAnchor":t.viewport_anchor,"requestedAnchor":format!("{:?}",t.requested_anchor),"origin":format!("{:?}",t.origin),"occurrence":t.occurrence.as_ref().map(|o|json!({"key":o.key,"index":o.index,"publication":o.publication})),"anchor":format!("{:?}",t.anchor),"text":t.data.text(&TooltipOptions::default())?,"frame":frame(d.frame())}),
    )
}
fn layer_frame(f: &BrowserTreeSvgLayerFrame) -> Result<Value> {
    let data = TreeEventFrame::new(f.data.config().clone(), f.data.layout().clone())?;
    let mut value = frame(&data);
    value["activity"]=f.data.activity().map(|a|json!({"focus":a.focus_id,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links]})).unwrap_or(Value::Null);
    Ok(
        json!({"key":f.key,"index":f.index,"sourceGeneration":f.source_generation,"publication":f.publication,"view":[f.view.scale,f.view.translate_x,f.view.translate_y],"frame":value}),
    )
}
fn row_read<A: std::fmt::Display, B: std::fmt::Display>(
    v: std::result::Result<std::result::Result<Option<Value>, A>, B>,
) -> (Option<Value>, Option<String>) {
    match v {
        Ok(Ok(value)) => (value, None),
        Ok(Err(error)) => (None, Some(error.to_string())),
        Err(error) => (None, Some(error.to_string())),
    }
}
fn membership(m: &Metrics, kind: &str, ctor: u32) -> Value {
    let actual = (|| -> std::result::Result<Value, JsValue> {
        let root = m
            .root
            .borrow()
            .clone()
            .ok_or_else(|| js("link tooltip fixture DOM missing"))?;
        let selector = if kind == "source" {
            format!(".tlt-source-{ctor}")
        } else if kind == "layer" {
            ".tlt-custom".to_owned()
        } else {
            format!(".tlt-{kind}-tip-{ctor}")
        };
        let Some(child) = root.query_selector(&selector)? else {
            return Ok(json!({"connected":false,"inPanel":false}));
        };
        let window = web_sys::window().ok_or_else(|| js("missing Window"))?;
        let style = window
            .get_computed_style(&child)?
            .ok_or_else(|| js("missing child computed style"))?;
        let panel = if kind == "node" {
            child.closest(".kartis-tree-tooltip")?
        } else if kind == "link" {
            child.closest(".kartis-tree-link-tooltip")?
        } else {
            None
        };
        let component = if kind == "node" {
            child.closest(".kartis-tree-tooltip-component")?
        } else if kind == "link" {
            child.closest(".kartis-tree-link-tooltip-component")?
        } else {
            None
        };
        let display = panel
            .as_ref()
            .map(|panel| {
                window
                    .get_computed_style(panel)
                    .and_then(|s| s.ok_or_else(|| js("missing panel computed style")))
                    .and_then(|s| s.get_property_value("display"))
            })
            .transpose()?;
        Ok(
            json!({"connected":child.is_connected(),"inPanel":panel.is_some(),"panelClass":panel.as_ref().and_then(|v|v.get_attribute("class")),"panelDisplay":display,"componentStyle":component.as_ref().and_then(|n|n.get_attribute("style")),"childDisplay":style.get_property_value("display")?,"pointerEvents":style.get_property_value("pointer-events")?,"occurrence":child.closest("[data-tree-svg-key]")?.and_then(|v|v.get_attribute("data-tree-svg-key")),"outerActive":child.closest(".kartis-tree-svg-layers")?.and_then(|v|v.get_attribute("data-tree-svg-active"))}),
        )
    })();
    json!({"value":actual.as_ref().ok(),"error":actual.err().map(|e|format!("{e:?}"))})
}
fn rows(m: &Metrics) -> Value {
    let source=m.sources.borrow().iter().map(|r|{let v=r.frame.try_with_untracked(source_frame);json!({"kind":"source","ctor":r.ctor,"born":r.born,"key":r.key,"alive":r.frame.is_alive(),"retired":m.retired.borrow().contains(&format!("source:{}",r.ctor)),"membership":membership(m,"source",r.ctor),"value":v.as_ref().ok(),"error":v.err().map(|e|e.to_string())})}).collect::<Vec<_>>();
    let nodes=m.nodes.borrow().iter().map(|r|{let(value,error)=row_read(r.frame.try_with_untracked(|f|f.as_ref().map(node_frame).transpose()));json!({"kind":"node","ctor":r.ctor,"born":r.born,"alive":r.frame.is_alive(),"retired":m.retired.borrow().contains(&format!("node:{}",r.ctor)),"membership":membership(m,"node",r.ctor),"value":value,"error":error})}).collect::<Vec<_>>();
    let links=m.links.borrow().iter().map(|r|{let(value,error)=row_read(r.frame.try_with_untracked(|f|f.as_ref().map(link_frame).transpose()));json!({"kind":"link","ctor":r.ctor,"born":r.born,"alive":r.frame.is_alive(),"retired":m.retired.borrow().contains(&format!("link:{}",r.ctor)),"membership":membership(m,"link",r.ctor),"value":value,"error":error})}).collect::<Vec<_>>();
    let layers=m.layers.borrow().iter().map(|r|{let(value,error)=row_read(r.frame.try_with_untracked(|f|f.as_ref().map(layer_frame).transpose()));json!({"kind":"layer","key":r.key,"ctor":r.ctor,"born":r.born,"alive":r.frame.is_alive(),"retired":m.retired.borrow().contains(&format!("layer:{}",r.ctor)),"membership":membership(m,"layer",r.ctor),"value":value,"error":error})}).collect::<Vec<_>>();
    json!({"sources":source,"nodes":nodes,"links":links,"layers":layers})
}
fn owner_data(h: &BrowserTreeHandle) -> Result<Value> {
    let s = h.snapshot()?;
    let f = TreeEventFrame::with_activity(s.config.clone(), s.layout.clone())?;
    let generation = h.source_generation();
    if s.active {
        generation.as_ref().map_err(Clone::clone)?;
    }
    let node = h.node_tooltip()?.as_ref().map(node_frame).transpose()?;
    let link = h.link_tooltip()?.as_ref().map(link_frame).transpose()?;
    let mut layers = BTreeMap::new();
    for key in ["links", "links-again", "nodes", "labels", "mesh", "panel"] {
        let value = h.svg_layer_frame(key);
        match value {
            Ok(value) => {
                layers.insert(
                    key,
                    json!({"value":value.as_ref().map(layer_frame).transpose()?,"error":null}),
                );
            }
            Err(error) => {
                if s.active {
                    return Err(error);
                }
                layers.insert(key, json!({"value":null,"error":error.to_string()}));
            }
        }
    }
    Ok(
        json!({"data":frame(&f),"sourceGeneration":generation.as_ref().ok(),"sourceGenerationError":generation.as_ref().err().map(|e|e.to_string()),"nodeTooltip":node,"linkTooltip":link,"layers":layers,"active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"revision":s.widget_revision,"desiredFocus":s.desired_focus,"error":s.error.map(|e|e.to_string()),"chart":s.chart.map(|v|json!({"view":[v.view.scale,v.view.translate_x,v.view.translate_y],"selected":v.selected,"hovered":v.hovered,"hidden":v.hidden,"revision":v.revision}))}),
    )
}
fn observe(m: &Metrics, kind: &str, own: Value) {
    let owner = m.owner.borrow().as_ref().map(owner_data).transpose();
    m.probes.borrow_mut().push(json!({"kind":kind,"own":own,"operation":m.operation.get(),"owner":owner.as_ref().ok(),"ownerError":owner.err().map(|e|e.to_string()),"rows":rows(m)}));
}
fn build(
    m: &Rc<Metrics>,
    c: &ChartConfig,
    s: &[HierarchyDatum],
    o: &TreeOptions,
    a: &TreeActivityOptions,
    layers: Option<&[TreeSvgLayer]>,
) -> Result<TreeActivityLayout> {
    m.builds.set(m.builds.get() + 1);
    if m.fault.get() == 1 {
        return Err(ChartError("link tooltip source builder rejected".into()));
    }
    let count = m.clone();
    let mut size = move |v: &TreeNodeFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(26. + 6. * v.depth as f64)
    };
    let count = m.clone();
    let mut color = move |v: &TreeNodeFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(match v.datum.id.as_str() {
            "r" => Color(160, 80, 30, 255),
            "a" => Color(0, 100, 255, 255),
            "b" => Color(0, 200, 0, 255),
            "c" => Color(230, 110, 20, 255),
            _ => Color(100, 20, 180, 255),
        })
    };
    let count = m.clone();
    let mut thickness = move |v: &TreeLinkFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(
            if count.phase.get() == 3 && v.source[v.link.target_index].id == "c" {
                0.
            } else {
                8.
            },
        )
    };
    let count = m.clone();
    let mut link_color = move |v: &TreeLinkFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(Color(
            230,
            110,
            20,
            if count.phase.get() == 4 && v.source[v.link.target_index].id == "c" {
                0
            } else {
                255
            },
        ))
    };
    let count = m.clone();
    let mut label = move |v: &TreeLabelFunctionContext<'_>| {
        count.functions.set(count.functions.get() + 1);
        Ok(format!("paint-{}", v.datum.id))
    };
    let mut base = TreeFactories {
        node_size: Some(&mut size),
        node_color: Some(&mut color),
        link_thickness: Some(&mut thickness),
        link_color: Some(&mut link_color),
        label: Some(&mut label),
    };
    let mut modifiers = TreeActivityFactories::default();
    if let Some(layers) = layers {
        tree_svg_layout_with_activity(c, s, o, layers, &mut base, a, &mut modifiers)
    } else {
        tree_layout_with_activity(c, s, o, &mut base, a, &mut modifiers)
    }
}
fn placement(
    m: Rc<Metrics>,
) -> impl FnMut(&TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    move |c| {
        m.placements.set(m.placements.get() + 1);
        if m.fault.get() == 2 {
            return Err(ChartError("link tooltip source placement rejected".into()));
        }
        Ok(match c.component {
            TreeComponentContext::Node(v) if v.datum.id == "b" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [24., 28.],
                    measured: TextSize {
                        width: 60.,
                        height: 24.,
                    },
                })
            }
            TreeComponentContext::Link(v) if v.layout.source()[v.link.target_index].id == "c" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-34., -30.],
                    measured: TextSize {
                        width: 68.,
                        height: 22.,
                    },
                })
            }
            TreeComponentContext::Label(v) if v.datum.id == "d" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-28., 24.],
                    measured: TextSize {
                        width: 56.,
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
        m.source_factories.set(m.source_factories.get() + 1);
        let ctor = m.source_factories.get();
        let born = c.revision;
        let key = c.key;
        let frame = c.frame;
        let initial = frame
            .try_with_untracked(source_frame)
            .map_err(|e| ChartError(format!("source constructor actual frame {e:?}")))?;
        observe(
            &m,
            "source-constructor",
            json!({"ctor":ctor,"born":born,"key":key,"initial":initial}),
        );
        let m = m.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.sources.borrow_mut().push(SourceRow {
                ctor,
                born,
                key: key.clone(),
                frame,
            });
            let cleanup = m.clone();
            scope.on_cleanup(move || {
                cleanup.cleanups.set(cleanup.cleanups.get() + 1);
                cleanup
                    .retired
                    .borrow_mut()
                    .insert(format!("source:{ctor}"));
            })?;
            let watched = m.clone();
            scope.effect(move || {
                observe(
                    &watched,
                    "source-effect",
                    json!({"ctor":ctor,"born":born,"frame":source_frame(&frame.get())}),
                )
            });
            let count = scope.cage(0u32);
            let label_key = key.clone();
            glory::ui::button().class(format!("tlt-source tlt-source-{ctor}")).style(scope.bond(move||Style{background:Some(if frame.with(|v|v.data.is_active)==Some(true){Color(0,100,255,255)}else{Color(100,20,180,255)}),..Style::new().position(Position::Relative).width(Length::Percent(100.)).height(Length::Percent(100.))})).label(scope.bond(move||json!({"ctor":ctor,"born":born,"key":label_key,"count":count.get(),"frame":source_frame(&frame.get())}).to_string())).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).show_in(scope)
        })) as Box<dyn Widget>)
    }
}

fn install_node(f: &Fixture, revision: u64) -> Result<()> {
    let m = f.m.clone();
    f.owner.handle().set_node_tooltip_component(revision,move|c:BrowserTreeTooltipComponentContext|{m.node_factories.set(m.node_factories.get()+1);let ctor=m.node_factories.get();if m.fault.get()==4{return Err(ChartError("link tooltip node factory rejected".into()));}
if m.fault.get()==6&&let Some(h)=m.owner.borrow().as_ref(){h.stop();}let frame=c.frame;let born=c.revision;let initial_none=frame.try_with_untracked(Option::is_none).map_err(|e|ChartError(format!("node constructor actual frame {e:?}")))?;observe(&m,"node-constructor",json!({"ctor":ctor,"born":born,"initialNone":initial_none}));let m=m.clone();Ok(glory::ui::widget(move|scope|{m.nodes.borrow_mut().push(NodeRow{ctor,born,frame});let cleaned=m.clone();scope.on_cleanup(move||{cleaned.cleanups.set(cleaned.cleanups.get()+1);cleaned.retired.borrow_mut().insert(format!("node:{ctor}"));})?;let watched=m.clone();scope.effect(move||{let own=frame.get().as_ref().map(node_frame).transpose();observe(&watched,"node-tooltip-effect",json!({"ctor":ctor,"born":born,"frame":own.as_ref().ok(),"error":own.err().map(|e|e.to_string())}));});let count=scope.cage(0u32);let focus=scope.cage(false);let styled=m.clone();
 glory::ui::button().class(format!("tlt-tip tlt-node-tip tlt-node-tip-{ctor}"))
 .style(scope.bond(move||{let large=frame.with(|f|f.as_ref().is_some_and(|f|f.data.source().iter().any(|d|d.id=="c"&&d.value==35.)));Style{background:Some(Color(100,20,180,255)),..Style::new().position(Position::Relative).width(Length::Px(if large&&styled.fault.get()==5{f64::NAN}else if large{144.}else{112.})).height(Length::Px(if large{52.}else{44.}))}}))
 .label(scope.bond(move||{let own=frame.get().as_ref().map(node_frame).transpose();json!({"kind":"node","ctor":ctor,"born":born,"count":count.get(),"focused":focus.get(),"frame":own.as_ref().ok(),"error":own.err().map(|e|e.to_string())}).to_string()}))
 .fill(glory::ui::text().content("").class("tlt-tip-dot").style(Style{background:Some(Color(255,220,0,255)),..Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(8.)).height(Length::Px(8.))}))
 .on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|event|if let UiEvent::FocusChanged(value)=event{focus.revise(|old|*old=value);}).show_in(scope)
 }))})
}

fn install_link(f: &Fixture, revision: u64) -> Result<()> {
    let m = f.m.clone();
    f.owner.handle().set_link_tooltip_component(revision,move|c:BrowserTreeLinkTooltipComponentContext|{m.link_factories.set(m.link_factories.get()+1);let ctor=m.link_factories.get();if m.fault.get()==3{return Err(ChartError("link tooltip link factory rejected".into()));}
if m.fault.get()==6&&let Some(h)=m.owner.borrow().as_ref(){h.stop();}let frame=c.frame;let born=c.revision;let initial_none=frame.try_with_untracked(Option::is_none).map_err(|e|ChartError(format!("link constructor actual frame {e:?}")))?;observe(&m,"link-constructor",json!({"ctor":ctor,"born":born,"initialNone":initial_none}));let m=m.clone();Ok(glory::ui::widget(move|scope|{m.links.borrow_mut().push(LinkRow{ctor,born,frame});let cleaned=m.clone();scope.on_cleanup(move||{cleaned.cleanups.set(cleaned.cleanups.get()+1);cleaned.retired.borrow_mut().insert(format!("link:{ctor}"));})?;let watched=m.clone();scope.effect(move||{let own=frame.get().as_ref().map(link_frame).transpose();observe(&watched,"link-tooltip-effect",json!({"ctor":ctor,"born":born,"frame":own.as_ref().ok(),"error":own.err().map(|e|e.to_string())}));});let count=scope.cage(0u32);let focus=scope.cage(false);let styled=m.clone();
 glory::ui::button().class(format!("tlt-tip tlt-link-tip tlt-link-tip-{ctor}"))
 .style(scope.bond(move||{let large=frame.with(|f|f.as_ref().is_some_and(|f|f.data.source().iter().any(|d|d.id=="c"&&d.value==35.)));Style{background:Some(Color(100,20,180,255)),..Style::new().position(Position::Relative).width(Length::Px(if large&&styled.fault.get()==5{f64::NAN}else if large{144.}else{112.})).height(Length::Px(if large{52.}else{44.}))}}))
 .label(scope.bond(move||{let own=frame.get().as_ref().map(link_frame).transpose();json!({"kind":"link","ctor":ctor,"born":born,"count":count.get(),"focused":focus.get(),"frame":own.as_ref().ok(),"error":own.err().map(|e|e.to_string())}).to_string()}))
 .fill(glory::ui::text().content("").class("tlt-tip-dot").style(Style{background:Some(Color(255,220,0,255)),..Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(8.)).height(Length::Px(8.))}))
 .on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|event|if let UiEvent::FocusChanged(value)=event{focus.revise(|old|*old=value);}).show_in(scope)
 }))})
}

fn specs(mode: u8) -> Result<Vec<TreeSvgLayer>> {
    use TreeSvgLayerKind as K;
    let rows = match mode {
        0 => vec![
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("labels", K::Labels),
        ],
        1 => vec![
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("links-again", K::Links),
            ("labels", K::Labels),
        ],
        2 => vec![("nodes", K::Nodes), ("labels", K::Labels)],
        3 => vec![
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("labels", K::Labels),
            ("mesh", K::Mesh),
        ],
        4 => vec![
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("labels", K::Labels),
            ("panel", K::Custom("panel".into())),
        ],
        5 => vec![
            ("mesh", K::Mesh),
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("labels", K::Labels),
            ("panel", K::Custom("panel".into())),
        ],
        6 => vec![
            ("links", K::Links),
            ("nodes", K::Nodes),
            ("labels", K::Labels),
            ("mesh", K::Mesh),
            ("panel", K::Custom("panel".into())),
        ],
        _ => return Err(ChartError("invalid link tooltip layer mode".into())),
    };
    rows.into_iter()
        .map(|(key, kind)| TreeSvgLayer::new(key, kind))
        .collect()
}
fn program(m: Rc<Metrics>, mode: u8) -> Result<BrowserTreeSvgProgram> {
    BrowserTreeSvgProgram::new(specs(mode)?)?.custom("panel",0,move|c:BrowserTreeSvgLayerContext|{m.layer_factories.set(m.layer_factories.get()+1);let ctor=m.layer_factories.get();let born=c.revision;let key=c.key;let frame=c.frame;let initial_none=frame.try_with_untracked(Option::is_none).map_err(|e|ChartError(format!("layer constructor actual frame {e:?}")))?;observe(&m,"layer-constructor",json!({"ctor":ctor,"born":born,"key":key,"initialNone":initial_none}));let m=m.clone();Ok(glory::ui::widget(move|scope|{m.layers.borrow_mut().push(LayerRow{ctor,born,key:key.clone(),frame});let clean=m.clone();scope.on_cleanup(move||{clean.cleanups.set(clean.cleanups.get()+1);clean.retired.borrow_mut().insert(format!("layer:{ctor}"));})?;let watch=m.clone();scope.effect(move||{let own=frame.get().as_ref().map(layer_frame).transpose();observe(&watch,"layer-effect",json!({"ctor":ctor,"born":born,"frame":own.as_ref().ok(),"error":own.err().map(|e|e.to_string())}));});let count=scope.cage(0u32);glory::ui::button().class("tlt-custom").label(scope.bond(move||json!({"ctor":ctor,"count":count.get()}).to_string())).style(Style{background:Some(Color(0,180,200,255)),..Style::new().position(Position::Absolute).left(Length::Px(244.)).top(Length::Px(82.)).width(Length::Px(80.)).height(Length::Px(36.))}).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).show_in(scope)}))}).map(|p|p.interactive(mode!=6))
}
fn native_input(i: &BrowserTreeMouseInput) -> Result<Value> {
    let window =
        web_sys::window().ok_or_else(|| ChartError("link tooltip Window missing".into()))?;
    let lookup = web_sys::js_sys::Reflect::get(
        &window,
        &JsValue::from_str("__treeLinkTooltipNativeSequence"),
    )
    .map_err(|e| ChartError(format!("link tooltip native lookup {e:?}")))?
    .dyn_into::<web_sys::js_sys::Function>()
    .map_err(|_| ChartError("link tooltip native sequence missing".into()))?;
    let sequence = lookup
        .call1(&JsValue::UNDEFINED, i.event.as_ref())
        .map_err(|e| ChartError(format!("link tooltip native event lookup failed {e:?}")))?
        .as_f64()
        .ok_or_else(|| ChartError("link tooltip native sequence is not numeric".into()))?;
    Ok(
        json!({"nativeSequence":sequence,"nativeType":i.event.type_(),"kind":format!("{:?}",i.kind),"trusted":i.event.is_trusted(),"eventPhase":i.event.event_phase(),"currentTargetIsWindow":i.event.current_target().is_some_and(|v|JsValue::from(v)==JsValue::from(window)),"client":i.client,"viewport":i.viewport,"scene":i.scene,"origin":format!("{:?}",i.origin),"occurrence":i.occurrence.as_ref().map(|o|json!({"key":o.key,"index":o.index,"publication":o.publication}))}),
    )
}
fn record_link(m: &Metrics, e: BrowserTreeLinkMouseEvent) -> Result<()> {
    let h = m
        .owner
        .borrow()
        .clone()
        .ok_or_else(|| ChartError("link tooltip callback owner missing".into()))?;
    let input = native_input(&e.input)?;
    let index = e.data.link_index();
    m.records.borrow_mut().push(json!({"input":input,"sourceGeneration":e.source_generation,"index":index,"frame":frame(e.data.frame()),"owner":owner_data(&h)?,"rows":rows(m)}));
    let result = if m.callback_flush.get() {
        h.flush()
    } else {
        Ok(())
    };
    let after = owner_data(&h);
    m.returns.borrow_mut().push(json!({"nativeSequence":input["nativeSequence"],"kind":input["kind"],"ok":result.is_ok(),"error":result.as_ref().err().map(|e|e.to_string()),"eventFrameAfter":frame(e.data.frame()),"ownerAfter":after.as_ref().ok(),"readError":after.as_ref().err().map(|e|e.to_string())}));
    after?;
    result
}
fn snapshot_state(f: &Fixture) -> Result<Value> {
    Ok(
        json!({"owner":owner_data(&f.owner.handle())?,"rows":rows(&f.m),"profile":f.profile,"phase":f.m.phase.get(),"mode":f.m.mode.get(),"operation":f.m.operation.get(),"events":f.m.errors.borrow().clone(),"metrics":{"builds":f.m.builds.get(),"functions":f.m.functions.get(),"placements":f.m.placements.get(),"sourceFactories":f.m.source_factories.get(),"nodeFactories":f.m.node_factories.get(),"linkFactories":f.m.link_factories.get(),"layerFactories":f.m.layer_factories.get(),"cleanups":f.m.cleanups.get(),"updates":f.m.updates.get()}}),
    )
}

#[wasm_bindgen]
pub fn tree_link_tooltip_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    if profile > 5 {
        return Err(js("invalid Tree link tooltip profile"));
    }
    tree_link_tooltip_stop(key)?;
    let document = web_sys::window()
        .ok_or_else(|| js("missing Window"))?
        .document()
        .ok_or_else(|| js("missing Document"))?;
    let root = document.create_element("section")?;
    root.set_id(&format!("tree-link-tooltip-{key}"));
    root.set_attribute("style","position:relative;width:520px;height:360px;margin:20px;background:white;transform-origin:0 0")?;
    document
        .body()
        .ok_or_else(|| js("missing Body"))?
        .append_child(&root)?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-link-tooltip-{key} .tlt-source,#tree-link-tooltip-{key} .tlt-tip,#tree-link-tooltip-{key} .tlt-custom{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;overflow:hidden;min-width:0;box-shadow:none}}#tree-link-tooltip-{key} .tlt-source{{pointer-events:auto!important}}")));
    document
        .query_selector("head")?
        .ok_or_else(|| js("missing Head"))?
        .append_child(&css)?;
    let m = Rc::new(Metrics::default());
    *m.root.borrow_mut() = Some(root.clone());
    let opts = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer: if profile == 4 {
                BrowserRenderer::Canvas
            } else {
                BrowserRenderer::Svg
            },
            legend: false,
            tooltip: profile != 5,
            highlight: BrowserHighlight::Controlled,
            motion: MotionPreference::Reduced,
            ..Default::default()
        },
        tree: options(0),
        picking: if profile == 1 || profile == 3 {
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
    let b = m.clone();
    let ordinary = move |c: &BrowserTreeBuildContext<'_>| {
        build(&b, c.config, c.source, c.tree, c.activity, None)
    };
    let b = m.clone();
    let literal = move |c: &BrowserTreeSvgBuildContext<'_>| {
        build(&b, c.config, c.source, c.tree, c.activity, Some(c.layers))
    };
    let mounted = if profile == 2 || profile == 3 {
        BrowserTree::mount_with_factory_and_svg_layers_and_components(
            root.clone(),
            raw(0),
            config(),
            opts,
            program(m.clone(), 0).map_err(js)?,
            literal,
            BrowserTreeWidgetOptions {
                source_ordered: true,
                revision: 0,
                replace_ink: false,
            },
            placement(m.clone()),
            source_factory(m.clone()),
        )
    } else {
        BrowserTree::mount_with_factory(root.clone(), raw(0), config(), opts, ordinary)
    };
    let owner = match mounted {
        Ok(owner) => owner,
        Err(error) => {
            root.remove();
            css.remove();
            return Err(js(error));
        }
    };
    *m.owner.borrow_mut() = Some(owner.handle());
    let events = m.clone();
    owner.on_event(move |e| events.errors.borrow_mut().push(format!("{e:?}")));
    let count = m.clone();
    owner.on_update(move |_| count.updates.set(count.updates.get() + 1));
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.to_owned(),
            Rc::new(Fixture {
                root,
                css,
                owner,
                m,
                profile,
            }),
        );
    });
    Ok(())
}
fn anchor(value: u8) -> std::result::Result<BrowserTreeTooltipAnchor, JsValue> {
    use BrowserTreeTooltipAnchor as A;
    Ok(match value {
        0 => A::Top,
        1 => A::TopRight,
        2 => A::Right,
        3 => A::BottomRight,
        4 => A::Bottom,
        5 => A::BottomLeft,
        6 => A::Left,
        7 => A::TopLeft,
        8 => A::Center,
        _ => return Err(js("invalid link tooltip anchor")),
    })
}
#[wasm_bindgen]
pub fn tree_link_tooltip_configure(
    key: &str,
    revision: u64,
    anchor_value: u8,
    gap: f64,
) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.m.operation.set(f.m.operation.get() + 1);
    f.owner
        .handle()
        .set_link_tooltip_options(BrowserTreeLinkTooltipOptions {
            anchor: anchor(anchor_value)?,
            gap,
        })
        .map_err(js)?;
    install_link(&f, revision).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_node(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.m.operation.set(f.m.operation.get() + 1);
    f.owner
        .handle()
        .set_node_tooltip_options(BrowserTreeTooltipOptions::default())
        .map_err(js)?;
    install_node(&f, revision).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_anchor(
    key: &str,
    anchor_value: u8,
    gap: f64,
) -> std::result::Result<(), JsValue> {
    fixture(key)?
        .owner
        .handle()
        .set_link_tooltip_options(BrowserTreeLinkTooltipOptions {
            anchor: anchor(anchor_value)?,
            gap,
        })
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_content(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    match mode {
        0 => f.owner.handle().clear_link_tooltip_component(),
        1 => f.owner.handle().reset_link_tooltip(),
        2 => f.owner.handle().reset_node_tooltip(),
        _ => return Err(js("invalid link tooltip content mode")),
    }
    .map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_flush(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?.owner.handle().flush().map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_refresh(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?.owner.handle().refresh().map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 7 {
        return Err(js("invalid link tooltip source phase"));
    }
    let f = fixture(key)?;
    f.m.operation.set(f.m.operation.get() + 1);
    f.m.phase.set(phase);
    f.owner
        .handle()
        .set_tree_options(options(phase))
        .map_err(js)?;
    f.owner.handle().set_source(raw(phase)).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_fault(key: &str, fault: u8) -> std::result::Result<(), JsValue> {
    if fault > 6 {
        return Err(js("invalid link tooltip fault"));
    }
    fixture(key)?.m.fault.set(fault);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_program(key: &str, mode: u8) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.m.operation.set(f.m.operation.get() + 1);
    f.owner
        .handle()
        .set_svg_program(program(f.m.clone(), mode).map_err(js)?)
        .map_err(js)?;
    f.m.mode.set(mode);
    Ok(())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    fixture(key)?
        .owner
        .handle()
        .set_widget_revision(revision)
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_current(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let accepted = f.owner.handle().snapshot().map_err(js)?;
    let mut activity = TreeActivityOptions {
        focus_id: None,
        highlight: accepted.layout.activity.highlight.clone(),
        fix_node_color_at_depth: accepted.layout.activity.fix_node_color_at_depth,
    };
    activity.focus_id = if id.is_empty() {
        None
    } else {
        Some(id.to_owned())
    };
    f.owner.handle().set_activity_options(activity).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_hidden(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    let mut hidden = BTreeSet::new();
    if !id.is_empty() {
        hidden.insert(id.to_owned());
    }
    fixture(key)?.owner.handle().set_hidden(hidden).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_resize(
    key: &str,
    width: f64,
    height: f64,
) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    let mut c = f.owner.handle().snapshot().map_err(js)?.config;
    c.width = width;
    c.height = height;
    f.root.set_attribute("style",&format!("position:relative;width:{width}px;height:{height}px;margin:20px;background:white;transform-origin:0 0"))?;
    f.owner.handle().set_config(c).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_zoom(key: &str, enabled: bool) -> std::result::Result<(), JsValue> {
    let h = fixture(key)?.owner.handle();
    h.dispatch(SurfaceInput::Reset).map_err(js)?;
    h.flush().map_err(js)?;
    if enabled {
        h.dispatch(SurfaceInput::Wheel {
            point: (260., 180.),
            delta: -50.,
        })
        .map_err(js)?;
    }
    Ok(())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_dispatch(key: &str, x: f64, y: f64) -> std::result::Result<(), JsValue> {
    fixture(key)?
        .owner
        .handle()
        .dispatch(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_callbacks(
    key: &str,
    enabled: bool,
    flush: bool,
) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.m.callback_flush.set(flush);
    let mut h = BrowserTreeMouseHandlers::default();
    if enabled {
        let a = f.m.clone();
        let b = f.m.clone();
        let c = f.m.clone();
        h.links = BrowserTreeLinkMouseHandlers {
            on_mouse_enter: Some(Box::new(move |e| record_link(&a, e))),
            on_mouse_move: Some(Box::new(move |e| record_link(&b, e))),
            on_mouse_leave: Some(Box::new(move |e| record_link(&c, e))),
            ..Default::default()
        };
    }
    f.owner.handle().set_mouse_handlers(h).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_owner_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    owner_data(&fixture(key)?.owner.handle())
        .map(|v| v.to_string())
        .map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    let f = fixture(key)?;
    let mut v = snapshot_state(&f).map_err(js)?;
    v["records"] = Value::Array(f.m.records.borrow().clone());
    v["returns"] = Value::Array(f.m.returns.borrow().clone());
    v["probes"] = Value::Array(f.m.probes.borrow().clone());
    Ok(v.to_string())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_snapshot_delta(
    key: &str,
    record_start: u32,
    return_start: u32,
    probe_start: u32,
) -> std::result::Result<String, JsValue> {
    let f = fixture(key)?;
    let (mut value, record_count, return_count, probe_count) = (
        snapshot_state(&f).map_err(js)?,
        f.m.records.borrow().len(),
        f.m.returns.borrow().len(),
        f.m.probes.borrow().len(),
    );
    let records =
        f.m.records
            .borrow()
            .get(record_start as usize..)
            .ok_or_else(|| js("link tooltip record cursor exceeds history"))?
            .to_vec();
    let returns =
        f.m.returns
            .borrow()
            .get(return_start as usize..)
            .ok_or_else(|| js("link tooltip return cursor exceeds history"))?
            .to_vec();
    let probes =
        f.m.probes
            .borrow()
            .get(probe_start as usize..)
            .ok_or_else(|| js("link tooltip probe cursor exceeds history"))?
            .to_vec();
    value["recordStart"] = json!(record_start);
    value["recordCount"] = json!(record_count);
    value["returnStart"] = json!(return_start);
    value["returnCount"] = json!(return_count);
    value["probeStart"] = json!(probe_start);
    value["probeCount"] = json!(probe_count);
    value["records"] = Value::Array(records);
    value["returns"] = Value::Array(returns);
    value["probes"] = Value::Array(probes);
    Ok(value.to_string())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_component(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    let f = fixture(key)?;
    f.m.operation.set(f.m.operation.get() + 1);
    install_link(&f, revision).map_err(js)
}
#[wasm_bindgen]
pub fn tree_link_tooltip_stop_owner(key: &str) -> std::result::Result<(), JsValue> {
    fixture(key)?.owner.stop();
    Ok(())
}
#[wasm_bindgen]
pub fn tree_link_tooltip_stop(key: &str) -> std::result::Result<bool, JsValue> {
    let Some(f) = FIXTURES.with_borrow_mut(|all| all.remove(key)) else {
        return Ok(false);
    };
    f.owner.stop();
    let m = &f.m;
    if m.sources.borrow().iter().any(|r| r.frame.is_alive())
        || m.nodes.borrow().iter().any(|r| r.frame.is_alive())
        || m.links.borrow().iter().any(|r| r.frame.is_alive())
        || m.layers.borrow().iter().any(|r| r.frame.is_alive())
    {
        return Err(js("link tooltip Stop retained live component frame"));
    }
    let constructed = m.sources.borrow().len()
        + m.nodes.borrow().len()
        + m.links.borrow().len()
        + m.layers.borrow().len();
    if m.cleanups.get() as usize != constructed {
        return Err(js("link tooltip Stop missed actual Scope cleanup"));
    }
    if f.owner.handle().node_tooltip().map_err(js)?.is_some()
        || f.owner.handle().link_tooltip().map_err(js)?.is_some()
    {
        return Err(js("link tooltip Stop retained presentation"));
    }
    f.root.remove();
    f.css.remove();
    Ok(true)
}
