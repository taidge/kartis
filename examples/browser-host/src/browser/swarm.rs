use super::*;

struct SwarmFixture {
    root: web_sys::Element,
    chart: BrowserChart,
    layout: SwarmLayout,
}

thread_local! {
    static SWARMS: RefCell<BTreeMap<&'static str, SwarmFixture>> = const { RefCell::new(BTreeMap::new()) };
}

fn layout(phase: bool) -> Result<SwarmLayout> {
    let value = if phase { 12. } else { 8. };
    let mut target = SwarmPoint::new("target", "Trial", value);
    target.label = Some("Target <safe>".into());
    swarm_layout(
        &ChartConfig {
            width: 640.,
            height: 360.,
            ..Default::default()
        },
        &[
            target,
            SwarmPoint::new("repeat", "Trial", value),
            SwarmPoint::new("low", "Trial", 2.),
            SwarmPoint::new("other", "Control", 24.),
        ],
        &SwarmLayoutOptions {
            orientation: SwarmOrientation::Horizontal,
            groups: Some(
                ["Trial", "Control"]
                    .into_iter()
                    .map(SwarmGroup::new)
                    .collect(),
            ),
            value_scale: ContinuousScaleOptions {
                kind: ScaleKind::Log,
                reverse: phase,
                log_base: 2.,
                ..Default::default()
            },
            value_domain: Some([1., 64.]),
            value_format: ValueFormat::Fixed(1),
            spacing: 5.,
            node: SwarmNodeStyle {
                shape: SwarmNodeShape::StrokedGlyph {
                    shape: StrokedGlyph::Diamond,
                    size: 400.,
                    rotation: 0.35,
                },
                stroke: Some(GraphicStroke {
                    color: Color(26, 86, 149, 255),
                    width: 3.,
                    cap: StrokeCap::Round,
                }),
                ..Default::default()
            },
            ..Default::default()
        },
    )
}

#[wasm_bindgen]
pub fn swarm_mount() -> std::result::Result<(), JsValue> {
    swarm_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let layout = layout(false).map_err(js)?;
        let root = document.create_element("div")?;
        root.set_id(&format!("swarm-{name}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let chart = BrowserChart::mount(
            root.clone(),
            layout.scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                highlight: browser::BrowserHighlight::Controlled,
                interaction_data: Some(layout.interaction_binding()),
                interaction: kartis::surface::SurfaceOptions {
                    plot: Some(layout.plot),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .map_err(js)?;
        SWARMS.with(|fixtures| {
            fixtures.borrow_mut().insert(
                name,
                SwarmFixture {
                    root,
                    chart,
                    layout,
                },
            )
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn swarm_phase() -> std::result::Result<(), JsValue> {
    SWARMS.with(|fixtures| {
        for fixture in fixtures.borrow_mut().values_mut() {
            let next = layout(true).map_err(js)?;
            fixture.chart.set_layout(next.clone().into()).map_err(js)?;
            fixture.chart.flush().map_err(js)?;
            fixture.layout = next;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn swarm_flush() -> std::result::Result<(), JsValue> {
    SWARMS.with(|fixtures| {
        for fixture in fixtures.borrow().values() {
            fixture.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn swarm_bad_layout() -> bool {
    SWARMS.with(|fixtures| {
        fixtures.borrow().values().all(|fixture| {
            let mut bad: SceneInteraction = fixture.layout.clone().into();
            bad.interaction.as_mut().unwrap().sites[0].position[0] += 1.;
            fixture.chart.set_layout(bad).is_err()
        })
    })
}

#[wasm_bindgen]
pub fn swarm_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    SWARMS.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let fixture = fixtures.get(renderer).ok_or_else(|| js("Swarm fixture is unmounted"))?;
        let snapshot = fixture.chart.snapshot().map_err(js)?;
        let target = fixture.layout.sites.iter().find(|site| site.node_id == "target").unwrap();
        let sibling = fixture.layout.sites.iter().find(|site| site.node_id == "repeat").unwrap();
        let Geometry::Path(path) = &fixture.layout.scene.marks[target.mark_index].geometry else { unreachable!() };
        let PathCommand::Move(edge) = path[0] else { unreachable!() };
        let active = snapshot.interaction.as_ref().and_then(|state| state.rows.first()).map(|row| json!({
            "value": row.value, "formatted": row.formatted_value, "position": row.position,
        }));
        Ok(json!({
            "width": snapshot.width, "height": snapshot.height, "revision": snapshot.revision,
            "hovered": snapshot.hovered, "selected": snapshot.selected, "hidden": snapshot.hidden,
            "active": active, "id": target.mark_id, "center": target.position,
            "sibling": sibling.position, "edge": [edge.x, edge.y], "inkRadius": target.ink_radius,
            "domain": fixture.layout.value_scale.display_domain(), "range": fixture.layout.value_scale.range(),
        }).to_string())
    })
}

#[wasm_bindgen]
pub fn swarm_stop() {
    for fixture in SWARMS
        .with(|fixtures| std::mem::take(&mut *fixtures.borrow_mut()))
        .into_values()
    {
        fixture.chart.stop();
        fixture.root.remove();
    }
}
