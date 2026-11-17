//! Actual native Tree Canvas and three typed retained Glory overlay roles.
use std::collections::BTreeMap;

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeCanvasProgram,
    BrowserTreeHandle, BrowserTreeOptions, BrowserTreePicking, BrowserTreeWidgetOptions,
    MotionPreference,
};

use super::*;

type Row = glory::Bond<TypedSceneWidgetFrame<TreeActivityComponentData>>;
#[derive(Default)]
struct Metrics {
    builds: Cell<u32>,
    functions: Cell<u32>,
    placements: Cell<u32>,
    factories: Cell<u32>,
    scopes: Cell<u32>,
    cleanups: Cell<u32>,
    raw_calls: Cell<u32>,
    fault: Cell<u8>,
    owner: RefCell<Option<BrowserTreeHandle>>,
    rows: RefCell<BTreeMap<(u64, String), Row>>,
    observations: RefCell<Vec<Value>>,
}
struct Fixture {
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    metrics: Rc<Metrics>,
    events: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<BTreeMap<String,Fixture>>=const{RefCell::new(BTreeMap::new())};}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    [
        ("a", Some("r"), 0.),
        ("c", Some("a"), if phase == 1 { 35. } else { 30. }),
        ("r", None, 0.),
        ("b", Some("r"), if phase == 1 { 5. } else { -0. }),
        ("d", Some("a"), 10.),
    ]
    .into_iter()
    .map(|(id, parent, value)| {
        let mut d = HierarchyDatum::new(id, parent.map(str::to_owned), value);
        d.label = format!("source-{id}");
        d
    })
    .collect()
}
fn config() -> ChartConfig {
    ChartConfig {
        width: 600.,
        height: 360.,
        margin: Margin {
            left: 40.,
            right: 40.,
            top: 40.,
            bottom: 40.,
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
fn label(l: &Label) -> Value {
    json!({"text":l.text,"position":[l.x,l.y],"size":l.size,"color":rgba(l.color),"rotation":l.rotation,"anchor":format!("{:?}",l.anchor)})
}
fn cohort(
    raw: &[HierarchyDatum],
    nodes: &[TreeNode],
    links: &[TreeLink],
    labels: &[Label],
    a: &TreeActivityState,
) -> Value {
    json!({"raw":raw.iter().map(datum).collect::<Vec<_>>(),"nodes":nodes.iter().map(|n|json!({"index":n.source_index,"parent":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"position":n.position,"radius":n.radius,"rawBits":n.raw_value.to_bits().to_string(),"value":n.value,"color":n.color.map(rgba),"markId":n.mark_id})).collect::<Vec<_>>(),"links":links.iter().map(|l|json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"position":l.position,"markId":l.mark_id})).collect::<Vec<_>>(),"labels":labels.iter().map(label).collect::<Vec<_>>(),"activity":{"focus":a.focus_id,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links]}})
}
fn owner_data(s: &kartis::browser::BrowserTreeSnapshot) -> Value {
    let l = &s.layout.layout;
    cohort(
        l.source(),
        &l.nodes,
        &l.links,
        &l.scene.labels,
        &s.layout.activity,
    )
}
fn data(d: &TreeActivityComponentData) -> Value {
    let (source, nodes, links, labels, role, index) = match &d.component {
        TreeComponentData::Node(n) => (
            &n.source,
            &n.nodes,
            &n.links,
            &n.labels,
            "node",
            n.source_index,
        ),
        TreeComponentData::Link(l) => (
            &l.source,
            &l.nodes,
            &l.links,
            &l.labels,
            "link",
            l.link_index,
        ),
        TreeComponentData::Label(l) => (
            &l.source,
            &l.nodes,
            &l.links,
            &l.labels,
            "label",
            l.source_index,
        ),
    };
    json!({"role":role,"index":index,"active":d.is_active,"cohort":cohort(source,nodes,links,&labels.labels,&d.activity)})
}
fn frame(f: &SceneWidgetFrame) -> Value {
    let r = f.rect();
    json!({"rect":[r.x,r.y,r.width,r.height],"visible":f.visible().map(|r|[r.x,r.y,r.width,r.height]),"interactive":f.interactive(),"label":f.is_label()})
}
fn row(r: &TypedSceneWidgetFrame<TreeActivityComponentData>) -> Value {
    json!({"frame":frame(&r.frame),"data":data(&r.data)})
}
fn rows(m: &Metrics, revision: u64) -> Value {
    let all = m
        .rows
        .borrow()
        .iter()
        .filter(|((born, _), _)| *born == revision)
        .map(|((_, key), bond)| {
            (
                key.clone(),
                bond.try_get()
                    .map(|r| row(&r))
                    .unwrap_or_else(|e| json!({"error":e.to_string()})),
            )
        })
        .collect::<BTreeMap<_, _>>();
    json!(all)
}
fn native(root: &web_sys::Element) -> Value {
    root.query_selector(".kartis-surface canvas")
        .ok()
        .flatten()
        .and_then(|c| c.dyn_into::<web_sys::HtmlCanvasElement>().ok())
        .map(|c| json!({"size":[c.width(),c.height()],"png":c.to_data_url().ok()}))
        .unwrap_or(Value::Null)
}
fn program(m: Rc<Metrics>, variant: u8) -> BrowserTreeCanvasProgram {
    let links = m.clone();
    let labels = m.clone();
    BrowserTreeCanvasProgram {
        layers: vec![
            TreeCanvasLayer::Links,
            TreeCanvasLayer::Nodes,
            TreeCanvasLayer::Labels,
        ],
        render_link: Some(Box::new(move |_, _| {
            links.raw_calls.set(links.raw_calls.get() + 1);
            Ok(())
        })),
        render_label: Some(Box::new(move |_, _| {
            labels.raw_calls.set(labels.raw_calls.get() + 1);
            Ok(())
        })),
        render_node: Some(Box::new(move |ctx, c| {
            m.raw_calls.set(m.raw_calls.get() + 1);
            if m.fault.get() == 1 && c.datum.id == "a" {
                return Err(ChartError("joint native renderer rejected".into()));
            }
            let phase1 = c.frame.source()[1].value == 35.;
            let color = if c.is_active == Some(true) {
                "#0064ff"
            } else {
                match (variant, phase1) {
                    (0, false) => "#6414b4",
                    (0, true) => "#e66e14",
                    (_, false) => "#00a064",
                    (_, true) => "#14b4dc",
                }
            };
            ctx.set_fill_style_str(color);
            ctx.fill_rect(c.node.position[0] + 38., c.node.position[1] - 8., 20., 16.);
            Ok(())
        })),
        ..Default::default()
    }
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_mount(key: &str) -> std::result::Result<(), JsValue> {
    tree_canvas_widgets_stop(key)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    root.set_id(&format!("tree-canvas-widgets-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:600px;height:360px;margin:12px 0;max-width:none",
    )?;
    let css = document.create_element("style")?;
    css.set_text_content(Some(&format!("#tree-canvas-widgets-{key} .tcw{{display:block;padding:0;border:0;border-radius:0;box-sizing:border-box;font:0px sans-serif;pointer-events:auto!important;overflow:hidden}}")));
    document.body().unwrap().append_child(&css)?;
    document.body().unwrap().append_child(&root)?;
    let m = Rc::new(Metrics::default());
    let built = m.clone();
    let build = move |c: &BrowserTreeBuildContext<'_>| {
        built.builds.set(built.builds.get() + 1);
        let s = built.clone();
        let color = built.clone();
        let width = built.clone();
        let lc = built.clone();
        let text = built.clone();
        let mut size = move |_: &TreeNodeFunctionContext<'_>| {
            s.functions.set(s.functions.get() + 1);
            Ok(32.)
        };
        let mut fill = move |_: &TreeNodeFunctionContext<'_>| {
            color.functions.set(color.functions.get() + 1);
            Ok(Color(40, 90, 180, 255))
        };
        let mut thickness = move |_: &TreeLinkFunctionContext<'_>| {
            width.functions.set(width.functions.get() + 1);
            Ok(6.)
        };
        let mut line = move |_: &TreeLinkFunctionContext<'_>| {
            lc.functions.set(lc.functions.get() + 1);
            Ok(Color(230, 110, 20, 255))
        };
        let mut label = move |c: &TreeLabelFunctionContext<'_>| {
            text.functions.set(text.functions.get() + 1);
            Ok(format!(
                "{}:{:?}",
                c.datum.id,
                c.activity.and_then(|a| a.nodes[c.source_index])
            ))
        };
        tree_canvas_layout_with_activity(
            c.config,
            c.source,
            c.tree,
            &mut TreeFactories {
                node_size: Some(&mut size),
                node_color: Some(&mut fill),
                link_thickness: Some(&mut thickness),
                link_color: Some(&mut line),
                label: Some(&mut label),
            },
            c.activity,
            &mut TreeActivityFactories::default(),
        )
    };
    let placed = m.clone();
    let placement = move |c: &TreeActivityComponentContext<'_>| {
        placed.placements.set(placed.placements.get() + 1);
        let offset = match c.component {
            TreeComponentContext::Node(n) if n.datum.id == "b" => Some(([-12., 30.], [90., 24.])),
            TreeComponentContext::Link(l) if l.layout.source()[l.link.target_index].id == "c" => {
                Some(([-45., -24.], [90., 24.]))
            }
            TreeComponentContext::Label(l) if l.datum.id == "a" => Some(([-36., -14.], [72., 20.])),
            _ => None,
        };
        if placed.fault.get() == 2
            && matches!(c.component,TreeComponentContext::Label(l)if l.datum.id=="a")
        {
            return Err(ChartError("joint placement rejected".into()));
        }
        Ok(
            offset.map(|(offset, [width, height])| PlotWidgetPlacement::AtDatum {
                offset,
                measured: TextSize { width, height },
            }),
        )
    };
    let metrics = m.clone();
    let observed_root = root.clone();
    let factory = move |c: SceneWidgetCompositionContext<TreeActivityComponentData>| {
        metrics.factories.set(metrics.factories.get() + 1);
        if metrics.fault.get() == 3 && c.key == "tree-label:a" {
            return Err(ChartError("joint factory rejected".into()));
        }
        let key = c.key.clone();
        let born = c.revision;
        let f = c.frame;
        let m = metrics.clone();
        let root = observed_root.clone();
        let reject = m.fault.get() == 4 && key == "tree-label:a";
        Ok(glory::ui::widget(move |scope| {
            m.scopes.set(m.scopes.get() + 1);
            m.rows.borrow_mut().insert((born, key.clone()), f);
            let cleaned = m.clone();
            let clean_key = key.clone();
            scope.on_cleanup(move || {
                cleaned.cleanups.set(cleaned.cleanups.get() + 1);
                cleaned.rows.borrow_mut().remove(&(born, clean_key));
            })?;
            let count = scope.cage(0u32);
            let focused = scope.cage(false);
            let watcher = m.clone();
            let own_key = key.clone();
            let effect_root = root.clone();
            scope.effect(move||{
    let own=f.get();let handle=watcher.owner.borrow().clone();let observation=match handle.map(|h|h.snapshot()) {Some(Ok(s))=>json!({"key":own_key,"born":born,"revision":s.widget_revision,"tentative":born!=s.widget_revision,"own":row(&own),"owner":owner_data(&s),"rows":rows(&watcher,s.widget_revision),"native":native(&effect_root)}),Some(Err(e))=>json!({"key":own_key,"born":born,"error":e.to_string()}),None=>json!({"key":own_key,"born":born,"initialConstruction":true,"own":row(&own),"native":native(&effect_root)})};watcher.observations.borrow_mut().push(observation);
   });
            let text_key = key.clone();
            glory::ui::button()
                .class("tcw")
                .style(scope.bond(move || {
                    f.with(|r| {
                        let color = if r.data.is_active == Some(true) {
                            Color(0, 100, 255, 255)
                        } else {
                            Color(100, 20, 180, 255)
                        };
                        Style {
                            background: Some(color),
                            color: Some(color),
                            ..Style::new()
                                .width(Length::Percent(100.))
                                .height(Length::Percent(100.))
                        }
                    })
                }))
                .label(scope.bond(move || {
                    f.with(|r| {
                        format!(
                            "{}|count={}|focus={}|born={}",
                            json!({"key":text_key,"row":row(r)}),
                            count.get(),
                            focused.get(),
                            born
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|n| *n += 1))
                .on(EventKind::FocusChanged, move |e| {
                    if let UiEvent::FocusChanged(v) = e {
                        focused.revise(|n| *n = v)
                    }
                })
                .show_in(scope)?;
            if reject {
                return Err(glory::render::Error::InvalidProps("joint child rejected").into());
            }
            Ok(())
        }))
    };
    let mut tree = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    tree.style.links.curve = TreeLinkCurve::Straight;
    tree.style.labels.position = TreeLabelPosition::Below;
    tree.style.labels.size = Some(16.);
    let options = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer: BrowserRenderer::Canvas,
            highlight: BrowserHighlight::Controlled,
            motion: MotionPreference::Reduced,
            tooltip: false,
            height: Some(360.),
            ..Default::default()
        },
        tree,
        picking: BrowserTreePicking::Nearest { distance: 1e6 },
        activity: TreeActivityOptions {
            highlight: TreeRelationOptions {
                ancestor_nodes: true,
                descendant_nodes: false,
                ancestor_links: true,
                descendant_links: false,
            },
            ..Default::default()
        },
    };
    let owner = match BrowserTree::mount_with_factory_canvas_and_components(
        root.clone(),
        raw(0),
        config(),
        options,
        build,
        program(m.clone(), 0),
        BrowserTreeWidgetOptions {
            revision: 0,
            replace_ink: false,
            source_ordered: false,
        },
        placement,
        factory,
    ) {
        Ok(owner) => owner,
        Err(e) => {
            root.remove();
            css.remove();
            return Err(js(e));
        }
    };
    *m.owner.borrow_mut() = Some(owner.handle());
    let events = Rc::new(RefCell::new(vec![]));
    let logs = events.clone();
    owner.on_event(move |e| logs.borrow_mut().push(format!("{e:?}")));
    owner.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().insert(
            key.into(),
            Fixture {
                root,
                css,
                owner,
                metrics: m,
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
    FIXTURES.with(|all| {
        f(all
            .borrow()
            .get(key)
            .ok_or_else(|| js("missing joint Canvas fixture"))?)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let s = f.owner.snapshot().map_err(js)?;
        let m = &f.metrics;
        Ok(json!({"accepted":owner_data(&s),"revision":s.widget_revision,"chart":s.chart.map(|c|json!({"revision":c.revision,"width":c.width,"height":c.height,"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"selected":c.selected,"hovered":c.hovered})),"rows":rows(m,s.widget_revision),"diagnostics":{"builds":m.builds.get(),"functions":m.functions.get(),"placements":m.placements.get(),"factories":m.factories.get(),"scopes":m.scopes.get(),"cleanups":m.cleanups.get(),"registry":m.rows.borrow().len(),"rawCalls":m.raw_calls.get(),"observations":m.observations.borrow().len(),"events":f.events.borrow().clone()}}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_observations(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        Ok(json!(f.metrics.observations.borrow().clone()).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 1 {
        return Err(js("unknown joint source"));
    }
    with(key, |f| {
        let h = f.owner.handle();
        h.set_source(raw(phase)).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_program(key: &str, variant: u8) -> std::result::Result<(), JsValue> {
    if variant > 1 {
        return Err(js("unknown joint program"));
    }
    with(key, |f| {
        let h = f.owner.handle();
        h.set_canvas_program(program(f.metrics.clone(), variant))
            .map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.set_widget_revision(revision).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_fail(key: &str, kind: u8) -> std::result::Result<(), JsValue> {
    if !(1..=4).contains(&kind) {
        return Err(js("unknown joint rejection"));
    }
    with(key, |f| {
        f.metrics.fault.set(kind);
        let h = f.owner.handle();
        if kind >= 3 {
            h.set_widget_revision(7).map_err(js)?;
        }
        h.set_source(raw(0)).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_recover(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.metrics.fault.set(0);
        let h = f.owner.handle();
        h.set_widget_revision(0).map_err(js)?;
        h.set_source(raw(1)).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_png(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| f.owner.handle().canvas_png_data_url().map_err(js))
}
#[wasm_bindgen]
pub fn tree_canvas_widgets_stop(key: &str) -> std::result::Result<String, JsValue> {
    let f = FIXTURES.with(|all| all.borrow_mut().remove(key));
    if let Some(f) = f {
        f.owner.stop();
        f.root.remove();
        f.css.remove();
        Ok(json!({"active":f.owner.handle().is_active(),"scopes":f.metrics.scopes.get(),"cleanups":f.metrics.cleanups.get(),"registry":f.metrics.rows.borrow().len()}).to_string())
    } else {
        Ok("null".into())
    }
}
