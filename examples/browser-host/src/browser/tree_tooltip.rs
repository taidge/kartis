//! Explicit managed Tree tooltip acceptance; legacy mounts are not changed.
use std::collections::BTreeSet;

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeCanvasProgram,
    BrowserTreeHandle, BrowserTreeOptions, BrowserTreePicking, BrowserTreeTooltipAnchor,
    BrowserTreeTooltipComponentContext, BrowserTreeTooltipOptions, BrowserTreeTooltipPosition,
    BrowserTreeWidgetOptions, MotionPreference, TreeTooltipFrame,
};

use super::*;

type SourceBond = glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>;
type TooltipBond = glory::Bond<Option<TreeTooltipFrame>>;
struct SourceRow {
    born: u64,
    key: String,
    ctor: u32,
    frame: SourceBond,
}
struct TooltipRow {
    born: u64,
    ctor: u32,
    frame: TooltipBond,
}
#[derive(Default)]
struct Metrics {
    phase: Cell<u8>,
    fault: Cell<u8>,
    builds: Cell<u32>,
    functions: Cell<u32>,
    raw_calls: Cell<u32>,
    source_factories: Cell<u32>,
    source_cleanups: Cell<u32>,
    tooltip_factories: Cell<u32>,
    tooltip_cleanups: Cell<u32>,
    operation: Cell<u8>,
    requested_tooltip_revision: Cell<u64>,
    owner: RefCell<Option<BrowserTreeHandle>>,
    source_rows: RefCell<Vec<SourceRow>>,
    tooltip_rows: RefCell<Vec<TooltipRow>>,
    retired_source: RefCell<BTreeSet<u32>>,
    retired_tooltip: RefCell<BTreeSet<u32>>,
    observations: RefCell<Vec<Value>>,
}
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    m: Rc<Metrics>,
    profile: u8,
    empty_text: bool,
    events: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    [
        ("a", Some("r"), 0.),
        ("c", Some("a"), if phase == 1 { 35. } else { 30. }),
        ("r", None, 0.),
        ("b", Some("r"), if phase == 6 { 0. } else { -0. }),
        ("d", Some("a"), 10.),
    ]
    .into_iter()
    .map(|(id, parent, value)| {
        let mut d = HierarchyDatum::new(id, parent.map(str::to_owned), value);
        d.label = if (phase == 4 || phase == 5) && id == "b" {
            String::new()
        } else {
            format!("source-{id}")
        };
        d
    })
    .collect()
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 720.,
        height: 340.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
        },
        ..Default::default()
    }
}
fn tree_options() -> TreeOptions {
    let mut o = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    o.style.labels.position = TreeLabelPosition::Center;
    o.style.labels.size = Some(12.);
    o.style.links.curve = TreeLinkCurve::Straight;
    o.style.links.color = TreeLinkColor::Fixed(Color(230, 110, 20, 255));
    o.style.links.width = 6.;
    o
}
fn tooltip_text_options(empty: bool) -> TooltipOptions {
    TooltipOptions {
        lines: if empty {
            vec![vec![TooltipPart::Text(String::new())]]
        } else {
            vec![vec![
                TooltipPart::Id,
                TooltipPart::Text("|".into()),
                TooltipPart::Label,
                TooltipPart::Text("|".into()),
                TooltipPart::Value,
            ]]
        },
        ..Default::default()
    }
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
fn tooltip_frame(f: &TreeTooltipFrame, empty: bool) -> Value {
    json!({"id":f.data.datum().id,"index":f.data.source_index(),"datum":datum(f.data.datum()),"nodeValue":f.data.node().value,"nodeRadius":f.data.node().radius,"active":f.data.is_active(),"sourceGeneration":f.source_generation,"pointerClient":f.pointer_client,"viewportAnchor":f.viewport_anchor,"anchor":format!("{:?}",f.anchor),"text":f.data.text(&tooltip_text_options(empty)).unwrap(),"cohort":layout_cohort(f.data.activity_layout())})
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
fn source_rows(m: &Metrics, revision: u64) -> Value {
    let mut result = BTreeMap::new();
    for r in m
        .source_rows
        .borrow()
        .iter()
        .filter(|r| r.born == revision && !m.retired_source.borrow().contains(&r.ctor))
    {
        let value = r
            .frame
            .try_with_untracked(source_frame)
            .map_err(|e| e.to_string());
        result.insert(
            r.key.clone(),
            json!({"born":r.born,"ctor":r.ctor,"alive":r.frame.is_alive(),"value":value.as_ref().ok(),"error":value.err()}),
        );
    }
    json!(result)
}
fn active_tooltip_record(root: &web_sys::Element, ctor: u32) -> std::result::Result<bool, JsValue> {
    let Some(node) = root.query_selector(&format!(".kartis-tree-tooltip .tt-tip-{ctor}"))? else {
        return Ok(false);
    };
    let mut parent = Some(node);
    while let Some(node) = parent {
        if node.get_attribute("class").is_some_and(|s| {
            s.split_whitespace()
                .any(|s| s == "kartis-tree-tooltip-component")
        }) {
            return Ok(web_sys::window()
                .unwrap()
                .get_computed_style(&node)?
                .is_some_and(|s| s.get_property_value("display").is_ok_and(|s| s != "none")));
        }
        parent = node.parent_element();
    }
    Ok(false)
}
fn tooltip_rows(m: &Metrics, root: &web_sys::Element, empty: bool) -> Value {
    json!(m.tooltip_rows.borrow().iter().map(|r|{let retired=m.retired_tooltip.borrow().contains(&r.ctor);let active=active_tooltip_record(root,r.ctor).map_err(|e|format!("{e:?}"));let value=r.frame.try_with_untracked(|f|f.as_ref().map(|f|tooltip_frame(f,empty))).map_err(|e|e.to_string());json!({"born":r.born,"ctor":r.ctor,"alive":r.frame.is_alive(),"retired":retired,"activeRecord":{"value":active.as_ref().ok(),"error":active.err()},"value":value.as_ref().ok(),"error":value.err()})}).collect::<Vec<_>>())
}
fn observe(m: &Metrics, root: &web_sys::Element, empty: bool, origin: &str, own: Value) {
    let Some(handle) = m.owner.borrow().clone() else {
        m.observations.borrow_mut().push(json!({"origin":origin,"own":own,"unavailable":"owner-not-installed","operation":m.operation.get()}));
        return;
    };
    let record = match handle.snapshot() {
        Ok(s) => {
            let tooltip = handle
                .node_tooltip()
                .map(|f| f.as_ref().map(|f| tooltip_frame(f, empty)))
                .map_err(|e| e.to_string());
            json!({"origin":origin,"own":own,"operation":m.operation.get(),"requestedPhase":m.phase.get(),"requestedTooltipRevision":m.requested_tooltip_revision.get(),"ownerRevision":s.widget_revision,"owner":layout_cohort(&s.layout),"publicTooltip":{"value":tooltip.as_ref().ok(),"error":tooltip.err()},"sourceRows":source_rows(m,s.widget_revision),"tooltipRows":tooltip_rows(m,root,empty)})
        }
        Err(e) => {
            json!({"origin":origin,"own":own,"error":e.to_string(),"operation":m.operation.get(),"rootConnected":root.is_connected()})
        }
    };
    m.observations.borrow_mut().push(record);
}
fn source_widget_factory(
    m: Rc<Metrics>,
    root: web_sys::Element,
    empty: bool,
) -> impl FnMut(SceneWidgetCompositionContext<TreeActivityComponentData>) -> Result<Box<dyn glory::Widget>>
{
    move |c| {
        m.source_factories.set(m.source_factories.get() + 1);
        let ctor = m.source_factories.get();
        let key = c.key;
        let born = c.revision;
        let frame = c.frame;
        let m = m.clone();
        let root = root.clone();
        Ok(Box::new(glory::ui::widget(move |scope| {
            m.source_rows.borrow_mut().push(SourceRow {
                born,
                key: key.clone(),
                ctor,
                frame,
            });
            let cleaned = m.clone();
            scope.on_cleanup(move || {
                cleaned
                    .source_cleanups
                    .set(cleaned.source_cleanups.get() + 1);
                cleaned.retired_source.borrow_mut().insert(ctor);
            })?;
            let count = scope.cage(0u32);
            let watched = m.clone();
            let observed_root = root.clone();
            let observed_key = key.clone();
            scope.effect(move || {
                let own = frame.get();
                observe(
                    &watched,
                    &observed_root,
                    empty,
                    "source-effect",
                    json!({"key":observed_key,"born":born,"ctor":ctor,"frame":source_frame(&own)}),
                );
            });
            let label_key = key.clone();
            glory::ui::button().class(format!("tt-source-widget tt-source-{ctor}")).style(scope.bond(move||frame.with(|f|Style{background:Some(if f.data.is_active==Some(true){Color(0,100,255,255)}else{Color(100,20,180,255)}),..Style::new().position(Position::Relative).width(Length::Percent(100.)).height(Length::Percent(100.))}))).label(scope.bond(move||json!({"key":label_key,"born":born,"ctor":ctor,"count":count.get(),"frame":source_frame(&frame.get())}).to_string())).fill(glory::ui::text().content("").class("tt-source-dot").style(Style{background:Some(Color(255,220,0,255)),..Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(4.)).height(Length::Px(4.))})).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).show_in(scope)
        })) as Box<dyn glory::Widget>)
    }
}
fn install_component(f: &Fixture, revision: u64) -> Result<()> {
    let m = f.m.clone();
    let root = f.root.clone();
    let empty = f.empty_text;
    f.m.requested_tooltip_revision.set(revision);
    f.m.operation.set(1);
    let result=f.owner.handle().set_node_tooltip_component(revision,move|c:BrowserTreeTooltipComponentContext|{m.tooltip_factories.set(m.tooltip_factories.get()+1);if m.fault.get()==3{return Err(ChartError("tooltip factory rejected".into()));}let ctor=m.tooltip_factories.get();let born=c.revision;let frame=c.frame;let m=m.clone();let root=root.clone();Ok(glory::ui::widget(move|scope|{m.tooltip_rows.borrow_mut().push(TooltipRow{born,ctor,frame});let cleaned=m.clone();scope.on_cleanup(move||{cleaned.tooltip_cleanups.set(cleaned.tooltip_cleanups.get()+1);cleaned.retired_tooltip.borrow_mut().insert(ctor);})?;let count=scope.cage(0u32);let focused=scope.cage(false);let effects=Rc::new(Cell::new(0u32));let watched=m.clone();let observed_root=root.clone();scope.effect(move||{let own=frame.get();let ordinal=effects.get();effects.set(ordinal+1);observe(&watched,&observed_root,empty,"tooltip-effect",json!({"born":born,"ctor":ctor,"ordinal":ordinal,"frame":own.as_ref().map(|f|tooltip_frame(f,empty))}));});let styled=m.clone();
        glory::ui::button().class(format!("tt-tooltip-widget tt-tip-{ctor}")).style(scope.bond(move||{let value=frame.get();let large=value.as_ref().is_some_and(|v|v.data.source().iter().any(|d|d.id=="c"&&d.value==35.));let mut width=if large{130.}else{100.};if large&&styled.fault.get()==5{width=f64::NAN;}Style{background:Some(Color(100,20,180,255)),..Style::new().position(Position::Relative).width(Length::Px(width)).height(Length::Px(if large{48.}else{40.}))}})).label(scope.bond(move||json!({"born":born,"ctor":ctor,"count":count.get(),"focus":focused.get(),"frame":frame.get().as_ref().map(|f|tooltip_frame(f,empty))}).to_string())).fill(glory::ui::text().content("").class("tt-tooltip-dot").style(Style{background:Some(Color(255,220,0,255)),..Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(4.)).width(Length::Px(8.)).height(Length::Px(8.))})).on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move|e|if let UiEvent::FocusChanged(v)=e{focused.revise(|old|*old=v);}).show_in(scope)
    }))}).and_then(|_|f.owner.flush());
    f.m.operation.set(0);
    result
}
fn program(m: Rc<Metrics>) -> BrowserTreeCanvasProgram {
    BrowserTreeCanvasProgram {
        render_node: Some(Box::new(move |ctx, c| {
            m.raw_calls.set(m.raw_calls.get() + 1);
            ctx.save();
            let result = (|| {
                let opacity = c.mark.map_or(1., |m| m.opacity);
                ctx.set_global_alpha(opacity);
                ctx.begin_path();
                ctx.arc(
                    c.node.position[0],
                    c.node.position[1],
                    c.node.radius,
                    0.,
                    std::f64::consts::TAU,
                )
                .map_err(|e| ChartError(format!("tooltip raw native host failed: {e:?}")))?;
                ctx.set_fill_style_str(&render::css_color(c.node.color.unwrap()));
                ctx.fill();
                if m.fault.get() == 4 && c.datum.id == "a" {
                    return Err(ChartError("tooltip native painter rejected".into()));
                }
                Ok(())
            })();
            ctx.restore();
            result
        })),
        ..Default::default()
    }
}

