//! Independent data-space focus controllers. Input requests a subtree layout
//! with raw interaction data and lets the retained host animate the focus.
use super::*;
struct HierarchyFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    view: HierarchyView,
    controller: HierarchyFocusController,
    layout: HierarchyLayout,
    pending: Rc<RefCell<Option<String>>>,
    events: Rc<RefCell<Vec<Value>>>,
    phase: u8,
}
thread_local! {static HIERARCHIES:RefCell<Vec<HierarchyFixture>>=const{RefCell::new(Vec::new())};}
fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 24.,
            right: 24.,
            top: 24.,
            bottom: 24.,
        },
        ..Default::default()
    }
}
fn data(phase: u8) -> Vec<HierarchyDatum> {
    let mut result = vec![
        HierarchyDatum::new("root", None, 0.),
        HierarchyDatum::new("group-a", Some("root".into()), 0.),
        HierarchyDatum::new("group-b", Some("root".into()), 0.),
    ];
    for (id, parent, value, label) in [
        (
            "leaf-a",
            "group-a",
            if phase == 0 { 2. } else { 9. },
            "<Leaf A>",
        ),
        ("leaf-b", "group-a", 3., "Leaf B"),
        ("leaf-c", "group-b", 4., "Leaf C"),
        ("leaf-d", "group-b", 5., "Leaf D"),
    ] {
        if phase == 2 && id == "leaf-a" {
            continue;
        }
        let mut node = HierarchyDatum::new(id, Some(parent.into()), value);
        node.label = label.into();
        result.push(node);
    }
    result
}
fn view(case: u8) -> Result<HierarchyView> {
    let border = Some(GraphicStroke {
        color: Color(255, 255, 255, 255),
        width: 1.,
        cap: StrokeCap::Butt,
    });
    let mut style = HierarchyStyleOptions::default();
    style.leaf.border = border;
    style.parent.border = border;
    style.parent.fill = HierarchyFill::Fixed(Color(220, 228, 238, 255));
    style.parent_labels.enabled = true;
    style.parent_labels.size = Some(13.);
    style.leaf_labels.size = Some(13.);
    style.leaf_labels.skip_width = 38.;
    style.leaf_labels.skip_height = 20.;
    style.leaf_labels.skip_radius = 14.;
    style.value_format = ValueFormat::Fixed(1);
    match case {
        0 => Ok(HierarchyView::Treemap(TreemapLayoutOptions {
            tile: HierarchyTreemapTile::Resquarify,
            inner_padding: 5.,
            outer_padding: 5.,
            parent_label_size: 24.,
            style,
            ..Default::default()
        })),
        1 => {
            style.parent.opacity = 0.18;
            style.parent_labels.enabled = false;
            style.leaf_labels.skip_width = 0.;
            style.leaf_labels.skip_height = 0.;
            Ok(HierarchyView::CirclePacking(CirclePackingLayoutOptions {
                padding: 3.,
                viewport_padding: 5.,
                style,
                ..Default::default()
            }))
        }
        2 | 3 => Ok(HierarchyView::Icicle(IcicleLayoutOptions {
            direction: if case == 2 {
                IcicleDirection::Right
            } else {
                IcicleDirection::Left
            },
            padding: 4.,
            style,
        })),
        _ => Err(ChartError("hierarchy fixture case must be 0..3".into())),
    }
}
fn commit(
    f: &mut HierarchyFixture,
    controller: HierarchyFocusController,
    layout: HierarchyLayout,
    animate: bool,
) -> Result<()> {
    let binding = layout.interaction_binding()?;
    if animate {
        f.chart
            .animate_to_with_interaction(layout.scene.clone(), binding)?;
    } else {
        f.chart
            .set_scene_with_interaction(layout.scene.clone(), binding)?;
    }
    f.chart.flush()?;
    f.controller = controller;
    f.layout = layout;
    Ok(())
}
fn activate(f: &mut HierarchyFixture, id: &str) -> Result<()> {
    let raw = f
        .layout
        .node_for_mark(id)
        .ok_or_else(|| ChartError("unknown hierarchy activation".into()))?
        .id
        .clone();
    let mut candidate = f.controller.clone();
    let layout = candidate.focus(&config(), &raw, &f.view)?;
    commit(f, candidate, layout, true)
}
#[wasm_bindgen]
pub fn hierarchy_mount(case: u8) -> std::result::Result<(), JsValue> {
    let view = view(case).map_err(js)?;
    let mut controller = HierarchyFocusController::new(&data(0)).map_err(js)?;
    let layout = controller.layout(&config(), &view).map_err(js)?;
    let binding = layout.interaction_binding().map_err(js)?;
    hierarchy_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("hierarchy-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                legend: false,
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(binding.clone()),
                interaction: surface::SurfaceOptions {
                    plot: Some(layout.plot()),
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(c) => c,
            Err(e) => {
                root.remove();
                hierarchy_stop();
                return Err(js(e));
            }
        };
        let events = Rc::new(RefCell::new(vec![]));
        let capture = events.clone();
        let pending = Rc::new(RefCell::new(None));
        let request = pending.clone();
        chart.on_event(move |event| {
            if let BrowserEvent::Interaction(SurfaceEvent::Activated(id)) = &event {
                *request.borrow_mut() = Some(id.clone());
            }
            let mut events = capture.borrow_mut();
            if events.len() == 64 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        if let Err(e) = chart.flush() {
            chart.stop();
            root.remove();
            hierarchy_stop();
            return Err(js(e));
        }
        HIERARCHIES.with(|slot| {
            slot.borrow_mut().push(HierarchyFixture {
                chart,
                root,
                renderer: name.into(),
                view: view.clone(),
                controller: controller.clone(),
                layout: layout.clone(),
                pending,
                events,
                phase: 0,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn hierarchy_flush() -> std::result::Result<(), JsValue> {
    HIERARCHIES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.flush().map_err(js)?;
            let pending = f.pending.borrow_mut().take();
            if let Some(id) = pending {
                activate(f, &id).map_err(js)?;
            }
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn hierarchy_action(renderer: &str, action: &str) -> std::result::Result<(), JsValue> {
    HIERARCHIES.with(|slot| {
        let mut entries = slot.borrow_mut();
        let f = entries
            .iter_mut()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing hierarchy renderer"))?;
        let mut candidate = f.controller.clone();
        let layout = match action {
            "back" => candidate.back(&config(), &f.view),
            "parent" => candidate.parent(&config(), &f.view),
            "root" => candidate.root(&config(), &f.view),
            "same" => candidate.layout(&config(), &f.view),
            id => candidate.focus(&config(), id, &f.view),
        }
        .map_err(js)?;
        commit(f, candidate, layout, action != "same").map_err(js)
    })
}
#[wasm_bindgen]
pub fn hierarchy_update(phase: u8) -> std::result::Result<(), JsValue> {
    if phase > 2 {
        return Err(js("hierarchy phase must be 0..2"));
    }
    HIERARCHIES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            let mut candidate = f.controller.clone();
            let layout = candidate
                .replace(&config(), &data(phase), &f.view)
                .map_err(js)?;
            commit(f, candidate, layout, false).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn hierarchy_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    HIERARCHIES.with(|slot| {
        let entries = slot.borrow();
        let f = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing hierarchy renderer"))?;
        f.chart
            .handle()
            .set_hidden(if hidden {
                ["leaf-a".into()].into_iter().collect()
            } else {
                Default::default()
            })
            .map_err(js)
    })
}
fn target(f: &HierarchyFixture) -> Value {
    let preferred = if f.controller.current() == Some("root") {
        "group-a"
    } else {
        "leaf-a"
    };
    let candidates = std::iter::once(preferred).chain(
        f.layout
            .nodes
            .iter()
            .filter(|n| n.visible)
            .map(|n| n.id.as_str()),
    );
    for id in candidates {
        let Some(node) = f.layout.node(id).filter(|n| n.visible) else {
            continue;
        };
        let Some(mark) = f
            .layout
            .scene
            .marks
            .iter()
            .find(|m| m.id == node.mark_id && m.has_fill() && m.opacity > 0.)
        else {
            continue;
        };
        let g = &mark.geometry;
        let (x, y, w, h) = match *g {
            Geometry::Rect {
                x,
                y,
                width,
                height,
            } => (x, y, width, height),
            Geometry::Circle { x, y, radius } => (x - radius, y - radius, 2. * radius, 2. * radius),
            _ => continue,
        };
        for row in 0..21 {
            for col in 0..21 {
                let px = x + w * (col as f64 + 0.5) / 21.;
                let py = y + h * (row as f64 + 0.5) / 21.;
                // Probe real filled ink, away from the source stroke/slop edge.
                // Trusted browser coordinates can quantize fractionally; the
                // full neighborhood must retain the same current paint owner.
                let inset = match *g {
                    Geometry::Rect {
                        x,
                        y,
                        width,
                        height,
                    } => [px - x, x + width - px, py - y, y + height - py]
                        .into_iter()
                        .fold(f64::INFINITY, f64::min),
                    Geometry::Circle { x, y, radius } => radius - (px - x).hypot(py - y),
                    _ => unreachable!(),
                };
                if inset < 1.5 {
                    continue;
                }
                let probes = [-1.5, 0., 1.5]
                    .into_iter()
                    .flat_map(|dx| {
                        [-1.5, 0., 1.5]
                            .into_iter()
                            .map(move |dy| (px + dx, py + dy))
                    })
                    .map(|(x, y)| (x, y, f.layout.scene.hit_test(x, y).map(|m| m.id.as_str())))
                    .collect::<Vec<_>>();
                if probes
                    .iter()
                    .all(|(_, _, id)| *id == Some(node.mark_id.as_str()))
                {
                    return json!({"id":node.mark_id,"raw":node.id,"label":node.label,"value":node.value,"tooltip":mark.label,"x":px,"y":py,"fillInset":inset,"probeMargin":1.5,"bounds":[x,y,w,h],"probeHits":probes.iter().map(|(x,y,id)|json!({"x":x,"y":y,"id":id})).collect::<Vec<_>>()});
                }
            }
        }
    }
    Value::Null
}
#[wasm_bindgen]
pub fn hierarchy_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    HIERARCHIES.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f| f.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let s = f.chart.snapshot().map_err(js)?;
        let nodes = f.layout.nodes.iter().map(|n| json!({
            "id":n.id,"parent":n.parent,"depth":n.relative_depth,"visible":n.visible,
            "value":n.value,"mark":n.mark_id,"labels":n.label_slots
        })).collect::<Vec<_>>();
        let interaction = s.interaction.map(|active| json!({
            "anchor":active.anchor,
            "rows":active.rows.iter().map(|site| json!({
                "id":site.mark_id,"label":site.label,"value":site.value,
                "formattedValue":site.formatted_value,"position":site.position
            })).collect::<Vec<_>>()
        }));
        Ok(json!({
            "active":s.active,"width":s.width,"height":s.height,"pending":s.pending,
            "revision":s.revision,"controllerRevision":f.controller.revision(),
            "focus":f.controller.current(),"ancestors":f.controller.ancestors(),"history":f.controller.history(),
            "phase":f.phase,"view":{"scale":s.view.scale,"x":s.view.translate_x,"y":s.view.translate_y},
            "animating":s.animating,"propertiesMotion":s.animation_kind==Some(browser::BrowserAnimationKind::Properties),
            "hovered":s.hovered,"selected":s.selected,"hidden":s.hidden,"interaction":interaction,
            "target":target(f),"nodes":nodes,"events":f.events.borrow().clone()
        }).to_string())
    })
}
#[wasm_bindgen]
pub fn hierarchy_rejected() -> bool {
    HIERARCHIES.with(|slot| {
        slot.borrow().iter().all(|f| {
            let mut candidate = f.controller.clone();
            let mut invalid = f.view.clone();
            let style = match &mut invalid {
                HierarchyView::Treemap(o) => &mut o.style,
                HierarchyView::CirclePacking(o) => &mut o.style,
                HierarchyView::Icicle(o) => &mut o.style,
            };
            style.leaf.opacity = f64::NAN;
            let refused = candidate.root(&config(), &invalid).is_err()
                && candidate.current() == f.controller.current()
                && candidate.revision() == f.controller.revision();
            let mut scene = f.layout.scene.clone();
            scene.marks[0].opacity = f64::NAN;
            refused
                && f.layout.interaction_binding().is_ok_and(|binding| {
                    f.chart.animate_to_with_interaction(scene, binding).is_err()
                })
        })
    })
}
#[wasm_bindgen]
pub fn hierarchy_stop() {
    let entries = HIERARCHIES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
