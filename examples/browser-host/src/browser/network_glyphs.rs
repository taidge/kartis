//! The retained force driver owns the rendered glyph, datum and drag target.
use std::collections::BTreeSet;

use kartis::browser::{BrowserNetwork, BrowserNetworkOptions};

use super::*;

struct Fixture {
    network: BrowserNetwork,
    root: web_sys::Element,
    renderer: &'static str,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! { static NETWORK_GLYPHS: RefCell<Vec<Fixture>> = const { RefCell::new(Vec::new()) }; }

fn config() -> ChartConfig {
    ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 55.,
            right: 31.,
            top: 29.,
            bottom: 39.,
        },
        title: "Physical network glyph drag".into(),
        ..Default::default()
    }
}

fn source(phase: u8) -> Result<(ForceSimulation, NetworkGlyphOptions)> {
    if phase > 1 {
        return Err(ChartError("invalid network glyph phase".into()));
    }
    let mut simulation = ForceSimulation::new(
        &[
            NetworkNode {
                id: "a:wing".into(),
                label: if phase == 0 {
                    "Star <safe>"
                } else {
                    "Wye updated <safe>"
                }
                .into(),
                radius: 3.,
            },
            NetworkNode {
                id: "b:hollow".into(),
                label: "Hollow border".into(),
                radius: 6.,
            },
            NetworkNode {
                id: "untouched".into(),
                label: "Ordinary circle".into(),
                radius: 10.,
            },
        ],
        &[NetworkLink::new("a:wing", "b:hollow", 4.)],
        ForceOptions {
            repulsion: 0.,
            link_strength: 0.,
            center_strength: 0.,
            collision_strength: 0.,
            alpha: 0.,
            alpha_decay: 0.5,
            labels: true,
            ..Default::default()
        },
    )?;
    simulation.set_fixed(
        "a:wing",
        Some(if phase == 0 { [-70., -15.] } else { [-57., 4.] }),
    )?;
    simulation.set_position("b:hollow", [68., -10.])?;
    simulation.set_position("untouched", [4., 74.])?;
    let glyphs = NetworkGlyphOptions {
        nodes: BTreeMap::from([
            (
                "a:wing".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Filled(if phase == 0 {
                            FilledGlyph::Star
                        } else {
                            FilledGlyph::Wye
                        }),
                        size: 1600.,
                        rotation: if phase == 0 { 0. } else { 0.36 },
                    },
                    fill: Some(Color(45, 103, 184, 255)),
                    stroke: None,
                },
            ),
            (
                "b:hollow".into(),
                NetworkNodeGlyph {
                    glyph: GlyphOptions {
                        shape: GlyphShape::Stroked(if phase == 0 {
                            StrokedGlyph::Triangle
                        } else {
                            StrokedGlyph::Square
                        }),
                        size: 1600.,
                        rotation: 0.31,
                    },
                    fill: None,
                    stroke: Some(GraphicStroke {
                        color: Color(175, 58, 87, 255),
                        width: 5.,
                        cap: StrokeCap::Round,
                    }),
                },
            ),
        ]),
    };
    Ok((simulation, glyphs))
}

#[wasm_bindgen]
pub fn network_glyphs_mount() -> std::result::Result<(), JsValue> {
    network_glyphs_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (renderer, backend) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("network-glyphs-{renderer}"));
        root.set_attribute("style", "width:640px;max-width:none;margin:12px 0")?;
        document.body().unwrap().append_child(&root)?;
        let (simulation, glyphs) = source(0).map_err(js)?;
        let network = BrowserNetwork::mount_with_glyphs(
            root.clone(),
            simulation,
            config(),
            BrowserNetworkOptions {
                auto_start: false,
                chart: BrowserOptions {
                    renderer: backend,
                    height: Some(360.),
                    motion: browser::MotionPreference::Reduced,
                    tooltip_options: TooltipOptions {
                        lines: vec![vec![TooltipPart::Label]],
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            },
            glyphs,
        )
        .map_err(js)?;
        let events = Rc::new(RefCell::new(vec![]));
        let log = events.clone();
        network.on_event(move |event| {
            let item = match event {
                BrowserEvent::Interaction(SurfaceEvent::Activated(id)) => {
                    json!({"kind":"activate","id":id})
                }
                BrowserEvent::Error(error) => json!({"kind":"error","message":error.0}),
                _ => return,
            };
            let mut log = log.borrow_mut();
            if log.len() == 64 {
                log.remove(0);
            }
            log.push(item);
        });
        network.flush().map_err(js)?;
        NETWORK_GLYPHS.with(|fixtures| {
            fixtures.borrow_mut().push(Fixture {
                network,
                root,
                renderer,
                events,
            })
        });
    }
    Ok(())
}

