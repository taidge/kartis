//! Actual BrowserTree native-paint owner controls; no Widget host claim.
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeCanvasProgram,
    BrowserTreeHandle, BrowserTreeOptions, BrowserTreePicking, BrowserTreeSnapshot,
    MotionPreference,
};
use web_sys::js_sys::{Function, Reflect};

use super::*;

#[derive(Default)]
struct Metrics {
    builds: Cell<u32>,
    calls: Cell<u32>,
    nodes: Cell<u32>,
    links: Cell<u32>,
    fault: Cell<u8>,
    observations: RefCell<Vec<Value>>,
    owner: RefCell<Option<BrowserTreeHandle>>,
}
struct Fixture {
    root: web_sys::Element,
    owner: BrowserTree,
    metrics: Rc<Metrics>,
    variant: Cell<u8>,
    events: Rc<RefCell<Vec<String>>>,
}
thread_local! {
    static FIXTURES: RefCell<BTreeMap<String, Fixture>> = const { RefCell::new(BTreeMap::new()) };
}
fn raw(phase: u8) -> Vec<HierarchyDatum> {
    let mut data = vec![
        HierarchyDatum::new("a", Some("r".into()), 0.),
        HierarchyDatum::new("c", Some("a".into()), if phase == 1 { 35. } else { 30. }),
        HierarchyDatum::new("r", None, 0.),
        HierarchyDatum::new("b", Some("r".into()), if phase == 9 { 0. } else { -0. }),
        HierarchyDatum::new("d", Some("a".into()), 10.),
    ];
    for datum in &mut data {
        datum.label = format!("source-{}", datum.id);
    }
    data
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
fn datum(d: &HierarchyDatum) -> Value {
    json!({"id":d.id,"parent":d.parent,"label":d.label,"value":d.value,"bits":d.value.to_bits().to_string()})
}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn nodes(layout: &TreeLayout) -> Vec<Value> {
    layout.nodes.iter().map(|n|json!({"index":n.source_index,"parent":n.parent_index,"children":n.children,"position":n.position,"radius":n.radius,"rawBits":n.raw_value.to_bits().to_string(),"value":n.value,"color":n.color.map(rgba),"markId":n.mark_id})).collect()
}
fn accepted(s: &BrowserTreeSnapshot) -> Value {
    json!({"sceneDebug":format!("{:?}",s.layout.layout.scene),"source":s.layout.layout.source().iter().map(datum).collect::<Vec<_>>(),"nodes":nodes(&s.layout.layout),"config":{"width":s.config.width,"height":s.config.height,"margin":[s.config.margin.left,s.config.margin.top,s.config.margin.right,s.config.margin.bottom]},"activity":{"focus":s.layout.activity.focus_id,"nodes":s.layout.activity.nodes,"links":s.layout.activity.links},"chart":s.chart.as_ref().map(|c|json!({"width":c.width,"height":c.height,"revision":c.revision,"view":[c.view.scale,c.view.translate_x,c.view.translate_y],"hovered":c.hovered,"selected":c.selected,"hidden":c.hidden})),"active":s.active,"pending":s.pending,"rafPending":s.raf_pending,"desiredFocus":s.desired_focus,"error":s.error.as_ref().map(ToString::to_string)})
}
fn frame(frame: &TreeCanvasFrame) -> Value {
    json!({"source":frame.source().iter().map(datum).collect::<Vec<_>>(),"nodes":nodes(frame.layout()),"config":{"width":frame.config().width,"height":frame.config().height,"margin":[frame.config().margin.left,frame.config().margin.top,frame.config().margin.right,frame.config().margin.bottom]},"activity":frame.activity().map(|a|json!({"focus":a.focus_id,"nodes":a.nodes,"links":a.links})),"links":frame.layout().links.iter().map(|l|json!({"index":l.link_index,"source":l.source_index,"target":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"markId":l.mark_id})).collect::<Vec<_>>(),"linkStyles":frame.computed_link_styles().iter().map(|s|json!({"thickness":s.thickness,"bits":s.thickness.to_bits().to_string(),"color":rgba(s.color)})).collect::<Vec<_>>(),"labels":frame.layout().computed_canvas_labels().unwrap().iter().map(|l|json!({"sourceIndex":l.source_index,"text":l.label.text,"position":[l.label.x,l.label.y],"eligible":l.eligible,"font":format!("{:?}",l.font),"whitespace":format!("{:?}",l.whitespace)})).collect::<Vec<_>>()})
}
fn host(e: JsValue) -> ChartError {
    ChartError(format!("owner fixture native call failed: {e:?}"))
}
fn matrix(ctx: &web_sys::CanvasRenderingContext2d) -> Result<Vec<f64>> {
    let method = Reflect::get(ctx, &"getTransform".into())
        .map_err(host)?
        .dyn_into::<Function>()
        .map_err(host)?;
    let value = method.call0(ctx).map_err(host)?;
    ["a", "b", "c", "d", "e", "f"]
        .into_iter()
        .map(|key| {
            Reflect::get(&value, &key.into())
                .map_err(host)?
                .as_f64()
                .ok_or_else(|| ChartError("native matrix number missing".into()))
        })
        .collect()
}
fn observe(
    metrics: &Metrics,
    f: &TreeCanvasFrame,
    role: &str,
    index: usize,
    variant: u8,
    ctx: &web_sys::CanvasRenderingContext2d,
) -> Result<bool> {
    metrics.calls.set(metrics.calls.get() + 1);
    let handle = metrics.owner.borrow().clone();
    let owner = handle
        .as_ref()
        .map(|h| h.snapshot().map(|s| accepted(&s)))
        .transpose()?;
    let mut observation = json!({"call":metrics.calls.get(),"role":role,"index":index,"variant":variant,"frame":frame(f),"owner":owner,"matrix":matrix(ctx)?});
    if metrics.fault.get() == 2 && role == "link" && index == 0 {
        let h = handle
            .as_ref()
            .ok_or_else(|| ChartError("nested control requires mounted owner".into()))?;
        let before = accepted(&h.snapshot()?);
        let mut attempted = raw(1);
        attempted[1].value = 999.;
        let attempts = [
            h.set_source(attempted),
            h.set_config(config()),
            h.refresh(),
            h.flush(),
        ];
        observation["nested"] = json!(
            attempts
                .into_iter()
                .map(|r| match r {
                    Ok(()) => json!({"accepted":true}),
                    Err(e) => json!({"accepted":false,"error":e.to_string()}),
                })
                .collect::<Vec<_>>()
        );
        observation["beforeNested"] = before;
        observation["afterNested"] = accepted(&h.snapshot()?);
    }
    metrics.observations.borrow_mut().push(observation);
    if metrics.fault.get() == 3 {
        handle
            .ok_or_else(|| ChartError("stop control requires mounted owner".into()))?
            .stop();
        return Ok(false);
    }
    Ok(true)
}
fn program(metrics: Rc<Metrics>, variant: u8) -> BrowserTreeCanvasProgram {
    let links = metrics.clone();
    let render_link = move |ctx: &web_sys::CanvasRenderingContext2d,
                            c: &mut TreeCanvasLinkContext<'_>| {
        links.links.set(links.links.get() + 1);
        let _ = observe(&links, c.frame, "link", c.link_index, variant, ctx)?;
        // Disabled source links remain callback roles. This application elects
        // not to draw them; the node-only visual oracle is explicit.
        Ok(())
    };
    let render_node = move |ctx: &web_sys::CanvasRenderingContext2d,
                            c: &TreeCanvasNodeContext<'_>| {
        metrics.nodes.set(metrics.nodes.get() + 1);
        if !observe(&metrics, c.frame, "node", c.source_index, variant, ctx)? {
            return Ok(());
        }
        if metrics.fault.get() == 1 && c.datum.id == "a" {
            return Err(ChartError("owner authored late node rejection".into()));
        }
        let color = match (variant, c.is_active) {
            (0, Some(true)) => "#0064ff",
            (0, _) => "#6414b4",
            (_, Some(true)) => "#14b4dc",
            _ => "#00a064",
        };
        ctx.set_fill_style_str(color);
        ctx.fill_rect(c.node.position[0] + 35., c.node.position[1] - 7., 20., 14.);
        Ok(())
    };
    BrowserTreeCanvasProgram {
        layers: vec![TreeCanvasLayer::Links, TreeCanvasLayer::Nodes],
        render_node: Some(Box::new(render_node)),
        render_link: Some(Box::new(render_link)),
        ..Default::default()
    }
}
#[wasm_bindgen]
pub fn tree_canvas_owner_mount(key: &str, picking: u8) -> std::result::Result<(), JsValue> {
    if picking > 1 {
        return Err(js("unknown owner picking profile"));
    }
    tree_canvas_owner_stop(key)?;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("section")?;
    root.set_id(&format!("tree-canvas-owner-{key}"));
    root.set_attribute(
        "style",
        "position:relative;width:600px;height:360px;margin:12px 0;max-width:none;",
    )?;
    document.body().unwrap().append_child(&root)?;
    let metrics = Rc::new(Metrics::default());
    let mut tree = TreeOptions {
        node_radius: 24.,
        ..Default::default()
    };
    tree.style.links.enabled = false;
    tree.hierarchy.labels = false;
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
        picking: if picking == 0 {
            BrowserTreePicking::Geometry
        } else {
            BrowserTreePicking::Nearest { distance: 1e6 }
        },
        ..Default::default()
    };
    let built = metrics.clone();
    let build = move |context: &BrowserTreeBuildContext<'_>| {
        built.builds.set(built.builds.get() + 1);
        tree_canvas_layout_with_activity(
            context.config,
            context.source,
            context.tree,
            &mut TreeFactories::default(),
            context.activity,
            &mut TreeActivityFactories::default(),
        )
    };
    let owner = match BrowserTree::mount_with_factory_and_canvas(
        root.clone(),
        raw(0),
        config(),
        options,
        build,
        program(metrics.clone(), 0),
    ) {
        Ok(owner) => owner,
        Err(e) => {
            root.remove();
            return Err(js(e));
        }
    };
    *metrics.owner.borrow_mut() = Some(owner.handle());
    let events = Rc::new(RefCell::new(Vec::new()));
    let recorded = events.clone();
    owner.on_event(move |e| recorded.borrow_mut().push(format!("{e:?}")));
    owner.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().insert(
            key.to_owned(),
            Fixture {
                root,
                owner,
                metrics,
                variant: Cell::new(0),
                events,
            },
        )
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
            .ok_or_else(|| js("missing Tree Canvas owner fixture"))?)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        let state = h.snapshot();
        Ok(json!({"active":h.is_active(),"accepted":state.as_ref().ok().map(accepted),"snapshotError":state.err().map(|e|e.to_string()),"builds":f.metrics.builds.get(),"calls":f.metrics.calls.get(),"nodeCalls":f.metrics.nodes.get(),"linkCalls":f.metrics.links.get(),"fault":f.metrics.fault.get(),"desiredVariant":f.variant.get(),"observations":f.metrics.observations.borrow().len(),"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_observations(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        Ok(json!(f.metrics.observations.borrow().clone()).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_png(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| f.owner.handle().canvas_png_data_url().map_err(js))
}
#[wasm_bindgen]
pub fn tree_canvas_owner_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_canvas_owner_source(key: &str, phase: u8) -> std::result::Result<(), JsValue> {
    if ![0, 1, 9].contains(&phase) {
        return Err(js("unknown owner source phase"));
    }
    with(key, |f| {
        let h = f.owner.handle();
        h.set_source(raw(phase)).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_program(key: &str, variant: u8) -> std::result::Result<(), JsValue> {
    if variant > 1 {
        return Err(js("unknown owner program"));
    }
    with(key, |f| {
        f.owner
            .handle()
            .set_canvas_program(program(f.metrics.clone(), variant))
            .map_err(js)?;
        f.variant.set(variant);
        f.owner.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_fault(key: &str, kind: u8) -> std::result::Result<(), JsValue> {
    if kind > 3 {
        return Err(js("unknown owner failure"));
    }
    with(key, |f| {
        f.metrics.fault.set(kind);
        let h = f.owner.handle();
        h.refresh().map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_zoom(key: &str, enabled: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        if enabled {
            let c = h.snapshot().map_err(js)?.config;
            h.dispatch(SurfaceInput::Wheel {
                point: (c.width / 2., c.height / 2.),
                delta: -2f64.ln() / 0.002,
            })
            .map_err(js)?;
        } else {
            h.reset_view().map_err(js)?;
        }
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_margin(key: &str, left: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        let mut c = h.snapshot().map_err(js)?.config;
        c.margin.left = left;
        h.set_config(c).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_hidden(key: &str, id: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let ids = if id.is_empty() {
            Default::default()
        } else {
            [id.to_owned()].into_iter().collect()
        };
        let h = f.owner.handle();
        h.set_hidden(ids).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_canvas_owner_stop(key: &str) -> std::result::Result<bool, JsValue> {
    let removed = FIXTURES.with(|all| all.borrow_mut().remove(key));
    if let Some(f) = removed {
        f.owner.stop();
        f.root.remove();
        Ok(true)
    } else {
        Ok(false)
    }
}
