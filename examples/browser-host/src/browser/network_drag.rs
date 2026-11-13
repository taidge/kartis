//! Actual BrowserNetwork fixtures; all pointer input is driven by the browser.
use kartis::browser::{BrowserNetwork, BrowserNetworkHandle, BrowserNetworkOptions};
use kartis::network::{ForceOptions, ForceSimulation};

use super::*;
struct NetworkEntry {
    owner: Option<BrowserNetwork>,
    handle: BrowserNetworkHandle,
    root: web_sys::Element,
    events: Rc<RefCell<Vec<Value>>>,
    frames: Rc<Cell<u32>>,
}
thread_local! {static NETWORKS:RefCell<BTreeMap<String,NetworkEntry>>=const{RefCell::new(BTreeMap::new())};}
#[wasm_bindgen]
pub fn network_drag_mount() -> std::result::Result<(), JsValue> {
    network_drag_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for id in ["svg", "canvas"] {
        let root = document.create_element("div")?;
        root.set_id(&format!("network-{id}"));
        root.set_attribute("style", "width:600px;max-width:100%;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let nodes = [
            NetworkNode {
                id: "a".into(),
                label: "Drag free A".into(),
                radius: 18.,
            },
            NetworkNode {
                id: "b".into(),
                label: "Drag pinned B".into(),
                radius: 18.,
            },
            NetworkNode {
                id: "c".into(),
                label: "Moving C".into(),
                radius: 14.,
            },
        ];
        let links = [
            NetworkLink {
                source: "a".into(),
                target: "b".into(),
                value: 1.,
            },
            NetworkLink {
                source: "b".into(),
                target: "c".into(),
                value: 1.,
            },
        ];
        let mut simulation = ForceSimulation::new(
            &nodes,
            &links,
            ForceOptions {
                alpha: 0.,
                alpha_decay: 0.25,
                center_strength: 0.,
                collision_strength: 0.1,
                max_speed: 20.,
                ..Default::default()
            },
        )
        .map_err(js)?;
        simulation.set_position("a", [-70., 0.]).map_err(js)?;
        simulation.set_fixed("b", Some([70., 0.])).map_err(js)?;
        simulation.set_position("c", [0., -80.]).map_err(js)?;
        let config = ChartConfig {
            width: 600.,
            height: 360.,
            margin: Margin {
                left: 40.,
                right: 20.,
                top: 20.,
                bottom: 40.,
            },
            title: format!("Network drag {id}"),
            ..Default::default()
        };
        let owner = BrowserNetwork::mount(
            root.clone(),
            simulation,
            config,
            BrowserNetworkOptions {
                auto_start: false,
                chart: BrowserOptions {
                    renderer: if id == "svg" {
                        BrowserRenderer::Svg
                    } else {
                        BrowserRenderer::Canvas
                    },
                    height: Some(360.),
                    legend: false,
                    data_controls: true,
                    tooltip: true,
                    motion: browser::MotionPreference::Reduced,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        owner.on_event(move |event| log.borrow_mut().push(json!({"event":format!("{event:?}")})));
        let frames = Rc::new(Cell::new(0));
        let count = frames.clone();
        owner.on_tick(move |_| count.set(count.get() + 1));
        let handle = owner.handle();
        NETWORKS.with(|entries| {
            entries.borrow_mut().insert(
                id.into(),
                NetworkEntry {
                    owner: Some(owner),
                    handle,
                    root,
                    events,
                    frames,
                },
            )
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn network_drag_snapshot(id: &str) -> String {
    NETWORKS.with(|entries|{let entries=entries.borrow();let Some(entry)=entries.get(id)else{return json!({"active":false}).to_string()};
        match entry.handle.snapshot(){Ok(s)=>json!({"active":s.active,"captured":s.captured,"dragging":s.dragging.map(|d|json!({"node":d.node_id,"pointer":d.pointer_id,"offset":d.grab_offset})),"alpha":s.alpha,"target":s.alpha_target,"ticks":s.ticks,"pending":s.pending,"raf":s.raf_pending,"suspended":s.suspended,"error":s.error.map(|e|e.to_string()),"frames":entry.frames.get(),"events":entry.events.borrow().len(),"nodes":s.nodes.iter().map(|n|json!({"id":n.id,"x":n.position[0],"y":n.position[1],"vx":n.velocity[0],"vy":n.velocity[1],"fixed":n.fixed})).collect::<Vec<_>>(),"chart":s.chart.map(|c|json!({"width":c.width,"height":c.height,"view":{"scale":c.view.scale,"x":c.view.translate_x,"y":c.view.translate_y},"hidden":c.hidden,"selected":c.selected,"hovered":c.hovered,"pending":c.pending,"revision":c.revision}))}).to_string(),Err(error)=>json!({"active":false,"dropped":true,"error":error.to_string()}).to_string()}
    })
}
#[wasm_bindgen]
pub fn network_drag_flush() -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        for entry in entries.borrow().values() {
            if entry.handle.is_active() {
                entry.handle.flush().map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn network_drag_view(id: &str, scale: f64, x: f64, y: f64) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .ok_or_else(|| js("network missing"))?
            .handle
            .set_view(ZoomTransform::new(scale, x, y).map_err(js)?)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_drag_hidden(id: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .ok_or_else(|| js("network missing"))?
            .handle
            .set_hidden(if hidden {
                ["a".into()].into_iter().collect()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_drag_resize(id: &str, width: f64) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .ok_or_else(|| js("network missing"))?
            .root
            .set_attribute(
                "style",
                &format!("width:{width}px;max-width:100%;margin:12px 0"),
            )
    })
}
#[wasm_bindgen]
pub fn network_drag_arm_stop(id: &str) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        let entries = entries.borrow();
        let e = entries.get(id).ok_or_else(|| js("network missing"))?;
        let h = e.handle.clone();
        e.owner
            .as_ref()
            .ok_or_else(|| js("network dropped"))?
            .on_tick(move |_| h.stop());
        Ok(())
    })
}
#[wasm_bindgen]
pub fn network_drag_drop(id: &str) {
    NETWORKS.with(|entries| {
        if let Some(entry) = entries.borrow_mut().get_mut(id) {
            entry.owner.take();
        }
    });
}
#[wasm_bindgen]
pub fn network_drag_stop() {
    let entries = NETWORKS.with(|entries| std::mem::take(&mut *entries.borrow_mut()));
    for (_, entry) in entries {
        drop(entry.owner);
        entry.root.remove();
    }
}
#[wasm_bindgen]
pub fn network_drag_select(id: &str, node: &str) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .ok_or_else(|| js("network missing"))?
            .handle
            .set_selected(Some(node))
            .map_err(js)
    })
}
fn updated_simulation(phase: u8) -> Result<ForceSimulation> {
    if phase == 3 {
        let mut simulation = ForceSimulation::new(
            &[
                NetworkNode {
                    id: "edge".into(),
                    label: "Edge".into(),
                    radius: 18.,
                },
                NetworkNode {
                    id: "near".into(),
                    label: "Near".into(),
                    radius: 18.,
                },
            ],
            &[],
            ForceOptions {
                alpha: 1.,
                repulsion: 1_000_000.,
                theta: 0.,
                center_strength: 0.,
                collision_strength: 0.,
                ..Default::default()
            },
        )?;
        simulation.set_position("edge", [500_000., 0.])?;
        simulation.set_position("near", [499_999., 0.])?;
        return Ok(simulation);
    }

    let values = match phase {
        0 => vec![("b", "Updated B", [-30., 10.]), ("d", "New D", [80., -20.])],
        1 => vec![("d", "Updated D", [35., -15.])],
        _ => vec![("d", "Final D", [30., 5.]), ("e", "Final E", [-60., -30.])],
    };
    let nodes = values
        .iter()
        .map(|(id, label, _)| NetworkNode {
            id: (*id).into(),
            label: (*label).into(),
            radius: 18.,
        })
        .collect::<Vec<_>>();
    let mut simulation = ForceSimulation::new(
        &nodes,
        &[],
        ForceOptions {
            alpha: 0.,
            repulsion: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            ..Default::default()
        },
    )?;
    for (id, _, point) in values {
        simulation.set_position(id, point)?;
    }
    Ok(simulation)
}
#[wasm_bindgen]
pub fn network_drag_replace(id: &str, phase: u8) -> std::result::Result<(), JsValue> {
    let next = updated_simulation(phase).map_err(js)?;
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .ok_or_else(|| js("network missing"))?
            .handle
            .replace_simulation(next)
            .map_err(js)
    })
}
#[wasm_bindgen]
pub fn network_drag_reject() -> bool {
    ForceSimulation::new(
        &[NetworkNode {
            id: "bad".into(),
            label: "bad".into(),
            radius: f64::NAN,
        }],
        &[],
        ForceOptions::default(),
    )
    .is_err()
}
#[wasm_bindgen]
pub fn network_drag_hide_node(
    id: &str,
    node: &str,
    hidden: bool,
) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        let entries = entries.borrow();
        let e = entries.get(id).ok_or_else(|| js("network missing"))?;
        let mut set = e
            .handle
            .snapshot()
            .map_err(js)?
            .chart
            .ok_or_else(|| js("network stopped"))?
            .hidden;
        if hidden {
            set.insert(node.into());
        } else {
            set.remove(node);
        }
        e.handle.set_hidden(set).map_err(js)
    })
}