#[wasm_bindgen]
pub fn network_glyphs_phase(phase: u8) -> std::result::Result<(), JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        for fixture in fixtures.borrow().iter() {
            let (simulation, glyphs) = source(phase).map_err(js)?;
            fixture
                .network
                .handle()
                .replace_simulation_with_glyphs(simulation, glyphs)
                .map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn network_glyphs_flush() -> std::result::Result<(), JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        for fixture in fixtures.borrow().iter() {
            fixture.network.flush().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn network_glyphs_reset_view() -> std::result::Result<(), JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        for fixture in fixtures.borrow().iter() {
            fixture.network.handle().reset_view().map_err(js)?;
        }
        Ok(())
    })
}

#[wasm_bindgen]
pub fn network_glyphs_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let fixture = fixtures
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing network glyph fixture"))?;
        let hidden = if hidden {
            BTreeSet::from(["a:wing".into()])
        } else {
            BTreeSet::new()
        };
        fixture.network.handle().set_hidden(hidden).map_err(js)
    })
}

#[wasm_bindgen]
pub fn network_glyphs_rejected(renderer: &str) -> std::result::Result<bool, JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let fixture = fixtures
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing network glyph fixture"))?;
        let (simulation, mut glyphs) = source(1).map_err(js)?;
        glyphs
            .nodes
            .insert("unknown".into(), NetworkNodeGlyph::default());
        Ok(fixture
            .network
            .handle()
            .replace_simulation_with_glyphs(simulation, glyphs)
            .is_err())
    })
}

#[wasm_bindgen]
pub fn network_glyphs_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    NETWORK_GLYPHS.with(|fixtures| {
        let fixtures = fixtures.borrow();
        let Some(fixture) = fixtures.iter().find(|f| f.renderer == renderer) else { return Ok(json!({"active":false}).to_string()); };
        let snapshot = fixture.network.snapshot().map_err(js)?;
        let Some(chart) = snapshot.chart else { return Ok(json!({"active":false}).to_string()); };
        let c = config();
        let offset = [(chart.width + c.margin.left - c.margin.right) / 2., (chart.height + c.margin.top - c.margin.bottom) / 2.];
        let nodes = snapshot.nodes.iter().map(|node| json!({
            "id":node.id,"label":node.label,"world":node.position,"fixed":node.fixed,"radius":node.radius,
            "anchor":[node.position[0] + offset[0],node.position[1] + offset[1]],
        })).collect::<Vec<_>>();
        let interaction = chart.interaction.map(|active| json!({
            "anchor":active.anchor,"ids":active.ids,
            "rows":active.rows.iter().map(|row|json!({"label":row.label,"value":row.value,"position":row.position})).collect::<Vec<_>>(),
        }));
        let dragging = snapshot.dragging.map(|session| json!({"node":session.node_id,"grab":session.grab_offset,"last":session.last_client}));
        Ok(json!({
            "active":snapshot.active,"nodes":nodes,"offset":offset,"width":chart.width,"height":chart.height,
            "view":[chart.view.scale,chart.view.translate_x,chart.view.translate_y],"interaction":interaction,
            "hovered":chart.hovered,"hidden":chart.hidden,"selected":chart.selected,"pending":snapshot.pending,
            "ticks":snapshot.ticks,"revision":chart.revision,"dragging":dragging,"capture":snapshot.captured,
            "error":snapshot.error.map(|error|error.0),"events":fixture.events.borrow().clone(),
        }).to_string())
    })
}

#[wasm_bindgen]
pub fn network_glyphs_stop() {
    for fixture in NETWORK_GLYPHS.with(|fixtures| std::mem::take(&mut *fixtures.borrow_mut())) {
        fixture.network.stop();
        fixture.root.remove();
    }
}
