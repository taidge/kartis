//! Real configured Tree scenes, with independent SVG and Canvas controllers.
use super::*;
struct TreeFixture {
    chart: BrowserChart,
    root: web_sys::Element,
    renderer: String,
    phase: u8,
    events: Rc<RefCell<Vec<Value>>>,
}
thread_local! {static TREES:RefCell<Vec<TreeFixture>>=const{RefCell::new(Vec::new())};}
fn source(phase: u8, invalid: bool) -> Result<Scene> {
    if phase > 2 {
        return Err(ChartError("tree fixture phase must be 0..2".into()));
    }
    let config = ChartConfig {
        width: 640.,
        height: 360.,
        margin: Margin {
            left: 70.,
            right: 70.,
            top: 45.,
            bottom: 45.,
        },
        title: "Configured tree".into(),
        ..Default::default()
    };
    let mut data = vec![HierarchyDatum::new("root", None, 0.)];
    for group in ["group-a", "group-b"] {
        data.push(HierarchyDatum::new(group, Some("root".into()), 0.));
    }
    for (id, parent, value, label) in [
        ("leaf-a", "group-a", 10., "<Leaf A>"),
        ("leaf-b", "group-a", 20., "Leaf B"),
        ("leaf-c", "group-b", 30., "Leaf C"),
        ("leaf-d", "group-b", 40., "Leaf D"),
    ] {
        let mut d = HierarchyDatum::new(id, Some(parent.into()), value);
        d.label = label.into();
        data.push(d);
    }
    let mut options = TreeOptions {
        orientation: if phase == 2 {
            TreeOrientation::Radial
        } else {
            TreeOrientation::LeftToRight
        },
        node_radius: 10.,
        rotation: 0.35,
        ..Default::default()
    };
    options.style.clip_plot = true;
    options.style.links.curve = match phase {
        0 => TreeLinkCurve::StepBefore,
        1 => TreeLinkCurve::Bump,
        _ => TreeLinkCurve::Step,
    };
    options.style.links.width = if phase == 1 { 4. } else { 3. };
    options.style.links.color = TreeLinkColor::Fixed(if phase == 1 {
        Color(165, 65, 135, 255)
    } else {
        Color(50, 80, 115, 255)
    });
    if phase != 1 {
        options.style.links.dash = Some(StrokeDash {
            segments: vec![10., 7.],
            offset: 2.,
        });
    }
    options.style.leaves = Some(TreeNodeStyle {
        shape: if phase == 1 {
            TreeNodeShape::Circle
        } else {
            TreeNodeShape::Square
        },
        radius: Some(if invalid { f64::NAN } else { 10. }),
        fill: TreeNodeFill::Fixed(if phase == 1 {
            Color(40, 115, 190, 255)
        } else {
            Color(40, 155, 100, 255)
        }),
        border: Some(GraphicStroke {
            color: Color(15, 45, 65, 255),
            width: 2.,
            cap: StrokeCap::Round,
        }),
        ..Default::default()
    });
    options.style.internal = Some(TreeNodeStyle {
        shape: if phase == 1 {
            TreeNodeShape::Square
        } else {
            TreeNodeShape::Diamond
        },
        radius: Some(13.),
        fill: TreeNodeFill::Fixed(Color(225, 140, 45, 255)),
        border: Some(GraphicStroke {
            color: Color(105, 55, 15, 255),
            width: 2.,
            cap: StrokeCap::Round,
        }),
        ..Default::default()
    });
    options.style.labels.visibility = if phase == 0 {
        TreeLabels::Leaves
    } else {
        TreeLabels::All
    };
    options.style.labels.position = match phase {
        0 => TreeLabelPosition::Left,
        1 => TreeLabelPosition::Below,
        _ => TreeLabelPosition::Outward,
    };
    options.style.labels.rotation = if phase == 2 {
        TreeLabelRotation::AlongLayout
    } else {
        TreeLabelRotation::Fixed(if phase == 1 { -12. } else { 0. })
    };
    options.style.labels.offset = 8.;
    options.style.labels.size = Some(14.);
    tree_with_options(&config, &data, &options)
}
#[wasm_bindgen]
pub fn tree_mount() -> std::result::Result<(), JsValue> {
    let scene = source(0, false).map_err(js)?;
    tree_stop();
    let document = web_sys::window().unwrap().document().unwrap();
    for (name, renderer) in [
        ("svg", BrowserRenderer::Svg),
        ("canvas", BrowserRenderer::Canvas),
    ] {
        let root = document.create_element("div")?;
        root.set_id(&format!("tree-{name}"));
        root.set_attribute(
            "style",
            "width:640px;max-width:none;position:relative;margin:12px 0",
        )?;
        document.body().unwrap().append_child(&root)?;
        let chart = match BrowserChart::mount(
            root.clone(),
            scene.clone(),
            BrowserOptions {
                renderer,
                height: Some(360.),
                legend: false,
                motion: browser::MotionPreference::Reduced,
                interaction: surface::SurfaceOptions {
                    zoom: false,
                    pan: false,
                    brush: false,
                    pinch: false,
                    ..Default::default()
                },
                ..Default::default()
            },
        ) {
            Ok(chart) => chart,
            Err(error) => {
                root.remove();
                tree_stop();
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
            tree_stop();
            return Err(js(error));
        }
        TREES.with(|slot| {
            slot.borrow_mut().push(TreeFixture {
                chart,
                root,
                renderer: name.into(),
                phase: 0,
                events,
            })
        });
    }
    Ok(())
}
#[wasm_bindgen]
pub fn tree_phase(phase: u8) -> std::result::Result<(), JsValue> {
    let scene = source(phase, false).map_err(js)?;
    TREES.with(|slot| {
        for f in slot.borrow_mut().iter_mut() {
            f.chart.set_scene(scene.clone()).map_err(js)?;
            f.phase = phase;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_hidden(renderer: &str, hidden: bool) -> std::result::Result<(), JsValue> {
    TREES.with(|slot| {
        let entries = slot.borrow();
        let f = entries
            .iter()
            .find(|f| f.renderer == renderer)
            .ok_or_else(|| js("missing tree renderer"))?;
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
#[wasm_bindgen]
pub fn tree_flush() -> std::result::Result<(), JsValue> {
    TREES.with(|slot| {
        for f in slot.borrow().iter() {
            f.chart.flush().map_err(js)?;
        }
        Ok(())
    })
}
#[wasm_bindgen]
pub fn tree_snapshot(renderer: &str) -> std::result::Result<String, JsValue> {
    TREES.with(|slot| {
        let entries = slot.borrow();
        let Some(f) = entries.iter().find(|f| f.renderer == renderer) else { return Ok(json!({"active":false}).to_string()); };
        let state = f.chart.snapshot().map_err(js)?;
        let scene = source(f.phase, false).map_err(js)?;
        let node = scene.marks.iter().find(|m| m.id == "node:leaf-a").unwrap();
        let (x,y) = match node.geometry { Geometry::Rect{x,y,width,height} => (x+width/2.,y+height/2.), Geometry::Circle{x,y,..} => (x,y), _ => unreachable!() };
        Ok(json!({"active":state.active,"width":state.width,"height":state.height,"revision":state.revision,"pending":state.pending,"hidden":state.hidden,"hovered":state.hovered,"selected":state.selected,"phase":f.phase,"sceneHit":scene.hit_test(x,y).map(|m|m.id.clone()),"target":{"id":node.id,"series":node.series,"label":node.label,"value":node.value,"x":x,"y":y},"events":f.events.borrow().clone()}).to_string())
    })
}
#[wasm_bindgen]
pub fn tree_rejected() -> bool {
    TREES.with(|slot| {
        slot.borrow().iter().all(|f| {
            if source(f.phase, true).is_ok() {
                return false;
            }
            let Ok(mut scene) = source(f.phase, false) else {
                return false;
            };
            scene.marks[0].opacity = f64::NAN;
            f.chart.set_scene(scene).is_err()
        })
    })
}
#[wasm_bindgen]
pub fn tree_stop() {
    let entries = TREES.with(|slot| std::mem::take(&mut *slot.borrow_mut()));
    for f in entries {
        f.chart.stop();
        f.root.remove();
    }
}
