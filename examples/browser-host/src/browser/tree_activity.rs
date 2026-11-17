//! Real public BrowserTree owners; application code supplies borrowed factories.
use std::collections::{BTreeMap, BTreeSet};

use glory::Widget;
use glory::render::{Event as UiEvent, EventKind};
use glory::style::{Length, Style};
use kartis::browser::{
    BrowserHighlight, BrowserTree, BrowserTreeBuildContext, BrowserTreeHandle, BrowserTreeOptions,
    BrowserTreePicking, BrowserTreeSnapshot, BrowserTreeWidgetOptions, MotionPreference,
};

use super::*;
#[path = "../../../shared/tree_activity.rs"]
pub(super) mod model;
type Logs = Rc<RefCell<Vec<(TreeActivityLayout, Vec<String>)>>>;
type UiFaults = Rc<RefCell<BTreeMap<(u64, String, u32), glory::Cage<bool>>>>;
type Rows =
    Rc<RefCell<BTreeMap<(u64, String), glory::Bond<TypedPlotWidgetFrame<TreeActivityWidgetData>>>>>;
struct Fixture {
    key: String,
    profile: u8,
    ordered: bool,
    requested_highlight: BrowserHighlight,
    relation_flags: u8,
    helper_overlay: bool,
    root: web_sys::Element,
    css: web_sys::Element,
    owner: BrowserTree,
    phase: Rc<Cell<u8>>,
    failure: Rc<Cell<u8>>,
    builds: Rc<Cell<u32>>,
    calls: Rc<Cell<u32>>,
    cleanups: Rc<Cell<u32>>,
    logs: Logs,
    probes: Rc<RefCell<Vec<Value>>>,
    rows: Rows,
    ui_faults: UiFaults,
    events: Rc<Cell<u32>>,
    updates: Rc<Cell<u32>>,
    replaced_events: Rc<Cell<u32>>,
    replaced_updates: Rc<Cell<u32>>,
    event_log: Rc<RefCell<Vec<String>>>,
}
thread_local! {static FIXTURES:RefCell<Vec<Fixture>>=const {RefCell::new(Vec::new())};}
fn rgba(Color(r, g, b, a): Color) -> [u8; 4] {
    [r, g, b, a]
}
fn datum(d: &HierarchyDatum) -> Value {
    json!({"id":d.id,"label":d.label,"parent":d.parent,"value":d.value,"bits":d.value.to_bits().to_string()})
}
fn node(n: &TreeNode) -> Value {
    json!({"sourceIndex":n.source_index,"parentIndex":n.parent_index,"children":n.children,"depth":n.depth,"height":n.height,"rawValue":n.raw_value,"rawBits":n.raw_value.to_bits().to_string(),"value":n.value,"radius":n.radius,"position":n.position,"color":n.color.map(rgba),"markId":n.mark_id})
}
fn link(l: &TreeLink) -> Value {
    json!({"index":l.link_index,"sourceIndex":l.source_index,"targetIndex":l.target_index,"sourcePosition":l.source_position,"targetPosition":l.target_position,"position":l.position,"markId":l.mark_id})
}
fn activity(a: &TreeActivityState) -> Value {
    json!({"focus":a.focus_id,"focusIndex":a.focus_index,"nodes":a.nodes,"links":a.links,"flags":[a.highlight.ancestor_nodes,a.highlight.descendant_nodes,a.highlight.ancestor_links,a.highlight.descendant_links],"depth":a.fix_node_color_at_depth})
}
fn data(d: &TreeActivityWidgetData) -> Value {
    let (raw, nodes, links, index, role) = match &d.tree {
        TreeWidgetData::Node(n) => (&n.source, &n.nodes, &n.links, n.source_index, "node"),
        TreeWidgetData::Link(l) => (&l.source, &l.nodes, &l.links, l.link_index, "link"),
    };
    json!({"role":role,"index":index,"active":d.is_active,"activity":activity(&d.activity),"raw":raw.iter().map(datum).collect::<Vec<_>>(),"nodes":nodes.iter().map(node).collect::<Vec<_>>(),"links":links.iter().map(link).collect::<Vec<_>>()})
}
fn metadata(s: &BrowserTreeSnapshot, logs: &Logs) -> Value {
    let a = &s.layout;
    let l = &a.layout;
    let log = logs
        .borrow()
        .iter()
        .rev()
        .find(|(layout, _)| layout == a)
        .map(|(_, log)| log.clone())
        .unwrap_or_default();
    json!({"raw":l.source().iter().map(datum).collect::<Vec<_>>(),"plot":[l.plot.x,l.plot.y,l.plot.width,l.plot.height],"activity":activity(&a.activity),"log":log,"nodes":l.nodes.iter().map(node).collect::<Vec<_>>(),"links":l.links.iter().map(link).collect::<Vec<_>>(),"marks":l.scene.marks.iter().map(|m|json!({"id":m.id,"series":m.series,"label":m.label,"value":m.value,"fill":m.fill.map(rgba),"stroke":m.stroke.map(|s|json!({"color":rgba(s.color),"width":s.width})),"geometry":format!("{:?}",m.geometry)})).collect::<Vec<_>>(),"labels":l.scene.labels.iter().enumerate().map(|(i,l)|json!({"text":l.text,"position":[l.x,l.y],"owner":a.layout.scene.label_series.get(&i)})).collect::<Vec<_>>(),"binding":tree_activity_frame(a).unwrap().interaction.unwrap().sites.iter().map(|s|json!({"id":s.mark_id,"series":s.series,"label":s.label,"value":s.value,"color":rgba(s.color),"position":s.position})).collect::<Vec<_>>()})
}
#[wasm_bindgen]
pub fn tree_activity_mount(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    mount(key, profile, true, 0, 5, false)
}
#[wasm_bindgen]
pub fn tree_activity_mount_overlay(key: &str, profile: u8) -> std::result::Result<(), JsValue> {
    mount(key, profile, false, 0, 5, false)
}
/// Application configuration used by the native input lifecycle acceptance.
/// All input still comes through real DOM listeners and the public owner API.
#[wasm_bindgen]
pub fn tree_activity_mount_input(
    key: &str,
    profile: u8,
    ordered: bool,
    highlight_mode: u8,
    relation_flags: u8,
    helper_overlay: bool,
) -> std::result::Result<(), JsValue> {
    mount(
        key,
        profile,
        ordered,
        highlight_mode,
        relation_flags,
        helper_overlay,
    )
}
fn relation_options(flags: u8) -> TreeRelationOptions {
    TreeRelationOptions {
        ancestor_nodes: flags & 1 != 0,
        descendant_nodes: flags & 2 != 0,
        ancestor_links: flags & 4 != 0,
        descendant_links: flags & 8 != 0,
    }
}
fn mount(
    key: &str,
    profile: u8,
    ordered: bool,
    highlight_mode: u8,
    relation_flags: u8,
    helper_overlay: bool,
) -> std::result::Result<(), JsValue> {
    if profile > 1 || highlight_mode > 1 || relation_flags > 15 {
        return Err(js("bad Tree activity input configuration"));
    }
    let requested_highlight = if highlight_mode == 0 {
        BrowserHighlight::Controlled
    } else {
        BrowserHighlight::Series
    };
    let renderer = match key {
        "svg" => BrowserRenderer::Svg,
        "canvas" => BrowserRenderer::Canvas,
        _ => return Err(js("bad Tree activity renderer")),
    };
    let doc = web_sys::window().unwrap().document().unwrap();
    let root = doc.create_element("div")?;
    root.set_id(&format!("tree-activity-{key}"));
    root.set_attribute(
        "style",
        "width:720px;max-width:none;margin:12px 0;transform-origin:0 0",
    )?;
    doc.body().unwrap().append_child(&root)?;
    let css = doc.create_element("style")?;
    css.set_text_content(Some(".tree-activity-widget{display:block!important;font-size:0!important;border:0!important;border-radius:0!important;padding:0!important;outline:none!important;}"));
    doc.body().unwrap().append_child(&css)?;
    let phase = Rc::new(Cell::new(0));
    let failure = Rc::new(Cell::new(0));
    let builds = Rc::new(Cell::new(0));
    let calls = Rc::new(Cell::new(0));
    let cleanups = Rc::new(Cell::new(0));
    let logs: Logs = Rc::new(RefCell::new(vec![]));
    let probes = Rc::new(RefCell::new(vec![]));
    let owner_handle: Rc<RefCell<Option<BrowserTreeHandle>>> = Rc::new(RefCell::new(None));
    let rows: Rows = Rc::new(RefCell::new(BTreeMap::new()));
    let ui_faults: UiFaults = Rc::new(RefCell::new(BTreeMap::new()));
    let (p, f, b, cache) = (phase.clone(), failure.clone(), builds.clone(), logs.clone());
    let build = move |ctx: &BrowserTreeBuildContext<'_>| {
        b.set(b.get() + 1);
        let mut requested = ctx.activity.clone();
        if f.get() == 101 {
            requested.focus_id = Some("r".into());
        }
        let alternate = model::data(2);
        let raw = if f.get() == 100 {
            &alternate[..]
        } else {
            ctx.source
        };
        let source = model::source(
            p.get(),
            ctx.config,
            raw,
            ctx.tree,
            &requested,
            if f.get() < 20 { f.get() } else { 0 },
        )?;
        let mut layout = source.layout;
        if f.get() == 102 {
            layout.layout.scene.width = f64::NAN;
        }
        if f.get() == 103 {
            layout.layout = tree_layout(ctx.config, &alternate, ctx.tree)?;
        }
        cache.borrow_mut().push((layout.clone(), source.log));
        if cache.borrow().len() > 32 {
            cache.borrow_mut().remove(0);
        }
        Ok(layout)
    };
    let fail = failure.clone();
    let placement = move |ctx: &TreeActivityWidgetContext<'_>| {
        let (offset, measured) = match &ctx.tree {
            TreeWidgetContext::Node(n) if n.datum.id == "b" => (
                [-10., 30.],
                TextSize {
                    width: 90.,
                    height: 22.,
                },
            ),
            TreeWidgetContext::Link(l) if l.target_datum.id == "c" => (
                [-50., -26.],
                TextSize {
                    width: 100.,
                    height: 22.,
                },
            ),
            _ => return Ok(None),
        };
        if fail.get() == 22 {
            return Ok(Some(PlotWidgetPlacement::Rect(LayoutRect {
                x: f64::NAN,
                y: 0.,
                width: measured.width,
                height: measured.height,
            })));
        }
        Ok(Some(PlotWidgetPlacement::AtDatum { offset, measured }))
    };
    let (fail, called, retired, observed, handle, live_rows) = (
        failure.clone(),
        calls.clone(),
        cleanups.clone(),
        probes.clone(),
        owner_handle.clone(),
        rows.clone(),
    );
    let style_faults = ui_faults.clone();
    let factory = move |context: PlotWidgetCompositionContext<TreeActivityWidgetData>| {
        called.set(called.get() + 1);
        let frame = context.frame;
        let born = context.revision;
        let fault_key = (born, context.mark_id.clone(), called.get());
        let style_faults = style_faults.clone();
        if fail.get() == 20 {
            return Err(ChartError("Tree activity Widget factory rejected".into()));
        }
        live_rows
            .borrow_mut()
            .insert((born, context.mark_id), frame);
        let reject = fail.get() == 21;
        let retired = retired.clone();
        let observed = observed.clone();
        let handle = handle.clone();
        let live_rows = live_rows.clone();
        Ok(glory::ui::widget(move |scope| {
            let retired = retired.clone();
            scope.on_cleanup(move || retired.set(retired.get() + 1))?;
            let ui_fault = scope.cage(false);
            style_faults
                .borrow_mut()
                .insert(fault_key.clone(), ui_fault);
            let cleanup_faults = style_faults.clone();
            let cleanup_fault_key = fault_key.clone();
            scope.on_cleanup(move || {
                cleanup_faults.borrow_mut().remove(&cleanup_fault_key);
            })?;
            let count = scope.cage(0u32);
            let focused = scope.cage(false);
            let observed = observed.clone();
            let handle = handle.clone();
            let live_rows = live_rows.clone();
            scope.effect(move || {
                let value=frame.get();
                let Some(handle)=handle.borrow().clone() else{return;};
                let record=match handle.snapshot() {
                    Ok(s)=>{
                        let actual=data(&value.data);let l=&s.layout.layout;
                        let activity=activity(&s.layout.activity);
                        let raw=json!(l.source().iter().map(datum).collect::<Vec<_>>());
                        let nodes=json!(l.nodes.iter().map(node).collect::<Vec<_>>());
                        let links=json!(l.links.iter().map(link).collect::<Vec<_>>());
                        let index=actual["index"].as_u64().unwrap() as usize;
                        let role_active=match &value.data.tree {TreeWidgetData::Node(_)=>s.layout.activity.nodes.get(index),TreeWidgetData::Link(_)=>s.layout.activity.links.get(index)};
                        let consistent=actual["raw"]==raw&&actual["nodes"]==nodes&&actual["links"]==links&&actual["activity"]==activity&&role_active==Some(&value.data.is_active);
                        let tentative=born!=s.widget_revision;
                        let mut rows=BTreeMap::new();let mut all_consistent=true;
                        if !tentative {for id in ["node:b","link:c"] {
                            if !l.scene.marks.iter().any(|m|m.id==id){continue;}
                            let row=live_rows.borrow().get(&(s.widget_revision,id.to_owned())).copied();
                            let row=row.and_then(|f|f.try_get().ok());
                            if let Some(row)=row {
                                let d=data(&row.data);let i=d["index"].as_u64().unwrap() as usize;
                                let expected=match &row.data.tree{TreeWidgetData::Node(_)=>s.layout.activity.nodes.get(i),TreeWidgetData::Link(_)=>s.layout.activity.links.get(i)};
                                let same=d["raw"]==raw&&d["nodes"]==nodes&&d["links"]==links&&d["activity"]==activity&&expected==Some(&row.data.is_active)&&row.frame.mark.id==id;
                                all_consistent&=same;rows.insert(id.to_owned(),d);
                            }else{all_consistent=false;rows.insert(id.to_owned(),json!({"error":"missing live accepted row"}));}
                        }}
                        json!({"born":born,"tentative":tentative,"id":value.frame.mark.id,"consistent":consistent,"allConsistent":all_consistent,"rows":rows,"child":actual,"owner":{"raw":raw,"nodes":nodes,"links":links,"activity":activity,"revision":s.widget_revision}})
                    },
                    Err(e)=>json!({"born":born,"error":e.to_string()}),
                };
                observed.borrow_mut().push(record);
            });
            glory::ui::button()
                .class("tree-activity-widget")
                .style(scope.bond(move || {
                    frame.with(|f| {
                        let color = if f.data.is_active == Some(true) {
                            Color(0, 100, 255, 255)
                        } else {
                            Color(100, 20, 180, 255)
                        };
                        // Acceptance fixture: phase0 always stays valid. Retained phase1
                        // fails only in the post-adoption dynamic UI binding.
                        let source = match &f.data.tree {
                            TreeWidgetData::Node(n) => &n.source,
                            TreeWidgetData::Link(l) => &l.source,
                        };
                        let phase1 = source.iter().any(|d| d.id == "c" && d.value == 35.);
                        let width = if ui_fault.get() && phase1 {
                            Length::Px(f64::NAN)
                        } else {
                            Length::Percent(100.)
                        };
                        Style {
                            background: Some(color),
                            color: Some(color),
                            ..Style::new().width(width).height(Length::Percent(100.))
                        }
                    })
                }))
                .label(scope.bond(move || {
                    frame.with(|f| {
                        format!(
                            "{}|count={}|focus={}|born={}",
                            {
                                let mut d = data(&f.data);
                                d["frameId"] = json!(f.frame.mark.id);
                                d["rect"] = json!([
                                    f.frame.rect.x,
                                    f.frame.rect.y,
                                    f.frame.rect.width,
                                    f.frame.rect.height
                                ]);
                                d
                            },
                            count.get(),
                            focused.get(),
                            born
                        )
                    })
                }))
                .on(EventKind::Activate, move |_| count.revise(|v| *v += 1))
                .on(EventKind::FocusChanged, move |e| {
                    if let UiEvent::FocusChanged(v) = e {
                        focused.revise(|old| *old = v);
                    }
                })
                .show_in(scope)?;
            if reject {
                return Err(glory::render::Error::InvalidProps(
                    "Tree activity Widget build rejected",
                )
                .into());
            }
            Ok(())
        }))
    };
    let options = BrowserTreeOptions {
        chart: BrowserOptions {
            renderer,
            height: Some(340.),
            legend: false,
            highlight: requested_highlight,
            crosshair: helper_overlay.then(CrosshairOptions::default),
            motion: MotionPreference::Reduced,
            ..Default::default()
        },
        tree: model::tree(0),
        activity: TreeActivityOptions {
            highlight: relation_options(relation_flags),
            ..Default::default()
        },
        picking: if profile == 0 {
            BrowserTreePicking::Geometry
        } else {
            BrowserTreePicking::Nearest {
                distance: 1_000_000.,
            }
        },
    };
    let owner = match BrowserTree::mount_with_factory_and_widgets(
        root.clone(),
        model::data(0),
        model::config(720.),
        options,
        build,
        BrowserTreeWidgetOptions {
            revision: 0,
            replace_ink: profile == 1,
            source_ordered: ordered,
        },
        placement,
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
    let events = Rc::new(Cell::new(0));
    let updates = Rc::new(Cell::new(0));
    let replaced_events = Rc::new(Cell::new(0));
    let replaced_updates = Rc::new(Cell::new(0));
    let replaced = replaced_events.clone();
    owner.on_event(move |_| replaced.set(replaced.get() + 1));
    let counted = events.clone();
    let event_log = Rc::new(RefCell::new(vec![]));
    let log = event_log.clone();
    owner.on_event(move |event| {
        counted.set(counted.get() + 1);
        let mut log = log.borrow_mut();
        log.push(format!("{event:?}"));
        if log.len() > 512 {
            log.remove(0);
        }
    });
    let replaced = replaced_updates.clone();
    owner.on_update(move |_| replaced.set(replaced.get() + 1));
    let counted = updates.clone();
    owner.on_update(move |_| counted.set(counted.get() + 1));
    owner.flush().map_err(js)?;
    FIXTURES.with(|all| {
        all.borrow_mut().push(Fixture {
            key: key.into(),
            profile,
            ordered,
            requested_highlight,
            relation_flags,
            helper_overlay,
            root,
            css,
            owner,
            phase,
            failure,
            builds,
            calls,
            cleanups,
            logs,
            probes,
            rows,
            ui_faults,
            events,
            updates,
            replaced_events,
            replaced_updates,
            event_log,
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
            .ok_or_else(|| js("missing Tree activity owner"))?)
    })
}
#[wasm_bindgen]
pub fn tree_activity_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let s = f.owner.snapshot().map_err(js)?;
        let c = s.chart.as_ref();
        Ok(json!({
            "accepted":metadata(&s,&f.logs),
            "state":{
                "active":s.active,"desiredFocus":s.desired_focus,
                "pending":s.pending,"rafPending":s.raf_pending,
                "error":s.error.as_ref().map(ToString::to_string),
                "revision":s.widget_revision,"width":s.config.width,"height":s.config.height,
                // The public owner snapshot intentionally permits no Chart
                // snapshot during an Engine-owned synchronous callback.
                "chartReadAvailable":c.is_some(),
                "chartRevision":c.map(|c|c.revision),
                "hovered":c.map(|c|&c.hovered),"selected":c.map(|c|&c.selected),
                "hidden":c.map(|c|&c.hidden),
                "view":c.map(|c|[c.view.scale,c.view.translate_x,c.view.translate_y]),
            },
            "diagnostics":{
                "builds":f.builds.get(),"calls":f.calls.get(),"cleanups":f.cleanups.get(),
                "profile":f.profile,"ordered":f.ordered,"events":f.events.get(),
                "updates":f.updates.get(),"replacedEvents":f.replaced_events.get(),
                "replacedUpdates":f.replaced_updates.get(),"probes":f.probes.borrow().len(),
            },
        })
        .to_string())
    })
}
#[wasm_bindgen]
pub fn tree_activity_set(
    key: &str,
    phase: u8,
    flags: u8,
    depth: i32,
) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.phase.set(phase);
        let handle = f.owner.handle();
        let s = handle.snapshot().map_err(js)?;
        let mut activity = TreeActivityOptions {
            focus_id: s.desired_focus,
            fix_node_color_at_depth: if depth < 0 {
                None
            } else {
                Some(depth as usize)
            },
            ..Default::default()
        };
        activity.highlight = TreeRelationOptions {
            ancestor_nodes: flags & 1 != 0,
            descendant_nodes: flags & 2 != 0,
            ancestor_links: flags & 4 != 0,
            descendant_links: flags & 8 != 0,
        };
        handle.set_activity_options(activity).map_err(js)?;
        handle.set_tree_options(model::tree(phase)).map_err(js)?;
        handle.set_source(model::data(phase)).map_err(js)?;
        handle.refresh().map_err(js)?;
        handle.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_failure(key: &str, failure: u8) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.failure.set(failure);
        let handle = f.owner.handle();
        let result = if (20..=21).contains(&failure) {
            handle.set_widget_revision(7)
        } else {
            handle.refresh()
        };
        result.and_then(|_| handle.flush()).map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_recover(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.failure.set(0);
        let h = f.owner.handle();
        h.set_widget_revision(0).map_err(js)?;
        h.refresh().map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_flush(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| f.owner.flush().map_err(js))
}
#[wasm_bindgen]
pub fn tree_activity_clear(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.owner.clear_current().map_err(js)?;
        f.owner.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_width(key: &str, width: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        f.root
            .unchecked_ref::<web_sys::HtmlElement>()
            .style()
            .set_property("width", &format!("{width}px"))?;
        let mut c = f.owner.snapshot().map_err(js)?.config;
        c.width = width;
        f.owner.handle().set_config(c).map_err(js)?;
        f.owner.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_zoom(key: &str, zoom: bool) -> std::result::Result<(), JsValue> {
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
pub fn tree_activity_stop(key: &str) -> std::result::Result<bool, JsValue> {
    let fixture = FIXTURES.with(|all| {
        let mut all = all.borrow_mut();
        let i = all
            .iter()
            .position(|f| f.key == key)
            .ok_or_else(|| js("missing Tree activity owner"))?;
        Ok::<_, JsValue>(all.remove(i))
    })?;
    // Stop can synchronously release native pointer capture. Application event
    // observers must be able to read the registry without reentrant borrowing.
    let h = fixture.owner.handle();
    fixture.owner.stop();
    let active = h.is_active();
    fixture.root.remove();
    fixture.css.remove();
    Ok(active)
}
#[wasm_bindgen]
pub fn tree_activity_probes(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        Ok(serde_json::to_string(&*f.probes.borrow()).unwrap())
    })
}
#[wasm_bindgen]
pub fn tree_activity_revision(key: &str, revision: u64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.set_widget_revision(revision).map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_hidden(key: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.set_hidden(if hidden {
            BTreeSet::from(["c".to_owned()])
        } else {
            BTreeSet::new()
        })
        .map_err(js)?;
        h.flush().map_err(js)
    })
}
#[wasm_bindgen]
pub fn tree_activity_dispatch(key: &str, x: f64, y: f64) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.dispatch(SurfaceInput::Move {
            id: 1,
            point: (x, y),
        })
        .map_err(js)?;
        h.flush().map_err(js)
    })
}
/// A single explicit public source replacement during a native gesture.
#[wasm_bindgen]
pub fn tree_activity_input_source(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.set_source(model::data(1)).map_err(js)?;
        h.flush().map_err(js)
    })
}
/// A single explicit public Tree option replacement during a native gesture.
#[wasm_bindgen]
pub fn tree_activity_input_options(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.set_tree_options(model::tree(3)).map_err(js)?;
        h.flush().map_err(js)
    })
}
/// Static preflight rejection; it must not cancel an otherwise valid gesture.
#[wasm_bindgen]
pub fn tree_activity_input_invalid_request(key: &str) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let mut source = model::data(0);
        source.push(source[0].clone());
        f.owner.handle().set_source(source).map_err(js)
    })
}
/// Requested policy and the chart's actual accepted policy are read separately.
#[wasm_bindgen]
pub fn tree_activity_input_snapshot(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let effective = f.owner.handle().highlight().map_err(js)?;
        Ok(json!({
            "requestedHighlight":format!("{:?}",f.requested_highlight),
            "effectiveHighlight":format!("{effective:?}"),
            "relationFlags":f.relation_flags,
            "helperOverlay":f.helper_overlay,
            "profile":f.profile,
            "ordered":f.ordered,
            "eventLog":&*f.event_log.borrow(),
        })
        .to_string())
    })
}
#[wasm_bindgen]
pub fn tree_activity_mesh_case(key: &str, kind: u8) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let h = f.owner.handle();
        h.clear_current().map_err(js)?;
        h.flush().map_err(js)?;
        f.phase.set(0);
        let mut tree = model::tree(0);
        let mut source = model::data(0);
        match kind {
            0 => tree.style.nodes.opacity = 0.,
            1 => source
                .iter_mut()
                .find(|d| d.id == "b")
                .unwrap()
                .label
                .clear(),
            _ => return Err(js("bad independent Tree mesh case")),
        }
        h.set_tree_options(tree).map_err(js)?;
        h.set_source(source).map_err(js)?;
        h.refresh().map_err(js)?;
        h.flush().map_err(js)?;
        let s = h.snapshot().map_err(js)?;
        Ok(json!({"opacity":s.layout.layout.options().style.nodes.opacity,"raw":s.layout.layout.source().iter().map(datum).collect::<Vec<_>>(),"binding":tree_activity_frame(&s.layout).unwrap().interaction.unwrap().sites.iter().map(|v|v.mark_id.clone()).collect::<Vec<_>>(),"picking":format!("{:?}",s.picking)}).to_string())
    })
}

