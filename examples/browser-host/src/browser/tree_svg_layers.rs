//! Real SVG occurrence/foreignObject/Glory layer acceptance fixture.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind, GraphicProps};
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeHandle, BrowserTreeOptions, BrowserTreePicking,
    BrowserTreeSvgBuildContext, BrowserTreeSvgCurrentNode, BrowserTreeSvgLayerContext,
    BrowserTreeSvgLayerFrame, BrowserTreeSvgProgram, BrowserTreeTooltipComponentContext,
    BrowserTreeTooltipOptions, BrowserTreeWidgetOptions, MotionPreference, TreeTooltipFrame,
};

use super::*;

type LayerBond = glory::Bond<Option<BrowserTreeSvgLayerFrame>>;
type SourceBond = glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>;
type TooltipBond = glory::Bond<Option<TreeTooltipFrame>>;
struct LayerRow {
    key: String,
    born: u64,
    ctor: u32,
    frame: LayerBond,
    initial: BrowserTreeSvgLayerFrame,
    current: BrowserTreeSvgCurrentNode,
}
struct SourceRow {
    key: String,
    born: u64,
    ctor: u32,
    frame: SourceBond,
}
struct TooltipRow {
    ctor: u32,
    frame: TooltipBond,
}
#[derive(Default)]
struct Metrics {
    phase: Cell<u8>,
    mode: Cell<u8>,
    theme: Cell<bool>,
    fault: Cell<u8>,
    operation: Cell<u8>,
    revision: Cell<u64>,
    source_revision: Cell<u64>,
    builds: Cell<u32>,
    functions: Cell<u32>,
    label_calls: Cell<u32>,
    placements: Cell<u32>,
    layer_factories: Cell<u32>,
    layer_bodies: Cell<u32>,
    source_factories: Cell<u32>,
    tooltip_factories: Cell<u32>,
    layer_cleanups: Cell<u32>,
    source_cleanups: Cell<u32>,
    tooltip_cleanups: Cell<u32>,
    cap_action: Cell<u8>,
    owner: RefCell<Option<BrowserTreeHandle>>,
    layer_rows: RefCell<Vec<LayerRow>>,
    source_rows: RefCell<Vec<SourceRow>>,
    tooltip_rows: RefCell<Vec<TooltipRow>>,
    retired_layers: RefCell<BTreeSet<u32>>,
    retired_sources: RefCell<BTreeSet<u32>>,
    retired_tooltips: RefCell<BTreeSet<u32>>,
    probes: RefCell<Vec<Value>>,
    calls: RefCell<Vec<Value>>,
    cap_events: RefCell<Vec<Value>>,
}
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    m: Rc<Metrics>,
    picking: u8,
    components: u8,
    events: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    let mut out = [
        ("a", Some("r"), 0.),
        ("c", Some("a"), if phase == 1 { 35. } else { 30. }),
        ("r", None, 0.),
        ("b", Some("r"), if phase == 6 { 0. } else { -0. }),
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
    if phase == 5 {
        out.reverse();
    }
    out
}
fn config(width: f64, height: f64, theme: bool) -> ChartConfig {
    let mut c = ChartConfig {
        width,
        height,
        margin: Margin {
            left: 31.,
            top: 23.,
            right: 47.,
            bottom: 37.,
        },
        ..Default::default()
    };
    c.theme.colors = vec![
        if theme {
            Color(0, 180, 200, 255)
        } else {
            Color(100, 20, 180, 255)
        },
        Color(0, 180, 200, 255),
        Color(255, 220, 0, 255),
    ];
    c
}
fn tree_options() -> TreeOptions {
    let mut o = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    o.style.labels.position = TreeLabelPosition::Center;
    o.style.labels.size = Some(12.);
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.width = 6.;
    o.style.links.color = TreeLinkColor::Fixed(Color(230, 110, 20, 255));
    o
}
fn specs(mode: u8) -> Result<Vec<TreeSvgLayer>> {
    use TreeSvgLayerKind as K;
    let e = || ("extras", K::SceneExtras);
    let l = || ("links", K::Links);
    let n = || ("nodes", K::Nodes);
    let t = || ("labels", K::Labels);
    let mesh = || ("mesh", K::Mesh);
    let u = || ("under", K::Custom("panel".into()));
    let o = || ("over", K::Custom("panel".into()));
    let rows = match mode {
        0 => vec![e(), l(), u(), n(), o(), t()],
        1 => vec![e(), l(), o(), n(), u(), t()],
        2 => vec![e(), l(), u(), n(), ("nodes-again", K::Nodes), o(), t()],
        3 => vec![u(), o()],
        4 => vec![],
        5 => vec![e(), l(), u(), n(), o(), t(), mesh()],
        6 => vec![e(), l(), n(), t(), mesh(), u(), o()],
        7 => vec![e(), l(), u(), o(), mesh(), n(), t()],
        8 | 9 => vec![e(), l(), u(), n(), o(), t(), mesh()],
        10 => vec![e(), l(), u(), n(), o(), t(), ("labels-again", K::Labels)],
        11 => vec![e(), l(), ("links-again", K::Links), u(), n(), o(), t()],
        _ => return Err(ChartError("unknown SVG layer fixture mode".into())),
    };
    rows.into_iter()
        .map(|(key, kind)| TreeSvgLayer::new(key, kind))
        .collect()
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn datum(d: &HierarchyDatum) -> Value {
    json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value,"bits":d.value.to_bits().to_string()})
}
fn cohort(
    source: &[HierarchyDatum],
    nodes: &[TreeNode],
    links: &[TreeLink],
    labels: &[Label],
    activity: &TreeActivityState,
) -> Value {
    json!({"raw":source.iter().map(datum).collect::<Vec<_>>(),"nodes":nodes.iter().map(|n|json!({"index":n.source_index,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"value":n.value,"rawBits":n.raw_value.to_bits().to_string(),"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":links.iter().map(|l|json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"position":l.position,"markId":l.mark_id})).collect::<Vec<_>>(),"labels":labels.iter().map(|l|json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"rotation":l.rotation,"anchor":format!("{:?}",l.anchor),"color":rgba(l.color)})).collect::<Vec<_>>(),"activity":{"focus":activity.focus_id,"nodes":activity.nodes,"links":activity.links,"flags":[activity.highlight.ancestor_nodes,activity.highlight.descendant_nodes,activity.highlight.ancestor_links,activity.highlight.descendant_links]}})
}
fn layout_cohort(layout: &TreeActivityLayout) -> Value {
    let l = &layout.layout;
    cohort(
        l.source(),
        &l.nodes,
        &l.links,
        &l.scene.labels,
        &layout.activity,
    )
}
fn source_data(d: &TreeActivityComponentData) -> Value {
    let (s, n, l, labels, role, index) = match &d.component {
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
    json!({"role":role,"index":index,"active":d.is_active,"cohort":cohort(s,n,l,&labels.labels,&d.activity)})
}
fn source_frame(f: &TypedSceneWidgetFrame<TreeActivityComponentData>) -> Value {
    let r = f.frame.rect();
    json!({"rect":[r.x,r.y,r.width,r.height],"visible":f.frame.visible().map(|r|[r.x,r.y,r.width,r.height]),"interactive":f.frame.interactive(),"data":source_data(&f.data)})
}

fn tree_data(f: &TreeSvgFrame) -> Value {
    let c = f.config();
    let l = f.layout();
    let activity = f.activity().expect("activity fixture");
    let generator = f.link_generator();
    let path = l.links.first().map(|link| {
        generator
            .svg_path(
                f.inner_position(link.source_position).unwrap(),
                f.inner_position(link.target_position).unwrap(),
            )
            .unwrap()
    });
    json!({"cohort":cohort(f.source(),f.nodes(),f.links(),&l.scene.labels,activity),"config":[c.width,c.height],"margin":[c.margin.left,c.margin.top,c.margin.right,c.margin.bottom],"plot":[l.plot.x,l.plot.y,l.plot.width,l.plot.height],"innerSize":f.inner_size(),"innerPositions":f.nodes().iter().map(|n|f.inner_position(n.position).unwrap()).collect::<Vec<_>>(),"idLookup":f.node_by_id(),"theme":{"colors":f.theme().colors.iter().copied().map(rgba).collect::<Vec<_>>(),"background":rgba(f.theme().background),"text":rgba(f.theme().text),"grid":rgba(f.theme().grid),"fontSize":f.theme().font_size,"font":format!("{:?}",f.theme().font)},"linkStyles":f.computed_link_styles().iter().map(|s|json!({"width":s.thickness,"color":rgba(s.color)})).collect::<Vec<_>>(),"generator":{"curve":format!("{:?}",generator.curve()),"orientation":format!("{:?}",generator.orientation()),"path":path},"marks":l.scene.marks.iter().map(|m|json!({"id":m.id,"label":m.label,"series":m.series,"value":m.value,"bits":m.value.to_bits().to_string(),"fill":m.fill.map(rgba),"opacity":m.opacity,"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>(),"order":l.scene.draw_items().iter().map(|v|format!("{v:?}")).collect::<Vec<_>>(),"extras":f.extras().iter().map(|v|format!("{v:?}")).collect::<Vec<_>>()})
}
fn layer_frame(f: &BrowserTreeSvgLayerFrame) -> Value {
    json!({"key":f.key,"index":f.index,"sourceGeneration":f.source_generation,"publication":f.publication,"view":[f.view.scale,f.view.translate_x,f.view.translate_y],"hidden":f.hidden,"interactive":f.interactive,"data":tree_data(&f.data)})
}
fn tooltip_frame(f: &TreeTooltipFrame) -> Value {
    json!({"id":f.data.datum().id,"index":f.data.source_index(),"datum":datum(f.data.datum()),"value":f.data.node().value,"active":f.data.is_active(),"sourceGeneration":f.source_generation,"pointerClient":f.pointer_client,"viewportAnchor":f.viewport_anchor,"cohort":layout_cohort(f.data.activity_layout())})
}
fn membership(root: &web_sys::Element, class: &str) -> Value {
    let result = (|| -> std::result::Result<Value, JsValue> {
        let Some(n) = root.query_selector(class)? else {
            return Ok(json!({"connected":false,"occurrence":null,"active":false}));
        };
        let occurrence = n
            .closest("[data-tree-svg-key]")?
            .and_then(|p| p.get_attribute("data-tree-svg-key"));
        let outer = n.closest(".kartis-tree-svg-layers")?;
        let active = if let Some(outer) = outer {
            let style = web_sys::window()
                .unwrap()
                .get_computed_style(&outer)?
                .unwrap();
            outer.get_attribute("data-tree-svg-active").as_deref() == Some("true")
                && style.get_property_value("display")? != "none"
                && style.get_property_value("visibility")? != "hidden"
                && outer.get_attribute("aria-hidden").as_deref() != Some("true")
        } else {
            false
        };
        Ok(json!({"connected":n.is_connected(),"occurrence":occurrence,"active":active}))
    })();
    match result {
        Ok(value) => json!({"value":value,"error":null}),
        Err(e) => json!({"value":null,"error":format!("{e:?}")}),
    }
}
fn layer_rows(m: &Metrics, root: &web_sys::Element) -> Value {
    json!(m.layer_rows.borrow().iter().map(|r|{let value=r.frame.try_with_untracked(|v|v.as_ref().map(layer_frame)).map_err(|e|e.to_string());json!({"key":r.key,"born":r.born,"ctor":r.ctor,"initial":layer_frame(&r.initial),"alive":r.frame.is_alive(),"retired":m.retired_layers.borrow().contains(&r.ctor),"membership":membership(root,&format!(".tsl-layer-{0}",r.ctor)),"value":value.as_ref().ok(),"error":value.err()})}).collect::<Vec<_>>())
}
fn source_rows(m: &Metrics, root: &web_sys::Element) -> Value {
    json!(m.source_rows.borrow().iter().map(|r|{let value=r.frame.try_with_untracked(source_frame).map_err(|e|e.to_string());json!({"key":r.key,"born":r.born,"ctor":r.ctor,"alive":r.frame.is_alive(),"retired":m.retired_sources.borrow().contains(&r.ctor),"membership":membership(root,&format!(".tsl-source-{0}",r.ctor)),"value":value.as_ref().ok(),"error":value.err()})}).collect::<Vec<_>>())
}
fn tooltip_rows(m: &Metrics) -> Value {
    json!(m.tooltip_rows.borrow().iter().map(|r|{let value=r.frame.try_with_untracked(|v|v.as_ref().map(tooltip_frame)).map_err(|e|e.to_string());json!({"ctor":r.ctor,"alive":r.frame.is_alive(),"retired":m.retired_tooltips.borrow().contains(&r.ctor),"value":value.as_ref().ok(),"error":value.err()})}).collect::<Vec<_>>())
}
fn public_layers(h: &BrowserTreeHandle) -> Value {
    json!(
        ["under", "over"]
            .into_iter()
            .map(|key| {
                let v = h
                    .svg_layer_frame(key)
                    .map(|f| f.as_ref().map(layer_frame))
                    .map_err(|e| e.to_string());
                (key, json!({"value":v.as_ref().ok(),"error":v.err()}))
            })
            .collect::<BTreeMap<_, _>>()
    )
}
fn observe(m: &Metrics, root: &web_sys::Element, origin: &str, own: Value) {
    let Some(h) = m.owner.borrow().clone() else {
        m.probes.borrow_mut().push(json!({"origin":origin,"own":own,"unavailable":"owner-not-installed","operation":m.operation.get(),"layers":layer_rows(m,root),"sources":source_rows(m,root),"tooltip":tooltip_rows(m)}));
        return;
    };
    let record = match h.snapshot() {
        Ok(s) => {
            let tip = h
                .node_tooltip()
                .map(|v| v.as_ref().map(tooltip_frame))
                .map_err(|e| e.to_string());
            json!({"origin":origin,"own":own,"operation":m.operation.get(),"requestedMode":m.mode.get(),"requestedPhase":m.phase.get(),"requestedRevision":m.revision.get(),"requestedSourceRevision":m.source_revision.get(),"ownerRevision":s.widget_revision,"owner":tree_data(&TreeSvgFrame::with_activity(s.config,s.layout).unwrap()),"publicLayers":public_layers(&h),"publicTooltip":{"value":tip.as_ref().ok(),"error":tip.err()},"layers":layer_rows(m,root),"sources":source_rows(m,root),"tooltip":tooltip_rows(m)})
        }
        Err(e) => {
            json!({"origin":origin,"own":own,"operation":m.operation.get(),"error":e.to_string()})
        }
    };
    m.probes.borrow_mut().push(record);
}
fn graphic() -> GraphicProps {
    GraphicProps {
        width: 8.,
        height: 8.,
        commands: vec![
            PathCommand::Move(GraphicPoint::new(0., 0.)),
            PathCommand::Line(GraphicPoint::new(8., 0.)),
            PathCommand::Line(GraphicPoint::new(8., 8.)),
            PathCommand::Line(GraphicPoint::new(0., 8.)),
            PathCommand::Close,
        ],
        fill: Some(Color(255, 220, 0, 255)),
        ..Default::default()
    }
}
fn panel_position(f: &BrowserTreeSvgLayerFrame, key: &str) -> [f64; 2] {
    let id = if key == "under" { "a" } else { "c" };
    let index = f.data.node_by_id()[id];
    let p = f
        .data
        .inner_position(f.data.nodes()[index].position)
        .unwrap();
    [p[0] - 24., p[1] - 20.]
}
fn whole_layer_factory(
    m: Rc<Metrics>,
    root: web_sys::Element,
) -> impl FnMut(BrowserTreeSvgLayerContext) -> Result<Box<dyn Widget>> {
    move |c| {
        m.layer_factories.set(m.layer_factories.get() + 1);
        let ctor = m.layer_factories.get();
        if m.fault.get() == 2 && c.key == "over" {
            return Err(ChartError("svg layer factory rejected".into()));
        }
        if m.fault.get() == 8
            && let Some(h) = m.owner.borrow().clone()
        {
            h.stop();
        }
        let key = c.key;
        let born = c.revision;
        let initial = c.initial;
        let frame = c.frame;
        let current = c.current;
        let m = m.clone();
        let root = root.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.layer_bodies.set(m.layer_bodies.get() + 1);
            m.layer_rows.borrow_mut().push(LayerRow {
                key: key.clone(),
                born,
                ctor,
                initial: initial.clone(),
                frame,
                current: current.clone(),
            });
            let cleaned = m.clone();
            scope.on_cleanup(move || {
                cleaned.layer_cleanups.set(cleaned.layer_cleanups.get() + 1);
                cleaned.retired_layers.borrow_mut().insert(ctor);
            })?;
            let count = scope.cage(0u32);
            let focus = scope.cage(false);
            let ordinal = Rc::new(Cell::new(0u32));
            let watched = m.clone();
            let observed_root = root.clone();
            let observed_key = key.clone();
            let observed_initial = initial.clone();
            scope.effect(move||{let own=frame.get();let index=ordinal.get();ordinal.set(index+1);observe(&watched,&observed_root,"layer-effect",json!({"key":observed_key,"born":born,"ctor":ctor,"ordinal":index,"initial":layer_frame(&observed_initial),"frame":own.as_ref().map(layer_frame)}));});
            let style_initial = initial.clone();
            let style_key = key.clone();
            let styled = m.clone();
            let label_key = key.clone();
            let initial_label = initial.clone();
            let activated = m.clone();
            let active_current = current.clone();
            let active_key = key.clone();
            let line_initial = initial.clone();
            let line = glory::ui::graphic()
                .class("tsl-generated-link")
                .drawing(scope.bond(move || {
                    let f = frame.get().unwrap_or_else(|| line_initial.clone());
                    let size = f.data.inner_size();
                    let commands = f
                        .data
                        .links()
                        .first()
                        .map(|l| {
                            f.data
                                .link_generator()
                                .geometry(
                                    f.data.inner_position(l.source_position).unwrap(),
                                    f.data.inner_position(l.target_position).unwrap(),
                                )
                                .unwrap()
                                .commands()
                        })
                        .unwrap_or_default();
                    GraphicProps {
                        width: size[0],
                        height: size[1],
                        commands,
                        stroke: Some(GraphicStroke {
                            color: Color(0, 180, 200, 255),
                            width: 2.,
                            cap: StrokeCap::Butt,
                        }),
                        ..Default::default()
                    }
                }))
                .style(
                    Style::new()
                        .position(Position::Absolute)
                        .left(Length::Px(0.))
                        .top(Length::Px(0.))
                        .width(Length::Percent(100.))
                        .height(Length::Percent(100.)),
                );
            let button=glory::ui::button().class(format!("tsl-layer-button tsl-layer-{ctor} tsl-key-{key}")).style(scope.bond(move||{let f=frame.get().unwrap_or_else(||style_initial.clone());let p=panel_position(&f,&style_key);let bad=styled.fault.get()==5&&style_key=="over"&&f.data.source().iter().any(|d|d.id=="c"&&d.value==35.);Style{background:Some(f.data.theme().colors[0]),..Style::new().position(Position::Absolute).left(Length::Px(p[0])).top(Length::Px(p[1])).width(Length::Px(if bad{f64::NAN}else{48.})).height(Length::Px(40.))}})).label(scope.bond(move||json!({"key":label_key,"born":born,"ctor":ctor,"count":count.get(),"focus":focus.get(),"initial":layer_frame(&initial_label),"frame":frame.get().as_ref().map(layer_frame)}).to_string())).fill(glory::ui::graphic().class("tsl-layer-dot").drawing(graphic()).style(Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(8.)).height(Length::Px(8.)))).on(EventKind::Activate,move |_|{count.revise(|v|*v+=1);let action=activated.cap_action.get();if action!=0{let result=active_current.set_current_node(match action{1=>Some("a"),2=>None,_=>Some("absent")});let owner=activated.owner.borrow().as_ref().map(|h|h.snapshot().map(|s|layout_cohort(&s.layout)).map_err(|e|e.to_string()));activated.cap_events.borrow_mut().push(json!({"key":active_key,"action":action,"ok":result.is_ok(),"error":result.err().map(|e|e.to_string()),"owner":owner.as_ref().and_then(|v|v.as_ref().ok()),"ownerError":owner.and_then(|v|v.err())}));}}).on(EventKind::FocusChanged,move|e|if let UiEvent::FocusChanged(value)=e{focus.revise(|v|*v=value);});
            glory::ui::view()
                .class(format!("tsl-whole tsl-whole-{ctor}"))
                .style(
                    Style::new()
                        .position(Position::Relative)
                        .width(Length::Percent(100.))
                        .height(Length::Percent(100.)),
                )
                .fill(line)
                .fill(button)
                .show_in(scope)?;
            if m.fault.get() == 3 && key == "over" {
                return Err(
                    glory::render::Error::InvalidProps("svg layer child build rejected").into(),
                );
            }
            Ok(())
        })) as Box<dyn Widget>)
    }
}

fn source_factory(
    m: Rc<Metrics>,
    root: web_sys::Element,
) -> impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<Box<dyn Widget>>
{
    move |c| {
        m.source_factories.set(m.source_factories.get() + 1);
        let ctor = m.source_factories.get();
        let key = c.key;
        if m.fault.get() == 6 && key == "tree-label:d" {
            return Err(ChartError("svg source factory rejected".into()));
        }
        let born = c.revision;
        let frame = c.frame;
        let m = m.clone();
        let root = root.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.source_rows.borrow_mut().push(SourceRow {
                key: key.clone(),
                born,
                ctor,
                frame,
            });
            let cleaned = m.clone();
            scope.on_cleanup(move || {
                cleaned
                    .source_cleanups
                    .set(cleaned.source_cleanups.get() + 1);
                cleaned.retired_sources.borrow_mut().insert(ctor);
            })?;
            let count = scope.cage(0u32);
            let watched = m.clone();
            let observed_root = root.clone();
            let observed_key = key.clone();
            scope.effect(move||observe(&watched,&observed_root,"source-effect",json!({"key":observed_key,"born":born,"ctor":ctor,"frame":source_frame(&frame.get())})));
            let label_key = key.clone();
            glory::ui::button().class(format!("tsl-source-button tsl-source-{ctor}")).style(scope.bond(move||Style{background:Some(if frame.with(|v|v.data.is_active)==Some(true){Color(0,100,255,255)}else{Color(100,20,180,255)}),..Style::new().position(Position::Relative).width(Length::Percent(100.)).height(Length::Percent(100.))})).label(scope.bond(move||json!({"key":label_key,"born":born,"ctor":ctor,"count":count.get(),"frame":source_frame(&frame.get())}).to_string())).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).show_in(scope)?;
            if m.fault.get() == 7 && key == "node:b" {
                return Err(
                    glory::render::Error::InvalidProps("svg source child build rejected").into(),
                );
            }
            Ok(())
        })) as Box<dyn Widget>)
    }
}
fn program(
    mode: u8,
    revision: u64,
    budget: Option<usize>,
    omit_extras: bool,
    m: Rc<Metrics>,
    root: web_sys::Element,
) -> Result<BrowserTreeSvgProgram> {
    let mut p = BrowserTreeSvgProgram::new(specs(mode)?)?
        .custom("panel", revision, whole_layer_factory(m, root))?
        .interactive(mode != 8)
        .debug_mesh(mode == 9);
    if let Some(components) = budget {
        p = p.with_budget(TreeSvgBudget {
            components,
            ..Default::default()
        })?;
    }
    if omit_extras {
        p = p.with_extras(TreeSvgExtrasPolicy::Omit);
    }
    Ok(p)
}
fn install_tooltip(f: &Fixture) -> Result<()> {
    f.owner
        .handle()
        .set_node_tooltip_options(BrowserTreeTooltipOptions::default())?;
    let m = f.m.clone();
    let root = f.root.clone();
    f.owner.handle().set_node_tooltip_component(0,move|c:BrowserTreeTooltipComponentContext|{m.tooltip_factories.set(m.tooltip_factories.get()+1);let ctor=m.tooltip_factories.get();let frame=c.frame;let m=m.clone();let root=root.clone();Ok(glory::ui::widget(move|scope|{m.tooltip_rows.borrow_mut().push(TooltipRow{ctor,frame});let cleaned=m.clone();scope.on_cleanup(move||{cleaned.tooltip_cleanups.set(cleaned.tooltip_cleanups.get()+1);cleaned.retired_tooltips.borrow_mut().insert(ctor);})?;let watched=m.clone();let observed_root=root.clone();let ordinal=Rc::new(Cell::new(0u32));scope.effect(move||{let own=frame.get();let index=ordinal.get();ordinal.set(index+1);observe(&watched,&observed_root,"tooltip-effect",json!({"ctor":ctor,"ordinal":index,"frame":own.as_ref().map(tooltip_frame)}));});glory::ui::button().class(format!("tsl-tooltip tsl-tooltip-{ctor}")).style(Style{background:Some(Color(80,0,160,255)),..Style::new().position(Position::Relative).width(Length::Px(96.)).height(Length::Px(32.))}).label(scope.bond(move||json!({"ctor":ctor,"frame":frame.get().as_ref().map(tooltip_frame)}).to_string())).fill(glory::ui::graphic().class("tsl-tooltip-dot").drawing(graphic()).style(Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(8.)).height(Length::Px(8.)))).show_in(scope)}))})
}