#[wasm_bindgen]
pub fn network_drag_events(id: &str) -> String {
    NETWORKS.with(|entries| {
        entries
            .borrow()
            .get(id)
            .map_or_else(|| "[]".into(), |e| json!(*e.events.borrow()).to_string())
    })
}

/// Replace and flush from Error while the old frame is unwinding. The nested
/// flush must defer work so the temporarily borrowed callback misses no event.
#[wasm_bindgen]
pub fn network_drag_arm_recover(id: &str) -> std::result::Result<(), JsValue> {
    NETWORKS.with(|entries| {
        let entries = entries.borrow();
        let entry = entries.get(id).ok_or_else(|| js("network missing"))?;
        let handle = entry.handle.clone();
        let log = entry.events.clone();
        let mut attempted = false;
        entry.handle.on_event(move |event| {
            let recover = matches!(event, browser::BrowserEvent::Error(_)) && !attempted;
            log.borrow_mut().push(json!({"event":format!("{event:?}")}));
            if recover {
                attempted = true;
                let result = (|| -> Result<()> {
                    handle.replace_simulation(updated_simulation(2)?)?;
                    handle.set_selected(Some("e"))?;
                    handle.flush()
                })();
                if let Err(error) = result {
                    log.borrow_mut()
                        .push(json!({"recoveryError":error.to_string()}));
                }
            }
        });
        Ok(())
    })
}
