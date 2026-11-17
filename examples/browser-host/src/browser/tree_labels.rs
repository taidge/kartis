//! Independent real retained Tree label component acceptance application.
use std::collections::{BTreeMap, BTreeSet};

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Position, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeHandle, BrowserTreeOptions,
    BrowserTreePicking, BrowserTreeWidgetOptions, MotionPreference,
};

use super::tree_activity::model;
use super::*;

type Rows = Rc<
    RefCell<BTreeMap<(u64, String), glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>>>,
>;
type Logs = Rc<RefCell<Vec<(TreeActivityLayout, Vec<String>)>>>;
struct Fixture {
    key: String,
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    phase: Rc<Cell<u8>>,
    failure: Rc<Cell<u8>>,
    builds: Rc<Cell<u32>>,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    rows: Rows,
    probes: Rc<RefCell<Vec<Value>>>,
    logs: Logs,
    events: Rc<RefCell<Vec<String>>>,
    ordered: bool,
    picking: u8,
}
thread_local! {static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn datum(d: &HierarchyDatum) -> Value {
    json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value,"bits":d.value.to_bits().to_string()})
}
fn node(n: &TreeNode) -> Value {
    json!({"index":n.source_index,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"rawBits":n.raw_value.to_bits().to_string(),"value":n.value,"markId":n.mark_id,"color":n.color.map(rgba)})
}
fn link(l: &TreeLink) -> Value {
    json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"position":l.position,"sourcePosition":l.source_position,"targetPosition":l.target_position,"markId":l.mark_id})
}
fn label(l: &Label) -> Value {
    json!({"position":[l.x,l.y],"text":l.text,"size":l.size,"color":rgba(l.color),"rotation":l.rotation,"anchor":format!("{:?}",l.anchor)})
}
fn activity(a: &TreeActivityState) -> Value {
    json!({"focus":a.focus_id,"index":a.focus_index,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links],"depth":a.fix_node_color_at_depth})
}
fn metadata(l: &TreeActivityLayout) -> Value {
    let tree = &l.layout;
    json!({"raw":tree.source().iter().map(datum).collect::<Vec<_>>(),"nodes":tree.nodes.iter().map(node).collect::<Vec<_>>(),"links":tree.links.iter().map(link).collect::<Vec<_>>(),"labels":tree.scene.labels.iter().enumerate().map(|(i,l)|json!({"index":i,"owner":tree.scene.label_series.get(&i),"label":label(l),"font":format!("{:?}",tree.scene.resolved_label_font(i)),"whitespace":format!("{:?}",tree.scene.typography.label_whitespace(i)),"clip":tree.scene.label_clips.get(&i).map(|c|format!("{c:?}"))})).collect::<Vec<_>>(),"activity":activity(&l.activity),"plot":[tree.plot.x,tree.plot.y,tree.plot.width,tree.plot.height],"drawOrder":tree.scene.draw_items().iter().map(|i|format!("{i:?}")).collect::<Vec<_>>(),"marks":tree.scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"opacity":m.opacity,"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>()})
}
fn current_keys(layout: &TreeActivityLayout) -> BTreeSet<String> {
    let scene = &layout.layout.scene;
    let mut keys = BTreeSet::new();
    for id in ["node:b", "link:c"] {
        if scene.marks.iter().any(|mark| mark.id == id) {
            keys.insert(id.to_owned());
        }
    }
    for owner in scene
        .label_series
        .values()
        .filter(|owner| owner.as_str() != "d")
    {
        keys.insert(format!("tree-label:{owner}"));
    }
    keys
}
fn data(d: &TreeActivityComponentData) -> Value {
    let (raw, nodes, links, labels, index, role, selected) = match &d.component {
        TreeComponentData::Node(n) => (
            &n.source,
            &n.nodes,
            &n.links,
            &n.labels,
            n.source_index,
            "node",
            Value::Null,
        ),
        TreeComponentData::Link(l) => (
            &l.source,
            &l.nodes,
            &l.links,
            &l.labels,
            l.link_index,
            "link",
            Value::Null,
        ),
        TreeComponentData::Label(l) => (
            &l.source,
            &l.nodes,
            &l.links,
            &l.labels,
            l.source_index,
            "label",
            json!({"index":l.label_index,"label":label(&l.label),"font":format!("{:?}",l.font),"whitespace":format!("{:?}",l.whitespace),"clip":l.clip.as_ref().map(|c|format!("{c:?}"))}),
        ),
    };
    json!({"role":role,"index":index,"raw":raw.iter().map(datum).collect::<Vec<_>>(),"nodes":nodes.iter().map(node).collect::<Vec<_>>(),"links":links.iter().map(link).collect::<Vec<_>>(),"labels":labels.labels.iter().map(label).collect::<Vec<_>>(),"activity":activity(&d.activity),"active":d.is_active,"selected":selected})
}
fn frame(f: &SceneWidgetFrame) -> Value {
    let r = f.rect();
    let visible = f.visible().map(|r| [r.x, r.y, r.width, r.height]);
    let source = match f {
        SceneWidgetFrame::Mark(m) => json!({"kind":"mark","id":m.mark.id}),
        SceneWidgetFrame::Label(l) => {
            json!({"kind":"label","index":l.label_index,"owner":l.owner,"label":label(&l.label),"font":format!("{:?}",l.font),"whitespace":format!("{:?}",l.whitespace),"clip":l.clip.as_ref().map(|c|format!("{c:?}"))})
        }
    };
    json!({"source":source,"rect":[r.x,r.y,r.width,r.height],"visible":visible,"interactive":f.interactive(),"transform":f.transform().map(|t|json!({"matrix":t.matrix,"measured":[t.measured.width,t.measured.height]}))})
}
fn source_phase(phase: u8) -> u8 {
    if phase == 1 {
        1
    } else if phase == 2 {
        2
    } else if phase == 12 {
        8
    } else {
        0
    }
}
fn source(phase: u8) -> Vec<HierarchyDatum> {
    let mut raw = model::data(source_phase(phase));
    if phase == 9 {
        for d in &mut raw {
            if d.id == "b" {
                d.label.clear();
            }
        }
    }
    raw
}
fn options(phase: u8) -> TreeOptions {
    let mut tree = model::tree(0);
    tree.style.labels.rotation = TreeLabelRotation::Fixed(match phase {
        3 => 30.,
        4 => -90.,
        _ => 0.,
    });
    tree.style.labels.visibility = match phase {
        5 => TreeLabels::Leaves,
        6 => TreeLabels::None,
        _ => TreeLabels::All,
    };
    tree
}
fn placement(c: &TreeActivityComponentContext<'_>) -> Result<Option<PlotWidgetPlacement>> {
    let (offset, measured) = match c.component {
        TreeComponentContext::Node(n) if n.datum.id == "b" => (
            [-10., 30.],
            TextSize {
                width: 90.,
                height: 22.,
            },
        ),
        TreeComponentContext::Link(l) if l.target_datum.id == "c" => (
            [-50., -26.],
            TextSize {
                width: 100.,
                height: 22.,
            },
        ),
        TreeComponentContext::Label(l) if l.datum.id != "d" => (
            [
                match l.label.anchor {
                    TextAnchor::Start => 0.,
                    TextAnchor::Middle => -24.,
                    TextAnchor::End => -48.,
                },
                -14.,
            ],
            TextSize {
                width: 48.,
                height: 18.,
            },
        ),
        _ => return Ok(None),
    };
    Ok(Some(PlotWidgetPlacement::AtDatum { offset, measured }))
}
#[wasm_bindgen]
pub fn tree_labels_mount(
    key: &str,
    picking: u8,
    ordered: bool,
) -> std::result::Result<(), JsValue> {
    let renderer = match key {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("unknown Tree label renderer")),
    };
    if picking > 1 {
        return Err(js("unknown Tree label picking"));
    }
    tree_labels_stop(key)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    root.set_id(&format!("tree-labels-{key}"));
    root.set_attribute(
        "style",
        "width:720px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-labels-{key} button.tree-label-component{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:10px sans-serif;overflow:hidden;min-width:0;white-space:nowrap}}")));
    document.body().unwrap().append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let phase = Rc::new(Cell::new(0u8));
    let failure = Rc::new(Cell::new(0u8));
    let builds = Rc::new(Cell::new(0u32));
    let calls = Rc::new(Cell::new(0u32));
    let cleanups = Rc::new(Cell::new(0u32));
    let rows: Rows = Rc::new(RefCell::new(BTreeMap::new()));
    let probes = Rc::new(RefCell::new(Vec::new()));
    let logs: Logs = Rc::new(RefCell::new(Vec::new()));
    let owner_handle = Rc::new(RefCell::new(None::<BrowserTreeHandle>));
    let (p, fail, built, logged) = (phase.clone(), failure.clone(), builds.clone(), logs.clone());
    let build = move |c: &BrowserTreeBuildContext<'_>| {
        built.set(built.get() + 1);
        if fail.get() == 1 {
            return Err(ChartError("Tree label builder rejected".into()));
        }
        let mut proposed = model::source(
            source_phase(p.get()),
            c.config,
            c.source,
            c.tree,
            c.activity,
            0,
        )?;
        let scene = &mut proposed.layout.layout.scene;
        for (i, l) in scene.labels.iter_mut().enumerate() {
            let id = scene.label_series.get(&i).map(String::as_str).unwrap_or("");
            l.anchor = match id {
                "r" | "c" => TextAnchor::Start,
                "b" => TextAnchor::End,
                _ => TextAnchor::Middle,
            };
            if p.get() == 7 && id == "c" {
                l.color = Color(100, 20, 180, 0);
            }
            if p.get() == 9 && id == "b" {
                l.text.clear();
            }
        }
        if p.get() == 8
            && let Some(m) = scene.marks.iter_mut().find(|m| m.id == "node:c")
        {
            m.opacity = 0.;
        }
        if p.get() == 10
            && let Some((i, l)) = scene
                .labels
                .iter()
                .enumerate()
                .find(|(i, _)| scene.label_series.get(i).is_some_and(|s| s == "c"))
        {
            scene.label_clips.insert(
                i,
                ClipPath {
                    geometry: Geometry::Rect {
                        x: l.x + 8.,
                        y: l.y - 12.,
                        width: 20.,
                        height: 12.,
                    },
                    rule: ClipRule::NonZero,
                },
            );
        }
        if p.get() == 11 {
            let (index, l) = scene
                .labels
                .iter()
                .enumerate()
                .find(|(i, _)| scene.label_series.get(i).is_some_and(|s| s == "a"))
                .unwrap();
            let origin = [l.x - 18., l.y - 11.];
            let label_index = index;
            let mut order = scene.draw_items();
            let slot = scene.marks.len();
            scene.marks.push(Mark {
                id: "label-sentinel".into(),
                series: String::new(),
                label: String::new(),
                value: 0.,
                geometry: Geometry::Rect {
                    x: origin[0],
                    y: origin[1],
                    width: 12.,
                    height: 10.,
                },
                fill: Some(Color(20, 180, 220, 255)),
                fill_paint: None,
                stroke: None,
                stroke_dash: None,
                opacity: 1.,
                clip_path: None,
                clip: None,
            });
            let at = order
                .iter()
                .position(|item| *item == DrawItem::Label(label_index))
                .unwrap()
                + 1;
            order.insert(at, DrawItem::Mark(slot));
            scene.draw_order = order;
        }
        logged
            .borrow_mut()
            .push((proposed.layout.clone(), proposed.log));
        Ok(proposed.layout)
    };
    let fail = failure.clone();
    let choose = move |c: &TreeActivityComponentContext<'_>| {
        if fail.get() == 2 {
            return Err(ChartError("Tree label placement rejected".into()));
        }
        let mut result = placement(c)?;
        if fail.get() == 3
            && let Some(PlotWidgetPlacement::AtDatum { measured, .. }) = &mut result
        {
            measured.width = f64::NAN;
        }
        Ok(result)
    };
    let (fail, called, retired, live, observed, handle) = (
        failure.clone(),
        calls.clone(),
        cleanups.clone(),
        rows.clone(),
        probes.clone(),
        owner_handle.clone(),
    );
    let factory = move |c: SceneWidgetCompositionContext<TreeActivityComponentData>| {
        called.set(called.get() + 1);
        if fail.get() == 4 {
            return Err(ChartError("Tree label factory rejected".into()));
        }
        let key = c.key;
        let born = c.revision;
        let f = c.frame;
        live.borrow_mut().insert((born, key.clone()), f);
        let (retired, live, observed, handle) = (
            retired.clone(),
            live.clone(),
            observed.clone(),
            handle.clone(),
        );
        let reject = fail.get() == 5;
        Ok(glory::ui::widget(move |scope| {
            let retired = retired.clone();
            scope.on_cleanup(move || retired.set(retired.get() + 1))?;
            let count = scope.cage(0u32);
            let focused = scope.cage(false);
            let key_for_text = key.clone();
            let (live, observed, handle, key_for_probe) =
                (live.clone(), observed.clone(), handle.clone(), key.clone());
            scope.effect(move || {
                let own=f.get();let Some(handle)=handle.borrow().clone() else {return};
                let record=match handle.snapshot() {
                    Ok(s)=>{let accepted=metadata(&s.layout);let selected=current_keys(&s.layout);let tentative=born!=s.widget_revision||!selected.contains(&key_for_probe);let mut all=BTreeMap::new();if !tentative {for key in selected {let row=live.borrow().get(&(s.widget_revision,key.clone())).copied();all.insert(key,row.and_then(|bond|bond.try_get().ok()).map(|r|json!({"frame":frame(&r.frame),"data":data(&r.data)})).unwrap_or_else(||json!({"error":"missing live accepted Tree component"})));}}json!({"key":key_for_probe,"born":born,"tentative":tentative,"child":{"frame":frame(&own.frame),"data":data(&own.data)},"owner":accepted,"rows":all,"revision":s.widget_revision})},
                    Err(e)=>json!({"key":key_for_probe,"born":born,"error":e.to_string()}),
                };observed.borrow_mut().push(record);
            });
            glory::ui::button().class("tree-label-component").style(scope.bond(move || {
                f.with(|value| {let color=if value.data.is_active==Some(true) {Color(0,100,255,255)} else {Color(100,20,180,255)};Style{background:Some(color),color:Some(color),..Style::new().position(Position::Relative).width(Length::Percent(100.)).height(Length::Percent(100.))}})
            })).label(scope.bond(move ||f.with(|value|format!("{}|count={}|focus={}|born={}",json!({"key":key_for_text,"frame":frame(&value.frame),"data":data(&value.data)}),count.get(),focused.get(),born))))
            .fill(glory::ui::text().content("").class("tree-label-component-dot").style(Style{background:Some(Color(255,220,0,255)),..Style::new().position(Position::Absolute).left(Length::Px(6.)).top(Length::Px(3.)).width(Length::Px(4.)).height(Length::Px(4.))}))
            .on(EventKind::Activate,move |_|count.revise(|v|*v+=1)).on(EventKind::FocusChanged,move |e|if let UiEvent::FocusChanged(v)=e {focused.revise(|old|*old=v);}).show_in(scope)?;
            if reject {
                return Err(glory::render::Error::InvalidProps("Tree label child rejected").into());
            }
            Ok(())
        }))
    };
    let browser = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer,
            highlight: if picking == 0 {
                BrowserHighlight::Controlled
            } else {
                BrowserHighlight::Series
            },
            motion: MotionPreference::Reduced,
            ..Default::default()
        },
        tree: options(0),
        activity: TreeActivityOptions {
            highlight: TreeRelationOptions {
                ancestor_nodes: true,
                descendant_nodes: false,
                ancestor_links: true,
                descendant_links: false,
            },
            ..Default::default()
        },
        picking: if picking == 0 {
            BrowserTreePicking::Geometry
        } else {
            BrowserTreePicking::Nearest { distance: 1e6 }
        },
    };
    let owner = match BrowserTree::mount_with_factory_and_components(
        root.clone(),
        source(0),
        model::config(720.),
        browser,
        build,
        BrowserTreeWidgetOptions {
            revision: 0,
            replace_ink: true,
            source_ordered: ordered,
        },
        choose,
        factory,
    ) {
        Ok(o) => o,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    *owner_handle.borrow_mut() = Some(owner.handle());
    let events = Rc::new(RefCell::new(Vec::new()));
    let captured = events.clone();
    owner.on_event(move |e| captured.borrow_mut().push(format!("{e:?}")));
    owner.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().push(Fixture {
            key: key.into(),
            root,
            css,
            owner,
            phase,
            failure,
            builds,
            calls,
            cleanups,
            rows,
            probes,
            logs,
            events,
            ordered,
            picking,
        })
    });
    Ok(())
}
fn with<T>(
    key: &str,
    f: impl FnOnce(&Fixture) -> std::result::Result<T, JsValue>,
) -> std::result::Result<T, JsValue> {
    FIXTURES.with(|all| {
        f(all
            .borrow()
            .iter()
            .find(|f| f.key == key)
            .ok_or_else(|| js("missing Tree label owner"))?)
    })
}
#[wasm_bindgen]
pub fn tree_labels_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let s = f.owner.snapshot().map_err(js)?;
        let mut rows = BTreeMap::new();
        let selected = current_keys(&s.layout);
        for ((revision, key), bond) in f.rows.borrow().iter() {
            if selected.contains(key)
                && *revision == s.widget_revision
                && let Ok(value) = bond.try_get()
            {
                rows.insert(
                    key.clone(),
                    json!({"frame":frame(&value.frame),"data":data(&value.data)}),
                );
            }
        }
        let log = f
            .logs
            .borrow()
            .iter()
            .rev()
            .find(|(l, _)| l == &s.layout)
            .map(|(_, log)| log.clone())
            .unwrap_or_default();
        Ok(json!({"accepted":metadata(&s.layout),"rows":rows,"log":log,"state":{"active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"error":s.error.map(|e|e.to_string()),"revision":s.widget_revision,"width":s.config.width,"height":s.config.height,"view":s.chart.as_ref().map(|c|[c.view.scale,c.view.translate_x,c.view.translate_y]),"hovered":s.chart.as_ref().map(|c|&c.hovered),"selected":s.chart.as_ref().map(|c|&c.selected)},"diagnostics":{"phase":f.phase.get(),"builds":f.builds.get(),"calls":f.calls.get(),"cleanups":f.cleanups.get(),"ordered":f.ordered,"picking":f.picking,"probes":f.probes.borrow().len()},"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_labels_set(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 12 {
        return Err(js("unknown Tree label phase"));
    }
    with(key, |f| {
        let h = f.owner.handle();
        h.clear_current().map_err(js)?;
        h.flush().map_err(js)?;
        f.phase.set(phase);
        h.set_source(source(phase)).map_err(js)?;
        h.set_tree_options(options(phase)).map_err(js)?;
        h.refresh().map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_failure(key: &str, kind: u8) -> std::result::Result<(), JsValue> {
    if kind > 5 {
        return Err(js("unknown Tree label rejection"));
    }
    with(key, |f| {
        f.failure.set(kind);
        let h = f.owner.handle();
        if kind >= 4 {
            h.set_widget_revision(7).map_err(js)?;
        } else {
            h.refresh().map_err(js)?;
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_recover(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.failure.set(0);
        let h = f.owner.handle();
        h.set_widget_revision(0).map_err(js)?;
        h.refresh().map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner.handle().set_widget_revision(revision).map_err(js)?;
        f.owner.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_labels_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.reset_view().map_err(js)?;
        if zoom {
            let s = h.snapshot().map_err(js)?;
            h.dispatch(SurfaceInput::Wheel {
                point: (s.config.width / 2., 170.),
                delta: -1.1f64.ln() / 0.002,
            })
            .map_err(js)?;
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))?;
        let mut config = f.owner.snapshot().map_err(js)?.config;
        config.width = width;
        f.owner.handle().set_config(config).map_err(js)?;
        f.owner.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_labels_probes(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| Ok(json!(f.probes.borrow().clone()).to_string()))
}
#[wasm_bindgen]
pub fn tree_labels_stop(key: &str) -> std::result::Result<bool, JsValue> {
    let removed = FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        all.iter().position(|f| f.key == key).map(|i| all.remove(i))
    });
    if let Some(f) = removed {
        f.owner.stop();
        f.root.remove();
        f.css.remove();
    }
    Ok(false)
}