#[wasm_bindgen]
pub fn tree_svg_layers_mount(
    key: &str,
    picking: u8,
    components: u8,
) -> std::result::Result<(), JsValue> {
    if key.is_empty()
        || key.len() > 32
        || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || picking > 1
        || components > 2
    {
        return Err(js("invalid Tree SVG fixture profile"));
    }
    tree_svg_layers_stop(key)?;
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("section")?;
    root.set_id(&format!("tree-svg-layers-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:480px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = doc.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-svg-layers-{key} .tsl-layer-button,#tree-svg-layers-{key} .tsl-source-button,#tree-svg-layers-{key} .tsl-tooltip{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;overflow:hidden;min-width:0;box-shadow:none}}#tree-svg-layers-{key} .tsl-layer-button,#tree-svg-layers-{key} .tsl-source-button{{pointer-events:auto!important}}#tree-svg-layers-{key} .tsl-generated-link{{pointer-events:none!important}}")));
    doc.body().unwrap().append_child(&css)?;
    doc.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let built = m.clone();
    let build = move |c: &BrowserTreeSvgBuildContext<'_>| {
        built.builds.set(built.builds.get() + 1);
        if built.fault.get() == 1 {
            return Err(ChartError("svg layer builder rejected".into()));
        }
        let phase = built.phase.get();
        let size_m = built.clone();
        let color_m = built.clone();
        let thickness_m = built.clone();
        let link_color_m = built.clone();
        let label_m = built.clone();
        let mut size = move |n: &TreeNodeFunctionContext<'_>| {
            size_m.functions.set(size_m.functions.get() + 1);
            size_m
                .calls
                .borrow_mut()
                .push(json!({"role":"size","id":n.datum.id}));
            Ok(if phase == 3 && n.datum.id == "c" {
                0.
            } else {
                24. + 4. * n.depth as f64
            })
        };
        let mut color = move |n: &TreeNodeFunctionContext<'_>| {
            color_m.functions.set(color_m.functions.get() + 1);
            color_m
                .calls
                .borrow_mut()
                .push(json!({"role":"color","id":n.datum.id}));
            Ok(match n.datum.id.as_str() {
                "r" => Color(160, 80, 30, 255),
                "a" => Color(0, 100, 255, 255),
                "b" => Color(0, 200, 0, 255),
                "c" => Color(230, 110, 20, 255),
                _ => Color(100, 20, 180, 255),
            })
        };
        let mut thickness = move |l: &TreeLinkFunctionContext<'_>| {
            thickness_m.functions.set(thickness_m.functions.get() + 1);
            thickness_m
                .calls
                .borrow_mut()
                .push(json!({"role":"thickness","id":l.source[l.link.target_index].id}));
            Ok(6.)
        };
        let mut link_color = move |l: &TreeLinkFunctionContext<'_>| {
            link_color_m.functions.set(link_color_m.functions.get() + 1);
            link_color_m
                .calls
                .borrow_mut()
                .push(json!({"role":"linkColor","id":l.source[l.link.target_index].id}));
            Ok(Color(230, 110, 20, 255))
        };
        let mut label = move |l: &TreeLabelFunctionContext<'_>| {
            label_m.functions.set(label_m.functions.get() + 1);
            label_m.label_calls.set(label_m.label_calls.get() + 1);
            label_m
                .calls
                .borrow_mut()
                .push(json!({"role":"label","id":l.datum.id}));
            Ok(format!("paint-{}", l.datum.id))
        };
        let mut layout = tree_svg_layout_with_activity(
            c.config,
            c.source,
            c.tree,
            c.layers,
            &mut TreeFactories {
                node_size: Some(&mut size),
                node_color: Some(&mut color),
                link_thickness: Some(&mut thickness),
                link_color: Some(&mut link_color),
                label: Some(&mut label),
            },
            c.activity,
            &mut TreeActivityFactories::default(),
        )?;
        for mark in &mut layout.layout.scene.marks {
            if phase == 2 && (mark.id.starts_with("node:") || mark.id.starts_with("link:")) {
                mark.opacity = 0.5;
            }
            if phase == 3 && mark.id == "node:a" {
                mark.opacity = 0.;
            }
        }
        if phase == 7 {
            layout.layout.scene.marks.push(Mark::new(
                "decorative-extra",
                Geometry::Rect {
                    x: 300.,
                    y: 238.,
                    width: 16.,
                    height: 12.,
                },
                Color(0, 180, 200, 255),
            ));
        }
        Ok(layout)
    };
    let placed = m.clone();
    let placement = move |c: &TreeActivityComponentContext<'_>| {
        placed.placements.set(placed.placements.get() + 1);
        if placed.fault.get() == 4 {
            return Err(ChartError("svg source placement rejected".into()));
        }
        Ok(match c.component {
            TreeComponentContext::Node(n) if n.datum.id == "b" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-12., 24.],
                    measured: TextSize {
                        width: 80.,
                        height: 20.,
                    },
                })
            }
            TreeComponentContext::Link(l) if l.layout.source()[l.link.target_index].id == "c" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-40., -24.],
                    measured: TextSize {
                        width: 80.,
                        height: 20.,
                    },
                })
            }
            TreeComponentContext::Label(l) if l.datum.id == "d" => {
                Some(PlotWidgetPlacement::AtDatum {
                    offset: [-24., 18.],
                    measured: TextSize {
                        width: 48.,
                        height: 18.,
                    },
                })
            }
            _ => None,
        })
    };
    let opts = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer: BrowserRenderer::Svg,
            legend: false,
            tooltip: true,
            highlight: BrowserHighlight::Controlled,
            motion: MotionPreference::Reduced,
            ..Default::default()
        },
        tree: tree_options(),
        activity: TreeActivityOptions {
            highlight: TreeRelationOptions {
                ancestor_nodes: true,
                ancestor_links: true,
                ..Default::default()
            },
            ..Default::default()
        },
        picking: if picking == 0 {
            BrowserTreePicking::Geometry
        } else {
            BrowserTreePicking::Nearest { distance: 64. }
        },
    };
    let p = program(0, 0, None, false, m.clone(), root.clone()).map_err(js)?;
    let owner = if components == 0 {
        BrowserTree::mount_with_factory_and_svg_layers(
            root.clone(),
            raw(0),
            config(480., 320., false),
            opts,
            p,
            build,
        )
    } else {
        BrowserTree::mount_with_factory_and_svg_layers_and_components(
            root.clone(),
            raw(0),
            config(480., 320., false),
            opts,
            p,
            build,
            BrowserTreeWidgetOptions {
                revision: 0,
                replace_ink: components == 2,
                source_ordered: true,
            },
            placement,
            source_factory(m.clone(), root.clone()),
        )
    }
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(owner.handle());
    let events = Rc::new(RefCell::new(vec![]));
    let ev = events.clone();
    owner.on_event(move |event| ev.borrow_mut().push(format!("{event:?}")));
    let fixture = Fixture {
        root,
        css,
        owner,
        m,
        picking,
        components,
        events,
    };
    fixture.m.operation.set(1);
    install_tooltip(&fixture).map_err(js)?;
    fixture.owner.flush().map_err(js)?;
    fixture.m.operation.set(0);
    FIXTURES.with_borrow_mut(|all| {
        all.insert(key.into(), fixture);
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with_borrow(|all| f(all.get(key).ok_or_else(|| js("missing Tree SVG fixture"))?))
}
#[wasm_bindgen]
pub fn tree_svg_layers_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        let s = h.snapshot().map_err(js)?;
        let data = TreeSvgFrame::with_activity(s.config.clone(), s.layout.clone()).map_err(js)?;
        Ok(json!({"owner":tree_data(&data),"state":{"active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"desiredFocus":s.desired_focus,"error":s.error.map(|e|e.to_string()),"revision":s.widget_revision,"chart":s.chart.map(|c|json!({"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"hovered":c.hovered,"selected":c.selected,"hidden":c.hidden,"revision":c.revision}))},"publicLayers":public_layers(&h),"publicTooltip":h.node_tooltip().map_err(js)?.as_ref().map(tooltip_frame),"layers":layer_rows(&f.m,&f.root),"sources":source_rows(&f.m,&f.root),"tooltip":tooltip_rows(&f.m),"metrics":{"phase":f.m.phase.get(),"mode":f.m.mode.get(),"theme":f.m.theme.get(),"revision":f.m.revision.get(),"sourceRevision":f.m.source_revision.get(),"builds":f.m.builds.get(),"functions":f.m.functions.get(),"labelCalls":f.m.label_calls.get(),"placements":f.m.placements.get(),"layerFactories":f.m.layer_factories.get(),"layerBodies":f.m.layer_bodies.get(),"sourceFactories":f.m.source_factories.get(),"tooltipFactories":f.m.tooltip_factories.get(),"layerCleanups":f.m.layer_cleanups.get(),"sourceCleanups":f.m.source_cleanups.get(),"tooltipCleanups":f.m.tooltip_cleanups.get()},"picking":f.picking,"components":f.components,"calls":f.m.calls.borrow().clone(),"capEvents":f.m.cap_events.borrow().clone(),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_probes(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| Ok(json!(f.m.probes.borrow().clone()).to_string()))
}
#[wasm_bindgen]
pub fn tree_svg_layers_program(
    key: &str,
    mode: u8,
    revision: u64,
    budget: i32,
    omit_extras: bool,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let p = program(
            mode,
            revision,
            (budget >= 0).then_some(budget as usize),
            omit_extras,
            f.m.clone(),
            f.root.clone(),
        )
        .map_err(js)?;
        f.m.mode.set(mode);
        f.m.revision.set(revision);
        f.m.operation.set(3);
        let r = f.owner.handle().set_svg_program(p).map_err(js);
        f.m.operation.set(0);
        r
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        if phase > 7 {
            return Err(js("invalid SVG source phase"));
        }
        f.m.phase.set(phase);
        f.owner.handle().set_source(raw(phase)).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_config(
    key: &str,
    width: f64,
    height: f64,
    theme: bool,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let c = config(width, height, theme);
        c.validate().map_err(js)?;
        f.m.theme.set(theme);
        f.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))?;
        if let Some(surface) = f.root.query_selector(".kartis-surface")? {
            surface
                .unchecked_ref::<web_sys::HtmlElement>()
                .style()
                .set_property("height", &format!("{height}px"))?;
        }
        f.owner.handle().set_config(c).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_failure(key: &str, fault: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        if fault > 8 {
            return Err(js("invalid SVG fixture fault"));
        }
        f.m.fault.set(fault);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_cap_action(key: &str, action: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.m.cap_action.set(action);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_capability(
    key: &str,
    ctor: u32,
    target: &str,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let current =
            f.m.layer_rows
                .borrow()
                .iter()
                .find(|r| r.ctor == ctor)
                .map(|r| r.current.clone())
                .ok_or_else(|| js("missing layer capability"))?;
        current
            .set_current_node((!target.is_empty()).then_some(target))
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.m.source_revision.set(revision);
        f.owner.handle().set_widget_revision(revision).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_hidden(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner
            .handle()
            .set_hidden(if id.is_empty() {
                BTreeSet::new()
            } else {
                [id.to_owned()].into_iter().collect()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_zoom(key: &str, on: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.reset_view().map_err(js)?;
        if on {
            let s = h.snapshot().map_err(js)?;
            h.dispatch(SurfaceInput::Wheel {
                point: (s.config.width / 2., s.config.height / 2.),
                delta: -1.1f64.ln() / 0.002,
            })
            .map_err(js)?;
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_svg_layers_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_svg_layers_refresh(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.refresh().map_err(js))
}
#[wasm_bindgen]
pub fn tree_svg_layers_stop(key: &str) -> std::result::Result<bool, JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        let Some(f) = all.remove(key) else {
            return Ok(false);
        };
        f.owner.stop();
        let retired = |alive: bool, result: std::result::Result<(), glory::core::AccessError>| {
            !alive
                && matches!(
                    result,
                    Err(glory::core::AccessError::Disposed | glory::core::AccessError::RuntimeGone)
                )
        };
        for row in f.m.layer_rows.borrow().iter() {
            if !f.m.retired_layers.borrow().contains(&row.ctor)
                || !retired(row.frame.is_alive(), row.frame.try_with_untracked(|_| ()))
            {
                return Err(js("SVG stop left a layer Scope live"));
            }
            if row.current.set_current_node(Some("a")).is_ok() {
                return Err(js("SVG stop left a layer current capability live"));
            }
        }
        for row in f.m.source_rows.borrow().iter() {
            if !f.m.retired_sources.borrow().contains(&row.ctor)
                || !retired(row.frame.is_alive(), row.frame.try_with_untracked(|_| ()))
            {
                return Err(js("SVG stop left a source Scope live"));
            }
        }
        for row in f.m.tooltip_rows.borrow().iter() {
            if !f.m.retired_tooltips.borrow().contains(&row.ctor)
                || !retired(row.frame.is_alive(), row.frame.try_with_untracked(|_| ()))
            {
                return Err(js("SVG stop left a tooltip Scope live"));
            }
        }
        if f.m.layer_cleanups.get() as usize != f.m.layer_rows.borrow().len()
            || f.m.source_cleanups.get() as usize != f.m.source_rows.borrow().len()
            || f.m.tooltip_cleanups.get() as usize != f.m.tooltip_rows.borrow().len()
            || f.owner.handle().node_tooltip().map_err(js)?.is_some()
            || f.root
                .query_selector(".kartis-tree-svg-layers,.kartis-tree-tooltip")?
                .is_some()
        {
            return Err(js("SVG stop did not retire all constructed instances"));
        }
        f.root.remove();
        f.css.remove();
        Ok(true)
    })
}
