use super::*;

struct Fixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: &'static str,
    target: Value,
    events: Rc<RefCell<Vec<Value>>>,
}

thread_local! {
    static FIXTURES: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) };
}

fn source(kind: &str, phase: u8) -> Result<(SceneInteraction, Value)> {
    if phase > 1 {
        return Err(ChartError("layout model phase must be 0 or 1".into()));
    }
    let c = ChartConfig {
        width: 640.,
        height: 360.,
        ..Default::default()
    };
    let delta = f64::from(phase) * 8.;
    let color = |Color(r, g, b, a)| [r, g, b, a];
    match kind {
        "funnel" => {
            let layout = funnel_layout(
                &c,
                &[
                    FunnelDatum::new("Reached", 100.),
                    FunnelDatum::new("Qualified", 60. + delta),
                    FunnelDatum::new("Converted", 24.),
                ],
                &FunnelLayoutOptions {
                    direction: FunnelDirection::Horizontal,
                    spacing: 12.,
                    current: Some("Qualified".into()),
                    current_size_extension: 8.,
                    ..Default::default()
                },
            )?;
            let part = &layout.parts[1];
            let target = json!({"kind":kind,"id":part.mark_id,"label":part.datum.label,"value":part.datum.value,
                "point":[part.center[0],part.center[1]-23.],"color":color(part.color),
                "metadata":{"separators":layout.separators.len(),"current":part.is_current,"sourceIndex":part.source_index}});
            Ok((layout.into_scene_interaction()?, target))
        }
        "waffle-cells" | "waffle-areas" => {
            let areas = kind == "waffle-areas";
            let layout = waffle_layout(
                &c,
                &[
                    RadialDatum::new("Mobile", 12. + delta),
                    RadialDatum::new("Desktop", 8.),
                ],
                &WaffleLayoutOptions {
                    rows: 4,
                    columns: 8,
                    total: Some(32.),
                    direction: WaffleFillDirection::Right,
                    mode: if areas {
                        WaffleMode::Areas
                    } else {
                        WaffleMode::Cells
                    },
                    padding: 9.,
                    corner_radius: 7.,
                    ..Default::default()
                },
            )?;
            let group = &layout.groups[0];
            let cell = &layout.cells[group.occupied.start];
            let bounds = cell.bounds;
            let point = [bounds.x + bounds.width / 2., bounds.y + bounds.height / 2.];
            let id = if areas {
                &group.area_mark_id
            } else {
                &cell.mark_id
            };
            let target = json!({"kind":kind,"id":id,"label":group.source.label,"value":group.source.value,
                "point":point,"color":color(group.color),"gap":[bounds.x+2.,point[1]],
                "metadata":{"cells":layout.cells.len(),"occupied":group.occupied.len(),"polygons":group.polygons.len()}});
            Ok((layout.into_scene_interaction(), target))
        }
        "sunburst" => {
            let layout = sunburst_layout(
                &c,
                &[
                    HierarchyDatum::new("All", None, 0.),
                    HierarchyDatum::new("Cloud", Some("All".into()), 3.),
                    HierarchyDatum::new("Devices", Some("All".into()), 25.),
                    HierarchyDatum::new("Compute", Some("Cloud".into()), 16. + delta),
                    HierarchyDatum::new("Storage", Some("Cloud".into()), 11.),
                ],
                &SunburstLayoutOptions {
                    corner_radius: 5.,
                    pad_angle: 0.025,
                    ..Default::default()
                },
            )?;
            let node = layout.node("Compute").unwrap();
            let target = json!({"kind":kind,"id":node.mark_id,"label":node.data.id,"value":node.value,
                "point":node.anchor,"color":color(node.color),
                "metadata":{"own":node.data.value,"parentTotal":layout.node("Cloud").unwrap().value,
                    "formatted":node.formatted_value,"angles":node.arc.angles}});
            Ok((layout.into_scene_interaction()?, target))
        }
        _ => super::flow_layouts::source(kind, phase, &c),
    }
}

#[wasm_bindgen]
pub fn layout_models_mount(kind: &str) -> std::result::Result<(), JsValue> {
    let (candidate, target) = source(kind, 0).map_err(js)?;
    layout_models_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("layout-model-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            candidate.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    plot: candidate.plot,
                    picking: match target["pick"].as_str() {
                        Some("slice-x") => surface::Picking::SliceX { distance: 80. },
                        Some("nearest") => surface::Picking::Nearest { distance: 18. },
                        _ => surface::Picking::Geometry,
                    },
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                interaction_data: candidate.interaction.clone(),
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                layout_models_stop();
                return Err(js(error));
            }
        };
        let events = Rc::new(RefCell::new(Vec::new()));
        let capture = events.clone();
        chart.on_event(move |event| {
            let mut events = capture.borrow_mut();
            if events.len() == 32 {
                events.remove(0);
            }
            events.push(event_value(event));
        });
        if let Err(error) = chart.flush() {
            chart.stop();
            root.remove();
            layout_models_stop();
            return Err(js(error));
        }
        FIXTURES.with(|slot| {
            slot.borrow_mut().push(Fixture {
                chart,
                root,
                renderer: name,
                target: target.clone(),
                events,
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn layout_models_phase(kind: &str, phase: u8) -> std::result::Result<(), JsValue> {
    let (candidate, target) = source(kind, phase).map_err(js)?;
    FIXTURES.with(|slot| {
        for fixture in slot.borrow_mut().iter_mut() {
            fixture.chart.set_layout(candidate.clone()).map_err(js)?;
            fixture.target = target.clone();
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn layout_models_flush() -> std::result::Result<(), JsValue> {
    FIXTURES.with(|slot| {
        for fixture in slot.borrow().iter() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn layout_models_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    FIXTURES.with(|slot| {
        let entries = slot.borrow();
        let Some(fixture) = entries.iter().find(|fixture| fixture.renderer == renderer) else {
            return Ok(json!({"active":false}).to_string());
        };
        let state = fixture.chart.snapshot().map_err(js)?;
        let interaction = state.interaction.as_ref().map(|active| {
            json!({"ids":active.ids,"anchor":active.anchor,"rows":active.rows.iter().map(|row| {
                json!({"id":row.mark_id,"series":row.series,"value":row.value,"formatted":row.formatted_value,"label":row.label})
            }).collect::<Vec<_>>()})
        });
        Ok(
            json!({"active":state.active,"revision":state.revision,"hovered":state.hovered,
            "selected":state.selected,"target":fixture.target,"interaction":interaction,"events":*fixture.events.borrow()})
            .to_string(),
        )
    })
}

#[wasm_bindgen]
pub fn layout_models_rejected(kind: &str, phase: u8) -> bool {
    let Ok((candidate, _)) = source(kind, phase) else {
        return false;
    };
    FIXTURES.with(|slot| {
        slot.borrow().iter().all(|fixture| {
            let mut invalid = candidate.clone();
            let Some(first) = invalid
                .interaction
                .as_mut()
                .and_then(|data| data.sites.first_mut())
            else {
                return false;
            };
            first.position[0] += 7.;
            fixture.chart.set_layout(invalid).is_err()
        })
    })
}

#[wasm_bindgen]
pub fn layout_models_stop() {
    for fixture in FIXTURES.with(|slot| std::mem::take(&mut *slot.borrow_mut())) {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