#[wasm_bindgen]
pub fn tree_tooltip_mount(
    key: &str,
    profile: u8,
    empty_text: bool,
) -> std::result::Result<(), JsValue> {
    if key.is_empty()
        || key.len() > 32
        || !key.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        || profile > 7
    {
        return Err(js("invalid Tree tooltip profile"));
    }
    tree_tooltip_stop(key)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    root.set_id(&format!("tree-tooltip-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:720px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-tooltip-{key} .tt-source-widget,#tree-tooltip-{key} .tt-tooltip-widget{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;overflow:hidden;min-width:0;box-shadow:none}}#tree-tooltip-{key} .tt-source-widget{{pointer-events:auto!important}}")));
    document.body().unwrap().append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let built = m.clone();
    let build = move |c: &BrowserTreeBuildContext<'_>| {
        built.builds.set(built.builds.get() + 1);
        if built.fault.get() == 1 {
            return Err(ChartError("tooltip source builder rejected".into()));
        }
        let phase = built.phase.get();
        let size_m = built.clone();
        let color_m = built.clone();
        let text_m = built.clone();
        let mut size = move |n: &TreeNodeFunctionContext<'_>| {
            size_m.functions.set(size_m.functions.get() + 1);
            Ok(if (phase == 2 || phase == 5) && n.datum.id == "c" {
                0.
            } else {
                24. + 4. * n.depth as f64
            })
        };
        let mut color = move |n: &TreeNodeFunctionContext<'_>| {
            color_m.functions.set(color_m.functions.get() + 1);
            Ok(match n.datum.id.as_str() {
                "r" => Color(160, 80, 30, 255),
                "a" => Color(0, 100, 255, 255),
                "b" => Color(0, 200, 0, 255),
                "c" => Color(230, 110, 20, 255),
                _ => Color(100, 20, 180, 255),
            })
        };
        let mut label = move |n: &TreeLabelFunctionContext<'_>| {
            text_m.functions.set(text_m.functions.get() + 1);
            Ok(format!("paint-{}", n.datum.id))
        };
        let mut layout = tree_canvas_layout_with_activity(
            c.config,
            c.source,
            c.tree,
            &mut TreeFactories {
                node_size: Some(&mut size),
                node_color: Some(&mut color),
                label: Some(&mut label),
                ..Default::default()
            },
            c.activity,
            &mut TreeActivityFactories::default(),
        )?;
        if (phase == 3 || phase == 5)
            && let Some(mark) = layout
                .layout
                .scene
                .marks
                .iter_mut()
                .find(|m| m.id == "node:a")
        {
            mark.opacity = 0.;
        }
        Ok(layout)
    };
    let raw_canvas = profile >= 6;
    let components = (2..=5).contains(&profile) || profile == 7;
    let ordered = profile == 2 || profile == 3;
    let renderer = if profile == 0 || profile == 2 || profile == 4 {
        BrowserRenderer::Svg
    } else {
        BrowserRenderer::Canvas
    };
    let options = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer,
            tooltip: true,
            tooltip_options: tooltip_text_options(empty_text),
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
        picking: if profile < 2 {
            BrowserTreePicking::Geometry
        } else {
            BrowserTreePicking::Nearest { distance: 64. }
        },
    };
    let placed = m.clone();
    let placement = move |c: &TreeActivityComponentContext<'_>| {
        if placed.fault.get() == 2 {
            return Err(ChartError("tooltip source placement rejected".into()));
        }
        let slot = match c.component {
            TreeComponentContext::Node(n) if n.datum.id == "b" => Some(([-10., 30.], [90., 22.])),
            TreeComponentContext::Link(l) if l.layout.source()[l.link.target_index].id == "c" => {
                Some(([-45., -26.], [90., 22.]))
            }
            TreeComponentContext::Label(l) if l.datum.id == "a" => Some(([-24., -14.], [48., 18.])),
            _ => None,
        };
        Ok(
            slot.map(|(offset, [width, height])| PlotWidgetPlacement::AtDatum {
                offset,
                measured: TextSize { width, height },
            }),
        )
    };
    let widgets = BrowserTreeWidgetOptions {
        revision: 0,
        replace_ink: false,
        source_ordered: ordered,
    };
    let owner = if raw_canvas && components {
        BrowserTree::mount_with_factory_canvas_and_components(
            root.clone(),
            raw(0),
            config(),
            options,
            build,
            program(m.clone()),
            widgets,
            placement,
            source_widget_factory(m.clone(), root.clone(), empty_text),
        )
    } else if raw_canvas {
        BrowserTree::mount_with_factory_and_canvas(
            root.clone(),
            raw(0),
            config(),
            options,
            build,
            program(m.clone()),
        )
    } else if components {
        BrowserTree::mount_with_factory_and_components(
            root.clone(),
            raw(0),
            config(),
            options,
            build,
            widgets,
            placement,
            source_widget_factory(m.clone(), root.clone(), empty_text),
        )
    } else {
        BrowserTree::mount_with_factory(root.clone(), raw(0), config(), options, build)
    }
    .map_err(js)?;
    *m.owner.borrow_mut() = Some(owner.handle());
    let events = Rc::new(RefCell::new(vec![]));
    let ev = events.clone();
    owner.on_event(move |e| ev.borrow_mut().push(format!("{e:?}")));
    owner.flush().map_err(js)?;
    FIXTURES.with_borrow_mut(|all| {
        all.insert(
            key.into(),
            Fixture {
                root,
                css,
                owner,
                m,
                profile,
                empty_text,
                events,
            },
        );
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with_borrow(|all| {
        f(all
            .get(key)
            .ok_or_else(|| js("missing Tree tooltip fixture"))?)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        let s = h.snapshot().map_err(js)?;
        Ok(json!({"owner":layout_cohort(&s.layout),"state":{"active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"desiredFocus":s.desired_focus,"error":s.error.map(|e|e.to_string()),"revision":s.widget_revision,"config":[s.config.width,s.config.height],"chart":s.chart.map(|c|json!({"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"hovered":c.hovered,"selected":c.selected,"hidden":c.hidden,"revision":c.revision}))},"tooltip":h.node_tooltip().map_err(js)?.as_ref().map(|v|tooltip_frame(v,f.empty_text)),"sourceRows":source_rows(&f.m,s.widget_revision),"tooltipRows":tooltip_rows(&f.m,&f.root,f.empty_text),"metrics":{"phase":f.m.phase.get(),"builds":f.m.builds.get(),"functions":f.m.functions.get(),"rawCalls":f.m.raw_calls.get(),"sourceFactories":f.m.source_factories.get(),"sourceCleanups":f.m.source_cleanups.get(),"tooltipFactories":f.m.tooltip_factories.get(),"tooltipCleanups":f.m.tooltip_cleanups.get(),"probes":f.m.observations.borrow().len()},"profile":f.profile,"emptyText":f.empty_text,"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_probes(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        Ok(json!(f.m.observations.borrow().clone()).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        if phase > 6 {
            return Err(js("invalid tooltip phase"));
        }
        f.m.phase.set(phase);
        f.owner.handle().set_source(raw(phase)).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_options(
    key: &str,
    cursor: bool,
    anchor: u8,
    gap: f64,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let anchor = match anchor {
            0 => BrowserTreeTooltipAnchor::Top,
            1 => BrowserTreeTooltipAnchor::TopRight,
            2 => BrowserTreeTooltipAnchor::Right,
            3 => BrowserTreeTooltipAnchor::BottomRight,
            4 => BrowserTreeTooltipAnchor::Bottom,
            5 => BrowserTreeTooltipAnchor::BottomLeft,
            6 => BrowserTreeTooltipAnchor::Left,
            7 => BrowserTreeTooltipAnchor::TopLeft,
            8 => BrowserTreeTooltipAnchor::Center,
            _ => return Err(js("invalid tooltip anchor")),
        };
        f.owner
            .handle()
            .set_node_tooltip_options(BrowserTreeTooltipOptions {
                position: if cursor {
                    BrowserTreeTooltipPosition::Cursor
                } else {
                    BrowserTreeTooltipPosition::Fixed
                },
                anchor,
                gap,
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_component(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| install_component(f, revision).map_err(js))
}
#[wasm_bindgen]
pub fn tree_tooltip_clear_component(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner.handle().clear_node_tooltip_component().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_reset(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.handle().reset_node_tooltip().map_err(js))
}
#[wasm_bindgen]
pub fn tree_tooltip_failure(key: &str, kind: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        if kind > 5 {
            return Err(js("invalid tooltip fault"));
        }
        f.m.fault.set(kind);
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_refresh(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.refresh().map_err(js))
}
#[wasm_bindgen]
pub fn tree_tooltip_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_tooltip_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.reset_view().map_err(js)?;
        if zoom {
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
pub fn tree_tooltip_focus(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner
            .handle()
            .set_activity_options(TreeActivityOptions {
                focus_id: (!id.is_empty()).then(|| id.to_owned()),
                highlight: TreeRelationOptions {
                    ancestor_nodes: true,
                    ancestor_links: true,
                    ..Default::default()
                },
                ..Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_resize(key: &str, width: f64, height: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let mut c = config();
        c.width = width;
        c.height = height;
        c.validate().map_err(js)?;
        f.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))?;
        let surface = f
            .root
            .query_selector(".kartis-surface")?
            .ok_or_else(|| js("missing tooltip Surface"))?;
        let style = surface.unchecked_ref::<web_sys::HtmlElement>().style();
        style.set_property("width", "100%")?;
        style.set_property("height", &format!("{height}px"))?;
        f.owner.handle().set_config(c).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_hidden_all(key: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner
            .handle()
            .set_hidden(if hidden {
                ["a", "c", "r", "b", "d"]
                    .into_iter()
                    .map(str::to_owned)
                    .collect()
            } else {
                BTreeSet::new()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_hidden(key: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner
            .handle()
            .set_hidden(if hidden {
                BTreeSet::from(["c".to_owned()])
            } else {
                BTreeSet::new()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner.handle().set_widget_revision(revision).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_dispatch(key: &str, x: f64, y: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner
            .handle()
            .dispatch(SurfaceInput::Move {
                id: 1,
                point: (x, y),
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_tooltip_stop(key: &str) -> std::result::Result<bool, JsValue> {
    FIXTURES.with_borrow_mut(|all| {
        if let Some(f) = all.remove(key) {
            f.m.operation.set(2);
            f.owner.stop();
            for row in f.m.source_rows.borrow().iter() {
                if !f.m.retired_source.borrow().contains(&row.ctor)
                    || row.frame.is_alive()
                    || !matches!(
                        row.frame.try_with_untracked(|_| ()),
                        Err(glory::core::AccessError::Disposed
                            | glory::core::AccessError::RuntimeGone)
                    )
                {
                    return Err(js(format!(
                        "Tree tooltip stop left a live source Scope: {}",
                        row.ctor
                    )));
                }
            }
            for row in f.m.tooltip_rows.borrow().iter() {
                if !f.m.retired_tooltip.borrow().contains(&row.ctor)
                    || row.frame.is_alive()
                    || !matches!(
                        row.frame.try_with_untracked(|_| ()),
                        Err(glory::core::AccessError::Disposed
                            | glory::core::AccessError::RuntimeGone)
                    )
                {
                    return Err(js(format!(
                        "Tree tooltip stop left a live tooltip Scope: {}",
                        row.ctor
                    )));
                }
            }
            if f.m.source_cleanups.get() as usize != f.m.source_rows.borrow().len()
                || f.m.tooltip_cleanups.get() as usize != f.m.tooltip_rows.borrow().len()
                || f.owner.handle().node_tooltip().map_err(js)?.is_some()
                || f.root.query_selector(".kartis-tree-tooltip")?.is_some()
            {
                return Err(js(
                    "Tree tooltip stop did not retire all constructed Scopes",
                ));
            }
            f.root.remove();
            f.css.remove();
            Ok(true)
        } else {
            Ok(false)
        }
    })
}
