//! Real typed graph relations on Tree/ForceNetwork/Chord/DAG/circular Sankey.
use std::collections::BTreeSet;

use super::*;
struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    case: u8,
    phase: Rc<Cell<u8>>,
    layout: Rc<RefCell<RelationLayout>>,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static RELATIONS:RefCell<Vec<Fixture>>=const{RefCell::new(Vec::new())};}
fn config(width: f64, height: f64) -> ChartConfig {
    ChartConfig {
        width,
        height,
        title: "Typed graph relations".into(),
        margin: Margin {
            left: 56.,
            right: 56.,
            top: 46.,
            bottom: 46.,
        },
        ..Default::default()
    }
}
fn options(case: u8, phase: u8) -> RelationOptions {
    RelationOptions {
        mode: match case {
            0 => RelationMode::Tree(TreeRelationOptions {
                ancestor_nodes: phase == 0,
                descendant_nodes: phase == 1,
                ancestor_links: phase == 1,
                descendant_links: phase == 0,
            }),
            2 => RelationMode::Chord,
            3 | 4 => RelationMode::Sankey,
            _ => RelationMode::Neighbors,
        },
        ..Default::default()
    }
}
fn add_legend(layout: RelationLayout, c: &ChartConfig) -> Result<RelationLayout> {
    let mut scene = layout.authored_scene().clone();
    scene.legend = layout
        .graph()
        .nodes()
        .iter()
        .enumerate()
        .map(|(i, n)| LegendItem {
            id: n.id.clone(),
            label: n.id.clone(),
            color: n
                .mark_ids
                .iter()
                .find_map(|id| {
                    scene
                        .marks
                        .iter()
                        .find(|m| &m.id == id)
                        .and_then(|m| m.fill.or(m.stroke.map(|s| s.color)))
                })
                .unwrap_or(c.theme.color(i)),
        })
        .collect();
    RelationLayout::new(scene, layout.graph().clone(), layout.options().clone())
}
fn proposal(case: u8, phase: u8, width: f64, height: f64) -> Result<RelationLayout> {
    if case > 4 || phase > 1 {
        return Err(ChartError(
            "relations case/phase outside fixture profile".into(),
        ));
    }
    let c = config(width, height);
    let opt = options(case, phase);
    match case {
        0 => {
            let data = [
                ("r", None, 0.),
                ("a", Some("r"), 0.),
                ("b", Some("r"), 0.),
                ("a1", Some("a"), if phase == 0 { 2. } else { 8. }),
                (
                    if phase == 0 { "a2" } else { "a3" },
                    Some("a"),
                    if phase == 0 { 3. } else { 4. },
                ),
                ("b1", Some("b"), 5.),
            ]
            .into_iter()
            .map(|(id, parent, value)| HierarchyDatum::new(id, parent.map(str::to_owned), value))
            .collect::<Vec<_>>();
            let mut tree = TreeOptions {
                hierarchy: HierarchyOptions {
                    include_root: true,
                    ..Default::default()
                },
                node_radius: 12.,
                ..Default::default()
            };
            tree.style.links.width = 4.;
            tree.style.links.color = TreeLinkColor::Fixed(Color(70, 90, 120, 255));
            add_legend(tree_relation_layout(&c, &data, &tree, &opt)?, &c)
        }
        1 => {
            let ids = ["a", "b", "c", if phase == 0 { "d" } else { "e" }];
            let nodes = ids.map(|id| NetworkNode {
                id: id.into(),
                label: id.into(),
                radius: if phase == 1 { 16. } else { 12. },
            });
            let edges = if phase == 0 {
                vec![
                    ("ab", "a", "b", 6.),
                    ("ac", "a", "c", 3.),
                    ("bd", "b", "d", 4.),
                ]
            } else {
                vec![
                    ("ab", "a", "b", 9.),
                    ("bc", "b", "c", 4.),
                    ("ae", "a", "e", 2.),
                ]
            };
            let links = edges
                .iter()
                .map(|(_, a, b, v)| NetworkLink::new(*a, *b, *v))
                .collect::<Vec<_>>();
            let mut sim = ForceSimulation::new(&nodes, &links, Default::default())?;
            for (id, position) in
                ids.into_iter()
                    .zip([[-160., -50.], [0., -95.], [100., 80.], [195., 10.]])
            {
                sim.set_position(id, position)?;
                sim.set_fixed(id, Some(position))?;
            }
            sim.step(4)?;
            add_legend(
                network_relation_layout(
                    &c,
                    &sim,
                    &edges.iter().map(|e| e.0.to_string()).collect::<Vec<_>>(),
                    &opt,
                )?,
                &c,
            )
        }
        2 => {
            let ids = ["a", "b", "c", if phase == 0 { "d" } else { "e" }].map(str::to_owned);
            let matrix = if phase == 0 {
                vec![
                    vec![0., 3., 2., 0.],
                    vec![2., 0., 0., 4.],
                    vec![1., 0., 0., 1.],
                    vec![0., 2., 1., 0.],
                ]
            } else {
                vec![
                    vec![0., 4., 0., 2.],
                    vec![3., 0., 1., 0.],
                    vec![0., 2., 0., 2.],
                    vec![1., 0., 2., 0.],
                ]
            };
            chord_relation_layout(&c, &ids, &matrix, 0.05, &opt)
        }
        _ => {
            let side = if phase == 0 { "c" } else { "e" };
            let nodes = ["a", "b", side, "d"].map(FlowNode::new);
            let mut links = vec![
                SankeyStyleLink::new("ab", "a", "b", if phase == 0 { 4. } else { 7. }),
                SankeyStyleLink::new("ab2", "a", "b", 2.),
                SankeyStyleLink::new(if phase == 0 { "ac" } else { "ae" }, "a", side, 3.),
                SankeyStyleLink::new("bd", "b", "d", if phase == 0 { 6. } else { 9. }),
                SankeyStyleLink::new(if phase == 0 { "cd" } else { "ed" }, side, "d", 3.),
            ];
            if case == 4 {
                links.extend([
                    SankeyStyleLink::new("return", "d", "a", 1.5),
                    SankeyStyleLink::new("self", "b", "b", 0.5),
                    SankeyStyleLink::new("ba", "b", "a", 1.),
                ]);
            }
            let style = SankeyStyleOptions {
                node: SankeyNodeStyle {
                    opacity: 0.85,
                    corner_radius: 5.,
                    ..Default::default()
                },
                link: SankeyLinkStyle {
                    color: SankeyLinkColor::Gradient,
                    opacity: Some(0.7),
                },
                label: SankeyLabelStyle {
                    position: SankeyLabelPosition::Inside,
                    ..Default::default()
                },
                ..Default::default()
            };
            let l = if case == 4 {
                circular_sankey_with_style(&c, &nodes, &links, &Default::default(), &style)?
            } else {
                sankey_with_style(&c, &nodes, &links, &Default::default(), &style)?
            };
            l.relation_layout(&opt)
        }
    }
}
fn publish_focus(
    layout: &Rc<RefCell<RelationLayout>>,
    weak: &BrowserHandle,
    focus: Option<RelationFocus>,
) -> Result<bool> {
    let mut next = {
        let old = layout.borrow();
        if old.focus() == focus.as_ref() {
            return Ok(false);
        }
        old.clone()
    };
    next.set_focus(focus)?;
    let unchanged = next == *layout.borrow();
    if unchanged {
        return Ok(false);
    }
    weak.set_scene(next.scene().clone())?;
    *layout.borrow_mut() = next;
    Ok(true)
}
fn visible_focus(layout: &RelationLayout, id: &str) -> Option<RelationFocus> {
    layout
        .scene()
        .marks
        .iter()
        .any(|m| m.id == id && !m.label.is_empty())
        .then(|| layout.graph().focus_for_mark(id).cloned())
        .flatten()
}
fn focus_value(f: Option<&RelationFocus>) -> Value {
    match f {
        Some(RelationFocus::Node(id)) => json!({"kind":"node","id":id}),
        Some(RelationFocus::Edge(id)) => json!({"kind":"edge","id":id}),
        None => Value::Null,
    }
}
#[wasm_bindgen]
pub fn relations_mount(case: u8) -> std::result::Result<(), JsValue> {
    relations_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for renderer in ["svg", "canvas"] {
        let initial = proposal(case, 0, 640., 420.).map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("relations-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            initial.scene().clone(),
            BrowserOptions {
                renderer: if renderer == "svg" {
                    BrowserRenderer::Svg
                } else {
                    BrowserRenderer::Canvas
                },
                height: Some(420.),
                highlight: browser::BrowserHighlight::Controlled,
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    pinch: false,
                    brush: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let layout = Rc::new(RefCell::new(initial));
        let phase = Rc::new(Cell::new(0));
        let accepted = layout.clone();
        let p = phase.clone();
        let keyboard = Rc::new(RefCell::new(None::<String>));
        let hovered = Rc::new(RefCell::new(Vec::<String>::new()));
        let keys = keyboard.clone();
        let hover_cache = hovered.clone();
        let controller = CategoryViewport::new(
            vec!["fixed".into()],
            ClipRect::plot(&config(640., 420.)),
            ViewportLimits::default(),
        )
        .map_err(js)?
        .into_controller();
        chart
            .on_viewport(controller, ViewportSelection::X, move |request| {
                let incoming = proposal(case, p.get(), request.width, request.height)?;
                let opts = incoming.options().clone();
                let mut next = accepted.borrow().clone();
                next.replace(incoming)?;
                next.set_options(opts)?;
                let hidden = request
                    .hidden
                    .into_iter()
                    .filter(|id| next.graph().node(id).is_some())
                    .collect();
                next.set_hidden(hidden)?;
                let retired_key = keys
                    .borrow()
                    .as_ref()
                    .is_some_and(|id| visible_focus(&next, id).is_none());
                if retired_key {
                    *keys.borrow_mut() = None;
                }
                hover_cache
                    .borrow_mut()
                    .retain(|id| visible_focus(&next, id).is_some());
                let scene = next.scene().clone();
                *accepted.borrow_mut() = next;
                Ok(scene)
            })
            .map_err(js)?;
        let events = Rc::new(RefCell::new(Vec::new()));
        let log = events.clone();
        let cache = layout.clone();
        let weak = chart.handle();
        chart.on_event(move |event| {
            {
                let mut log = log.borrow_mut();
                if log.len() == 128 {
                    log.remove(0);
                }
                log.push(event_value(event.clone()));
            }
            let desired = match &event {
                BrowserEvent::Interaction(SurfaceEvent::Hover(ids)) => {
                    *hovered.borrow_mut() = ids.clone();
                    let layout = cache.borrow();
                    Some(
                        ids.iter()
                            .find_map(|id| visible_focus(&layout, id))
                            .or_else(|| {
                                keyboard
                                    .borrow()
                                    .as_ref()
                                    .and_then(|id| visible_focus(&layout, id))
                            }),
                    )
                }
                BrowserEvent::Focus(id) => {
                    let layout = cache.borrow();
                    let focused = id.as_ref().and_then(|id| visible_focus(&layout, id));
                    *keyboard.borrow_mut() = id.as_ref().filter(|_| focused.is_some()).cloned();
                    if focused.is_some() {
                        hovered.borrow_mut().clear();
                    }
                    Some(
                        hovered
                            .borrow()
                            .iter()
                            .find_map(|id| visible_focus(&layout, id))
                            .or(focused),
                    )
                }
                BrowserEvent::Interaction(SurfaceEvent::Activated(id)) => {
                    let layout = cache.borrow();
                    let focused = visible_focus(&layout, id);
                    if focused.is_some() {
                        *keyboard.borrow_mut() = Some(id.clone());
                    }
                    Some(
                        hovered
                            .borrow()
                            .iter()
                            .find_map(|id| visible_focus(&layout, id))
                            .or(focused),
                    )
                }
                _ => None,
            };
            if let Some(focus) = desired
                && let Err(error) = publish_focus(&cache, &weak, focus)
            {
                log.borrow_mut()
                    .push(json!({"type":"fixture-error","message":error.0}));
            }
        });
        RELATIONS.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: renderer.into(),
                case,
                phase,
                layout,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn relations_phase(phase: u8) -> std::result::Result<(), JsValue> {
    RELATIONS.with(|slot| {
        for f in slot.borrow().iter() {
            let candidate = proposal(f.case, phase, 640., 420.).map_err(js)?;
            let hidden = f
                .chart
                .snapshot()
                .map_err(js)?
                .hidden
                .into_iter()
                .filter(|id| candidate.graph().node(id).is_some())
                .collect();
            f.phase.set(phase);
            f.chart.handle().set_hidden(hidden).map_err(js)?;
            f.chart.handle().refresh_viewport().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn relations_focus(renderer: &str, kind: &str, id: &str) -> std::result::Result<bool, JsValue> {
    RELATIONS.with(|slot| {
        let entries = slot.borrow();
        let f = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("relations renderer unavailable"))?;
        let focus = match kind {
            "node" => Some(RelationFocus::Node(id.into())),
            "edge" => Some(RelationFocus::Edge(id.into())),
            "none" => None,
            _ => return Err(js("invalid typed relation focus")),
        };
        publish_focus(&f.layout, &f.chart.handle(), focus).map_err(js)
    })
}
#[wasm_bindgen]
pub fn relations_flush() -> std::result::Result<(), JsValue> {
    RELATIONS.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
fn probe(layout: &RelationLayout, focus: &RelationFocus) -> Option<Value> {
    let ids = match focus {
        RelationFocus::Node(id) => &layout.graph().node(id)?.mark_ids,
        RelationFocus::Edge(id) => &layout.graph().edge(id)?.mark_ids,
    };
    for id in ids {
        let mark = layout.scene().marks.iter().find(|m| &m.id == id)?;
        if mark.label.is_empty() {
            continue;
        }
        let safe = |x: f64, y: f64| {
            [-1., 0., 1.].into_iter().all(|dx| {
                [-1., 0., 1.].into_iter().all(|dy| {
                    layout
                        .scene()
                        .hit_test(x + dx, y + dy)
                        .is_some_and(|m| &m.id == id)
                })
            })
        };
        let bounds = match &mark.geometry {
            Geometry::Circle { x, y, radius } => {
                if safe(*x, *y) {
                    return Some(
                        json!({"id":id,"focus":focus_value(Some(focus)),"x":x,"y":y,"value":mark.value,"label":mark.label,"control":format!("{}: {}",mark.label,mark.value)}),
                    );
                }
                [*x - *radius, *y - *radius, 2. * radius, 2. * radius]
            }
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => [*x, *y, *width, *height],
            Geometry::Path(commands) => {
                let points = commands
                    .iter()
                    .flat_map(|c| match c {
                        PathCommand::Move(p) | PathCommand::Line(p) => vec![*p],
                        PathCommand::Cubic { first, second, end } => vec![*first, *second, *end],
                        PathCommand::Close => vec![],
                    })
                    .collect::<Vec<_>>();
                let xmin = points.iter().map(|p| p.x).fold(f64::INFINITY, f64::min);
                let xmax = points.iter().map(|p| p.x).fold(f64::NEG_INFINITY, f64::max);
                let ymin = points.iter().map(|p| p.y).fold(f64::INFINITY, f64::min);
                let ymax = points.iter().map(|p| p.y).fold(f64::NEG_INFINITY, f64::max);
                [xmin, ymin, xmax - xmin, ymax - ymin]
            }
        };
        for iy in 1..22 {
            for ix in 1..22 {
                let x = bounds[0] + bounds[2] * f64::from(ix) / 22.;
                let y = bounds[1] + bounds[3] * f64::from(iy) / 22.;
                if safe(x, y) {
                    return Some(
                        json!({"id":id,"focus":focus_value(Some(focus)),"x":x,"y":y,"value":mark.value,"label":mark.label,"control":format!("{}: {}",mark.label,mark.value)}),
                    );
                }
            }
        }
    }
    None
}
#[wasm_bindgen]
pub fn relations_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    RELATIONS.with(|slot|{let entries=slot.borrow();let Some(f)=entries.iter().find(|f|f.renderer==renderer)else{return Ok(json!({"active":false}).to_string())};let s=f.chart.snapshot().map_err(js)?;let l=f.layout.borrow();let edge=l.graph().edges().iter().find(|e|e.source=="a"&&e.target=="b").map(|e|RelationFocus::Edge(e.id.clone()));Ok(json!({"active":s.active,"revision":s.revision,"pending":s.pending,"width":s.width,"height":s.height,"case":f.case,"phase":f.phase.get(),"hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"focus":focus_value(l.focus()),"relatedNodes":l.selection().nodes,"relatedEdges":l.selection().edges,"highlight":l.selection().highlight,"nodeTarget":probe(&l,&RelationFocus::Node("a".into())),"edgeTarget":edge.as_ref().and_then(|e|probe(&l,e)),"nodes":l.graph().nodes().iter().map(|n|json!({"id":n.id,"marks":n.mark_ids})).collect::<Vec<_>>(),"edges":l.graph().edges().iter().map(|e|json!({"id":e.id,"source":e.source,"target":e.target,"marks":e.mark_ids})).collect::<Vec<_>>(),"marks":l.scene().marks.iter().map(|m|json!({"id":m.id,"owner":focus_value(l.graph().focus_for_mark(&m.id)),"opacity":m.opacity,"label":m.label,"value":m.value})).collect::<Vec<_>>(),"authored":l.authored_scene().marks.iter().map(|m|json!({"id":m.id,"opacity":m.opacity,"value":m.value})).collect::<Vec<_>>(),"owned":l.scene().label_series.values().cloned().collect::<Vec<_>>(),"events":f.events.borrow().clone()}).to_string())})
}
#[wasm_bindgen]
pub fn relations_rejected() -> bool {
    RELATIONS.with(|slot| {
        slot.borrow().iter().all(|f| {
            let before = f.layout.borrow().clone();
            let mut candidate = before.clone();
            let missing = candidate
                .set_focus(Some(RelationFocus::Node("missing".into())))
                .is_err();
            let hidden = candidate
                .set_hidden(BTreeSet::from(["missing".into()]))
                .is_err();
            let unchanged = candidate == before;
            let mut bad = before.scene().clone();
            bad.marks[0].opacity = f64::NAN;
            missing && hidden && unchanged && f.chart.set_scene(bad).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn relations_stop() {
    let old = RELATIONS.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in old {
        f.chart.stop();
        f.root.remove();
    }
}