/// Acceptance fixture fault. Arming does not flush or change Tree source.
#[wasm_bindgen]
pub fn tree_activity_set_ui_fault(key: &str, armed: bool) -> std::result::Result<(), JsValue> {
    with(key, |f| {
        let flags = f.ui_faults.borrow().values().copied().collect::<Vec<_>>();
        for flag in flags {
            flag.revise(|value| *value = armed);
        }
        Ok(())
    })
}
/// Reads actual retained typed Bonds even if a failed UI binding leaves old DOM.
#[wasm_bindgen]
pub fn tree_activity_live_data_read(key: &str) -> std::result::Result<String, JsValue> {
    with(key, |f| {
        let snapshot = f.owner.snapshot().map_err(js)?;
        let mut result = BTreeMap::new();
        for id in ["node:b", "link:c"] {
            if !snapshot
                .layout
                .layout
                .scene
                .marks
                .iter()
                .any(|m| m.id == id)
            {
                continue;
            }
            let frame = f
                .rows
                .borrow()
                .get(&(snapshot.widget_revision, id.to_owned()))
                .copied()
                .ok_or_else(|| js("missing actual live Frame Bond"))?;
            let value = frame.try_get().map_err(js)?;
            result.insert(id, json!({"frameId":value.frame.mark.id,"revision":snapshot.widget_revision,"data":data(&value.data)}));
        }
        Ok(json!({"owner":metadata(&snapshot,&f.logs),"rows":result}).to_string())
    })
}
